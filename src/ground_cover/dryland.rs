//! Dry-surface patterns fixed to the terrain, from gravel bars to salt crust.

use super::{floor_i32, hash, smoothstep, strata_fold, strata_phase, strata_phase_tone};
use crate::world::Surface;

#[derive(Clone, Copy)]
pub(super) enum Material {
    Scrub,
    Wash,
    Dune,
    Sandstone,
    Salt,
}

impl Material {
    pub(super) fn from_surface(surface: Surface) -> Option<Self> {
        match surface {
            Surface::Desert => Some(Self::Scrub),
            Surface::DryWash => Some(Self::Wash),
            Surface::DuneSand => Some(Self::Dune),
            Surface::MesaCap | Surface::MesaCliff | Surface::MesaApron => Some(Self::Sandstone),
            Surface::SaltCrust => Some(Self::Salt),
            _ => None,
        }
    }

    pub(super) fn cell_size(self) -> f32 {
        match self {
            Self::Scrub => 19.0,
            Self::Wash => 18.0,
            Self::Dune => 37.0,
            Self::Sandstone => 29.0,
            Self::Salt => 26.0,
        }
    }

    pub(super) fn shade(
        self, [x, height, z]: [f32; 3], seed: u32, base: [f32; 3], fade: f32, grade: [f32; 2],
    ) -> [f32; 3] {
        // Rotate the sampling grid across the landscape. Detail fades out
        // within each tile so neighboring deposits can change without seams.
        let cell = 1.0 / self.cell_size();
        let direction = if seed & 1 == 0 { 1.0 } else { -1.0 };
        let east = x * 0.94 + z * (0.34 * direction);
        let north = z * 0.94 - x * (0.34 * direction);
        let [u, v] = [east * cell, north * cell];
        let [tx, tz] = [floor_i32(u), floor_i32(v)];
        let key = hash(tx, tz, seed);
        let district = hash(tx >> 2, tz >> 2, seed ^ 0x310a_689f);
        let profile = Self::profile(key, district);
        let [u, v] = [u - tx as f32, v - tz as f32];
        let edge = u.min(1.0 - u).min(v.min(1.0 - v));
        let envelope = smoothstep(edge * 7.0);
        let a = 2.0 * u - 1.0 + ((key >> 22 & 7) as f32 - 3.5) * 0.039;
        let b = 2.0 * v - 1.0 + ((key >> 26 & 7) as f32 - 3.5) * 0.039;
        let [a, b] = if key >> 29 & 1 == 0 { [a, b] } else { [-a, -b] };
        let variation = ((key >> 8 & 15) as f32) * (1.0 / 16.0);
        let drift = strata_fold(a * 0.96 + b * 0.62 + variation) - 0.5;
        let cross = strata_fold(a * 0.47 - b * 1.12 + variation * 1.63) - 0.5;
        let (mark, ink) = self.mark(profile, a + 0.32 * cross, b + 0.30 * drift);
        let speck = hash(floor_i32(east * 0.43), floor_i32(north * 0.52), seed ^ 0x71cf_45a3);
        let grain = (speck >> 16 & 15) as f32 * (1.0 / 15.0);
        let grey = (grain - 0.5) * (0.077 * fade);

        let [forward, across] = grade;
        let plants = if matches!(self, Self::Scrub) && profile < 4 {
            1.0 - smoothstep((forward * forward + across * across - 0.05) * 1.7)
        } else {
            1.0
        };
        let bedding = if matches!(self, Self::Sandstone) {
            strata_phase_tone(strata_phase([x, height, z], seed)) * (0.11 + fade * 0.09)
        } else {
            0.0
        };
        let fragments = 1.0 - fade * grain * 0.38;
        let fraction = mark * envelope * plants * fragments * (0.38 + 0.51 * fade);
        // The same deposit has small shade differences while a whole district
        // retains a shared color bias. Grain disappears first at distance.
        let stain = (district >> 10 & 7) as f32 - 3.5;
        let grain = grey + bedding;
        core::array::from_fn(|channel| {
            let ground = base[channel] * (1.0 + grain);
            let color = f32::from(ink[channel]) + stain * [2.4, 1.5, 0.7][channel];
            ground + (color - ground) * fraction
        })
    }

    pub(super) fn profile(key: u32, district: u32) -> u32 {
        if key >> 5 & 3 == 0 {
            (key >> 13) % 5
        } else {
            ((district >> 12) + (key >> 14 & 1)) % 5
        }
    }

