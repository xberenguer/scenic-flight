#!/usr/bin/env python3
"""Encode a Las Vegas pack for direct sampling from memory-mapped storage."""

import argparse
import gzip
from pathlib import Path
import struct

import numpy as np

HEADER = struct.Struct("<4s15i")
CELL_BITS = np.arange(256, dtype=np.uint16)


def compact(raw):
    header = HEADER.unpack_from(raw)
    magic, _, _, _, ground_width, ground_height, _, _, _, columns, rows, tiles, *_ = header
    if magic != b"LVS1" or min(ground_width, ground_height, columns, rows, tiles) <= 0:
        raise ValueError("expected an LVS1 terrain pack")
    ground_end = HEADER.size + 2 * ground_width * ground_height
    water_end = ground_end + ground_width * ground_height
    index_end = water_end + 4 * columns * rows
    if len(raw) != index_end + tiles * 512:
        raise ValueError("pack length disagrees with its header")

    ground = raw[HEADER.size:ground_end]
    water = np.frombuffer(raw, dtype=np.uint8, count=water_end - ground_end, offset=ground_end)
    index = raw[water_end:index_end]
    cells = np.frombuffer(raw, dtype="<u2", offset=index_end).reshape(tiles, 256)
    records = bytearray()
    offsets = [0]
    for tile in cells:
        palette, codes = np.unique(tile, return_inverse=True)
        last = len(palette) - 1
        width = last.bit_length()
        records.append(last)
        records.extend(palette.astype("<u2", copy=False).tobytes())
        if width:
            bit = CELL_BITS * width
            positioned = codes << (bit & 7)
            block = np.zeros(32 * width + 1, dtype=np.uint8)
            np.bitwise_or.at(block, bit >> 3, positioned.astype(np.uint8))
            np.bitwise_or.at(block, (bit >> 3) + 1, (positioned >> 8).astype(np.uint8))
            words = block[bit >> 3].astype(np.uint16) | (block[(bit >> 3) + 1].astype(np.uint16) << 8)
            if not np.array_equal(palette[(words >> (bit & 7)) & ((1 << width) - 1)], tile):
                raise ValueError("tile failed lossless round trip")
            records.extend(block[:-1])
        offsets.append(len(records))
    record_index = np.asarray(offsets, dtype="<u4").tobytes()
    water_bitmap = np.packbits(water != 0, bitorder="little").tobytes()
    return b"LVS2" + raw[4:HEADER.size] + ground + water_bitmap + index + record_index + records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="LVS1 pack, raw or gzip-compressed")
    parser.add_argument("output", type=Path, help="uncompressed LVS2 pack")
    args = parser.parse_args()
    payload = args.input.read_bytes()
    if payload.startswith(b"\x1f\x8b"):
        payload = gzip.decompress(payload)
    result = compact(payload)
    args.output.write_bytes(result)
    print(f"{len(payload):,} → {len(result):,} bytes ({len(result) / len(payload):.1%} of original)")


if __name__ == "__main__":
    main()
