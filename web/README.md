# Scenic Flight browser engine

Build and run the WebAssembly smoke check.

```sh
rustup target add wasm32-unknown-unknown
cargo build --locked --release --target wasm32-unknown-unknown
node smoke.mjs
```

The module exports its linear memory and four functions.

| Export | Contract |
| --- | --- |
| `scenic_reset(seed_low, seed_high)` | Reset to the picker with a 64-bit seed from two unsigned 32-bit words. |
| `scenic_frame(elapsed_ms, buttons)` | Advance the engine and return the byte address of the framebuffer. Buttons bit 0 is Small, bit 1 is Large. Pass increasing unsigned milliseconds with hidden time excluded. |
| `scenic_side()` | Frame side in pixels. The 144×144 raster is row-major little-endian RGB565. |
| `scenic_screen()` | Screen identifier. Picker is 0 and flight is 1. |

Recreate the framebuffer view from the current `memory.buffer` after calling
`scenic_frame`. WebAssembly memory can grow while preparing a fresh world.

The wrapper uses the same world, controls, aircraft, renderer and encounter art
as the Waveshare build. Its scenery scheduler provides terrain caches, trees,
ground cover, settlements, local roads and rail, coastal scenery, rivers and
natural landmarks. The browser variant uses synchronous terrain generation and
a single 144-pixel raster for terrain, aircraft and overlays. Regional highway
and rail connectors, the Waveshare-specific panel worker and native overlay
compositor are outside this wrapper.
