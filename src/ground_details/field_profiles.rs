//! Surveyed farmland patterns drawn in blocks large enough to read from the air.

use crate::world::Surface;

#[derive(Clone, Copy)]
pub struct FieldProfile {
    pub name: &'static str,
    rows: [u16; 6],
}

impl FieldProfile {
    pub const ALL: [Self; 25] = [
        Self {
            name: "Twin meadows",
            rows: [0xa15, 0xa15, 0xa15, 0x0, 0x53f, 0x53f],
        },
        Self {
            name: "Four paddocks",
            rows: [0xa85, 0xa85, 0x0, 0x53f, 0x53f, 0x53f],
        },
        Self {
            name: "Staggered fields",
            rows: [0x15, 0xa15, 0xa15, 0xac0, 0xfc, 0x5fc],
        },
        Self {
            name: "Pocket pasture",
            rows: [0xa94, 0xa95, 0x805, 0x805, 0xaf5, 0xaf4],
        },
        Self {
            name: "L-shaped holding",
            rows: [0x55, 0x55, 0x55, 0x5, 0xa5, 0xaa5],
        },
        Self {
            name: "Farm lane",
            rows: [0x54a, 0x54a, 0x54f, 0x54f, 0xa8f, 0xa8f],
        },
        Self {
            name: "Long hay strips",
            rows: [0x5aa, 0x5aa, 0x0, 0xf55, 0xf55, 0xfaa],
        },
        Self {
            name: "Short rotation strips",
            rows: [0x555, 0xfff, 0xaaa, 0x0, 0xa95, 0xa95],
        },
        Self {
            name: "Offset wheat belts",
            rows: [0x16a, 0x16a, 0x0, 0xabc, 0xabc, 0xabc],
        },
        Self {
            name: "Market garden beds",
            rows: [0x7e4, 0x7e5, 0x7e5, 0x0, 0xa5f, 0x25f],
        },
        Self {
            name: "Riparian setback",
            rows: [0xaa0, 0xaa4, 0xa5, 0x5, 0xf5, 0xfd],
        },
        Self {
            name: "Wet corner",
            rows: [0xa55, 0xa55, 0xa55, 0x257, 0xff, 0x3f],
        },
        Self {
            name: "Meadow bend",
            rows: [0x554, 0x555, 0x15, 0xe85, 0xe81, 0xea0],
        },
        Self {
            name: "Ridge pasture edge",
            rows: [0x55, 0x55, 0xa54, 0xa94, 0xfa0, 0xfe0],
        },
        Self {
            name: "Floodplain fingers",
            rows: [0x6a, 0x6a, 0x140, 0x54f, 0x54f, 0x54c],
        },
        Self {
            name: "Harvest mosaic",
            rows: [0xfe5, 0xfe5, 0x550, 0x55a, 0xfda, 0xffa],
        },
        Self {
            name: "Turnover fallows",
            rows: [0xaff, 0xaff, 0xff, 0x55, 0xa55, 0xa55],
        },
        Self {
            name: "Grain and fallow",
            rows: [0xfaa, 0xfaa, 0xfaa, 0x550, 0x550, 0x550],
        },
        Self {
            name: "Outer grazing ring",
            rows: [0x555, 0x7a9, 0x7e9, 0xaa9, 0x955, 0x955],
        },
        Self {
            name: "Stubble reserve",
            rows: [0xaaa, 0x95a, 0x81a, 0x415, 0x7d5, 0x7d5],
        },
        Self {
            name: "Eastern patchwork",
            rows: [0xaa0, 0xaa0, 0xa54, 0xe95, 0xfd5, 0xff5],
        },
        Self {
            name: "West gate holding",
            rows: [0x2a, 0xaa, 0x14a, 0x550, 0x55c, 0x5c],
        },
        Self {
            name: "Sunken field lane",
            rows: [0xa95, 0x955, 0xa0f, 0xa0f, 0xaaf, 0xaa5],
        },
        Self {
            name: "Corner exchange",
            rows: [0x15a, 0x55a, 0x50a, 0x50f, 0xabf, 0x2bf],
        },
        Self {
            name: "Broken western edge",
            rows: [0x550, 0x558, 0xf5a, 0xffa, 0x558, 0x550],
        },
    ];

    /// Inputs are local coordinates within a 64-unit surveyed field.
    pub fn sample(self, x: f32, z: f32) -> Surface {
        const START: f32 = 4.5;
        const END: f32 = 59.5;
        const CELL_SCALE: f32 = 6.0 / (END - START);
        if !(START..END).contains(&x) || !(START..END).contains(&z) {
            return Surface::Grass;
        }
        let row = (((z - START) * CELL_SCALE) as usize).min(5);
        let col = (((x - START) * CELL_SCALE) as usize).min(5);
        match (self.rows[row] >> (col * 2)) & 3 {
            1 => Surface::FieldGreen,
            2 => Surface::FieldGold,
            3 => Surface::FieldSoil,
            _ => Surface::Grass,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_parcel_has_a_distinct_cultivated_footprint_and_multiple_crop_surfaces() {
        let mut shapes = std::collections::HashSet::new();
        for profile in FieldProfile::ALL {
            let mut shape = [0u64; 6];
            let mut crop_kinds = 0u8;
            for row in 0..18 {
                for col in 0..18 {
                    let surface = profile.sample(
                        4.5 + (col as f32 + 0.5) * (55.0 / 18.0),
                        4.5 + (row as f32 + 0.5) * (55.0 / 18.0),
                    );
                    crop_kinds |= match surface {
                        Surface::FieldGreen => 1,
                        Surface::FieldGold => 2,
                        Surface::FieldSoil => 4,
                        _ => 0,
                    };
                    let i = row * 18 + col;
                    shape[i / 64] |= u64::from(surface != Surface::Grass) << (i % 64);
                }
            }
            assert!(crop_kinds.count_ones() >= 2, "{}", profile.name);
            assert!(
                shapes.insert(shape),
                "{} repeats a cultivated footprint",
                profile.name
            );
            assert_eq!(profile.sample(3.0, 27.0), Surface::Grass);
            assert_eq!(profile.sample(61.0, 27.0), Surface::Grass);
        }
    }
}
