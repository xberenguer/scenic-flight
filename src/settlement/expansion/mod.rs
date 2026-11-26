//! Prepared additions kept separate from the established town.

use super::*;

mod boroughs;
mod neighborhoods;

use self::boroughs::Boroughs;
pub(crate) use self::boroughs::BoroughPaintCache;
use self::neighborhoods::Neighborhoods;

pub(crate) struct TownExpansion {
    local: Local,
    neighborhoods: Neighborhoods,
    boroughs: Boroughs,
}

impl TownExpansion {
    pub(crate) fn survey(
        world: &World,
        road: &RoadNetwork,
        end: Terminus,
        town: &Settlement,
        other: Option<&Settlement>,
        reserved: &impl Fn(f32, f32) -> bool,
        may_reserve: &dyn Fn([f32; 4]) -> bool,
        near_waterway: &dyn Fn(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Self {
        let local = town.local;
        let key = town_key(world, local);
        let claim = TownClaim::survey(world, road, end, pulse);
        let original = [Some(town), other];
        let authority = Reservation {
            local,
            original,
            original_bounds: original.map(|town| town.map(Settlement::bounds)),
            neighborhood: None,
            neighborhood_bounds: None,
            reserved,
            may_reserve,
        };
        let neighborhoods = Neighborhoods::survey(world, road, town, claim, key, &authority, pulse);
        let authority = Reservation {
            neighborhood: Some(&neighborhoods),
            neighborhood_bounds: neighborhoods.extent().map(|[x, z, u, v]| {
                [x - STREET_HALF_WIDTH, z - STREET_HALF_WIDTH, u + STREET_HALF_WIDTH, v + STREET_HALF_WIDTH]
            }),
            ..authority
        };
        let boroughs = Boroughs::survey(world, road, town, claim, key, &authority, near_waterway, pulse);
        Self {
            local,
            neighborhoods,
            boroughs,
        }
    }

    pub(crate) fn sample(&self, x: f32, z: f32) -> Option<SettlementSample> {
        self.sample_with_cache(x, z, None)
    }

    pub(crate) fn sample_with_cache(
        &self,
        x: f32,
        z: f32,
        cache: Option<&BoroughPaintCache>,
    ) -> Option<SettlementSample> {
        let (across, ahead) = self.local.grid(x, z);
        self.neighborhoods
            .sample(across, ahead)
            .or_else(|| self.boroughs.sample_with_cache(across, ahead, cache))
    }

    pub(crate) fn obstruction(&self, x: f32, z: f32) -> f32 {
        let (across, ahead) = self.local.grid(x, z);
        self.neighborhoods
            .obstruction(across, ahead)
            .max(self.boroughs.obstruction(across, ahead))
    }

    pub(crate) fn bounds(&self) -> Option<[f32; 4]> {
        let [left, near, right, far] = [self.neighborhoods.extent(), self.boroughs.extent()]
            .into_iter()
            .flatten()
            .reduce(|a, b| {
                [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ]
            })?;
        let corners = [
            [left - STREET_HALF_WIDTH, near - STREET_HALF_WIDTH],
            [right + STREET_HALF_WIDTH, near - STREET_HALF_WIDTH],
            [right + STREET_HALF_WIDTH, far + STREET_HALF_WIDTH],
            [left - STREET_HALF_WIDTH, far + STREET_HALF_WIDTH],
        ];
        corners
            .into_iter()
            .map(|[x, z]| self.local.world(x, z))
            .map(|(x, z)| [x, z, x, z])
            .reduce(|a, b| {
                [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ]
            })
    }

    pub(crate) fn roof_groups(&self) -> impl Iterator<Item = crate::skyline::SkylineTower> + '_ {
        self.neighborhoods
            .roof_groups(self.local)
            .chain(self.boroughs.roof_groups(self.local))
    }

    pub(crate) fn footprints(&self, mut emit: impl FnMut([f32; 4])) {
        let mut world_bounds = |[left, near, right, far]: [f32; 4]| {
            let mut output = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
            for x in [left, right] {
                for z in [near, far] {
                    let (x, z) = self.local.world(x, z);
                    output = [output[0].min(x), output[1].min(z), output[2].max(x), output[3].max(z)];
                }
            }
            emit(output);
        };
        if let Some([left, near, right, far]) = self.neighborhoods.extent() {
            let w = STREET_HALF_WIDTH;
            world_bounds([left - w, near - w, right + w, far + w]);
        }
        self.boroughs.footprints(world_bounds);
    }
}

struct Reservation<'a, F> {
    local: Local,
    original: [Option<&'a Settlement>; 2],
    original_bounds: [Option<[f32; 4]>; 2],
    neighborhood: Option<&'a Neighborhoods>,
    neighborhood_bounds: Option<[f32; 4]>,
    reserved: &'a F,
    may_reserve: &'a dyn Fn([f32; 4]) -> bool,
}

impl<F: Fn(f32, f32) -> bool> Reservation<'_, F> {
    fn clear_bounds(&self, local: [f32; 4]) -> bool {
        let overlap = |[a, b, c, d]: [f32; 4], [x, z, u, v]: [f32; 4]| {
            a <= u && b <= v && c >= x && d >= z
        };
        if self.neighborhood_bounds.is_some_and(|bounds| overlap(bounds, local)) {
            return false;
        }
        let [left, near, right, far] = local;
        let (x, z) = self.local.world((left + right) * 0.5, (near + far) * 0.5);
        let [dx, dz] = [(right - left) * 0.5, (far - near) * 0.5];
        let [fx, fz] = [self.local.fx.abs(), self.local.fz.abs()];
        let half_width = dx * fz + dz * fx + 0.125;
        let half_depth = dx * fx + dz * fz + 0.125;
        let world = [x - half_width, z - half_depth, x + half_width, z + half_depth];
        !(self.may_reserve)(world)
            && !self.original_bounds.iter().flatten().any(|&bounds| overlap(bounds, world))
    }

