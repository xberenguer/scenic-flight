#!/bin/sh
set -eu

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
game=$(CDPATH='' cd -- "$here/../../.." && pwd)
target=${CARGO_TARGET_DIR:-"$game/target"}
out=${SCENIC_ARTWORK_OUTPUT:-"$here"}
mkdir -p "$out"
out=$(CDPATH='' cd -- "$out" && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/scenic-desert-art.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

# Include production meshes with paths and visibility adapted for this standalone exporter.
sed -e '1{/^\/\/!/d;}' -e 's/^mod habitats;$/#[path = "habitats.rs"]\
mod habitats;/' "$game/src/desert_props.rs" > "$scratch/desert_props.rs"
sed -e 's/^mod shore;$/#[path = "shore.rs"]\
pub(super) mod shore;/' \
    -e 's/^mod farm;$/#[path = "farm.rs"]\
pub(super) mod farm;/' \
    -e 's/^mod mountain;$/#[path = "mountain.rs"]\
pub(super) mod mountain;/' \
    -e 's/^mod freshwater;$/#[path = "freshwater.rs"]\
pub(super) mod freshwater;/' \
    -e 's/^mod woodland;$/#[path = "woodland.rs"]\
pub(super) mod woodland;/' \
    -e 's/^mod dryland;$/#[path = "dryland.rs"]\
pub(super) mod dryland;/' \
    -e 's/^pub(super) mod foothills;$/#[path = "foothills.rs"]\
pub(super) mod foothills;/' \
    -e 's/^mod grassland;$/#[path = "grassland.rs"]\
pub(super) mod grassland;/' \
    -e 's/^mod headland;$/#[path = "headland.rs"]\
pub(super) mod headland;/' \
    -e 's/^mod alpine_ground;$/#[path = "alpine_ground.rs"]\
pub(super) mod alpine_ground;/' \
    -e 's/^mod wetland;$/#[path = "wetland.rs"]\
pub(super) mod wetland;/' "$game/src/desert_props/habitats.rs" > "$scratch/habitats.rs"
sed 's/pub(in crate::desert_props) enum ShoreFeature/pub(crate) enum ShoreFeature/' \
    "$game/src/desert_props/habitats/shore.rs" > "$scratch/shore.rs"
sed 's/pub(in crate::desert_props) enum HeadlandFeature/pub(crate) enum HeadlandFeature/' \
    "$game/src/desert_props/habitats/headland.rs" > "$scratch/headland.rs"
cp "$game/src/desert_props/habitats/farm.rs" "$game/src/desert_props/habitats/mountain.rs" \
    "$game/src/desert_props/habitats/freshwater.rs" \
    "$game/src/desert_props/habitats/woodland.rs" \
    "$game/src/desert_props/habitats/dryland.rs" \
    "$game/src/desert_props/habitats/foothills.rs" \
    "$game/src/desert_props/habitats/grassland.rs" "$game/src/desert_props/habitats/alpine_ground.rs" \
    "$game/src/desert_props/habitats/wetland.rs" "$scratch/"
sed -n '/^pub(crate) fn floor_i32(/,/^}/p' "$game/src/render.rs" > "$scratch/floor.rs"
sed -n '/^pub(crate) fn bearing(/,/^}/p' "$game/src/flight.rs" > "$scratch/bearing.rs"

cargo build --manifest-path "$game/Cargo.toml" --locked --release --lib \
    --features terrain-fork,small-raster-ray-budget,ground-cover
SCENIC_MATH_SOURCE="$game/src/math.rs" \
SCENIC_FLOOR_SOURCE="$scratch/floor.rs" \
SCENIC_BEARING_SOURCE="$scratch/bearing.rs" \
SCENIC_DESERT_SOURCE="$scratch/desert_props.rs" \
rustc --edition=2021 -O "$here/export.rs" \
    --extern "scenic_flight=$target/release/libscenic_flight.rlib" \
    -L "dependency=$target/release/deps" \
    -o "$scratch/export"

render_atlas() {
    ffmpeg -hide_banner -loglevel error -y -i "$scratch/$1-00.ppm" -frames:v 1 "$out/$2.png"
    ffmpeg -hide_banner -loglevel error -y -framerate 6 -i "$scratch/$1-%02d.ppm" \
        -filter_complex_threads 1 \
        -filter_complex 'split[a][b];[a]palettegen=max_colors=112[p];[b][p]paletteuse=dither=bayer' \
        -loop 0 "$out/$2.gif"
}

case "${1:-}" in
    --shore-only)
        "$scratch/export" "$scratch" --shore-only
        render_atlas shore coastal-details
        exit
        ;;
    --farm-only)
        "$scratch/export" "$scratch" --farm-only
        render_atlas farm farm-details
        exit
        ;;
    --mountain-only)
        "$scratch/export" "$scratch" --mountain-only
        render_atlas mountain mountain-details
        exit
        ;;
    --freshwater-only)
        "$scratch/export" "$scratch" --freshwater-only
        render_atlas freshwater freshwater-details
        exit
        ;;
    --woodland-only)
        "$scratch/export" "$scratch" --woodland-only
        render_atlas woodland woodland-details
        exit
        ;;
    --dryland-only)
        "$scratch/export" "$scratch" --dryland-only
        render_atlas dryland dryland-details
        exit
        ;;
    --foothills-only)
        "$scratch/export" "$scratch" --foothills-only
        render_atlas foothills dry-foothill-models
        exit
        ;;
    --grassland-only)
        "$scratch/export" "$scratch" --grassland-only
        render_atlas grassland grassland-details
        exit
        ;;
    --headland-only)
        "$scratch/export" "$scratch" --headland-only
        render_atlas headland headland-details
        exit
        ;;
    --alpine-ground-only)
        "$scratch/export" "$scratch" --alpine-ground-only
        render_atlas alpine-ground alpine-ground-details
        exit
        ;;
    --wetland-only)
        "$scratch/export" "$scratch" --wetland-only
        render_atlas wetland floodplain-details
        exit
        ;;
esac
"$scratch/export" "$scratch" > "$scratch/render-verification.txt"
render_atlas shore coastal-details
render_atlas farm farm-details
render_atlas mountain mountain-details
render_atlas freshwater freshwater-details
render_atlas woodland woodland-details
render_atlas dryland dryland-details
render_atlas foothills dry-foothill-models
render_atlas grassland grassland-details
render_atlas headland headland-details
render_atlas alpine-ground alpine-ground-details
render_atlas wetland floodplain-details
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
