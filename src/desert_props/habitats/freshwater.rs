//! Riparian details selected from the river and floodplain material overlays.

use super::Mesh;
use crate::world::{Sample, Surface};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum FreshwaterFeature {
    CobbleRidge,
    SandRipple,
    ShaleShingle,
    FloodWrack,
    RootWad,
    RiverLog,
    ForkedDriftwood,
    SiltButte,
    PebbleFan,
    ChannelBoulders,
    CattailStand,
    BulrushUmbels,
    SweetFlag,
    BurReed,
    ReedWall,
    MarshMarigold,
    IrisStand,
    Meadowsweet,
    Loosestrife,
    WaterMint,
    WillowStool,
    AlderSaplings,
    RedOsierThicket,
    BrackenBreak,
    RushTussocks,
}

impl FreshwaterFeature {
    pub(super) fn choose(ground: Sample, key: u64, district: u64) -> Option<Self> {
        use FreshwaterFeature::*;
        const SILT: [FreshwaterFeature; 10] = [
            CobbleRidge,
            SandRipple,
            ShaleShingle,
            FloodWrack,
            RootWad,
            RiverLog,
            ForkedDriftwood,
            SiltButte,
            PebbleFan,
            ChannelBoulders,
        ];
        const MARSH: [FreshwaterFeature; 5] =
            [CattailStand, BulrushUmbels, SweetFlag, BurReed, ReedWall];
        const MEADOW: [FreshwaterFeature; 10] = [
            MarshMarigold,
            IrisStand,
            Meadowsweet,
            Loosestrife,
            WaterMint,
            WillowStool,
            AlderSaplings,
            RedOsierThicket,
            BrackenBreak,
            RushTussocks,
        ];

        let forms: &[Self] = match ground.surface {
            Surface::RiverSilt if ground.elevation >= 0.45 => &SILT,
            Surface::ReedMarsh => &MARSH,
            // Floods leave deposits above the active bar as well.
            Surface::WetMeadow if district >> 38 & 3 == 0 => &SILT,
            Surface::WetMeadow => &MEADOW,
            _ => return None,
        };
        // Adjacent cells can share a colony while each watershed draws
        // from the full set. A quarter of cells use their own independent pick.
        let index = if key >> 29 & 3 == 0 {
            key >> 32
        } else {
            (district >> 22) + (key >> 37 & 1)
        };
        Some(forms[(index % forms.len() as u64) as usize])
    }

