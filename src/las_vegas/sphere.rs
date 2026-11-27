//! Original LED programs for the real Exosphere.

use crate::flight::TICKS_PER_SECOND;
use crate::math::{sin_cos, smoothstep};
use crate::render::{ScreenProjection, ScreenRect};
use crate::world_mesh::WorldMesh;

pub(super) const CENTER: [f32; 2] = [982.0, 1045.0];
pub(super) const HEIGHT: f32 = 111.56;
pub(super) const RADIUS: f32 = 78.64;
pub(super) const REACH: f32 = 2_600.0;

pub(super) const SHOW_TICKS: u64 = 16 * TICKS_PER_SECOND as u64;
const TRANSITION_TICKS: u64 = 2 * TICKS_PER_SECOND as u64;
pub(super) const SHOWS: u64 = 10;

pub(super) struct Sphere {
    ground: f32,
    bounds: ScreenRect,
}

impl Sphere {
    pub(super) fn at(ground: f32, projection: &ScreenProjection) -> Option<Self> {
        let [x, z] = CENTER;
        let bounds = projection.bounds([
            [x - RADIUS, ground, z - RADIUS],
            [x + RADIUS, ground + HEIGHT, z + RADIUS],
        ])?;
        Some(Self { ground, bounds })
    }

    pub(super) fn bounds(&self) -> ScreenRect { self.bounds }

    pub(super) fn draw(&self, mesh: &mut WorldMesh<'_>, ticks: u64) {
        let show = ticks / SHOW_TICKS % SHOWS;
        let local = ticks % SHOW_TICKS;
        let transition = smoothstep(
            local.saturating_sub(SHOW_TICKS - TRANSITION_TICKS) as f32 / TRANSITION_TICKS as f32,
        );
        let time = (ticks % (SHOW_TICKS * SHOWS)) as f32 / TICKS_PER_SECOND as f32;
        let [x, z] = CENTER;
        mesh.with_landmark_reach(REACH, |mesh| {
            mesh.luminous_sphere(
                [x, self.ground + HEIGHT - RADIUS, z],
                RADIUS, self.ground, self.bounds,
                |normal| {
                    let color = program(show, time, normal);
                    if transition == 0.0 { return color; }
                    mix(color, program((show + 1) % SHOWS, time, normal), transition)
                },
            );
        });
    }
}

