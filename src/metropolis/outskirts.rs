//! The metropolitan fringe is laid out on the city's street axes. Major
//! streets continue into farming lanes, villages, and wooded foothills.

use super::{floor, CITY_RADIUS};
use crate::math::{fraction, mix, sin_cos, smoothstep};
use crate::world::{Sample, Surface};

const ARTERIAL: f32 = 3_456.0;
const PARCEL: f32 = 192.0;
const VILLAGE_CROSS: f32 = 11_760.0;
const VILLAGE_POINTS: [[f32; 2]; 12] = [
    [-ARTERIAL, -VILLAGE_CROSS],
    [0.0, -12_288.0],
    [ARTERIAL, -VILLAGE_CROSS],
    [-VILLAGE_CROSS, -ARTERIAL],
    [-12_288.0, 0.0],
    [-VILLAGE_CROSS, ARTERIAL],
    [VILLAGE_CROSS, -ARTERIAL],
    [12_288.0, 0.0],
    [VILLAGE_CROSS, ARTERIAL],
    [-ARTERIAL, VILLAGE_CROSS],
    [0.0, 12_288.0],
    [ARTERIAL, VILLAGE_CROSS],
];

#[derive(Clone, Copy)]
enum Lot {
    Wheat,
    Vegetable,
    Vineyard,
    Orchard,
    Pasture,
    Windbreak,
    Meadow,
    Heath,
    Woodland,
    Coppice,
    Cottages,
    Terraces,
    Courtyard,
    Inn,
    School,
    Dairy,
    Silos,
    Glasshouses,
    Stables,
    Equipment,
    Workshop,
    Warehouse,
    FireStation,
    TransportYard,
    FarmMarket,
}

use Lot::*;
use Surface::*;

const LOTS: [[Lot; 5]; 5] = [
    [Wheat, Vegetable, Vineyard, Orchard, Pasture],
    [Windbreak, Meadow, Heath, Woodland, Coppice],
    [Cottages, Terraces, Courtyard, Inn, School],
    [Dairy, Silos, Glasshouses, Stables, Equipment],
    [Workshop, Warehouse, FireStation, TransportYard, FarmMarket],
];

pub(super) fn eligible(seed: u64, u: f32, v: f32, grade: f32) -> bool {
    // The inner basin is analytically shaped and continuously low-gradient.
    // Preserve the natural relief outside it, where the hills resume.
    let x = u / CITY_RADIUS;
    let z = v / CITY_RADIUS;
    let radius = x * x + z * z;
    if grade > 0.84 || !(0.77..=1.65).contains(&radius) {
        return false;
    }
    if grade < 0.65 || arterial_distance(u).min(arterial_distance(v)) < 410.0 {
        return true;
    }
    // Broad woodland wedges interrupt the cultivated belt. Settlement and
    // cultivation can still reach farther out along the existing roads.
    let phase = fraction(seed) * 2.3;
    let wave = sin_cos(x * 10.5 + z * 6.1 + phase).0 * 0.077
        + sin_cos(z * 13.0 - x * 5.6 - phase).0 * 0.032;
    grade <= 0.735 + wave
}

fn cell(value: f32, step: f32) -> (i32, f32) {
    let offset = value / step;
    let tile = floor(offset);
    (tile, offset - tile as f32 - 0.5)
}

fn arterial_distance(value: f32) -> f32 {
    if value.abs() > ARTERIAL + 500.0 {
        return f32::MAX;
    }
    (value - floor(value / ARTERIAL + 0.5) as f32 * ARTERIAL).abs()
}

pub(super) fn road(u: f32, v: f32, grade: f32) -> bool {
    let x = u / CITY_RADIUS;
    let z = v / CITY_RADIUS;
    grade <= 0.96
        && (0.71..=1.69).contains(&(x * x + z * z))
        && arterial_distance(u).min(arterial_distance(v)) <= 11.4
}

pub(super) fn roadbed_influence(u: f32, v: f32, grade: f32) -> f32 {
    if !(0.84..1.08).contains(&grade) {
        return 0.0;
    }
    // Blend the existing low-lying grade into the road shoulder. The
    // correction fades in the lateral and longitudinal directions.
    let distance = arterial_distance(u).min(arterial_distance(v));
    (1.0 - smoothstep((distance - 24.0) / 118.0)) * (1.0 - smoothstep((grade - 0.96) / 0.12))
}

