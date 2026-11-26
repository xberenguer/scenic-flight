#!/usr/bin/env python3
"""Refresh the README gallery using the published gameplay media manifest."""

import argparse
import concurrent.futures
import html
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
PAGES = "https://xberenguer.github.io/scenic-flight/"
FEATURED = ('airfield-visitor-stands', 'sedge-strand-tarn', 'lantern-causeway-skyline', 'high-cruise', 'quay-customs-annex')
AIRCRAFT = ("Bush Plane", "Racer", "Sport Jet", "Ultralight", "Glider",
            "F/A-18 Fighter", "SR-71 Blackbird", "Bald Eagle")
ARTWORK = (("Airfield aprons", "airfield-aprons", 25), ("Hillside quarries", "quarry-profiles", 25), ("Quayside workshops", "quay-workshops", 25), ("Sheltered cove dunes", "cove-dunes", 25), ("Cultivated fields", "cultivated-fields", 25), ("Columnar rock faces", "columnar-headwalls", 25), ("Desert alluvial fans", "alluvial-fans", 25), ("Solar array layouts", "solar-layouts", 25), ("Municipal water towers", "water-towers", 25), ("Glacial moraines", "moraines", 25), ("Mountain lake shores", "tarns", 25), ("Alpine glaciers", "glaciers", 25), ("River islands", "river-islands", 25), ("River and coastal bridges", "bridges", 25), ("Mountain cableways", "cableways", 25), ("Lighthouses", "lighthouses", 25), ("Coastal rock formations", "coastal-rocks", 25), ("Countryside buildings", "countryside-buildings", 25), ("Sandstone mesas", "mesa-profiles", 25), ("Floodplain details", "floodplain-details", 25), ("Industrial landmarks", "industrial-landmarks", 25), ("Alpine ground details", "alpine-ground-details", 25), ("Headland details", "headland-details", 25), ("Grassland details", "grassland-details", 25), ("Dryland details", "dryland-details", 25), ("Woodland details", "woodland-details", 25), ("Airfield buildings", "airfield-buildings", 25), ("Street buildings", "street-buildings", 25), ("Freshwater details", "freshwater-details", 25), ("Mountain details", "mountain-details", 25), ("Roadside models", "roadside-models", 30), ("Harbor models", "harbor-models", 30), ("Railway fixtures", "rail-stations", 25), ("Farm details", "farm-details", 25), ("Coastal details", "coastal-details", 25), ("Skyline landmarks", "skyline-landmarks", 75), ("Summit landmarks", "summit-landmarks", 25), ("Town landmarks", "town-landmarks", 25),
           ("Neighborhood buildings", "neighborhood-buildings", 27),
           ("Habitat details", "habitat-details", 30), ("City towers", "towers", 250), ("Trees", "trees", 256), ("Rocks", "rocks", 256), ("Bushes", "bushes", 256),
           ("Cacti", "cacti", 256), ("Freight wagons", "wagons", 250), ("Vessels", "vessels", 250), ("Service vessels", "service-vessels", 25),
           ("Wetland plants", "wetlands", 256),
           ("Locomotives", "locomotives", 6), ("Waterfront landmarks", "waterfront-landmarks", 4))


def make_preview(entry):
    path = DOCS / f"assets/readme/{entry['id']}.gif"
    path.parent.mkdir(exist_ok=True)
    subprocess.run([
        "ffmpeg", "-v", "error", "-y", "-i", str(DOCS / entry["src"]),
        "-filter_complex_threads", "1", "-filter_complex",
        "fps=10,scale=216:216:flags=neighbor,split[a][b];"
        "[a]palettegen=max_colors=96:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3",
        "-loop", "0", str(path),
    ], check=True)


def preview(label, file, href=None, width=220):
    escaped = html.escape(label, quote=True)
    source = html.escape(file, quote=True)
    destination = html.escape(href or file, quote=True)
    return f'<a href="{destination}"><img src="{source}" width="{width}" alt="{escaped}"></a>'


