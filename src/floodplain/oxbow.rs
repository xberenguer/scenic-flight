//! Cutoff meanders on dry low terraces beside a retained river reach.

use super::{length, natural_cover, region_origin, Area, FloodplainKind, Site};
use crate::ground_details::ParcelSurvey;
use crate::math::{fraction, mix, smoothstep};
use crate::watershed::{Reach, Watershed, REGION_SIZE};
use crate::world::{Sample, Surface, World, SEA_LEVEL};

const MAX_TRIALS: usize = 7;
const BANK_GAP: f32 = 2.0;

pub(super) fn survey(
    world: &World,
    water: &Watershed,
    reserved: &mut impl FnMut(f32, f32) -> bool,
    pulse: &mut impl FnMut(),
) -> Option<Site> {
    let first = water.streams().first()?;
    let key = mix(point_key(first.outlet) ^ point_key(first.source) ^ 0xba07_58d2_c2b5_4261);
    if key & 3 != 0 {
        return None;
    }
    let mut parcels = ParcelSurvey::new();
    let mut trials = 0;
    for reach in water.reaches() {
        pulse();
        if reach.reaches_sea || !(0.0..=2.15).contains(&(reach.start_level - reach.end_level)) {
            continue;
        }
        let shape = mix(key ^ point_key(reach.start));
        let side = if shape & 1 == 0 { -1.0 } else { 1.0 };
        for side in [side, -side] {
            let Some(bow) = fit(world, water, reach, side, shape) else {
                continue;
            };
            trials += 1;
            if prove(bow, world, water, &mut parcels, reserved, pulse) {
                return Some(bow);
            }
            if trials == MAX_TRIALS {
                return None;
            }
        }
    }
    None
}

fn fit(world: &World, water: &Watershed, reach: Reach, side: f32, key: u64) -> Option<Site> {
    let middle = reach.position(0.5);
    let nearby = water.sample(middle[0], middle[1])?;
    if !nearby.is_channel() || !(SEA_LEVEL + 2.0..=SEA_LEVEL + 30.0).contains(&nearby.level) {
        return None;
    }
    let a = reach.position(0.12);
    let b = reach.position(0.88);
    let [dx, dz] = [b[0] - a[0], b[1] - a[1]];
    let span = length(dx, dz);
    if !(23.0..=47.0).contains(&span) {
        return None;
    }
    // The positive cross axis always points away from the active channel.
    let axis = [side * dx / span, side * dz / span];
    let half_length = 29.0 + fraction(mix(key ^ 0xa92)) * 11.0;
    let half_width = 19.0 + fraction(mix(key ^ 0x322)) * 7.0;
    let offset = nearby.half_width + 6.8 + half_width * 0.54;
    let center = [middle[0] - axis[1] * offset, middle[1] + axis[0] * offset];
    let curve = (fraction(mix(key ^ 0x564b)) - 0.5) * 0.24;
    let mut shape = Site {
        center,
        axis,
        inverse_size: [1.0 / half_length, 1.0 / half_width],
        curve,
        water_level: 0.0,
        level_slope: 0.0,
        bounds: extent(center, axis, half_length, half_width),
        kind: FloodplainKind::OxbowLake,
    };
    let region = reach.start.map(region_origin);
    if (0..2).any(|axis| {
        shape.bounds[axis] < region[axis] + 2.0
            || shape.bounds[axis + 2] >= region[axis] + REGION_SIZE as f32 - 2.0
    }) {
        return None;
    }
    // Measure the entire proposed abandoned channel against the native shelf.
    // Silting can leave it slightly perched above the present river.
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for [u, v] in [
        [-0.76, -0.23],
        [-0.78, 0.0],
        [-0.69, 0.39],
        [-0.42, 0.66],
        [0.0, 0.80],
        [0.42, 0.66],
        [0.69, 0.39],
        [0.78, 0.0],
        [0.76, -0.23],
    ] {
        let [x, z] = point(shape, u, v);
        let native = world.sample(x, z);
        if !natural_cover(native.surface) {
            return None;
        }
        low = low.min(native.elevation);
        high = high.max(native.elevation);
    }
    let level = (nearby.level + 0.18).max(low - 0.68);
    if high - low > 2.25 || low < level + 0.16 || level > nearby.level + 2.7 {
        return None;
    }
    shape.water_level = level;
    Some(shape)
}

