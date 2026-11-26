//! Exposed alpine ground, glacier interiors, and the last vegetation above the treeline.

use super::Mesh;
use crate::world::{Sample, Surface};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum AlpineGround {
    WindSculptedDrift, SnowBridgeLip, SnowPillows, FrostedTalus, SnowbreakSlabs,
    BlueIceRibs, PressureHummocks, MeltwaterRunnels, RefrozenLens, LayeredIceStep,
    GarnetScree, ShaleBook, QuartziteNeedles, FrostSplitTor, VeinedSchist,
    MixedErratics, SortedStoneRing, IceMarginalBank, GlacialBoulderFan, MoraineCobbleRibs,
    WhiteSaxifrage, AlpinePoppy, WillowMat, LichenCushions, SnowbedSedge,
}

use AlpineGround::*;

const SNOWFIELD: [AlpineGround; 5] =
    [WindSculptedDrift, SnowBridgeLip, SnowPillows, FrostedTalus, SnowbreakSlabs];
const GLACIER: [AlpineGround; 5] =
    [BlueIceRibs, PressureHummocks, MeltwaterRunnels, RefrozenLens, LayeredIceStep];
const EXPOSED: [AlpineGround; 5] =
    [GarnetScree, ShaleBook, QuartziteNeedles, FrostSplitTor, VeinedSchist];
const DEBRIS: [AlpineGround; 5] =
    [MixedErratics, SortedStoneRing, IceMarginalBank, GlacialBoulderFan, MoraineCobbleRibs];
const TUNDRA: [AlpineGround; 5] =
    [WhiteSaxifrage, AlpinePoppy, WillowMat, LichenCushions, SnowbedSedge];

impl AlpineGround {
    pub(super) fn choose(ground: Sample, key: u64, patch: u64) -> Option<Self> {
        let family = match ground.surface {
            Surface::Snow => &SNOWFIELD,
            Surface::GlacierIce => &GLACIER,
            Surface::Rock if ground.elevation >= 104.0 && patch >> 19 & 3 == 0 => &DEBRIS,
            Surface::Rock | Surface::ScreeEdge if ground.elevation >= 89.0 => &EXPOSED,
            Surface::GlacierMoraine | Surface::ScreeFine | Surface::ScreeCoarse
                if ground.elevation >= 66.0 => &DEBRIS,
            Surface::AlpineMeadow | Surface::Grass if ground.elevation >= 69.0 => &TUNDRA,
            _ => return None,
        };
        let slot = if key >> 32 & 3 == 0 { key >> 34 } else { (patch >> 19) + (key >> 37 & 1) };
        Some(family[(slot % 5) as usize])
    }

