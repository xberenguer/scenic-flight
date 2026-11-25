//! Grass-capped bluffs fitted to existing seaward points.

use crate::ground_details::ParcelSurvey;
use crate::math::{fraction, mix, smoothstep};
use crate::world::{Sample, Surface, World, SEA_LEVEL};

const KNOTS: usize = 5;
const BACK: f32 = 65.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CoastalBluff {
    seaward: [f32; 2],
    axis: [f32; 2],
    inverse_length: f32,
    shores: [f32; KNOTS],
    height: f32,
    oblique: f32,
}

pub(crate) struct BluffSampler {
    pub(crate) center: [f32; 2],
    pub(crate) bounds: [f32; 4],
    pub(crate) plan: CoastalBluff,
}

impl BluffSampler {
    #[inline]
    pub(crate) fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let [left, near, right, far] = self.bounds;
        if !(left <= x && x < right && near <= z && z < far) {
            return ground;
        }
        match ground.surface {
            Surface::Sand => {}
            Surface::Grass | Surface::Forest if ground.elevation <= 64.0 => {}
            _ => return ground,
        }
        self.plan.overlay([x - self.center[0], z - self.center[1]], ground)
    }
}

impl CoastalBluff {
    pub(crate) fn survey(
        world: &World,
        seed: u64,
        origin: [f32; 2],
        key: u64,
        mut accept: impl FnMut([f32; 2], [f32; 4], Self) -> bool,
        pulse: &mut impl FnMut(),
    ) {
        if key & 3 != 0 {
            return;
        }
        const SIDE: u64 = 9;
        let start = (key >> 18) % (SIDE * SIDE);
        let jitter = [
            fraction(mix(key ^ 61)) * 11.0,
            fraction(mix(key ^ 79)) * 11.0,
        ];
        let mut proved = 0;
        for step in 0..SIDE * SIDE {
            pulse();
            let place = (start + step * 23) % (SIDE * SIDE);
            let hint = [
                origin[0] + 65.0 + (place % SIDE) as f32 * 46.0 + jitter[0],
                origin[1] + 65.0 + (place / SIDE) as f32 * 46.0 + jitter[1],
            ];
            if !(-0.053..=0.114).contains(&world.coast_signal(hint[0], hint[1])) {
                continue;
            }
            let gradient = [0, 1].map(|side| {
                let mut land = hint;
                let mut water = hint;
                land[side] -= 11.0;
                water[side] += 11.0;
                world.coast_signal(land[0], land[1]) - world.coast_signal(water[0], water[1])
            });
            let norm = gradient[0] * gradient[0] + gradient[1] * gradient[1];
            if norm < 0.0017 {
                continue;
            }
            let factor = inverse_root(norm);
            let seaward = gradient.map(|v| v * factor);
            let axis = [-seaward[1], seaward[0]];
            let design = mix(key ^ place ^ 0x44b8_fd87);
            let length = 54.0 + fraction(mix(design ^ 321)) * 19.0;
            let coord = |u: f32, v: f32| [0, 1].map(|i| hint[i] + axis[i] * u + seaward[i] * v);
            let levels = [-1.0, -0.5, 0.0, 0.5, 1.0].map(|u| {
                find_shore(|v| {
                    let [x, z] = coord(u * length, v);
                    world.coast_signal(x, z)
                })
            });
            let [Some(a), Some(b), Some(tip), Some(c), Some(d)] = levels else {
                continue;
            };
            if !(10.0..=37.0).contains(&(tip - (a + d) * 0.5))
                || a.max(d) > tip - 6.0
                || b.max(c) > tip + 4.0
                || [(a - b), (b - tip), (tip - c), (c - d)]
                    .iter()
                    .any(|delta| delta.abs() > 26.0)
            {
                continue;
            }
            let [x, z] = coord(0.0, tip - 71.0);
            if world.elevation(x, z) < 9.0 {
                continue;
            }
            let center = coord(0.0, tip);
            let bluff = Self {
                seaward,
                axis,
                inverse_length: 1.0 / length,
                shores: [a - tip, b - tip, 0.0, c - tip, d - tip],
                height: 5.4 + fraction(mix(design ^ 57)) * 2.9,
                oblique: fraction(mix(design ^ 13)) * 2.0 - 1.0,
            };
            let bounds = bluff.bounds(center);
            if !(0..2).all(|i| bounds[i] > origin[i] + 2.0 && bounds[i + 2] < origin[i] + 510.0) {
                continue;
            }
            proved += 1;
            if bluff.prove_ground(world, seed, center, pulse)
                && crate::cove_dunes::clears_fixed_sites(world, seed, bounds, pulse)
                && accept(center, bounds, bluff)
            {
                return;
            }
            if proved == 4 {
                return;
            }
        }
    }

    pub(crate) fn seaward(self) -> [f32; 2] {
        self.seaward
    }

    fn point(self, center: [f32; 2], along: f32, out: f32) -> [f32; 2] {
        let along = along / self.inverse_length;
        [0, 1].map(|i| center[i] + self.axis[i] * along + self.seaward[i] * out)
    }

    fn bounds(self, center: [f32; 2]) -> [f32; 4] {
        let mut bounds = [center[0], center[1], center[0], center[1]];
        for (index, level) in self.shores.into_iter().enumerate() {
            let u = index as f32 * 0.5 - 1.0;
            for v in [level - BACK, level] {
                let [x, z] = self.point(center, u, v);
                bounds = [
                    bounds[0].min(x),
                    bounds[1].min(z),
                    bounds[2].max(x),
                    bounds[3].max(z),
                ];
            }
        }
        [
            bounds[0] - 1.0,
            bounds[1] - 1.0,
            bounds[2] + 1.0,
            bounds[3] + 1.0,
        ]
    }

