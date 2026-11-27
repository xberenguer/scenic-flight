#!/usr/bin/env python3
"""Pack the original OSM motorway segments into a sparse Southwest road grid."""

import argparse
from collections import defaultdict
import gzip
import json
import math
from pathlib import Path
import re
import struct


ORIGIN = [384629, 3768405]
CELL = 512
PAGE = 8
PAGE_METRES = CELL * PAGE
ROUTES = ("I15", "I10", "I605", "I210", "I215", "US95", "US395", "CA210")
CLASSES = ("motorway", "trunk", "motorway_link", "trunk_link")
HEADER = struct.Struct("<4sii4H2I")
SEGMENT = struct.Struct("<iihh4B")
MAX_CELL_REFERENCES = 128


def from_osm(sources):
    import osmium
    from pyproj import Transformer

    projection = Transformer.from_crs(4326, 32611, always_xy=True)
    ways = {}

    class Highways(osmium.SimpleHandler):
        def way(self, way):
            road_class = way.tags.get("highway")
            if road_class not in CLASSES or way.id in ways:
                return
            refs = {
                re.sub(r"[^A-Z0-9]", "", ref.upper())
                for ref in re.split(r"[;,]", way.tags.get("ref", ""))
            }.intersection(ROUTES)
            if not refs:
                return
            points = []
            for node in way.nodes:
                if node.location.valid():
                    east, north = projection.transform(node.lon, node.lat)
                    points.append([round(east - ORIGIN[0]), round(north - ORIGIN[1])])
            if len(points) < 2 or not any(
                -43_000 <= east <= 323_000 and -43_000 <= north <= 263_000
                for east, north in points
            ):
                return
            ways[way.id] = {
                "way": way.id, "routes": sorted(refs), "class": road_class,
                "lanes": way.tags.get("lanes"), "points_m": points,
            }

    importer = Highways()
    for source in sources:
        importer.apply_file(str(source), locations=True, idx="flex_mem")
    return {"projection": "EPSG:32611", "origin_utm_m": ORIGIN, "road_segments": list(ways.values())}


def intersects_box(start, delta, box):
    """Test a segment against the rectangle expanded by its shoulder width."""
    first, last = 0.0, 1.0
    for axis in range(2):
        low, high = box[axis], box[axis + 2]
        origin, step = start[axis], delta[axis]
        if step == 0:
            if not low <= origin <= high:
                return False
            continue
        a, b = (low - origin) / step, (high - origin) / step
        first = max(first, min(a, b))
        last = min(last, max(a, b))
        if first > last:
            return False
    return True


def road_widths(lanes, kind):
    count = int(lanes) if isinstance(lanes, str) and lanes.isdecimal() else 2
    count = min(max(count, 1), 12)
    paved_margin = 1.6 if kind == "motorway" else 0.8
    shoulder = 2.7 if kind == "motorway" else 1.8
    asphalt = math.ceil((count * 3.66 / 2 + paved_margin) * 4)
    return asphalt, asphalt + math.ceil(shoulder * 4)


def pack(source):
    if source["projection"] != "EPSG:32611" or source["origin_utm_m"] != ORIGIN:
        raise ValueError("Expected the Southwest atlas UTM coordinate system")
    records = []
    cell_segments = defaultdict(list)
    for feature in source["road_segments"]:
        kind = feature["class"]
        inner, outer = road_widths(feature["lanes"], kind)
        routes = sum(1 << ROUTES.index(route) for route in set(feature["routes"]))
        if not routes:
            raise ValueError(f"OSM way {feature['way']} has no mapped highway reference")
        for (east, north), (next_east, next_north) in zip(feature["points_m"], feature["points_m"][1:]):
            dx, dz = next_east - east, next_north - north
            if dx == dz == 0:
                continue
            record = SEGMENT.pack(east, north, dx, dz, inner, outer, routes, CLASSES.index(kind))
            radius = outer / 4
            min_x = math.floor((min(east, next_east) - radius) / CELL)
            max_x = math.floor((max(east, next_east) + radius) / CELL)
            min_z = math.floor((min(north, next_north) - radius) / CELL)
            max_z = math.floor((max(north, next_north) + radius) / CELL)
            index = len(records)
            records.append(record)
            for z in range(min_z, max_z + 1):
                for x in range(min_x, max_x + 1):
                    bounds = (x * CELL - radius, z * CELL - radius,
                              (x + 1) * CELL + radius, (z + 1) * CELL + radius)
                    if intersects_box((east, north), (dx, dz), bounds):
                        cell_segments[x, z].append(index)

    if not cell_segments or len(records) > 65_536:
        raise ValueError("Segment count exceeds the format or the map is empty")
    pages = defaultdict(dict)
    for (x, z), references in cell_segments.items():
        page_x, cell_x = divmod(x, PAGE)
        page_z, cell_z = divmod(z, PAGE)
        pages[page_x, page_z][cell_z * PAGE + cell_x] = references
    west, south = (min(page[axis] for page in pages) for axis in range(2))
    east, north = (max(page[axis] for page in pages) for axis in range(2))
    cols, rows = east - west + 1, north - south + 1
    directory = [0] * (cols * rows)
    starts = []
    references = []
    for index, page in enumerate(sorted(pages, key=lambda cell: (cell[1], cell[0]))):
        x, z = page
        directory[(z - south) * cols + x - west] = index + 1
        cells = pages[page]
        for cell in range(PAGE * PAGE):
            starts.append(len(references))
            references.extend(cells.get(cell, ()))
        starts.append(len(references))

    maximum = max(map(len, cell_segments.values()))
    if maximum > MAX_CELL_REFERENCES:
        raise ValueError(f"A road cell has {maximum} segment references")
    header = HEADER.pack(b"SRD1", west * PAGE_METRES, south * PAGE_METRES, cols, rows,
                         len(pages), 0, len(records), len(references))
    raw = (header + struct.pack(f"<{len(directory)}H", *directory)
           + struct.pack(f"<{len(starts)}I", *starts)
           + struct.pack(f"<{len(references)}H", *references) + b"".join(records))
    return raw, {
        "osm_ways": len(source["road_segments"]),
        "segments": len(records),
        "cells": len(cell_segments),
        "occupied_pages": len(pages),
        "max_cell_references": maximum,
        "references": len(references),
        "uncompressed_bytes": len(raw),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="Extracted JSON or the first OpenStreetMap PBF")
    parser.add_argument("output", type=Path)
    parser.add_argument("--additional-osm", type=Path, nargs="*", default=[], help="Other PBFs in source order")
    args = parser.parse_args()
    if args.input.suffix == ".pbf":
        source = from_osm([args.input, *args.additional_osm])
    else:
        source = json.loads(args.input.read_text())
    raw, audit = pack(source)
    with args.output.open("wb") as output:
        with gzip.GzipFile(filename="", fileobj=output, mode="wb", mtime=0, compresslevel=9) as compressed:
            compressed.write(raw)
    audit["compressed_bytes"] = args.output.stat().st_size
    print(json.dumps(audit, indent=2))


if __name__ == "__main__":
    main()
