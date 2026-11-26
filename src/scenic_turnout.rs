//! Retained roadside pull-offs above real downhill vistas.

use crate::ground_details::ParcelSurvey;
use crate::math::{fraction, mix, smoothstep};
use crate::roads::{RoadNetwork, ROAD_HALF_WIDTH, ROAD_REGION_SIZE};
use crate::world::{Sample, Surface, World};

const INNER: f32 = ROAD_HALF_WIDTH - 0.22;
const WALL_HALF: f32 = 0.84;

#[derive(Clone, Copy, Debug)]
pub struct ScenicTurnout {
    center: [f32; 3],
    axis: [f32; 2],
    side: f32,
    grade: f32,
    half_length: f32,
    inverse_half_length: f32,
    width: f32,
    wall: Surface,
    bounds: [f32; 4],
}

impl ScenicTurnout {
    pub(crate) fn survey(
        world: &World,
        seed: u64,
        region: [i32; 2],
        road: &RoadNetwork,
        mut reserved: impl FnMut(f32, f32) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        let tile = (u64::from(region[0] as u32) << 32) | u64::from(region[1] as u32);
        let key = mix(seed ^ tile ^ 0xb793_d280_e4f1_7bd6);
        if key & 3 != 0 {
            return None;
        }
        let points = road.polyline();
        let count = points.len().checked_sub(6)?;
        let origin = region.map(|axis| axis as f32 * ROAD_REGION_SIZE as f32);
        let mut parcels = ParcelSurvey::new();
        for candidate in 0..count.min(14) {
            pulse();
            let index = 3 + ((key >> 21) as usize + candidate) % count;
            let [a, b] = [points[index - 1], points[index]];
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let length = distance(dx, dz);
            if length < 20.0 || (a.elevation - b.elevation).abs() > length * 0.082 {
                continue;
            }
            let [x, z] = [(a.x + b.x) * 0.5, (a.z + b.z) * 0.5];
            let h = world.elevation(x, z);
            if !(13.0..=64.0).contains(&h) {
                continue;
            }
            let [ax, az] = [dx / length, dz / length];
            let half_length = (length * 0.46).min(16.0);
            let width = 7.1 + 2.8 * fraction(mix(key ^ index as u64));
            let wall = if key & 16 == 0 {
                Surface::Rock
            } else {
                Surface::ScreeCoarse
            };
            for side in [-1.0, 1.0] {
                let downhill = |span| world.elevation(x - az * side * span, z + ax * side * span);
                let near = downhill(30.0);
                let far = downhill(76.0);
                if h - near < 2.2 || h - far < 8.0 || far > near - 2.0 {
                    continue;
                }
                let mut plan = Self {
                    center: [x, h, z],
                    axis: [ax, az],
                    side,
                    grade: (b.elevation - a.elevation) / length,
                    half_length,
                    inverse_half_length: 1.0 / half_length,
                    width,
                    wall,
                    bounds: [
                        f32::INFINITY,
                        f32::INFINITY,
                        f32::NEG_INFINITY,
                        f32::NEG_INFINITY,
                    ],
                };
                for u in [-half_length, half_length] {
                    for v in [INNER, INNER + width + 2.0] {
                        let [px, pz] = plan.place(u, v);
                        plan.bounds[0] = plan.bounds[0].min(px - 0.2);
                        plan.bounds[1] = plan.bounds[1].min(pz - 0.2);
                        plan.bounds[2] = plan.bounds[2].max(px + 0.2);
                        plan.bounds[3] = plan.bounds[3].max(pz + 0.2);
                    }
                }
                if (0..2).all(|axis| {
                    plan.bounds[axis] >= origin[axis] + 4.0
                        && plan.bounds[axis + 2] < origin[axis] + ROAD_REGION_SIZE as f32 - 4.0
                }) && plan.fits(world, seed, road, &mut parcels, &mut reserved, &mut pulse)
                {
                    return Some(plan);
                }
            }
        }
        None
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub fn road_contact(&self) -> [f32; 3] {
        self.center
    }

    pub fn downhill(&self) -> [f32; 2] {
        [-self.axis[1] * self.side, self.axis[0] * self.side]
    }

    fn place(&self, along: f32, across: f32) -> [f32; 2] {
        let [x, _, z] = self.center;
        let [dx, dz] = self.downhill();
        [
            x + self.axis[0] * along + dx * across,
            z + self.axis[1] * along + dz * across,
        ]
    }

    fn outer(&self, along: f32) -> f32 {
        let t = along * self.inverse_half_length;
        INNER + self.width * (1.0 - t * t).max(0.0)
    }

    fn deck(&self, along: f32) -> f32 {
        self.center[1] + self.grade * along - 0.08
    }

    fn fits(
        &self,
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        parcels: &mut ParcelSurvey,
        reserved: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        for row in -14..=14 {
            pulse();
            let along = self.half_length * row as f32 / 14.0;
            let outer = self.outer(along);
            let deck = self.deck(along);
            for col in 0..=14 {
                let across = INNER + (outer + 1.65 - INNER) * col as f32 / 14.0;
                let [x, z] = self.place(along, across);
                let raw = world.sample(x, z);
                if raw.elevation < 6.0
                    || !matches!(
                        raw.surface,
                        Surface::Grass | Surface::Forest | Surface::Rock | Surface::Desert
                    )
                    || !(-2.3..=1.45).contains(&(raw.elevation - deck))
                    || reserved(x, z)
                    || (across > ROAD_HALF_WIDTH + 1.4
                        && road.proximity(x, z).is_some_and(|hit| {
                            hit.distance_squared < ROAD_HALF_WIDTH * ROAD_HALF_WIDTH
                        }))
                    || (raw.surface == Surface::Grass && parcels.occupies_grass(world, seed, x, z))
                {
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn sample(&self, x: f32, z: f32, raw: Sample) -> Option<Sample> {
        let [left, near, right, far] = self.bounds;
        if !(left..=right).contains(&x) || !(near..=far).contains(&z) || raw.elevation < 6.0 {
            return None;
        }
        let [dx, dz] = [x - self.center[0], z - self.center[2]];
        let [ax, az] = self.axis;
        let along = dx * ax + dz * az;
        let across = self.side * (dz * ax - dx * az);
        if along.abs() >= self.half_length || across < INNER {
            return None;
        }
        let end = along.abs() * self.inverse_half_length;
        let edge = across - self.outer(along);
        let deck = self.deck(along);
        if end < 0.68 && edge.abs() < WALL_HALF {
            let cap = 0.88 * smoothstep((0.68 - end) * 11.0);
            return Some(Sample {
                elevation: deck + cap,
                surface: self.wall,
            });
        }
        if edge <= 0.0 {
            let grade = smoothstep((across - INNER) * 0.58);
            return Some(Sample {
                elevation: raw.elevation + (deck - raw.elevation) * grade,
                surface: if across < INNER + 1.9 || end > 0.73 {
                    Surface::Pavement
                } else {
                    Surface::Gravel
                },
            });
        }
        if end < 0.69 && edge < 1.65 {
            let taper = smoothstep((1.65 - edge) * 1.24) * smoothstep((0.69 - end) * 12.0);
            return Some(Sample {
                elevation: raw.elevation + (deck - raw.elevation).max(0.0) * taper,
                surface: Surface::ScreeCoarse,
            });
        }
        None
    }
}

fn distance(x: f32, z: f32) -> f32 {
    let squared = x * x + z * z;
    let mut root = x.abs().max(z.abs());
    for _ in 0..3 {
        root = 0.5 * (root + squared / root);
    }
    root
}
