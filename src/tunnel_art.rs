//! Road cuts, retaining rings, and visible bore interiors.

use crate::math::sin_cos;
use crate::render::{Camera, view_distance};
use crate::tunnels::{RoadTunnel, TUNNEL_HALF_WIDTH, TunnelMouth, TunnelPoint};
use crate::world::{SEA_LEVEL, Sample, Surface};
use crate::world_mesh::WorldMesh;

const NEAR: f32 = 0.8;
const DEPTH_STEP: f32 = 1.0 / 32.0;
const INTERIOR_AMBIENT: u16 = (2 << 11) | (4 << 5) | 2;
type Point = [f32; 3];

/// Lower the surveyed approach to its road grade. The original pavement and
/// ground rejoin at the planned junctions; the roof remains untouched.
pub fn overlay_approach(tunnel: &RoadTunnel, x: f32, z: f32, ground: Sample) -> Sample {
    if ground.elevation <= SEA_LEVEL || ground.surface.is_industry()
        || matches!(
            ground.surface,
            Surface::DeepWater
                | Surface::Shallows
                | Surface::RiverWater
                | Surface::TarnWater
                | Surface::TarnShallows
                | Surface::Pavement
                | Surface::FlatRoof
                | Surface::PitchedRoof
                | Surface::TowerRoof
                | Surface::RailTimber
                | Surface::RailMetal
                | Surface::RailGlass
                | Surface::RailRust
                | Surface::RailApron
                | Surface::HarborCrane
                | Surface::ShipHull
                | Surface::ShipDeck
                | Surface::ShipCabin
                | Surface::ShipCargo
                | Surface::CargoOchre
                | Surface::CargoBlue
                | Surface::RunwayMarking
                | Surface::RunwayLight
                | Surface::GrassRunway
        )
    {
        return ground;
    }
    let Some(approach) = tunnel.approach_at(x, z) else {
        return ground;
    };
    if approach.blend <= 0.000_001 {
        return ground;
    }
    let surface = if approach.offset <= TUNNEL_HALF_WIDTH - 0.26 {
        Surface::Pavement
    } else {
        Surface::Gravel
    };
    Sample {
        elevation: ground.elevation + (approach.elevation - ground.elevation) * approach.blend,
        surface,
    }
}

/// Open surveyed portals after rendering terrain with `overlay_approach`.
/// Only rays through the actual arch can change. The fallback color is the
/// unlit bore; the lining and road are depth-tested geometry drawn by
/// `draw_tunnel`.
///
/// The input depth is the terrain-only camera-space Z buffer, in 1/32-unit
/// increments. A heightfield span repeats its source-column Z across several
/// pixels. Allow a short false foreground hit only inside the fully graded
/// road cut and strictly above its floor. For exact coverage at grazing
/// angles, the terrain rasterizer must retain the source column's x/z, test
/// its vertical interval against `RoadTunnel::void_at`, then keep only the
/// portions outside the bore.
pub fn open_portals(
    tunnel: &RoadTunnel,
    camera: Camera,
    pixels: &mut [u16],
    depth: &mut [u16],
    dimensions: (usize, usize),
) -> usize {
    let (width, height) = dimensions;
    if width == 0
        || height == 0
        || width > 512
        || height > 512
        || pixels.len() < width * height
        || depth.len() < width * height
    {
        return 0;
    }
    let view = BoreView::new(camera, width, height);
    let mut cleared = 0;
    for mouth in tunnel.mouths() {
        let eye_side = along(mouth, view.eye);
        if eye_side > -0.45 {
            continue;
        }
        let Some([left, top, right, bottom]) = view.window(mouth) else {
            continue;
        };
        let diameter = width.min(height) as i32;
        for y in top..bottom {
            let dy = y as i32 * 2 + 1 - height as i32;
            for x in left..right {
                let dx = x as i32 * 2 + 1 - width as i32;
                if dx * dx + dy * dy > diameter * diameter {
                    continue;
                }
                let index = y * width + x;
                let z = f32::from(depth[index]) * DEPTH_STEP;
                if depth[index] == u16::MAX || z < NEAR {
                    continue;
                }
                let ray = view.ray(x, y);
                let forward = ray[0] * mouth.inward_x + ray[2] * mouth.inward_z;
                if forward <= 0.001 {
                    continue;
                }
                let entry_depth = -eye_side / forward;
                if entry_depth < NEAR || !in_bore(tunnel, view.at(ray, entry_depth)) {
                    continue;
                }
                let shortfall = entry_depth - z;
                let hit = view.at(ray, z);
                let remove = if shortfall <= 0.055 {
                    in_bore(
                        tunnel,
                        if shortfall > 0.0 {
                            view.at(ray, entry_depth)
                        } else {
                            hit
                        },
                    )
                } else {
                    shortfall <= 2.4
                        && tunnel
                            .approach_at(hit[0], hit[2])
                            .is_some_and(|cut| cut.blend >= 0.999 && hit[1] > cut.elevation + 0.075)
                };
                if !remove {
                    continue;
                }
                depth[index] = u16::MAX;
                pixels[index] = INTERIOR_AMBIENT;
                cleared += 1;
            }
        }
    }
    cleared
}