fn program(show: u64, time: f32, [x, y, z]: [f32; 3]) -> [u8; 3] {
    match show {
        0 => emoji(time, [x, y, z]),
        1 => {
            let band = sin_cos(y * 12.0 - x * 3.0 + z * 5.0 - time * 2.0).0;
            let sheen = sin_cos(y * 23.0 + x * 2.0 + z * 6.0 - time * 1.5).0;
            let wave = smoothstep((band + 0.10) / 1.10);
            let color = mix([16, 29, 97], [23, 201, 196], wave);
            mix(color, [179, 246, 231], 0.50 * smoothstep((sheen - 0.55) / 0.45))
        }
        2 => {
            let (sin, cos) = sin_cos(time * 0.15);
            let light = smoothstep((x * sin - z * cos + 0.20) / 0.53);
            let mut rock = [169, 173, 185];
            for (centre, radius) in [
                ([0.42, 0.43, -0.80], 0.21), ([-0.48, -0.06, -0.87], 0.15),
                ([0.68, -0.25, -0.69], 0.12), ([-0.21, 0.63, 0.74], 0.18),
                ([-0.72, 0.02, 0.68], 0.21), ([0.69, 0.28, 0.66], 0.15),
            ] {
                let dx = x - centre[0];
                let dy = y - centre[1];
                let dz = z - centre[2];
                let r2 = dx * dx + dy * dy + dz * dz;
                rock = mix(rock, [91, 101, 121], 0.75 * smoothstep((radius * radius - r2) / 0.032));
            }
            mix([26, 32, 58], rock, light)
        }
        3 => {
            let twist = sin_cos(x * 4.0 + z * 5.0 - time * 0.86).0 * 0.42;
            let ribbons = sin_cos(y * 18.0 + twist * 7.0 - time * 1.35).0;
            let color = mix([84, 22, 106], [248, 66, 91], smoothstep((y + 0.36) / 1.2));
            mix(color, [255, 204, 119], 0.93 * smoothstep((ribbons - 0.09) / 0.91))
        }
        4 => {
            let facing = (x * -0.78 + z * -0.63).abs();
            let lateral = x * -0.63 + z * 0.78;
            let centre = 0.25 * sin_cos(time * 0.34).0;
            let radius = (lateral - centre) * (lateral - centre) + y * y;
            let corona = smoothstep((radius - 0.23) * 30.0)
                * smoothstep((0.58 - radius) * 3.5) * smoothstep((facing - 0.15) * 2.5);
            let pulse = 0.68 + 0.32 * sin_cos(lateral * 24.0 - time * 2.3).0;
            mix([10, 13, 39], [255, 157, 72], corona * pulse)
        }
        5 => earth(time, [x, y, z]),
        6 => {
            let lane = sin_cos((x + z) * 8.0 - y * 12.0).0;
            let cycle = (y * 1.4 - (x - z) * 0.9 + time * 0.95) % 2.4;
            let phase = (cycle + 2.4) % 2.4;
            let trail = smoothstep((lane - 0.82) * 5.5) * smoothstep((0.68 - phase) * 1.65);
            let core = smoothstep((0.075 - phase) * 13.0);
            let color = mix([18, 14, 68], [115, 61, 255], trail);
            mix(color, [238, 243, 255], core * trail)
        }
        7 => {
            let fold = sin_cos(x * 6.0 + z * 8.0 + time * 0.87).0;
            let curtain = sin_cos(y * 13.0 + fold * 2.8 - time * 0.62).0;
            let band = smoothstep((curtain + 0.18) * 0.85) * smoothstep((y + 0.55) * 1.25);
            let tint = smoothstep((fold + 0.55) * 0.75);
            let glow = mix([89, 91, 229], [94, 246, 143], tint);
            mix([7, 22, 52], glow, band)
        }
        8 => {
            let mut color = [36, 9, 60];
            for (index, tint) in [[255, 64, 157], [66, 219, 255], [251, 226, 82]].into_iter().enumerate() {
                let phase = index as f32 * 2.1 + time * (0.98 + index as f32 * 0.14);
                let (s, c) = sin_cos(phase);
                let axis = if index == 1 { z } else { x };
                let ring = (y * c + axis * s - 0.28 * sin_cos(phase * 1.4).0).abs();
                color = mix(color, tint, smoothstep((0.14 - ring) * 8.3));
            }
            color
        }
        _ => eye(time, [x, y, z]),
    }
}

fn earth(time: f32, [x, y, z]: [f32; 3]) -> [u8; 3] {
    let (sin, cos) = sin_cos(time * 0.15);
    let east = x * cos - z * sin;
    let west = x * sin + z * cos;
    let islands = sin_cos(east * 22.0 + y * 12.0 + west * 5.0).0 * 0.065;
    let mut land = 0.0f32;
    for (lat, lon, size) in [
        (0.54, 0.97, 0.62), (0.04, 1.33, 0.27), (-0.43, 1.23, 0.33),
        (0.51, -1.22, 0.54), (0.17, -1.37, 0.45), (0.06, -2.08, 0.39),
        (-0.52, -2.29, 0.31), (0.79, 0.13, 0.31),
    ] {
        let (s_lat, c_lat) = sin_cos(lat);
        let (s_lon, c_lon) = sin_cos(lon);
        let distance = 1.0 - (east * c_lat * c_lon + y * s_lat + west * c_lat * s_lon);
        land = land.max(smoothstep((size * size * 0.5 + islands - distance) * 17.0));
    }
    let land_color = mix([196, 184, 99], [70, 162, 104], y.abs());
    let ocean = mix([28, 96, 204], land_color, land);
    let snow = smoothstep((y.abs() - 0.81) * 6.5);
    let vapor = sin_cos(y * 23.0 + east * 6.0 + west * 5.0 + time * 0.18).0;
    mix(ocean, [235, 248, 255], snow.max(smoothstep((vapor - 0.76) * 3.8) * 0.54))
}

