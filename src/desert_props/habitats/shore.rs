use super::Mesh;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(in crate::desert_props) enum ShoreFeature {
    ShellBank,
    PumiceRaft,
    KelpStipes,
    TangledRoots,
    OysterRidge,
    IcePlant,
    DuneSedge,
    SandVerbena,
    BeachPea,
    SaltPrunedPine,
    PebbleTerrace,
    BoulderTripod,
    BasaltFingers,
    SplitSlab,
    ErodedSandstone,
    HauledSkiff,
    LobsterPots,
    NetRack,
    FishCrates,
    OarTrestle,
    RescueStand,
    DuneGate,
    ShadeSail,
    BeachSteps,
    TrailKiosk,
}

use ShoreFeature::*;

const STRAND: [ShoreFeature; 5] = [ShellBank, PumiceRaft, KelpStipes, TangledRoots, OysterRidge];
const DUNES: [ShoreFeature; 5] = [IcePlant, DuneSedge, SandVerbena, BeachPea, SaltPrunedPine];
const STONE: [ShoreFeature; 5] = [
    PebbleTerrace,
    BoulderTripod,
    BasaltFingers,
    SplitSlab,
    ErodedSandstone,
];
const WORKING: [ShoreFeature; 5] = [HauledSkiff, LobsterPots, NetRack, FishCrates, OarTrestle];
const ACCESS: [ShoreFeature; 5] = [RescueStand, DuneGate, ShadeSail, BeachSteps, TrailKiosk];

impl ShoreFeature {
    pub(super) fn choose(elevation: f32, key: u64, district: u64) -> Option<Self> {
        // The low strand stays mostly natural. Working gear sits further up
        // dry sand, while stairs and shelters belong behind the high strand.
        let group = match (district >> 16) & 7 {
            0 | 1 if elevation < 1.40 => &STRAND,
            2 | 3 if elevation >= 1.15 => &DUNES,
            4 | 5 => &STONE,
            6 if (0.85..=1.95).contains(&elevation) => &WORKING,
            7 if elevation >= 1.55 => &ACCESS,
            _ => return None,
        };
        Some(group[((key >> 42) % 5) as usize])
    }

