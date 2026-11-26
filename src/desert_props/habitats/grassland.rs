//! Small forms for open ground between forests, farms, and hills.

use super::Mesh;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum GrasslandFeature {
    OxeyeDaisy,
    Knapweed,
    BirdsfootTrefoil,
    Chicory,
    YarrowUmbels,
    BlueStemFan,
    FescueTufts,
    OatGrass,
    FoxtailPatch,
    FeatherGrass,
    HawthornScrub,
    GorseCushion,
    BroomStand,
    DogRose,
    JuniperMound,
    LimestonePavement,
    ChalkHummocks,
    GlacialBoulder,
    ScatteredFlint,
    LowStoneBank,
    WindPrunedThorn,
    WeatheredLog,
    BrackenIsland,
    ThornRootBall,
    MeadowAnthill,
}

impl GrasslandFeature {
    pub(super) fn choose(key: u64, patch: u64) -> Self {
        use GrasslandFeature::*;
        const FORMS: [[GrasslandFeature; 5]; 5] = [
            [
                OxeyeDaisy,
                Knapweed,
                BirdsfootTrefoil,
                Chicory,
                YarrowUmbels,
            ],
            [
                BlueStemFan,
                FescueTufts,
                OatGrass,
                FoxtailPatch,
                FeatherGrass,
            ],
            [
                HawthornScrub,
                GorseCushion,
                BroomStand,
                DogRose,
                JuniperMound,
            ],
            [
                LimestonePavement,
                ChalkHummocks,
                GlacialBoulder,
                ScatteredFlint,
                LowStoneBank,
            ],
            [
                WindPrunedThorn,
                WeatheredLog,
                BrackenIsland,
                ThornRootBall,
                MeadowAnthill,
            ],
        ];
        let family = (patch >> 24) % FORMS.len() as u64;
        let choice = if key >> 42 & 3 == 0 {
            key >> 33
        } else {
            (patch >> 12) + (key >> 46 & 1)
        };
        FORMS[family as usize][(choice % 5) as usize]
    }