fn poles(x: f32, z: f32, spacing: f32) -> (f32, f32) {
    (cell(x + 0.5, spacing).1, cell(z + 0.5, spacing).1)
}

fn tinted(ground: Sample, surface: Surface, height: f32) -> Sample {
    Sample {
        surface,
        elevation: ground.elevation + height,
    }
}

pub(super) fn paint(seed: u64, u: f32, v: f32, grade: f32, ground: Sample) -> Sample {
    let (cx, px) = cell(u, PARCEL);
    let (cz, pz) = cell(v, PARCEL);
    if arterial_distance(u).min(arterial_distance(v)) <= 11.4 {
        return tinted(ground, Pavement, 0.0);
    }

    let code = mix(seed
        ^ (cx as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (cz as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9));
    // The closest candidate is one of three villages along this edge.
    // Neighboring sites are far enough apart that their envelopes cannot overlap.
    let east = (cx as f32 + 0.5) * PARCEL;
    let north = (cz as f32 + 0.5) * PARCEL;
    let (base, lateral) = if east.abs() > north.abs() {
        (if east > 0.0 { 6 } else { 3 }, north)
    } else {
        (if north > 0.0 { 9 } else { 0 }, east)
    };
    let lane = (floor(lateral / ARTERIAL + 0.5) + 1).clamp(0, 2) as usize;
    let i = base + lane;
    let [center_x, center_z] = VILLAGE_POINTS[i];
    let reach = 370.0 + ((seed.rotate_left(i as u32 * 5) >> 56) & 127) as f32 * 2.0;
    let village_x = (east - center_x) / reach;
    let village_z = (north - center_z) / reach;
    let village = 1.0 - 0.62 * (village_x * village_x + village_z * village_z);
    let orchard_country = (seed >> 19) & 3 != 0;
    let canopy = if orchard_country {
        TreeCanopy
    } else {
        JuniperCanopy
    };
    let ridge = (u.abs().max(v.abs()) - 11_240.0) / 4_400.0;
    let broad = ((cx + cz).div_euclid(6) ^ (cz - 2 * cx).div_euclid(8)).rem_euclid(7);
    let margin = smoothstep((grade - 0.73) / 0.11);
    let woodland = broad <= 1
        || ridge > 0.52
        || (ground.surface == Forest && broad <= 3)
        || fraction(code.rotate_left(23)) < margin * 0.77;
    let group = if village > fraction(code) * 0.85 + 0.18 {
        if (code >> 9) % 5 == 0 {
            4
        } else {
            2
        }
    } else if woodland {
        1
    } else if (code >> 17) % 9 == 0 {
        3
    } else {
        0
    };

    // Farm lanes and village streets share parcel boundaries. Occasional
    // hedges on other boundaries join woodland strips across fields.
    if px.abs() > 0.475 || pz.abs() > 0.475 {
        if group >= 2
            || (px.abs() > 0.475 && cx.rem_euclid(3) == 0)
            || (pz.abs() > 0.475 && cz.rem_euclid(3) == 0)
        {
            return tinted(ground, if group == 2 { Pavement } else { Gravel }, 0.0);
        }
        return if (px.abs() - pz.abs()).abs() > 0.06 && code.rotate_left(12) % 3 == 0 {
            tinted(ground, canopy, 2.8)
        } else {
            tinted(ground, Park, 0.0)
        };
    }
    let (mut x, mut z) = if code & 64 == 0 { (px, pz) } else { (pz, px) };
    if code & 128 != 0 {
        x = -x;
    }
    if code & 256 != 0 {
        z = -z;
    }
    let lot = LOTS[group][(code % 5) as usize];
    sample(lot, x, z, ground, canopy)
}

fn sample(lot: Lot, x: f32, z: f32, ground: Sample, canopy: Surface) -> Sample {
    let a = x.abs();
    let b = z.abs();
    let at = |surface, height| tinted(ground, surface, height);
    let ground_cover = |surface| at(surface, 0.0);
    let shore = || ground_cover(Park);
    let rect = |cx: f32, cz: f32, width: f32, length: f32| {
        (x - cx).abs() <= width && (z - cz).abs() <= length
    };
    let grove = |spacing, limit| {
        let (east, north) = poles(x, z, spacing);
        east * east + north * north < limit
    };

    match lot {
        Wheat => ground_cover(if a > 0.42 || b > 0.43 {
            FieldSoil
        } else if cell(z, 0.048).1 < -0.35 {
            FieldSoil
        } else {
            FieldGold
        }),
        Vegetable => ground_cover(if a < 0.028 || b < 0.022 || cell(x, 0.102).1.abs() > 0.39 {
            FieldSoil
        } else if z < 0.0 {
            FieldGreen
        } else {
            FieldGold
        }),
        Vineyard => {
            let row = cell(x + 0.034 * sin_cos(z * 7.0).0, 0.087).1;
            if row.abs() < 0.25 && b < 0.416 {
                at(FieldGreen, 1.6)
            } else {
                ground_cover(FieldSoil)
            }
        }
        Orchard => {
            if a < 0.42 && b < 0.43 && grove(0.168, 0.11) {
                at(canopy, 5.2)
            } else {
                ground_cover(FieldGreen)
            }
        }
        Pasture => {
            let px = (x - 0.24) / 0.135;
            let pz = (z + 0.25) / 0.105;
            let pond = px * px + pz * pz;
            if (a - 0.37).abs() < 0.024 && b < 0.4 || (b - 0.38).abs() < 0.024 && a < 0.4 {
                at(FieldSoil, 0.65)
            } else if pond < 0.69 {
                at(TarnShallows, -0.43)
            } else if pond < 1.0 {
                ground_cover(WetMeadow)
            } else {
                ground_cover(AlpineMeadow)
            }
        }
        Windbreak => {
            if (x + z * 0.6).abs() < 0.085 && grove(0.092, 0.23) {
                at(SpruceCanopy, 8.1)
            } else {
                ground_cover(FieldGreen)
            }
        }
        Meadow => {
            if (x + 0.28).abs() < 0.095 && (z - 0.19).abs() < 0.14 {
                at(canopy, 5.5)
            } else if (z + 0.32 * x * x).abs() < 0.032 {
                ground_cover(Gravel)
            } else if a + b > 0.54 {
                ground_cover(WetMeadow)
            } else {
                ground_cover(AlpineMeadow)
            }
        }
        Heath => {
            let soft = sin_cos(x * 16.0 + z * 7.0).0 * 0.25 + sin_cos(z * 14.0 - x * 4.0).0 * 0.15;
            if soft + a * 0.44 - b * 0.2 > 0.31 {
                at(JuniperCanopy, 2.8)
            } else if soft < -0.18 {
                ground_cover(ScreeFine)
            } else {
                shore()
            }
        }
        Woodland => {
            let winding = x + 0.10 * sin_cos(9.0 * z).0;
            if winding.abs() < 0.047 {
                ground_cover(Gravel)
            } else if grove(0.115, 0.24) && a + b < 0.78 {
                at(canopy, 9.1)
            } else {
                ground_cover(Forest)
            }
        }
        Coppice => {
            if a > 0.14 && b > 0.10 && grove(0.093, 0.17) {
                at(if x < 0.0 { canopy } else { AspenCanopy }, 6.6)
            } else if a < 0.053 || b < 0.047 {
                ground_cover(Gravel)
            } else {
                shore()
            }
        }
        Cottages => {
            if (a - 0.21).abs() < 0.082 && (b - 0.23).abs() < 0.088 {
                at(PitchedRoof, 5.5)
            } else if a < 0.054 || b < 0.062 {
                ground_cover(Pavement)
            } else {
                shore()
            }
        }
        Terraces => {
            if a < 0.382 && (b - 0.18).abs() < 0.066 {
                at(
                    LandmarkRoof08,
                    6.0 + 1.4 * u8::from(cell(x, 0.126).1.abs() <= 0.40) as f32,
                )
            } else if b < 0.048 {
                ground_cover(Pavement)
            } else {
                shore()
            }
        }
        Courtyard => {
            if a < 0.335 && b < 0.295 && (a > 0.22 || z > 0.17) {
                at(LandmarkRoof05, 7.2)
            } else if a < 0.28 && b < 0.28 {
                ground_cover(Gravel)
            } else {
                shore()
            }
        }
        Inn => {
            if rect(-0.16, 0.05, 0.16, 0.24) || rect(0.11, 0.23, 0.18, 0.09) {
                at(LandmarkRoof12, 8.0)
            } else if rect(0.18, -0.15, 0.19, 0.16) {
                ground_cover(Pavement)
            } else {
                shore()
            }
        }
        School => {
            if rect(0.08, 0.15, 0.26, 0.11) || rect(0.24, -0.03, 0.10, 0.16) {
                at(FlatRoof, 9.0)
            } else if rect(-0.16, -0.16, 0.22, 0.16) {
                ground_cover(if (z + 0.16).abs() < 0.009 || (x + 0.16).abs() < 0.009 {
                    RunwayMarking
                } else {
                    FieldGreen
                })
            } else {
                shore()
            }
        }
        Dairy => {
            if rect(-0.17, 0.12, 0.16, 0.22) {
                at(PitchedRoof, 11.6)
            } else if rect(0.18, 0.16, 0.11, 0.15) {
                at(IndustryMetal, 7.1)
            } else if b < 0.39 {
                ground_cover(WetMeadow)
            } else {
                shore()
            }
        }
        Silos => {
            let (east, north) = poles(x, z, 0.20);
            if (x + 0.08).abs() < 0.25 && b < 0.28 && east * east + north * north < 0.14 {
                at(IndustryMetal, 15.1 - b * 8.0)
            } else if rect(0.29, -0.02, 0.12, 0.30) {
                at(IndustryConcrete, 9.6)
            } else {
                ground_cover(Gravel)
            }
        }
        Glasshouses => {
            if a < 0.39 && b < 0.33 && cell(x, 0.149).1.abs() < 0.33 {
                at(RailGlass, 4.9)
            } else {
                ground_cover(FieldSoil)
            }
        }
        Stables => {
            if a < 0.34 && (z + 0.27).abs() < 0.10 {
                at(PitchedRoof, 8.3)
            } else if a < 0.31 && (0.0..0.30).contains(&z) {
                if (a - 0.28).abs() < 0.025 || (z - 0.26).abs() < 0.024 {
                    at(Gravel, 1.2)
                } else {
                    ground_cover(Park)
                }
            } else {
                shore()
            }
        }
        Equipment => {
            if rect(-0.24, 0.06, 0.12, 0.28) {
                at(IndustryMetal, 10.0)
            } else if rect(0.12, -0.05, 0.20, 0.24) && cell(z, 0.105).1.abs() < 0.28 {
                at(IndustryBrick, 2.8)
            } else {
                ground_cover(Gravel)
            }
        }
        Workshop => {
            if rect(0.10, -0.17, 0.29, 0.17) {
                at(IndustryBrick, 10.1)
            } else if rect(-0.14, 0.19, 0.17, 0.14) {
                at(IndustryMetal, 8.6)
            } else {
                ground_cover(Gravel)
            }
        }
        Warehouse => {
            if rect(0.0, 0.10, 0.35, 0.24) {
                at(
                    IndustryConcrete,
                    11.6 + 2.4 * u8::from(cell(x, 0.204).1.abs() < 0.32) as f32,
                )
            } else {
                ground_cover(Pavement)
            }
        }
        FireStation => {
            if rect(0.0, 0.15, 0.30, 0.18) {
                at(LandmarkRoof14, 10.1)
            } else if rect(-0.25, -0.13, 0.085, 0.09) {
                at(IndustryBrick, 15.0)
            } else if rect(0.07, -0.16, 0.22, 0.14) {
                ground_cover(Pavement)
            } else {
                shore()
            }
        }
        TransportYard => {
            if rect(-0.24, 0.16, 0.12, 0.22) {
                at(IndustryMetal, 9.3)
            } else if rect(0.13, -0.02, 0.21, 0.29) && cell(z, 0.113).1.abs() < 0.27 {
                at(RailMetal, 3.3)
            } else {
                ground_cover(Pavement)
            }
        }
        FarmMarket => {
            if rect(-0.17, 0.20, 0.18, 0.16) {
                at(LandmarkRoof03, 8.8)
            } else if rect(0.18, 0.08, 0.14, 0.28) && cell(z, 0.121).1.abs() < 0.29 {
                at(LandmarkRoof27, 3.0)
            } else {
                ground_cover(Gravel)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_regional_lot_has_a_different_physical_or_planted_layout() {
        let base = Sample {
            elevation: 19.0,
            surface: Grass,
        };
        let mut signatures = [0_u64; 25];
        for (index, lot) in LOTS.into_iter().flatten().enumerate() {
            let mut key = 0_u64;
            for z in -18..19 {
                for x in -18..19 {
                    let site = sample(lot, x as f32 / 42.0, z as f32 / 42.0, base, TreeCanopy);
                    let bits = u64::from(site.elevation.to_bits()) | ((site.surface as u64) << 32);
                    key = mix(key ^ bits);
                }
            }
            assert!(!signatures[..index].contains(&key));
            signatures[index] = key;
        }
    }
}
