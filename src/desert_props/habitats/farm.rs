use super::Mesh;
use crate::world::Surface;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum FarmFeature {
    RoundBales,
    StrawStack,
    CoveredWagon,
    GrainAuger,
    Combine,
    GambrelBarn,
    PoleShed,
    MilkParlor,
    Glasshouse,
    RootCellar,
    TankStand,
    PumpCabin,
    HoseReel,
    StoneTrough,
    SluiceHouse,
    OrchardBins,
    VineTrellis,
    PickingLadder,
    PickingPlatform,
    ColdStore,
    ProduceStall,
    Weighbridge,
    FeedDepot,
    Windpump,
    FieldGate,
}

use FarmFeature::*;

const HARVEST: [FarmFeature; 5] = [RoundBales, StrawStack, CoveredWagon, GrainAuger, Combine];
const BUILDINGS: [FarmFeature; 5] = [GambrelBarn, PoleShed, MilkParlor, Glasshouse, RootCellar];
const WATER: [FarmFeature; 5] = [TankStand, PumpCabin, HoseReel, StoneTrough, SluiceHouse];
const ORCHARD: [FarmFeature; 5] = [
    OrchardBins,
    VineTrellis,
    PickingLadder,
    PickingPlatform,
    ColdStore,
];
const ACCESS: [FarmFeature; 5] = [ProduceStall, Weighbridge, FeedDepot, Windpump, FieldGate];

impl FarmFeature {
    pub(super) fn choose(crop: Surface, key: u64, district: u64) -> Self {
        let families = match crop {
            Surface::FieldGold => [&HARVEST, &HARVEST, &BUILDINGS, &ACCESS],
            Surface::FieldGreen => [&ORCHARD, &ORCHARD, &BUILDINGS, &WATER],
            _ => [&WATER, &WATER, &ACCESS, &BUILDINGS],
        };
        let pattern = if (key >> 42) & 3 == 0 { key >> 29 } else { district >> 33 };
        let group = families[(pattern & 3) as usize];
        group[((key >> 51) % 5) as usize]
    }

