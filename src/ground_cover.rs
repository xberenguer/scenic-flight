//! World-aligned plants and ground cover for the terrain raster.

use crate::math::smoothstep;
use crate::render::floor_i32;
use crate::world::Surface;

#[derive(Clone, Copy)]
struct Family {
    color: [u8; 3],
    inverse_radius: f32,
    spread: f32,
    strength: f32,
}

const fn family(color: [u8; 3], radius: f32, spread: f32, strength: f32) -> Family {
    Family {
        color,
        inverse_radius: 1.0 / radius,
        spread,
        strength,
    }
}

// Desert, meadow, woodland, alpine, and coast. Each group has five families.
const FAMILIES: [Family; 25] = [
    family([90, 113, 94], 0.39, 1.0, 0.79),    // Sagebrush
    family([108, 104, 56], 0.41, 1.2, 0.77),   // Creosote
    family([62, 102, 67], 0.33, 1.5, 0.85),    // Prickly pear
    family([133, 88, 68], 0.43, 0.8, 0.84),    // Sandstone
    family([192, 160, 93], 0.42, 1.3, 0.73),   // Bunchgrass
    family([196, 146, 55], 0.30, 1.2, 0.76),   // Poppies
    family([124, 105, 153], 0.32, 0.9, 0.78),  // Lupine
    family([108, 154, 83], 0.40, 1.1, 0.68),   // Clover
    family([143, 153, 85], 0.36, 1.4, 0.72),   // Tussocks
    family([69, 111, 75], 0.36, 0.9, 0.82),    // Juniper
    family([81, 135, 77], 0.40, 1.2, 0.71),    // Fern
    family([75, 103, 69], 0.36, 1.1, 0.82),    // Huckleberry
    family([103, 132, 66], 0.43, 0.9, 0.72),   // Moss
    family([117, 86, 55], 0.28, 2.2, 0.83),    // Fallen timber
    family([153, 112, 66], 0.42, 1.0, 0.67),   // Needle litter
    family([132, 107, 132], 0.37, 1.05, 0.73), // Heather
    family([174, 164, 116], 0.36, 1.24, 0.68), // Lichen
    family([116, 110, 104], 0.40, 0.86, 0.81), // Talus
    family([101, 127, 85], 0.30, 1.18, 0.72),  // Pillow plants
    family([158, 146, 125], 0.41, 0.8, 0.78),  // Moraine stones
    family([125, 139, 72], 0.31, 1.3, 0.78),   // Dune grass
    family([171, 121, 145], 0.28, 1.1, 0.79),  // Sea thrift
    family([122, 96, 61], 0.29, 2.2, 0.78),    // Sea wrack
    family([151, 138, 120], 0.41, 1.28, 0.75), // Shingle
    family([105, 130, 99], 0.39, 1.1, 0.76),   // Saltbush
];

const PALETTES: [[f32; 3]; 10] = [
    [0.0, 0.0, 0.0],
    [22.0, 22.0, 18.0],
    [21.0, -8.0, -7.0],
    [20.0, 13.0, -15.0],
    [12.0, 17.0, 26.0],
    [17.0, 2.0, -15.0],
    [25.0, -14.0, -14.0],
    [-14.0, 15.0, -7.0],
    [-9.0, 2.0, 24.0],
    [-22.0, -19.0, -16.0],
];

fn hash(x: i32, z: i32, seed: u32) -> u32 {
    let state = (x as u32).wrapping_mul(0x85eb_ca6b) ^ (z as u32).wrapping_mul(0xc2b2_ae35) ^ seed;
    let mixed = (state ^ (state >> 16)).wrapping_mul(0x7feb_352d);
    mixed ^ (mixed >> 15)
}

