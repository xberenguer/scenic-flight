#!/bin/sh
set -eu

tools=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
assets="$tools/../docs/assets/artwork"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-artwork.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

SCENIC_ARTWORK_OUTPUT="$scratch" python3 "$tools/artwork/trees/export_tree_crowns.py"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/streets/export.sh"
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
rustc --edition 2024 -O "$tools/artwork/trains/export-stations.rs" \
    --extern "scenic_flight=$game/target/release/libscenic_flight.rlib" \
    -L "dependency=$game/target/release/deps" -o "$scratch/export-stations"
"$scratch/export-stations" "$scratch"
ffmpeg -hide_banner -loglevel error -y -i "$scratch/rail-stations-00.ppm" -vf 'scale=2*iw:2*ih:flags=neighbor' -frames:v 1 "$scratch/rail-stations.png"
ffmpeg -hide_banner -loglevel error -y -framerate 8 -i "$scratch/rail-stations-%02d.ppm" \
    -filter_complex_threads 1 \
    -filter_complex 'scale=2*iw:2*ih:flags=neighbor,split[a][b];[a]palettegen=max_colors=112[p];[b][p]paletteuse=dither=bayer' \
    -loop 0 "$scratch/rail-stations.gif"
rustc --edition 2024 -O "$tools/artwork/vessels/export.rs" \
    --extern "scenic_flight=$game/target/release/libscenic_flight.rlib" \
    -L "dependency=$game/target/release/deps" -o "$scratch/export-vessels"
"$scratch/export-vessels" "$scratch/vessels.json" > "$scratch/vessels.ppm"
ffmpeg -hide_banner -loglevel error -y -i "$scratch/vessels.ppm" -frames:v 1 "$scratch/vessels.png"
rustc --edition 2024 -O "$tools/artwork/harbors/export.rs" \
    --extern "scenic_flight=$game/target/release/libscenic_flight.rlib" \
    -L "dependency=$game/target/release/deps" -o "$scratch/export-harbors"
"$scratch/export-harbors" "$scratch"
ffmpeg -hide_banner -loglevel error -y -i "$scratch/harbor-00.ppm" -vf 'scale=2*iw:2*ih:flags=neighbor' -frames:v 1 "$scratch/harbor-models.png"
ffmpeg -hide_banner -loglevel error -y -framerate 8 -i "$scratch/harbor-%02d.ppm" \
    -filter_complex_threads 1 \
    -filter_complex 'scale=2*iw:2*ih:flags=neighbor,split[a][b];[a]palettegen=max_colors=112[p];[b][p]paletteuse=dither=bayer' \
    -loop 0 "$scratch/harbor-models.gif"
rustc --edition 2024 -O "$tools/artwork/roadside/export.rs" \
    --extern "scenic_flight=$game/target/release/libscenic_flight.rlib" \
    -L "dependency=$game/target/release/deps" -o "$scratch/export-roadside"
"$scratch/export-roadside" "$scratch"
ffmpeg -hide_banner -loglevel error -y -i "$scratch/roadside-00.ppm" -vf 'scale=2*iw:2*ih:flags=neighbor' -frames:v 1 "$scratch/roadside-models.png"
ffmpeg -hide_banner -loglevel error -y -framerate 8 -i "$scratch/roadside-%02d.ppm" \
    -filter_complex_threads 1 \
    -filter_complex 'scale=2*iw:2*ih:flags=neighbor,split[a][b];[a]palettegen=max_colors=112[p];[b][p]paletteuse=dither=bayer' \
    -loop 0 "$scratch/roadside-models.gif"
rustc --edition 2024 -O "$tools/artwork/towers/export.rs" \
    --extern "scenic_flight=$game/target/release/libscenic_flight.rlib" \
    -L "dependency=$game/target/release/deps" -o "$scratch/export-towers"
"$scratch/export-towers" > "$scratch/towers.ppm"
"$scratch/export-towers" --landmarks > "$scratch/skyline-landmarks.ppm"
"$scratch/export-towers" --landmark-names > "$scratch/skyline-landmarks.txt"
ffmpeg -hide_banner -loglevel error -y -i "$scratch/skyline-landmarks.ppm" -frames:v 1 "$scratch/skyline-landmarks.png"
ffmpeg -hide_banner -loglevel error -y -i "$scratch/towers.ppm" -frames:v 1 "$scratch/towers.png"
mkdir -p "$assets"
cp "$scratch/towers.png" "$scratch/skyline-landmarks.png" "$scratch/skyline-landmarks.txt" "$assets/"
cp "$scratch/tree-crown-atlas.png" "$assets/trees.png"
cp "$scratch/wagons.png" "$scratch/wagons.json" "$scratch/locomotives.png" "$assets/"
cp "$scratch/vessels.png" "$scratch/vessels.json" "$assets/"
cp "$scratch/wetland-plants.png" "$assets/wetlands.png"
cp "$scratch/rail-stations.png" "$scratch/rail-stations.gif" "$assets/"
cp "$scratch/harbor-models.png" "$scratch/harbor-models.gif" "$assets/"
cp "$scratch/roadside-models.png" "$scratch/roadside-models.gif" "$assets/"
cp "$scratch/coastal-details.png" "$scratch/coastal-details.gif" "$assets/"
cp "$scratch/farm-details.png" "$scratch/farm-details.gif" "$assets/"
cp "$scratch/mountain-details.png" "$scratch/mountain-details.gif" "$assets/"
cp "$scratch/freshwater-details.png" "$scratch/freshwater-details.gif" "$assets/"
cp "$scratch/street-buildings.png" "$scratch/street-buildings.gif" "$assets/"
cp "$scratch/airfield-buildings.png" "$scratch/airfield-buildings.gif" "$assets/"
cp "$scratch/woodland-details.png" "$scratch/woodland-details.gif" "$assets/"
for category in rocks bushes cacti; do
    cp "$scratch/desert-$category.png" "$assets/$category.png"
done
