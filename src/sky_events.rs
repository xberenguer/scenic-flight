//! Small world-direction overlays for sky encounters.

use crate::encounters::Meteor;
use crate::math::sin_cos;
use crate::render::Camera;

/// Blend a meteor where the compositor reports visible sky.
/// Visibility is zero behind terrain and may fade under cloud.
pub fn paint_meteor(
    event: Meteor,
    camera: Camera,
    pixels: &mut [u16],
    width: usize,
    height: usize,
    mut sky_visibility: impl FnMut(usize, usize) -> f32,
) {
    if width.checked_mul(height).is_none_or(|area| area > pixels.len()) {
        return;
    }
    streak(event, camera, width, height, |x, y, alpha| {
        let index = y * width + x;
        pixels[index] = blend_meteor(pixels[index], alpha * sky_visibility(x, y));
    });
}

/// Visit the lit pixels of a world-anchored meteor, clipped to the round viewport.
/// The terrain compositor can use this before painting sky to retain exact occlusion.
pub fn streak(
    event: Meteor,
    camera: Camera,
    width: usize,
    height: usize,
    mut pixel: impl FnMut(usize, usize, f32),
) {
    if width == 0 || height == 0 || width > 512 || height > 512 || event.brightness <= 0.0 {
        return;
    }
    let (heading_sin, heading_cos) = sin_cos(camera.heading);
    let (pitch_sin, pitch_cos) = sin_cos(camera.pitch.clamp(-1.3, 1.3));
    let (roll_sin, roll_cos) = sin_cos(-camera.bank);
    let view = |[x, up, z]: [f32; 3]| {
        let forward = x * heading_sin + z * heading_cos;
        [
            x * heading_cos - z * heading_sin,
            -(up * pitch_cos + forward * pitch_sin),
            forward * pitch_cos - up * pitch_sin,
        ]
    };
    let mut tail = view(event.tail);
    let mut head = view(event.head);
    const NEAR: f32 = 0.08;
    if (tail[2] < NEAR && head[2] < NEAR)
        || !tail.iter().chain(&head).all(|value| value.is_finite())
    {
        return;
    }
    let clip_near = |behind: &mut [f32; 3], ahead: [f32; 3]| {
        if behind[2] < NEAR {
            let t = (NEAR - behind[2]) / (ahead[2] - behind[2]);
            behind[0] += (ahead[0] - behind[0]) * t;
            behind[1] += (ahead[1] - behind[1]) * t;
            behind[2] = NEAR;
        }
    };
    clip_near(&mut tail, head);
    clip_near(&mut head, tail);
    let diameter = width.min(height);
    let focal = diameter as f32 * 0.73;
    let project = |[u, v, depth]: [f32; 3]| {
        let scale = focal / depth;
        [
            (width - 1) as f32 * 0.5 + (u * roll_cos - v * roll_sin) * scale,
            (height - 1) as f32 * 0.5 + (u * roll_sin + v * roll_cos) * scale,
        ]
    };
    let [ax, ay] = project(tail);
    let [bx, by] = project(head);
    let dx = bx - ax;
    let dy = by - ay;
    let length_squared = dx * dx + dy * dy;
    if !length_squared.is_finite() || length_squared < 0.001 {
        return;
    }
    let inverse_length = 1.0 / length_squared;
    let use_x = dx.abs() >= dy.abs();
    let (first, last, minor, slope, extent) = if use_x {
        (ax, bx, ay, dy / dx, width)
    } else {
        (ay, by, ax, dx / dy, height)
    };
    let start = ((first.min(last) - 1.5) as i32).max(0);
    let end = ((first.max(last) + 1.5) as i32).min(extent as i32 - 1);
    let radius_squared = (diameter * diameter) as i32;
    for major in start..=end {
        let center = (minor + (major as f32 - first) * slope) as i32;
        for offset in -2..=2 {
            let other = center.saturating_add(offset);
            let (x, y) = if use_x { (major, other) } else { (other, major) };
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                continue;
            }
            let cx = 2 * x + 1 - width as i32;
            let cy = 2 * y + 1 - height as i32;
            if cx * cx + cy * cy > radius_squared {
                continue;
            }
            let rx = x as f32 - ax;
            let ry = y as f32 - ay;
            let trail = ((rx * dx + ry * dy) * inverse_length).clamp(0.0, 1.0);
            let ox = rx - dx * trail;
            let oy = ry - dy * trail;
            let glow = (0.48 + 0.83 * trail - ox * ox - oy * oy).clamp(0.0, 1.0);
            let alpha = event.brightness * trail * trail * glow;
            if alpha > 0.015 {
                pixel(x as usize, y as usize, alpha);
            }
        }
    }
}

/// Composite a cool-white meteor sample onto an RGB565 sky color.
pub fn blend_meteor(background: u16, opacity: f32) -> u16 {
    let alpha = (opacity.clamp(0.0, 1.0) * 256.0 + 0.5) as u32;
    let inverse = 256 - alpha;
    let channel = |value, target| ((u32::from(value) * inverse + target * alpha + 128) >> 8) as u16;
    (channel(background >> 11, 29) << 11)
        | (channel((background >> 5) & 63, 61) << 5)
        | channel(background & 31, 31)
}
