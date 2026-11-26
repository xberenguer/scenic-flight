//! Roof and equipment envelopes inside already reserved harbor parcels.

use super::PortKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(super) enum HarborForm {
    SailLoft,
    YachtClub,
    Boatworks,
    MarinaOffice,
    Chandlery,
    GlazedConcourse,
    CustomsHall,
    TransferStation,
    TicketWings,
    FerryGallery,
    SawtoothWarehouse,
    RefrigeratedHall,
    BondedDepot,
    TransferHangar,
    MillSupplyDepot,
    DinghyRack,
    BoatLift,
    FuelIsland,
    MastCradle,
    FloatingDockStaging,
    RampWinch,
    BaggageCartShed,
    ShuttleCanopy,
    FerryInfoPylon,
    ServiceStairs,
    WeighStation,
    PipeRacks,
    TransloadHopper,
    PumpCompound,
    ForkliftShelter,
}

use HarborForm::*;

const MARINA: [HarborForm; 5] = [SailLoft, YachtClub, Boatworks, MarinaOffice, Chandlery];
const FERRY: [HarborForm; 5] = [
    GlazedConcourse,
    CustomsHall,
    TransferStation,
    TicketWings,
    FerryGallery,
];
const FREIGHT: [HarborForm; 5] = [
    SawtoothWarehouse,
    RefrigeratedHall,
    BondedDepot,
    TransferHangar,
    MillSupplyDepot,
];
const MARINA_YARD: [HarborForm; 5] = [
    DinghyRack,
    BoatLift,
    FuelIsland,
    MastCradle,
    FloatingDockStaging,
];
const FERRY_YARD: [HarborForm; 5] = [
    RampWinch,
    BaggageCartShed,
    ShuttleCanopy,
    FerryInfoPylon,
    ServiceStairs,
];
const FREIGHT_YARD: [HarborForm; 5] = [
    WeighStation,
    PipeRacks,
    TransloadHopper,
    PumpCompound,
    ForkliftShelter,
];

impl HarborForm {
    pub(super) fn building(kind: PortKind, key: u64) -> Self {
        let group = match kind {
            PortKind::Marina => MARINA,
            PortKind::Ferry => FERRY,
            PortKind::Freight => FREIGHT,
        };
        group[((key >> 42) % 5) as usize]
    }

    pub(super) fn equipment(kind: PortKind, key: u64) -> Self {
        let group = match kind {
            PortKind::Marina => MARINA_YARD,
            PortKind::Ferry => FERRY_YARD,
            PortKind::Freight => FREIGHT_YARD,
        };
        group[((key >> 23) % 5) as usize]
    }

    pub(super) fn height(self) -> f32 {
        match self {
            MarinaOffice => 3.9,
            SailLoft | YachtClub | Boatworks | Chandlery => 3.2,
            GlazedConcourse | CustomsHall | FerryGallery => 5.4,
            TransferStation | TicketWings => 4.5,
            SawtoothWarehouse | BondedDepot | TransferHangar => 6.6,
            RefrigeratedHall | MillSupplyDepot => 5.9,
            BoatLift | FerryInfoPylon | TransloadHopper => 4.3,
            DinghyRack | BaggageCartShed | ShuttleCanopy | ForkliftShelter => 2.5,
            FuelIsland | MastCradle | FloatingDockStaging | RampWinch | PipeRacks => 1.8,
            ServiceStairs | WeighStation | PumpCompound => 2.9,
        }
    }

