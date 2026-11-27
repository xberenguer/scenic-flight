//! Low, grounded rock and plant assemblages for dry slopes and drainage.

use super::{tint, Mesh};
use crate::world::Surface;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum Foothill {
    RoundedCobbleBar, BoulderJam, WashedJuniper, SideChannelRibbon, GravelSplay,
    DuneSedgeCrests, BleachedYucca, SandSageApron, DuneEveningPrimrose, IndianRicegrass,
    IronstoneCrest, BlockfallToe, SandstoneSteps, BrokenCaprock, ErodedRibs,
    BarrelAndScree, DesertMarigold, CreosoteAndGravel, AgaveAndTalus, BeavertailLedge,
    SlantedFins, BasaltScatter, QuartzCobbles, ShaleShingles, SandstoneFlakes,
}

impl Foothill {
    pub(crate) const GROUPS: [[Self; 5]; 5] = [
        [Self::RoundedCobbleBar, Self::BoulderJam, Self::WashedJuniper,
            Self::SideChannelRibbon, Self::GravelSplay],
        [Self::DuneSedgeCrests, Self::BleachedYucca, Self::SandSageApron,
            Self::DuneEveningPrimrose, Self::IndianRicegrass],
        [Self::IronstoneCrest, Self::BlockfallToe, Self::SandstoneSteps,
            Self::BrokenCaprock, Self::ErodedRibs],
        [Self::BarrelAndScree, Self::DesertMarigold, Self::CreosoteAndGravel,
            Self::AgaveAndTalus, Self::BeavertailLedge],
        [Self::SlantedFins, Self::BasaltScatter, Self::QuartzCobbles,
            Self::ShaleShingles, Self::SandstoneFlakes],
    ];

    pub(crate) fn supports(surface: Surface) -> bool {
        matches!(surface, Surface::Desert | Surface::DryWash | Surface::DuneSand
            | Surface::MesaCap | Surface::MesaApron)
    }

    pub(crate) fn choose(surface: Surface, grade_squared: f32, key: u64, district: u64) -> Self {
        let group = match surface {
            Surface::DryWash => 0,
            Surface::DuneSand => 1,
            Surface::MesaCap | Surface::MesaApron => 2,
            _ if grade_squared > 0.125 => 4,
            _ => 3,
        };
        let index = if key >> 35 & 3 == 0 {
            key >> 32
        } else {
            (district >> 13) + (key >> 37 & 1)
        };
        Self::GROUPS[group][(index % 5) as usize]
    }