/// Draw the original masonry rings and both lit approaches to the actual bore.
/// Call after `open_portals` when the terrain depth buffer is available.
pub fn draw_tunnel(tunnel: &RoadTunnel, mesh: &mut WorldMesh<'_>) {
    let [first, last] = tunnel.mouths();
    for mouth in [first, last] {
        for (index, panel) in mouth.frame().into_iter().enumerate() {
            let color = if index == 4 || index == 5 {
                [184, 175, 153]
            } else {
                [154, 147, 133]
            };
            quad(mesh, panel.map(world_point), color, true);
        }
        draw_wings(tunnel, mesh, mouth);
    }

    let length = tunnel.covered_length();
    let grade = (last.center.y - first.center.y) / length;
    let mut near = first.opening().map(world_point);
    for (index, distance) in [2.85, 8.20, length - 8.20, length - 2.85, length]
        .into_iter()
        .enumerate()
    {
        let far = first
            .opening()
            .map(|point| shifted(point, first, grade, distance));
        let (wall, roof, floor) = match index {
            0 | 4 => ([142, 135, 119], [108, 101, 92], [76, 78, 77]),
            1 | 3 => ([84, 79, 70], [58, 56, 57], [54, 55, 57]),
            _ => ([32, 34, 37], [22, 24, 27], [33, 35, 38]),
        };
        for panel in 0..10 {
            let color = if panel == 0 || panel == 9 { wall } else { roof };
            quad(
                mesh,
                [near[panel], near[panel + 1], far[panel + 1], far[panel]],
                color,
                false,
            );
        }
        quad(mesh, [near[0], far[0], far[10], near[10]], floor, false);
        near = far;
    }
    draw_lamps(mesh, first, grade);
    draw_lamps(mesh, last, -grade);
}

fn draw_wings(tunnel: &RoadTunnel, mesh: &mut WorldMesh<'_>, mouth: TunnelMouth) {
    let opening = mouth.opening();
    let frame = mouth.frame();
    for (side, edge, top) in [
        (-1.0, opening[0], frame[0][3]),
        (1.0, opening[10], frame[9][0]),
    ] {
        let mut tail = None;
        for distance in [8.0, 6.0, 4.0] {
            let sample = lateral(mouth, side * (TUNNEL_HALF_WIDTH - 0.10), -distance);
            if let Some(cut) = tunnel.approach_at(sample[0], sample[1]) {
                tail = Some((distance, cut.elevation));
                break;
            }
        }
        let Some((distance, elevation)) = tail else {
            continue;
        };
        let [x, z] = lateral(mouth, side * TUNNEL_HALF_WIDTH, -distance);
        let [dx, dz] = lateral(mouth, side * (TUNNEL_HALF_WIDTH + 0.53), -distance);
        quad(
            mesh,
            [
                world_point(edge),
                [x, elevation, z],
                [dx, elevation + 0.19, dz],
                world_point(top),
            ],
            [142, 132, 116],
            true,
        );
    }
}

fn draw_lamps(mesh: &mut WorldMesh<'_>, mouth: TunnelMouth, grade: f32) {
    for side in [-1.0, 1.0] {
        let width = side * (TUNNEL_HALF_WIDTH - 0.055);
        let point = |distance, height| {
            let [x, z] = lateral(mouth, width, distance);
            [x, mouth.center.y + grade * distance + height, z]
        };
        quad(
            mesh,
            [
                point(1.70, 2.46),
                point(2.29, 2.46),
                point(2.29, 2.72),
                point(1.70, 2.72),
            ],
            [213, 169, 98],
            false,
        );
    }
}

fn lateral(mouth: TunnelMouth, side: f32, distance: f32) -> [f32; 2] {
    [
        mouth.center.x + mouth.inward_z * side + mouth.inward_x * distance,
        mouth.center.z - mouth.inward_x * side + mouth.inward_z * distance,
    ]
}

fn shifted(point: TunnelPoint, mouth: TunnelMouth, grade: f32, distance: f32) -> Point {
    [
        point.x + mouth.inward_x * distance,
        point.y + grade * distance,
        point.z + mouth.inward_z * distance,
    ]
}

fn world_point(point: TunnelPoint) -> Point {
    [point.x, point.y, point.z]
}

