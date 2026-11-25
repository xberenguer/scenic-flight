//! Small sampled timing probe for the board's terrain composition pipeline.

use core::cell::Cell;

use crate::micros;

pub const CLOCK: usize = 0;
pub const TERRAIN: usize = 1;
pub const NATURAL: usize = 2;
pub const DETAILS: usize = 3;
pub const RIVER: usize = 4;
pub const FACILITIES: usize = 5;
const STAGES: usize = 6;

#[derive(Clone, Copy)]
struct Totals {
    samples: u32,
    micros: [u64; STAGES],
}

impl Totals {
    const EMPTY: Self = Self { samples: 0, micros: [0; STAGES] };
}

pub struct SampleProbe {
    totals: Cell<Totals>,
    calls: Cell<u32>,
    enabled: Cell<bool>,
}

impl SampleProbe {
    pub fn new() -> Self {
        Self { totals: Cell::new(Totals::EMPTY), calls: Cell::new(0), enabled: Cell::new(false) }
    }

    pub fn begin(&self) -> Option<SampleTimer<'_>> {
        if !self.enabled.get() {
            return None;
        }
        let index = self.calls.get();
        self.calls.set(index + 1);
        // Move the sample position each block so instrumentation visits different
        // columns even when a sequence of ray depths uses a fixed band count.
        (index & 31 == index >> 5 & 31).then(|| SampleTimer {
            probe: self,
            last: micros(),
            measures: [0; STAGES],
        })
    }

    pub fn enable(&self, enabled: bool) {
        self.enabled.set(enabled);
    }

    pub fn report(&self, frame: u32) {
        let calls = self.calls.replace(0);
        let Totals { samples, micros: [clock, terrain, natural, details, river, facilities] } =
            self.totals.replace(Totals::EMPTY);
        esp_println::println!(
            "sample frame={} calls={} samples={} clock_us={} terrain_us={} natural_us={} details_land_us={} river_us={} facilities_us={}",
            frame, calls, samples, clock, terrain, natural, details, river, facilities,
        );
    }
}

pub struct SampleTimer<'a> {
    probe: &'a SampleProbe,
    last: u64,
    measures: [u64; STAGES],
}

impl SampleTimer<'_> {
    #[inline(always)]
    pub fn mark(&mut self, stage: usize) {
        let now = micros();
        self.measures[stage] = now - self.last;
        self.last = now;
    }

    pub fn finish(self) {
        let mut totals = self.probe.totals.get();
        totals.samples += 1;
        for (total, measure) in totals.micros.iter_mut().zip(self.measures) {
            *total += measure;
        }
        self.probe.totals.set(totals);
    }
}
