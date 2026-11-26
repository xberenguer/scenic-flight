//! Principal buildings for the dry service pads at surveyed roadside stops.

use super::TurnoutKind;
use crate::world::Surface::{self, FlatRoof, IndustryBrick, IndustryConcrete, IndustryMetal, PitchedRoof};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RoadsideBuilding(u8);

impl RoadsideBuilding {
    pub const COUNT: usize = BUILDINGS.len();

    pub const fn from_index(index: usize) -> Option<Self> {
        if index < Self::COUNT { Some(Self(index as u8)) } else { None }
    }

    pub const fn name(self) -> &'static str {
        BUILDINGS[self.0 as usize].name
    }

    pub(super) const fn select(kind: TurnoutKind, variant: u8) -> Self {
        let index = match kind {
            TurnoutKind::Overlook => variant / 50,
            TurnoutKind::Trailhead => 5 + variant / 25,
            TurnoutKind::RestStop => 15 + variant / 25,
        };
        Self(index)
    }

    /// Height above the surveyed pad, in the local coordinate range of -1 to 1.
    pub fn profile(self, [u, v]: [f32; 2]) -> Option<(f32, Surface)> {
        let mut highest = None;
        for part in BUILDINGS[usize::from(self.0)].parts {
            let x = ((u - part.center[0]) * part.inverse[0]).abs();
            let across = (v - part.center[1]) * part.inverse[1];
            let z = across.abs();
            if x >= 1.0 || z >= 1.0 { continue; }
            let shape = match part.roof {
                Roof::Flat => 1.0,
                Roof::LongGable => 1.0 - z,
                Roof::CrossGable => 1.0 - x,
                Roof::Hip => 1.0 - x.max(z),
                Roof::Barrel => 1.0 - z * z,
                Roof::Round => {
                    if x * x + z * z >= 1.0 { continue; }
                    1.0
                }
                Roof::Shed => 0.5 + across * 0.5,
            };
            let height = part.eave + (part.ridge - part.eave) * shape;
            if highest.is_none_or(|(top, _)| height > top) {
                highest = Some((height, part.surface));
            }
        }
        highest
    }
}

#[derive(Clone, Copy)]
enum Roof { Flat, LongGable, CrossGable, Hip, Barrel, Round, Shed }
use Roof::*;

struct Part {
    center: [f32; 2],
    inverse: [f32; 2],
    eave: f32,
    ridge: f32,
    roof: Roof,
    surface: Surface,
}

const fn part(center: [f32; 2], half: [f32; 2], eave: f32, ridge: f32, roof: Roof, surface: Surface) -> Part {
    Part { center, inverse: [1.0 / half[0], 1.0 / half[1]], eave, ridge, roof, surface }
}

const fn flat(center: [f32; 2], half: [f32; 2], height: f32, surface: Surface) -> Part {
    part(center, half, height, height, Flat, surface)
}

struct Building { name: &'static str, parts: &'static [Part] }

const fn building(name: &'static str, parts: &'static [Part]) -> Building {
    Building { name, parts }
}

