//! Small permanent objects inside the already surveyed station boundary.

use crate::math::smoothstep;
use crate::rail::{GrainTerminal, RailAnchorKind, RailMaterial, RailService};
use crate::render::{Camera, ScreenProjection, ScreenRect};
use crate::train_art::FreightRoute;
use crate::world_mesh::WorldMesh;

type Point = [f32; 3];
type Color = [u8; 3];

/// Model coordinates share the surveyed pad axes, with positive `across`
/// toward truck receiving and negative `across` toward the railway.
pub(crate) struct GrainElevatorArt {
    origin: Point,
    along: [f32; 2],
    outward: [f32; 2],
    track: Point,
}

impl GrainElevatorArt {
    pub(crate) fn new(terminal: &GrainTerminal) -> Self {
        Self { origin: terminal.origin, along: terminal.along, outward: terminal.outward, track: terminal.track }
    }

    pub(crate) fn visible(&self, camera: Camera, dimensions: (usize, usize)) -> Option<(ScreenRect, f32)> {
        let reach = dimensions.0.min(dimensions.1) as f32 * 1.17 + 19.0;
        let dx = self.origin[0] - camera.x;
        let dy = self.origin[1] - camera.altitude;
        let dz = self.origin[2] - camera.z;
        let distance = dx * dx + dy * dy + dz * dz;
        if distance >= reach * reach {
            return None;
        }
        let opacity = smoothstep((1.0 - distance / (reach * reach)) * 2.5);
        let bounds = ScreenProjection::new(camera, dimensions)?.bounds(self.bounds())?;
        Some((bounds, opacity))
    }

    fn bounds(&self) -> [Point; 2] {
        let [x, y, z] = self.origin;
        let dx = self.along[0].abs() * GrainTerminal::HALF_LENGTH
            + self.outward[0].abs() * GrainTerminal::HALF_WIDTH;
        let dz = self.along[1].abs() * GrainTerminal::HALF_LENGTH
            + self.outward[1].abs() * GrainTerminal::HALF_WIDTH;
        let [tx, ty, tz] = self.track;
        [
            [(x - dx).min(tx - 0.78), (y - 0.40).min(ty + 4.65), (z - dz).min(tz - 0.78)],
            [(x + dx).max(tx + 0.78), (y + GrainTerminal::MAX_HEIGHT).max(ty + 6.62), (z + dz).max(tz + 0.78)],
        ]
    }

    fn point(&self, [along, height, across]: Point) -> Point {
        let [x, y, z] = self.origin;
        [x + self.along[0] * along + self.outward[0] * across, y + height,
         z + self.along[1] * along + self.outward[1] * across]
    }

    pub(crate) fn draw(&self, mesh: &mut WorldMesh<'_>, opacity: f32) {
        self.faces(|triangle, color| mesh.faded_triangle(triangle, color, opacity));
    }

    fn faces(&self, mut emit: impl FnMut([Point; 3], Color)) {
        let placement = u64::from(self.origin[0].to_bits()) << 32 | u64::from(self.origin[2].to_bits());
        let variant = crate::math::mix(placement) % 3;
        let bins: &[[f32; 3]] = match variant {
            0 => &[[-7.35, 2.65, 7.05], [-0.15, 2.72, 7.80]],
            1 => &[[-8.30, 2.45, 6.10], [-1.70, 2.65, 7.65], [4.95, 2.48, 6.95]],
            _ => &[[-8.95, 2.05, 6.60], [-4.14, 2.12, 6.92], [0.65, 2.08, 7.13], [5.45, 2.16, 6.52]],
        };
        let mut model = GrainMesh { emit: &mut emit, site: self };
        model.quad([[-11.20, 0.07, 3.92], [11.20, 0.07, 3.92], [11.20, 0.07, 5.94], [-11.20, 0.07, 5.94]], [143, 128, 102]);
        for &[along, radius, height] in bins {
            model.bin(along, radius, height);
        }
        // The truck pull-through and low receiving house sit outside the bin row.
        model.block([9.68, -0.36, 2.25], [3.40, 3.36, 3.78], [151, 137, 114]);
        model.block([9.68, 2.78, 2.25], [3.66, 0.63, 4.02], [99, 109, 106]);
        model.quad([[8.68, 0.09, 4.154], [10.72, 0.09, 4.154], [10.72, 2.32, 4.154], [8.68, 2.32, 4.154]], [60, 69, 68]);
        // The headhouse is higher than its bins and feeds the covered gallery.
        model.block([0.0, -0.36, -3.48], [3.36, 10.16, 2.96], [168, 156, 130]);
        model.block([0.0, 9.50, -3.48], [3.63, 0.44, 3.21], [216, 205, 173]);
        model.block([0.0, 9.87, -3.48], [2.74, 1.26, 2.60], [185, 126, 77]);
        model.load_track();
    }
}

