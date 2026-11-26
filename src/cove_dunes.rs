//! Foredunes on surveyed crescent coves with existing shallow water and sheltered forest.

use crate::beach::{BEACH_TILE_SIZE, BeachSite};
use crate::ground_details::ParcelSurvey;
use crate::lighthouses::{LIGHTHOUSE_TILE, Lighthouse};
use crate::math::{fraction, mix, smoothstep};
use crate::world::{SEA_LEVEL, Sample, Surface, World};

mod profiles;
pub use profiles::CoveDuneProfile;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CoveDune {
    axis: [f32; 2],
    seaward: [f32; 2],
    inverse_length: f32,
    inverse_width: f32,
    bow: f32,
    tilt: f32,
    relief: f32,
    profile: u8,
}

impl CoveDune {
    pub(crate) fn survey(
        world: &World,
        seed: u64,
        origin: [f32; 2],
        key: u64,
        mut accept: impl FnMut([f32; 2], [f32; 4], Self) -> bool,
        pulse: &mut impl FnMut(),
    ) {
        if key & 1 == 0 {
            return;
        }

        const GRID: usize = 10;
        let shift = [fraction(key) * 13.0, fraction(mix(key)) * 13.0];
        let first = ((key >> 24) as usize) % (GRID * GRID);
        for i in 0..GRID * GRID {
            pulse();
            let cell = (first + i * 23) % (GRID * GRID);
            let center = [
                origin[0] + 70.0 + (cell % GRID) as f32 * 39.0 + shift[0],
                origin[1] + 70.0 + (cell / GRID) as f32 * 39.0 + shift[1],
            ];
            let coast = world.coast_signal(center[0], center[1]);
            if !(0.034..=0.172).contains(&coast) {
                continue;
            }
            let design = mix(key ^ (cell as u64 + 0x282a));
            let (length, width) = (
                42.0 + fraction(mix(design ^ 0x8889)) * 12.0,
                12.0 + fraction(mix(design ^ 0x0429)) * 4.0,
            );
            let gradient = [0, 1].map(|i| {
                let mut a = center;
                let mut b = center;
                a[i] -= 12.0;
                b[i] += 12.0;
                world.coast_signal(a[0], a[1]) - world.coast_signal(b[0], b[1])
            });
            let d2 = gradient[0] * gradient[0] + gradient[1] * gradient[1];
            if d2 < 0.022 * 0.022 {
                continue;
            }
            let step = inverse_root(d2);
            let seaward = gradient.map(|v| v * step);
            let axis = [-seaward[1], seaward[0]];
            let shore = |along: f32, across: f32| {
                let [x, z] = [0, 1].map(|i| center[i] + axis[i] * along + seaward[i] * across);
                world.coast_signal(x, z)
            };
            let depths = [-length, 0.0, length].map(|along| shore_distance(|v| shore(along, v)));
            let [Some(left), Some(middle), Some(right)] = depths else {
                continue;
            };
            let bow = (left + right) * 0.5 - middle;
            let tilt = (right - left) * 0.5;
            if !(8.0..=22.0).contains(&bow)
                || tilt.abs() > 12.0
                || left - middle < 4.0
                || right - middle < 4.0
            {
                continue;
            }
            let real_water = |along: f32| shore(along, middle + 9.0);
            if !(-0.111..-0.016).contains(&real_water(0.0))
                || shore(0.0, middle + 26.0) > -0.026
                || real_water(-length) < -0.012
                || real_water(length) < -0.012
            {
                continue;
            }
            // The active beach remains between the waterline and vegetated
            // crest. The surveyed direction makes the lee face point inland.
            let retreat = 40.0 + fraction(mix(design ^ 0x397a)) * 7.0;
            let vegetation = [-0.78, -0.39, 0.0, 0.39, 0.78].map(|u| {
                let v = middle + bow * u * u + tilt * u - retreat - 37.0;
                let [x, z] = [0, 1].map(|i| center[i] + axis[i] * length * u + seaward[i] * v);
                world.sample(x, z).surface == Surface::Forest
            });
            if vegetation.into_iter().filter(|tree| *tree).count() < 2 {
                continue;
            }
            let center = [0, 1].map(|i| center[i] + seaward[i] * (middle - retreat));
            let plan = Self {
                axis,
                seaward,
                inverse_length: 1.0 / length,
                inverse_width: 1.0 / width,
                bow: bow / width,
                tilt: tilt / width,
                relief: 2.50 + fraction(mix(design ^ 0x680f)) * 0.92,
                profile: (mix(design ^ 0x761f) % CoveDuneProfile::ALL.len() as u64) as u8,
            };
            let bounds = plan.bounds(center);
            if !(0..2).all(|i| bounds[i] > origin[i] + 2.0 && bounds[i + 2] < origin[i] + 510.0) {
                continue;
            }
            if plan.prove_ground(world, seed, center, pulse)
                && clears_fixed_sites(world, seed, bounds, pulse)
                && accept(center, bounds, plan)
            {
                return;
            }
        }
    }

