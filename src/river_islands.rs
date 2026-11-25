//! Small vegetated sediment islands with a surveyed wet passage on each side.

use crate::math::{fraction, mix, smoothstep};
use crate::watershed::{Reach, RiverSample, Watershed, REGION_SIZE};
use crate::world::{Sample, Surface, World, SEA_LEVEL};

const MIN_HALF_CHANNEL: f32 = 3.65;
const WATER_PASSAGE: f32 = 1.55;
const MAX_PROOFS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RiverIsland {
    center: [f32; 2],
    axis: [f32; 2],
    inverse_length: f32,
    inverse_width: f32,
    bend: f32,
    crest: f32,
    bounds: [f32; 4],
}

impl RiverIsland {
    pub(crate) fn survey(
        world: &World,
        water: &Watershed,
        mut reserved: impl FnMut(f32, f32) -> bool,
        mut excluded: impl FnMut([f32; 4]) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let first = water.streams().first()?;
        let key = mix(point_key(first.outlet) ^ point_key(first.source) ^ 0xc1be_4ec6_3e47_749f);
        if key & 3 != 0 {
            return None;
        }
        let mut attempts = 0;
        for reach in water.reaches() {
            pulse();
            let Some(island) = Self::fit(water, reach, mix(key ^ point_key(reach.start))) else {
                continue;
            };
            if excluded(island.bounds) {
                continue;
            }
            attempts += 1;
            if island.prove(world, water, &mut reserved, pulse) {
                return Some(island);
            }
            if attempts == MAX_PROOFS {
                break;
            }
        }
        None
    }

    fn fit(water: &Watershed, reach: Reach, key: u64) -> Option<Self> {
        // Widening, tidal water at the mouth is a separate environment.
        let drop = reach.start_level - reach.end_level;
        if reach.reaches_sea || reach.end_level < SEA_LEVEL + 1.15 || !(0.0..=2.7).contains(&drop) {
            return None;
        }
        let middle = reach.position(0.5);
        let wet = water.sample(middle[0], middle[1])?;
        if !wet.is_channel() || wet.half_width < MIN_HALF_CHANNEL {
            return None;
        }
        let first = reach.position(0.10);
        let last = reach.position(0.90);
        let dx = last[0] - first[0];
        let dz = last[1] - first[1];
        let squared = dx * dx + dz * dz;
        if !(22.0 * 22.0..=42.0 * 42.0).contains(&squared) {
            return None;
        }
        let guess = f32::from_bits((squared.to_bits() >> 1) + 0x1fc0_0000);
        let guess = (guess + squared / guess) * 0.5;
        let length = (guess + squared / guess) * 0.5;
        let axis = [dx / length, dz / length];
        let center = [(first[0] + last[0]) * 0.5, (first[1] + last[1]) * 0.5];
        let bend = (middle[1] - center[1]) * axis[0] - (middle[0] - center[0]) * axis[1];
        let half_length = length * (0.42 + 0.075 * fraction(key));
        let half_width = wet.half_width * (0.46 + 0.09 * fraction(mix(key)));
        let spread = bend.abs() + half_width;
        let width = axis[0].abs() * half_length + axis[1].abs() * spread;
        let depth = axis[1].abs() * half_length + axis[0].abs() * spread;
        let bounds = [
            center[0] - width,
            center[1] - depth,
            center[0] + width,
            center[1] + depth,
        ];
        let origin = reach.start.map(region_origin);
        if !(0..2).all(|i| {
            bounds[i] >= origin[i] + 4.0 && bounds[i + 2] <= origin[i] + REGION_SIZE as f32 - 4.0
        }) {
            return None;
        }
        Some(Self {
            center,
            axis,
            inverse_length: 1.0 / half_length,
            inverse_width: 1.0 / half_width,
            bend,
            crest: 0.62 + 0.33 * fraction(mix(key ^ 0x2a98)),
            bounds,
        })
    }

    fn point(&self, along: f32, lateral: f32) -> [f32; 2] {
        let forward = along / self.inverse_length;
        let across = self.bend * (1.0 - along * along) + lateral;
        [
            self.center[0] + self.axis[0] * forward - self.axis[1] * across,
            self.center[1] + self.axis[1] * forward + self.axis[0] * across,
        ]
    }

    fn prove(
        &self,
        world: &World,
        water: &Watershed,
        reserved: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        let rows = (2.0 / (self.inverse_length * 0.64)) as usize + 1;
        let mut upstream = f32::NEG_INFINITY;
        let mut previous = f32::INFINITY;
        for row in 0..=rows {
            pulse();
            let along = 2.0 * row as f32 / rows as f32 - 1.0;
            let width = (1.0 - along * along) / self.inverse_width;
            let center = self.point(along, 0.0);
            let Some(level) = channel_level(water, center, world, reserved) else {
                return false;
            };
            if row == 0 {
                upstream = level;
            }
            if row != 0 && (level - previous).abs() > 0.19 {
                return false;
            }
            previous = level;
            // Certify the island footprint together with both wetted side
            // passages. Widen at the ends so the two passages connect there.
            let span = width + WATER_PASSAGE;
            let columns = (span / 0.47) as usize + 1;
            for col in 1..=columns {
                let lateral = span * col as f32 / columns as f32;
                for sign in [-1.0, 1.0] {
                    let point = self.point(along, lateral * sign);
                    if channel_level(water, point, world, reserved)
                        .is_none_or(|height| (height - level).abs() > 0.18)
                    {
                        return false;
                    }
                }
            }
        }
        let drop = upstream - previous;
        drop >= -0.10 && drop * self.inverse_length <= 0.14
    }

