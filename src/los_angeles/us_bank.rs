//! U.S. Bank Tower on the Downtown Los Angeles metre grid.

#[path = "us_bank_geometry.rs"]
mod geometry;

use crate::{math::sin_cos, world_mesh::WorldMesh};
use core::f32::consts::TAU;
use geometry::*;

pub(crate) const CENTER: [f32; 2] = [-406.27, 122.77];
// Architect's 1,018 feet converted to metres. The lower terrace heights are OSM part tags.
pub(crate) const HEIGHT: f32 = 310.2864;

struct Tier {
    bottom: f32,
    top: f32,
    facades: &'static [[[f32; 2]; 2]],
    terraces: &'static [[[f32; 2]; 3]],
}

const TIERS: [Tier; 5] = [
    Tier { bottom: -7.0, top: 195.0, facades: &FACADES_0, terraces: &TERRACES_0 },
    Tier { bottom: 195.0, top: 225.0, facades: &FACADES_1, terraces: &TERRACES_1 },
    Tier { bottom: 225.0, top: 250.0, facades: &FACADES_2, terraces: &TERRACES_2 },
    Tier { bottom: 250.0, top: 280.0, facades: &FACADES_3, terraces: &TERRACES_3 },
    Tier { bottom: 280.0, top: HEIGHT, facades: &FACADES_4, terraces: &TERRACES_4 },
];

pub(crate) fn contains(x: f32, z: f32) -> bool {
    let [east, north] = [x - CENTER[0], z - CENTER[1]];
    if east.abs() > 31.0 || north.abs() > 31.0 {
        return false;
    }
    let mut inside = false;
    for &[[a_x, a_z], [b_x, b_z]] in &FACADES_0 {
        if (a_z > north) != (b_z > north)
            && east < a_x + (b_x - a_x) * (north - a_z) / (b_z - a_z)
        {
            inside = !inside;
        }
    }
    inside
}

pub(crate) fn draw(mesh: &mut WorldMesh<'_>, base: f32) {
    const STONE: [u8; 3] = [177, 175, 165];
    const TERRACE: [u8; 3] = [151, 155, 155];
    let point = |[east, north]: [f32; 2], height| [CENTER[0] + east, base + height, CENTER[1] + north];
    for tier in &TIERS {
        for &triangle in tier.terraces {
            mesh.triangle(triangle.map(|position| point(position, tier.top)), TERRACE);
        }
        for &[a, b] in tier.facades {
            let horizontal = [b[0] - a[0], b[1] - a[1]];
            let length = horizontal[0].abs() + horizontal[1].abs();
            let light = horizontal[0].abs() * 18.0 / length.max(0.01);
            let glass = [98, 120, 135].map(|tone| tone + light as u8);
            let face = |lo, hi| [point(a, lo), point(b, lo), point(b, hi), point(a, hi)];
            mesh.faded_quad(face(tier.bottom, tier.top), glass, 1.0);

            // Group the regular granite spandrels at a scale that remains legible from a flyby.
            let outward = [-horizontal[1] * 0.13 / length.max(0.01), horizontal[0] * 0.13 / length.max(0.01)];
            let inset = |p: [f32; 2]| [p[0] + outward[0], p[1] + outward[1]];
            let band = |lo, hi| [
                point(inset(a), lo), point(inset(b), lo), point(inset(b), hi), point(inset(a), hi),
            ];
            let count = ((tier.top - tier.bottom) / 17.0) as usize;
            for stripe in 0..count {
                let lo = tier.bottom + 10.0 + stripe as f32 * 17.0;
                mesh.faded_quad(band(lo, (lo + 1.15).min(tier.top)), STONE, 1.0);
            }
            mesh.faded_quad(band(tier.top - 1.1, tier.top), STONE, 1.0);
            if length > 2.2 {
                let c = [a[0] + horizontal[0] * 0.16, a[1] + horizontal[1] * 0.16];
                mesh.faded_quad([
                    point(inset(a), tier.bottom), point(inset(c), tier.bottom),
                    point(inset(c), tier.top), point(inset(a), tier.top),
                ], STONE, 1.0);
            }
            if tier.top == HEIGHT {
                let [a, b, c, d] = band(300.0, 303.8);
                let tint = [139, 168, 194];
                mesh.luminous_triangle([a, b, c], tint, 0.52);
                mesh.luminous_triangle([a, c, d], tint, 0.52);
            }
        }
    }

    // The rooftop landing area stays within the surveyed 33 m crown.
    let pad = |index: usize, radius: f32| {
        let (sin, cos) = sin_cos(index as f32 * TAU / 16.0);
        point([radius * cos, radius * sin], HEIGHT + 0.08)
    };
    for index in 0..16 {
        mesh.faded_quad(
            [pad(index, 10.5), pad(index + 1, 10.5), pad(index + 1, 10.0), pad(index, 10.0)],
            [218, 217, 198], 1.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapped_tower_uses_its_own_block_and_documented_height() {
        assert!(contains(CENTER[0], CENTER[1]));
        assert!(contains(-431.0, 117.0));
        assert!(!contains(-436.0, 145.0));
        assert!(!contains(-363.0, 126.0));
        assert!((HEIGHT - 1018.0 * 0.3048).abs() < 0.0001);
    }
}
