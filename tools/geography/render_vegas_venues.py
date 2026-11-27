#!/usr/bin/env python3
"""Export isometric PNGs from Scenic Flight's Las Vegas terrain pack.

Requires NumPy and Pillow. Run from the checkout with no arguments to refresh
the published venue catalogue. Optional runtime model artwork is imported from
docs/assets/vegas/landmarks/index.json.
"""

import argparse
from collections import defaultdict, deque
import gzip
import hashlib
import json
import math
import os
from pathlib import Path
import re
import struct
import unicodedata

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "docs/assets"
PACK = ASSETS / "geography/las-vegas.bin.gz"
MANIFEST = ASSETS / "geography/las-vegas.json"
OUTPUT = ASSETS / "vegas/venues"
MODELS = ASSETS / "vegas/landmarks/index.json"

BACKGROUND = (20, 26, 32)
SIZE = (128, 104)
HEADER = struct.Struct("<4s15i")
TILE_SIDE = 16
SOURCE = "Scenic Flight Las Vegas terrain pack"
STANDALONE_LANDMARKS = {
    "allegiant stadium", "fremont street experience", "high roller", "hoover dam",
    "sphere", "t-mobile arena", "harry reid international airport", "north las vegas airport",
}
AIRFIELD_RADIUS = {
    "harry reid international airport": 2600,
    "north las vegas airport": 1500,
}
COMPONENT_BASE_M = {
    114703388: 120,  # Bellagio domed crown, above the mapped main tower.
}


class Pack:
    def __init__(self, filename):
        raw = filename.read_bytes()
        self.digest = hashlib.sha256(raw).hexdigest()
        if raw.startswith(b"\x1f\x8b"):
            raw = gzip.decompress(raw)
        (magic, _, _, _, ground_width, ground_height, self.west, self.south, self.cell,
         columns, rows, count, *_) = HEADER.unpack_from(raw)
        if magic != b"LVS1" or min(ground_width, ground_height, self.cell, columns, rows, count) <= 0:
            raise ValueError("Expected the LVS1 reference terrain pack")
        index_at = HEADER.size + 3 * ground_width * ground_height
        tile_at = index_at + 4 * columns * rows
        if len(raw) != tile_at + count * TILE_SIDE**2 * 2:
            raise ValueError("Terrain pack length disagrees with its header")
        self.index = np.frombuffer(raw, "<u4", columns * rows, index_at).reshape(rows, columns)
        self.tiles = np.frombuffer(raw, "<u2", count * TILE_SIDE**2, tile_at).reshape(count, TILE_SIDE**2)

    def window(self, x, z, radius):
        col = (x - self.west) // self.cell
        row = (z - self.south) // self.cell
        half = math.ceil(radius / self.cell)
        cols, rows = np.meshgrid(np.arange(col - half, col + half + 1), np.arange(row - half, row + half + 1))
        tx, tz = cols // TILE_SIDE, rows // TILE_SIDE
        valid = (0 <= tx) & (tx < self.index.shape[1]) & (0 <= tz) & (tz < self.index.shape[0])
        tile = self.index[np.clip(tz, 0, self.index.shape[0] - 1), np.clip(tx, 0, self.index.shape[1] - 1)]
        slot = (rows % TILE_SIDE) * TILE_SIDE + (cols % TILE_SIDE)
        words = np.where(valid & (tile > 0), self.tiles[np.maximum(tile, 1) - 1, slot], 0)
        left = self.west + (col - half) * self.cell
        bottom = self.south + (row - half) * self.cell
        return words & 127, words >> 7, left, bottom


