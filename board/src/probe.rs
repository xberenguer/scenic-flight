//! Deterministic bench input. Compiled only into the separate scene probe.

use scenic_flight::{controls::Buttons, flight::{Aircraft, Flight, Game}};

#[cfg(feature = "coastal-fixture")]
use scenic_flight::flight::TICKS_PER_SECOND;
use scenic_flight::render::Camera;

#[cfg(feature = "coastal-fixture")]
const COASTAL_WORLD_SEED: u64 = 4_329;
#[cfg(feature = "coastal-fixture")]
const COASTAL_LAST_TICK: u64 = 149;
#[cfg(feature = "cascade-fixture")]
const CASCADE_WORLD_SEED: u64 = 3;

pub fn initial_game() -> Game {
    let (seed, aircraft) = if cfg!(feature = "coastal-fixture") {
        // This input produces the surveyed world after the ordinary menu launch.
        (0x3d1b_fc8c_f68a_1dc8, 0)
    } else if cfg!(feature = "cascade-fixture") {
        // The usual Game seed mixer yields world 3, with the tarn in tile (4, -3).
        (0x8692_f695_0ca2_a279, 0)
    } else if cfg!(any(feature = "caprock-fixture", feature = "outwash-fixture")) {
        (0xab70_d03b_6e80_6156, 0)
    } else {
        let seed = option_env!("SCENIC_PROFILE_SEED")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0x51ce_00d1_34b7_26a9);
        let aircraft = option_env!("SCENIC_PROFILE_AIRCRAFT")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0)
            .min(Aircraft::ALL.len() as u32 - 1);
        (seed, aircraft)
    };
    let mut game = Game::new(seed);
    if cfg!(feature = "lineup-probe") {
        return game;
    }
    for choice in 0..aircraft {
        let time = choice * 80;
        let small = Buttons { small: true, large: false };
        let released = Buttons::default();
        game.read_buttons(time, small);
        game.read_buttons(time + 30, small);
        game.read_buttons(time + 40, released);
        game.read_buttons(time + 70, released);
    }
    let press = Buttons {
        small: false,
        large: true,
    };
    game.read_buttons(aircraft * 80, press);
    game.read_buttons(aircraft * 80 + 30, press);
    if let Some(ticks) = option_env!("SCENIC_PROFILE_PREFLIGHT_TICKS")
        .and_then(|value| value.parse::<u32>().ok())
    {
        for tick in 1..=ticks {
            game.tick();
            if tick % 6_000 == 0 || tick == ticks {
                navigation("preflight", game.world_seed().unwrap(), game.active().unwrap().1);
            }
        }
    }
    game
}

#[cfg(not(feature = "lineup-probe"))]
pub fn input(physical: Buttons, elapsed_ms: u64) -> Buttons {
    if cfg!(any(feature = "coastal-fixture", feature = "cascade-fixture", feature = "caprock-fixture", feature = "outwash-fixture"))
        || option_env!("SCENIC_PROFILE_HOLD_FLIGHT") == Some("1")
    {
        return physical;
    }
    Buttons {
        small: physical.small
            || (8_000..8_180).contains(&elapsed_ms)    // Tap for flight data.
            || (10_000..11_480).contains(&elapsed_ms) // Hold through return to picker.
            || (12_200..12_380).contains(&elapsed_ms), // Browse to Racer.
        large: physical.large || (13_200..13_380).contains(&elapsed_ms),
    }
}

#[cfg(feature = "lineup-probe")]
#[derive(Default)]
pub struct Lineup {
    picker_since: Option<u64>,
    has_flown: bool,
}

#[cfg(feature = "lineup-probe")]
impl Lineup {
    pub fn input(&mut self, physical: Buttons, game: &Game, now_ms: u64) -> Buttons {
        if let Some((_, flight)) = game.active() {
            self.picker_since = None;
            self.has_flown = true;
            return Buttons {
                // Let three full timing windows finish, then hold New World.
                small: physical.small
                    || (flight.ticks >= 250 && Some(&flight.aircraft) != Aircraft::ALL.last()),
                large: physical.large,
            };
        }
        let elapsed = now_ms - *self.picker_since.get_or_insert(now_ms);
        Buttons {
            small: physical.small || (self.has_flown && (120..500).contains(&elapsed)),
            large: physical.large || elapsed >= 720,
        }
    }
}

