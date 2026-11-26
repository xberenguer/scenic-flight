#!/bin/sh
set -eu

tools=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
assets="$tools/../docs/assets/artwork"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-artwork.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

SCENIC_ARTWORK_OUTPUT="$scratch" python3 "$tools/artwork/trees/export_tree_crowns.py"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/desert/export.sh"
game=$(CDPATH='' cd -- "$tools/.." && pwd)
SCENIC_WAGON_SOURCE="$game/src/train_art/wagons.rs" \
    SCENIC_FREIGHT_CATALOG="$game/src/train_art/freight_catalog.rs" \
    rustc --edition 2024 -O "$tools/artwork/trains/export.rs" \
    --extern "scenic_flight=$game/target/release/libscenic_flight.rlib" \
    -L "dependency=$game/target/release/deps" -o "$scratch/export-wagons"
"$scratch/export-wagons" "$scratch/wagons.json" "$scratch/locomotives.ppm" > "$scratch/wagons.ppm"
ffmpeg -hide_banner -loglevel error -y -i "$scratch/wagons.ppm" -frames:v 1 "$scratch/wagons.png"
ffmpeg -hide_banner -loglevel error -y -i "$scratch/locomotives.ppm" -frames:v 1 -vf scale=2*iw:2*ih:flags=neighbor "$scratch/locomotives.png"
rustc --edition 2024 -O "$tools/artwork/vessels/export.rs" \
    --extern "scenic_flight=$game/target/release/libscenic_flight.rlib" \
    -L "dependency=$game/target/release/deps" -o "$scratch/export-vessels"
"$scratch/export-vessels" "$scratch/vessels.json" > "$scratch/vessels.ppm"
ffmpeg -hide_banner -loglevel error -y -i "$scratch/vessels.ppm" -frames:v 1 "$scratch/vessels.png"
mkdir -p "$assets"
cp "$scratch/tree-crown-atlas.png" "$assets/trees.png"
cp "$scratch/wagons.png" "$scratch/wagons.json" "$scratch/locomotives.png" "$assets/"
cp "$scratch/vessels.png" "$scratch/vessels.json" "$assets/"
cp "$scratch/wetland-plants.png" "$assets/wetlands.png"
for category in rocks bushes cacti; do
    cp "$scratch/desert-$category.png" "$assets/$category.png"
done