const BUILDINGS: [Building; 25] = [
    building("Crescent lookout pavilion", &[
        flat([0.0, 0.44], [0.66, 0.42], 0.58, IndustryConcrete),
        part([-0.56, 0.14], [0.35, 0.65], 2.70, 3.57, CrossGable, PitchedRoof),
        part([0.57, 0.12], [0.33, 0.58], 2.57, 3.39, CrossGable, PitchedRoof),
        part([0.02, -0.47], [0.55, 0.31], 3.10, 4.09, LongGable, PitchedRoof),
    ]),
    building("Stone panorama rotunda", &[
        part([-0.16, 0.06], [0.52, 0.74], 4.38, 4.38, Round, IndustryConcrete),
        part([-0.16, 0.06], [0.36, 0.52], 4.83, 4.83, Round, IndustryMetal),
        flat([0.57, -0.28], [0.31, 0.37], 1.82, IndustryBrick),
        flat([0.48, 0.51], [0.45, 0.29], 0.43, IndustryConcrete),
    ]),
    building("Two-level landscape gallery", &[
        flat([-0.24, 0.01], [0.69, 0.82], 2.01, IndustryConcrete),
        flat([-0.34, -0.34], [0.49, 0.45], 3.86, IndustryMetal),
        flat([-0.47, -0.52], [0.26, 0.25], 4.82, IndustryMetal),
        flat([0.68, 0.05], [0.25, 0.66], 1.21, IndustryConcrete),
    ]),
    building("Ridge shelter with a tilted roof", &[
        part([-0.22, 0.09], [0.71, 0.74], 2.54, 4.08, Shed, IndustryMetal),
        flat([0.56, -0.34], [0.40, 0.43], 2.78, FlatRoof),
        flat([0.66, 0.49], [0.26, 0.28], 1.10, IndustryConcrete),
    ]),
    building("Overlook refuge with a stone chimney", &[
        part([0.13, -0.03], [0.78, 0.65], 2.72, 4.68, Hip, PitchedRoof),
        flat([-0.61, -0.21], [0.20, 0.20], 5.29, IndustryBrick),
        flat([-0.61, -0.21], [0.24, 0.24], 5.53, IndustryConcrete),
        part([0.54, 0.62], [0.40, 0.31], 1.81, 2.51, Shed, IndustryMetal),
    ]),
    building("Forestry patrol cabin", &[
        part([-0.24, -0.18], [0.69, 0.58], 2.43, 4.10, LongGable, PitchedRoof),
        flat([-0.65, -0.23], [0.13, 0.21], 4.76, IndustryBrick),
        part([0.57, 0.29], [0.36, 0.52], 1.60, 2.53, Shed, IndustryMetal),
    ]),
    building("Forest volunteer store and drying room", &[
        part([-0.58, 0.03], [0.37, 0.85], 2.30, 3.75, CrossGable, PitchedRoof),
        part([0.50, -0.15], [0.39, 0.72], 2.22, 3.23, LongGable, PitchedRoof),
        flat([-0.01, -0.58], [0.46, 0.26], 1.78, IndustryMetal),
    ]),
    building("Trail registration hall with a raised lantern", &[
        part([0.10, -0.08], [0.81, 0.79], 2.76, 3.81, LongGable, PitchedRoof),
        part([0.04, -0.04], [0.41, 0.29], 4.03, 4.77, LongGable, IndustryMetal),
        flat([-0.58, 0.67], [0.33, 0.24], 1.67, IndustryConcrete),
    ]),
    building("Fire crew garage and hose tower", &[
        part([0.14, -0.04], [0.75, 0.71], 2.66, 3.77, Shed, IndustryMetal),
        flat([-0.56, 0.48], [0.30, 0.36], 4.76, IndustryBrick),
        part([-0.56, 0.48], [0.34, 0.40], 4.70, 5.77, Hip, PitchedRoof),
        flat([0.64, 0.68], [0.25, 0.23], 1.93, IndustryConcrete),
    ]),
    building("Warden post with a covered porch", &[
        part([0.28, -0.23], [0.62, 0.63], 2.47, 4.35, CrossGable, PitchedRoof),
        part([0.02, 0.54], [0.67, 0.34], 1.97, 2.72, Shed, IndustryMetal),
        flat([-0.67, -0.33], [0.29, 0.42], 1.74, FlatRoof),
    ]),
    building("Covered bicycle workshop", &[
        part([-0.29, 0.10], [0.67, 0.69], 2.05, 3.68, Barrel, IndustryMetal),
        flat([0.64, -0.31], [0.30, 0.40], 2.57, IndustryBrick),
        flat([0.64, 0.42], [0.24, 0.34], 0.56, IndustryConcrete),
    ]),
    building("Paddling outfitter and gear lockers", &[
        part([-0.43, -0.20], [0.50, 0.67], 2.58, 3.58, LongGable, PitchedRoof),
        part([0.48, -0.47], [0.42, 0.43], 2.06, 2.86, Shed, IndustryMetal),
        flat([0.56, 0.38], [0.36, 0.35], 1.04, IndustryBrick),
        flat([-0.13, 0.56], [0.25, 0.28], 0.70, IndustryConcrete),
    ]),
    building("Cross-country ski warming lodge", &[
        part([0.11, -0.12], [0.78, 0.73], 2.13, 5.09, LongGable, PitchedRoof),
        part([-0.30, 0.25], [0.31, 0.58], 2.34, 3.80, CrossGable, PitchedRoof),
        flat([0.64, -0.12], [0.13, 0.22], 4.70, IndustryBrick),
    ]),
    building("Bridle trail shelters and tack room", &[
        part([-0.65, -0.14], [0.30, 0.72], 2.64, 3.55, CrossGable, PitchedRoof),
        part([0.58, 0.04], [0.35, 0.77], 2.09, 2.94, CrossGable, PitchedRoof),
        part([0.0, -0.60], [0.47, 0.26], 1.81, 2.42, Shed, IndustryMetal),
        flat([0.03, 0.62], [0.22, 0.19], 0.57, IndustryConcrete),
    ]),
    building("Camp check-in kiosk and supply shed", &[
        part([-0.53, 0.07], [0.36, 0.65], 2.46, 4.36, Hip, PitchedRoof),
        part([0.50, -0.17], [0.44, 0.71], 2.10, 3.15, Shed, IndustryMetal),
        flat([-0.04, 0.61], [0.38, 0.28], 0.45, IndustryConcrete),
    ]),
    building("Road repair garage with a monitor roof", &[
        part([0.02, -0.16], [0.91, 0.68], 2.69, 3.81, LongGable, IndustryMetal),
        part([0.04, -0.17], [0.65, 0.28], 3.85, 4.71, LongGable, IndustryMetal),
        flat([0.62, 0.59], [0.32, 0.31], 2.36, IndustryBrick),
    ]),
    building("Snowplow depot with salt store", &[
        part([-0.41, -0.06], [0.54, 0.77], 2.29, 3.38, Shed, IndustryMetal),
        part([0.59, -0.17], [0.35, 0.64], 2.50, 4.44, Barrel, IndustryMetal),
        flat([0.56, 0.64], [0.34, 0.24], 0.87, IndustryConcrete),
    ]),
    building("Rural weigh station and inspection booth", &[
        flat([-0.63, 0.04], [0.34, 0.73], 3.20, IndustryConcrete),
        flat([0.38, -0.28], [0.55, 0.33], 2.42, IndustryMetal),
        flat([0.65, 0.34], [0.27, 0.21], 1.55, IndustryBrick),
        flat([0.28, 0.62], [0.40, 0.22], 0.38, IndustryConcrete),
    ]),
    building("Farm stand with a cold room", &[
        part([0.10, -0.02], [0.79, 0.66], 2.03, 3.37, LongGable, PitchedRoof),
        part([-0.55, 0.53], [0.39, 0.35], 1.51, 2.08, Shed, IndustryMetal),
        flat([0.58, -0.49], [0.36, 0.34], 3.90, IndustryConcrete),
        flat([0.63, 0.64], [0.25, 0.22], 0.56, IndustryConcrete),
    ]),
    building("Rural post office and mail depot", &[
        part([0.35, -0.10], [0.52, 0.74], 2.63, 4.18, Hip, PitchedRoof),
        flat([-0.51, -0.28], [0.43, 0.55], 2.70, IndustryBrick),
        flat([-0.62, 0.43], [0.35, 0.37], 1.87, IndustryMetal),
        flat([0.63, 0.46], [0.21, 0.33], 3.33, IndustryBrick),
    ]),
    building("Highway café with a rounded corner", &[
        flat([-0.25, -0.03], [0.68, 0.72], 3.05, IndustryConcrete),
        part([0.59, -0.06], [0.35, 0.70], 3.05, 3.05, Round, IndustryConcrete),
        flat([0.56, -0.16], [0.22, 0.32], 3.76, IndustryMetal),
        flat([-0.39, 0.66], [0.48, 0.24], 1.93, IndustryMetal),
    ]),
    building("Coach rest hall with twin barrel roofs", &[
        part([-0.46, -0.06], [0.48, 0.71], 2.87, 4.23, Barrel, IndustryMetal),
        part([0.52, 0.08], [0.45, 0.70], 2.61, 3.95, Barrel, IndustryMetal),
        flat([0.02, -0.65], [0.40, 0.24], 2.07, IndustryBrick),
    ]),
    building("Wayside tea house with a hipped wing", &[
        part([-0.25, -0.06], [0.65, 0.68], 2.27, 4.04, Hip, PitchedRoof),
        part([0.64, -0.23], [0.29, 0.54], 2.17, 3.40, CrossGable, PitchedRoof),
        flat([0.12, 0.65], [0.75, 0.23], 1.21, IndustryConcrete),
    ]),
    building("Split-roof highway picnic hall", &[
        part([-0.53, -0.16], [0.37, 0.70], 2.25, 3.81, Shed, IndustryMetal),
        part([0.47, -0.10], [0.45, 0.75], 2.46, 4.26, LongGable, PitchedRoof),
        flat([0.03, -0.62], [0.49, 0.27], 1.96, IndustryMetal),
        flat([0.14, 0.60], [0.73, 0.23], 0.45, IndustryConcrete),
    ]),
    building("Visitor information hall and clock column", &[
        part([0.02, -0.34], [0.85, 0.56], 2.33, 3.51, LongGable, PitchedRoof),
        flat([-0.62, 0.43], [0.28, 0.41], 2.10, IndustryBrick),
        flat([0.62, 0.29], [0.21, 0.27], 4.79, IndustryBrick),
        part([0.62, 0.29], [0.28, 0.36], 4.75, 5.40, Hip, IndustryMetal),
    ]),
];
