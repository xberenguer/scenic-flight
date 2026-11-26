//! Additional roof plans for homes, main streets and working neighborhoods.
use super::{bay, gable, FlatRoof, PitchedRoof, Roof, TowerRoof};

fn square(value: f32) -> f32 {
    value * value
}

pub(super) fn cottage(style: u8, [u, v]: [f32; 2]) -> Roof {
    let roof = match style {
        // A chalet shelters a recessed front porch under its deep eaves.
        0 if v > 0.65 && u.abs() > 0.72 => return None,
        0 => (PitchedRoof, 0.47 + 0.53 * gable(u)),
        // Adobe patio house with a deep, open courtyard at the street.
        1 if v > -0.52 && u.abs() < 0.37 => return None,
        1 if v < -0.53 && u < -0.31 => (FlatRoof, 1.0),
        1 => (FlatRoof, 0.66),
        // A pyramidal cottage with a small entrance pavilion.
        2 if v > 0.61 => {
            if u.abs() > 0.49 {
                return None;
            }
            (PitchedRoof, 0.52 + 0.22 * gable(u * 2.0))
        }
        2 => (
            PitchedRoof,
            0.67 + 0.33 * gable(u).min(gable((v + 0.16) / 0.84)),
        ),
        // An A-frame cabin with a broad gable behind its narrow front deck.
        3 if v > 0.75 => {
            if u.abs() > 0.54 {
                return None;
            }
            (FlatRoof, 0.20)
        }
        3 => (PitchedRoof, 0.32 + 0.68 * gable(u)),
        // A courtyard ranch wraps low wings around a garden at the back.
        4 if v < -0.21 && u.abs() < 0.47 => return None,
        4 if v < -0.21 => (PitchedRoof, 0.49 + 0.21 * gable((u.abs() - 0.73) / 0.27)),
        4 => (PitchedRoof, 0.66 + 0.34 * gable((v - 0.39) / 0.61)),
        // A gambrel barn conversion keeps an attached, low lean-to.
        _ if u > 0.67 => {
            if v.abs() > 0.58 {
                return None;
            }
            (PitchedRoof, 0.55 - u * 0.16)
        }
        _ => {
            let edge = ((u + 0.165) / 0.835).abs();
            (
                PitchedRoof,
                1.0 - 0.19 * edge - 0.43 * (edge - 0.48).max(0.0),
            )
        }
    };
    Some(roof)
}

pub(super) fn duplex(style: u8, [u, v]: [f32; 2]) -> Roof {
    let (unit, fraction) = bay(u, 2.0);
    let center = (fraction - 0.5).abs();
    let roof = match style {
        // Paired butterfly roofs drain inward toward each center line.
        0 if v < -0.66 && center > 0.38 => return None,
        0 => (PitchedRoof, 0.62 + 0.70 * center),
        // Two linked villas step toward a narrow shared garden passage.
        1 if u.abs() < 0.20 => {
            if v.abs() > 0.24 {
                return None;
            }
            (FlatRoof, 0.50)
        }
        1 => (PitchedRoof, 0.69 + 0.30 * gable(v)),
        // Twin saltboxes mirror the tall ends toward the party wall.
        2 => (
            PitchedRoof,
            0.58 + 0.42 * if unit == 0 { fraction } else { 1.0 - fraction },
        ),
        // An offset pair has recessed rooftop patios over the street frontage.
        3 if v > 0.73 && unit == 1 => return None,
        3 if v < -0.37 + unit as f32 * 0.59 && center < 0.37 => (FlatRoof, 1.0),
        3 => (FlatRoof, 0.68 + unit as f32 * 0.10),
        // Connected half-barrel homes have single-storey side wings.
        _ if center > 0.38 => (FlatRoof, 0.52),
        _ => (
            PitchedRoof,
            (1.0 - center * center * 2.4) * if unit == 0 { 1.0 } else { 0.85 },
        ),
    };
    Some(roof)
}

pub(super) fn rowhouse(style: u8, [face, side]: [f32; 2]) -> Roof {
    let (unit, fraction) = bay(side, if style == 3 { 5.0 } else { 3.0 });
    let center = (fraction - 0.5).abs();
    let roof = match style {
        // Three individual barrel vaults above shared street frontage.
        0 if face > 0.76 && center > 0.30 => return None,
        0 => (PitchedRoof, 1.0 - center * center * 1.13),
        // Raised stairhouses punctuate a row of rooftop gardens.
        1 if face < -0.17 && center < 0.22 => (TowerRoof, [0.92, 1.0, 0.96][unit]),
        1 if face > 0.69 && center > 0.32 => return None,
        1 => (FlatRoof, 0.60),
        // Three Dutch stepped-gable fronts along a narrow street.
        2 if face > 0.68 => {
            let steps = (center * 8.0) as u8;
            (PitchedRoof, 1.0 - 0.09 * f32::from(steps))
        }
        2 => (PitchedRoof, 0.60 + 0.25 * gable(fraction * 2.0 - 1.0)),
        // Five alternating sawtooth roofs frame a low shared rear wing.
        3 if face < -0.61 => (FlatRoof, 0.51),
        3 => (
            PitchedRoof,
            0.60 + 0.36
                * if unit % 2 == 0 {
                    fraction
                } else {
                    1.0 - fraction
                },
        ),
        // Deep terraces across the front of a stepped hillside row.
        _ if face > [0.38, 0.96, 0.61][unit] => return None,
        _ if face > 0.05 => (FlatRoof, [0.55, 0.66, 0.59][unit]),
        _ if face > -0.44 => (FlatRoof, [0.69, 0.76, 0.73][unit]),
        _ => (PitchedRoof, 0.77 + 0.23 * gable(fraction * 2.0 - 1.0)),
    };
    Some(roof)
}