    pub(super) fn draw(self, model: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        const TIMBER: [u8; 3] = [141, 111, 77];
        const STRAW: [u8; 3] = [193, 158, 82];
        const RED: [u8; 3] = [151, 82, 61];
        const STEEL: [u8; 3] = [118, 139, 142];
        const GLASS: [u8; 3] = [128, 168, 165];
        const CREAM: [u8; 3] = [196, 180, 145];
        const GREEN: [u8; 3] = [86, 125, 69];
        match self {
            RoundBales => {
                for [x, y, z] in [[-0.74, 0.0, 0.05], [0.68, 0.0, 0.05], [-0.12, 0.90, 0.04]] {
                    model.drum([x, y, z], 0.54, 0.57, STRAW, true);
                }
            }
            StrawStack => {
                for [x, z, w, d] in [
                    [-0.51, 0.0, 0.48, 0.55],
                    [0.51, 0.02, 0.48, 0.56],
                    [0.04, -0.66, 0.86, 0.31],
                ] {
                    model.block([x, 0.0, z], [w, 0.46, d], STRAW);
                }
                model.block([0.09, 0.48, 0.02], [0.69, 0.33, 0.48], STRAW);
                model.block([0.18, 0.80, 0.07], [0.45, 0.26, 0.37], STRAW);
            }
            CoveredWagon => {
                wagon(model, [0.0, 0.0], TIMBER);
                model.gable([0.0, 0.48, 0.0], [0.77, 0.76, 0.96], CREAM);
                model.wood([0.0, 0.26, 0.94], [0.08, 0.15, 1.61], 0.13, TIMBER);
            }
            GrainAuger => {
                model.block([0.0, 0.0, 0.21], [0.69, 0.30, 0.56], STEEL);
                model.wood([-0.86, 0.12, 0.51], [0.94, 2.53, -0.53], 0.19, CREAM);
                model.wood([-0.15, 0.22, 0.07], [0.48, 1.85, -0.25], 0.11, STEEL);
                model.block([-0.93, 0.0, 0.55], [0.36, 0.27, 0.45], CREAM);
                model.block([0.95, 2.45, -0.58], [0.17, 0.25, 0.26], RED);
            }
            Combine => {
                wagon(model, [0.0, 0.01], [43, 52, 48]);
                model.block([0.0, 0.38, 0.04], [0.76, 0.75, 0.84], RED);
                model.block([-0.15, 0.99, 0.35], [0.49, 0.56, 0.47], GLASS);
                model.block([0.0, 0.12, 1.10], [1.43, 0.32, 0.43], [154, 120, 65]);
                for x in [-0.77, 0.0, 0.77] {
                    model.block([x, 0.26, 1.08], [0.045, 0.24, 0.40], CREAM);
                }
            }
            GambrelBarn => {
                model.block([0.0, 0.0, 0.0], [1.01, 1.13, 0.85], RED);
                model.block([0.0, 1.00, 0.0], [0.75, 0.44, 0.90], CREAM);
                model.gable([0.0, 1.35, 0.0], [0.92, 0.56, 0.93], STEEL);
                model.block([0.0, 0.0, 0.85], [0.42, 0.71, 0.06], TIMBER);
                model.drum([1.32, 0.0, -0.39], 0.31, 1.62, STEEL, false);
            }
            PoleShed => {
                for x in [-1.10, 0.0, 1.10] {
                    for z in [-0.71, 0.71] {
                        model.block([x, 0.0, z], [0.11, 1.48, 0.12], TIMBER);
                    }
                }
                model.gable([0.0, 1.38, 0.0], [1.32, 0.49, 0.87], CREAM);
                model.block([-0.53, 0.0, -0.37], [0.48, 0.49, 0.31], STRAW);
            }
            MilkParlor => {
                model.block([-0.34, 0.0, -0.02], [0.63, 0.94, 0.83], CREAM);
                model.gable([-0.34, 0.85, -0.02], [0.72, 0.46, 0.92], STEEL);
                model.block([0.67, 0.0, 0.27], [0.46, 0.63, 0.66], CREAM);
                model.drum([0.92, 0.0, -0.62], 0.27, 0.93, STEEL, false);
                model.block([-0.28, 0.10, 0.90], [0.42, 0.54, 0.08], RED);
            }
            Glasshouse => {
                for x in [-0.66, 0.66] {
                    model.block([x, 0.0, 0.03], [0.58, 0.68, 1.01], GLASS);
                    model.gable([x, 0.61, 0.03], [0.62, 0.59, 1.07], GLASS);
                    model.wood([x, 1.18, -1.02], [x, 1.18, 1.11], 0.08, CREAM);
                }
                model.block([0.0, 0.0, 0.03], [0.15, 0.45, 1.04], CREAM);
            }
            RootCellar => {
                model.stone([0.0, 0.0, -0.21], [1.16, 1.13, 0.97], [115, 139, 81]);
                model.block([0.0, 0.0, 0.53], [0.65, 0.73, 0.32], CREAM);
                model.gable([0.0, 0.63, 0.59], [0.72, 0.36, 0.33], TIMBER);
                model.block([0.0, 0.0, 0.85], [0.29, 0.61, 0.07], TIMBER);
                model.drum([0.23, 0.85, -0.33], 0.14, 0.49, STEEL, false);
            }
            TankStand => {
                for x in [-0.48, 0.48] {
                    for z in [-0.48, 0.48] {
                        model.wood([x * 1.18, 0.0, z * 1.18], [x, 1.48, z], 0.14, TIMBER);
                    }
                }
                model.block([0.0, 1.38, 0.0], [0.65, 0.17, 0.65], TIMBER);
                model.drum([0.0, 1.53, 0.0], 0.63, 0.72, STEEL, false);
                model.stone([0.0, 2.18, 0.0], [0.65, 0.26, 0.65], CREAM);
            }
            PumpCabin => {
                model.block([-0.28, 0.0, -0.12], [0.62, 1.02, 0.55], RED);
                model.gable([-0.28, 0.91, -0.12], [0.72, 0.53, 0.65], CREAM);
                model.block([0.63, 0.0, 0.17], [0.37, 0.18, 0.73], STEEL);
                model.wood([0.50, 0.11, 0.49], [0.52, 1.48, 0.49], 0.15, STEEL);
                model.wood([0.51, 1.37, 0.46], [1.03, 1.36, 0.53], 0.13, STEEL);
            }
            HoseReel => {
                model.block([0.0, 0.0, 0.0], [0.92, 0.24, 0.79], STEEL);
                model.drum([0.0, 0.23, -0.43], 0.71, 0.21, STEEL, true);
                model.drum([0.0, 0.40, -0.13], 0.54, 0.33, [55, 73, 66], true);
                model.drum([0.0, 0.23, 0.36], 0.71, 0.21, STEEL, true);
                model.wood([0.50, 0.12, 0.65], [1.28, 0.08, 1.10], 0.10, STEEL);
            }
            StoneTrough => {
                model.block([-0.76, 0.0, 0.0], [0.17, 0.42, 0.51], STEEL);
                model.block([0.76, 0.0, 0.0], [0.17, 0.42, 0.51], STEEL);
                for z in [-0.48, 0.48] {
                    model.block([0.0, 0.0, z], [0.79, 0.42, 0.13], STEEL);
                }
                model.block([0.0, 0.13, 0.0], [0.67, 0.09, 0.37], [65, 130, 143]);
                model.wood([0.78, 0.28, 0.40], [0.78, 1.24, 0.39], 0.11, TIMBER);
            }
            SluiceHouse => {
                model.block([0.0, 0.0, -0.51], [0.91, 0.81, 0.41], TIMBER);
                model.gable([0.0, 0.75, -0.52], [0.99, 0.44, 0.47], RED);
                for x in [-0.68, 0.68] {
                    model.block([x, 0.0, 0.36], [0.12, 0.64, 0.73], STEEL);
                }
                model.block([0.0, 0.12, 0.34], [0.55, 0.05, 0.74], [78, 132, 144]);
                model.block([0.0, 0.06, 0.36], [0.59, 0.62, 0.08], TIMBER);
            }
            OrchardBins => {
                for [x, z, y] in [[-0.68, -0.16, 0.0], [0.49, 0.31, 0.0], [-0.61, -0.23, 0.48]] {
                    model.block([x, y, z], [0.50, 0.46, 0.39], TIMBER);
                    model.stone([x, y + 0.39, z], [0.43, 0.29, 0.31], [165, 94, 61]);
                }
            }
            VineTrellis => {
                for x in [-0.90, 0.0, 0.90] {
                    for z in [-0.62, 0.62] {
                        model.block([x, 0.0, z], [0.10, 1.36, 0.11], TIMBER);
                    }
                    model.rosette([x, 1.02, 0.0], 0.50, 0.41, GREEN);
                }
                model.block([0.0, 1.29, 0.0], [1.09, 0.11, 0.69], TIMBER);
                model.rosette([0.37, 0.0, 0.42], 0.74, 1.15, GREEN);
            }
            PickingLadder => {
                for x in [-0.44, 0.44] {
                    model.wood([x, 0.0, 0.88], [x * 0.53, 2.36, -0.03], 0.14, TIMBER);
                }
                model.wood([0.0, 0.0, -1.03], [0.0, 2.32, -0.02], 0.12, TIMBER);
                for i in 1_u8..5 {
                    let t = f32::from(i) / 5.0;
                    model.block(
                        [0.0, 2.36 * t, 0.88 - 0.91 * t],
                        [0.42 - t * 0.17, 0.10, 0.10],
                        CREAM,
                    );
                }
            }
            PickingPlatform => {
                wagon(model, [0.0, 0.0], TIMBER);
                model.block([0.0, 0.49, 0.0], [0.86, 0.15, 0.82], RED);
                for x in [-0.75, 0.75] {
                    model.wood([x, 0.57, -0.65], [x, 1.21, 0.65], 0.10, STEEL);
                    model.wood([x, 0.57, 0.65], [x, 1.21, -0.65], 0.10, STEEL);
                }
                model.block([0.0, 1.18, -0.06], [0.80, 0.10, 0.59], CREAM);
            }
            ColdStore => {
                model.block([-0.12, 0.0, 0.0], [0.92, 1.19, 0.85], CREAM);
                model.block([-0.12, 1.14, 0.0], [0.99, 0.18, 0.91], STEEL);
                model.block([0.39, 1.31, 0.21], [0.37, 0.32, 0.31], STEEL);
                model.block([0.74, 0.0, 0.89], [0.51, 0.25, 0.37], TIMBER);
                model.block([0.29, 0.18, 0.86], [0.31, 0.69, 0.07], RED);
            }
            ProduceStall => {
                for x in [-0.89, 0.89] {
                    for z in [-0.49, 0.49] {
                        model.block([x, 0.0, z], [0.10, 1.44, 0.10], TIMBER);
                    }
                }
                model.gable([0.0, 1.35, 0.0], [1.02, 0.43, 0.72], RED);
                model.block([0.0, 0.30, 0.37], [0.83, 0.31, 0.28], CREAM);
                for x in [-0.52, 0.0, 0.52] {
                    model.stone([x, 0.59, 0.34], [0.20, 0.21, 0.18], [183, 109, 53]);
                }
            }
            Weighbridge => {
                model.block([0.0, 0.0, 0.13], [0.58, 0.17, 1.36], STEEL);
                for x in [-0.64, 0.64] {
                    model.block([x, 0.0, 0.13], [0.08, 0.36, 1.26], CREAM);
                }
                model.block([1.09, 0.0, -0.35], [0.35, 0.93, 0.49], CREAM);
                model.gable([1.09, 0.88, -0.35], [0.43, 0.32, 0.55], RED);
            }
            FeedDepot => {
                model.drum([-0.64, 0.0, -0.32], 0.35, 1.81, STEEL, false);
                model.drum([0.18, 0.0, -0.32], 0.34, 1.64, STEEL, false);
                model.stone([-0.64, 1.80, -0.32], [0.36, 0.23, 0.36], CREAM);
                model.stone([0.18, 1.63, -0.32], [0.34, 0.23, 0.34], CREAM);
                model.block([0.40, 0.0, 0.49], [0.74, 0.58, 0.44], TIMBER);
                model.gable([0.40, 0.48, 0.49], [0.80, 0.39, 0.50], RED);
            }
            Windpump => {
                for [x, z] in [[-0.55, -0.48], [-0.55, 0.48], [0.55, -0.48], [0.55, 0.48]] {
                    model.wood([x, 0.0, z], [x * 0.30, 2.46, z * 0.30], 0.14, STEEL);
                }
                for [x, z] in [[-0.49, -0.44], [0.49, -0.44]] {
                    model.wood([x, 0.26, z], [-x * 0.48, 1.55, z * 0.50], 0.09, STEEL);
                }
                let hub = [0.0, 2.45, 0.22];
                for [x, y] in [
                    [0.0, 0.80],
                    [0.72, 0.39],
                    [0.72, -0.39],
                    [0.0, -0.80],
                    [-0.72, -0.39],
                    [-0.72, 0.39],
                ] {
                    model.stalk(hub, [x, 2.45 + y, 0.22], 0.15, CREAM);
                }
            }
            FieldGate => {
                for x in [-1.10, 1.10] {
                    model.block([x, 0.0, 0.0], [0.16, 1.48, 0.14], TIMBER);
                }
                for y in [0.39, 0.72, 1.02] {
                    model.block([0.0, y, 0.0], [1.07, 0.09, 0.085], CREAM);
                }
                model.wood([-1.03, 0.34, 0.02], [1.05, 1.15, 0.02], 0.11, TIMBER);
            }
        }
    }
}

fn wagon(model: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>, [x, z]: [f32; 2], color: [u8; 3]) {
    model.block([x, 0.29, z], [0.73, 0.22, 0.76], color);
    for side in [-0.68, 0.68] {
        for axle in [-0.50, 0.50] {
            model.block([x + side, 0.0, z + axle], [0.16, 0.43, 0.31], [58, 63, 55]);
        }
    }
}
