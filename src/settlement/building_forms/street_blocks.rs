//! Additional block profiles for low-rise neighborhoods and working districts.

use super::{bay, gable, FlatRoof, PitchedRoof, Roof, TowerRoof};

fn square(value: f32) -> f32 {
    value * value
}

pub(super) fn cottage(style: u8, [u, v]: [f32; 2]) -> Roof {
    Some(match style {
        // A cross-gable house has a high transverse wing and quiet rear annex.
        0 if v < -0.58 && u > 0.32 => return None,
        0 if v > -0.24 && u < 0.28 => (PitchedRoof, 0.65 + 0.35 * gable((v - 0.38) / 0.62)),
        0 => (PitchedRoof, 0.53 + 0.34 * gable(u)),
        // A staggered split-level house leaves two opposing corners open.
        1 if v < -0.10 && u > 0.48 || v > 0.54 && u < -0.45 => return None,
        1 if v < -0.10 => (PitchedRoof, 0.82 + 0.18 * gable((u + 0.26) / 0.74)),
        1 => (FlatRoof, 0.59),
        // A tapered courtyard around a detached-looking one-storey studio.
        2 if v > 0.12 && u.abs() < 0.39 + v * 0.22 => return None,
        2 if u > 0.46 => (PitchedRoof, 0.54 + 0.19 * gable((u - 0.73) / 0.27)),
        2 if u < -0.34 => (FlatRoof, 0.68),
        2 => (PitchedRoof, 0.66 + 0.34 * gable((v + 0.41) / 0.62)),
        // A wide hipped main roof and a small gabled garage at right angles.
        3 if u > 0.52 && v < -0.19 => return None,
        3 if u > 0.52 => (PitchedRoof, 0.42 + 0.21 * gable((v - 0.43) / 0.61)),
        3 => (
            PitchedRoof,
            0.60 + 0.40 * gable((u + 0.24) / 0.76).min(gable(v) * 1.5),
        ),
        // A flat-roofed desert house gathers rooms around a shaded terrace.
        _ if v > 0.07 && (-0.48..0.42).contains(&u) => return None,
        _ if v < -0.50 && u < -0.22 => (TowerRoof, 1.0),
        _ if u < -0.49 => (FlatRoof, 0.75),
        _ if u > 0.42 => (FlatRoof, 0.62),
        _ => (FlatRoof, 0.84),
    })
}

pub(super) fn duplex(style: u8, [u, v]: [f32; 2]) -> Roof {
    let (unit, side) = bay(u, 2.0);
    Some(match style {
        // Mirrored stepped roofs above side patios and a common entry.
        0 if v > 0.48 && u.abs() > 0.61 => return None,
        0 if v < -0.26 && u.abs() > 0.31 => (TowerRoof, 1.0),
        0 if v < 0.42 => (FlatRoof, 0.76),
        0 => (FlatRoof, 0.48),
        // Offset gable ridges give the pair unequal apparent heights.
        1 if v > [0.89, 0.59][unit] || v < [-0.59, -0.91][unit] => return None,
        1 if unit == 0 => (PitchedRoof, 0.61 + 0.23 * gable(v)),
        1 => (PitchedRoof, 0.58 + 0.42 * gable(side * 2.0 - 1.0)),
        // Two narrow mansards flank a recessed roof deck.
        2 if u.abs() < 0.26 && v.abs() > 0.42 => return None,
        2 if u.abs() < 0.26 => (FlatRoof, 0.48),
        2 => {
            let eave = gable((u.abs() - 0.63) / 0.37).min(gable(v));
            (PitchedRoof, 0.52 + 0.48 * (1.7 * eave).min(1.0))
        }
        // Two barrel roofs face the street behind a shallow front arcade.
        3 if v > 0.63 && (side - 0.5).abs() > 0.39 => return None,
        3 if v > 0.63 => (FlatRoof, 0.40),
        3 => (
            PitchedRoof,
            [0.94, 1.0][unit] - 0.94 * (side - 0.5) * (side - 0.5),
        ),
        // A pair of shed roofs tilt toward a long shared entry court.
        _ if v < -0.74 && (side - 0.5).abs() > 0.31 => return None,
        _ if u.abs() < 0.20 && v > 0.10 => (FlatRoof, 0.39),
        _ if unit == 0 => (PitchedRoof, 0.59 + 0.29 * (v + 1.0) * 0.5),
        _ => (PitchedRoof, 0.60 + 0.40 * (1.0 - v) * 0.5),
    })
}