/// Blend woodland pigment over the same world coordinates at every viewing
/// distance. Two independent channels share the coarse lattice hash.
pub(crate) fn woodland_color([x, height, z]: [f32; 3], seed: u32) -> [f32; 3] {
    let u = x * 0.0062 + z * 0.0017;
    let v = z * 0.0062 - x * 0.0017;
    let [ix, iz] = [floor_i32(u), floor_i32(v)];
    let along = smoothstep(u - ix as f32);
    let across = smoothstep(v - iz as f32);
    let key = |dx, dz| hash(ix + dx, iz + dz, seed ^ 0x557c_91e2);
    let [a, b, c, d] = [key(0, 0), key(1, 0), key(0, 1), key(1, 1)];
    let channel = |shift| {
        let [a, b, c, d] = [a, b, c, d].map(|bits| ((bits >> shift) & 255_u32) as f32);
        let first = a + (b - a) * along;
        let second = c + (d - c) * along;
        (first + (second - first) * across) * (1.0 / 255.0)
    };
    let moisture = channel(0);
    let clearing = channel(15);
    let conifers = smoothstep((height - 29.0) * (1.0 / 34.0) + (moisture - 0.5) * 0.46);
    let aspen = smoothstep((clearing - 0.50) * 2.5)
        * smoothstep((height - 14.0) * (1.0 / 26.0))
        * (1.0 - smoothstep((height - 51.0) * (1.0 / 23.0)));
    let broadleaf = [76.0 - 24.0 * moisture, 105.0 + 18.0 * moisture, 82.0 - 24.0 * moisture];
    let evergreen = [39.0, 87.0, 76.0];
    let golden = [148.0, 137.0, 73.0];
    core::array::from_fn(|i| {
        let main = broadleaf[i] + (evergreen[i] - broadleaf[i]) * conifers;
        main + (golden[i] - main) * aspen
    })
}

fn selection(key: u32, district: u32) -> (usize, usize, usize) {
    let local = (key & 7) < 3;
    let family = if local { district } else { key >> 3 } % 5;
    (
        family as usize,
        ((key >> 6) % 10) as usize,
        ((key >> 16) % 10) as usize,
    )
}

fn shape(profile: usize, u: f32, v: f32) -> f32 {
    let [a, b] = [u.abs(), v.abs()];
    match profile {
        0 => u * u + v * v,
        1 => 0.64 * u * u + 1.42 * v * v,
        2 => (a + b * 0.84).min(b + a * 0.84),
        3 => (a - 0.40) * (a - 0.40) + 1.55 * v * v,
        4 => ((a + b) * 0.77).max(a.min(b) * 1.54),
        5 => a * (1.05 + v * 0.27) + b * 0.61,
        6 => 0.78 * u * u + (v + 0.43 * u * u) * (v + 0.43 * u * u),
        7 => (0.71 * a + 0.56 * b).max((u - v * 0.64).abs() * 0.69),
        8 => (u - v * 0.58) * (u - v * 0.58) + 1.29 * v * v,
        _ => (a * 0.78).max(b * 1.56),
    }
}

