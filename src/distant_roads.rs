//! Subpixel pavement along surveyed roads beyond the close ribbon pass.

use crate::highway_stream::HighwayStream;
use crate::land_stream::LandStream;
use crate::math::smoothstep;
use crate::render::{
    CITY_HORIZON, Camera, SCENERY_HORIZON, ScreenProjection, ScreenRect, encoded_depth, view_distance,
};
use crate::roads::{RoadPoint, ROAD_HALF_WIDTH};

const STROKES: usize = 112;
const OUTER_STROKES: usize = 24;
const PATHS: usize = 24;
const REVEAL_TICKS: u64 = 36;
const OUTER_JOIN_DISTANCE: f32 = 2_000.0;
const OUTER_SEGMENT_LENGTH: f32 = 144.0;

#[derive(Clone, Copy)]
struct Stroke {
    from: [f32; 3],
    to: [f32; 3],
    rect: ScreenRect,
    half_width: f32,
    alpha: f32,
    outer: bool,
}

impl Stroke {
    const EMPTY: Self = Self {
        from: [0.0; 3],
        to: [0.0; 3],
        rect: ScreenRect { left: 0, top: 0, right: 0, bottom: 0 },
        half_width: 0.0,
        alpha: 0.0,
        outer: false,
    };
}

#[derive(Clone, Copy)]
struct RoadSeen {
    ends: [u32; 4],
    first: u64,
    last: u64,
}

pub struct DistantRoads {
    strokes: [Stroke; STROKES],
    count: usize,
    outer_count: usize,
    region: Option<ScreenRect>,
    known: [Option<RoadSeen>; PATHS],
    tick: u64,
}

impl DistantRoads {
    pub const fn new() -> Self {
        Self {
            strokes: [Stroke::EMPTY; STROKES],
            count: 0,
            outer_count: 0,
            region: None,
            known: [None; PATHS],
            tick: 0,
        }
    }

    pub fn reset(&mut self) {
        self.count = 0;
        self.outer_count = 0;
        self.region = None;
        self.known.fill(None);
        self.tick = 0;
    }

    pub fn prepare<const LAND: usize, const LINKS: usize>(
        &mut self,
        land: &LandStream<LAND>,
        highways: &HighwayStream<LINKS>,
        camera: Camera,
        ticks: u64,
        dimensions: (usize, usize),
        mut horizon_ground: impl FnMut(f32, f32) -> f32,
    ) {
        self.count = 0;
        self.outer_count = 0;
        self.region = None;
        if ticks < self.tick {
            self.reset();
        }
        self.tick = ticks;
        let Some(view) = RoadView::new(camera, dimensions) else { return; };

        for (_, link) in highways.links() {
            self.add(link.polyline(), &view, &mut horizon_ground);
        }
        for (_, link) in highways.coastal_links() {
            for approach in link.approaches() {
                self.add(approach, &view, &mut horizon_ground);
            }
        }
        for (key, district) in land.districts() {
            if land.district_published(key) {
                if let Some(road) = district.road() {
                    self.add(road.polyline(), &view, &mut horizon_ground);
                }
            }
        }
    }

    pub fn bounds(&self) -> Option<ScreenRect> { self.region }

    pub fn len(&self) -> usize { self.count }

