#!/usr/bin/env python3
"""Build regional LAS flight data from a Nevada OSM PBF and a USGS 3DEP GeoTIFF."""

import argparse
from collections import Counter, defaultdict
import csv
import io
import gzip
import json
import math
from pathlib import Path
import re
import struct
from zipfile import ZipFile

import numpy as np
import osmium
from pyproj import Transformer
import rasterio
from rasterio.enums import Resampling
from rasterio.features import rasterize
from rasterio.transform import Affine
from rasterio.warp import reproject
from shapely import from_wkb
from shapely.geometry import LineString, Point, box
from shapely.ops import transform

PROJECTION = Transformer.from_crs(4326, 32611, always_xy=True)
ORIGIN = tuple(round(value) for value in PROJECTION.transform(-115.1732, 36.1120))
CELL = 6
SIDE = 16
TILE = CELL * SIDE
BATCH = 12
BATCH_METRES = BATCH * TILE
BASE_METRES = 300
# Start with the Strip, downtown, airport terminals and western Henderson.
WEST, SOUTH, EAST, NORTH = (-10560, -11520, 13440, 13440)
DEM_WEST, DEM_SOUTH, DEM_EAST, DEM_NORTH = (-43_200, -16_800, 50_400, 33_600)
DEM_CELL = 60
GRID_WIDTH = (EAST - WEST) // TILE
GRID_HEIGHT = (NORTH - SOUTH) // TILE
SHAPES = osmium.geom.WKBFactory()
WIDE_BOUNDS = box(DEM_WEST, DEM_SOUTH, DEM_EAST, DEM_NORTH)
DETAIL_BOUNDS = box(WEST, SOUTH, EAST, NORTH)
AREA_BOXES = [(name, box(*bounds)) for name, bounds in {
    "city": (-4800, -3300, 4700, 10600),
    "south": (-6000, -8500, 7000, -2900),
    "east": (2000, -11000, 13440, 7500),
}.items()]
WIDTHS = dict(motorway=22, trunk=17, primary=15, secondary=12, tertiary=10,
              residential=8, unclassified=8, service=5, living_street=6,
              motorway_link=8, trunk_link=8, primary_link=7, secondary_link=6,
              tertiary_link=5, pedestrian=7, cycleway=3, footway=3, path=2.5)
HEIGHT = re.compile(r"^\s*(\d+(?:\.\d+)?)\s*(m|ft|')?")


def metres(value, default):
    match = HEIGHT.match(value or "")
    if not match:
        return default
    return float(match[1]) * (0.3048 if match[2] in ("ft", "'") else 1)


def local(lon, lat):
    x, z = PROJECTION.transform(lon, lat)
    return np.asarray(x) - ORIGIN[0], np.asarray(z) - ORIGIN[1]


def projected(shape):
    return transform(local, shape)


def near_vegas(lon, lat):
    return -115.72 <= lon <= -114.59 and 35.94 <= lat <= 36.46