#[cfg(feature = "lineup-probe")]
pub fn report_aircraft(world: &scenic_flight::world::World, flight: &Flight) {
    let ground = world.elevation(flight.x, flight.z);
    esp_println::println!(
        "lineup aircraft={} tick={} altitude={:.2} clearance={:.2} speed={:.2} recoveries={}",
        flight.aircraft as u8, flight.ticks, flight.altitude, flight.altitude - ground,
        flight.speed(), flight.terrain_recoveries,
    );
}

#[cfg(feature = "caprock-fixture")]
pub fn caprock_camera(seed: u64) -> Option<Camera> {
    (seed == 8).then_some(Camera {
        x: -4314.0884,
        z: -4518.721,
        altitude: 63.125362,
        heading: -1.7361945,
        pitch: 0.43911266,
        bank: 0.035,
    })
}

#[cfg(feature = "outwash-fixture")]
pub fn outwash_camera(seed: u64) -> Option<Camera> {
    (seed == 8).then_some(Camera {
        x: 1469.1619,
        z: 1407.6356,
        altitude: 100.218124,
        heading: -1.7880973,
        pitch: 0.43911266,
        bank: 0.035,
    })
}

pub fn report_launch(seed: u64, camera: Camera) {
    esp_println::println!(
        "launch seed={:016x} x_bits={:08x} z_bits={:08x} altitude_bits={:08x} heading_bits={:08x}",
        seed,
        camera.x.to_bits(),
        camera.z.to_bits(),
        camera.altitude.to_bits(),
        camera.heading.to_bits(),
    );
}

/// An optional surveyed view for features that do not require scripted motion.
pub fn fixed_camera() -> Option<Camera> {
    let mut fields = option_env!("SCENIC_PROFILE_FIXED_CAMERA")?.split(',');
    let mut number = || fields.next()
        .and_then(|field| field.trim().parse::<f32>().ok())
        .filter(|field| field.is_finite())
        .expect("fixed camera needs six finite numbers");
    let camera = Camera {
        x: number(),
        z: number(),
        altitude: number(),
        heading: number(),
        pitch: number(),
        bank: number(),
    };
    assert!(fields.next().is_none(), "fixed camera needs exactly six numbers");
    Some(camera)
}

pub fn fixed_day() -> Option<f32> {
    let value = option_env!("SCENIC_PROFILE_DAY_FRACTION")?.parse::<f32>().expect("invalid day fraction");
    assert!((0.0..1.0).contains(&value));
    Some(value)
}

/// Trace the actual aircraft rather than the interpolated chase camera.
pub fn navigation(phase: &str, seed: u64, flight: &Flight) {
    if option_env!("SCENIC_PROFILE_PREFLIGHT_TICKS").is_none() {
        return;
    }
    let reheat = match flight.aircraft {
        Aircraft::Blackbird => scenic_flight::encounters::blackbird_reheat(seed, flight.ticks),
        Aircraft::Fighter => scenic_flight::encounters::fighter_reheat(seed, flight.ticks),
        _ => 0.0,
    };
    esp_println::println!(
        "navigation phase={} tick={} x_bits={:08x} z_bits={:08x} altitude_bits={:08x} heading_bits={:08x} turn_bits={:08x} bank_bits={:08x} camera_bank_bits={:08x} chase_lift={:.2} reheat={:.3} refuel={:.3} scenic_leg={} recoveries={}",
        phase,
        flight.ticks,
        flight.x.to_bits(),
        flight.z.to_bits(),
        flight.altitude.to_bits(),
        flight.heading.to_bits(),
        flight.turn_rate().to_bits(),
        flight.bank.to_bits(),
        flight.view.bank.to_bits(),
        flight.view.altitude - flight.altitude,
        reheat,
        flight.refueling_progress().unwrap_or(-1.0),
        flight.following_scenic_leg(),
        flight.terrain_recoveries,
    );
}

