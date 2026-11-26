#!/bin/sh
set -eu
tools=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
game=$(CDPATH='' cd -- "$tools/../../.." && pwd)
assets=${SCENIC_ARTWORK_OUTPUT:-"$game/docs/assets/artwork"}
target=${CARGO_TARGET_DIR:-"$game/target"}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-tunnel-facades.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$assets"

rustc --edition 2024 -O "$tools/export.rs" \
    --extern "scenic_flight=$target/release/libscenic_flight.rlib" \
    -L "dependency=$target/release/deps" -o "$scratch/export"
"$scratch/export" "$scratch"
ffmpeg -v error -y -i "$scratch/facade-00.ppm" -frames:v 1 "$assets/tunnel-facades.png"
ffmpeg -v error -y -framerate 8 -i "$scratch/facade-%02d.ppm" \
    -filter_complex_threads 1 \
    -filter_complex 'split[a][b];[a]palettegen=max_colors=112[p];[b][p]paletteuse=dither=bayer' \
    -loop 0 "$assets/tunnel-facades.gif"
cp "$scratch/tunnel-facades.txt" "$assets/"
