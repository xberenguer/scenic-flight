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
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/vessels/export.sh"
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
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/towers/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/summits/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/mesas/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/countryside/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/coastal-rocks/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/lighthouses/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/cableways/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/bridges/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/river-islands/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/glaciers/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/tarns/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/moraines/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/water-towers/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/solar-layouts/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/alluvial-fans/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/columnar-headwalls/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/cultivated-fields/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/cove-dunes/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/quay-workshops/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/quarry-profiles/export.sh"
SCENIC_ARTWORK_OUTPUT="$scratch" sh "$tools/artwork/airfield-aprons/export.sh"
mkdir -p "$assets"
cp "$scratch/airfield-aprons.png" "$scratch/airfield-aprons.gif" "$scratch/airfield-aprons.txt" "$assets/"
cp "$scratch/quarry-profiles.png" "$scratch/quarry-profiles.gif" "$scratch/quarry-profiles.txt" "$assets/"
cp "$scratch/quay-workshops.png" "$scratch/quay-workshops.gif" "$scratch/quay-workshops.txt" "$assets/"
cp "$scratch/cove-dunes.png" "$scratch/cove-dunes.gif" "$scratch/cove-dunes.txt" "$assets/"
cp "$scratch/cultivated-fields.png" "$scratch/cultivated-fields.gif" "$scratch/cultivated-fields.txt" "$assets/"
cp "$scratch/columnar-headwalls.png" "$scratch/columnar-headwalls.gif" "$scratch/columnar-headwalls.txt" "$assets/"
cp "$scratch/alluvial-fans.png" "$scratch/alluvial-fans.gif" "$scratch/alluvial-fans.txt" "$assets/"
cp "$scratch/solar-layouts.png" "$scratch/solar-layouts.gif" "$scratch/solar-layouts.txt" "$assets/"
cp "$scratch/water-towers.png" "$scratch/water-towers.gif" "$scratch/water-towers.txt" "$assets/"
cp "$scratch/moraines.png" "$scratch/moraines.gif" "$scratch/moraines.txt" "$assets/"
cp "$scratch/tarns.png" "$scratch/tarns.gif" "$scratch/tarns.txt" "$assets/"
cp "$scratch/glaciers.png" "$scratch/glaciers.gif" "$scratch/glaciers.txt" "$assets/"
cp "$scratch/river-islands.png" "$scratch/river-islands.gif" "$scratch/river-islands.txt" "$assets/"
cp "$scratch/bridges.png" "$scratch/bridges.gif" "$scratch/bridges.txt" "$assets/"
cp "$scratch/cableways.png" "$scratch/cableways.gif" "$scratch/cableways.txt" "$assets/"
cp "$scratch/lighthouses.png" "$scratch/lighthouses.gif" "$scratch/lighthouses.txt" "$assets/"
cp "$scratch/coastal-rocks.png" "$scratch/coastal-rocks.gif" "$scratch/coastal-rocks.txt" "$assets/"
cp "$scratch/towers.png" "$scratch/skyline-landmarks.png" "$scratch/skyline-landmarks.gif" "$scratch/skyline-landmarks.txt" "$assets/"
cp "$scratch/summit-landmarks.png" "$scratch/summit-landmarks.gif" "$scratch/summit-landmarks.txt" "$assets/"
cp "$scratch/mesa-profiles.png" "$scratch/mesa-profiles.gif" "$scratch/mesa-profiles.txt" "$assets/"
cp "$scratch/countryside-buildings.png" "$scratch/countryside-buildings.gif" "$scratch/countryside-buildings.txt" "$assets/"
cp "$scratch/tree-crown-atlas.png" "$assets/trees.png"
cp "$scratch/wagons.png" "$scratch/wagons.json" "$scratch/locomotives.png" "$assets/"
cp "$scratch/vessels.png" "$scratch/vessels.json" "$assets/"
cp "$scratch/service-vessels.png" "$scratch/service-vessels.gif" "$scratch/service-vessels.json" "$assets/"
cp "$scratch/wetland-plants.png" "$assets/wetlands.png"
cp "$scratch/rail-stations.png" "$scratch/rail-stations.gif" "$assets/"
cp "$scratch/harbor-models.png" "$scratch/harbor-models.gif" "$assets/"
cp "$scratch/roadside-models.png" "$scratch/roadside-models.gif" "$assets/"
cp "$scratch/coastal-details.png" "$scratch/coastal-details.gif" "$assets/"
cp "$scratch/farm-details.png" "$scratch/farm-details.gif" "$assets/"
cp "$scratch/mountain-details.png" "$scratch/mountain-details.gif" "$assets/"
cp "$scratch/freshwater-details.png" "$scratch/freshwater-details.gif" "$assets/"
cp "$scratch/street-buildings.png" "$scratch/street-buildings.gif" "$assets/"
cp "$scratch/town-landmarks.png" "$scratch/town-landmarks.gif" "$scratch/town-landmarks.txt" "$assets/"
cp "$scratch/industrial-landmarks.png" "$scratch/industrial-landmarks.gif" "$scratch/industrial-landmarks.txt" "$assets/"
cp "$scratch/airfield-buildings.png" "$scratch/airfield-buildings.gif" "$assets/"
cp "$scratch/woodland-details.png" "$scratch/woodland-details.gif" "$assets/"
cp "$scratch/dryland-details.png" "$scratch/dryland-details.gif" "$assets/"
cp "$scratch/grassland-details.png" "$scratch/grassland-details.gif" "$assets/"
cp "$scratch/headland-details.png" "$scratch/headland-details.gif" "$assets/"
cp "$scratch/alpine-ground-details.png" "$scratch/alpine-ground-details.gif" "$assets/"
cp "$scratch/floodplain-details.png" "$scratch/floodplain-details.gif" "$assets/"
for category in rocks bushes cacti; do
    cp "$scratch/desert-$category.png" "$assets/$category.png"
done