    fn bounds(self, center: [f32; 2]) -> [f32; 4] {
        let extent = [0, 1].map(|i| {
            self.axis[i].abs() / self.inverse_length
                + self.seaward[i].abs() * (1.0 + self.bow.abs() + self.tilt.abs())
                    / self.inverse_width
        });
        [
            center[0] - extent[0],
            center[1] - extent[1],
            center[0] + extent[0],
            center[1] + extent[1],
        ]
    }

    fn point(self, center: [f32; 2], u: f32, v: f32) -> [f32; 2] {
        let along = u / self.inverse_length;
        let across = (v + u * (self.tilt + self.bow * u)) / self.inverse_width;
        [0, 1].map(|i| center[i] + self.axis[i] * along + self.seaward[i] * across)
    }

    fn prove_ground(
        self,
        world: &World,
        seed: u64,
        center: [f32; 2],
        pulse: &mut impl FnMut(),
    ) -> bool {
        let mut parcels = ParcelSurvey::new();
        for row in -12..=12 {
            pulse();
            let u = row as f32 * (1.0 / 12.0);
            let mut low = f32::INFINITY;
            let mut high = 0.0_f32;
            for column in -5..=5 {
                let v = column as f32 * 0.2;
                let [x, z] = self.point(center, u, v);
                let ground = world.elevation(x, z);
                if !(SEA_LEVEL + 0.85..=SEA_LEVEL + 6.8).contains(&ground)
                    || (ground >= SEA_LEVEL + 4.0 && parcels.occupies_grass(world, seed, x, z))
                {
                    return false;
                }
                low = low.min(ground);
                high = high.max(ground);
            }
            if high - low > 3.6 {
                return false;
            }
        }
        true
    }

    pub(crate) fn overlay(self, offset: [f32; 2], ground: Sample) -> Sample {
        self.overlay_profile(offset, ground, self.profile())
    }

    fn overlay_profile(
        self,
        offset: [f32; 2],
        ground: Sample,
        profile: &CoveDuneProfile,
    ) -> Sample {
        if ground.elevation < SEA_LEVEL + 0.65
            || ground.elevation > SEA_LEVEL + 7.1
            || !matches!(
                ground.surface,
                Surface::Sand | Surface::Grass | Surface::Forest
            )
        {
            return ground;
        }
        let [x, z] = offset;
        let u = (x * self.axis[0] + z * self.axis[1]) * self.inverse_length;
        if u.abs() >= 1.0 {
            return ground;
        }
        let v = (x * self.seaward[0] + z * self.seaward[1]) * self.inverse_width
            - u * (self.tilt + self.bow * u);
        if v.abs() >= 1.0 {
            return ground;
        }
        let [lean, height] = profile.crest(u);
        let from_crest = (v + 0.36 - 0.13 * u * u - lean) * profile.breadth;
        let face = 1.0 - (1.22 * from_crest).max(-1.75 * from_crest);
        let crest = smoothstep(face) * height;
        let foredune = smoothstep((v - profile.crest_gap) * 2.85) * smoothstep((1.0 - v) * 2.4);
        let ends = smoothstep((1.0 - u.abs()) * 2.3);
        let support = smoothstep((ground.elevation - (SEA_LEVEL + 0.65)) * 1.55)
            * smoothstep((SEA_LEVEL + 7.1 - ground.elevation) * 0.55)
            * smoothstep((1.0 - v.abs()) * 6.0);
        let primary = self.relief * ends * support * crest;
        let rise = (primary + profile.apron * foredune * ends * ends * support).min(3.75);
        Sample {
            elevation: ground.elevation + rise,
            surface: if primary > 1.30 && face > 0.68 {
                Surface::DuneSand
            } else {
                ground.surface
            },
        }
    }

