//! Subpixel pavement along surveyed roads beyond the close ribbon pass.

use crate::highway_stream::HighwayStream;
use crate::land_stream::LandStream;
use crate::math::smoothstep;
use crate::render::{encoded_depth, view_distance, Camera, ScreenProjection, ScreenRect};
use crate::roads::{RoadPoint, ROAD_HALF_WIDTH};

const STROKES: usize = 112;
const PATHS: usize = 24;
const REVEAL_TICKS: u64 = 36;

#[derive(Clone, Copy)]
struct Stroke {
    from: [f32; 3],
    to: [f32; 3],
    rect: ScreenRect,
    half_width: f32,
    alpha: f32,
}

impl Stroke {
    const EMPTY: Self = Self {
        from: [0.0; 3],
        to: [0.0; 3],
        rect: ScreenRect { left: 0, top: 0, right: 0, bottom: 0 },
        half_width: 0.0,
        alpha: 0.0,
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
    region: Option<ScreenRect>,
    known: [Option<RoadSeen>; PATHS],
    tick: u64,
}

impl DistantRoads {
    pub const fn new() -> Self {
        Self {
            strokes: [Stroke::EMPTY; STROKES],
            count: 0,
            region: None,
            known: [None; PATHS],
            tick: 0,
        }
    }

    pub fn reset(&mut self) {
        self.count = 0;
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
    ) {
        self.count = 0;
        self.region = None;
        if ticks < self.tick {
            self.reset();
        }
        self.tick = ticks;
        let Some(view) = RoadView::new(camera, dimensions) else { return; };

        for (_, link) in highways.links() {
            self.add(link.polyline(), &view);
        }
        for (_, link) in highways.coastal_links() {
            for approach in link.approaches() {
                self.add(approach, &view);
            }
        }
        for (key, district) in land.districts() {
            if land.district_published(key) {
                if let Some(road) = district.road() {
                    self.add(road.polyline(), &view);
                }
            }
        }
    }

    pub fn bounds(&self) -> Option<ScreenRect> { self.region }

    pub fn len(&self) -> usize { self.count }

    fn add(&mut self, points: &[RoadPoint], view: &RoadView) {
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
        for pair in points.windows(2) {
            if self.count == STROKES {
                break;
            }
            let Some(stroke) = view.stroke(pair[0], pair[1], reveal) else { continue; };
            self.region = Some(self.region.map_or(stroke.rect, |rect| rect.union(stroke.rect)));
            self.strokes[self.count] = stroke;
            self.count += 1;
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
                    let road_depth = encoded_depth(1.0 / (inv_a + inv_delta * t));
                    let index = y * width + x;
                    let ground_depth = depth[index];
                    let tolerance = road_depth / 32 + 30;
                    if ground_depth != u16::MAX && road_depth.abs_diff(ground_depth) <= tolerance {
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
    reach: f32,
}

impl RoadView {
    fn new(camera: Camera, dimensions: (usize, usize)) -> Option<Self> {
        let (width, height) = dimensions;
        if width > 512 || height > 512 { return None; }
        Some(Self {
            projection: ScreenProjection::new(camera, dimensions)?,
            camera,
            dimensions,
            focal: width.min(height) as f32 * 0.73,
            reach: view_distance(camera.altitude) * 0.91,
        })
    }

    fn stroke(&self, a: RoadPoint, b: RoadPoint, reveal: f32) -> Option<Stroke> {
        let center = [(a.x + b.x) * 0.5, (a.elevation + b.elevation) * 0.5, (a.z + b.z) * 0.5];
        let dx = center[0] - self.camera.x;
        let dz = center[2] - self.camera.z;
        let distance_2d = dx * dx + dz * dz;
        if distance_2d > self.reach * self.reach || center[1] <= 0.0 { return None; }
        let dy = center[1] - self.camera.altitude;
        let distance_3d = distance_2d + dy * dy;
        let near = (self.focal * 2.17).min(self.reach * 0.55);
        let near_end = (self.focal * 3.13).min(self.reach * 0.73);
        let far_start = self.reach * 0.78;
        let enter = smoothstep((distance_3d - near * near) / (near_end * near_end - near * near));
        let leave = smoothstep((self.reach * self.reach - distance_3d) / (self.reach * self.reach - far_start * far_start));
        let alpha = reveal * enter * leave;
        if alpha < 0.075 { return None; }
        let from = self.projection.point([a.x, a.elevation + 0.16, a.z])?;
        let to = self.projection.point([b.x, b.elevation + 0.16, b.z])?;
        let direction = [to[0] - from[0], to[1] - from[1]];
        let length_sq = direction[0] * direction[0] + direction[1] * direction[1];
        if length_sq < 0.02 || length_sq > 3600.0 { return None; }
        let along = [b.x - a.x, b.z - a.z];
        let width_sq = along[0] * along[0] + along[1] * along[1];
        if width_sq < 0.01 { return None; }
        let cross = ROAD_HALF_WIDTH / root(width_sq);
        let mid = self.projection.point(center)?;
        let outer = self.projection.point([center[0] - along[1] * cross, center[1], center[2] + along[0] * cross])?;
        let sideways = (direction[0] * (outer[1] - mid[1]) - direction[1] * (outer[0] - mid[0])).abs();
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
        Some(Stroke { from, to, rect, half_width, alpha })
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
            roads.add(path, &view);
            assert!(roads.bounds().is_none());
            roads.tick = 18;
            roads.add(path, &view);
            let half = roads.strokes[..roads.count].to_vec();
            roads.region = None;
            roads.count = 0;
            roads.tick = REVEAL_TICKS;
            roads.add(path, &view);
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
        assert!(near.stroke(mid, path[path.len() / 2 + 1], 1.0).is_none());
    }
}
