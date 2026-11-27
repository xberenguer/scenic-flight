import assert from 'node:assert/strict';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { gunzipSync } from 'node:zlib';

const [wasmPath, packPath, output] = process.argv.slice(2);
const { instance } = await WebAssembly.instantiate(await readFile(wasmPath));
const engine = instance.exports;
const data = gunzipSync(await readFile(packPath));
const address = engine.scenic_vegas_buffer(data.byteLength);
assert(address);
new Uint8Array(engine.memory.buffer, address, data.byteLength).set(data);
const side = engine.scenic_side();
const departures = [
  ['vegas-strip', 0, 4, -52, -2370],
  ['vegas-airport', 1, 7, 2730, -4470],
  ['vegas-downtown', 2, 7, 2170, 5250],
  ['vegas-sphere', 3, 4, 500, 660],
];

function frame(time, buttons = 0) {
  const pointer = engine.scenic_frame(time, buttons);
  return new Uint16Array(engine.memory.buffer, pointer, side * side);
}

async function save(image, filename) {
  const rgb = Buffer.alloc(image.length * 3);
  for (let i = 0; i < image.length; i++) {
    const x = 2 * (i % side) + 1 - side;
    const y = 2 * Math.floor(i / side) + 1 - side;
    const px = image[i];
    rgb.set(x * x + y * y > side * side ? [20, 26, 32] :
      [(px >>> 11) * 255 / 31, (px >>> 5 & 63) * 255 / 63, (px & 31) * 255 / 31], i * 3);
  }
  await writeFile(filename, Buffer.concat([Buffer.from(`P6\n${side} ${side}\n255\n`), rgb]));
}

if (output) await mkdir(output, { recursive: true });
for (const [name, departure, aircraft, east, north] of departures) {
  assert.equal(engine.scenic_reset_las_vegas(215, 0, departure), 1);
  frame(0);
  let t = 0;
  for (let i = 0; i < aircraft; i++) {
    t += 100;
    frame(t, 1);
    frame(t + 35, 1);
    frame(t + 48);
    frame(t + 83);
  }
  frame(t + 100, 2);
  frame(t + 136, 2);
  assert.equal(engine.scenic_screen(), 1, name);
  const startX = engine.scenic_vegas_view(0);
  const startZ = engine.scenic_vegas_view(1);
  assert(Math.hypot(startX - east, startZ - north) < 100, name);
  const began = performance.now();
  const initial = frame(t + 174).slice();
  for (let i = 1; i <= 240; i++) {
    const image = frame(t + 174 + i * 34);
    if (output && i % 2 === 0) {
      const order = String(i / 2 - 1).padStart(3, '0');
      await save(image, join(output, `${name}-${order}.ppm`));
    }
  }
  const last = frame(t + 8401).slice();
  assert.notDeepEqual(last, initial);
  const travelled = Math.hypot(engine.scenic_vegas_view(0) - startX, engine.scenic_vegas_view(1) - startZ);
  assert(travelled > 40, name);
  console.log(`${name}\t${travelled.toFixed(1)} m\t${((performance.now() - began) / 240).toFixed(2)} ms/frame`);
}
