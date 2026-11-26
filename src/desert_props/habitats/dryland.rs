//! Local erosion and plant forms for inland dry terrain.

use super::Mesh;
use crate::world::{Sample, Surface};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum DrylandFeature {
    PiledWashCobbles,
    DryRootFan,
    GravelRibs,
    CutbankShelf,
    DriftBranch,
    DuneSlipFace,
    RippleField,
    SandGrassLine,
    DuneSkirt,
    SandVerbenaMat,
    SandstonePedestal,
    TwinSandstoneFins,
    BandedSandstone,
    BasaltTalus,
    DesertPavement,
    CreosoteRing,
    SaltbushHummocks,
    Brittlebush,
    Blackbrush,
    MojaveAster,
    SpanishBayonet,
    AgaveStalk,
    ChollaSkeleton,
    BeavertailClump,
    BarrelCactusGroup,
}

impl DrylandFeature {
    pub(super) fn choose(ground: Sample, key: u64, district: u64) -> Option<Self> {
        use DrylandFeature::*;
        const WASH: [DrylandFeature; 5] = [
            PiledWashCobbles,
            DryRootFan,
            GravelRibs,
            CutbankShelf,
            DriftBranch,
        ];
        const DUNE: [DrylandFeature; 5] = [
            DuneSlipFace,
            RippleField,
            SandGrassLine,
            DuneSkirt,
            SandVerbenaMat,
        ];
        const STONE: [DrylandFeature; 5] = [
            SandstonePedestal,
            TwinSandstoneFins,
            BandedSandstone,
            BasaltTalus,
            DesertPavement,
        ];
        const SCRUB: [DrylandFeature; 5] = [
            CreosoteRing,
            SaltbushHummocks,
            Brittlebush,
            Blackbrush,
            MojaveAster,
        ];
        const SUCCULENT: [DrylandFeature; 5] = [
            SpanishBayonet,
            AgaveStalk,
            ChollaSkeleton,
            BeavertailClump,
            BarrelCactusGroup,
        ];

        let forms = match ground.surface {
            Surface::DryWash => &WASH,
            Surface::DuneSand if ground.elevation >= 1.4 => &DUNE,
            Surface::Desert => match (district >> 26) % 3 {
                0 => &STONE,
                1 => &SCRUB,
                _ => &SUCCULENT,
            },
            _ => return None,
        };
        let index = if ground.surface != Surface::Desert || key >> 35 & 3 == 0 {
            key >> 37
        } else {
            (district >> 10) + (key >> 44 & 1)
        };
        Some(forms[(index % 5) as usize])
    }

