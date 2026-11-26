//! Low-rise roofs sampled inside a reserved building lot.
//!
//! Each profile returns a fraction of the building's surveyed maximum height.
//! Recessed wings can leave part of the footprint open for gardens or service yards.

use super::SettlementSurface;
use SettlementSurface::{FlatRoof, PitchedRoof, TowerRoof};

type Roof = Option<(SettlementSurface, f32)>;

fn gable(across: f32) -> f32 {
    (1.0 - across.abs()).max(0.0)
}

fn bay(across: f32, units: f32) -> (usize, f32) {
    let position = ((across + 1.0) * 0.5 * units).clamp(0.0, units - 0.0001);
    let index = position as usize;
    (index, position - index as f32)
}

pub(super) fn cottage(variant: u8, [u, v]: [f32; 2]) -> Roof {
    let roof = match variant % 6 {
        // Long farmhouse with a narrower, lower kitchen wing to the rear.
        0 if v < -0.45 => {
            if !(-0.43..0.74).contains(&u) { return None; }
            (PitchedRoof, 0.55 + 0.23 * gable((u - 0.15) / 0.59))
        }
        0 => (PitchedRoof, 0.69 + 0.31 * gable(u)),
        // A bungalow's main roof slopes down toward each side and end.
        1 if v > 0.76 => {
            if u.abs() > 0.72 { return None; }
            (PitchedRoof, 0.53 + 0.15 * (1.0 - v))
        }
        1 => (PitchedRoof, 0.66 + 0.34 * gable(u).min(gable(v / 1.19) * 1.28)),
        // An L-shaped house joins a main cross-gable to a lower side wing.
        2 if u > 0.34 => {
            if v < -0.19 { return None; }
            (PitchedRoof, 0.62 + 0.29 * gable((v - 0.41) / 0.61))
        }
        2 => (PitchedRoof, 0.66 + 0.34 * gable((u + 0.33) / 0.67)),
        // Saltbox roof. One steep slope sits over the tall front of the house.
        3 => {
            let ridge = if v > 0.27 { gable((v - 0.27) / 0.73) } else { gable((v - 0.27) / 1.27) };
            let rear_eave = 0.62 + (v * 0.09).min(0.0);
            (PitchedRoof, rear_eave + (1.0 - rear_eave) * ridge)
        }
        // Split-level ranch house and its lower attached garage.
        4 if u < -0.17 => {
            if v < -0.56 { return None; }
            (FlatRoof, 0.57 + 0.08 * gable(v))
        }
        4 => (PitchedRoof, 0.77 + 0.23 * gable(v)),
        // Two shed roof planes with a clerestory at the change of height.
        _ if v < -0.58 && u.abs() > 0.63 => return None,
        _ if u < 0.12 => (PitchedRoof, 0.60 + 0.30 * ((u + 1.0) / 1.12)),
        _ => (PitchedRoof, 0.74 + 0.26 * ((u - 0.12) / 0.88)),
    };
    Some(roof)
}

pub(super) fn duplex(variant: u8, [u, v]: [f32; 2]) -> Roof {
    let (unit, fraction) = bay(u, 2.0);
    let roof_peak = gable(2.0 * fraction - 1.0);
    let roof = match variant % 5 {
        0 if v > 0.81 && (fraction < 0.23 || fraction > 0.77) => return None,
        // Each home has its own front-facing gable and shares the party wall.
        0 => (PitchedRoof, 0.68 + 0.32 * roof_peak),
        // The primary roof is hipped, with paired lower entry bays in front.
        1 if v > 0.68 => {
            if (fraction - 0.5).abs() > 0.27 { return None; }
            (PitchedRoof, 0.49 + 0.20 * roof_peak)
        }
        1 => (PitchedRoof, 0.68 + 0.31 * gable(u).min(gable(v / 1.19) * 1.25)),
        // A pair of flat roofs with party-wall and perimeter parapets.
        2 => {
            let cap = if unit == 0 { 1.0 } else { 0.86 };
            let inset = (0.075..0.925).contains(&fraction) && v.abs() < 0.89;
            (FlatRoof, if inset { cap - 0.10 } else { cap })
        }
        // Shared mansard cap with a steep shoulder and a level roof deck.
        3 => {
            let shoulder = ((0.96 - u.abs()).min(0.91 - v.abs()) * 3.35).clamp(0.0, 1.0);
            (PitchedRoof, 0.69 + 0.31 * shoulder)
        }
        // Mirrored cottages stagger at the entry and rear garden.
        _ if unit == 0 && v > 0.68 || unit == 1 && v < -0.67 => return None,
        _ => {
            let cap = if unit == 0 { 1.0 } else { 0.91 };
            (PitchedRoof, cap * (0.70 + 0.30 * roof_peak))
        }
    };
    Some(roof)
}

