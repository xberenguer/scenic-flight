#!/bin/sh
set -eu

tools=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
game=$(CDPATH='' cd -- "$tools/../../.." && pwd)
assets=${SCENIC_ARTWORK_OUTPUT:-"$game/docs/assets/artwork"}
target=${CARGO_TARGET_DIR:-"$game/target"}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-towers.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$assets"

rustc --edition 2024 -O "$tools/export.rs" \
    --extern "scenic_flight=$target/release/libscenic_flight.rlib" \
    -L "dependency=$target/release/deps" -o "$scratch/export"
if [ "${1:-}" != "--landmarks-only" ]; then
    "$scratch/export" > "$scratch/towers.ppm"
    ffmpeg -v error -y -i "$scratch/towers.ppm" -frames:v 1 "$assets/towers.png"
fi
"$scratch/export" --landmark-names > "$assets/skyline-landmarks.txt"
for frame in 00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15; do
    angle=$(awk -v frame="$frame" 'BEGIN {print 0.706 + frame * 6.2831853 / 16}')
    "$scratch/export" --landmarks "$angle" > "$scratch/landmarks-$frame.ppm"
done
ffmpeg -v error -y -i "$scratch/landmarks-00.ppm" -frames:v 1 "$assets/skyline-landmarks.png"
ffmpeg -v error -y -framerate 8 -i "$scratch/landmarks-%02d.ppm" \
    -filter_complex_threads 1 \
    -filter_complex 'split[a][b];[a]palettegen=max_colors=112[p];[b][p]paletteuse=dither=bayer' \
    -loop 0 "$assets/skyline-landmarks.gif"