#[inline(never)]
pub(crate) fn paint(
    surface: Surface,
    [x, height, z]: [f32; 3],
    seed: u32,
    base: [f32; 3],
    detail_fade: f32,
    grade: [f32; 2],
) -> Option<([f32; 3], f32)> {
    if surface == Surface::DryWash {
        return Some((dry_gravel(x, z, seed, base, detail_fade), 0.0));
    }
    let (group, inverse_cell, opacity) = match surface {
        Surface::Desert => (0, 1.0 / 13.0, 0.94),
        Surface::DuneSand => (0, 1.0 / 23.0, 0.53),
        Surface::Grass | Surface::Park | Surface::WetMeadow => (1, 1.0 / 18.0, 1.0),
        Surface::Forest => (2, 1.0 / 22.0, 0.87),
        Surface::Rock => (3, 1.0 / 15.0, 0.67),
        Surface::ScreeEdge => (3, 1.0 / 18.0, 0.68),
        Surface::ScreeFine => (3, 1.0 / 14.0, 0.47),
        Surface::ScreeCoarse | Surface::GlacierMoraine => (3, 1.0 / 16.0, 0.74),
        Surface::AlpineMeadow => (3, 1.0 / 16.0, 0.94),
        Surface::Sand | Surface::RiverSilt if height >= 0.3 => (4, 1.0 / 19.0, 0.87),
        Surface::SaltCrust => (4, 1.0 / 26.0, 0.37),
        _ => return None,
    };
    let [u, v] = [x * inverse_cell, z * inverse_cell];
    let [tx, tz] = [floor_i32(u), floor_i32(v)];
    let key = hash(tx, tz, seed);
    let district = hash(tx >> 3, tz >> 3, seed ^ 0x310a_689f);
    let (kind, profile, palette) = selection(key, district);
    let kind = if group == 3 { alpine_family(surface, kind, key) } else { kind };
    let style = FAMILIES[group * 5 + kind];
    let [u, v] = [u - tx as f32, v - tz as f32];
    let [a, b] = [
        u - (0.42 + ((key >> 20) & 7) as f32 * 0.021),
        v - (0.42 + ((key >> 26) & 7) as f32 * 0.021),
    ];
    let [a, b] = if key & 1 == 0 { [a, b] } else { [b, -a] };
    let inverse_size = style.inverse_radius * (1.15 - ((key >> 29) & 7) as f32 * 0.047);
    let distance = shape(profile, a * inverse_size * style.spread, b * inverse_size);
    let grain = hash(floor_i32(x * 0.39), floor_i32(z * 0.39), seed ^ 0x692d_7fab);
    let fine = if surface == Surface::Desert {
        0.016
    } else {
        0.006
    };
    let grain = (grain & 15) as f32 - 7.5;
    let phase = if group == 3 { strata_phase([x, height, z], seed) } else { 0.0 };
    let ground = if group == 3 {
        alpine_floor(
            surface,
            [x, z],
            seed,
            phase,
            grain / 15.0,
            base,
            detail_fade,
        )
    } else {
        base.map(|channel| channel * (1.0 + grain * fine * detail_fade))
    };
    // Broad, muted fringes remain visible where a small core would alias.
    let mask = 1.0 - smoothstep((distance - 0.35) * 1.12);
    let edge = u.min(1.0 - u).min(v.min(1.0 - v));
    let local_cover = if group == 3 && matches!(kind, 0 | 3) {
        alpine_vegetation(height, grade)
    } else {
        1.0
    };
    let strength = mask
        * opacity * local_cover
        * style.strength
        * (0.38 + 0.62 * detail_fade)
        * (edge * 12.0).clamp(0.0, 1.0);
    Some((core::array::from_fn(|i| {
        let ink = f32::from(style.color[i]) + PALETTES[palette][i];
        ground[i] + (ink - ground[i]) * strength
    }), phase))
}

// The same catalog covers bedrock, recent deposits and established meadows.
fn alpine_family(surface: Surface, kind: usize, key: u32) -> usize {
    match surface {
        Surface::Rock | Surface::GlacierMoraine => [1, 1, 2, 2, 4][kind],
        Surface::ScreeFine => [1, 1, 2, 3, 4][kind],
        Surface::ScreeCoarse => [2, 1, 2, 4, 4][kind],
        Surface::AlpineMeadow if key & 4 != 0 => [0, 0, 3, 3, 1][kind],
        _ => kind,
    }
}

fn alpine_vegetation(height: f32, [along, across]: [f32; 2]) -> f32 {
    let pitch_squared = along * along + across * across;
    let soil = 1.0 - smoothstep((pitch_squared - 0.10) * 1.85);
    let warmth = 1.0 - smoothstep((height - 104.0) * (1.0 / 35.0));
    soil * warmth
}

