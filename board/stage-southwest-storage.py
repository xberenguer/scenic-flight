#!/usr/bin/env python3
"""Stage Scenic Flight v0.1.142 atlas files against a copy of the target's storage partition.

This offline tool does not open a serial port or flash a device. The complete
image is for verification. Apply only the manifest's ranges after confirming
the target still matches the recorded original digest.
"""

import argparse
from collections import defaultdict
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import zlib


SECTOR = 4096
FLASH_OFFSET = 0x510000
PARTITION_BYTES = 0x1AF0000
WL_HEADER = 64
WL_RECORD = 16
WL_STATE_BYTES = ((WL_HEADER + PARTITION_BYTES // SECTOR * WL_RECORD + SECTOR - 1) // SECTOR) * SECTOR
WL_CONFIG_OFFSET = PARTITION_BYTES - SECTOR
WL_STATE_OFFSET = WL_CONFIG_OFFSET - 2 * WL_STATE_BYTES
DATA_SECTORS = WL_STATE_OFFSET // SECTOR - 1
MAX_RANGE_SECTORS = 256
DIRECTORY_ENTRY = 32
DIRECTORY = "SCENIC"
PHASES = ("pack-data", "fat-mirror", "fat-primary", "scenic-directory", "root-directory")
ASSETS = {
    "LOSANGEL.PAK": (11931302, b"SCG1", "bb8e3c98da0632ae1b5c4aac0e01804cbdf686922845c000acdf27edb10b38a7"),
    "VEGAS.PAK": (3104264, b"SCG1", "878f1ba8990fe185ef7059063bdebdd171c889982db04db0d772a87924e5b9f0"),
    "SOUTHWST.PAK": (3131379, b"SWT1", "403489cc4f930e8b83feda6f18302dff6d6ecd5a54db3f476622bbab3cb66c85"),
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def crc(data):
    return struct.pack("<I", zlib.crc32(data, 0xffffffff))


def sector(image, index):
    return memoryview(image)[index * SECTOR:(index + 1) * SECTOR]


def wear_mapping(raw):
    require(len(raw) == PARTITION_BYTES, f"Expected a {PARTITION_BYTES}-byte target-board storage partition")
    config = memoryview(raw)[WL_CONFIG_OFFSET:WL_CONFIG_OFFSET + 36]
    expected = (0, PARTITION_BYTES, SECTOR, SECTOR, 16, WL_RECORD, 2, 32)
    require(struct.unpack_from("<8I", config) == expected and crc(config[:32]) == config[32:],
            "Target snapshot has invalid or incompatible ESP-IDF wear-level configuration")
    state = memoryview(raw)[WL_STATE_OFFSET:WL_STATE_OFFSET + WL_STATE_BYTES]
    replica = memoryview(raw)[WL_STATE_OFFSET + WL_STATE_BYTES:WL_CONFIG_OFFSET]
    require(state == replica and crc(state[:60]) == state[60:64],
            "Wear-level state copies disagree or have an invalid checksum")
    start, slots, rotation, _, _, sector_size, version, identifier = struct.unpack_from("<8I", state)
    require((slots, sector_size, version) == (DATA_SECTORS + 1, SECTOR, 2)
            and start < slots and rotation < DATA_SECTORS, "Invalid wear-level state geometry")
    erased = b"\xff" * WL_RECORD
    dummy = None
    for position in range(slots):
        record = state[WL_HEADER + position * WL_RECORD:WL_HEADER + (position + 1) * WL_RECORD]
        if record == erased:
            if dummy is None:
                dummy = position
        else:
            require(dummy is None, "Wear-level journal has active records after an erased record")
            expected = b"".join(crc(struct.pack("<I", (identifier + 4 * position + i) & 0xffffffff))
                                for i in range(4))
            require(record == expected, "Wear-level journal checksum failed")
    require(dummy is not None, "Wear-level journal has no dummy sector")
    rotated = [(logical - rotation) % DATA_SECTORS for logical in range(DATA_SECTORS)]
    mapping = [physical + (physical >= dummy) for physical in rotated]
    require(set(mapping) == set(range(slots)) - {dummy}, "Wear-level sector map is invalid")
    return mapping, dummy


@dataclass(frozen=True)
class Entry:
    kind: str
    size: int
    chain: tuple
    entry_sha: str
    sha: str | None

    def manifest(self):
        return {"kind": self.kind, "bytes": self.size, "clusters": len(self.chain),
                "directory_entry_sha256": self.entry_sha, "sha256": self.sha}


class Fat16:
    def __init__(self, image):
        self.image = image
        boot = sector(image, 0)
        require(boot[510:512] == b"\x55\xaa", "FAT boot signature is missing")
        sector_size, cluster_sectors, reserved, copies, root_entries = struct.unpack_from("<HBHBH", boot, 11)
        total = struct.unpack_from("<H", boot, 19)[0] or struct.unpack_from("<I", boot, 32)[0]
        fat_sectors = struct.unpack_from("<H", boot, 22)[0]
        require(sector_size == SECTOR and cluster_sectors and cluster_sectors & (cluster_sectors - 1) == 0
                and reserved and copies and root_entries and fat_sectors and 4085 <= total <= DATA_SECTORS,
                "Target storage does not contain a supported 4 KiB-sector FAT16 volume")
        self.tables = [range(reserved + i * fat_sectors, reserved + (i + 1) * fat_sectors)
                       for i in range(copies)]
        self.root_start = self.tables[-1].stop
        self.root_entries = root_entries
        self.data_start = self.root_start + (root_entries * DIRECTORY_ENTRY + SECTOR - 1) // SECTOR
        self.cluster_sectors = cluster_sectors
        self.cluster_bytes = SECTOR * cluster_sectors
        self.clusters = (total - self.data_start) // cluster_sectors
        require(4085 <= self.clusters < 65525 and (self.clusters + 2) * 2 <= SECTOR * fat_sectors,
                "FAT table or data area is outside the volume")
        primary = memoryview(image)[self.tables[0].start * SECTOR:self.tables[0].stop * SECTOR]
        for mirror in self.tables[1:]:
            require(primary == memoryview(image)[mirror.start * SECTOR:mirror.stop * SECTOR],
                    "FAT copies disagree")
        self.table = list(struct.unpack_from(f"<{self.clusters + 2}H", primary))
        require(self.table[0] & 0xff00 == 0xff00 and self.table[1] & 0x3fff == 0x3fff,
                "FAT reserved entries are invalid")
        self.entries = {}
        self.owners = set()
        self.read_entries()
        require({index for index, value in enumerate(self.table) if index >= 2 and value not in (0, 0xfff7)}
                == self.owners, "FAT contains lost or inconsistent cluster chains")

    @property
    def free_bytes(self):
        return self.table[2:].count(0) * self.cluster_bytes

    def cluster_offset(self, cluster):
        return (self.data_start + (cluster - 2) * self.cluster_sectors) * SECTOR

    def cluster_view(self, cluster):
        offset = self.cluster_offset(cluster)
        return memoryview(self.image)[offset:offset + self.cluster_bytes]

    def root_slots(self):
        start = self.root_start * SECTOR
        return range(start, start + self.root_entries * DIRECTORY_ENTRY, DIRECTORY_ENTRY)

    def directory_slots(self, chain):
        for cluster in chain:
            start = self.cluster_offset(cluster)
            yield from range(start, start + self.cluster_bytes, DIRECTORY_ENTRY)

    def sectors_for(self, chain):
        return {self.cluster_offset(cluster) // SECTOR + index
                for cluster in chain for index in range(self.cluster_sectors)}

    def read_chain(self, first, name):
        result = []
        require(first == 0 or 2 <= first < len(self.table), f"Invalid first cluster for {name}")
        while first:
            require(2 <= first < len(self.table) and first not in self.owners,
                    f"Invalid or overlapping FAT chain for {name}")
            self.owners.add(first)
            result.append(first)
            first = self.table[first]
            require(first != 0, f"Free cluster terminates {name}")
            if first >= 0xfff8:
                break
        return tuple(result)

    def file_hash(self, size, chain):
        result = hashlib.sha256()
        for cluster in chain:
            used = min(self.cluster_bytes, size)
            result.update(self.cluster_view(cluster)[:used])
            size -= used
        return result.hexdigest()

    def read_entries(self):
        pending = [("", self.root_slots())]
        while pending:
            parent, slots = pending.pop()
            lfn = bytearray()
            for offset in slots:
                record = self.image[offset:offset + DIRECTORY_ENTRY]
                first, attr = record[0], record[11]
                if not first:
                    break
                if first == 0xe5:
                    lfn.clear()
                    continue
                if attr == 0x0f:
                    lfn.extend(record)
                    continue
                entry_hash = digest(lfn + record)
                lfn.clear()
                if attr & 0x08 or record[:1] == b".":
                    continue
                stem = bytes(record[:8]).rstrip(b" ").decode("cp437")
                extension = bytes(record[8:11]).rstrip(b" ").decode("cp437")
                path = parent + stem + ("." + extension if extension else "")
                require(path not in self.entries, f"Duplicate FAT entry {path}")
                cluster, size = struct.unpack_from("<HI", record, 26)
                chain = self.read_chain(cluster, path)
                if attr & 0x10:
                    require(bool(chain) and size == 0, f"Invalid directory {path}")
                    self.entries[path] = Entry("directory", size, chain, entry_hash, None)
                    pending.append((path + "/", self.directory_slots(chain)))
                else:
                    require(len(chain) == (size + self.cluster_bytes - 1) // self.cluster_bytes,
                            f"Wrong FAT chain length for {path}")
                    self.entries[path] = Entry("file", size, chain, entry_hash, self.file_hash(size, chain))

    def set_link(self, cluster, link):
        self.table[cluster] = link
        for table in self.tables:
            struct.pack_into("<H", self.image, table.start * SECTOR + 2 * cluster, link)

    def allocate(self, count):
        chain = [index for index in range(2, len(self.table)) if self.table[index] == 0][:count]
        require(len(chain) == count, "Insufficient free space to stage Scenic Flight maps")
        for index, cluster in enumerate(chain):
            self.set_link(cluster, chain[index + 1] if index + 1 < len(chain) else 0xffff)
        return chain

    def empty_directory(self):
        cluster, = self.allocate(1)
        self.cluster_view(cluster)[:] = bytes(self.cluster_bytes)
        return cluster

    def directory_entry(self, slots, record):
        slots = tuple(slots)
        prior_lfn = False
        for index, offset in enumerate(slots):
            first = self.image[offset]
            if first == 0 or (first == 0xe5 and not prior_lfn):
                self.image[offset:offset + DIRECTORY_ENTRY] = record
                if first == 0 and index + 1 < len(slots):
                    self.image[slots[index + 1]] = 0
                return True
            prior_lfn = first != 0xe5 and self.image[offset + 11] == 0x0f
        return False


def record(name, cluster, size=0, directory=False):
    if name in (".", ".."):
        title = name.ljust(11).encode("ascii")
    else:
        stem, _, tail = name.partition(".")
        title = (stem.ljust(8) + tail.ljust(3)).encode("ascii")
    require(len(title) == 11, f"Invalid staged short filename {name}")
    entry = bytearray(DIRECTORY_ENTRY)
    entry[:11] = title
    entry[11] = 0x10 if directory else 0x20
    # A deterministic valid DOS date for new entries only.
    struct.pack_into("<H", entry, 16, 0x21)
    struct.pack_into("<H", entry, 18, 0x21)
    struct.pack_into("<H", entry, 24, 0x21)
    struct.pack_into("<HI", entry, 26, cluster, size)
    return entry


def load_assets(directory):
    assets = {}
    for name, (size, magic, sha) in ASSETS.items():
        path = directory / name
        require(path.stat().st_size == size, f"Unexpected byte count for {name}")
        data = path.read_bytes()
        require(data[:4] == magic and digest(data) == sha, f"Unverified v0.1.142 map asset: {name}")
        assets[name] = data
    return assets


def stage_files(fat, assets):
    for name, data in assets.items():
        path = DIRECTORY + "/" + name
        old = fat.entries.get(path)
        stem = path.removesuffix(".PAK")
        require(not any(stem + suffix in fat.entries for suffix in (".BAK", ".TMP")),
                f"An interrupted map change must be resolved before replacing {path}")
        if old is not None:
            require(old.kind == "file" and old.size == len(data) and old.sha == ASSETS[name][2],
                    f"Existing map conflicts with the release asset: {path}")
    missing = {name: data for name, data in assets.items() if DIRECTORY + "/" + name not in fat.entries}
    directory = fat.entries.get(DIRECTORY)
    require(directory is None or directory.kind == "directory", "SCENIC exists as a regular file")
    if not missing:
        return set(), ()
    need_data = sum((len(data) + fat.cluster_bytes - 1) // fat.cluster_bytes * fat.cluster_bytes
                    for data in missing.values())
    require(fat.free_bytes >= need_data + (fat.cluster_bytes if directory is None else 0),
            "Insufficient FAT space for the missing atlas maps")

    if directory is None:
        cluster = fat.empty_directory()
        chain = [cluster]
        first = fat.cluster_offset(cluster)
        fat.image[first:first + DIRECTORY_ENTRY] = record(".", cluster, directory=True)
        fat.image[first + DIRECTORY_ENTRY:first + 2 * DIRECTORY_ENTRY] = record("..", 0, directory=True)
        require(fat.directory_entry(fat.root_slots(), record(DIRECTORY, cluster, directory=True)),
                "FAT root directory has no space for SCENIC")
    else:
        chain = list(directory.chain)

    data_sectors = set()
    for name, data in missing.items():
        allocated = fat.allocate((len(data) + fat.cluster_bytes - 1) // fat.cluster_bytes)
        data_sectors.update(fat.sectors_for(allocated))
        view = memoryview(data)
        for index, cluster in enumerate(allocated):
            start = index * fat.cluster_bytes
            chunk = view[start:start + fat.cluster_bytes]
            fat.cluster_view(cluster)[:len(chunk)] = chunk

        entry = record(name, allocated[0], len(data))
        if not fat.directory_entry(fat.directory_slots(chain), entry):
            following = fat.empty_directory()
            fat.set_link(chain[-1], following)
            chain.append(following)
            require(fat.directory_entry(fat.directory_slots([following]), entry),
                    "Could not write the new SCENIC directory entry")
    return data_sectors, tuple(missing)


def verify_entries(before, after):
    targets = {DIRECTORY + "/" + name for name in ASSETS}
    require(after.entries.keys() == before.entries.keys() | targets | {DIRECTORY},
            "Unrelated FAT entries changed when staging the maps")
    for name, entry in before.entries.items():
        actual = after.entries[name]
        if name == DIRECTORY and actual.chain != entry.chain:
            require(actual.chain[:len(entry.chain)] == entry.chain and actual.kind == entry.kind
                    and actual.entry_sha == entry.entry_sha and actual.size == entry.size,
                    "Existing SCENIC directory changed while extending it")
        else:
            require(actual == entry, f"Staging changed existing entry {name}")
    for name, (size, _, sha) in ASSETS.items():
        entry = after.entries[DIRECTORY + "/" + name]
        require(entry.kind == "file" and (entry.size, entry.sha) == (size, sha),
                f"Staged map readback failed: {name}")


def changed_ranges(raw, logical, mapping, before, after, data_sectors):
    phases = {
        "pack-data": data_sectors,
        "fat-mirror": {sector for table in after.tables[1:] for sector in table},
        "fat-primary": set(after.tables[0]),
        "scenic-directory": after.sectors_for(after.entries[DIRECTORY].chain),
        "root-directory": set(range(after.root_start, after.data_start)) if DIRECTORY not in before.entries else set(),
    }
    grouped = defaultdict(list)
    for index, physical in enumerate(mapping):
        if sector(raw, physical) == sector(logical, index):
            continue
        matches = [name for name in PHASES if index in phases[name]]
        require(len(matches) == 1, f"Staging would modify an unrelated sector: {index}")
        grouped[matches[0]].append((physical, index))

    result = []
    cursor = 0
    for phase in PHASES:
        runs = []
        for physical, index in sorted(grouped[phase]):
            if runs and physical == runs[-1][-1][0] + 1 and len(runs[-1]) < MAX_RANGE_SECTORS:
                runs[-1].append((physical, index))
            else:
                runs.append([(physical, index)])
        for run in runs:
            start = run[0][0] * SECTOR
            length = len(run) * SECTOR
            result.append({"id": len(result) + 1, "phase": phase, "partition_offset": start,
                           "flash_offset": FLASH_OFFSET + start, "length": length,
                           "blob_offset": cursor, "logical_sectors": [index for _, index in run]})
            cursor += length
    return result


def write_artifacts(directory, raw, logical, mapping, dummy, ranges):
    target = bytearray(raw)
    original = memoryview(raw)
    with (directory / "forward-sectors.bin").open("wb") as forward, (directory / "rollback-sectors.bin").open("wb") as undo:
        for span in ranges:
            start, length = span["partition_offset"], span["length"]
            expected = bytearray(length)
            for index, logical_sector in enumerate(span["logical_sectors"]):
                offset = index * SECTOR
                expected[offset:offset + SECTOR] = sector(logical, logical_sector)
            previous = original[start:start + length]
            span["before_sha256"] = digest(previous)
            span["after_sha256"] = digest(expected)
            forward.write(expected)
            undo.write(previous)
            target[start:start + length] = expected
    require(sector(target, dummy) == sector(raw, dummy)
            and memoryview(target)[WL_STATE_OFFSET:] == original[WL_STATE_OFFSET:],
            "Staging changed wear-level metadata or the dummy sector")
    for index, physical in enumerate(mapping):
        require(sector(target, physical) == sector(logical, index),
                "Forward ranges do not reconstruct the intended FAT volume")
    staged = directory / "storage-after.bin"
    staged.write_bytes(target)

    # Replay the actual output blobs in both directions. Work in a buffer so
    # no additional partition copies or write-capable device state are needed.
    with (directory / "rollback-sectors.bin").open("rb") as stream:
        for span in reversed(ranges):
            start, length = span["partition_offset"], span["length"]
            stream.seek(span["blob_offset"])
            undo = stream.read(length)
            require(digest(undo) == span["before_sha256"], "Rollback range checksum mismatch")
            require(digest(memoryview(target)[start:start + length]) == span["after_sha256"],
                    "Rollback starting range has changed")
            target[start:start + length] = undo
    require(target == raw, "Rollback did not reconstruct the complete source")
    with (directory / "forward-sectors.bin").open("rb") as stream:
        for span in ranges:
            data = stream.read(span["length"])
            require(digest(data) == span["after_sha256"], "Forward range checksum mismatch")
            start = span["partition_offset"]
            target[start:start + span["length"]] = data
    with staged.open("rb") as stream:
        require(hashlib.file_digest(stream, "sha256").hexdigest() == digest(target),
                "Forward replay does not match the staged partition")


def identify(path):
    with path.open("rb") as stream:
        sha = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"file": path.name, "bytes": path.stat().st_size, "sha256": sha}


def stage(storage, asset_dir, output):
    require(not output.exists() and not output.is_symlink(), "Output directory must not exist")
    require(storage.stat().st_size == PARTITION_BYTES,
            f"Expected a {PARTITION_BYTES}-byte target-board storage partition")
    raw = storage.read_bytes()
    mapping, dummy = wear_mapping(raw)
    assets = load_assets(asset_dir)
    logical = bytearray(DATA_SECTORS * SECTOR)
    for index, physical in enumerate(mapping):
        logical[index * SECTOR:(index + 1) * SECTOR] = sector(raw, physical)
    before = Fat16(logical)
    free_before = before.free_bytes
    if DIRECTORY in before.entries:
        require(before.entries[DIRECTORY].kind == "directory", "SCENIC exists as a regular file")
    # Keep an immutable inventory for verifying the changes to the writable image.
    original_entries = before.entries.copy()
    data_sectors, installed = stage_files(before, assets)
    after = Fat16(logical)
    require(before.entries == original_entries, "Original FAT inventory was modified")
    verify_entries(before, after)
    ranges = changed_ranges(raw, logical, mapping, before, after, data_sectors)
    with tempfile.TemporaryDirectory(dir=output.parent, prefix=".scenic-stage-") as temp:
        staged = Path(temp)
        write_artifacts(staged, raw, logical, mapping, dummy, ranges)
        manifest = {
            "schema": 1,
            "scenic_flight_version": "0.1.142",
            "partition": {"flash_offset": FLASH_OFFSET, "bytes": PARTITION_BYTES, "erase_sector_bytes": SECTOR},
            "base": {"bytes": len(raw), "sha256": digest(raw)},
            "assets": {name: after.entries[DIRECTORY + "/" + name].manifest() for name in ASSETS},
            "installed": list(installed),
            "already_identical": [name for name in ASSETS if name not in installed],
            "preserved_entries": {name: entry.manifest() for name, entry in original_entries.items()
                                  if name not in {DIRECTORY + "/" + file for file in ASSETS}},
            "fat_free_before_bytes": free_before,
            "fat_free_after_bytes": after.free_bytes,
            "artifacts": {kind: identify(staged / name) for kind, name in {
                "staged": "storage-after.bin", "forward": "forward-sectors.bin",
                "rollback": "rollback-sectors.bin",
            }.items()},
            "ranges": ranges,
            "forward_order": [item["id"] for item in ranges],
            "rollback_order": [item["id"] for item in reversed(ranges)],
        }
        (staged / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        require(storage.stat().st_size == len(raw) and identify(storage)["sha256"] == manifest["base"]["sha256"],
                "The target-board snapshot changed during staging")
        require(not output.exists() and not output.is_symlink(), "Output directory was created during staging")
        staged.rename(output)
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("storage", type=Path, help="The target board's 28,246,016-byte storage partition copy")
    parser.add_argument("asset_dir", type=Path, help="Directory of the three exact v0.1.142 PAK assets")
    parser.add_argument("output", type=Path, help="A new directory for the staged image, ranges and manifest")
    args = parser.parse_args()
    try:
        result = stage(args.storage, args.asset_dir, args.output)
    except (OSError, ValueError, struct.error, OverflowError) as error:
        parser.exit(1, f"{error}\n")
    print(f"Staged {len(result['installed'])} map files in {len(result['ranges'])} aligned ranges.")
    print(f"{len(result['already_identical'])} map files already match. {result['fat_free_after_bytes']:,} FAT bytes free.")
    print(args.output / "manifest.json")


if __name__ == "__main__":
    main()