fn prove(
    bow: Site,
    world: &World,
    water: &Watershed,
    parcels: &mut ParcelSurvey,
    reserved: &mut impl FnMut(f32, f32) -> bool,
    pulse: &mut impl FnMut(),
) -> bool {
    // In both tips the fringe fades back to the intact floodplain before it
    // can touch the new channel, leaving two separate sediment plugs.
    for u in [-0.76, 0.76] {
        let [x, z] = point(bow, u, -0.51);
        let native = world.sample(x, z);
        if !natural_cover(native.surface)
            || native.elevation < bow.water_level + 0.42
            || water
                .sample(x, z)
                .is_some_and(|channel| channel.distance - channel.half_width < BANK_GAP)
            || reserved(x, z)
        {
            return false;
        }
    }

    let mut wet = 0;
    for line in 0..=32 {
        pulse();
        let u = (line as f32 / 16.0 - 1.0) * 1.04;
        for column in 0..=23 {
            let v = column as f32 * (1.72 / 23.0) - 0.62;
            let shape = profile(u, v, bow.curve);
            if shape <= 0.075 {
                continue;
            }
            let [x, z] = point(bow, u, v);
            let native = world.sample(x, z);
            let rise = native.elevation - bow.water_level;
            if !natural_cover(native.surface)
                || !(0.10..=3.75).contains(&rise)
                || water
                    .sample(x, z)
                    .is_some_and(|channel| channel.distance - channel.half_width < BANK_GAP)
                || reserved(x, z)
                || (native.surface == Surface::Grass
                    && parcels.occupies_grass(world, water.seed(), x, z))
            {
                return false;
            }
            wet += usize::from(shape >= 0.69);
        }
    }
    wet > 90
}

pub(super) fn overlay(site: Site, u: f32, v: f32, ground: Sample) -> Option<Sample> {
    let score = profile(u, v, site.curve);
    let rise = ground.elevation - site.water_level;
    if score < 0.13 || !(0.08..=4.0).contains(&rise) {
        return None;
    }
    let depth = smoothstep(score / 0.69);
    let surface = if score >= 0.69 {
        Surface::RiverWater
    } else if score >= 0.34 {
        Surface::ReedMarsh
    } else {
        Surface::WetMeadow
    };
    Some(Sample {
        elevation: ground.elevation - rise * depth,
        surface,
    })
}

fn profile(u: f32, v: f32, skew: f32) -> f32 {
    let u = u + skew * (v + 0.08);
    let radius_squared = u * u + v * v;
    let depth = 1.0 - smoothstep(((radius_squared - 0.64).abs() - 0.115) * 4.5);
    let tip = smoothstep((v + 0.55) * 3.30);
    depth * tip
}

fn point(site: Site, u: f32, v: f32) -> [f32; 2] {
    let u = u / site.inverse_size[0];
    let v = v / site.inverse_size[1];
    [
        site.center[0] + site.axis[0] * u - site.axis[1] * v,
        site.center[1] + site.axis[1] * u + site.axis[0] * v,
    ]
}

fn extent(center: [f32; 2], axis: [f32; 2], half_length: f32, half_width: f32) -> Area {
    let [ax, az] = axis.map(f32::abs);
    let x = ax * half_length * 1.06 + az * half_width * 1.10;
    let z = az * half_length * 1.06 + ax * half_width * 1.10;
    [center[0] - x, center[1] - z, center[0] + x, center[1] + z]
}

fn point_key([x, z]: [f32; 2]) -> u64 {
    (u64::from(x.to_bits()) << 32) | u64::from(z.to_bits())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facility_stream::FacilityStream;
    use crate::land_stream::LandStream;
    use crate::render::Camera;
    use crate::river_stream::RiverStream;
    use crate::watershed::WatershedWork;

    #[test]
    fn seeded_oxbow_publishes_standing_water_with_two_dry_plugs() {
        let seed = 2;
        let region = (-7, -3);
        let world = World::new(seed);
        let camera = Camera {
            x: (region.0 * REGION_SIZE) as f32 + 512.0,
            z: (region.1 * REGION_SIZE) as f32 + 512.0,
            altitude: 55.0,
            heading: 0.0,
            pitch: 0.2,
            bank: 0.0,
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        land.preflight(camera, false);
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        let mut rivers = RiverStream::<3>::new(world, seed);
        rivers.preflight(&land, &facilities, camera, &mut WatershedWork::new());
        let district = rivers.planned(region).flatten().unwrap();
        let oxbow = *district
            .floodplain()
            .sites
            .iter()
            .flatten()
            .find(|site| site.kind == FloodplainKind::OxbowLake)
            .unwrap();
        let sampler = rivers.sampler();
        let sample = |x, z| {
            let raw = world.sample(x, z);
            sampler.overlay(x, z, facilities.overlay(x, z, raw, land.overlay(x, z, raw)))
        };

        for [u, v] in [[-0.78, 0.0], [0.0, 0.80], [0.78, 0.0]] {
            let [x, z] = point(oxbow, u, v);
            let standing = sample(x, z);
            assert_eq!(standing.surface, Surface::RiverWater);
            assert!((standing.elevation - oxbow.water_level).abs() < 0.01);
            assert!(district
                .watershed()
                .sample(x, z)
                .is_none_or(|water| !water.is_channel()));
        }
        for [u, v] in [[-0.76, -0.51], [0.0, 0.0], [0.76, -0.51]] {
            let [x, z] = point(oxbow, u, v);
            let plug = sample(x, z);
            assert_ne!(plug.surface, Surface::RiverWater);
            assert!(plug.elevation > oxbow.water_level);
        }

        let [x, z] = point(oxbow, 0.0, 0.80);
        let blocked = survey(
            &world,
            district.watershed(),
            &mut |px, pz| (px - x).abs() < 3.0 && (pz - z).abs() < 3.0,
            &mut || {},
        );
        assert!(blocked.is_none_or(|candidate| candidate.center != oxbow.center));
    }
}