    pub(super) fn draw(self, m: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        use FreshwaterFeature::*;

        const GRAVEL: [u8; 3] = [151, 147, 135];
        const SHALE: [u8; 3] = [103, 115, 115];
        const SILT: [u8; 3] = [162, 130, 99];
        const BARK: [u8; 3] = [105, 84, 65];
        const WOOD: [u8; 3] = [171, 144, 106];
        const REED: [u8; 3] = [119, 145, 71];
        const SEDGE: [u8; 3] = [146, 162, 95];
        const LEAF: [u8; 3] = [81, 132, 82];

        match self {
            CobbleRidge => {
                for [x, z, w, h] in [
                    [-1.23, -0.15, 0.41, 0.28],
                    [-0.66, -0.06, 0.58, 0.51],
                    [0.12, 0.19, 0.54, 0.63],
                    [0.65, 0.39, 0.52, 0.44],
                    [1.31, 0.31, 0.35, 0.31],
                ] {
                    m.stone([x, 0.0, z], [w, h, 0.48], GRAVEL);
                }
            }
            SandRipple => {
                for [x, z, w, h] in [
                    [-0.24, -0.52, 1.34, 0.24],
                    [0.11, 0.02, 1.50, 0.32],
                    [0.31, 0.53, 1.16, 0.19],
                ] {
                    m.gable([x, 0.0, z], [w, h, 0.22], SILT);
                }
            }
            ShaleShingle => {
                m.stone([-0.49, 0.0, 0.0], [1.01, 0.38, 0.63], SHALE);
                m.stone([0.46, 0.0, 0.22], [0.96, 0.28, 0.76], [130, 139, 131]);
                m.quad(
                    [
                        [-0.71, 0.21, -0.40],
                        [0.21, 0.24, -0.51],
                        [0.09, 0.7, -0.14],
                        [-0.62, 0.66, 0.10],
                    ],
                    SHALE,
                );
                m.stone([0.91, 0.07, -0.25], [0.58, 0.18, 0.38], GRAVEL);
            }
            FloodWrack => {
                for [x, z, h] in [
                    [-1.0, -0.32, 0.33],
                    [-0.61, 0.18, 0.24],
                    [-0.18, -0.15, 0.51],
                    [0.29, -0.29, 0.38],
                    [0.72, 0.29, 0.25],
                    [1.15, -0.07, 0.28],
                ] {
                    m.wood(
                        [x - 0.47, 0.0, z + 0.23],
                        [x + 0.57, h, z - 0.21],
                        0.11,
                        WOOD,
                    );
                }
                m.stone([0.12, 0.0, 0.02], [1.21, 0.26, 0.65], BARK);
            }
            RootWad => {
                m.stone([-0.17, 0.0, 0.05], [0.83, 0.92, 0.53], BARK);
                m.wood([-0.21, 0.43, 0.03], [0.24, 1.34, 0.35], 0.23, WOOD);
                for tip in [
                    [-1.26, 0.56, -0.30],
                    [-0.87, 1.23, 0.34],
                    [-0.26, 1.62, -0.23],
                    [0.7, 0.80, -0.54],
                    [0.83, 0.0, 0.65],
                    [-0.61, 0.0, -0.78],
                ] {
                    m.wood([-0.11, 0.53, 0.0], tip, 0.10, WOOD);
                }
            }
            RiverLog => {
                m.drum([0.0, 0.0, 0.0], 0.39, 2.9, BARK, true);
                m.drum([0.0, 0.12, 1.43], 0.27, 0.11, WOOD, true);
                m.wood([0.07, 0.56, -0.45], [0.57, 0.98, -0.64], 0.15, BARK);
                m.wood([-0.16, 0.46, 0.60], [-0.50, 0.71, 0.72], 0.11, BARK);
            }
            ForkedDriftwood => {
                let fork = [-0.28, 0.67, 0.0];
                m.wood([-1.44, 0.0, 0.21], fork, 0.18, BARK);
                m.wood(fork, [1.34, 0.19, -0.57], 0.14, WOOD);
                m.wood(fork, [0.86, 1.18, 0.68], 0.12, WOOD);
                m.wood([0.42, 0.92, 0.42], [0.49, 1.22, 0.85], 0.08, WOOD);
            }
            SiltButte => {
                m.stone([0.0, 0.0, 0.0], [1.07, 0.55, 0.95], SILT);
                m.block([0.14, 0.30, -0.08], [0.65, 0.32, 0.52], [124, 100, 81]);
                m.stone([0.10, 0.54, -0.11], [0.75, 0.32, 0.59], [187, 161, 128]);
            }
            PebbleFan => {
                for [x, z, w] in [
                    [-0.19, -0.73, 0.43],
                    [-0.56, -0.09, 0.39],
                    [0.21, -0.11, 0.48],
                    [-1.02, 0.52, 0.24],
                    [-0.31, 0.64, 0.33],
                    [0.37, 0.49, 0.33],
                    [0.89, 0.67, 0.24],
                ] {
                    m.stone([x, 0.0, z], [w, w * 0.75, w * 0.85], GRAVEL);
                }
            }
            ChannelBoulders => {
                m.stone([-0.68, 0.0, -0.07], [0.91, 1.16, 0.79], SHALE);
                m.stone([0.57, 0.0, 0.14], [0.77, 0.72, 0.66], [125, 122, 112]);
                m.stone([-0.62, 0.58, 0.09], [0.64, 0.49, 0.57], GRAVEL);
                m.rosette([0.14, 0.0, 0.62], 0.40, 0.48, REED);
            }
            CattailStand => {
                for [x, z, h] in [
                    [-0.80, 0.04, 1.83],
                    [-0.42, -0.21, 2.41],
                    [0.18, 0.21, 2.02],
                    [0.69, -0.11, 1.56],
                ] {
                    m.rosette([x, 0.0, z], 0.37, 1.17, REED);
                    m.stalk([x, 0.0, z], [x + 0.09, h, z], 0.062, SEDGE);
                    m.stone([x + 0.09, h - 0.20, z], [0.14, 0.44, 0.13], [122, 78, 54]);
                }
            }
            BulrushUmbels => {
                for [x, z, h] in [[-0.58, 0.18, 1.51], [0.03, -0.25, 2.27], [0.55, 0.27, 1.85]] {
                    m.stalk([x * 0.66, 0.0, z], [x, h, z], 0.09, REED);
                    m.rosette([x, h - 0.12, z], 0.55, 0.22, SEDGE);
                    for [dx, dz] in [[-0.37, 0.18], [0.24, 0.35], [0.14, -0.28]] {
                        m.flower([x + dx, h, z + dz], 0.13, BARK);
                    }
                }
            }
            SweetFlag => {
                for [x, z, h] in [[-0.61, 0.15, 1.61], [0.02, -0.37, 1.92], [0.55, 0.29, 1.26]] {
                    for dx in [-0.47, -0.26, 0.0, 0.24, 0.49] {
                        m.stalk(
                            [x, 0.0, z],
                            [x + dx, h - dx.abs(), z - dx * 0.22],
                            0.16,
                            REED,
                        );
                    }
                    m.stone([x + 0.15, h * 0.62, z], [0.12, 0.41, 0.12], [191, 171, 85]);
                }
            }
            BurReed => {
                for [x, z, h] in [
                    [-0.58, 0.19, 1.50],
                    [-0.06, -0.41, 2.17],
                    [0.61, 0.24, 1.73],
                ] {
                    m.rosette([x, 0.0, z], 0.40, 0.82, REED);
                    m.stalk([x, 0.0, z], [x + 0.33, h, z], 0.07, SEDGE);
                    for t in [0.61, 0.77, 0.93] {
                        m.stone([x + t * 0.33, t * h, z], [0.16, 0.25, 0.16], [168, 155, 89]);
                    }
                }
            }
            ReedWall => {
                for [x, z, h] in [
                    [-1.22, 0.04, 1.48],
                    [-0.78, -0.25, 2.13],
                    [-0.39, 0.29, 2.20],
                    [0.01, 0.05, 2.43],
                    [0.43, 0.31, 2.03],
                    [0.82, -0.25, 1.88],
                    [1.18, 0.04, 1.31],
                ] {
                    m.stalk([x, 0.0, z], [x + 0.23, h, z], 0.09, REED);
                    m.stalk(
                        [x, 0.53 * h, z],
                        [x - 0.30, 0.72 * h, z + 0.34],
                        0.12,
                        SEDGE,
                    );
                    m.stone([x + 0.22, h - 0.25, z], [0.16, 0.39, 0.14], [159, 131, 104]);
                }
            }
            MarshMarigold => {
                for [x, y, z] in [
                    [-0.67, 0.48, 0.18],
                    [-0.35, 0.62, -0.26],
                    [0.09, 0.72, 0.08],
                    [0.53, 0.52, 0.46],
                    [0.76, 0.39, -0.27],
                ] {
                    m.rosette([x, 0.0, z], 0.43, 0.34, LEAF);
                    m.stalk([x, 0.0, z], [x, y, z], 0.06, REED);
                    m.flower([x, y, z], 0.30, [239, 190, 59]);
                }
            }
            IrisStand => {
                for [x, z, h] in [
                    [-0.70, 0.02, 1.25],
                    [-0.19, 0.28, 1.68],
                    [0.46, -0.24, 1.48],
                ] {
                    m.rosette([x, 0.0, z], 0.47, 1.2, LEAF);
                    m.stalk([x, 0.0, z], [x + 0.08, h, z], 0.09, REED);
                    m.flower([x + 0.08, h, z], 0.29, [110, 114, 184]);
                    m.flower([x + 0.08, h - 0.09, z], 0.39, [152, 138, 211]);
                }
            }
            Meadowsweet => {
                for [x, h, z] in [
                    [-0.79, 1.26, 0.03],
                    [-0.18, 1.70, -0.42],
                    [0.49, 1.50, 0.28],
                    [0.72, 1.13, -0.31],
                ] {
                    m.stalk([x * 0.62, 0.0, z], [x, h, z], 0.105, LEAF);
                    m.rosette([x * 0.86, h * 0.48, z], 0.47, 0.25, LEAF);
                    m.stone([x, h - 0.22, z], [0.37, 0.31, 0.35], [230, 219, 175]);
                }
            }
            Loosestrife => {
                for [x, h, z] in [
                    [-0.75, 1.42, 0.25],
                    [-0.40, 2.02, -0.31],
                    [0.16, 2.47, 0.09],
                    [0.62, 1.79, 0.28],
                ] {
                    m.stalk([x * 0.53, 0.0, z], [x, h, z], 0.08, LEAF);
                    m.rosette([x * 0.77, h * 0.52, z], 0.35, 0.28, REED);
                    m.stone([x, h - 0.62, z], [0.15, 0.69, 0.14], [171, 92, 154]);
                }
            }
            WaterMint => {
                for [x, y, z] in [
                    [-0.93, 0.0, 0.08],
                    [-0.41, 0.13, -0.26],
                    [0.13, 0.29, 0.16],
                    [0.60, 0.19, 0.31],
                    [0.93, 0.0, -0.06],
                ] {
                    m.stone([x, y, z], [0.43, 0.38, 0.35], [85, 139, 101]);
                    m.stone([x, y + 0.31, z], [0.13, 0.18, 0.14], [184, 156, 192]);
                }
            }
            WillowStool => {
                m.stone([0.0, 0.0, 0.0], [0.48, 0.43, 0.48], BARK);
                for [x, h, z] in [
                    [-1.02, 0.93, 0.14],
                    [-0.48, 1.43, -0.51],
                    [0.26, 1.61, 0.29],
                    [0.89, 1.12, -0.13],
                ] {
                    m.wood([0.0, 0.22, 0.0], [x, h, z], 0.12, WOOD);
                    m.rosette([x, h - 0.10, z], 0.62, 0.39, [118, 156, 97]);
                }
            }
            AlderSaplings => {
                for [x, z, h] in [[-0.69, 0.17, 2.25], [0.01, -0.29, 2.57], [0.64, 0.22, 1.89]] {
                    m.wood([x, 0.0, z], [x + 0.19, h, z], 0.12, BARK);
                    m.stone([x + 0.12, h - 0.81, z], [0.46, 0.95, 0.49], LEAF);
                    m.stone(
                        [x + 0.14, h - 1.11, z + 0.28],
                        [0.53, 0.58, 0.39],
                        [95, 145, 88],
                    );
                }
            }
            RedOsierThicket => {
                for [x, h, z] in [
                    [-0.82, 1.10, -0.24],
                    [-0.47, 1.67, 0.23],
                    [0.10, 1.52, -0.33],
                    [0.58, 1.31, 0.28],
                    [0.96, 0.80, 0.0],
                ] {
                    m.wood([x * 0.25, 0.0, z * 0.42], [x, h, z], 0.094, [152, 66, 61]);
                    m.rosette([x * 0.76, h * 0.66, z], 0.48, 0.31, [134, 143, 95]);
                    m.flower([x, h, z], 0.17, [216, 212, 179]);
                }
            }
            BrackenBreak => {
                for [x, z, h] in [
                    [-0.89, 0.20, 0.84],
                    [-0.53, -0.48, 1.18],
                    [0.09, 0.60, 1.27],
                    [0.51, -0.50, 0.95],
                    [0.94, 0.15, 0.74],
                ] {
                    m.stalk([0.0, 0.0, 0.0], [x, h, z], 0.07, LEAF);
                    for t in [0.46, 0.67, 0.86] {
                        m.stalk(
                            [x * t, h * t, z * t],
                            [x * t - z * 0.42, h * t + 0.06, z * t + x * 0.42],
                            0.11,
                            LEAF,
                        );
                        m.stalk(
                            [x * t, h * t, z * t],
                            [x * t + z * 0.42, h * t + 0.10, z * t - x * 0.42],
                            0.11,
                            REED,
                        );
                    }
                }
            }
            RushTussocks => {
                for [x, z, w, h] in [
                    [-0.72, 0.05, 0.56, 1.12],
                    [-0.13, -0.39, 0.69, 0.84],
                    [0.68, 0.33, 0.59, 1.26],
                ] {
                    m.stone([x, 0.0, z], [w * 0.73, 0.29, w * 0.66], [96, 104, 74]);
                    m.rosette([x, 0.18, z], w, h, SEDGE);
                    m.rosette([x, 0.16, z], w * 0.8, h * 0.64, REED);
                }
            }
        }
    }
}
