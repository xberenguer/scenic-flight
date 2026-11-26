//! Eroded caprock remnants on surveyed dry plateau shoulders.

use crate::dry_landmarks::DRY_LANDMARK_TILE;
use crate::math::{fraction, mix, sin_cos, smoothstep};
use crate::world::{Sample, Surface, World};

mod profiles;
pub use profiles::MesaProfile;

const AXES: [[f32; 2]; 8] = [
    [1.0, 0.0],
    [0.707_106_77, 0.707_106_77],
    [0.0, 1.0],
    [-0.707_106_77, 0.707_106_77],
    [-1.0, 0.0],
    [-0.707_106_77, -0.707_106_77],
    [0.0, -1.0],
    [0.707_106_77, -0.707_106_77],
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct Remnant {
    center: [f32; 2],
    axis: [f32; 2],
    inverse: [f32; 2],
    level: f32,
    tilt: f32,
    corners: f32,
    profile: MesaProfile,
}

impl Remnant {
    fn bounds(self) -> [f32; 4] {
        let [ax, az] = self.axis.map(f32::abs);
        let [span, width] = self.inverse.map(|value| 1.0 / value);
        let [x, z] = self.center;
        let xr = ax * span + az * width;
        let zr = az * span + ax * width;
        [x - xr, z - zr, x + xr, z + zr]
    }

    fn local(&self, x: f32, z: f32) -> [f32; 2] {
        let [dx, dz] = [x - self.center[0], z - self.center[1]];
        let [ax, az] = self.axis;
        [
            (dx * ax + dz * az) * self.inverse[0],
            (dz * ax - dx * az) * self.inverse[1],
        ]
    }

    fn radius(&self, u: f32, v: f32) -> f32 {
        let [a, b] = [u.abs(), v.abs()];
        a.max(b) + self.corners * a.min(b)
    }

    fn prove_foot(&self, world: &World, center_height: f32, pulse: &mut impl FnMut()) -> bool {
        let [width, depth] = self.inverse.map(|inverse| 1.0 / inverse);
        for row in -4..=4 {
            pulse();
            let u = row as f32 * 0.25;
            for col in -4..=4 {
                let v = col as f32 * 0.25;
                if self.radius(u, v) > 1.02 {
                    continue;
                }
                let [x, z] = shifted(self.center, self.axis, width * u, depth * v);
                let ground = world.sample(x, z);
                let delta = ground.elevation - center_height;
                if ground.surface != Surface::Desert
                    || !(-6.0..=5.0).contains(&delta)
                    || self.level - ground.elevation > 20.0
                {
                    return false;
                }
            }
        }
        true
    }

    fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let [u, v] = self.local(x, z);
        let radius = self.radius(u, v);
        if radius >= 1.0 {
            return ground;
        }
        let top = self.level + self.tilt * u + 0.26 * v;
        let (relief, crown) = self.profile.relief(u, v, radius);
        let support = smoothstep((ground.elevation - 15.0) * (1.0 / 3.0))
            * smoothstep((61.0 - ground.elevation) * (1.0 / 3.0));
        let rise =
            (top - ground.elevation).clamp(0.0, 20.0) * relief * support;
        let surface = MesaProfile::material(relief, crown, rise).unwrap_or(ground.surface);
        Sample { elevation: ground.elevation + rise, surface }
    }
}

impl MesaProfile {
    fn relief(self, u: f32, v: f32, radius: f32) -> (f32, f32) {
        let apron = smoothstep((1.0 - radius) * (1.0 / 0.48));
        let crown = self.crown(u, v, radius);
        let rim = smoothstep((1.0 - radius) * (1.0 / 0.19));
        (0.25 * apron + 0.75 * crown * rim, crown)
    }

    fn material(relief: f32, crown: f32, rise: f32) -> Option<Surface> {
        if rise <= 0.24 { return None; }
        Some(if crown >= 0.82 && relief > 0.78 {
            Surface::MesaCap
        } else if crown > 0.29 && rise >= 1.9 {
            Surface::MesaCliff
        } else {
            Surface::MesaApron
        })
    }

