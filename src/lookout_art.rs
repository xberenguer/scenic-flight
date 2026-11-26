//! Fire lookout and summit shelter meshes on a surveyed foundation.

use crate::math::{sin_cos, smoothstep};
use crate::world_mesh::WorldMesh;

mod designs;
pub use designs::SummitDesign;
use designs::{Part, Profile};

type Point = [f32; 3];
type Color = [u8; 3];

const FOUNDATION_BOTTOM: f32 = -3.0;
const STONE: Color = [131, 127, 115];
const TIMBER: Color = [107, 86, 66];
const BRACE: Color = [126, 103, 79];
const PLATFORM: Color = [155, 132, 100];
const TRIM: Color = [82, 91, 87];
const GLASS: Color = [58, 87, 101];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LookoutKind {
    Watchtower,
    SummitHut,
    Landmark(SummitDesign),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookoutArt {
    center: Point,
    heading: f32,
    kind: LookoutKind,
    variation: u32,
}

impl LookoutArt {
    /// The surveyed point is [east, foundation grade, north]. Heading points
    /// from the structure toward its approach. A 7 m radius covers either kind.
    pub const fn new(center: Point, heading: f32, kind: LookoutKind, variation: u32) -> Self {
        Self {
            center,
            heading,
            kind,
            variation,
        }
    }

    pub fn mesh_bounds(&self) -> [Point; 2] {
        let (half_x, half_z) = match self.kind {
            LookoutKind::Watchtower => (4.7, 4.7),
            LookoutKind::SummitHut => (3.85, 5.15),
            LookoutKind::Landmark(design) => {
                design.data().parts.iter().fold((0.0_f32, 0.0_f32), |(w, d), part| {
                    (w.max(part.center[0].abs() + part.half[0]), d.max(part.center[1].abs() + part.half[1]))
                })
            }
        };
        let (sin, cos) = sin_cos(self.heading);
        let (x, y, z) = (self.center[0], self.center[1], self.center[2]);
        let along_x = half_x * cos.abs() + half_z * sin.abs();
        let along_z = half_x * sin.abs() + half_z * cos.abs();
        [
            [x - along_x, y + FOUNDATION_BOTTOM, z - along_z],
            [x + along_x, y + self.height(), z + along_z],
        ]
    }

    pub(crate) fn height(&self) -> f32 {
        match self.kind {
            LookoutKind::Watchtower => 20.0,
            LookoutKind::SummitHut => 7.0,
            LookoutKind::Landmark(design) => design.data().parts.iter()
                .map(|part| part.levels[1]).fold(0.0_f32, f32::max),
        }
    }

    pub fn draw(&self, mesh: &mut WorldMesh<'_>, daylight: f32) {
        let dusk = smoothstep((0.55 - daylight) / 0.46);
        let (sin, cos) = sin_cos(self.heading);
        let [x, y, z] = self.center;
        let amber = [255.0, 201.0, 113.0];
        let lamp = core::array::from_fn(|i| {
            let glass = f32::from(GLASS[i]);
            (glass + (amber[i] - glass) * dusk) as u8
        });
        let mut render = |points: [Point; 3], color: Color, lit_window: bool| {
            let world = points.map(|[east, height, north]| {
                [
                    x + east * cos + north * sin,
                    y + height,
                    z - east * sin + north * cos,
                ]
            });
            if lit_window {
                mesh.luminous_triangle(world, lamp, 0.26 + 0.58 * dusk);
            } else {
                mesh.lit_triangle(world, color);
            }
        };
        self.build(&mut render);
    }

    fn build(&self, emit: &mut impl FnMut([Point; 3], Color, bool)) {
        let (wall, roof) = match self.variation % 3 {
            0 => ([199, 182, 145], [83, 103, 87]),
            1 => ([156, 132, 108], [119, 78, 61]),
            _ => ([181, 187, 171], [87, 101, 111]),
        };
        let mut art = Builder { emit };
        let occupied = (self.variation >> 2) & 3 == 0;
        match self.kind {
            LookoutKind::Watchtower => art.tower(wall, roof, occupied),
            LookoutKind::SummitHut => art.hut(wall, roof, occupied),
            LookoutKind::Landmark(design) => {
                for part in design.data().parts {
                    art.landmark_part(part);
                }
            }
        }
    }
}

struct Builder<'a, F> {
    emit: &'a mut F,
}