struct GrainMesh<'a, F> {
    emit: &'a mut F,
    site: &'a GrainElevatorArt,
}

impl<F: FnMut([Point; 3], Color)> GrainMesh<'_, F> {
    fn triangle(&mut self, triangle: [Point; 3], color: Color) {
        (self.emit)(triangle.map(|point| self.site.point(point)), color);
    }

    fn quad(&mut self, [a, b, c, d]: [Point; 4], color: Color) {
        self.triangle([a, b, c], color);
        self.triangle([a, c, d], color);
    }

    fn block(&mut self, [x, y, z]: Point, [width, height, depth]: Point, color: Color) {
        let [w, d] = [width * 0.5, depth * 0.5];
        let corners = |level| [
            [x - w, level, z - d], [x + w, level, z - d],
            [x + w, level, z + d], [x - w, level, z + d],
        ];
        let lower = corners(y);
        let upper = corners(y + height);
        for (i, light) in [1.0, 0.82, 0.95, 0.76].into_iter().enumerate() {
            let next = (i + 1) % lower.len();
            let color = color.map(|component| (f32::from(component) * light) as u8);
            self.quad([lower[i], lower[next], upper[next], upper[i]], color);
        }
        self.quad(upper, color);
    }

    fn bin(&mut self, along: f32, radius: f32, height: f32) {
        const D: f32 = core::f32::consts::FRAC_1_SQRT_2;
        const RING: [[f32; 2]; 8] = [[1.0, 0.0], [D, D], [0.0, 1.0], [-D, D],
                                    [-1.0, 0.0], [-D, -D], [0.0, -1.0], [D, -D]];
        const FACETS: [Color; 8] = [[151, 158, 159], [181, 186, 178], [200, 201, 186], [185, 189, 179],
                                   [154, 164, 163], [127, 142, 146], [119, 134, 143], [136, 149, 150]];
        let side = 1.12;
        let floor = RING.map(|[u, v]| [along + radius * u, -0.37, side + radius * v]);
        let edge = RING.map(|[u, v]| [along + radius * u, height, side + radius * v]);
        let peak = [along, height + radius * 0.56, side];
        for (i, color) in FACETS.into_iter().enumerate() {
            let next = (i + 1) % FACETS.len();
            self.quad([floor[i], floor[next], edge[next], edge[i]], color);
            self.triangle([edge[i], edge[next], peak], color.map(|shade| shade.saturating_add(28)));
        }
    }

    fn load_track(&mut self) {
        let dx = self.site.track[0] - self.site.origin[0];
        let dz = self.site.track[2] - self.site.origin[2];
        let rail_along = dx * self.site.along[0] + dz * self.site.along[1];
        let rail_across = dx * self.site.outward[0] + dz * self.site.outward[1];
        let rail_grade = self.site.track[1] - self.site.origin[1];
        let end = rail_grade + 6.22;
        let [left, right] = [rail_along - 0.51, rail_along + 0.51];
        let [sl, sr] = [-0.48, 0.48];
        let [sh, sz] = [8.81, -4.92];
        self.quad([[sl, sh, sz], [sr, sh, sz], [right, end, rail_across], [left, end, rail_across]], [202, 177, 111]);
        self.quad([[sl, sh - 0.74, sz], [sl, sh, sz], [left, end, rail_across], [left, end - 0.72, rail_across]], [132, 119, 87]);
        self.quad([[sr, sh, sz], [sr, sh - 0.74, sz], [right, end - 0.72, rail_across], [right, end, rail_across]], [115, 106, 84]);
        self.block([rail_along, rail_grade + 4.65, rail_across], [0.77, 1.58, 0.78], [130, 113, 81]);
        self.block([rail_along, end - 0.25, rail_across], [1.39, 0.55, 1.34], [205, 172, 103]);
    }
}

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
        cargo: FreightRoute,
    ) {
        for (i, opacity, _) in self.visible(camera, projection, range_squared) {
            let mut art = Builder {
                mesh,
                opacity,
                fixtures: self,
                i,
            };
            match (self.layout, i) {
                (StationLayout::FreightApron { .. }, 0)
                    if matches!(cargo, FreightRoute::Aggregate | FreightRoute::Quarry) => art.bulk_feeder(),
                (StationLayout::FreightApron { .. }, 0) => art.loading_gantry(),
                (StationLayout::FreightApron { .. }, _) => art.storage_lane(cargo),
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

    fn bulk_feeder(&mut self) {
        const STEEL: Color = [82, 95, 97];
        let [half_width, _] = self.fixtures.footprint(self.i);
        for x in [-half_width * 0.73, half_width * 0.73] {
            self.block([x, -1.35, 0.0], [0.53, 1.67, 0.70], [92, 94, 91]);
            self.block([x, 0.32, 0.0], [0.34, 2.82, 0.54], STEEL);
        }
        // The upper bin tapers into a sloped chute on the track-facing side.
        self.block([0.0, 2.87, 0.0], [half_width * 1.84, 0.21, 0.67], STEEL);
        self.tapered_load([0.0, 2.20, 0.0], [half_width * 0.39, 0.12],
                         [0.0, 3.61, 0.0], [half_width * 0.88, 0.36], [102, 115, 114]);
        self.block([0.0, 1.87, -0.15], [0.64, 0.35, 0.39], [151, 126, 80]);
    }

    fn storage_lane(&mut self, cargo: FreightRoute) {
        let [half_width, _] = self.fixtures.footprint(self.i);
        const POSTS: Color = [93, 106, 116];
        for x in [-half_width + 0.17, 0.0, half_width - 0.17] {
            self.block([x, -1.35, 0.28], [0.21, 2.84, 0.20], POSTS);
        }
        self.block([0.0, 1.18, 0.28], [2.0 * half_width, 0.19, 0.18], [166, 166, 141]);
        self.block([0.0, 0.52, 0.28], [2.0 * half_width, 0.14, 0.17], POSTS);
        let left = -half_width * 0.45;
        let right = half_width * 0.43;
        match cargo {
            FreightRoute::Aggregate | FreightRoute::Quarry => {
                self.tapered_load([left, -1.35, -0.07], [half_width * 0.41, 0.28],
                                  [left + 0.08, 1.08, -0.08], [half_width * 0.17, 0.08], [171, 153, 116]);
                self.tapered_load([right, -1.35, -0.05], [half_width * 0.43, 0.28],
                                  [right - 0.09, 0.69, -0.11], [half_width * 0.19, 0.10], [139, 145, 133]);
            }
            FreightRoute::Intermodal => {
                self.block([left, -1.35, -0.06], [half_width * 0.84, 2.70, 0.63], [151, 76, 58]);
                self.block([left, 1.27, -0.06], [half_width * 0.75, 0.16, 0.52], [181, 108, 80]);
                self.block([right, -1.35, -0.05], [half_width * 0.74, 2.44, 0.57], [62, 127, 146]);
                self.block([right, 1.03, -0.05], [half_width * 0.65, 0.12, 0.50], [110, 160, 164]);
            }
            _ => {
                self.block([left, -1.35, -0.06], [half_width * 0.76, 2.36, 0.58], [111, 143, 142]);
                self.block([left, 0.94, -0.06], [half_width * 0.71, 0.18, 0.52], [155, 172, 156]);
                self.block([right, -1.35, -0.06], [half_width * 0.71, 1.95, 0.55], [168, 115, 68]);
            }
        }
    }

    fn tapered_load(&mut self, foot: Point, lower_size: [f32; 2], cap: Point, upper_size: [f32; 2], color: Color) {
        let corners = |[x, height, z]: Point, [w, d]: [f32; 2]| {
            [
                [x - w, height, z - d],
                [x + w, height, z - d],
                [x + w, height, z + d],
                [x - w, height, z + d],
            ]
            .map(|point| self.fixtures.place(self.i, point))
        };
        let lower = corners(foot, lower_size);
        let upper = corners(cap, upper_size);
        for side in 0..4 {
            let next = (side + 1) % 4;
            let shade = [0.79, 0.85, 1.0, 0.92][side];
            let tint = color.map(|component| (f32::from(component) * shade) as u8);
            self.quad([lower[side], lower[next], upper[next], upper[side]], tint);
        }
        self.quad(upper, color);
    }

    fn block(&mut self, [x, y, z]: Point, [width, height, depth]: Point, color: Color) {
        let size = [width * 0.5, depth * 0.5];
        self.tapered_load([x, y, z], size, [x, y + height, z], size, color);
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
    use crate::render::render_terrain_layer_region;
    use crate::scenery::DistrictRegion;
    use crate::settlement::Terminus;
    use crate::sky::Sky;
    use crate::world::World;

    #[test]
    fn grain_elevator_geometry_stays_in_the_pad_or_clear_loading_corridor() {
        let along = [0.6, 0.8];
        let outward = [-0.8, 0.6];
        for position in 0..12 {
            let origin = [512.0 + position as f32 * 24.0, 9.5, -300.0];
            let track = [origin[0] - 13.0 * outward[0], origin[1] + 1.3, origin[2] - 13.0 * outward[1]];
            let scene = GrainElevatorArt { origin, along, outward, track };
            let [low, high] = scene.bounds();
            let mut count = 0;
            scene.faces(|triangle, _| {
                count += 1;
                for point in triangle {
                    assert!((0..3).all(|axis| (low[axis] - 0.001..=high[axis] + 0.001).contains(&point[axis])));
                    let dx = point[0] - origin[0];
                    let dz = point[2] - origin[2];
                    let along = dx * along[0] + dz * along[1];
                    let across = dx * outward[0] + dz * outward[1];
                    if across >= -6.001 {
                        assert!(along.abs() <= 13.001 && across <= 6.001);
                        assert!((origin[1] - 0.401..=origin[1] + 11.501).contains(&point[1]));
                    } else {
                        assert!(along.abs() < 2.0 && across >= -14.0);
                        assert!(point[1] >= track[1] + 4.649);
                    }
                }
            });
            assert!((115..=195).contains(&count), "grain elevator produced {count} triangles");
        }
    }

    #[test]
    fn grain_elevator_depth_region_covers_the_complete_draw_at_sixty_pixels() {
        use crate::world::{Sample, Surface};

        const SIDE: usize = 60;
        let dimensions = (SIDE, SIDE);
        let scene = GrainElevatorArt {
            origin: [0.0, 10.0, 31.0], along: [1.0, 0.0], outward: [0.0, 1.0],
            track: [0.0, 9.5, 18.0],
        };
        let sky = Sky::from_time(1, 0, 0.46);
        for (x, roll) in [(14.0_f32, 0.21), (-13.0, -0.37)] {
            let camera = Camera { x, z: -2.0, altitude: 25.0,
                                  heading: (-x).atan2(33.0), pitch: 0.24, bank: roll };
            let (visible, opacity) = scene.visible(camera, dimensions).unwrap();
            let draw = |region, scenery: bool| {
                let mut frame = [0_u16; SIDE * SIDE];
                let mut work = frame;
                let mut depth = [u16::MAX; SIDE * SIDE];
                // The raised slope hides the foot of the loader from one side.
                let sample = |x: f32, z: f32| Sample {
                    elevation: if x > 2.0 && z > 11.0 && z < 19.0 { 14.8 } else { 10.0 },
                    surface: Surface::Grass,
                };
                render_terrain_layer_region(
                    (sample, |_, _, g: Sample| g.surface), camera, &sky, &mut frame,
                    &mut work, Some((&mut depth, region)), dimensions, |_| {},
                ).unwrap();
                if scenery {
                    let mut mesh = WorldMesh::new(&mut frame, &mut depth, dimensions, camera, &sky).unwrap();
                    scene.draw(&mut mesh, opacity);
                }
                frame
            };
            let expected = draw(ScreenRect::full(dimensions), true);
            assert_eq!(draw(visible, true), expected);
            let ground = draw(visible, false);
            let mut visible_pixels = 0;
            for (index, (&a, &b)) in expected.iter().zip(&ground).enumerate() {
                if a != b {
                    let (x, y) = (index % SIDE, index / SIDE);
                    assert!((visible.left..visible.right).contains(&x));
                    assert!((visible.top..visible.bottom).contains(&y));
                    visible_pixels += 1;
                }
            }
            assert!(visible_pixels > 30, "model too small at the game raster");
        }
    }

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