def palette():
    """Read the shared game finishes without duplicating the palette in Python."""
    source = (ROOT / "src/settlement/building_palettes.rs").read_text()
    channels = r"\[(\d+), (\d+), (\d+)\]"
    standard = re.findall(rf"material\({channels}, {channels}\)", source)
    resort = re.findall(rf"resort\({channels}, {channels}, {channels}, ([0-9.]+)\)", source)
    if len(standard) != 50 or len(resort) != 10:
        raise ValueError("The game's palette format changed")
    colors = {
        0: ((137, 120, 96), (137, 120, 96), None, None),
        1: ((80, 87, 92), (80, 87, 92), None, None),
        2: ((65, 128, 161), (65, 128, 161), None, None),
        3: ((163, 149, 118), (163, 149, 118), None, None),
        5: ((209, 206, 190), (209, 206, 190), None, None),
        6: ((58, 65, 71), (58, 65, 71), None, None),
        7: ((87, 135, 70), (87, 135, 70), None, None),
        8: ((119, 104, 86), (119, 104, 86), None, None),
        10: ((43, 110, 168), (70, 164, 204), None, None),
        11: ((173, 144, 113), (171, 85, 61), None, None),
        18: ((119, 142, 161), (157, 175, 188), None, None),
        19: ((113, 130, 129), (117, 145, 149), None, None),
        20: ((49, 58, 73), (63, 74, 93), None, None),
    }
    for material, entry in zip(range(12, 18), (6, 9, 11, 20, 25, 35)):
        red, green, blue, roof_r, roof_g, roof_b = map(int, standard[entry - 1])
        colors[material] = ((red, green, blue), (roof_r, roof_g, roof_b), None, None)
    for material, entry in enumerate(resort, 21):
        wall = tuple(map(int, entry[0:3]))
        roof = tuple(map(int, entry[3:6]))
        glass = tuple(map(int, entry[6:9]))
        colors[material] = (wall, roof, glass, float(entry[9]))
    return colors


def nearest(eligible, x, z, left, bottom, cell, radius):
    rows, cols = np.nonzero(eligible)
    if not len(rows):
        return None
    distance2 = (left + (cols + 0.5) * cell - x)**2 + (bottom + (rows + 0.5) * cell - z)**2
    index = int(distance2.argmin())
    if distance2[index] > radius**2:
        return None
    return int(rows[index]), int(cols[index]), round(math.sqrt(distance2[index]), 1)


def connected(mask, row, col):
    region = np.zeros(mask.shape, dtype=bool)
    region[row, col] = True
    frontier = deque([(row, col)])
    rows, cols = mask.shape
    while frontier:
        r, c = frontier.popleft()
        for nr, nc in ((r + 1, c), (r - 1, c), (r, c + 1), (r, c - 1)):
            if 0 <= nr < rows and 0 <= nc < cols and mask[nr, nc] and not region[nr, nc]:
                region[nr, nc] = True
                frontier.append((nr, nc))
    return region


def venue_cells(pack, entry, reviewed):
    name = entry["name"].casefold()
    x, z = entry["x"], entry["z"]
    airfield = AIRFIELD_RADIUS.get(name)
    if airfield:
        materials, heights, _, _ = pack.window(x, z, airfield)
        chosen = np.isin(materials, [3, 5, 6, 19])
        return materials, heights, chosen, "Packed runways and terminals", None

    radius = 146 if reviewed else 202
    materials, heights, left, bottom = pack.window(x, z, radius)
    buildings = (materials >= 10) & (heights > 0)
    if reviewed:
        # Each component records its own material and source height. Matching both
        # lets the pack distinguish towers even when they share a podium.
        height = entry["height_m"]
        rise = 18 if entry.get("roof_profile") else 1
        eligible = buildings & (materials == entry["material"]) & (heights <= height + 1)
        eligible &= heights >= max(1, height - rise)
    else:
        eligible = buildings
    seed = nearest(eligible, x, z, left, bottom, pack.cell, 65 if reviewed else 48)
    if seed:
        row, col, offset = seed
        chosen = connected(eligible, row, col)
        label = "Reviewed packed component" if reviewed else "Generic packed footprint"
        if not reviewed and offset > 1.5 * pack.cell:
            label = "Nearby packed footprint"
        base = COMPONENT_BASE_M.get(entry.get("osm_id"), 0)
        if base:
            heights = np.maximum(0, heights.astype(np.int16) - base)
        return materials, heights, chosen, label, offset

    # Labels sometimes point to a forecourt, a separate runtime model, or an
    # interior venue. Display real map context when no silhouette can be isolated.
    materials, heights, _, _ = pack.window(x, z, 24)
    chosen = np.ones(materials.shape, dtype=bool)
    return materials, heights, chosen, "Packed map location", None


