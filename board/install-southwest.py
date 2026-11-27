#!/usr/bin/env python3
"""Install the optional v0.1.142 Southwest atlas release on a compatible Waveshare."""

import argparse
from collections import defaultdict
from contextlib import redirect_stderr, redirect_stdout
from dataclasses import dataclass
from functools import reduce
import hashlib
import importlib.util
import json
from operator import xor
import os
from pathlib import Path
import re
import shutil
import struct
import sys
from tempfile import TemporaryDirectory, mkdtemp

HERE = Path(__file__).resolve().parent
VERSION = "0.1.142"
APP_NAME = "scenic-flight-waveshare175c-southwest-app.bin"
STAGE_NAME = "stage-southwest-storage.py"
PARTITION_TABLE = 0x8000
SECTOR = 4096
FACTORY, FACTORY_BYTES = 0x10000, 0x500000
STORAGE, STORAGE_BYTES = 0x510000, 0x1af0000
FLASH_BYTES, FLASH_ID = 0x2000000, 0x1940c8
BAUD = 1_500_000
FREE_BYTES = 160 * 1024 * 1024
PHASES = ("pack-data", "fat-mirror", "fat-primary", "scenic-directory", "root-directory")
PARTITIONS = (
    ("nvs", 1, 2, 0x9000, 0x6000),
    ("phy_init", 1, 1, 0xf000, 0x1000),
    ("factory", 0, 0, FACTORY, FACTORY_BYTES),
    ("storage", 1, 0x81, STORAGE, STORAGE_BYTES),
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def file_sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def load_stager(directory):
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location("scenic_southwest_stager", directory / STAGE_NAME)
    require(spec is not None and spec.loader is not None, "Missing Southwest storage staging tool")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _validate_app(data):
    require(24 < len(data) <= FACTORY_BYTES and data[0] == 0xe9
            and 1 <= data[1] <= 16 and data[3] >> 4 == 5
            and struct.unpack_from("<H", data, 12)[0] == 9 and data[23] == 1,
            "Release app is not the expected digest-bearing ESP32-S3 32 MiB image")
    cursor = 24
    checksum = 0xef
    for _ in range(data[1]):
        require(cursor + 8 <= len(data), "Truncated app segment header")
        length = struct.unpack_from("<I", data, cursor + 4)[0]
        cursor += 8
        require(cursor + length <= len(data), "Truncated app segment")
        checksum = reduce(xor, memoryview(data)[cursor:cursor + length], checksum)
        cursor += length
    end = (cursor + 16) // 16 * 16
    require(end + 32 == len(data) and data[end - 1] == checksum
            and hashlib.sha256(data[:end]).digest() == data[end:],
            "Release app internal checksum or SHA-256 failed")


def _validate_release(directory):
    sums = {}
    for line in (directory / "SHA256SUMS").read_text().splitlines():
        if not line.strip():
            continue
        match = re.fullmatch(r"([0-9a-fA-F]{64}) [ *]([^/\\]+)", line)
        require(match and match[2] not in (".", "..") and match[2] not in sums, "Invalid or duplicate SHA256SUMS entry")
        sums[match[2]] = match[1].lower()
    expected = {APP_NAME, "LOSANGEL.PAK", "VEGAS.PAK", "SOUTHWST.PAK"}
    require(expected <= sums.keys(), "SHA256SUMS is missing the app or an audited map")
    for name, digest in sums.items():
        require(file_sha(directory / name) == digest, f"Release checksum failed for {name}")
    app = (directory / APP_NAME).read_bytes()
    require(sha(app) == sums[APP_NAME], "Release app changed during verification")
    _validate_app(app)
    stager = load_stager(directory)
    stager.load_assets(directory)
    require(set(stager.ASSETS) == expected - {APP_NAME}, "Unexpected release map set")
    return app, stager


def _validate_partitions(raw):
    require(len(raw) == SECTOR, "Incomplete flash partition table")
    for index, (label, kind, subtype, start, length) in enumerate(PARTITIONS):
        record = struct.unpack_from("<HBBII16sI", raw, index * 32)
        require(record == (0x50aa, kind, subtype, start, length, label.encode().ljust(16, b"\0"), 0),
                "Connected board does not have the measured Waveshare partition table")
    end = len(PARTITIONS) * 32
    require(raw[end:end + 16] == b"\xeb\xeb" + b"\xff" * 14
            and raw[end + 16:end + 32] == hashlib.md5(raw[:end]).digest()
            and raw[end + 32:] == b"\xff" * (SECTOR - end - 32),
            "Partition table digest or unused area is incompatible")


@dataclass(frozen=True)
class Range:
    address: int
    old: bytes
    new: bytes


@dataclass(frozen=True)
class Plan:
    staged: Path
    phases: tuple


def _artifact(directory, meta, filename):
    require(meta["file"] == filename, f"Unexpected artifact path for {filename}")
    path = directory / filename
    data = path.read_bytes()
    require(len(data) == meta["bytes"] and sha(data) == meta["sha256"], f"Staged artifact changed: {filename}")
    return path, data


def _validate(storage, directory, stager):
    plan = json.loads((directory / "manifest.json").read_text())
    require(plan["schema"] == 1 and plan["scenic_flight_version"] == VERSION
            and plan["partition"] == {"flash_offset": STORAGE, "bytes": STORAGE_BYTES, "erase_sector_bytes": SECTOR},
            "Unexpected storage manifest format or partition")
    before = storage.read_bytes()
    require(plan["base"] == {"bytes": STORAGE_BYTES, "sha256": sha(before)} and len(before) == STORAGE_BYTES,
            "Storage backup no longer matches the staged manifest")
    artifacts = plan["artifacts"]
    staged, after = _artifact(directory, artifacts["staged"], "storage-after.bin")
    _, forward = _artifact(directory, artifacts["forward"], "forward-sectors.bin")
    _, rollback = _artifact(directory, artifacts["rollback"], "rollback-sectors.bin")
    require(len(after) == STORAGE_BYTES and len(forward) == len(rollback), "Unexpected storage artifact size")
    mapping, dummy = stager.wear_mapping(before)
    require(stager.wear_mapping(after) == (mapping, dummy)
            and stager.sector(before, dummy) == stager.sector(after, dummy)
            and before[stager.WL_STATE_OFFSET:] == after[stager.WL_STATE_OFFSET:],
            "Storage delta modifies wear-level metadata or its dummy sector")
    old_fat = stager.Fat16(b"".join(stager.sector(before, index) for index in mapping))
    new_fat = stager.Fat16(b"".join(stager.sector(after, index) for index in mapping))
    stager.verify_entries(old_fat, new_fat)
    added = [name for name in stager.ASSETS if f"SCENIC/{name}" not in old_fat.entries]
    existing = [name for name in stager.ASSETS if name not in added]
    require(plan["installed"] == added and plan["already_identical"] == existing
            and plan["assets"] == {name: new_fat.entries[f"SCENIC/{name}"].manifest() for name in stager.ASSETS},
            "Staged map inventory differs from the planned release")
    data = set()
    for name in added:
        data.update(new_fat.sectors_for(new_fat.entries[f"SCENIC/{name}"].chain))
    allowed = {
        "pack-data": data,
        "fat-mirror": {i for table in new_fat.tables[1:] for i in table},
        "fat-primary": set(new_fat.tables[0]),
        "scenic-directory": new_fat.sectors_for(new_fat.entries["SCENIC"].chain),
        "root-directory": set(range(new_fat.root_start, new_fat.data_start)) if "SCENIC" not in old_fat.entries else set(),
    }
    named = {physical: logical for logical, physical in enumerate(mapping)}
    updated, restored = bytearray(before), bytearray(after)
    ranges = plan["ranges"]
    ids = list(range(1, len(ranges) + 1))
    require([item["id"] for item in ranges] == ids and plan["forward_order"] == ids
            and plan["rollback_order"] == ids[::-1], "Missing or out-of-order forward and rollback ranges")
    grouped = defaultdict(list)
    touched = set()
    cursor, previous = 0, -1
    for item in ranges:
        start, length, address = (item[key] for key in ("partition_offset", "length", "flash_offset"))
        phase = item["phase"]
        require(phase in PHASES and PHASES.index(phase) >= previous
                and start >= 0 and length > 0 and start % SECTOR == length % SECTOR == 0
                and address == STORAGE + start and start + length <= stager.WL_STATE_OFFSET
                and item["blob_offset"] == cursor, "Invalid storage range address, phase or alignment")
        physical = list(range(start // SECTOR, (start + length) // SECTOR))
        logical = [named.get(index) for index in physical]
        require(logical == item["logical_sectors"] and set(logical) <= allowed[phase]
                and not touched.intersection(physical), "Range touches unrelated or overlapping FAT sectors")
        old, new = rollback[cursor:cursor + length], forward[cursor:cursor + length]
        require(len(old) == len(new) == length and sha(old) == item["before_sha256"]
                and sha(new) == item["after_sha256"] and before[start:start + length] == old
                and after[start:start + length] == new
                and all(stager.sector(old, i) != stager.sector(new, i) for i in range(length // SECTOR)),
                "Invalid or non-changing storage range payload")
        updated[start:start + length] = new
        restored[start:start + length] = old
        grouped[phase].append(Range(address, old, new))
        touched.update(physical)
        cursor += length
        previous = PHASES.index(phase)
    require(cursor == len(forward) == len(rollback) and updated == after and restored == before
            and bool(added) == bool(ranges), "Ranges do not reconstruct both complete storage images")
    return Plan(staged, tuple(grouped.items()))


class Device:
    def __init__(self, port):
        import esptool
        require(esptool.__version__ == "4.9.0", "Install esptool==4.9.0 for the verified Waveshare loader")
        self.tool, self.port = esptool, port
        self.esp, self.mac = None, None

    def identity(self):
        require(self.esp is not None and self.esp.CHIP_NAME == "ESP32-S3", "Connected chip is not ESP32-S3")
        mac = bytes(self.esp.read_mac())
        require(len(mac) == 6 and mac not in (b"\0" * 6, b"\xff" * 6)
                and (self.mac is None or mac == self.mac), "Connected board MAC does not match this transaction")
        return mac

    def connect(self):
        self.close()
        self.esp = self.tool.detect_chip(port=self.port)
        mac = self.identity()
        require(not self.esp.secure_download_mode, "Target did not enter the expected SPI flash loader")
        self.esp = self.esp.run_stub()
        self.esp.change_baud(BAUD)
        require(self.identity() == mac and self.esp.flash_id() == FLASH_ID,
                "Target must have the measured 32 MiB GigaDevice SPI flash")
        self.esp.flash_set_parameters(FLASH_BYTES)
        self.mac = mac
        return mac

    def close(self):
        if self.esp is not None:
            port, self.esp = self.esp._port, None
            port.close()

    def read(self, address, length):
        self.identity()
        data = self.esp.read_flash(address, length)
        self.identity()
        require(len(data) == length, "The device returned a partial backup")
        return data

    def run(self, label, operation, pairs, log_dir):
        path = log_dir / f"{label}.log"
        args = [item for address, file in pairs for item in (hex(address), str(file))]
        with path.open("w") as stream, redirect_stdout(stream), redirect_stderr(stream):
            self.identity()
            self.tool.main(["--chip", "esp32s3", "--port", self.port, "--baud", str(BAUD),
                            "--before", "no_reset_no_sync", "--no-stub", "--after", "no_reset_stub",
                            operation, *args], esp=self.esp)
            self.identity()
        if operation == "verify_flash":
            require("-- verify OK (digest matched)" in path.read_text(errors="replace"),
                    f"Device verification did not succeed. See {path}")
        print(f"{label} verified. {path}", flush=True)

    def verify(self, label, address, file, log_dir):
        self.run(label, "verify_flash", [(address, file)], log_dir)

    def write(self, label, pairs, log_dir):
        self.run(label, "write_flash", pairs, log_dir)

    def reset(self):
        self.identity()
        self.esp.hard_reset()


def _save(path, content):
    with path.open("xb") as stream:
        stream.write(content)
        stream.flush()
        os.fsync(stream.fileno())
    return {"file": path.name, "bytes": len(content), "sha256": sha(content)}


def _write_ranges(device, label, ranges, scratch, logs, undo=False):
    with TemporaryDirectory(prefix=".ranges-", dir=scratch) as temporary:
        files = []
        for item in ranges:
            path = Path(temporary) / f"{item.address:08x}-{'old' if undo else 'new'}.bin"
            path.write_bytes(item.old if undo else item.new)
            files.append((item.address, path))
        device.write(label, files, logs)


def install(device, release, backup_parent, app, stager):
    mac = device.connect()
    table = device.read(PARTITION_TABLE, SECTOR)
    _validate_partitions(table)
    backup_parent.mkdir(parents=True, exist_ok=True)
    require(shutil.disk_usage(backup_parent).free >= FREE_BYTES,
            "The backup location needs at least 160 MiB free for the backups and temporary staging")
    backup = Path(mkdtemp(prefix=f"ScenicFlight-backup-{mac.hex()}-", dir=backup_parent))
    print(f"Verified hardware {mac.hex(':')}. Saving backups to {backup}", flush=True)
    factory, storage = backup / "factory-before.bin", backup / "storage-before.bin"
    inventory = {"mac": mac.hex(":"), "flash_bytes": FLASH_BYTES, "release_app_sha256": sha(app),
                 "partition_table": _save(backup / "partition-table.bin", table)}
    for name, address, length, target in (("factory", FACTORY, FACTORY_BYTES, factory),
                                          ("storage", STORAGE, STORAGE_BYTES, storage)):
        print(f"Reading all {length:,} bytes of {name}", flush=True)
        inventory[name] = _save(target, device.read(address, length))
        require(file_sha(target) == inventory[name]["sha256"], f"The saved {name} backup failed readback")
    _save(backup / "backup.json", (json.dumps(inventory, indent=2) + "\n").encode())
    logs = backup / "logs"
    logs.mkdir()
    device.verify("before-factory", FACTORY, factory, logs)
    device.verify("before-storage", STORAGE, storage, logs)
    attempted, app_attempted = [], False
    with TemporaryDirectory(prefix=".install-", dir=backup) as temporary:
        scratch = Path(temporary)
        stage_dir = scratch / "staged-storage"
        stager.stage(storage, release, stage_dir)
        plan = _validate(storage, stage_dir, stager)
        _save(logs / "storage-plan.json", (stage_dir / "manifest.json").read_bytes())
        app_file = scratch / APP_NAME
        app_file.write_bytes(app)
        try:
            for phase, ranges in plan.phases:
                attempted.append((phase, ranges))
                _write_ranges(device, phase, ranges, scratch, logs)
            device.verify("installed-storage", STORAGE, plan.staged, logs)
            app_attempted = True
            device.write("install-app", [(FACTORY, app_file)], logs)
            device.verify("installed-app", FACTORY, app_file, logs)
        except (Exception, KeyboardInterrupt, SystemExit) as failure:
            if not attempted and not app_attempted:
                raise
            try:
                require(file_sha(factory) == inventory["factory"]["sha256"]
                        and file_sha(storage) == inventory["storage"]["sha256"],
                        "A verified rollback backup changed after capture")
                require(device.connect() == mac and device.read(PARTITION_TABLE, SECTOR) == table,
                        "Rollback target or partition table changed")
                if app_attempted:
                    device.write("rollback-app", [(FACTORY, factory)], logs)
                for phase, ranges in reversed(attempted):
                    _write_ranges(device, f"rollback-{phase}", tuple(reversed(ranges)), scratch, logs, undo=True)
                device.verify("rollback-factory", FACTORY, factory, logs)
                device.verify("rollback-storage", STORAGE, storage, logs)
            except (Exception, KeyboardInterrupt, SystemExit) as rollback_error:
                raise RuntimeError(f"Installation failed ({failure}). Rollback could not be verified. Backups: {backup}") from rollback_error
            raise RuntimeError(f"Installation failed. The original factory and storage images were restored and verified. Backups: {backup}") from failure
    try:
        device.reset()
    except Exception as error:
        raise RuntimeError(f"Installation verified, but reset did not complete. Backups: {backup}") from error
    return backup


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", help="Explicit serial port for the target Waveshare")
    parser.add_argument("--backup-dir", type=Path, default=Path.cwd(), help="Parent directory for a new backup folder")
    parser.add_argument("--validate-only", action="store_true", help="Verify the local release without accessing a device")
    args = parser.parse_args()
    if not args.validate_only and not args.port:
        parser.error("--port is required to install")
    try:
        app, stager = _validate_release(HERE)
        print("Verified the release app, SHA256SUMS and all three audited map packs.", flush=True)
        if args.validate_only:
            return
        device = Device(args.port)
        try:
            backup = install(device, HERE, args.backup_dir.resolve(), app, stager)
        finally:
            device.close()
        print(f"Installed and verified Scenic Flight {VERSION}. Backups: {backup}")
    except (Exception, KeyboardInterrupt, SystemExit) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
