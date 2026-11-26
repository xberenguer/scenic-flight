//! Graded railway terrain and depth-tested permanent way.

use crate::math::sin_cos;
use crate::rail::{RailLine, RailMaterial, RailPoint, RailSample};
use crate::rail_corridor::RailCorridor;
use crate::rail_station_art::{GrainElevatorArt, StationFixtures};
use crate::render::{view_distance, Camera, ScreenProjection, ScreenRect};
use crate::train_art::FreightRoute;
use crate::world::{Sample, Surface, SEA_LEVEL};
use crate::world_mesh::WorldMesh;

const HALF_GAUGE: f32 = 0.72;
const HALF_RAIL_WIDTH: f32 = 0.147;
const RAIL_TOP: f32 = 0.24;
const SLEEPER_TOP: f32 = 0.105;
const HALF_SLEEPER: f32 = 1.16;
const SLEEPER_PERIOD: f32 = 2.0;
const HALF_TIE_DEPTH: f32 = 0.40;
const MESH_NEAR: f32 = 0.8;
const RAIL_COLOR: [u8; 3] = [92, 106, 110];
const WOOD_COLOR: [u8; 3] = [106, 80, 62];
const BALLAST_COLOR: [u8; 3] = [146, 129, 108];

/// Keep the rail grade, access paths, platforms, and station roofs in the
/// regular terrain sampler. Close steel and timber detail is drawn separately.
pub fn overlay_rail(line: &RailLine, x: f32, z: f32, ground: Sample) -> Sample {
    line.sample(x, z)
        .map_or(ground, |piece| cover_rail(piece, ground))
}

pub(crate) fn cover_rail(piece: RailSample, ground: Sample) -> Sample {
    if ground.elevation <= SEA_LEVEL
        || ground.surface.is_station_roof()
        || matches!(
            ground.surface,
            Surface::RiverWater
                | Surface::DeepWater
                | Surface::Shallows
                | Surface::TarnWater
                | Surface::TarnShallows
                | Surface::PitchedRoof
                | Surface::FlatRoof
                | Surface::TowerRoof
                | Surface::ShipHull
                | Surface::ShipDeck
                | Surface::ShipCabin
                | Surface::ShipCargo
                | Surface::HarborCrane
                | Surface::CargoOchre
                | Surface::CargoBlue
                | Surface::GrassRunway
                | Surface::RunwayMarking
                | Surface::RunwayLight
        )
    {
        return ground;
    }
    let surface = match piece.material {
        RailMaterial::Ballast | RailMaterial::Sleepers | RailMaterial::Steel => Surface::Gravel,
        RailMaterial::StationPlatform => Surface::Pavement,
        RailMaterial::FreightPlatform => Surface::RailApron,
        RailMaterial::StationAccess => {
            if ground.surface == Surface::Pavement {
                Surface::Pavement
            } else {
                Surface::Gravel
            }
        }
        RailMaterial::StationTimber => Surface::RailTimber,
        RailMaterial::StationMetal => Surface::RailMetal,
        RailMaterial::StationGlass => Surface::RailGlass,
        RailMaterial::StationRust => Surface::RailRust,
    };
    Sample {
        surface,
        elevation: piece.elevation.unwrap_or(ground.elevation),
    }
}

/// Dress the corridor's surveyed cuts and fills without changing its grade.
/// The signed displacement gives continuous bands in the actual cross-section
/// and returns gradually to the native surface at the toe of the earthwork.
pub(crate) fn cover_rail_earthwork(elevation: f32, ground: Sample) -> Option<Sample> {
    use Surface::{Desert, Forest, Grass, Gravel, Rock, Sand, ScreeCoarse, ScreeEdge, Snow};
    if ground.elevation <= SEA_LEVEL
        || !matches!(ground.surface, Grass | Forest | Rock | Sand | Snow | Desert | Gravel)
    {
        return None;
    }
    let change = elevation - ground.elevation;
    let surface = if change <= -0.66 {
        Rock
    } else if change <= -0.33 {
        ScreeCoarse
    } else if change >= 0.57 {
        Gravel
    } else if change.abs() >= 0.22 && matches!(ground.surface, Grass | Forest) {
        ScreeEdge
    } else {
        ground.surface
    };
    Some(Sample { elevation, surface })
}