    fn shore(self, along: f32) -> f32 {
        let q = (along + 1.0) * 2.0;
        let index = (q as usize).min(KNOTS - 2);
        let t = q - index as f32;
        self.shores[index] + (self.shores[index + 1] - self.shores[index]) * t
    }

    fn prove_ground(
        self,
        world: &World,
        seed: u64,
        center: [f32; 2],
        pulse: &mut impl FnMut(),
    ) -> bool {
        let mut fields = ParcelSurvey::new();
        for step in -11_i32..=11 {
            pulse();
            let u = step as f32 / 11.0;
            let shore = self.shore(u);
            let [wx, wz] = self.point(center, u, shore + 5.0);
            if world.coast_signal(wx, wz) >= 0.0 {
                return false;
            }
            for d in [4.5, 9.0, 17.0, 26.0, 36.0, 47.0, 57.0, BACK] {
                let [x, z] = self.point(center, u, shore - d);
                let raw = world.sample(x, z);
                if raw.elevation <= SEA_LEVEL + 0.11
                    || raw.elevation > 23.0
                    || !matches!(
                        raw.surface,
                        Surface::Sand | Surface::Grass | Surface::Forest
                    )
                    || (raw.surface == Surface::Grass && fields.occupies_grass(world, seed, x, z))
                {
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn overlay(&self, [x, z]: [f32; 2], ground: Sample) -> Sample {
        if ground.elevation <= SEA_LEVEL + 0.11
            || ground.elevation > 23.0
            || !matches!(
                ground.surface,
                Surface::Sand | Surface::Grass | Surface::Forest
            )
        {
            return ground;
        }
        let u = (x * self.axis[0] + z * self.axis[1]) * self.inverse_length;
        if u.abs() >= 1.0 {
            return ground;
        }
        let out = x * self.seaward[0] + z * self.seaward[1];
        let inland = self.shore(u) - out;
        if !(2.5..BACK).contains(&inland) {
            return ground;
        }
        let flank = smoothstep((1.0 - u.abs()) * 1.95);
        let face_start = (2.5 + 3.8 * u * u + self.oblique * u).max(2.5);
        let rise = self.height
            * flank
            * smoothstep((inland - face_start) * 0.16)
            * smoothstep((BACK - inland) * 0.024)
            * smoothstep((ground.elevation - 0.11) * 2.0)
            * smoothstep((23.0 - ground.elevation) * 0.15);
        if rise <= 0.0 {
            return ground;
        }
        let rock_edge = 13.5 + 4.0 * (1.0 - u * u) + u * self.oblique * 3.0;
        let surface = if rise > 1.30 && inland < rock_edge {
            Surface::Rock
        } else if rise > 2.50 && inland < 36.0 {
            Surface::Grass
        } else {
            ground.surface
        };
        Sample {
            elevation: ground.elevation + rise,
            surface,
        }
    }
}

fn find_shore(mut coast: impl FnMut(f32) -> f32) -> Option<f32> {
    let mut dry = -68.0;
    let mut wet = 68.0;
    if coast(dry) <= 0.01 || coast(wet) >= -0.016 {
        return None;
    }
    for _ in 0..10 {
        let middle = (dry + wet) * 0.5;
        if coast(middle) > 0.0 {
            dry = middle;
        } else {
            wet = middle;
        }
    }
    Some((dry + wet) * 0.5)
}

fn inverse_root(value: f32) -> f32 {
    let first = f32::from_bits(0x5f37_59df - (value.to_bits() >> 1));
    let next = first * (1.5 - 0.5 * value * first * first);
    next * (1.5 - 0.5 * value * next * next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surveyed_bluff_is_a_continuous_attached_dry_landform() {
        let world = World::new(0);
        let tile = (u64::from(-3_i32 as u32) << 32) | u64::from(-14_i32 as u32);
        let mut selected = None;
        CoastalBluff::survey(
            &world,
            0,
            [-1536.0, -7168.0],
            mix(tile ^ 0xf902_627a_e1ec_5c81),
            |center, bounds, plan| {
                selected = Some((center, bounds, plan));
                true
            },
            &mut || {},
        );
        let (center, bounds, plan) = selected.unwrap();
        let mut wet = 0;
        let mut grass = 0;
        let mut rock = 0;
        let mut previous = [0.0_f32; 333];
        for along in -100_i32..=100 {
            let u = along as f32 * 0.01;
            let mut last = 0.0_f32;
            for (index, across) in (-64..=268).enumerate() {
                let d = across as f32 * 0.25;
                let [x, z] = plan.point(center, u, plan.shore(u) - d);
                let ground = world.sample(x, z);
                let formed = plan.overlay([x - center[0], z - center[1]], ground);
                let rise = formed.elevation - ground.elevation;
                if ground.elevation == SEA_LEVEL
                    || d <= -0.25
                    || d >= BACK + 0.25
                    || along.abs() == 100
                {
                    assert_eq!(ground, formed);
                }
                if rise > 0.0 {
                    assert!(
                        (bounds[0]..bounds[2]).contains(&x) && (bounds[1]..bounds[3]).contains(&z)
                    );
                }
                assert!((rise - last).abs() < 0.64, "seaward edge break");
                assert!(
                    (rise - previous[index]).abs() < 0.33,
                    "change along shoreline"
                );
                last = rise;
                previous[index] = rise;
                wet += usize::from(ground.elevation == SEA_LEVEL);
                rock += usize::from(formed.surface == Surface::Rock);
                grass += usize::from(rise > 1.0 && formed.surface == Surface::Grass);
            }
        }
        assert!(wet > 5_000 && rock > 4_000 && grass > 8_000);
    }
}
