//! Terminal moraine arcs on surveyed mountain valley floors.

use crate::alpine_lakes::ALPINE_TILE_SIZE;
use crate::math::{fraction, mix, sin_cos, smoothstep};
use crate::world::{Sample, Surface, World};

const S: f32 = 0.866_025_4;
const DIRECTIONS: [[f32; 2]; 12] = [
    [1.0, 0.0],
    [S, 0.5],
    [0.5, S],
    [0.0, 1.0],
    [-0.5, S],
    [-S, 0.5],
    [-1.0, 0.0],
    [-S, -0.5],
    [-0.5, -S],
    [0.0, -1.0],
    [0.5, -S],
    [S, -0.5],
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlacialMoraine {
    center: [f32; 2],
    downhill: [f32; 2],
    bounds: [f32; 4],
    inverse_span: f32,
    inverse_thickness: f32,
    bend: f32,
    skew: f32,
    outlet: [f32; 2],
    base_height: f32,
    relief: f32,
}

impl GlacialMoraine {
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
        let key = mix(seed ^ tile ^ 0xaf9e_7a42_b13c_58de);
        if key & 3 >= 2 {
            return None;
        }

        let origin = [tile_x, tile_z].map(|i| (i * ALPINE_TILE_SIZE) as f32);
        const GRID: usize = 8;
        const PROOF_LIMIT: u8 = 6;
        let first = ((key >> 20) & 63) as usize;
        let phase = [fraction(key) * 8.0, fraction(mix(key)) * 8.0];
        let mut proofs = 0;
        for i in 0..GRID * GRID {
            pulse();
            let cell = (first + i * 29) & 63;
            let center = [
                origin[0] + 63.0 + (cell % GRID) as f32 * 35.0 + phase[0],
                origin[1] + 63.0 + (cell / GRID) as f32 * 35.0 + phase[1],
            ];
            let ground = world.elevation(center[0], center[1]);
            if !(65.0..=105.0).contains(&ground) {
                continue;
            }
            let design = mix(key ^ (0x90f2 + cell as u64));
            let turn = fraction(mix(design ^ 0x7732)) * core::f32::consts::TAU;
            let span = 41.0 + 10.0 * fraction(mix(design ^ 0xffc1));
            let (sin, cos) = sin_cos(turn);
            for [axis_x, axis_z] in DIRECTIONS {
                let downhill = [cos * axis_x - sin * axis_z, sin * axis_x + cos * axis_z];
                let at = |forward, side| {
                    let [x, z] = local_point(center, downhill, forward, side);
                    world.elevation(x, z)
                };
                if at(27.0, 0.0) > ground - 1.2 || at(57.0, 0.0) > ground - 6.0 {
                    continue;
                }
                let upstream = at(-38.0, 0.0);
                if upstream < ground + 2.8
                    || upstream > ground + 15.0
                    || at(-77.0, 0.0) < ground + 7.0
                {
                    continue;
                }
                let walls = [at(-13.0, -span), at(-13.0, span)];
                if walls[0].min(walls[1]) < ground + 7.0 || walls[0].max(walls[1]) < 86.0 {
                    continue;
                }
                let bend = 18.0 + 10.0 * fraction(mix(design ^ 0x7139));
                let skew = (fraction(mix(design ^ 0x6823)) - 0.5) * 11.0;
                let thickness = 15.0 + 7.0 * fraction(mix(design ^ 0x3718));
                let normal_extent = span;
                let axis_extent = bend + skew.abs() + thickness;
                let extent = [
                    downhill[0].abs() * axis_extent + downhill[1].abs() * normal_extent,
                    downhill[1].abs() * axis_extent + downhill[0].abs() * normal_extent,
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
                if proofs == PROOF_LIMIT {
                    return None;
                }
                proofs += 1;
                let Some(outlet) = prove_outlet(at, pulse) else {
                    continue;
                };
                let plan = Self {
                    center,
                    downhill,
                    bounds,
                    inverse_span: 1.0 / span,
                    inverse_thickness: 1.0 / thickness,
                    bend,
                    skew,
                    outlet,
                    base_height: ground,
                    relief: 5.9 + 3.4 * fraction(mix(design ^ 0xbe32)),
                };
                if plan.prove_banks(at, ground, pulse) {
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

    fn forward(&self, transverse: f32) -> f32 {
        transverse * (self.skew - self.bend * transverse)
    }

    fn prove_banks(
        &self,
        height: impl Fn(f32, f32) -> f32,
        ground: f32,
        pulse: &mut impl FnMut(),
    ) -> bool {
        let span = 1.0 / self.inverse_span;
        let step = 0.42 / self.inverse_thickness;
        for side in [-1.0, 1.0] {
            for position in [0.38, 0.53, 0.69] {
                pulse();
                let u = side * position;
                let v = self.forward(u);
                let t = u * span;
                let h = height(v, t);
                if h < ground - 10.0
                    || h > ground + 27.0
                    || (height(v + step, t) - height(v - step, t)).abs() > 12.0
                    || (height(v, t - 4.5) - height(v, t + 4.5)).abs() > 9.0
                {
                    return false;
                }
            }
        }
        true
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let [left, near, right, far] = self.bounds;
        if !(left..right).contains(&x)
            || !(near..far).contains(&z)
            || !matches!(
                ground.surface,
                Surface::Grass | Surface::Forest | Surface::Rock
            )
        {
            return ground;
        }
        let [a, b] = self.downhill;
        let dx = x - self.center[0];
        let dz = z - self.center[1];
        let side = dz * a - dx * b;
        let cross = side * self.inverse_span;
        let forward = dx * a + dz * b;
        let from_outlet = (side - (self.outlet[0] + self.outlet[1] * forward)).abs();
        if cross.abs() >= 1.0 || from_outlet <= 11.0 {
            return ground;
        }
        let offset = (forward - self.forward(cross)) * self.inverse_thickness;
        if offset.abs() >= 1.0 {
            return ground;
        }
        let arc = smoothstep((1.0 - cross.abs()) * 3.35)
            * smoothstep((from_outlet - 11.0) * (1.0 / 12.0));
        // Meltwater reshapes the downhill face. Leave the upstream apron longer.
        let profile = smoothstep(1.0 - offset.abs()) * (1.0 - 0.24 * offset);
        let support = smoothstep((ground.elevation - 65.0) * (1.0 / 5.0))
            * smoothstep((self.base_height + 30.0 - ground.elevation) * (1.0 / 8.0));
        let amplitude = arc * profile * support;
        let rise = self.relief * amplitude;
        Sample {
            elevation: ground.elevation + rise,
            surface: if amplitude > 0.50 {
                Surface::Gravel
            } else {
                ground.surface
            },
        }
    }
}

fn local_point(center: [f32; 2], axis: [f32; 2], forward: f32, side: f32) -> [f32; 2] {
    [
        center[0] + axis[0] * forward - axis[1] * side,
        center[1] + axis[1] * forward + axis[0] * side,
    ]
}

// Keep the original meltwater path open through the crest and its aprons.
fn prove_outlet(height: impl Fn(f32, f32) -> f32, pulse: &mut impl FnMut()) -> Option<[f32; 2]> {
    let mut points = [[0.0; 2]; 11];
    for (i, point) in points.iter_mut().enumerate() {
        pulse();
        let forward = -30.0 + i as f32 * 7.5;
        *point = [0.0, f32::INFINITY];
        for cross in -8..=8 {
            let side = cross as f32 * 3.0;
            let h = height(forward, side);
            if h < point[1] {
                *point = [side, h];
            }
        }
    }
    let outlet = points[4][0];
    let slope = (points[6][0] - points[2][0]) * (1.0 / 30.0);
    if outlet.abs() > 12.0
        || slope.abs() > 0.4
        || points[0][1] - points[10][1] < 8.2
        || points.windows(2).any(|w| w[1][1] > w[0][1] + 0.65)
    {
        return None;
    }
    for (i, [side, _]) in points[..=8].iter().enumerate() {
        let estimate = outlet + slope * (-30.0 + i as f32 * 7.5);
        if (side - estimate).abs() > 6.0 || side.abs() > 21.0 {
            return None;
        }
    }
    Some([outlet, slope])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_moraine_lobes_leave_the_surveyed_outlet_untouched() {
        for (seed, [tile_x, tile_z]) in [(1, [2, -1]), (7, [-1, -5]), (9, [3, 1])] {
            let world = World::new(seed);
            let plan = GlacialMoraine::generate(&world, seed, tile_x, tile_z).unwrap();
            assert_eq!(
                Some(plan),
                GlacialMoraine::generate(&world, seed, tile_x, tile_z)
            );
            let [left, near, right, far] = plan.bounds();
            let mut gravel = [0; 2];
            let mut raised = [0.0_f32; 2];
            for row in -6..=(((far - near) * 2.0) as i32 + 7) {
                for col in -6..=(((right - left) * 2.0) as i32 + 7) {
                    let x = left + col as f32 * 0.5;
                    let z = near + row as f32 * 0.5;
                    let before = world.sample(x, z);
                    let after = plan.overlay(x, z, before);
                    let [ax, az] = plan.downhill;
                    let dx = x - plan.center[0];
                    let dz = z - plan.center[1];
                    let channel =
                        dz * ax - dx * az - (plan.outlet[0] + plan.outlet[1] * (dx * ax + dz * az));
                    if !((left..right).contains(&x) && (near..far).contains(&z))
                        || channel.abs() <= 10.9
                        || !matches!(
                            before.surface,
                            Surface::Grass | Surface::Forest | Surface::Rock
                        )
                    {
                        assert_eq!(before, after);
                    }
                    let side = usize::from(channel > 0.0);
                    assert!(after.elevation >= before.elevation);
                    raised[side] = raised[side].max(after.elevation - before.elevation);
                    gravel[side] += usize::from(after.surface == Surface::Gravel);
                }
            }
            assert!(gravel.into_iter().all(|pixels| pixels > 120));
            assert!(raised.into_iter().all(|rise| (3.5..=9.6).contains(&rise)));
        }
    }
}
