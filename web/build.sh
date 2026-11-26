#!/bin/sh
set -eu

web=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
target="$web/target"
cargo build --manifest-path "$web/Cargo.toml" --locked --release \
    --target wasm32-unknown-unknown --target-dir "$target"
cp "$target/wasm32-unknown-unknown/release/scenic_flight_web.wasm" \
    "$web/../docs/assets/scenic-flight.wasm"