/// A five-second look up the rocky slope, with the aircraft beside the seep.
#[cfg(feature = "cascade-fixture")]
pub fn cascade_camera(seed: u64, ticks: u64) -> Option<Camera> {
    if seed != CASCADE_WORLD_SEED {
        return None;
    }
    const LAST_TICK: u64 = 149;
    let progress = ticks.min(LAST_TICK) as f32 * (1.0 / LAST_TICK as f32);
    let distance = 52.0 - progress * 30.0;
    let altitude = 128.0 - progress * 12.5;
    let slope = (altitude - 98.821) / distance;
    // Center the atan series around 0.5, keeping its argument below 0.20.
    let angle = (slope - 0.5) / (1.0 + slope * 0.5);
    let square = angle * angle;
    let pitch = 0.463_647_6 + angle * (1.0 + square * (-1.0 / 3.0 + square * (1.0 / 5.0 - square / 7.0)));
    let setback = core::f32::consts::FRAC_1_SQRT_2 * distance;
    Some(Camera {
        x: 1662.8743 + setback,
        z: -1047.1257 + setback,
        altitude,
        heading: 3.686_991,
        pitch: pitch - 0.095,
        bank: 0.045,
    })
}

/// A fixed flyby of the town-to-town coastal link, then a hold for inspection.
/// Pass zero to stage the real scene at the first camera before playback.
#[cfg(feature = "coastal-fixture")]
pub fn coastal_camera(seed: u64, ticks: u64) -> Option<Camera> {
    if seed != COASTAL_WORLD_SEED {
        return None;
    }
    if option_env!("SCENIC_PROFILE_FAR_BRIDGE") == Some("1") {
        return Some(Camera {
            x: -6197.212,
            z: -8892.682,
            altitude: 40.0,
            heading: 4.31969,
            pitch: 0.10,
            bank: 0.035,
        });
    }
    const HEADING: f32 = 1.832_595_8;
    const SIN: f32 = 0.965_925_8;
    const COS: f32 = -0.258_819_16;
    const ALTITUDE: f32 = 22.0;
    const CENTER_X: f32 = -6797.734;
    const CENTER_Z: f32 = -9141.426;
    let elapsed = ticks.min(COASTAL_LAST_TICK) as f32 / TICKS_PER_SECOND as f32;
    let distance = 135.0 - elapsed * 16.9;
    let slope = (ALTITUDE - 0.9) / distance;
    let square = slope * slope;
    // atan on this fixture's short approach to the inlet.
    let pitch = slope * (1.0 + square * (-1.0 / 3.0 + square * (1.0 / 5.0
        + square * (-1.0 / 7.0 + square * (1.0 / 9.0
        + square * (-1.0 / 11.0 + square / 13.0))))));
    Some(Camera {
        x: CENTER_X - SIN * distance,
        z: CENTER_Z - COS * distance,
        altitude: ALTITUDE,
        heading: HEADING,
        pitch,
        bank: 0.0,
    })
}

/// Start the real scenery planning from the region pair's survey view.
#[cfg(feature = "coastal-fixture")]
pub fn coastal_preflight_camera(seed: u64) -> Option<Camera> {
    if seed != COASTAL_WORLD_SEED {
        return None;
    }
    Some(Camera {
        x: -6706.7876,
        z: -9505.119,
        altitude: 335.0,
        heading: -0.143_583_4,
        pitch: 0.53,
        bank: 0.0,
    })
}

#[cfg(feature = "coastal-fixture")]
pub fn report_coastal(seed: u64, ticks: u64, frame: u32, camera: Camera, visible_links: usize, visible_bridges: usize) {
    if seed != COASTAL_WORLD_SEED || !matches!(ticks, 0 | 30 | 60 | 90 | 120 | COASTAL_LAST_TICK) {
        return;
    }
    esp_println::println!(
        "coastal_fixture frame={} tick={} seed={:016x} x_bits={:08x} z_bits={:08x} altitude_bits={:08x} heading_bits={:08x} pitch_bits={:08x} bank_bits={:08x} visible_links={} visible_bridges={}",
        frame,
        ticks,
        seed,
        camera.x.to_bits(),
        camera.z.to_bits(),
        camera.altitude.to_bits(),
        camera.heading.to_bits(),
        camera.pitch.to_bits(),
        camera.bank.to_bits(),
        visible_links,
        visible_bridges,
    );
}

pub fn capture(frame: u32, pixels: &[u16], side: usize) {
    if option_env!("SCENIC_PROFILE_CAPTURE").and_then(|value| value.parse().ok()) != Some(frame) {
        return;
    }
    emit_pixels(frame, pixels, side);
}

