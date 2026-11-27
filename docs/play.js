const canvas = document.querySelector("#flight-canvas");
const notice = document.querySelector("#flight-notice");
const message = document.querySelector("#flight-message");
const retry = document.querySelector("#flight-retry");
const state = document.querySelector("#flight-state");
const smallLabel = document.querySelector("#flight-small-label");
const buttons = [
  document.querySelector("#flight-small"),
  document.querySelector("#flight-large"),
];

const FRAME_MS = 1000 / 30;
const DEBOUNCE_MS = 24;
const PICKER = 0;
const FLIGHT = 1;

class ButtonSignal {
  constructor(button) {
    this.button = button;
    this.sources = new Set();
    this.pending = [];
    this.pressed = false;
    this.settled = true;
    this.changedAt = 0;
  }

  set(source, down) {
    const wasHeld = this.sources.size > 0;
    if (down) this.sources.add(source);
    else this.sources.delete(source);
    const held = this.sources.size > 0;
    if (wasHeld === held) return;
    this.pending.push(held);
    this.button.dataset.held = String(held);
  }

  tap() {
    this.set("activation", true);
    this.set("activation", false);
  }

  release() {
    this.sources.clear();
    this.pending.length = 0;
    if (this.pressed) this.pending.push(false);
    this.button.dataset.held = "false";
  }

  sample(now) {
    if (this.settled && this.pending.length > 0) {
      this.pressed = this.pending.shift();
      this.changedAt = now;
      this.settled = false;
    } else if (!this.settled && now - this.changedAt >= DEBOUNCE_MS) {
      // Give the engine a stable reading before applying a queued release or press.
      this.settled = true;
    }
    return Number(this.pressed);
  }
}

const signals = buttons.map(button => new ButtonSignal(button));
const activeKeys = new Map();
const shortcuts = new Map([
  ["KeyA", 0], ["ArrowRight", 0],
  ["KeyF", 1], ["ArrowLeft", 1], ["Enter", 1],
]);

let runtime;
let frameRequest = null;
let lastRender = null;
let nextRender = null;
let elapsed = 0;
let screen = null;

for (const [index, button] of buttons.entries()) {
  const signal = signals[index];
  button.addEventListener("pointerdown", event => {
    if (button.disabled || event.button !== 0) return;
    button.setPointerCapture(event.pointerId);
    signal.set(`pointer:${event.pointerId}`, true);
  });
  for (const eventName of ["pointerup", "pointercancel", "lostpointercapture"]) {
    button.addEventListener(eventName, event => signal.set(`pointer:${event.pointerId}`, false));
  }
  button.addEventListener("click", event => {
    if (!button.disabled && event.detail === 0) signal.tap();
  });
}

window.addEventListener("keydown", event => {
  if (!runtime || event.altKey || event.ctrlKey || event.metaKey || event.defaultPrevented) return;
  const target = event.target instanceof Element ? event.target : null;
  if (target?.closest("input, textarea, select, [contenteditable]")) return;
  const control = target?.closest("[data-flight-button]");
  const activation = event.code === "Enter" || event.code === "Space";
  if (activation && !control && target?.closest("button, a")) return;
  const index = control && activation ? Number(control.dataset.flightButton) : shortcuts.get(event.code);
  if (index === undefined || buttons[index].disabled) return;
  event.preventDefault();
  if (event.repeat || activeKeys.has(event.code)) return;
  activeKeys.set(event.code, index);
  signals[index].set(`key:${event.code}`, true);
});

window.addEventListener("keyup", event => {
  const index = activeKeys.get(event.code);
  if (index === undefined) return;
  event.preventDefault();
  activeKeys.delete(event.code);
  signals[index].set(`key:${event.code}`, false);
});

function releaseInputs() {
  activeKeys.clear();
  for (const signal of signals) signal.release();
}

window.addEventListener("blur", releaseInputs);

