//! A depth-tested asphalt ribbon for the surveyed highway links.

use crate::highway_links::HighwayLink;
use crate::math::sin_cos;
use crate::render::{view_distance, Camera, ScreenProjection, ScreenRect};
use crate::roads::{RoadPoint, ROAD_HALF_WIDTH};
use crate::world::{Sample, Surface, SEA_LEVEL};
use crate::world_mesh::WorldMesh;

const HALF_WIDTH: f32 = ROAD_HALF_WIDTH - 0.24;
const DRAPE_LIMIT: f32 = 1.28;
const SURFACE_BIAS: f32 = 0.10;
const ASPHALT: [u8; 3] = [56, 63, 72];
const GENTLE_STEP_CM: u32 = 600;
const STEEP_STEP_CM: u32 = 420;
const NEAR: f32 = 0.8;

/// Frame-local, cheap coverage plan. It borrows the checked link and stores
/// only a screen rectangle and the chords that can actually contribute.
pub struct HighwayRibbon<'a> {
    points: &'a [RoadPoint],
    bridge_cap: bool,
    eye: [f32; 3],
    region: ScreenRect,
    visible: [u32; 5],
    reach_squared: f32,
    inverse_fade: f32,
}

impl<'a> HighwayRibbon<'a> {
    pub fn new(road: &'a HighwayLink, camera: Camera, dimensions: (usize, usize)) -> Option<Self> {
        Self::project(road.polyline(), road.bounds(), false, camera, dimensions)
    }

    /// The path is a certified dry approach, ordered from the existing road
    /// endpoint to its raised bridge ramp. Only the last section uses the
    /// ramp's exact grade; the rest stays draped onto the displayed surface.
    pub fn for_approach(
        points: &'a [RoadPoint],
        camera: Camera,
        dimensions: (usize, usize),
    ) -> Option<Self> {
        let first = points.first()?;
        let [left, near, right, far] = points
            .iter()
            .fold([first.x, first.z, first.x, first.z], |b, p| {
                [b[0].min(p.x), b[1].min(p.z), b[2].max(p.x), b[3].max(p.z)]
            });
        Self::project(
            points,
            [
                left - HALF_WIDTH,
                near - HALF_WIDTH,
                right + HALF_WIDTH,
                far + HALF_WIDTH,
            ],
            true,
            camera,
            dimensions,
        )
    }

    fn project(
        points: &'a [RoadPoint],
        bounds: [f32; 4],
        bridge_cap: bool,
        camera: Camera,
        dimensions: (usize, usize),
    ) -> Option<Self> {
        let (width, height) = dimensions;
        if width == 0
            || height == 0
            || width > 512
            || height > 512
            || !(2..=136).contains(&points.len())
        {
            return None;
        }
        let eye = [camera.x, camera.altitude, camera.z];
        let focal = width.min(height) as f32 * 0.73;
        // The supplemental pavement is only useful while it spans pixels.
        let reach = focal * 2.7;
        let fade_from = reach * 0.79;
        let reach_squared = reach * reach;
        let inverse_fade = 1.0 / (reach_squared - fade_from * fade_from);
        let [left, near, right, far] = bounds;
        let dx = camera.x - camera.x.clamp(left, right);
        let dz = camera.z - camera.z.clamp(near, far);
        if dx * dx + dz * dz >= reach_squared {
            return None;
        }

        let mut visible = [0; 5];
        let mut region = None;
        let projection = ScreenProjection::new(camera, dimensions)?;
        let mut near_projection = None;
        for (index, edge) in points.windows(2).enumerate() {
            let [a, b] = [edge[0], edge[1]];
            let envelope = [
                [
                    a.x.min(b.x) - HALF_WIDTH,
                    a.elevation.min(b.elevation) - DRAPE_LIMIT,
                    a.z.min(b.z) - HALF_WIDTH,
                ],
                [
                    a.x.max(b.x) + HALF_WIDTH,
                    a.elevation.max(b.elevation) + DRAPE_LIMIT + SURFACE_BIAS,
                    a.z.max(b.z) + HALF_WIDTH,
                ],
            ];
            if squared_distance(eye, envelope) >= reach_squared {
                continue;
            }
            let Some(mut screen) = projection.bounds(envelope) else {
                continue;
            };
            if screen.right == width
                && screen.bottom == height
                && screen.left == 0
                && screen.top == 0
            {
                let clip =
                    near_projection.get_or_insert_with(|| NearProjection::new(camera, dimensions));
                let Some(tighter) = clip.bounds(envelope) else {
                    continue;
                };
                screen = tighter;
            }
            region = Some(region.map_or(screen, |before: ScreenRect| before.union(screen)));
            visible[index / 32] |= 1 << (index % 32);
        }
        Some(Self {
            points,
            bridge_cap,
            eye,
            region: region?,
            visible,
            reach_squared,
            inverse_fade,
        })
    }

