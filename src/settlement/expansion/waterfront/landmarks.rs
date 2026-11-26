//! Small waterfront buildings with connected landings and distinct silhouettes.

use super::*;
use crate::settlement::riverfront_buildings::{District, RiverfrontBuilding};
use crate::skyline::{SkylineTower, COTTAGE_GROUP, INDUSTRIAL_GROUP, SHOP_GROUP};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    BoatShed,
    PicnicPavilion,
    Lookout,
    Market,
    Designed(u8),
}

impl Kind {
    fn size(self) -> [f32; 2] {
        match self {
            Self::BoatShed => [4.2, 3.0],
            Self::PicnicPavilion => [3.4, 3.4],
            Self::Lookout => [2.5, 2.5],
            Self::Market => [4.8, 3.4],
            Self::Designed(index) => RiverfrontBuilding::ALL[usize::from(index)].size,
        }
    }

    fn height(self) -> f32 {
        match self {
            Self::BoatShed => 5.3,
            Self::PicnicPavilion => 4.1,
            Self::Lookout => 8.9,
            Self::Market => 5.6,
            Self::Designed(index) => RiverfrontBuilding::ALL[usize::from(index)].height,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct WalkLandmark {
    center: [f32; 2],
    base: f32,
    kind: Kind,
    direction: u8,
}

impl WalkLandmark {
    fn outward(self) -> [f32; 2] {
        let (x, z, _, _) = DIRECTIONS[usize::from(self.direction)];
        [x as f32, z as f32]
    }

    fn roof(self) -> Area {
        let [along, depth] = self.kind.size();
        let [x, z] = self.center;
        let [dx, _] = self.outward();
        let [half_width, half_depth] = if dx == 0.0 { [along, depth] } else { [depth, along] };
        Area { x, z, half_width, half_depth }
    }

    fn landing(self) -> Area {
        let [dx, dz] = self.outward();
        let [x, z] = self.center;
        let [_, depth] = self.kind.size();
        // A short timber apron extends from the front doors to the dry walk.
        let offset = depth + 1.15;
        Area {
            x: x - dx * offset,
            z: z - dz * offset,
            half_width: if dx == 0.0 { 2.05 } else { 1.25 },
            half_depth: if dx == 0.0 { 1.25 } else { 2.05 },
        }
    }

    pub(super) fn areas(&self) -> [Area; 2] {
        [self.roof(), self.landing()]
    }

    pub(super) fn sample(&self, x: f32, z: f32) -> Option<SettlementSample> {
        let roof = self.roof();
        if !roof.contains(x, z) {
            return self.landing().contains(x, z).then_some(SettlementSample {
                surface: SettlementSurface::RiverWalk, elevation: None,
            });
        }
        let [dx, dz] = self.outward();
        let [along, depth] = self.kind.size();
        let [rx, rz] = [x - roof.x, z - roof.z];
        let u = (rx * dz - rz * dx) / along;
        let v = (rx * dx + rz * dz) / depth;
        let [a, b] = [u.abs(), v.abs()];
        let (surface, height) = match self.kind {
            Kind::Designed(index) => RiverfrontBuilding::ALL[usize::from(index)].roof([u, v])?,
            Kind::BoatShed if a > 0.83 && v < 0.1 => (SettlementSurface::FlatRoof, 3.05),
            Kind::BoatShed => (SettlementSurface::PitchedRoof, 3.5 + (1.0 - b) * 1.8),
            Kind::PicnicPavilion if a + b > 1.66 => return None,
            Kind::PicnicPavilion => (SettlementSurface::PitchedRoof, 2.45 + (1.0 - a.max(b)) * 1.65),
            Kind::Lookout if a.max(b) < 0.56 => (SettlementSurface::TowerRoof, 7.6 + (1.0 - a.max(b) / 0.56) * 1.3),
            Kind::Lookout => (SettlementSurface::FlatRoof, 4.95),
            Kind::Market if v < -0.55 => {
                let bay = ((u + 1.0) * 2.0) as u8;
                let roof = if bay & 1 == 0 { SettlementSurface::PitchedRoof } else { SettlementSurface::FlatRoof };
                (roof, 2.25 + (v + 1.0) * 1.0)
            }
            Kind::Market => (SettlementSurface::PitchedRoof, 4.0 + (1.0 - b) * 1.6),
        };
        Some(SettlementSample { surface, elevation: Some(self.base + height) })
    }

    pub(super) fn skyline(&self, local: Local) -> SkylineTower {
        let roof = self.roof();
        let (x, z) = local.world(roof.x, roof.z);
        let profile = match self.kind {
            Kind::BoatShed => INDUSTRIAL_GROUP,
            Kind::PicnicPavilion => COTTAGE_GROUP,
            Kind::Lookout | Kind::Market => SHOP_GROUP,
            Kind::Designed(index) => match RiverfrontBuilding::ALL[usize::from(index)].district {
                District::Boating | District::Service => INDUSTRIAL_GROUP,
                District::Park => COTTAGE_GROUP,
                District::Lookout | District::Market => SHOP_GROUP,
            },
        };
        SkylineTower {
            center: [x, z], footprint: [roof.half_width, roof.half_depth],
            direction: [local.fx, local.fz], base: self.base, height: self.kind.height(), profile,
        }
    }

    pub(super) fn design_site(self, local: Local) -> Option<([f32; 3], RiverfrontBuilding)> {
        let Kind::Designed(index) = self.kind else { return None };
        let (x, z) = local.world(self.center[0], self.center[1]);
        Some(([x, self.base, z], RiverfrontBuilding::ALL[usize::from(index)]))
    }
}

impl WaterfrontVillage {
    pub(super) fn site_landmarks(
        &mut self,
        walk: WaterfrontWalk,
        world: &World,
        road: &RoadNetwork,
        reservation: &Reservation<'_, impl Fn(f32, f32) -> bool>,
        pulse: &mut impl FnMut(),
    ) {
        let [x, z] = walk.direction();
        let end = walk.end();
        let mut count = 0;
        for (arm, steps) in walk.wings.into_iter().enumerate() {
            let along = if arm == 0 { [-z, x] } else { [z, -x] };
            'find_lot: for step in (1..=steps).rev() {
                let start = walk_point(end, along, step);
                let key = block_key(self.key ^ 0x735b_a4ce, arm * 8 + usize::from(step));
                let preferred = ((key >> 23) & 3) as usize;
                let catalog_size = RiverfrontBuilding::ALL.len();
                let design = ((key >> 12) as usize) % catalog_size;
                for flip in [false, true] {
                    let direction = (walk.direction + if flip { 2 } else { 0 }) & 3;
                    let (dx, dz, _, _) = DIRECTIONS[usize::from(direction)];
                    let [dx, dz] = [dx as f32, dz as f32];
                    let styles = [Kind::BoatShed, Kind::PicnicPavilion, Kind::Lookout, Kind::Market];
                    let mut choices = [
                        Kind::Designed(design as u8),
                        Kind::Designed(((design + 11) % catalog_size) as u8),
                        Kind::Designed(((design + 17) % catalog_size) as u8),
                        styles[preferred],
                        styles[(preferred + 1) % styles.len()],
                    ];
                    if key & 15 == 0 {
                        choices.swap(0, 3);
                    }
                    for kind in choices {
                        pulse();
                        if self.landmarks.iter().flatten().any(|landmark| landmark.kind == kind) {
                            continue;
                        }
                        let [_, depth] = kind.size();
                        let offset = WALK_HALF_WIDTH + depth + 2.3;
                        let mut landmark = WalkLandmark {
                            center: [start[0] + dx * offset, start[1] + dz * offset],
                            base: 0.0, kind, direction,
                        };
                        let [roof, landing] = landmark.areas();
                        if self.areas().any(|area| overlaps(roof, area))
                            || self.lanes.iter().flatten().chain(self.square.iter()).any(|&area| overlaps(landing, area))
                            || self.lots().any(|lot| overlaps(landing, self.design(lot).0))
                            || self.landmarks.iter().flatten().flat_map(WalkLandmark::areas)
                                .any(|area| overlaps(landing, area))
                            || [roof, landing].into_iter().any(|area| {
                                highway_overlaps(road, self.local, area) || !reservation.clear_lot(area)
                            })
                        {
                            continue;
                        }
                        let Some(base) = foundation(world, self.local, roof, 1.65) else { continue; };
                        if foundation(world, self.local, landing, 1.3).is_none() {
                            continue;
                        }
                        landmark.base = base;
                        self.landmarks[count] = Some(landmark);
                        count += 1;
                        break 'find_lot;
                    }
                }
            }
        }
    }
}
