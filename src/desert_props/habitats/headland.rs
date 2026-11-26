use super::Mesh;
use crate::world::{Sample, Surface};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(in crate::desert_props) enum HeadlandFeature {
    SandHopperBurrows, RazorShellFan, KelpRibbon, CorkWrack, BarnacleCobbles,
    BeachMorningGlory, SeasideDaisy, HonckenyaCushion, DuneSpurge, BeachEveningPrimrose,
    GraniteShingle, FlintSpit, SandstoneLedge, IronstoneRubble, ChalkBreccia,
    DuneWillow, BayberryThicket, SeaBuckthorn, CoastalHeather, SaltBurnedJuniper,
    SandFescue, LymeGrass, SeaRush, DuneReed, SaltMeadowHay,
}

use HeadlandFeature::*;

const STRAND: [HeadlandFeature; 5] =
    [SandHopperBurrows, RazorShellFan, KelpRibbon, CorkWrack, BarnacleCobbles];
const FLOWERS: [HeadlandFeature; 5] =
    [BeachMorningGlory, SeasideDaisy, HonckenyaCushion, DuneSpurge, BeachEveningPrimrose];
const STONES: [HeadlandFeature; 5] =
    [GraniteShingle, FlintSpit, SandstoneLedge, IronstoneRubble, ChalkBreccia];
const SCRUB: [HeadlandFeature; 5] =
    [DuneWillow, BayberryThicket, SeaBuckthorn, CoastalHeather, SaltBurnedJuniper];
const GRASSES: [HeadlandFeature; 5] =
    [SandFescue, LymeGrass, SeaRush, DuneReed, SaltMeadowHay];

impl HeadlandFeature {
    pub(super) fn eligible(ground: Sample) -> bool {
        (0.73..=12.0).contains(&ground.elevation)
            && matches!(ground.surface, Surface::Sand | Surface::DuneSand | Surface::Grass | Surface::Rock)
    }

    pub(super) fn choose(ground: Sample, key: u64, patch: u64) -> Self {
        let exposure = patch >> 25 & 7;
        let family = if ground.elevation < 1.35 {
            &STRAND
        } else {
            match exposure {
                0 | 1 => &STONES,
                2 | 3 if ground.elevation >= 1.68 => &SCRUB,
                4 | 5 => &GRASSES,
                _ => &FLOWERS,
            }
        };
        let common = (patch >> 33) + (key >> 34 & 1);
        let choice = if key >> 37 & 3 == 0 { key >> 42 } else { common };
        family[(choice % 5) as usize]
    }

