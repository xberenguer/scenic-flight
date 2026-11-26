//! Waterfront workshops fitted inside surveyed harbor lots.

use crate::world::Surface;

#[derive(Clone, Copy)]
pub(crate) enum Roof {
    GableAcross,
    GableAlong,
    Barrel,
    Shed,
    Flat,
    Tower,
}

use Roof::*;

#[derive(Clone, Copy)]
pub(crate) struct RoofMass {
    pub center: [f32; 2],
    pub half: [f32; 2],
    pub top: f32,
    roof: Roof,
}

#[derive(Clone, Copy)]
pub struct WorkshopProfile {
    pub name: &'static str,
    pub(crate) commercial: bool,
    pub(crate) masses: &'static [RoofMass],
}

const fn mass(x: f32, z: f32, width: f32, depth: f32, top: f32, roof: Roof) -> RoofMass {
    RoofMass {
        center: [x, z],
        half: [width, depth],
        top,
        roof,
    }
}

const fn profile(
    name: &'static str,
    commercial: bool,
    masses: &'static [RoofMass],
) -> WorkshopProfile {
    WorkshopProfile {
        name,
        commercial,
        masses,
    }
}

impl WorkshopProfile {
    pub const ALL: [Self; 25] = [
        profile(
            "Slipway boatworks",
            false,
            &[
                mass(-0.18, -0.14, 0.78, 0.74, 0.9, Barrel),
                mass(0.53, 0.5, 0.41, 0.38, 0.6, Shed),
                mass(-0.45, 0.56, 0.43, 0.36, 0.47, Flat),
            ],
        ),
        profile(
            "Twin fish sheds",
            false,
            &[
                mass(-0.57, -0.02, 0.39, 0.85, 0.94, GableAcross),
                mass(0.54, -0.12, 0.38, 0.75, 0.74, GableAcross),
                mass(0.0, -0.58, 0.51, 0.37, 0.54, Flat),
            ],
        ),
        profile(
            "Harbor pilot house",
            true,
            &[
                mass(-0.12, -0.12, 0.86, 0.76, 0.63, Flat),
                mass(-0.49, -0.41, 0.43, 0.42, 1.0, Tower),
                mass(0.48, 0.42, 0.49, 0.4, 0.43, Shed),
            ],
        ),
        profile(
            "Open fish market",
            true,
            &[
                mass(0.0, -0.55, 0.93, 0.39, 0.95, GableAlong),
                mass(-0.62, 0.21, 0.32, 0.58, 0.7, GableAcross),
                mass(0.62, 0.21, 0.32, 0.58, 0.7, GableAcross),
            ],
        ),
        profile(
            "Sailmaker's loft",
            false,
            &[
                mass(-0.31, -0.12, 0.6, 0.83, 0.95, GableAcross),
                mass(0.46, 0.31, 0.49, 0.6, 0.57, Shed),
                mass(-0.45, -0.68, 0.35, 0.29, 1.0, Tower),
            ],
        ),
        profile(
            "Horseshoe chandlery",
            true,
            &[
                mass(0.0, -0.61, 0.95, 0.32, 0.91, GableAlong),
                mass(-0.64, 0.13, 0.31, 0.6, 0.76, GableAcross),
                mass(0.64, 0.13, 0.31, 0.6, 0.59, Flat),
                mass(0.53, 0.64, 0.38, 0.29, 0.44, Shed),
            ],
        ),
        profile(
            "Net mending sheds",
            false,
            &[
                mass(-0.62, 0.0, 0.32, 0.84, 0.65, Shed),
                mass(-0.07, -0.12, 0.33, 0.72, 0.81, Barrel),
                mass(0.56, -0.22, 0.38, 0.67, 0.96, GableAcross),
                mass(0.5, 0.57, 0.4, 0.36, 0.47, Flat),
            ],
        ),
        profile(
            "Dock ledger office",
            true,
            &[
                mass(-0.18, 0.0, 0.74, 0.81, 0.79, GableAcross),
                mass(0.57, -0.43, 0.38, 0.44, 1.0, Tower),
                mass(0.48, 0.52, 0.46, 0.42, 0.44, Shed),
            ],
        ),
        profile(
            "Tidal pump house",
            false,
            &[
                mass(-0.32, -0.26, 0.62, 0.65, 0.93, Barrel),
                mass(0.52, -0.3, 0.44, 0.57, 0.71, GableAlong),
                mass(0.23, 0.53, 0.55, 0.43, 0.47, Flat),
            ],
        ),
        profile(
            "Ice merchant",
            false,
            &[
                mass(0.0, 0.0, 0.92, 0.85, 0.76, Flat),
                mass(-0.58, -0.51, 0.32, 0.34, 1.0, Tower),
                mass(0.5, 0.49, 0.41, 0.41, 0.54, Shed),
            ],
        ),
        profile(
            "Quay bakery",
            true,
            &[
                mass(-0.4, -0.22, 0.53, 0.71, 0.91, GableAlong),
                mass(0.41, 0.31, 0.52, 0.54, 0.7, GableAcross),
                mass(0.62, -0.44, 0.32, 0.34, 1.0, Tower),
            ],
        ),
        profile(
            "Covered ropewalk",
            false,
            &[
                mass(0.0, -0.57, 0.93, 0.36, 0.82, GableAlong),
                mass(-0.57, 0.05, 0.36, 0.62, 0.95, GableAcross),
                mass(-0.38, 0.62, 0.55, 0.33, 0.5, Shed),
            ],
        ),
        profile(
            "Coastal mail office",
            true,
            &[
                mass(-0.24, -0.19, 0.7, 0.74, 0.69, Flat),
                mass(0.44, -0.4, 0.44, 0.52, 1.0, Tower),
                mass(0.54, 0.5, 0.42, 0.43, 0.47, Shed),
            ],
        ),
        profile(
            "Lifeboat station",
            false,
            &[
                mass(0.16, -0.05, 0.71, 0.89, 0.98, Barrel),
                mass(-0.61, -0.39, 0.33, 0.51, 0.62, GableAcross),
                mass(-0.48, 0.52, 0.42, 0.39, 0.48, Shed),
            ],
        ),
        profile(
            "Open ticket court",
            true,
            &[
                mass(-0.59, -0.22, 0.35, 0.69, 0.96, GableAcross),
                mass(0.62, -0.12, 0.3, 0.78, 0.77, GableAcross),
                mass(0.0, -0.54, 0.66, 0.37, 0.58, GableAlong),
                mass(0.56, 0.6, 0.36, 0.33, 0.44, Flat),
            ],
        ),
        profile(
            "Harbor customs annex",
            true,
            &[
                mass(-0.19, -0.2, 0.75, 0.74, 0.71, GableAlong),
                mass(-0.52, -0.49, 0.42, 0.43, 1.0, Tower),
                mass(0.54, 0.39, 0.4, 0.53, 0.66, GableAcross),
            ],
        ),
        profile(
            "Warehouse staircase",
            false,
            &[
                mass(-0.69, -0.13, 0.26, 0.76, 0.55, Flat),
                mass(-0.22, -0.09, 0.31, 0.81, 0.68, Flat),
                mass(0.28, -0.12, 0.32, 0.8, 0.84, Flat),
                mass(0.7, -0.26, 0.26, 0.68, 1.0, Tower),
            ],
        ),
        profile(
            "Crane service shop",
            false,
            &[
                mass(-0.19, 0.0, 0.75, 0.83, 0.83, GableAlong),
                mass(0.57, -0.49, 0.36, 0.43, 1.0, Tower),
                mass(0.53, 0.44, 0.4, 0.47, 0.51, Shed),
            ],
        ),
        profile(
            "Coastal chandlery row",
            true,
            &[
                mass(-0.64, -0.35, 0.29, 0.59, 0.92, GableAcross),
                mass(-0.13, -0.12, 0.3, 0.74, 0.77, GableAcross),
                mass(0.44, 0.11, 0.43, 0.75, 0.67, GableAcross),
                mass(0.58, -0.57, 0.35, 0.36, 0.51, Flat),
            ],
        ),
        profile(
            "Harbor cafe terrace",
            true,
            &[
                mass(-0.37, -0.3, 0.56, 0.63, 0.86, GableAlong),
                mass(0.51, -0.35, 0.43, 0.58, 0.74, Flat),
                mass(0.0, 0.52, 0.73, 0.4, 0.38, Shed),
            ],
        ),
        profile(
            "Boatyard paint hall",
            false,
            &[
                mass(0.0, -0.29, 0.89, 0.65, 0.96, Barrel),
                mass(-0.53, 0.51, 0.41, 0.38, 0.54, Shed),
                mass(0.56, 0.53, 0.37, 0.42, 0.62, Flat),
            ],
        ),
        profile(
            "Survey office terrace",
            true,
            &[
                mass(-0.38, -0.22, 0.55, 0.71, 0.76, GableAcross),
                mass(0.5, -0.49, 0.42, 0.45, 1.0, Tower),
                mass(0.43, 0.43, 0.46, 0.51, 0.51, Flat),
            ],
        ),
        profile(
            "Ribbed smokehouse",
            false,
            &[
                mass(-0.63, -0.23, 0.31, 0.7, 0.86, GableAlong),
                mass(-0.04, -0.13, 0.34, 0.78, 0.99, Barrel),
                mass(0.59, -0.22, 0.34, 0.72, 0.78, GableAlong),
                mass(-0.46, 0.59, 0.46, 0.34, 0.48, Flat),
            ],
        ),
        profile(
            "Clock landing",
            true,
            &[
                mass(-0.54, -0.21, 0.4, 0.7, 0.7, GableAlong),
                mass(0.43, -0.1, 0.5, 0.78, 0.58, GableAcross),
                mass(0.61, -0.51, 0.29, 0.31, 1.0, Tower),
                mass(-0.48, 0.56, 0.41, 0.38, 0.49, Shed),
            ],
        ),
        profile(
            "Quay repair cloister",
            false,
            &[
                mass(-0.65, -0.01, 0.3, 0.88, 0.96, GableAcross),
                mass(0.0, -0.56, 0.65, 0.38, 0.83, GableAlong),
                mass(0.63, -0.29, 0.32, 0.58, 0.72, GableAcross),
            ],
        ),
    ];

    /// Height relative to the workshop's overall rise, within its normalized lot.
    pub fn sample(self, x: f32, z: f32) -> Option<(f32, Surface)> {
        let mut found = None;
        for mass in self.masses {
            let u = (x - mass.center[0]) / mass.half[0];
            let v = (z - mass.center[1]) / mass.half[1];
            if u.abs() > 1.0 || v.abs() > 1.0 {
                continue;
            }
            let (scale, material) = match mass.roof {
                GableAcross => (1.0 - u.abs() * 0.27, Surface::PitchedRoof),
                GableAlong => (1.0 - v.abs() * 0.27, Surface::PitchedRoof),
                Barrel => (1.0 - u * u * 0.28, Surface::PitchedRoof),
                Shed => (0.89 + u * 0.11, Surface::PitchedRoof),
                Flat => (1.0, Surface::FlatRoof),
                Tower => (1.0, Surface::TowerRoof),
            };
            let height = mass.top * scale;
            if found.is_none_or(|(old, _)| height > old) {
                found = Some((height, material));
            }
        }
        found
    }
}
