//! Mountain models grouped by the ground on which they can occur.

use super::Mesh;
use crate::world::{Sample, Surface};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum MountainFeature {
    CreepingJuniper,
    StuntedSpruce,
    MountainMahogany,
    SplitPineSnag,
    RootPlate,
    HeatherBank,
    AlpineAvens,
    DwarfWillow,
    BunchgrassRing,
    GentianSpikes,
    QuartzSeam,
    SlateBlades,
    RockStep,
    SplitGranite,
    BasaltColumns,
    SnowCornice,
    RimIceFin,
    SnowPocket,
    WindCrust,
    RimedBoulder,
    BoulderTrain,
    GlacialPavement,
    PairedErratics,
    MoraineCrest,
    MeltwaterCobbles,
}

impl MountainFeature {
    pub(super) fn choose(ground: Sample, key: u64, region: u64) -> Option<Self> {
        use MountainFeature::*;
        const FOOTHILL: [MountainFeature; 5] = [
            CreepingJuniper,
            StuntedSpruce,
            MountainMahogany,
            SplitPineSnag,
            RootPlate,
        ];
        const MEADOW: [MountainFeature; 5] = [
            HeatherBank,
            AlpineAvens,
            DwarfWillow,
            BunchgrassRing,
            GentianSpikes,
        ];
        const CRAG: [MountainFeature; 5] = [
            QuartzSeam,
            SlateBlades,
            RockStep,
            SplitGranite,
            BasaltColumns,
        ];
        const SNOW: [MountainFeature; 5] =
            [SnowCornice, RimIceFin, SnowPocket, WindCrust, RimedBoulder];
        const MORAINE: [MountainFeature; 5] = [
            BoulderTrain,
            GlacialPavement,
            PairedErratics,
            MoraineCrest,
            MeltwaterCobbles,
        ];

        let forms = match ground.surface {
            Surface::Forest if ground.elevation >= 48.0 => &FOOTHILL,
            Surface::Grass | Surface::AlpineMeadow if ground.elevation >= 55.0 => &MEADOW,
            // Loose debris also gathers below exposed, frost-weathered ridges.
            Surface::Rock if ground.elevation >= 104.0 && (region >> 23) & 3 == 0 => &MORAINE,
            Surface::Rock | Surface::ScreeEdge
                if ground.elevation >= 69.0 =>
            {
                &CRAG
            }
            Surface::Snow => &SNOW,
            Surface::GlacierMoraine | Surface::ScreeFine | Surface::ScreeCoarse
                if ground.elevation >= 66.0 =>
            {
                &MORAINE
            }
            _ => return None,
        };
        let choice = if key >> 52 & 3 == 0 {
            key >> 54
        } else {
            (region >> 17) + (key >> 57 & 1)
        };
        Some(forms[(choice % 5) as usize])
    }