pub(super) fn rowhouse(variant: u8, [face, side]: [f32; 2]) -> Roof {
    let style = variant % 6;
    let units = if style == 0 || style == 5 { 4.0 } else { 3.0 };
    let (unit, fraction) = bay(side, units);
    let peak = gable(2.0 * fraction - 1.0);
    let edge = fraction < 0.065 || fraction > 0.935;
    let roof = match style {
        // Four party walls create a stepped street silhouette above flat decks.
        0 => {
            let cap = [0.81, 0.95, 1.0, 0.88][unit];
            (FlatRoof, if edge || face.abs() > 0.92 { cap } else { cap - 0.085 })
        }
        // Front-facing gables and small dormers mark individual houses.
        1 => {
            let cap = [0.96, 0.86, 1.0][unit];
            let dormer = face > 0.35 && face < 0.74 && (fraction - 0.5).abs() < 0.14;
            let shape = if dormer { 1.0 } else { 0.67 + 0.33 * peak };
            (PitchedRoof, cap * shape)
        }
        // The end houses retain their shallow forecourts around the main ridge.
        2 if unit != 1 && face > 0.75 => return None,
        2 => (PitchedRoof, [0.90, 1.0, 0.94][unit] * (0.76 + 0.24 * gable(face))),
        // A mansard street front below recessed level roof decks.
        3 => {
            let rise = ((1.0 - face.abs()) * 3.2).min(1.0);
            let cap = [0.90, 1.0, 0.95][unit];
            (if edge { FlatRoof } else { PitchedRoof }, cap * (0.68 + 0.32 * rise))
        }
        // Rear wings flank sheltered gardens and share frontage down the block.
        4 if face < -0.38 && (unit == 1 || fraction > 0.53) => return None,
        4 if face < -0.38 => (PitchedRoof, 0.53 + 0.17 * peak),
        4 => (PitchedRoof, 0.70 + 0.30 * peak),
        // Two blocks are separated by a pedestrian passage at the party line.
        _ if side.abs() < 0.073 => return None,
        _ => (PitchedRoof, [0.85, 1.0, 0.93, 0.80][unit] * (0.72 + 0.28 * peak)),
    };
    Some(roof)
}

pub(super) fn industrial(variant: u8, [face, side]: [f32; 2]) -> Roof {
    let roof = match variant % 6 {
        // Long shallow fields rise toward short glazed north-light slopes.
        0 if face > 0.82 => {
            if side.abs() > 0.81 { return None; }
            (FlatRoof, 0.57)
        }
        0 => {
            let daylight_span = (face + 1.0) * (2.0 / 1.82) - 1.0;
            let (_, pitch) = bay(daylight_span, 3.0);
            let rise = if pitch < 0.80 { pitch / 0.80 } else { (1.0 - pitch) / 0.20 };
            let material = if pitch < 0.80 { FlatRoof } else { TowerRoof };
            (material, 0.65 + 0.35 * rise)
        }
        // Two low warehouse bays run back from a wide loading apron.
        1 if face > 0.70 => {
            if side.abs() > 0.88 { return None; }
            (FlatRoof, 0.53)
        }
        1 => {
            let (section, slope) = bay(side, 2.0);
            let cap = if section == 0 { 1.0 } else { 0.87 };
            (PitchedRoof, cap * (0.69 + 0.31 * gable(slope * 2.0 - 1.0)))
        }
        // A raised continuous monitor lights the central workshop floor.
        2 if side.abs() < 0.26 && face.abs() < 0.72 => (TowerRoof, 0.91 + 0.09 * gable(side / 0.26)),
        2 => (PitchedRoof, 0.59 + 0.13 * gable(side)),
        // A shallow barrel vault spans the loading hall beside a low annex.
        3 if side > 0.60 => {
            if face.abs() > 0.66 { return None; }
            (FlatRoof, 0.64)
        }
        3 => {
            let arc = (side + 0.20) * 1.25;
            (PitchedRoof, 0.68 + 0.32 * (1.0 - arc * arc).max(0.0))
        }
        // Offset service stair and roof lantern above a stepped machine shop.
        4 if side < -0.43 && face < -0.29 => (TowerRoof, 1.0),
        4 if face > 0.63 && side > 0.27 => return None,
        4 if side.abs() < 0.21 && face.abs() < 0.54 => (TowerRoof, 0.89),
        4 => (FlatRoof, if side > 0.56 { 0.56 } else { 0.73 }),
        // Inset roof lights and a lower loading canopy on a logistics shed.
        _ if face > 0.74 => {
            let (_, door) = bay(side, 4.0);
            if !(0.12..0.90).contains(&door) { return None; }
            (FlatRoof, 0.52)
        }
        _ => {
            let (_, light) = bay(side, 4.0);
            if face.abs() < 0.61 && (0.36..0.59).contains(&light) {
                (TowerRoof, 0.93)
            } else if side.abs() > 0.93 || face < -0.93 {
                (FlatRoof, 1.0)
            } else {
                (FlatRoof, 0.87)
            }
        }
    };
    Some(roof)
}

