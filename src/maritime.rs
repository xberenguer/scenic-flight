//! Seeded anchorages in navigable coastal water.

use core::f32::consts::{FRAC_PI_2, TAU};

use crate::math::{fraction, mix, sin_cos};
use crate::world::{SEA_LEVEL, World};

mod service;

pub const ANCHORAGE_TILE: i32 = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vessel {
    Freighter,
    Ferry,
    Sailboat,
    FishingBoat,
    Tug,
    Service,
}

impl Vessel {
    pub const ALL: [Self; 6] = [
        Self::Freighter, Self::Ferry, Self::Sailboat, Self::FishingBoat, Self::Tug, Self::Service,
    ];
    pub const VARIANTS: u8 = 50;

    pub const fn variants(self) -> u8 {
        match self {
            Self::Service => service::HULLS.len() as u8,
            _ => Self::VARIANTS,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Freighter => "Freighter",
            Self::Ferry => "Ferry",
            Self::Sailboat => "Sailboat",
            Self::FishingBoat => "Fishing boat",
            Self::Tug => "Tug",
            Self::Service => "Service vessel",
        }
    }

    pub const fn configuration(self, variant: u8) -> Option<&'static str> {
        if variant >= self.variants() {
            return None;
        }
        let configurations = match self {
            Self::Freighter => ["Container ship", "Bulk carrier", "Heavy-lift freighter", "Multipurpose freighter", "Coastal freighter"],
            Self::Ferry => ["Car ferry", "Commuter ferry", "Catamaran ferry", "Excursion ferry", "Island ferry"],
            Self::Sailboat => ["Cabin sloop", "Cutter", "Ketch", "Schooner", "Day sailer"],
            Self::FishingBoat => ["Stern trawler", "Crabber", "Longliner", "Purse seiner", "Lobster boat"],
            Self::Tug => ["Harbor tug", "Pusher tug", "Towboat", "Line-handling tug", "Escort tug"],
            Self::Service => return Some(service::HULLS[variant as usize].name),
        };
        Some(configurations[(variant / 5 % 5) as usize])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeckMaterial {
    Hull,
    Deck,
    Bridge,
    Cargo,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipSample {
    pub height: f32,
    pub material: DeckMaterial,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchorage {
    pub x: f32,
    pub z: f32,
    pub vessel: Vessel,
    forward: (f32, f32),
    half_length: f32,
    half_width: f32,
    variant: u8,
}

impl Anchorage {
    pub fn generate(world: &World, seed: u64, tile_x: i32, tile_z: i32) -> Option<Self> {
        let key = mix(
            seed ^ (u64::from(tile_x as u32) << 32) ^ u64::from(tile_z as u32) ^ 0x5ea1_4b91_872d_c8ff,
        );
        let service_mooring = matches!(key & 7, 3 | 4);
        if key & 7 >= 3 && !service_mooring {
            return None;
        }
        let tile_x = tile_x as f32 * ANCHORAGE_TILE as f32;
        let tile_z = tile_z as f32 * ANCHORAGE_TILE as f32;
        for attempt in 0..10 {
            let state = mix(key ^ attempt);
            let x = tile_x + 32.0 + 192.0 * fraction(state);
            let z = tile_z + 32.0 + 192.0 * fraction(mix(state));
            let bay = world.coast_signal(x, z);
            if !(-0.26..=-0.045).contains(&bay) {
                continue;
            }
            let mut land = 0;
            let mut sea = 0;
            let departure = fraction(mix(state ^ 794)) * TAU;
            for i in 0..8 {
                let (sin, cos) = sin_cos(departure + i as f32 * TAU / 8.0);
                let height = world.elevation(x + sin * 94.0, z + cos * 94.0);
                land += u8::from(height >= 2.0);
                sea += u8::from(height == SEA_LEVEL);
            }
            if land < 2 || sea < 2 {
                continue;
            }
            let role = (state >> 14) & 7;
            let vessel = if service_mooring {
                Vessel::Service
            } else if state & 7 == 0 {
                Vessel::Ferry
            } else if bay > -0.19 && land >= 3 && role < 3 {
                Vessel::Tug
            } else if bay > -0.13 && state & 3 <= 1 {
                Vessel::Sailboat
            } else if bay > -0.24 && sea >= 3 && (3..5).contains(&role) {
                Vessel::FishingBoat
            } else {
                Vessel::Freighter
            };
            let variant = (mix(state ^ 192) % u64::from(vessel.variants())) as u8;
            for angle in [departure, departure + FRAC_PI_2] {
                let ship = Self::configured(vessel, variant, x, z, sin_cos(angle));
                if ship.has_water_clearance(world) {
                    return Some(ship);
                }
            }
        }
        None
    }

    /// An isolated hull at the origin, facing along +Z, for inspecting the artwork.
    pub fn catalog(vessel: Vessel, variant: u8) -> Option<Self> {
        (variant < vessel.variants()).then(|| Self::configured(vessel, variant, 0.0, 0.0, (0.0, 1.0)))
    }

    pub fn half_extents(self) -> [f32; 2] {
        [self.half_length, self.half_width]
    }

    pub fn configuration(self) -> &'static str {
        self.vessel.configuration(self.variant).expect("valid vessel configuration")
    }

    fn configured(vessel: Vessel, variant: u8, x: f32, z: f32, forward: (f32, f32)) -> Self {
        if vessel == Vessel::Service {
            let [half_length, half_width] = service::HULLS[usize::from(variant)].size;
            return Self { x, z, vessel, forward, half_length, half_width, variant };
        }
        let size = f32::from(variant % 5) * 0.25;
        let breadth = if variant >= 25 { 1.0 } else { 0.84 };
        let (length, beam) = match vessel {
            Vessel::Freighter => (15.0 + size * 8.0, 0.257),
            Vessel::Ferry => (10.0 + size * 4.0, 0.319),
            Vessel::Sailboat => (4.8 + size * 2.2, 0.385),
            Vessel::FishingBoat => (5.7 + size * 3.2, 0.330),
            Vessel::Tug => (4.4 + size * 2.0, 0.457),
            Vessel::Service => unreachable!(),
        };
        Self { x, z, vessel, forward, half_length: length, half_width: length * beam * breadth, variant }
    }

    fn has_water_clearance(self, world: &World) -> bool {
        let (sin, cos) = self.forward;
        for longitudinal in [-1.20, -0.65, 0.0, 0.65, 1.20] {
            for lateral in [-1.8, 0.0, 1.8] {
                let along = longitudinal * self.half_length;
                let across = lateral * self.half_width;
                let x = self.x + sin * along + cos * across;
                let z = self.z + cos * along - sin * across;
                if world.coast_signal(x, z) > 0.0 {
                    return false;
                }
            }
        }
        true
    }

    pub fn sample(&self, x: f32, z: f32) -> Option<ShipSample> {
        let dx = x - self.x;
        let dz = z - self.z;
        if dx.abs() > self.half_length + self.half_width || dz.abs() > self.half_length + self.half_width {
            return None;
        }
        let (sin, cos) = self.forward;
        let along = (dx * sin + dz * cos) / self.half_length;
        let signed_across = (dx * cos - dz * sin) / self.half_width;
        let across = signed_across.abs();
        let [shoulder, stern, bow] = match self.vessel {
            Vessel::Freighter => [0.48, 1.23, 1.86],
            Vessel::Ferry => [0.57, 1.26, 1.60],
            Vessel::Sailboat => [0.11, 0.69, 1.14],
            Vessel::FishingBoat => [0.43, 0.71, 1.64],
            Vessel::Tug => [0.65, 0.56, 1.64],
            Vessel::Service => service::HULLS[usize::from(self.variant)].profile,
        };
        let beam = 1.0 - (along.abs() - shoulder).max(0.0) * if along > 0.0 { bow } else { stern };
        if along.abs() >= 1.0 || across >= beam {
            return None;
        }
        let layout = self.variant / 5 % 5;
        let (height, material) = match self.vessel {
            Vessel::Freighter => freighter(along, across, beam, layout),
            Vessel::Ferry => ferry(along, across, beam, layout)?,
            Vessel::Sailboat => sailboat(along, across, beam, layout),
            Vessel::FishingBoat => fishing_boat(along, across, beam, layout),
            Vessel::Tug => tug(along, across, beam, layout),
            Vessel::Service => service::HULLS[usize::from(self.variant)].sample(along, signed_across, beam)?,
        };
        Some(ShipSample { height: SEA_LEVEL + height, material })
    }
}

fn freighter(along: f32, across: f32, beam: f32, layout: u8) -> (f32, DeckMaterial) {
    use DeckMaterial::{Bridge, Cargo, Deck, Hull};
    let superstructure = if layout == 4 { 0.56 } else { -0.62 };
    if (along - superstructure).abs() < 0.15 && across < 0.63 {
        return (if across < 0.39 { 5.1 } else { 3.65 }, Bridge);
    }
    if across > beam - 0.10 {
        return (1.35, Hull);
    }
    if !(-0.37..0.62).contains(&along) || across >= 0.74 {
        return (1.6, Deck);
    }
    let bay = (along + 0.37) * 7.5;
    let row = bay as u8;
    let slot = bay - f32::from(row);
    match layout {
        0 if slot < 0.80 => (if row & 1 == 0 { 4.15 } else { 3.35 }, Cargo),
        1 if slot < 0.68 => (2.35, Deck),
        2 if slot < 0.70 && row % 2 == 0 && across < 0.23 => (4.80, Bridge),
        2 if row % 2 == 1 => (2.90, Cargo),
        3 if slot < 0.84 && row % 3 != 0 => (3.65, Cargo),
        4 if slot < 0.77 && along < 0.28 => (2.85, Cargo),
        _ => (1.6, Deck),
    }
}

fn ferry(along: f32, across: f32, beam: f32, layout: u8) -> Option<(f32, DeckMaterial)> {
    use DeckMaterial::{Bridge, Deck, Hull};
    if layout == 2 && across < 0.32 && along.abs() > 0.58 {
        return None; // Water between the two hulls past the cross deck.
    }
    if across > beam - 0.10 {
        return Some((1.35, Hull));
    }
    let (aft, fore, width, height) = match layout {
        0 => (-0.54, 0.54, 0.74, 3.30),
        1 => (-0.70, 0.60, 0.72, 4.40),
        2 => (-0.46, 0.67, 0.66, 3.45),
        3 => (-0.18, 0.74, 0.64, 3.80),
        _ => (-0.69, 0.36, 0.54, 4.30),
    };
    Some(if (aft..fore).contains(&along) && across < width {
        let wheelhouse = along > fore - 0.23 && across < width * 0.70;
        (if wheelhouse { height + 0.62 } else { height }, Bridge)
    } else {
        (1.6, Deck)
    })
}

fn sailboat(along: f32, across: f32, beam: f32, layout: u8) -> (f32, DeckMaterial) {
    use DeckMaterial::{Bridge, Deck, Hull};
    let (mast, cabin_aft, cabin_fore, mast_height) = match layout {
        0 => (0.19, -0.27, 0.24, 4.85),
        1 => (0.27, -0.18, 0.34, 5.0),
        2 => (0.15, -0.31, 0.21, 4.78),
        3 => (0.32, -0.25, 0.38, 4.35),
        _ => (0.38, -0.08, 0.28, 4.2),
    };
    if across < 0.17 {
        if (along - mast).abs() < 0.068 {
            return (mast_height, Bridge);
        }
        if (layout == 2 || layout == 3) && (along + 0.39).abs() < 0.068 {
            return (if layout == 2 { 3.75 } else { 4.9 }, Bridge);
        }
        if (mast - 0.27..mast).contains(&along) && layout < 4 {
            return (2.65, Bridge); // Stowed boom and sail.
        }
    }
    if (cabin_aft..cabin_fore).contains(&along) && across < 0.38 && layout != 4 {
        (2.16, Bridge)
    } else if across > beam - 0.15 {
        (0.84, Hull)
    } else {
        (1.12, Deck)
    }
}

fn fishing_boat(along: f32, across: f32, beam: f32, layout: u8) -> (f32, DeckMaterial) {
    use DeckMaterial::{Bridge, Cargo, Deck, Hull};
    let cabin = if layout == 0 { 0.49 } else { 0.29 };
    if (along - cabin).abs() < 0.14 && across < 0.54 {
        return (if across < 0.21 { 4.15 } else { 3.35 }, Bridge);
    }
    if across > beam - 0.16 {
        return (1.22, Hull);
    }
    match layout {
        0 if (-0.72..-0.55).contains(&along) && across > 0.39 => (3.30, Bridge), // Trawl gantry legs.
        1 if (-0.74..-0.14).contains(&along) && across > 0.27 => (2.62, Cargo), // Crab pots.
        2 if (-0.40..-0.24).contains(&along) && across < 0.50 => (2.25, Deck), // Longline reel.
        3 if (-0.78..-0.26).contains(&along) && across < 0.55 => (2.85, Cargo), // Net well.
        4 if (-0.62..0.05).contains(&along) && across < 0.40 => (1.95, Cargo), // Pot-hauling deck.
        _ => (1.45, Deck),
    }
}

fn tug(along: f32, across: f32, beam: f32, layout: u8) -> (f32, DeckMaterial) {
    use DeckMaterial::{Bridge, Cargo, Deck, Hull};
    let cabin = if layout == 2 { 0.04 } else { 0.24 };
    if (along - cabin).abs() < 0.29 && across < 0.68 {
        let upper = (along - cabin).abs() < 0.18 && across < 0.43;
        return (if upper { 4.15 + 0.18 * f32::from(layout) } else { 2.75 }, Bridge);
    }
    if layout == 1 && along > 0.80 && (0.27..0.54).contains(&across) {
        return (2.4, Hull); // Pushing knees.
    }
    if across > beam - 0.19 {
        return (1.42, Hull);
    }
    if (-0.58..-0.25).contains(&along) && across < 0.31 {
        return (if layout == 2 { 2.55 } else { 1.98 }, Cargo);
    }
    (1.43, Deck)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_layouts_show_role_specific_shapes() {
        let sample = |vessel, variant, along: f32, across: f32| {
            let ship = Anchorage::catalog(vessel, variant).unwrap();
            let [half_length, half_width] = ship.half_extents();
            ship.sample(across * half_width, along * half_length)
        };
        // The ferry's separated bows leave water beneath the centerline.
        assert!(sample(Vessel::Ferry, 10, 0.82, 0.0).is_none());
        assert!(sample(Vessel::Ferry, 10, 0.82, 0.46).is_some());
        assert!(sample(Vessel::Ferry, 0, 0.82, 0.0).is_some());
        // Two masts on the ketch, a single mast on the sloop.
        assert!(sample(Vessel::Sailboat, 10, -0.39, 0.0).unwrap().height > 3.0);
        assert!(sample(Vessel::Sailboat, 0, -0.39, 0.0).unwrap().height < 2.0);
        assert_eq!(sample(Vessel::FishingBoat, 5, -0.39, 0.44).unwrap().material, DeckMaterial::Cargo);
        assert_eq!(sample(Vessel::Tug, 5, 0.86, 0.44).unwrap().material, DeckMaterial::Hull);
        assert_eq!(sample(Vessel::Freighter, 0, -0.32, 0.0).unwrap().material, DeckMaterial::Cargo);
        assert_eq!(sample(Vessel::Freighter, 5, -0.32, 0.0).unwrap().material, DeckMaterial::Deck);
        assert!(sample(Vessel::Service, 4, 0.72, 0.0).is_none());
        assert!(sample(Vessel::Service, 4, 0.72, 0.56).is_some());
        let net_launch = Anchorage::catalog(Vessel::Service, 22).unwrap();
        let width = net_launch.half_extents()[1];
        assert_ne!(net_launch.sample(-0.48 * width, -1.0), net_launch.sample(0.48 * width, -1.0));
    }

    #[test]
    fn catalog_contains_distinct_hulls_for_every_configuration() {
        use std::collections::BTreeSet;

        for vessel in Vessel::ALL {
            let mut signatures = BTreeSet::new();
            for variant in 0..vessel.variants() {
                let ship = Anchorage::catalog(vessel, variant).unwrap();
                let [length, width] = ship.half_extents();
                let mut silhouette = std::vec::Vec::new();
                for z in -40..=40 {
                    for x in -16..=16 {
                        let cell = ship.sample(x as f32 * 0.4, z as f32 * 0.6);
                        silhouette.push(cell.map_or(0, |sample| sample.height.to_bits()
                            ^ (sample.material as u32) << 28));
                    }
                }
                assert!(silhouette.iter().any(|&point| point != 0));
                assert!(signatures.insert(silhouette), "{vessel:?} variant {variant}");
                assert!(length + width < 32.0);
            }
            assert!(Anchorage::catalog(vessel, vessel.variants()).is_none());
        }
    }

    #[test]
    fn anchorages_are_repeatable_coastal_and_clear_of_land() {
        let mut vessels = [0; Vessel::ALL.len()];
        for seed in [1, 14, 31, 827, 3819] {
            let world = World::new(seed);
            for tz in -7..=7 {
                for tx in -7..=7 {
                    let first = Anchorage::generate(&world, seed, tx, tz);
                    let second = Anchorage::generate(&world, seed, tx, tz);
                    assert_eq!(first, second);
                    let Some(ship) = first else { continue };
                    assert!(ship.has_water_clearance(&world));
                    vessels[Vessel::ALL.iter().position(|&kind| kind == ship.vessel).unwrap()] += 1;
                    for dz in -27..=27 {
                        for dx in -27..=27 {
                            let x = ship.x + dx as f32;
                            let z = ship.z + dz as f32;
                            if let Some(sample) = ship.sample(x, z) {
                                assert_eq!(world.elevation(x, z), SEA_LEVEL);
                                assert!((0.8..=5.2).contains(&sample.height));
                            }
                        }
                    }
                }
            }
        }
        assert!(vessels[0] >= 10 && vessels[1] >= 4 && vessels[2] >= 5, "{vessels:?}");
        assert!(vessels[3] >= 5 && vessels[4] >= 3, "{vessels:?}");
        assert!(vessels[5] >= 5, "{vessels:?}");
        assert!(vessels.iter().sum::<i32>() < 170, "{vessels:?}");
    }

    #[test]
    fn every_service_vessel_can_spawn_on_seeded_water() {
        let mut found = [false; service::HULLS.len()];
        'catalog: for seed in 0..192 {
            let world = World::new(seed);
            for tz in -7..=7 {
                for tx in -7..=7 {
                    if let Some(ship) = Anchorage::generate(&world, seed, tx, tz) {
                        if ship.vessel == Vessel::Service {
                            found[usize::from(ship.variant)] = true;
                            if found.iter().all(|&seen| seen) { break 'catalog; }
                        }
                    }
                }
            }
        }
        assert!(found.into_iter().all(|seen| seen), "{found:?}");
    }
}