    pub(super) fn draw(self, m: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        use GrasslandFeature::*;
        const GRASS: [u8; 3] = [120, 151, 82];
        const LEAF: [u8; 3] = [82, 129, 71];
        const DRY: [u8; 3] = [173, 160, 102];
        const BARK: [u8; 3] = [120, 101, 80];
        const PALE: [u8; 3] = [170, 156, 132];
        const STONE: [u8; 3] = [140, 139, 127];
        match self {
            OxeyeDaisy => {
                for [x, z, h] in [
                    [-0.80, 0.14, 0.81],
                    [-0.38, -0.33, 0.65],
                    [0.14, 0.26, 0.91],
                    [0.67, 0.38, 0.67],
                ] {
                    m.stalk([x * 0.8, 0.0, z], [x, h, z], 0.07, LEAF);
                    m.flower([x, h, z], 0.29, [229, 227, 197]);
                    m.flower([x, h + 0.10, z], 0.10, [199, 171, 62]);
                }
            }
            Knapweed => {
                for [x, z, h] in [[-0.49, 0.10, 1.34], [0.09, -0.23, 1.79], [0.54, 0.33, 1.20]] {
                    m.rosette([x, 0.0, z], 0.39, 0.45, GRASS);
                    m.stalk([x, 0.0, z], [x + 0.12, h, z], 0.075, LEAF);
                    m.stone([x + 0.11, h - 0.12, z], [0.20, 0.34, 0.22], [169, 104, 146]);
                }
            }
            BirdsfootTrefoil => {
                for [x, z, h] in [
                    [-0.98, -0.12, 0.31],
                    [-0.33, 0.22, 0.52],
                    [0.31, 0.37, 0.45],
                    [0.86, -0.05, 0.32],
                ] {
                    m.stone([x, 0.0, z], [0.48, h, 0.36], LEAF);
                    m.flower([x - 0.17, h * 0.79, z - 0.1], 0.17, [225, 178, 63]);
                    m.flower([x + 0.12, h * 0.86, z + 0.12], 0.16, [228, 194, 73]);
                }
            }
            Chicory => {
                m.rosette([0.0, 0.0, 0.0], 0.71, 0.40, GRASS);
                for [x, y, z] in [[-0.63, 0.94, 0.20], [0.13, 1.46, 0.06], [0.58, 1.10, -0.16]] {
                    m.stalk([0.0, 0.0, 0.0], [x, y, z], 0.075, LEAF);
                    m.flower([x, y, z], 0.25, [113, 146, 204]);
                    m.flower([x * 0.75, y * 0.68, z + 0.18], 0.15, [141, 162, 216]);
                }
            }
            YarrowUmbels => {
                for [x, z, h] in [
                    [-0.66, 0.28, 0.92],
                    [-0.17, -0.31, 1.25],
                    [0.48, 0.22, 1.07],
                ] {
                    m.stalk([x * 0.47, 0.0, z], [x, h, z], 0.09, GRASS);
                    for [dx, dz] in [[-0.25, 0.01], [0.13, -0.21], [0.23, 0.19]] {
                        m.flower([x + dx, h, z + dz], 0.18, [221, 214, 174]);
                    }
                }
            }
            BlueStemFan => {
                for [x, z, h] in [
                    [-0.76, 0.27, 1.07],
                    [-0.21, -0.31, 1.61],
                    [0.31, 0.18, 1.49],
                    [0.77, 0.41, 1.18],
                ] {
                    m.stalk([x * 0.43, 0.0, z * 0.44], [x, h, z], 0.10, [101, 133, 111]);
                    m.rosette([x * 0.50, h * 0.39, z * 0.55], 0.44, 0.62, GRASS);
                    m.rosette([x, h, z], 0.28, 0.22, [138, 106, 85]);
                }
            }
            FescueTufts => {
                for [x, z, w, h] in [
                    [-0.62, 0.17, 0.56, 0.84],
                    [-0.06, -0.38, 0.73, 1.07],
                    [0.57, 0.26, 0.60, 0.74],
                ] {
                    m.stone([x, 0.0, z], [w * 0.72, h * 0.23, w * 0.68], [90, 114, 74]);
                    m.rosette([x, 0.06, z], w, h, GRASS);
                    m.rosette([x, 0.12, z], w * 0.76, h * 0.68, [151, 164, 99]);
                }
            }
            OatGrass => {
                for [x, z, h] in [
                    [-0.64, 0.22, 1.24],
                    [-0.19, -0.31, 1.86],
                    [0.38, 0.18, 1.52],
                    [0.70, 0.40, 1.04],
                ] {
                    m.stalk([x, 0.0, z], [x + 0.29, h, z], 0.063, DRY);
                    for t in [0.72, 0.84, 0.96] {
                        m.stalk(
                            [x + 0.25 * t, h * t, z],
                            [x + 0.54 * t, h * t - 0.09, z + 0.20],
                            0.087,
                            PALE,
                        );
                    }
                }
            }
            FoxtailPatch => {
                for [x, z, h] in [
                    [-0.79, 0.05, 0.93],
                    [-0.37, -0.26, 1.38],
                    [0.15, 0.30, 1.70],
                    [0.63, 0.22, 1.28],
                ] {
                    m.rosette([x, 0.0, z], 0.41, 0.45, GRASS);
                    m.stalk([x, 0.0, z], [x + 0.14, h, z], 0.075, LEAF);
                    m.stone([x + 0.11, h - 0.44, z], [0.15, 0.52, 0.13], [178, 143, 79]);
                }
            }
            FeatherGrass => {
                for [x, z, h] in [
                    [-0.78, 0.17, 0.88],
                    [-0.41, -0.36, 1.12],
                    [0.10, -0.15, 1.43],
                    [0.58, 0.26, 1.07],
                ] {
                    m.stalk([x, 0.0, z], [x + 0.17, h, z], 0.08, GRASS);
                    m.stalk(
                        [x + 0.17, h, z],
                        [x + 0.64, h + 0.23, z + 0.17],
                        0.15,
                        [208, 192, 148],
                    );
                }
            }
            HawthornScrub => {
                m.wood([0.0, 0.0, 0.0], [-0.18, 1.57, 0.04], 0.15, BARK);
                for [x, y, z] in [[-0.63, 1.07, 0.22], [0.06, 1.51, -0.26], [0.57, 1.23, 0.14]] {
                    m.wood([-0.09, 0.49, 0.0], [x, y, z], 0.09, BARK);
                    m.stone([x, y - 0.17, z], [0.54, 0.58, 0.47], LEAF);
                    m.flower([x + 0.13, y + 0.34, z], 0.15, [221, 210, 185]);
                }
            }
            GorseCushion => {
                for [x, z, w] in [
                    [-0.70, 0.26, 0.60],
                    [-0.12, -0.25, 0.74],
                    [0.57, 0.30, 0.63],
                ] {
                    m.stone([x, 0.0, z], [w, w * 0.98, w], LEAF);
                    m.rosette([x, w * 0.28, z], w, w * 0.66, GRASS);
                    m.flower([x + 0.12, w * 0.89, z], 0.23, [220, 174, 56]);
                }
            }
            BroomStand => {
                for [x, y, z] in [
                    [-0.75, 1.33, 0.17],
                    [-0.37, 1.79, -0.39],
                    [0.04, 2.16, 0.32],
                    [0.66, 1.57, 0.14],
                ] {
                    m.stalk([x * 0.22, 0.0, z * 0.2], [x, y, z], 0.13, LEAF);
                    m.rosette([x * 0.77, y * 0.56, z * 0.77], 0.38, 0.42, GRASS);
                    m.stone([x, y - 0.25, z], [0.20, 0.42, 0.19], [225, 187, 74]);
                }
            }
            DogRose => {
                m.wood([-0.75, 0.0, -0.26], [-0.26, 1.27, 0.02], 0.12, BARK);
                m.wood([-0.26, 1.27, 0.02], [0.94, 0.48, 0.38], 0.12, BARK);
                for [x, y, z] in [
                    [-0.54, 0.93, -0.07],
                    [-0.13, 1.32, 0.15],
                    [0.52, 0.95, 0.30],
                    [0.91, 0.59, 0.31],
                ] {
                    m.rosette([x, y - 0.10, z], 0.47, 0.35, LEAF);
                    m.flower([x, y + 0.18, z], 0.24, [215, 154, 172]);
                }
            }
            JuniperMound => {
                m.wood([-0.52, 0.0, 0.18], [0.54, 0.56, -0.18], 0.16, BARK);
                for [x, y, z, w] in [
                    [-0.62, 0.08, 0.23, 0.69],
                    [-0.09, 0.40, -0.26, 0.69],
                    [0.52, 0.35, 0.22, 0.65],
                    [0.95, 0.12, -0.14, 0.46],
                ] {
                    m.stone([x, y, z], [w, 0.50, 0.44], [82, 125, 95]);
                }
            }
            LimestonePavement => {
                for [x, z, w] in [
                    [-0.67, -0.42, 0.55],
                    [-0.69, 0.35, 0.61],
                    [0.18, -0.23, 0.42],
                    [0.77, 0.30, 0.55],
                ] {
                    m.block([x, 0.0, z], [w, 0.19 + w * 0.18, 0.31], STONE);
                }
                m.rosette([0.36, 0.0, 0.09], 0.44, 0.27, GRASS);
            }
            ChalkHummocks => {
                m.stone([-0.63, 0.0, 0.16], [0.96, 0.58, 0.70], PALE);
                m.stone([0.49, 0.0, -0.23], [1.05, 0.69, 0.68], [192, 178, 151]);
                m.stone([-0.81, 0.34, 0.28], [0.39, 0.30, 0.43], [144, 151, 97]);
                m.rosette([0.86, 0.31, 0.12], 0.48, 0.37, GRASS);
            }
            GlacialBoulder => {
                m.stone([0.09, 0.0, 0.04], [1.09, 1.47, 0.87], STONE);
                m.stone([-0.68, 0.0, 0.36], [0.59, 0.58, 0.50], PALE);
                m.stone([0.22, 0.76, -0.19], [0.66, 0.43, 0.49], [146, 151, 94]);
            }
            ScatteredFlint => {
                for [x, z, w] in [
                    [-0.92, 0.27, 0.39],
                    [-0.35, -0.43, 0.46],
                    [0.18, 0.25, 0.50],
                    [0.72, -0.22, 0.38],
                    [1.06, 0.53, 0.25],
                ] {
                    m.stone([x, 0.0, z], [w, w * 0.74, w * 0.63], [99, 102, 97]);
                }
            }
            LowStoneBank => {
                for [x, z, w, h] in [
                    [-1.22, -0.06, 0.49, 0.41],
                    [-0.52, -0.18, 0.59, 0.67],
                    [0.18, 0.06, 0.59, 0.52],
                    [0.88, 0.24, 0.47, 0.30],
                ] {
                    m.stone([x, 0.0, z], [w, h, 0.37], STONE);
                }
                m.stone([-0.38, 0.43, -0.16], [0.54, 0.31, 0.29], PALE);
            }
            WindPrunedThorn => {
                m.wood([-0.42, 0.0, 0.05], [0.39, 1.34, -0.14], 0.16, BARK);
                m.wood([-0.05, 0.59, -0.02], [1.07, 1.03, 0.35], 0.13, BARK);
                m.stone([0.46, 0.97, -0.10], [0.86, 0.47, 0.48], LEAF);
                m.rosette([0.93, 1.10, 0.24], 0.60, 0.39, GRASS);
            }
            WeatheredLog => {
                m.wood([-1.64, 0.0, -0.10], [1.49, 0.29, 0.36], 0.28, BARK);
                m.wood([-0.23, 0.46, 0.01], [0.43, 0.79, -0.49], 0.11, PALE);
                m.rosette([-0.92, 0.0, 0.33], 0.46, 0.42, GRASS);
                m.rosette([0.73, 0.0, -0.25], 0.51, 0.54, GRASS);
            }
            BrackenIsland => {
                for [x, z, h] in [
                    [-1.01, 0.22, 0.73],
                    [-0.61, -0.60, 1.02],
                    [0.31, 0.61, 1.18],
                    [0.79, -0.26, 0.92],
                ] {
                    m.stalk([0.0, 0.0, 0.0], [x, h, z], 0.19, LEAF);
                    m.rosette([x * 0.74, h * 0.60, z * 0.72], 0.48, 0.39, GRASS);
                    m.stalk(
                        [x * 0.67, h * 0.51, z * 0.67],
                        [x * 1.14, h, z * 1.11],
                        0.13,
                        GRASS,
                    );
                }
            }
            ThornRootBall => {
                m.stone([-0.31, 0.0, -0.09], [0.75, 0.73, 0.59], [123, 105, 76]);
                m.wood([-0.42, 0.15, -0.07], [1.08, 0.51, 0.59], 0.22, BARK);
                for root in [
                    [-1.18, 0.55, 0.25],
                    [-0.55, 1.14, -0.15],
                    [0.52, 0.96, -0.42],
                    [-0.69, 0.0, 0.63],
                ] {
                    m.wood([-0.31, 0.46, -0.09], root, 0.09, PALE);
                }
            }
            MeadowAnthill => {
                m.stone([-0.24, 0.0, 0.0], [0.69, 1.28, 0.56], [155, 115, 80]);
                m.stone([0.51, 0.0, 0.19], [0.58, 0.62, 0.44], [171, 128, 87]);
                m.rosette([0.70, 0.0, -0.43], 0.46, 0.52, DRY);
            }
        }
    }
}