class Features(osmium.SimpleHandler):
    def __init__(self, official_runways):
        super().__init__()
        self.blocks = defaultdict(list)
        self.waters = []
        self.points = {}
        self.count = Counter()
        self.sectors = defaultdict(Counter)
        self.runways = {}
        self.official_runways = official_runways
        self.sphere = None

    def add(self, shape, layer, value, kind):
        if shape.is_empty:
            return
        if kind == "water" and WIDE_BOUNDS.intersects(shape):
            self.waters.append(shape)
        if not DETAIL_BOUNDS.intersects(shape):
            return
        self.count[kind] += 1
        for name, sector in AREA_BOXES:
            if sector.intersects(shape):
                self.sectors[name][kind] += 1
        left, bottom, right, top = DETAIL_BOUNDS.intersection(shape.envelope).bounds
        for row in range(math.floor((bottom - SOUTH) / BATCH_METRES), math.floor((top - SOUTH) / BATCH_METRES) + 1):
            for col in range(math.floor((left - WEST) / BATCH_METRES), math.floor((right - WEST) / BATCH_METRES) + 1):
                self.blocks[row, col].append((layer, shape, value))

    def named(self, tags, x, z):
        name = tags.get("name")
        if not name:
            return
        casino = (tags.get("amenity") == "casino" or tags.get("leisure") == "casino"
                  or "casino" in name.lower() and tags.get("tourism") == "hotel")
        if casino or tags.get("tourism") == "hotel" or tags.get("aeroway") == "aerodrome" or name in {
            "Sphere", "Allegiant Stadium", "T-Mobile Arena", "High Roller", "Hoover Dam",
            "Fremont Street Experience", "Harry Reid International Airport",
        }:
            kind = "casino" if casino else "landmark"
            key = name.casefold()
            if DETAIL_BOUNDS.covers(Point(x,z)) and (key not in self.points or casino):
                self.points[key] = {"name": name, "kind": kind, "x": round(x), "z": round(z)}

    def node(self, node):
        if "name" in node.tags and near_vegas(node.location.lon, node.location.lat):
            self.named(node.tags, *local(node.location.lon, node.location.lat))

    def way(self, way):
        highway = way.tags.get("highway")
        air = way.tags.get("aeroway")
        rail = way.tags.get("railway")
        if highway not in WIDTHS and air not in ("runway", "taxiway") and rail not in ("rail", "monorail", "light_rail"):
            return
        nodes = way.nodes
        if len(nodes) < 2 or not any(p.location.valid() and near_vegas(p.lon, p.lat) for p in (nodes[0], nodes[-1], nodes[len(nodes)//2])):
            return
        coords = [local(p.lon, p.lat) for p in nodes if p.location.valid()]
        if len(coords) < 2:
            return
        line = LineString(coords)
        if air == "runway":
            ref = way.tags.get("ref", str(way.id))
            if ref in self.official_runways:
                return
            width = metres(way.tags.get("width"), 46)
            if DETAIL_BOUNDS.intersects(line):
                self.runways[ref] = {"source": "OpenStreetMap", "length_m": round(line.length), "width_m": round(width),
                                      "ends": [[round(a), round(b)] for a,b in (coords[0],coords[-1])]}
            self.add(line.buffer(width / 2, cap_style="flat"), 6, 6, "runway")
            self.add(line.buffer(1.8, cap_style="flat"), 7, 5, "runway marking")
        elif air == "taxiway":
            width = metres(way.tags.get("width"), 17)
            self.add(line.buffer(width / 2, cap_style="flat"), 5, 3, "taxiway")
        elif rail in ("rail", "monorail", "light_rail"):
            lift = 8 if rail == "monorail" or way.tags.get("bridge") else 0
            self.add(line.buffer(3, cap_style="flat"), 4, (lift << 7) | 8, "rail")
        else:
            if way.tags.get("tunnel") not in (None, "no") or str(way.tags.get("layer", "")).startswith("-"):
                return
            width = metres(way.tags.get("width"), WIDTHS[highway])
            lanes = str(way.tags.get("lanes", "")).split(";")[0]
            if lanes.isdigit() and int(lanes) > 1:
                width = max(width, int(lanes)*3.25)
            self.add(line.buffer(min(width, 36) / 2, cap_style="flat"), 3, 1, "road")

    def area(self, area):
        tags = area.tags
        building = tags.get("building")
        part = tags.get("building:part")
        water = tags.get("natural") == "water" or tags.get("waterway") == "riverbank"
        apr = tags.get("aeroway") == "apron"
        park = tags.get("leisure") in ("park", "golf_course", "recreation_ground")
        if not (building or part or water or apr or park or tags.get("aeroway") == "aerodrome"
                or tags.get("amenity") == "casino"):
            return
        rings = list(area.outer_rings())
        if not rings or not rings[0] or not near_vegas(rings[0][0].lon, rings[0][0].lat):
            return
        shape = projected(from_wkb(bytes.fromhex(SHAPES.create_multipolygon(area))))
        if not shape.is_valid:
            shape = shape.buffer(0)
        if shape.is_empty:
            return
        point = shape.representative_point()
        self.named(tags, point.x, point.y)
        if water:
            self.add(shape, 2, 2, "water")
        elif park:
            self.add(shape, 1, 7, "park")
        elif apr:
            self.add(shape, 4, 3, "apron")
        elif (building or part) and building not in ("no", "construction", "demolished"):
            name = tags.get("name", "")
            default = 5 if shape.area < 260 else 8 if shape.area < 2500 else 14
            height = metres(tags.get("height"), metres(tags.get("building:levels"), default/3.2)*3.2)
            if tags.get("height") or tags.get("building:levels"):
                self.count["source building heights"] += 1
            if name.lower() in ("sphere", "msg sphere", "las vegas sphere") and shape.area > 6000:
                center = shape.centroid
                self.sphere = (round(center.x), round(center.y))
                # The exposed dome is georeferenced to its footprint and uses the published dimensions.
                root = Point(center.x, center.y)
                for layer, radius, metres_high in [(12, 78.64, 37), (13, 70, 65),
                                                   (14, 56, 85), (15, 36, 102), (16, 15, 112)]:
                    self.add(root.buffer(radius, quad_segs=32), layer, (metres_high << 7) | 10, "sphere shell")
                return
            if area.orig_id() == 135457469:
                # The renderer draws the shaft, observation pod, and mast over this base.
                height = 15
            material = 11 + (area.orig_id() % 8)
            if area.orig_id() == 27858544 and shape.area > 5500:
                bounds = shape.bounds
                half_width = min(bounds[2] - bounds[0], bounds[3] - bounds[1]) / 2
                for layer in range(12, 28):
                    level = (layer - 12) / 16
                    shell = shape.buffer(-half_width * level, join_style="mitre")
                    self.add(shell, layer, (round(107 * (level + 1/16)) << 7) | 20, "pyramid shell")
                return
            if tags.get("aeroway") == "terminal" or building == "terminal":
                material = 19
                self.count["terminal footprints"] += 1
            self.add(shape, 10 if part else 9, (min(511, max(1, round(height))) << 7) | material, "building")


def faa_runways(apt_zip):
    def rows(archive, file):
        with archive.open(file) as source:
            return [row for row in csv.DictReader(io.TextIOWrapper(source, encoding="utf-8-sig", errors="replace"))
                    if row["ARPT_ID"] == "LAS"]

    with ZipFile(apt_zip) as archive:
        dimensions = {row["RWY_ID"]: (int(row["RWY_LEN"]), int(row["RWY_WIDTH"]))
                      for row in rows(archive, "APT_RWY.csv")}
        ends = defaultdict(list)
        for row in rows(archive, "APT_RWY_END.csv"):
            ends[row["RWY_ID"]].append((row["RWY_END_ID"], float(row["LONG_DECIMAL"]), float(row["LAT_DECIMAL"])))
    assert len(dimensions) == 4 and all(len(value) == 2 for value in ends.values())
    return {ref: (length, width, ends[ref]) for ref, (length,width) in dimensions.items()}


def draw_faa_runways(features):
    for ref, (length, width, endpoints) in features.official_runways.items():
        coords = [local(lon,lat) for _,lon,lat in endpoints]
        line = LineString(coords)
        width_m = width * 0.3048
        features.add(line.buffer(width_m/2, cap_style="flat"), 6, 6, "FAA runway")
        for distance in range(66, int(line.length) - 90, 66):
            dash = LineString([line.interpolate(distance), line.interpolate(distance+32)])
            features.add(dash.buffer(1.8, cap_style="flat"), 7, 5, "runway marking")
        features.runways[ref] = {"source": "FAA NASR 2026-11-26", "length_ft": length, "width_ft": width,
                                 "length_m": round(length*0.3048), "width_m": round(width_m),
                                 "ends_utm_relative_m": [[round(x),round(z)] for x,z in coords],
                                 "ends_lon_lat": [[lon,lat] for _,lon,lat in endpoints]}


def make_terrain(dem, waters):
    width = (DEM_EAST - DEM_WEST) // DEM_CELL
    height = (DEM_NORTH - DEM_SOUTH) // DEM_CELL
    tx = Affine.translation(ORIGIN[0] + DEM_WEST, ORIGIN[1] + DEM_NORTH) * Affine.scale(DEM_CELL, -DEM_CELL)
    meters = np.full((height, width), np.nan, dtype=np.float32)
    with rasterio.open(dem) as dataset:
        reproject(rasterio.band(dataset, 1), meters, src_transform=dataset.transform,
                  src_crs=dataset.crs, src_nodata=dataset.nodata, dst_transform=tx,
                  dst_crs="EPSG:32611", dst_nodata=np.nan, resampling=Resampling.bilinear)
    missing = int(np.isnan(meters).sum())
    meters = np.nan_to_num(meters, nan=640.0).clip(0, 5000).astype("<u2")
    local_tx = Affine.translation(DEM_WEST, DEM_NORTH) * Affine.scale(DEM_CELL, -DEM_CELL)
    water = rasterize(((shape, 1) for shape in waters), out_shape=meters.shape, transform=local_tx, dtype="uint8")
    return meters, water, missing


def build(features, dem, out):
    meters, water, missing = make_terrain(dem, features.waters)
    index = np.zeros((GRID_HEIGHT, GRID_WIDTH), dtype="<u4")
    count = 0
    tiles = out / "tiles.raw"
    with tiles.open("wb") as target:
        for (r, c), shapes in sorted(features.blocks.items()):
            row, col = r * BATCH, c * BATCH
            tile_rows = min(BATCH, GRID_HEIGHT-row)
            tile_cols = min(BATCH, GRID_WIDTH-col)
            if tile_rows <= 0 or tile_cols <= 0:
                continue
            width, height = tile_cols * SIDE, tile_rows * SIDE
            tx = Affine.translation(WEST + col*TILE, SOUTH + (row+tile_rows)*TILE) * Affine.scale(CELL, -CELL)
            patch = rasterize(((shape, value) for _, shape, value in sorted(shapes, key=lambda item:item[0])),
                              out_shape=(height,width), transform=tx, dtype="uint16")[::-1]
            for y in range(tile_rows):
                for x in range(tile_cols):
                    tile = patch[y*SIDE:(y+1)*SIDE, x*SIDE:(x+1)*SIDE]
                    if tile.any():
                        count += 1
                        index[row+y,col+x] = count
                        target.write(tile.astype("<u2", copy=False).tobytes())
    header = struct.pack("<4s15i", b"LVS1", DEM_WEST, DEM_NORTH, DEM_CELL,
                         meters.shape[1], meters.shape[0], WEST, SOUTH, CELL,
                         GRID_WIDTH, GRID_HEIGHT, count, ORIGIN[0], ORIGIN[1], BASE_METRES, 0)
    with (out/"las-vegas.bin").open("wb") as output:
        output.write(header)
        output.write(meters.tobytes())
        output.write(water.tobytes())
        output.write(index.tobytes())
        with tiles.open("rb") as source:
            while chunk := source.read(1024 * 1024):
                output.write(chunk)
    with (out/"las-vegas.bin").open("rb") as source, gzip.open(out/"las-vegas.bin.gz", "wb", 6) as output:
        while chunk := source.read(1024 * 1024):
            output.write(chunk)
    manifest = {
        "coordinates": "UTM 11N. One game unit is one metre on all axes.",
        "origin_utm_m": ORIGIN, "detail_bounds_m": [WEST,SOUTH,EAST,NORTH],
        "dem_cell_m": DEM_CELL, "detail_cell_m": CELL, "missing_dem_cells": missing,
        "tiles": count, "coverage": dict(features.count), "sectors": dict(features.sectors),
        "runways": features.runways, "sphere": features.sphere,
        "places": sorted(features.points.values(), key=lambda item: item["name"]),
    }
    (out/"las-vegas.json").write_text(json.dumps(manifest, indent=2)+"\n")
    print({key: value for key,value in manifest.items() if key != "places"}, flush=True)
    assert count > 500 and features.count["building"] > 1000 and features.count["road"] > 1000
    assert features.sphere and features.runways


def main():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument("osm", type=Path)
    args.add_argument("dem", type=Path)
    args.add_argument("faa", type=Path, help="FAA APT CSV ZIP effective 2026-11-26")
    args.add_argument("output", type=Path)
    options = args.parse_args()
    options.output.mkdir(parents=True, exist_ok=True)
    features = Features(faa_runways(options.faa))
    features.apply_file(str(options.osm), locations=True, idx="flex_mem")
    print("Indexed OSM geometry", features.count, flush=True)
    draw_faa_runways(features)
    build(features, options.dem, options.output)


if __name__ == "__main__":
    main()