/// Add nearby rail heads and sleepers after the terrain and its depth buffer.
/// Use the same camera and dimensions supplied to the terrain renderer.
pub fn draw_rail(
    mesh: &mut WorldMesh<'_>,
    line: &RailLine,
    camera: Camera,
    dimensions: (usize, usize),
) {
    draw_tracks(mesh, line.bounds(), [line.polyline(), line.siding().map_or(&[], |points| &points[..])], camera, dimensions);
    draw_station_fixtures(
        mesh, line.stations().map(|station| station.fixtures(line.service)), line.freight_route,
        camera, dimensions,
    );
    if let Some(art) = line.grain_terminal().map(GrainElevatorArt::new) {
        if let Some((_, opacity)) = art.visible(camera, dimensions) {
            art.draw(mesh, opacity);
        }
    }
}

pub fn draw_rail_corridor(
    mesh: &mut WorldMesh<'_>,
    line: &RailCorridor,
    camera: Camera,
    dimensions: (usize, usize),
) {
    if let Some(view) = DetailView::for_camera(camera, dimensions) {
        corridor_bed(line.polyline(), &view, camera, |[a, b, c, d], opacity| {
            mesh.faded_triangle([a, b, c], BALLAST_COLOR, opacity);
            mesh.faded_triangle([a, c, d], BALLAST_COLOR, opacity);
        });
    }
    draw_tracks(mesh, line.bounds(), [line.polyline(), &[]], camera, dimensions);
    draw_station_fixtures(
        mesh, line.stations().map(|station| station.fixtures(line.service)), line.freight_route,
        camera, dimensions,
    );
}

fn draw_station_fixtures(
    mesh: &mut WorldMesh<'_>,
    fixtures: [StationFixtures; 2],
    cargo: FreightRoute,
    camera: Camera,
    dimensions: (usize, usize),
) {
    let Some(range_squared) = fixture_reach_squared(dimensions) else {
        return;
    };
    if let Some(projection) = ScreenProjection::new(camera, dimensions) {
        for fixture in fixtures {
            fixture.draw(mesh, camera, &projection, range_squared, cargo);
        }
    }
}

fn station_screen_region(
    fixtures: [StationFixtures; 2],
    camera: Camera,
    dimensions: (usize, usize),
) -> Option<ScreenRect> {
    let range_squared = fixture_reach_squared(dimensions)?;
    let projection = ScreenProjection::new(camera, dimensions)?;
    fixtures.iter()
        .filter_map(|fixture| fixture.region(camera, &projection, range_squared))
        .reduce(ScreenRect::union)
}

fn fixture_reach_squared((width, height): (usize, usize)) -> Option<f32> {
    if width == 0 || height == 0 || width > 512 || height > 512 {
        return None;
    }
    let reach = width.min(height) as f32 * 0.78;
    Some(reach * reach)
}

fn draw_tracks(
    mesh: &mut WorldMesh<'_>,
    bounds: [f32; 4],
    tracks: [&[RailPoint]; 2],
    camera: Camera,
    dimensions: (usize, usize),
) {
    let Some(view) = DetailView::new(bounds, camera, dimensions) else {
        return;
    };
    for track in tracks {
        track_geometry(track, &view, false, &mut |quad, color| mesh.faded_quad(quad, color, 1.0));
    }
}

/// Terrain-depth coverage for the rail and sleeper meshes drawn this frame.
/// Platforms, station roofs and freight aprons are part of terrain sampling.
pub fn rail_screen_region(
    line: &RailLine,
    camera: Camera,
    dimensions: (usize, usize),
) -> Option<ScreenRect> {
    tracks_screen_region(line.bounds(), [line.polyline(), line.siding().map_or(&[], |points| &points[..])], camera, dimensions)
        .into_iter()
        .chain(station_screen_region(
            line.stations().map(|station| station.fixtures(line.service)), camera, dimensions,
        ))
        .chain(line.grain_terminal().and_then(|terminal| {
            GrainElevatorArt::new(terminal).visible(camera, dimensions).map(|(region, _)| region)
        }))
        .reduce(ScreenRect::union)
}

