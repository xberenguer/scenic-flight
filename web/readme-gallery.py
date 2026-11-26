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
FEATURED = ("mountain-pass", "coastal-soaring", "city-approach", "high-cruise", "fighter-afterburners")
AIRCRAFT = ("Bush Plane", "Racer", "Sport Jet", "Ultralight", "Glider",
            "F/A-18 Fighter", "SR-71 Blackbird", "Bald Eagle")
ARTWORK = (("Trees", "trees", 256), ("Rocks", "rocks", 256), ("Bushes", "bushes", 256),
           ("Cacti", "cacti", 256), ("Freight wagons", "wagons", 25))


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
    return label, preview(label, f"docs/{source}", full_size)


def render(entries):
    videos = {entry["id"]: entry for entry in entries}
    sections = [
        "## Gameplay showcase",
        "Select a preview to open the full-size clip or screenshot.",
        grid([gameplay_card(videos[key]) for key in FEATURED]),
    ]
    categories = dict.fromkeys(entry["category"] for entry in entries)
    for category in categories:
        group = [gameplay_card(entry) for entry in entries
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