    pub(super) fn mark(self, profile: u32, a: f32, b: f32) -> (f32, [u8; 3]) {
        let [x, z] = [a.abs(), b.abs()];
        let dab = |dx: f32, dz: f32, radius: f32| {
            1.0 - smoothstep(((a - dx).abs() + (b - dz).abs()) / radius)
        };
        let line = |distance: f32, width: f32| {
            1.0 - smoothstep(distance.abs() / width)
        };
        match (self, profile) {
            // Creosote clusters, saltbush hummocks, sage on drainage slopes, bunchgrass and desert pavement.
            (Self::Scrub, 0) => (dab(-0.49, -0.03, 0.57).max(dab(-0.17, 0.40, 0.64))
                .max(dab(0.44, 0.34, 0.46)).max(dab(0.26, -0.38, 0.51)), [95, 110, 69]),
            (Self::Scrub, 1) => (dab(-0.31, 0.24, 0.72).max(dab(0.27, -0.25, 0.92)), [130, 144, 108]),
            (Self::Scrub, 2) => (line(a - b * 0.79 + b * b * 0.36, 0.47)
                * smoothstep((0.56 - b) * 1.9), [105, 123, 95]),
            (Self::Scrub, 3) => (dab(-0.39, -0.26, 0.59).max(dab(0.43, -0.31, 0.54))
                .max(dab(0.11, 0.49, 0.62)), [199, 178, 112]),
            (Self::Scrub, _) => (smoothstep(1.05 - (x * 0.64 + z * 1.46)) * (0.72 + b * 0.17), [135, 93, 74]),
            // The channel floor stays gravel, with crescents and sorted sediment.
            (Self::Wash, 0) => (line(b - 0.37 * a * a + 0.21, 0.43), [123, 107, 92]),
            (Self::Wash, 1) => (line(a + 0.42 * b + 0.36 * b * b + 0.21, 0.25)
                .max(line(a + 0.17 * b - 0.29, 0.27) * smoothstep((b + 0.35) * 2.6)), [214, 197, 161]),
            (Self::Wash, 2) => (dab(0.22, 0.02, 0.79).max(dab(-0.26, 0.39, 0.63)), [130, 126, 116]),
            (Self::Wash, 3) => (line(b - x * 0.62, 0.39) * (1.0 - x * 0.82), [213, 177, 135]),
            (Self::Wash, _) => (line(b - 0.46 * a + 0.13, 0.37)
                * smoothstep((a + 0.73) * 1.21), [146, 112, 91]),
            // Wind ripples, lee crescents and dunes of differing shapes.
            (Self::Dune, 0) => (line(b + a * a * 0.56 + 0.18, 0.31), [219, 177, 119]),
            (Self::Dune, 1) => (line(a + b * b * b * 0.44, 0.42), [139, 102, 71]),
            (Self::Dune, 2) => (line(b + a * 0.25 - 0.29, 0.24)
                .max(line(b + a * 0.25 + 0.27, 0.19)), [220, 183, 132]),
            (Self::Dune, 3) => (line(b + x * 0.64 - 0.16, 0.30)
                * smoothstep((1.0 - x) * 1.5), [157, 108, 71]),
            (Self::Dune, _) => (dab(-0.15, 0.32, 1.02).max(dab(0.53, -0.30, 0.61)), [198, 162, 111]),
            // Iron staining, detached blocks, talus tongues, weathered cap and pale seams.
            (Self::Sandstone, 0) => (line(a - b * 0.57 + 0.15, 0.45), [126, 75, 63]),
            (Self::Sandstone, 1) => (dab(-0.26, 0.47, 0.60).max(dab(0.31, -0.02, 0.75)), [207, 139, 100]),
            (Self::Sandstone, 2) => (smoothstep((1.0 - x * (1.34 + b * 0.67) - z * 0.58) * 1.72), [125, 102, 90]),
            (Self::Sandstone, 3) => (line(x + z * 0.66 - 0.43, 0.27)
                * smoothstep((a - b + 0.85) * 1.9), [211, 151, 104]),
            (Self::Sandstone, _) => (line(b + 0.28 * a - 0.27, 0.24)
                .max(line(b - a * 0.41 + 0.17, 0.14)), [218, 176, 124]),
            // Desiccation seams, fresh crust rims, deposits and silt streaks.
            (Self::Salt, 0) => (line((x * 0.89 + z * 0.72).max(z * 1.30) - 0.49, 0.19), [170, 153, 132]),
            (Self::Salt, 1) => (line(x - 0.36 + b * 0.17 + b * b * 0.38, 0.15)
                .max(line(b + a * 0.22 - 0.19, 0.16) * smoothstep((0.19 - a) * 2.6)), [241, 226, 199]),
            (Self::Salt, 2) => (dab(-0.33, -0.35, 0.89).max(dab(0.41, 0.35, 0.80)), [237, 224, 202]),
            (Self::Salt, 3) => (line(b - 0.37 * a * a + 0.03, 0.40)
                * (1.0 - 0.92 * x), [174, 152, 136]),
            (Self::Salt, _) => (line(a + 0.29 * b, 0.26)
                .max(line(b - x * 0.52, 0.20)), [236, 214, 177]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_dry_surface_profiles_have_distinct_masks() {
        let materials = [Material::Scrub, Material::Wash, Material::Dune, Material::Sandstone, Material::Salt];
        let mut shapes = std::collections::HashSet::new();
        for material in materials {
            for profile in 0..5 {
                let pixels: std::vec::Vec<_> = (-4..=4).flat_map(|z| (-4..=4).map(move |x| {
                    let (mask, _) = material.mark(profile, x as f32 * 0.17, z as f32 * 0.17);
                    (mask * 127.0) as u8
                })).collect();
                assert!(shapes.insert(pixels));
            }
        }
        assert_eq!(shapes.len(), 25);
    }

    #[test]
    fn dry_surface_patterns_are_seeded_and_keep_cell_boundaries_continuous() {
        for material in [Material::Scrub, Material::Wash, Material::Dune, Material::Sandstone, Material::Salt] {
            let cell = material.cell_size();
            let shade = |u: f32, v: f32, seed| {
                let [x, z] = [0.94 * u - 0.34 * v, 0.34 * u + 0.94 * v];
                let factor = 0.94_f32 * 0.94 + 0.34 * 0.34;
                material.shade([x / factor, 27.0, z / factor], seed, [168.0, 133.0, 96.0], 0.0, [0.0; 2])
            };
            let mut varied = 0;
            for offset in -8..8 {
                let u = offset as f32 * cell;
                let v = (offset as f32 + 0.42) * cell;
                let left = shade(u - 0.001, v, 2);
                let right = shade(u + 0.001, v, 2);
                assert!(left.into_iter().zip(right).all(|(l, r)| (l - r).abs() < 0.01));
                varied += usize::from(shade(u + cell * 0.58, v, 2) != shade(u + cell * 0.58, v, 6));
            }
            assert!(varied >= 7);
        }
    }
}
