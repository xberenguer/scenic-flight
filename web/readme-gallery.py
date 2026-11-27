#!/usr/bin/env python3
"""Refresh the README gallery using the published gameplay and artwork manifests."""

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
    source = DOCS / entry["src"]
    if source.resolve() == path.resolve():
        if not source.is_file():
            raise FileNotFoundError(source)
        return
    path.parent.mkdir(exist_ok=True)
    subprocess.run([
        "ffmpeg", "-v", "error", "-y", "-i", str(source),
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


def vegas_artwork():
    sections = ["## Las Vegas artwork"]
    for directory in ("landmarks", "venues"):
        manifest = DOCS / f"assets/vegas/{directory}/index.json"
        groups = json.loads(manifest.read_text())["groups"]

        def asset(file):
            path = (manifest.parent / file).resolve()
            if not path.is_file():
                raise FileNotFoundError(path)
            return path.relative_to(ROOT).as_posix()

        for group in groups:
            cards = []
            for entry in group["entries"]:
                name = entry["name"]
                image = asset(entry["image"])
                poster = asset(entry["poster"]) if entry.get("poster") else image
                animation = asset(entry["animation"]) if entry.get("animation") else None
                thumbnail = (animation or poster) if directory == "landmarks" else poster
                width = 158 if directory == "landmarks" else 105
                card = preview(name, thumbnail, animation or image, width=width)
                kind = html.escape(entry["representation"])
                cards.append((name, f'{card}<br><sub>{kind}</sub>'))
            sections.extend((f'### {group["title"]}', grid(cards, columns=4 if directory == "landmarks" else 6)))
    return sections


def render(entries, order):
    videos = {entry["id"]: entry for entry in entries}
    featured = order["featured"][:6]
    sections = [
        "## Gameplay showcase",
        "Select a preview to open the full-size clip or screenshot.",
        grid([gameplay_card(videos[key]) for key in featured]),
    ]
    sections.extend((
        "[Sky and weather](#sky--weather) · [Encounters](#encounters) · [Biomes](#biomes) · "
        "[Built world](#built-world) · [Aircraft](#aircraft) · [Interface](#interface) · "
        "[Aircraft and picker screens](#aircraft-and-picker-screens) · [Rare flight moments](#rare-flight-moments) · [Artwork gallery](#artwork-gallery) · "
        "[Las Vegas artwork](#las-vegas-artwork) · [World generation](#world-generation)",
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

    moment_file = DOCS / "assets/artwork/flight-moments.tsv"
    moments = {int(index): names for index, _, *names in (
        row.split("\t") for row in moment_file.read_text().splitlines()
    )}
    sections.extend((
        "## Rare flight moments",
        "Each flight has a two-in-three chance at a rare moment every 4 min 20 sec. "
        "Events last 9.5–14.5 simulated seconds.",
    ))
    for entry in aircraft:
        index, name = entry["asset"], entry["name"]
        labels = " · ".join(moments[index])
        sections.extend((f"### {name}", labels, preview(
            f"{name} rare flight moments. {labels}",
            f"docs/assets/artwork/flight-moments-{index}.gif", width=720,
        )))

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
            animation = f"assets/artwork/{slug}.gif"
            sections.append(f"#### {title} · {count} variants")
            if slug == "snowfall-profiles" and (DOCS / animation).is_file():
                sections.append(preview(title, f"docs/{animation}", width=780))
            else:
                sections.append(preview(title, path, width=780))
                if (DOCS / animation).is_file():
                    sections.append(preview(f"{title} animated atlas", f"docs/{animation}", width=550))
    sections.extend((
        "## Los Angeles",
        "[Fly Los Angeles to Las Vegas](https://xberenguer.github.io/scenic-flight/play.html?destination=southwest&departure=to-las-vegas). "
        "Both cities sit at their real locations in the same metre-scale world. The route passes Barstow, Baker, and Primm over USGS terrain, "
        "with mapped streets and buildings, rail around Barstow, and a runway at Baker. "
        "Scenery outside the mapped areas is generated from the flight seed. [Southwest map details](docs/LOS-ANGELES.md)",
        "Fly twelve mapped approaches, from Downtown and Griffith Observatory to LAX, Santa Monica Pier, Venice Beach, and Getty Center. "
        "The 48 × 48 km detail area is at metre scale. Downtown landmarks include U.S. Bank Tower, "
        "Wilshire Grand Center, Los Angeles City Hall, and Walt Disney Concert Hall.",
        grid([
            ("Los Angeles City Hall", preview("City Hall game model", "docs/assets/los-angeles/los-angeles-city-hall.png")),
            ("Walt Disney Concert Hall", preview("Concert Hall game model", "docs/assets/los-angeles/walt-disney-concert-hall.png")),
        ], columns=2),
        "[Los Angeles coverage, checkpoint audit, and source data](docs/LOS-ANGELES.md)",
        "## Las Vegas",
        "Select **Las Vegas** under **Destination**, then choose the **Strip**, **Harry Reid Airport**, "
        "**Downtown**, **Sphere**, **Paris Las Vegas**, **High Roller**, **Venetian and Palazzo**, "
        "**Allegiant Stadium**, or **Fremont Street**. "
        "One world unit is one metre. The first area uses mapped road "
        "and building footprints with USGS terrain and FAA runway geometry.",
        grid([
            ("The Strip", preview("Mapped Las Vegas Strip, including the Sphere", "docs/assets/planning/las-vegas-strip.png", width=250)),
            ("Harry Reid Airport", preview("Runways and terminals at Harry Reid Airport", "docs/assets/planning/las-vegas-airport.png", width=250)),
            ("Downtown", preview("Mapped streets and buildings in downtown Las Vegas", "docs/assets/planning/las-vegas-downtown.png", width=250)),
        ]),
        preview("Las Vegas Valley elevation, the mapped urban area, and Lake Mead",
                "docs/assets/planning/las-vegas-wide.png", width=820),
        "[Las Vegas coverage and source data](docs/LAS-VEGAS.md)",
    ))
    sections.extend(vegas_artwork())
    sections.extend((
        "## World generation",
        "Metropolises cover a broad basin with connected residential, commercial, and industrial "
        "districts. Seed 15 spans approximately **23,936 × 24,704 world units**. "
        "Ordinary settlements grow along regional roads.",
        preview("Map of the full metropolis from seed 15",
                "docs/assets/planning/metropolis-seed-15.png", width=640),
        grid([
            ("Largest developed footprint · seed 149", preview(
                "Map of generated streets and buildings", "docs/assets/planning/urban-149--6--4.png", width=320)),
            ("Longest surveyed settlement · seed 241", preview(
                "Map of generated districts and approach roads", "docs/assets/planning/urban-241--2--4.png", width=320)),
        ], columns=2),
        "Metropolitan airports connect two runways to a shared gate apron and a city street.",
        preview("Map of the metropolitan airport from seed 19",
                "docs/assets/planning/metropolitan-airport.png", width=640),
        "Metropolitan trains stop at elevated stations and connect the airport to two city corridors.",
        preview("Metropolitan rail routes and stations in seed 15",
                "docs/assets/planning/metro-rail-seed-15.png", width=640),
        "Major city roads continue into villages, farmland, and wooded foothills.",
        preview("Metropolitan outskirts in seed 15",
                "docs/assets/planning/outskirts-seed-15.png", width=640),
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