pub(super) fn shop(variant: u8, [face, side]: [f32; 2]) -> Roof {
    let roof = match variant % 4 {
        // Narrow corner store with a projecting street canopy and raised parapet.
        0 if face > 0.72 => {
            if side.abs() > 0.79 { return None; }
            (FlatRoof, 0.53)
        }
        0 => (FlatRoof, if face > 0.61 || side.abs() > 0.90 { 1.0 } else { 0.86 }),
        // Long public market hall with an open-fronted lean-to roof.
        1 if face > 0.58 => {
            if side.abs() > 0.89 { return None; }
            (PitchedRoof, 0.56 - 0.13 * face)
        }
        1 => (PitchedRoof, 0.66 + 0.34 * gable(side)),
        // Low shops support a smaller mixed-use floor above the back of the lot.
        2 if face < 0.24 && side.abs() < 0.62 => (FlatRoof, 1.0),
        2 if face > 0.68 && side.abs() > 0.67 => return None,
        2 => (FlatRoof, 0.64),
        // Two frontage bays and a lower rear service wing.
        _ if face < -0.41 => {
            if side < -0.33 { return None; }
            (FlatRoof, 0.59)
        }
        _ => {
            let (unit, ridge) = bay(side, 2.0);
            (PitchedRoof, [1.0, 0.87][unit] * (0.70 + 0.30 * gable(ridge * 2.0 - 1.0)))
        }
    };
    Some(roof)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settlement::{Area, Building, Form, Frontage};
    use std::collections::BTreeSet;

    #[test]
    fn all_low_rise_variants_have_distinct_profiles_inside_their_reserve() {
        assert_eq!(core::mem::size_of::<Building>(), 28);
        let lot = Area { x: -8.0, z: 13.0, half_width: 5.25, half_depth: 8.25 };
        let variants = (0..6).map(Form::Cottage)
            .chain((0..5).map(Form::Duplex))
            .chain((0..6).map(|style| Form::Rowhouse(Frontage::Far, style)))
            .chain((0..6).map(|style| Form::Industrial(Frontage::Far, style)))
            .chain((0..4).map(|style| Form::Shop(Frontage::Far, style)));
        let mut shapes = BTreeSet::new();
        for form in variants {
            let building = Building::new(lot, 7.0, 10.0, form);
            let mut signature = [0_u8; 21 * 21];
            let mut occupied = 0;
            for (index, pixel) in signature.iter_mut().enumerate() {
                let [u, v] = [(index % 21) as f32 * 0.1 - 1.0, (index / 21) as f32 * 0.1 - 1.0];
                if let Some(roof) = building.sample(lot.x + u * lot.half_width, lot.z + v * lot.half_depth) {
                    let top = roof.elevation.unwrap();
                    assert!((7.0..=17.0001).contains(&top), "{form:?} reaches {top}");
                    *pixel = 1 + ((top - 7.0) * 10.0) as u8;
                    occupied += 1;
                }
            }
            assert!(occupied > 255, "{form:?} does not cover enough of its reserved lot");
            assert!(building.sample(lot.x, lot.z - lot.half_depth - 0.001).is_none());
            assert!(shapes.insert(signature), "duplicated height profile for {form:?}");
        }
        assert_eq!(shapes.len(), 27);
    }

    #[test]
    fn sawtooth_glazing_follows_each_steep_roof_slope() {
        let face = |bay, offset| (bay as f32 + offset) * (1.82 / 3.0) - 1.0;
        for index in 0..3 {
            let [flat_at, glass_at] = [face(index, 0.65), face(index, 0.84)];
            let (flat, flat_height) = industrial(0, [flat_at, 0.0]).unwrap();
            let (glass, glass_height) = industrial(0, [glass_at, 0.0]).unwrap();
            assert_eq!((flat, glass), (FlatRoof, TowerRoof));
            assert!(glass_height > flat_height - 0.018);
            assert!(industrial(0, [face(index, 0.10), 0.0]).unwrap().1 < flat_height);
        }
    }
}
