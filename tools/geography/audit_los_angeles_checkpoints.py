#!/usr/bin/env python3
"""Compare the MiniWorld reference locations with the packed Los Angeles map."""

import argparse
from collections import Counter
import json
import math
from pathlib import Path
import re

import numpy as np
from pyproj import Transformer

from render_vegas_venues import Pack

ALIASES = {
    "Grand Central Market": {"Homer Laughlin Building"},
    "LAX": {"Los Angeles International Airport"},
    "LA Memorial Coliseum": {"Los Angeles Memorial Coliseum"},
    "Little Tokyo": {"Japanese Village Plaza"},
    "Natural History Museum": {"Natural History Museum of Los Angeles County"},
    "Rose Bowl": {"Rose Bowl Stadium"},
    "The Broad": {"The Broad Museum"},
    "Union Station": {"Los Angeles Union Station"},
    "USC": {"University of Southern California"},
}
AREA_TYPES = {
    "Arts District": "district",
    "Echo Park Lake": "lake",
    "Exposition Park Rose Garden": "park",
    "Little Tokyo": "district",
    "MacArthur Park": "park",
    "Olvera Street": "street",
    "USC": "campus",
}

# The published reference resolves to a Comfort Inn on Sunset Boulevard.
# Use a sample inside the lake identified by the city, retaining the original
# reference position in the result so the correction is reproducible.
ECHO_PARK = {
    "lon_lat": [-118.2610, 34.0735],
    "reference": "https://www.laparks.org/sites/default/files/pdf/commissioner/2025/aug07/25-139.pdf",
    "note": "Lake sample west of the city boathouse. The reference label points to the Comfort Inn on Sunset Boulevard.",
}


def normalized(name):
    return re.sub(r"[^a-z0-9]", "", name.casefold())


def inside(x, z, bounds):
    west, south, east, north = bounds
    return west <= x < east and south <= z < north


def sample(pack, x, z, radius):
    material, height, _, _ = pack.window(x, z, radius)
    center = (material.shape[0] // 2, material.shape[1] // 2)
    return {
        "center_material": int(material[center]),
        "center_height_m": int(height[center]),
        "building_cells": int(np.count_nonzero((material >= 10) & (height > 0))),
        "water_cells": int(np.count_nonzero(material == 2)),
        "park_cells": int(np.count_nonzero(material == 7)),
        "road_cells": int(np.count_nonzero(material == 1)),
        "rail_cells": int(np.count_nonzero(material == 8)),
    }


def nearest_named(checkpoint, x, z, candidates):
    names = {normalized(name) for name in (checkpoint, *ALIASES.get(checkpoint, ()))}
    matches = [candidate for candidate in candidates if normalized(candidate["name"]) in names]
    if not matches:
        return None
    closest = min(matches, key=lambda item: math.hypot(item["x"] - x, item["z"] - z))
    distance = math.hypot(closest["x"] - x, closest["z"] - z)
    if distance > 430:
        return None
    fields = ("name", "x", "z", "kind", "osm_type", "osm_id", "height_m", "height_source")
    result = {key: closest[key] for key in fields if key in closest}
    return dict(result, distance_from_reference_m=round(distance))


def audit(checkpoints, manifest, pack):
    transformer = Transformer.from_crs(4326, 32611, always_xy=True)
    ox, oz = manifest["origin_utm_m"]
    detail, terrain = manifest["detail_bounds_m"], manifest["dem_bounds_m"]
    entries = []
    for point in checkpoints["places"]:
        name = point["name"]
        location = [point["lon"], point["lat"]]
        row = {"name": name, "reference_lon_lat": location}
        if name == "Echo Park Lake":
            location = ECHO_PARK["lon_lat"]
            row["location_correction"] = ECHO_PARK
        east, north = transformer.transform(*location)
        x, z = round(east - ox), round(north - oz)
        row.update(x=x, z=z)
        if not inside(x, z, terrain):
            row.update(status="outside-terrain")
        elif not inside(x, z, detail):
            row.update(status="terrain-only")
        else:
            cells = sample(pack, x, z, 30)
            row["sample_within_30m"] = cells
            kind = AREA_TYPES.get(name)
            candidates = manifest["streets"] if kind == "street" else manifest["places"]
            matched = nearest_named(name, x, z, candidates)
            if matched:
                row["named_feature"] = matched
                named_cells = sample(pack, matched["x"], matched["z"], 18)
                if kind == "street" and named_cells["road_cells"]:
                    row.update(status="mapped-street")
                elif "height_m" in matched and named_cells["building_cells"]:
                    row.update(status="mapped-building")
                elif kind == "park" and (named_cells["park_cells"] or named_cells["water_cells"]):
                    row.update(status="mapped-park")
                else:
                    row.update(status="mapped-context")
            elif kind == "lake" and cells["center_material"] == 2:
                row.update(status="mapped-water")
            elif kind == "park" and cells["park_cells"]:
                row.update(status="mapped-park-area")
            elif any(cells[key] for key in ("building_cells", "water_cells", "park_cells", "road_cells", "rail_cells")):
                row.update(status="mapped-context")
            else:
                row.update(status="unverified-in-detail-area")
        entries.append(row)
    return {
        "schema": 1, "map": "Los Angeles", "packed_cell_m": pack.cell,
        "source": "MiniWorld's 46 public checkpoints, independently projected from WGS84",
        "comparison": "Packed representations and names. Height-based meshes may use estimates identified in the geography manifest.",
        "count": len(entries), "coverage": dict(sorted(Counter(entry["status"] for entry in entries).items())),
        "detail_bounds_m": detail, "dem_bounds_m": terrain, "entries": entries,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkpoints", type=Path)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("pack", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    report = audit(json.loads(args.checkpoints.read_text()), json.loads(args.manifest.read_text()), Pack(args.pack))
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(report["count"], report["coverage"])


if __name__ == "__main__":
    main()
