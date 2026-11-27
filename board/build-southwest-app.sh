#!/bin/sh
set -eu

board=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
cd "$board"
mkdir -p build

exec python3 ../tools/build-scratch.py "$board/build" 2048 sh -eu -c '
    assets=../docs/assets/geography
    gzip -dc "$assets/southwest-elevation.bin.gz" > "$SCENIC_BUILD_SCRATCH/southwest-elevation.bin"
    export SCENIC_ATLAS_OVERVIEW="$SCENIC_BUILD_SCRATCH/southwest-ground-overview.swc1"
    python3 ../tools/geography/pack_southwest_overview.py \
        "$SCENIC_BUILD_SCRATCH/southwest-elevation.bin" "$SCENIC_ATLAS_OVERVIEW"
    export SCENIC_ATLAS_ROADS="$SCENIC_BUILD_SCRATCH/southwest-roads.bin"
    export SCENIC_ATLAS_TOWNS="$SCENIC_BUILD_SCRATCH/southwest-towns.bin"
    gzip -dc "$assets/southwest-roads.bin.gz" > "$SCENIC_ATLAS_ROADS"
    gzip -dc "$assets/southwest-towns.bin.gz" > "$SCENIC_ATLAS_TOWNS"
    rustup run "$1" cargo build --locked --release \
        --features native-southwest-file,panel-worker,small-raster-ray-budget \
        --bin scenic-flight-vegas-waveshare
    image="$SCENIC_BUILD_SCRATCH/scenic-flight-waveshare175c-southwest-app.bin"
    "$2" -m esptool --chip esp32s3 elf2image --flash_mode dio --flash_freq 80m --flash_size 32MB \
        -o "$image" "$CARGO_TARGET_DIR/xtensa-esp32s3-none-elf/release/scenic-flight-vegas-waveshare"
    mv "$image" build/scenic-flight-waveshare175c-southwest-app.bin
    shasum -a 256 build/scenic-flight-waveshare175c-southwest-app.bin
' scenic-southwest "${ESP_RUST_TOOLCHAIN:-esp}" "${ESPTOOL_PYTHON:-python3}"