    pub(crate) fn profile(self) -> &'static CoveDuneProfile {
        &CoveDuneProfile::ALL[usize::from(self.profile)]
    }

    pub(crate) fn seaward(self) -> [f32; 2] {
        self.seaward
    }
}

fn shore_distance(mut coast: impl FnMut(f32) -> f32) -> Option<f32> {
    let mut land = -25.0;
    let mut water = 85.0;
    if coast(land) <= 0.008 || coast(water) >= -0.022 {
        return None;
    }
    for _ in 0..8 {
        let middle = (land + water) * 0.5;
        if coast(middle) <= 0.0 {
            water = middle;
        } else {
            land = middle;
        }
    }
    Some((land + water) * 0.5)
}

fn inverse_root(value: f32) -> f32 {
    let first = f32::from_bits(0x5f37_59df - (value.to_bits() >> 1));
    let next = first * (1.5 - 0.5 * value * first * first);
    next * (1.5 - 0.5 * value * next * next)
}

pub(crate) fn clears_fixed_sites(
    world: &World,
    seed: u64,
    bounds: [f32; 4],
    pulse: &mut impl FnMut(),
) -> bool {
    fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
        a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
    }
    let [left, near, right, far] = bounds;
    let tile = |position: f32, size: i32| {
        let scaled = position / size as f32;
        let index = scaled as i32;
        index - i32::from(scaled < index as f32)
    };
    for z in tile(near, BEACH_TILE_SIZE)..=tile(far, BEACH_TILE_SIZE) {
        for x in tile(left, BEACH_TILE_SIZE)..=tile(right, BEACH_TILE_SIZE) {
            pulse();
            if BeachSite::reserved_footprint(world, seed, x, z)
                .is_some_and(|site| overlaps(bounds, site.bounds()))
            {
                return false;
            }
        }
    }
    for z in tile(near, LIGHTHOUSE_TILE)..=tile(far, LIGHTHOUSE_TILE) {
        for x in tile(left, LIGHTHOUSE_TILE)..=tile(right, LIGHTHOUSE_TILE) {
            pulse();
            if Lighthouse::generate(world, seed, x, z)
                .is_some_and(|site| overlaps(bounds, site.bounds()))
            {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cove_profiles_have_distinct_sand_crests_and_stay_below_the_relief_cap() {
        let mut signatures = std::collections::HashSet::new();
        for profile in CoveDuneProfile::ALL {
            let mut signature = std::vec::Vec::new();
            for z in (-22..31).step_by(2) {
                for x in (-50..51).step_by(2) {
                    let sample = profile.study([x as f32, z as f32]);
                    assert!(sample.is_none_or(|grade| (3.4..=7.151).contains(&grade.elevation)));
                    signature.push(sample.map(|grade| grade.surface == Surface::DuneSand));
                }
            }
            assert!(
                signature.iter().filter(|&&bit| bit == Some(true)).count() > 20,
                "{}",
                profile.name
            );
            assert!(
                signatures.insert(signature),
                "{} duplicates a dune",
                profile.name
            );
        }
    }
}
