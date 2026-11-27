#!/usr/bin/env python3
"""Extract and pack 4 m surveyed town surfaces along the Southwest atlas."""

import argparse
from collections import Counter
import gzip
import json
import math
from pathlib import Path
import struct

import numpy as np
import osmium
from pyproj import Transformer
from rasterio.features import rasterize
from rasterio.transform import Affine
from shapely.geometry import LineString, Polygon, box

from build_las_vegas import WIDTHS, metres


ORIGIN = (384629, 3768405)
PROJECT = Transformer.from_crs(4326, 32611, always_xy=True)
UNPROJECT = Transformer.from_crs(32611, 4326, always_xy=True)
CELL = 4
TILE = 16
SPAN = CELL * TILE
HEADER = struct.Struct("<4sHHII")
PATCH = struct.Struct("<iiHHI")
PATCHES = (
    ("Baker", -116.074646, 35.264686, 2500, 1950),
    ("Barstow", -117.0220, 34.8958, 5600, 3700),
    ("Primm", -115.3905, 35.6110, 1900, 1950),
)
COUNTY_ROADS = {
    "motorway", "trunk", "primary", "secondary", "tertiary", "unclassified", "residential",
    "living_street", "service", "road", "track", "motorway_link", "trunk_link", "primary_link",
    "secondary_link", "tertiary_link",
}
RAILS = {"rail", "light_rail", "narrow_gauge", "tram"}
FEATURE_TAGS = (
    "building", "building:part", "building:levels", "height", "min_height", "building:material",
    "roof:shape", "highway", "surface", "width", "lanes", "railway", "aeroway", "amenity",
    "landuse", "leisure", "name", "ref", "bridge", "oneway",
)
PAVEMENT, GRAVEL, RAIL, LOT, PARK, FLAT_ROOF, PITCHED_ROOF, INDUSTRIAL_ROOF = range(1, 9)


def patch_bounds(lon, lat, east_half, north_half):
    x, z = PROJECT.transform(lon, lat)
    west = math.floor((x - ORIGIN[0] - east_half) / SPAN) * SPAN
    south = math.floor((z - ORIGIN[1] - north_half) / SPAN) * SPAN
    east = math.ceil((x - ORIGIN[0] + east_half) / SPAN) * SPAN
    north = math.ceil((z - ORIGIN[1] + north_half) / SPAN) * SPAN
    corners = [UNPROJECT.transform(x + ORIGIN[0], z + ORIGIN[1])
               for x in (west, east) for z in (south, north)]
    lons, lats = zip(*corners)
    return (west, south, east, north), (min(lons), min(lats), max(lons), max(lats))


def relevant(probes, bounds, linear):
    points = [(point.lon, point.lat) for point in probes if point.location.valid()]
    if not points:
        return False
    west, south, east, north = bounds
    if any(west <= lon <= east and south <= lat <= north for lon, lat in points):
        return True
    if not linear:
        return False
    lons, lats = zip(*points)
    return min(lons) <= east and max(lons) >= west and min(lats) <= north and max(lats) >= south


def feature_kind(tags):
    building = tags.get("building") or tags.get("building:part")
    if building and building not in ("no", "construction", "proposed", "demolished"):
        return "building"
    if tags.get("highway") in COUNTY_ROADS:
        return "road"
    if tags.get("railway") in RAILS:
        return "rail"
    if tags.get("aeroway") in ("runway", "taxiway", "apron"):
        return "air"
    if ((tags.get("amenity") == "parking" and tags.get("parking") not in ("underground", "multi-storey"))
            or tags.get("amenity") == "fuel" or tags.get("landuse") in ("retail", "commercial")):
        return "lot"
    if tags.get("leisure") in ("pitch", "park"):
        return "park"
    return None