    pub fn depth_region(&self) -> ScreenRect {
        self.region
    }

    /// Supply the same geometry sampler used by terrain, so the drape is on
    /// the displayed ground and an intervening water/structure keeps priority.
    pub fn draw(&self, mesh: &mut WorldMesh<'_>, mut sample: impl FnMut(f32, f32) -> Sample) {
        let points = self.points;
        let mut previous = None;
        for (index, edge) in points.windows(2).enumerate() {
            if self.visible[index / 32] & (1 << (index % 32)) == 0 {
                previous = None;
                continue;
            }
            let [a, b] = [edge[0], edge[1]];
            let normal = normal_to(a, b);
            let start = if index > 0 {
                join(normal_to(points[index - 1], a), normal)
            } else {
                normal
            };
            let end = points
                .get(index + 2)
                .map_or(normal, |&next| join(normal, normal_to(b, next)));
            let dx = b.x - a.x;
            let dy = b.elevation - a.elevation;
            let dz = b.z - a.z;
            let cross_section = |t: f32| -> [f32; 3] {
                if t == 1.0 {
                    [b.x, b.elevation, b.z]
                } else {
                    [a.x + dx * t, a.elevation + dy * t, a.z + dz * t]
                }
            };
            let length = root(dx * dx + dz * dz);
            // A fixed chord grade selects tessellation once in world space.
            // Only the opacity varies as the aircraft moves away.
            let steep = u32::from(dy.abs() > length * 0.06);
            let step_cm = GENTLE_STEP_CM - steep * (GENTLE_STEP_CM - STEEP_STEP_CM);
            let intervals = ((length * 100.0) as u32 / step_cm + 1) as usize;
            let inverse = 1.0 / intervals as f32;
            let mut last =
                previous.or_else(|| drape(cross_section(0.0), start, false, &mut sample));
            for i in 1..=intervals {
                let t = if i == intervals {
                    1.0
                } else {
                    i as f32 * inverse
                };
                let center = cross_section(t);
                let side = if i == intervals {
                    end
                } else {
                    [
                        start[0] + (end[0] - start[0]) * t,
                        start[1] + (end[1] - start[1]) * t,
                    ]
                };
                let bridge_cap = self.bridge_cap && i == intervals && index + 2 == points.len();
                let next = drape(center, side, bridge_cap, &mut sample);
                let middle = cross_section((i as f32 - 0.5) * inverse);
                let t = ((self.reach_squared - distance_to(middle, self.eye)) * self.inverse_fade)
                    .clamp(0.0, 1.0);
                let opacity = t * t * (3.0 - 2.0 * t);
                if let (Some([a, b]), Some([c, d])) = (last, next) {
                    if opacity > 0.015625 {
                        mesh.faded_triangle([a, b, d], ASPHALT, opacity);
                        mesh.faded_triangle([a, d, c], ASPHALT, opacity);
                    }
                }
                last = next;
            }
            previous = last;
        }
    }
}

// The shared projection normally suffices. Its near-plane fallback is the
// full frame, which is disproportionately costly for a short ground ribbon.
// Clip the conservative box in camera space only when that fallback occurs.
struct NearProjection {
    eye: [f32; 3],
    heading: (f32, f32),
    pitch: (f32, f32),
    roll: (f32, f32),
    dimensions: (usize, usize),
    curvature: f32,
}

impl NearProjection {
    fn new(camera: Camera, dimensions: (usize, usize)) -> Self {
        Self {
            eye: [camera.x, camera.altitude, camera.z],
            heading: sin_cos(camera.heading),
            pitch: sin_cos(camera.pitch.clamp(-1.3, 1.3)),
            roll: sin_cos(-camera.bank),
            dimensions,
            curvature: (view_distance(camera.altitude) - 512.0) * (0.000040 / 384.0),
        }
    }

