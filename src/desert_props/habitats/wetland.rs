//! Grounded details for floodplains, wet meadows, marshes, and exposed river bars.

use super::Mesh;
use crate::world::{Sample, Surface};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum WetlandFeature {
    PeatHummocks, SphagnumLobes, FenMarlLedge, TussockIslands, BankOverhang,
    RaggedRobin, FenOrchid, GrassOfParnassus, YellowRattle, Globeflower,
    ReedPanicles, SpikeRushLawn, FloweringRush, ArrowheadStand, ReedCaneLitter,
    ClayScrolls, GravelTongue, SnagWithSiltApron, IronSeepCrust, SandbarRipple,
    BogMyrtle, LeatherleafMat, DwarfWillow, BuckthornFenClump, CranberryRunner,
}

use WetlandFeature::*;

const GROUND: [WetlandFeature; 5] =
    [PeatHummocks, SphagnumLobes, FenMarlLedge, TussockIslands, BankOverhang];
const FLOWERS: [WetlandFeature; 5] =
    [RaggedRobin, FenOrchid, GrassOfParnassus, YellowRattle, Globeflower];
const EMERGENT: [WetlandFeature; 5] =
    [ReedPanicles, SpikeRushLawn, FloweringRush, ArrowheadStand, ReedCaneLitter];
const BAR: [WetlandFeature; 5] =
    [ClayScrolls, GravelTongue, SnagWithSiltApron, IronSeepCrust, SandbarRipple];
const SHRUBS: [WetlandFeature; 5] =
    [BogMyrtle, LeatherleafMat, DwarfWillow, BuckthornFenClump, CranberryRunner];

impl WetlandFeature {
    pub(super) fn choose(sample: Sample, key: u64, patch: u64) -> Option<Self> {
        let family = match sample.surface {
            Surface::ReedMarsh => &EMERGENT,
            Surface::RiverSilt if sample.elevation >= 0.45 => &BAR,
            Surface::WetMeadow => match (patch >> 19) % 7 {
                0 | 1 => &GROUND,
                2 | 3 => &FLOWERS,
                4 | 5 => &SHRUBS,
                _ => &BAR,
            },
            _ => return None,
        };
        let member = if key >> 33 & 3 == 0 { key >> 35 } else { (patch >> 23) + (key >> 39 & 1) };
        Some(family[(member % 5) as usize])
    }