pub fn corridor_screen_region(
    line: &RailCorridor,
    camera: Camera,
    dimensions: (usize, usize),
) -> Option<ScreenRect> {
    let view = DetailView::for_camera(camera, dimensions)?;
    let mut coverage = RailCoverage {
        dimensions,
        roll: sin_cos(-camera.bank),
        curvature: (view_distance(camera.altitude) - 512.0) * (0.000040 / 384.0),
        view: &view,
        region: tracks_screen_region(line.bounds(), [line.polyline(), &[]], camera, dimensions),
    };
    corridor_bed(line.polyline(), &view, camera, |quad, _| coverage.include(quad));
    coverage.region.into_iter()
        .chain(station_screen_region(
            line.stations().map(|station| station.fixtures(line.service)), camera, dimensions,
        ))
        .reduce(ScreenRect::union)
}

fn tracks_screen_region(
    bounds: [f32; 4],
    tracks: [&[RailPoint]; 2],
    camera: Camera,
    dimensions: (usize, usize),
) -> Option<ScreenRect> {
    let view = DetailView::new(bounds, camera, dimensions)?;
    let mut coverage = RailCoverage {
        dimensions,
        roll: sin_cos(-camera.bank),
        curvature: (view_distance(camera.altitude) - 512.0) * (0.000040 / 384.0),
        view: &view,
        region: None,
    };
    for track in tracks {
        let sleeper_envelope = coverage.curvature == 0.0;
        track_geometry(track, &view, sleeper_envelope, &mut |quad, _| coverage.include(quad));
    }
    coverage.region
}

type Face = [[f32; 3]; 4];

struct DetailView {
    eye: [f32; 3],
    heading: (f32, f32),
    pitch: (f32, f32),
    focal: f32,
    rails_reach: f32,
    sleeper_reach: f32,
}

impl DetailView {
    fn new([left, near, right, far]: [f32; 4], camera: Camera, dimensions: (usize, usize)) -> Option<Self> {
        let view = Self::for_camera(camera, dimensions)?;
        if camera.x < left - view.rails_reach
            || camera.x > right + view.rails_reach
            || camera.z < near - view.rails_reach
            || camera.z > far + view.rails_reach
        {
            return None;
        }
        Some(view)
    }

    fn for_camera(camera: Camera, (width, height): (usize, usize)) -> Option<Self> {
        if width == 0 || height == 0 || width > 512 || height > 512 {
            return None;
        }
        let focal = width.min(height) as f32 * 0.73;
        Some(Self {
            eye: [camera.x, camera.altitude, camera.z],
            heading: sin_cos(camera.heading),
            pitch: sin_cos(camera.pitch.clamp(-1.3, 1.3)),
            focal,
            rails_reach: focal * 0.94,
            sleeper_reach: focal * 0.49,
        })
    }

    fn camera_point(&self, point: [f32; 3], curvature: f32) -> [f32; 3] {
        let [x, y, z] = subtract(point, self.eye);
        let y = y - curvature * (x * x + z * z);
        let (hs, hc) = self.heading;
        let (ps, pc) = self.pitch;
        let forward = hs * x + hc * z;
        [
            hc * x - hs * z,
            -ps * forward - pc * y,
            pc * forward - ps * y,
        ]
    }

    fn projected(&self, point: [f32; 3]) -> Option<[f32; 2]> {
        let [x, y, depth] = self.camera_point(point, 0.0);
        if depth < MESH_NEAR {
            return None;
        }
        let scale = self.focal / depth;
        Some([scale * x, scale * y])
    }

    fn resolves_sleeper(&self, a: [f32; 3], b: [f32; 3]) -> bool {
        let (Some(a), Some(b)) = (self.projected(a), self.projected(b)) else {
            return false;
        };
        let [x, y] = [b[0] - a[0], b[1] - a[1]];
        x * x + y * y >= 1.24
    }
}

struct RailCoverage<'a> {
    dimensions: (usize, usize),
    roll: (f32, f32),
    curvature: f32,
    view: &'a DetailView,
    region: Option<ScreenRect>,
}