fn along(mouth: TunnelMouth, point: Point) -> f32 {
    (point[0] - mouth.center.x) * mouth.inward_x + (point[2] - mouth.center.z) * mouth.inward_z
}

fn in_bore(tunnel: &RoadTunnel, point: Point) -> bool {
    tunnel
        .void_at(point[0], point[2])
        .is_some_and(|inside| point[1] > inside.floor + 0.035 && point[1] < inside.ceiling - 0.025)
}

fn quad(mesh: &mut WorldMesh<'_>, [a, b, c, d]: [Point; 4], color: [u8; 3], lit: bool) {
    if lit {
        mesh.lit_triangle([a, b, c], color);
        mesh.lit_triangle([a, c, d], color);
    } else {
        mesh.triangle([a, b, c], color);
        mesh.triangle([a, c, d], color);
    }
}

struct BoreView {
    eye: Point,
    heading: (f32, f32),
    pitch: (f32, f32),
    roll: (f32, f32),
    focal: f32,
    inverse_focal: f32,
    center: [f32; 2],
    width: usize,
    height: usize,
    curvature: f32,
}

impl BoreView {
    fn new(camera: Camera, width: usize, height: usize) -> Self {
        let focal = width.min(height) as f32 * 0.73;
        Self {
            eye: [camera.x, camera.altitude, camera.z],
            heading: sin_cos(camera.heading),
            pitch: sin_cos(camera.pitch.clamp(-1.3, 1.3)),
            roll: sin_cos(-camera.bank),
            focal,
            inverse_focal: 1.0 / focal,
            center: [(width - 1) as f32 * 0.5, (height - 1) as f32 * 0.5],
            width,
            height,
            curvature: (view_distance(camera.altitude) - 512.0) * (0.000040 / 384.0),
        }
    }

    fn project(&self, point: Point) -> Option<[f32; 2]> {
        let [x, y, z] = [
            point[0] - self.eye[0],
            point[1] - self.eye[1],
            point[2] - self.eye[2],
        ];
        let height = y - self.curvature * (x * x + z * z);
        let (hs, hc) = self.heading;
        let (ps, pc) = self.pitch;
        let (rs, rc) = self.roll;
        let forward = hs * x + hc * z;
        let depth = pc * forward - ps * height;
        if depth < NEAR || !depth.is_finite() {
            return None;
        }
        let u = hc * x - hs * z;
        let v = -ps * forward - pc * height;
        let scale = self.focal / depth;
        Some([
            self.center[0] + (u * rc - v * rs) * scale,
            self.center[1] + (u * rs + v * rc) * scale,
        ])
    }

    fn window(&self, mouth: TunnelMouth) -> Option<[usize; 4]> {
        let mut bounds = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        let mut intersects_near = false;
        for point in mouth.opening() {
            if let Some([x, y]) = self.project(world_point(point)) {
                bounds = [
                    bounds[0].min(x),
                    bounds[1].min(y),
                    bounds[2].max(x),
                    bounds[3].max(y),
                ];
            } else {
                intersects_near = true;
            }
        }
        let [left, top, right, bottom] = bounds;
        if !left.is_finite() {
            return None;
        }
        if intersects_near {
            return Some([0, 0, self.width, self.height]);
        }
        let left = (left as usize).min(self.width);
        let top = (top as usize).min(self.height);
        let right = (right as usize).saturating_add(1).min(self.width);
        let bottom = (bottom as usize).saturating_add(1).min(self.height);
        (left < right && top < bottom).then_some([left, top, right, bottom])
    }

    fn ray(&self, x: usize, y: usize) -> Point {
        let (rs, rc) = self.roll;
        let [xx, yy] = [x as f32 - self.center[0], y as f32 - self.center[1]];
        let u = (xx * rc + yy * rs) * self.inverse_focal;
        let v = (-xx * rs + yy * rc) * self.inverse_focal;
        let (ps, pc) = self.pitch;
        let (hs, hc) = self.heading;
        let forward = pc - ps * v;
        [hc * u + hs * forward, -ps - pc * v, hc * forward - hs * u]
    }

