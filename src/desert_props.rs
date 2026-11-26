//! Low-poly foreground silhouettes anchored to the surveyed desert surface.

use crate::math::sin_cos;
use crate::render::{Camera, ScreenProjection, ScreenRect, floor_i32};
use crate::world::{Sample, Surface};
use crate::world_mesh::WorldMesh;

const CELL: f32 = 12.0;
const REACH: f32 = 64.0;
const HALF_SPAN: f32 = 3.1;
const HEIGHT: f32 = 4.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PropKind {
    Cactus,
    Rock,
    Bush,
}

#[derive(Clone, Copy)]
struct Prop {
    origin: [f32; 3],
    size: f32,
    kind: PropKind,
    variant: u8,
    opacity: f32,
}

impl Prop {
    fn triangles(self, emit: impl FnMut([[f32; 3]; 3], [u8; 3])) {
        // Four independent, four-step form traits make 256 meshes per kind.
        let [a, b, c, d] = [0, 2, 4, 6].map(|shift| usize::from((self.variant >> shift) & 3));
        match self.kind {
            PropKind::Cactus => cactus_triangles(a, b, c, d, emit),
            PropKind::Rock => rock_triangles(a, b, c, d, emit),
            PropKind::Bush => bush_triangles(a, b, c, d, emit),
        }
    }
}

fn cactus_triangles(
    girth: usize, height: usize, reach: usize, arrangement: usize,
    mut triangle: impl FnMut([[f32; 3]; 3], [u8; 3]),
) {
    let stem = [0.28, 0.39, 0.50, 0.61][girth];
    let height = [2.35, 2.90, 3.45, 4.00][height];
    let arm = [0.69, 0.94, 1.22, 1.50][reach];
    let [left_fork, left_tip, right_fork, right_tip] = [
        [0.30, 0.58, 0.49, 0.82],
        [0.48, 0.81, 0.25, 0.56],
        [0.36, 0.65, 0.34, 0.69],
        [0.52, 0.91, 0.43, 0.70],
    ][arrangement];
    let lean = [-0.25, 0.28, -0.07, 0.14][arrangement];
    for swap in [false, true] {
        let at = |x, y| if swap { [0.0, y, x] } else { [x, y, 0.0] };
        triangle([at(-stem, 0.0), at(stem, 0.0), at(lean, height)], [51, 90, 60]);
        triangle([
            at(lean * left_fork, height * left_fork),
            at(-arm, height * left_fork + 0.22),
            at(-arm * 0.85, height * left_tip),
        ], [77, 115, 67]);
        triangle([
            at(lean * right_fork, height * right_fork),
            at(arm * 0.85, height * right_fork + 0.21),
            at(arm * 0.74, height * right_tip),
        ], [89, 122, 68]);
    }
}

fn rock_triangles(
    width: usize, depth: usize, relief: usize, crown: usize,
    mut triangle: impl FnMut([[f32; 3]; 3], [u8; 3]),
) {
    let width = [0.82, 1.10, 1.38, 1.66][width];
    let depth = [0.74, 0.98, 1.24, 1.50][depth];
    let height = [0.46, 0.86, 1.26, 1.66][relief];
    let [peak_x, peak_z] = [[-0.55, 0.27], [-0.17, -0.51], [0.34, 0.45], [0.59, -0.18]][crown];
    let corners = [
        [-width, 0.0, -0.56 * depth], [0.82 * width, 0.0, -depth],
        [width, 0.0, 0.76 * depth], [-0.66 * width, 0.0, depth],
    ];
    let peak = [peak_x * width, height, peak_z * depth];
    for (i, color) in [[151, 124, 100], [114, 98, 87], [129, 101, 80], [173, 145, 118]].into_iter().enumerate() {
        triangle([corners[i], corners[(i + 1) % 4], peak], color);
    }
    let toe = [width + 1.08, 0.0, -0.54];
    let crest = [width + 0.34, 0.22 + height * 0.39, peak_z * 0.66];
    triangle([[width * 0.62, 0.0, -0.45], toe, crest], [161, 131, 108]);
    triangle([toe, [width + 0.72, 0.0, 0.72], crest], [112, 91, 76]);
}

