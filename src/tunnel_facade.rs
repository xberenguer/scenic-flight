//! Portal masonry profiles inside the surveyed retaining ring.

use crate::tunnels::{RoadTunnel, TunnelMouth, TunnelPoint};
use crate::world_mesh::WorldMesh;

#[derive(Clone, Copy)]
enum Stone {
    Granite,
    Limestone,
    Sandstone,
    Brick,
    Concrete,
}

#[derive(Clone, Copy)]
enum Trim {
    Segmental,
    Crowned,
    Stepped,
    Buttressed,
    Banded,
}

#[derive(Clone, Copy)]
pub struct TunnelFacade {
    pub name: &'static str,
    stone: Stone,
    trim: Trim,
}

use Stone::*;
use Trim::*;

impl TunnelFacade {
    pub const ALL: [Self; 25] = [
        Self::new("Granite segmental arch", Granite, Segmental),
        Self::new("Granite keystone portal", Granite, Crowned),
        Self::new("Granite stepped shoulders", Granite, Stepped),
        Self::new("Granite buttressed portal", Granite, Buttressed),
        Self::new("Granite double-ring portal", Granite, Banded),
        Self::new("Limestone segmental arch", Limestone, Segmental),
        Self::new("Limestone keystone portal", Limestone, Crowned),
        Self::new("Limestone stepped shoulders", Limestone, Stepped),
        Self::new("Limestone buttressed portal", Limestone, Buttressed),
        Self::new("Limestone double-ring portal", Limestone, Banded),
        Self::new("Sandstone segmental arch", Sandstone, Segmental),
        Self::new("Sandstone keystone portal", Sandstone, Crowned),
        Self::new("Sandstone stepped shoulders", Sandstone, Stepped),
        Self::new("Sandstone buttressed portal", Sandstone, Buttressed),
        Self::new("Sandstone double-ring portal", Sandstone, Banded),
        Self::new("Brick segmental arch", Brick, Segmental),
        Self::new("Brick keystone portal", Brick, Crowned),
        Self::new("Brick stepped shoulders", Brick, Stepped),
        Self::new("Brick buttressed portal", Brick, Buttressed),
        Self::new("Brick double-ring portal", Brick, Banded),
        Self::new("Concrete segmental arch", Concrete, Segmental),
        Self::new("Concrete keystone portal", Concrete, Crowned),
        Self::new("Concrete stepped shoulders", Concrete, Stepped),
        Self::new("Concrete buttressed portal", Concrete, Buttressed),
        Self::new("Concrete double-ring portal", Concrete, Banded),
    ];

    const fn new(name: &'static str, stone: Stone, trim: Trim) -> Self {
        Self { name, stone, trim }
    }

    pub fn for_tunnel(tunnel: &RoadTunnel) -> Self {
        let [first, last] = tunnel.junctions();
        let seed = u64::from(first.x.to_bits()) << 32 | u64::from(first.z.to_bits());
        let end = u64::from(last.x.to_bits()) << 32 | u64::from(last.z.to_bits());
        Self::ALL[(crate::math::mix(seed ^ end.rotate_left(13)) % Self::ALL.len() as u64) as usize]
    }

    fn widths(self) -> [f32; 6] {
        // One side from road level to arch crown, mirrored on the other.
        let trim = match self.trim {
            Segmental => [0.60, 0.67, 0.77, 0.82, 0.80, 0.76],
            Crowned => [0.82, 0.77, 0.62, 0.57, 0.78, 1.00],
            Stepped => [0.94, 0.97, 0.61, 0.89, 0.62, 0.88],
            Buttressed => [1.00, 0.92, 0.86, 0.58, 0.52, 0.58],
            Banded => [0.85, 0.98, 0.93, 0.95, 0.96, 0.94],
        };
        let masonry = match self.stone {
            Granite => [1.00, 0.90, 0.98, 1.00, 0.82, 0.92],
            Limestone => [0.89, 0.93, 0.90, 0.81, 0.97, 1.00],
            Sandstone => [0.82, 0.98, 0.80, 0.90, 1.00, 0.97],
            Brick => [0.76, 0.78, 1.00, 0.91, 0.94, 1.00],
            Concrete => [1.00, 1.00, 0.88, 0.89, 0.80, 0.84],
        };
        core::array::from_fn(|index| trim[index] * masonry[index])
    }

