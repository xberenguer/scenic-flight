//! A metropolitan airport with a single terrain reservation and shared axes
//! for runway pavement, taxi guidance, gate stands, and vertical obstacles.

use crate::math::smoothstep;
use crate::world::{Sample, Surface};

const EAST_LIMIT: f32 = 1_320.0;
pub(super) const NORTH_LIMIT: f32 = 620.0;
const RUNWAY_LENGTH: f32 = 1_115.0;
const RUNWAYS: [f32; 2] = [-438.0, -224.0];
const TAXIWAYS: [f32; 2] = [-366.0, -152.0];
const EXITS: [f32; 5] = [-1_028.0, -636.0, -174.0, 388.0, 1_028.0];
const GATES: [f32; 8] = [-513.0, -387.0, -261.0, -135.0, 135.0, 261.0, 387.0, 513.0];

#[derive(Clone, Copy)]
pub(super) struct Airport {
    center: [f32; 2],
    terminal: u8,
    parking: u8,
}

fn terminal_roof(style: u8, x: f32, z: f32) -> Option<(Surface, f32)> {
    let a = x.abs();
    if a > 594.0 || !(225.0..=388.0).contains(&z) {
        return None;
    }
    let concourse = z <= 248.0;
    let (depth, ridge, height, material) = match style {
        // Stepped central pavilion with shorter wings.
        0 => (
            if a < 175.0 { 385.0 } else { 307.0 },
            a < 175.0,
            22.0,
            Surface::RailGlass,
        ),
        // Two halls either side of an open arrivals plaza.
        1 => (
            if (a - 322.0).abs() < 161.0 {
                374.0
            } else {
                257.0
            },
            (a - 321.0).abs() < 92.0,
            26.0,
            Surface::LandmarkRoof28,
        ),
        // Tapered central pier with two secondary side halls.
        2 => (
            if a < (408.0 - z * 0.83) {
                388.0
            } else if a > 296.0 {
                309.0
            } else {
                270.0
            },
            a < 92.0,
            21.0,
            Surface::RailMetal,
        ),
        // Long arched hall, represented by three stepped roof bands.
        3 => (
            367.0 - 0.00030 * x * x,
            (z - 292.0).abs() < 21.0 && a < 481.0,
            29.0,
            Surface::RailGlass,
        ),
        // Three independent pavilions sharing one raised concourse.
        _ => (
            if a < 93.0 || (a - 360.0).abs() < 119.0 {
                373.0
            } else {
                253.0
            },
            a < 93.0 || (a - 360.0).abs() < 68.0,
            29.0,
            Surface::LandmarkRoof19,
        ),
    };
    if concourse {
        Some((Surface::RailGlass, 11.0))
    } else if z <= depth {
        Some((
            if ridge { material } else { Surface::FlatRoof },
            if ridge { height } else { height * 0.63 },
        ))
    } else {
        None
    }
}

fn parked_jet(model: u8, side: f32, z: f32) -> Option<(Surface, f32)> {
    let (length, body, span, height, trim) = match model {
        0 => (23.0, 3.2, 18.0, 4.2, Surface::LandmarkRoof28),
        1 => (30.0, 3.8, 23.0, 5.1, Surface::LandmarkRoof25),
        2 => (35.0, 5.0, 28.0, 6.2, Surface::LandmarkRoof34),
        3 => (29.0, 4.1, 27.0, 5.8, Surface::LandmarkRoof09),
        _ => (26.0, 3.1, 21.0, 4.6, Surface::LandmarkRoof23),
    };
    let x = side.abs();
    let longitudinal = z - 140.0;
    if x <= body && longitudinal.abs() <= length {
        return Some((
            if longitudinal > length * 0.66 {
                trim
            } else {
                Surface::RunwayMarking
            },
            height,
        ));
    }
    let wing = (longitudinal + x * 0.44).abs() <= 4.5 && x <= span;
    let tail = (longitudinal - length * 0.71 + x * 0.24).abs() <= 2.6 && x <= span * 0.37;
    (wing || tail).then_some((
        if tail { trim } else { Surface::RunwayMarking },
        height * 0.56,
    ))
}

impl Airport {
    pub(super) fn new(seed: u64) -> Self {
        let east = 4_150.0 + ((seed >> 32) & 63) as f32 * 21.0;
        let north = -4_310.0 - ((seed >> 40) & 31) as f32 * 18.0;
        Self {
            center: [if seed & 4 == 0 { east } else { -east }, north],
            terminal: ((seed >> 56) % 5) as u8,
            parking: ((seed >> 24) % 5) as u8,
        }
    }

    pub(super) fn point(self, local: [f32; 2]) -> [f32; 2] {
        [self.center[0] + local[0], self.center[1] + local[1]]
    }

    pub(super) fn local(self, east: f32, north: f32) -> [f32; 2] {
        [east - self.center[0], north - self.center[1]]
    }

