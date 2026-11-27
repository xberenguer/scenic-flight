//! Depth-tested scenery in the same world-space camera as the terrain.

use crate::math::{sin_cos, smoothstep};
use crate::render::{encoded_depth, horizon_distance, view_distance, Camera};
use crate::sky::Sky;

const NEAR: f32 = 0.8;

pub fn bounds_in_view(camera: Camera, bounds: [f32; 4]) -> bool {
    bounds_in_range(camera, bounds, view_distance(camera.altitude))
}

pub fn landmark_reach(altitude: f32) -> f32 {
    horizon_distance(altitude)
}

pub fn bounds_in_range(camera: Camera, bounds: [f32; 4], reach: f32) -> bool {
    let [min_x, min_z, max_x, max_z] = bounds;
    let dx = (min_x + max_x) * 0.5 - camera.x;
    let dz = (min_z + max_z) * 0.5 - camera.z;
    let (sin, cos) = sin_cos(camera.heading);
    let along = dx * sin + dz * cos;
    let across = (dx * cos - dz * sin).abs();
    let margin = (max_x - min_x + max_z - min_z) * 0.5;
    let near_margin = 12.0 + camera.altitude.abs() * 0.27;
    along + margin > -near_margin && along - margin <= reach
        && across - margin < (along + margin).max(0.0) * 0.84 + near_margin
}

#[derive(Clone, Copy)]
struct Vertex {
    x: f32,
    y: f32,
    inverse_depth: f32,
}

struct TriangleStyle {
    ink: [u8; 3],
    distance: f32,
    emission: f32,
    alpha: u32,
    shaded: Option<u16>,
}

pub struct WorldMesh<'a> {
    pixels: &'a mut [u16],
    depth: &'a mut [u16],
    width: usize,
    height: usize,
    camera: Camera,
    heading: (f32, f32),
    pitch: (f32, f32),
    roll: (f32, f32),
    focal: f32,
    far: f32,
    clip_far: f32,
    curvature: f32,
    daylight: f32,
    ambient: f32,
    fog_strength: f32,
    horizon: [f32; 3],
    sun: [f32; 3],
}

impl<'a> WorldMesh<'a> {
    pub fn new(
        pixels: &'a mut [u16], depth: &'a mut [u16], dimensions: (usize, usize),
        camera: Camera, sky: &Sky,
    ) -> Option<Self> {
        let (width, height) = dimensions;
        let count = width.checked_mul(height)?;
        if width == 0 || height == 0 || width > 512 || height > 512
            || pixels.len() < count || depth.len() < count
        {
            return None;
        }
        let far = view_distance(camera.altitude);
        let daylight = sky.daylight.clamp(0.0, 1.0);
        let haze_color = [24.0, 38.0, 63.0];
        let day_color = [175.0, 207.0, 232.0];
        Some(Self {
            pixels, depth, width, height, camera,
            heading: sin_cos(camera.heading),
            pitch: sin_cos(camera.pitch.clamp(-1.3, 1.3)),
            roll: sin_cos(-camera.bank),
            focal: width.min(height) as f32 * 0.73,
            far,
            clip_far: far * 1.25,
            curvature: (far - 512.0) * (0.000040 / 384.0),
            daylight,
            ambient: 0.28 + daylight * 0.72,
            fog_strength: 0.42 + sky.haze.clamp(0.0, 1.0) * 0.64,
            horizon: core::array::from_fn(|channel| {
                haze_color[channel] + (day_color[channel] - haze_color[channel]) * daylight
            }),
            sun: sky.sun_direction,
        })
    }

    pub fn with_landmark_reach(&mut self, reach: f32, draw: impl FnOnce(&mut Self)) {
        let original = self.clip_far;
        self.clip_far = original.max(reach);
        draw(self);
        self.clip_far = original;
    }

    pub(crate) fn with_origin(&mut self, [east, up, north]: [f32; 3], draw: impl FnOnce(&mut Self)) {
        let camera = self.camera;
        self.camera.x -= east;
        self.camera.altitude -= up;
        self.camera.z -= north;
        draw(self);
        self.camera = camera;
    }

