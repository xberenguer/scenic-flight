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


def render(entries, order):
    videos = {entry["id"]: entry for entry in entries}
    featured = order["featured"]
    sections = [
        "## Gameplay showcase",
        "Select a preview to open the full-size clip or screenshot.",
        grid([gameplay_card(videos[key]) for key in featured]),
    ]
    sections.extend((
        "[Sky and weather](#sky--weather) · [Encounters](#encounters) · [Biomes](#biomes) · "
        "[Built world](#built-world) · [Aircraft](#aircraft) · [Interface](#interface) · "
        "[Aircraft and picker screens](#aircraft-and-picker-screens) · [Artwork gallery](#artwork-gallery) · "
        "[World generation](#world-generation)",
    ))
    categories = dict.fromkeys(["Sky & Weather", "Encounters", "Biomes", "Built World",
                                "Aircraft", "Interface", *(entry["category"] for entry in entries)])
    for category in categories:
        group = [gameplay_card(entry) for entry in reversed(entries)
                 if entry["category"] == category and entry["id"] not in featured]
        if group:
            sections.extend((f"### {category}", grid(group)))
    sections.append("## Aircraft and picker screens")
    aircraft = order["aircraft"]
    for start in range(0, len(aircraft), 4):
        cards = []
        for entry in aircraft[start:start + 4]:
            name, index = entry["name"], entry["asset"]
            turntable = preview(f"{name} turntable", f"docs/assets/turntable-{index}.gif", width=168)
            picker = preview(f"{name} picker screen", f"docs/assets/picker-{index}.png", width=168)
            cards.append((name, f"{picker}<br>{turntable}"))
        sections.append(grid(cards, columns=4))

    sections.append("## Artwork gallery")
    groups = order["scenery"]
    anchors = " · ".join(
        f'[{group["title"]}](#{group["title"].lower().replace(" ", "-")})'
        for group in groups
    )
    sections.append(anchors)
    for group in groups:
        sections.append(f'### {group["title"]}')
        for entry in group["sections"]:
            title, slug, count = entry["title"], entry["id"], entry["count"]
            path = f"docs/assets/artwork/{slug}.png"
            sections.extend((f"#### {title} · {count} variants", preview(title, path, width=780)))
            animation = f"assets/artwork/{slug}.gif"
            if (DOCS / animation).is_file():
                sections.append(preview(f"{title} animated atlas", f"docs/{animation}", width=550))
    sections.extend((
        "## World generation",
        "Cities grow around road approaches and expand where the land allows. "
        "The longest of 531 sampled settlements spanned **814 world units**.",
        grid([
            ("Largest developed footprint · seed 149", preview(
                "Map of generated streets and buildings", "docs/assets/planning/urban-149--6--4.png", width=320)),
            ("Longest surveyed settlement · seed 241", preview(
                "Map of generated districts and approach roads", "docs/assets/planning/urban-241--2--4.png", width=320)),
        ], columns=2),
        "[City dimensions, street grids, and placement rules](docs/WORLD-GENERATION.md)",
    ))
    return "\n\n".join(sections)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--refresh-previews", action="store_true", help="Regenerate animated thumbnails with FFmpeg.")
    args = parser.parse_args()
    entries = json.loads((DOCS / "assets/media.json").read_text())
    order = json.loads((DOCS / "assets/gallery-order.json").read_text())
    if args.refresh_previews:
        with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
            list(pool.map(make_preview, (item for item in entries if item["format"] in {"Video", "GIF"})))
    readme = ROOT / "README.md"
    before, start, remainder = readme.read_text().partition("<!-- BEGIN GALLERY -->")
    _, end, after = remainder.partition("<!-- END GALLERY -->")
    if not start or not end:
        raise ValueError("README gallery markers are missing")
    readme.write_text(f"{before}{start}\n\n{render(entries, order)}\n\n{end}{after}")


if __name__ == "__main__":
    main()
