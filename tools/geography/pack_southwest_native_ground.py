#!/usr/bin/env python3
"""Pack and cross-check the whole SWE1 Southwest DEM as independent native tiles."""

import argparse
import array
import json
import mmap
import struct
import sys
import tempfile
import zlib
from pathlib import Path


HEADER = struct.Struct("<4s5i")
TILE_SIDE = 64
MAX_ENCODED_BYTES = 2 * TILE_SIDE * TILE_SIDE + 32


def tile_rows(source, width, height, left, top):
    end_col = min(left + TILE_SIDE, width)
    for row in range(top, min(top + TILE_SIDE, height)):
        start = HEADER.size + 2 * (row * width + left)
        end = HEADER.size + 2 * (row * width + end_col)
        heights = array.array("h", source[start:end])
        if sys.byteorder != "little":
            heights.byteswap()
        yield heights


def encode(rows):
    count = sum(len(row) for row in rows)
    planes = bytearray(count * 2)
    position = 0
    for row in rows:
        previous = 0
        for elevation in row:
            difference = ((elevation - previous + 32768) & 65535) - 32768
            zigzag = (difference << 1) ^ (difference >> 15)
            planes[position] = zigzag & 255
            planes[count + position] = zigzag >> 8
            previous = elevation
            position += 1
    return zlib.compress(planes, 6)


def verify(encoded, rows):
    planes = zlib.decompress(encoded)
    count = sum(len(row) for row in rows)
    if len(planes) != count * 2:
        raise ValueError("Decoded tile has an incorrect length")
    position = 0
    for row in rows:
        previous = 0
        for elevation in row:
            zigzag = planes[position] | (planes[count + position] << 8)
            difference = (zigzag >> 1) ^ -(zigzag & 1)
            previous = ((previous + difference + 32768) & 65535) - 32768
            if elevation != previous:
                raise ValueError("Decoded tile differs from the input DEM")
            position += 1


def pack(source_path, output_path):
    if source_path.resolve() == output_path.resolve():
        raise ValueError("Input and output must be different files")
    with source_path.open("rb") as source_file:
        if source_path.stat().st_size < HEADER.size:
            raise ValueError("Input has no SWE1 header")
        with mmap.mmap(source_file.fileno(), 0, access=mmap.ACCESS_READ) as source:
            magic, west, north, step, width, height = HEADER.unpack_from(source)
            if (magic != b"SWE1" or not 20 <= step <= 300 or min(width, height) < 2
                    or len(source) != HEADER.size + 2 * width * height):
                raise ValueError("Input is not a valid SWE1 elevation grid")
            columns = (width + TILE_SIDE - 1) // TILE_SIDE
            rows = (height + TILE_SIDE - 1) // TILE_SIDE
            payload_start = HEADER.size + 4 * (columns * rows + 1)
            offsets = [0]
            largest_input = 0
            output = tempfile.NamedTemporaryFile(dir=output_path.parent, delete=False)
            staged_path = Path(output.name)
            try:
                with output:
                    output.write(HEADER.pack(b"SWT1", west, north, step, width, height))
                    output.seek(payload_start)
                    for top in range(0, height, TILE_SIDE):
                        for left in range(0, width, TILE_SIDE):
                            elevations = tuple(tile_rows(source, width, height, left, top))
                            encoded = encode(elevations)
                            verify(encoded, elevations)
                            end = offsets[-1] + len(encoded)
                            if len(encoded) > MAX_ENCODED_BYTES or end > 0xffffffff:
                                raise ValueError("Tile payload exceeds native format limits")
                            output.write(encoded)
                            offsets.append(end)
                            largest_input = max(largest_input, len(encoded))
                    output.seek(HEADER.size)
                    output.write(struct.pack(f"<{len(offsets)}I", *offsets))
                staged_path.replace(output_path)
            finally:
                staged_path.unlink(missing_ok=True)
    return {
        "format": "SWT1",
        "source_bytes": HEADER.size + 2 * width * height,
        "pack_bytes": payload_start + offsets[-1],
        "directory_bytes": payload_start - HEADER.size,
        "ground_grid": [width, height],
        "tile_grid": [columns, rows],
        "tile_side": TILE_SIDE,
        "spacing_metres": step,
        "largest_encoded_tile_bytes": largest_input,
        "verified_elevations": width * height,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(pack(args.source, args.output), indent=2))
    except (OSError, ValueError, zlib.error) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