impl RailCoverage<'_> {
    fn include(&mut self, quad: Face) {
        let points = quad.map(|point| self.view.camera_point(point, self.curvature));
        if !points.iter().flatten().all(|value| value.is_finite()) {
            return;
        }
        let (width, height) = self.dimensions;
        let center_x = (width - 1) as f32 * 0.5;
        let center_y = (height - 1) as f32 * 0.5;
        let (sin, cos) = self.roll;
        let mut limits = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        let mut include = |[x, y, depth]: [f32; 3]| {
            let scale = self.view.focal * (1.0 / depth);
            let px = center_x + scale * (x * cos - y * sin);
            let py = center_y + scale * (x * sin + y * cos);
            limits = [
                limits[0].min(px),
                limits[1].min(py),
                limits[2].max(px),
                limits[3].max(py),
            ];
        };
        let in_front = points.map(|point| point[2] >= MESH_NEAR);
        for (point, visible) in points.into_iter().zip(in_front) {
            if visible {
                include(point);
            }
        }
        if in_front.iter().any(|&visible| visible) && in_front.iter().any(|&visible| !visible) {
            // These are the edges of both triangles, including the shared diagonal.
            for (i, j) in [(0, 1), (1, 2), (2, 3), (3, 0), (0, 2)] {
                if in_front[i] != in_front[j] {
                    let (a, b) = (points[i], points[j]);
                    let t = (MESH_NEAR - a[2]) / (b[2] - a[2]);
                    include([
                        a[0] + (b[0] - a[0]) * t,
                        a[1] + (b[1] - a[1]) * t,
                        MESH_NEAR,
                    ]);
                }
            }
        }
        let [left, top, right, bottom] = limits;
        // Allow a pixel of slack for clipping direction and integer raster rounding.
        let rect = ScreenRect {
            left: ((left - 1.0) as usize).min(width),
            top: ((top - 1.0) as usize).min(height),
            right: ((right + 2.0) as usize).min(width),
            bottom: ((bottom + 2.0) as usize).min(height),
        };
        if rect.left < rect.right && rect.top < rect.bottom {
            self.region = Some(self.region.map_or(rect, |current| current.union(rect)));
        }
    }
}

#[derive(Clone, Copy)]
struct Segment {
    start: [f32; 3],
    delta: [f32; 3],
    length: f32,
    normal: [f32; 2],
}

impl Segment {
    fn between(a: RailPoint, b: RailPoint) -> Self {
        let delta = [b.x - a.x, b.elevation - a.elevation, b.z - a.z];
        let length = root(delta[0] * delta[0] + delta[2] * delta[2]);
        let inverse = 1.0 / length.max(0.001);
        let normal = [-delta[2] * inverse, delta[0] * inverse];
        Self {
            start: [a.x, a.elevation, a.z],
            delta,
            length,
            normal,
        }
    }

    fn center(self, t: f32, rise: f32) -> [f32; 3] {
        [
            self.start[0] + self.delta[0] * t,
            self.start[1] + self.delta[1] * t + rise,
            self.start[2] + self.delta[2] * t,
        ]
    }

    fn portion(self, eye: [f32; 3], radius: f32) -> Option<(f32, f32)> {
        let start = subtract(self.start, eye);
        let squared_length = dot(self.delta, self.delta);
        let middle = -dot(start, self.delta) / squared_length;
        let perpendicular = core::array::from_fn(|i| start[i] + self.delta[i] * middle);
        let remainder = radius * radius - dot(perpendicular, perpendicular);
        if remainder <= 0.0 {
            return None;
        }
        let t = root(remainder / squared_length);
        let first = (middle - t).max(0.0);
        let last = (middle + t).min(1.0);
        (first < last).then_some((first, last))
    }
}

