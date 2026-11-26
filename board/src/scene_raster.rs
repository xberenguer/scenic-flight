//! Keep the round display and its overlays fixed while adapting scenery cost.

pub const FULL_SIDE: usize = 60;
pub const BALANCED_SIDE: usize = 54;
pub const REDUCED_SIDE: usize = 48;
pub const HEAVY_SIDE: usize = 42;
pub const MIN_SIDE: usize = 36;

const TARGET_US: u64 = 1_000_000 / 30;
const OVER_BUDGET_US: u64 = 37_000;
const HEAVY_FRAME_US: u64 = 43_000;
const OBSERVATION_CEILING_US: u64 = 100_000;
const DROP_FRAMES: u8 = 12;

#[derive(Clone, Copy)]
enum Quality {
    Full,
    Balanced,
    Reduced,
    Heavy,
    Minimum,
}

pub struct SceneRaster {
    average_us: u64,
    average_higher_us: u64,
    slow_frames: u8,
    spare_frames: u8,
    quality: Quality,
}

impl SceneRaster {
    pub const fn new() -> Self {
        Self::at(Quality::Full)
    }

    const fn at(quality: Quality) -> Self {
        Self { average_us: TARGET_US, average_higher_us: TARGET_US,
            slow_frames: 0, spare_frames: 0, quality }
    }

    pub const fn side(&self) -> usize {
        match self.quality {
            Quality::Full => FULL_SIDE,
            Quality::Balanced => BALANCED_SIDE,
            Quality::Reduced => REDUCED_SIDE,
            Quality::Heavy => HEAVY_SIDE,
            Quality::Minimum => MIN_SIDE,
        }
    }

    pub fn observe(&mut self, frame_us: u64, scenery_us: u64) {
        let frame_us = frame_us.min(OBSERVATION_CEILING_US);
        self.average_us = (7 * self.average_us + frame_us) / 8;
        let budget = match self.quality {
            Quality::Balanced => 38_500,
            Quality::Reduced => 39_000,
            Quality::Heavy => 37_500,
            _ => OVER_BUDGET_US,
        };
        self.slow_frames = if self.average_us > budget {
            self.slow_frames.saturating_add(1)
        } else {
            0
        };

        // One slow frame stays at the current resolution. Sustained heavy
        // work can skip the middle tier if it would already be over budget.
        if self.slow_frames >= DROP_FRAMES {
            match self.quality {
                Quality::Full if self.average_us <= HEAVY_FRAME_US => {
                    *self = Self::at(Quality::Balanced);
                    return;
                }
                Quality::Full | Quality::Balanced => {
                    *self = Self::at(Quality::Reduced);
                    return;
                }
                Quality::Reduced => {
                    *self = Self::at(Quality::Heavy);
                    return;
                }
                Quality::Heavy => {
                    *self = Self::at(Quality::Minimum);
                    return;
                }
                Quality::Minimum => {}
            }
        }

        let (higher, budget, frames) = match self.quality {
            Quality::Minimum => (Quality::Heavy, 34_500, 85),
            Quality::Heavy => (Quality::Reduced, 36_500, 60),
            Quality::Reduced => (Quality::Balanced, 37_750, 40),
            Quality::Balanced => (Quality::Full, 32_000, 60),
            Quality::Full => return,
        };
        // Only the scenery cost grows with the raster. Each step adds six
        // pixels per axis, so require spare time before increasing detail.
        let estimate = (frame_us + scenery_us.min(frame_us) * 3 / 16).min(OBSERVATION_CEILING_US);
        self.average_higher_us = (7 * self.average_higher_us + estimate) / 8;
        self.spare_frames = if self.average_higher_us < budget {
            self.spare_frames + 1
        } else {
            0
        };
        if self.spare_frames == frames {
            *self = Self::at(higher);
        }
    }
}

/// Use the second core when serial terrain threatens the frame budget.
#[cfg(feature = "urban-background")]
pub struct TerrainWorker {
    average_serial_us: u32,
    split: bool,
    retry_in: u8,
}

#[cfg(feature = "urban-background")]
impl TerrainWorker {
    const ENGAGE_US: u32 = 24_000;
    const RELEASE_US: u32 = 19_500;

    pub const fn new() -> Self {
        Self { average_serial_us: Self::RELEASE_US, split: false, retry_in: 0 }
    }

    pub fn wants_help(&self) -> bool {
        self.split && self.retry_in == 0
    }