pub(super) fn rowhouse(style: u8, [face, side]: [f32; 2]) -> Roof {
    let (unit, cell) = bay(side, if style == 3 { 4.0 } else { 3.0 });
    Some(match style {
        // A recessed full-width veranda shifts the tall crests to the back.
        0 if face > 0.58 => (FlatRoof, 0.40),
        0 if face < -0.53 => (
            PitchedRoof,
            [0.85, 1.0, 0.89][unit] * (0.72 + 0.28 * gable(cell * 2.0 - 1.0)),
        ),
        0 => (PitchedRoof, 0.54 + 0.32 * gable((face + 0.12) / 0.70)),
        // Corner end terraces share a broad central stair pavilion.
        1 if face > 0.24 && unit == 1 => return None,
        1 if face < -0.32 && unit == 1 => (TowerRoof, 1.0),
        1 if face > 0.42 => (FlatRoof, 0.52),
        1 => (FlatRoof, [0.71, 0.65, 0.87][unit]),
        // Tall front drums break the line of a shared lower mansard.
        2 if face > 0.28 => {
            let radius = square((face - 0.56) / 0.44) + square((cell - 0.50) / 0.47);
            if radius > 1.0 {
                return None;
            }
            (TowerRoof, [0.86, 1.0, 0.88][unit] - 0.20 * radius)
        }
        2 => (PitchedRoof, 0.58 + 0.17 * gable(face)),
        // A four-house roofline with staggered eaves and alternate roof axes.
        3 if face > [0.54, 0.91, 0.69, 1.0][unit] => return None,
        3 if unit % 2 == 0 => (PitchedRoof, 0.59 + 0.28 * gable(cell * 2.0 - 1.0)),
        3 => (PitchedRoof, 0.65 + 0.35 * gable((face + 0.14) * 0.88)),
        // Walk-up flats recess the central street frontage between end blocks.
        _ if face > 0.57 && unit == 1 => return None,
        _ if face < -0.63 && unit != 1 => (TowerRoof, [1.0, 0.0, 0.90][unit]),
        _ if face > 0.34 => (FlatRoof, 0.44),
        _ => (FlatRoof, 0.69 + 0.10 * gable(side - 0.13)),
    })
}

pub(super) fn industrial(style: u8, [face, side]: [f32; 2]) -> Roof {
    Some(match style {
        // A barrel shed with a lower dispatch office on one side.
        0 if side > 0.43 && face < -0.38 => return None,
        0 if side > 0.43 => (FlatRoof, 0.44),
        0 => (PitchedRoof, 1.0 - 0.39 * square((side + 0.29) / 0.72)),
        // A long factory hall capped by a high stepped monitor.
        1 if face > 0.72 && side.abs() > 0.74 => return None,
        1 if side.abs() < 0.18 && face.abs() < 0.71 => (TowerRoof, 1.0),
        1 if side.abs() < 0.38 => (PitchedRoof, 0.65 + 0.21 * gable(side / 0.38)),
        1 => (PitchedRoof, 0.52 + 0.17 * gable(side)),
        // A distribution cross-dock has projecting loading roofs on both sides.
        2 if side.abs() > 0.58 => {
            let (_, dock) = bay(face, 3.0);
            if side.abs() > 0.86 && !(0.16..0.84).contains(&dock) {
                return None;
            }
            (FlatRoof, 0.36)
        }
        2 if face < -0.48 && side < -0.24 => (TowerRoof, 1.0),
        2 => (FlatRoof, 0.76),
        // A kilnhouse pairs a cylindrical stack with two tall rectangular bays.
        3 if square((face + 0.62) / 0.32) + square((side - 0.60) / 0.28) < 1.0 => (TowerRoof, 1.0),
        3 if face < -0.24 && side > 0.30 => return None,
        3 if side < -0.44 => (PitchedRoof, 0.68 + 0.22 * gable((side + 0.70) / 0.30)),
        3 => (FlatRoof, 0.52 + 0.15 * gable(face)),
        // A lumber shed has an offset ridge above a long open loading face.
        _ if face > 0.66 && side < 0.23 => return None,
        _ if side > 0.68 => (FlatRoof, 0.37),
        _ => {
            let offset = side + 0.41;
            let slope = (offset * (1.0 / 1.09)).max(-offset * (1.0 / 0.59));
            (PitchedRoof, 0.59 + 0.41 * (1.0 - slope).max(0.0))
        }
    })
}

pub(super) fn shop(style: u8, [face, side]: [f32; 2]) -> Roof {
    Some(match style {
        // A corner pharmacy with a wedge-shaped entrance and a tall corner sign.
        0 if side + face > 1.25 => return None,
        0 if side + face > 0.86 => (FlatRoof, 0.52),
        0 if (side - 0.37).abs() < 0.19 && face > 0.51 => (TowerRoof, 1.0),
        0 => (FlatRoof, 0.73),
        // A neighborhood cinema with a projecting ticket canopy.
        1 if face > 0.45 => {
            if side.abs() > 0.55 {
                return None;
            }
            (FlatRoof, 0.43)
        }
        1 if face < -0.44 && side.abs() < 0.69 => (TowerRoof, 1.0),
        1 => (PitchedRoof, 0.69 + 0.17 * gable(side)),
        // A market has individual front stalls and a long market roof behind.
        2 if face > 0.26 => {
            let (_, stall) = bay(side, 4.0);
            if face > 0.61 && !(0.12..0.88).contains(&stall) {
                return None;
            }
            (PitchedRoof, 0.41 + 0.20 * gable(stall * 2.0 - 1.0))
        }
        2 => (PitchedRoof, 0.64 + 0.36 * gable((face + 0.38) / 0.64)),
        // A narrow hotel tops a recessed street arcade with two setbacks.
        3 if face > 0.68 && side.abs() > 0.63 => return None,
        3 if face < -0.17 && side.abs() < 0.50 => (TowerRoof, 1.0),
        3 if face < 0.38 && side.abs() < 0.72 => (FlatRoof, 0.78),
        3 => (FlatRoof, 0.51),
        // A main-street bakery has a lower service wing around a rear court.
        _ if face < -0.36 && side > 0.14 => return None,
        _ if side < -0.36 => (PitchedRoof, 0.69 + 0.21 * gable((side + 0.66) / 0.34)),
        _ if face > 0.56 => (FlatRoof, 0.47),
        _ => (PitchedRoof, 0.55 + 0.45 * gable((face - 0.13) / 0.69)),
    })
}
