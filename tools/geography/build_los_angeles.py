#!/usr/bin/env python3
"""Bake a metre-scale Downtown Los Angeles flight map from OSM and a USGS DEM."""

import argparse
from collections import Counter, defaultdict, deque
import gzip
import json
import math
from pathlib import Path
import struct
import tempfile

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

from build_las_vegas import WIDTHS, metres

PROJECTION = Transformer.from_crs(4326, 32611, always_xy=True)
UNPROJECT = Transformer.from_crs(32611, 4326, always_xy=True)
ORIGIN = tuple(round(value) for value in PROJECTION.transform(-118.2500, 34.0500))
MAP_ID = 1
CELL = 6
DEM_CELL = 60
TILE_SIDE = 16
TILE_METRES = CELL * TILE_SIDE
BATCH_TILES = 12
BATCH_METRES = TILE_METRES * BATCH_TILES
SHAPES = osmium.geom.WKBFactory()
RAILS = {"rail", "light_rail", "monorail", "tram", "subway"}
LANDMARK_AMENITIES = {"arts_centre", "college", "courthouse", "library", "marketplace",
                       "place_of_worship", "theatre", "townhall", "university"}
LANDMARK_TOURISM = {"attraction", "gallery", "hotel", "museum", "viewpoint", "zoo"}


def rounded_radius(radius, cell):
    return math.ceil(radius / cell) * cell


def local(lon, lat):
    east, north = PROJECTION.transform(lon, lat)
    return east - ORIGIN[0], north - ORIGIN[1]


def footprint_color(tags):
    facade = tags.get("building:material", "")
    if facade in {"glass", "mirror", "steel", "metal"}:
        return 24
    if facade in {"brick", "clay", "terracotta"}:
        return 26
    if facade in {"limestone", "sandstone", "stone", "masonry"}:
        return 21
    if facade in {"adobe", "stucco", "plaster"}:
        return 29
    return 30