    pub fn observe_serial(&mut self, elapsed_us: u64) {
        self.retry_in = self.retry_in.saturating_sub(1);
        self.observe(elapsed_us.min(50_000) as u32);
    }

    pub fn observe_split(&mut self, left_us: u32, right_us: u32, join_us: u32, background: bool) {
        // A missed handoff can be more expensive than a serial frame.
        // Let the survey worker advance for a few frames before trying again.
        if !background || join_us >= right_us {
            self.retry_in = 8;
        }
        self.observe(left_us.saturating_add(right_us).min(50_000));
    }

    fn observe(&mut self, estimated_serial_us: u32) {
        self.average_serial_us = (3 * self.average_serial_us + estimated_serial_us) / 4;
        self.split = if self.split {
            self.average_serial_us >= Self::RELEASE_US
        } else {
            self.average_serial_us >= Self::ENGAGE_US
        };
    }
}

const fn circle_insets<const SIDE: usize>() -> [usize; SIDE] {
    let mut insets = [0; SIDE];
    let mut row = 0;
    while row < SIDE {
        let dy = SIDE as i32 - (2 * row + 1) as i32;
        let mut col = 0;
        loop {
            let dx = SIDE as i32 - (2 * col + 1) as i32;
            if dx * dx + dy * dy <= (SIDE * SIDE) as i32 {
                break;
            }
            col += 1;
        }
        insets[row] = col;
        row += 1;
    }
    insets
}

const fn source_axis<const SIDE: usize>() -> [usize; FULL_SIDE] {
    let mut result = [0; FULL_SIDE];
    let mut i = 0;
    while i < FULL_SIDE {
        result[i] = (2 * i + 1) * SIDE / (2 * FULL_SIDE);
        i += 1;
    }
    result
}

/// Expand the compact frame backwards so no second frame buffer is needed.
/// Continue real scenery colors up to the circular edge before panel scanout.
pub fn expand_reduced(pixels: &mut [u16], side: usize) {
    const BALANCED_AXIS: [usize; FULL_SIDE] = source_axis::<BALANCED_SIDE>();
    const REDUCED_AXIS: [usize; FULL_SIDE] = source_axis::<REDUCED_SIDE>();
    const HEAVY_AXIS: [usize; FULL_SIDE] = source_axis::<HEAVY_SIDE>();
    const MIN_AXIS: [usize; FULL_SIDE] = source_axis::<MIN_SIDE>();
    const BALANCED_INSETS: [usize; BALANCED_SIDE] = circle_insets();
    const REDUCED_INSETS: [usize; REDUCED_SIDE] = circle_insets();
    const HEAVY_INSETS: [usize; HEAVY_SIDE] = circle_insets();
    const MIN_INSETS: [usize; MIN_SIDE] = circle_insets();
    match side {
        BALANCED_SIDE => expand(pixels, &BALANCED_AXIS, &BALANCED_INSETS),
        REDUCED_SIDE => expand(pixels, &REDUCED_AXIS, &REDUCED_INSETS),
        HEAVY_SIDE => expand(pixels, &HEAVY_AXIS, &HEAVY_INSETS),
        MIN_SIDE => expand(pixels, &MIN_AXIS, &MIN_INSETS),
        _ => unreachable!("unsupported compact scenery side"),
    }
}

