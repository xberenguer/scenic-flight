//! Budgeted, world-aligned elevation for the distant ground silhouette.

use core::cell::Cell;

use crate::world::World;

const SCALE: f32 = 64.0;
const BACKDROP_BLEND_START: f32 = 2000.0 * 2000.0;
const BACKDROP_BLEND_END: f32 = 2450.0 * 2450.0;
const OUTER_BLEND_START: f32 = 3200.0 * 3200.0;
const OUTER_BLEND_END: f32 = 3650.0 * 3650.0;

struct Strip {
    axis: usize,
    direction: i32,
    cursor: usize,
}

/// A height-only grid independent of the foreground samplers.
/// Negative values retain the coastline through interpolation.
struct HorizonGrid<const SIDE: usize, const SPACING: u16> {
    elevation: [[i16; SIDE]; SIDE],
    world: World,
    origin: [i32; 2],
    retained: [[i32; 2]; 2],
    strip: Option<Strip>,
    ready: bool,
}

impl<const SIDE: usize, const SPACING: u16> HorizonGrid<SIDE, SPACING> {
    pub const fn new() -> Self {
        Self {
            elevation: [[0; SIDE]; SIDE],
            world: World::new(0),
            origin: [0; 2],
            retained: [[0; 2]; 2],
            strip: None,
            ready: false,
        }
    }

    pub fn reset_cooperatively(&mut self, world: World, x: f32, z: f32, mut pulse: impl FnMut()) {
        self.world = world;
        self.origin = Self::origin(x, z);
        self.strip = None;
        let [left, near] = self.origin;
        for z in near..near + SIDE as i32 {
            pulse();
            for x in left..left + SIDE as i32 {
                self.place(x, z);
            }
        }
        self.ready = true;
        self.update_retained();
    }

    /// Replenish no more than `budget` cells. The coordinate of an unfinished
    /// strip remains unavailable until every cell has its new value.
    pub fn follow(&mut self, x: f32, z: f32, budget: usize) -> usize {
        if !self.ready {
            return 0;
        }
        let target = Self::origin(x, z);
        let mut spent = 0;
        while spent < budget {
            let delta = [target[0] - self.origin[0], target[1] - self.origin[1]];
            if self.strip.is_none() {
                let axis = usize::from(delta[1].abs() > delta[0].abs());
                let direction = delta[axis].signum();
                if direction == 0 {
                    break;
                }
                self.strip = Some(Strip {
                    axis,
                    direction,
                    cursor: 0,
                });
            }
            let Strip {
                axis,
                direction,
                cursor,
            } = self.strip.take().unwrap();
            let count = (SIDE - cursor).min(budget - spent);
            let mut cell = self.origin;
            cell[axis] += if direction > 0 { SIDE as i32 } else { -1 };
            for offset in cursor..cursor + count {
                cell[axis ^ 1] = self.origin[axis ^ 1] + offset as i32;
                self.place(cell[0], cell[1]);
            }
            spent += count;
            if cursor + count == SIDE {
                self.origin[axis] += direction;
            } else {
                self.strip = Some(Strip {
                    axis,
                    direction,
                    cursor: cursor + count,
                });
            }
        }
        if spent != 0 {
            self.update_retained();
        }
        spent
    }