    pub(super) fn draw(self, m: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        const PEAT: [u8; 3] = [94, 82, 61];
        const MOSS: [u8; 3] = [127, 146, 71];
        const SILT: [u8; 3] = [167, 138, 108];
        const CLAY: [u8; 3] = [119, 106, 94];
        const PALE: [u8; 3] = [183, 174, 139];
        const REED: [u8; 3] = [106, 143, 66];
        const STEM: [u8; 3] = [100, 130, 75];
        const LEAF: [u8; 3] = [73, 123, 72];
        const DARK: [u8; 3] = [57, 97, 59];
        const WOOD: [u8; 3] = [143, 121, 89];

        match self {
            PeatHummocks => {
                for [x, z, w, h] in [[-0.92, 0.32, 0.82, 0.43], [0.11, -0.40, 0.95, 0.74], [0.86, 0.44, 0.62, 0.48]] {
                    m.stone([x, 0.0, z], [w, h, w * 0.72], PEAT);
                    m.stone([x, h * 0.68, z], [w * 0.66, h * 0.33, w * 0.44], MOSS);
                }
            }
            SphagnumLobes => {
                for [x, z, w, h] in [[-0.84, 0.34, 0.83, 0.22], [-0.24, -0.29, 0.94, 0.36], [0.50, -0.35, 0.79, 0.33], [0.90, 0.36, 0.55, 0.20]] {
                    m.stone([x, 0.0, z], [w, h, 0.64], [146, 149, 61]);
                    m.stone([x + 0.14, h * 0.50, z - 0.15], [w * 0.55, h * 0.53, 0.34], [164, 139, 64]);
                }
            }
            FenMarlLedge => {
                m.gable([-0.16, 0.0, 0.02], [1.60, 0.40, 0.78], PALE);
                m.gable([0.09, 0.14, -0.19], [1.16, 0.44, 0.50], [206, 199, 161]);
                m.gable([0.47, 0.27, -0.39], [0.61, 0.39, 0.32], [218, 211, 181]);
                m.rosette([-0.88, 0.32, 0.05], 0.58, 0.39, REED);
            }
            TussockIslands => {
                for [x, z, w, h] in [[-1.02, -0.25, 0.56, 0.64], [0.08, 0.48, 0.74, 1.06], [0.91, -0.32, 0.53, 0.58]] {
                    m.stone([x, 0.0, z], [w, 0.28, w * 0.80], PEAT);
                    m.rosette([x, 0.23, z], w * 1.15, h, REED);
                }
            }
            BankOverhang => {
                m.gable([0.06, 0.0, 0.07], [1.36, 0.55, 0.56], PEAT);
                m.stone([-0.12, 0.44, -0.10], [1.51, 0.40, 0.78], MOSS);
                for dx in [-0.86, -0.42, 0.13, 0.59, 1.02] {
                    m.wood([dx, 0.45, 0.37], [dx + 0.19, 0.08, 0.76], 0.07, WOOD);
                }
            }
            RaggedRobin => {
                for [x, z, h] in [[-0.86, 0.16, 1.22], [-0.42, -0.32, 1.70], [0.08, 0.42, 2.04], [0.63, -0.22, 1.52], [0.94, 0.26, 1.08]] {
                    m.stalk([x * 0.64, 0.0, z * 0.64], [x, h, z], 0.06, LEAF);
                    m.flower([x, h, z], 0.30, [215, 106, 147]);
                    m.flower([x + 0.15, h + 0.02, z - 0.11], 0.17, [240, 153, 171]);
                }
            }
            FenOrchid => {
                for [x, z, h] in [[-0.64, 0.23, 1.24], [0.04, -0.24, 2.36], [0.73, 0.19, 1.68]] {
                    m.rosette([x, 0.0, z], 0.48, 0.49, LEAF);
                    m.stalk([x, 0.0, z], [x + 0.09, h, z], 0.10, STEM);
                    for n in [0.66, 0.81, 0.94] {
                        m.flower([x + 0.10, h * n, z], 0.24, [169, 129, 164]);
                    }
                }
            }
            GrassOfParnassus => {
                for [x, z, h] in [[-0.94, -0.35, 0.72], [-0.42, 0.38, 1.02], [0.12, -0.15, 1.27], [0.71, 0.25, 0.89]] {
                    m.rosette([x, 0.0, z], 0.39, 0.28, MOSS);
                    m.stalk([x, 0.0, z], [x, h, z], 0.062, STEM);
                    m.flower([x, h, z], 0.30, [232, 229, 197]);
                    m.flower([x, h + 0.05, z], 0.09, [183, 167, 86]);
                }
            }
            YellowRattle => {
                for [x, z, h] in [[-0.92, 0.23, 1.04], [-0.43, -0.30, 1.37], [0.06, 0.15, 1.49], [0.51, -0.45, 1.16], [0.88, 0.36, 0.95]] {
                    m.stalk([x * 0.76, 0.0, z], [x, h, z], 0.08, LEAF);
                    m.stone([x, h * 0.63, z], [0.19, 0.25, 0.17], [156, 159, 76]);
                    m.stone([x + 0.07, h * 0.81, z], [0.15, 0.27, 0.14], [222, 187, 57]);
                }
            }
            Globeflower => {
                m.rosette([0.01, 0.0, 0.0], 0.92, 0.64, DARK);
                for [x, z, h] in [[-0.68, 0.26, 1.35], [-0.20, -0.15, 1.95], [0.49, 0.33, 1.59]] {
                    m.stalk([x * 0.36, 0.0, z * 0.5], [x, h, z], 0.10, LEAF);
                    m.stone([x, h - 0.10, z], [0.28, 0.41, 0.27], [228, 177, 57]);
                }
            }
            ReedPanicles => {
                for [x, z, h] in [[-0.90, -0.32, 2.49], [-0.46, 0.24, 2.90], [0.07, -0.18, 3.31], [0.43, 0.30, 2.94], [0.90, -0.18, 2.66]] {
                    m.stalk([x * 0.75, 0.0, z], [x, h, z], 0.07, REED);
                    m.stone([x + 0.07, h - 0.28, z], [0.23, 0.54, 0.14], [158, 133, 96]);
                    m.stalk([x, h * 0.41, z], [x - 0.36, h * 0.66, z + 0.13], 0.12, STEM);
                }
            }
            SpikeRushLawn => {
                for [x, z, h] in [[-1.13, -0.28, 1.05], [-0.76, 0.40, 0.92], [-0.47, -0.46, 1.26], [-0.03, 0.13, 1.18], [0.37, -0.27, 1.25], [0.76, 0.42, 1.13], [1.11, -0.03, 0.92]] {
                    m.stalk([x, 0.0, z], [x + 0.10, h, z - 0.06], 0.075, REED);
                    m.stone([x + 0.10, h - 0.09, z - 0.06], [0.09, 0.21, 0.07], PEAT);
                }
                m.stone([0.0, 0.0, 0.02], [1.35, 0.21, 0.68], MOSS);
            }
            FloweringRush => {
                for [x, z, h] in [[-0.65, 0.34, 1.64], [0.07, -0.15, 2.48], [0.62, 0.29, 1.95]] {
                    m.rosette([x, 0.0, z], 0.49, 0.72, REED);
                    m.stalk([x, 0.0, z], [x, h, z], 0.08, STEM);
                    for [dx, dz] in [[0.0, -0.24], [-0.26, 0.12], [0.26, 0.12]] {
                        m.stalk([x, h - 0.33, z], [x + dx, h, z + dz], 0.045, LEAF);
                        m.flower([x + dx, h, z + dz], 0.22, [214, 156, 163]);
                    }
                }
            }
            ArrowheadStand => {
                for [x, z, h] in [[-0.77, -0.22, 1.25], [-0.22, 0.31, 1.78], [0.41, -0.23, 1.91], [0.89, 0.27, 1.17]] {
                    m.stalk([x * 0.80, 0.0, z], [x, h, z], 0.10, STEM);
                    m.quad([[x, h + 0.55, z], [x + 0.37, h - 0.20, z + 0.16],
                            [x, h, z], [x - 0.34, h - 0.16, z + 0.14]], LEAF);
                }
            }
            ReedCaneLitter => {
                for [x, z, y] in [[-0.91, -0.25, 0.31], [-0.45, 0.34, 0.44], [0.04, -0.30, 0.29], [0.53, 0.20, 0.67], [0.93, 0.42, 0.45]] {
                    m.wood([x - 0.56, 0.0, z - 0.32], [x + 0.52, y, z + 0.38], 0.08, PALE);
                }
                m.rosette([0.62, 0.0, -0.45], 0.56, 0.71, REED);
            }
            ClayScrolls => {
                for [x, z, w, h] in [[-0.18, -0.48, 1.48, 0.21], [0.14, -0.08, 1.58, 0.32], [-0.18, 0.32, 1.31, 0.29], [-0.48, 0.65, 0.88, 0.17]] {
                    m.gable([x, 0.0, z], [w, h, 0.18], CLAY);
                    m.gable([x, h * 0.48, z - 0.06], [w * 0.75, h * 0.50, 0.12], SILT);
                }
            }
            GravelTongue => {
                m.stone([-0.13, 0.0, -0.02], [1.51, 0.22, 0.71], SILT);
                for [x, z, w, h] in [[-1.03, 0.07, 0.37, 0.39], [-0.52, -0.29, 0.47, 0.31], [-0.10, 0.24, 0.42, 0.42], [0.41, -0.08, 0.39, 0.28], [0.87, 0.20, 0.25, 0.18]] {
                    m.stone([x, 0.08, z], [w, h, w * 0.82], PALE);
                }
            }
            SnagWithSiltApron => {
                m.stone([0.16, 0.0, 0.16], [1.51, 0.31, 0.72], SILT);
                m.wood([-1.18, 0.0, 0.49], [0.94, 0.71, -0.28], 0.21, WOOD);
                m.wood([0.20, 0.38, 0.08], [0.78, 1.45, 0.40], 0.11, WOOD);
                m.wood([0.52, 0.53, -0.05], [0.99, 1.02, -0.55], 0.10, PEAT);
            }
            IronSeepCrust => {
                m.stone([-0.23, 0.0, -0.18], [1.40, 0.29, 0.75], [141, 88, 58]);
                m.stone([0.51, 0.09, 0.23], [0.94, 0.27, 0.67], [181, 107, 60]);
                m.gable([-0.42, 0.14, 0.18], [1.28, 0.36, 0.24], [187, 130, 68]);
                m.stone([-0.76, 0.10, -0.29], [0.46, 0.47, 0.40], CLAY);
            }
            SandbarRipple => {
                m.gable([-0.15, 0.0, -0.51], [1.58, 0.16, 0.21], [188, 163, 120]);
                m.gable([0.12, 0.0, -0.06], [1.36, 0.25, 0.21], [194, 171, 128]);
                m.gable([0.36, 0.0, 0.47], [1.12, 0.14, 0.16], [184, 160, 120]);
                m.stone([1.01, 0.0, -0.38], [0.45, 0.38, 0.30], SILT);
            }
            BogMyrtle => {
                for [x, z, h] in [[-0.73, 0.22, 0.94], [-0.40, -0.33, 1.43], [0.03, 0.18, 1.71], [0.57, -0.17, 1.48], [0.87, 0.34, 0.96]] {
                    m.wood([0.0, 0.0, 0.0], [x, h, z], 0.078, WOOD);
                    m.stone([x, h * 0.72, z], [0.31, h * 0.45, 0.30], [101, 135, 76]);
                }
            }
            LeatherleafMat => {
                for [x, z, w, h] in [[-0.85, 0.25, 0.63, 0.63], [-0.35, -0.29, 0.58, 0.76], [0.32, -0.34, 0.71, 0.72], [0.78, 0.35, 0.65, 0.52]] {
                    m.stone([x, 0.0, z], [w, h, w * 0.71], DARK);
                    m.rosette([x, h * 0.62, z], w * 0.71, h * 0.49, LEAF);
                }
            }
            DwarfWillow => {
                m.wood([-0.71, 0.0, 0.38], [0.70, 0.75, -0.42], 0.12, WOOD);
                for [x, z, h] in [[-0.83, -0.09, 0.51], [-0.21, 0.33, 0.90], [0.53, -0.36, 1.09], [0.95, 0.06, 0.85]] {
                    m.stone([x, h * 0.42, z], [0.52, h * 0.60, 0.37], [94, 132, 79]);
                }
            }
            BuckthornFenClump => {
                m.wood([0.15, 0.0, -0.03], [-0.06, 2.12, 0.07], 0.17, WOOD);
                for [x, z, y, h] in [[-0.81, 0.20, 0.89, 0.73], [0.61, 0.28, 1.03, 0.93], [-0.39, -0.16, 1.70, 0.94], [0.44, -0.28, 1.82, 0.78]] {
                    m.wood([0.10, y * 0.63, 0.0], [x, y, z], 0.10, WOOD);
                    m.stone([x, y, z], [0.58, h, 0.43], LEAF);
                }
            }
            CranberryRunner => {
                m.stone([0.04, 0.0, 0.0], [1.29, 0.20, 0.70], PEAT);
                for [x, z] in [[-0.95, 0.11], [-0.51, -0.29], [0.04, 0.23], [0.54, -0.26], [0.92, 0.24]] {
                    m.rosette([x, 0.14, z], 0.40, 0.28, DARK);
                    m.stone([x + 0.15, 0.30, z - 0.09], [0.13, 0.21, 0.13], [154, 53, 64]);
                }
            }
        }
    }
}
