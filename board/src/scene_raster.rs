//! Keep the round display and its overlays fixed while adapting scenery cost.

pub const FULL_SIDE: usize = 60;
pub const REDUCED_SIDE: usize = 48;

const TARGET_US: u64 = 1_000_000 / 30;
const OVER_BUDGET_US: u64 = 37_000;
const RECOVERY_BUDGET_US: u64 = 32_000;
const OBSERVATION_CEILING_US: u64 = 100_000;
const DROP_FRAMES: u8 = 12;
const RECOVERY_FRAMES: u8 = 60;

pub struct SceneRaster {
    average_us: u64,
    qualifying_frames: u8,
    reduced: bool,
}

impl SceneRaster {
    pub const fn new() -> Self {
        Self {
            average_us: TARGET_US,
            qualifying_frames: 0,
            reduced: false,
        }
    }

    pub const fn side(&self) -> usize {
        if self.reduced {
            REDUCED_SIDE
        } else {
            FULL_SIDE
        }
    }

    pub fn observe(&mut self, frame_us: u64, scenery_us: u64) {
        // At 48px, estimate the extra full-resolution work before restoring it.
        // The fixed simulation, survey, aircraft and panel costs already appear
        // in the measured frame and are left out of the scaling estimate.
        // The board's scenery time grows more slowly than pixel area.
        let frame_us = frame_us.min(OBSERVATION_CEILING_US);
        let full_extra = if self.reduced {
            (scenery_us.min(frame_us) * 3) / 8
        } else {
            0
        };
        let measured = (frame_us + full_extra).min(OBSERVATION_CEILING_US);
        self.average_us = (7 * self.average_us + measured) / 8;
        let (ready, required) = if self.reduced {
            (self.average_us < RECOVERY_BUDGET_US, RECOVERY_FRAMES)
        } else {
            (self.average_us > OVER_BUDGET_US, DROP_FRAMES)
        };
        self.qualifying_frames = if ready { self.qualifying_frames + 1 } else { 0 };
        if self.qualifying_frames == required {
            self.reduced = !self.reduced;
            self.average_us = TARGET_US;
            self.qualifying_frames = 0;
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

const fn source_axis() -> [usize; FULL_SIDE] {
    let mut result = [0; FULL_SIDE];
    let mut i = 0;
    while i < FULL_SIDE {
        result[i] = (2 * i + 1) * REDUCED_SIDE / (2 * FULL_SIDE);
        i += 1;
    }
    result
}

/// Expand the compact frame backwards so no second frame buffer is needed.
/// Continue real scenery colors up to the circular edge before panel scanout.
pub fn expand_reduced(pixels: &mut [u16]) {
    const SOURCE: [usize; FULL_SIDE] = source_axis();
    const INSETS: [usize; FULL_SIDE] = circle_insets();
    const SOURCE_INSETS: [usize; REDUCED_SIDE] = circle_insets();
    let mut row = [0; REDUCED_SIDE];
    for y in (0..FULL_SIDE).rev() {
        let sy = SOURCE[y];
        let source = sy * REDUCED_SIDE;
        row.copy_from_slice(&pixels[source..source + REDUCED_SIDE]);
        let edge = SOURCE_INSETS[sy];
        let target = &mut pixels[y * FULL_SIDE..][..FULL_SIDE];
        let inset = INSETS[y];
        target[..inset].fill(0);
        target[FULL_SIDE - inset..].fill(0);
        for (pixel, &x) in target[inset..FULL_SIDE - inset]
            .iter_mut()
            .zip(&SOURCE[inset..])
        {
            *pixel = row[x.clamp(edge, REDUCED_SIDE - 1 - edge)];
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
    fn isolated_stall_never_changes_quality_and_sustained_work_does() {
        let mut raster = SceneRaster::new();
        raster.observe(920_000, 12_000);
        for _ in 0..40 {
            raster.observe(29_000, 12_000);
        }
        assert_eq!(raster.side(), FULL_SIDE);
        raster.observe(90_000, 12_000);
        for _ in 0..40 {
            raster.observe(29_000, 12_000);
        }
        assert_eq!(raster.side(), FULL_SIDE);
        for _ in 0..30 {
            raster.observe(44_000, 25_000);
        }
        assert_eq!(raster.side(), REDUCED_SIDE);
    }

    #[test]
    fn reduced_quality_recovers_only_after_sustained_headroom() {
        let mut raster = SceneRaster::new();
        for _ in 0..DROP_FRAMES * 2 {
            raster.observe(44_000, 25_000);
        }
        for _ in 0..120 {
            raster.observe(28_000, 17_000);
        }
        assert_eq!(raster.side(), REDUCED_SIDE);
        for _ in 0..RECOVERY_FRAMES - 1 {
            raster.observe(24_200, 17_960);
        }
        assert_eq!(raster.side(), REDUCED_SIDE);
        for _ in 0..30 {
            raster.observe(24_200, 17_960);
        }
        assert_eq!(raster.side(), FULL_SIDE);
    }

    #[test]
    fn in_place_expansion_matches_reference_and_fills_the_aperture() {
        let mut pixels = [u16::MAX; FULL_SIDE * FULL_SIDE];
        let low_insets: [usize; REDUCED_SIDE] = circle_insets();
        let full_insets: [usize; FULL_SIDE] = circle_insets();
        for (y, &edge) in low_insets.iter().enumerate() {
            for x in 0..REDUCED_SIDE {
                pixels[y * REDUCED_SIDE + x] = if (edge..REDUCED_SIDE - edge).contains(&x) {
                    (y * REDUCED_SIDE + x + 1) as u16
                } else {
                    0
                };
            }
        }
        let reference = pixels;
        expand_reduced(&mut pixels);
        for (y, &edge) in full_insets.iter().enumerate() {
            for x in 0..FULL_SIDE {
                let value = pixels[y * FULL_SIDE + x];
                if (edge..FULL_SIDE - edge).contains(&x) {
                    let sy = ((2 * y + 1) * REDUCED_SIDE) / (2 * FULL_SIDE);
                    let sx = ((2 * x + 1) * REDUCED_SIDE) / (2 * FULL_SIDE);
                    let sx = sx.clamp(low_insets[sy], REDUCED_SIDE - 1 - low_insets[sy]);
                    assert_eq!(value, reference[sy * REDUCED_SIDE + sx]);
                    assert_ne!(value, 0);
                } else {
                    assert_eq!(value, 0);
                }
            }
        }
    }
}
