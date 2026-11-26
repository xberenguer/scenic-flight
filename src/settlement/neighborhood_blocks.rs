//! Street-fronting block plans for the surveyed outer neighborhoods.

use super::{Area, Building, Form, Frontage, Parcel, SettlementSample};

#[derive(Clone, Copy)]
enum Roof {
    Cottage(u8),
    Duplex(u8),
    Row(u8),
    Shop(u8),
    Industry(u8),
    Landmark(u8),
    Flat,
}

#[derive(Clone, Copy)]
struct Mass {
    center: [f32; 2],
    half_extent: [f32; 2],
    height: f32,
    roof: Roof,
}

impl Mass {
    fn build(self, frontage: Frontage, lot: Area, base: f32, variation: f32) -> Building {
        let [along, setback] = self.center;
        let [half_width, half_depth] = self.half_extent;
        let shape = frontage.lot(lot.x, lot.z, along, setback, half_width, half_depth);
        let form = match self.roof {
            Roof::Cottage(i) => Form::Cottage(i),
            Roof::Duplex(i) => Form::Duplex(i),
            Roof::Row(i) => Form::Rowhouse(frontage, i),
            Roof::Shop(i) => Form::Shop(frontage, i),
            Roof::Industry(i) => Form::Industrial(frontage, i),
            Roof::Landmark(i) => Form::TownLandmark(
                frontage,
                super::TownLandmark::from_index(i as usize).unwrap(),
            ),
            Roof::Flat => Form::Hall,
        };
        Building::new(shape, base, self.height * variation, form)
    }
}