pub(super) fn industrial(style: u8, [face, side]: [f32; 2]) -> Roof {
    let roof = match style {
        // Bulk terminal with three cylindrical silos and a low transfer gallery.
        0 => {
            let (_, fraction) = bay(side, 3.0);
            let distance = square(fraction * 2.0 - 1.0) + square((face + 0.22) / 0.60);
            if distance < 0.79 {
                (TowerRoof, 1.0 - 0.17 * distance)
            } else if face > 0.38 && side.abs() < 0.92 {
                (FlatRoof, 0.37)
            } else {
                return None;
            }
        }
        // A power house with a loading hall and paired mechanical stacks.
        1 if face < -0.43 && (side.abs() - 0.56).abs() < 0.16 => (TowerRoof, 1.0),
        1 if face > 0.39 => {
            if side.abs() > 0.59 {
                return None;
            }
            (PitchedRoof, 0.47 + 0.16 * gable(side))
        }
        1 => (FlatRoof, 0.70),
        // A twin-bay hangar with broad arches over separate approach lanes.
        2 if side.abs() < 0.11 || face > 0.85 => return None,
        2 => {
            let arc = (side.abs() - 0.55) / 0.45;
            (PitchedRoof, 1.0 - 0.37 * arc * arc)
        }
        // Cooling plant with separate, round ventilator housings on a low deck.
        3 if face > 0.63 && side < 0.29 => return None,
        3 => {
            let (_, fraction) = bay(face, 3.0);
            let distance = square(fraction * 2.0 - 1.0) + square((side - 0.24) / 0.47);
            if distance < 0.71 {
                (TowerRoof, 0.91 - 0.13 * distance)
            } else {
                (FlatRoof, 0.53)
            }
        }
        // A repair works encloses a service court between high and low sheds.
        4 if face > -0.21 && side.abs() < 0.36 => return None,
        4 if side < -0.40 => (PitchedRoof, 1.0 - 0.39 * ((side + 0.70) / 0.30).abs()),
        4 if side > 0.40 => (PitchedRoof, 0.65 - 0.20 * face),
        4 => (FlatRoof, 0.57),
        // A glasshouse range with clear ridges and a central service spine.
        _ if face.abs() < 0.17 => (FlatRoof, 0.57),
        _ => {
            let (_, span) = bay(side, 4.0);
            if !(0.07..0.93).contains(&span) {
                return None;
            }
            (TowerRoof, 0.58 + 0.40 * gable(span * 2.0 - 1.0))
        }
    };
    Some(roof)
}

pub(super) fn shop(style: u8, [face, side]: [f32; 2]) -> Roof {
    let roof = match style {
        // An arcade wraps an open market court.
        0 if face > -0.42 && side.abs() < 0.39 => return None,
        0 if side.abs() > 0.48 => (PitchedRoof, 0.77 + 0.23 * gable((side.abs() - 0.74) / 0.26)),
        0 => (FlatRoof, 0.63),
        // A roadside diner with a much lower projecting awning.
        1 if face > 0.52 => {
            if side.abs() > 0.82 {
                return None;
            }
            (FlatRoof, 0.41)
        }
        1 if face < -0.61 && side.abs() > 0.56 => return None,
        1 => (PitchedRoof, 1.0 - 0.21 * side * side),
        // A square corner bank with clipped corners and a central lantern.
        2 if face.abs() + side.abs() > 1.55 => return None,
        2 if face.abs() < 0.40 && side.abs() < 0.40 => (TowerRoof, 1.0),
        2 => (FlatRoof, 0.68 + 0.06 * side.abs()),
        // A theater narrows toward the foyer below its raised stage house.
        3 if face < -0.58 && side.abs() < 0.77 => (TowerRoof, 1.0),
        3 if face > 0.46 => {
            if side.abs() > 0.46 {
                return None;
            }
            (FlatRoof, 0.50)
        }
        3 => (PitchedRoof, 0.57 + 0.18 * gable(side)),
        // Attached corner storefronts share two wings at different heights.
        _ if face < -0.28 && side > 0.24 => return None,
        _ if side < -0.37 => (FlatRoof, 0.93 + 0.07 * gable(face)),
        _ => (PitchedRoof, 0.65 + 0.23 * gable((face - 0.35) / 0.65)),
    };
    Some(roof)
}
