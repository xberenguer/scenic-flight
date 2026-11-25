#!/bin/sh
set -eu

board_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
esp_toolchain=${ESP_RUST_TOOLCHAIN:-esp}
esp_python=${ESPTOOL_PYTHON:-python3}

cd "$board_dir"
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$board_dir/target"}
export SCENIC_RENDER_SIDE=60
features=logical-112,regional-rail,shore-details,cooperative-sky,distant-skyline,distant-roads,distant-bridges,native-flight-overlay,terrain-opportunistic,ground-cover,small-raster-ray-budget

rustup run "$esp_toolchain" cargo build --locked --release --features "$features"
mkdir -p build
"$esp_python" -m esptool --chip esp32s3 elf2image --flash_mode dio --flash_freq 80m --flash_size 32MB \
  -o build/scenic-flight-waveshare175c-app.bin "$CARGO_TARGET_DIR/xtensa-esp32s3-none-elf/release/scenic-flight-waveshare"
shasum -a 256 build/scenic-flight-waveshare175c-app.bin