def grid(items, columns=3):
    lines = ["<table>"]
    for row_start in range(0, len(items), columns):
        lines.append("  <tr>")
        for label, artwork in items[row_start:row_start + columns]:
            lines.append(f'    <td align="center">{artwork}<br>{html.escape(label)}</td>')
        lines.append("  </tr>")
    return "\n".join(lines + ["</table>"])


def gameplay_card(entry):
    animation = entry["format"] in {"Video", "GIF"}
    original = entry["src"]
    source = f"assets/readme/{entry['id']}.gif" if animation else entry["poster"]
    if not (DOCS / source).is_file():
        raise FileNotFoundError(source)
    label = entry["title"]
    full_size = PAGES + original if animation else f"docs/{original}"
    card = preview(label, f"docs/{source}", full_size)
    if animation and entry.get("poster") and entry["poster"] != original:
        still = html.escape(f"docs/{entry['poster']}", quote=True)
        card += f'<br><a href="{still}">Still</a>'
    return label, card


def render(entries):
    videos = {entry["id"]: entry for entry in entries}
    sections = [
        "## Gameplay showcase",
        "Select a preview to open the full-size clip or screenshot.",
        grid([gameplay_card(videos[key]) for key in FEATURED]),
    ]
    categories = dict.fromkeys(entry["category"] for entry in entries)
    for category in categories:
        group = [gameplay_card(entry) for entry in reversed(entries)
                 if entry["category"] == category and entry["id"] not in FEATURED]
        if group:
            sections.extend((f"### {category}", grid(group)))
    sections.extend(("## Artwork gallery", "### Aircraft and picker screens"))
    for start in range(0, len(AIRCRAFT), 4):
        aircraft = []
        for index in range(start, min(start + 4, len(AIRCRAFT))):
            name = AIRCRAFT[index]
            turntable = preview(f"{name} turntable", f"docs/assets/turntable-{index}.gif", width=168)
            picker = preview(f"{name} picker screen", f"docs/assets/picker-{index}.png", width=168)
            aircraft.append((name, f"{turntable}<br>{picker}"))
        sections.append(grid(aircraft, columns=4))
    sections.append("### Scenery sprites")
    for title, slug, count in ARTWORK:
        path = f"docs/assets/artwork/{slug}.png"
        sections.extend((f"#### {title} · {count} variants", preview(title, path, width=780)))
        if slug in {"airfield-aprons", "quarry-profiles", "quay-workshops", "cove-dunes", "cultivated-fields", "columnar-headwalls", "alluvial-fans", "solar-layouts", "water-towers", "moraines", "tarns", "glaciers", "river-islands", "bridges", "cableways", "lighthouses", "coastal-rocks", "countryside-buildings", "mesa-profiles", "floodplain-details", "industrial-landmarks", "alpine-ground-details", "town-landmarks", "summit-landmarks", "service-vessels", "headland-details", "grassland-details", "dryland-details", "woodland-details", "airfield-buildings", "street-buildings", "freshwater-details", "mountain-details", "skyline-landmarks", "coastal-details", "farm-details", "rail-stations", "harbor-models", "roadside-models"}:
            sections.append(preview(f"{title} rotating", f"docs/assets/artwork/{slug}.gif", width=550))
    return "\n\n".join(sections)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--refresh-previews", action="store_true", help="Regenerate animated thumbnails with FFmpeg.")
    args = parser.parse_args()
    entries = json.loads((DOCS / "assets/media.json").read_text())
    if args.refresh_previews:
        with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
            list(pool.map(make_preview, (item for item in entries if item["format"] in {"Video", "GIF"})))
    readme = ROOT / "README.md"
    before, start, remainder = readme.read_text().partition("<!-- BEGIN GALLERY -->")
    _, end, after = remainder.partition("<!-- END GALLERY -->")
    if not start or not end:
        raise ValueError("README gallery markers are missing")
    readme.write_text(f"{before}{start}\n\n{render(entries)}\n\n{end}{after}")


if __name__ == "__main__":
    main()