    pub(super) fn draw(self, m: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        const BARK: [u8; 3] = [116, 104, 86];
        const PINE: [u8; 3] = [76, 107, 80];
        const WILLOW: [u8; 3] = [122, 139, 92];
        const GRANITE: [u8; 3] = [151, 149, 147];
        const SHALE: [u8; 3] = [102, 112, 123];
        const ICE: [u8; 3] = [143, 183, 200];
        const SNOW: [u8; 3] = [218, 226, 225];
        use MountainFeature::*;
        match self {
            CreepingJuniper => {
                m.wood([-0.7, 0.0, 0.15], [1.1, 0.61, 0.08], 0.16, BARK);
                for [x, y, z, w] in [
                    [-0.86, 0.0, 0.18, 0.55],
                    [-0.31, 0.22, -0.20, 0.69],
                    [0.39, 0.29, 0.29, 0.71],
                    [1.01, 0.49, 0.07, 0.50],
                ] {
                    m.stone([x, y, z], [w, 0.51, 0.45], PINE);
                }
            }
            StuntedSpruce => {
                m.wood([0.0, 0.0, 0.0], [0.36, 2.14, 0.0], 0.17, BARK);
                for [x, y, w, h] in [
                    [-0.14, 0.45, 0.95, 0.97],
                    [0.12, 0.98, 0.67, 0.91],
                    [0.25, 1.49, 0.46, 0.96],
                ] {
                    m.gable([x, y, 0.0], [w, h, 0.58 * w], PINE);
                    m.rosette([x, y, 0.0], w, h, [86, 119, 93]);
                }
            }
            MountainMahogany => {
                for [x, y, z, w] in [
                    [-0.82, 0.83, -0.25, 0.52],
                    [-0.13, 1.43, 0.26, 0.53],
                    [0.68, 1.07, 0.09, 0.62],
                    [0.22, 0.53, -0.48, 0.60],
                ] {
                    m.wood([0.0, 0.0, 0.0], [x, y, z], 0.09, BARK);
                    m.rosette([x, y, z], w, 0.44, [126, 132, 105]);
                    m.stone([x, y, z], [w * 0.62, 0.33, w * 0.58], WILLOW);
                }
            }
            SplitPineSnag => {
                m.wood([0.0, 0.0, 0.0], [-0.23, 2.52, -0.21], 0.23, BARK);
                m.wood([-0.10, 1.05, -0.09], [0.69, 2.08, 0.24], 0.11, BARK);
                m.wood(
                    [-0.14, 1.72, -0.15],
                    [-0.95, 1.97, 0.13],
                    0.12,
                    [144, 132, 111],
                );
                m.stalk(
                    [0.04, 1.36, -0.07],
                    [-0.13, 2.65, -0.18],
                    0.12,
                    [185, 163, 131],
                );
            }
            RootPlate => {
                m.stone([-0.55, 0.0, 0.03], [0.82, 1.37, 0.35], [116, 94, 73]);
                m.wood([-0.60, 0.64, 0.12], [1.82, 0.17, -0.16], 0.30, BARK);
                for [x, y, z] in [
                    [-1.29, 1.21, -0.17],
                    [-0.34, 1.66, 0.07],
                    [-0.79, 0.0, 0.96],
                    [0.33, 0.0, -0.74],
                ] {
                    m.wood([-0.58, 0.61, 0.08], [x, y, z], 0.13, [162, 139, 108]);
                }
            }
            HeatherBank => {
                for [x, y, z, w] in [
                    [-0.92, 0.0, 0.25, 0.48],
                    [-0.35, 0.0, -0.37, 0.60],
                    [0.30, 0.0, 0.12, 0.64],
                    [0.88, 0.0, 0.50, 0.47],
                ] {
                    m.stone([x, y, z], [w, w * 0.8, w * 0.69], [123, 118, 97]);
                    m.rosette([x, w * 0.49, z], w * 0.69, 0.25, [173, 111, 146]);
                }
            }
            AlpineAvens => {
                for [x, y, z] in [
                    [-0.90, 0.52, 0.08],
                    [-0.44, 0.66, -0.26],
                    [0.26, 0.87, 0.0],
                    [0.91, 0.47, 0.36],
                    [0.37, 0.58, 0.62],
                ] {
                    m.rosette([x, 0.0, z], 0.40, 0.22, WILLOW);
                    m.stalk([x, 0.0, z], [x + 0.1, y, z], 0.065, WILLOW);
                    m.flower([x + 0.1, y, z], 0.24, [233, 191, 76]);
                }
            }
            DwarfWillow => {
                m.wood([-0.68, 0.0, -0.12], [1.3, 0.57, 0.06], 0.12, BARK);
                for [x, y, z] in [
                    [-0.46, 0.35, 0.46],
                    [-0.35, 0.56, -0.56],
                    [0.29, 0.50, 0.51],
                    [0.56, 0.73, -0.45],
                    [1.16, 0.63, 0.25],
                ] {
                    m.wood([x * 0.63, 0.30, 0.02], [x, y, z], 0.07, BARK);
                    m.rosette([x, y, z], 0.46, 0.23, [146, 159, 114]);
                }
            }
            BunchgrassRing => {
                for [x, z, h] in [
                    [-0.86, -0.19, 0.65],
                    [-0.29, -0.69, 0.82],
                    [0.52, -0.5, 0.75],
                    [0.67, 0.15, 0.58],
                    [0.33, 0.57, 0.51],
                    [-0.52, 0.57, 0.49],
                ] {
                    m.rosette([x, 0.0, z], 0.49, h, [175, 159, 102]);
                    m.stalk([x, 0.0, z], [x + 0.29, h + 0.35, z], 0.08, [194, 178, 130]);
                }
            }
            GentianSpikes => {
                for [x, z, h] in [[-0.53, 0.10, 1.25], [0.0, -0.32, 1.59], [0.48, 0.23, 1.02]] {
                    m.rosette([x, 0.0, z], 0.63, 0.36, [81, 115, 76]);
                    m.stalk([x, 0.0, z], [x + 0.19, h, z], 0.075, PINE);
                    m.stone([x + 0.17, h - 0.35, z], [0.16, 0.43, 0.18], [82, 112, 177]);
                    m.flower([x + 0.19, h, z], 0.18, [135, 158, 221]);
                }
            }
            QuartzSeam => {
                m.stone([-0.3, 0.0, 0.01], [1.33, 1.43, 0.73], SHALE);
                m.stone([0.65, 0.0, 0.06], [0.94, 0.81, 0.55], GRANITE);
                m.quad(
                    [
                        [-1.33, 0.34, -0.43],
                        [0.38, 1.1, -0.36],
                        [0.31, 1.36, -0.27],
                        [-1.29, 0.52, -0.49],
                    ],
                    [218, 209, 192],
                );
            }
            SlateBlades => {
                for [x, y, z] in [
                    [-0.96, 1.46, 0.0],
                    [-0.27, 2.09, -0.24],
                    [0.44, 1.59, 0.19],
                    [0.99, 1.02, 0.36],
                ] {
                    m.gable([x, 0.0, z], [0.31, y, 0.61], SHALE);
                }
                m.stone([-0.23, 0.0, 0.68], [1.37, 0.42, 0.42], [126, 133, 136]);
            }
            RockStep => {
                m.block([0.0, 0.0, 0.0], [1.63, 0.44, 0.92], SHALE);
                m.block([0.29, 0.44, 0.11], [1.24, 0.36, 0.72], GRANITE);
                m.block([0.58, 0.8, 0.19], [0.86, 0.34, 0.55], [172, 150, 135]);
                m.stone([0.79, 1.14, 0.21], [0.56, 0.45, 0.45], GRANITE);
            }
            SplitGranite => {
                m.stone([-0.62, 0.0, -0.13], [0.66, 2.24, 0.76], GRANITE);
                m.stone([0.82, 0.0, 0.26], [0.67, 1.43, 0.68], [164, 154, 144]);
                m.stone([0.08, 0.0, 0.71], [0.30, 0.34, 0.28], SHALE);
            }
            BasaltColumns => {
                for [x, z, h] in [
                    [-0.91, 0.21, 1.23],
                    [-0.32, -0.40, 2.38],
                    [0.25, 0.31, 1.74],
                    [0.86, -0.33, 0.96],
                ] {
                    m.drum([x, 0.0, z], 0.37, h, [86, 94, 104], false);
                }
            }
            SnowCornice => {
                m.stone([-0.61, 0.0, 0.17], [1.07, 1.21, 0.76], SNOW);
                m.stone([0.75, 0.0, 0.13], [0.93, 0.75, 0.69], SNOW);
                m.quad(
                    [
                        [-1.57, 0.11, -0.40],
                        [1.41, 0.08, -0.31],
                        [0.90, 0.80, -0.53],
                        [-0.79, 1.04, -0.70],
                    ],
                    ICE,
                );
            }
            RimIceFin => {
                m.gable([0.0, 0.0, 0.14], [0.47, 2.24, 1.09], ICE);
                m.gable([0.42, 0.0, 0.31], [0.33, 0.87, 0.64], SNOW);
                m.stone([-0.68, 0.0, 0.05], [0.49, 0.48, 0.61], SHALE);
            }
            SnowPocket => {
                m.stone([0.0, 0.0, 0.0], [1.12, 0.39, 0.84], SNOW);
                for [x, z, w, h] in [
                    [-0.82, -0.15, 0.48, 0.97],
                    [-0.39, 0.42, 0.45, 0.68],
                    [0.43, 0.31, 0.50, 0.81],
                    [0.77, -0.33, 0.38, 0.54],
                ] {
                    m.stone([x, 0.0, z], [w, h, 0.43], SHALE);
                }
            }
            WindCrust => {
                for [x, y, z, w] in [
                    [-0.39, 0.36, -0.55, 0.99],
                    [0.0, 0.28, 0.02, 1.41],
                    [0.37, 0.18, 0.55, 1.06],
                ] {
                    m.gable([x, 0.0, z], [w, y, 0.23], SNOW);
                }
            }
            RimedBoulder => {
                m.stone([0.0, 0.0, 0.0], [1.0, 1.66, 0.92], SHALE);
                m.stone([-0.22, 0.91, -0.17], [0.72, 0.80, 0.69], SNOW);
                m.stone([-0.79, 0.0, 0.38], [0.57, 0.50, 0.43], ICE);
            }
            BoulderTrain => {
                for [x, z, w, h] in [
                    [-1.61, 0.25, 0.29, 0.34],
                    [-0.86, 0.07, 0.48, 0.65],
                    [0.0, -0.07, 0.64, 0.94],
                    [0.80, -0.32, 0.39, 0.55],
                    [1.37, -0.44, 0.26, 0.38],
                ] {
                    m.stone([x, 0.0, z], [w, h, w * 0.73], GRANITE);
                }
            }
            GlacialPavement => {
                m.stone([0.0, 0.0, 0.0], [1.65, 0.40, 1.02], [119, 131, 135]);
                for z in [-0.39, -0.13, 0.14, 0.40] {
                    m.wood(
                        [-0.7, 0.21, z + 0.12],
                        [0.68, 0.35, z],
                        0.045,
                        [169, 181, 181],
                    );
                }
            }
            PairedErratics => {
                m.stone([-0.69, 0.0, 0.02], [0.82, 0.61, 0.66], GRANITE);
                m.stone([0.57, 0.0, -0.12], [0.57, 1.95, 0.53], [133, 116, 106]);
                m.stone([-0.77, 0.39, 0.03], [0.51, 0.59, 0.46], SHALE);
            }
            MoraineCrest => {
                m.gable([-0.25, 0.0, 0.03], [0.85, 0.79, 1.62], [139, 123, 104]);
                m.stone([-0.48, 0.42, -0.52], [0.50, 0.64, 0.42], GRANITE);
                m.stone([-0.05, 0.40, 0.60], [0.57, 0.52, 0.50], SHALE);
                m.stone([0.81, 0.0, 0.47], [0.62, 0.38, 0.59], GRANITE);
            }
            MeltwaterCobbles => {
                for [x, z, w] in [
                    [-0.84, -0.49, 0.36],
                    [-0.64, 0.37, 0.51],
                    [-0.31, 0.78, 0.39],
                    [0.12, -0.60, 0.31],
                    [0.65, -0.39, 0.54],
                    [1.02, 0.24, 0.37],
                ] {
                    m.stone([x, 0.0, z], [w, w * 0.57, w * 0.72], [138, 149, 154]);
                }
            }
        }
    }
}