    fn at(&self, ray: Point, depth: f32) -> Point {
        let dx = ray[0] * depth;
        let dz = ray[2] * depth;
        [
            self.eye[0] + dx,
            self.eye[1] + ray[1] * depth + self.curvature * (dx * dx + dz * dz),
            self.eye[2] + dz,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{encoded_depth, render_with_depth};
    use crate::roads::RoadNetwork;
    use crate::scenery::DistrictRegion;
    use crate::sky::Sky;
    use crate::world::World;

    #[test]
    fn the_new_grade_joins_the_existing_road_and_leaves_the_roof() {
        let world = World::new(8);
        let road = RoadNetwork::generate(&world, 0, 0).unwrap();
        let tunnel = RoadTunnel::generate(&world, 8, &road).unwrap();
        let overlay = |x, z| {
            let ground = world.sample(x, z);
            (ground, overlay_approach(&tunnel, x, z, ground))
        };
        for junction in tunnel.junctions() {
            let (before, after) = overlay(junction.x, junction.z);
            assert_eq!(before, after);
        }
        let district = DistrictRegion::build(&world, 0, 0);
        let mut bypass_points = 0;
        for edge in road.polyline().windows(2) {
            for step in 0..=8 {
                let fraction = step as f32 * 0.125;
                let x = edge[0].x + (edge[1].x - edge[0].x) * fraction;
                let z = edge[0].z + (edge[1].z - edge[0].z) * fraction;
                let before = district.overlay(x, z, world.sample(x, z));
                if before.surface == Surface::Pavement {
                    assert_eq!(overlay_approach(&tunnel, x, z, before), before);
                    bypass_points += 1;
                }
            }
        }
        assert!(bypass_points > 100);
        for mouth in tunnel.mouths() {
            let [x, z] = lateral(mouth, 0.0, -0.6);
            let (_, after) = overlay(x, z);
            let cut = tunnel.approach_at(x, z).unwrap();
            assert_eq!(after.surface, Surface::Pavement);
            assert!((after.elevation - cut.elevation).abs() < 0.000_1);
            let protected_roof = Sample {
                elevation: 40.0,
                surface: Surface::FlatRoof,
            };
            assert_eq!(
                overlay_approach(&tunnel, x, z, protected_roof),
                protected_roof
            );

            let [x, z] = lateral(mouth, 0.0, 5.0);
            let bore = tunnel.void_at(x, z).unwrap();
            let (before, after) = overlay(x, z);
            assert_eq!(before, after);
            assert!(after.elevation > bore.ceiling);
        }
    }

    #[test]
    fn the_lined_opening_clears_only_inside_rays_and_keeps_earlier_foreground() {
        const WIDTH: usize = 112;
        let dimensions = (WIDTH, WIDTH);
        let world = World::new(8);
        let district = DistrictRegion::build(&world, 0, 0);
        let road = RoadNetwork::generate(&world, 0, 0).unwrap();
        let tunnel = RoadTunnel::generate(&world, 8, &road).unwrap();
        let mouth = tunnel.mouths()[0];
        let [x, z] = lateral(mouth, 1.0, -11.5);
        let camera = Camera {
            x,
            z,
            altitude: mouth.center.y + 3.0,
            heading: (mouth.center.x - x).atan2(mouth.center.z - z),
            pitch: (0.9f32 / 11.5).atan(),
            bank: 0.01,
        };
        let sky = Sky::from_time(8, 0, 0.43);
        let mut original = [0; WIDTH * WIDTH];
        let mut scratch = original;
        let mut terrain_depth = original;
        render_with_depth(
            |x, z| overlay_approach(&tunnel, x, z, district.overlay(x, z, world.sample(x, z))),
            camera,
            &sky,
            &mut original,
            &mut scratch,
            &mut terrain_depth,
            dimensions,
        )
        .unwrap();

        let mut pixels = original;
        let mut depth = terrain_depth;
        assert!(open_portals(&tunnel, camera, &mut pixels, &mut depth, dimensions) > 100);
        let view = BoreView::new(camera, WIDTH, WIDTH);
        let mut sample = None;
        for index in 0..pixels.len() {
            if pixels[index] != original[index] {
                let ray = view.ray(index % WIDTH, index / WIDTH);
                let forward = ray[0] * mouth.inward_x + ray[2] * mouth.inward_z;
                let entry_depth = -along(mouth, view.eye) / forward;
                assert!(in_bore(&tunnel, view.at(ray, entry_depth)));
                assert_eq!(depth[index], u16::MAX);
                sample = Some((index, entry_depth));
            }
        }
        let (index, entry_depth) = sample.unwrap();
        let earlier = encoded_depth(entry_depth - 3.0);
        terrain_depth[index] = earlier;
        let mut blocked = original;
        open_portals(
            &tunnel,
            camera,
            &mut blocked,
            &mut terrain_depth,
            dimensions,
        );
        assert_eq!(terrain_depth[index], earlier);
        assert_eq!(blocked[index], original[index]);

        let unlined = pixels;
        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, dimensions, camera, &sky).unwrap();
        draw_tunnel(&tunnel, &mut mesh);
        assert!(
            pixels
                .iter()
                .zip(unlined)
                .filter(|(after, before)| **after != *before)
                .count()
                > 100
        );
    }
}
