#!/bin/sh
set -eu

web=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
assets="$web/../docs/assets"
exec python3 "$web/../tools/build-scratch.py" "$assets" 768 sh -eu -c '
    cargo build --manifest-path "$1/Cargo.toml" --locked --release \
        --target wasm32-unknown-unknown
    cp "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/scenic_flight_web.wasm" \
        "$SCENIC_BUILD_SCRATCH/scenic-flight.wasm"
    mv "$SCENIC_BUILD_SCRATCH/scenic-flight.wasm" "$2/scenic-flight.wasm"
' scenic-web "$web" "$assets"
