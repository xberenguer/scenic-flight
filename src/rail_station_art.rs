//! Small permanent objects inside the already surveyed station boundary.

use crate::math::smoothstep;
use crate::rail::{RailAnchorKind, RailMaterial, RailService};
use crate::render::{Camera, ScreenProjection, ScreenRect};
use crate::world_mesh::WorldMesh;

type Point = [f32; 3];
type Color = [u8; 3];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StationLayout {
    RuralHalt,
    TownStop,
    RegionalStation,
    FreightApron { regional: bool },
    FreightShed,
}

impl StationLayout {
    pub(crate) fn for_stop(anchor: RailAnchorKind, service: RailService, regional: bool) -> Self {
        match (service, anchor, regional) {
            (RailService::Freight, RailAnchorKind::Port, regional) => {
                Self::FreightApron { regional }
            }
            (RailService::Freight, _, _) => Self::FreightShed,
            (_, anchor, true) if anchor == RailAnchorKind::Town || anchor.is_port() => Self::RegionalStation,
            (_, anchor, false) if anchor == RailAnchorKind::Town || anchor.is_port() => Self::TownStop,
            _ => Self::RuralHalt,
        }
    }

    /// Material and height inside the surveyed roof envelope.
    pub(crate) fn roof(self, along: f32, across: f32) -> Option<(RailMaterial, f32)> {
        let span = along.abs();
        let width = across.abs();
        if span > 1.0 || width > 1.0 {
            return None;
        }
        use RailMaterial::{StationGlass, StationMetal, StationRust, StationTimber};
        Some(match self {
            // A short enclosed shelter with a steep gable and long exposed ends.
            Self::RuralHalt if span <= 0.53 => (StationTimber, 2.12 + 1.34 * (1.0 - width)),
            // Two thin, offset wings with a breezeway through the middle.
            Self::TownStop if along < -0.13 && across >= -0.53 => {
                (StationMetal, 2.88 + across * 0.34)
            }
            Self::TownStop if along > 0.10 && across <= 0.54 => {
                (StationMetal, 2.72 - across * 0.28)
            }
            // A prominent raised glass hall with shallow metal eaves.
            Self::RegionalStation if span <= 0.88 && width <= 0.63 => {
                (StationGlass, 3.74 - width * 0.55)
            }
            Self::RegionalStation => (StationMetal, 2.72),
            // Two unequal bays shelter the inland interchange. The light fascia
            // faces the unloading lane along the outer edge of the platform.
            Self::FreightShed if span <= 0.95 && across >= 0.56 => (StationMetal, 2.53),
            Self::FreightShed if along >= -0.20 => (StationRust, 3.57 - across * 0.33),
            Self::FreightShed if along >= -0.86 && across >= -0.76 => {
                (StationRust, 2.87 - across * 0.25)
            }
            _ => return None,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct StationFixtures {
    mounts: [Point; 2],
    along: [f32; 2],
    outside: [f32; 2],
    layout: StationLayout,
    mask: u8,
}

impl StationFixtures {
    pub(crate) fn new(
        mounts: [Point; 2],
        along: [f32; 2],
        outside: [f32; 2],
        layout: StationLayout,
        mask: u8,
    ) -> Self {
        Self {
            mounts,
            along,
            outside,
            layout,
            mask,
        }
    }

    /// A route may curve inside a station boundary. Its final trackbed, roof
    /// and access callback decide whether each separate piece actually fits.
    pub(crate) fn fit(&self, mut surface_at: impl FnMut(f32, f32) -> Option<RailMaterial>) -> u8 {
        let required = match self.layout {
            StationLayout::FreightApron { .. } | StationLayout::FreightShed => {
                RailMaterial::FreightPlatform
            }
            _ => RailMaterial::StationPlatform,
        };
        let mut approved = 0;
        for i in 0..self.mounts.len() {
            let [wide, deep] = self.footprint(i);
            if [-wide, 0.0, wide].into_iter().all(|u| {
                [-deep, 0.0, deep].into_iter().all(|v| {
                    let [x, _, z] = self.place(i, [u, 0.0, v]);
                    surface_at(x, z) == Some(required)
                })
            }) {
                approved |= 1 << i;
            }
        }
        approved
    }

    fn footprint(&self, i: usize) -> [f32; 2] {
        match (self.layout, i) {
            (StationLayout::FreightApron { regional: false }, _) => [2.56, 0.41],
            (StationLayout::FreightApron { regional: true }, _) => [1.42, 0.39],
            (StationLayout::FreightShed, _) => [1.51, 0.39],
            (StationLayout::RegionalStation, 1) => [0.77, 0.34],
            (_, 0) => [1.43, 0.21],
            _ => [1.42, 0.31],
        }
    }

    fn place(&self, i: usize, [u, h, v]: Point) -> Point {
        let [x, y, z] = self.mounts[i];
        [
            x + self.along[0] * u + self.outside[0] * v,
            y + h,
            z + self.along[1] * u + self.outside[1] * v,
        ]
    }

    fn bounds(&self, i: usize) -> [Point; 2] {
        let [wide, deep] = self.footprint(i);
        let [x, y, z] = self.mounts[i];
        let dx = self.along[0].abs() * wide + self.outside[0].abs() * deep;
        let dz = self.along[1].abs() * wide + self.outside[1].abs() * deep;
        let height = match (self.layout, i) {
            (StationLayout::FreightApron { .. }, 0) => 3.89,
            (StationLayout::FreightApron { .. }, _) => 1.49,
            (StationLayout::FreightShed, _) => 1.81,
            (StationLayout::RegionalStation, 1) => 2.08,
            (_, 0) => 2.72,
            _ => 1.20,
        };
        [[x - dx, y - 1.35, z - dz], [x + dx, y + height, z + dz]]
    }

    fn visible<'a>(
        &'a self,
        camera: Camera,
        projection: &'a ScreenProjection,
        range_squared: f32,
    ) -> impl Iterator<Item = (usize, f32, ScreenRect)> + 'a {
        (0..self.mounts.len()).filter_map(move |i| {
            if self.mask & (1 << i) == 0 {
                return None;
            }
            let [x, y, z] = self.mounts[i];
            let [dx, dy, dz] = [camera.x - x, camera.altitude - y, camera.z - z];
            let distance = dx * dx + dy * dy + dz * dz;
            if distance >= range_squared {
                return None;
            }
            let opacity = smoothstep((range_squared - distance) * (2.0 / range_squared));
            Some((i, opacity, projection.bounds(self.bounds(i))?))
        })
    }

    pub(crate) fn region(
        &self,
        camera: Camera,
        projection: &ScreenProjection,
        range_squared: f32,
    ) -> Option<ScreenRect> {
        self.visible(camera, projection, range_squared)
            .map(|(_, _, bounds)| bounds)
            .reduce(ScreenRect::union)
    }

    pub(crate) fn draw(
        &self,
        mesh: &mut WorldMesh<'_>,
        camera: Camera,
        projection: &ScreenProjection,
        range_squared: f32,
    ) {
        for (i, opacity, _) in self.visible(camera, projection, range_squared) {
            let mut art = Builder {
                mesh,
                opacity,
                fixtures: self,
                i,
            };
            match (self.layout, i) {
                (StationLayout::FreightApron { .. }, 0) => art.loading_gantry(),
                (StationLayout::FreightApron { .. }, _) => art.storage_lane(),
                (StationLayout::FreightShed, 0) => art.pallets([81, 114, 102]),
                (StationLayout::FreightShed, _) => art.pallets([170, 119, 67]),
                (StationLayout::RegionalStation, 1) => art.ticket_machine(),
                (StationLayout::RegionalStation, _) => art.notice([32, 102, 102]),
                (StationLayout::TownStop, 0) => art.notice([142, 72, 51]),
                (_, 0) => art.notice([46, 84, 115]),
                _ => art.bench(),
            }
        }
    }
}

struct Builder<'a, 'b> {
    mesh: &'a mut WorldMesh<'b>,
    fixtures: &'a StationFixtures,
    opacity: f32,
    i: usize,
}

impl Builder<'_, '_> {
    fn notice(&mut self, enamel: Color) {
        const STEEL: Color = [111, 130, 139];
        for u in [-1.02, 1.02] {
            self.block([u, -1.35, 0.0], [0.14, 3.53, 0.17], STEEL);
        }
        self.block([0.0, 1.72, 0.0], [2.80, 0.95, 0.24], enamel);
        self.block([0.0, 2.25, -0.132], [2.30, 0.22, 0.04], [231, 226, 201]);
        self.block([0.0, 2.25, 0.132], [2.30, 0.22, 0.04], [231, 226, 201]);
    }

    fn bench(&mut self) {
        for u in [-1.0, 1.0] {
            self.block([u, -1.35, 0.0], [0.20, 1.91, 0.40], [107, 117, 116]);
        }
        self.block([0.0, 0.56, -0.04], [2.82, 0.13, 0.52], [152, 107, 72]);
        self.block([0.0, 0.64, 0.245], [2.73, 0.51, 0.11], [139, 95, 62]);
    }

    fn ticket_machine(&mut self) {
        self.block([0.0, -1.35, 0.0], [1.45, 3.38, 0.62], [53, 102, 107]);
        self.block([0.0, 1.28, -0.318], [1.13, 0.53, 0.03], [213, 220, 191]);
        self.block([0.0, 0.84, -0.318], [0.64, 0.12, 0.03], [175, 141, 76]);
    }

    fn pallets(&mut self, cover: Color) {
        self.block([0.0, -1.35, 0.0], [2.96, 1.49, 0.73], [115, 83, 60]);
        self.block([-0.74, 0.14, 0.0], [1.43, 0.97, 0.72], [145, 110, 74]);
        self.block([0.69, 0.14, 0.0], [1.26, 0.70, 0.69], [173, 132, 84]);
        self.block([-0.74, 1.11, 0.0], [1.47, 0.64, 0.76], cover);
        self.block([0.69, 0.84, 0.0], [1.29, 0.12, 0.74], [193, 167, 113]);
    }

    fn loading_gantry(&mut self) {
        const FRAME: Color = [185, 126, 56];
        const STEEL: Color = [73, 83, 86];
        let [half_width, _] = self.fixtures.footprint(self.i);
        let upright = half_width - 0.36;
        for x in [-upright, upright] {
            self.block([x, -1.35, 0.0], [0.61, 1.71, 0.70], STEEL);
            self.block([x, 0.36, 0.0], [0.43, 3.06, 0.54], FRAME);
        }
        self.block([0.0, 3.26, 0.0], [2.0 * half_width, 0.61, 0.74], FRAME);
        self.block([0.32, 2.85, -0.02], [0.98, 0.41, 0.59], STEEL);
        self.block([0.32, 1.81, -0.02], [0.16, 1.04, 0.18], STEEL);
        self.block([0.32, 1.60, -0.02], [0.74, 0.21, 0.48], [194, 150, 57]);
    }

    fn storage_lane(&mut self) {
        let [half_width, _] = self.fixtures.footprint(self.i);
        const POSTS: Color = [93, 106, 116];
        for x in [-half_width + 0.17, 0.0, half_width - 0.17] {
            self.block([x, -1.35, 0.28], [0.21, 2.84, 0.20], POSTS);
        }
        self.block([0.0, 1.18, 0.28], [2.0 * half_width, 0.19, 0.18], [166, 166, 141]);
        self.block([0.0, 0.52, 0.28], [2.0 * half_width, 0.14, 0.17], POSTS);
        let left = -half_width * 0.45;
        let right = half_width * 0.43;
        self.block([left, -1.35, -0.06], [half_width * 0.76, 2.36, 0.58], [111, 143, 142]);
        self.block([left, 0.94, -0.06], [half_width * 0.71, 0.18, 0.52], [155, 172, 156]);
        self.block([right, -1.35, -0.06], [half_width * 0.71, 1.95, 0.55], [168, 115, 68]);
    }

    fn block(&mut self, [x, y, z]: Point, [width, height, depth]: Point, color: Color) {
        let corners = |grade| {
            let [w, d] = [width * 0.5, depth * 0.5];
            [
                [x - w, grade, z - d],
                [x + w, grade, z - d],
                [x + w, grade, z + d],
                [x - w, grade, z + d],
            ]
            .map(|point| self.fixtures.place(self.i, point))
        };
        let lower = corners(y);
        let upper = corners(y + height);
        for side in 0..4 {
            let next = (side + 1) % 4;
            let shade = [0.79, 0.85, 1.0, 0.92][side];
            let tint = color.map(|component| (f32::from(component) * shade) as u8);
            self.quad([lower[side], lower[next], upper[next], upper[side]], tint);
        }
        self.quad(upper, color);
    }

    fn quad(&mut self, [a, b, c, d]: [Point; 4], color: Color) {
        self.mesh.faded_triangle([a, b, c], color, self.opacity);
        self.mesh.faded_triangle([a, c, d], color, self.opacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facilities::{FacilitySample, RegionalFacilities};
    use crate::roads::RoadNetwork;
    use crate::scenery::DistrictRegion;
    use crate::settlement::Terminus;
    use crate::world::World;

    #[test]
    fn station_roles_select_distinct_roof_profiles_within_the_same_pad() {
        use RailAnchorKind::{Port, RoadTerminus, Town};
        use RailService::{Commuter, Freight};
        assert_eq!(StationLayout::for_stop(RoadTerminus, Commuter, false), StationLayout::RuralHalt);
        assert_eq!(StationLayout::for_stop(Town, Commuter, false), StationLayout::TownStop);
        assert_eq!(StationLayout::for_stop(Town, Commuter, true), StationLayout::RegionalStation);
        for regional in [true, false] {
            assert_eq!(
                StationLayout::for_stop(Port, Freight, regional),
                StationLayout::FreightApron { regional },
            );
        }
        assert_eq!(StationLayout::for_stop(Port, Commuter, false), StationLayout::TownStop);
        assert_eq!(StationLayout::for_stop(Town, Freight, false), StationLayout::FreightShed);

        let signatures = [
            StationLayout::RuralHalt,
            StationLayout::TownStop,
            StationLayout::RegionalStation,
            StationLayout::FreightApron { regional: false },
            StationLayout::FreightShed,
        ].map(|layout| {
            assert_eq!(layout.roof(1.01, 0.0), None);
            assert_eq!(layout.roof(0.0, -1.01), None);
            [[-0.85, 0.0], [0.0, -0.65], [0.0, 0.0], [0.0, 0.65]]
                .map(|[u, v]| layout.roof(u, v).map(|(material, height)| (material, height.to_bits())))
        });
        for (i, signature) in signatures.iter().enumerate() {
            assert!(!signatures[..i].contains(signature));
        }
        use RailMaterial::{StationGlass, StationMetal, StationRust, StationTimber};
        assert_eq!(StationLayout::RuralHalt.roof(0.0, 0.0).unwrap().0, StationTimber);
        assert_eq!(StationLayout::TownStop.roof(-0.6, 0.0).unwrap().0, StationMetal);
        assert_eq!(StationLayout::TownStop.roof(0.0, 0.0), None);
        assert_eq!(StationLayout::RegionalStation.roof(0.0, 0.0).unwrap().0, StationGlass);
        assert_eq!(StationLayout::RegionalStation.roof(0.0, 0.9).unwrap().0, StationMetal);
        assert_eq!(StationLayout::FreightShed.roof(0.6, 0.0).unwrap().0, StationRust);
        assert_eq!(StationLayout::FreightShed.roof(-0.6, 0.0).unwrap().0, StationRust);
        assert_eq!(StationLayout::FreightShed.roof(0.0, 0.75).unwrap().0, StationMetal);
        assert_eq!(StationLayout::FreightApron { regional: false }.roof(0.0, 0.75), None);
    }

    #[test]
    fn port_furnishings_stay_on_the_station_edges_at_both_rail_termini() {
        let world = World::new(1);
        let road = RoadNetwork::generate(&world, -1, -3).unwrap();
        let facilities = RegionalFacilities::prepare(&world, 1, &road, [None, None]);
        let rail = facilities.rail().unwrap();
        for station in rail.stations() {
            let fixtures = station.fixtures(rail.service);
            assert_eq!(fixtures.layout, StationLayout::TownStop);
            assert_ne!(fixtures.mask, 0);
            assert_eq!(
                fixtures.mask,
                fixtures.fit(|x, z| {
                    match facilities.sample(x, z, world.sample(x, z)) {
                        Some(FacilitySample::Rail(piece)) => Some(piece.material),
                        _ => None,
                    }
                })
            );
            let (x, z) = station.road_contact();
            assert_eq!(
                rail.sample(x, z).unwrap().material,
                RailMaterial::StationAccess
            );
            assert_eq!(fixtures.fit(|_, _| Some(RailMaterial::StationAccess)), 0);
            assert_eq!(fixtures.fit(|_, _| Some(RailMaterial::StationRust)), 0);
        }
    }

    #[test]
    fn retained_freight_terminal_furnishings_fit_the_real_grade_and_leave_access_clear() {
        let world = World::new(130);
        let district = DistrictRegion::build(&world, 11, 4);
        let towns = [Terminus::Start, Terminus::End].map(|end| district.town(end));
        let facilities = RegionalFacilities::prepare(&world, 130, district.road().unwrap(), towns);
        let rail = facilities.rail().unwrap();
        assert_eq!(rail.service, RailService::Freight);
        let [town, port] = rail.stations();
        assert_eq!(town.anchor.kind, RailAnchorKind::Town);
        assert_eq!(port.anchor.kind, RailAnchorKind::Port);
        for (station, layout) in [
            (town, StationLayout::FreightShed),
            (port, StationLayout::FreightApron { regional: false }),
        ] {
            let fixtures = station.fixtures(rail.service);
            assert_eq!(fixtures.layout, layout);
            assert_eq!(fixtures.mask, 0b11, "fixtures must fit {layout:?}");
            assert_eq!(
                fixtures.fit(|x, z| rail.sample(x, z).map(|piece| piece.material)),
                0b11,
            );
            assert_eq!(fixtures.fit(|_, _| Some(RailMaterial::StationPlatform)), 0);
            let (x, z) = station.road_contact();
            assert_eq!(rail.sample(x, z).unwrap().material, RailMaterial::StationAccess);
            // Check the whole occupied strip, including the small spaces between
            // survey probes. None of the structures can overlap the track or access.
            for i in 0..fixtures.mounts.len() {
                let [wide, deep] = fixtures.footprint(i);
                for u in -8..=8 {
                    for v in -4..=4 {
                        let point = [wide * u as f32 / 8.0, 0.0, deep * v as f32 / 4.0];
                        let [x, _, z] = fixtures.place(i, point);
                        assert_eq!(
                            rail.sample(x, z).unwrap().material, RailMaterial::FreightPlatform,
                            "load strip {layout:?} at {x},{z}"
                        );
                    }
                }
            }
        }
    }
}
