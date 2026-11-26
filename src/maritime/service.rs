//! Working craft. Dimensions and topsides are authored per role.

use super::DeckMaterial::{self, Bridge, Cargo, Deck, Hull};

pub(super) struct ServiceHull {
    pub name: &'static str,
    pub size: [f32; 2],
    pub profile: [f32; 3],
    deck: f32,
    open_water: Option<[f32; 3]>,
    fittings: &'static [Fitting],
}

struct Fitting {
    along: [f32; 2],
    center: f32,
    half: f32,
    height: f32,
    material: DeckMaterial,
}

const fn fitting(
    at: [f32; 2], center: f32, half: f32, height: f32, material: DeckMaterial,
) -> Fitting {
    Fitting { along: at, center, half, height, material }
}

const fn craft(
    name: &'static str, size: [f32; 2], profile: [f32; 3], deck: f32,
    open_water: Option<[f32; 3]>, fittings: &'static [Fitting],
) -> ServiceHull {
    ServiceHull { name, size, profile, deck, open_water, fittings }
}

impl ServiceHull {
    pub fn sample(&self, along: f32, across: f32, beam: f32) -> Option<(f32, DeckMaterial)> {
        if let Some([start, end, half]) = self.open_water {
            if (start..end).contains(&along) && across.abs() < half {
                return None;
            }
        }

        let mut result = if across.abs() > beam - 0.095 {
            (self.deck - 0.22, Hull)
        } else {
            (self.deck, Deck)
        };
        for item in self.fittings {
            if item.height > result.0
                && (item.along[0]..item.along[1]).contains(&along)
                && (across - item.center).abs() < item.half
            {
                result = (item.height, item.material);
            }
        }
        Some(result)
    }
}

