//! Sandstone ridges surveyed against their underlying slope before publication.

use super::in_dry_tile;
use crate::math::{fraction, mix, smoothstep};
use crate::world::{Sample, Surface, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RidgeProfile(u8);

impl RidgeProfile {
    pub const COUNT: usize = NAMES.len();

    pub const fn from_index(index: usize) -> Option<Self> {
        if index < Self::COUNT { Some(Self(index as u8)) } else { None }
    }

    pub const fn name(self) -> &'static str { NAMES[self.0 as usize] }

    /// Relief and material in the two normalized axes of a surveyed ridge.
    pub fn study(self, u: f32, v: f32) -> Option<(f32, Surface)> {
        let [x, z] = [u.abs(), v.abs()];
        let radius = x.max(z) + 0.18 * x.min(z);
        if radius >= 1.0 { return None; }
        let crown = match self.0 {
            0 => ridge(v - 0.25 * u, 0.35) * shoulder(x, 0.93, 0.37),
            1 => ridge(v - 0.42 * u * u + 0.15, 0.29) * shoulder(x, 0.95, 0.39),
            2 => ridge(v + 0.38 * x - 0.14, 0.32) * shoulder(x, 0.83, 0.31),
            3 => ridge(v - 0.21 * u + 0.41 * u * u * u, 0.31)
                * shoulder(x, 0.92, 0.26) * (0.83 + 0.17 * u),
            4 => ridge(v + 0.18 + 0.38 * u * u, 0.21 + 0.31 * x)
                * shoulder(x, 0.97, 0.45),
            5 => ridge(v - 0.26, 0.23).max(0.72 * ridge(v + 0.36 + 0.13 * u, 0.27))
                * shoulder(x, 0.83, 0.36),
            6 => lobe(u + 0.24, v - 0.23 * u, [0.64, 0.30])
                .max(0.86 * lobe(u - 0.38, v + 0.28, [0.54, 0.32])),
            7 => ridge(v - 0.39 * u, 0.24).max(0.67 * ridge(v + 0.38 * u + 0.23, 0.22))
                * shoulder(x, 0.94, 0.30),
            8 => ridge(v - 0.13 * u - 0.26, 0.23) * shoulder(x, 0.79, 0.24)
                + 0.43 * ridge(v + 0.31, 0.34) * shoulder(x, 0.89, 0.45),
            9 => lobe(u - 0.08, v - 0.24, [0.72, 0.31])
                .max(0.84 * lobe(u + 0.29, v + 0.29, [0.54, 0.34]))
                .max(0.64 * lobe(u - 0.47, v + 0.31, [0.32, 0.31])),
            10 => terrace(x.max(z * 1.42), 0.49, 0.86, 0.43),
            11 => terrace(x.max((v - 0.35 * u).abs() * 1.58), 0.44, 0.82, 0.58),
            12 => terrace((u + 0.11).abs().max((v + 0.22 * u * u).abs() * 1.22),
                0.43, 0.89, 0.38),
            13 => terrace((x * 0.68 + z * 0.96).max(x * 0.92) + 0.16 * v, 0.44, 0.80, 0.52),
            14 => terrace((x * 0.67 + (v - 0.18 * u).abs() * 1.37).max(x * 0.91),
                0.40, 0.79, 0.67) * (1.0 - 0.53 * ridge(u - 0.27, 0.23) * shoulder(-v, 0.12, 0.44)),
            15 => peak_line(u, v, [-0.59, 0.01, 0.58], [1.00, 0.73, 0.91], -0.15),
            16 => peak_line(u, v, [-0.56, -0.04, 0.49], [0.67, 1.00, 0.54], 0.45),
            17 => peak_line(u, v, [-0.66, -0.21, 0.47], [0.62, 0.83, 1.00], -0.45),
            18 => peak_line(u, v, [-0.42, 0.06, 0.59], [0.96, 0.75, 0.66], 0.06)
                .max(0.53 * lobe(u + 0.49, v - 0.55, [0.34, 0.27])),
            19 => peak_line(u, v - 0.37 * u * u, [-0.63, -0.09, 0.44], [0.72, 1.00, 0.82], 0.12),
            20 => lobe(u + 0.31, v + 0.13, [0.45, 0.40])
                .max(0.92 * lobe(u - 0.52, v - 0.15, [0.26, 0.25])),
            21 => lobe(u + 0.02, v + 0.12, [0.32, 0.44])
                .max(0.69 * lobe(u + 0.57, v - 0.30, [0.24, 0.26]))
                .max(0.59 * lobe(u - 0.54, v - 0.17, [0.26, 0.40])),
            22 => lobe(u + 0.33, v - 0.34, [0.38, 0.25])
                .max(0.83 * lobe(u - 0.23, v + 0.36, [0.49, 0.29])),
            23 => lobe(u + 0.53, v, [0.27, 0.36])
                .max(0.72 * lobe(u - 0.02, v + 0.15, [0.25, 0.30]))
                .max(0.96 * lobe(u - 0.51, v - 0.21, [0.27, 0.25])),
            _ => lobe(u + 0.19, v + 0.01, [0.59, 0.38])
                .max(0.82 * lobe(u - 0.49, v + 0.23, [0.29, 0.26]))
                .max(0.65 * lobe(u - 0.18, v - 0.49, [0.27, 0.30])),
        };
        let skirt = shoulder(radius, 1.0, 0.58);
        let rim = shoulder(radius, 1.0, 0.22);
        let lift = 0.15 * skirt + 0.85 * rim * crown;
        let surface = if crown >= 0.83 && lift >= 0.75 { Surface::MesaCap }
            else if crown >= 0.23 && lift >= 0.29 { Surface::MesaCliff }
            else { Surface::MesaApron };
        Some((lift, surface))
    }
}

fn ridge(cross: f32, width: f32) -> f32 { shoulder(cross.abs(), width, width * 0.83) }

fn shoulder(distance: f32, edge: f32, fade: f32) -> f32 {
    smoothstep((edge - distance) / fade)
}

fn lobe(u: f32, v: f32, [width, depth]: [f32; 2]) -> f32 {
    let [a, b] = [u.abs() / width, v.abs() / depth];
    shoulder(a.max(b) + 0.33 * a.min(b), 1.0, 0.61)
}

fn terrace(distance: f32, inner: f32, outer: f32, step: f32) -> f32 {
    step * shoulder(distance, outer, 0.16) + (1.0 - step) * shoulder(distance, inner, 0.20)
}

fn peak_line(u: f32, v: f32, positions: [f32; 3], heights: [f32; 3], tilt: f32) -> f32 {
    positions.into_iter().zip(heights).fold(0.0_f32, |height, (crest, rise)| {
        height.max(rise * lobe(u - crest, v - crest * tilt, [0.36, 0.44 - 0.18 * crest.abs()]))
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Ridge {
    axis: [f32; 2],
    inverse: [f32; 2],
    rise: f32,
    pub(super) profile: RidgeProfile,
}

impl Ridge {
    pub(super) fn survey(
        world: &World, origin: [f32; 2], key: u64, mut excluded: impl FnMut([f32; 4]) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<super::DryLandmark> {
        if key >> 40 & 7 >= 4 { return None; }
        let first = (key >> 16 & 15) as usize;
        for probe in 0..16 {
            pulse();
            let site = (first + probe * 7) & 15;
            let salt = mix(key ^ site as u64);
            let [x, z] = [
                origin[0] + 95.0 + (site % 4) as f32 * 99.0 + fraction(salt) * 22.0,
                origin[1] + 95.0 + (site / 4) as f32 * 99.0 + fraction(mix(salt)) * 22.0,
            ];
            let (center, coast) = world.sample_and_shore(x, z);
            if center.surface != Surface::Desert || coast < 0.30
                || !(24.0..=55.0).contains(&center.elevation) { continue; }
            let dx = world.elevation(x + 40.0, z) - world.elevation(x - 40.0, z);
            let dz = world.elevation(x, z + 40.0) - world.elevation(x, z - 40.0);
            let length = dx * dx + dz * dz;
            if !(25.0..=729.0).contains(&length) { continue; }
            let mut direction = dx.abs().max(dz.abs());
            for _ in 0..3 { direction = 0.5 * (direction + length / direction); }
            let axis = [dz / direction, -dx / direction];
            let span = 53.0 + fraction(mix(salt ^ 0x120a)) * 33.0;
            let width = 24.0 + fraction(mix(salt ^ 0xfe51)) * 20.0;
            let xr = axis[0].abs() * span + axis[1].abs() * width;
            let zr = axis[1].abs() * span + axis[0].abs() * width;
            let bounds = [x - xr, z - zr, x + xr, z + zr];
            if !in_dry_tile(origin, bounds) { continue; }
            let ground_clear = [-0.80, -0.40, 0.0, 0.40, 0.80].into_iter().all(|u| {
                pulse();
                [-0.75, 0.0, 0.75].into_iter().all(|v| {
                    let along = u * span;
                    let across = v * width;
                    let ground = world.sample(x + axis[0] * along - axis[1] * across,
                        z + axis[1] * along + axis[0] * across);
                    ground.surface == Surface::Desert
                        && (ground.elevation - center.elevation).abs() <= 14.0
                })
            });
            if !ground_clear || excluded(bounds) { continue; }
            let profile = RidgeProfile((mix(salt ^ 0x378a) % RidgeProfile::COUNT as u64) as u8);
            let rise = 13.0 + fraction(mix(salt ^ 0x17e4)) * 15.0;
            return Some(super::DryLandmark {
                center: [x, z], bounds, floor: center.elevation, rim: center.elevation + rise,
                shape: super::Shape::Ridge(Self { axis, inverse: [1.0 / span, 1.0 / width], rise, profile }),
            });
        }
        None
    }

    pub(super) fn overlay(self, [x, z]: [f32; 2], ground: Sample) -> Sample {
        let [ax, az] = self.axis;
        let [along, across] = self.inverse;
        let Some((relief, surface)) = self.profile.study((x * ax + z * az) * along,
            (z * ax - x * az) * across)
        else { return ground };
        let support = smoothstep((ground.elevation - 12.0) * 0.25)
            * smoothstep((67.0 - ground.elevation) * 0.22);
        let rise = self.rise * relief * support;
        Sample { elevation: ground.elevation + rise,
            surface: if rise >= 0.36 { surface } else { ground.surface } }
    }
}

const NAMES: [&str; 25] = [
    "Oblique spine", "Bowed fin", "Angular ridge", "Swept crest", "Broad-ended divide",
    "Uneven parallel ribs", "Overlapping ridges", "Forked ridge", "Offset parallel ledges",
    "Staggered triple ridge", "Two-step escarpment", "Slanted terraces", "Curved upper bench",
    "Angular stepped divide", "Terrace with a cleft", "Three eroded summits", "Central pinnacle",
    "Rising sawtooth", "Crest with a separate buttress", "Bowed line of summits",
    "Butte and tower", "Tall butte with two remnants", "Separated sandstone fins",
    "Three sandstone pillars", "Long cap with offset shoulders",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ridge_profiles_have_distinct_crest_shapes_and_taper_at_bounds() {
        let mut crests = HashSet::new();
        for index in 0..RidgeProfile::COUNT {
            let profile = RidgeProfile::from_index(index).unwrap();
            let samples: std::vec::Vec<_> = (-20..=20).flat_map(|z| (-20..=20).map(move |x| {
                profile.study(x as f32 * 0.05, z as f32 * 0.05)
                    .map_or(0, |(height, _)| (height * 32.0) as u8)
            })).collect();
            assert!(samples.iter().any(|&height| height > 19), "{}", profile.name());
            assert!(crests.insert(samples), "{}", profile.name());
            for cross in [-0.64, -0.21, 0.0, 0.21, 0.64] {
                assert_eq!(profile.study(1.0, cross), None);
                assert_eq!(profile.study(cross, -1.0), None);
            }
        }
        assert_eq!(crests.len(), 25);
    }
}