    fn bounds(&self, [min, max]: [[f32; 3]; 2]) -> Option<ScreenRect> {
        let x = [min[0] - self.eye[0], max[0] - self.eye[0]];
        let z = [min[2] - self.eye[2], max[2] - self.eye[2]];
        let square = |v: f32| v * v;
        let max_radius = square(x[0].abs().max(x[1].abs())) + square(z[0].abs().max(z[1].abs()));
        let min_radius = square(0.0_f32.clamp(x[0], x[1])) + square(0.0_f32.clamp(z[0], z[1]));
        // WorldMesh applies curvature per vertex; widen the vertical range
        // before the linear clip so all actual projected vertices stay inside.
        let y = [
            min[1] - self.eye[1] - self.curvature * max_radius,
            max[1] - self.eye[1] - self.curvature * min_radius,
        ];
        let (hs, hc) = self.heading;
        let (ps, pc) = self.pitch;
        let (rs, rc) = self.roll;
        let mut points = [[0.0; 3]; 8];
        for (i, p) in points.iter_mut().enumerate() {
            let (x, y, z) = (x[i & 1], y[(i >> 1) & 1], z[i >> 2]);
            let forward = hs * x + hc * z;
            let right = hc * x - hs * z;
            let down = -ps * forward - pc * y;
            *p = [
                right * rc - down * rs,
                right * rs + down * rc,
                pc * forward - ps * y,
            ];
        }
        let (width, height) = self.dimensions;
        let focal = width.min(height) as f32 * 0.73;
        let center = [(width - 1) as f32 * 0.5, (height - 1) as f32 * 0.5];
        let mut bounds = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        let mut include = |[x, y, depth]: [f32; 3]| {
            let scale = focal * (1.0 / depth);
            let px = center[0] + x * scale;
            let py = center[1] + y * scale;
            bounds = [
                bounds[0].min(px),
                bounds[1].min(py),
                bounds[2].max(px),
                bounds[3].max(py),
            ];
        };
        for (index, &point) in points.iter().enumerate() {
            if point[2] >= NEAR {
                include(point);
            }
            for axis in [1, 2, 4] {
                if index & axis != 0 {
                    continue;
                }
                let other = points[index | axis];
                if (point[2] >= NEAR) != (other[2] >= NEAR) {
                    let t = (NEAR - point[2]) / (other[2] - point[2]);
                    include([
                        point[0] + (other[0] - point[0]) * t,
                        point[1] + (other[1] - point[1]) * t,
                        NEAR,
                    ]);
                }
            }
        }
        let [left, top, right, bottom] = bounds;
        let rect = ScreenRect {
            left: ((left - 2.0) as usize).min(width),
            top: ((top - 2.0) as usize).min(height),
            right: ((right + 3.0) as usize).min(width),
            bottom: ((bottom + 3.0) as usize).min(height),
        };
        (rect.left < rect.right && rect.top < rect.bottom).then_some(rect)
    }
}

fn drape(
    center: [f32; 3],
    normal: [f32; 2],
    bridge_cap: bool,
    sample: &mut impl FnMut(f32, f32) -> Sample,
) -> Option<[[f32; 3]; 2]> {
    let mut edge = [[0.0; 3]; 2];
    for (side, point) in [-HALF_WIDTH, HALF_WIDTH].into_iter().zip(&mut edge) {
        let x = center[0] + normal[0] * side;
        let z = center[2] + normal[1] * side;
        let ground = sample(x, z);
        if ground.surface != Surface::Pavement
            || ground.elevation <= SEA_LEVEL
            || (ground.elevation - center[1]).abs() > DRAPE_LIMIT
            || !ground.elevation.is_finite()
        {
            return None;
        }
        let elevation = if bridge_cap {
            center[1] + 0.015
        } else {
            ground.elevation + SURFACE_BIAS
        };
        *point = [x, elevation, z];
    }
    Some(edge)
}

fn normal_to(a: RoadPoint, b: RoadPoint) -> [f32; 2] {
    let (x, z) = (b.x - a.x, b.z - a.z);
    let inverse = 1.0 / root(x * x + z * z).max(0.0001);
    [-z * inverse, x * inverse]
}

fn join(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    let x = a[0] + b[0];
    let z = a[1] + b[1];
    let inverse = 1.0 / root(x * x + z * z).max(0.0001);
    [x * inverse, z * inverse]
}

fn root(value: f32) -> f32 {
    if value == 0.0 {
        return 0.0;
    }
    let estimate = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    let estimate = 0.5 * (estimate + value / estimate);
    0.5 * (estimate + value / estimate)
}

fn squared_distance(eye: [f32; 3], [min, max]: [[f32; 3]; 2]) -> f32 {
    let nearest = core::array::from_fn(|i| eye[i].clamp(min[i], max[i]));
    distance_to(nearest, eye)
}