    pub(super) fn roof(self, [u, v]: [f32; 2]) -> f32 {
        let [x, z] = [u.abs(), v.abs()];
        let ridge = |position: f32| (1.0 - position.abs()).max(0.0);
        let bay = |along: f32, divisions: f32| {
            let measure = ((along + 1.0) * 0.5 * divisions).clamp(0.0, divisions - 0.001);
            let index = measure as u8;
            (index, measure - f32::from(index))
        };
        match self {
            SailLoft if u > 0.66 && v > 0.29 => 0.43,
            SailLoft => 0.54 + 0.46 * ridge(v),
            YachtClub if x < 0.54 && v < 0.35 => 0.83 + 0.17 * ridge(u / 0.55),
            YachtClub if v > 0.45 => 0.31 + 0.09 * ridge(u),
            YachtClub => 0.65,
            Boatworks if u < -0.53 => 0.49,
            Boatworks => 0.69 + 0.31 * (1.0 - v * v),
            MarinaOffice if (u + 0.31).abs() < 0.28 && (v + 0.34).abs() < 0.31 => 1.0,
            MarinaOffice if v > 0.54 => 0.38,
            MarinaOffice => 0.65 + 0.12 * ridge(v),
            Chandlery if u < -0.20 && v < -0.44 => 0.47,
            Chandlery => {
                let (unit, fraction) = bay(u, 2.0);
                [1.0, 0.82][usize::from(unit)] * (0.57 + 0.43 * ridge(fraction * 2.0 - 1.0))
            }

            GlazedConcourse if x < 0.72 && z < 0.64 => {
                0.72 + 0.28 * (1.0 - (v / 0.64) * (v / 0.64))
            }
            GlazedConcourse if v > 0.47 => 0.42,
            GlazedConcourse => 0.64,
            CustomsHall if u < -0.56 && v < 0.30 => 1.0,
            CustomsHall if x > 0.74 && v > 0.47 => 0.38,
            CustomsHall => 0.69 + 0.13 * ridge(v),
            TransferStation if v < -0.15 => {
                let (_, fraction) = bay(u, 3.0);
                0.66 + 0.34 * ridge(fraction * 2.0 - 1.0)
            }
            TransferStation => 0.47 + 0.09 * ridge(v),
            TicketWings if x < 0.16 => 0.37,
            TicketWings if u < 0.0 => 0.70 + 0.30 * ridge((v - 0.21) / 0.83),
            TicketWings => 0.60 + 0.24 * ridge((v + 0.29) / 0.71),
            FerryGallery if v > 0.63 => 0.39,
            FerryGallery if x < 0.67 && v > -0.55 => 0.87 + 0.13 * ridge(u / 0.67),
            FerryGallery => 0.59,

            SawtoothWarehouse => {
                let (_, phase) = bay(u, 4.0);
                let split = if phase < 0.76 {
                    phase / 0.76
                } else {
                    (1.0 - phase) / 0.24
                };
                if z > 0.88 {
                    0.55
                } else {
                    0.62 + 0.38 * split
                }
            }
            RefrigeratedHall if v > 0.67 => 0.57,
            RefrigeratedHall if (u + 0.62).abs() < 0.12 && v < -0.34 => 1.0,
            RefrigeratedHall if (u - 0.42).abs() < 0.16 && v < -0.34 => 0.91,
            RefrigeratedHall => 0.75,
            BondedDepot if (u + 0.74).abs() < 0.24 && v > -0.25 => 1.0,
            BondedDepot if u > 0.37 && v > 0.22 => 0.52,
            BondedDepot => 0.70 + 0.13 * ridge(v),
            TransferHangar if u > 0.70 => 0.57,
            TransferHangar => 0.69 + 0.31 * (1.0 - v * v),
            MillSupplyDepot if v > 0.56 => 0.46,
            MillSupplyDepot if u < -0.20 => 0.62 + 0.36 * ridge(v),
            MillSupplyDepot => 0.64 + 0.17 * ridge((u - 0.34) / 0.66),

            DinghyRack if z < 0.71 => {
                let (bay, shift) = bay(u, 3.0);
                if !(0.19..0.81).contains(&shift) {
                    return 0.0;
                }
                [0.59, 1.0, 0.79][usize::from(bay)] - z * 0.18
            }
            BoatLift if x > 0.69 && z < 0.82 => 0.78,
            BoatLift if z < 0.22 && x < 0.91 => 1.0,
            FuelIsland if z < 0.57 && x > 0.59 => 0.70,
            FuelIsland if z < 0.68 && x < 0.31 => 1.0,
            FuelIsland if z < 0.74 => 0.27,
            MastCradle if z < 0.49 && x < 0.98 => 0.4 + 0.35 * ridge(v * 1.65),
            MastCradle if z < 0.79 && (x - 0.69).abs() < 0.13 => 0.64,
            FloatingDockStaging if v > 0.10 && x < 0.93 => 0.32,
            FloatingDockStaging if v < -0.18 && u < 0.69 => 0.68,
            FloatingDockStaging if (u + 0.31).abs() < 0.31 => 1.0,

            RampWinch if (u + 0.48).abs() < 0.34 && z < 0.56 => 1.0,
            RampWinch if x < 0.89 && z < 0.63 => 0.40,
            BaggageCartShed if z < 0.74 => 0.66 + 0.34 * ridge(v / 0.76),
            BaggageCartShed if u < -0.32 && v < -0.74 => 0.47,
            ShuttleCanopy if x < 0.91 && z < 0.62 => 0.74 + 0.26 * (v + 0.62) / 1.24,
            ShuttleCanopy if z < 0.87 && (x - 0.72).abs() < 0.13 => 0.39,
            FerryInfoPylon if x < 0.29 && z < 0.40 => 1.0,
            FerryInfoPylon if v < -0.12 && x < 0.68 => 0.71,
            FerryInfoPylon if x < 0.89 && z < 0.78 => 0.22,
            ServiceStairs if x < 0.74 && z < 0.93 => 0.23 + ((v + 1.0) * 2.5) as u8 as f32 * 0.16,

            WeighStation if u < -0.25 && z < 0.66 => 0.82 + 0.18 * ridge(v / 0.66),
            WeighStation if x < 0.94 && z < 0.82 => 0.12,
            PipeRacks if z < 0.79 && x < 0.91 => {
                let (_, fraction) = bay(v, 3.0);
                0.39 + 0.53 * ridge(fraction * 2.0 - 1.0)
            }
            TransloadHopper if x < 0.65 && z < 0.56 => 1.0,
            TransloadHopper if x < 0.89 && z < 0.70 => 0.26,
            PumpCompound if (u + 0.31) * (u + 0.31) + v * v * 0.77 < 0.33 => 0.82,
            PumpCompound if (u - 0.59).abs() < 0.23 && z < 0.76 => 1.0,
            PumpCompound if x < 0.92 && z < 0.77 => 0.22,
            ForkliftShelter if x < 0.90 && z < 0.66 => 0.65 + 0.35 * ridge(v / 0.68),
            _ => 0.0,
        }
    }
}