    fn add(&mut self, points: &[RoadPoint], view: &RoadView, horizon_ground: &mut impl FnMut(f32, f32) -> f32) {
        let [Some(a), Some(b)] = [points.first(), points.last()] else { return; };
        if self.count == STROKES {
            return;
        }
        let ends = [a.x, a.z, b.x, b.z].map(f32::to_bits);
        let current = self.tick;
        let first = if let Some(record) = self.known.iter_mut().flatten().find(|road| road.ends == ends) {
            if record.last.saturating_add(REVEAL_TICKS) < current {
                record.first = current;
            }
            record.last = current;
            record.first
        } else {
            let slot = self.known.iter_mut().min_by_key(|entry| entry.map_or(0, |r| r.last.saturating_add(1))).unwrap();
            *slot = Some(RoadSeen { ends, first: current, last: current });
            current
        };
        let reveal = smoothstep((current - first) as f32 / REVEAL_TICKS as f32);
        if reveal == 0.0 {
            return;
        }
        let mut first = 0;
        while first + 1 < points.len() && self.count < STROKES {
            let start = points[first];
            let end = view.outer_segment_end(points, first);
            first = end;
            let Some(stroke) = view.stroke(start, points[end], reveal, horizon_ground) else { continue; };
            if stroke.outer && self.outer_count == OUTER_STROKES {
                continue;
            }
            self.region = Some(self.region.map_or(stroke.rect, |rect| rect.union(stroke.rect)));
            self.strokes[self.count] = stroke;
            self.count += 1;
            self.outer_count += usize::from(stroke.outer);
        }
    }

    /// Use ground coverage only. A road can never add color to sky or cover
    /// a substantially nearer hillside.
    pub fn draw(&self, pixels: &mut [u16], depth: &[u16], coverage: &mut [u16], (width, height): (usize, usize)) {
        let Some(region) = self.region else { return; };
        if width == 0 || height == 0 || width > 512 || height > 512
            || pixels.len() < width * height || depth.len() < width * height || coverage.len() < width * height
        {
            return;
        }
        let left = region.left.min(width);
        let right = region.right.min(width);
        let rows = region.top.min(height)..region.bottom.min(height);
        for y in rows.clone() {
            coverage[y * width + left..y * width + right].fill(0);
        }
        let aperture = width.min(height) as i32;
        for line in &self.strokes[..self.count] {
            let [ax, ay, az] = line.from;
            let [bx, by, bz] = line.to;
            let dx = bx - ax;
            let dy = by - ay;
            let inverse_square = 1.0 / (dx * dx + dy * dy);
            let inv_a = 1.0 / az;
            let inv_delta = 1.0 / bz - inv_a;
            for y in line.rect.top..line.rect.bottom.min(height) {
                let cy = (2 * y + 1) as i32 - height as i32;
                let py = y as f32 - ay;
                for x in line.rect.left..line.rect.right.min(width) {
                    let cx = (2 * x + 1) as i32 - width as i32;
                    if cx * cx + cy * cy > aperture * aperture {
                        continue;
                    }
                    let px = x as f32 - ax;
                    let t = ((px * dx + py * dy) * inverse_square).clamp(0.0, 1.0);
                    let ex = px - t * dx;
                    let ey = py - t * dy;
                    let reach = line.half_width + 0.55;
                    let amount = (1.0 - (ex * ex + ey * ey) / (reach * reach)).clamp(0.0, 1.0);
                    if amount <= 0.03 {
                        continue;
                    }
                    let index = y * width + x;
                    let ground_depth = depth[index];
                    if ground_depth == u16::MAX {
                        continue;
                    }
                    let road_z = 1.0 / (inv_a + inv_delta * t);
                    let matched = if !line.outer && road_z < SCENERY_HORIZON - 80.0 {
                        let road_depth = encoded_depth(road_z);
                        road_depth.abs_diff(ground_depth) <= road_depth / 32 + 30
                    } else {
                        // Terrain beyond the old horizon is sampled in
                        // 160-unit steps, and its encoded precision is lower.
                        let tolerance = road_z * (1.0 / 32.0) + 0.94
                            + if line.outer { 72.0 } else { 0.0 };
                        (encoded_depth(road_z - tolerance)..=encoded_depth(road_z + tolerance))
                            .contains(&ground_depth)
                    };
                    if matched {
                        let strength = (amount * line.alpha * 13.0 + 0.5) as u16;
                        coverage[index] = coverage[index].max(strength);
                    }
                }
            }
        }
        for y in rows {
            let span = y * width + left..y * width + right;
            for (pixel, &strength) in pixels[span.clone()].iter_mut().zip(&coverage[span]) {
                if strength != 0 {
                    let shade = 32 - u32::from(strength);
                    let rgb = u32::from(*pixel);
                    let red = ((rgb >> 11) * shade + 16) >> 5;
                    let green = (((rgb >> 5) & 0x3f) * shade + 16) >> 5;
                    let blue = ((rgb & 0x1f) * ((shade + 32) / 2) + 16) >> 5;
                    *pixel = ((red << 11) | (green << 5) | blue) as u16;
                }
            }
        }
    }
}