def shade(color, factor):
    return tuple(min(255, round(channel * factor)) for channel in color)


def iso_image(materials, heights, chosen, cell, colors):
    occupied = np.argwhere(chosen)
    if not len(occupied):
        return Image.new("RGB", SIZE, BACKGROUND)
    rows, cols = occupied.T
    highest = heights[rows, cols].astype(float) / cell
    min_u, max_u = float((cols - rows).min() - 1), float((cols - rows).max() + 1)
    min_v = float((0.43 * (cols + rows) - 0.97 * highest).min())
    max_v = float((0.43 * (cols + rows + 2)).max())
    scale = min((SIZE[0] - 18) / (max_u - min_u), (SIZE[1] - 18) / (max_v - min_v))
    center_u, center_v = (min_u + max_u) / 2, (min_v + max_v) / 2
    sample = 2
    image = Image.new("RGB", tuple(unit * sample for unit in SIZE), BACKGROUND)
    canvas = ImageDraw.Draw(image)

    def screen(x, z, height):
        u = x - z
        v = 0.43 * (x + z) - 0.97 * height / cell
        return (round(sample * (SIZE[0] / 2 + (u - center_u) * scale)),
                round(sample * (SIZE[1] / 2 + (v - center_v) * scale)))

    def top(x, z, height):
        return [screen(x, z, height), screen(x + 1, z, height),
                screen(x + 1, z + 1, height), screen(x, z + 1, height)]

    def side(first, second, high, low, wall, glass, floor, light):
        def face(upper, lower):
            return [screen(*first, upper), screen(*second, upper),
                    screen(*second, lower), screen(*first, lower)]

        canvas.polygon(face(high, low), fill=shade(wall, light))
        if floor and high >= 20 and scale * (floor / cell) >= 0.65:
            visible_floor = max(floor, cell * 1.35 / scale)
            band = math.floor((high - 2) / visible_floor) * visible_floor
            while band > low:
                canvas.polygon(face(band, max(low, band - visible_floor * 0.38)),
                               fill=shade(glass, light))
                band -= visible_floor

    for z, x in occupied.tolist():
        if heights[z, x]:
            canvas.polygon([(u + 2, v + 3) for u, v in top(x, z, 0)], fill=(12, 17, 24))

    # Paint the exposed south/east faces of each occupied height-field cell in
    # depth order. The packed width, depth and height share the same metre scale.
    for z, x in sorted(occupied.tolist(), key=lambda tile: (sum(tile), tile[0])):
        height = int(heights[z, x])
        wall, roof, glass, floor = colors.get(int(materials[z, x]), colors[0])
        east = int(heights[z, x + 1]) if x + 1 < chosen.shape[1] and chosen[z, x + 1] else 0
        south = int(heights[z + 1, x]) if z + 1 < chosen.shape[0] and chosen[z + 1, x] else 0
        if height > east:
            side((x + 1, z), (x + 1, z + 1), height, east, wall, glass, floor, 0.76)
        if height > south:
            side((x, z + 1), (x + 1, z + 1), height, south, wall, glass, floor, 0.93)
        canvas.polygon(top(x, z, height), fill=shade(roof, 1.08))
    return image.resize(SIZE, Image.Resampling.LANCZOS)


def slug(text):
    ascii_text = unicodedata.normalize("NFKD", text.casefold()).encode("ascii", "ignore").decode()
    return re.sub(r"[^a-z0-9]+", "-", ascii_text).strip("-")


def model_entries(filename, output):
    if not filename.is_file():
        return []
    index = json.loads(filename.read_text())
    entries = index.get("entries")
    if entries is None:
        entries = [entry for group in index.get("groups", []) for entry in group["entries"]]
    results = []
    for entry in entries:
        image = entry.get("poster", entry.get("image"))
        if not image:
            continue
        path = filename.parent / image
        if not path.is_file():
            raise FileNotFoundError(path)
        result = dict(entry, image=Path(os.path.relpath(path, output)).as_posix(),
                      representation=entry.get("representation", "Runtime model"))
        if entry.get("animation"):
            animation = filename.parent / entry["animation"]
            if animation.is_file():
                result["animation"] = Path(os.path.relpath(animation, output)).as_posix()
        results.append(result)
    return results