    pub(crate) fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub(crate) fn overlay(&self, x: f32, z: f32, developed: Sample, covered: Sample) -> Sample {
        let [left, near, right, far] = self.bounds;
        if covered.surface != Surface::RiverWater
            || !(left..=right).contains(&x)
            || !(near..=far).contains(&z)
            || !matches!(
                developed.surface,
                Surface::Grass
                    | Surface::Forest
                    | Surface::Sand
                    | Surface::Desert
                    | Surface::Rock
                    | Surface::TreeCanopy
                    | Surface::WetMeadow
            )
        {
            return covered;
        }
        let dx = x - self.center[0];
        let dz = z - self.center[1];
        let along = (dx * self.axis[0] + dz * self.axis[1]) * self.inverse_length;
        let taper = 1.0 - along * along;
        let across = dz * self.axis[0] - dx * self.axis[1] - self.bend * taper;
        let inset = taper - across.abs() * self.inverse_width;
        if inset <= 0.0 {
            return covered;
        }
        let rise = self.crest * smoothstep(inset);
        let surface = if inset > 0.51 {
            Surface::ReedMarsh
        } else if inset > 0.29 {
            Surface::WetMeadow
        } else {
            Surface::RiverSilt
        };
        Sample {
            elevation: covered.elevation + rise,
            surface,
        }
    }
}

fn channel_level(
    water: &Watershed,
    point: [f32; 2],
    world: &World,
    reserved: &mut impl FnMut(f32, f32) -> bool,
) -> Option<f32> {
    let [x, z] = point;
    let wet = water
        .sample(x, z)
        .filter(|sample: &RiverSample| sample.is_channel())?;
    (wet.level >= SEA_LEVEL + 1.0 && world.elevation(x, z) > SEA_LEVEL + 0.25 && !reserved(x, z))
        .then_some(wet.level)
}

fn point_key([x, z]: [f32; 2]) -> u64 {
    u64::from(x.to_bits()) | (u64::from(z.to_bits()) << 32)
}

fn region_origin(position: f32) -> f32 {
    let value = position / REGION_SIZE as f32;
    let integer = value as i32;
    (integer - i32::from(value < integer as f32)) as f32 * REGION_SIZE as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::floodplain::FloodplainKind;
    use crate::river_district::RiverDistrict;
    use crate::watershed::WatershedWork;

    #[test]
    fn surveyed_island_leaves_two_wet_passages_joined_at_both_ends() {
        let world = World::new(22);
        let mut water = Watershed::new();
        water.rebuild(&world, 22, 0, 0, &mut WatershedWork::new());
        let plan = RiverDistrict::from_prepared_land_cooperatively(
            &world,
            None,
            water,
            |_, _| false,
            &mut || {},
        )
        .unwrap();
        let (_, bounds) = plan
            .floodplain()
            .sites()
            .find(|(kind, _)| *kind == FloodplainKind::RiverIsland)
            .unwrap();
        let intersects_bank = |[a, b, c, d]: [f32; 4]| {
            plan.floodplain().sites().any(|(kind, [l, n, r, f])| {
                kind != FloodplainKind::RiverIsland && a <= r && b <= f && c >= l && d >= n
            })
        };
        let island =
            RiverIsland::survey(&world, &water, |_, _| false, intersects_bank, &mut || {}).unwrap();
        assert_eq!(island.bounds(), bounds);
        for row in -112..=112 {
            let along = row as f32 * 0.009;
            let half_width = (1.0 - along * along).max(0.0) / island.inverse_width;
            for sign in [-1.0, 1.0] {
                for lane in 1..=6 {
                    let [x, z] = island.point(along, sign * (half_width + lane as f32 * 0.23));
                    let wet = water.sample(x, z).unwrap();
                    assert!(wet.is_channel(), "{along} {sign} {lane}");
                    assert_eq!(
                        plan.overlay(x, z, world.sample(x, z)).surface,
                        Surface::RiverWater
                    );
                }
            }
            let [x, z] = island.point(along, 0.0);
            let wet = water.sample(x, z).unwrap();
            let covered = Sample {
                surface: Surface::RiverWater,
                elevation: wet.level,
            };
            let developed = world.sample(x, z);
            let sample = island.overlay(x, z, developed, covered);
            if along.abs() < 0.9 {
                assert!(sample.elevation > wet.level);
                assert!(sample.surface != covered.surface);
            } else if along.abs() > 1.0 {
                assert_eq!(sample, covered);
            }
            let pavement = Sample {
                surface: Surface::Pavement,
                ..developed
            };
            assert_eq!(island.overlay(x, z, pavement, covered), covered);
        }
        let protected = RiverIsland::survey(
            &world,
            &water,
            |x, z| (bounds[0]..=bounds[2]).contains(&x) && (bounds[1]..=bounds[3]).contains(&z),
            intersects_bank,
            &mut || {},
        );
        assert_ne!(protected.map(|place| place.bounds()), Some(bounds));
    }
}
