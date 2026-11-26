//! Compact relief models for the dry furnishing strips at surveyed turnouts.

use super::TurnoutKind;
use crate::world::Surface;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(super) enum RoadsideFixture {
    Viewfinder,
    ReliefMap,
    ViewingDeck,
    StoneMonument,
    LookoutSteps,
    PanoramaCanopy,
    Sundial,
    WindScreen,
    CairnPair,
    OrientationWheel,
    TrailMap,
    GearRack,
    CycleShelter,
    BootBench,
    TrailArch,
    CallBox,
    BottleFountain,
    PicnicLeanTo,
    RouteRegister,
    FireCache,
    FuelCanopy,
    PumpHouse,
    RoadsideCarport,
    ChargingBank,
    RouteKiosk,
    ProduceMarket,
    PicnicPavilion,
    VendingShelter,
    BusShelter,
    RoadGate,
}

use RoadsideFixture::*;

const OVERLOOK: [RoadsideFixture; 10] = [
    Viewfinder,
    ReliefMap,
    ViewingDeck,
    StoneMonument,
    LookoutSteps,
    PanoramaCanopy,
    Sundial,
    WindScreen,
    CairnPair,
    OrientationWheel,
];
const TRAIL: [RoadsideFixture; 10] = [
    TrailMap,
    GearRack,
    CycleShelter,
    BootBench,
    TrailArch,
    CallBox,
    BottleFountain,
    PicnicLeanTo,
    RouteRegister,
    FireCache,
];
const REST: [RoadsideFixture; 10] = [
    FuelCanopy,
    PumpHouse,
    RoadsideCarport,
    ChargingBank,
    RouteKiosk,
    ProduceMarket,
    PicnicPavilion,
    VendingShelter,
    BusShelter,
    RoadGate,
];

impl RoadsideFixture {
    pub(super) fn select(kind: TurnoutKind, variant: u8) -> Self {
        let forms = match kind {
            TurnoutKind::Overlook => OVERLOOK,
            TurnoutKind::Trailhead => TRAIL,
            TurnoutKind::RestStop => REST,
        };
        forms[usize::from(variant / 25)]
    }