    fn colors(self) -> ([u8; 3], [u8; 3]) {
        match self.stone {
            Granite => ([131, 140, 147], [181, 188, 190]),
            Limestone => ([175, 163, 139], [208, 197, 171]),
            Sandstone => ([167, 112, 77], [220, 162, 109]),
            Brick => ([149, 86, 71], [195, 146, 123]),
            Concrete => ([151, 155, 149], [199, 196, 182]),
        }
    }

    pub fn frame(self, mouth: TunnelMouth) -> [[TunnelPoint; 4]; 10] {
        let widths = self.widths();
        let envelope = mouth.frame();
        core::array::from_fn(|index| {
            let [a, b, c, d] = envelope[index];
            [
                across(b, a, widths[index.min(10 - index)]),
                b,
                c,
                across(c, d, widths[(index + 1).min(9 - index)]),
            ]
        })
    }

    /// Draw the masonry around the actual opening using the same mesh as flight.
    pub fn draw(self, mouth: TunnelMouth, mesh: &mut WorldMesh<'_>) {
        let (stone, cap) = self.colors();
        for (panel, [a, b, c, d]) in self.frame(mouth).into_iter().enumerate() {
            let side = panel.min(9 - panel);
            match self.trim {
                Banded => {
                    let inner_a = across(b, a, 0.41);
                    let inner_b = across(c, d, 0.41);
                    draw_quad(mesh, [a, inner_a, inner_b, d], stone);
                    draw_quad(mesh, [inner_a, b, c, inner_b], cap);
                }
                _ => {
                    let feature = match self.trim {
                        Segmental => side % 2 == 0,
                        Crowned => side == 4,
                        Stepped => matches!(side, 1 | 3),
                        Buttressed => side == 0,
                        Banded => false,
                    };
                    draw_quad(mesh, [a, b, c, d], if feature { cap } else { stone });
                }
            }
        }
    }
}

fn across(near: TunnelPoint, far: TunnelPoint, weight: f32) -> TunnelPoint {
    TunnelPoint {
        x: near.x + (far.x - near.x) * weight,
        y: near.y + (far.y - near.y) * weight,
        z: near.z + (far.z - near.z) * weight,
    }
}

fn draw_quad(mesh: &mut WorldMesh<'_>, points: [TunnelPoint; 4], color: [u8; 3]) {
    let [a, b, c, d] = points.map(|point| [point.x, point.y, point.z]);
    mesh.lit_triangle([a, b, c], color);
    mesh.lit_triangle([a, c, d], color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_facades_keep_the_exact_portal_opening_and_original_envelope() {
        let mouth = TunnelMouth {
            center: TunnelPoint {
                x: 73.0,
                y: 12.0,
                z: -11.0,
            },
            inward_x: 0.6,
            inward_z: 0.8,
        };
        let mut seen = [[[mouth.center; 4]; 10]; 25];
        for (index, design) in TunnelFacade::ALL.into_iter().enumerate() {
            let frame = design.frame(mouth);
            assert!(!seen[..index].contains(&frame));
            seen[index] = frame;
            for (part, original) in frame.into_iter().zip(mouth.frame()) {
                assert_eq!(part[1], original[1]);
                assert_eq!(part[2], original[2]);
                for (edge, inside) in [(0, 1), (3, 2)] {
                    for axis in [
                        |p: TunnelPoint| p.x,
                        |p: TunnelPoint| p.y,
                        |p: TunnelPoint| p.z,
                    ] {
                        let a = axis(original[edge]);
                        let b = axis(original[inside]);
                        assert!((a.min(b)..=a.max(b)).contains(&axis(part[edge])));
                    }
                }
            }
        }
    }
}
