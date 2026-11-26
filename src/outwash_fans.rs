//! Relict glacial outwash spread below a surveyed mountain-valley mouth.

use crate::alpine_lakes::ALPINE_TILE_SIZE;
use crate::math::{fraction, mix, smoothstep};
use crate::world::{Sample, Surface, World};
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutwashProfile {
    pub name: &'static str,
    width: [f32; 2],
    split: [f32; 2],
    rejoin: [f32; 2],
    branch_reach: f32,
    deposit: f32,
}

impl OutwashProfile {
    pub const ALL: [Self; 25] = [
        Self::new(
            "Wide braidplain",
            [0.99, 0.00],
            [0.05, 0.18],
            [0.76, 0.92],
            1.00,
            0.92,
        ),
        Self::new(
            "Narrow meltwater chute",
            [0.69, 0.19],
            [0.03, 0.14],
            [0.79, 0.89],
            0.80,
            0.82,
        ),
        Self::new(
            "Early left fork",
            [0.91, -0.17],
            [0.01, 0.25],
            [0.63, 0.94],
            1.12,
            1.00,
        ),
        Self::new(
            "Early right fork",
            [0.90, 0.17],
            [0.27, 0.01],
            [0.94, 0.63],
            1.12,
            0.97,
        ),
        Self::new(
            "Paired gravel bars",
            [0.97, 0.00],
            [0.08, 0.10],
            [0.85, 0.88],
            0.90,
            1.15,
        ),
        Self::new(
            "Long left anabranch",
            [0.92, -0.09],
            [0.01, 0.19],
            [0.96, 0.72],
            1.16,
            0.94,
        ),
        Self::new(
            "Long right anabranch",
            [0.94, 0.10],
            [0.20, 0.02],
            [0.72, 0.96],
            1.14,
            0.99,
        ),
        Self::new(
            "Tapered gravel tongue",
            [0.78, 0.10],
            [0.07, 0.18],
            [0.77, 0.90],
            0.85,
            1.12,
        ),
        Self::new(
            "Broad shingle toe",
            [0.98, -0.22],
            [0.13, 0.19],
            [0.89, 0.95],
            1.03,
            1.20,
        ),
        Self::new(
            "Offset silver braid",
            [0.94, 0.23],
            [0.03, 0.21],
            [0.81, 0.95],
            0.97,
            0.85,
        ),
        Self::new(
            "Deep central runnel",
            [0.87, -0.11],
            [0.10, 0.18],
            [0.79, 0.93],
            0.88,
            0.79,
        ),
        Self::new(
            "Stepped spill apron",
            [0.81, 0.21],
            [0.06, 0.11],
            [0.75, 0.86],
            0.92,
            1.10,
        ),
        Self::new(
            "Open gravel delta",
            [0.98, 0.20],
            [0.11, 0.23],
            [0.93, 0.97],
            1.10,
            1.28,
        ),
        Self::new(
            "Late left splay",
            [0.92, -0.25],
            [0.26, 0.08],
            [0.96, 0.76],
            1.02,
            0.95,
        ),
        Self::new(
            "Late right splay",
            [0.94, 0.24],
            [0.07, 0.27],
            [0.77, 0.96],
            1.04,
            0.95,
        ),
        Self::new(
            "Twin needle bars",
            [0.83, 0.05],
            [0.06, 0.07],
            [0.92, 0.92],
            0.67,
            0.84,
        ),
        Self::new(
            "Shallow fan bench",
            [0.97, -0.23],
            [0.07, 0.15],
            [0.77, 0.83],
            1.20,
            1.07,
        ),
        Self::new(
            "Pebble-rib apron",
            [0.95, -0.25],
            [0.02, 0.16],
            [0.78, 0.89],
            0.83,
            1.25,
        ),
        Self::new(
            "Hooked fan shoulder",
            [0.92, 0.27],
            [0.12, 0.07],
            [0.85, 0.89],
            1.16,
            1.05,
        ),
        Self::new(
            "Short split apron",
            [0.83, -0.07],
            [0.09, 0.16],
            [0.68, 0.76],
            0.98,
            0.95,
        ),
        Self::new(
            "Late-joining braid",
            [0.99, 0.13],
            [0.09, 0.14],
            [0.94, 0.96],
            1.08,
            0.87,
        ),
        Self::new(
            "Inset gravel islands",
            [0.86, -0.16],
            [0.12, 0.15],
            [0.88, 0.92],
            0.88,
            1.15,
        ),
        Self::new(
            "Splayed braid arms",
            [0.98, 0.04],
            [0.04, 0.11],
            [0.88, 0.95],
            1.24,
            1.24,
        ),
        Self::new(
            "Uneven terrace toes",
            [0.84, -0.24],
            [0.23, 0.06],
            [0.95, 0.84],
            0.96,
            1.24,
        ),
        Self::new(
            "Diverging lower apron",
            [0.99, -0.07],
            [0.21, 0.22],
            [0.96, 0.98],
            1.16,
            1.15,
        ),
    ];

