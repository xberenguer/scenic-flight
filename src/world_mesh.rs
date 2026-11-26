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
    haze: f32,
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
        Some(Self {
            pixels, depth, width, height, camera,
            heading: sin_cos(camera.heading),
            pitch: sin_cos(camera.pitch.clamp(-1.3, 1.3)),
            roll: sin_cos(-camera.bank),
            focal: width.min(height) as f32 * 0.73,
            far,
            clip_far: far * 1.25,
            curvature: (far - 512.0) * (0.000040 / 384.0),
            daylight: sky.daylight.clamp(0.0, 1.0),
            haze: sky.haze.clamp(0.0, 1.0),
            sun: sky.sun_direction,
        })
    }

    pub fn with_landmark_reach(&mut self, reach: f32, draw: impl FnOnce(&mut Self)) {
        let original = self.clip_far;
        self.clip_far = original.max(reach);
        draw(self);
        self.clip_far = original;
    }

    /// Positions are [east, height, north]. Winding may be either direction.
    pub fn triangle(&mut self, points: [[f32; 3]; 3], color: [u8; 3]) {
        self.faded_triangle(points, color, 1.0);
    }

    pub fn faded_triangle(&mut self, points: [[f32; 3]; 3], color: [u8; 3], opacity: f32) {
        self.raster_triangle(points, color, opacity, 0.0);
    }

    pub fn luminous_triangle(&mut self, points: [[f32; 3]; 3], color: [u8; 3], emission: f32) {
        self.raster_triangle(points, color, 1.0, emission.clamp(0.0, 1.0));
    }

    fn raster_triangle(&mut self, points: [[f32; 3]; 3], color: [u8; 3], opacity: f32, emission: f32) {
        let camera_points = points.map(|point| self.camera_point(point));
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
        let middle_depth = camera_points.iter().map(|point| point[2]).sum::<f32>() / 3.0;
        let fade_start = self.far * 1.25;
        let fade = if self.clip_far > fade_start && middle_depth > fade_start {
            smoothstep((self.clip_far - middle_depth) / (self.clip_far - fade_start))
        } else {
            1.0
        };
        let alpha = ((opacity * fade).clamp(0.0, 1.0) * 32.0 + 0.5) as u32;
        if alpha == 0 { return; }
        let color = self.shade(color, middle_depth, emission);
        let first = self.project(clipped[0]);
        for index in 1..count - 1 {
            self.paint([first, self.project(clipped[index]), self.project(clipped[index + 1])], color, alpha);
        }
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
        let fog = (d * (0.42 + self.haze * 0.64) + d * d * 0.38).min(0.97);
        let ambient = (0.28 + self.daylight * 0.72).max(emission);
        let haze_color = [24.0, 38.0, 63.0];
        let day_color = [175.0, 207.0, 232.0];
        let shaded: [f32; 3] = core::array::from_fn(|channel| {
            let horizon = haze_color[channel] + (day_color[channel] - haze_color[channel]) * self.daylight;
            f32::from(rgb[channel]) * ambient * (1.0 - fog) + horizon * fog
        });
        let r = (shaded[0] * (31.0 / 255.0) + 0.5) as u16;
        let g = (shaded[1] * (63.0 / 255.0) + 0.5) as u16;
        let b = (shaded[2] * (31.0 / 255.0) + 0.5) as u16;
        (r << 11) | (g << 5) | b
    }

    fn paint(&mut self, [a, b, c]: [Vertex; 3], color: u16, alpha: u32) {
        let area = orient(a, b, c.x, c.y);
        if area.abs() < 0.01 {
            return;
        }
        let inverse_area = 1.0 / area;
        let weight_x = [(b.y - c.y) * inverse_area, (c.y - a.y) * inverse_area];
        let min_x = a.x.min(b.x).min(c.x).max(0.0) as usize;
        let min_y = a.y.min(b.y).min(c.y).max(0.0) as usize;
        let end_x = ((a.x.max(b.x).max(c.x).max(0.0) as usize).saturating_add(1)).min(self.width);
        let end_y = ((a.y.max(b.y).max(c.y).max(0.0) as usize).saturating_add(1)).min(self.height);
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
