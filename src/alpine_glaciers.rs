//! A small valley glacier and its meltwater, fitted to an existing trough.

use crate::alpine_lakes::ALPINE_TILE_SIZE;
use crate::math::{fraction, mix, smoothstep};
use crate::world::{Sample, Surface, World};

const STATIONS: usize = 19;
const STEP: f32 = 8.0;
const ICE_END: usize = 13;
const XZ_LENGTH: f32 = (STATIONS - 1) as f32 * STEP;
const GRID: usize = 65;
const GRID_STEP: f32 = 8.0;
const HALO: f32 = 64.0;
const HEIGHT_CACHE: usize = 256;
const C: f32 = 0.923_879_5;
const S: f32 = 0.382_683_43;
const D: f32 = 0.707_106_77;
const DIRECTIONS: [[f32; 2]; 16] = [
    [1.0, 0.0],
    [C, S],
    [D, D],
    [S, C],
    [0.0, 1.0],
    [-S, C],
    [-D, D],
    [-C, S],
    [-1.0, 0.0],
    [-C, -S],
    [-D, -D],
    [-S, -C],
    [0.0, -1.0],
    [S, -C],
    [D, -D],
    [C, -S],
];

#[derive(Clone, Copy)]
struct HeightSample {
    index: u16,
    elevation: f32,
}

const EMPTY_HEIGHT: HeightSample = HeightSample {
    index: u16::MAX,
    elevation: 0.0,
};

/// Cache the 8-unit lattice on demand. Failed troughs only read their own
/// short reach and leave the rest of the tile unsampled.
pub struct GlacierWork {
    heights: [HeightSample; HEIGHT_CACHE],
    origin: [f32; 2],
    misses: usize,
}

impl GlacierWork {
    pub const fn new() -> Self {
        Self {
            heights: [EMPTY_HEIGHT; HEIGHT_CACHE],
            origin: [0.0; 2],
            misses: 0,
        }
    }

    fn prepare(&mut self, origin: [f32; 2]) {
        self.origin = origin.map(|p| p - HALO);
        self.heights.fill(EMPTY_HEIGHT);
        self.misses = 0;
    }

    fn node(&mut self, world: &World, i: usize, j: usize, pulse: &mut impl FnMut()) -> f32 {
        let index = (j * GRID + i) as u16;
        let slot = &mut self.heights[usize::from(index) & (HEIGHT_CACHE - 1)];
        if slot.index != index {
            if self.misses % 8 == 0 {
                pulse();
            }
            self.misses += 1;
            *slot = HeightSample {
                index,
                elevation: world.elevation(
                    self.origin[0] + GRID_STEP * i as f32,
                    self.origin[1] + GRID_STEP * j as f32,
                ),
            };
        }
        slot.elevation
    }

    fn height(&mut self, world: &World, p: [f32; 2], pulse: &mut impl FnMut()) -> Option<f32> {
        let x = (p[0] - self.origin[0]) * (1.0 / GRID_STEP);
        let z = (p[1] - self.origin[1]) * (1.0 / GRID_STEP);
        let extent = (GRID - 1) as f32;
        if !(0.0..extent).contains(&x) || !(0.0..extent).contains(&z) {
            return None;
        }
        let [i, j] = [x as usize, z as usize];
        let u = x - i as f32;
        let v = z - j as f32;
        let a = lerp(
            self.node(world, i, j, pulse),
            self.node(world, i + 1, j, pulse),
            u,
        );
        let b = lerp(
            self.node(world, i, j + 1, pulse),
            self.node(world, i + 1, j + 1, pulse),
            u,
        );
        Some(lerp(a, b, v))
    }
}

#[derive(Clone, Copy, Debug)]
struct Station {
    offset: f32,
    floor: f32,
    width: f32,
}