impl<F: FnMut([Point; 3], Color, bool)> Builder<'_, F> {
    fn landmark_part(&mut self, part: &Part) {
        let [x, z] = part.center;
        let [low, high] = part.levels;
        let color = part.color;
        match part.profile {
            Profile::Box => self.block([x, low, z], part.half, high, color),
            Profile::Gable => {
                let [a, b, c, d] = ring([x, low, z], part.half);
                let front = [x, high, a[2]];
                let back = [x, high, d[2]];
                self.quad([a, d, back, front], color, false);
                self.quad([front, back, c, b], color, false);
                (self.emit)([a, front, b], color, false);
                (self.emit)([c, back, d], color, false);
            }
            Profile::Hip => {
                let crest = ring([x, high, z], [part.half[0] * 0.21, part.half[1] * 0.29]);
                self.shell(ring([x, low, z], part.half), crest, color);
                self.top(crest, color);
            }
            Profile::Dome => {
                const SECTION: [[f32; 2]; 8] = [
                    [-0.414, -1.0], [0.414, -1.0], [1.0, -0.414], [1.0, 0.414],
                    [0.414, 1.0], [-0.414, 1.0], [-1.0, 0.414], [-1.0, -0.414],
                ];
                let section = |rise, scale| SECTION.map(|[u, v]| {
                    [x + u * part.half[0] * scale, low + (high - low) * rise, z + v * part.half[1] * scale]
                });
                let rim = section(0.0, 1.0);
                let shoulder = section(0.46, 0.86);
                let summit = section(0.82, 0.56);
                self.shell(rim, shoulder, color);
                self.shell(shoulder, summit, color);
                for edge in 0..summit.len() {
                    (self.emit)([summit[edge], [x, high, z], summit[(edge + 1) % summit.len()]], color, false);
                }
            }
        }
    }

    fn tower(&mut self, wall: Color, roof: Color, occupied: bool) {
        let bases = ring([0.0, 0.8, 0.0], [3.9, 3.9]);
        let cap = ring([0.0, 14.14, 0.0], [2.74, 2.74]);
        for ([x, y, z], top) in bases.into_iter().zip(cap) {
            self.block([x, FOUNDATION_BOTTOM, z], [0.74, 0.74], 0.97, STONE);
            self.shell(
                ring([x, y, z], [0.34, 0.34]),
                ring(top, [0.29, 0.29]),
                TIMBER,
            );
        }
        for side in 0..4 {
            let next = (side + 1) % 4;
            let at = |along: f32, rise: f32| {
                lerp(
                    lerp(bases[side], bases[next], along),
                    lerp(cap[side], cap[next], along),
                    rise,
                )
            };
            for lower in [0.04, 0.52] {
                let upper = lower + 0.42;
                self.quad(
                    [
                        at(0.06, lower),
                        at(0.13, lower),
                        at(0.94, upper),
                        at(0.86, upper),
                    ],
                    BRACE,
                    false,
                );
                self.quad(
                    [
                        at(0.94, lower),
                        at(0.87, lower),
                        at(0.06, upper),
                        at(0.14, upper),
                    ],
                    BRACE,
                    false,
                );
            }
        }

        // Four flights and their landings form a visible switchback inside the frame.
        for flight in 0..4 {
            let y = 0.52 + 3.48 * flight as f32;
            let x = if flight & 1 == 0 { -2.62 } else { 2.62 };
            let mut treads = [
                [x, y, 1.84],
                [x, y, 3.1],
                [-x, y + 3.48, 3.1],
                [-x, y + 3.48, 1.84],
            ];
            if x > 0.0 {
                treads.reverse();
            }
            self.quad(treads, PLATFORM, false);
            self.top(ring([-x, y + 3.49, 2.5], [0.41, 0.63]), TIMBER);
        }

        let deck_bottom = ring([0.0, 14.09, 0.0], [4.31, 4.31]);
        let deck_top = ring([0.0, 14.77, 0.0], [4.31, 4.31]);
        self.shell(deck_bottom, deck_top, TIMBER);
        self.quad(deck_bottom, TIMBER, false);
        self.top(deck_top, PLATFORM);

        for side in 0..4 {
            for fractions in [[0.07, 0.46], [0.54, 0.93]] {
                self.panel(
                    [3.1, 3.03],
                    side,
                    fractions,
                    [15.50, 17.27],
                    GLASS,
                    occupied && side != 0,
                );
            }
        }
        self.block([0.0, 14.75, 0.0], [3.1, 3.03], 17.53, wall);
        self.shell(
            ring([0.0, 17.42, 0.0], [3.73, 3.66]),
            ring([0.0, 17.68, 0.0], [3.73, 3.66]),
            TRIM,
        );
        let rim = ring([0.0, 17.68, 0.0], [3.73, 3.66]);
        let crest = ring([0.0, 19.46, 0.0], [0.38, 0.38]);
        self.shell(rim, crest, roof);
        self.top(crest, roof);

        let rail = ring([0.0, 15.68, 0.0], [4.14, 4.14]);
        self.shell(rail, ring([0.0, 15.92, 0.0], [4.14, 4.14]), TRIM);
        for [x, _, z] in rail {
            self.quad(
                [
                    [x - 0.13, 14.76, z],
                    [x - 0.13, 15.91, z],
                    [x + 0.13, 15.91, z],
                    [x + 0.13, 14.76, z],
                ],
                TRIM,
                false,
            );
            self.quad(
                [
                    [x, 14.76, z - 0.13],
                    [x, 15.91, z - 0.13],
                    [x, 15.91, z + 0.13],
                    [x, 14.76, z + 0.13],
                ],
                TRIM,
                false,
            );
        }
    }

    fn hut(&mut self, wall: Color, roof: Color, occupied: bool) {
        let walls = [3.19, 3.92];
        self.block([0.0, FOUNDATION_BOTTOM, 0.0], [3.39, 4.16], 0.58, STONE);
        self.block([0.0, FOUNDATION_BOTTOM, 4.57], [1.42, 0.53], 0.30, STONE);
        self.block([0.0, 0.27, 4.30], [1.28, 0.27], 0.58, PLATFORM);
        self.panel(walls, 2, [0.35, 0.62], [0.57, 2.65], TIMBER, false);
        self.panel(walls, 2, [0.70, 0.90], [1.71, 2.84], GLASS, occupied);
        for side in [0, 1, 3] {
            self.panel(
                walls,
                side,
                [0.27, 0.69],
                [1.53, 2.91],
                GLASS,
                occupied && side == 3,
            );
        }
        self.block([0.0, 0.53, 0.0], walls, 3.72, wall);
        let [left_front, right_front, right_back, left_back] = ring([0.0, 3.62, 0.0], [3.80, 4.52]);
        let ridge_height = 5.98;
        let front = [0.0, ridge_height, -4.52];
        let back = [0.0, ridge_height, 4.52];
        self.quad([left_front, left_back, back, front], roof, false);
        self.quad([front, back, right_back, right_front], roof, false);
        (self.emit)(
            [
                [-walls[0], 3.62, -walls[1]],
                [0.0, ridge_height, -walls[1]],
                [walls[0], 3.62, -walls[1]],
            ],
            wall,
            false,
        );
        (self.emit)(
            [
                [walls[0], 3.62, walls[1]],
                [0.0, ridge_height, walls[1]],
                [-walls[0], 3.62, walls[1]],
            ],
            wall,
            false,
        );
        self.block([1.63, 4.56, -1.94], [0.40, 0.42], 6.27, STONE);
        self.block([1.63, 6.24, -1.94], [0.51, 0.54], 6.49, TRIM);
    }

    fn panel(
        &mut self,
        half: [f32; 2],
        side: usize,
        across: [f32; 2],
        height: [f32; 2],
        color: Color,
        lamp: bool,
    ) {
        let footprint = ring([0.0, 0.0, 0.0], half);
        let [a, b] = [footprint[side], footprint[(side + 1) % 4]];
        let normal_x = (b[2] - a[2]).clamp(-1.0, 1.0) * 0.082;
        let normal_z = (a[0] - b[0]).clamp(-1.0, 1.0) * 0.082;
        let point = |fraction, y| {
            let [x, _, z] = lerp(a, b, fraction);
            [x + normal_x, y, z + normal_z]
        };
        let [lo, hi] = across;
        let [floor, top] = height;
        self.quad(
            [
                point(lo, floor),
                point(lo, top),
                point(hi, top),
                point(hi, floor),
            ],
            color,
            lamp,
        );
    }

    fn block(&mut self, low_center: Point, half: [f32; 2], top: f32, color: Color) {
        let low = ring(low_center, half);
        let high = ring([low_center[0], top, low_center[2]], half);
        self.shell(low, high, color);
        self.top(high, color);
    }

    fn shell<const N: usize>(&mut self, low: [Point; N], high: [Point; N], color: Color) {
        for i in 0..N {
            let j = (i + 1) % N;
            self.quad([low[i], high[i], high[j], low[j]], color, false);
        }
    }

    fn top(&mut self, [a, b, c, d]: [Point; 4], color: Color) {
        self.quad([d, c, b, a], color, false);
    }

    fn quad(&mut self, [a, b, c, d]: [Point; 4], color: Color, lamp: bool) {
        (self.emit)([a, b, c], color, lamp);
        (self.emit)([a, c, d], color, lamp);
    }
}

