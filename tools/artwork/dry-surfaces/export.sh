#!/bin/sh
set -eu
tools=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
game=$(CDPATH='' cd -- "$tools/../../.." && pwd)
assets=${SCENIC_ARTWORK_OUTPUT:-"$game/docs/assets/artwork"}
library=${SCENIC_ARTWORK_LIBRARY:-"$game/target/release/libscenic_flight.rlib"}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-dry-surfaces.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$assets" "$scratch/ground_cover"

cp "$game/src/math.rs" "$scratch/math.rs"
cp "$game/src/ground_cover/dryland.rs" "$scratch/ground_cover/dryland.rs"
cp "$game/src/ground_cover.rs" "$scratch/ground_cover.rs"
printf '\ninclude!(env!("SCENIC_DRY_ATLAS"));\n' >> "$scratch/ground_cover.rs"
cp "$tools/export.rs" "$scratch/main.rs"
SCENIC_DRY_ATLAS="$tools/atlas.rs" rustc --edition 2021 -O "$scratch/main.rs" \
    --extern "scenic_flight=$library" -L "dependency=$(dirname "$library")/deps" \
    -o "$scratch/export"
"$scratch/export" > "$scratch/dry-surfaces.ppm"
ffmpeg -hide_banner -loglevel error -y -i "$scratch/dry-surfaces.ppm" -frames:v 1 "$assets/dry-surfaces.png"
