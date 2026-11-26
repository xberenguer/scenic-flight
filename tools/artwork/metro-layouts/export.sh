#!/bin/sh
set -eu
tools=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
game=$(CDPATH='' cd -- "$tools/../../.." && pwd)
assets=${SCENIC_ARTWORK_OUTPUT:-"$game/docs/assets/artwork"}
target=${CARGO_TARGET_DIR:-"$game/target"}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-metro-layouts.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$assets"
rustc --edition 2024 -O "$tools/export.rs" \
    --extern "scenic_flight=$target/release/libscenic_flight.rlib" \
    -L "dependency=$target/release/deps" -o "$scratch/export"
"$scratch/export" "$scratch"
ffmpeg -v error -y -i "$scratch/metro-layouts.ppm" -frames:v 1 "$assets/metro-layouts.png"
cp "$scratch/metro-layouts.txt" "$assets/"
