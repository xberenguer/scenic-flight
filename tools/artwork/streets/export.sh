#!/bin/sh
set -eu
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
game=$(CDPATH='' cd -- "$here/../../.." && pwd)
out=${SCENIC_ARTWORK_OUTPUT:-"$here"}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-street-art.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

# Include the runtime samples with paths adapted for the standalone exporter.
sed -e '/^\/\/!/d' \
    -e 's/^mod districts;$/#[path = "districts.rs"]\
mod districts;/' \
    -e 's/^mod street_blocks;$/#[path = "street_blocks.rs"]\
mod street_blocks;/' "$game/src/settlement/building_forms.rs" >"$scratch/building_forms.rs"
cp "$game"/src/settlement/building_forms/*.rs "$scratch/"

sed '/^\/\/!/d' "$game/src/airfield/buildings.rs" >"$scratch/airfield.rs"

cargo build --manifest-path "$game/Cargo.toml" --locked --release --lib \
    --features terrain-fork,small-raster-ray-budget,ground-cover
SCENIC_BUILDING_FORMS="$scratch/building_forms.rs" \
SCENIC_AIRFIELD_ROOFS="$scratch/airfield.rs" \
rustc --edition 2021 -O "$here/export.rs" \
    --extern "scenic_flight=$game/target/release/libscenic_flight.rlib" \
    -L "dependency=$game/target/release/deps" -o "$scratch/export"
mkdir -p "$out"
encode() {
    ffmpeg -hide_banner -loglevel error -y -i "$scratch/$1-00.ppm" \
        -frames:v 1 "$out/$1.png"
    ffmpeg -hide_banner -loglevel error -y -framerate 6 -i "$scratch/$1-%02d.ppm" \
        -filter_complex_threads 1 \
        -filter_complex 'split[a][b];[a]palettegen=max_colors=112[p];[b][p]paletteuse=dither=bayer' \
        -loop 0 "$out/$1.gif"
}
if [ "${1:-}" != "--airfield-only" ]; then
    "$scratch/export" "$scratch" street
    encode street-buildings
fi
"$scratch/export" "$scratch" airfield
encode airfield-buildings
