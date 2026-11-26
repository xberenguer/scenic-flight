#!/bin/sh
set -eu

tools=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
game=$(CDPATH='' cd -- "$tools/../../.." && pwd)
assets=${SCENIC_ARTWORK_OUTPUT:-"$game/docs/assets/artwork"}
target=${CARGO_TARGET_DIR:-"$game/target"}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-vessels.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$assets"

rustc --edition 2024 -O "$tools/export.rs" \
    --extern "scenic_flight=$target/release/libscenic_flight.rlib" \
    -L "dependency=$target/release/deps" -o "$scratch/export"
if [ "${1:-}" != "--service-only" ]; then
    "$scratch/export" "$assets/vessels.json" > "$scratch/vessels.ppm"
    ffmpeg -v error -y -i "$scratch/vessels.ppm" -frames:v 1 "$assets/vessels.png"
fi
for frame in 00 01 02 03 04 05 06 07 08 09 10 11; do
    angle=$(awk -v frame="$frame" 'BEGIN {print frame * 6.2831853 / 12}')
    "$scratch/export" "$assets/service-vessels.json" --service "$angle" > "$scratch/service-$frame.ppm"
done
ffmpeg -v error -y -i "$scratch/service-00.ppm" -frames:v 1 "$assets/service-vessels.png"
ffmpeg -v error -y -framerate 6 -i "$scratch/service-%02d.ppm" \
    -filter_complex_threads 1 \
    -filter_complex 'split[a][b];[a]palettegen=max_colors=112[p];[b][p]paletteuse=dither=bayer' \
    -loop 0 "$assets/service-vessels.gif"