fn expand<const SIDE: usize>(
    pixels: &mut [u16], source_axis: &[usize; FULL_SIDE], source_insets: &[usize; SIDE],
) {
    const INSETS: [usize; FULL_SIDE] = circle_insets();
    let mut row = [0; SIDE];
    for y in (0..FULL_SIDE).rev() {
        let sy = source_axis[y];
        let source = sy * SIDE;
        row.copy_from_slice(&pixels[source..source + SIDE]);
        let edge = source_insets[sy];
        let target = &mut pixels[y * FULL_SIDE..][..FULL_SIDE];
        let inset = INSETS[y];
        target[..inset].fill(0);
        target[FULL_SIDE - inset..].fill(0);
        for (pixel, &x) in target[inset..FULL_SIDE - inset]
            .iter_mut()
            .zip(&source_axis[inset..])
        {
            *pixel = row[x.clamp(edge, SIDE - 1 - edge)];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "urban-background")]
    #[test]
    fn worker_follows_sustained_render_cost_and_backs_off_after_a_missed_handoff() {
        let mut worker = TerrainWorker::new();
        worker.observe_serial(100_000);
        // A cold launch can be expensive, but the worker stops once the
        // measured cost falls below its release threshold.
        for _ in 0..12 {
            worker.observe_serial(14_000);
        }
        assert!(!worker.wants_help());
        for _ in 0..8 {
            worker.observe_serial(31_000);
        }
        assert!(worker.wants_help());
        worker.observe_split(13_000, 14_000, 3_000, true);
        assert!(worker.wants_help());
        worker.observe_split(13_000, 14_000, 14_000, false);
        for _ in 0..7 {
            worker.observe_serial(29_000);
            assert!(!worker.wants_help());
        }
        worker.observe_serial(29_000);
        assert!(worker.wants_help());
    }

    #[test]
    fn ordinary_city_load_uses_the_middle_tier_before_reducing_again() {
        let mut raster = SceneRaster::new();
        raster.observe(920_000, 12_000);
        for _ in 0..40 {
            raster.observe(29_000, 12_000);
        }
        assert_eq!(raster.side(), FULL_SIDE);
        for _ in 0..26 {
            raster.observe(39_500, 25_000);
        }
        assert_eq!(raster.side(), BALANCED_SIDE);
        for _ in 0..120 {
            raster.observe(34_000, 20_000);
        }
        assert_eq!(raster.side(), BALANCED_SIDE);
        for _ in 0..30 {
            raster.observe(44_000, 25_000);
        }
        assert_eq!(raster.side(), REDUCED_SIDE);
    }

    #[test]
    fn heavy_load_can_skip_to_low_and_recovery_moves_one_tier_at_a_time() {
        let mut raster = SceneRaster::new();
        for _ in 0..22 {
            raster.observe(55_000, 33_000);
        }
        assert_eq!(raster.side(), REDUCED_SIDE);
        for _ in 0..39 {
            raster.observe(22_000, 14_000);
        }
        assert_eq!(raster.side(), REDUCED_SIDE);
        for _ in 0..15 {
            raster.observe(22_000, 14_000);
        }
        assert_eq!(raster.side(), BALANCED_SIDE);
        for _ in 0..70 {
            raster.observe(22_000, 14_000);
        }
        assert_eq!(raster.side(), FULL_SIDE);
    }

    #[test]
    fn in_place_expansion_matches_reference_and_fills_the_aperture() {
        check_expansion::<BALANCED_SIDE>();
        check_expansion::<REDUCED_SIDE>();
        check_expansion::<HEAVY_SIDE>();
        check_expansion::<MIN_SIDE>();
    }

    #[test]
    fn sustained_overload_steps_down_and_gradually_recovers() {
        let mut raster = SceneRaster::at(Quality::Reduced);
        for _ in 0..28 {
            raster.observe(44_000, 25_000);
        }
        assert_eq!(raster.side(), HEAVY_SIDE);
        for _ in 0..30 {
            raster.observe(29_000, 19_000);
        }
        assert_eq!(raster.side(), HEAVY_SIDE);
        for _ in 0..42 {
            raster.observe(29_000, 19_000);
        }
        assert_eq!(raster.side(), REDUCED_SIDE);
        for _ in 0..62 {
            raster.observe(44_000, 25_000);
        }
        assert_eq!(raster.side(), MIN_SIDE);
        for _ in 0..100 {
            raster.observe(28_000, 18_000);
        }
        assert_eq!(raster.side(), HEAVY_SIDE);
    }

    fn check_expansion<const SIDE: usize>() {
        let mut pixels = [u16::MAX; FULL_SIDE * FULL_SIDE];
        let low_insets: [usize; SIDE] = circle_insets();
        let full_insets: [usize; FULL_SIDE] = circle_insets();
        for (y, &edge) in low_insets.iter().enumerate() {
            for x in 0..SIDE {
                pixels[y * SIDE + x] = if (edge..SIDE - edge).contains(&x) {
                    (y * SIDE + x + 1) as u16
                } else {
                    0
                };
            }
        }
        let reference = pixels;
        expand_reduced(&mut pixels, SIDE);
        for (y, &edge) in full_insets.iter().enumerate() {
            for x in 0..FULL_SIDE {
                let value = pixels[y * FULL_SIDE + x];
                if (edge..FULL_SIDE - edge).contains(&x) {
                    let sy = ((2 * y + 1) * SIDE) / (2 * FULL_SIDE);
                    let sx = ((2 * x + 1) * SIDE) / (2 * FULL_SIDE);
                    let sx = sx.clamp(low_insets[sy], SIDE - 1 - low_insets[sy]);
                    assert_eq!(value, reference[sy * SIDE + sx]);
                    assert_ne!(value, 0);
                } else {
                    assert_eq!(value, 0);
                }
            }
        }
    }
}