fn distance_to(point: [f32; 3], eye: [f32; 3]) -> f32 {
    let [x, y, z] = core::array::from_fn(|i| point[i] - eye[i]);
    x * x + y * y + z * z
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facility_stream::FacilityStream;
    use crate::flight::{Aircraft, Flight};
    use crate::highway_stream::{highway_reservation, HighwayStream};
    use crate::land_stream::LandStream;
    use crate::local_routes::{route_reservation, LocalRouteStream};
    use crate::render::render_terrain_layer_region;
    use crate::river_stream::RiverStream;
    use crate::scenery::overlay_highway_link;
    use crate::sky::Sky;
    use crate::terrain_cache::TerrainCache;
    use crate::watershed::WatershedWork;
    use crate::world::World;

    #[test]
    fn ribbon_stays_inside_its_depth_region_and_yields_to_foreground() {
        const WIDTH: usize = 112;
        const PIXELS: usize = WIDTH * WIDTH;
        let dimensions = (WIDTH, WIDTH);
        let seed = 0xb98c_4fe9_3d37_483a;
        let world = World::new(seed);
        let view = Flight::start(&world, seed, Aircraft::Racer).view;
        let start = Camera {
            x: view.x,
            z: view.z,
            altitude: view.altitude,
            heading: view.heading,
            pitch: view.pitch,
            bank: view.bank,
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        land.preflight(start, false);
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        let mut rivers = RiverStream::<3>::new(world, seed);
        rivers.preflight(&land, &facilities, start, &mut WatershedWork::new());
        let mut routes = LocalRouteStream::<8>::new(world, seed);
        routes.preflight(&land, start, |key, route| {
            route_reservation(&world, &facilities, &rivers, key, route)
        });
        let mut highways = HighwayStream::<4>::new(world, seed);
        highways.preflight(
            &land,
            start,
            |key, link| highway_reservation(&land, &facilities, &rivers, &routes, key, link),
            || {},
        );
        let (_, link) = highways.links().next().unwrap();
        let mid = link.polyline()[link.polyline().len() / 2];
        let mut painted = 0;
        for camera in [
            Camera {
                x: 1201.3877,
                z: 625.8279,
                altitude: 57.33333,
                heading: -2.5849566,
                pitch: 0.15023792,
                bank: -0.20,
            },
            Camera {
                x: 1178.0105,
                z: 588.2924,
                altitude: 55.815044,
                heading: -2.5824776,
                pitch: 0.15321375,
                bank: 0.20,
            },
            Camera {
                x: mid.x - 1.2,
                z: mid.z + 0.8,
                altitude: mid.elevation + 6.0,
                heading: 0.3,
                pitch: 0.38,
                bank: 0.40,
            },
        ] {
            let mut cache = TerrainCache::new();
            cache.reset_for_altitude(world, camera.x, camera.z, camera.altitude);
            let sample = |x, z| overlay_highway_link(link, x, z, cache.sample(x, z));
            let sky = Sky::from_time(seed, 419, 0.40);
            let ribbon = HighwayRibbon::new(link, camera, dimensions).unwrap();
            let rect = ribbon.depth_region();
            let mut images = [[0; PIXELS]; 2];
            let mut depth = [0; PIXELS];
            let mut work = depth;
            for (image, region) in images.iter_mut().zip([rect, ScreenRect::full(dimensions)]) {
                render_terrain_layer_region(
                    (sample, |_, _, ground| ground.surface),
                    camera,
                    &sky,
                    image,
                    &mut work,
                    Some((&mut depth, region)),
                    dimensions,
                    |_| {},
                )
                .unwrap();
                let original = *image;
                ribbon.draw(
                    &mut WorldMesh::new(image, &mut depth, dimensions, camera, &sky).unwrap(),
                    sample,
                );
                painted += image.iter().zip(&original).filter(|(a, b)| a != b).count();
                for (i, (pixel, previous)) in image.iter().zip(&original).enumerate() {
                    if pixel == previous {
                        continue;
                    }
                    let (x, y) = (i % WIDTH, i / WIDTH);
                    assert!(x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom);
                }

                let mut blocked = original;
                let mut blocker = [0; PIXELS];
                ribbon.draw(
                    &mut WorldMesh::new(&mut blocked, &mut blocker, dimensions, camera, &sky)
                        .unwrap(),
                    sample,
                );
                assert_eq!(blocked, original);
                ribbon.draw(
                    &mut WorldMesh::new(&mut blocked, &mut depth, dimensions, camera, &sky)
                        .unwrap(),
                    |x, z| Sample {
                        surface: Surface::RiverWater,
                        ..sample(x, z)
                    },
                );
                assert_eq!(blocked, original);
            }
            assert_eq!(images[0], images[1]);
            let far = Camera {
                altitude: camera.altitude + 420.0,
                ..camera
            };
            assert!(HighwayRibbon::new(link, far, dimensions).is_none());
        }
        assert!(painted > 120);
    }
}