fn alpine_floor(
    surface: Surface,
    [x, z]: [f32; 2],
    seed: u32,
    phase: f32,
    grain: f32,
    base: [f32; 3],
    detail_fade: f32,
) -> [f32; 3] {
    let [beds, loose] = match surface {
        Surface::Rock => [0.25, 0.08],
        Surface::ScreeEdge => [0.10, 0.14],
        Surface::ScreeFine => [0.03, 0.11],
        Surface::ScreeCoarse => [0.05, 0.23],
        Surface::GlacierMoraine => [0.08, 0.20],
        Surface::AlpineMeadow => [0.0, 0.12],
        _ => [0.0, 0.0],
    };
    let bedding = strata_phase_tone(phase);
    let broken = hash(
        floor_i32(x * 0.071 + z * 0.031),
        floor_i32(z * 0.094 - x * 0.041),
        seed ^ 0x8fe3_d153,
    );
    let fragments = ((broken >> 20) & 15) as f32 / 15.0 - 0.5;
    let broad = if surface == Surface::AlpineMeadow {
        0.13
    } else {
        0.10
    };
    let weathering = bedding * beds * (0.34 + detail_fade * 0.66);
    let shade =
        weathering + fragments * broad * (0.35 + detail_fade * 0.65) + grain * loose * detail_fade;
    // Warm sunny bands read as weathered rock. Silver scree and meadow
    // grains keep the color of their adjoining surface in the distance.
    let clay = weathering * 18.0;
    [
        base[0] * (1.0 + shade) + clay,
        base[1] * (1.0 + shade) + clay * 0.28,
        base[2] * (1.0 + shade) - clay * 0.62,
    ]
}

pub(crate) fn strata_phase([x, height, z]: [f32; 3], seed: u32) -> f32 {
    // Fold the beds across two axes. The rocks keep their seams through
    // bends in the slope while loose fragments follow a separate scale.
    let dip = ((seed >> 10) & 7) as f32 * 0.002 - 0.006;
    let broad_fold = (strata_fold(x * 0.008 + z * 0.004) - 1.0) * 0.38;
    let cross_fold = (strata_fold(z * 0.011 - x * 0.006) - 1.0) * 0.19;
    let section = height * 0.083 + x * dip + z * 0.007 + broad_fold - cross_fold;
    section - floor_i32(section) as f32
}

// The triangle wave is symmetric around each bed boundary. A normalized
// phase minus one bed of height can be folded without another floor.
pub(crate) fn strata_phase_tone(phase: f32) -> f32 {
    smoothstep(((2.0 * phase.abs() - 1.0).abs() - 0.13) * 1.58) - 0.56
}

fn strata_fold(value: f32) -> f32 {
    let phase = value - floor_i32(value) as f32;
    (2.0 * phase - 1.0).abs()
}

