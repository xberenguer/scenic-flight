#!/usr/bin/env python3
"""Export isolated runtime landmark PNGs, GIFs, and their gallery index."""

import argparse
import gzip
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
GROUPS = {
    "sphere-programs": "Sphere LED programs",
    "animated-landmarks": "Animated landmarks",
    "landmark-models": "Landmark models",
}


def ffmpeg(*arguments):
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", *map(str, arguments)], check=True)


def export(pack, output, side):
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="scenic-vegas-gallery-") as temporary:
        temporary = Path(temporary)
        packed = temporary / "las-vegas.bin"
        if pack.suffix == ".gz":
            with gzip.open(pack, "rb") as source, packed.open("wb") as target:
                shutil.copyfileobj(source, target)
        else:
            packed = pack
        frames = temporary / "frames"
        subprocess.run([
            "cargo", "run", "--quiet", "--release", "--features", "landmark-gallery",
            "--example", "vegas-landmarks", "--", str(packed), str(frames), str(side),
        ], cwd=ROOT, check=True)
        groups = {key: [] for key in GROUPS}
        for row in (frames / "metadata.tsv").read_text().splitlines():
            id_, name, group, venue, source, count, x, z, terrain = row.split("\t")
            poster = f"{id_}.png"
            source_directory = frames / id_
            ffmpeg("-i", source_directory / "poster.ppm", "-frames:v", 1, output / poster)
            entry = {
                "id": id_, "name": name, "image": poster,
                "x": float(x), "z": float(z), "source": source,
                "representation": "Packed terrain" if terrain == "true" else "Runtime model",
            }
            if venue:
                entry["venue"] = venue
            if int(count):
                animation = f"{id_}.gif"
                ffmpeg(
                    "-framerate", 10, "-i", source_directory / "frame-%03d.ppm",
                    "-filter_complex",
                    "[0:v]split[a][b];[a]palettegen=max_colors=96:stats_mode=diff[p];"
                    "[b][p]paletteuse=dither=none",
                    "-loop", 0, output / animation,
                )
                entry.update(image=animation, poster=poster, animation=animation)
            groups[group].append(entry)
        index = {
            "schema": 1, "background": "#141a20",
            "groups": [
                {"key": key, "title": title, "entries": groups[key]}
                for key, title in GROUPS.items()
            ],
        }
        (output / "index.json").write_text(json.dumps(index, indent=2, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", type=Path, default=ROOT / "docs/assets/geography/las-vegas.bin.gz")
    parser.add_argument("--output", type=Path, default=ROOT / "docs/assets/vegas/landmarks")
    parser.add_argument("--size", type=int, default=192)
    args = parser.parse_args()
    export(args.pack, args.output, args.size)
