//! Roofs for small buildings reached from a surveyed waterfront walk.

use super::SettlementSurface;

#[derive(Clone, Copy)]
enum Roof {
    Flat,
    Gable,
    CrossGable,
    Hip,
    Shed,
    Barrel,
    Glass,
}

#[derive(Clone, Copy)]
struct Tier {
    center: [f32; 2],
    half: [f32; 2],
    height: f32,
    roof: Roof,
}

impl Tier {
    const EMPTY: Self = tier([0.0, 0.0], [0.0, 0.0], 0.0, Roof::Flat);

    fn sample(self, [u, v]: [f32; 2]) -> Option<(SettlementSurface, f32)> {
        if self.height == 0.0 {
            return None;
        }
        let x = (u - self.center[0]) / self.half[0];
        let z = (v - self.center[1]) / self.half[1];
        let [a, b] = [x.abs(), z.abs()];
        if a > 1.0 || b > 1.0 {
            return None;
        }
        use SettlementSurface::{FlatRoof, IndustryMetal, PitchedRoof, TowerRoof};
        let (surface, shape) = match self.roof {
            Roof::Flat => (FlatRoof, if a.max(b) < 0.83 { 0.90 } else { 1.0 }),
            Roof::Gable => (PitchedRoof, 1.0 - 0.30 * a),
            Roof::CrossGable => (PitchedRoof, 1.0 - 0.32 * b),
            Roof::Hip => (PitchedRoof, 1.0 - 0.32 * a.max(b)),
            Roof::Shed => (IndustryMetal, 0.83 + x * 0.17),
            Roof::Barrel => (IndustryMetal, 1.0 - 0.33 * a * a),
            Roof::Glass => (TowerRoof, 1.0 - 0.23 * b),
        };
        Some((surface, self.height * shape))
    }
}

