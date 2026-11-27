#!/bin/sh
set -eu

board_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
esp_toolchain=${ESP_RUST_TOOLCHAIN:-esp}
esp_python=${ESPTOOL_PYTHON:-python3}

cd "$board_dir"
export SCENIC_RENDER_SIDE=60
features=logical-112,regional-rail,shore-details,cooperative-sky,distant-skyline,distant-roads,distant-bridges,native-flight-overlay,terrain-opportunistic,ground-cover,small-raster-ray-budget

mkdir -p build
exec python3 "$board_dir/../tools/build-scratch.py" "$board_dir/build" 2048 sh -eu -c '
    rustup run "$1" cargo build --locked --release --features "$3"
    image="$SCENIC_BUILD_SCRATCH/scenic-flight-waveshare175c-app.bin"
    "$2" -m esptool --chip esp32s3 elf2image --flash_mode dio --flash_freq 80m --flash_size 32MB \
        -o "$image" "$CARGO_TARGET_DIR/xtensa-esp32s3-none-elf/release/scenic-flight-waveshare"
    mv "$image" build/scenic-flight-waveshare175c-app.bin
    shasum -a 256 build/scenic-flight-waveshare175c-app.bin
' scenic-board "$esp_toolchain" "$esp_python" "$features"