struct RoadView {
    projection: ScreenProjection,
    camera: Camera,
    dimensions: (usize, usize),
    focal: f32,
    detail: f32,
}

impl RoadView {
    fn outer_segment_end(&self, points: &[RoadPoint], first: usize) -> usize {
        let start = points[first];
        let near = |point: RoadPoint| {
            let dx = point.x - self.camera.x;
            let dz = point.z - self.camera.z;
            dx * dx + dz * dz < OUTER_JOIN_DISTANCE * OUTER_JOIN_DISTANCE
        };
        let mut end = first + 1;
        if near(start) || near(points[end]) {
            return end;
        }
        // At this distance short route edges project to fractions of a pixel.
        // Join almost straight edges before projection so they form a stroke.
        while end + 1 < points.len() && end - first < 6 {
            let next = points[end + 1];
            let dx = next.x - start.x;
            let dz = next.z - start.z;
            let length_squared = dx * dx + dz * dz;
            if !(1.0..=OUTER_SEGMENT_LENGTH * OUTER_SEGMENT_LENGTH).contains(&length_squared)
                || near(next)
            {
                break;
            }
            let rise = next.elevation - start.elevation;
            let follows_chord = points[first + 1..=end].iter().all(|point| {
                let x = point.x - start.x;
                let z = point.z - start.z;
                let along = x * dx + z * dz;
                let across = x * dz - z * dx;
                (0.0..=length_squared).contains(&along)
                    && across * across <= 16.0 * length_squared
                    && (point.elevation - start.elevation - rise * along / length_squared).abs() <= 2.0
            });
            if !follows_chord {
                break;
            }
            end += 1;
        }
        end
    }

    fn new(camera: Camera, dimensions: (usize, usize)) -> Option<Self> {
        let (width, height) = dimensions;
        if width > 512 || height > 512 { return None; }
        Some(Self {
            projection: ScreenProjection::new(camera, dimensions)?,
            camera,
            dimensions,
            focal: width.min(height) as f32 * 0.73,
            detail: view_distance(camera.altitude),
        })
    }

