use std::cell::RefCell;

use scenic_flight::aircraft_art::{draw_flight_scene, draw_preview};
use scenic_flight::controls::{Buttons, Screen};
use scenic_flight::flight::{Game, TICKS_PER_SECOND};
use scenic_flight::ui::{draw_flight_info, draw_new_world, draw_picker};

mod scene;
use scene::Scene;

const SIDE: usize = 144;
const DIMENSIONS: (usize, usize) = (SIDE, SIDE);
const PIXELS: usize = SIDE * SIDE;
const MAX_FRAME_DELTA_MS: u32 = 250;

thread_local! {
    static ENGINE: RefCell<Option<Engine>> = const { RefCell::new(None) };
    static VEGAS_UPLOAD: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static VEGAS_DATA: std::cell::Cell<Option<&'static scenic_flight::LasVegasData>> = const { std::cell::Cell::new(None) };
}

struct Engine {
    game: Game,
    scene: Option<Box<Scene>>,
    pixels: Box<[u16]>,
    clock: Option<u32>,
    boot_picker: Option<u32>,
    picker_started: u32,
    tick_credit: u32,
    last_draw: Option<u32>,
}

impl Engine {
    fn new(seed: u64) -> Self {
        Self::with_game(Game::new(seed))
    }

    fn with_game(game: Game) -> Self {
        Self {
            game,
            scene: None,
            pixels: vec![0; PIXELS].into_boxed_slice(),
            clock: None,
            boot_picker: None,
            picker_started: 0,
            tick_credit: 0,
            last_draw: None,
        }
    }

    fn frame(&mut self, now: u32, inputs: u32) -> *const u16 {
        let elapsed = now.wrapping_sub(self.clock.replace(now).unwrap_or(now));
        let prior_screen = self.game.screen();
        let prior_aircraft = self.game.selected();
        let command = self.game.read_buttons(
            now,
            Buttons {
                small: inputs & 1 != 0,
                large: inputs & 2 != 0,
            },
        );
        if let Some(since) = self.boot_picker {
            self.game
                .launch_if_boot_idle(u64::from(now.wrapping_sub(since)));
        }
        let screen = self.game.screen();
        let transition = screen != prior_screen;
        if screen == Screen::Flight {
            if self.scene.is_none() {
                self.scene = Some(Scene::new(&mut self.game));
                self.tick_credit = 0;
            } else {
                self.tick_credit += elapsed.min(MAX_FRAME_DELTA_MS) * TICKS_PER_SECOND;
                while self.tick_credit >= 1_000 {
                    self.tick_credit -= 1_000;
                    self.scene.as_mut().unwrap().advance(&mut self.game);
                }
            }
        } else {
            self.scene = None;
            if transition || self.game.selected() != prior_aircraft || self.last_draw.is_none() {
                self.picker_started = now;
            }
        }

        // Poll inputs every call. Rendering at most once per simulation step
        // leaves high-refresh browsers time for other work.
        let paint_period = if screen == Screen::Flight {
            1_000 / TICKS_PER_SECOND
        } else {
            50
        };
        let paint_due = self
            .last_draw
            .is_none_or(|last| now.wrapping_sub(last) >= paint_period);
        if transition || command.is_some() || paint_due {
            self.last_draw = Some(now);
            if let Some(scene) = &mut self.scene {
                let (_, flight) = self.game.active().unwrap();
                let pose = flight.display_pose(self.tick_credit as f32 * 0.001);
                let sky = scene.draw(&pose, &mut self.pixels);
                draw_flight_scene(
                    &pose,
                    self.game.world_seed().unwrap(),
                    &sky,
                    &mut self.pixels,
                    SIDE,
                    SIDE,
                );
                if self.game.flight_info_visible() {
                    draw_flight_info(&pose, &mut self.pixels, SIDE, SIDE);
                }
                if let Some(progress) = self.game.new_world_progress(now) {
                    draw_new_world(progress, &mut self.pixels, SIDE, SIDE);
                }
            } else {
                self.boot_picker.get_or_insert(now);
                let selected = self.game.selected();
                let orbit = now.wrapping_sub(self.picker_started) / (1_000 / TICKS_PER_SECOND);
                draw_picker(selected, &mut self.pixels, SIDE, SIDE);
                draw_preview(selected, orbit, &mut self.pixels, SIDE, SIDE);
            }
        }
        self.pixels.as_ptr()
    }
}

