//! Fremont Street's Viva Vision barrel vault over the mapped pedestrian segment.

use crate::{math::{sin_cos, smoothstep}, world::{Surface, World}, world_mesh::WorldMesh};
use core::f32::consts::{PI, TAU};

// Centre and bearing come from OSM way 1428180251 and the covered street below it.
pub(super) const CENTER: [f32; 2] = [2_497.84, 6_562.65];
pub(super) const HEIGHT: f32 = 27.432;
pub(super) const REACH: f32 = 2_100.0;
const DIRECTION: [f32; 2] = [0.89041, -0.45517];
const HALF_LENGTH: f32 = 209.55;
const RADIUS: f32 = 13.716;
const BASE: f32 = HEIGHT - RADIUS;
const BAYS: usize = 28;
const SECTORS: usize = 6;
const STEEL: [u8; 3] = [135, 161, 180];

fn world_point(along: f32, across: f32) -> [f32; 2] {
    let [east, north] = DIRECTION;
    [CENTER[0] + along * east - across * north, CENTER[1] + along * north + across * east]
}

pub(super) fn contains(x: f32, z: f32) -> bool {
    let [dx, dz] = [x - CENTER[0], z - CENTER[1]];
    let [east, north] = DIRECTION;
    (east * dx + north * dz).abs() < HALF_LENGTH + 2.5
        && (east * dz - north * dx).abs() < RADIUS + 1.2
}

pub(super) fn legacy_prism(x: f32, z: f32, surface: Surface, height: f32) -> bool {
    surface == Surface::LandmarkRoof11 && height == 14.0 && contains(x, z)
}

pub(super) struct Canopy {
    grade: [f32; 2],
    overhead_view: f32,
}

impl Canopy {
    pub(super) fn new(world: &World, altitude: f32, center_ground: f32) -> Self {
        let grade = [-HALF_LENGTH, HALF_LENGTH].map(|along| {
            let [x, z] = world_point(along, 0.0);
            world.elevation(x, z)
        });
        let overhead_view = smoothstep((altitude - center_ground - BASE) / 55.0);
        Self { grade, overhead_view }
    }

    fn point(&self, t: f32, across: f32, height: f32) -> [f32; 3] {
        let [x, z] = world_point((2.0 * t - 1.0) * HALF_LENGTH, across);
        [x, self.grade[0] + (self.grade[1] - self.grade[0]) * t + height, z]
    }

    pub(super) fn draw(&self, mesh: &mut WorldMesh<'_>, ticks: u64) {
        let night = smoothstep((0.62 - mesh.daylight()) / 0.62);
        let arc = core::array::from_fn::<_, { SECTORS + 1 }, _>(|index| {
            let (sin, cos) = sin_cos(PI * index as f32 / SECTORS as f32);
            [RADIUS * cos, BASE + RADIUS * sin]
        });
        let point = |along: f32, [across, height]: [f32; 2]| self.point(along, across, height);
        let phase_tick_count = u64::from(crate::flight::TICKS_PER_SECOND) * 18;
        let phase = (ticks % phase_tick_count) as f32 * TAU / phase_tick_count as f32;
        for bay in 0..BAYS {
            let from = bay as f32 / BAYS as f32;
            let to = (bay + 1) as f32 / BAYS as f32;
            for (sector, pair) in arc.windows(2).enumerate() {
                let [left, right] = [pair[0], pair[1]];
                let [a, b, c, d] = [point(from, left), point(to, left), point(to, right), point(from, right)];
                let skin = [100, 130, 153].map(|component| component - (sector.abs_diff(3) as u8 * 6));
                let (light, band) = light_run(phase, from, sector as f32 / SECTORS as f32);
                let exposed = 1.0 - self.overhead_view * 0.35;
                let effect = exposed * (0.06 + night * 0.90) * (0.34 + light * 0.66);
                let tint = interpolate(skin, band, effect);
                if night > 0.02 {
                    let emission = night * (0.66 + exposed * light * 0.34);
                    mesh.luminous_triangle([a, b, c], tint, emission);
                    mesh.luminous_triangle([a, c, d], tint, emission);
                } else {
                    mesh.faded_quad([a, b, c, d], tint, 1.0);
                }
                if bay % 2 == 0 {
                    mesh.faded_segment([point(from, [left[0], left[1] + 0.14]),
                        point(from, [right[0], right[1] + 0.14])], 0.40, STEEL, 0.84);
                }
            }
            for side in [-1.0, 1.0] {
                let across = side * RADIUS;
                let ends = [self.point(from, across, BASE), self.point(to, across, BASE)];
                if night > 0.5 {
                    let (intensity, color) = light_run(phase, from, (1.0 + side) * 0.5);
                    let trim = interpolate(STEEL, color, 0.57 * intensity);
                    mesh.luminous_segment(ends, 0.52, trim);
                } else {
                    mesh.faded_segment(ends, 0.58, STEEL, 1.0);
                }
            }
        }
        for pair in arc.windows(2) {
            mesh.faded_segment([point(1.0, pair[0]), point(1.0, pair[1])], 0.55, STEEL, 1.0);
        }
        for support in 0..8 {
            let along = (support as f32 + 0.5) / 8.0;
            for side in [-1.0, 1.0] {
                let across = side * RADIUS;
                mesh.faded_segment([self.point(along, across, 0.2),
                    self.point(along, across, BASE + 0.9)], 0.92, [102, 125, 143], 1.0);
            }
        }
    }
}

fn interpolate(a: [u8; 3], b: [u8; 3], amount: f32) -> [u8; 3] {
    core::array::from_fn(|channel| {
        let a = f32::from(a[channel]);
        let b = f32::from(b[channel]);
        (a + (b - a) * amount) as u8
    })
}

fn light_run(phase: f32, along: f32, across: f32) -> (f32, [u8; 3]) {
    // Original, slowly travelling washes for the flyby. The venue screen plays its own music reels.
    let (wave, tint) = sin_cos(phase - along * TAU * 2.0 + across * 1.7);
    let glow = smoothstep(wave * 0.51 + 0.49);
    (glow, interpolate([81, 196, 251], [242, 70, 165], tint * 0.5 + 0.5))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roof_prism_replacement_follows_the_street_axis() {
        let inside = world_point(175.0, 6.0);
        let outside = world_point(175.0, 21.0);
        assert!(legacy_prism(inside[0], inside[1], Surface::LandmarkRoof11, 14.0));
        assert!(!legacy_prism(outside[0], outside[1], Surface::LandmarkRoof11, 14.0));
        assert!(!legacy_prism(inside[0], inside[1], Surface::LandmarkRoof11, 38.0));
    }

    #[test]
    fn light_pattern_moves_along_the_canopy() {
        assert_ne!(light_run(0.0, 0.3, 0.75), light_run(0.9, 0.3, 0.75));
        let (strength, color) = light_run(0.0, 0.3, 0.75);
        assert!((0.0..=1.0).contains(&strength));
        assert!(color[2] > 170);
    }
}
