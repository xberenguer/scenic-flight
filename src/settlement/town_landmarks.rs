//! Site-sized buildings for the streets between downtown and the open countryside.

use super::SettlementSurface::{self, FlatRoof, IndustryBrick, IndustryConcrete, IndustryMetal, PitchedRoof, TowerRoof};

mod industry;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TownLandmark(u8);

impl TownLandmark {
    pub const INDUSTRY_COUNT: usize = industry::LANDMARKS.len();
    pub const COUNT: usize = LANDMARKS.len() + Self::INDUSTRY_COUNT;

    pub fn from_index(index: usize) -> Option<Self> {
        (index < Self::COUNT).then_some(Self(index as u8))
    }

    pub fn industry_from_index(index: usize) -> Option<Self> {
        (index < Self::INDUSTRY_COUNT).then_some(Self((LANDMARKS.len() + index) as u8))
    }

    pub fn is_industry(self) -> bool {
        usize::from(self.0 & 63) >= LANDMARKS.len()
    }

    pub(crate) fn industry(key: u32) -> Self {
        Self::industry_from_index(key as usize % Self::INDUSTRY_COUNT).unwrap()
    }

    pub(crate) fn select(family: usize, choice: u32) -> Self {
        Self((family * 5 + choice as usize % 5) as u8)
    }

    pub fn name(self) -> &'static str {
        self.design().name
    }

    pub fn dimensions(self) -> [f32; 3] {
        self.design().size
    }

    pub fn roof(self, [side, face]: [f32; 2]) -> Option<(SettlementSurface, f32)> {
        self.design().masses.iter().filter_map(|mass| {
            let side = (side - mass.center[0]).abs();
            let face = (face - mass.center[1]).abs();
            if side >= mass.half[0] || face >= mass.half[1] {
                return None;
            }
            if matches!(mass.form, Roof::Slab | Roof::Lantern) {
                let roof = if matches!(mass.form, Roof::Slab) { FlatRoof } else { TowerRoof };
                return Some((self.material(mass.form, roof), mass.top));
            }
            let u = side / mass.half[0];
            let v = face / mass.half[1];
            let (surface, slope) = match mass.form {
                Roof::Slab => (FlatRoof, 1.0),
                Roof::Gable => (PitchedRoof, 1.0 - u),
                Roof::CrossGable => (PitchedRoof, 1.0 - v),
                Roof::Hip => (PitchedRoof, 1.0 - u.max(v)),
                Roof::Barrel => (PitchedRoof, 1.0 - u * u),
                Roof::Monitor => (PitchedRoof, if u < 0.24 { 1.0 } else { (1.0 - u) * 0.67 }),
                Roof::Dome => (PitchedRoof, (1.0 - u * u - v * v).max(0.0)),
                Roof::Drum => (TowerRoof, 1.0),
                Roof::Lantern => (TowerRoof, 1.0),
            };
            if matches!(mass.form, Roof::Dome | Roof::Drum) && u * u + v * v >= 1.0 {
                return None;
            }
            let height = mass.eave + (mass.top - mass.eave) * slope;
            Some((self.material(mass.form, surface), height))
        }).max_by(|a, b| a.1.total_cmp(&b.1))
    }

    pub(crate) fn design(self) -> &'static Landmark {
        let index = usize::from(self.0 & 63);
        if self.is_industry() { &industry::LANDMARKS[index - LANDMARKS.len()] } else { &LANDMARKS[index] }
    }

    fn material(self, form: Roof, ordinary: SettlementSurface) -> SettlementSurface {
        if !self.is_industry() { return ordinary; }
        let index = usize::from(self.0 & 63) - LANDMARKS.len();
        match form {
            Roof::Barrel | Roof::Monitor | Roof::Lantern => IndustryMetal,
            Roof::Drum | Roof::Dome if !matches!(index, 0 | 6 | 8 | 14 | 21) => IndustryConcrete,
            _ if matches!(index, 0 | 6 | 8 | 14 | 21) => IndustryBrick,
            _ if (10..20).contains(&index) => IndustryMetal,
            _ => IndustryConcrete,
        }
    }

    pub(crate) fn with_region(self, region: u8) -> Self {
        debug_assert!(region < 4);
        Self((self.0 & 63) | (region << 6))
    }

    pub(crate) fn from_profile(profile: u16) -> Option<Self> {
        (profile <= u8::MAX as u16 && usize::from(profile & 63) < Self::COUNT)
            .then_some(Self(profile as u8))
    }

    pub(crate) fn palette_id(self) -> Option<u8> {
        (!self.is_industry()).then(|| super::building_palettes::town_finish(self.0 & 63, self.0 >> 6))
    }

    pub(crate) fn id(self) -> u16 {
        u16::from(self.0)
    }
}