const fn mass(center: [f32; 2], half_extent: [f32; 2], height: f32, roof: Roof) -> Mass {
    Mass {
        center,
        half_extent,
        height,
        roof,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Ground {
    Garden,
    Yard,
    None,
}

#[derive(Clone, Copy)]
pub struct NeighborhoodBlock {
    pub name: &'static str,
    primary: Mass,
    secondary: Option<Mass>,
    ground: Ground,
}

impl NeighborhoodBlock {
    pub const ALL: [Self; 25] = [
        Self::pair(
            "Offset cottage gardens",
            mass([-3.6, 1.3], [2.6, 3.6], 5.4, Roof::Cottage(2)),
            mass([3.5, -1.5], [2.8, 4.4], 4.7, Roof::Cottage(4)),
            Ground::Garden,
        ),
        Self::pair(
            "Twin hipped villas",
            mass([-3.8, 0.0], [2.5, 5.6], 6.8, Roof::Cottage(13)),
            mass([3.8, 0.0], [2.4, 5.1], 6.3, Roof::Cottage(1)),
            Ground::Garden,
        ),
        Self::pair(
            "Duplex and rear coach house",
            mass([0.0, 3.0], [6.0, 2.8], 6.3, Roof::Duplex(12)),
            mass([-2.5, -4.1], [2.8, 2.1], 4.1, Roof::Cottage(6)),
            Ground::Garden,
        ),
        Self::single(
            "Raised bungalow court",
            mass([0.0, -1.5], [5.9, 4.65], 5.0, Roof::Cottage(15)),
            Ground::Garden,
        ),
        Self::pair(
            "Saltbox and gate cottage",
            mass([-2.0, -1.9], [4.3, 4.6], 7.2, Roof::Cottage(3)),
            mass([3.5, 4.85], [2.35, 1.85], 4.3, Roof::Cottage(9)),
            Ground::Garden,
        ),
        Self::pair(
            "Corner apartment court",
            mass([0.0, -3.8], [6.8, 2.4], 13.4, Roof::Row(12)),
            mass([-4.8, 2.8], [2.0, 4.0], 10.5, Roof::Flat),
            Ground::None,
        ),
        Self::pair(
            "Parallel walk-up wings",
            mass([-4.2, 0.4], [2.2, 6.2], 10.2, Roof::Row(5)),
            mass([4.2, -0.5], [2.3, 5.8], 12.5, Roof::Row(14)),
            Ground::Garden,
        ),
        Self::pair(
            "Townhouse lane",
            mass([0.0, 4.15], [6.4, 2.45], 9.6, Roof::Row(13)),
            mass([0.0, -4.0], [6.1, 2.4], 6.1, Roof::Row(1)),
            Ground::Garden,
        ),
        Self::single(
            "Terraced apartment slab",
            mass([0.0, -1.2], [6.5, 4.2], 15.4, Roof::Landmark(16)),
            Ground::Garden,
        ),
        Self::pair(
            "Point block and long wing",
            mass([-4.1, 2.9], [2.35, 2.8], 16.8, Roof::Flat),
            mass([1.1, -3.8], [5.1, 2.25], 9.7, Roof::Row(0)),
            Ground::None,
        ),
        Self::pair(
            "Bakery and upstairs flats",
            mass([-4.3, 2.1], [2.35, 3.5], 6.0, Roof::Shop(10)),
            mass([2.7, -0.5], [4.0, 5.4], 12.0, Roof::Row(4)),
            Ground::None,
        ),
        Self::pair(
            "Corner shops and courtyard homes",
            mass([-3.0, 3.7], [3.8, 2.8], 8.1, Roof::Shop(13)),
            mass([1.5, -3.3], [4.85, 2.5], 7.9, Roof::Row(3)),
            Ground::Garden,
        ),
        Self::pair(
            "Market house and corner duplex",
            mass([1.1, -2.5], [5.5, 4.1], 10.2, Roof::Shop(7)),
            mass([-2.8, 4.4], [3.25, 2.15], 6.4, Roof::Duplex(3)),
            Ground::None,
        ),
        Self::single(
            "Art Deco corner parade",
            mass([0.0, 1.4], [6.75, 4.05], 13.8, Roof::Shop(11)),
            Ground::Yard,
        ),
        Self::pair(
            "Arcaded row and rear workshop",
            mass([0.0, 3.45], [6.5, 3.3], 10.4, Roof::Shop(9)),
            mass([1.7, -4.8], [4.0, 1.85], 5.5, Roof::Industry(4)),
            Ground::None,
        ),
        Self::single(
            "Long gabled market",
            mass([0.0, -2.2], [6.65, 4.35], 8.3, Roof::Shop(12)),
            Ground::Yard,
        ),
        Self::pair(
            "Retail court and kiosk",
            mass([1.3, -2.0], [5.2, 4.4], 7.8, Roof::Shop(3)),
            mass([-3.9, 4.05], [2.0, 1.7], 3.9, Roof::Cottage(8)),
            Ground::None,
        ),
        Self::single(
            "Office on a stepped podium",
            mass([0.2, -1.15], [6.25, 4.75], 16.7, Roof::Landmark(14)),
            Ground::Yard,
        ),
        Self::pair(
            "Corner store and service hall",
            mass([3.6, 3.5], [3.0, 3.0], 7.3, Roof::Shop(8)),
            mass([-3.5, -1.15], [2.8, 5.4], 6.3, Roof::Industry(14)),
            Ground::None,
        ),
        Self::pair(
            "Studio sheds and showrooms",
            mass([0.0, -4.2], [6.4, 2.35], 7.6, Roof::Industry(9)),
            mass([0.0, 3.9], [6.5, 2.7], 5.6, Roof::Shop(1)),
            Ground::Garden,
        ),
        Self::single(
            "North-light works and apron",
            mass([0.0, -2.5], [6.6, 4.0], 8.8, Roof::Industry(0)),
            Ground::Yard,
        ),
        Self::pair(
            "Foundry and small tool shed",
            mass([-0.9, -2.9], [5.65, 3.75], 11.7, Roof::Industry(13)),
            mass([4.7, 3.7], [1.8, 2.35], 5.8, Roof::Industry(2)),
            Ground::None,
        ),
        Self::single(
            "Barrel-vault depot",
            mass([0.0, -2.0], [5.9, 4.6], 10.5, Roof::Industry(7)),
            Ground::Yard,
        ),
        Self::pair(
            "Three-bay garage and office",
            mass([0.0, -3.7], [6.75, 3.1], 7.4, Roof::Industry(16)),
            mass([-3.5, 3.5], [2.95, 2.15], 8.0, Roof::Shop(4)),
            Ground::None,
        ),
        Self::pair(
            "Paired workshop yards",
            mass([-4.1, -0.7], [2.55, 5.95], 7.8, Roof::Industry(10)),
            mass([4.0, -0.3], [2.5, 6.2], 9.7, Roof::Industry(12)),
            Ground::Garden,
        ),
    ];

    const fn pair(name: &'static str, primary: Mass, secondary: Mass, ground: Ground) -> Self {
        Self {
            name,
            primary,
            secondary: Some(secondary),
            ground,
        }
    }

    const fn single(name: &'static str, primary: Mass, ground: Ground) -> Self {
        Self {
            name,
            primary,
            secondary: None,
            ground,
        }
    }

    pub(super) fn for_site(district: u32, key: u32, distance: f32) -> Self {
        let zone = (district >> 5) & 7;
        let family = match zone {
            0 | 1 if distance >= 56.0 => 4,
            2 | 3 if distance < 136.0 => 1,
            4 | 5 => 2,
            6 if distance < 167.0 => 3,
            _ => 0,
        };
        Self::ALL[5 * family + ((key >> 10) % 5) as usize]
    }

    pub(super) fn parcel(self, lot: Area, frontage: Frontage, base: f32, key: u32) -> Parcel {
        let scale = 0.94 + f32::from((key >> 24) as u8) * (0.12 / 255.0);
        let primary = self.primary.build(frontage, lot, base, scale);
        if let Some(mass) = self.secondary {
            let secondary = mass.build(frontage, lot, base, 2.0 - scale);
            if self.ground == Ground::Garden {
                return Parcel::with_courtyard([primary, secondary]);
            }
            let mut block = Parcel::EMPTY;
            block.add(primary);
            block.add(secondary);
            return block;
        }
        let [along, setback] = self.primary.center;
        let front = setback + self.primary.half_extent[1];
        let yard = frontage.lot(
            lot.x,
            lot.z,
            along * 0.5,
            0.5 * (front + 6.85),
            6.55,
            0.5 * (6.85 - front),
        );
        let mut block = Parcel::with_yard(primary, yard);
        if self.ground == Ground::Garden {
            block.open = Some(super::SettlementSurface::Park);
        }
        block
    }

    /// Artwork uses the same block builder and roof sampler as the world.
    pub fn study(self, across: f32, forward: f32) -> Option<SettlementSample> {
        let lot = Area {
            x: 0.0,
            z: 0.0,
            half_width: 7.1,
            half_depth: 7.1,
        };
        self.parcel(lot, Frontage::Far, 0.0, 0x8000_0000)
            .sample(across, forward)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_plans_have_unique_roofs_inside_their_surveyed_lot() {
        let lot = Area {
            x: 0.0,
            z: 0.0,
            half_width: 7.1,
            half_depth: 7.1,
        };
        let mut seen = [0_u64; NeighborhoodBlock::ALL.len()];
        for (i, plan) in NeighborhoodBlock::ALL.into_iter().enumerate() {
            for facing in [
                Frontage::Near,
                Frontage::Far,
                Frontage::Left,
                Frontage::Right,
            ] {
                let parcel = plan.parcel(lot, facing, 0.0, 0);
                for building in parcel.buildings() {
                    assert!(
                        building.area.x.abs() + building.area.half_width <= 7.1,
                        "{}",
                        plan.name
                    );
                    assert!(
                        building.area.z.abs() + building.area.half_depth <= 7.1,
                        "{}",
                        plan.name
                    );
                }
                if let Some((open, _)) = parcel.open() {
                    assert!(
                        open.half_width > 0.0 && open.half_depth > 0.0,
                        "{}",
                        plan.name
                    );
                    assert!(
                        !parcel
                            .buildings()
                            .iter()
                            .any(|building| { building.area.contains(open.x, open.z) }),
                        "{}",
                        plan.name
                    );
                }
            }
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            let mut roof_count = 0;
            for row in -14..=14 {
                for col in -14..=14 {
                    let height = plan
                        .study(col as f32 * 0.5, row as f32 * 0.5)
                        .and_then(|sample| sample.elevation)
                        .unwrap_or(0.0);
                    roof_count += usize::from(height > 0.0);
                    hash = (hash ^ u64::from(height.to_bits())).wrapping_mul(0x100_0000_01b3);
                }
            }
            assert!(roof_count > 88, "{}", plan.name);
            assert!(!seen[..i].contains(&hash), "{}", plan.name);
            seen[i] = hash;
        }
    }
}
