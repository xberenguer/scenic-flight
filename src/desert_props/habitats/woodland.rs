//! Understory forms for seeded groves and forest margins.

use super::Mesh;
use crate::world::{Sample, Surface};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum WoodlandFeature {
    SpruceSeedling,
    HemlockWhorls,
    YewBush,
    HazelCoppice,
    BirchSaplings,
    OstrichFern,
    WoodSedge,
    BroadleafHellebore,
    FoamflowerMat,
    EvergreenHolly,
    NurseLog,
    SplitStump,
    WindthrowPit,
    BarkHeap,
    BranchTangle,
    ShelfFungi,
    ParasolPatch,
    MorelCluster,
    PuffballRing,
    CoralFungi,
    BrackenFronds,
    Foxglove,
    RoseHipBramble,
    BilberryMound,
    HoneysuckleArch,
}

const SAPLINGS: [WoodlandFeature; 5] = [
    WoodlandFeature::SpruceSeedling,
    WoodlandFeature::HemlockWhorls,
    WoodlandFeature::YewBush,
    WoodlandFeature::HazelCoppice,
    WoodlandFeature::BirchSaplings,
];
const SHADE: [WoodlandFeature; 5] = [
    WoodlandFeature::OstrichFern,
    WoodlandFeature::WoodSedge,
    WoodlandFeature::BroadleafHellebore,
    WoodlandFeature::FoamflowerMat,
    WoodlandFeature::EvergreenHolly,
];
const DEADWOOD: [WoodlandFeature; 5] = [
    WoodlandFeature::NurseLog,
    WoodlandFeature::SplitStump,
    WoodlandFeature::WindthrowPit,
    WoodlandFeature::BarkHeap,
    WoodlandFeature::BranchTangle,
];
const FUNGI: [WoodlandFeature; 5] = [
    WoodlandFeature::ShelfFungi,
    WoodlandFeature::ParasolPatch,
    WoodlandFeature::MorelCluster,
    WoodlandFeature::PuffballRing,
    WoodlandFeature::CoralFungi,
];
const CLEARING: [WoodlandFeature; 5] = [
    WoodlandFeature::BrackenFronds,
    WoodlandFeature::Foxglove,
    WoodlandFeature::RoseHipBramble,
    WoodlandFeature::BilberryMound,
    WoodlandFeature::HoneysuckleArch,
];

impl WoodlandFeature {
    pub(super) fn choose(ground: Sample, key: u64, district: u64) -> Option<Self> {
        if ground.surface != Surface::Forest {
            return None;
        }
        let forms = match (district >> 27) % 5 {
            0 => &SAPLINGS,
            1 => &SHADE,
            2 => &DEADWOOD,
            // Keep fungi near the shaded woodland floor and taller shrubs in upland groves.
            3 if ground.elevation < 48.0 => &FUNGI,
            _ => &CLEARING,
        };
        let shape = if key >> 32 & 3 == 0 {
            key >> 35
        } else {
            (district >> 17) + (key >> 36 & 1)
        };
        Some(forms[(shape % 5) as usize])
    }

