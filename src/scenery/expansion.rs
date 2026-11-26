//! A detached survey of additional town scenery. The district remains the planning authority.

use super::{cover_settlement, region_axis, DistrictRegion, ROAD_REGION_SIZE};
use crate::settlement::{BoroughPaintCache, SettlementSample, Terminus, TownExpansion, WaterfrontVillage};
use crate::skyline::SkylineTower;
use crate::world::{Sample, Surface, World};

/// A worker can take this snapshot after the ordinary district is ready.
pub struct DistrictExpansionRequest {
    world: World,
    district: DistrictRegion,
    revision: u32,
}

/// Complete extra town scenery for one owner and one reservation revision.
pub struct DistrictExpansion {
    region: [i32; 2],
    revision: u32,
    towns: [Option<TownExpansion>; 2],
    waterfront: Option<WaterfrontVillage>,
    bounds: Option<[f32; 4]>,
    occupied: [u32; 32],
}

impl DistrictRegion {
    /// Capture the original town and road without changing either planning record.
    pub fn expansion_request(
        &self,
        world: World,
        revision: u32,
    ) -> Option<DistrictExpansionRequest> {
        (self.road.is_some() && self.towns.iter().any(Option::is_some)).then(|| {
            DistrictExpansionRequest {
                world,
                district: self.clone(),
                revision,
            }
        })
    }

    /// Compose an approved addition after the original road and both old towns.
    pub fn overlay_expansion(
        &self,
        expansion: &DistrictExpansion,
        x: f32,
        z: f32,
        ground: Sample,
    ) -> Sample {
        let established = self.overlay(x, z, ground);
        if established != ground
            || matches!(ground.surface, Surface::Pavement | Surface::Gravel)
            || expansion.region != [self.region_x, self.region_z]
        {
            return established;
        }
        expansion
            .sample(x, z)
            .map_or(established, |detail| cover_settlement(detail, established))
    }
}

impl DistrictExpansionRequest {
    pub fn region(&self) -> [i32; 2] {
        [self.district.region_x, self.district.region_z]
    }

    /// Conservative extent for the owner and its external reservation snapshots.
    pub fn region_bounds(&self) -> [f32; 4] {
        let [x, z] = self
            .region()
            .map(|axis| axis as f32 * ROAD_REGION_SIZE as f32);
        [
            x,
            z,
            x + ROAD_REGION_SIZE as f32,
            z + ROAD_REGION_SIZE as f32,
        ]
    }

    pub fn revision(&self) -> u32 {
        self.revision
    }

    /// Exclude retained scenery through a world-coordinate reservation probe.
    /// The probe is called along each proposed lane and throughout each lot.
    pub fn prepare(
        &self,
        reserved: impl Fn(f32, f32) -> bool,
        pulse: impl FnMut(),
    ) -> DistrictExpansion {
        self.prepare_with_bounds(reserved, |_| true, |_, _| false, |_, _| false, pulse)
    }

    /// The broad test may overestimate reservations, but must never miss one.
    pub(crate) fn prepare_with_bounds(
        &self,
        reserved: impl Fn(f32, f32) -> bool,
        may_reserve: impl Fn([f32; 4]) -> bool,
        near_waterway: impl Fn(f32, f32) -> bool,
        waterway_cell: impl Fn(f32, f32) -> bool,
        mut pulse: impl FnMut(),
    ) -> DistrictExpansion {
        let waterfront = self.district.road().and_then(|road| WaterfrontVillage::survey(
            &self.world, road, [Terminus::Start, Terminus::End].map(|end| self.district.town(end)),
            &reserved, &may_reserve, &near_waterway, &waterway_cell, &mut pulse,
        ));
        let waterfront_bounds = waterfront.as_ref().map(WaterfrontVillage::bounds);
        let reserved = |x, z| reserved(x, z)
            || waterfront.as_ref().is_some_and(|village| village.reserves(x, z));
        let overlaps = |[a, b, c, d]: [f32; 4], [x, z, u, v]: [f32; 4]| {
            a <= u && b <= v && c >= x && d >= z
        };
        let may_reserve = |bounds| {
            may_reserve(bounds) || waterfront_bounds.is_some_and(|envelope| {
                if !overlaps(bounds, envelope) {
                    return false;
                }
                let mut touches = false;
                if let Some(village) = &waterfront {
                    village.footprints(|footprint| touches |= overlaps(bounds, footprint));
                }
                touches
            })
        };
        let towns = [Terminus::Start, Terminus::End].map(|end| {
            let town = self.district.town(end)?;
            let other_end = match end {
                Terminus::Start => Terminus::End,
                Terminus::End => Terminus::Start,
            };
            Some(TownExpansion::survey(
                &self.world,
                self.district.road()?,
                end,
                town,
                self.district.town(other_end),
                &reserved,
                &may_reserve,
                &near_waterway,
                &mut pulse,
            ))
        });
        let bounds = towns
            .iter()
            .flatten()
            .filter_map(TownExpansion::bounds)
            .chain(waterfront_bounds)
            .reduce(|a, b| {
                [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ]
            });
        let [left, near, _, _] = self.region_bounds();
        let mut occupied = [0_u32; 32];
        let cell = |offset: f32| ((offset * (32.0 / ROAD_REGION_SIZE as f32)) as usize).min(31);
        let mut occupy = |[x, z, far_x, far_z]: [f32; 4]| {
            let start = cell(x - left - 0.02);
            let end = cell(far_x - left + 0.02);
            let mask = (u32::MAX << start) & (u32::MAX >> (31 - end));
            for row in &mut occupied[cell(z - near - 0.02)..=cell(far_z - near + 0.02)] {
                *row |= mask;
            }
        };
        for town in towns.iter().flatten() {
            town.footprints(&mut occupy);
        }
        if let Some(village) = &waterfront {
            village.footprints(&mut occupy);
        }
        DistrictExpansion {
            region: self.region(),
            revision: self.revision,
            towns,
            waterfront,
            bounds,
            occupied,
        }
    }
}