    pub(crate) fn daylight(&self) -> f32 { self.daylight }

    /// Positions are [east, height, north]. Winding may be either direction.
    pub fn triangle(&mut self, points: [[f32; 3]; 3], color: [u8; 3]) {
        self.faded_triangle(points, color, 1.0);
    }

    pub fn faded_triangle(&mut self, points: [[f32; 3]; 3], color: [u8; 3], opacity: f32) {
        self.raster_triangle(points, color, opacity, 0.0);
    }

    /// Draw the (0, 2) diagonal in the same order and with the same shading
    /// as two triangles, reusing the shared projections for an unclipped face.
    pub fn faded_quad(&mut self, points: [[f32; 3]; 4], color: [u8; 3], opacity: f32) {
        let [a, b, c, d] = points.map(|point| self.camera_point(point));
        if [a, b, c, d].iter().all(|point| point[2] >= NEAR && point.iter().all(|v| v.is_finite())) {
            let [va, vb, vc, vd] = [a, b, c, d].map(|point| self.project(point));
            for ([v0, v1, v2], depths) in [
                ([va, vb, vc], [a[2], b[2], c[2]]),
                ([va, vc, vd], [a[2], c[2], d[2]]),
            ] {
                if depths.iter().all(|&distance| distance > self.clip_far) {
                    continue;
                }
                if let Some(mut style) = self.triangle_style(depths, color, opacity, 0.0) {
                    self.paint([v0, v1, v2], &mut style);
                }
            }
        } else {
            self.raster_camera_triangle([a, b, c], color, opacity, 0.0);
            self.raster_camera_triangle([a, c, d], color, opacity, 0.0);
        }
    }

    pub fn luminous_triangle(&mut self, points: [[f32; 3]; 3], color: [u8; 3], emission: f32) {
        self.raster_triangle(points, color, 1.0, emission.clamp(0.0, 1.0));
    }

    /// Shade the visible cap of a sphere in world space with terrain depth.
    pub(crate) fn luminous_sphere(
        &mut self, mut center: [f32; 3], radius: f32, ground: f32,
        region: crate::render::ScreenRect, ink: impl Fn([f32; 3]) -> [u8; 3],
    ) {
        let dx = center[0] - self.camera.x;
        let dz = center[2] - self.camera.z;
        let fall = self.curvature * (dx * dx + dz * dz);
        center[1] -= fall;
        let cutoff = (ground - fall - center[1]) / radius;
        let origin = [
            -dx / radius, (self.camera.altitude - center[1]) / radius, -dz / radius,
        ];
        let c = dot(origin, origin) - 1.0;
        let (hs, hc) = self.heading;
        let (ps, pc) = self.pitch;
        let (rs, rc) = self.roll;
        let cx = (self.width - 1) as f32 * 0.5;
        let cy = (self.height - 1) as f32 * 0.5;
        let aperture = self.width.min(self.height) as i32;
        for y in region.top.min(self.height)..region.bottom.min(self.height) {
            let screen_y = (y as f32 - cy) / self.focal;
            let dy = 2 * y as i32 + 1 - self.height as i32;
            for x in region.left.min(self.width)..region.right.min(self.width) {
                let dx = 2 * x as i32 + 1 - self.width as i32;
                if dx * dx + dy * dy > aperture * aperture { continue; }
                let screen_x = (x as f32 - cx) / self.focal;
                let right = screen_x * rc + screen_y * rs;
                let down = screen_y * rc - screen_x * rs;
                let forward = pc - ps * down;
                let ray = [hc * right + hs * forward, -ps - pc * down, hc * forward - hs * right];
                let a = dot(ray, ray);
                let b = dot(origin, ray);
                let discriminant = b * b - a * c;
                if discriminant <= 0.0 { continue; }
                let root = positive_sqrt(discriminant);
                let near = (-b - root) / a;
                let far = (-b + root) / a;
                let distance = if near * radius >= NEAR { near } else { far };
                let depth = distance * radius;
                if !(NEAR..self.clip_far).contains(&depth) { continue; }
                let normal = core::array::from_fn(|axis| origin[axis] + ray[axis] * distance);
                if normal[1] < cutoff { continue; }
                let index = y * self.width + x;
                let encoded = encoded_depth(depth);
                if encoded < self.depth[index] {
                    self.depth[index] = encoded;
                    self.pixels[index] = self.shade(ink(normal), depth, 0.98).max(1);
                }
            }
        }
    }