    fn blocks(&self, x: f32, z: f32, road: bool) -> bool {
        let (wx, wz) = self.local.world(x, z);
        let conflicts = |spot: SettlementSample| !road || spot.surface != SettlementSurface::Street;
        (self.reserved)(wx, wz)
            || self
                .original
                .iter()
                .flatten()
                .any(|town| town.sample(wx, wz).is_some_and(conflicts))
            || self
                .neighborhood
                .and_then(|area| area.sample(x, z))
                .is_some_and(conflicts)
    }

    fn clear_road(&self, start: [f32; 2], direction: [f32; 2], length: f32) -> bool {
        let [x, z] = start;
        let [dx, dz] = direction;
        let count = (length / 1.5) as usize + 1;
        let width = STREET_HALF_WIDTH + 0.8;
        let [tx, tz] = [x + length * dx, z + length * dz];
        if self.clear_bounds([x.min(tx) - width, z.min(tz) - width, x.max(tx) + width, z.max(tz) + width]) {
            return true;
        }
        (0..=count).all(|step| {
            let d = length * step as f32 / count as f32;
            [-width, -width * 0.5, 0.0, width * 0.5, width]
                .into_iter()
                .all(|s| !self.blocks(x + d * dx - s * dz, z + d * dz + s * dx, true))
        })
    }

    fn clear_lot(&self, lot: Area) -> bool {
        let half_width = lot.half_width + 0.8;
        let half_depth = lot.half_depth + 0.8;
        if self.clear_bounds([lot.x - half_width, lot.z - half_depth, lot.x + half_width, lot.z + half_depth]) {
            return true;
        }
        let cols = (half_width / 0.75) as usize + 1;
        let rows = (half_depth / 0.75) as usize + 1;
        (0..=rows).all(|row| {
            let z = lot.z - half_depth + 2.0 * half_depth * row as f32 / rows as f32;
            (0..=cols).all(|col| {
                let x = lot.x - half_width + 2.0 * half_width * col as f32 / cols as f32;
                !self.blocks(x, z, false)
            })
        })
    }
}
/// A shared dividing line balanced on available dry land instead of the
/// midpoint between termini. Both ends independently derive the same line.
#[derive(Clone, Copy)]
struct TownClaim {
    start: [f32; 2],
    direction: [f32; 2],
    boundary: f32,
    toward_end: bool,
}

impl TownClaim {
    fn survey(world: &World, road: &RoadNetwork, end: Terminus, pulse: &mut impl FnMut()) -> Self {
        let points = road.polyline();
        let a = points[0];
        let b = points[points.len() - 1];
        let start = [a.x, a.z];
        let direction = [b.x - a.x, b.z - a.z];
        let length_squared = direction[0] * direction[0] + direction[1] * direction[1];
        let owner_x = crate::scenery::region_axis(a.x, ROAD_REGION_SIZE) * ROAD_REGION_SIZE;
        let owner_z = crate::scenery::region_axis(a.z, ROAD_REGION_SIZE) * ROAD_REGION_SIZE;
        let mut dry = [0_u16; 32];
        let project = |x: f32, z: f32| {
            ((x - start[0]) * direction[0] + (z - start[1]) * direction[1]) / length_squared
        };
        if length_squared > 1.0 {
            for iz in 0..12 {
                pulse();
                let z = owner_z as f32 + 122.0 + 70.0 * iz as f32;
                for ix in 0..12 {
                    let x = owner_x as f32 + 122.0 + 70.0 * ix as f32;
                    if (DRY_HEIGHT..=SEA_LEVEL + 45.0).contains(&world.elevation(x, z)) {
                        let along = (project(x, z).clamp(0.0, 0.999) * 32.0) as usize;
                        dry[along] += 1;
                    }
                }
            }
        }
        let half = dry.iter().copied().sum::<u16>() / 2;
        let mut sum = 0;
        let median = dry
            .into_iter()
            .enumerate()
            .find(|(_, count)| {
                sum += count;
                half > 0 && sum >= half
            })
            .map_or(16.0, |(index, _)| index as f32 + 0.5);
        let split = (median / 32.0).clamp(0.30, 0.70);
        let toward_end = end == Terminus::End;
        let other = if toward_end {
            Local::at_end(a, points[1])
        } else {
            Local::at_end(b, points[points.len() - 2])
        };
        let has_other = town_key(world, other) & 7 != 0
            && street_fits(world, other, ENTRY_COL as f32 * SPACING, 0.0, 0, 1);
        let clearance = 4.5 * horizontal_length(direction[0], direction[1]);
        let boundary = if has_other {
            split * length_squared + if toward_end { clearance } else { -clearance }
        } else if toward_end {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
        Self {
            start,
            direction,
            boundary,
            toward_end,
        }
    }

    fn permits(self, x: f32, z: f32) -> bool {
        let scalar =
            (x - self.start[0]) * self.direction[0] + (z - self.start[1]) * self.direction[1];
        if self.toward_end {
            scalar > self.boundary
        } else {
            scalar < self.boundary
        }
    }
}