fn ring([x, y, z]: Point, [half_x, half_z]: [f32; 2]) -> [Point; 4] {
    [
        [x - half_x, y, z - half_z],
        [x + half_x, y, z - half_z],
        [x + half_x, y, z + half_z],
        [x - half_x, y, z + half_z],
    ]
}

fn lerp(from: Point, to: Point, fraction: f32) -> Point {
    core::array::from_fn(|axis| from[axis] + (to[axis] - from[axis]) * fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_face_fits_the_surveyed_footprint_and_projected_bounds() {
        let center = [-2021.0, 176.7, -229.0];
        let designs = (0..SummitDesign::COUNT).map(|index| {
            (LookoutKind::Landmark(SummitDesign::from_index(index).unwrap()), 150)
        });
        for (kind, max_faces) in [(LookoutKind::Watchtower, 200), (LookoutKind::SummitHut, 88)]
            .into_iter().chain(designs)
        {
            for heading in [0.0, 0.3, 1.57, 3.88, 5.98] {
                for variation in [0, 1, 2, 13] {
                    let art = LookoutArt::new(center, heading, kind, variation);
                    let [min, max] = art.mesh_bounds();
                    let (sin, cos) = sin_cos(heading);
                    let mut faces = 0;
                    art.build(&mut |face, _, _| {
                        faces += 1;
                        for [x, y, z] in face {
                            assert!(x * x + z * z <= 49.0);
                            let point = [
                                center[0] + x * cos + z * sin,
                                center[1] + y,
                                center[2] - x * sin + z * cos,
                            ];
                            for axis in 0..3 {
                                assert!(
                                    point[axis] >= min[axis] - 0.00025
                                        && point[axis] <= max[axis] + 0.00025
                                );
                            }
                        }
                    });
                    assert!(
                        faces > 0 && faces <= max_faces,
                        "{kind:?} triangles: {faces}"
                    );
                }
            }
        }
    }
}