    fn raster_triangle(&mut self, points: [[f32; 3]; 3], color: [u8; 3], opacity: f32, emission: f32) {
        let camera_points = points.map(|point| self.camera_point(point));
        self.raster_camera_triangle(camera_points, color, opacity, emission);
    }

    fn raster_camera_triangle(&mut self, camera_points: [[f32; 3]; 3], color: [u8; 3], opacity: f32, emission: f32) {
        if !camera_points.iter().flatten().all(|value| value.is_finite())
            || camera_points.iter().all(|point| point[2] > self.clip_far)
        {
            return;
        }

        // Clip in camera coordinates so a bridge or arch stays intact on approach.
        let mut clipped = [[0.0; 3]; 4];
        let mut count = 0;
        let mut previous = camera_points[2];
        for point in camera_points {
            let inside = point[2] >= NEAR;
            if inside != (previous[2] >= NEAR) {
                let t = (NEAR - previous[2]) / (point[2] - previous[2]);
                clipped[count] = [
                    previous[0] + (point[0] - previous[0]) * t,
                    previous[1] + (point[1] - previous[1]) * t,
                    NEAR,
                ];
                count += 1;
            }
            if inside {
                clipped[count] = point;
                count += 1;
            }
            previous = point;
        }
        if count < 3 {
            return;
        }
        let Some(mut style) = self.triangle_style(camera_points.map(|point| point[2]), color, opacity, emission) else {
            return;
        };
        let first = self.project(clipped[0]);
        for index in 1..count - 1 {
            self.paint([first, self.project(clipped[index]), self.project(clipped[index + 1])], &mut style);
        }
    }

    fn triangle_style(&self, depths: [f32; 3], color: [u8; 3], opacity: f32, emission: f32) -> Option<TriangleStyle> {
        let middle_depth = depths.into_iter().sum::<f32>() / 3.0;
        let fade_start = self.far * 1.25;
        let fade = if self.clip_far > fade_start && middle_depth > fade_start {
            smoothstep((self.clip_far - middle_depth) / (self.clip_far - fade_start))
        } else {
            1.0
        };
        let alpha = ((opacity * fade).clamp(0.0, 1.0) * 32.0 + 0.5) as u32;
        (alpha != 0).then_some(TriangleStyle {
            ink: color,
            distance: middle_depth,
            emission,
            alpha,
            shaded: None,
        })
    }

    pub fn lit_triangle(&mut self, points: [[f32; 3]; 3], color: [u8; 3]) {
        let [a, b, c] = points;
        let ab: [f32; 3] = core::array::from_fn(|axis| b[axis] - a[axis]);
        let ac: [f32; 3] = core::array::from_fn(|axis| c[axis] - a[axis]);
        let normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let length = normal[0].abs() + normal[1].abs() + normal[2].abs();
        if length < 0.001 {
            return;
        }
        let incident = (normal[0] * self.sun[0] + normal[1] * self.sun[1] + normal[2] * self.sun[2]) / length;
        let light = 0.79 + self.daylight * (0.16 + incident.clamp(-1.0, 1.0) * 0.22);
        self.triangle(points, color.map(|channel| (f32::from(channel) * light + 0.5) as u8));
    }