    pub(super) fn draw(self, m: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        use DrylandFeature::*;
        const SAND: [u8; 3] = [193, 159, 111];
        const PEBBLE: [u8; 3] = [154, 137, 115];
        const RED: [u8; 3] = [179, 104, 71];
        const BASALT: [u8; 3] = [93, 83, 77];
        const WOOD: [u8; 3] = [132, 107, 79];
        const LEAF: [u8; 3] = [103, 129, 80];
        const DRY: [u8; 3] = [162, 147, 101];
        const SAGE: [u8; 3] = [129, 148, 103];

        match self {
            PiledWashCobbles => {
                for [x, z, w, h] in [
                    [-1.00, 0.34, 0.46, 0.36],
                    [-0.53, -0.12, 0.65, 0.56],
                    [0.12, 0.28, 0.54, 0.69],
                    [0.76, -0.18, 0.47, 0.43],
                    [1.12, 0.28, 0.33, 0.31],
                ] {
                    m.stone([x, 0.0, z], [w, h, w * 0.78], PEBBLE);
                }
            }
            DryRootFan => {
                m.wood([-0.28, 0.0, 0.36], [0.11, 0.92, -0.28], 0.26, WOOD);
                for point in [
                    [-1.12, 0.0, -0.43],
                    [-1.39, 0.51, 0.09],
                    [-0.46, 1.17, -0.53],
                    [0.57, 1.06, 0.04],
                    [1.06, 0.0, 0.56],
                    [1.04, 0.42, -0.52],
                ] {
                    m.wood([-0.18, 0.45, 0.07], point, 0.115, DRY);
                }
            }
            GravelRibs => {
                for [x, z, w, h] in [
                    [-0.53, -0.56, 0.78, 0.24],
                    [-0.17, -0.20, 1.09, 0.33],
                    [0.17, 0.23, 1.35, 0.39],
                    [0.42, 0.64, 1.10, 0.27],
                ] {
                    m.stone([x, 0.0, z], [w, h, 0.24], PEBBLE);
                }
            }
            CutbankShelf => {
                m.stone([0.0, 0.0, 0.02], [1.42, 0.75, 0.94], SAND);
                m.block([0.15, 0.38, -0.25], [1.18, 0.35, 0.54], RED);
                m.stone([0.24, 0.66, -0.21], [1.14, 0.47, 0.52], [205, 166, 118]);
                m.stone([-0.70, 0.0, 0.65], [0.48, 0.28, 0.29], PEBBLE);
            }
            DriftBranch => {
                m.wood([-1.60, 0.0, 0.11], [-0.44, 0.31, -0.20], 0.16, WOOD);
                m.wood([-0.44, 0.31, -0.20], [0.78, 0.46, 0.28], 0.16, DRY);
                m.wood([0.43, 0.38, 0.12], [1.63, 0.14, 0.85], 0.087, DRY);
                m.wood([0.49, 0.44, 0.15], [1.04, 0.83, -0.47], 0.10, WOOD);
                m.wood([-0.75, 0.23, -0.16], [-0.43, 0.49, -0.90], 0.075, DRY);
            }
            DuneSlipFace => {
                let [left, right, ridge, tail] = [
                    [-1.52, 0.0, -0.38],
                    [1.39, 0.0, -0.49],
                    [0.10, 1.0, 0.31],
                    [0.05, 0.0, 1.03],
                ];
                m.quad(
                    [left, [-0.92, 0.23, -0.76], [0.93, 0.35, -0.78], right],
                    SAND,
                );
                (m.0)([left, right, ridge], SAND);
                (m.0)([left, ridge, tail], [171, 132, 85]);
                (m.0)([right, tail, ridge], [204, 168, 119]);
            }
            RippleField => {
                for [x, h, z, w] in [
                    [-0.32, 0.22, -0.63, 1.21],
                    [-0.09, 0.30, -0.19, 1.43],
                    [0.21, 0.24, 0.28, 1.33],
                    [0.44, 0.15, 0.72, 1.02],
                ] {
                    m.stone([x, 0.0, z], [w, h, 0.16], SAND);
                }
            }
            SandGrassLine => {
                for [x, z, h] in [
                    [-1.32, 0.18, 0.72],
                    [-0.68, -0.19, 1.06],
                    [0.08, -0.30, 1.22],
                    [0.69, -0.23, 0.82],
                    [1.24, 0.01, 0.62],
                ] {
                    m.rosette([x, 0.0, z], 0.48, h, DRY);
                    m.stalk([x, 0.0, z], [x + 0.29, h * 1.22, z], 0.065, SAGE);
                }
            }
            DuneSkirt => {
                m.stone([-0.21, 0.0, 0.0], [1.38, 0.49, 0.72], SAND);
                m.stone([0.47, 0.14, 0.05], [1.11, 0.88, 0.56], [210, 171, 112]);
                m.stone([0.82, 0.0, 0.69], [0.63, 0.28, 0.41], SAND);
                m.rosette([0.91, 0.56, -0.03], 0.34, 0.49, DRY);
            }
            SandVerbenaMat => {
                for [x, z, w] in [
                    [-0.86, 0.22, 0.45],
                    [-0.32, -0.38, 0.54],
                    [0.31, 0.15, 0.61],
                    [0.91, 0.41, 0.43],
                ] {
                    m.stone([x, 0.0, z], [w, w * 0.55, w], [113, 132, 86]);
                    m.flower([x - 0.14, w * 0.48, z + 0.12], 0.18, [167, 131, 186]);
                    m.flower([x + 0.16, w * 0.43, z - 0.10], 0.15, [184, 147, 193]);
                }
            }
            SandstonePedestal => {
                m.stone([0.0, 0.0, 0.04], [0.70, 1.79, 0.61], RED);
                m.stone([0.14, 1.36, -0.13], [1.01, 0.67, 0.81], [116, 87, 72]);
                m.stone([-0.65, 0.0, 0.44], [0.42, 0.33, 0.38], SAND);
            }
            TwinSandstoneFins => {
                m.gable([-0.56, 0.0, -0.09], [0.44, 2.44, 1.12], RED);
                m.gable([0.56, 0.0, 0.25], [0.46, 1.53, 0.88], [200, 134, 88]);
                m.stone([0.10, 0.0, 0.64], [0.76, 0.33, 0.47], SAND);
            }
            BandedSandstone => {
                m.stone([0.0, 0.0, 0.04], [1.21, 0.97, 0.74], RED);
                m.stone([0.24, 0.49, -0.08], [1.04, 0.53, 0.62], SAND);
                m.stone([0.33, 0.82, -0.10], [0.85, 0.70, 0.56], RED);
                m.stone([0.10, 1.16, -0.14], [0.67, 0.46, 0.42], [205, 161, 105]);
            }
            BasaltTalus => {
                for [x, z, w, h] in [
                    [-1.00, 0.30, 0.53, 0.44],
                    [-0.41, -0.27, 0.66, 0.95],
                    [0.24, 0.14, 0.73, 1.17],
                    [0.78, -0.32, 0.53, 0.66],
                    [1.18, 0.47, 0.30, 0.29],
                ] {
                    m.stone([x, 0.0, z], [w, h, w * 0.74], BASALT);
                }
            }
            DesertPavement => {
                for [x, z, w, h] in [
                    [-0.73, -0.48, 0.65, 0.24],
                    [-0.55, 0.43, 0.70, 0.28],
                    [0.14, -0.18, 0.64, 0.38],
                    [0.78, 0.28, 0.53, 0.24],
                    [0.93, -0.51, 0.38, 0.19],
                ] {
                    m.stone([x, 0.0, z], [w, h, w * 0.67], [117, 95, 78]);
                }
            }
            CreosoteRing => {
                for [x, z, h] in [
                    [-0.94, 0.05, 0.88],
                    [-0.44, -0.56, 1.11],
                    [0.29, -0.55, 1.03],
                    [0.77, 0.14, 0.80],
                    [0.37, 0.62, 0.69],
                ] {
                    m.wood([x, 0.0, z], [x * 1.16, h, z], 0.10, WOOD);
                    m.rosette([x, h * 0.70, z], 0.49, 0.37, LEAF);
                    m.flower([x + 0.15, h, z], 0.12, [213, 171, 79]);
                }
            }
            SaltbushHummocks => {
                for [x, z, w, h] in [
                    [-0.60, 0.26, 0.69, 0.58],
                    [0.03, -0.27, 0.67, 0.86],
                    [0.69, 0.18, 0.61, 0.53],
                ] {
                    m.stone([x, 0.0, z], [w, h, w * 0.81], SAGE);
                    m.rosette([x, h * 0.56, z], w * 0.7, h * 0.47, [153, 161, 113]);
                }
            }
            Brittlebush => {
                m.rosette([0.0, 0.0, 0.0], 0.94, 0.51, SAGE);
                for [x, z, h] in [
                    [-0.74, 0.16, 0.71],
                    [-0.35, -0.42, 0.96],
                    [0.14, 0.25, 1.15],
                    [0.60, -0.06, 0.85],
                ] {
                    m.stalk([x * 0.28, 0.0, z * 0.28], [x, h, z], 0.082, LEAF);
                    m.flower([x, h, z], 0.24, [229, 184, 79]);
                }
            }
            Blackbrush => {
                for [x, z, h] in [
                    [-0.77, 0.29, 0.85],
                    [-0.29, -0.39, 1.19],
                    [0.33, 0.30, 0.93],
                    [0.77, -0.10, 0.70],
                ] {
                    m.wood([x * 0.25, 0.0, z * 0.34], [x, h, z], 0.13, [83, 73, 61]);
                    m.rosette([x, h * 0.62, z], 0.55, 0.37, [102, 107, 75]);
                    m.stone([x, h * 0.68, z], [0.34, 0.27, 0.38], [125, 119, 80]);
                }
            }
            MojaveAster => {
                for [x, z, h] in [
                    [-0.75, 0.19, 0.61],
                    [-0.33, -0.44, 0.81],
                    [0.02, 0.30, 1.05],
                    [0.42, -0.12, 0.75],
                    [0.72, 0.45, 0.62],
                ] {
                    m.rosette([x, 0.0, z], 0.34, 0.22, LEAF);
                    m.stalk([x, 0.0, z], [x + 0.12, h, z], 0.068, SAGE);
                    m.flower([x + 0.12, h, z], 0.24, [177, 139, 183]);
                    m.flower([x + 0.12, h + 0.05, z], 0.09, [228, 185, 78]);
                }
            }
            SpanishBayonet => {
                m.rosette([-0.13, 0.0, 0.07], 1.14, 1.04, [105, 136, 91]);
                m.rosette([0.51, 0.0, -0.23], 0.66, 0.65, SAGE);
                m.stalk([0.0, 0.0, 0.0], [0.07, 2.27, 0.02], 0.11, DRY);
                m.stone([0.07, 1.67, 0.02], [0.33, 0.72, 0.27], [217, 204, 157]);
            }
            AgaveStalk => {
                m.rosette([0.0, 0.0, 0.0], 1.01, 0.67, [103, 149, 128]);
                m.wood([0.0, 0.0, 0.0], [0.14, 2.87, 0.11], 0.13, WOOD);
                for [x, z, h] in [
                    [-0.59, 0.24, 1.83],
                    [0.62, -0.17, 2.11],
                    [-0.35, -0.27, 2.47],
                    [0.45, 0.26, 2.67],
                ] {
                    m.wood([0.10, h - 0.40, 0.07], [x, h, z], 0.083, DRY);
                    m.flower([x, h, z], 0.17, [211, 180, 93]);
                }
            }
            ChollaSkeleton => {
                m.wood([-0.06, 0.0, -0.01], [0.06, 1.82, 0.09], 0.20, DRY);
                for [x, z, h] in [
                    [-0.65, 0.20, 1.41],
                    [0.62, 0.23, 1.52],
                    [-0.41, -0.43, 1.94],
                    [0.26, -0.14, 2.27],
                ] {
                    m.wood([0.02, h * 0.55, 0.02], [x, h, z], 0.13, OLD_CHOLLA);
                    m.wood([x, h, z], [x * 1.09, h + 0.32, z - 0.12], 0.09, DRY);
                }
            }
            BeavertailClump => {
                for [x, z, h] in [
                    [-0.72, 0.11, 0.51],
                    [-0.30, -0.35, 0.74],
                    [0.30, 0.30, 0.88],
                    [0.68, -0.16, 0.63],
                ] {
                    m.stone([x, 0.0, z], [0.34, h, 0.20], [111, 151, 115]);
                    m.stone([x + 0.17, h * 0.57, z - 0.06], [0.27, h * 0.58, 0.16], SAGE);
                }
                m.flower([0.48, 0.97, 0.25], 0.19, [193, 98, 141]);
            }
            BarrelCactusGroup => {
                for [x, z, r, h] in [
                    [-0.51, 0.12, 0.40, 0.76],
                    [0.29, -0.26, 0.49, 1.16],
                    [0.68, 0.48, 0.36, 0.63],
                ] {
                    m.drum([x, 0.0, z], r, h, LEAF, false);
                    m.stone([x, h - 0.17, z], [r * 0.95, 0.34, r * 0.95], SAGE);
                    m.flower([x, h + 0.13, z], r * 0.48, [182, 153, 83]);
                }
            }
        }
    }

    pub(crate) fn scale(self) -> f32 {
        use DrylandFeature::*;
        match self {
            SandstonePedestal | TwinSandstoneFins | BandedSandstone | BasaltTalus
            | CutbankShelf => 1.40,
            _ => 1.12,
        }
    }
}

const OLD_CHOLLA: [u8; 3] = [146, 124, 94];