    /// Sample the same normalized relief and materials used by surveyed remnants.
    pub fn study(self, u: f32, v: f32) -> Option<(f32, Surface)> {
        let [a, b] = [u.abs(), v.abs()];
        let radius = a.max(b) + 0.28 * a.min(b);
        if radius >= 1.0 { return None; }
        let (relief, crown) = self.relief(u, v, radius);
        Self::material(relief, crown, 14.0 * relief).map(|surface| (relief, surface))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaprockMesa {
    remnants: [Remnant; 2],
    bounds: [f32; 4],
    descent: [f32; 2],
}

impl CaprockMesa {
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
        let key = mix(seed ^ tile ^ 0x882c_57ef_97a1_1295);
        if key & 7 >= 2 {
            return None;
        }
        let origin = [tile_x, tile_z].map(|i| (i * DRY_LANDMARK_TILE) as f32);
        const SIDE: usize = 7;
        let shift = [fraction(key) * 10.0, fraction(mix(key)) * 10.0];
        let first = ((key >> 24) as usize) % (SIDE * SIDE);
        let mut proofs = 0;
        for i in 0..SIDE * SIDE {
            pulse();
            let cell = (first + i * 18) % (SIDE * SIDE);
            let center = [
                origin[0] + 80.0 + (cell % SIDE) as f32 * 57.0 + shift[0],
                origin[1] + 80.0 + (cell / SIDE) as f32 * 57.0 + shift[1],
            ];
            let sample = world.sample(center[0], center[1]);
            let floor = sample.elevation;
            if sample.surface != Surface::Desert || !(22.0..=53.0).contains(&floor) {
                continue;
            }
            let key = mix(key ^ (cell as u64 + 0x2b1c));
            let orientation = ((key >> 33) & 7) as usize;
            for step in 0..AXES.len() {
                let descent = AXES[(orientation + step) & 7];
                let down = shifted(center, descent, 64.0, 0.0);
                let below = world.sample(down[0], down[1]);
                if below.surface != Surface::Desert
                    || !(7.0..=20.0).contains(&(floor - below.elevation))
                {
                    continue;
                }
                let up = shifted(center, descent, -52.0, 0.0);
                let higher = world.elevation(up[0], up[1]) - floor;
                if !(2.5..=16.0).contains(&higher) {
                    continue;
                }
                let strike = (fraction(mix(key ^ 0x3327)) - 0.5) * 0.59;
                let (sine, cosine) = sin_cos(strike);
                let axis = [
                    descent[0] * cosine - descent[1] * sine,
                    descent[1] * cosine + descent[0] * sine,
                ];
                let level = floor + 9.75 + 4.8 * fraction(mix(key ^ 0x775b));
                let [main_profile, companion_profile] = MesaProfile::pair(mix(key ^ 0xc714));
                let mesa = Remnant {
                    center,
                    axis,
                    inverse: [
                        1.0 / (19.0 + 6.0 * fraction(mix(key ^ 0x7489))),
                        1.0 / (28.0 + 9.0 * fraction(mix(key ^ 0xafc8))),
                    ],
                    level,
                    tilt: (fraction(mix(key ^ 0x7832)) - 0.5) * 1.16,
                    corners: 0.21 + 0.15 * fraction(mix(key ^ 0xfe86)),
                    profile: main_profile,
                };
                let first_bank = if key & 1 == 0 { -1.0 } else { 1.0 };
                for side in [first_bank, -first_bank] {
                    let center = shifted(
                        center,
                        axis,
                        9.0 + 8.0 * fraction(mix(key ^ 0x8628)),
                        side * 61.0,
                    );
                    let ground = world.sample(center[0], center[1]);
                    let height = floor - ground.elevation;
                    if ground.surface != Surface::Desert || !(-2.8..=6.7).contains(&height) {
                        continue;
                    }
                    let butte_level = level - 1.2 - 4.7 * fraction(mix(key ^ 0x3ab0));
                    if !(5.5..=15.8).contains(&(butte_level - ground.elevation)) {
                        continue;
                    }
                    let butte = Remnant {
                        center,
                        axis,
                        inverse: [
                            1.0 / (12.0 + 6.0 * fraction(mix(key ^ 0xb887))),
                            1.0 / (13.0 + 4.0 * fraction(mix(key ^ 0xf326))),
                        ],
                        level: butte_level,
                        tilt: mesa.tilt * -0.5,
                        corners: 0.29 + 0.17 * fraction(mix(key ^ 0x8772)),
                        profile: companion_profile,
                    };
                    let [a, b, c, d] = mesa.bounds();
                    let [e, f, g, h] = butte.bounds();
                    let bounds = [a.min(e), b.min(f), c.max(g), d.max(h)];
                    if !(0..2).all(|j| {
                        bounds[j] >= origin[j] + 4.0
                            && bounds[j + 2] <= origin[j] + DRY_LANDMARK_TILE as f32 - 4.0
                    }) {
                        continue;
                    }
                    if proofs == 5 {
                        return None;
                    }
                    proofs += 1;
                    if mesa.prove_foot(world, floor, pulse)
                        && butte.prove_foot(world, ground.elevation, pulse)
                    {
                        return Some(Self {
                            remnants: [mesa, butte],
                            bounds,
                            descent,
                        });
                    }
                }
            }
        }
        None
    }

    pub fn centers(&self) -> [[f32; 2]; 2] {
        self.remnants.map(|remnant| remnant.center)
    }

    pub fn profiles(&self) -> [MesaProfile; 2] {
        self.remnants.map(|remnant| remnant.profile)
    }

    pub fn downhill(&self) -> [f32; 2] {
        self.descent
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let [left, near, right, far] = self.bounds;
        if !matches!(ground.surface, Surface::Desert | Surface::Grass)
            || !(15.0..=61.0).contains(&ground.elevation)
            || !(left..right).contains(&x)
            || !(near..far).contains(&z)
        {
            return ground;
        }
        let mesa = self.remnants[0].overlay(x, z, ground);
        if mesa == ground {
            self.remnants[1].overlay(x, z, ground)
        } else {
            mesa
        }
    }
}

fn shifted(origin: [f32; 2], [ax, az]: [f32; 2], along: f32, across: f32) -> [f32; 2] {
    [
        origin[0] + ax * along - az * across,
        origin[1] + az * along + ax * across,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_remnants_keep_dry_caps_and_the_saddle_open() {
        for (seed, [tx, tz]) in [(1, [1, -1]), (1, [-4, -12]), (2, [1, 5])] {
            let world = World::new(seed);
            let mesa = CaprockMesa::generate(&world, seed, tx, tz).unwrap();
            assert_eq!(CaprockMesa::generate(&world, seed, tx, tz), Some(mesa));
            let [a, b] = mesa.centers();
            let [x, z] = [0, 1].map(|i| a[i] + (b[i] - a[i]) * 0.66);
            let saddle = world.sample(x, z);
            assert_eq!(mesa.overlay(x, z, saddle), saddle);

            for remnant in mesa.remnants {
                let [x, z] = remnant.center;
                let raw = world.sample(x, z);
                let fill = mesa.overlay(x, z, raw);
                assert!(fill.elevation > raw.elevation + 5.5);
                assert_eq!(fill.surface, Surface::MesaCap);
                for along in [-5.0, 0.0, 5.0] {
                    for across in [-5.0, 0.0, 5.0] {
                        let [x, z] = shifted(remnant.center, remnant.axis, along, across);
                        let raw = world.sample(x, z);
                        let lift = mesa.overlay(x, z, raw);
                        assert_eq!(raw.surface, Surface::Desert);
                        assert!(lift.elevation >= raw.elevation);
                        assert!(lift.elevation <= remnant.level + 0.80);
                    }
                }
            }
            let [left, near, right, far] = mesa.bounds();
            let mut altered = 0;
            for row in -8..=((far - near) * 2.0) as i32 + 9 {
                for col in -8..=((right - left) * 2.0) as i32 + 9 {
                    let x = left + col as f32 * 0.5;
                    let z = near + row as f32 * 0.5;
                    let raw = world.sample(x, z);
                    let fill = mesa.overlay(x, z, raw);
                    if raw != fill {
                        assert!((left..right).contains(&x) && (near..far).contains(&z));
                        assert_eq!(raw.surface, Surface::Desert);
                        assert!(
                            fill.elevation >= raw.elevation
                                && fill.elevation - raw.elevation <= 20.0
                        );
                        altered += 1;
                    }
                }
            }
            assert!(altered > 6000);
        }
    }

    #[test]
    fn all_profiles_make_distinct_heightfields_and_blend_to_the_footprint_edge() {
        use std::collections::HashSet;

        let mut profiles = HashSet::new();
        for index in 0..MesaProfile::COUNT {
            let profile = MesaProfile::from_index(index).unwrap();
            let mut levels = [0_u8; 25 * 25];
            for (point, level) in levels.iter_mut().enumerate() {
                let u = (point % 25) as f32 / 12.0 - 1.0;
                let v = (point / 25) as f32 / 12.0 - 1.0;
                let [a, b] = [u.abs(), v.abs()];
                let rim = a.max(b) + 0.28 * a.min(b);
                if let Some((lift, surface)) = profile.study(u, v) {
                    assert!(rim < 1.0 && (0.0..=1.0).contains(&lift));
                    assert!(surface.sandstone_color().is_some());
                    *level = (lift * 18.0) as u8;
                } else if rim < 1.0 {
                    assert!(profile.relief(u, v, rim).0 < 0.018);
                }
            }
            assert!(levels.iter().any(|&height| height >= 16), "{}", profile.name());
            assert!(profiles.insert(levels), "Duplicate profile {}", profile.name());
            assert_eq!(profile.relief(1.0, 0.0, 1.0).0, 0.0);
        }
        assert_eq!(profiles.len(), 25);
    }
}