    pub(super) fn draw(self, m: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        use WoodlandFeature::*;
        const WOOD: [u8; 3] = [116, 92, 68];
        const OLD: [u8; 3] = [155, 132, 105];
        const LEAF: [u8; 3] = [82, 128, 70];
        const MOSS: [u8; 3] = [116, 144, 79];
        const CONIFER: [u8; 3] = [61, 106, 78];
        const CREAM: [u8; 3] = [225, 207, 165];

        match self {
            SpruceSeedling => {
                m.wood([0.0, 0.0, 0.0], [0.12, 2.70, 0.0], 0.12, WOOD);
                for [y, w] in [[0.32, 0.92], [0.93, 0.71], [1.48, 0.51], [1.96, 0.34]] {
                    m.gable([0.07, y, 0.0], [w, w * 1.4, w * 0.82], CONIFER);
                    m.rosette([0.07, y, 0.0], w, 0.44, LEAF);
                }
            }
            HemlockWhorls => {
                m.wood([-0.12, 0.0, 0.01], [0.38, 2.47, 0.08], 0.13, WOOD);
                for [x, y, z, span] in [
                    [-0.29, 0.75, 0.16, 1.0],
                    [0.18, 1.17, -0.12, 0.75],
                    [0.29, 1.67, 0.06, 0.54],
                    [0.48, 2.13, 0.14, 0.32],
                ] {
                    m.wood([0.0, y - 0.16, 0.0], [x, y, z], 0.09, WOOD);
                    m.rosette([x, y, z], span, 0.22, [78, 114, 88]);
                }
            }
            YewBush => {
                for [x, y, z, w] in [
                    [-0.65, 0.48, 0.21, 0.64],
                    [-0.19, 0.99, -0.23, 0.69],
                    [0.40, 0.85, 0.22, 0.60],
                    [0.89, 0.34, -0.03, 0.49],
                ] {
                    m.wood([0.0, 0.0, 0.0], [x, y, z], 0.13, WOOD);
                    m.stone([x, y, z], [w, w * 0.87, w * 0.81], [59, 103, 74]);
                }
                m.flower([-0.30, 1.51, -0.05], 0.15, [185, 71, 62]);
                m.flower([0.65, 1.17, 0.26], 0.13, [190, 81, 62]);
            }
            HazelCoppice => {
                m.stone([0.0, 0.0, 0.0], [0.64, 0.42, 0.52], OLD);
                for [x, y, z] in [
                    [-0.75, 1.56, 0.36],
                    [-0.32, 2.19, -0.42],
                    [0.36, 1.86, 0.35],
                    [0.83, 1.33, -0.22],
                ] {
                    m.wood([x * 0.25, 0.18, z * 0.25], [x, y, z], 0.11, WOOD);
                    m.rosette([x, y, z], 0.59, 0.43, MOSS);
                }
            }
            BirchSaplings => {
                for [x, z, h] in [
                    [-0.64, 0.13, 2.52],
                    [-0.04, -0.31, 2.85],
                    [0.66, 0.29, 2.22],
                ] {
                    m.wood([x, 0.0, z], [x + 0.22, h, z], 0.14, [197, 189, 172]);
                    m.wood(
                        [x + 0.11, h * 0.50, z],
                        [x - 0.21, h * 0.78, z + 0.36],
                        0.09,
                        OLD,
                    );
                    m.stone([x + 0.16, h - 0.59, z], [0.39, 0.73, 0.44], [125, 155, 85]);
                }
            }
            OstrichFern => {
                for [x, z, h] in [
                    [-0.64, 0.48, 1.22],
                    [-0.70, -0.31, 1.46],
                    [0.04, -0.65, 1.74],
                    [0.64, 0.12, 1.38],
                    [0.27, 0.64, 1.54],
                ] {
                    m.stalk([0.0, 0.0, 0.0], [x, h, z], 0.18, LEAF);
                    m.stalk(
                        [x * 0.67, h * 0.63, z * 0.67],
                        [x * 1.48, h * 0.85, z * 1.48],
                        0.17,
                        MOSS,
                    );
                }
            }
            WoodSedge => {
                for [x, z, h] in [
                    [-0.80, 0.32, 0.86],
                    [-0.28, -0.41, 1.16],
                    [0.43, 0.26, 0.54],
                    [0.92, -0.06, 0.72],
                ] {
                    m.rosette([x, 0.0, z], 0.53, h, [133, 150, 89]);
                    m.stalk([x, 0.0, z], [x + 0.38, h * 1.32, z], 0.08, MOSS);
                }
            }
            BroadleafHellebore => {
                for [x, z, h] in [[-0.58, 0.12, 0.85], [0.09, -0.41, 1.14], [0.54, 0.42, 0.73]] {
                    m.rosette([x, 0.0, z], 0.67, h, LEAF);
                    m.stalk([x, 0.0, z], [x, h, z], 0.13, MOSS);
                    m.flower([x, h, z], 0.28, [183, 157, 152]);
                }
            }
            FoamflowerMat => {
                for [x, z, h] in [
                    [-0.92, 0.02, 0.47],
                    [-0.43, -0.34, 0.60],
                    [0.04, 0.20, 0.67],
                    [0.54, 0.49, 0.44],
                    [0.88, -0.14, 0.52],
                ] {
                    m.stone([x, 0.0, z], [0.50, 0.24, 0.41], LEAF);
                    m.stalk([x, 0.15, z], [x, h, z], 0.06, MOSS);
                    m.stone([x, h - 0.13, z], [0.14, 0.27, 0.12], CREAM);
                }
            }
            EvergreenHolly => {
                m.wood([0.0, 0.0, 0.0], [0.02, 1.65, -0.09], 0.16, WOOD);
                for [x, y, z, w] in [
                    [-0.53, 0.45, 0.15, 0.66],
                    [0.51, 0.61, 0.28, 0.60],
                    [0.0, 1.28, -0.05, 0.72],
                ] {
                    m.stone([x, y, z], [w, 0.69, w * 0.91], CONIFER);
                    m.flower([x + 0.19, y + 0.54, z + 0.27], 0.17, [196, 72, 60]);
                }
            }
            NurseLog => {
                m.wood([-1.66, 0.12, -0.33], [1.60, 0.23, 0.32], 0.33, WOOD);
                m.stone([0.0, 0.0, 0.0], [1.28, 0.38, 0.42], MOSS);
                m.flower([1.60, 0.26, 0.31], 0.29, OLD);
                for [x, h] in [[-0.67, 0.94], [0.18, 1.35], [0.73, 0.87]] {
                    m.stalk([x, 0.21, 0.03], [x + 0.14, h, 0.03], 0.10, WOOD);
                    m.rosette([x + 0.12, h * 0.8, 0.03], 0.38, 0.34, CONIFER);
                }
            }
            SplitStump => {
                m.drum([0.0, 0.0, 0.0], 0.64, 0.70, WOOD, false);
                m.wood([-0.33, 0.54, 0.04], [-0.48, 1.79, 0.07], 0.21, OLD);
                m.wood([0.28, 0.54, -0.10], [0.39, 1.12, -0.37], 0.21, WOOD);
                m.stalk([0.0, 0.22, 0.0], [-0.79, 0.0, 0.69], 0.31, OLD);
            }
            WindthrowPit => {
                m.stone([-0.32, 0.0, 0.17], [0.98, 1.22, 0.49], [112, 96, 76]);
                m.wood([-0.35, 0.53, 0.13], [1.67, 0.21, -0.33], 0.22, OLD);
                for root in [
                    [-1.04, 1.39, 0.0],
                    [-0.37, 1.58, 0.05],
                    [0.37, 1.15, 0.62],
                    [-0.25, 0.0, 1.03],
                ] {
                    m.wood([-0.38, 0.67, 0.24], root, 0.08, OLD);
                }
            }
            BarkHeap => {
                for [x, z, w, h] in [
                    [-0.72, 0.15, 0.64, 0.46],
                    [0.04, -0.34, 0.66, 0.77],
                    [0.53, 0.37, 0.76, 0.62],
                    [1.01, -0.31, 0.45, 0.29],
                ] {
                    m.stone([x, 0.0, z], [w, h, w * 0.58], WOOD);
                }
                m.wood([-0.72, 0.29, -0.32], [0.95, 0.54, 0.27], 0.14, OLD);
            }
            BranchTangle => {
                for [x, h, z] in [
                    [-1.30, 0.92, 0.12],
                    [-0.60, 0.0, -0.77],
                    [0.06, 1.31, -0.59],
                    [0.89, 0.55, 0.62],
                    [1.42, 0.0, -0.35],
                ] {
                    m.wood([x * -0.29, 0.11, z * -0.42], [x, h, z], 0.12, OLD);
                    m.wood(
                        [x * 0.53, h * 0.62, z * 0.53],
                        [x * 0.74, h + 0.26, z - 0.25],
                        0.063,
                        OLD,
                    );
                }
            }
            ShelfFungi => {
                m.wood([-0.20, 0.0, 0.0], [0.10, 1.73, -0.10], 0.38, WOOD);
                for [x, h, z, w] in [
                    [0.36, 0.52, 0.19, 0.40],
                    [-0.38, 0.98, 0.10, 0.36],
                    [0.28, 1.31, -0.28, 0.33],
                ] {
                    m.stone([x, h, z], [w, 0.16, w * 0.63], CREAM);
                }
            }
            ParasolPatch => {
                for [x, z, h, w] in [
                    [-0.67, 0.27, 0.86, 0.49],
                    [0.0, -0.37, 1.20, 0.55],
                    [0.60, 0.37, 0.57, 0.37],
                ] {
                    m.stalk([x, 0.0, z], [x, h, z], 0.12, CREAM);
                    m.stone([x, h - 0.08, z], [w, 0.30, w * 0.89], [188, 163, 119]);
                }
            }
            MorelCluster => {
                for [x, z, h] in [
                    [-0.55, 0.22, 0.62],
                    [0.02, -0.31, 1.06],
                    [0.51, 0.12, 0.81],
                    [0.19, 0.55, 0.48],
                ] {
                    m.stalk([x, 0.0, z], [x, h, z], 0.12, CREAM);
                    m.stone([x, h * 0.39, z], [0.22, h * 0.71, 0.23], [154, 122, 80]);
                    m.flower([x, h * 0.64, z - 0.17], 0.13, [99, 76, 56]);
                }
            }
            PuffballRing => {
                for [x, z, w] in [
                    [-0.66, -0.31, 0.25],
                    [-0.55, 0.36, 0.39],
                    [-0.02, 0.64, 0.33],
                    [0.52, 0.31, 0.29],
                    [0.71, -0.13, 0.31],
                    [0.20, -0.59, 0.38],
                ] {
                    m.stone([x, 0.0, z], [w, w * 1.35, w * 0.93], CREAM);
                }
            }
            CoralFungi => {
                for [x, z, h] in [[-0.52, 0.22, 0.63], [0.05, -0.37, 0.97], [0.48, 0.36, 0.66]] {
                    let node = [x, h * 0.40, z];
                    m.stalk([x, 0.0, z], node, 0.18, [199, 159, 105]);
                    for [dx, dz, lift] in
                        [[-0.31, 0.13, 0.90], [0.04, -0.22, 1.21], [0.26, 0.16, 1.02]]
                    {
                        let head = [x + dx, h * lift, z + dz];
                        m.stalk(node, head, 0.115, CREAM);
                        m.stalk(
                            head,
                            [head[0] + dx * 0.44, head[1] + 0.20, head[2]],
                            0.07,
                            CREAM,
                        );
                    }
                }
            }
            BrackenFronds => {
                for [x, z, h] in [
                    [-1.10, 0.17, 0.64],
                    [-0.62, -0.58, 0.83],
                    [0.13, 0.66, 0.76],
                    [0.70, -0.61, 0.82],
                    [1.02, 0.31, 0.65],
                ] {
                    m.stalk([0.0, 0.0, 0.0], [x, h, z], 0.10, LEAF);
                    for t in [0.40, 0.61, 0.79] {
                        m.stalk(
                            [x * t, h * t, z * t],
                            [x * t - z * 0.42, h * t + 0.16, z * t + x * 0.42],
                            0.10,
                            MOSS,
                        );
                        m.stalk(
                            [x * t, h * t, z * t],
                            [x * t + z * 0.42, h * t + 0.08, z * t - x * 0.42],
                            0.10,
                            LEAF,
                        );
                    }
                }
            }
            Foxglove => {
                for [x, z, h] in [
                    [-0.60, 0.12, 1.69],
                    [-0.06, -0.37, 2.47],
                    [0.50, 0.25, 1.92],
                ] {
                    m.rosette([x, 0.0, z], 0.55, 0.38, LEAF);
                    m.stalk([x, 0.0, z], [x + 0.10, h, z], 0.10, MOSS);
                    for t in [0.66, 0.79, 0.92] {
                        m.flower([x + 0.19, h * t, z], 0.23, [187, 103, 143]);
                    }
                }
            }
            RoseHipBramble => {
                m.wood([-0.93, 0.0, 0.30], [0.86, 1.27, -0.18], 0.12, WOOD);
                m.wood([0.16, 0.44, -0.05], [1.35, 0.31, 0.51], 0.088, WOOD);
                for [x, y, z] in [
                    [-0.62, 0.48, 0.14],
                    [-0.23, 0.80, 0.02],
                    [0.51, 1.06, -0.18],
                    [0.87, 1.22, -0.09],
                    [1.08, 0.43, 0.36],
                ] {
                    m.rosette([x, y, z], 0.41, 0.31, LEAF);
                    m.stone(
                        [x + 0.13, y + 0.11, z + 0.13],
                        [0.13, 0.22, 0.13],
                        [198, 78, 56],
                    );
                }
            }
            BilberryMound => {
                for [x, z, w] in [
                    [-0.83, 0.13, 0.54],
                    [-0.23, -0.39, 0.67],
                    [0.22, 0.36, 0.56],
                    [0.84, 0.04, 0.48],
                ] {
                    m.stone([x, 0.0, z], [w, w * 0.91, w], LEAF);
                    m.flower([x + 0.08, w * 0.77, z], 0.17, [91, 114, 152]);
                    m.flower([x - 0.18, w * 0.51, z + 0.16], 0.13, [98, 120, 161]);
                }
            }
            HoneysuckleArch => {
                m.wood([-1.02, 0.0, 0.02], [-0.56, 1.34, -0.15], 0.12, WOOD);
                m.wood([-0.56, 1.34, -0.15], [0.49, 1.43, 0.16], 0.11, WOOD);
                m.wood([0.49, 1.43, 0.16], [1.12, 0.0, 0.27], 0.12, WOOD);
                for [x, y, z] in [
                    [-0.80, 0.78, -0.06],
                    [-0.43, 1.40, 0.10],
                    [0.20, 1.52, 0.15],
                    [0.76, 1.02, 0.27],
                ] {
                    m.rosette([x, y, z], 0.49, 0.31, LEAF);
                    m.flower([x + 0.13, y + 0.27, z], 0.21, [227, 203, 126]);
                }
            }
        }
    }
}