    const fn new(
        name: &'static str,
        width: [f32; 2],
        split: [f32; 2],
        rejoin: [f32; 2],
        branch_reach: f32,
        deposit: f32,
    ) -> Self {
        Self {
            name,
            width,
            split,
            rejoin,
            branch_reach,
            deposit,
        }
    }

    fn width_at(self, t: f32) -> f32 {
        self.width[0] * (1.0 + self.width[1] * (t - 0.5))
    }

    fn strand_offsets(self, t: f32) -> [f32; 3] {
        let [left, right] = core::array::from_fn(|i| {
            let p = ((t - self.split[i]) / (self.rejoin[i] - self.split[i])).clamp(0.0, 1.0);
            4.0 * p * (1.0 - p) * self.branch_reach
        });
        [-0.52 * left, 0.0, 0.61 * right]
    }

    fn thread(self, t: f32, side: f32, width: f32, sway: f32) -> f32 {
        let near = self
            .strand_offsets(t)
            .map(|path| (path * width + sway - side).abs())
            .into_iter()
            .fold(f32::INFINITY, f32::min);
        let lane = 1.7 + 1.95 * t * (1.55 - t) + 0.45 * (1.0 - (2.5 * t - 0.91).abs()).max(0.0);
        smoothstep((lane + 0.9 - near) * 0.48)
    }