class Features(osmium.SimpleHandler):
    def __init__(self, radius, dem_radius):
        super().__init__()
        self.radius = radius
        self.bounds = box(-radius, -radius, radius, radius)
        self.dem_bounds = box(-dem_radius, -dem_radius, dem_radius, dem_radius)
        self.near = self.lon_lat_bounds(radius)
        self.near_dem = self.lon_lat_bounds(dem_radius)
        self.blocks = defaultdict(list)
        self.waters = []
        self.places = {}
        self.streets = {}
        self.count = Counter()

    @staticmethod
    def lon_lat_bounds(radius):
        corners = [UNPROJECT.transform(ORIGIN[0] + x, ORIGIN[1] + z)
                   for x in (-radius, radius) for z in (-radius, radius)]
        lons, lats = zip(*corners)
        return min(lons) - 0.015, min(lats) - 0.015, max(lons) + 0.015, max(lats) + 0.015

    @staticmethod
    def possible(lon, lat, bounds):
        west, south, east, north = bounds
        return west <= lon <= east and south <= lat <= north

    def add(self, shape, layer, word, kind):
        if shape.is_empty:
            return
        if kind == "water" and self.dem_bounds.intersects(shape):
            self.waters.append(shape)
        if not self.bounds.intersects(shape):
            return
        self.count[kind] += 1
        left, bottom, right, top = self.bounds.intersection(shape.envelope).bounds
        for row in range(math.floor((bottom + self.radius) / BATCH_METRES),
                         math.floor((top + self.radius) / BATCH_METRES) + 1):
            for col in range(math.floor((left + self.radius) / BATCH_METRES),
                             math.floor((right + self.radius) / BATCH_METRES) + 1):
                self.blocks[row, col].append((layer, shape, word))

    def named(self, tags, x, z, element, osm_id, height=None, height_source=None):
        name = tags.get("name")
        if not name or not self.bounds.covers(Point(x, z)):
            return
        if tags.get("railway") in {"station", "halt"} or tags.get("public_transport") == "station":
            kind = "station"
        elif tags.get("leisure") in {"park", "stadium", "garden", "nature_reserve"}:
            kind = tags["leisure"]
        elif tags.get("tourism") in LANDMARK_TOURISM:
            kind = tags["tourism"]
        elif tags.get("amenity") in LANDMARK_AMENITIES:
            kind = tags["amenity"]
        elif tags.get("aeroway") == "aerodrome":
            kind = "aerodrome"
        elif tags.get("building") or tags.get("building:part"):
            kind = "building"
        else:
            return
        entry = {"name": name, "kind": kind, "x": round(x), "z": round(z), "osm_type": element, "osm_id": osm_id}
        if height is not None:
            entry.update(height_m=height, height_source=height_source)
        key = (name.casefold(), round(x / 28), round(z / 28))
        previous = self.places.get(key)
        if not previous or "height_m" in entry and "height_m" not in previous:
            self.places[key] = entry

    def node(self, node):
        # Most PBF nodes are untagged vertices. Avoid projection for them.
        if "name" in node.tags and self.possible(node.location.lon, node.location.lat, self.near):
            self.named(node.tags, *local(node.location.lon, node.location.lat), "node", node.id)

    def way(self, way):
        tags = way.tags
        highway, rail, water = tags.get("highway"), tags.get("railway"), tags.get("waterway")
        air = tags.get("aeroway")
        if highway not in WIDTHS and rail not in RAILS and water not in {"river", "canal"} and air not in {"runway", "taxiway"}:
            return
        if len(way.nodes) < 2 or tags.get("tunnel") not in (None, "no") or tags.get("location") == "underground":
            return
        window = self.near_dem if water in {"river", "canal"} else self.near
        nodes = way.nodes
        if not any(n.location.valid() and self.possible(n.lon, n.lat, window)
                   for n in (nodes[0], nodes[-1], nodes[len(nodes) // 2])):
            return
        path = LineString([local(n.lon, n.lat) for n in nodes if n.location.valid()])
        if not path.length:
            return
        if water in {"river", "canal"}:
            width = metres(tags.get("width"), 6)
            self.add(path.buffer(width / 2, cap_style="flat"), 2, 2, "water")
        elif rail in RAILS:
            self.add(path.buffer(3, cap_style="flat"), 4, 8, "rail")
        elif air in {"runway", "taxiway"}:
            width = metres(tags.get("width"), 46 if air == "runway" else 18)
            self.add(path.buffer(width / 2, cap_style="flat"), 6 if air == "runway" else 5,
                     6 if air == "runway" else 3, air)
        else:
            width = metres(tags.get("width"), None)
            lanes = tags.get("lanes", "")
            if width is None:
                width = max(WIDTHS[highway], 3.25 * int(lanes)) if lanes.isdecimal() else WIDTHS[highway]
            self.add(path.buffer(min(width, 64) / 2, cap_style="flat"), 3, 1, "road")
            name = tags.get("name") or tags.get("ref")
            if name and self.bounds.intersects(path):
                center = self.bounds.intersection(path).representative_point()
                key = name.casefold()
                previous = self.streets.get(key)
                if not previous or previous["length"] < path.length:
                    self.streets[key] = {"name": name, "kind": highway, "osm_id": way.id,
                                         "x": round(center.x), "z": round(center.y), "length": round(path.length)}

    def area(self, area):
        tags = area.tags
        building, part = tags.get("building"), tags.get("building:part")
        water = tags.get("natural") == "water" or tags.get("waterway") == "riverbank"
        park = tags.get("leisure") in {"park", "garden", "golf_course", "recreation_ground"}
        apron = tags.get("aeroway") == "apron"
        if not (building or part or water or park or apron or "name" in tags):
            return
        outer = next(iter(area.outer_rings()), None)
        if outer is None or not outer or not self.possible(outer[0].lon, outer[0].lat, self.near_dem if water else self.near):
            return
        footprint = from_wkb(bytes.fromhex(SHAPES.create_multipolygon(area)))
        # Reproject source coordinates together for complex multipolygons.
        from shapely import transform
        footprint = transform(footprint, local, interleaved=False)
        if not footprint.is_valid:
            footprint = footprint.buffer(0)
        if footprint.is_empty:
            return
        point = footprint.representative_point()
        kind = "way" if area.from_way() else "relation"
        name_args = (tags, point.x, point.y, kind, area.orig_id())
        if water:
            self.add(footprint, 2, 2, "water")
        elif park:
            self.add(footprint, 1, 7, "park")
        elif apron:
            self.add(footprint, 4, 3, "apron")
        elif (building or part) and building not in {"no", "construction", "demolished"}:
            source_height = metres(tags.get("height"), None)
            levels = metres(tags.get("building:levels"), None)
            if source_height is not None:
                height, source = source_height, "OpenStreetMap height tag"
            elif levels is not None:
                height, source = levels * 3.2, "Estimated from OpenStreetMap floors"
            else:
                height = 5 if footprint.area < 260 else 8 if footprint.area < 2500 else 14
                source = "Estimated from mapped footprint"
            height = min(511, max(1, round(height)))
            material = 19 if tags.get("aeroway") == "terminal" or building == "terminal" else footprint_color(tags)
            self.add(footprint, 9, (height << 7) | material, "building")
            self.count[source] += int(self.bounds.intersects(footprint))
            self.named(*name_args, height, source)
            return
        self.named(*name_args)


def ocean_mask(elevations):
    """Use the connected zero-elevation southwest area in the coastal DEM."""
    ocean = np.zeros(elevations.shape, dtype=bool)
    if elevations[-1, 0] > 0:
        return ocean
    rows, cols = ocean.shape
    frontier = deque([(rows - 1, 0)])
    ocean[-1, 0] = True
    while frontier:
        row, col = frontier.popleft()
        for r, c in ((row - 1, col), (row + 1, col), (row, col - 1), (row, col + 1)):
            if 0 <= r < rows and 0 <= c < cols and not ocean[r, c] and elevations[r, c] <= 0:
                ocean[r, c] = True
                frontier.append((r, c))
    return ocean


def terrain(dem_path, waters, radius):
    side = (radius * 2) // DEM_CELL
    local_transform = Affine.translation(-radius, radius) * Affine.scale(DEM_CELL, -DEM_CELL)
    target = Affine.translation(*ORIGIN) * local_transform
    elevations = np.full((side, side), np.nan, dtype=np.float32)
    with rasterio.open(dem_path) as dataset:
        reproject(rasterio.band(dataset, 1), elevations, src_transform=dataset.transform,
                  src_crs=dataset.crs, src_nodata=dataset.nodata, dst_transform=target,
                  dst_crs="EPSG:32611", dst_nodata=np.nan, resampling=Resampling.bilinear)
    missing = int(np.isnan(elevations).sum())
    if missing:
        raise ValueError(f"DEM must cover the requested region ({missing:,} missing cells)")
    water = ocean_mask(elevations).astype("uint8")
    if waters:
        water |= rasterize(((shape, 1) for shape in waters), out_shape=elevations.shape,
                           transform=local_transform, dtype="uint8")
    return elevations.clip(0, 65535).round().astype("<u2"), water


def bake(features, dem_path, dem_radius, output):
    elevation, water = terrain(dem_path, features.waters, dem_radius)
    radius = features.radius
    side = (2 * radius) // TILE_METRES
    index = np.zeros((side, side), dtype="<u4")
    count = 0
    with tempfile.TemporaryFile(dir=output) as tile_stream:
        for (row, col), shapes in sorted(features.blocks.items()):
            row *= BATCH_TILES
            col *= BATCH_TILES
            rows, cols = min(BATCH_TILES, side - row), min(BATCH_TILES, side - col)
            if rows <= 0 or cols <= 0:
                continue
            top = -radius + (row + rows) * TILE_METRES
            left = -radius + col * TILE_METRES
            tx = Affine.translation(left, top) * Affine.scale(CELL, -CELL)
            ordered = sorted(shapes, key=lambda feature: (feature[0], feature[2] >> 7))
            patch = rasterize(((shape, word) for _, shape, word in ordered),
                              out_shape=(rows * TILE_SIDE, cols * TILE_SIDE), transform=tx, dtype="uint16")[::-1]
            for block_z in range(rows):
                for block_x in range(cols):
                    z, x = block_z * TILE_SIDE, block_x * TILE_SIDE
                    tile = patch[z:z + TILE_SIDE, x:x + TILE_SIDE]
                    if tile.any():
                        count += 1
                        index[row + block_z, col + block_x] = count
                        tile_stream.write(tile.astype("<u2", copy=False).tobytes())
        if not count:
            raise ValueError("No mapped features overlapped the requested Los Angeles region")
        header = struct.pack("<4s15i", b"LVS1", -dem_radius, dem_radius, DEM_CELL,
                             elevation.shape[1], elevation.shape[0], -radius, -radius, CELL,
                             side, side, count, *ORIGIN, 0, MAP_ID)
        raw_path = output / "los-angeles.bin"
        with raw_path.open("wb") as raw:
            raw.write(header)
            raw.write(elevation.tobytes())
            raw.write(water.tobytes())
            raw.write(index.tobytes())
            tile_stream.seek(0)
            while chunk := tile_stream.read(1024 * 1024):
                raw.write(chunk)
        with raw_path.open("rb") as raw, (output / "los-angeles.bin.gz").open("wb") as target:
            with gzip.GzipFile(fileobj=target, mode="wb", compresslevel=6, mtime=0) as compressed:
                while chunk := raw.read(1024 * 1024):
                    compressed.write(chunk)

    manifest = {
        "name": "Los Angeles", "map_id": MAP_ID,
        "coordinates": "UTM 11N. One game unit is one metre on all axes.",
        "origin_utm_m": ORIGIN, "detail_bounds_m": [-radius, -radius, radius, radius],
        "dem_bounds_m": [-dem_radius, -dem_radius, dem_radius, dem_radius],
        "dem_cell_m": DEM_CELL, "detail_cell_m": CELL,
        "tiles": count, "coverage": dict(features.count),
        "sources": {"footprints_and_network": "OpenStreetMap via Geofabrik", "elevation": "USGS 3DEP"},
        "places": sorted(features.places.values(), key=lambda p: (p["name"].casefold(), p["x"], p["z"])),
        "streets": sorted(features.streets.values(), key=lambda p: p["name"].casefold()),
    }
    (output / "los-angeles.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"{count:,} terrain tiles, {len(features.places):,} named places, {len(features.streets):,} named streets")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("osm", type=Path)
    parser.add_argument("dem", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--radius-m", type=int, default=6000, help="Detailed half-width, rounded up to whole tiles")
    parser.add_argument("--dem-radius-m", type=int, default=43200, help="Outer elevation half-width")
    args = parser.parse_args()
    if not 0 < args.radius_m <= args.dem_radius_m:
        parser.error("Choose a positive detail radius inside the elevation coverage")
    detail_radius = rounded_radius(args.radius_m, TILE_METRES)
    dem_radius = rounded_radius(args.dem_radius_m, DEM_CELL)
    args.output.mkdir(parents=True, exist_ok=True)
    features = Features(detail_radius, dem_radius)
    features.apply_file(str(args.osm), locations=True, idx="flex_mem")
    print("Indexed Los Angeles geometry", dict(features.count), flush=True)
    bake(features, args.dem, dem_radius, args.output)


if __name__ == "__main__":
    main()