fn eye(time: f32, [x, y, z]: [f32; 3]) -> [u8; 3] {
    let forward = x * -0.78 - z * 0.63;
    let lateral = (x * -0.63 + z * 0.78) * if forward < 0.0 { -1.0 } else { 1.0 };
    let glance = 0.16 * sin_cos(time * 0.91).0;
    let blink = smoothstep((0.20 - (time % 3.4 - 2.10).abs()) * 6.0);
    let open = ellipse(lateral, y, 0.77, 0.45 - 0.41 * blink)
        * smoothstep((forward.abs() - 0.34) * 3.5);
    let iris = ellipse(lateral - glance, y, 0.28, 0.26);
    let pupil = ellipse(lateral - glance, y, 0.13, 0.13);
    let specular = ellipse(lateral - glance + 0.06, y - 0.08, 0.048, 0.042);
    let white = mix([231, 240, 250], [41, 172, 201], iris);
    let lens = mix(mix(white, [8, 19, 29], pupil), [255, 254, 242], specular);
    mix([38, 17, 62], lens, open)
}

fn emoji(time: f32, [x, y, z]: [f32; 3]) -> [u8; 3] {
    let turn = -2.25 + 0.16 * sin_cos(time * 0.28).0;
    let (sin, cos) = sin_cos(turn);
    let forward = x * sin + z * cos;
    let horizontal = (x * cos - z * sin) * if forward < 0.0 { -1.0 } else { 1.0 };
    let tint = smoothstep((y + 0.38) * 1.12);
    let gold = mix([233, 126, 23], [255, 221, 75], tint);
    let face = smoothstep((forward.abs() - 0.38) / 0.19);
    let glance = 0.085 * sin_cos(time * 1.3).0;
    let blink_phase = (time % 2.8 - 1.48).abs();
    let blink = smoothstep((0.19 - blink_phase) / 0.117);
    let mut color = gold;
    for eye_x in [-0.31, 0.31] {
        let dx = horizontal - eye_x;
        let eyelid = 0.225 - 0.202 * blink;
        let eye = ellipse(dx, y - 0.25, 0.151, eyelid);
        let pupil = ellipse(dx - glance, y - 0.245 - 0.026 * sin_cos(time * 0.82).0, 0.083, eyelid * 0.79);
        let glint = ellipse(dx - glance + 0.025, y - 0.305, 0.032, 0.040) * (1.0 - blink);
        color = mix(color, [255, 251, 214], eye * face);
        color = mix(color, [49, 33, 32], pupil * eye * face);
        color = mix(color, [255, 255, 242], glint * eye * face);
        let cheek = ellipse(horizontal - eye_x * 1.55, y + 0.095, 0.177, 0.091);
        color = mix(color, [236, 88, 32], cheek * face * 0.55);
    }
    let smile_y = -0.264 + 0.70 * horizontal * horizontal
        + 0.018 * sin_cos(time * 1.2).0;
    let smile = smoothstep((0.306 - horizontal.abs()) / 0.08)
        * smoothstep((0.063 - (y - smile_y).abs()) / 0.046);
    mix(color, [85, 42, 38], smile * face)
}

fn ellipse(x: f32, y: f32, width: f32, height: f32) -> f32 {
    let nx = x / width;
    let ny = y / height;
    smoothstep((1.14 - nx * nx - ny * ny) * 3.3)
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    core::array::from_fn(|index| {
        (f32::from(a[index]) + (f32::from(b[index]) - f32::from(a[index])) * t) as u8
    })
}