fn bush_triangles(
    width: usize, height: usize, spread: usize, crown: usize,
    mut triangle: impl FnMut([[f32; 3]; 3], [u8; 3]),
) {
    let width = [0.84, 1.11, 1.38, 1.65][width];
    let height = [0.65, 1.03, 1.41, 1.79][height];
    let spread = [0.48, 0.78, 1.08, 1.38][spread];
    let [lean, split] = [[-0.57, 0.55], [-0.21, 0.82], [0.21, 0.69], [0.55, 1.00]][crown];
    for swap in [false, true] {
        let at = |x, y| if swap { [0.0, y, x] } else { [x, y, 0.0] };
        triangle([at(-width, 0.0), at(0.39, 0.0), at(-width * 0.48 + lean, height)], [99, 104, 59]);
        triangle([at(-0.38, 0.0), at(width, 0.0), at(width * 0.51 + lean * 0.6, height * split)], [130, 126, 68]);
        triangle([at(width * 0.72, 0.0), at(width + spread, 0.0), at(width + spread * 0.46, height * 0.56)], [113, 118, 72]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::Sky;
    use std::collections::BTreeSet;

    #[test]
    fn foreground_props_follow_the_same_seed_and_respect_developed_ground() {
        let camera = Camera { x: -7.0, z: -11.0, altitude: 21.0, heading: 0.2, pitch: 0.45, bank: 0.0 };
        let placements = |seed, surface| {
            let sample = |_, _| Sample { surface, elevation: 4.0 };
            DesertProps::gather(camera, (72, 72), seed, sample, sample)
        };
        let original = placements(19, Surface::Desert);
        let repeated = placements(19, Surface::Desert);
        let different = placements(22, Surface::Desert);
        let points = |set: &DesertProps| set.props.map(|prop| prop.map(|p| (p.origin, p.kind, p.variant)));
        assert!(original.region().is_some());
        assert!(original.props.iter().flatten().count() >= 5);
        assert_eq!(points(&original), points(&repeated));
        assert_ne!(points(&original), points(&different));
        for surface in [Surface::Pavement, Surface::GrassRunway, Surface::RiverWater, Surface::CargoOchre] {
            assert!(placements(19, surface).region().is_none());
        }
    }

    #[test]
    fn world_seeds_use_every_mesh_variant_of_each_kind() {
        let camera = Camera { x: 17.0, z: -13.0, altitude: 20.0, heading: 0.48, pitch: 0.50, bank: 0.0 };
        let ground = |_, _| Sample { surface: Surface::Desert, elevation: 4.0 };
        let mut seen = [[false; 256]; 3];
        for seed in 0..512 {
            let scene = DesertProps::gather(camera, (60, 60), seed, ground, ground);
            for prop in scene.props.into_iter().flatten() {
                seen[prop.kind as usize][usize::from(prop.variant)] = true;
            }
        }
        assert!(seen.into_iter().flatten().all(|found| found));
    }

    #[test]
    fn desert_variants_have_distinct_rendered_silhouettes_and_fit_the_cull_bounds() {
        const SIDE: usize = 60;
        const ROWS: usize = (SIDE * SIDE).div_ceil(64);
        let cameras = [
            Camera { x: -3.5, z: -7.0, altitude: 3.8, heading: 0.464, pitch: 0.30, bank: 0.0 },
            Camera { x: 7.0, z: -3.5, altitude: 5.5, heading: -1.107, pitch: 0.48, bank: 0.0 },
        ];
        let sky = Sky::from_time(41, 0, 0.48);
        for kind in [PropKind::Cactus, PropKind::Rock, PropKind::Bush] {
            let mut forms = BTreeSet::new();
            for variant in 0..=u8::MAX {
                let prop = Prop { origin: [0.0; 3], size: 1.0, kind, variant, opacity: 1.0 };
                let mut silhouettes = [[0_u64; ROWS]; 2];
                for (view, camera) in cameras.into_iter().enumerate() {
                    let mut pixels = [0; SIDE * SIDE];
                    let mut depth = [u16::MAX; SIDE * SIDE];
                    let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
                    let mut count = 0;
                    let mut grounded = false;
                    prop.triangles(|points, _| {
                        for [x, y, z] in points {
                            assert!(x.abs() <= HALF_SPAN && z.abs() <= HALF_SPAN);
                            assert!((0.0..=HEIGHT).contains(&y));
                            grounded |= y == 0.0;
                        }
                        count += 1;
                        mesh.triangle(points, [255; 3]);
                    });
                    assert_eq!(count, 6);
                    assert!(grounded);
                    for (i, color) in pixels.into_iter().enumerate() {
                        silhouettes[view][i / 64] |= u64::from(color != 0) << (i % 64);
                    }
                    assert!(silhouettes[view].iter().any(|row| *row != 0));
                }
                forms.insert(silhouettes);
            }
            assert_eq!(forms.len(), 256, "{kind:?}");
        }
    }
}

pub struct DesertProps {
    props: [Option<Prop>; 72],
    region: Option<ScreenRect>,
}

impl DesertProps {
    #[inline(never)]
    pub fn gather(
        camera: Camera, dimensions: (usize, usize), seed: u64,
        terrain: impl Fn(f32, f32) -> Sample, sample: impl Fn(f32, f32) -> Sample,
    ) -> Self {
        let mut result = Self { props: [None; 72], region: None };
        let (sin, cos) = sin_cos(camera.heading);
        let Some(projection) = ScreenProjection::new(camera, dimensions) else {
            return result;
        };
        let cx = floor_i32(camera.x / CELL);
        let cz = floor_i32(camera.z / CELL);
        let mut count = 0;
        for tz in cz - 6..=cz + 6 {
            for tx in cx - 6..=cx + 6 {
                let id = (u64::from(tx as u32) << 32) | u64::from(tz as u32);
                let key = crate::math::mix(seed ^ id ^ 0x98c4_c6a8_51ea_759f);
                let x = (tx as f32 + 0.17 + (key & 63) as f32 * 0.010) * CELL;
                let z = (tz as f32 + 0.17 + ((key >> 6) & 63) as f32 * 0.010) * CELL;
                let [dx, dz] = [x - camera.x, z - camera.z];
                let distance = dx * dx + dz * dz;
                let forward = sin * dx + cos * dz;
                if distance > REACH * REACH || forward < -12.0 {
                    continue;
                }
                if !matches!(terrain(x, z).surface, Surface::Desert | Surface::DuneSand | Surface::Sand) { continue; }
                let ground = sample(x, z);
                if !matches!(ground.surface, Surface::Desert | Surface::DuneSand) { continue; }
                let opacity = ((REACH * REACH - distance) / 1500.0).clamp(0.0, 1.0)
                    * ((66.0 - camera.altitude + ground.elevation) / 18.0).clamp(0.0, 1.0)
                    * ((forward + 12.0) / 6.0).clamp(0.0, 1.0);
                if opacity < 0.02 { continue; }
                let size = 1.12 + ((key >> 16) & 7) as f32 * 0.12;
                let y = ground.elevation - 0.14;
                let bounds = [[x - HALF_SPAN * size, y, z - HALF_SPAN * size],
                              [x + HALF_SPAN * size, y + HEIGHT * size, z + HALF_SPAN * size]];
                let Some(rect) = projection.bounds(bounds) else { continue; };
                result.region = Some(result.region.map_or(rect, |old| old.union(rect)));
                let district = (u64::from((tx >> 2) as u32) << 32) | u64::from((tz >> 2) as u32);
                let district = crate::math::mix(seed ^ district ^ 0x6863_583d_6157_6cbc);
                let roll = (key >> 12) & 15;
                let kind = if ground.surface == Surface::DuneSand {
                    if roll < 4 { PropKind::Rock } else { PropKind::Bush }
                } else {
                    let cactus = [1, 2, 5, 6][(district & 3) as usize];
                    let rock = [12, 9, 11, 8][(district & 3) as usize];
                    if roll < cactus {
                        PropKind::Cactus
                    } else if roll < rock {
                        PropKind::Rock
                    } else {
                        PropKind::Bush
                    }
                };
                result.props[count] = Some(Prop {
                    origin: [x, y, z], size, kind, variant: (key >> 19) as u8, opacity,
                });
                count += 1;
                if count == result.props.len() { return result; }
            }
        }
        result
    }

    pub fn region(&self) -> Option<ScreenRect> { self.region }

    pub fn draw(&self, mesh: &mut WorldMesh<'_>) {
        for prop in self.props.iter().flatten() {
            prop.triangles(|points, color| {
                mesh.faded_triangle(points.map(|point| {
                    core::array::from_fn(|i| prop.origin[i] + point[i] * prop.size)
                }), color, prop.opacity);
            });
        }
    }
}