document.addEventListener("visibilitychange", () => {
  if (document.hidden) {
    releaseInputs();
    stopLoop();
  } else {
    requestFrame();
  }
});

function requestFrame() {
  if (runtime && frameRequest === null && !document.hidden) {
    frameRequest = requestAnimationFrame(renderLoop);
  }
}

function stopLoop() {
  if (frameRequest !== null) cancelAnimationFrame(frameRequest);
  frameRequest = null;
  nextRender = null;
  lastRender = null;
}

function renderLoop(timestamp) {
  frameRequest = null;
  if (!runtime || document.hidden) return;
  try {
    if (nextRender === null || timestamp + 1 >= nextRender) {
      if (lastRender !== null) elapsed += timestamp - lastRender;
      lastRender = timestamp;
      nextRender = nextRender !== null && timestamp - nextRender < FRAME_MS
        ? nextRender + FRAME_MS
        : timestamp + FRAME_MS;
      drawFrame();
    }
    requestFrame();
  } catch {
    showError("Scenic Flight stopped. Try loading the game again.");
  }
}

function createPalette() {
  const colors = new Uint32Array(65536);
  const bytes = new Uint8Array(colors.buffer);
  for (let color = 0; color < colors.length; color++) {
    const red = color >> 11;
    const green = (color >> 5) & 63;
    const blue = color & 31;
    const offset = color * 4;
    bytes[offset] = (red << 3) | (red >> 2);
    bytes[offset + 1] = (green << 2) | (green >> 4);
    bytes[offset + 2] = (blue << 3) | (blue >> 2);
    bytes[offset + 3] = 255;
  }
  return colors;
}

function drawFrame() {
  const now = Math.floor(elapsed);
  const input = signals[0].sample(now) | (signals[1].sample(now) << 1);
  const { engine, context, image, target, palette } = runtime;
  const address = engine.scenic_frame(now >>> 0, input) >>> 0;
  const buffer = engine.memory.buffer;
  if (address === 0) throw new Error("Game frame unavailable");
  if (buffer !== runtime.buffer || address !== runtime.address) {
    runtime.source = new Uint16Array(buffer, address, target.length);
    runtime.buffer = buffer;
    runtime.address = address;
  }
  const source = runtime.source;
  for (let i = 0; i < target.length; i++) target[i] = palette[source[i]];
  context.putImageData(image, 0, 0);
  const current = engine.scenic_screen();
  if (current !== screen) updateControls(current);
  if (runtime.places && (runtime.placeCheck ?? 0) <= now) {
    runtime.placeCheck = now + 400;
    updateMappedLandmark(current, engine, runtime.places);
  }
  if (!notice.hidden) notice.hidden = true;
}

function updateControls(current) {
  if (current !== PICKER && current !== FLIGHT) throw new Error("Unknown game screen");
  screen = current;
  const flying = current === FLIGHT;
  smallLabel.textContent = flying ? "Flight info / new world" : "Next aircraft";
  buttons[0].disabled = false;
  buttons[1].disabled = flying;
  if (flying) signals[1].release();
  state.textContent = flying ? "In flight" : "Aircraft picker";
  state.hidden = false;
  if (!flying) document.querySelector("#vegas-landmark").hidden = true;
}

function showError(text) {
  stopLoop();
  releaseInputs();
  runtime = undefined;
  for (const button of buttons) button.disabled = true;
  state.hidden = true;
  notice.hidden = false;
  message.textContent = text;
  retry.hidden = false;
}

function flightSession(search) {
  const session = new URLSearchParams(search).get("session");
  if (session && /^\d{1,20}$/.test(session)) {
    const value = BigInt(session);
    if (value <= 0xffffffffffffffffn) {
      return [Number(value & 0xffffffffn), Number(value >> 32n)];
    }
  }
  return crypto.getRandomValues(new Uint32Array(2));
}

