//! Small permanent objects inside the already surveyed station boundary.

use crate::math::smoothstep;
use crate::rail::RailMaterial;
use crate::render::{Camera, ScreenProjection, ScreenRect};
use crate::world_mesh::WorldMesh;

type Point = [f32; 3];
type Color = [u8; 3];

#[derive(Clone, Copy)]
pub(crate) struct StationFixtures {
    mounts: [Point; 2],
    along: [f32; 2],
    outside: [f32; 2],
    port: bool,
    mask: u8,
}

impl StationFixtures {
    pub(crate) fn new(
        mounts: [Point; 2],
        along: [f32; 2],
        outside: [f32; 2],
        port: bool,
        mask: u8,
    ) -> Self {
        Self {
            mounts,
            along,
            outside,
            port,
            mask,
        }
    }

    /// A route may curve inside a station boundary. Its final trackbed, roof
    /// and access callback decide whether each separate piece actually fits.
    pub(crate) fn fit(&self, mut surface_at: impl FnMut(f32, f32) -> Option<RailMaterial>) -> u8 {
        let mut approved = 0;
        for i in 0..self.mounts.len() {
            let [wide, deep] = self.footprint(i);
            if [-wide, 0.0, wide].into_iter().all(|u| {
                [-deep, 0.0, deep].into_iter().all(|v| {
                    let [x, _, z] = self.place(i, [u, 0.0, v]);
                    matches!(
                        surface_at(x, z),
                        Some(RailMaterial::StationPlatform | RailMaterial::FreightPlatform)
                    )
                })
            }) {
                approved |= 1 << i;
            }
        }
        approved
    }

    fn footprint(&self, i: usize) -> [f32; 2] {
        match (self.port, i) {
            (false, 0) => [1.43, 0.21],
            (false, _) => [1.42, 0.31],
            (true, 0) => [1.51, 0.39],
            (true, _) => [0.94, 0.33],
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
        let height = match (self.port, i) {
            (false, 0) => 2.72,
            (false, _) => 1.20,
            (true, 0) => 1.79,
            (true, _) => 1.76,
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
            match (self.port, i) {
                (false, 0) => art.notice(),
                (false, _) => art.bench(),
                (true, 0) => art.pallets(),
                (true, _) => art.loading_marker(),
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
    fn notice(&mut self) {
        const STEEL: Color = [111, 130, 139];
        for u in [-1.02, 1.02] {
            self.block([u, -1.35, 0.0], [0.14, 3.53, 0.17], STEEL);
        }
        self.block([0.0, 1.72, 0.0], [2.80, 0.95, 0.24], [46, 84, 115]);
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

    fn pallets(&mut self) {
        self.block([0.0, -1.35, 0.0], [2.96, 1.49, 0.73], [115, 83, 60]);
        self.block([-0.74, 0.14, 0.0], [1.43, 0.97, 0.72], [145, 110, 74]);
        self.block([0.69, 0.14, 0.0], [1.26, 0.70, 0.69], [173, 132, 84]);
        self.block([-0.74, 1.11, 0.0], [1.47, 0.64, 0.76], [81, 114, 102]);
        self.block([0.69, 0.84, 0.0], [1.29, 0.12, 0.74], [193, 167, 113]);
    }

    fn loading_marker(&mut self) {
        self.block([0.0, -1.35, 0.0], [0.18, 3.11, 0.20], [124, 132, 130]);
        self.block([0.0, 1.09, 0.0], [1.83, 0.61, 0.19], [196, 150, 60]);
        self.block([0.0, 1.33, 0.096], [1.52, 0.10, 0.035], [68, 73, 71]);
        self.block([0.0, 1.33, -0.096], [1.52, 0.10, 0.035], [68, 73, 71]);
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
    use crate::world::World;

    #[test]
    fn port_furnishings_stay_on_the_station_edges_at_both_rail_termini() {
        let world = World::new(1);
        let road = RoadNetwork::generate(&world, -1, -3).unwrap();
        let facilities = RegionalFacilities::prepare(&world, 1, &road, [None, None]);
        let rail = facilities.rail().unwrap();
        for station in rail.stations() {
            let fixtures = station.fixtures();
            assert!(fixtures.port);
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
            assert_eq!(fixtures.fit(|_, _| Some(RailMaterial::StationRoof)), 0);
        }
    }
}