    /// Sample the same world-aligned lattice even beyond the retained grid.
    /// Missing vertices cost a procedural query; they never change the shape.
    #[inline]
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        Self::sample_using(x, z, |cell| self.quad(cell).map(f32::from))
    }

    #[inline]
    fn sample_using(x: f32, z: f32, corners: impl FnOnce([i32; 2]) -> [f32; 4]) -> f32 {
        let gx = x * (1.0 / SPACING as f32);
        let gz = z * (1.0 / SPACING as f32);
        let ix = floor(gx);
        let iz = floor(gz);
        let u = gx - ix as f32;
        let v = gz - iz as f32;
        let [a, b, c, d] = corners([ix, iz]);
        let near = a + (b - a) * u;
        let far = c + (d - c) * u;
        (near + (far - near) * v) * (1.0 / SCALE)
    }

    #[inline]
    fn quad(&self, [ix, iz]: [i32; 2]) -> [i16; 4] {
        let [lo, hi] = self.retained;
        if ix >= lo[0] && ix < hi[0] && iz >= lo[1] && iz < hi[1] {
            let near = &self.elevation[Self::slot(iz)];
            let far = &self.elevation[Self::slot(iz + 1)];
            let [left, right] = [Self::slot(ix), Self::slot(ix + 1)];
            [near[left], near[right], far[left], far[right]]
        } else {
            self.procedural_quad(ix, iz)
        }
    }

    pub fn covers(&self, x: f32, z: f32) -> bool {
        let x = floor(x * (1.0 / SPACING as f32));
        let z = floor(z * (1.0 / SPACING as f32));
        let [lo, hi] = self.retained;
        x >= lo[0] && x < hi[0] && z >= lo[1] && z < hi[1]
    }

    // Keep interpolation origins whose four corners are all ready. A
    // partially overwritten row or column is excluded until it commits.
    fn update_retained(&mut self) {
        let mut lo = self.origin;
        let mut hi = [lo[0] + SIDE as i32 - 1, lo[1] + SIDE as i32 - 1];
        if let Some(strip) = &self.strip {
            if strip.direction > 0 {
                lo[strip.axis] += 1;
            } else {
                hi[strip.axis] -= 1;
            }
        }
        self.retained = [lo, hi];
    }

    #[cold]
    #[inline(never)]
    fn procedural_quad(&self, ix: i32, iz: i32) -> [i16; 4] {
        [[ix, iz], [ix + 1, iz], [ix, iz + 1], [ix + 1, iz + 1]]
            .map(|[x, z]| Self::relief(&self.world, x, z))
    }

    fn place(&mut self, x: i32, z: i32) {
        self.elevation[Self::slot(z)][Self::slot(x)] = Self::relief(&self.world, x, z);
    }
}

impl<const SIDE: usize, const SPACING: u16> HorizonGrid<SIDE, SPACING> {
    fn relief(world: &World, x: i32, z: i32) -> i16 {
        let h = world.signed_horizon_height(x as f32 * SPACING as f32, z as f32 * SPACING as f32);
        (h * SCALE + h.signum() * 0.5) as i16
    }
    fn slot(cell: i32) -> usize {
        (cell as usize) & (SIDE - 1)
    }
    fn origin(x: f32, z: f32) -> [i32; 2] {
        [x, z].map(|v| floor(v * (1.0 / SPACING as f32)) - SIDE as i32 / 2)
    }
}

fn floor(value: f32) -> i32 {
    let n = value as i32;
    n - i32::from(value < n as f32)
}

/// Coast and relief grids for the landscape beyond the detailed terrain.
pub struct HorizonCache {
    near: HorizonGrid<64, 80>,
    middle: HorizonGrid<32, 256>,
    outer: HorizonGrid<32, 1800>,
    eye: [f32; 2],
    last_outer: Cell<Option<OuterQuad>>,
}

#[derive(Clone, Copy)]
struct OuterQuad {
    cell: [i32; 2],
    heights: [f32; 4],
}