class TownExtractor(osmium.SimpleHandler):
    def __init__(self):
        super().__init__()
        self.zones = [(name, *patch_bounds(lon, lat, ew, ns)) for name, lon, lat, ew, ns in PATCHES]
        self.patches = {name: {} for name, _, _ in self.zones}

    def way(self, way):
        tags = way.tags
        kind = feature_kind(tags)
        nodes = way.nodes
        if not kind or len(nodes) < 2 or tags.get("tunnel") not in (None, "no") or tags.get("location") == "underground":
            return
        probes = (nodes[0], nodes[len(nodes) - 1], nodes[len(nodes) // 2])
        zones = [name for name, _, geographic in self.zones if way.id not in self.patches[name]
                 and relevant(probes, geographic, kind in ("road", "rail", "air"))]
        if not zones:
            return
        points = [(node.lon, node.lat) for node in nodes if node.location.valid()]
        if len(points) < 2:
            return
        lons, lats = zip(*points)
        east, north = PROJECT.transform(lons, lats)
        coords = [(round(x - ORIGIN[0], 2), round(z - ORIGIN[1], 2)) for x, z in zip(east, north)]
        feature = {"way": way.id, "kind": kind, "coords": coords,
                   "tags": {key: tags[key] for key in FEATURE_TAGS if key in tags}}
        for name in zones:
            self.patches[name][way.id] = feature

    def survey(self):
        return {"projection": "EPSG:32611", "origin_utm_m": ORIGIN,
                "patches": [{"name": name, "bounds_m": bounds,
                             "features": list(self.patches[name].values())} for name, bounds, _ in self.zones]}


def roof_material(tags):
    building = tags.get("building")
    if tags.get("roof:shape") in ("gabled", "hipped", "pyramidal", "gambrel", "half-hipped", "round"):
        return PITCHED_ROOF
    if tags.get("building:material") == "metal" or building in ("industrial", "warehouse", "hangar"):
        return INDUSTRIAL_ROOF
    return FLAT_ROOF


def building_word(tags, area):
    height = metres(tags.get("height"), None)
    levels = metres(tags.get("building:levels"), None)
    if height is not None:
        evidence = "height tag"
    elif levels is not None:
        height, evidence = levels * 3.2, "floor estimate"
    else:
        building = tags.get("building")
        height = 3.0 if area < 60 or building in ("shed", "garage", "garages", "carport") else 4.0
        if building in ("warehouse", "industrial", "hangar") or area > 1800:
            height = 5.8
        evidence = "generic low building"
    return min(4095, max(1, round(height * 10))) << 4 | roof_material(tags), evidence


def footprint(points):
    if len(points) < 4 or points[0] != points[-1]:
        return None
    shape = Polygon(points)
    return shape if shape.is_valid else shape.buffer(0)


def geometry(feature):
    tags, points, kind = (feature[key] for key in ("tags", "coords", "kind"))
    if kind in ("building", "lot", "park") or tags.get("aeroway") == "apron":
        shape = footprint(points)
        if shape is None or shape.is_empty or shape.area < 4:
            return None
        if kind == "building":
            word, evidence = building_word(tags, shape.area)
            return shape, word, 4, evidence
        return shape, PARK if kind == "park" else LOT, 0, None
    line = LineString(points)
    if not line.length:
        return None
    width = metres(tags.get("width"), None)
    if kind == "rail":
        return line.buffer(2.2 if width is None else width / 2, cap_style="flat"), RAIL, 2, None
    if kind == "air":
        width = width or (32.0 if tags.get("aeroway") == "runway" else 12.0)
        return line.buffer(width / 2, cap_style="flat"), PAVEMENT, 3, None
    highway = tags["highway"]
    lanes = tags.get("lanes", "")
    if width is None:
        default = WIDTHS.get(highway, 5.2)
        width = max(default, 3.2 * int(lanes)) if lanes.isdecimal() else default
    rough = highway == "track" or tags.get("surface") in ("unpaved", "gravel", "dirt", "ground", "sand", "compacted")
    return line.buffer(min(64, max(1, width)) / 2, cap_style="flat"), GRAVEL if rough else PAVEMENT, 1, None


def bake_patch(patch):
    west, south, east, north = patch["bounds_m"]
    bounds = box(west, south, east, north)
    layers = [[] for _ in range(5)]
    counts, heights = Counter(), Counter()
    for feature in patch["features"]:
        paint = geometry(feature)
        if paint is None:
            continue
        shape, word, layer, evidence = paint
        clipped = bounds.intersection(shape)
        if clipped.is_empty or clipped.area == 0:
            continue
        layers[layer].append((clipped, word))
        counts[feature["kind"]] += 1
        if evidence:
            heights[evidence] += 1
    dimensions = ((north - south) // CELL, (east - west) // CELL)
    pixels = np.zeros(dimensions, dtype=np.uint16)
    transform = Affine.translation(west, south) * Affine.scale(CELL, CELL)
    layers[-1].sort(key=lambda entry: entry[1] >> 4)
    for layer, entries in enumerate(layers):
        if entries:
            rasterize(entries, out=pixels, transform=transform, all_touched=layer in (1, 2))
    return pixels, {"features": dict(counts), "building_heights": dict(heights),
                    "height_cells": int(np.count_nonzero(pixels >> 4))}


def tile_record(tile):
    colors, codes = np.unique(tile, return_inverse=True)
    last = len(colors) - 1
    width = last.bit_length()
    record = bytearray([last])
    record.extend(colors.astype("<u2", copy=False).tobytes())
    if width:
        bit = np.arange(TILE * TILE, dtype=np.uint16) * width
        values = codes << (bit & 7)
        packed = np.zeros(TILE * TILE * width // 8 + 1, dtype=np.uint8)
        np.bitwise_or.at(packed, bit // 8, values.astype(np.uint8))
        np.bitwise_or.at(packed, bit // 8 + 1, (values >> 8).astype(np.uint8))
        record.extend(packed[:-1].tobytes())
    return bytes(record)


def pack(survey):
    if survey["projection"] != "EPSG:32611" or tuple(survey["origin_utm_m"]) != ORIGIN:
        raise ValueError("Expected the Southwest atlas UTM coordinate system")
    patches, tile_index, offsets = [], [], [0]
    records, used = bytearray(), {}
    audits = {}
    for patch in survey["patches"]:
        west, south, east, north = patch["bounds_m"]
        pixels, audit = bake_patch(patch)
        cols, rows = (east - west) // SPAN, (north - south) // SPAN
        patches.append(PATCH.pack(west, south, cols, rows, len(tile_index)))
        active = 0
        for row in range(rows):
            for col in range(cols):
                tile = pixels[row * TILE:(row + 1) * TILE, col * TILE:(col + 1) * TILE].reshape(TILE * TILE)
                if not tile.any():
                    tile_index.append(0)
                    continue
                active += 1
                record = tile_record(tile)
                index = used.get(record)
                if index is None:
                    index = len(offsets)
                    used[record] = index
                    records.extend(record)
                    offsets.append(len(records))
                tile_index.append(index)
        audit.update(bounds_m=patch["bounds_m"], active_tiles=active)
        audits[patch["name"]] = audit
    if len(used) > 65_535:
        raise ValueError("Palette tile count exceeds the town format")
    header = HEADER.pack(b"STN1", len(patches), CELL, len(tile_index), len(used))
    raw = (header + b"".join(patches) + struct.pack(f"<{len(tile_index)}H", *tile_index)
           + struct.pack(f"<{len(offsets)}I", *offsets) + records)
    return raw, audits


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--survey", type=Path, help="A cached survey from --save-survey")
    source.add_argument("--pbf", nargs="+", type=Path, help="Local OSM PBF snapshots")
    parser.add_argument("--save-survey", type=Path)
    args = parser.parse_args()
    if args.survey:
        data = json.loads(args.survey.read_text())
    else:
        extractor = TownExtractor()
        for pbf in args.pbf:
            extractor.apply_file(str(pbf), locations=True, idx="flex_mem")
        data = extractor.survey()
    if args.save_survey:
        args.save_survey.write_text(json.dumps(data, separators=(",", ":")))
    raw, audits = pack(data)
    with args.output.open("wb") as output:
        with gzip.GzipFile(filename="", fileobj=output, mode="wb", mtime=0, compresslevel=9) as compressed:
            compressed.write(raw)
    print(json.dumps({"towns": audits, "raw_bytes": len(raw), "gzip_bytes": args.output.stat().st_size}, indent=2))


if __name__ == "__main__":
    main()