    pub fn study(self, t: f32, side: f32) -> (f32, f32) {
        let width = 5.0 + 29.0 * t * (1.53 - 0.53 * t) * self.width_at(t).min(1.0);
        let edge = width - side.abs();
        let apron = smoothstep(edge * 0.13) * smoothstep(t * 12.0) * smoothstep((1.0 - t) * 7.5);
        let thread = self.thread(t, side, width, 0.0);
        (
            apron * thread,
            apron * (self.deposit * (1.0 - thread) - 0.40 * thread),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutwashFan {
    center: [f32; 2],
    downhill: [f32; 2],
    bounds: [f32; 4],
    length: f32,
    inverse: f32,
    spread: f32,
    bend: f32,
    drift: f32,
    deposit: f32,
    design: u8,
}

impl OutwashFan {
    pub fn generate(world: &World, seed: u64, x: i32, z: i32) -> Option<Self> {
        Self::generate_cooperatively(world, seed, x, z, &mut || {})
    }

    pub(crate) fn generate_cooperatively(
        world: &World,
        seed: u64,
        x: i32,
        z: i32,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        pulse();
        if !(-4096..=4096).contains(&x) || !(-4096..=4096).contains(&z) {
            return None;
        }
        let tile = (u64::from(x as u32) << 32) | u64::from(z as u32);
        let key = mix(seed ^ tile ^ 0x4dd4_4e8a_bb8f_2bf3);
        if key & 3 >= 2 {
            return None;
        }
        let origin = [x, z].map(|v| (v * ALPINE_TILE_SIZE) as f32);
        let jitter = [fraction(key) * 9.0, fraction(mix(key)) * 9.0];
        let first = ((key >> 20) & 63) as usize;
        let mut proofs = 0;
        for i in 0..64 {
            pulse();
            let cell = (first + i * 23) & 63;
            let center = [
                origin[0] + 64.0 + (cell % 8) as f32 * 35.0 + jitter[0],
                origin[1] + 64.0 + (cell / 8) as f32 * 35.0 + jitter[1],
            ];
            let ground = world.elevation(center[0], center[1]);
            if !(55.0..=92.0).contains(&ground) {
                continue;
            }
            let key = mix(key ^ (0x4e34 + cell as u64));
            let rotation = ((key >> 13) & 15) as usize;
            for direction in 0..16 {
                let downhill = DIRECTIONS[(direction + rotation) & 15];
                let at = |forward, side| {
                    let [x, z] = shifted(center, downhill, forward, side);
                    world.elevation(x, z)
                };
                let h = ground;
                let toe = at(56.0, 0.0);
                if !(4.5..=15.5).contains(&(h - toe)) {
                    continue;
                }
                let apex = at(-38.0, 0.0);
                if !(2.5..=14.0).contains(&(apex - h)) || at(-76.0, 0.0) < (h + 7.0).max(80.0) {
                    continue;
                }
                let [left_wall, right_wall] = [at(-39.0, -26.0) - apex, at(-39.0, 26.0) - apex];
                if left_wall.max(right_wall) < 4.2 || left_wall.min(right_wall) < -1.25 {
                    continue;
                }
                let toe_start = at(46.0, 0.0);
                let openness = (at(46.0, -27.0) - toe_start)
                    .abs()
                    .max((at(46.0, 27.0) - toe_start).abs());
                if openness > left_wall.max(right_wall).min(7.1) + 0.85 {
                    continue;
                }
                let length = 94.0 + fraction(mix(key ^ 0x772c)) * 16.0;
                let spread = 28.0 + fraction(mix(key ^ 0x1f6d)) * 13.0;
                let bend = (fraction(mix(key ^ 0x7771)) - 0.5) * 10.5;
                let left = at(45.0, -19.0);
                let right = at(45.0, 19.0);
                let drift = ((left - right) * 2.15).clamp(-11.0, 11.0);
                let normal = spread + bend.abs() + drift.abs() + 1.0;
                let [ax, az] = downhill.map(f32::abs);
                let extent = [
                    ax * length * 0.58 + az * normal,
                    az * length * 0.58 + ax * normal,
                ];
                let bounds = [
                    center[0] - extent[0],
                    center[1] - extent[1],
                    center[0] + extent[0],
                    center[1] + extent[1],
                ];
                if !(0..2).all(|j| {
                    bounds[j] >= origin[j] + 2.0
                        && bounds[j + 2] <= origin[j] + ALPINE_TILE_SIZE as f32 - 2.0
                }) {
                    continue;
                }
                if proofs == 6 {
                    return None;
                }
                proofs += 1;
                if !outwash_ground(world.sample(center[0], center[1]).surface) {
                    break;
                }
                let plan = Self {
                    center,
                    downhill,
                    bounds,
                    length,
                    inverse: 1.0 / length,
                    spread,
                    bend,
                    drift,
                    deposit: 0.92 + fraction(mix(key ^ 0xccf8)) * 0.51,
                    design: (mix(key ^ 0xbf19) % OutwashProfile::ALL.len() as u64) as u8,
                };
                if plan.prove(world, pulse) {
                    return Some(plan);
                }
            }
        }
        None
    }

    pub fn center(&self) -> [f32; 2] {
        self.center
    }

    pub fn downhill(&self) -> [f32; 2] {
        self.downhill
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    fn dimensions(&self, t: f32) -> (f32, f32) {
        let spread =
            5.0 + (self.spread - 5.0) * t * (1.53 - 0.53 * t) * self.profile().width_at(t).min(1.0);
        let middle = self.drift * t + self.bend * (4.0 * t * (1.0 - t)) * (2.0 * t - 0.5);
        (middle, spread)
    }

    pub fn profile(&self) -> OutwashProfile {
        OutwashProfile::ALL[self.design as usize]
    }

    fn sway(&self, t: f32) -> f32 {
        self.bend * (4.0 * t * (1.0 - t)) * (t - 0.47)
    }

    fn channels(&self, t: f32, middle: f32, spread: f32) -> [f32; 3] {
        self.profile()
            .strand_offsets(t)
            .map(|offset| middle + self.sway(t) + spread * offset)
    }

    fn prove(&self, world: &World, pulse: &mut impl FnMut()) -> bool {
        let mut previous = [f32::INFINITY; 3];
        let mut first = [0.0; 3];
        for i in 0..=18 {
            pulse();
            let t = i as f32 * (1.0 / 18.0);
            let forward = (t - 0.42) * self.length;
            let (middle, spread) = self.dimensions(t);
            let [sx, sz] = shifted(self.center, self.downhill, forward, middle);
            let center_height = world.elevation(sx, sz);
            for edge in [-0.82, -0.39, 0.39, 0.82] {
                let [x, z] = shifted(self.center, self.downhill, forward, middle + spread * edge);
                let g = world.sample(x, z);
                let relative = g.elevation - center_height;
                if !outwash_ground(g.surface) || !(-4.7..=6.5).contains(&relative) {
                    return false;
                }
            }
            for (branch, side) in self.channels(t, middle, spread).into_iter().enumerate() {
                let [x, z] = shifted(self.center, self.downhill, forward, side);
                let ground = world.sample(x, z);
                if !outwash_ground(ground.surface) || !(46.0..=103.0).contains(&ground.elevation) {
                    return false;
                }
                let level = self.overlay(x, z, ground).elevation;
                if level > previous[branch] + 0.81 {
                    return false;
                }
                if i == 0 {
                    first[branch] = level;
                }
                previous[branch] = level;
            }
        }
        first.iter().zip(previous).all(|(a, b)| a - b >= 7.5)
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let [left, near, right, far] = self.bounds;
        if !(left..right).contains(&x)
            || !(near..far).contains(&z)
            || !outwash_ground(ground.surface)
        {
            return ground;
        }
        let [dx, dz] = [x - self.center[0], z - self.center[1]];
        let [ax, az] = self.downhill;
        let t = (dx * ax + dz * az) * self.inverse + 0.42;
        if !(0.0..1.0).contains(&t) {
            return ground;
        }
        let side = dz * ax - dx * az;
        let (middle, spread) = self.dimensions(t);
        let edge = spread - (side - middle).abs();
        if edge <= 0.0 {
            return ground;
        }
        let ends = smoothstep(t * 12.0) * smoothstep((1.0 - t) * 7.5);
        let profile = self.profile();
        let thread = profile.thread(t, side - middle, spread, self.sway(t));
        let ground_support = smoothstep((ground.elevation - 46.0) * (1.0 / 5.0))
            * smoothstep((104.0 - ground.elevation) * (1.0 / 7.0));
        let apron = smoothstep(edge * 0.13) * ends * ground_support;
        let rise = (self.deposit * profile.deposit * (1.0 - thread) - 0.40 * thread) * apron;
        let surface = if apron > 0.20 && thread > 0.58 {
            Surface::Gravel
        } else if apron > 0.38 && thread > 0.14 && ground.surface != Surface::Rock {
            Surface::AlpineMeadow
        } else {
            ground.surface
        };
        Sample {
            elevation: ground.elevation + rise,
            surface,
        }
    }
}

fn outwash_ground(surface: Surface) -> bool {
    matches!(surface, Surface::Grass | Surface::Forest | Surface::Rock)
}

fn shifted(center: [f32; 2], [ax, az]: [f32; 2], forward: f32, side: f32) -> [f32; 2] {
    [
        center[0] + ax * forward - az * side,
        center[1] + az * forward + ax * side,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outwash_designs_have_distinct_relief_and_close_their_channels() {
        let designs = OutwashProfile::ALL.map(|profile| {
            assert_eq!(profile.strand_offsets(0.0), [0.0; 3]);
            assert_eq!(profile.strand_offsets(1.0), [0.0; 3]);
            [0.18, 0.31, 0.5, 0.67, 0.86].map(|t| {
                [-24.0, -16.0, -12.0, -6.0, 0.0, 6.0, 12.0, 16.0, 24.0].map(|side| {
                    let (gravel, relief) = profile.study(t, side);
                    ((gravel * 20.0) as i8, (relief * 20.0) as i8)
                })
            })
        });
        for (index, design) in designs.iter().enumerate() {
            assert!(
                !designs[..index].contains(design),
                "{}",
                OutwashProfile::ALL[index].name
            );
        }
    }

    #[test]
    fn the_gravel_threads_split_rejoin_and_follow_a_downhill_valley() {
        for (seed, [tx, tz]) in [(8, [3, 3]), (9, [-10, -2]), (10, [1, -9])] {
            let world = World::new(seed);
            let fan = OutwashFan::generate(&world, seed, tx, tz).unwrap();
            assert_eq!(Some(fan), OutwashFan::generate(&world, seed, tx, tz));
            let (middle, width) = fan.dimensions(0.47);
            let [left, trunk, right] = fan.channels(0.47, middle, width);
            assert!(trunk - left > 6.0 && right - trunk > 6.0);
            for end in [0.0, 1.0] {
                let (middle, spread) = fan.dimensions(end);
                let [left, trunk, right] = fan.channels(end, middle, spread);
                assert_eq!([left, right], [trunk; 2]);
            }
            let mut low = [f32::INFINITY; 3];
            let mut heights = [0.0; 3];
            let mut exposed = [0; 3];
            for step in 0..=90 {
                let t = step as f32 * (1.0 / 90.0);
                let (middle, width) = fan.dimensions(t);
                for (i, cross) in fan.channels(t, middle, width).into_iter().enumerate() {
                    let [x, z] = shifted(fan.center, fan.downhill, (t - 0.42) * fan.length, cross);
                    let ground = world.sample(x, z);
                    let wash = fan.overlay(x, z, ground);
                    if step == 0 {
                        heights[i] = wash.elevation;
                    }
                    assert!(wash.elevation <= low[i] + 0.47);
                    low[i] = wash.elevation;
                    exposed[i] += usize::from(wash.surface == Surface::Gravel);
                }
            }
            assert!(heights.into_iter().zip(low).all(|(a, b)| a - b > 7.5));
            assert!(exposed.into_iter().all(|n| n >= 56));
            let [left, near, right, far] = fan.bounds();
            let mut deposits = 0;
            let mut worn = 0;
            for row in -3..=((far - near) as i32 + 4) {
                for col in -3..=((right - left) as i32 + 4) {
                    let x = left + col as f32;
                    let z = near + row as f32;
                    let ground = world.sample(x, z);
                    let wash = fan.overlay(x, z, ground);
                    if ground != wash {
                        assert!(outwash_ground(ground.surface));
                        assert!((left..right).contains(&x) && (near..far).contains(&z));
                        deposits += usize::from(wash.elevation - ground.elevation > 0.5);
                        worn += usize::from(wash.elevation - ground.elevation < -0.18);
                    }
                }
            }
            assert!(deposits >= 450 && worn >= 250);
        }
    }
}