    pub(super) fn draw(self, model: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        const WOOD: [u8; 3] = [138, 111, 81];
        const AGED: [u8; 3] = [157, 151, 132];
        const STONE: [u8; 3] = [125, 131, 130];
        const SHELL: [u8; 3] = [221, 199, 164];
        const FOLIAGE: [u8; 3] = [99, 132, 69];
        const PAINT: [u8; 3] = [157, 84, 61];
        match self {
            ShellBank => {
                for [x, z, w, h] in [
                    [-1.18, 0.21, 0.62, 0.25],
                    [-0.44, -0.22, 0.53, 0.38],
                    [0.24, 0.22, 0.45, 0.21],
                    [0.80, 0.44, 0.54, 0.31],
                    [1.32, 0.19, 0.33, 0.22],
                ] {
                    model.stone([x, 0.0, z], [w, h, w * 0.52], SHELL);
                }
            }
            PumiceRaft => {
                for [x, z, w, h] in [
                    [-0.9, -0.4, 0.68, 0.49],
                    [0.11, -0.55, 0.56, 0.33],
                    [-0.32, 0.31, 0.49, 0.28],
                    [0.55, 0.55, 0.69, 0.51],
                ] {
                    model.stone([x, 0.0, z], [w, h, w * 0.82], [155, 139, 124]);
                }
            }
            KelpStipes => {
                for [x, z, h] in [
                    [-1.01, -0.38, 0.53],
                    [-0.49, 0.47, 0.45],
                    [0.14, -0.42, 0.68],
                    [0.80, 0.32, 0.51],
                ] {
                    model.stalk([0.0, 0.0, 0.0], [x, h, z], 0.19, [105, 88, 48]);
                    model.flower([x, h, z], 0.24, [134, 111, 57]);
                }
            }
            TangledRoots => {
                model.wood([-1.53, 0.0, 0.08], [0.45, 0.66, -0.15], 0.27, AGED);
                for [x, y, z] in [
                    [1.21, 1.15, -0.68],
                    [1.18, 0.66, 0.68],
                    [0.08, 1.44, 0.70],
                    [1.46, 0.0, 0.19],
                ] {
                    model.wood([0.42, 0.60, -0.12], [x, y, z], 0.17, AGED);
                }
            }
            OysterRidge => {
                model.stone([0.0, 0.0, 0.05], [1.48, 0.23, 0.54], [114, 125, 101]);
                for [x, z, w, h] in [
                    [-0.97, 0.10, 0.42, 0.56],
                    [-0.40, -0.23, 0.30, 0.73],
                    [0.13, 0.16, 0.42, 0.88],
                    [0.72, -0.21, 0.34, 0.63],
                    [1.10, 0.12, 0.29, 0.56],
                ] {
                    model.stone([x, 0.03, z], [w, h, 0.24], [179, 183, 164]);
                }
            }
            IcePlant => {
                for [x, z, w] in [[-0.76, 0.23, 0.66], [0.03, -0.43, 0.77], [0.69, 0.33, 0.57]] {
                    model.stone([x, 0.0, z], [w, w * 0.63, w], [85, 122, 71]);
                    model.flower([x + 0.1, w * 0.63, z], 0.25, [196, 92, 148]);
                }
            }
            DuneSedge => {
                for [x, z, span, rise] in [
                    [-0.77, 0.21, 0.65, 1.66],
                    [0.36, 0.14, 0.87, 1.43],
                    [0.12, -0.57, 0.53, 1.19],
                ] {
                    model.rosette([x, 0.0, z], span, rise, [159, 146, 85]);
                    model.stalk(
                        [x, 0.0, z],
                        [x - 0.25, rise + 0.33, z + 0.14],
                        0.13,
                        [190, 168, 102],
                    );
                }
            }
            SandVerbena => {
                model.rosette([-0.43, 0.0, 0.13], 1.08, 0.46, FOLIAGE);
                model.rosette([0.53, 0.0, 0.17], 0.77, 0.41, FOLIAGE);
                for [x, y, z] in [
                    [-0.87, 0.54, -0.32],
                    [-0.13, 0.78, 0.27],
                    [0.55, 0.48, -0.34],
                    [0.97, 0.55, 0.20],
                ] {
                    model.stalk([x, 0.0, z], [x, y, z], 0.09, FOLIAGE);
                    model.flower([x, y, z], 0.24, [171, 123, 172]);
                }
            }
            BeachPea => {
                for [x, z, h] in [
                    [-1.13, -0.18, 0.43],
                    [-0.62, 0.57, 0.74],
                    [0.0, -0.33, 1.01],
                    [0.56, 0.56, 0.76],
                    [1.16, 0.05, 0.45],
                ] {
                    model.stalk([0.0, 0.0, 0.0], [x, h, z], 0.16, [88, 138, 87]);
                    model.stone([x, h * 0.60, z], [0.24, 0.34, 0.19], [134, 94, 151]);
                }
            }
            SaltPrunedPine => {
                model.wood([-0.36, 0.0, 0.0], [0.18, 2.42, 0.08], 0.25, WOOD);
                for [x, y, z, w] in [
                    [0.36, 1.29, -0.62, 0.70],
                    [0.62, 1.77, 0.35, 0.85],
                    [0.50, 2.17, -0.07, 0.71],
                    [-0.16, 1.56, 0.69, 0.55],
                ] {
                    model.wood([0.01, y * 0.74, 0.02], [x, y, z], 0.13, WOOD);
                    model.stone([x, y, z], [w, 0.42, w * 0.72], [76, 113, 83]);
                }
            }
            PebbleTerrace => {
                for [z, w, h, d] in [
                    [0.62, 1.62, 0.16, 0.62],
                    [0.17, 1.40, 0.39, 0.69],
                    [-0.43, 1.17, 0.70, 0.58],
                ] {
                    model.stone([0.0, 0.0, z], [w, h, d], STONE);
                }
            }
            BoulderTripod => {
                for [x, z, w, h] in [
                    [-0.70, 0.34, 0.63, 1.43],
                    [0.62, 0.37, 0.56, 1.91],
                    [-0.12, -0.63, 0.66, 2.24],
                ] {
                    model.stone([x, 0.0, z], [w, h, w * 0.84], [135, 126, 119]);
                }
            }
            BasaltFingers => {
                for [x, z, h] in [
                    [-0.93, 0.03, 1.53],
                    [-0.45, -0.18, 2.61],
                    [0.02, 0.11, 2.81],
                    [0.48, 0.17, 1.81],
                    [0.93, 0.27, 1.11],
                ] {
                    model.block([x, 0.0, z], [0.23, h, 0.31], [100, 109, 113]);
                }
            }
            SplitSlab => {
                model.stone([-0.68, 0.0, 0.1], [0.66, 1.24, 0.91], STONE);
                model.stone([0.68, 0.0, -0.03], [0.58, 0.77, 0.94], STONE);
                model.stone([0.19, 0.0, 0.98], [0.43, 0.22, 0.38], [164, 149, 133]);
            }
            ErodedSandstone => {
                model.stone([-0.61, 0.0, 0.0], [1.01, 1.38, 0.66], [170, 123, 84]);
                model.stone([0.62, 0.0, -0.16], [0.67, 1.77, 0.69], [191, 139, 91]);
                model.stone([-0.22, 0.70, -0.14], [1.09, 0.44, 0.43], [205, 153, 106]);
            }
            HauledSkiff => {
                let outline = [
                    [-1.78, 0.0],
                    [-1.18, -0.59],
                    [0.92, -0.54],
                    [1.48, 0.0],
                    [0.92, 0.54],
                    [-1.18, 0.59],
                ];
                for side in 0..outline.len() {
                    let [x0, z0] = outline[side];
                    let [x1, z1] = outline[(side + 1) % outline.len()];
                    model.quad(
                        [
                            [x0 * 0.76, 0.0, z0 * 0.72],
                            [x1 * 0.76, 0.0, z1 * 0.72],
                            [x1, 0.57, z1],
                            [x0, 0.57, z0],
                        ],
                        [79, 132, 136],
                    );
                    model.quad(
                        [
                            [x0, 0.57, z0],
                            [x1, 0.57, z1],
                            [x1 * 0.83, 0.46, z1 * 0.80],
                            [x0 * 0.83, 0.46, z0 * 0.80],
                        ],
                        AGED,
                    );
                }
                model.block([-0.30, 0.25, 0.0], [0.13, 0.27, 0.53], WOOD);
                model.block([0.59, 0.24, 0.0], [0.13, 0.26, 0.49], WOOD);
            }
            LobsterPots => {
                for [x, z, height] in [
                    [-0.72, 0.14, 0.60],
                    [0.35, -0.31, 0.60],
                    [0.66, 0.54, 0.67],
                    [-0.59, -0.29, 1.22],
                ] {
                    model.gable([x, height - 0.60, z], [0.49, 0.60, 0.36], [99, 107, 76]);
                    model.stalk(
                        [x - 0.43, height - 0.58, z + 0.37],
                        [x + 0.43, height - 0.16, z + 0.37],
                        0.12,
                        AGED,
                    );
                }
            }
            NetRack => {
                for x in [-1.32, 1.32] {
                    model.block([x, 0.0, 0.0], [0.11, 1.9, 0.12], WOOD);
                }
                model.block([0.0, 1.84, 0.0], [1.47, 0.15, 0.11], AGED);
                model.quad(
                    [
                        [-1.22, 1.77, 0.08],
                        [1.22, 1.77, 0.08],
                        [0.82, 0.46, 0.28],
                        [-1.04, 0.82, 0.35],
                    ],
                    [123, 146, 116],
                );
            }
            FishCrates => {
                model.block([-0.68, 0.0, 0.12], [0.55, 0.59, 0.47], [120, 144, 143]);
                model.block([-0.76, 0.59, 0.05], [0.43, 0.48, 0.40], [86, 115, 127]);
                model.block([0.54, 0.0, -0.19], [0.62, 0.46, 0.53], [194, 161, 102]);
                model.block([0.56, 0.45, -0.15], [0.62, 0.09, 0.53], SHELL);
            }
            OarTrestle => {
                for x in [-0.63, 0.63] {
                    model.wood([x, 0.0, -0.38], [x, 1.10, 0.0], 0.14, WOOD);
                    model.wood([x, 0.0, 0.43], [x, 1.10, 0.0], 0.14, WOOD);
                }
                for [z, shift] in [[-0.30, 0.1], [0.05, -0.12], [0.37, 0.20]] {
                    model.wood(
                        [-1.46 + shift, 1.08, z],
                        [1.46 + shift, 1.13, z],
                        0.10,
                        AGED,
                    );
                    model.block([1.34 + shift, 1.10, z], [0.32, 0.10, 0.13], PAINT);
                }
            }
            RescueStand => {
                for x in [-0.63, 0.63] {
                    for z in [-0.47, 0.47] {
                        model.block([x, 0.0, z], [0.11, 1.15, 0.11], WOOD);
                    }
                }
                model.block([0.0, 1.12, 0.0], [0.86, 0.17, 0.71], AGED);
                model.block([0.0, 1.25, 0.0], [0.59, 1.08, 0.43], [209, 166, 104]);
                model.gable([0.0, 2.27, 0.0], [0.81, 0.50, 0.68], PAINT);
                model.block([0.0, 0.0, 0.76], [0.26, 0.35, 0.17], AGED);
                model.block([0.0, 0.33, 0.66], [0.26, 0.36, 0.15], AGED);
            }
            DuneGate => {
                for x in [-0.88, 0.88] {
                    model.block([x, 0.0, 0.0], [0.14, 1.88, 0.17], WOOD);
                }
                model.block([0.0, 1.55, 0.0], [1.04, 0.18, 0.13], AGED);
                model.block([-1.41, 0.53, 0.02], [0.51, 0.08, 0.07], WOOD);
                model.block([1.39, 0.53, 0.02], [0.49, 0.08, 0.07], WOOD);
            }
            ShadeSail => {
                let [a, b, c] = [
                    [-1.14, 2.28, -0.71],
                    [1.12, 1.59, -0.42],
                    [0.14, 1.91, 1.03],
                ];
                for [x, y, z] in [a, b, c] {
                    model.block([x, 0.0, z], [0.09, y, 0.09], WOOD);
                }
                (model.0)([a, b, c], [200, 159, 104]);
            }
            BeachSteps => {
                for i in 0..5 {
                    let step = i as f32;
                    model.block(
                        [0.0, 0.0, 1.05 - 0.43 * step],
                        [0.73, 0.22 + 0.20 * step, 0.24],
                        AGED,
                    );
                }
                for x in [-0.77, 0.77] {
                    model.wood([x, 0.54, 1.21], [x, 1.63, -0.87], 0.13, WOOD);
                }
            }
            TrailKiosk => {
                for x in [-0.58, 0.58] {
                    model.block([x, 0.0, 0.0], [0.12, 2.30, 0.13], WOOD);
                }
                model.block([0.0, 1.09, 0.0], [0.62, 0.88, 0.115], SHELL);
                model.block([0.0, 1.30, 0.12], [0.37, 0.48, 0.025], [100, 139, 108]);
                model.gable([0.0, 1.99, 0.0], [0.84, 0.49, 0.44], [92, 102, 103]);
            }
        }
    }
}