fn emit_pixels(frame: u32, pixels: &[u16], side: usize) {
    let mut row_hex = [0_u8; crate::LOGICAL_SIDE * 4];
    esp_println::println!("capture frame={} size={}", frame, side);
    for (row, pixels) in pixels.chunks_exact(side).enumerate() {
        let text = rgb565_hex(pixels, &mut row_hex[..side * 4]);
        esp_println::println!("capture row={} {}", row, text);
    }
    esp_println::println!("capture end");
}

fn rgb565_hex<'a>(pixels: &[u16], row_hex: &'a mut [u8]) -> &'a str {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (pixel, text) in pixels.iter().zip(row_hex.chunks_exact_mut(4)) {
        for (digit, character) in text.iter_mut().enumerate() {
            *character = HEX[(usize::from(*pixel) >> ((3 - digit) * 4)) & 15];
        }
    }
    core::str::from_utf8(row_hex).unwrap()
}

#[cfg(feature = "native-airframe-probe")]
pub fn native_capture_frame() -> Option<u32> {
    option_env!("SCENIC_PROFILE_NATIVE_CAPTURE").map(|value| {
        value.parse::<u32>().expect("invalid native packed capture frame")
    })
}

#[cfg(feature = "native-airframe-probe")]
pub fn emit_native_aircraft(frame: u32, pixels: &[u16]) {
    use crate::packed_airframe::{CAPTURE_HEIGHT, CAPTURE_WIDTH, CAPTURE_X, CAPTURE_Y};
    let mut row_hex = [0_u8; CAPTURE_WIDTH * 4];
    esp_println::println!(
        "native capture frame={} x={} y={} width={} height={}",
        frame, CAPTURE_X, CAPTURE_Y, CAPTURE_WIDTH, CAPTURE_HEIGHT,
    );
    for (row, pixels) in pixels.chunks_exact(CAPTURE_WIDTH).enumerate() {
        let text = rgb565_hex(pixels, &mut row_hex);
        esp_println::println!("native capture row={} {}", row, text);
    }
    esp_println::println!("native capture end");
}

#[cfg(feature = "burst-capture")]
mod burst {
    use crate::{LOGICAL_SIDE, frame_fingerprint, scene_memory::SceneMemory};

    const FRAMES: usize = 72;
    const PIXELS: usize = LOGICAL_SIDE * LOGICAL_SIDE;

    #[derive(Clone, Copy, Default)]
    struct Time {
        frame: u32,
        tick: u64,
        micros: u64,
    }

    /// Capture only by memory copy during flight. Serial output begins after
    /// the last recorded image has been handed to the display worker.
    pub struct Burst {
        images: &'static mut [u16],
        times: [Time; FRAMES],
        first_frame: u32,
        recorded: usize,
    }

    impl Burst {
        pub fn new(memory: &mut SceneMemory) -> Self {
            let first_frame = option_env!("SCENIC_PROFILE_BURST_START")
                .and_then(|value| value.parse().ok())
                .unwrap_or(60);
            Self {
                images: memory.place_pixels(FRAMES * PIXELS),
                times: [Time::default(); FRAMES],
                first_frame,
                recorded: 0,
            }
        }

        pub fn record(&mut self, frame: u32, tick: u64, micros: u64, image: &[u16]) -> bool {
            if frame < self.first_frame || self.recorded == FRAMES {
                return false;
            }
            let start = self.recorded * PIXELS;
            self.images[start..start + PIXELS].copy_from_slice(image);
            self.times[self.recorded] = Time { frame, tick, micros };
            self.recorded += 1;
            self.recorded == FRAMES
        }

        pub fn emit(&self) {
            esp_println::println!("burst begin count={} size={}", FRAMES, LOGICAL_SIDE);
            for (time, image) in self.times.iter().zip(self.images.chunks_exact(PIXELS)) {
                esp_println::println!(
                    "burst frame={} tick={} us={} rgb565={:08x}",
                    time.frame, time.tick, time.micros, frame_fingerprint(image),
                );
                super::emit_pixels(time.frame, image, LOGICAL_SIDE);
            }
            esp_println::println!("burst end");
        }
    }
}

#[cfg(feature = "burst-capture")]
pub use burst::Burst;
