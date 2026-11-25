//! A cooperative producer of terrain-independent sky colors for the display core.

use core::cell::{Cell, RefCell};
use core::sync::atomic::{AtomicU32, Ordering};

use critical_section::Mutex;
use scenic_flight::{
    render::{Camera, SkyTileRaster},
    sky::Sky,
};

use crate::LOGICAL_SIDE;

const WORDS: usize = SkyTileRaster::square_tile_capacity(LOGICAL_SIDE).div_ceil(2);
const INTERNAL_WORDS: usize = if LOGICAL_SIDE <= 192 {
    WORDS
} else {
    0
};

#[derive(Clone, Copy)]
struct Request {
    generation: u32,
    camera: Camera,
    sky: Sky,
}

static MAILBOX: Mutex<RefCell<Option<Request>>> = Mutex::new(RefCell::new(None));
static REQUESTED: AtomicU32 = AtomicU32::new(0);
static READY_GENERATION: AtomicU32 = AtomicU32::new(0);
static READY_COLORS: AtomicU32 = AtomicU32::new(0);
static INTERNAL_COLORS: [AtomicU32; INTERNAL_WORDS] = [const { AtomicU32::new(0) }; INTERNAL_WORDS];
static COLORS: Mutex<Cell<&'static [AtomicU32]>> = Mutex::new(Cell::new(&INTERNAL_COLORS));

#[cfg(feature = "transport-stream")]
pub fn initialize(memory: &mut crate::scene_memory::SceneMemory) {
    if INTERNAL_WORDS == 0 {
        let colors = memory.place_atomic_words(WORDS);
        critical_section::with(|cs| COLORS.borrow(cs).set(colors));
    }
}

fn colors() -> &'static [AtomicU32] {
    critical_section::with(|cs| COLORS.borrow(cs).get())
}

/// Read side. The display owner requests only one generation per full frame.
pub struct PreparedSky {
    generation: u32,
    colors: &'static [AtomicU32],
    prefix: Option<usize>,
    pub reused: u16,
    pub fallback: u16,
}

impl PreparedSky {
    pub fn color(&mut self, index: usize) -> Option<u16> {
        let prefix = *self.prefix.get_or_insert_with(|| {
            if READY_GENERATION.load(Ordering::Acquire) == self.generation {
                READY_COLORS.load(Ordering::Acquire) as usize
            } else {
                0
            }
        });
        if index >= prefix {
            self.fallback += 1;
            return None;
        }
        // The worker's Release count publishes fully written words. It can
        // write additional words concurrently, never this ready prefix.
        let word = self.colors[index / 2].load(Ordering::Relaxed);
        self.reused += 1;
        Some((word >> ((index & 1) * 16)) as u16)
    }
}

pub fn request(camera: Camera, sky: Sky) -> PreparedSky {
    let colors = colors();
    if colors.is_empty() {
        return PreparedSky { generation: 0, colors, prefix: Some(0), reused: 0, fallback: 0 };
    }
    let generation = REQUESTED.load(Ordering::Relaxed).wrapping_add(1);
    critical_section::with(|cs| {
        *MAILBOX.borrow_ref_mut(cs) = Some(Request {
            generation,
            camera,
            sky,
        });
    });
    REQUESTED.store(generation, Ordering::Release);
    PreparedSky {
        generation,
        colors,
        prefix: None,
        reused: 0,
        fallback: 0,
    }
}

pub struct SkyWorker {
    generation: u32,
    colors: &'static [AtomicU32],
    raster: Option<SkyTileRaster>,
    count: usize,
    odd: Option<u16>,
}

impl SkyWorker {
    pub fn new() -> Self {
        Self {
            generation: 0,
            colors: colors(),
            raster: None,
            count: 0,
            odd: None,
        }
    }

    /// Starts one request, shades one group or publishes one tail. Panel
    /// service always gets another chance before any further work.
    pub fn poll(&mut self) -> bool {
        if REQUESTED.load(Ordering::Acquire) != self.generation {
            let request = critical_section::with(|cs| MAILBOX.borrow_ref_mut(cs).take());
            if let Some(request) = request {
                self.generation = request.generation;
                self.raster =
                    SkyTileRaster::new(request.camera, &request.sky, (LOGICAL_SIDE, LOGICAL_SIDE))
                        .ok();
                self.count = 0;
                self.odd = None;
                READY_COLORS.store(0, Ordering::Release);
                READY_GENERATION.store(self.generation, Ordering::Release);
                return true;
            }
        }

        let Some(raster) = self.raster.as_mut() else {
            return false;
        };
        match raster.next() {
            Some(color) => {
                if let Some(first) = self.odd.take() {
                    self.publish(u32::from(first) | (u32::from(color) << 16), 2);
                } else {
                    self.odd = Some(color);
                }
            }
            None => {
                self.raster = None;
                if let Some(last) = self.odd.take() {
                    self.publish(u32::from(last), 1);
                }
            }
        }
        true
    }

    fn publish(&mut self, word: u32, colors: usize) {
        self.colors[self.count / 2].store(word, Ordering::Relaxed);
        self.count += colors;
        READY_COLORS.store(self.count as u32, Ordering::Release);
    }
}