    pub(super) fn grade(self, local: [f32; 2], terrain: f32) -> f32 {
        let [x, z] = local.map(f32::abs);
        let margin = (x - EAST_LIMIT).max(z - NORTH_LIMIT);
        if margin >= 210.0 {
            return terrain;
        }
        let engineered = 20.0 + local[0] * 0.0017 + local[1] * 0.0010;
        let influence = 1.0 - smoothstep(margin / 210.0);
        terrain + (engineered - terrain) * influence
    }

    pub(super) fn contains(local: [f32; 2]) -> bool {
        local[0].abs() <= EAST_LIMIT && local[1].abs() <= NORTH_LIMIT
    }

    pub(super) fn overlay(self, local: [f32; 2], ground: Sample) -> Option<Sample> {
        if !Self::contains(local) {
            return None;
        }
        let ground_cover = |surface| Sample { surface, ..ground };
        let roof = |surface, height| Sample {
            elevation: ground.elevation + height,
            surface,
        };
        let [x, z] = local;
        let ax = x.abs();

        // Two parallel runways, a hold point at every entrance, and two
        // through taxiways connected at the ends and via the central apron.
        for runway in RUNWAYS {
            let cross = (z - runway).abs();
            if cross <= 21.0 && ax <= RUNWAY_LENGTH {
                let from_end = RUNWAY_LENGTH - ax;
                let threshold = (9.0..=23.0).contains(&from_end) && (cross - 4.0) % 4.5 < 2.3;
                let centerline = cross <= 2.2 && ax % 67.0 < 38.0;
                let edge = cross >= 18.3;
                let aiming = (112.0..152.0).contains(&from_end) && (cross - 11.0).abs() < 2.6;
                let touchdown = (207.0..367.0).contains(&from_end)
                    && (from_end - 207.0) % 71.0 < 13.0
                    && (cross - 10.0).abs() < 2.5;
                return Some(ground_cover(
                    if threshold || centerline || edge || aiming || touchdown {
                        Surface::RunwayMarking
                    } else {
                        Surface::Pavement
                    },
                ));
            }
            if cross <= 3.0 && ax <= 1_258.0 && (ax - RUNWAY_LENGTH) % 18.0 < 4.8 {
                return Some(ground_cover(Surface::RunwayLight));
            }
        }
        for taxiway in TAXIWAYS {
            let lateral = (z - taxiway).abs();
            if lateral <= 10.0 && ax <= 1_040.0 {
                return Some(ground_cover(if lateral < 1.5 {
                    Surface::AirfieldGuide
                } else {
                    Surface::Pavement
                }));
            }
        }
        if (RUNWAYS[0]..=-38.0).contains(&z) {
            if let Some(offset) = EXITS
                .into_iter()
                .map(|exit| (x - exit).abs())
                .find(|&offset| offset <= 9.5)
            {
                let hold = RUNWAYS.iter().any(|&runway| {
                    let lead = (z - runway).abs();
                    (lead - 29.0).abs() < 1.5 || (lead - 34.0).abs() < 1.5
                });
                return Some(ground_cover(if hold || offset <= 1.5 {
                    Surface::AirfieldGuide
                } else {
                    Surface::Pavement
                }));
            }
        }

        // The airside concourses sit between the gate apron and the landside
        // forecourt. Individual fingers and bridges leave room for each stand.
        if (105.0..=231.0).contains(&z) && ax <= 550.0 {
            for (i, gate) in GATES.into_iter().enumerate() {
                let side = x - gate;
                if side.abs() > 34.0 {
                    continue;
                }
                let occupied = (i + usize::from(self.parking)) % 4 != 2;
                let model = (i as u8 + self.parking) % 5;
                if occupied {
                    if let Some((surface, height)) = parked_jet(model, side, z) {
                        return Some(roof(surface, height));
                    }
                }
                if side.abs() <= 5.0 && (162.0..=231.0).contains(&z) {
                    return Some(roof(Surface::RailGlass, 7.0));
                }
                let stand_box = (side.abs() - 31.0).abs() < 1.8 && (106.0..=183.0).contains(&z)
                    || side.abs() < 31.0 && (z - 106.0).abs() < 1.7;
                if stand_box {
                    return Some(ground_cover(Surface::RunwayMarking));
                }
            }
        }
        if let Some((surface, height)) = terminal_roof(self.terminal, x, z) {
            return Some(roof(surface, height));
        }
        if (x - 691.0).abs() <= 13.0 && (z - 315.0).abs() <= 13.0 {
            let upper = (x - 691.0).abs().max((z - 315.0).abs()) <= 7.0;
            return Some(roof(Surface::TowerRoof, if upper { 42.0 } else { 33.0 }));
        }
        if (x + 820.0).abs() <= 136.0 && (z - 278.0).abs() <= 58.0 {
            return Some(roof(Surface::IndustryMetal, 16.0));
        }
        if (x - 888.0).abs() <= 142.0 && (z - 225.0).abs() <= 64.0 {
            let seams = (x + 1_000.0) % 61.0 < 8.0;
            return Some(roof(
                Surface::IndustryConcrete,
                if seams { 15.0 } else { 19.0 },
            ));
        }
        if (x - 730.0).abs() <= 31.0 && (z - 427.0).abs() <= 69.0 {
            return Some(roof(Surface::IndustryBrick, 8.5));
        }
        if (x + 652.0).abs() <= 76.0 && (z - 381.0).abs() <= 29.0 {
            return Some(roof(Surface::IndustryMetal, 8.0));
        }

        if ax <= 585.0 && (380.0..397.0).contains(&z) {
            let crossing = ax < 150.0 && (z - 381.0) % 5.0 < 1.5;
            return Some(ground_cover(if crossing {
                Surface::RunwayMarking
            } else {
                Surface::RailApron
            }));
        }
        if (617.0..=1_057.0).contains(&ax) && (161.0..389.0).contains(&z) {
            let cargo_lane = (ax - 660.0).abs() <= 7.0;
            return Some(ground_cover(if cargo_lane {
                Surface::Pavement
            } else {
                Surface::RailApron
            }));
        }
        let perimeter = ax >= EAST_LIMIT - 20.0 || z.abs() >= NORTH_LIMIT - 17.0;
        let arrivals = ax < 668.0 && (397.0..435.0).contains(&z);
        let access = (x.abs() <= 18.0 || (ax - 660.0).abs() <= 12.0) && z >= 397.0;
        if perimeter || arrivals || access {
            return Some(ground_cover(Surface::Pavement));
        }
        if (x.abs() - 341.0).abs() <= 169.0 && (453.0..563.0).contains(&z) {
            let rows = ((z - 453.0) % 26.0 - 13.0).abs();
            return Some(ground_cover(if rows < 1.6 && (x + 756.0) % 11.0 < 6.0 {
                Surface::RunwayMarking
            } else {
                Surface::RailApron
            }));
        }
        if ax <= 1_046.0 && (-53.0..=225.0).contains(&z) {
            let guide_to_stand =
                (43.0..=175.0).contains(&z) && GATES.iter().any(|&gate| (x - gate).abs() <= 2.1);
            let apron_lane = (z - 42.0).abs() <= 2.1 || (z - 8.0).abs() <= 1.8;
            let apron_edge = ax >= 1_040.0;
            return Some(ground_cover(if apron_lane || guide_to_stand {
                Surface::AirfieldGuide
            } else if apron_edge {
                Surface::RunwayMarking
            } else {
                Surface::RailApron
            }));
        }
        Some(ground_cover(if z < 0.0 {
            Surface::GrassRunway
        } else {
            Surface::Park
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_and_aircraft_designs_change_footprints_and_stay_on_the_apron() {
        let mut roofs = [0_u64; 5];
        let mut aircraft = [0_u64; 5];
        for mode in 0..5 {
            for x in (-580..581).step_by(7) {
                for z in (250..389).step_by(9) {
                    let height = terminal_roof(mode, x as f32, z as f32).map_or(0.0, |(_, h)| h);
                    roofs[mode as usize] =
                        roofs[mode as usize].rotate_left(5) ^ u64::from(height.to_bits());
                }
            }
            for x in -31..32 {
                for z in 106..174 {
                    let height = parked_jet(mode, x as f32, z as f32).map_or(0.0, |(_, h)| h);
                    aircraft[mode as usize] =
                        aircraft[mode as usize].rotate_left(5) ^ u64::from(height.to_bits());
                }
            }
            assert!(!roofs[..mode as usize].contains(&roofs[mode as usize]));
            assert!(!aircraft[..mode as usize].contains(&aircraft[mode as usize]));
        }
    }

    #[test]
    fn airport_pavement_links_runway_exits_to_each_apron_and_the_city_entrance() {
        let ground = Sample {
            elevation: 20.0,
            surface: Surface::Grass,
        };
        let airport = Airport::new(0);
        let paved = |x, z| {
            let sample = airport.overlay([x, z], ground).unwrap();
            assert_eq!(sample.elevation, ground.elevation);
            assert!(
                matches!(
                    sample.surface,
                    Surface::Pavement
                        | Surface::RunwayMarking
                        | Surface::AirfieldGuide
                        | Surface::RailApron
                ),
                "gap at ({x}, {z})"
            );
        };
        for runway in RUNWAYS {
            for x in (-1_110..=1_110).step_by(4) {
                paved(x as f32, runway);
            }
            for exit in EXITS {
                for z in (runway as i32..=41).step_by(2) {
                    paved(exit, z as f32);
                }
            }
        }
        for x in -1_000..=1_000 {
            paved(x as f32, 42.0);
        }
        for &gate in &GATES {
            assert!(airport.overlay([gate, 210.0], ground).unwrap().elevation > 20.0);
        }
        for z in 398..=620 {
            paved(0.0, z as f32);
        }
        assert_eq!(
            airport.overlay([691.0, 315.0], ground).unwrap().elevation,
            62.0
        );
    }
}