const destination = document.querySelector("#flight-destination");
const destinationUrl = new URL(location.href);
const cities = new Map([
  ["las-vegas", { name: "Las Vegas", mapId: 0 }],
  ["los-angeles", { name: "Los Angeles", mapId: 1 }],
  ["southwest", { name: "Los Angeles + Las Vegas", atlas: true }],
]);
const requestedCity = destinationUrl.searchParams.get("destination");
const city = cities.get(requestedCity);
destination.value = city ? requestedCity : "";
destination.addEventListener("change", () => {
  if (destination.value) destinationUrl.searchParams.set("destination", destination.value);
  else destinationUrl.searchParams.delete("destination");
  destinationUrl.searchParams.delete("departure");
  location.assign(destinationUrl);
});

const departure = document.querySelector("#vegas-departure");
const departureLabel = document.querySelector("#vegas-departure-label");
departureLabel.hidden = !city;
if (requestedCity === "los-angeles") {
  departure.replaceChildren(
    new Option("Downtown", ""),
    new Option("Dodger Stadium", "dodger-stadium"),
    new Option("Exposition Park", "exposition-park"),
    new Option("Union Station", "union-station"),
    new Option("Griffith Observatory", "griffith-observatory"),
    new Option("Hollywood", "hollywood"),
    new Option("Pasadena", "pasadena"),
    new Option("Inglewood", "inglewood"),
    new Option("Los Angeles International Airport", "lax"),
    new Option("Santa Monica Pier", "santa-monica"),
    new Option("Venice Beach", "venice-beach"),
    new Option("Getty Center", "getty-center"),
  );
} else if (city?.atlas) {
  departure.replaceChildren(
    new Option("Los Angeles", ""),
    new Option("Las Vegas Strip", "vegas-strip"),
    new Option("Los Angeles to Las Vegas", "to-las-vegas"),
    new Option("Las Vegas to Los Angeles", "to-los-angeles"),
  );
}
const departures = Array.from(departure.options, option => option.value);
if (city) {
  document.querySelector("h1").textContent = `Fly ${city.name}`;
  document.querySelector("#flight-help").textContent =
    "Choose an aircraft and select Fly. In flight, tap the right button for flight info or hold it to return to the picker.";
}
const requestedDeparture = destinationUrl.searchParams.get("departure");
departure.value = departures.includes(requestedDeparture) ? requestedDeparture : "";
departure.addEventListener("change", () => {
  if (departure.value) destinationUrl.searchParams.set("departure", departure.value);
  else destinationUrl.searchParams.delete("departure");
  location.assign(destinationUrl);
});

function updateMappedLandmark(current, engine, places) {
  const label = document.querySelector("#vegas-landmark");
  if (current !== FLIGHT) { label.hidden = true; return; }
  const x = engine.scenic_vegas_view(0);
  const z = engine.scenic_vegas_view(1);
  const heading = engine.scenic_vegas_view(2);
  const sin = Math.sin(heading);
  const cos = Math.cos(heading);
  let closest;
  let score = 1120;
  for (const place of places) {
    const east = place.x - x;
    const north = place.z - z;
    const along = east * sin + north * cos;
    const across = Math.abs(east * cos - north * sin);
    if (along < -35 || across > 155 + along * 0.85) continue;
    const distance = Math.hypot(east, north) + across * 0.52;
    if (distance < score) { closest = place; score = distance; }
  }
  label.hidden = !closest;
  if (closest && label.textContent !== closest.name) label.textContent = closest.name;
}

async function fetchPackedMap(path) {
  const pack = await fetch(path);
  if (!pack.ok || !pack.body) throw new Error("Map unavailable");
  return new Response(pack.body.pipeThrough(new DecompressionStream("gzip"))).arrayBuffer();
}

async function fetchCity(name, mapId) {
  const source = `./assets/geography/${name}`;
  const [data, catalog] = await Promise.all([
    fetchPackedMap(`${source}.bin.gz`),
    fetch(`${source}.json`),
  ]);
  if (!catalog.ok) throw new Error("Map catalog unavailable");
  if (data.byteLength < 64 || new DataView(data).getUint32(60, true) !== mapId) {
    throw new Error("Unexpected city in map pack");
  }
  const { places, origin_utm_m: origin } = await catalog.json();
  if (!Array.isArray(places)) throw new Error("Invalid landmark catalog");
  return { data, places, origin };
}

