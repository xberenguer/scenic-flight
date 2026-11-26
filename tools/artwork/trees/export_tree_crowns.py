#!/usr/bin/env python3
"""Export every current Scenic Flight crown as isolated native geometry."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import zlib

HERE = Path(__file__).resolve().parent
GAME = HERE.parents[2]
OUTPUT = Path(os.environ.get("SCENIC_ARTWORK_OUTPUT", HERE)).resolve()
CELL = 64
GRID = 16
SIDE = CELL * GRID
SHEET_CELL = 77
SHEET_MARGIN = 10
BACKGROUND = bytes((19, 24, 30, 255))
LABEL = bytes((186, 195, 182, 255))
DIGITS = (
    ("111", "101", "101", "101", "111"), ("010", "110", "010", "010", "111"),
    ("111", "001", "111", "100", "111"), ("111", "001", "111", "001", "111"),
    ("101", "101", "111", "001", "001"), ("111", "100", "111", "001", "111"),
    ("111", "100", "111", "101", "111"), ("111", "001", "010", "010", "010"),
    ("111", "101", "111", "101", "111"), ("111", "101", "111", "001", "111"),
)


def png(path, width, height, rgba):
    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data))

    stride = width * 4
    scanlines = b"".join(b"\0" + rgba[i:i + stride] for i in range(0, len(rgba), stride))
    assert len(scanlines) == (stride + 1) * height
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(scanlines, 9))
        + chunk(b"IEND", b"")
    )


def render():
    crate_source = GAME / "src"
    env = {**os.environ, "SCENIC_GAME_SOURCE": str(crate_source)}
    with tempfile.TemporaryDirectory(prefix="crown-export-", dir=HERE) as temp:
        scratch = Path(temp)
        command = [
            "cargo", "build", "--manifest-path", str(GAME / "Cargo.toml"),
            "--lib", "--locked", "--release", "--message-format=json",
            "--target-dir", str(scratch / "target"),
        ]
        artifacts = subprocess.run(command, check=True, text=True, stdout=subprocess.PIPE).stdout
        engine, = [
            name for line in artifacts.splitlines()
            if (item := json.loads(line)).get("reason") == "compiler-artifact"
            if item["target"]["name"] == "scenic_flight"
            for name in item["filenames"] if name.endswith(".rlib")
        ]
        shutil.copy(HERE / "render.rs", scratch / "render.rs")
        geometry = (crate_source / "ground_details.rs").read_bytes()
        (scratch / "ground_details_export.rs").write_bytes(
            geometry + (HERE / "crown-adapter.rs").read_bytes()
        )
        binary = scratch / "render"
        subprocess.run(
            ["rustc", "--edition=2024", "-O", "-A", "dead_code", "--extern",
             f"scenic_flight={engine}", str(scratch / "render.rs"), "-o", str(binary)],
            env=env, check=True,
        )
        rgba = subprocess.check_output([binary], cwd=scratch)
        measurements = (scratch / "crown-metrics.tsv").read_text()
        return rgba, measurements, hashlib.sha256(geometry).hexdigest()


def contact_sheet(rgba, measurements):
    atlas_stride = SIDE * 4
    sheet_side = GRID * SHEET_CELL + 2 * SHEET_MARGIN
    sheet = bytearray(BACKGROUND * (sheet_side ** 2))
    records = []
    signatures = set()

    for variant, entry in enumerate(measurements.splitlines()):
        variant_id, form, width, depth, height = entry.split("\t")
        assert int(variant_id) == variant
        x = variant % GRID * CELL
        y = variant // GRID * CELL
        sx = SHEET_MARGIN + variant % GRID * SHEET_CELL + (SHEET_CELL - CELL) // 2
        sy = SHEET_MARGIN + variant // GRID * SHEET_CELL
        cell = bytearray()
        for ry in range(CELL):
            start = (y + ry) * atlas_stride + x * 4
            row = rgba[start:start + CELL * 4]
            cell.extend(row)
            for rx in range(CELL):
                piece = row[rx * 4:rx * 4 + 4]
                if piece[3] != 0:
                    index = ((sy + ry) * sheet_side + sx + rx) * 4
                    sheet[index:index + 4] = piece
        opaque = [(i % CELL, i // CELL) for i in range(CELL * CELL) if cell[i * 4 + 3]]
        assert len(opaque) > 200, f"tree crown {variant} is empty or illegible"
        assert all(0 < ox < CELL - 1 and 0 < oy < CELL - 1 for ox, oy in opaque), f"clipped tree crown {variant}"
        signature = hashlib.sha256(cell).digest()
        assert signature not in signatures, f"identical isolated artwork at tree crown {variant}"
        signatures.add(signature)

        tag = f"{variant:03d}"
        for position, digit in enumerate(tag):
            for dy, glyph_row in enumerate(DIGITS[int(digit)]):
                for dx, pixel in enumerate(glyph_row):
                    if pixel == "1":
                        start_y = sy + 58 + dy * 2
                        start_x = sx + (CELL - 22) // 2 + position * 8 + dx * 2
                        for repeat in range(2):
                            index = ((start_y + repeat) * sheet_side + start_x) * 4
                            sheet[index:index + 8] = LABEL * 2

        records.append({
            "id": variant,
            "form": form,
            "width": float(width),
            "depth": float(depth),
            "height": float(height),
            "x": x,
            "y": y,
        })

    assert len(records) == len(signatures) == GRID * GRID
    png(OUTPUT / "tree-crown-contact-sheet.png", sheet_side, sheet_side, sheet)
    return records


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    rgba, measurements, source_sha = render()
    assert len(rgba) == SIDE * SIDE * 4
    records = contact_sheet(rgba, measurements)
    png(OUTPUT / "tree-crown-atlas.png", SIDE, SIDE, rgba)
    manifest = {
        "columns": GRID,
        "rows": GRID,
        "tile_width": CELL,
        "tile_height": CELL,
        "ground_details_sha256": source_sha,
        "variants": records,
    }
    (OUTPUT / "tree-crown-atlas.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Verified {len(records)} distinct, unclipped transparent cells at {CELL} x {CELL}.")


if __name__ == "__main__":
    main()
