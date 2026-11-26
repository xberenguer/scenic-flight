use super::{Color, BRACE, GLASS, PLATFORM, STONE, TIMBER, TRIM};

const CONCRETE: Color = [179, 177, 157];
const COPPER: Color = [70, 119, 113];
const SLATE: Color = [93, 105, 118];
const RED: Color = [154, 77, 61];
const WHITE: Color = [223, 216, 184];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SummitDesign(u8);

impl SummitDesign {
    pub const COUNT: usize = DESIGNS.len();

    pub const fn from_index(index: usize) -> Option<Self> {
        if index < Self::COUNT { Some(Self(index as u8)) } else { None }
    }

    pub(crate) const fn seeded(key: u32) -> Self {
        Self((key % Self::COUNT as u32) as u8)
    }

    pub const fn name(self) -> &'static str {
        self.data().name
    }

    pub(super) const fn data(self) -> &'static Design {
        &DESIGNS[self.0 as usize]
    }
}

pub(super) struct Design {
    pub name: &'static str,
    pub parts: &'static [Part],
}

#[derive(Clone, Copy)]
pub(super) enum Profile {
    Box,
    Gable,
    Hip,
    Dome,
}

pub(super) struct Part {
    pub center: [f32; 2],
    pub half: [f32; 2],
    pub levels: [f32; 2],
    pub color: Color,
    pub profile: Profile,
}

const fn part(x: f32, z: f32, w: f32, d: f32, base: f32, top: f32, color: Color) -> Part {
    Part { center: [x, z], half: [w, d], levels: [base, top], color, profile: Profile::Box }
}

const fn roof(profile: Profile, mut part: Part) -> Part {
    part.profile = profile;
    part
}

const fn design(name: &'static str, parts: &'static [Part]) -> Design {
    Design { name, parts }
}

use Profile::{Dome, Gable, Hip};