impl Station {
    const ZERO: Self = Self {
        offset: 0.0,
        floor: 0.0,
        width: 0.0,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct AlpineGlacier {
    head: [f32; 2],
    axis: [f32; 2],
    stations: [Station; STATIONS],
    bounds: [f32; 4],
    crack_phase: f32,
    confined: usize,
}

impl AlpineGlacier {
    /// Survey terrain with no changes to its height, water, or reservations.
    pub fn generate(
        world: &World,
        seed: u64,
        tile: [i32; 2],
        work: &mut GlacierWork,
        mut reserve: impl FnMut([f32; 4]) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        if tile.into_iter().any(|v| !(-4096..=4096).contains(&v)) {
            return None;
        }
        let tag = (u64::from(tile[0] as u32) << 32) | u64::from(tile[1] as u32);
        let key = mix(seed ^ tag ^ 0x56aa_0120_f130_c14a);
        let origin = tile.map(|v| (v * ALPINE_TILE_SIZE) as f32);
        let mut heads = [([0.0; 2], 0.0_f32); 12];
        let start = ((key >> 17) % 196) as usize;
        let jitter = fraction(key) * 4.0;
        for k in 0..196 {
            pulse();
            let i = (start + k * 67) % 196;
            let head = [
                origin[0] + 36.0 + (i % 14) as f32 * 24.0 + jitter,
                origin[1] + 36.0 + (i / 14) as f32 * 24.0 - jitter,
            ];
            let height = world.elevation(head[0], head[1]);
            if !(130.0..=185.0).contains(&height)
                || world.sample(head[0], head[1]).surface != Surface::Snow
            {
                continue;
            }
            let score = (height - 148.0).abs();
            let j = heads.partition_point(|(_, old)| *old > 0.0 && (*old - 148.0).abs() <= score);
            let capacity = heads.len();
            if j < capacity {
                heads.copy_within(j..capacity - 1, j + 1);
                heads[j] = (head, height);
            }
        }
        if heads[0].1 == 0.0 {
            return None;
        }
        work.prepare(origin);
        let mut best = None;
        let mut best_score = f32::NEG_INFINITY;
        for (head, height) in heads.into_iter().filter(|(_, height)| *height > 0.0) {
            for turn in 0..DIRECTIONS.len() {
                pulse();
                let angle = (turn + ((key >> 40) as usize & 15)) & 15;
                let axis = DIRECTIONS[angle];
                let Some((stations, score)) = trace(world, work, head, height, axis, &mut pulse)
                else {
                    continue;
                };
                if score <= best_score {
                    continue;
                }
                let Some(plan) = Self::certify(world, head, axis, stations, key, &mut pulse) else {
                    continue;
                };
                if !origin.into_iter().enumerate().all(|(i, corner)| {
                    plan.bounds[i] >= corner + 2.0
                        && plan.bounds[i + 2] <= corner + ALPINE_TILE_SIZE as f32 - 2.0
                }) {
                    continue;
                }
                if reserve(plan.bounds) {
                    continue;
                }
                best_score = score;
                best = Some(plan);
            }
        }
        best
    }

    fn certify(
        world: &World,
        head: [f32; 2],
        axis: [f32; 2],
        mut stations: [Station; STATIONS],
        key: u64,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let at = |i: f32, lateral: f32| {
            let [x, z] = point(head, axis, i * STEP, lateral);
            world.elevation(x, z)
        };
        let mut confined = 0;
        let mut bounds = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        let mut previous = f32::INFINITY;
        for i in 0..STATIONS {
            pulse();
            let offset = stations[i].offset;
            let floor = at(i as f32, offset);
            let drop = previous - floor;
            if i > 0 {
                if !(0.20..=8.1).contains(&drop) {
                    return None;
                }
                let last_offset = stations[i - 1].offset;
                let middle = at(i as f32 - 0.5, (last_offset + offset) * 0.5);
                if middle > previous + 0.08 || middle < floor - 0.08 {
                    return None;
                }
            }
            let [left, right] = [at(i as f32, offset - 22.0), at(i as f32, offset + 22.0)];
            let wall = left.min(right) - floor;
            if (1..ICE_END).contains(&i) && wall >= 2.7 {
                confined += 1;
            }
            if (3..ICE_END).contains(&i) && wall < -1.2 {
                return None;
            }
            if floor < 65.0 {
                return None;
            }
            let width = if i <= ICE_END {
                let opening = (14.0 + 0.45 * (wall - 3.0)).clamp(12.0, 24.0);
                let expansion = smoothstep(i as f32 * 0.48);
                let tongue = 1.0 - 0.52 * smoothstep((i as f32 - 9.0) * 0.27);
                (opening * (0.77 + 0.23 * expansion) * tongue).max(7.0)
            } else {
                3.5
            };
            stations[i] = Station {
                offset,
                floor,
                width,
            };
            let radius = width + 2.0;
            for lateral in [offset - radius, offset + radius] {
                let [x, z] = point(head, axis, i as f32 * STEP, lateral);
                bounds = [
                    bounds[0].min(x),
                    bounds[1].min(z),
                    bounds[2].max(x),
                    bounds[3].max(z),
                ];
            }
            previous = floor;
        }
        if confined < 8
            || stations[0].floor - stations[ICE_END].floor < 28.0
            || stations[ICE_END].floor > 119.0
            || stations[ICE_END].floor - previous < 5.0
        {
            return None;
        }
        // The source must be an accumulation area with some width, not one
        // incidental snow-covered point along an otherwise bare rock ridge.
        let source_snow = [-6.0, 0.0, 6.0]
            .into_iter()
            .flat_map(|side| [0.0, STEP].map(|s| point(head, axis, s, side)))
            .filter(|p| world.sample(p[0], p[1]).surface == Surface::Snow)
            .count();
        if source_snow < 3 {
            return None;
        }
        Some(Self {
            head,
            axis,
            stations,
            bounds,
            crack_phase: 0.9 + fraction(key) * 3.0,
            confined,
        })
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }
    pub fn downhill(&self) -> [f32; 2] {
        self.axis
    }
    pub fn confined_stations(&self) -> usize {
        self.confined
    }
    pub fn profile(&self) -> impl Iterator<Item = [f32; 4]> + '_ {
        self.stations.iter().enumerate().map(|(i, station)| {
            let [x, z] = point(self.head, self.axis, i as f32 * STEP, station.offset);
            [x, station.floor, z, station.width]
        })
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let [x0, z0, x1, z1] = self.bounds;
        if !(x0..x1).contains(&x)
            || !(z0..z1).contains(&z)
            || !matches!(
                ground.surface,
                Surface::Snow | Surface::Rock | Surface::Grass | Surface::Forest
            )
        {
            return ground;
        }
        let [dx, dz] = [x - self.head[0], z - self.head[1]];
        let along = dx * self.axis[0] + dz * self.axis[1];
        if !(0.0..XZ_LENGTH).contains(&along) {
            return ground;
        }
        let t = along * (1.0 / STEP);
        let i = t as usize;
        let u = t - i as f32;
        let a = self.stations[i];
        let b = self.stations[i + 1];
        let across = dz * self.axis[0] - dx * self.axis[1] - lerp(a.offset, b.offset, u);
        let floor = lerp(a.floor, b.floor, u);
        let signed_high = ground.elevation - floor;
        let melt = ICE_END as f32 * STEP;
        if along < melt {
            let width = lerp(a.width, b.width, u);
            let side = across.abs() / width;
            let end = melt - along;
            if side >= 1.0 || signed_high > 7.0 || signed_high < -3.5 || end < 6.0 * side * side {
                return ground;
            }
            let upper = smoothstep(along * (1.0 / 12.0));
            let weight = upper
                * smoothstep((1.0 - side) * 3.5)
                * smoothstep(end * 0.095)
                * smoothstep((7.0 - signed_high) * 0.37);
            if weight < 0.035 {
                return ground;
            }
            let snow_head = 1.0 - smoothstep((along - 13.0) * (1.0 / 25.0));
            let bend = 6.6 * side * side - 0.21 * across
                + 5.0 * (along * (1.0 / 104.0) - 0.4).abs() * side;
            let crack = (along + bend + self.crack_phase) * (1.0 / 10.2);
            let rift = (crack - (crack as i32) as f32 - 0.5).abs();
            let steep = smoothstep((a.floor - b.floor - 1.2) * 0.19);
            let open = upper
                * smoothstep((along - 8.0) * 0.058)
                * smoothstep(end * 0.1)
                * (0.65 + 0.35 * steep);
            let cut = (1.0 - smoothstep((rift - 0.05) * 8.5)) * open * weight;
            let ridge = 4.8 * (0.68 + 0.32 * upper) * weight;
            let surface = if cut > 0.27 && (across + 0.07 * along).abs() < width * 0.82 {
                Surface::GlacierCrevasse
            } else if snow_head > 0.62 && ground.surface == Surface::Snow {
                Surface::Snow
            } else if side > 0.83 && along > 49.0 && weight > 0.13 {
                Surface::GlacierMoraine
            } else {
                Surface::GlacierIce
            };
            Sample {
                elevation: ground.elevation + (ridge - 0.52 * cut).max(0.0),
                surface,
            }
        } else {
            let clear_tail = XZ_LENGTH - along;
            let width = 2.2 + 0.8 * smoothstep((along - melt) * 0.10);
            let fade = smoothstep(clear_tail * (1.0 / 12.0));
            let edge = across.abs() / width;
            if edge >= 1.55 || signed_high.abs() > 2.2 {
                return ground;
            }
            let hollow =
                smoothstep((1.0 - edge) * 2.6) * fade * smoothstep((along - melt) * (1.0 / 4.5));
            let surface = if edge < 0.67 && fade > 0.4 {
                Surface::GlacierMeltwater
            } else if fade > 0.22 {
                Surface::Gravel
            } else {
                ground.surface
            };
            Sample {
                elevation: ground.elevation - 0.48 * hollow,
                surface,
            }
        }
    }
}

fn trace(
    world: &World,
    work: &mut GlacierWork,
    head: [f32; 2],
    first: f32,
    axis: [f32; 2],
    pulse: &mut impl FnMut(),
) -> Option<([Station; STATIONS], f32)> {
    let mut stations = [Station::ZERO; STATIONS];
    stations[0].floor = first;
    let mut score = 0.0;
    let mut confined = 0;
    for i in 1..STATIONS {
        let previous = stations[i - 1];
        let distance = i as f32 * STEP;
        let mut winner = None;
        let mut best = f32::NEG_INFINITY;
        for step in -3_i32..=3 {
            let offset = previous.offset + step as f32 * 1.7;
            let Some(h) = work.height(world, point(head, axis, distance, offset), pulse) else {
                continue;
            };
            let drop = previous.floor - h;
            if !(0.16..=7.9).contains(&drop) {
                continue;
            }
            let Some(a) = work.height(world, point(head, axis, distance, offset - 22.0), pulse)
            else {
                continue;
            };
            let Some(b) = work.height(world, point(head, axis, distance, offset + 22.0), pulse)
            else {
                continue;
            };
            let wall = a.min(b) - h;
            let curve = if i > 1 {
                (offset + stations[i - 2].offset - 2.0 * previous.offset).abs()
            } else {
                0.0
            };
            let bias = (a - b).abs() * 0.045 + curve * 0.10;
            let grade = (drop - 3.2).abs() * 0.21 + step.unsigned_abs() as f32 * 0.13;
            let quality = wall.clamp(-16.0, 11.5) - bias - grade;
            if quality > best {
                winner = Some((
                    Station {
                        offset,
                        floor: h,
                        width: 0.0,
                    },
                    wall,
                ));
                best = quality;
            }
        }
        let (chosen, wall) = winner?;
        if i < ICE_END && wall > 2.5 {
            confined += 1;
        }
        if (3..ICE_END).contains(&i) && wall < -1.5 {
            return None;
        }
        stations[i] = chosen;
        score += best;
    }
    if confined < 8 || first - stations[ICE_END].floor < 27.0 || stations[ICE_END].floor > 120.0 {
        return None;
    }
    Some((stations, score + confined as f32 * 2.0))
}

fn point(origin: [f32; 2], axis: [f32; 2], along: f32, across: f32) -> [f32; 2] {
    [
        origin[0] + axis[0] * along - axis[1] * across,
        origin[1] + axis[1] * along + axis[0] * across,
    ]
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
