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
    engine.scenic_reset(seed[0], seed[1]);
    runtime = { engine, context, image, target, palette: createPalette() };
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
