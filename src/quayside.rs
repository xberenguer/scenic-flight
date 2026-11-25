//! A short waterfront street extending a prepared harbor apron along dry shore.

use crate::ground_details::ParcelSurvey;
use crate::math::{mix, smoothstep};
use crate::ports::{Port, PortKind, PortSurface};
use crate::world::{Sample, Surface, World};

const KNOTS: usize = 9;
const STEP: f32 = 8.5;
const WALK_HALF: f32 = 2.6;
const REACH: f32 = (KNOTS - 1) as f32 * STEP;

#[derive(Clone, Copy, Debug)]
struct Workshop {
    center: [f32; 2],
    base: f32,
    rise: f32,
    id: u8,
}

/// Built only as an extension from an existing terminal's paved apron.
#[derive(Clone, Copy, Debug)]
pub struct Quayside {
    contact: [f32; 2],
    sea: [f32; 2],
    lateral: [f32; 2],
    shore_offsets: [f32; KNOTS],
    walk_height: [f32; KNOTS],
    shops: [Option<Workshop>; 4],
    bounds: [f32; 4],
}

impl Quayside {
    pub fn plan(
        world: &World,
        seed: u64,
        port: &Port,
        mut blocked: impl FnMut(f32, f32) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        if port.kind() == PortKind::Freight {
            return None;
        }
        let [sx, sz] = port.shore_origin();
        let key = mix(seed
            ^ u64::from(sx.to_bits()) << 32
            ^ u64::from(sz.to_bits())
            ^ 0x109d_a877_61ce_8df5);
        if key & 7 >= 5 {
            return None;
        }
        let sea = port.seaward();
        let right = [-sea[1], sea[0]];
        let contacts = port.quayside_contacts();
        let mut parcels = ParcelSurvey::new();
        for index in 0..contacts.len() {
            let index = (index + ((key >> 9) & 1) as usize * 2) % contacts.len();
            let (point, side) = contacts[index];
            let start = world.sample(point[0], point[1]);
            if blocked(point[0], point[1])
                || port.sample(point[0], point[1], start).is_none_or(|sample| {
                    !matches!(sample.surface, PortSurface::Quay | PortSurface::Apron)
                })
            {
                continue;
            }
            let mut plan = Self {
                contact: point,
                sea,
                lateral: [right[0] * side, right[1] * side],
                shore_offsets: [0.0; KNOTS],
                walk_height: [start.elevation; KNOTS],
                shops: [None; 4],
                bounds: [
                    f32::INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                ],
            };
            if !plan.trace(world, seed, port, &mut parcels, &mut blocked, &mut pulse) {
                continue;
            }
            if plan.site_workshops(
                world,
                seed,
                port,
                key,
                &mut parcels,
                &mut blocked,
                &mut pulse,
            ) < 2
            {
                continue;
            }
            for knot in 0..KNOTS {
                for lag in [-4.5, 4.5] {
                    let [x, z] = plan.world(knot as f32 * STEP, plan.shore_offsets[knot] + lag);
                    plan.include([x, z]);
                }
            }
            for shop in plan.shops.into_iter().flatten() {
                for dx in [-6.0, 6.0] {
                    for dz in [-8.5, 9.5] {
                        let [x, z] = plan.world(shop.center[0] + dx, shop.center[1] + dz);
                        plan.include([x, z]);
                    }
                }
            }
            return Some(plan);
        }
        None
    }

    pub fn contact(&self) -> [f32; 2] {
        self.contact
    }

    pub fn end(&self) -> [f32; 2] {
        self.world(REACH, self.shore_offsets[KNOTS - 1])
    }

