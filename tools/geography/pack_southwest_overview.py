#!/usr/bin/env python3
"""Build the coarse resident Southwest atlas fallback from the same SWE1 survey."""

import argparse
import array
import json
import struct
import sys
import tempfile
from pathlib import Path

HEADER = struct.Struct("<4s5i")
STRIDE = 16


def nodes(size):
    return [*range(0, size - 1, STRIDE), size - 1]


def neighbourhood(source, center, limit, radius):
    lower = max(0, center - radius)
    upper = min(limit, center + radius + 1)
    return [row[lower:upper] for row in source]


def pack(source_path, output_path):
    if source_path.resolve() == output_path.resolve():
        raise ValueError("Input and output must be different files")
    with source_path.open("rb") as source:
        magic, west, north, step, width, height = HEADER.unpack(source.read(HEADER.size))
        if (magic != b"SWE1" or not 20 <= step <= 300 or min(width, height) < 2
                or source_path.stat().st_size != HEADER.size + 2 * width * height):
            raise ValueError("Input is not a valid SWE1 elevation grid")
        values = array.array("h", source.read())
    if sys.byteorder != "little":
        values.byteswap()

    # The average produces a quiet far-field relief. A second layer stores
    # the maximum across a full coarse-cell radius, so interpolation between
    # any four maxima bounds the original fine heights inside that cell.
    columns = nodes(width)
    rows = nodes(height)
    view = memoryview(values)
    result = bytearray(HEADER.pack(b"SWC1", west, north, step, width, height))
    result.extend(struct.pack("<I", STRIDE))
    peak = -32768
    for row in rows:
        average_lower = max(0, row - STRIDE // 2)
        average_upper = min(height, row + STRIDE // 2 + 1)
        maximum_lower = max(0, row - STRIDE)
        maximum_upper = min(height, row + STRIDE + 1)
        average_rows = [view[r * width:(r + 1) * width] for r in range(average_lower, average_upper)]
        maximum_rows = [view[r * width:(r + 1) * width] for r in range(maximum_lower, maximum_upper)]
        for col in columns:
            band = neighbourhood(average_rows, col, width, STRIDE // 2)
            average = round(sum(sum(samples) for samples in band) / sum(map(len, band)))
            maximum = max(max(samples) for samples in neighbourhood(maximum_rows, col, width, STRIDE))
            peak = max(peak, maximum)
            result.extend(struct.pack("<hh", average, maximum))
    output = tempfile.NamedTemporaryFile(dir=output_path.parent, delete=False)
    staged_path = Path(output.name)
    try:
        with output:
            output.write(result)
        staged_path.replace(output_path)
    finally:
        staged_path.unlink(missing_ok=True)
    return {
        "format": "SWC1",
        "bytes": len(result),
        "overview_grid": [len(columns), len(rows)],
        "nominal_spacing_metres": step * STRIDE,
        "survey_peak_metres": peak,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(pack(args.source, args.output), indent=2))
    except (OSError, ValueError, struct.error) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