    /// A suspended cable has subpixel coverage when viewed at a distance.
    pub fn wire(&mut self, from: [f32; 3], to: [f32; 3], diameter: f32, color: [u8; 3]) {
        let a = self.camera_point(from);
        let b = self.camera_point(to);
        if a[2] < NEAR || b[2] < NEAR || !a.iter().chain(&b).all(|value| value.is_finite()) {
            return;
        }
        let a = self.project(a);
        let b = self.project(b);
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let extent = dx.abs().max(dy.abs());
        if extent < 1.0 || extent > (self.width + self.height) as f32 * 2.0 {
            return;
        }
        let steps = extent as usize + 1;
        let diameter_screen = (diameter * self.focal * (a.inverse_depth + b.inverse_depth) * 0.5).clamp(0.24, 1.0);
        let alpha = (diameter_screen * 32.0) as u32;
        let d = self.width.min(self.height) as i32;
        let fill = self.shade(color, 2.0 / (a.inverse_depth + b.inverse_depth), 0.0);
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let x = (a.x + t * dx + 0.5) as i32;
            let y = (a.y + t * dy + 0.5) as i32;
            if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                continue;
            }
            let sx = 2 * x + 1 - self.width as i32;
            let sy = 2 * y + 1 - self.height as i32;
            if sx * sx + sy * sy > d * d {
                continue;
            }
            let index = y as usize * self.width + x as usize;
            let depth = encoded_depth(1.0 / (a.inverse_depth + t * (b.inverse_depth - a.inverse_depth)));
            if depth < self.depth[index] {
                self.pixels[index] = blend(self.pixels[index], fill, alpha);
                self.depth[index] = depth;
            }
        }
    }

    /// Draw small distant geometry with fractional pixel coverage and terrain depth.
    pub(crate) fn faded_segment(&mut self, points: [[f32; 3]; 2], diameter: f32, color: [u8; 3], opacity: f32) {
        self.segment(points, diameter, color, opacity, 0.0);
    }

    pub(crate) fn luminous_segment(&mut self, points: [[f32; 3]; 2], diameter: f32, color: [u8; 3]) {
        self.segment(points, diameter, color, 1.0, 0.96);
    }

    fn segment(&mut self, [from, to]: [[f32; 3]; 2], diameter: f32, color: [u8; 3], opacity: f32, emission: f32) {
        let [a, b] = [from, to].map(|point| self.camera_point(point));
        if [a, b].iter().any(|point| point[2] < NEAR || point.iter().any(|v| !v.is_finite())) {
            return;
        }
        let [a, b] = [a, b].map(|point| self.project(point));
        let horizontal = (b.x - a.x).abs() >= (b.y - a.y).abs();
        let axes = |v: Vertex| if horizontal { (v.x, v.y) } else { (v.y, v.x) };
        let (a_major, a_minor) = axes(a);
        let (b_major, b_minor) = axes(b);
        let lower = a_major.min(b_major);
        let upper = a_major.max(b_major);
        let extent = if horizontal { self.width } else { self.height };
        let first = ((lower + 0.5) as usize).min(extent);
        let end = ((upper + 0.5) as usize).saturating_add(1).min(extent);
        if first >= end || upper - lower < 0.001 { return; }
        let size = (diameter * self.focal * (a.inverse_depth + b.inverse_depth) * 0.5).clamp(0.46, 1.0);
        let strength = opacity * size * 32.0;
        if strength * (upper - lower).min(1.0) < 0.499 { return; }
        let fill = self.shade(color, 2.0 / (a.inverse_depth + b.inverse_depth), emission);
        let step = 1.0 / (b_major - a_major);
        let radius = self.width.min(self.height) as i32;
        for major in first..end {
            let at = major as f32;
            let cover = (upper.min(at + 0.5) - lower.max(at - 0.5)).max(0.0);
            let t = ((at - a_major) * step).clamp(0.0, 1.0);
            let minor = a_minor + (b_minor - a_minor) * t;
            let row = crate::render::floor_i32(minor);
            let fraction = minor - row as f32;
            let depth = encoded_depth(1.0 / (a.inverse_depth + (b.inverse_depth - a.inverse_depth) * t));
            for (minor, weight) in [(row, 1.0 - fraction), (row.saturating_add(1), fraction)] {
                let alpha = (weight * cover * strength + 0.5) as u32;
                let (x, y) = if horizontal { (major as i32, minor) } else { (minor, major as i32) };
                if alpha == 0 || x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
                    continue;
                }
                let dx = 2 * x + 1 - self.width as i32;
                let dy = 2 * y + 1 - self.height as i32;
                if dx * dx + dy * dy > radius * radius { continue; }
                let index = y as usize * self.width + x as usize;
                if depth < self.depth[index] {
                    self.depth[index] = depth;
                    self.pixels[index] = blend(self.pixels[index], fill, alpha);
                }
            }
        }
    }

    fn camera_point(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let dx = x - self.camera.x;
        let dz = z - self.camera.z;
        let height = y - self.camera.altitude - self.curvature * (dx * dx + dz * dz);
        let (hs, hc) = self.heading;
        let (ps, pc) = self.pitch;
        let forward = hs * dx + hc * dz;
        [hc * dx - hs * dz, -ps * forward - pc * height, pc * forward - ps * height]
    }

    fn project(&self, [x, y, depth]: [f32; 3]) -> Vertex {
        let inverse_depth = 1.0 / depth;
        let scale = self.focal * inverse_depth;
        let (sin, cos) = self.roll;
        Vertex {
            x: (self.width - 1) as f32 * 0.5 + scale * (x * cos - y * sin),
            y: (self.height - 1) as f32 * 0.5 + scale * (x * sin + y * cos),
            inverse_depth,
        }
    }

    fn shade(&self, rgb: [u8; 3], distance: f32, emission: f32) -> u16 {
        let d = (distance / self.far).clamp(0.0, 1.0);
        let fog = (d * self.fog_strength + d * d * 0.38).min(0.97);
        let ambient = self.ambient.max(emission);
        let shaded: [f32; 3] = core::array::from_fn(|channel| {
            f32::from(rgb[channel]) * ambient * (1.0 - fog) + self.horizon[channel] * fog
        });
        let r = (shaded[0] * (31.0 / 255.0) + 0.5) as u16;
        let g = (shaded[1] * (63.0 / 255.0) + 0.5) as u16;
        let b = (shaded[2] * (31.0 / 255.0) + 0.5) as u16;
        (r << 11) | (g << 5) | b
    }

    fn paint(&mut self, [a, b, c]: [Vertex; 3], style: &mut TriangleStyle) {
        let TriangleStyle { ink, distance, emission, alpha, shaded } = style;
        let alpha = *alpha;
        let min_x = a.x.min(b.x).min(c.x).max(0.0) as usize;
        let min_y = a.y.min(b.y).min(c.y).max(0.0) as usize;
        let end_x = ((a.x.max(b.x).max(c.x).max(0.0) as usize).saturating_add(1)).min(self.width);
        let end_y = ((a.y.max(b.y).max(c.y).max(0.0) as usize).saturating_add(1)).min(self.height);
        if min_x >= end_x || min_y >= end_y {
            return;
        }
        let area = orient(a, b, c.x, c.y);
        if area.abs() < 0.01 {
            return;
        }
        let inverse_area = 1.0 / area;
        let weight_x = [(b.y - c.y) * inverse_area, (c.y - a.y) * inverse_area];
        let diameter = self.width.min(self.height) as i32;
        for y in min_y..end_y {
            let dy = y as i32 * 2 + 1 - self.height as i32;
            let mut wa = orient(b, c, min_x as f32, y as f32) * inverse_area;
            let mut wb = orient(c, a, min_x as f32, y as f32) * inverse_area;
            for x in min_x..end_x {
                let wc = 1.0 - wa - wb;
                let dx = x as i32 * 2 + 1 - self.width as i32;
                if wa >= -0.00001 && wb >= -0.00001 && wc >= -0.00001
                    && dx * dx + dy * dy <= diameter * diameter
                {
                    let reciprocal = a.inverse_depth * wa + b.inverse_depth * wb + c.inverse_depth * wc;
                    let depth = encoded_depth(1.0 / reciprocal);
                    let index = y * self.width + x;
                    if depth < self.depth[index] || (alpha == 32 && depth == self.depth[index]) {
                        // Shade only a face that reaches a pixel. The clipped fan shares the result.
                        let color = *shaded.get_or_insert_with(|| self.shade(*ink, *distance, *emission));
                        self.depth[index] = depth;
                        self.pixels[index] = if alpha == 32 { color } else { blend(self.pixels[index], color, alpha) };
                    }
                }
                wa += weight_x[0];
                wb += weight_x[1];
            }
        }
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn positive_sqrt(value: f32) -> f32 {
    let estimate = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    let estimate = 0.5 * (estimate + value / estimate);
    0.5 * (estimate + value / estimate)
}

fn orient(a: Vertex, b: Vertex, x: f32, y: f32) -> f32 {
    (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x)
}

fn blend(bottom: u16, top: u16, alpha: u32) -> u16 {
    let from = u32::from(bottom);
    let to = u32::from(top);
    let back = 32 - alpha;
    let rb = ((from & 0xf81f) * back + (to & 0xf81f) * alpha) >> 5;
    let g = ((from & 0x07e0) * back + (to & 0x07e0) * alpha) >> 5;
    ((rb & 0xf81f) | (g & 0x07e0)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_reuses_projections_without_changing_color_or_depth() {
        const SIDE: usize = 60;
        let sky = Sky::from_time(1, 0, 0.38);
        for bank in [-0.28, 0.0, 0.31] {
            let camera = Camera { x: 0.0, z: 0.0, altitude: 5.0, heading: 0.07, pitch: 0.15, bank };
            for [first, last, span, bottom, top] in [
                [16.0, 40.0, 4.0, 0.0, 8.0],
                [-2.0, 14.0, 4.0, 3.0, 12.0],
                [14.0, 25.0, 70.0, 2.0, 10.0],
                [636.0, 755.0, 450.0, -130.0, 160.0],
                [16.0, 39.0, 6.0, -3.0, f32::NAN],
            ] {
                let points = [
                    [-span, bottom, first], [span, bottom, first],
                    [span, bottom + 7.0, last], [-span, top, last],
                ];
                for opacity in [0.0, 0.38, 1.0] {
                    let mut expected = [0x4242; SIDE * SIDE];
                    let mut actual = expected;
                    let mut expected_depth = core::array::from_fn::<_, { SIDE * SIDE }, _>(|i| {
                        if i % 7 > 2 { u16::MAX } else { encoded_depth(25.0) }
                    });
                    let mut actual_depth = expected_depth;
                    let mut reference = WorldMesh::new(&mut expected, &mut expected_depth, (SIDE, SIDE), camera, &sky).unwrap();
                    let mut quad = WorldMesh::new(&mut actual, &mut actual_depth, (SIDE, SIDE), camera, &sky).unwrap();
                    let color = [180, 122, 82];
                    let [a, b, c, d] = points;
                    reference.with_landmark_reach(790.0, |mesh| {
                        mesh.faded_triangle([a, b, c], color, opacity);
                        mesh.faded_triangle([a, c, d], color, opacity);
                    });
                    quad.with_landmark_reach(790.0, |mesh| mesh.faded_quad(points, color, opacity));
                    assert_eq!(actual, expected);
                    assert_eq!(actual_depth, expected_depth);
                }
            }
        }
    }

    #[test]
    fn distant_landmark_uses_terrain_depth_and_leaves_the_normal_clip_intact() {
        const SIDE: usize = 144;
        let size = (SIDE, SIDE);
        let camera = Camera { x: 0.0, z: 0.0, altitude: 32.0, heading: 0.0, pitch: 0.0, bank: 0.0 };
        let sky = Sky::from_time(1, 0, 0.46);
        let reach = landmark_reach(camera.altitude);
        let triangle = [[-12.0, 23.0, 720.0], [12.0, 23.0, 720.0], [0.0, 49.0, 720.0]];
        let base = [0x394a; SIDE * SIDE];
        let mut pixels = base;
        let mut depth = [u16::MAX; SIDE * SIDE];
        WorldMesh::new(&mut pixels, &mut depth, size, camera, &sky).unwrap()
            .triangle(triangle, [105, 92, 76]);
        assert_eq!(pixels, base);
        WorldMesh::new(&mut pixels, &mut depth, size, camera, &sky).unwrap()
            .with_landmark_reach(reach, |mesh| mesh.triangle(triangle, [105, 92, 76]));
        assert!(pixels.iter().zip(base).filter(|(a, b)| **a != *b).count() > 3);

        let mut pixels = base;
        let mut depth = [encoded_depth(560.0); SIDE * SIDE];
        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, size, camera, &sky).unwrap();
        mesh.with_landmark_reach(reach, |mesh| mesh.triangle(triangle, [105, 92, 76]));
        mesh.triangle(triangle, [105, 92, 76]);
        assert_eq!(pixels, base);
    }
    use crate::render::{render_from_with_sky_buffered, render_terrain_layer_region, render_with_depth, ScreenRect};
    use crate::world::{Sample, Surface};

    #[test]
    fn ground_depth_preserves_the_circular_banked_frame_and_occludes_meshes() {
        const SIDE: usize = 48;
        let mut pixels = [0; SIDE * SIDE];
        let mut expected = pixels;
        let mut work = pixels;
        let mut depth = pixels;
        let camera = Camera { x: 0.0, z: 0.0, altitude: 16.0, heading: 0.0, pitch: 0.2, bank: 0.23 };
        let sky = Sky::from_time(33, 0, 0.5);
        let terrain = |_, _| Sample { elevation: 0.0, surface: Surface::Shallows };
        render_from_with_sky_buffered(terrain, camera, &sky, &mut expected, &mut work, SIDE, SIDE).unwrap();
        render_with_depth(terrain, camera, &sky, &mut pixels, &mut work, &mut depth, (SIDE, SIDE)).unwrap();
        assert_eq!(pixels, expected);
        assert_eq!(depth[0], u16::MAX);
        {
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
            mesh.triangle([[-4.0, -8.0, 42.0], [4.0, -8.0, 42.0], [0.0, -4.0, 42.0]], [240, 12, 12]);
        }
        assert_eq!(pixels, expected);
        {
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
            mesh.triangle([[-4.0, 8.0, 42.0], [4.0, 8.0, 42.0], [0.0, 12.0, 42.0]], [240, 12, 12]);
        }
        assert!(pixels.iter().zip(expected).filter(|(a, b)| **a != *b).count() > 10);
    }

    #[test]
    fn depth_subregion_matches_full_depth_after_camera_roll() {
        const SIDE: usize = 48;
        let mut pixels = [0; SIDE * SIDE];
        let mut work = pixels;
        let mut full = pixels;
        let mut cropped = pixels;
        let rect = ScreenRect { left: 20, top: 23, right: 33, bottom: 44 };
        let sky = Sky::from_time(33, 0, 0.5);
        let terrain = |x: f32, _| Sample { elevation: x.abs() * 0.27, surface: Surface::Grass };
        for bank in [-0.37, 0.0, 0.26] {
            let camera = Camera { x: 0.0, z: 0.0, altitude: 13.0, heading: 0.3, pitch: 0.2, bank };
            render_with_depth(terrain, camera, &sky, &mut pixels, &mut work, &mut full, (SIDE, SIDE)).unwrap();
            let expected = pixels;
            render_terrain_layer_region((terrain, |_, _, ground| ground.surface), camera, &sky,
                &mut pixels, &mut work, Some((&mut cropped, rect)), (SIDE, SIDE), |_| {}).unwrap();
            assert_eq!(pixels, expected);
            for y in 0..SIDE {
                for x in 0..SIDE {
                    let index = y * SIDE + x;
                    let inside = (rect.left..rect.right).contains(&x) && (rect.top..rect.bottom).contains(&y);
                    assert_eq!(cropped[index], if inside { full[index] } else { u16::MAX });
                }
            }
        }
    }
}