// Half-length and half-beam are in world units. Deck fixtures use hull-local
// coordinates, with positive length towards the bow and positive beam to port.
pub(super) const HULLS: [ServiceHull; 25] = [
    craft("Pilot cutter", [6.5, 2.24], [0.40, 0.75, 1.52], 1.37, None, &[
        fitting([-0.16, 0.54], 0.0, 0.73, 3.00, Bridge),
        fitting([0.23, 0.51], 0.0, 0.52, 3.65, Bridge),
        fitting([0.08, 0.17], 0.0, 0.16, 4.44, Bridge),
    ]),
    craft("All-weather rescue launch", [7.37, 2.91], [0.53, 1.22, 1.79], 1.55, None, &[
        fitting([-0.39, 0.34], 0.0, 0.77, 3.25, Cargo),
        fitting([-0.14, 0.26], 0.0, 0.55, 3.84, Bridge),
        fitting([-0.07, 0.04], 0.0, 0.14, 4.75, Bridge),
        fitting([0.43, 0.67], -0.46, 0.17, 2.14, Cargo),
    ]),
    craft("Harbor fireboat", [9.42, 3.42], [0.55, 0.78, 1.97], 1.59, None, &[
        fitting([-0.42, 0.08], 0.0, 0.77, 2.85, Cargo),
        fitting([0.09, 0.48], 0.0, 0.60, 3.55, Bridge),
        fitting([0.47, 0.60], -0.37, 0.19, 4.15, Cargo),
        fitting([0.47, 0.60], 0.37, 0.19, 4.15, Cargo),
        fitting([-0.67, -0.47], 0.0, 0.24, 3.65, Cargo),
    ]),
    craft("Customs launch", [9.02, 2.42], [0.38, 0.76, 1.53], 1.51, None, &[
        fitting([-0.31, 0.46], 0.0, 0.61, 3.11, Bridge),
        fitting([-0.14, 0.11], 0.0, 0.43, 3.98, Bridge),
        fitting([-0.25, -0.13], 0.0, 0.12, 4.85, Bridge),
        fitting([-0.57, -0.38], 0.50, 0.22, 2.10, Cargo),
    ]),
    craft("Medical catamaran", [8.57, 3.78], [0.58, 0.75, 0.86], 1.43,
        Some([0.45, 1.0, 0.33]), &[
        fitting([-0.60, 0.35], 0.0, 0.77, 3.15, Bridge),
        fitting([0.03, 0.43], 0.0, 0.57, 3.85, Bridge),
        fitting([-0.43, -0.25], 0.0, 0.27, 4.39, Cargo),
    ]),
    craft("Echo-sounder launch", [6.79, 2.51], [0.52, 0.59, 1.86], 1.32, None, &[
        fitting([0.05, 0.47], 0.0, 0.63, 2.90, Bridge),
        fitting([-0.51, -0.22], 0.40, 0.27, 2.52, Cargo),
        fitting([-0.62, -0.49], -0.36, 0.13, 3.68, Bridge),
    ]),
    craft("Hydrographic survey cat", [10.21, 4.40], [0.61, 0.67, 1.12], 1.46,
        Some([-1.0, -0.49, 0.32]), &[
        fitting([0.00, 0.57], 0.0, 0.76, 3.42, Bridge),
        fitting([-0.46, -0.24], -0.54, 0.18, 3.96, Cargo),
        fitting([-0.46, -0.24], 0.54, 0.18, 3.96, Cargo),
        fitting([0.16, 0.31], 0.0, 0.19, 4.70, Bridge),
    ]),
    craft("Oceanographic laboratory ship", [15.85, 4.27], [0.52, 0.63, 1.73], 1.69, None, &[
        fitting([-0.38, 0.37], 0.0, 0.68, 3.18, Bridge),
        fitting([0.19, 0.46], 0.0, 0.52, 4.51, Bridge),
        fitting([-0.54, -0.32], 0.56, 0.20, 4.17, Cargo),
        fitting([-0.77, -0.60], 0.0, 0.40, 2.53, Cargo),
    ]),
    craft("Dive support ship", [13.07, 4.79], [0.66, 0.81, 1.28], 1.58, None, &[
        fitting([0.42, 0.70], 0.0, 0.57, 4.18, Bridge),
        fitting([-0.07, 0.33], -0.57, 0.30, 3.03, Cargo),
        fitting([-0.27, 0.03], 0.53, 0.25, 3.29, Cargo),
        fitting([-0.68, -0.49], -0.46, 0.21, 4.82, Bridge),
    ]),
    craft("Ocean sampling catamaran", [12.29, 5.03], [0.45, 0.64, 1.09], 1.42,
        Some([0.52, 1.0, 0.42]), &[
        fitting([-0.47, 0.42], 0.0, 0.36, 3.15, Bridge),
        fitting([0.14, 0.43], 0.0, 0.27, 4.29, Bridge),
        fitting([-0.46, -0.14], 0.59, 0.25, 2.35, Cargo),
        fitting([-0.41, -0.06], -0.58, 0.24, 2.09, Cargo),
    ]),
    craft("Buoy tender", [10.10, 4.00], [0.64, 0.54, 1.31], 1.47, None, &[
        fitting([0.31, 0.67], 0.0, 0.59, 4.22, Bridge),
        fitting([-0.36, 0.03], -0.57, 0.29, 3.10, Cargo),
        fitting([-0.58, -0.37], 0.38, 0.26, 2.65, Cargo),
        fitting([0.02, 0.22], -0.48, 0.18, 4.92, Cargo),
    ]),
    craft("Channel dredger", [15.92, 5.95], [0.72, 0.58, 1.39], 1.70,
        Some([0.62, 1.0, 0.25]), &[
        fitting([-0.64, -0.24], 0.0, 0.68, 4.05, Bridge),
        fitting([-0.17, 0.43], 0.0, 0.36, 2.77, Cargo),
        fitting([0.37, 0.64], -0.34, 0.23, 4.85, Cargo),
        fitting([0.43, 0.81], 0.48, 0.17, 3.43, Cargo),
    ]),
    craft("Floating crane barge", [13.8, 5.87], [0.82, 0.58, 0.92], 1.44, None, &[
        fitting([-0.52, -0.11], 0.0, 0.60, 2.48, Cargo),
        fitting([-0.32, 0.04], -0.24, 0.24, 4.09, Bridge),
        fitting([-0.18, 0.17], -0.20, 0.14, 5.06, Cargo),
        fitting([0.17, 0.58], -0.20, 0.12, 4.53, Cargo),
        fitting([0.19, 0.40], 0.49, 0.27, 2.45, Cargo),
    ]),
    craft("Oil-recovery twin hull", [9.74, 4.64], [0.69, 0.48, 0.92], 1.34,
        Some([0.30, 1.0, 0.43]), &[
        fitting([-0.71, -0.17], 0.0, 0.68, 3.62, Bridge),
        fitting([-0.12, 0.24], 0.0, 0.45, 2.39, Cargo),
        fitting([0.28, 0.74], 0.60, 0.15, 2.83, Cargo),
        fitting([0.28, 0.74], -0.60, 0.15, 2.83, Cargo),
    ]),
    craft("Cable repair ship", [18.8, 5.06], [0.52, 0.72, 1.70], 1.83, None, &[
        fitting([0.18, 0.62], 0.0, 0.70, 4.83, Bridge),
        fitting([-0.40, -0.12], 0.0, 0.43, 3.12, Cargo),
        fitting([-0.70, -0.46], 0.0, 0.40, 3.12, Cargo),
        fitting([-0.88, -0.70], 0.0, 0.26, 2.47, Bridge),
    ]),
    craft("Island landing craft", [10.61, 3.67], [0.76, 0.60, 1.10], 1.39, None, &[
        fitting([-0.71, -0.38], 0.46, 0.32, 3.41, Bridge),
        fitting([-0.47, 0.52], -0.56, 0.15, 2.17, Hull),
        fitting([-0.36, 0.22], 0.54, 0.19, 2.02, Hull),
        fitting([0.55, 0.92], 0.0, 0.59, 1.62, Cargo),
    ]),
    craft("Wind farm crew cat", [10.34, 4.68], [0.65, 0.88, 1.14], 1.63,
        Some([0.54, 1.0, 0.37]), &[
        fitting([-0.42, 0.51], 0.0, 0.73, 3.28, Bridge),
        fitting([0.01, 0.39], 0.0, 0.51, 4.27, Bridge),
        fitting([-0.68, -0.48], -0.47, 0.27, 2.82, Cargo),
        fitting([0.69, 0.87], 0.0, 0.48, 2.12, Cargo),
    ]),
    craft("Offshore platform supply ship", [17.20, 5.72], [0.69, 0.79, 1.55], 1.78, None, &[
        fitting([0.24, 0.67], 0.0, 0.72, 4.92, Bridge),
        fitting([-0.32, 0.16], 0.42, 0.32, 2.99, Cargo),
        fitting([-0.68, -0.38], -0.49, 0.24, 2.82, Cargo),
        fitting([-0.66, -0.30], 0.76, 0.12, 2.68, Hull),
    ]),
    craft("Harbor fuel tanker", [15.29, 4.68], [0.60, 0.59, 1.52], 1.55, None, &[
        fitting([-0.78, -0.42], 0.0, 0.59, 4.02, Bridge),
        fitting([-0.36, 0.08], -0.39, 0.30, 2.78, Cargo),
        fitting([-0.36, 0.08], 0.39, 0.30, 2.78, Cargo),
        fitting([0.14, 0.46], 0.0, 0.53, 3.02, Cargo),
    ]),
    craft("Refrigerated coastal carrier", [16.36, 4.49], [0.54, 0.65, 1.75], 1.66, None, &[
        fitting([-0.75, -0.46], 0.0, 0.58, 4.67, Bridge),
        fitting([-0.38, 0.14], 0.0, 0.70, 3.53, Bridge),
        fitting([0.23, 0.55], 0.0, 0.63, 3.26, Cargo),
        fitting([0.05, 0.17], -0.48, 0.18, 4.02, Cargo),
    ]),
    craft("Fish farm feeder barge", [10.91, 4.87], [0.78, 0.51, 0.99], 1.38, None, &[
        fitting([0.38, 0.69], 0.39, 0.45, 3.36, Bridge),
        fitting([-0.56, 0.26], -0.38, 0.41, 3.48, Cargo),
        fitting([-0.42, 0.12], 0.38, 0.29, 2.89, Cargo),
        fitting([-0.31, -0.13], -0.37, 0.18, 4.52, Bridge),
    ]),
    craft("Aquaculture harvesting cat", [8.36, 4.19], [0.64, 0.52, 1.06], 1.32,
        Some([-1.0, -0.38, 0.32]), &[
        fitting([0.27, 0.61], -0.18, 0.63, 3.19, Bridge),
        fitting([-0.40, 0.12], 0.53, 0.29, 2.24, Cargo),
        fitting([-0.49, -0.31], -0.53, 0.23, 4.36, Cargo),
    ]),
    craft("Net service launch", [6.98, 3.01], [0.65, 0.75, 1.74], 1.28, None, &[
        fitting([0.16, 0.58], 0.31, 0.44, 2.87, Bridge),
        fitting([-0.57, -0.05], -0.44, 0.31, 1.89, Cargo),
        fitting([-0.68, -0.48], 0.43, 0.20, 3.53, Cargo),
    ]),
    craft("Live well carrier", [12.36, 4.45], [0.57, 0.76, 1.46], 1.51, None, &[
        fitting([0.40, 0.68], 0.0, 0.54, 3.79, Bridge),
        fitting([-0.49, -0.16], 0.0, 0.59, 2.72, Cargo),
        fitting([-0.07, 0.27], 0.0, 0.59, 3.08, Cargo),
        fitting([-0.74, -0.55], 0.43, 0.21, 3.24, Bridge),
    ]),
    craft("Harbor debris skimmer", [7.58, 3.81], [0.75, 0.62, 0.83], 1.34,
        Some([0.18, 1.0, 0.45]), &[
        fitting([-0.81, -0.37], -0.32, 0.51, 3.13, Bridge),
        fitting([-0.36, 0.24], 0.63, 0.23, 2.17, Cargo),
        fitting([0.25, 0.74], -0.59, 0.17, 2.26, Cargo),
        fitting([-0.22, 0.12], -0.20, 0.28, 2.66, Cargo),
    ]),
];