const DESIGNS: [Design; 25] = [
    design("Stone stair lookout", &[
        part(0.0, 0.0, 2.78, 2.73, -3.0, 1.0, STONE),
        part(0.0, 0.0, 1.95, 1.86, 1.0, 10.8, CONCRETE),
        part(0.0, 0.0, 2.82, 2.78, 10.8, 12.04, STONE),
        part(0.0, 0.0, 2.22, 2.05, 12.04, 13.63, GLASS),
        roof(Hip, part(0.0, 0.0, 2.59, 2.42, 13.63, 15.28, SLATE)),
    ]),
    design("Ridge gantry lookout", &[
        part(-2.56, 0.0, 0.55, 1.21, -3.0, 13.05, TIMBER),
        part(2.56, 0.0, 0.55, 1.21, -3.0, 13.05, TIMBER),
        part(0.0, 0.0, 2.68, 1.19, 5.4, 6.08, BRACE),
        part(0.0, 0.0, 3.28, 2.19, 12.34, 13.16, PLATFORM),
        part(-0.62, 0.0, 1.96, 1.61, 13.16, 15.4, GLASS),
        roof(Gable, part(-0.62, 0.0, 2.29, 1.94, 15.4, 16.63, RED)),
    ]),
    design("Cantilever fire lookout", &[
        part(-1.50, 0.17, 0.96, 1.24, -3.0, 12.5, CONCRETE),
        part(0.82, 0.0, 2.78, 1.88, 11.72, 12.70, TRIM),
        part(1.15, 0.0, 1.94, 1.53, 12.70, 14.95, GLASS),
        part(0.79, 0.0, 2.49, 2.08, 14.95, 15.47, WHITE),
        part(-1.87, 0.16, 0.32, 0.32, 4.9, 17.5, TRIM),
    ]),
    design("Four-pier watch cabin", &[
        part(-2.71, -2.10, 0.47, 0.47, -3.0, 10.5, STONE),
        part(2.71, -2.10, 0.47, 0.47, -3.0, 10.5, STONE),
        part(-2.71, 2.10, 0.47, 0.47, -3.0, 10.5, STONE),
        part(2.71, 2.10, 0.47, 0.47, -3.0, 10.5, STONE),
        part(0.0, 0.0, 3.53, 2.88, 10.5, 11.16, PLATFORM),
        part(0.0, 0.0, 2.54, 2.22, 11.16, 13.38, GLASS),
        roof(Hip, part(0.0, 0.0, 2.95, 2.64, 13.38, 14.78, COPPER)),
    ]),
    design("Twin observation decks", &[
        part(-1.93, 0.20, 0.85, 1.36, -3.0, 15.28, CONCRETE),
        part(1.80, -0.30, 0.82, 1.33, -3.0, 10.38, CONCRETE),
        part(0.0, 0.0, 2.78, 1.40, 8.40, 9.06, PLATFORM),
        part(-1.93, 0.20, 1.76, 2.03, 14.6, 15.48, SLATE),
        part(1.80, -0.30, 1.57, 1.74, 9.92, 10.78, RED),
        part(-1.93, 0.20, 1.31, 1.26, 15.48, 17.23, GLASS),
    ]),
    design("A-frame bivouac", &[
        part(0.0, 0.0, 2.54, 3.58, -3.0, 0.50, STONE),
        part(0.0, 0.0, 2.17, 3.20, 0.50, 1.34, TIMBER),
        roof(Gable, part(0.0, 0.0, 2.61, 3.47, 1.34, 6.80, RED)),
        part(0.0, 2.68, 0.61, 0.64, 0.50, 2.71, GLASS),
    ]),
    design("Elliptical vault refuge", &[
        part(0.0, 0.0, 2.47, 3.60, -3.0, 0.63, STONE),
        part(0.0, 0.0, 2.27, 3.36, 0.63, 2.05, CONCRETE),
        roof(Dome, part(0.0, 0.0, 2.52, 3.59, 2.05, 4.80, WHITE)),
        part(0.0, -3.45, 1.04, 0.65, 0.55, 2.75, GLASS),
    ]),
    design("Split-roof alpine refuge", &[
        part(-1.28, -0.55, 2.11, 3.36, -3.0, 3.28, TIMBER),
        part(1.68, 0.92, 1.91, 2.71, -3.0, 2.64, STONE),
        roof(Gable, part(-1.26, -0.54, 2.34, 3.62, 3.28, 6.32, SLATE)),
        roof(Gable, part(1.70, 0.95, 2.04, 2.95, 2.64, 4.87, RED)),
        part(-2.52, -1.99, 0.43, 0.44, 3.28, 6.59, STONE),
    ]),
    design("Storm hut on piers", &[
        part(-1.81, 0.0, 0.57, 2.70, -3.0, 1.82, TRIM),
        part(1.81, 0.0, 0.57, 2.70, -3.0, 1.82, TRIM),
        part(0.0, 0.0, 2.61, 3.15, 1.82, 4.31, RED),
        roof(Hip, part(0.0, 0.0, 2.82, 3.39, 4.31, 5.72, SLATE)),
        part(0.0, 3.33, 1.03, 0.48, 0.12, 1.82, PLATFORM),
    ]),
    design("Terraced stone shelter", &[
        part(0.0, 1.25, 3.44, 1.92, -3.0, 1.05, STONE),
        part(-0.26, -0.08, 2.91, 2.38, -3.0, 2.80, CONCRETE),
        part(-0.84, -0.81, 2.19, 2.16, 2.8, 4.12, STONE),
        part(-0.87, -1.66, 1.33, 1.19, 4.12, 5.25, TURF),
    ]),
    design("Weather station mast", &[
        part(0.0, 0.0, 1.90, 1.88, -3.0, 2.85, WHITE),
        part(-0.72, 0.0, 0.34, 0.34, 2.85, 13.85, SLATE),
        part(0.93, 0.0, 0.21, 0.21, 2.85, 8.92, TRIM),
        part(0.17, 0.0, 1.45, 0.30, 8.12, 8.74, WHITE),
        part(-0.70, 0.0, 1.32, 0.28, 12.90, 13.54, TRIM),
    ]),
    design("Twin relay shelters", &[
        part(-2.08, 0.0, 1.42, 2.27, -3.0, 3.48, WHITE),
        part(1.93, 0.43, 1.57, 2.67, -3.0, 4.20, CONCRETE),
        part(-2.16, 0.18, 0.32, 0.31, 3.48, 12.65, TRIM),
        part(2.0, 0.3, 0.30, 0.27, 4.20, 10.98, TRIM),
        part(2.0, 0.3, 0.75, 0.63, 9.14, 9.92, WHITE),
    ]),
    design("Radar dome on drum", &[
        part(0.0, 0.0, 2.34, 2.34, -3.0, 1.17, CONCRETE),
        part(0.0, 0.0, 1.35, 1.35, 1.17, 5.81, SLATE),
        part(0.0, 0.0, 2.18, 2.18, 5.81, 6.52, WHITE),
        roof(Dome, part(0.0, 0.0, 2.56, 2.56, 6.52, 9.84, WHITE)),
    ]),
    design("Microwave relay crossbar", &[
        part(0.0, 0.0, 2.09, 1.91, -3.0, 2.31, STONE),
        part(0.0, 0.0, 0.57, 0.57, 2.31, 13.12, TRIM),
        part(0.0, 0.0, 2.68, 0.39, 10.05, 10.75, SLATE),
        part(-2.45, 0.0, 0.58, 0.64, 9.36, 12.11, WHITE),
        part(2.45, 0.0, 0.58, 0.64, 9.36, 12.11, WHITE),
    ]),
    design("High-altitude instrument yard", &[
        part(0.0, 0.0, 3.31, 2.87, -3.0, 1.01, STONE),
        part(-1.66, -0.54, 1.61, 1.75, 1.01, 4.37, WHITE),
        part(1.95, -0.58, 0.37, 0.37, 1.01, 12.17, RED),
        part(1.62, 1.17, 0.75, 0.79, 1.01, 3.29, GLASS),
        part(-1.27, -0.59, 0.28, 0.25, 4.37, 7.32, TRIM),
    ]),
    design("Broken gatehouse", &[
        part(-2.37, -0.49, 1.03, 1.70, -3.0, 7.39, STONE),
        part(2.12, -0.42, 1.25, 1.63, -3.0, 9.31, CONCRETE),
        part(-0.04, -0.60, 2.35, 0.93, 5.48, 6.83, STONE),
        part(2.20, 1.60, 0.89, 1.05, -3.0, 2.42, STONE),
    ]),
    design("Crenellated survey cairn", &[
        part(0.0, 0.0, 2.55, 2.18, -3.0, 1.1, STONE),
        part(0.0, 0.0, 1.97, 1.61, 1.10, 3.33, STONE),
        part(0.0, 0.0, 1.50, 1.15, 3.33, 4.83, CONCRETE),
        part(-1.02, 0.13, 0.44, 0.56, 4.83, 5.94, STONE),
        part(0.91, -0.14, 0.49, 0.55, 4.83, 5.78, STONE),
    ]),
    design("Fallen ridge chapel", &[
        part(0.0, 0.0, 2.52, 3.29, -3.0, 0.46, STONE),
        part(-2.02, -0.05, 0.47, 3.30, 0.46, 3.78, CONCRETE),
        part(2.04, 0.78, 0.48, 2.45, 0.46, 2.49, CONCRETE),
        part(0.16, -2.46, 1.84, 0.52, 0.46, 3.69, CONCRETE),
        part(-1.62, 2.56, 0.68, 0.68, 3.78, 6.39, STONE),
    ]),
    design("Hillfort corner tower", &[
        part(-1.41, -0.97, 1.79, 1.96, -3.0, 7.29, STONE),
        part(1.35, -2.08, 1.82, 0.66, -3.0, 3.90, CONCRETE),
        part(-2.30, 1.14, 0.72, 1.99, -3.0, 2.74, CONCRETE),
        part(-2.46, -1.95, 0.63, 0.66, 7.29, 8.52, STONE),
        part(-0.52, 0.07, 0.63, 0.66, 7.29, 8.52, STONE),
    ]),
    design("Triangulation obelisk", &[
        part(0.0, 0.0, 3.09, 2.89, -3.0, 0.83, STONE),
        part(0.0, 0.0, 1.72, 1.60, 0.83, 2.09, CONCRETE),
        part(0.0, 0.0, 0.87, 0.86, 2.09, 7.37, CONCRETE),
        roof(Hip, part(0.0, 0.0, 0.97, 0.96, 7.37, 9.42, WHITE)),
    ]),
    design("Twin optical domes", &[
        part(0.0, 0.0, 3.53, 2.83, -3.0, 2.85, CONCRETE),
        part(-1.70, -0.39, 1.42, 1.56, 2.85, 4.08, WHITE),
        part(1.63, 0.32, 1.25, 1.35, 2.85, 3.57, WHITE),
        roof(Dome, part(-1.70, -0.39, 1.52, 1.57, 4.08, 6.31, WHITE)),
        roof(Dome, part(1.63, 0.32, 1.34, 1.40, 3.57, 5.15, SLATE)),
    ]),
    design("Solar observatory tower", &[
        part(0.0, 0.0, 2.87, 2.79, -3.0, 1.15, STONE),
        part(0.0, 0.0, 0.97, 0.96, 1.15, 11.74, WHITE),
        part(0.0, 0.0, 2.14, 2.08, 10.89, 12.05, CONCRETE),
        part(0.0, 0.0, 1.74, 1.65, 12.05, 14.42, WHITE),
        roof(Dome, part(0.0, 0.0, 1.90, 1.79, 14.42, 16.28, SLATE)),
    ]),
    design("Sliding-roof telescope shed", &[
        part(0.0, 0.0, 3.40, 2.80, -3.0, 2.75, CONCRETE),
        part(1.24, 0.0, 2.07, 2.76, 2.75, 3.60, SLATE),
        roof(Gable, part(1.24, 0.0, 2.14, 2.84, 3.60, 5.28, SLATE)),
        part(-1.98, 0.06, 0.57, 0.55, 2.75, 5.15, TRIM),
        part(-1.81, -0.32, 1.05, 0.51, 5.15, 5.82, WHITE),
    ]),
    design("Offset dome observatory", &[
        part(0.0, 0.0, 3.6, 2.82, -3.0, 0.76, STONE),
        part(0.95, -0.53, 2.48, 2.21, 0.76, 3.18, CONCRETE),
        part(-2.0, 0.75, 1.50, 1.62, 0.76, 3.69, WHITE),
        roof(Dome, part(-2.0, 0.75, 1.64, 1.75, 3.69, 6.49, WHITE)),
        part(1.84, 0.88, 0.74, 0.40, 3.18, 3.72, GLASS),
    ]),
    design("Three-pier survey platform", &[
        part(-2.00, 0.95, 0.36, 0.36, -3.0, 6.22, RED),
        part(2.00, 0.95, 0.36, 0.36, -3.0, 6.22, RED),
        part(0.0, -2.1, 0.36, 0.36, -3.0, 6.22, RED),
        part(0.0, -0.43, 2.43, 2.18, 6.22, 6.81, TRIM),
        part(0.0, -0.30, 0.48, 0.47, 6.81, 10.17, RED),
        part(0.0, -0.30, 0.91, 0.88, 9.49, 10.49, WHITE),
    ]),
];

const TURF: Color = [107, 129, 83];