const fn tier(center: [f32; 2], half: [f32; 2], height: f32, roof: Roof) -> Tier {
    Tier {
        center,
        half,
        height,
        roof,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum District {
    Boating,
    Park,
    Lookout,
    Market,
    Service,
}

#[derive(Clone, Copy)]
pub struct RiverfrontBuilding {
    pub name: &'static str,
    pub size: [f32; 2],
    pub height: f32,
    pub(crate) district: District,
    tiers: [Tier; 3],
}

use District::*;
use Roof::*;

impl RiverfrontBuilding {
    pub const ALL: [Self; 25] = [
        Self::new(
            "Cross-gabled boathouse",
            Boating,
            [4.2, 3.1],
            5.4,
            [
                tier([-0.24, 0.04], [0.70, 0.91], 0.99, Gable),
                tier([0.61, -0.39], [0.36, 0.52], 0.71, CrossGable),
                Tier::EMPTY,
            ],
        ),
        Self::new(
            "Twin barrel boat sheds",
            Boating,
            [4.4, 3.2],
            5.5,
            [
                tier([-0.52, 0.02], [0.44, 0.93], 1.0, Barrel),
                tier([0.52, 0.13], [0.44, 0.82], 0.81, Barrel),
                tier([0.0, 0.31], [0.24, 0.35], 0.49, Flat),
            ],
        ),
        Self::new(
            "Slipway shelter",
            Boating,
            [4.1, 3.4],
            5.1,
            [
                tier([0.0, 0.34], [0.96, 0.61], 1.0, Shed),
                tier([-0.72, -0.28], [0.24, 0.65], 0.67, Flat),
                tier([0.72, -0.23], [0.24, 0.52], 0.55, Flat),
            ],
        ),
        Self::new(
            "Canoe hire hut and racks",
            Boating,
            [3.8, 3.2],
            4.7,
            [
                tier([-0.39, 0.01], [0.52, 0.84], 1.0, Hip),
                tier([0.56, -0.40], [0.33, 0.26], 0.52, Shed),
                tier([0.56, 0.42], [0.33, 0.26], 0.64, Shed),
            ],
        ),
        Self::new(
            "Sailing clubhouse with verandah",
            Boating,
            [4.6, 3.4],
            6.6,
            [
                tier([0.12, 0.19], [0.75, 0.74], 1.0, Hip),
                tier([-0.58, 0.29], [0.40, 0.46], 0.70, Gable),
                tier([0.32, -0.56], [0.64, 0.38], 0.60, Shed),
            ],
        ),
        Self::new(
            "Twin picnic canopies",
            Park,
            [4.3, 3.0],
            4.2,
            [
                tier([-0.51, 0.22], [0.42, 0.70], 1.0, Hip),
                tier([0.53, -0.23], [0.41, 0.66], 0.87, Hip),
                Tier::EMPTY,
            ],
        ),
        Self::new(
            "Long riverside pergola",
            Park,
            [4.7, 3.0],
            4.4,
            [
                tier([-0.68, 0.25], [0.26, 0.59], 0.88, Flat),
                tier([0.0, 0.25], [0.23, 0.68], 1.0, Glass),
                tier([0.68, 0.25], [0.26, 0.59], 0.88, Flat),
            ],
        ),
        Self::new(
            "Garden conservatory",
            Park,
            [3.9, 3.3],
            5.8,
            [
                tier([0.0, 0.12], [0.37, 0.84], 1.0, Glass),
                tier([-0.53, 0.26], [0.35, 0.52], 0.67, Glass),
                tier([0.54, -0.16], [0.34, 0.50], 0.73, Glass),
            ],
        ),
        Self::new(
            "Bandstand and dressing room",
            Park,
            [4.1, 3.4],
            5.8,
            [
                tier([0.18, -0.13], [0.74, 0.63], 1.0, Hip),
                tier([-0.62, 0.50], [0.29, 0.46], 0.60, Shed),
                tier([0.35, 0.48], [0.52, 0.43], 0.53, Flat),
            ],
        ),
        Self::new(
            "Arcaded park shelter",
            Park,
            [4.6, 3.3],
            4.7,
            [
                tier([0.0, 0.46], [0.92, 0.43], 1.0, CrossGable),
                tier([-0.65, -0.22], [0.29, 0.64], 0.85, Gable),
                tier([0.65, -0.22], [0.29, 0.64], 0.85, Gable),
            ],
        ),
        Self::new(
            "Corner observation tower",
            Lookout,
            [3.2, 3.0],
            8.8,
            [
                tier([-0.48, 0.44], [0.40, 0.40], 1.0, Hip),
                tier([0.16, 0.08], [0.75, 0.75], 0.55, Flat),
                Tier::EMPTY,
            ],
        ),
        Self::new(
            "Twin stair belvedere",
            Lookout,
            [3.8, 3.2],
            8.1,
            [
                tier([-0.61, 0.04], [0.31, 0.63], 1.0, Glass),
                tier([0.61, 0.04], [0.31, 0.63], 0.88, Glass),
                tier([0.0, 0.16], [0.40, 0.52], 0.60, Flat),
            ],
        ),
        Self::new(
            "Terraced river overlook",
            Lookout,
            [4.6, 3.3],
            7.2,
            [
                tier([0.0, 0.08], [0.96, 0.86], 0.53, Flat),
                tier([0.06, 0.28], [0.67, 0.60], 0.76, Flat),
                tier([0.44, 0.47], [0.40, 0.39], 1.0, Glass),
            ],
        ),
        Self::new(
            "Covered bird hide",
            Lookout,
            [3.9, 2.9],
            5.5,
            [
                tier([0.0, 0.22], [0.94, 0.56], 0.96, Shed),
                tier([-0.57, -0.38], [0.34, 0.54], 0.52, Flat),
                Tier::EMPTY,
            ],
        ),
        Self::new(
            "Water-level observation station",
            Lookout,
            [3.2, 3.0],
            7.5,
            [
                tier([0.12, 0.30], [0.40, 0.42], 1.0, Flat),
                tier([-0.42, -0.14], [0.52, 0.65], 0.62, Gable),
                tier([0.46, -0.38], [0.33, 0.44], 0.39, Flat),
            ],
        ),
        Self::new(
            "Fish market with covered stalls",
            Market,
            [4.8, 3.4],
            6.1,
            [
                tier([-0.07, 0.29], [0.86, 0.61], 1.0, CrossGable),
                tier([-0.57, -0.53], [0.33, 0.37], 0.58, Shed),
                tier([0.43, -0.53], [0.52, 0.39], 0.63, Glass),
            ],
        ),
        Self::new(
            "Two quayside kiosks",
            Market,
            [4.0, 3.0],
            4.8,
            [
                tier([-0.55, 0.18], [0.40, 0.66], 0.96, Gable),
                tier([0.54, 0.18], [0.36, 0.62], 1.0, Hip),
                Tier::EMPTY,
            ],
        ),
        Self::new(
            "Corner cafe and terrace",
            Market,
            [4.3, 3.3],
            5.8,
            [
                tier([-0.24, 0.23], [0.70, 0.71], 1.0, Barrel),
                tier([0.57, -0.48], [0.38, 0.43], 0.56, Flat),
                tier([-0.29, -0.51], [0.48, 0.43], 0.64, Glass),
            ],
        ),
        Self::new(
            "Glazed market arcade",
            Market,
            [4.8, 3.3],
            6.6,
            [
                tier([-0.69, 0.18], [0.27, 0.77], 0.83, Gable),
                tier([0.0, 0.11], [0.45, 0.83], 1.0, Glass),
                tier([0.68, 0.18], [0.27, 0.77], 0.83, Gable),
            ],
        ),
        Self::new(
            "Ferry ticket hall and canopy",
            Market,
            [4.4, 3.4],
            6.0,
            [
                tier([0.23, 0.31], [0.73, 0.58], 1.0, Flat),
                tier([0.01, -0.38], [0.91, 0.56], 0.64, Glass),
                tier([-0.59, 0.38], [0.35, 0.51], 0.81, Shed),
            ],
        ),
        Self::new(
            "Rescue station and equipment bay",
            Service,
            [4.5, 3.1],
            6.9,
            [
                tier([-0.34, 0.05], [0.61, 0.86], 0.79, Barrel),
                tier([0.61, 0.39], [0.33, 0.53], 1.0, Flat),
                tier([0.60, -0.43], [0.34, 0.43], 0.50, Shed),
            ],
        ),
        Self::new(
            "Pump house and valve annex",
            Service,
            [4.1, 3.2],
            7.8,
            [
                tier([-0.24, 0.30], [0.66, 0.64], 0.83, CrossGable),
                tier([0.59, 0.50], [0.32, 0.40], 1.0, Flat),
                tier([0.14, -0.49], [0.55, 0.43], 0.57, Barrel),
            ],
        ),
        Self::new(
            "North-light repair yard",
            Service,
            [4.7, 3.3],
            6.2,
            [
                tier([-0.61, 0.20], [0.35, 0.74], 0.79, Shed),
                tier([0.02, 0.19], [0.35, 0.75], 0.91, Shed),
                tier([0.61, 0.17], [0.35, 0.72], 1.0, Shed),
            ],
        ),
        Self::new(
            "River freight weigh house",
            Service,
            [4.4, 3.2],
            7.0,
            [
                tier([0.39, 0.04], [0.55, 0.87], 0.72, Barrel),
                tier([-0.60, 0.53], [0.34, 0.37], 1.0, Hip),
                tier([-0.50, -0.34], [0.43, 0.51], 0.41, Flat),
            ],
        ),
        Self::new(
            "Triple-gable lock maintenance depot",
            Service,
            [4.8, 3.3],
            7.3,
            [
                tier([-0.60, 0.20], [0.36, 0.72], 0.79, Gable),
                tier([0.0, 0.26], [0.36, 0.68], 1.0, Gable),
                tier([0.61, -0.07], [0.35, 0.79], 0.86, Gable),
            ],
        ),
    ];

    const fn new(
        name: &'static str,
        district: District,
        size: [f32; 2],
        height: f32,
        tiers: [Tier; 3],
    ) -> Self {
        Self {
            name,
            size,
            height,
            district,
            tiers,
        }
    }

    pub fn roof(self, point: [f32; 2]) -> Option<(SettlementSurface, f32)> {
        self.tiers
            .into_iter()
            .filter_map(|tier| tier.sample(point))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(surface, height)| (surface, height * self.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roof_catalog_has_distinct_silhouettes_within_the_validated_envelope() {
        let mut signatures = [0_u64; RiverfrontBuilding::ALL.len()];
        for (i, building) in RiverfrontBuilding::ALL.into_iter().enumerate() {
            for tier in building.tiers {
                for axis in 0..2 {
                    assert!(
                        tier.center[axis].abs() + tier.half[axis] <= 1.0,
                        "{}",
                        building.name
                    );
                }
            }
            let mut signature = 0xcbf2_9ce4_8422_2325_u64;
            let mut covered = 0;
            for v in -13..=13 {
                for u in -13..=13 {
                    let point = [u as f32 * 0.08, v as f32 * 0.08];
                    let height = building.roof(point).map_or(0.0, |(_, height)| height);
                    assert!((0.0..=building.height).contains(&height));
                    covered += usize::from(height > 0.0);
                    signature =
                        (signature ^ u64::from(height.to_bits())).wrapping_mul(0x100_0000_01b3);
                }
            }
            assert!(covered > 125, "{}", building.name);
            assert!(!signatures[..i].contains(&signature), "{}", building.name);
            signatures[i] = signature;
        }
    }
}