fn track_geometry(
    track: &[RailPoint],
    view: &DetailView,
    sleeper_envelope: bool,
    emit: &mut impl FnMut(Face, [u8; 3]),
) {
    let mut distance = 0.0;
    for (index, pair) in track.windows(2).enumerate() {
        let segment = Segment::between(pair[0], pair[1]);
        if segment.length < 0.001 {
            continue;
        }
        if let Some((first, last)) = segment.portion(view.eye, view.rails_reach) {
            let start = if index == 0 {
                segment.normal
            } else {
                miter(
                    Segment::between(track[index - 1], pair[0]).normal,
                    segment.normal,
                )
            };
            let end = track.get(index + 2).map_or(segment.normal, |next| {
                miter(segment.normal, Segment::between(pair[1], *next).normal)
            });
            let station = |t, across| {
                let normal = [
                    start[0] + (end[0] - start[0]) * t,
                    start[1] + (end[1] - start[1]) * t,
                ];
                lateral(segment.center(t, RAIL_TOP), normal, across)
            };
            for side in [-HALF_GAUGE, HALF_GAUGE] {
                emit(
                    [
                        station(first, side - HALF_RAIL_WIDTH),
                        station(first, side + HALF_RAIL_WIDTH),
                        station(last, side + HALF_RAIL_WIDTH),
                        station(last, side - HALF_RAIL_WIDTH),
                    ],
                    RAIL_COLOR,
                );
            }

            if let Some((near, far)) = segment.portion(view.eye, view.sleeper_reach) {
                if sleeper_envelope {
                    // With zero world curvature, perspective and near-plane clipping
                    // preserve containment. A single strip covers every tie centered
                    // in this span, including its ends. Drawing still resolves each tie.
                    let end_guard = (HALF_TIE_DEPTH + 0.005) / segment.length;
                    emit(sleeper_strip(segment, (near - end_guard).max(0.0), (far + end_guard).min(1.0)), WOOD_COLOR);
                } else {
                    sleeper_geometry(view, segment, distance, near, far, emit);
                }
            }
        }
        distance += segment.length;
    }
}

fn corridor_bed(
    track: &[RailPoint],
    view: &DetailView,
    camera: Camera,
    mut emit: impl FnMut(Face, f32),
) {
    let reach = view_distance(camera.altitude) * 0.84;
    for (index, pair) in track.windows(2).enumerate() {
        let segment = Segment::between(pair[0], pair[1]);
        let Some((from, to)) = segment.portion(view.eye, reach) else { continue };
        let start = if index == 0 {
            segment.normal
        } else {
            miter(Segment::between(track[index - 1], pair[0]).normal, segment.normal)
        };
        let end = track.get(index + 2).map_or(segment.normal, |next| {
            miter(segment.normal, Segment::between(pair[1], *next).normal)
        });
        let point = |t: f32, across| {
            let normal = [start[0] + (end[0] - start[0]) * t, start[1] + (end[1] - start[1]) * t];
            lateral(segment.center(t, 0.11), normal, across)
        };
        let center = segment.center((from + to) * 0.5, 0.0);
        let delta = subtract(center, view.eye);
        let fading = 1.0 - (dot(delta, delta) / (reach * reach)).clamp(0.0, 1.0);
        let opacity = (fading * 2.4).clamp(0.0, 1.0);
        if opacity >= 0.025 {
            emit([point(from, -1.38), point(from, 1.38), point(to, 1.38), point(to, -1.38)], opacity);
        }
    }
}

fn sleeper_geometry(
    view: &DetailView,
    segment: Segment,
    distance: f32,
    near: f32,
    far: f32,
    emit: &mut impl FnMut(Face, [u8; 3]),
) {
    let offset = distance + near * segment.length;
    let mut station = ((offset / SLEEPER_PERIOD) as u32) as f32 * SLEEPER_PERIOD + HALF_TIE_DEPTH;
    if station < offset {
        station += SLEEPER_PERIOD;
    }
    let stop = distance + far * segment.length;
    let step = SLEEPER_PERIOD / segment.length;
    let tie_depth = HALF_TIE_DEPTH / segment.length;
    while station < stop {
        let t = (station - distance) / segment.length;
        let center = segment.center(t, SLEEPER_TOP);
        if view.resolves_sleeper(center, segment.center(t + step, SLEEPER_TOP)) {
            emit(
                sleeper_strip(segment, (t - tie_depth).max(0.0), (t + tie_depth).min(1.0)),
                WOOD_COLOR,
            );
        }
        station += SLEEPER_PERIOD;
    }
}