function uploadMap(engine, data) {
  const pointer = engine.scenic_vegas_buffer(data.byteLength) >>> 0;
  if (!pointer) throw new Error("Map exceeds the upload limit");
  new Uint8Array(engine.memory.buffer, pointer, data.byteLength).set(new Uint8Array(data));
}

async function loadMappedCity(engine, seed) {
  document.querySelector("#vegas-credit").hidden = false;
  if (city.atlas) {
    const [la, vegas, ground, roads, towns] = await Promise.all([
      fetchCity("los-angeles", 1), fetchCity("las-vegas", 0),
      fetchPackedMap("./assets/geography/southwest-elevation.bin.gz"),
      fetchPackedMap("./assets/geography/southwest-roads.bin.gz"),
      fetchPackedMap("./assets/geography/southwest-towns.bin.gz"),
    ]);
    for (const [id, map] of [[1, la], [0, vegas]]) {
      uploadMap(engine, map.data);
      if (!engine.scenic_commit_atlas_map(id)) throw new Error("Invalid atlas map");
    }
    uploadMap(engine, ground);
    if (!engine.scenic_commit_atlas_ground()) throw new Error("Invalid Southwest elevation map");
    uploadMap(engine, roads);
    if (!engine.scenic_commit_atlas_roads()) throw new Error("Invalid Southwest road map");
    uploadMap(engine, towns);
    if (!engine.scenic_commit_atlas_towns()) throw new Error("Invalid Southwest towns map");
    if (!engine.scenic_reset_atlas(seed[0], seed[1], departure.selectedIndex)) throw new Error("Invalid flight route");
    const [east, north] = vegas.origin.map((coordinate, axis) => coordinate - la.origin[axis]);
    return la.places.concat(vegas.places.map(place => ({ ...place, x: place.x + east, z: place.z + north })));
  }
  const { data, places } = await fetchCity(destination.value, city.mapId);
  uploadMap(engine, data);
  if (!engine.scenic_reset_las_vegas(seed[0], seed[1], departure.selectedIndex)) throw new Error("Invalid city map");
  return places;
}

async function start() {
  retry.hidden = true;
  notice.hidden = false;
  message.textContent = "Loading Scenic Flight…";
  if (typeof WebAssembly === "undefined") {
    showError("This browser cannot run Scenic Flight.");
    retry.hidden = true;
    return;
  }
  try {
    const response = await fetch("./assets/scenic-flight.wasm");
    if (!response.ok) throw new Error(`Game download returned HTTP ${response.status}`);
    const { instance } = await WebAssembly.instantiate(await response.arrayBuffer());
    const engine = instance.exports;
    const side = engine.scenic_side();
    if (!Number.isInteger(side) || side < 1 || side > 512) throw new Error("Invalid game display size");
    const context = canvas.getContext("2d", { alpha: false });
    if (!context) throw new Error("Canvas drawing unavailable");
    canvas.width = side;
    canvas.height = side;
    const image = context.createImageData(side, side);
    const target = new Uint32Array(image.data.buffer, image.data.byteOffset, side * side);
    const seed = flightSession(location.search);
    let places;
    if (city) {
      message.textContent = `Loading ${city.name} map…`;
      places = await loadMappedCity(engine, seed);
    } else {
      engine.scenic_reset(seed[0], seed[1]);
    }
    runtime = { engine, context, image, target, places, palette: createPalette() };
    elapsed = 0;
    screen = null;
    drawFrame();
    requestFrame();
  } catch {
    showError("Couldn’t load Scenic Flight. Check your connection and try again.");
  }
}

retry.addEventListener("click", start);
start();
