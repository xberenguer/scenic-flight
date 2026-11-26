//! Prismatic bedrock exposed in the headwall above a talus apron.

use crate::math::{fraction, mix};
use crate::render::{Camera, ScreenProjection, ScreenRect};
use crate::world::{Surface, World};
use crate::world_mesh::{WorldMesh, bounds_in_view};

use super::HeadwallProfile;

const COUNT: usize = 8;
const ACROSS: [f32; COUNT] = [-17.6, -12.9, -7.7, -2.5, 2.8, 8.2, 13.5, 1.9];
const HEX: [[f32; 2]; 6] = [
    [1.0, 0.0],
    [0.5, 0.8660254],
    [-0.5, 0.8660254],
    [-1.0, 0.0],
    [-0.5, -0.8660254],
    [0.5, -0.8660254],
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct Column {
    center: [f32; 2],
    toe: f32,
    crown: f32,
    radius: f32,
    darkening: u8,
}

impl Column {
    const EMPTY: Self = Self {
        center: [0.0; 2],
        toe: 0.0,
        crown: 0.0,
        radius: 0.0,
        darkening: 0,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnarHeadwall {
    downhill: [f32; 2],
    columns: [Column; COUNT],
    bounds: [f32; 4],
    vertical: [f32; 2],
    profile: u8,
}

impl ColumnarHeadwall {
    pub(super) fn survey(
        world: &World,
        key: u64,
        center: [f32; 2],
        downhill: [f32; 2],
        length: f32,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let profile_id =
            (mix(key ^ 0x351c_271d_f24b_8720) % HeadwallProfile::ALL.len() as u64) as u8;
        let profile = HeadwallProfile::ALL[usize::from(profile_id)];
        let mut face = Self::empty(downhill, profile_id);
        let mirror = if key & 8 == 0 { -1.0 } else { 1.0 };
        let back = -length * 0.58;
        for (index, across) in ACROSS.into_iter().enumerate() {
            pulse();
            let retained = profile.crowns[index];
            if retained == 0.0 {
                continue;
            }
            let advance = profile.advances[index];
            let radius = profile.radii[index];
            let across = across * profile.spread;
            let seed = mix(key ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
            let [x, z] = local(center, downhill, back + advance, across * mirror);
            let native = world.sample(x, z);
            if native.surface != Surface::Rock {
                continue;
            }
            let radius = radius * (0.89 + 0.23 * fraction(seed));
            let crest = local([x, z], downhill, -22.0, 0.0);
            let upstream = world.sample(crest[0], crest[1]);
            let inherited_relief = upstream.elevation - native.elevation;
            if upstream.surface != Surface::Rock || !(6.0..=24.0).contains(&inherited_relief) {
                continue;
            }
            let mut toe = native.elevation;
            let mut high = native.elevation;
            for [u, v] in HEX {
                let p = local([x, z], downhill, v * radius, u * radius);
                let h = world.elevation(p[0], p[1]);
                toe = toe.min(h);
                high = high.max(h);
            }
            if toe < 86.0 || high - toe > 6.2 {
                continue;
            }
            let erosion = 0.84 + 0.16 * fraction(mix(seed));
            let crown = native.elevation + (inherited_relief * retained * erosion).min(14.0);
            if crown < high + 1.8 {
                continue;
            }
            let toe = toe - 0.75;
            face.place(
                index,
                Column {
                    center: [x, z],
                    toe,
                    crown,
                    radius,
                    darkening: (seed >> 61) as u8,
                },
            );
        }

        let pillars = face
            .columns
            .iter()
            .filter(|column| column.radius > 0.0)
            .count();
        let relief = face
            .columns
            .iter()
            .any(|pillar| pillar.crown - pillar.toe > 11.0);
        let width = (face.bounds[2] - face.bounds[0]).max(face.bounds[3] - face.bounds[1]);
        (pillars >= 5 && width >= 24.0 && relief).then_some(face)
    }

    fn empty(downhill: [f32; 2], profile: u8) -> Self {
        Self {
            downhill,
            columns: [Column::EMPTY; COUNT],
            bounds: [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ],
            vertical: [f32::INFINITY, f32::NEG_INFINITY],
            profile,
        }
    }

    fn place(&mut self, index: usize, column: Column) {
        let [x, z] = column.center;
        let shell = column.radius + 1.25;
        self.columns[index] = column;
        self.bounds[0] = self.bounds[0].min(x - shell);
        self.bounds[1] = self.bounds[1].min(z - shell);
        self.bounds[2] = self.bounds[2].max(x + shell);
        self.bounds[3] = self.bounds[3].max(z + shell);
        self.vertical[0] = self.vertical[0].min(column.toe);
        self.vertical[1] = self.vertical[1].max(column.crown + 0.35);
    }

    pub fn profile(&self) -> HeadwallProfile {
        HeadwallProfile::ALL[usize::from(self.profile)]
    }

    /// An isolated face using the same column geometry as a surveyed site.
    pub fn artwork(index: usize) -> Self {
        let mut face = Self::empty([0.0, 1.0], index as u8);
        let profile = HeadwallProfile::ALL[index];
        for (column, across) in ACROSS.into_iter().enumerate() {
            let retained = profile.crowns[column];
            if retained > 0.0 {
                let advance = profile.advances[column];
                let toe = 4.5 - 0.47 * advance;
                face.place(
                    column,
                    Column {
                        center: [-across * profile.spread, advance],
                        toe,
                        crown: toe + (14.2 * retained).min(14.0),
                        radius: profile.radii[column],
                        darkening: (column % 4) as u8,
                    },
                );
            }
        }
        face
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub fn crown(&self) -> f32 {
        self.vertical[1]
    }

    pub fn screen_region(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        if !bounds_in_view(camera, self.bounds) {
            return None;
        }
        let [left, near, right, far] = self.bounds;
        ScreenProjection::new(camera, dimensions)?.bounds([
            [left, self.vertical[0], near],
            [right, self.vertical[1], far],
        ])
    }

    pub(crate) fn obstruction(&self, x: f32, z: f32) -> f32 {
        let [left, near, right, far] = self.bounds;
        if !(left..=right).contains(&x) || !(near..=far).contains(&z) {
            return 0.0;
        }
        self.columns
            .iter()
            .filter_map(|column| {
                let [dx, dz] = [x - column.center[0], z - column.center[1]];
                let margin = column.radius + 1.25;
                (column.radius > 0.0 && dx * dx + dz * dz <= margin * margin)
                    .then_some(column.crown + 0.35)
            })
            .fold(0.0, f32::max)
    }

    pub fn draw(&self, camera: Camera, mesh: &mut WorldMesh<'_>) {
        if !bounds_in_view(camera, self.bounds) {
            return;
        }
        for column in self.columns.iter().filter(|column| column.radius > 0.0) {
            let base = HEX.map(|[u, v]| {
                let [x, z] = local(
                    column.center,
                    self.downhill,
                    v * column.radius,
                    u * column.radius,
                );
                [x, column.toe, z]
            });
            let top = core::array::from_fn::<_, 6, _>(|i| {
                let [u, v] = HEX[i];
                let radius = column.radius * [0.92, 0.98, 1.0, 0.95, 0.90, 0.96][i];
                let [x, z] = local(column.center, self.downhill, v * radius - 0.42, u * radius);
                [
                    x,
                    column.crown + [0.32, 0.10, -0.16, -0.30, -0.10, 0.18][i],
                    z,
                ]
            });
            let [x, z] = local(column.center, self.downhill, -0.42, 0.0);
            let crown = [x, column.crown + 0.12, z];
            let fade = column.darkening;
            for i in 0..6 {
                let j = (i + 1) % 6;
                let light = [12, 4, 17, 8, 0, 9][i];
                let side = [73 + light - fade, 77 + light - fade, 83 + light - fade];
                mesh.lit_triangle([base[i], base[j], top[j]], side);
                mesh.lit_triangle([base[i], top[j], top[i]], side);
                mesh.lit_triangle(
                    [top[i], top[j], crown],
                    [116 - fade, 121 - fade, 127 - fade],
                );
            }
        }
    }
}

fn local([x, z]: [f32; 2], [dx, dz]: [f32; 2], along: f32, across: f32) -> [f32; 2] {
    [x + dx * along - dz * across, z + dz * along + dx * across]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::Sky;

    #[test]
    fn headwall_profiles_remain_distinct_in_the_distant_raster() {
        const SIDE: usize = 60;
        let mut shapes = std::collections::HashSet::new();
        let sky = Sky::from_time(2, 0, 0.46);
        for (index, profile) in HeadwallProfile::ALL.iter().enumerate() {
            let face = ColumnarHeadwall::artwork(index);
            let mut signature = std::vec::Vec::new();
            for distance in [95.0, 145.0, 205.0] {
                for heading in [2.52_f32, 3.10, 3.76] {
                    let (sin, cos) = heading.sin_cos();
                    let camera = Camera {
                        x: -distance * sin,
                        z: -distance * cos,
                        altitude: 27.0,
                        heading,
                        pitch: ((27.0 - 9.0) / distance).atan(),
                        bank: 0.0,
                    };
                    let mut pixels = [0; SIDE * SIDE];
                    let mut depth = [u16::MAX; SIDE * SIDE];
                    let mut mesh =
                        WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky)
                            .unwrap();
                    face.draw(camera, &mut mesh);
                    let form = pixels.map(|pixel| pixel != 0);
                    assert!(
                        form.iter().filter(|&&lit| lit).count() >= 5,
                        "{}",
                        profile.name
                    );
                    signature.extend(form);
                }
            }
            assert!(
                shapes.insert(signature),
                "{} duplicates another headwall",
                profile.name
            );
        }
    }
}