    pub(super) fn draw(self, tone: i16, mesh: &mut Mesh<impl FnMut([[f32; 3]; 3], [u8; 3])>) {
        use Foothill::*;
        const COBBLE: [u8; 3] = [168, 151, 132];
        const SAND: [u8; 3] = [202, 157, 105];
        const RED: [u8; 3] = [176, 94, 65];
        const SHALE: [u8; 3] = [122, 102, 91];
        const BASALT: [u8; 3] = [86, 77, 75];
        const QUARTZ: [u8; 3] = [217, 195, 176];
        const LEAF: [u8; 3] = [98, 136, 92];
        const SAGE: [u8; 3] = [133, 153, 114];
        const STRAW: [u8; 3] = [173, 156, 103];
        const BARK: [u8; 3] = [123, 95, 68];
        const GOLD: [u8; 3] = [231, 177, 65];
        const CREAM: [u8; 3] = [231, 211, 179];

        let color = |material| tint(material, tone);
        match self {
            RoundedCobbleBar => {
                for [x, z, span, high] in [[-1.42, 0.17, 0.43, 0.37], [-0.67, -0.31, 0.58, 0.50],
                    [0.12, -0.42, 0.64, 0.49], [0.81, -0.04, 0.54, 0.33], [1.44, 0.33, 0.34, 0.28]] {
                    mesh.stone([x, 0.0, z], [span, high, span * 0.68], color(COBBLE));
                }
            }
            BoulderJam => {
                mesh.stone([-0.43, 0.0, -0.32], [0.81, 1.31, 0.82], color(SHALE));
                mesh.stone([0.54, 0.0, 0.13], [0.69, 0.87, 0.53], color(COBBLE));
                mesh.wood([-1.39, 0.0, 0.56], [0.91, 0.52, 0.67], 0.14, color(BARK));
            }
            WashedJuniper => {
                mesh.wood([-1.59, 0.03, 0.25], [0.58, 0.24, -0.24], 0.20, color(BARK));
                for point in [[1.35, 0.95, -0.61], [1.72, 0.55, 0.36], [0.99, 0.79, 0.78]] {
                    mesh.wood([0.37, 0.20, -0.14], point, 0.11, color(STRAW));
                }
                mesh.stone([-0.61, 0.0, 0.56], [0.42, 0.45, 0.39], color(COBBLE));
            }
            SideChannelRibbon => {
                mesh.stone([-0.55, 0.0, -0.22], [1.22, 0.34, 0.57], color(SAND));
                mesh.stone([0.53, 0.0, 0.43], [1.01, 0.25, 0.46], color(COBBLE));
                mesh.stone([-0.19, 0.09, -0.28], [1.03, 0.19, 0.21], color(QUARTZ));
            }
            GravelSplay => {
                for [x, z, span] in [[-0.96, 0.76, 0.54], [-0.34, 0.28, 0.51],
                    [0.26, -0.24, 0.36], [0.52, 0.69, 0.60], [0.84, 0.12, 0.42]] {
                    mesh.stone([x, 0.0, z], [span, span * 0.56, span * 0.84], color(tint(COBBLE, (z * 24.0) as i16)));
                }
            }
            DuneSedgeCrests => {
                for [x, z, h] in [[-1.03, 0.32, 0.67], [-0.26, -0.22, 1.12], [0.59, -0.09, 0.83], [1.20, 0.36, 0.57]] {
                    mesh.rosette([x, 0.0, z], 0.56, h, color(STRAW));
                }
            }
            BleachedYucca => {
                mesh.rosette([-0.19, 0.0, 0.12], 1.06, 0.69, color(STRAW));
                mesh.wood([-0.19, 0.0, 0.12], [0.20, 2.30, -0.18], 0.13, color(BARK));
                mesh.rosette([0.61, 0.0, -0.11], 0.48, 0.78, color(LEAF));
            }
            SandSageApron => {
                mesh.stone([-0.36, 0.0, 0.13], [1.28, 0.37, 0.63], color(SAND));
                mesh.stone([-0.68, 0.26, 0.10], [0.58, 0.46, 0.51], color(SAGE));
                mesh.stone([0.22, 0.17, -0.12], [0.71, 0.32, 0.45], color(LEAF));
            }
            DuneEveningPrimrose => {
                for [x, z, span] in [[-0.80, 0.36, 0.50], [-0.36, -0.35, 0.48], [0.45, 0.16, 0.63]] {
                    mesh.rosette([x, 0.0, z], span, span * 0.81, color(SAGE));
                    mesh.flower([x + 0.17, span * 0.66, z - 0.09], 0.28, color(CREAM));
                }
            }
            IndianRicegrass => {
                for [x, z, high] in [[-0.64, 0.16, 0.96], [0.17, -0.41, 1.23], [0.68, 0.32, 0.70]] {
                    mesh.rosette([x, 0.0, z], 0.41, high * 0.55, color(STRAW));
                    mesh.stalk([x, 0.0, z], [x + 0.31, high, z + 0.14], 0.086, color(SAGE));
                    mesh.flower([x + 0.31, high, z + 0.14], 0.17, color(CREAM));
                }
            }
            IronstoneCrest => {
                mesh.stone([0.0, 0.0, 0.08], [1.27, 0.71, 0.63], color(RED));
                mesh.stone([-0.32, 0.48, -0.04], [1.03, 0.39, 0.54], color(BASALT));
                mesh.stone([0.58, 0.58, 0.01], [0.58, 0.29, 0.46], color(SHALE));
            }
            BlockfallToe => {
                for ([x, z], width, high) in [([-0.89, -0.29], 0.70, 1.44),
                    ([0.32, 0.01], 0.56, 0.89), ([0.99, 0.46], 0.41, 0.60)] {
                    mesh.stone([x, 0.0, z], [width, high, width * 0.87], color(RED));
                }
            }
            SandstoneSteps => {
                for [x, y, z, width] in [[-0.19, 0.0, 0.35, 1.31], [0.22, 0.30, 0.04, 1.05],
                    [0.39, 0.61, -0.32, 0.77], [0.63, 0.91, -0.50, 0.51]] {
                    mesh.stone([x, y, z], [width, 0.40, 0.59], color(tint(SAND, (y * -22.0) as i16)));
                }
            }
            BrokenCaprock => {
                mesh.stone([-0.64, 0.0, -0.28], [0.86, 1.42, 0.93], color(RED));
                mesh.stone([-0.62, 0.94, -0.27], [0.95, 0.41, 1.03], color(SAND));
                mesh.stone([0.72, 0.0, 0.31], [0.72, 0.48, 0.61], color(SHALE));
            }
            ErodedRibs => {
                for [x, z, high] in [[-1.05, -0.25, 0.81], [-0.48, 0.17, 1.33],
                    [0.12, -0.06, 1.61], [0.68, 0.20, 1.07], [1.16, 0.41, 0.58]] {
                    mesh.gable([x, 0.0, z], [0.30, high, 0.73], color(SAND));
                }
            }
            BarrelAndScree => {
                mesh.stone([0.05, 0.0, -0.19], [0.48, 1.06, 0.47], color(LEAF));
                mesh.flower([0.05, 0.90, -0.19], 0.23, color(GOLD));
                mesh.stone([-0.87, 0.0, 0.48], [0.47, 0.38, 0.44], color(SHALE));
                mesh.stone([0.68, 0.0, 0.21], [0.38, 0.29, 0.36], color(SHALE));
            }
            DesertMarigold => {
                for [x, z, high] in [[-0.83, -0.21, 0.55], [-0.37, 0.36, 0.92], [0.22, -0.34, 1.04], [0.73, 0.23, 0.79]] {
                    mesh.stalk([x * 0.68, 0.0, z], [x, high, z], 0.12, color(SAGE));
                    mesh.flower([x, high, z], 0.28, color(GOLD));
                }
            }
            CreosoteAndGravel => {
                mesh.stone([-0.43, 0.0, 0.30], [0.69, 0.30, 0.46], color(COBBLE));
                mesh.wood([0.10, 0.0, -0.14], [0.41, 0.72, -0.28], 0.17, color(BARK));
                mesh.rosette([0.32, 0.50, -0.24], 0.91, 0.72, color(LEAF));
                mesh.flower([0.90, 0.84, -0.09], 0.18, color(GOLD));
            }
            AgaveAndTalus => {
                mesh.stone([-0.73, 0.0, 0.10], [0.61, 0.45, 0.45], color(SHALE));
                mesh.rosette([0.10, 0.0, 0.09], 1.09, 1.17, color(SAGE));
                mesh.rosette([0.87, 0.0, -0.53], 0.60, 0.71, color(LEAF));
            }
            BeavertailLedge => {
                mesh.stone([-0.16, 0.0, 0.39], [1.04, 0.44, 0.51], color(RED));
                for [x, z, high] in [[-0.38, -0.29, 0.65], [0.09, -0.31, 1.08], [0.55, -0.27, 0.76]] {
                    mesh.stone([x, 0.0, z], [0.30, high, 0.18], color(LEAF));
                }
                mesh.flower([0.17, 0.93, -0.28], 0.21, color([197, 108, 146]));
            }
            SlantedFins => {
                mesh.gable([-0.58, 0.0, -0.18], [0.36, 1.75, 0.99], color(SHALE));
                mesh.gable([0.49, 0.0, 0.39], [0.44, 1.11, 0.83], color(RED));
                mesh.stone([0.72, 0.0, -0.62], [0.48, 0.36, 0.42], color(SAND));
            }
            BasaltScatter => {
                for [x, z, span, high] in [[-1.07, -0.25, 0.49, 0.51], [-0.43, 0.47, 0.39, 0.92],
                    [0.24, -0.24, 0.55, 1.12], [1.03, 0.37, 0.52, 0.62]] {
                    mesh.stone([x, 0.0, z], [span, high, span * 0.71], color(BASALT));
                }
            }
            QuartzCobbles => {
                mesh.stone([0.24, 0.0, -0.12], [0.75, 0.88, 0.62], color(QUARTZ));
                mesh.stone([-0.78, 0.0, 0.39], [0.53, 0.41, 0.48], color(COBBLE));
                mesh.stone([0.89, 0.0, 0.50], [0.45, 0.30, 0.30], color(QUARTZ));
                mesh.rosette([-0.16, 0.0, 0.67], 0.33, 0.41, color(STRAW));
            }
            ShaleShingles => {
                for [x, z, high, span] in [[-0.71, -0.47, 0.44, 0.85], [0.51, -0.19, 0.63, 0.74],
                    [-0.29, 0.48, 0.31, 1.17]] {
                    mesh.stone([x, 0.0, z], [span, high, 0.28], color(tint(SHALE, (z * 18.0) as i16)));
                }
            }
            SandstoneFlakes => {
                mesh.stone([-0.36, 0.0, -0.33], [1.08, 0.44, 0.70], color(RED));
                for [x, z, span] in [[-0.87, 0.52, 0.47], [0.03, 0.50, 0.43], [0.67, 0.32, 0.52], [1.08, -0.19, 0.37]] {
                    mesh.gable([x, 0.0, z], [span, span * 1.24, 0.30], color(SAND));
                }
            }
        }
    }
}