    pub fn shops(&self) -> usize {
        self.shops.iter().flatten().count()
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    fn include(&mut self, p: [f32; 2]) {
        self.bounds = [
            self.bounds[0].min(p[0]),
            self.bounds[1].min(p[1]),
            self.bounds[2].max(p[0]),
            self.bounds[3].max(p[1]),
        ];
    }

    fn world(&self, u: f32, v: f32) -> [f32; 2] {
        [
            self.contact[0] + self.lateral[0] * u + self.sea[0] * v,
            self.contact[1] + self.lateral[1] * u + self.sea[1] * v,
        ]
    }

    fn local(&self, x: f32, z: f32) -> (f32, f32) {
        let [dx, dz] = [x - self.contact[0], z - self.contact[1]];
        (
            dx * self.lateral[0] + dz * self.lateral[1],
            dx * self.sea[0] + dz * self.sea[1],
        )
    }

    fn walk(&self, u: f32) -> (f32, f32) {
        let index = (u * (1.0 / STEP)) as usize;
        let index = index.min(KNOTS - 2);
        let t = (u * (1.0 / STEP) - index as f32).clamp(0.0, 1.0);
        (
            self.shore_offsets[index]
                + (self.shore_offsets[index + 1] - self.shore_offsets[index]) * t,
            self.walk_height[index] + (self.walk_height[index + 1] - self.walk_height[index]) * t,
        )
    }

    fn trace(
        &mut self,
        world: &World,
        seed: u64,
        port: &Port,
        parcels: &mut ParcelSurvey,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        for index in 1..KNOTS {
            let u = index as f32 * STEP;
            let previous = self.shore_offsets[index - 1];
            let previous_y = self.walk_height[index - 1];
            let mut best = None;
            let mut grade = f32::INFINITY;
            for step in -5..=5 {
                pulse();
                let out = previous + step as f32 * 0.95;
                let [x, z] = self.world(u, out);
                let mid = world.sample(x, z);
                if !(0.78..=7.8).contains(&mid.elevation)
                    || (mid.elevation - previous_y).abs() > 1.3
                    || ![9.0, 22.0, 34.0].into_iter().any(|v| {
                        let [x, z] = self.world(u, out + v);
                        world.coast_signal(x, z) < -0.015
                    })
                {
                    continue;
                }
                if !(1..=8).all(|offset| {
                    let t = offset as f32 * 0.125;
                    let pin = u - STEP + STEP * t;
                    let offset = previous + (out - previous) * t;
                    let elevation = previous_y + (mid.elevation - previous_y) * t;
                    [-3.1, 0.0, 3.1].into_iter().all(|shift| {
                        let [x, z] = self.world(pin, offset + shift);
                        let ground = world.sample(x, z);
                        if ground.elevation < 0.35
                            || (ground.elevation - elevation).abs() > 1.05
                            || matches!(ground.surface, Surface::Rock | Surface::Snow)
                            || blocked(x, z)
                            || (ground.surface == Surface::Grass
                                && parcels.occupies_grass(world, seed, x, z))
                        {
                            return false;
                        }
                        port.sample(x, z, ground).is_none_or(|piece| {
                            pin < 4.3
                                && matches!(piece.surface, PortSurface::Quay | PortSurface::Apron)
                        })
                    })
                }) {
                    continue;
                }
                let score = (mid.elevation - 2.3).abs() * 0.7
                    + (out - previous).abs() * 0.14
                    + (mid.elevation - previous_y).abs() * 0.30;
                if score < grade {
                    grade = score;
                    best = Some((out, mid.elevation));
                }
            }
            let Some((out, y)) = best else {
                return false;
            };
            self.shore_offsets[index] = out;
            self.walk_height[index] = y;
        }
        true
    }

    fn site_workshops(
        &mut self,
        world: &World,
        seed: u64,
        port: &Port,
        key: u64,
        parcels: &mut ParcelSurvey,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> usize {
        let mut found = 0;
        for along in [13.0, 27.5, 42.0, 56.5] {
            pulse();
            let (row, height) = self.walk(along);
            let offset = row - 10.0;
            let [x, z] = self.world(along, offset);
            let base = world.elevation(x, z);
            let state = mix(key ^ along.to_bits() as u64);
            let rise = 2.9 + ((state >> 18) & 7) as f32 * 0.31;
            if (base - height).abs() > 2.1 {
                continue;
            }
            if (-5..=5).any(|du| {
                (-6..=9).any(|dv| {
                    let [x, z] = self.world(along + du as f32, offset + dv as f32);
                    let ground = world.sample(x, z);
                    ground.elevation < 0.52
                        || (ground.elevation - base).abs() > 1.65
                        || blocked(x, z)
                        || port.sample(x, z, ground).is_some()
                        || (ground.surface == Surface::Grass
                            && parcels.occupies_grass(world, seed, x, z))
                })
            }) {
                continue;
            }
            self.shops[found] = Some(Workshop {
                center: [along, offset],
                base,
                rise,
                id: (state >> 43) as u8,
            });
            found += 1;
        }
        found
    }

    /// The main heightfield handles the low deck, joined paving, and
    /// workshop roofs as occluding geometry.
    pub fn sample(&self, x: f32, z: f32, raw: Sample) -> Option<Sample> {
        let [left, near, right, far] = self.bounds;
        if x < left || x > right || z < near || z > far || raw.elevation < 0.2 {
            return None;
        }
        let (u, v) = self.local(x, z);
        if !(0.0..=REACH).contains(&u) {
            return None;
        }
        let (center, grade) = self.walk(u);
        let side = (v - center).abs();
        if side <= WALK_HALF {
            let weight = smoothstep(u / 4.3) * smoothstep((WALK_HALF - side) / 1.4);
            return Some(Sample {
                elevation: raw.elevation + (grade + 0.22 - raw.elevation) * weight,
                surface: if side > WALK_HALF - 0.57 {
                    Surface::Gravel
                } else {
                    Surface::Boardwalk
                },
            });
        }
        for shop in self.shops.iter().flatten() {
            let x = (u - shop.center[0]).abs();
            let z = v - shop.center[1];
            if x <= 4.6 && (-5.7..=2.05).contains(&z) {
                let roof = if shop.id & 2 != 0 {
                    shop.rise + 0.95 * (1.0 - x / 4.6)
                } else {
                    shop.rise
                };
                return Some(Sample {
                    elevation: raw.elevation.max(shop.base + roof),
                    surface: match shop.id & 3 {
                        0 => Surface::FlatRoof,
                        1 => Surface::TowerRoof,
                        _ => Surface::PitchedRoof,
                    },
                });
            }
            if x <= 3.8 && z > 2.05 && v < center && center - v < 8.0 {
                return Some(Sample {
                    surface: Surface::Gravel,
                    ..raw
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facilities::{RegionalFacilities, RoadsideSite};
    use crate::scenery::{overlay_facilities, overlay_port, DistrictRegion};
    use crate::settlement::Terminus;

    #[test]
    fn waterfront_walk_keeps_a_dry_continuous_join_to_the_real_apron() {
        let seed = 0;
        let world = World::new(seed);
        let region = DistrictRegion::build(&world, 3, -5);
        let road = region.road().unwrap();
        let mut facilities = RegionalFacilities::prepare(
            &world,
            seed,
            road,
            [region.town(Terminus::Start), region.town(Terminus::End)],
        );
        let port = facilities.port(Terminus::Start).unwrap();
        let walk = Quayside::plan(
            &world,
            seed,
            port,
            |x, z| {
                road.proximity(x, z).is_some()
                    || region.towns().any(|town| town.sample(x, z).is_some())
            },
            || {},
        )
        .expect("seeded dry shoreline beside the existing ferry berth");
        assert_eq!(walk.shops(), 4);
        let [cx, cz] = walk.contact();
        let raw = world.sample(cx, cz);
        let port_ground = overlay_port(port, cx, cz, raw);
        assert_ne!(port_ground, raw);
        facilities.install_roadside(RoadsideSite::Quayside(walk));
        assert_eq!(
            overlay_facilities(&facilities, cx, cz, raw, raw),
            port_ground
        );
        let mut last = port_ground.elevation;
        let mut oak = 0;
        for sample in 0..=340 {
            let u = sample as f32 * 0.2;
            let (center, _) = walk.walk(u);
            let [x, z] = walk.world(u, center);
            let raw = world.sample(x, z);
            assert!(raw.elevation >= 0.35);
            let ground = overlay_facilities(&facilities, x, z, raw, raw);
            assert!(matches!(
                ground.surface,
                Surface::Boardwalk | Surface::Gravel | Surface::Pavement
            ));
            assert!((ground.elevation - last).abs() < 0.27);
            oak += usize::from(ground.surface == Surface::Boardwalk);
            last = ground.elevation;
        }
        assert!(oak >= 310);
    }
}