pub(crate) struct Landmark {
    name: &'static str,
    size: [f32; 3],
    pub masses: &'static [Mass],
}

pub(crate) struct Mass {
    pub center: [f32; 2],
    pub half: [f32; 2],
    pub top: f32,
    eave: f32,
    form: Roof,
}

#[derive(Clone, Copy)]
enum Roof {
    Slab,
    Gable,
    CrossGable,
    Hip,
    Barrel,
    Monitor,
    Dome,
    Drum,
    Lantern,
}

use Roof::{CrossGable, Dome, Drum, Gable, Hip, Lantern, Slab};

const fn mass(center: [f32; 2], half: [f32; 2], eave: f32, top: f32, form: Roof) -> Mass {
    Mass { center, half, top, eave, form }
}

const fn level(center: [f32; 2], half: [f32; 2], height: f32, form: Roof) -> Mass {
    mass(center, half, height, height, form)
}

const fn landmark(name: &'static str, size: [f32; 3], masses: &'static [Mass]) -> Landmark {
    Landmark { name, size, masses }
}

const LANDMARKS: [Landmark; 25] = [
    landmark("Fire hall with hose tower", [6.12, 5.77, 15.2], &[
        mass([-0.18, -0.09], [0.76, 0.73], 0.35, 0.49, Gable),
        level([-0.25, 0.69], [0.66, 0.23], 0.29, Slab),
        level([0.64, -0.39], [0.23, 0.25], 0.86, Lantern),
        mass([0.64, -0.39], [0.25, 0.27], 0.86, 1.0, Hip),
    ]),
    landmark("Library reading court", [6.78, 5.79, 10.8], &[
        mass([-0.74, 0.0], [0.23, 0.90], 0.60, 0.76, Gable),
        mass([0.74, -0.12], [0.23, 0.81], 0.57, 0.79, Gable),
        level([0.0, -0.68], [0.76, 0.27], 0.67, Slab),
        mass([0.0, -0.65], [0.32, 0.28], 0.67, 1.0, Dome),
    ]),
    landmark("Town hall clock tower", [5.64, 6.22, 19.4], &[
        mass([0.0, -0.18], [0.82, 0.71], 0.36, 0.58, Hip),
        level([0.34, 0.45], [0.27, 0.31], 0.80, Slab),
        level([0.34, 0.45], [0.22, 0.26], 0.88, Lantern),
        mass([0.34, 0.45], [0.28, 0.32], 0.87, 1.0, Hip),
        level([-0.57, 0.58], [0.28, 0.27], 0.27, Slab),
    ]),
    landmark("Corner post office", [5.92, 5.78, 9.0], &[
        level([-0.27, -0.08], [0.66, 0.82], 0.67, Slab),
        level([0.61, 0.02], [0.34, 0.60], 0.82, Slab),
        level([0.55, 0.46], [0.40, 0.46], 0.90, Drum),
        mass([0.55, 0.46], [0.37, 0.43], 0.83, 1.0, Dome),
    ]),
    landmark("Clinic with ambulance court", [6.58, 6.0, 12.6], &[
        level([0.0, -0.63], [0.90, 0.33], 0.73, Slab),
        level([-0.64, -0.24], [0.30, 0.50], 0.79, Slab),
        level([0.60, 0.01], [0.34, 0.65], 0.64, Slab),
        level([0.64, 0.61], [0.32, 0.29], 0.44, Slab),
        level([-0.58, -0.38], [0.22, 0.25], 1.0, Lantern),
    ]),
    landmark("School with gymnasium", [6.82, 6.32, 11.6], &[
        level([-0.53, 0.03], [0.42, 0.84], 0.74, Slab),
        level([0.0, -0.69], [0.84, 0.24], 0.54, Slab),
        mass([0.51, -0.38], [0.42, 0.53], 0.70, 1.0, Gable),
        level([-0.49, -0.28], [0.19, 0.21], 0.87, Lantern),
    ]),
    landmark("Long classroom wings", [6.78, 6.42, 10.4], &[
        mass([-0.46, 0.0], [0.28, 0.86], 0.60, 0.82, Gable),
        mass([0.48, -0.04], [0.29, 0.90], 0.64, 0.89, Gable),
        level([0.0, 0.0], [0.64, 0.23], 0.62, Slab),
        level([-0.45, 0.63], [0.22, 0.22], 1.0, Lantern),
    ]),
    landmark("Trade school workshops", [6.82, 6.25, 11.8], &[
        mass([-0.51, -0.17], [0.43, 0.74], 0.61, 0.91, CrossGable),
        mass([0.40, -0.42], [0.48, 0.51], 0.61, 0.81, CrossGable),
        level([0.58, 0.60], [0.38, 0.27], 0.60, Slab),
        level([-0.43, 0.27], [0.22, 0.21], 1.0, Lantern),
    ]),
    landmark("Community recreation hall", [6.23, 6.26, 12.5], &[
        mass([0.0, -0.13], [0.93, 0.77], 0.56, 1.0, Dome),
        level([0.05, 0.62], [0.53, 0.28], 0.37, Slab),
        level([-0.62, -0.55], [0.31, 0.40], 0.41, Slab),
    ]),
    landmark("Nursery school pavilions", [6.67, 5.87, 8.65], &[
        mass([-0.67, 0.06], [0.30, 0.66], 0.53, 0.92, Hip),
        mass([0.14, -0.53], [0.45, 0.39], 0.60, 1.0, Hip),
        mass([0.60, 0.31], [0.34, 0.52], 0.44, 0.76, Hip),
        level([0.01, -0.14], [0.62, 0.18], 0.31, Slab),
    ]),
    landmark("Covered market court", [6.70, 6.26, 10.6], &[
        mass([0.0, -0.40], [0.86, 0.49], 0.57, 1.0, CrossGable),
        mass([-0.69, 0.34], [0.26, 0.58], 0.39, 0.62, Gable),
        mass([0.69, 0.35], [0.25, 0.57], 0.39, 0.56, Gable),
    ]),
    landmark("Depot with loading canopy", [6.78, 5.78, 10.1], &[
        mass([0.05, -0.18], [0.68, 0.67], 0.52, 0.86, CrossGable),
        level([-0.65, 0.48], [0.31, 0.40], 0.44, Slab),
        level([0.29, 0.60], [0.65, 0.25], 0.38, Slab),
        level([0.56, -0.24], [0.18, 0.21], 1.0, Lantern),
    ]),
    landmark("Drive-in service garage", [6.60, 5.81, 9.25], &[
        level([0.0, -0.59], [0.95, 0.36], 0.68, Slab),
        level([-0.66, -0.02], [0.29, 0.50], 0.64, Slab),
        level([0.63, 0.09], [0.30, 0.59], 0.44, Slab),
        level([-0.72, 0.58], [0.20, 0.20], 1.0, Lantern),
    ]),
    landmark("Cooperative grain store", [6.43, 6.18, 15.6], &[
        mass([0.0, -0.39], [0.90, 0.52], 0.38, 0.62, CrossGable),
        level([-0.68, 0.37], [0.23, 0.27], 0.87, Drum),
        level([-0.16, 0.37], [0.24, 0.28], 1.0, Drum),
        level([0.45, 0.38], [0.38, 0.33], 0.39, Slab),
    ]),
    landmark("Roadside lodge court", [6.75, 6.48, 10.1], &[
        mass([-0.68, 0.0], [0.27, 0.87], 0.59, 0.76, Gable),
        mass([0.64, -0.01], [0.31, 0.88], 0.54, 0.75, Gable),
        mass([0.17, -0.67], [0.79, 0.27], 0.57, 0.76, CrossGable),
        mass([0.68, 0.57], [0.29, 0.32], 0.68, 1.0, Hip),
    ]),
    landmark("Brick pumping station", [5.89, 6.08, 13.8], &[
        mass([-0.20, -0.28], [0.68, 0.61], 0.43, 0.68, Gable),
        level([0.46, 0.44], [0.37, 0.38], 0.36, Slab),
        level([0.68, -0.54], [0.22, 0.25], 1.0, Drum),
        level([-0.48, 0.55], [0.37, 0.31], 0.41, Slab),
    ]),
    landmark("Water tank and pump shed", [6.44, 5.96, 16.2], &[
        level([-0.54, -0.07], [0.39, 0.46], 0.82, Drum),
        mass([-0.54, -0.07], [0.38, 0.43], 0.79, 1.0, Dome),
        mass([0.47, -0.38], [0.45, 0.52], 0.29, 0.50, Gable),
        level([0.65, 0.45], [0.30, 0.41], 0.25, Slab),
    ]),
    landmark("Municipal works depot", [6.82, 6.44, 11.4], &[
        mass([-0.56, -0.21], [0.41, 0.69], 0.53, 0.80, Gable),
        mass([0.45, -0.48], [0.51, 0.43], 0.45, 0.64, CrossGable),
        level([0.57, 0.52], [0.39, 0.36], 0.55, Slab),
        level([-0.77, 0.33], [0.20, 0.21], 1.0, Lantern),
    ]),
    landmark("Local telegraph exchange", [5.74, 5.82, 16.4], &[
        mass([0.18, -0.19], [0.65, 0.69], 0.37, 0.58, Hip),
        level([-0.62, 0.26], [0.30, 0.43], 0.47, Slab),
        level([-0.62, 0.26], [0.19, 0.15], 0.97, Lantern),
        level([-0.62, 0.26], [0.32, 0.09], 1.0, Lantern),
    ]),
    landmark("Electric substation house", [6.44, 6.12, 11.0], &[
        level([-0.50, -0.18], [0.43, 0.71], 0.74, Slab),
        level([0.49, -0.52], [0.34, 0.30], 0.57, Slab),
        level([0.35, 0.23], [0.26, 0.20], 0.43, Drum),
        level([0.69, 0.55], [0.23, 0.19], 0.81, Lantern),
        level([-0.75, -0.55], [0.18, 0.22], 1.0, Lantern),
    ]),
    landmark("Ceramics kiln and drying sheds", [6.73, 6.31, 18.0], &[
        mass([-0.42, -0.12], [0.54, 0.74], 0.36, 0.61, Gable),
        level([0.63, -0.52], [0.27, 0.28], 1.0, Drum),
        mass([0.60, 0.52], [0.33, 0.37], 0.31, 0.46, CrossGable),
    ]),
    landmark("Dairy cold-storage plant", [6.73, 6.04, 12.3], &[
        level([0.0, -0.40], [0.92, 0.55], 0.69, Slab),
        level([-0.61, 0.31], [0.30, 0.32], 0.56, Slab),
        level([0.51, 0.42], [0.24, 0.28], 0.86, Drum),
        level([0.12, 0.44], [0.19, 0.24], 1.0, Drum),
        level([0.66, -0.52], [0.23, 0.23], 0.85, Lantern),
    ]),
    landmark("Three-bay sawmill", [6.82, 6.28, 13.6], &[
        mass([-0.62, -0.04], [0.34, 0.74], 0.48, 0.75, Gable),
        mass([0.0, -0.15], [0.34, 0.78], 0.61, 1.0, Gable),
        mass([0.64, -0.23], [0.33, 0.69], 0.46, 0.70, Gable),
        level([0.65, 0.59], [0.30, 0.27], 0.32, Slab),
    ]),
    landmark("Mill tower and packing hall", [6.79, 6.43, 17.2], &[
        mass([-0.29, -0.38], [0.64, 0.50], 0.42, 0.60, CrossGable),
        level([0.57, -0.32], [0.33, 0.47], 0.91, Slab),
        mass([0.57, -0.32], [0.35, 0.49], 0.91, 1.0, Gable),
        level([-0.57, 0.40], [0.38, 0.39], 0.34, Slab),
    ]),
    landmark("Quarry repair workshop", [6.71, 6.20, 11.9], &[
        mass([-0.43, -0.05], [0.50, 0.86], 0.52, 1.0, Dome),
        level([0.57, -0.39], [0.36, 0.53], 0.72, Slab),
        level([0.66, 0.48], [0.27, 0.39], 0.48, Slab),
        level([-0.39, -0.37], [0.15, 0.18], 0.94, Lantern),
    ]),
];
