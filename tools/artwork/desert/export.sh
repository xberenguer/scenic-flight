#!/bin/sh
set -eu

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
game=$(CDPATH='' cd -- "$here/../../.." && pwd)
out=${SCENIC_ARTWORK_OUTPUT:-"$here"}
mkdir -p "$out"
out=$(CDPATH='' cd -- "$out" && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-desert-art.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

# Include the production mesh file verbatim apart from its module-level doc line.
sed '1{/^\/\/!/d;}' "$game/src/desert_props.rs" > "$scratch/desert_props.rs"
sed -n '/^pub(crate) fn floor_i32(/,/^}/p' "$game/src/render.rs" > "$scratch/floor.rs"

cargo build --manifest-path "$game/Cargo.toml" --locked --release --lib \
    --features terrain-fork,small-raster-ray-budget,ground-cover
SCENIC_MATH_SOURCE="$game/src/math.rs" \
SCENIC_FLOOR_SOURCE="$scratch/floor.rs" \
SCENIC_DESERT_SOURCE="$scratch/desert_props.rs" \
rustc --edition=2021 -O "$here/export.rs" \
    --extern "scenic_flight=$game/target/release/libscenic_flight.rlib" \
    -L "dependency=$game/target/release/deps" \
    -o "$scratch/export"

"$scratch/export" "$scratch" > "$scratch/render-verification.txt"
for name in desert-rocks desert-bushes desert-cacti wetland-plants; do
    ffmpeg -hide_banner -loglevel error -y -i "$scratch/$name.ppm" -frames:v 1 "$scratch/$name.png"
    ffmpeg -hide_banner -loglevel error -y -i "$scratch/$name.png" -frames:v 1 "$scratch/$name.decoded.ppm"
    cmp "$scratch/$name.ppm" "$scratch/$name.decoded.ppm"
    cp "$scratch/$name.png" "$out/$name.png"
done
{
    printf '16 × 16 row-major tiles, 96 × 96 px each, #13181e background\n'
    printf 'Production mesh source SHA-256: '
    shasum -a 256 "$game/src/desert_props.rs" | cut -d ' ' -f 1
    printf 'World mesh renderer SHA-256: '
    shasum -a 256 "$game/src/world_mesh.rs" | cut -d ' ' -f 1
    cat "$scratch/render-verification.txt"
    cd "$out"
    shasum -a 256 desert-rocks.png desert-bushes.png desert-cacti.png wetland-plants.png
} > "$out/verification.txt"
cat "$out/verification.txt"
