//! Prepared tarns and alpine meadows from actual terrain depressions.

use crate::math::mix;
use crate::waterfall::Waterfall;
use crate::world::{Sample, Surface, World};

mod shorelines;
pub use shorelines::TarnShoreline;

pub const ALPINE_TILE_SIZE: i32 = 384;
const COARSE: usize = 32;
const CELLS: usize = COARSE * COARSE;
const FINE: usize = COARSE * 2 - 1;
const MAX_PROOF: usize = 980;
const PREVIEW: usize = 16;
const STEP: f32 = 3.0;
const SPACING: f32 = STEP * 0.5;
const QUANTUM: f32 = 64.0;
const CANDIDATES: usize = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlpineSurface {
    Tarn,
    TarnShallows,
    Meadow,
    OutletGravel,
    Shore(Surface),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlpineSample {
    pub elevation: f32,
    pub surface: AlpineSurface,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlpineOutlet {
    /// Actual sampled ground points in x, height, z order.
    pub crest: [f32; 3],
    pub downstream: [f32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlpineLake {
    origin: [f32; 2],
    level: f32,
    floor: [f32; 3],
    wet: [u64; FINE],
    meadow: [u64; FINE],
    outlet: Option<AlpineOutlet>,
    waterfall: Option<Waterfall>,
    shoreline: TarnShoreline,
    shore_center: [f32; 2],
    shore_axis: [f32; 2],
    shore_inverse: [f32; 2],
}

impl AlpineLake {
    pub fn generate(world: &World, seed: u64, tile_x: i32, tile_z: i32) -> Option<Self> {
        Self::generate_cooperatively(world, seed, tile_x, tile_z, &mut || {})
    }

    pub(crate) fn generate_cooperatively(
        world: &World,
        seed: u64,
        tile_x: i32,
        tile_z: i32,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        pulse();
        if !(-4096..=4096).contains(&tile_x) || !(-4096..=4096).contains(&tile_z) {
            return None;
        }
        let tile = (u64::from(tile_x as u32) << 32) | u64::from(tile_z as u32);
        let roll = mix(seed ^ tile ^ 0x05b3_6d82_4417_a1da);
        if roll & 7 >= 5 {
            return None;
        }
        let base = [
            (tile_x * ALPINE_TILE_SIZE) as f32,
            (tile_z * ALPINE_TILE_SIZE) as f32,
        ];
        let proposals = scout(world, base, roll, pulse);
        let mut work = Work::new();
        for (_, local_x, local_z) in proposals {
            if local_x == 0 {
                break;
            }
            let origin = [
                base[0] + f32::from(local_x) - 48.0,
                base[1] + f32::from(local_z) - 48.0,
            ];
            let shore_key = mix(roll ^ (u64::from(local_x) << 32) ^ u64::from(local_z));
            let shoreline = TarnShoreline::choose(shore_key);
            if let Some(mut lake) = work.prepare(world, origin, shoreline, pulse) {
                lake.waterfall = Waterfall::survey(world, &lake, seed, pulse);
                return Some(lake);
            }
        }
        None
    }

    pub fn shoreline(&self) -> TarnShoreline {
        self.shoreline
    }

    fn shore_coordinates(&self, x: f32, z: f32) -> [f32; 2] {
        let [dx, dz] = [x - self.shore_center[0], z - self.shore_center[1]];
        let [ux, uz] = self.shore_axis;
        [
            (dx * ux + dz * uz) * self.shore_inverse[0],
            (dz * ux - dx * uz) * self.shore_inverse[1],
        ]
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    /// The lowest measured point in the wet footprint, in x, height, z order.
    pub fn floor(&self) -> [f32; 3] {
        self.floor
    }

    pub fn outlet(&self) -> Option<AlpineOutlet> {
        self.outlet
    }

    pub fn waterfall(&self) -> Option<&Waterfall> {
        self.waterfall.as_ref()
    }

    /// Minimum x, minimum z, maximum x, maximum z, including meadow benches.
    pub fn bounds(&self) -> [f32; 4] {
        let edge = (FINE - 1) as f32 * SPACING;
        [
            self.origin[0],
            self.origin[1],
            self.origin[0] + edge,
            self.origin[1] + edge,
        ]
    }

    /// Area of the connected survey footprint in square world units.
    pub fn surface_area(&self) -> f32 {
        count(&self.wet) as f32 * SPACING * SPACING
    }

    /// Check the lake and exposed cascade against other prepared scenery.
    pub fn conflicts_with(&self, mut occupied: impl FnMut(f32, f32) -> bool) -> bool {
        for (z, &row) in self.wet.iter().enumerate() {
            let mut nodes = row;
            while nodes != 0 {
                let x = nodes.trailing_zeros() as usize;
                nodes &= nodes - 1;
                let [x, z] = point(self.origin, x, z, SPACING);
                for (dx, dz) in [(0.0, 0.0), (-0.6, 0.0), (0.6, 0.0), (0.0, -0.6), (0.0, 0.6)] {
                    if occupied(x + dx, z + dz) {
                        return true;
                    }
                }
            }
        }
        self.waterfall
            .as_ref()
            .is_some_and(|fall| fall.conflicts_with(occupied))
    }

    /// Sample the prepared surface against the supplied underlying terrain.
    /// Only water changes elevation; vegetation and outlet gravel follow the land.
    pub fn sample(&self, x: f32, z: f32, ground: Sample) -> Option<AlpineSample> {
        let px = (x - self.origin[0]) / SPACING;
        let pz = (z - self.origin[1]) / SPACING;
        let extent = (FINE - 1) as f32;
        if !(0.0..extent).contains(&px)
            || !(0.0..extent).contains(&pz)
            || !matches!(
                ground.surface,
                Surface::Grass | Surface::Forest | Surface::Rock | Surface::Snow
            )
        {
            return None;
        }
        let [ix, iz] = [px as usize, pz as usize];
        let [u, v] = [px - ix as f32, pz - iz as f32];
        let depth = self.level - ground.elevation;
        if depth > 0.0 && weight(&self.wet, ix, iz, u, v) >= 0.48 {
            let surface = if depth
                < self
                    .shoreline
                    .spec()
                    .shallow_depth(self.shore_coordinates(x, z))
            {
                AlpineSurface::TarnShallows
            } else {
                AlpineSurface::Tarn
            };
            return Some(AlpineSample {
                elevation: self.level,
                surface,
            });
        }
        if let Some(outlet) = self.outlet {
            if (-2.8..=0.95).contains(&(ground.elevation - self.level)) && outlet_near(outlet, x, z)
            {
                return Some(AlpineSample {
                    elevation: ground.elevation,
                    surface: AlpineSurface::OutletGravel,
                });
            }
        }
        if ground.surface != Surface::Snow
            && (-6.8..=-0.13).contains(&depth)
            && weight(&self.meadow, ix, iz, u, v) >= 0.55
        {
            let (surface, relief) = self
                .shoreline
                .spec()
                .bench(self.shore_coordinates(x, z), -depth);
            let under_cascade = self.waterfall.as_ref().is_some_and(|fall| {
                let [lo, hi] = fall.bounds();
                (lo[0] - 2.0..=hi[0] + 2.0).contains(&x) && (lo[2] - 2.0..=hi[2] + 2.0).contains(&z)
            });
            return Some(if under_cascade {
                AlpineSample {
                    elevation: ground.elevation,
                    surface: AlpineSurface::Meadow,
                }
            } else {
                AlpineSample {
                    elevation: ground.elevation + relief,
                    surface: if surface == Surface::AlpineMeadow {
                        AlpineSurface::Meadow
                    } else {
                        AlpineSurface::Shore(surface)
                    },
                }
            });
        }
        None
    }
}

fn scout(
    world: &World,
    base: [f32; 2],
    roll: u64,
    pulse: &mut impl FnMut(),
) -> [(f32, u16, u16); CANDIDATES] {
    let phase_x = 9 + ((roll >> 16) & 7) as u16;
    let phase_z = 9 + ((roll >> 24) & 7) as u16;
    let node = |x: usize, z: usize| [phase_x + x as u16 * 24, phase_z + z as u16 * 24];
    let mut heights = [[0.0; PREVIEW]; PREVIEW];
    for (z, row) in heights.iter_mut().enumerate() {
        for (x, elevation) in row.iter_mut().enumerate() {
            if x & 3 == 0 {
                pulse();
            }
            let [x, z] = node(x, z);
            *elevation = world.elevation(base[0] + f32::from(x), base[1] + f32::from(z));
        }
    }
    let mut best = [(f32::NEG_INFINITY, 0, 0); CANDIDATES];
    for z in 2..PREVIEW - 2 {
        pulse();
        for x in 2..PREVIEW - 2 {
            let ground = heights[z][x];
            if ground < 65.0 {
                continue;
            }
            let mut higher = 0;
            let mut relief = 0.0_f32;
            let mut peak = 0.0_f32;
            for dz in z - 1..=z + 1 {
                for dx in x - 1..=x + 1 {
                    let h = heights[dz][dx] - ground;
                    higher += usize::from(h > 0.3);
                    relief += h.max(0.0).min(15.0);
                    peak = peak.max(h);
                }
            }
            if higher < 5 || peak < 5.0 {
                continue;
            }
            let weight = higher as f32 * 3.3 + relief * 0.17;
            if let Some(i) = best.iter().position(|entry| weight > entry.0) {
                for j in (i + 1..CANDIDATES).rev() {
                    best[j] = best[j - 1];
                }
                let [u, v] = node(x, z);
                best[i] = (weight, u, v);
            }
        }
    }
    best
}

struct Work {
    bed: [u16; CELLS],
    // The fine flood aborts before it can exceed the coarse queue's capacity.
    queue: [u16; CELLS],
    parents: [u16; CELLS],
}

impl Work {
    const fn new() -> Self {
        Self {
            bed: [0; CELLS],
            queue: [0; CELLS],
            parents: [u16::MAX; CELLS],
        }
    }

    fn prepare(
        &mut self,
        world: &World,
        origin: [f32; 2],
        shoreline: TarnShoreline,
        pulse: &mut impl FnMut(),
    ) -> Option<AlpineLake> {
        for (i, h) in self.bed.iter_mut().enumerate() {
            if i & 3 == 0 {
                pulse();
            }
            let [x, z] = point(origin, i % COARSE, i / COARSE, STEP);
            *h = (world.elevation(x, z) * QUANTUM + 0.5) as u16;
        }
        let mut floor = 15 * COARSE + 15;
        for z in 10..=22 {
            for x in 10..=22 {
                let i = z * COARSE + x;
                if self.bed[i] < self.bed[floor] {
                    floor = i;
                }
            }
        }
        let floor_q = self.bed[floor];
        if !(66.0..=150.0).contains(&(f32::from(floor_q) / QUANTUM)) {
            return None;
        }
        let mut low = floor_q + 110;
        let mut high = floor_q + 835;
        if self.coarse_flood(floor, low, false, pulse).1.is_some() {
            return None;
        }
        let open = self.coarse_flood(floor, high, false, pulse).1.is_some();
        if open {
            while high - low > 1 {
                let threshold = low + (high - low) / 2;
                if self
                    .coarse_flood(floor, threshold, false, pulse)
                    .1
                    .is_some()
                {
                    high = threshold;
                } else {
                    low = threshold;
                }
            }
        }
        let desired = if open {
            (high - 31).min(floor_q + 512)
        } else {
            floor_q + 435
        };
        let anchor = [floor % COARSE * 2, floor / COARSE * 2];
        let [x, z] = point(origin, anchor[0], anchor[1], SPACING);
        let floor_height = world.elevation(x, z);
        let preferred = if open { 0 } else { shoreline.spec().drawdown };
        let drawdowns = [preferred, 0, 22, 45];
        for (i, drawdown) in drawdowns.into_iter().enumerate() {
            if drawdowns[..i].contains(&drawdown) {
                continue;
            }
            let level = (i32::from(desired) - i32::from(drawdown)) as f32 / QUANTUM;
            if level - floor_height < 1.50 {
                continue;
            }
            let (proof, _, leaks) =
                self.fine_flood(world, origin, anchor, level + 0.09, true, pulse);
            if leaks || !(60..=MAX_PROOF).contains(&count(&proof)) {
                continue;
            }
            let (wet, lowest, leaks) =
                self.fine_flood(world, origin, anchor, level - 0.015, false, pulse);
            if leaks
                || !(56..=920).contains(&count(&wet))
                || !strong_rim(world, origin, anchor, level, pulse)
            {
                continue;
            }
            let meadow = meadow_benches(world, origin, level, wet, pulse);
            if count(&meadow) < 23 {
                continue;
            }
            let outlet = if open && f32::from(high) / QUANTUM - level < 0.88 {
                self.outlet(world, origin, floor, high, level, pulse)
            } else {
                None
            };
            let (shore_center, shore_axis, shore_inverse) = shore_frame(origin, &wet, outlet);
            return Some(AlpineLake {
                origin,
                level,
                floor: lowest,
                wet,
                meadow,
                outlet,
                waterfall: None,
                shoreline,
                shore_center,
                shore_axis,
                shore_inverse,
            });
        }
        None
    }

    fn coarse_flood(
        &mut self,
        floor: usize,
        cap: u16,
        trace: bool,
        pulse: &mut impl FnMut(),
    ) -> (usize, Option<usize>) {
        let mut visited = [0_u32; COARSE];
        visited[floor / COARSE] |= 1 << (floor % COARSE);
        self.queue[0] = floor as u16;
        if trace {
            self.parents[floor] = u16::MAX;
        }
        let (mut read, mut write) = (0, 1);
        while read < write {
            if read & 31 == 0 {
                pulse();
            }
            let i = usize::from(self.queue[read]);
            read += 1;
            let [x, z] = [i % COARSE, i / COARSE];
            if x == 0 || z == 0 || x == COARSE - 1 || z == COARSE - 1 {
                return (write, Some(i));
            }
            for nz in z - 1..=z + 1 {
                for nx in x - 1..=x + 1 {
                    let bit = 1 << nx;
                    let next = nz * COARSE + nx;
                    if visited[nz] & bit == 0 && self.bed[next] <= cap {
                        visited[nz] |= bit;
                        if trace {
                            self.parents[next] = i as u16;
                        }
                        self.queue[write] = next as u16;
                        write += 1;
                    }
                }
            }
        }
        (write, None)
    }

    fn fine_flood(
        &mut self,
        world: &World,
        origin: [f32; 2],
        anchor: [usize; 2],
        level: f32,
        diagonals: bool,
        pulse: &mut impl FnMut(),
    ) -> ([u64; FINE], [f32; 3], bool) {
        let mut seen = [0_u64; FINE];
        let mut wet = [0_u64; FINE];
        let [x, z] = anchor;
        let [px, pz] = point(origin, x, z, SPACING);
        let mut lowest = [px, world.elevation(px, pz), pz];
        self.queue[0] = (z * FINE + x) as u16;
        seen[z] |= 1 << x;
        wet[z] |= 1 << x;
        let (mut read, mut write) = (0, 1);
        while read < write {
            pulse();
            let i = usize::from(self.queue[read]);
            read += 1;
            let [x, z] = [i % FINE, i / FINE];
            if x <= 1 || z <= 1 || x >= FINE - 2 || z >= FINE - 2 || write > MAX_PROOF {
                return (wet, lowest, true);
            }
            for nz in z - 1..=z + 1 {
                for nx in x - 1..=x + 1 {
                    if !diagonals && nx != x && nz != z {
                        continue;
                    }
                    let bit = 1 << nx;
                    if seen[nz] & bit == 0 {
                        seen[nz] |= bit;
                        let [x, z] = point(origin, nx, nz, SPACING);
                        let height = world.elevation(x, z);
                        if height < level {
                            if height < lowest[1] {
                                lowest = [x, height, z];
                            }
                            wet[nz] |= bit;
                            self.queue[write] = (nz * FINE + nx) as u16;
                            write += 1;
                        }
                    }
                }
            }
        }
        (wet, lowest, false)
    }

    fn outlet(
        &mut self,
        world: &World,
        origin: [f32; 2],
        floor: usize,
        spill: u16,
        level: f32,
        pulse: &mut impl FnMut(),
    ) -> Option<AlpineOutlet> {
        let mut current = self.coarse_flood(floor, spill, true, pulse).1?;
        let mut after = current;
        let mut sill = None;
        while self.parents[current] != u16::MAX {
            pulse();
            if self.bed[current] >= spill.saturating_sub(4) {
                sill = Some((current, after));
            }
            after = current;
            current = usize::from(self.parents[current]);
        }
        let (crest, end) = sill?;
        if crest == end {
            return None;
        }
        let sample = |i| {
            let [x, z] = point(origin, i % COARSE, i / COARSE, STEP);
            [x, world.elevation(x, z), z]
        };
        let crest = sample(crest);
        let downstream = sample(end);
        if crest[1] - downstream[1] < 0.17 || crest[1] - level > 0.90 {
            return None;
        }
        Some(AlpineOutlet { crest, downstream })
    }
}

fn strong_rim(
    world: &World,
    origin: [f32; 2],
    anchor: [usize; 2],
    level: f32,
    pulse: &mut impl FnMut(),
) -> bool {
    pulse();
    let [x, z] = point(origin, anchor[0], anchor[1], SPACING);
    let mut arms = 0;
    let mut high = 0;
    for [dx, dz] in [
        [38.0, 0.0],
        [27.0, 27.0],
        [0.0, 38.0],
        [-27.0, 27.0],
        [-38.0, 0.0],
        [-27.0, -27.0],
        [0.0, -38.0],
        [27.0, -27.0],
    ] {
        let relief = world.elevation(x + dx, z + dz) - level;
        arms += usize::from(relief >= 1.4);
        high += usize::from(relief >= 4.2);
    }
    arms >= 6 && high >= 3
}

fn meadow_benches(
    world: &World,
    origin: [f32; 2],
    level: f32,
    lake: [u64; FINE],
    pulse: &mut impl FnMut(),
) -> [u64; FINE] {
    let mut reach = lake;
    for _ in 0..7 {
        pulse();
        let prior = reach;
        for z in 1..FINE - 1 {
            reach[z] |= prior[z] << 1 | prior[z] >> 1 | prior[z - 1] | prior[z + 1];
        }
    }
    let mut meadow = [0_u64; FINE];
    for z in 2..FINE - 2 {
        let mut candidates = reach[z] & !lake[z];
        while candidates != 0 {
            let x = candidates.trailing_zeros() as usize;
            candidates &= candidates - 1;
            if !(2..FINE - 2).contains(&x) {
                continue;
            }
            pulse();
            let [px, pz] = point(origin, x, z, SPACING);
            let center = world.sample(px, pz);
            if center.surface == Surface::Snow
                || !(0.20..=6.7).contains(&(center.elevation - level))
            {
                continue;
            }
            if [(-2.25, 0.0), (2.25, 0.0), (0.0, -2.25), (0.0, 2.25)]
                .into_iter()
                .all(|(dx, dz)| {
                    (world.elevation(px + dx, pz + dz) - center.elevation).abs() <= 0.47
                })
            {
                meadow[z] |= 1 << x;
            }
        }
    }
    meadow
}

fn shore_frame(
    origin: [f32; 2],
    mask: &[u64; FINE],
    outlet: Option<AlpineOutlet>,
) -> ([f32; 2], [f32; 2], [f32; 2]) {
    let mut lo = [FINE; 2];
    let mut hi = [0; 2];
    for (z, &row) in mask.iter().enumerate().filter(|(_, row)| **row != 0) {
        lo[0] = lo[0].min(row.trailing_zeros() as usize);
        hi[0] = hi[0].max(63 - row.leading_zeros() as usize);
        lo[1] = lo[1].min(z);
        hi[1] = hi[1].max(z);
    }
    let center = core::array::from_fn(|i| origin[i] + (lo[i] + hi[i]) as f32 * SPACING * 0.5);
    let extent =
        core::array::from_fn::<_, 2, _>(|i| ((hi[i] - lo[i]) as f32 * SPACING * 0.5).max(5.0));
    let fallback = if extent[0] >= extent[1] {
        [1.0, 0.0]
    } else {
        [0.0, 1.0]
    };
    let axis = outlet.map_or(fallback, |outlet| {
        let d = [outlet.crest[0] - center[0], outlet.crest[2] - center[1]];
        let scale = d[0].abs().max(d[1].abs());
        if scale < 0.05 {
            return fallback;
        }
        let [x, z] = d.map(|v| v / scale);
        let square = x * x + z * z;
        let root = (1.0 + square) * 0.5;
        [x, z].map(|v| v / (0.5 * (root + square / root)))
    });
    let inv = [
        1.0 / (axis[0].abs() * extent[0] + axis[1].abs() * extent[1]),
        1.0 / (axis[1].abs() * extent[0] + axis[0].abs() * extent[1]),
    ];
    (center, axis, inv)
}

fn count(mask: &[u64; FINE]) -> usize {
    mask.iter().map(|row| row.count_ones() as usize).sum()
}

fn point(origin: [f32; 2], x: usize, z: usize, spacing: f32) -> [f32; 2] {
    [
        origin[0] + x as f32 * spacing,
        origin[1] + z as f32 * spacing,
    ]
}

fn weight(mask: &[u64; FINE], x: usize, z: usize, u: f32, v: f32) -> f32 {
    let cell = |row: usize, col: usize| ((mask[row] >> col) & 1) as f32;
    let a = cell(z, x) * (1.0 - u) + cell(z, x + 1) * u;
    let b = cell(z + 1, x) * (1.0 - u) + cell(z + 1, x + 1) * u;
    a * (1.0 - v) + b * v
}

fn outlet_near(outlet: AlpineOutlet, x: f32, z: f32) -> bool {
    let [a, _, b] = outlet.crest;
    let [c, _, d] = outlet.downstream;
    let [dx, dz] = [c - a, d - b];
    let t = ((x - a) * dx + (z - b) * dz) / (dx * dx + dz * dz);
    if !(-0.35..=1.12).contains(&t) {
        return false;
    }
    let [rx, rz] = [x - a - dx * t, z - b - dz * t];
    rx * rx + rz * rz < 0.95
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tarns_fill_contiguous_high_basins_and_leave_a_dry_rim() {
        for (seed, tx, tz, has_outlet) in [(7, 0, -6, true), (82, 2, 7, true), (1, -2, 3, false)] {
            let world = World::new(seed);
            let lake = AlpineLake::generate(&world, seed, tx, tz).unwrap();
            assert_eq!(
                AlpineLake::generate(&world, seed, tx, tz),
                Some(lake.clone())
            );
            assert_eq!(lake.outlet().is_some(), has_outlet);
            let [min_x, min_z, max_x, max_z] = lake.bounds();
            assert!(
                min_x > (tx * ALPINE_TILE_SIZE) as f32
                    && max_x < ((tx + 1) * ALPINE_TILE_SIZE) as f32
            );
            assert!(
                min_z > (tz * ALPINE_TILE_SIZE) as f32
                    && max_z < ((tz + 1) * ALPINE_TILE_SIZE) as f32
            );
            let [fx, floor, fz] = lake.floor();
            assert!(floor >= 65.0 && lake.level() - floor >= 1.50);
            assert!(lake.conflicts_with(|x, z| (x - fx).abs() < 0.01 && (z - fz).abs() < 0.01));
            assert!(
                lake.sample(
                    fx,
                    fz,
                    Sample {
                        elevation: floor,
                        surface: Surface::Pavement
                    }
                )
                .is_none()
            );

            let anchor = [
                ((fx - min_x) / SPACING) as usize,
                ((fz - min_z) / SPACING) as usize,
            ];
            let mut work = Work::new();
            let (_, _, leaks) = work.fine_flood(
                &world,
                lake.origin,
                anchor,
                lake.level + 0.09,
                true,
                &mut || {},
            );
            assert!(!leaks);
            let (joined, _, leaks) = work.fine_flood(
                &world,
                lake.origin,
                anchor,
                lake.level - 0.015,
                false,
                &mut || {},
            );
            assert!(!leaks);
            assert_eq!(joined, lake.wet);

            if let Some(outlet) = lake.outlet() {
                for [x, height, z] in [outlet.crest, outlet.downstream] {
                    assert_eq!(world.elevation(x, z), height);
                }
                assert!((0.0..0.90).contains(&(outlet.crest[1] - lake.level())));
                assert!(outlet.crest[1] > outlet.downstream[1]);
            }

            let mut meadow_count = 0;
            for z in 0..FINE {
                for x in 0..FINE {
                    let bit = 1 << x;
                    let [px, pz] = point(lake.origin, x, z, SPACING);
                    let ground = world.sample(px, pz);
                    if lake.wet[z] & bit != 0 {
                        let painted = lake.sample(px, pz, ground).unwrap();
                        assert!(matches!(
                            painted.surface,
                            AlpineSurface::Tarn | AlpineSurface::TarnShallows
                        ));
                        assert!(
                            ground.elevation < painted.elevation
                                && painted.elevation == lake.level()
                        );
                    } else if lake.meadow[z] & bit != 0 {
                        assert!(ground.elevation > lake.level());
                        for (dx, dz) in [(-2.25, 0.0), (2.25, 0.0), (0.0, -2.25), (0.0, 2.25)] {
                            assert!(
                                (world.elevation(px + dx, pz + dz) - ground.elevation).abs()
                                    <= 0.4701
                            );
                        }
                        if let Some(sample) = lake.sample(px, pz, ground) {
                            if matches!(
                                sample.surface,
                                AlpineSurface::Meadow | AlpineSurface::Shore(_)
                            ) {
                                assert!(
                                    (ground.elevation..=ground.elevation + 0.46)
                                        .contains(&sample.elevation)
                                );
                                meadow_count += 1;
                            }
                        }
                    }
                }
            }
            assert!(meadow_count >= 20);
        }
    }
}