    pub(super) fn draw(self, mesh: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        const POWDER: [u8; 3] = [218, 228, 234];
        const PACK: [u8; 3] = [184, 205, 218];
        const ICE: [u8; 3] = [107, 167, 194];
        const BLUE: [u8; 3] = [78, 139, 166];
        const ROCK: [u8; 3] = [138, 142, 145];
        const SLATE: [u8; 3] = [93, 107, 124];
        const LICHEN: [u8; 3] = [164, 158, 107];
        const TUNDRA: [u8; 3] = [116, 137, 84];
        match self {
            WindSculptedDrift => {
                mesh.stone([-0.71, 0.0, 0.22], [1.20, 0.61, 0.55], POWDER);
                mesh.stone([0.61, 0.0, -0.27], [1.06, 0.43, 0.58], PACK);
                mesh.stone([0.06, 0.21, 0.02], [1.65, 0.30, 0.28], POWDER);
            }
            SnowBridgeLip => {
                mesh.stone([-0.84, 0.0, 0.0], [0.86, 0.69, 0.57], PACK);
                mesh.stone([0.76, 0.0, 0.12], [0.95, 0.50, 0.58], PACK);
                mesh.wood([-0.77, 0.59, 0.13], [0.68, 0.57, -0.09], 0.29, POWDER);
                mesh.stone([0.48, 0.39, -0.35], [0.57, 0.45, 0.56], POWDER);
            }
            SnowPillows => {
                for [x, z, w, h] in [[-0.98, 0.45, 0.66, 0.56], [0.04, 0.34, 0.82, 0.67],
                                      [0.79, -0.47, 0.81, 0.91], [-0.26, -0.58, 0.58, 0.45]] {
                    mesh.stone([x, 0.0, z], [w, h, w * 0.66], POWDER);
                }
            }
            FrostedTalus => {
                for [x, z, scale] in [[-0.70, -0.38, 0.92], [0.51, -0.27, 1.10], [-0.09, 0.61, 0.63]] {
                    mesh.stone([x, 0.0, z], [0.73 * scale, scale, 0.62 * scale], SLATE);
                    mesh.stone([x, scale * 0.62, z], [0.46 * scale, 0.44 * scale, 0.39 * scale], POWDER);
                }
            }
            SnowbreakSlabs => {
                mesh.stone([-0.67, 0.0, 0.50], [0.93, 0.48, 0.46], ROCK);
                mesh.stone([0.57, 0.0, -0.29], [0.92, 1.12, 0.59], SLATE);
                mesh.stone([0.50, 0.57, -0.19], [0.68, 0.59, 0.58], POWDER);
                mesh.stone([-0.48, 0.35, 0.45], [0.55, 0.29, 0.37], PACK);
            }
            BlueIceRibs => {
                for [x, z, h] in [[-0.89, 0.46, 0.86], [-0.40, -0.08, 1.25], [0.17, -0.33, 1.58], [0.74, -0.51, 1.12]] {
                    mesh.stone([x, 0.0, z], [0.27, h, 0.74], ICE);
                    mesh.stone([x + 0.03, h * 0.64, z - 0.18], [0.19, h * 0.38, 0.51], PACK);
                }
            }
            PressureHummocks => {
                mesh.stone([0.09, 0.0, 0.0], [1.29, 1.09, 0.63], BLUE);
                mesh.stone([-0.63, 0.48, 0.03], [0.72, 0.83, 0.54], ICE);
                mesh.stone([0.41, 0.69, -0.14], [0.73, 1.03, 0.42], PACK);
                mesh.stone([0.92, 0.0, 0.38], [0.65, 0.38, 0.54], ICE);
            }
            MeltwaterRunnels => {
                for [x, z, h] in [[-1.0, -0.31, 0.31], [-0.51, -0.27, 0.39], [0.0, 0.14, 0.37], [0.53, 0.37, 0.38], [1.04, 0.34, 0.29]] {
                    mesh.stone([x, 0.0, z], [0.47, h, 0.28], BLUE);
                    mesh.stone([x, 0.0, z + 0.48], [0.44, h * 0.90, 0.25], PACK);
                }
            }
            RefrozenLens => {
                mesh.stone([0.0, 0.0, 0.0], [1.30, 0.29, 0.76], ICE);
                mesh.stone([-0.18, 0.14, -0.16], [1.00, 0.24, 0.53], BLUE);
                mesh.stone([0.65, 0.10, 0.29], [0.63, 0.31, 0.33], PACK);
                mesh.stone([-0.80, 0.0, 0.48], [0.45, 0.39, 0.36], POWDER);
            }
            LayeredIceStep => {
                mesh.stone([0.0, 0.0, 0.11], [1.17, 0.61, 0.89], BLUE);
                mesh.stone([-0.16, 0.43, -0.16], [1.02, 0.46, 0.66], ICE);
                mesh.stone([-0.24, 0.78, -0.44], [0.83, 0.59, 0.51], PACK);
                mesh.stone([-0.40, 1.17, -0.57], [0.51, 0.36, 0.32], POWDER);
            }
            GarnetScree => {
                for [x, z, r] in [[-1.16, 0.45, 0.40], [-0.58, -0.25, 0.55], [0.13, -0.66, 0.38],
                                   [0.32, 0.49, 0.52], [1.09, 0.31, 0.56], [1.23, -0.48, 0.35]] {
                    mesh.stone([x, 0.0, z], [r, r * 0.76, r * 0.85], [126, 92, 102]);
                    mesh.flower([x, r * 0.72, z], r * 0.37, [163, 104, 116]);
                }
            }
            ShaleBook => {
                mesh.stone([0.0, 0.0, 0.0], [1.13, 0.32, 1.06], ROCK);
                for [x, z, h] in [[-0.68, 0.24, 0.88], [-0.35, 0.07, 1.49], [-0.01, -0.16, 1.97], [0.31, -0.36, 2.09], [0.63, -0.56, 1.51]] {
                    mesh.stone([x, 0.2, z], [0.26, h, 0.88], SLATE);
                }
            }
            QuartziteNeedles => {
                mesh.stone([-0.61, 0.0, 0.17], [0.42, 2.36, 0.40], ROCK);
                mesh.stone([0.41, 0.0, -0.31], [0.38, 2.72, 0.43], [195, 186, 164]);
                mesh.stone([0.85, 0.0, 0.42], [0.37, 1.26, 0.37], ROCK);
                mesh.stone([-0.41, 0.0, -0.58], [0.45, 0.72, 0.32], LICHEN);
            }
            FrostSplitTor => {
                mesh.stone([-0.69, 0.0, 0.0], [0.63, 2.13, 0.86], ROCK);
                mesh.stone([0.51, 0.0, 0.04], [0.48, 1.77, 0.74], SLATE);
                mesh.stone([-0.53, 1.72, -0.24], [0.58, 0.78, 0.54], PACK);
                mesh.stone([0.46, 1.36, 0.01], [0.39, 0.54, 0.42], ROCK);
            }
            VeinedSchist => {
                mesh.stone([0.0, 0.0, 0.13], [1.37, 0.45, 0.81], SLATE);
                mesh.stone([-0.07, 0.30, 0.09], [1.21, 0.29, 0.70], PACK);
                mesh.stone([0.12, 0.50, -0.10], [1.00, 0.56, 0.62], SLATE);
                mesh.stone([-0.28, 0.83, -0.10], [0.68, 0.23, 0.42], ROCK);
            }
            MixedErratics => {
                mesh.stone([-0.58, 0.0, -0.06], [0.93, 1.14, 0.75], [162, 127, 106]);
                mesh.stone([0.71, 0.0, 0.22], [0.74, 1.65, 0.82], SLATE);
                mesh.stone([-0.16, 0.0, 0.86], [0.52, 0.62, 0.42], ROCK);
                mesh.flower([0.75, 1.27, 0.12], 0.27, LICHEN);
            }
            SortedStoneRing => {
                for [x, z] in [[1.24, 0.1], [0.66, 0.98], [-0.61, 0.98], [-1.24, 0.1], [-0.62, -0.86], [0.62, -0.86]] {
                    mesh.stone([x, 0.0, z], [0.49, 0.30, 0.47], ROCK);
                }
                mesh.stone([0.03, 0.0, 0.04], [0.63, 0.27, 0.57], LICHEN);
            }
            IceMarginalBank => {
                mesh.stone([-0.77, 0.0, 0.15], [0.97, 0.69, 0.67], ROCK);
                mesh.stone([0.61, 0.0, -0.12], [0.98, 0.86, 0.70], SLATE);
                mesh.stone([-0.24, 0.40, -0.33], [0.87, 0.84, 0.52], ROCK);
                mesh.stone([0.50, 0.16, 0.51], [0.72, 0.43, 0.49], PACK);
            }
            GlacialBoulderFan => {
                for [x, z, r] in [[-1.18, 0.53, 0.41], [-0.59, 0.36, 0.55], [0.01, -0.49, 0.78],
                                   [0.49, 0.18, 0.52], [1.17, 0.72, 0.36], [0.73, 0.97, 0.42]] {
                    mesh.stone([x, 0.0, z], [r, r * 1.2, r * 0.89], ROCK);
                }
            }
            MoraineCobbleRibs => {
                for [x, z, r] in [[-1.15, -0.44, 0.51], [-0.47, -0.31, 0.64], [0.33, -0.41, 0.55],
                                   [-0.72, 0.47, 0.49], [0.03, 0.60, 0.67], [0.88, 0.43, 0.62]] {
                    mesh.stone([x, 0.0, z], [r, r * 0.52, r * 0.61], LICHEN);
                }
            }
            WhiteSaxifrage => {
                for [x, z, r] in [[-0.76, 0.25, 0.68], [0.16, -0.47, 0.61], [0.75, 0.23, 0.59]] {
                    mesh.stone([x, 0.0, z], [r, r * 0.69, r * 0.83], TUNDRA);
                    mesh.flower([x, r * 0.67, z], 0.32, POWDER);
                    mesh.flower([x + 0.29, r * 0.50, z - 0.22], 0.21, POWDER);
                }
            }
            AlpinePoppy => {
                for [x, z, h] in [[-0.81, 0.36, 0.66], [-0.26, -0.30, 1.30], [0.38, 0.26, 1.22], [0.86, -0.42, 0.81]] {
                    mesh.rosette([x * 0.83, 0.0, z], 0.42, 0.40, TUNDRA);
                    mesh.stalk([x * 0.83, 0.0, z], [x, h, z], 0.08, TUNDRA);
                    mesh.flower([x, h, z], 0.27, [233, 175, 62]);
                }
            }
            WillowMat => {
                mesh.wood([-1.17, 0.05, 0.43], [1.17, 0.37, 0.12], 0.16, [116, 105, 83]);
                for [x, z, r] in [[-0.99, 0.48, 0.58], [-0.41, -0.48, 0.66], [0.30, 0.07, 0.74], [0.97, -0.28, 0.57]] {
                    mesh.rosette([x, 0.17, z], r, 0.50, TUNDRA);
                }
            }
            LichenCushions => {
                for [x, z, r] in [[-0.92, -0.36, 0.67], [0.04, 0.26, 0.79], [0.91, -0.40, 0.58]] {
                    mesh.stone([x, 0.0, z], [r, r * 0.55, r * 0.79], ROCK);
                    mesh.stone([x + 0.05, r * 0.30, z - 0.15], [r * 0.71, r * 0.38, r * 0.63], LICHEN);
                }
            }
            SnowbedSedge => {
                for [x, z, h] in [[-0.75, -0.24, 1.34], [0.01, 0.39, 1.80], [0.68, -0.28, 1.42]] {
                    mesh.rosette([x, 0.0, z], 0.63, h * 0.78, TUNDRA);
                    mesh.stalk([x, 0.0, z], [x + 0.27, h, z - 0.09], 0.10, LICHEN);
                }
                mesh.stone([0.70, 0.0, 0.45], [0.46, 0.21, 0.36], PACK);
            }
        }
    }
}