    pub(super) fn profile(self, [u, v]: [f32; 2]) -> Option<(f32, Surface)> {
        let [x, z] = [u.abs(), v.abs()];
        use Surface::{
            Boardwalk as Wood, CargoBlue as Blue, CargoOchre as Amber, FlatRoof as Metal,
            PitchedRoof as Tile, Rock as Stone, ScreeCoarse as Flint,
        };
        Some(match self {
            Viewfinder if (u + 0.43).abs() < 0.23 && z < 0.43 => (1.69, Blue),
            Viewfinder if (u - 0.56).abs() < 0.19 && z < 0.34 => (1.30, Metal),
            Viewfinder if x < 0.88 && z < 0.63 => (0.26, Stone),
            ReliefMap if x < 0.63 && z < 0.68 => {
                let land = ((u + 0.8) * 3.0) as u8;
                (0.65 + (v + 1.0) * 0.28 + f32::from(land % 3) * 0.09, Wood)
            }
            ReliefMap if x < 0.84 && z < 0.43 => (0.57, Stone),
            ViewingDeck if x < 0.90 && z < 0.82 && (v > 0.60 || x > 0.76) => (0.89, Wood),
            ViewingDeck if x < 0.90 && z < 0.84 => (0.43, Wood),
            StoneMonument if (u + 0.31).abs() < 0.29 && z < 0.43 => (1.86 - z * 0.35, Stone),
            StoneMonument if x < 0.74 && z < 0.70 => (0.41, Flint),
            LookoutSteps if x < 0.80 && z < 0.84 => {
                (0.18 + f32::from(((v + 1.0) * 2.7) as u8) * 0.18, Wood)
            }
            PanoramaCanopy if x < 0.94 && z < 0.71 - u * 0.14 => {
                (1.38 + (u + v + 1.7) * 0.24, Metal)
            }
            Sundial if (u + 0.07).abs() + z * 0.85 < 0.71 => {
                let face = if u > -0.09 && u < 0.22 && z < 0.35 {
                    0.68 - z * 0.52
                } else {
                    0.0
                };
                (0.34 + face, Stone)
            }
            WindScreen if x < 0.91 && v > 0.37 && v < 0.78 => (1.39 - u * 0.29, Blue),
            WindScreen if x > 0.74 && x < 0.90 && z < 0.54 => (0.86, Metal),
            CairnPair if (u + 0.51).abs() + z * 0.42 < 0.37 => {
                (1.27 - z * 0.62 - (u + 0.51).abs(), Flint)
            }
            CairnPair if (u - 0.50).abs() + z * 0.51 < 0.44 => {
                (0.88 - z * 0.43 - (u - 0.50).abs(), Stone)
            }
            OrientationWheel if u * u + v * v * 0.55 < 0.49 => {
                let radial = u * u + v * v * 0.55;
                (
                    if radial < 0.12 {
                        0.55
                    } else if radial > 0.33 {
                        0.45
                    } else {
                        0.30
                    },
                    Amber,
                )
            }

            TrailMap if x < 0.74 && (v - 0.22).abs() < 0.33 => (1.68 + (1.0 - x) * 0.23, Wood),
            TrailMap if (u + 0.62).abs() < 0.15 && z < 0.71 => (0.40, Stone),
            GearRack if x < 0.93 && z < 0.23 => (1.31, Wood),
            GearRack if (x - 0.66).abs() < 0.22 && z < 0.61 => (0.67, Amber),
            CycleShelter if x < 0.94 && z < 0.70 => (1.45 + 0.41 * (1.0 - v * v), Blue),
            BootBench if x < 0.68 && z < 0.45 => (0.59, Wood),
            BootBench if (u - 0.72).abs() < 0.24 && z < 0.55 => (0.33, Metal),
            TrailArch if (x - 0.67).abs() < 0.19 && z < 0.48 => (2.54, Wood),
            TrailArch if x < 0.81 && (v - 0.45).abs() < 0.22 => (2.74 - 0.24 * x, Wood),
            CallBox if (u + 0.34).abs() < 0.25 && z < 0.66 => (2.29, Amber),
            CallBox if x < 0.80 && z < 0.68 => (0.31, Metal),
            BottleFountain if (u + 0.21).abs() < 0.23 && z < 0.41 => (1.52, Metal),
            BottleFountain if (u - 0.24).abs() < 0.29 && z < 0.56 => (0.79, Stone),
            PicnicLeanTo if x < 0.88 && z < 0.83 => (1.26 + 0.53 * (v + 0.83) / 1.66, Tile),
            RouteRegister if (u - 0.17).abs() < 0.44 && z < 0.57 => (1.13 + 0.42 * (1.0 - z), Tile),
            RouteRegister if u < -0.39 && z < 0.38 => (0.90, Wood),
            FireCache if x < 0.66 && z < 0.62 => (1.09 + 0.66 * (1.0 - z), Amber),
            FireCache if (u - 0.72).abs() < 0.23 && z < 0.71 => (0.40, Stone),

            FuelCanopy if x < 0.91 && z < 0.76 => (1.91 + 0.17 * (1.0 - z), Amber),
            PumpHouse if (u + 0.14).abs() < 0.76 && z < 0.67 => (1.12 + 0.73 * (1.0 - z), Tile),
            PumpHouse if u > 0.59 && u < 0.96 && z < 0.42 => (1.26, Metal),
            RoadsideCarport if x < 0.94 && z < 0.70 => (1.12 + 0.78 * (1.0 - v * v), Metal),
            ChargingBank if z < 0.59 && x < 0.91 => {
                let lane = ((u + 1.0) * 2.0) % 1.0;
                (
                    if (0.24..0.61).contains(&lane) {
                        1.46
                    } else {
                        0.24
                    },
                    Blue,
                )
            }
            RouteKiosk if (u + 0.30).abs() < 0.38 && z < 0.67 => (2.04 + 0.24 * v, Blue),
            RouteKiosk if x < 0.91 && (v + 0.63).abs() < 0.17 => (1.47, Amber),
            ProduceMarket if u < 0.69 && u > -0.96 && z < 0.68 => (
                1.26 + 0.66 * (1.0 - z),
                if u < -0.11 { Tile } else { Amber },
            ),
            ProduceMarket if u > 0.63 && z < 0.57 => (0.58, Wood),
            PicnicPavilion if x + z < 1.39 && x < 0.92 && z < 0.83 => {
                (1.28 + 0.55 * (1.0 - x.max(z)), Tile)
            }
            VendingShelter if x < 0.84 && (v - 0.14).abs() < 0.63 => {
                (1.50 + 0.59 * (1.0 - x), Metal)
            }
            VendingShelter if x < 0.67 && v < -0.39 && v > -0.72 => (0.75, Blue),
            BusShelter if x < 0.94 && (v - 0.21).abs() < 0.43 => (1.67 + 0.32 * v, Blue),
            BusShelter if x < 0.66 && v < -0.12 && v > -0.57 => (0.48, Wood),
            RoadGate if z < 0.56 && (x - 0.74).abs() < 0.11 => (1.65, Stone),
            RoadGate if x < 0.87 && z < 0.27 => (1.01, Amber),
            _ => return None,
        })
    }
}