fn sleeper_strip(segment: Segment, first: f32, last: f32) -> Face {
    let a = segment.center(first, SLEEPER_TOP);
    let b = segment.center(last, SLEEPER_TOP);
    let n = segment.normal;
    [lateral(a, n, -HALF_SLEEPER), lateral(a, n, HALF_SLEEPER),
     lateral(b, n, HALF_SLEEPER), lateral(b, n, -HALF_SLEEPER)]
}

fn lateral(mut point: [f32; 3], normal: [f32; 2], offset: f32) -> [f32; 3] {
    point[0] += normal[0] * offset;
    point[2] += normal[1] * offset;
    point
}

fn miter(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    let inverse = 1.0 / (1.0 + a[0] * b[0] + a[1] * b[1]).max(0.85);
    [(a[0] + b[0]) * inverse, (a[1] + b[1]) * inverse]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn root(value: f32) -> f32 {
    if value == 0.0 {
        return 0.0;
    }
    let estimate = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    let settled = 0.5 * (estimate + value / estimate);
    0.5 * (settled + value / settled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rail::RailAnchorKind;
    use crate::render::{render_terrain_layer_region, render_with_depth};
    use crate::roads::RoadNetwork;
    use crate::sky::Sky;
    use crate::world::World;

    fn freight() -> (World, RailLine) {
        let world = World::new(1);
        let road = RoadNetwork::generate(&world, -1, -3).unwrap();
        let line =
            RailLine::for_road_with_terminals(&world, 1, &road, [RailAnchorKind::Port; 2]).unwrap();
        (world, line)
    }

    #[test]
    fn overlay_retains_the_grade_and_connects_station_access() {
        let (world, rail) = freight();
        let start = rail.polyline()[0];
        let ground = world.sample(start.x, start.z);
        let bed = overlay_rail(&rail, start.x, start.z, ground);
        assert_eq!(bed.surface, Surface::Gravel);
        assert_eq!(bed.elevation, start.elevation);

        for station in rail.stations() {
            let (x, z) = station.road_contact();
            let access = rail.sample(x, z).unwrap();
            assert_eq!(access.material, RailMaterial::StationAccess);
            let paved = Sample {
                surface: Surface::Pavement,
                ..world.sample(x, z)
            };
            assert_eq!(overlay_rail(&rail, x, z, paved), paved);
        }

        let lake = Sample {
            elevation: 6.0,
            surface: Surface::RiverWater,
        };
        assert_eq!(overlay_rail(&rail, start.x, start.z, lake), lake);
    }

    #[test]
    fn native_track_respects_terrain_depth_and_high_view_lod() {
        const SIDE: usize = 112;
        const AREA: usize = SIDE * SIDE;
        let (world, line) = freight();
        let camera = Camera {
            x: -531.18,
            z: -2542.667,
            altitude: 19.603,
            heading: 1.972,
            pitch: 0.35,
            bank: -0.10,
        };
        let sky = Sky::from_time(1, 0, 0.43);
        let mut base = [0; AREA];
        let mut work = [0; AREA];
        let mut depth = [0; AREA];
        render_with_depth(
            |x, z| overlay_rail(&line, x, z, world.sample(x, z)),
            camera,
            &sky,
            &mut base,
            &mut work,
            &mut depth,
            (SIDE, SIDE),
        )
        .unwrap();
        let original_depth = depth;
        let mut near = base;
        draw_rail(
            &mut WorldMesh::new(&mut near, &mut depth, (SIDE, SIDE), camera, &sky).unwrap(),
            &line,
            camera,
            (SIDE, SIDE),
        );
        assert!(near.iter().zip(base).filter(|(a, b)| **a != *b).count() >= 70);
        assert!(depth.iter().zip(original_depth).all(|(a, b)| *a <= b));

        let mut occluded = base;
        depth.fill(0);
        draw_rail(
            &mut WorldMesh::new(&mut occluded, &mut depth, (SIDE, SIDE), camera, &sky).unwrap(),
            &line,
            camera,
            (SIDE, SIDE),
        );
        assert_eq!(occluded, base);

        let above = Camera {
            altitude: 120.0,
            ..camera
        };
        let mut high = base;
        depth.fill(u16::MAX);
        draw_rail(
            &mut WorldMesh::new(&mut high, &mut depth, (SIDE, SIDE), above, &sky).unwrap(),
            &line,
            above,
            (SIDE, SIDE),
        );
        assert_eq!(high, base);
        assert!(rail_screen_region(&line, above, (SIDE, SIDE)).is_none());
    }

    #[test]
    fn banked_track_and_freight_siding_match_full_terrain_depth() {
        const SIDE: usize = 112;
        const AREA: usize = SIDE * SIDE;
        let dimensions = (SIDE, SIDE);
        let (world, freight) = freight();
        assert!(freight.siding().is_some());
        let town = World::new(119);
        let road = RoadNetwork::generate(&town, -1, 3).unwrap();
        let commuter = RailLine::for_road(&town, 119, &road).unwrap();
        let mut drawn = 0;
        let mut bounded = 0;
        for (world, line, seed) in [(world, &freight, 1), (town, &commuter, 119)] {
            let sky = Sky::from_time(seed, 0, 0.43);
            let sample = |x, z| overlay_rail(line, x, z, world.sample(x, z));
            for track in
                core::iter::once(line.polyline()).chain(line.siding().map(|line| &line[..]))
            {
                for pair in [
                    &track[..2],
                    &track[(track.len() / 2 - 1)..=(track.len() / 2)],
                    &track[(track.len() - 2)..],
                ] {
                    let segment = Segment::between(pair[0], pair[1]);
                    let [x, y, z] = segment.center(0.45, 0.0);
                    let [nx, nz] = segment.normal;
                    let [ax, az] = [nz, -nx];
                    for (back, side, height, pitch, bank, yaw) in [
                        (0.2, 0.5, 1.2, 0.19, 0.65, 0.02),
                        (0.8, -1.4, 6.0, 0.54, -0.53, -0.52),
                        (16.0, 3.8, 13.0, 0.46, 0.20, 0.2),
                        (32.0, -18.0, 23.0, 0.34, -0.35, -0.54),
                        (50.0, 18.0, 39.0, 0.43, 0.45, 0.24),
                        (18.0, 0.8, 25.0, 0.38, -0.24, 2.74),
                    ] {
                        let camera = Camera {
                            x: x - ax * back + nx * side,
                            z: z - az * back + nz * side,
                            altitude: y + height,
                            heading: ax.atan2(az) + yaw,
                            pitch,
                            bank,
                        };
                        let region = rail_screen_region(line, camera, dimensions);
                        let mut full = [0; AREA];
                        let mut partial = full;
                        let mut work = full;
                        let mut full_depth = full;
                        let mut partial_depth = [u16::MAX; AREA];
                        render_with_depth(
                            sample,
                            camera,
                            &sky,
                            &mut full,
                            &mut work,
                            &mut full_depth,
                            dimensions,
                        )
                        .unwrap();
                        render_terrain_layer_region(
                            (sample, |_, _, ground| ground.surface),
                            camera,
                            &sky,
                            &mut partial,
                            &mut work,
                            region.map(|rect| (&mut partial_depth[..], rect)),
                            dimensions,
                            |_| {},
                        )
                        .unwrap();
                        assert_eq!(full, partial);
                        let ground_frame = full;
                        for (pixels, depth) in [
                            (&mut full, &mut full_depth),
                            (&mut partial, &mut partial_depth),
                        ] {
                            draw_rail(
                                &mut WorldMesh::new(pixels, depth, dimensions, camera, &sky)
                                    .unwrap(),
                                line,
                                camera,
                                dimensions,
                            );
                        }
                        assert_eq!(full, partial, "{camera:?}");
                        for index in 0..AREA {
                            let (x, y) = (index % SIDE, index / SIDE);
                            if region.is_some_and(|r| {
                                x >= r.left && x < r.right && y >= r.top && y < r.bottom
                            }) {
                                assert_eq!(full_depth[index], partial_depth[index]);
                            } else {
                                assert_eq!(partial_depth[index], u16::MAX, "{camera:?} at {x},{y}");
                            }
                        }
                        drawn += usize::from(full != ground_frame);
                        bounded += usize::from(
                            region.is_some_and(|r| (r.right - r.left) * (r.bottom - r.top) < AREA),
                        );
                    }
                }
            }
        }
        assert!(drawn >= 35);
        assert!(bounded >= 25);
    }
}