/// Start a new browser session with a seed assembled from low and high 32-bit words.
#[no_mangle]
pub extern "C" fn scenic_reset(seed_low: u32, seed_high: u32) {
    let seed = u64::from(seed_low) | (u64::from(seed_high) << 32);
    ENGINE.with_borrow_mut(|engine| *engine = Some(Engine::new(seed)));
}

/// Prepare memory for the browser to upload a decompressed geographic pack.
#[no_mangle]
pub extern "C" fn scenic_vegas_buffer(length: u32) -> *mut u8 {
    if !(64..=64 * 1024 * 1024).contains(&length) { return std::ptr::null_mut(); }
    VEGAS_UPLOAD.with_borrow_mut(|buffer| {
        buffer.resize(length as usize, 0);
        buffer.as_mut_ptr()
    })
}

#[no_mangle]
pub extern "C" fn scenic_reset_las_vegas(seed_low: u32, seed_high: u32, departure: u32) -> u32 {
    let data = VEGAS_DATA.with(|slot| {
        if let Some(data) = slot.get() { return Some(data); }
        let bytes = VEGAS_UPLOAD.with_borrow_mut(std::mem::take).into_boxed_slice();
        let data = scenic_flight::LasVegasData::parse(Box::leak(bytes))
            .map(|layout| &*Box::leak(Box::new(layout)));
        slot.set(data);
        data
    });
    let Some(data) = data else { return 0; };
    let seed = u64::from(seed_low) | (u64::from(seed_high) << 32);
    ENGINE.with_borrow_mut(|engine| *engine = Some(Engine::with_game(Game::new_las_vegas(seed, data, departure))));
    1
}

#[no_mangle]
pub extern "C" fn scenic_vegas_view(component: u32) -> f32 {
    ENGINE.with_borrow(|engine| {
        let Some((_, flight)) = engine.as_ref().and_then(|engine| engine.game.active()) else { return 0.0; };
        match component {
            0 => flight.view.x,
            1 => flight.view.z,
            _ => flight.view.heading,
        }
    })
}

/// Return the byte address of the row-major RGB565 framebuffer in WebAssembly memory.
/// Its address is stable across frames until scenic_reset is called.
#[no_mangle]
pub extern "C" fn scenic_frame(elapsed_ms: u32, buttons: u32) -> *const u16 {
    ENGINE.with_borrow_mut(|engine| {
        engine
            .get_or_insert_with(|| Engine::new(0))
            .frame(elapsed_ms, buttons)
    })
}

#[no_mangle]
pub extern "C" fn scenic_side() -> u32 {
    SIDE as u32
}

#[no_mangle]
pub extern "C" fn scenic_screen() -> u32 {
    ENGINE.with_borrow(|engine| {
        u32::from(
            engine
                .as_ref()
                .is_some_and(|engine| engine.game.screen() == Screen::Flight),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scenic_flight::flight::Aircraft;

    #[test]
    fn manual_flight_and_new_world_use_the_native_game_controls() {
        let mut engine = Engine::new(12);
        let initial = engine.frame(0, 0);
        assert!(engine.pixels.iter().any(|&pixel| pixel != 0));
        engine.frame(100, 1);
        engine.frame(130, 1);
        assert_eq!(engine.game.selected(), Aircraft::Racer);
        engine.frame(160, 0);
        engine.frame(200, 2);
        assert_eq!(engine.frame(230, 2), initial);
        let seed = engine.game.world_seed().unwrap();
        let first_flight = engine.pixels.to_vec();
        engine.frame(260, 0);
        engine.frame(295, 0);
        assert_ne!(engine.pixels.as_ref(), first_flight);
        engine.frame(500, 1);
        engine.frame(530, 1);
        engine.frame(820, 1);
        assert!(engine.game.new_world_progress(820).unwrap() > 0.0);
        engine.frame(1_700, 1);
        assert_eq!(engine.game.screen(), Screen::AircraftPicker);
        assert_eq!(engine.game.selected(), Aircraft::Racer);
        engine.frame(1_800, 0);
        engine.frame(1_850, 2);
        engine.frame(1_880, 2);
        assert_ne!(engine.game.world_seed().unwrap(), seed);
    }

    #[test]
    fn untouched_picker_launches_after_ten_seconds_in_browser_time() {
        scenic_reset(12, 0);
        let first = scenic_frame(3_000, 0);
        assert_eq!(scenic_side(), SIDE as u32);
        assert_eq!(scenic_screen(), 0);
        scenic_frame(12_999, 0);
        assert_eq!(scenic_screen(), 0);
        assert_eq!(scenic_frame(13_000, 0), first);
        assert_eq!(scenic_screen(), 1);
    }
}