    fn stroke(&self, a: RoadPoint, b: RoadPoint, reveal: f32,
        horizon_ground: &mut impl FnMut(f32, f32) -> f32,
    ) -> Option<Stroke> {
        let mut center = [(a.x + b.x) * 0.5, (a.elevation + b.elevation) * 0.5, (a.z + b.z) * 0.5];
        let dx = center[0] - self.camera.x;
        let dz = center[2] - self.camera.z;
        let distance_2d = dx * dx + dz * dz;
        let reach = CITY_HORIZON;
        let outer = distance_2d > SCENERY_HORIZON * SCENERY_HORIZON;
        if distance_2d > reach * reach || center[1] <= 0.0 { return None; }
        let dy = center[1] - self.camera.altitude;
        let distance_3d = distance_2d + dy * dy;
        let near = (self.focal * 2.17).min(self.detail * 0.50);
        let near_end = (self.focal * 3.13).min(self.detail * 0.66);
        let far_start = reach * 0.78;
        let enter = smoothstep((distance_3d - near * near) / (near_end * near_end - near * near));
        let leave = smoothstep((reach * reach - distance_3d) / (reach * reach - far_start * far_start));
        let handoff_start = self.detail * 0.98;
        let handoff_end = self.detail * 1.20;
        let blend = smoothstep((distance_2d - handoff_start * handoff_start)
            / (handoff_end * handoff_end - handoff_start * handoff_start));
        let [mut ah, mut bh] = [a.elevation, b.elevation];
        let mut alpha = reveal * enter * leave;
        if blend > 0.0 && alpha >= 0.075 {
            let ground = horizon_ground(center[0], center[2]);
            let agreement = smoothstep(ground / 2.0)
                * (1.0 - smoothstep(((ground - center[1]).abs() - 8.0) / 18.0));
            alpha *= 1.0 - blend * (1.0 - agreement);
            let a_ground = horizon_ground(a.x, a.z);
            let b_ground = horizon_ground(b.x, b.z);
            if outer && a_ground.min(b_ground) <= 0.0 { return None; }
            ah += blend * (a_ground.max(0.0) - ah);
            bh += blend * (b_ground.max(0.0) - bh);
            center[1] = (ah + bh) * 0.5;
        }
        if alpha < 0.075 { return None; }
        let from = self.projection.backdrop_point([a.x, ah + 0.16, a.z])?;
        let to = self.projection.backdrop_point([b.x, bh + 0.16, b.z])?;
        let direction = [to[0] - from[0], to[1] - from[1]];
        let length_sq = direction[0] * direction[0] + direction[1] * direction[1];
        if length_sq < 0.02 || length_sq > 3600.0 { return None; }
        let along = [b.x - a.x, b.z - a.z];
        let width_sq = along[0] * along[0] + along[1] * along[1];
        if width_sq < 0.01 { return None; }
        let cross = ROAD_HALF_WIDTH / root(width_sq);
        let mid = self.projection.backdrop_point(center)?;
        let edge = self.projection.backdrop_point([center[0] - along[1] * cross, center[1], center[2] + along[0] * cross])?;
        let sideways = (direction[0] * (edge[1] - mid[1]) - direction[1] * (edge[0] - mid[0])).abs();
        let half_width = (sideways / root(length_sq)).clamp(0.14, 1.30);
        let margin = half_width + 1.55;
        let (width, height) = self.dimensions;
        let rect = ScreenRect {
            left: ((from[0].min(to[0]) - margin) as usize).min(width),
            top: ((from[1].min(to[1]) - margin) as usize).min(height),
            right: ((from[0].max(to[0]) + margin) as usize).min(width),
            bottom: ((from[1].max(to[1]) + margin) as usize).min(height),
        };
        if rect.left >= rect.right || rect.top >= rect.bottom { return None; }
        Some(Stroke { from, to, rect, half_width, alpha, outer })
    }
}

