//! Small trail structures inside a certified mountain rest pad.

use super::{Art, Point};
use crate::world_mesh::WorldMesh;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Form {
    Block,
    Gable,
    Lean,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Material {
    Stone,
    Timber,
    Steel,
    Roof,
    Light,
    Dark,
}

impl Material {
    const fn color(self) -> [u8; 3] {
        match self {
            Self::Stone => [150, 148, 139],
            Self::Timber => [146, 105, 71],
            Self::Steel => [84, 112, 121],
            Self::Roof => [171, 81, 60],
            Self::Light => [218, 187, 130],
            Self::Dark => [92, 92, 88],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Piece {
    form: Form,
    center: Point,
    size: Point,
    material: Material,
}

const fn block(center: Point, size: Point, material: Material) -> Piece {
    Piece {
        form: Form::Block,
        center,
        size,
        material,
    }
}

const fn gable(center: Point, size: Point, material: Material) -> Piece {
    Piece {
        form: Form::Gable,
        center,
        size,
        material,
    }
}

const fn lean(center: Point, size: Point, material: Material) -> Piece {
    Piece {
        form: Form::Lean,
        center,
        size,
        material,
    }
}

use Material::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailRestDesign {
    pub name: &'static str,
    pieces: &'static [Piece],
}

impl TrailRestDesign {
    pub const ALL: [Self; 25] = [
        Self {
            name: "Windward stone shelter",
            pieces: &[
                block([-2.04, -1.10, 0.00], [0.55, 3.80, 3.40], Stone),
                block([2.04, -1.10, 0.00], [0.55, 3.80, 3.40], Stone),
                block([0.00, -1.10, -1.74], [4.63, 3.80, 0.46], Stone),
                gable([0.00, 2.70, 0.00], [5.17, 0.81, 4.24], Roof),
            ],
        },
        Self {
            name: "Split-rail overlook",
            pieces: &[
                block([-2.30, -0.58, -1.55], [0.31, 2.35, 0.31], Timber),
                block([2.30, -0.58, -1.55], [0.31, 2.35, 0.31], Timber),
                block([0.00, 1.36, -1.55], [4.95, 0.23, 0.25], Timber),
                block([0.00, 0.78, -1.55], [4.95, 0.18, 0.25], Timber),
                block([-1.48, -0.70, 0.78], [0.40, 1.45, 0.72], Stone),
                block([1.48, -0.70, 0.78], [0.40, 1.45, 0.72], Stone),
                block([0.00, 0.70, 0.78], [4.35, 0.28, 0.90], Timber),
            ],
        },
        Self {
            name: "Cantilevered map kiosk",
            pieces: &[
                block([-1.62, -0.84, 0.00], [0.25, 3.34, 0.35], Steel),
                block([1.62, -0.84, 0.00], [0.25, 3.34, 0.35], Steel),
                block([0.00, 1.17, 0.13], [3.62, 1.62, 0.22], Timber),
                block([0.00, 1.38, 0.28], [3.22, 1.12, 0.11], Light),
                lean([0.00, 2.92, -0.16], [4.29, 0.50, 1.48], Steel),
            ],
        },
        Self {
            name: "Twin cairn crossing",
            pieces: &[
                block([-1.62, -0.43, -0.64], [1.57, 1.04, 1.57], Stone),
                block([-1.62, 0.54, -0.64], [1.26, 1.37, 1.24], Stone),
                block([1.73, -0.43, 0.47], [1.33, 1.04, 1.33], Stone),
                block([1.73, 0.54, 0.47], [1.06, 1.10, 1.05], Stone),
                block([-1.62, 1.85, -0.64], [0.75, 0.48, 0.66], Light),
                block([1.73, 1.58, 0.47], [0.67, 0.42, 0.66], Dark),
            ],
        },
        Self {
            name: "Leaning ridge canopy",
            pieces: &[
                block([-1.88, -0.64, -0.80], [0.32, 3.25, 0.40], Timber),
                block([1.88, -0.64, -0.80], [0.32, 3.25, 0.40], Timber),
                lean([0.00, 2.61, 0.18], [4.81, 1.28, 3.65], Roof),
                block([-1.07, -0.70, -0.67], [0.40, 1.45, 0.66], Stone),
                block([1.07, -0.70, -0.67], [0.40, 1.45, 0.66], Stone),
                block([0.00, 0.70, -0.67], [3.15, 0.28, 0.82], Timber),
            ],
        },
        Self {
            name: "Horseshoe rest wall",
            pieces: &[
                block([0.00, -0.58, -1.58], [5.00, 1.75, 0.60], Stone),
                block([-2.25, -0.58, -0.21], [0.53, 1.75, 2.83], Stone),
                block([2.25, -0.58, -0.59], [0.53, 1.75, 2.10], Stone),
                block([-1.23, -0.70, -0.92], [0.40, 1.45, 0.67], Stone),
                block([1.23, -0.70, -0.92], [0.40, 1.45, 0.67], Stone),
                block([0.00, 0.70, -0.92], [3.63, 0.28, 0.84], Timber),
            ],
        },
        Self {
            name: "Timber gate and milepost",
            pieces: &[
                block([-1.65, -0.68, 0.00], [0.42, 3.44, 0.43], Timber),
                block([1.65, -0.68, 0.00], [0.42, 3.44, 0.43], Timber),
                block([0.00, 2.57, 0.00], [4.04, 0.35, 0.55], Timber),
                block([0.00, 2.92, 0.00], [1.77, 0.47, 0.28], Light),
                block([2.85, -0.43, 1.28], [0.61, 1.04, 0.61], Stone),
                block([2.85, 0.54, 1.28], [0.49, 1.16, 0.48], Dark),
            ],
        },
        Self {
            name: "Low slate storm refuge",
            pieces: &[
                block([0.00, -0.72, -1.18], [4.56, 2.18, 1.65], Dark),
                block([-1.93, -0.72, 0.17], [0.58, 2.18, 2.75], Stone),
                gable([-0.18, 1.46, -0.10], [4.85, 1.09, 3.81], Steel),
            ],
        },
        Self {
            name: "Trail register on a plinth",
            pieces: &[
                block([0.00, -0.75, 0.00], [2.21, 1.67, 1.75], Stone),
                block([0.00, 0.92, 0.00], [1.58, 1.24, 1.12], Dark),
                lean([0.00, 2.16, 0.10], [1.76, 0.46, 1.24], Timber),
                block([0.08, 1.27, 0.61], [1.13, 0.58, 0.10], Light),
                block([1.65, -0.70, 0.57], [0.40, 1.45, 0.56], Stone),
                block([3.03, -0.70, 0.57], [0.40, 1.45, 0.56], Stone),
                block([2.34, 0.70, 0.57], [2.02, 0.28, 0.70], Timber),
            ],
        },
        Self {
            name: "Stone arches rest stop",
            pieces: &[
                block([-1.73, -0.75, -0.49], [0.55, 3.04, 0.79], Stone),
                block([1.73, -0.75, -0.49], [0.55, 3.04, 0.79], Stone),
                block([-1.27, 1.72, -0.49], [0.54, 0.57, 0.82], Stone),
                block([1.27, 1.72, -0.49], [0.54, 0.57, 0.82], Stone),
                block([0.00, 2.29, -0.49], [2.43, 0.51, 0.87], Light),
                block([0.00, 2.79, -0.49], [1.46, 0.40, 0.91], Stone),
            ],
        },
        Self {
            name: "Two-level viewing decks",
            pieces: &[
                block([-1.48, -0.67, 0.00], [2.55, 1.50, 3.13], Stone),
                block([1.11, -0.67, -0.77], [2.59, 2.06, 2.85], Stone),
                block([1.11, 1.40, -0.77], [2.77, 0.22, 3.04], Timber),
                block([1.17, 1.66, -2.08], [2.68, 0.90, 0.22], Steel),
                block([-1.48, 0.83, 0.00], [2.75, 0.19, 3.29], Timber),
            ],
        },
        Self {
            name: "Red gable information hut",
            pieces: &[
                block([-1.51, -0.70, -0.09], [0.45, 3.11, 2.67], Timber),
                block([1.51, -0.70, -0.09], [0.45, 3.11, 2.67], Timber),
                block([0.00, -0.70, -1.26], [3.34, 3.11, 0.37], Dark),
                gable([0.00, 2.41, -0.10], [4.19, 1.20, 3.53], Roof),
                block([0.81, 0.88, 1.23], [1.07, 1.15, 0.15], Light),
            ],
        },
        Self {
            name: "Split boulder refuge",
            pieces: &[
                block([-1.37, -0.74, 0.00], [2.81, 2.69, 4.12], Stone),
                block([-1.31, 1.93, -0.31], [2.10, 0.78, 2.28], Light),
                block([1.36, -0.59, -0.71], [1.97, 1.95, 3.17], Dark),
                block([0.33, -0.70, 1.27], [0.40, 1.45, 0.54], Stone),
                block([1.61, -0.70, 1.27], [0.40, 1.45, 0.54], Stone),
                block([0.97, 0.70, 1.27], [1.89, 0.28, 0.67], Timber),
            ],
        },
        Self {
            name: "Trailhead compass table",
            pieces: &[
                block([0.00, -0.69, 0.00], [1.71, 1.04, 1.71], Stone),
                block([0.00, 0.35, 0.00], [0.83, 1.02, 0.84], Dark),
                block([0.00, 1.37, 0.00], [2.05, 0.36, 2.05], Light),
                block([0.00, 1.72, 0.00], [0.85, 0.12, 0.13], Steel),
                block([2.76, -0.43, -1.10], [0.56, 1.04, 0.56], Stone),
                block([2.76, 0.54, -1.10], [0.45, 1.16, 0.44], Timber),
            ],
        },
        Self {
            name: "Alpine emergency cache",
            pieces: &[
                block([-1.21, -0.53, -0.68], [3.29, 1.68, 2.08], Stone),
                block([-1.21, 1.05, -0.68], [2.89, 1.42, 1.64], Roof),
                lean([-1.21, 2.44, -0.68], [3.10, 0.55, 1.83], Dark),
                block([-1.63, 1.40, 0.20], [1.01, 0.43, 0.12], Light),
                block([1.77, -0.43, 0.90], [0.80, 1.04, 0.80], Stone),
                block([1.77, 0.54, 0.90], [0.64, 1.80, 0.63], Steel),
            ],
        },
        Self {
            name: "Open log shelter",
            pieces: &[
                block([-1.74, -0.63, -1.20], [0.42, 3.30, 0.42], Timber),
                block([1.74, -0.63, -1.20], [0.42, 3.30, 0.42], Timber),
                block([-1.74, -0.63, 1.13], [0.42, 3.30, 0.42], Timber),
                block([1.74, -0.63, 1.13], [0.42, 3.30, 0.42], Timber),
                gable([0.00, 2.67, 0.00], [4.71, 1.04, 3.27], Timber),
                block([-1.14, -0.70, -0.79], [0.40, 1.45, 0.54], Stone),
                block([1.14, -0.70, -0.79], [0.40, 1.45, 0.54], Stone),
                block([0.00, 0.70, -0.79], [3.36, 0.28, 0.68], Timber),
            ],
        },
        Self {
            name: "Ridge direction mast",
            pieces: &[
                block([0.00, -0.76, 0.00], [0.42, 4.68, 0.42], Timber),
                block([0.80, 2.71, -0.04], [2.21, 0.41, 0.29], Roof),
                block([-0.64, 1.91, 0.13], [1.88, 0.42, 0.28], Light),
                block([0.05, 1.21, 0.77], [0.26, 0.42, 1.63], Steel),
                block([0.47, -0.70, 1.47], [0.40, 1.45, 0.52], Stone),
                block([2.09, -0.70, 1.47], [0.40, 1.45, 0.52], Stone),
                block([1.28, 0.70, 1.47], [2.38, 0.28, 0.65], Timber),
            ],
        },
        Self {
            name: "Lichen stone stair and bench",
            pieces: &[
                block([-1.11, -0.54, 1.20], [3.21, 0.76, 1.31], Stone),
                block([-1.11, -0.54, 0.06], [3.21, 1.18, 1.24], Dark),
                block([-1.11, -0.54, -1.10], [3.21, 1.62, 1.31], Stone),
                block([-1.11, 1.04, -1.45], [3.02, 0.21, 0.51], Light),
                block([1.53, -0.70, -0.18], [0.40, 1.45, 0.58], Stone),
                block([2.43, -0.70, -0.18], [0.40, 1.45, 0.58], Stone),
                block([1.98, 0.70, -0.18], [1.31, 0.28, 0.72], Timber),
            ],
        },
        Self {
            name: "Sheltered double bench",
            pieces: &[
                block([-2.44, -0.70, 0.33], [0.40, 1.45, 0.72], Stone),
                block([-0.82, -0.70, 0.33], [0.40, 1.45, 0.72], Stone),
                block([-1.63, 0.70, 0.33], [2.38, 0.28, 0.90], Timber),
                block([0.82, -0.70, 0.33], [0.40, 1.45, 0.72], Stone),
                block([2.44, -0.70, 0.33], [0.40, 1.45, 0.72], Stone),
                block([1.63, 0.70, 0.33], [2.38, 0.28, 0.90], Timber),
                block([0.00, -0.72, -1.17], [5.78, 2.87, 0.49], Stone),
                block([0.00, 2.04, -1.17], [5.64, 0.35, 0.63], Dark),
                block([0.00, -0.54, 0.25], [0.44, 1.98, 2.92], Stone),
            ],
        },
        Self {
            name: "Asymmetric rain porch",
            pieces: &[
                block([-1.90, -0.76, -0.95], [0.51, 3.43, 0.59], Timber),
                block([1.55, -0.76, -0.95], [0.55, 3.43, 0.59], Timber),
                block([1.55, -0.76, 1.04], [0.55, 3.43, 0.59], Timber),
                lean([-0.12, 2.68, 0.03], [4.95, 0.98, 3.44], Steel),
                block([1.57, -0.65, 0.05], [0.35, 2.81, 2.28], Timber),
            ],
        },
        Self {
            name: "Stone windbreak zigzag",
            pieces: &[
                block([-1.72, -0.72, 0.54], [2.55, 2.19, 0.47], Stone),
                block([-0.48, -0.72, -0.40], [0.53, 2.19, 2.29], Light),
                block([0.89, -0.72, -1.33], [2.51, 2.19, 0.47], Stone),
                block([1.99, -0.72, -0.26], [0.50, 1.62, 2.45], Dark),
            ],
        },
        Self {
            name: "Alpine survey platform",
            pieces: &[
                block([0.00, -0.62, 0.00], [4.89, 1.44, 3.09], Stone),
                block([0.00, 0.80, 0.00], [4.97, 0.29, 3.17], Timber),
                block([-1.68, 0.91, 0.31], [0.20, 1.30, 0.20], Steel),
                block([-1.94, 2.17, 0.40], [1.21, 0.25, 0.22], Steel),
                block([1.80, 0.91, -1.01], [0.21, 1.42, 0.20], Steel),
                block([1.80, 2.28, -1.02], [0.96, 0.27, 0.33], Light),
            ],
        },
        Self {
            name: "Stone drum and guide flag",
            pieces: &[
                block([0.00, -0.73, 0.00], [2.36, 1.74, 1.49], Stone),
                block([0.00, -0.72, 0.00], [1.54, 1.86, 2.38], Stone),
                block([0.00, 1.06, 0.00], [1.61, 0.71, 1.57], Light),
                block([0.00, 1.65, 0.00], [0.19, 2.06, 0.22], Dark),
                block([0.61, 3.15, 0.00], [1.31, 0.50, 0.16], Roof),
            ],
        },
        Self {
            name: "Bridge-side rest table",
            pieces: &[
                block([-2.74, -0.70, 0.00], [0.40, 1.45, 0.71], Stone),
                block([-1.06, -0.70, 0.00], [0.40, 1.45, 0.71], Stone),
                block([-1.90, 0.70, 0.00], [2.47, 0.28, 0.89], Timber),
                block([1.06, -0.70, 0.00], [0.40, 1.45, 0.71], Stone),
                block([2.74, -0.70, 0.00], [0.40, 1.45, 0.71], Stone),
                block([1.90, 0.70, 0.00], [2.47, 0.28, 0.89], Timber),
                block([0.00, -0.62, 0.00], [0.47, 2.02, 0.43], Stone),
                block([0.00, 1.39, 0.00], [1.38, 0.26, 2.61], Timber),
                block([0.00, -0.65, -1.81], [6.04, 1.62, 0.32], Steel),
            ],
        },
        Self {
            name: "Twin roofs on a trail saddle",
            pieces: &[
                block([-1.45, -0.60, -0.73], [2.48, 2.85, 0.42], Dark),
                block([1.43, -0.60, 0.74], [2.52, 2.85, 0.42], Timber),
                gable([-1.36, 2.23, -0.72], [2.72, 0.54, 2.16], Roof),
                gable([1.37, 2.23, 0.72], [2.71, 0.77, 2.13], Steel),
                block([0.00, -0.58, 0.00], [0.26, 2.84, 0.29], Stone),
            ],
        },
    ];

    pub(super) fn index_for(variant: u32) -> u8 {
        let draw = crate::math::mix(u64::from(variant) ^ 0x19ec_bb65);
        (draw % Self::ALL.len() as u64) as u8
    }

    pub fn draw(self, mesh: &mut WorldMesh<'_>, point: Point, front: [f32; 2]) {
        let mut art = Art { mesh, point, front };
        for piece in self.pieces {
            let [width, height, depth] = piece.size;
            let color = piece.material.color();
            match piece.form {
                Form::Block => art.block(piece.center, piece.size, color),
                Form::Gable => art.roof(piece.center, width * 0.5, depth * 0.5, height, color),
                Form::Lean => {
                    let [x, y, z] = piece.center;
                    let (w, d) = (width * 0.5, depth * 0.5);
                    art.polygon(
                        [
                            [x - w, y, z - d],
                            [x + w, y, z - d],
                            [x + w, y + height, z + d],
                            [x - w, y + height, z + d],
                        ],
                        color,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trail_rests_have_distinct_geometry_and_fit_the_surveyed_pad() {
        for (index, design) in TrailRestDesign::ALL.iter().enumerate() {
            for piece in design.pieces {
                let [x, y, z] = piece.center;
                let [w, h, d] = piece.size;
                let dx = x.abs() + w * 0.5;
                let dz = z.abs() + d * 0.5;
                assert!(
                    dx * dx + dz * dz < super::super::REST_RADIUS.powi(2),
                    "{}",
                    design.name
                );
                assert!(y + h <= 4.2 && y >= -1.7, "{}", design.name);
            }
            assert!(
                TrailRestDesign::ALL[..index].iter().all(|previous| {
                    let same_geometry = previous.pieces.len() == design.pieces.len()
                        && previous.pieces.iter().zip(design.pieces).all(|(a, b)| {
                            a.center == b.center && a.size == b.size && a.form == b.form
                        });
                    !same_geometry
                }),
                "{}",
                design.name
            );
        }
    }
}
