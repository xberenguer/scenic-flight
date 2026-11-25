use crate::facilities::RegionalFacilities;

const SIDE: usize = 16;

#[derive(Clone, Copy)]
pub(super) struct FacilityCoverage {
    scale: [f32; 2],
    rows: [u16; SIDE],
}

impl FacilityCoverage {
    pub(super) fn new(bounds: Option<[f32; 4]>, facilities: &RegionalFacilities) -> Option<Self> {
        let bounds = bounds?;
        // The airport includes multiple approach and service areas. Keep its
        // existing query until an equivalent exact footprint is available.
        if facilities.airfield().is_some() {
            return None;
        }
        let [x, z, far_x, far_z] = bounds;
        let [width, length] = [far_x - x, far_z - z];
        if !width.is_finite() || !length.is_finite() || width <= 0.0 || length <= 0.0 {
            return None;
        }
        let mut coverage = Self {
            scale: [SIDE as f32 / width, SIDE as f32 / length],
            rows: [0; SIDE],
        };
        for port in facilities.ports() {
            coverage.include(bounds, port.bounds());
        }
        if let Some(site) = facilities.roadside() {
            coverage.include(bounds, site.bounds());
        }
        if let Some(line) = facilities.rail() {
            line.sample_bounds(|area| coverage.include(bounds, area));
        }
        // Compact rectangular facilities are cheaper with the existing
        // broad bounds alone.
        let cells: u32 = coverage.rows.iter().map(|row| row.count_ones()).sum();
        (cells < (SIDE * SIDE * 3 / 4) as u32).then_some(coverage)
    }

    fn cell(&self, x: f32, z: f32, bounds: [f32; 4]) -> [usize; 2] {
        [
            (((x - bounds[0]) * self.scale[0]) as usize).min(SIDE - 1),
            (((z - bounds[1]) * self.scale[1]) as usize).min(SIDE - 1),
        ]
    }

    fn include(&mut self, bounds: [f32; 4], area: [f32; 4]) {
        let [left, near] = self.cell(area[0], area[1], bounds);
        let [right, far] = self.cell(area[2], area[3], bounds);
        let mask = (u16::MAX << left) & (u16::MAX >> (SIDE - 1 - right));
        for row in &mut self.rows[near..=far] {
            *row |= mask;
        }
    }

    pub(super) fn covers(&self, x: f32, z: f32, bounds: [f32; 4]) -> bool {
        let [column, row] = self.cell(x, z, bounds);
        self.rows[row] & (1 << column) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facilities::FacilitySample;
    use crate::roads::RoadNetwork;
    use crate::settlement::{Settlement, Terminus};
    use crate::world::{Sample, Surface, World, SEA_LEVEL};

    #[test]
    fn coverage_keeps_port_rail_and_access_samples_across_seeded_footprints() {
        let mut masks = 0;
        let mut ports = 0;
        let mut tracks = 0;
        let mut roads = 0;
        for (seed, tx, tz) in [
            (1, -1, -3),
            (41, 5, -4),
            (46, -5, -3),
            (85, -4, 4),
            (90, 2, -14),
            (108, 0, -9),
        ] {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, tx, tz).unwrap();
            let towns = [Terminus::Start, Terminus::End]
                .map(|end| Settlement::generate(&world, &road, end));
            let towns = if seed == 1 {
                [None, None]
            } else {
                towns.each_ref().map(Option::as_ref)
            };
            let plan = RegionalFacilities::prepare(&world, seed, &road, towns);
            let Some(bounds) = plan.bounds() else {
                continue;
            };
            let Some(cover) = FacilityCoverage::new(Some(bounds), &plan) else {
                continue;
            };
            masks += 1;
            let [left, near, right, far] = bounds;
            for row in (near * 2.0) as i32..=(far * 2.0) as i32 {
                for column in (left * 2.0) as i32..=(right * 2.0) as i32 {
                    let (x, z) = (column as f32 * 0.5, row as f32 * 0.5);
                    if !(left..=right).contains(&x) || !(near..=far).contains(&z) {
                        continue;
                    }
                    for terrain in [
                        Sample {
                            elevation: SEA_LEVEL + 0.3,
                            surface: Surface::Sand,
                        },
                        Sample {
                            elevation: SEA_LEVEL,
                            surface: Surface::Shallows,
                        },
                    ] {
                        if let Some(sample) = plan.sample(x, z, terrain) {
                            assert!(cover.covers(x, z, bounds), "{seed}: {x}, {z}, {sample:?}");
                            ports += usize::from(matches!(sample, FacilitySample::Port(_)));
                            tracks += usize::from(matches!(sample, FacilitySample::Rail(_)));
                            roads += usize::from(matches!(sample, FacilitySample::Rail(part)
                                if part.material == crate::rail::RailMaterial::StationAccess));
                        }
                    }
                }
            }
        }
        assert!(masks >= 4 && ports > 1_000 && tracks > 10_000 && roads > 1_000);
    }
}