fn root(value: f32) -> f32 {
    let start = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    let estimate = (start + value / start) * 0.5;
    (estimate + value / estimate) * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::sin_cos;
    use crate::render::render_with_depth;
    use crate::roads::RoadNetwork;
    use crate::scenery::DistrictRegion;
    use crate::sky::Sky;
    use crate::world::World;

    #[test]
    fn long_range_roads_use_the_same_horizon_and_stay_on_dry_ground() {
        for altitude in [30.0, 320.0] {
            let view = RoadView::new(Camera {
                x: 0.0, z: 0.0, altitude, heading: 0.0, pitch: 0.19, bank: 0.0,
            }, (60, 60)).unwrap();
            let a = RoadPoint { x: -18.0, z: 1500.0, elevation: 18.0 };
            let b = RoadPoint { x: 18.0, ..a };
            let stroke = view.stroke(a, b, 1.0, &mut |_, _| 20.0).unwrap();
            let flat = view.stroke(a, b, 1.0, &mut |_, _| 18.0).unwrap();
            assert!(stroke.from[1] < flat.from[1]);
            assert!(view.stroke(a, b, 1.0, &mut |_, _| -4.0).is_none());
            assert!(view.stroke(RoadPoint { z: 2240.0, ..a }, RoadPoint { z: 2240.0, ..b },
                1.0, &mut |_, _| 20.0).unwrap().outer);
            assert!(view.stroke(RoadPoint { z: 3200.0, ..a }, RoadPoint { z: 3200.0, ..b },
                1.0, &mut |_, _| 20.0).unwrap().outer);
            let beyond = CITY_HORIZON + 10.0;
            assert!(view.stroke(RoadPoint { z: beyond, ..a }, RoadPoint { z: beyond, ..b },
                1.0, &mut |_, _| 20.0).is_none());
        }
    }

    #[test]
    fn short_edges_connect_into_a_visible_road_at_four_thousand_units() {
        const SIZE: (usize, usize) = (60, 60);
        const AREA: usize = SIZE.0 * SIZE.1;
        let view = RoadView::new(Camera {
            x: 0.0, z: 0.0, altitude: 68.0, heading: 0.0, pitch: 0.15, bank: 0.0,
        }, SIZE).unwrap();
        let far = core::array::from_fn::<_, 7, _>(|i| RoadPoint {
            x: (i as f32 - 3.0) * 8.0,
            z: 4_200.0,
            elevation: 20.0,
        });
        let ground = &mut |_, _| 20.0;
        assert!(far.windows(2).all(|pair| view.stroke(pair[0], pair[1], 1.0, ground).is_none()));
        assert_eq!(view.outer_segment_end(&far, 0), far.len() - 1);
        let mut bend = far;
        bend[2].z += 14.0;
        assert!(view.outer_segment_end(&bend, 0) < bend.len() - 1);
        let nearer = far.map(|point| RoadPoint { z: 1_900.0, ..point });
        assert_eq!(view.outer_segment_end(&nearer, 0), 1);

        let mut roads = DistantRoads::new();
        roads.add(&far, &view, ground);
        roads.tick = REVEAL_TICKS;
        roads.add(&far, &view, ground);
        assert_eq!(roads.len(), 1);
        let stroke = roads.strokes[0];
        let depth = [encoded_depth(stroke.from[2]); AREA];
        let terrain = [0x7bef; AREA];
        let mut pixels = terrain;
        let mut work = [0; AREA];
        roads.draw(&mut pixels, &depth, &mut work, SIZE);
        assert_ne!(pixels, terrain);
        for depth in [encoded_depth(1_800.0), u16::MAX] {
            pixels = terrain;
            roads.draw(&mut pixels, &[depth; AREA], &mut work, SIZE);
            assert_eq!(pixels, terrain);
        }
    }

    #[test]
    fn surveyed_roads_fade_in_pick_up_the_ground_and_leave_sky_and_near_geometry_alone() {
        const SIDE: usize = 112;
        const AREA: usize = SIDE * SIDE;
        const SIZE: (usize, usize) = (SIDE, SIDE);
        let world = World::new(1);
        let road = RoadNetwork::generate(&world, -1, 0).unwrap();
        let district = DistrictRegion::build(&world, -1, 0);
        let path = road.polyline();
        let mid = path[path.len() / 2];
        let sky = Sky::from_time(1, 0, 0.46);
        let mut evidence = 0;
        for step in 0..8 {
            let heading = step as f32 * core::f32::consts::TAU / 8.0;
            let (east, north) = sin_cos(heading);
            let camera = Camera {
                x: mid.x - east * 330.0,
                z: mid.z - north * 330.0,
                altitude: mid.elevation + 72.0,
                heading,
                pitch: 0.20,
                bank: -0.12,
            };
            let view = RoadView::new(camera, SIZE).unwrap();
            let mut roads = DistantRoads::new();
            roads.add(path, &view, &mut |_, _| 0.0);
            assert!(roads.bounds().is_none());
            roads.tick = 18;
            roads.add(path, &view, &mut |_, _| 0.0);
            let half = roads.strokes[..roads.count].to_vec();
            roads.region = None;
            roads.count = 0;
            roads.tick = REVEAL_TICKS;
            roads.add(path, &view, &mut |_, _| 0.0);
            if let Some(region) = roads.bounds() {
                for earlier in half {
                    let later = roads.strokes[..roads.count].iter().find(|line| line.from == earlier.from).unwrap();
                    assert!(later.alpha > earlier.alpha);
                }
                let mut before = [0; AREA];
                let mut depth = [0; AREA];
                let mut work = [0; AREA];
                render_with_depth(|x, z| district.overlay(x, z, world.sample(x, z)), camera, &sky,
                    &mut before, &mut work, &mut depth, SIZE).unwrap();
                let mut after = before;
                roads.draw(&mut after, &depth, &mut work, SIZE);
                for (index, (&a, &b)) in before.iter().zip(&after).enumerate() {
                    if a != b {
                        evidence += 1;
                        assert_ne!(depth[index], u16::MAX);
                        assert!((region.left..region.right).contains(&(index % SIDE)));
                        assert!((region.top..region.bottom).contains(&(index / SIDE)));
                    }
                }
                let mut behind_foreground = before;
                depth.fill(encoded_depth(10.0));
                roads.draw(&mut behind_foreground, &depth, &mut work, SIZE);
                assert_eq!(behind_foreground, before);
            }
        }
        assert!(evidence > 50, "{evidence}");
        let near = RoadView::new(Camera {
            x: mid.x, z: mid.z - 40.0, altitude: mid.elevation + 20.0,
            heading: 0.0, pitch: 0.2, bank: 0.0,
        }, SIZE).unwrap();
        assert!(near.stroke(mid, path[path.len() / 2 + 1], 1.0, &mut |_, _| mid.elevation).is_none());
    }

    #[test]
    fn seeded_roads_draw_past_the_old_horizon_on_the_board_raster() {
        use crate::horizon_cache::HorizonCache;
        use crate::render::render_terrain_layer_region_with_horizon;
        use crate::terrain_cache::TerrainCache;

        const SIDE: usize = 60;
        const AREA: usize = SIDE * SIDE;
        const SIZE: (usize, usize) = (SIDE, SIDE);
        let world = World::new(1);
        let network = RoadNetwork::generate(&world, -1, 0).unwrap();
        let path = network.polyline();
        let focus = path[path.len() / 2];
        let sky = Sky::from_time(1, 0, 0.46);
        for altitude in [48.0, 335.0] {
            let evidence = (0..12).any(|step| {
                let heading = step as f32 * (core::f32::consts::TAU / 12.0);
                let (east, north) = sin_cos(heading);
                let camera = Camera {
                    x: focus.x - east * 2240.0, z: focus.z - north * 2240.0, altitude, heading,
                    pitch: 0.18, bank: 0.03,
                };
                let mut horizon = HorizonCache::new();
                horizon.reset(world, camera.x, camera.z);
                let backdrop = |x, z| horizon.backdrop(x, z);
                let view = RoadView::new(camera, SIZE).unwrap();
                let mut roads = DistantRoads::new();
                roads.add(path, &view, &mut |x, z| backdrop(x, z));
                roads.tick = REVEAL_TICKS;
                roads.add(path, &view, &mut |x, z| backdrop(x, z));
                let mut outer = 0;
                for index in 0..roads.count {
                    if roads.strokes[index].outer {
                        roads.strokes[outer] = roads.strokes[index];
                        outer += 1;
                    }
                }
                roads.count = outer;
                if outer == 0 {
                    return false;
                }
                let mut terrain = TerrainCache::new();
                terrain.reset_for_altitude(world, camera.x, camera.z, altitude);
                let mut before = [0; AREA];
                let mut work = [0; AREA];
                let mut depth = [u16::MAX; AREA];
                render_terrain_layer_region_with_horizon(
                    (|x, z| terrain.sample(x, z), |_, _, s| s.surface), backdrop, camera, &sky,
                    &mut before, &mut work, Some((&mut depth, ScreenRect::full(SIZE))),
                    SIZE, |_| {}, |_| None,
                ).unwrap();
                let mut after = before;
                roads.draw(&mut after, &depth, &mut work, SIZE);
                if after == before {
                    return false;
                }
                assert!(after.iter().zip(before).zip(depth)
                    .all(|((&after, before), depth)| after == before || depth != u16::MAX));
                let mut behind_foreground = before;
                roads.draw(&mut behind_foreground, &[encoded_depth(1800.0); AREA], &mut work, SIZE);
                assert_eq!(behind_foreground, before);
                true
            });
            assert!(evidence, "no distant road pixels drawn at altitude {altitude}");
        }
    }
}
