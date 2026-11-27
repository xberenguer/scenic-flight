#!/bin/sh
set -eu
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
art=$1
target=$2
turntables=${3:-}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-flight-moments.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$art"
rustc --edition 2021 -O "$here/export.rs" \
    --extern "scenic_flight=$target/release/libscenic_flight.rlib" \
    -L "dependency=$target/release/deps" -o "$scratch/export"
"$scratch/export" "$scratch"
cp "$scratch/flight-moments.tsv" "$art/"

animate() {
    ffmpeg -hide_banner -loglevel error -y -framerate 12 -i "$scratch/$1-%03d.ppm" \
        -filter_complex_threads 1 \
        -filter_complex 'split[a][b];[a]palettegen=max_colors=96:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3' \
        -loop 0 "$2"
}

for id in 0 1 2 3 4 5 6 7; do
    animate "moments-$id" "$art/flight-moments-$id.gif"
    ffmpeg -hide_banner -loglevel error -y -i "$scratch/moments-$id-027.ppm" \
        -frames:v 1 "$art/flight-moments-$id.png"
done
if [ -n "$turntables" ]; then
    mkdir -p "$turntables"
    for id in 0 1 3; do
        animate "turntable-$id" "$turntables/turntable-$id.gif"
        ffmpeg -hide_banner -loglevel error -y -i "$scratch/turntable-$id-027.ppm" \
            -frames:v 1 "$turntables/turntable-$id.png"
    done
fi