    pub(super) fn draw(self, mesh: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        const SAND: [u8; 3] = [184, 157, 115];
        const SHELL: [u8; 3] = [225, 214, 177];
        const WRACK: [u8; 3] = [111, 92, 48];
        const GREEN: [u8; 3] = [85, 119, 76];
        const SILVER: [u8; 3] = [134, 151, 115];
        const DRY: [u8; 3] = [172, 153, 96];
        const WOOD: [u8; 3] = [124, 109, 80];
        match self {
            SandHopperBurrows => {
                for [x, z, size] in [[-1.10, 0.34, 0.50], [0.10, -0.21, 0.7], [1.03, 0.50, 0.42]] {
                    mesh.stone([x, 0.0, z], [size, size * 0.53, size * 0.72], SAND);
                    mesh.flower([x + 0.05, size * 0.37, z - 0.12], size * 0.35, WRACK);
                }
            }
            RazorShellFan => {
                for [x, z, slant] in [[-0.96, -0.35, -0.51], [-0.48, 0.26, -0.23], [0.05, 0.47, 0.07],
                                      [0.58, 0.16, 0.20], [1.03, -0.26, 0.45]] {
                    mesh.wood([x, 0.08, z], [x + slant, 0.28, z + 0.66], 0.18, SHELL);
                }
            }
            KelpRibbon => {
                mesh.wood([-1.65, 0.12, -0.22], [1.36, 0.15, 0.34], 0.12, WRACK);
                for [x, z] in [[-1.1, 0.71], [-0.60, -0.61], [0.09, 0.73], [0.68, -0.54], [1.38, 0.63]] {
                    mesh.quad([[x - 0.43, 0.0, 0.07], [x - 0.12, 0.0, 0.02],
                               [x + 0.21, 0.18, z], [x - 0.18, 0.27, z + 0.18]], WRACK);
                }
            }
            CorkWrack => {
                mesh.wood([-1.32, 0.13, 0.21], [1.24, 0.40, -0.13], 0.28, WOOD);
                mesh.wood([-0.42, 0.25, 0.05], [-0.88, 0.66, -0.79], 0.16, SHELL);
                mesh.stone([0.42, 0.0, 0.45], [0.85, 0.24, 0.48], WRACK);
                mesh.stone([-0.83, 0.0, -0.34], [0.40, 0.42, 0.38], SAND);
            }
            BarnacleCobbles => {
                for [x, z, h] in [[-0.98, 0.05, 0.46], [0.06, -0.29, 0.84], [0.79, 0.46, 0.61]] {
                    mesh.stone([x, 0.0, z], [0.62, h, 0.44], [113, 122, 120]);
                    mesh.stone([x - 0.10, h * 0.70, z + 0.02], [0.19, 0.27, 0.17], SHELL);
                    mesh.stone([x + 0.22, h * 0.62, z - 0.07], [0.15, 0.19, 0.15], SHELL);
                }
            }
            BeachMorningGlory => {
                for [x, z, span] in [[-1.24, 0.15, 0.55], [-0.45, 0.42, 0.68], [0.31, -0.45, 0.66], [1.16, -0.13, 0.61]] {
                    mesh.rosette([x, 0.0, z], span, 0.27, GREEN);
                    mesh.flower([x + 0.09, 0.31, z], 0.33, [193, 104, 160]);
                }
            }
            SeasideDaisy => {
                mesh.rosette([0.0, 0.0, 0.0], 0.80, 0.30, SILVER);
                for [x, y, z] in [[-0.71, 0.83, 0.28], [-0.28, 1.37, -0.41],
                                  [0.33, 1.20, 0.26], [0.81, 0.73, -0.12]] {
                    mesh.stalk([x * 0.4, 0.0, z * 0.4], [x, y, z], 0.09, GREEN);
                    mesh.flower([x, y, z], 0.26, [220, 198, 206]);
                    mesh.flower([x, y + 0.06, z], 0.10, [203, 153, 56]);
                }
            }
            HonckenyaCushion => {
                for [x, z, w] in [[-0.65, -0.39, 0.65], [0.11, 0.26, 0.84], [0.73, -0.16, 0.59], [-0.52, 0.50, 0.46]] {
                    mesh.stone([x, 0.0, z], [w, w * 0.65, w * 0.81], GREEN);
                    mesh.flower([x, w * 0.62, z], 0.14, SHELL);
                }
            }
            DuneSpurge => {
                for [x, z, h] in [[-0.72, -0.3, 1.35], [0.03, 0.40, 1.82], [0.67, -0.14, 1.52]] {
                    mesh.stalk([x, 0.0, z], [x + 0.12, h, z], 0.15, GREEN);
                    mesh.rosette([x + 0.07, h * 0.41, z], 0.40, 0.51, SILVER);
                    mesh.rosette([x + 0.12, h * 0.83, z], 0.31, 0.34, [173, 171, 87]);
                }
            }
            BeachEveningPrimrose => {
                for [x, z, h] in [[-0.82, 0.21, 0.74], [0.05, -0.38, 1.50], [0.74, 0.28, 1.11]] {
                    mesh.rosette([x, 0.0, z], 0.55, 0.24, GREEN);
                    mesh.stalk([x, 0.0, z], [x + 0.23, h, z], 0.12, SILVER);
                    mesh.flower([x + 0.22, h, z], 0.36, [225, 176, 61]);
                }
            }
            GraniteShingle => {
                for [x, z, w, h] in [[-1.40, 0.31, 0.47, 0.20], [-0.67, 0.05, 0.63, 0.51],
                                      [0.17, 0.28, 0.57, 0.33], [0.84, 0.14, 0.51, 0.26], [1.33, -0.22, 0.45, 0.32]] {
                    mesh.stone([x, 0.0, z], [w, h, w * 0.68], [141, 147, 147]);
                }
            }
            FlintSpit => {
                mesh.stone([0.0, 0.0, 0.02], [1.53, 0.43, 0.65], [115, 117, 125]);
                mesh.stone([-0.44, 0.25, -0.21], [0.79, 0.23, 0.45], [78, 90, 101]);
                mesh.stone([0.70, 0.35, 0.12], [0.64, 0.28, 0.31], [94, 103, 108]);
            }
            SandstoneLedge => {
                mesh.stone([-0.43, 0.0, 0.06], [1.16, 0.62, 0.86], [177, 129, 83]);
                mesh.stone([-0.32, 0.43, 0.11], [1.04, 0.36, 0.76], [202, 163, 116]);
                mesh.stone([0.52, 0.58, -0.05], [0.72, 0.89, 0.74], [158, 110, 78]);
            }
            IronstoneRubble => {
                mesh.stone([-0.88, 0.0, -0.40], [0.64, 0.74, 0.48], [121, 83, 63]);
                mesh.stone([0.21, 0.0, 0.24], [0.89, 1.21, 0.68], [151, 102, 74]);
                mesh.stone([1.05, 0.0, -0.63], [0.49, 0.48, 0.39], [114, 76, 58]);
                mesh.stone([-0.70, 0.0, 0.65], [0.43, 0.30, 0.35], [127, 92, 71]);
            }
            ChalkBreccia => {
                mesh.stone([-0.75, 0.0, 0.18], [0.89, 1.65, 0.59], [207, 202, 178]);
                mesh.stone([0.58, 0.0, -0.31], [0.56, 2.01, 0.62], [188, 185, 164]);
                mesh.stone([0.72, 0.0, 0.65], [0.67, 0.51, 0.43], SHELL);
            }
            DuneWillow => {
                for [x, y, z, w] in [[-0.89, 1.89, 0.32, 0.57], [0.17, 2.40, -0.20, 0.68], [0.92, 1.36, 0.57, 0.74]] {
                    mesh.wood([x * 0.23, 0.0, 0.0], [x, y, z], 0.15, WOOD);
                    mesh.rosette([x, y - 0.22, z], w, 0.53, SILVER);
                    mesh.stone([x, y - 0.31, z], [w * 0.75, 0.56, w * 0.55], GREEN);
                }
            }
            BayberryThicket => {
                for [x, z, w, h] in [[-0.75, 0.2, 0.77, 1.26], [0.19, -0.22, 0.81, 1.72], [0.84, 0.44, 0.62, 0.99]] {
                    mesh.stone([x, 0.0, z], [w, h, w * 0.73], GREEN);
                    mesh.stone([x + 0.2, h * 0.54, z - 0.18], [0.39, 0.34, 0.42], [108, 133, 91]);
                }
            }
            SeaBuckthorn => {
                mesh.wood([-0.29, 0.0, 0.20], [0.14, 1.79, 0.08], 0.19, WOOD);
                for [x, y, z] in [[-0.91, 1.08, 0.28], [-0.16, 1.68, -0.33], [0.73, 2.09, 0.32], [1.19, 1.32, 0.03]] {
                    mesh.wood([0.05, y * 0.63, 0.08], [x, y, z], 0.13, WOOD);
                    mesh.rosette([x, y - 0.20, z], 0.59, 0.41, SILVER);
                    mesh.flower([x - 0.15, y + 0.19, z], 0.27, [198, 120, 53]);
                }
            }
            CoastalHeather => {
                for [x, z, w, h] in [[-0.76, 0.41, 0.64, 0.52], [-0.05, -0.24, 0.82, 0.73], [0.69, 0.43, 0.66, 0.63]] {
                    mesh.stone([x, 0.0, z], [w, h, w * 0.66], GREEN);
                    mesh.stone([x - 0.12, h * 0.65, z], [w * 0.66, 0.31, w * 0.53], [143, 102, 130]);
                }
            }
            SaltBurnedJuniper => {
                mesh.wood([-0.96, 0.0, 0.30], [-0.40, 1.18, 0.05], 0.31, WOOD);
                mesh.wood([-0.48, 1.12, 0.12], [1.10, 1.77, -0.27], 0.23, WOOD);
                for [x, y, z] in [[-0.58, 1.24, -0.41], [0.19, 1.31, 0.57], [0.73, 1.62, 0.04], [1.32, 1.57, -0.52]] {
                    mesh.stone([x, y, z], [0.67, 0.41, 0.53], GREEN);
                }
            }
            SandFescue => {
                for [x, z, h] in [[-0.75, -0.12, 0.95], [0.0, 0.26, 1.29], [0.72, -0.28, 0.78]] {
                    mesh.rosette([x, 0.0, z], 0.62, h, SILVER);
                }
            }
            LymeGrass => {
                for [x, z, h] in [[-0.67, 0.29, 1.57], [0.07, -0.27, 2.25], [0.59, 0.36, 1.82]] {
                    mesh.rosette([x, 0.0, z], 0.58, h * 0.56, [99, 133, 120]);
                    mesh.stalk([x, 0.0, z], [x + 0.18, h, z - 0.12], 0.09, SILVER);
                    mesh.stone([x + 0.17, h - 0.29, z - 0.10], [0.12, 0.37, 0.12], DRY);
                }
            }
            SeaRush => {
                mesh.stone([0.0, 0.0, 0.0], [0.89, 0.28, 0.62], WRACK);
                for [x, z, h] in [[-0.65, 0.36, 1.02], [-0.19, -0.32, 1.70], [0.15, 0.21, 1.54], [0.60, -0.32, 1.28]] {
                    mesh.stalk([x * 0.61, 0.0, z * 0.55], [x, h, z], 0.12, GREEN);
                }
            }
            DuneReed => {
                for [x, z, h] in [[-0.83, 0.04, 2.17], [-0.10, -0.42, 2.67], [0.59, 0.29, 2.41]] {
                    mesh.stalk([x, 0.0, z], [x + 0.31, h, z - 0.15], 0.13, DRY);
                    mesh.stalk([x + 0.17, h * 0.44, z], [x - 0.39, h * 0.63, z + 0.23], 0.19, SILVER);
                    mesh.wood([x + 0.27, h - 0.43, z - 0.12], [x + 0.46, h + 0.22, z - 0.18], 0.19, SAND);
                }
            }
            SaltMeadowHay => {
                for [x, z, w] in [[-1.04, -0.12, 0.65], [-0.40, 0.53, 0.70], [0.31, -0.38, 0.78], [1.05, 0.24, 0.67]] {
                    mesh.rosette([x, 0.0, z], w, 0.49, DRY);
                    mesh.stalk([x - 0.26, 0.0, z + 0.18], [x + 0.30, 0.49, z - 0.27], 0.16, GREEN);
                }
            }
        }
    }
}
