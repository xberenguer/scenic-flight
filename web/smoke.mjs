import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";

const artifact = process.argv[2] ?? new URL(
  "./target/wasm32-unknown-unknown/release/scenic_flight_web.wasm",
  import.meta.url,
);
const { instance } = await WebAssembly.instantiate(await readFile(artifact), {});
const engine = instance.exports;
const side = engine.scenic_side();
assert.equal(side, 144);

const frame = (ms, buttons = 0) => {
  const address = engine.scenic_frame(ms, buttons);
  const pixels = new Uint16Array(engine.memory.buffer, address, side * side);
  assert.ok(pixels.some(pixel => pixel !== 0));
  return { address, pixels };
};

engine.scenic_reset(12, 34);
const picker = frame(0).pixels.slice();
assert.equal(engine.scenic_screen(), 0);
frame(100, 1);
const racer = frame(130, 1).pixels.slice();
assert.notDeepEqual(racer, picker);
frame(160);
frame(190, 2);
const launch = frame(220, 2);
assert.equal(engine.scenic_screen(), 1);
const firstFlight = launch.pixels.slice();
assert.notDeepEqual(firstFlight, racer);

const start = performance.now();
for (let i = 1; i <= 180; i++) {
  const next = frame(220 + Math.ceil(i * 1_000 / 30));
  assert.equal(launch.address, next.address, "the active framebuffer moved");
}
const msPerFrame = (performance.now() - start) / 180;
const airborne = frame(6_220).pixels.slice();
assert.notDeepEqual(airborne, firstFlight, "flight scene failed to advance");

frame(6_300, 1);
frame(6_330, 1);
const progress = frame(6_950, 1).pixels.slice();
assert.notDeepEqual(progress, airborne);
frame(7_500, 1);
assert.equal(engine.scenic_screen(), 0);
frame(7_560);
frame(7_610, 2);
const freshWorld = frame(7_640, 2).pixels.slice();
assert.equal(engine.scenic_screen(), 1);
assert.notDeepEqual(freshWorld, firstFlight);

engine.scenic_reset(12, 34);
frame(200);
frame(10_199);
assert.equal(engine.scenic_screen(), 0);
frame(10_200);
assert.equal(engine.scenic_screen(), 1);
console.log(`WASM smoke passed at ${side}×${side}; flight averaged ${msPerFrame.toFixed(2)} ms/frame in Node.`);
