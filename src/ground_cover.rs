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
    Family { color, inverse_radius: 1.0 / radius, spread, strength }
}

// Desert, meadow, woodland, alpine, and coast. Each group has five families.
const FAMILIES: [Family; 25] = [
    family([102, 120, 85], 0.32, 1.0, 0.79), // Sagebrush
    family([118, 113, 72], 0.34, 1.2, 0.77), // Creosote
    family([71, 108, 64], 0.27, 1.5, 0.85),  // Prickly pear
    family([145, 109, 82], 0.39, 0.8, 0.84), // Sandstone
    family([169, 147, 88], 0.36, 1.3, 0.73), // Bunchgrass
    family([196, 146, 55], 0.30, 1.2, 0.76), // Poppies
    family([124, 105, 153], 0.32, 0.9, 0.78), // Lupine
    family([108, 154, 83], 0.40, 1.1, 0.68), // Clover
    family([143, 153, 85], 0.36, 1.4, 0.72), // Tussocks
    family([69, 111, 75], 0.36, 0.9, 0.82),  // Juniper
    family([81, 135, 77], 0.40, 1.2, 0.71),  // Fern
    family([75, 103, 69], 0.36, 1.1, 0.82),  // Huckleberry
    family([103, 132, 66], 0.43, 0.9, 0.72), // Moss
    family([117, 86, 55], 0.28, 2.2, 0.83),  // Fallen timber
    family([153, 112, 66], 0.42, 1.0, 0.67), // Needle litter
    family([132, 107, 132], 0.37, 1.05, 0.73), // Heather
    family([174, 164, 116], 0.36, 1.24, 0.68), // Lichen
    family([116, 110, 104], 0.40, 0.86, 0.81), // Talus
    family([101, 127, 85], 0.30, 1.18, 0.72), // Pillow plants
    family([158, 146, 125], 0.41, 0.8, 0.78), // Moraine stones
    family([125, 139, 72], 0.31, 1.3, 0.78), // Dune grass
    family([171, 121, 145], 0.28, 1.1, 0.79), // Sea thrift
    family([122, 96, 61], 0.29, 2.2, 0.78), // Sea wrack
    family([151, 138, 120], 0.41, 1.28, 0.75), // Shingle
    family([105, 130, 99], 0.39, 1.1, 0.76), // Saltbush
];

const PALETTES: [[f32; 3]; 10] = [
    [0.0, 0.0, 0.0], [22.0, 22.0, 18.0], [21.0, -8.0, -7.0],
    [20.0, 13.0, -15.0], [12.0, 17.0, 26.0], [17.0, 2.0, -15.0],
    [25.0, -14.0, -14.0], [-14.0, 15.0, -7.0], [-9.0, 2.0, 24.0],
    [-22.0, -19.0, -16.0],
];

fn hash(x: i32, z: i32, seed: u32) -> u32 {
    let state = (x as u32).wrapping_mul(0x85eb_ca6b)
        ^ (z as u32).wrapping_mul(0xc2b2_ae35) ^ seed;
    let mixed = (state ^ (state >> 16)).wrapping_mul(0x7feb_352d);
    mixed ^ (mixed >> 15)
}

fn selection(key: u32, district: u32) -> (usize, usize, usize) {
    let local = (key & 7) < 3;
    let family = if local { district } else { key >> 3 } % 5;
    (family as usize, ((key >> 6) % 10) as usize, ((key >> 16) % 10) as usize)
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
    surface: Surface, [x, height, z]: [f32; 3], seed: u32, base: [f32; 3],
    detail_fade: f32,
) -> Option<[f32; 3]> {
    let (group, inverse_cell, opacity) = match surface {
        Surface::Desert => (0, 1.0 / 22.0, 1.0),
        Surface::DuneSand => (0, 1.0 / 26.0, 0.62),
        Surface::Grass | Surface::Park | Surface::WetMeadow => (1, 1.0 / 18.0, 1.0),
        Surface::Forest => (2, 1.0 / 22.0, 0.87),
        Surface::Rock | Surface::ScreeEdge | Surface::ScreeFine | Surface::ScreeCoarse
        | Surface::AlpineMeadow | Surface::GlacierMoraine => (3, 1.0 / 24.0, 0.91),
        Surface::Sand | Surface::RiverSilt if height >= 0.3 => (4, 1.0 / 19.0, 0.87),
        Surface::SaltCrust => (4, 1.0 / 26.0, 0.37),
        _ => return None,
    };
    let [u, v] = [x * inverse_cell, z * inverse_cell];
    let [tx, tz] = [floor_i32(u), floor_i32(v)];
    let key = hash(tx, tz, seed);
    let district = hash(tx >> 3, tz >> 3, seed ^ 0x310a_689f);
    let (kind, profile, palette) = selection(key, district);
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
    let ground = base.map(|channel| channel * (1.0 + (f32::from((grain & 15) as u8) - 7.5) * 0.006 * detail_fade));
    // Broad, muted fringes remain visible where a small core would alias.
    let mask = 1.0 - smoothstep((distance - 0.35) * 1.12);
    let edge = u.min(1.0 - u).min(v.min(1.0 - v));
    let strength = mask * opacity * style.strength * (0.38 + 0.62 * detail_fade)
        * (edge * 12.0).clamp(0.0, 1.0);
    Some(core::array::from_fn(|i| {
        let ink = f32::from(style.color[i]) + PALETTES[palette][i];
        ground[i] + (ink - ground[i]) * strength
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        for surface in [Surface::Pavement, Surface::FieldGold, Surface::GrassRunway,
                        Surface::DeepWater, Surface::FlatRoof, Surface::Snow] {
            assert_eq!(paint(surface, [-4.0, 36.0, 1.0], 88, base, 1.0), None);
        }
        for surface in [Surface::Desert, Surface::Grass, Surface::Forest, Surface::Rock, Surface::Sand] {
            let mut different = 0;
            for x in -48..48 {
                let position = [x as f32, 5.0, 17.0];
                let first = paint(surface, position, 88, base, 0.6);
                assert_eq!(first, paint(surface, position, 88, base, 0.6));
                different += usize::from(first != paint(surface, position, 99, base, 0.6));
            }
            assert!(different > 20);
        }
    }
}