fn dry_gravel(x: f32, z: f32, seed: u32, base: [f32; 3], detail_fade: f32) -> [f32; 3] {
    let bar = hash(
        floor_i32(x * 0.17 + z * 0.07),
        floor_i32(z * 0.24 - x * 0.08),
        seed,
    );
    let chip = hash(floor_i32(x * 0.72), floor_i32(z * 0.81), seed ^ 0x71cf_45a3);
    let relief =
        ((bar & 7) as f32 - 3.5) * 0.018 + ((chip & 15) as f32 - 7.5) * 0.012 * detail_fade;
    base.map(|channel| channel * (1.0 + relief))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn woodland_transition_is_continuous_and_keeps_distinct_foothills_and_uplands() {
        let length = 1.0 / 0.0062;
        let mut broadleaf = 0;
        let mut golden = 0;
        for seed in 0..32 {
            let low = woodland_color([-100.0, 17.0, 22.0], seed);
            let high = woodland_color([-100.0, 69.0, 22.0], seed);
            assert!(high[1] + 12.0 < low[1]);
            broadleaf += usize::from(low[1] > 115.0);
            for i in -4..4 {
                let x = i as f32 * length;
                let a = woodland_color([x - 0.01, 42.0, 0.0], seed);
                let b = woodland_color([x + 0.01, 42.0, 0.0], seed);
                assert!(a.into_iter().zip(b).all(|(a, b)| (a - b).abs() < 0.06));
                golden += usize::from(a[0] > 97.0);
            }
        }
        assert!(broadleaf >= 4 && golden >= 10);
    }

    #[test]
    fn exposed_beds_are_continuous_and_lose_contrast_downrange() {
        let seed = 1;
        let rock = |height, fade| {
            alpine_floor(
                Surface::Rock,
                [0.0, 0.0],
                seed,
                strata_phase([0.0, height, 0.0], seed),
                0.0,
                [148.0; 3],
                fade,
            )[0]
        };
        let start = 8.0 / 0.083;
        let end = 9.0 / 0.083;
        let span = |fade| {
            let colors = (0..=64).map(|i| rock(start + (end - start) * i as f32 / 64.0, fade));
            let low = colors.clone().fold(f32::INFINITY, f32::min);
            let high = colors.fold(f32::NEG_INFINITY, f32::max);
            high - low
        };
        let close = span(1.0);
        let distant = span(0.0);
        assert!(close > 25.0 && distant > 6.0 && distant < close * 0.45);
        assert!((rock(start - 0.001, 1.0) - rock(start + 0.001, 1.0)).abs() < 0.01);
    }

    #[test]
    fn alpine_cover_favors_minerals_on_bare_rock_and_vegetation_on_sheltered_soil() {
        for surface in [Surface::Rock, Surface::GlacierMoraine, Surface::ScreeCoarse] {
            let kinds = [0, 1, 2, 3, 4].map(|kind| alpine_family(surface, kind, 4));
            assert!(kinds.iter().all(|&kind| matches!(kind, 1 | 2 | 4)));
            assert!(kinds.contains(&2) && kinds.contains(&4));
        }
        assert_eq!(alpine_family(Surface::AlpineMeadow, 2, 4), 3);
        assert_eq!(alpine_vegetation(95.0, [0.0, 0.0]), 1.0);
        assert_eq!(alpine_vegetation(95.0, [1.0, 0.0]), 0.0);
        assert_eq!(alpine_vegetation(145.0, [0.0, 0.0]), 0.0);
        assert!((alpine_vegetation(95.0, [0.5, 0.0])
            - alpine_vegetation(95.0, [0.0, -0.5])).abs() < f32::EPSILON);
    }

    #[test]
    fn all_catalog_variants_are_selectable_and_terrain_stays_seeded() {
        let mut seen = [[[false; 10]; 10]; 5];
        for z in -55..55 {
            for x in -55..55 {
                let (family, shape, tone) = selection(hash(x, z, 88), hash(x >> 3, z >> 3, 9));
                seen[family][shape][tone] = true;
            }
        }
        assert!(seen.iter().flatten().flatten().all(|&seen| seen));
        let base = [172.0, 146.0, 99.0];
        for surface in [
            Surface::Pavement,
            Surface::FieldGold,
            Surface::GrassRunway,
            Surface::DeepWater,
            Surface::FlatRoof,
            Surface::Snow,
        ] {
            assert_eq!(paint(surface, [-4.0, 36.0, 1.0], 88, base, 1.0, [0.0; 2]), None);
        }
        for surface in [
            Surface::Desert,
            Surface::Grass,
            Surface::Forest,
            Surface::Rock,
            Surface::Sand,
        ] {
            let mut different = 0;
            for x in -48..48 {
                let position = [x as f32, 5.0, 17.0];
                let first = paint(surface, position, 88, base, 0.6, [0.0; 2]);
                assert_eq!(first, paint(surface, position, 88, base, 0.6, [0.0; 2]));
                different += usize::from(first != paint(surface, position, 99, base, 0.6, [0.0; 2]));
            }
            assert!(different > 20);
        }
    }
}