def export(pack_path, manifest_path, output, sidecar):
    pack = Pack(pack_path)
    data = json.loads(manifest_path.read_text())
    models = {entry.get("venue", entry["name"]).casefold(): entry for entry in model_entries(sidecar, output)}
    colors = palette()
    output.mkdir(parents=True, exist_ok=True)
    index_path = output / "index.json"
    old_index = json.loads(index_path.read_text()) if index_path.is_file() else {"groups": []}
    old_images = {entry["image"] for group in old_index["groups"] for entry in group["entries"]
                  if Path(entry["image"]).name == entry["image"] and entry["image"].endswith(".png")}
    groups = {
        "landmarks": ("Landmarks and airfields", []),
        "casinos": ("Casinos", []),
        "hotels": ("Hotels and other venues", []),
    }
    used_ids = set()
    rendered = {}

    def append(entry, group, kind, reviewed=False):
        name = entry["name"]
        ident = slug(name)
        if reviewed:
            ident = f"{ident}-{entry['osm_type']}-{entry['osm_id']}"
        if ident in used_ids:
            ident = f"{ident}-{entry['x']}-{entry['z']}"
        if ident in used_ids:
            raise ValueError(f"Duplicate catalogue entry: {name}")
        used_ids.add(ident)
        result = {
            "id": ident, "name": name, "kind": kind,
            "x": entry["x"], "z": entry["z"], "source": SOURCE,
        }
        runtime = models.pop(name.casefold(), None) if not reviewed else None
        if runtime:
            result.update({key: runtime[key] for key in ("image", "animation", "representation", "source") if key in runtime})
        else:
            material, heights, chosen, label, offset = venue_cells(pack, entry, reviewed)
            image = iso_image(material, heights, chosen, pack.cell, colors)
            digest = hashlib.sha256(image.tobytes()).digest()
            if digest not in rendered:
                rendered[digest] = f"{ident}.png"
                image.save(output / rendered[digest], optimize=True)
            result.update(image=rendered[digest], representation=label)
            if offset is not None:
                result["anchor_offset_m"] = offset
        if reviewed:
            for key in ("property", "osm_type", "osm_id", "height_m", "height_source", "material", "roof_profile"):
                if key in entry:
                    result[key] = entry[key]
            result["references"] = data.get("reviewed_building_sources", {}).get(entry["property"], [])
            if entry["osm_id"] in COMPONENT_BASE_M:
                result["component_base_m"] = COMPONENT_BASE_M[entry["osm_id"]]
        groups[group][1].append(result)

    for place in data["places"]:
        if place["kind"] == "casino":
            group, kind = "casinos", "casino"
        elif place["name"].casefold() in STANDALONE_LANDMARKS:
            group, kind = "landmarks", "landmark"
        else:
            group, kind = "hotels", "hotel-or-venue"
        append(place, group, kind)
    for entry in sorted(models.values(), key=lambda model: model["name"].casefold()):
        append(entry, "landmarks", "landmark")
    for component in data.get("reviewed_buildings", []):
        group = f"components-{slug(component['property'])}"
        groups.setdefault(group, (component["property"], []))
        append(component, group, "reviewed-component", reviewed=True)

    index = {
        "schema": 1, "background": "#%02x%02x%02x" % BACKGROUND, "packed_cell_m": pack.cell,
        "source": SOURCE, "pack_sha256": pack.digest, "icon_size": list(SIZE),
        "groups": [{"key": key, "title": title, "entries": entries}
                   for key, (title, entries) in groups.items() if entries],
    }
    index_path.write_text(json.dumps(index, ensure_ascii=False, indent=2) + "\n")
    for stale in sorted(old_images.difference(rendered.values())):
        (output / stale).unlink(missing_ok=True)
    print(f"{sum(len(group['entries']) for group in index['groups'])} labelled entries, {len(rendered)} generated PNGs")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", type=Path, default=PACK)
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    parser.add_argument("--runtime-models", type=Path, default=MODELS)
    args = parser.parse_args()
    export(args.pack, args.manifest, args.output, args.runtime_models)


if __name__ == "__main__":
    main()