impl HorizonCache {
    pub const fn new() -> Self {
        Self {
            near: HorizonGrid::new(),
            middle: HorizonGrid::new(),
            outer: HorizonGrid::new(),
            eye: [0.0; 2],
            last_outer: Cell::new(None),
        }
    }
    pub fn reset(&mut self, world: World, x: f32, z: f32) {
        self.reset_cooperatively(world, x, z, || {});
    }
    pub fn reset_cooperatively(&mut self, world: World, x: f32, z: f32, mut pulse: impl FnMut()) {
        self.eye = [x, z];
        self.last_outer.set(None);
        self.near.reset_cooperatively(world, x, z, &mut pulse);
        self.middle.reset_cooperatively(world, x, z, &mut pulse);
        self.outer.reset_cooperatively(world, x, z, pulse);
    }
    pub fn follow(&mut self, x: f32, z: f32, budget: usize) -> usize {
        self.eye = [x, z];
        let outer = self.outer.follow(x, z, budget.min(2));
        let middle = self.middle.follow(x, z, (budget - outer).min(2));
        outer + middle + self.near.follow(x, z, budget - outer - middle)
    }
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        // Keep mesh anchoring and all skyline queries on the original lattice.
        self.near.sample(x, z)
    }
    pub fn backdrop(&self, x: f32, z: f32) -> f32 {
        let dx = x - self.eye[0];
        let dz = z - self.eye[1];
        let distance = dx * dx + dz * dz;
        if distance <= BACKDROP_BLEND_START {
            return self.near.sample(x, z);
        }
        if distance < BACKDROP_BLEND_END {
            let near = self.near.sample(x, z);
            let middle = self.middle.sample(x, z);
            let fade = crate::math::smoothstep(
                (distance - BACKDROP_BLEND_START) / (BACKDROP_BLEND_END - BACKDROP_BLEND_START),
            );
            return near + (middle - near) * fade;
        }
        if distance <= OUTER_BLEND_START {
            return self.middle.sample(x, z);
        }
        // Adjacent distant rays often use the same four vertices. Their
        // heights are world-aligned, so they survive strip refills.
        let outer = HorizonGrid::<32, 1800>::sample_using(x, z, |cell| {
            if let Some(last) = self.last_outer.get().filter(|last| last.cell == cell) {
                return last.heights;
            }
            let heights = self.outer.quad(cell).map(f32::from);
            self.last_outer.set(Some(OuterQuad { cell, heights }));
            heights
        });
        if distance >= OUTER_BLEND_END {
            return outer;
        }
        let fade = crate::math::smoothstep(
            (distance - OUTER_BLEND_START) / (OUTER_BLEND_END - OUTER_BLEND_START),
        );
        let middle = self.middle.sample(x, z);
        middle + (outer - middle) * fade
    }
    pub fn covers(&self, x: f32, z: f32) -> bool {
        self.near.covers(x, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outer_lattice_covers_the_extended_aperture_and_keeps_near_anchoring() {
        let mut cache = HorizonCache::new();
        cache.reset(World::new(11), -512.0, 1880.0);
        for heading in [0.0, 0.73, 1.57, 2.22, 3.14, 4.1, 5.7] {
            let (east, north) = crate::math::sin_cos(heading);
            for lateral in [-13200.0, 0.0, 13200.0] {
                let x = -512.0 + east * 19200.0 + north * lateral;
                let z = 1880.0 + north * 19200.0 - east * lateral;
                assert!(cache.outer.covers(x, z));
                assert_eq!(cache.backdrop(x, z), cache.outer.sample(x, z));
            }
        }
        for x in [-1800.0, 0.0, 1300.0] {
            assert_eq!(cache.backdrop(x, 1880.0), cache.sample(x, 1880.0));
        }
    }

    #[test]
    fn middle_backdrop_retains_more_of_the_seeded_coast_and_relief() {
        let mut cache = HorizonCache::new();
        let mut old = (0, 0.0);
        let mut new = (0, 0.0);
        for (seed, x, z) in [
            (1, -425.0, 340.0),
            (3, 780.0, -692.0),
            (11, -512.0, 1_880.0),
            (22, -1_100.0, 1_258.0),
            (41, 320.0, -600.0),
            (98, 1_680.0, -560.0),
        ] {
            let world = World::new(seed);
            cache.reset(world, x, z);
            for angle in 0..48 {
                let (sin, cos) = crate::math::sin_cos(angle as f32 * core::f32::consts::TAU / 48.0);
                for range in [2_500.0, 2_750.0, 3_000.0, 3_150.0] {
                    let [east, north] = [x + sin * range, z + cos * range];
                    assert!(cache.middle.covers(east, north));
                    let truth = world.signed_horizon_height(east, north);
                    for (score, sample) in [
                        (&mut old, cache.outer.sample(east, north)),
                        (&mut new, cache.backdrop(east, north)),
                    ] {
                        score.0 += usize::from((truth > 0.0) != (sample > 0.0));
                        score.1 += (truth - sample).abs();
                    }
                }
                for range in [2_000.0, 2_450.0, 3_200.0, 3_650.0] {
                    let sample = |r: f32| cache.backdrop(x + sin * r, z + cos * r);
                    assert!((sample(range - 0.01) - sample(range + 0.01)).abs() < 0.08);
                }
            }
        }
        assert!(
            new.0 * 4 < old.0 * 3,
            "land or water mismatches: {old:?} -> {new:?}"
        );
        assert!(
            new.1 * 4.0 < old.1 * 3.0,
            "total elevation error: {old:?} -> {new:?}"
        );
        std::println!(
            "1,152 samples: {old:?} -> {new:?} (coast mismatches, summed absolute elevation error)"
        );
    }

    #[test]
    fn reseeding_refreshes_the_outer_vertices_even_at_the_same_position() {
        let mut cache = HorizonCache::new();
        let position = (6351.5, -9812.25);
        cache.reset(World::new(11), 0.0, 0.0);
        let first = cache.backdrop(position.0, position.1);
        assert!(cache.last_outer.get().is_some());
        cache.reset(World::new(98), 0.0, 0.0);
        assert!(cache.last_outer.get().is_none());
        let second = cache.backdrop(position.0, position.1);
        assert_eq!(
            second.to_bits(),
            cache.outer.sample(position.0, position.1).to_bits()
        );
        assert_ne!(first.to_bits(), second.to_bits());
    }

    #[test]
    fn moving_blackbird_renders_the_same_backdrop_while_cache_strips_are_incomplete() {
        use crate::{
            flight::{Aircraft, Flight},
            render::{Camera, ScreenRect, render_terrain_layer_region_with_horizon},
            sky::Sky,
            terrain_cache::TerrainCache,
        };

        const SIDE: usize = 60;
        const AREA: usize = SIDE * SIDE;
        let world = World::new(11);
        let mut flight = Flight::start(&world, 11, Aircraft::Blackbird);
        let mut cache = HorizonCache::new();
        let mut reference = HorizonCache {
            near: HorizonGrid {
                world,
                ..HorizonGrid::new()
            },
            middle: HorizonGrid {
                world,
                ..HorizonGrid::new()
            },
            outer: HorizonGrid {
                world,
                ..HorizonGrid::new()
            },
            ..HorizonCache::new()
        };
        let (start_x, start_z) = (flight.view.x, flight.view.z);
        cache.reset(world, start_x, start_z);
        let mut terrain = TerrainCache::new();
        terrain.reset_for_altitude(world, start_x, start_z, flight.view.altitude);
        let mut with_cache = ([0; AREA], [0; AREA]);
        let mut without_cache = ([0; AREA], [0; AREA]);
        let mut scratch = [0; 2 * AREA];
        let mut outer_frames = 0;

        for step in 0..120 {
            for _ in 0..30 {
                flight.update(&world);
            }
            let view = flight.view;
            let camera = Camera {
                x: view.x,
                z: view.z,
                altitude: view.altitude,
                heading: view.heading,
                pitch: view.pitch,
                bank: view.bank,
            };
            // Follow at the board's per-frame budget. Each one-second
            // checkpoint also samples a deliberately unfinished refill.
            let budget = if step % 3 == 0 { 0 } else { 16 };
            assert!(cache.follow(view.x, view.z, budget) <= budget);
            reference.follow(view.x, view.z, 0);
            terrain.follow(view.x, view.z);
            let sky = Sky::from_time(11, flight.ticks, 0.49);
            for (horizon, (pixels, depth)) in
                [(&cache, &mut with_cache), (&reference, &mut without_cache)]
            {
                render_terrain_layer_region_with_horizon(
                    (|x, z| terrain.sample(x, z), |_, _, sample| sample.surface),
                    |x, z| horizon.backdrop(x, z),
                    camera,
                    &sky,
                    pixels,
                    &mut scratch,
                    Some((depth, ScreenRect::full((SIDE, SIDE)))),
                    (SIDE, SIDE),
                    |_| {},
                    |_| None,
                )
                .unwrap();
            }
            assert!(
                with_cache == without_cache,
                "color or depth changed at second {}",
                step + 1
            );
            let outer_depth = crate::render::encoded_depth(crate::render::CITY_HORIZON + 32.0);
            outer_frames += usize::from(
                with_cache
                    .1
                    .iter()
                    .any(|&depth| (outer_depth..u16::MAX).contains(&depth)),
            );
        }
        assert!(
            outer_frames > 0,
            "fixture must draw beyond the ordinary depth range"
        );
    }

    #[test]
    fn pending_strips_and_fast_repositions_keep_the_same_lattice() {
        let world = World::new(7911);
        let mut cache = HorizonCache::new();
        cache.reset(world, -73.0, 45.0);
        let uncached = HorizonGrid::<64, 80> {
            world,
            ..HorizonGrid::new()
        };
        for [x, z] in [
            [129.0, -151.0],
            [-210.0, 79.0],
            [6430.0, -8220.0],
            [6451.0, -8173.0],
        ] {
            for budget in [0, 7, 16, 16, 32, 16, 64] {
                assert!(cache.follow(x, z, budget) <= budget);
                for dz in [-2345.5, -1015.3, -74.25, 0.0, 792.5, 2335.1] {
                    for dx in [-2401.3, -833.2, -31.4, 0.0, 802.1, 2380.7] {
                        assert_eq!(
                            cache.sample(x + dx, z + dz).to_bits(),
                            uncached.sample(x + dx, z + dz).to_bits()
                        );
                    }
                }
            }
        }
        cache.reset(world, 6451.0, -8173.0);
        assert!(cache.covers(6451.0 - 2290.0, -8173.0 + 2290.0));
        assert!(core::mem::size_of::<HorizonCache>() < 12_600);
    }
}