impl DistrictExpansion {
    pub fn region(&self) -> [i32; 2] {
        self.region
    }

    pub fn revision(&self) -> u32 {
        self.revision
    }

    /// Bounds of accepted new development only. The established planning extent is separate.
    pub fn bounds(&self) -> Option<[f32; 4]> {
        self.bounds
    }

    fn contains(&self, x: f32, z: f32) -> bool {
        self.bounds.is_some_and(|[left, near, right, far]| {
            (left..=right).contains(&x)
                && (near..=far).contains(&z)
                && [
                    region_axis(x, ROAD_REGION_SIZE),
                    region_axis(z, ROAD_REGION_SIZE),
                ] == self.region
        })
    }

    pub fn sample(&self, x: f32, z: f32) -> Option<SettlementSample> {
        self.contains(x, z).then(|| self.sample_within_bounds(x, z, None)).flatten()
    }

    pub(crate) fn sample_within_bounds(
        &self, x: f32, z: f32, cache: Option<&BoroughPaintCache>,
    ) -> Option<SettlementSample> {
        let cell = |axis: i32, position: f32| {
            ((position - axis as f32 * ROAD_REGION_SIZE as f32) * (32.0 / ROAD_REGION_SIZE as f32)) as usize
        };
        let [column, row] = [cell(self.region[0], x), cell(self.region[1], z)];
        if column >= 32 || row >= 32 || self.occupied[row] & (1 << column) == 0 {
            return None;
        }
        self.towns.iter().flatten().find_map(|town| town.sample_with_cache(x, z, cache))
            .or_else(|| self.waterfront_sample(x, z))
    }

    pub fn waterfront_sample(&self, x: f32, z: f32) -> Option<SettlementSample> {
        self.waterfront.as_ref()?.sample(x, z)
    }

    pub fn waterfront_landmark_designs(
        &self,
    ) -> impl Iterator<Item = ([f32; 3], crate::settlement::riverfront_buildings::RiverfrontBuilding)> + '_ {
        self.waterfront.iter().flat_map(|village| village.landmark_designs())
    }

    /// Extra scenery at one road terminus, before it is combined with its original town.
    pub fn town_sample(&self, end: Terminus, x: f32, z: f32) -> Option<SettlementSample> {
        if !self.contains(x, z) {
            return None;
        }
        let index = match end {
            Terminus::Start => 0,
            Terminus::End => 1,
        };
        self.towns[index].as_ref()?.sample(x, z)
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        self.towns
            .iter()
            .flatten()
            .map(|town| town.obstruction(x, z))
            .chain(self.waterfront.iter().map(|village| village.obstruction(x, z)))
            .fold(0.0, f32::max)
    }

    pub fn skyline(&self) -> impl Iterator<Item = SkylineTower> + '_ {
        self.towns
            .iter()
            .flatten()
            .flat_map(TownExpansion::roof_groups)
            .chain(self.waterfront.iter().flat_map(WaterfrontVillage::roof_groups))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facilities::RegionalFacilities;

    #[test]
    fn expansion_leaves_the_regional_airport_and_original_towns_intact() {
        let seed = 9;
        let world = World::new(seed);
        let [rx, rz] = [-6, -7];
        let district = DistrictRegion::build(&world, rx, rz);
        let facilities = RegionalFacilities::prepare(
            &world,
            seed,
            district.road().unwrap(),
            [district.town(Terminus::Start), district.town(Terminus::End)],
        );
        assert!(facilities.airfield().is_some());
        assert_eq!(facilities.ports().count(), 2);
        let limits = facilities.bounds().unwrap();
        let reserved = |x, z| {
            (limits[0]..=limits[2]).contains(&x)
                && (limits[1]..=limits[3]).contains(&z)
                && facilities.sample(x, z, world.sample(x, z)).is_some()
        };
        let request = district.expansion_request(world, 26).unwrap();
        let denied = request.prepare(|_, _| true, || {});
        assert!(denied.bounds().is_none());
        assert_eq!(denied.skyline().count(), 0);
        let approved = request.prepare(reserved, || {});
        let [left, near, right, far] = approved.bounds().unwrap();
        assert_eq!([rx, rz], approved.region());
        assert_eq!(26, approved.revision());
        let flat = Sample {
            elevation: 6.0,
            surface: Surface::Grass,
        };
        let mut additional = 0;
        for z in (near as i32..=far as i32).step_by(2) {
            for x in (left as i32..=right as i32).step_by(2) {
                let [x, z] = [x as f32, z as f32];
                let old = district.overlay(x, z, flat);
                let overlay = district.overlay_expansion(&approved, x, z, flat);
                if approved.sample(x, z).is_some() {
                    assert!(
                        !reserved(x, z),
                        "addition claims protected site at ({x}, {z})"
                    );
                }
                if old != flat {
                    assert_eq!(
                        overlay, old,
                        "addition changed established occupancy at ({x}, {z})"
                    );
                } else {
                    additional += usize::from(overlay != old);
                }
            }
        }
        assert!(
            additional > 100,
            "only {additional} new samples in reserved town"
        );
    }
}
