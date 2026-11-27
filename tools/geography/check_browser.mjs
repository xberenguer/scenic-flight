import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { gunzipSync } from 'node:zlib';

const [wasmPath, packPath, output, ...captures] = process.argv.slice(2);
const { instance } = await WebAssembly.instantiate(await readFile(wasmPath));
const engine = instance.exports;
const packed = await readFile(packPath);
const data = packed[0] === 0x1f && packed[1] === 0x8b ? gunzipSync(packed) : packed;
const address = engine.scenic_vegas_buffer(data.byteLength);
assert(address);
new Uint8Array(engine.memory.buffer, address, data.byteLength).set(data);
const side = engine.scenic_side();
const leadInFrames = Math.round(Number(process.env.SCENIC_CAPTURE_LEAD_IN_SECONDS ?? 0) / 0.034);
assert(Number.isInteger(leadInFrames) && leadInFrames >= 0);
const departures = data.readUInt32LE(60) === 1 ? [
  ['la-downtown', 0, 2, -2270, -1245],
  ['la-dodger-stadium', 1, 3, 1260, 3250],
  ['la-exposition-park', 2, 2, -4230, -4530],
  ['la-union-station', 3, 5, 2190, 1040],
  ...(data.readInt32LE(24) <= -12000 ? [
    ['la-griffith-observatory', 4, 3, -3300, 6940],
    ['la-hollywood', 5, 4, -7300, 5000],
    ['la-pasadena', 6, 2, 8640, 9590],
    ['la-inglewood', 7, 5, -9480, -11400],
  ] : []),
  ...(data.readInt32LE(24) <= -24000 ? [
    ['la-lax', 8, 5, -18150, -11620],
    ['la-santa-monica', 9, 3, -23400, -5900],
    ['la-venice-beach', 10, 2, -23090, -7640],
    ['la-getty-center', 11, 4, -21460, 1540],
  ] : []),
] : [
  ['vegas-strip', 0, 4, -52, -2370],
  ['vegas-airport', 1, 7, 2730, -4470],
  ['vegas-downtown', 2, 7, 2170, 5250],
  ['vegas-sphere', 3, 4, 770, 565],
  ['vegas-paris', 4, 5, -240, 135],
  ['vegas-high-roller', 5, 5, 815, 682],
  ['vegas-venetian', 6, 5, -310, 980],
  ['vegas-stadium', 7, 5, -510, -1890],
  ['vegas-fremont', 8, 5, 2140, 6180, 142],
];

function frame(time, buttons = 0) {
  const pointer = engine.scenic_frame(time, buttons);
  return new Uint16Array(engine.memory.buffer, pointer, side * side);
}

async function save(image, filename) {
  const rgb = Buffer.alloc(captureSide * captureSide * 3);
  for (let i = 0; i < captureCoverage.length; i++) {
    const px = image[captureSource[i]];
    const coverage = captureCoverage[i];
    rgb[i * 3] = 20 + ((px >>> 11) * 255 / 31 - 20) * coverage;
    rgb[i * 3 + 1] = 26 + ((px >>> 5 & 63) * 255 / 63 - 26) * coverage;
    rgb[i * 3 + 2] = 32 + ((px & 31) * 255 / 31 - 32) * coverage;
  }
  await writeFile(filename, Buffer.concat([Buffer.from(`P6\n${captureSide} ${captureSide}\n255\n`), rgb]));
}

const captureScale = Math.max(1, Math.round(432 / side));
const captureSide = side * captureScale;
const captureCoverage = new Float32Array(output ? captureSide * captureSide : 0);
const captureSource = new Uint32Array(captureCoverage.length);
if (output) {
  await mkdir(output, { recursive: true });
  const radius = captureSide / 2;
  for (let i = 0; i < captureCoverage.length; i++) {
    const x = i % captureSide;
    const y = Math.floor(i / captureSide);
    captureCoverage[i] = Math.max(0, Math.min(1, radius + 0.5 - Math.hypot(x + 0.5 - radius, y + 0.5 - radius)));
    captureSource[i] = Math.floor(y / captureScale) * side + Math.floor(x / captureScale);
  }
}
for (const [name, departure, aircraft, east, north, seed = 215] of departures) {
  const capture = output && (!captures.length || captures.includes(name));
  assert.equal(engine.scenic_reset_las_vegas(seed, 0, departure), 1);
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
  if (capture) {
    for (let i = 1; i <= leadInFrames; i++) frame(t + 136 + 34 * i);
    t += 34 * leadInFrames;
  }
  const began = performance.now();
  const initial = frame(t + 174).slice();
  const hash = createHash('sha256');
  for (let i = 1; i <= 240; i++) {
    const image = frame(t + 174 + i * 34);
    hash.update(new Uint8Array(image.buffer, image.byteOffset, image.byteLength));
    if (capture && i % 2 === 0) {
      const order = String(i / 2 - 1).padStart(3, '0');
      await save(image, join(output, `${name}-${order}.ppm`));
    }
  }
  const last = frame(t + 8401).slice();
  assert.notDeepEqual(last, initial);
  const travelled = Math.hypot(engine.scenic_vegas_view(0) - startX, engine.scenic_vegas_view(1) - startZ);
  assert(travelled > 40, name);
  console.log(`${name}\t${travelled.toFixed(1)} m\t${((performance.now() - began) / 240).toFixed(2)} ms/frame	${hash.digest('hex')}`);
}
