//! Seeded anchorages in navigable coastal water.

use core::f32::consts::{FRAC_PI_2, TAU};

use crate::math::{fraction, mix, sin_cos};
use crate::world::{SEA_LEVEL, Surface, World};

pub const ANCHORAGE_TILE: i32 = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vessel {
    Freighter,
    Ferry,
    Sailboat,
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
    cargo_pattern: u8,
}

impl Anchorage {
    pub fn generate(world: &World, seed: u64, tile_x: i32, tile_z: i32) -> Option<Self> {
        let key = mix(
            seed ^ (u64::from(tile_x as u32) << 32) ^ u64::from(tile_z as u32) ^ 0x5ea1_4b91_872d_c8ff,
        );
        if key & 7 >= 3 {
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
            let vessel = if state & 7 == 0 {
                Vessel::Ferry
            } else if bay > -0.13 && state & 3 <= 1 {
                Vessel::Sailboat
            } else {
                Vessel::Freighter
            };
            let size = fraction(mix(state ^ 192));
            let length = match vessel {
                Vessel::Freighter => 15.0 + size * 8.0,
                Vessel::Ferry => 10.0 + size * 4.0,
                Vessel::Sailboat => 4.8 + size * 2.2,
            };
            let width = match vessel {
                Vessel::Freighter => length * 0.234,
                Vessel::Ferry => length * 0.29,
                Vessel::Sailboat => length * 0.35,
            };
            for angle in [departure, departure + FRAC_PI_2] {
                let forward = sin_cos(angle);
                let ship = Self {
                    x, z, vessel, forward, half_length: length, half_width: width,
                    cargo_pattern: (state >> 8) as u8,
                };
                if ship.has_water_clearance(world) {
                    return Some(ship);
                }
            }
        }
        None
    }

    fn has_water_clearance(self, world: &World) -> bool {
        let (sin, cos) = self.forward;
        for longitudinal in [-1.20, -0.65, 0.0, 0.65, 1.20] {
            for lateral in [-1.8, 0.0, 1.8] {
                let along = longitudinal * self.half_length;
                let across = lateral * self.half_width;
                let x = self.x + sin * along + cos * across;
                let z = self.z + cos * along - sin * across;
                if !matches!(world.sample(x, z).surface, Surface::DeepWater | Surface::Shallows) {
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
        let across = (dx * cos - dz * sin).abs() / self.half_width;
        let beam = (1.0 - (along.abs() - 0.48).max(0.0) * (if along > 0.0 { 1.86 } else { 1.23 })).max(0.0);
        if along.abs() >= 1.0 || across >= beam {
            return None;
        }
        let (height, material) = if self.vessel == Vessel::Sailboat {
            if (-0.14..=0.25).contains(&along) && across < 0.37 {
                (2.3, DeckMaterial::Bridge)
            } else if across > beam - 0.16 {
                (0.84, DeckMaterial::Hull)
            } else {
                (1.12, DeckMaterial::Deck)
            }
        } else if across > beam - 0.10 {
            (1.35, DeckMaterial::Hull)
        } else {
            match self.vessel {
                Vessel::Freighter if (-0.74..-0.46).contains(&along) && across < 0.65 => {
                    let bridge = if across < 0.39 { 5.1 } else { 3.65 };
                    (bridge, DeckMaterial::Bridge)
                }
                Vessel::Freighter if (-0.35..0.58).contains(&along) && across < 0.74 => {
                    let bay = ((along + 0.35) * 7.5) as u8;
                    let slot = ((along + 0.35) * 7.5) - f32::from(bay);
                    if slot < 0.85 && self.cargo_pattern.wrapping_add(17 << (bay & 3)) & 7 > 1 {
                        (2.8 + f32::from((self.cargo_pattern >> (bay & 3)) & 1) * 0.65, DeckMaterial::Cargo)
                    } else {
                        (1.6, DeckMaterial::Deck)
                    }
                }
                Vessel::Ferry if along.abs() < 0.63 && across < 0.69 => {
                    let upper = if along > 0.20 && across < 0.46 { 4.65 } else { 3.05 };
                    (upper, DeckMaterial::Bridge)
                }
                _ => (1.6, DeckMaterial::Deck),
            }
        };
        Some(ShipSample { height: SEA_LEVEL + height, material })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchorages_are_repeatable_coastal_and_clear_of_land() {
        let mut vessels = [0; 3];
        for seed in [1, 14, 31, 827, 3819] {
            let world = World::new(seed);
            for tz in -7..=7 {
                for tx in -7..=7 {
                    let first = Anchorage::generate(&world, seed, tx, tz);
                    let second = Anchorage::generate(&world, seed, tx, tz);
                    assert_eq!(first, second);
                    let Some(ship) = first else { continue };
                    assert!(ship.has_water_clearance(&world));
                    vessels[match ship.vessel {
                        Vessel::Freighter => 0,
                        Vessel::Ferry => 1,
                        Vessel::Sailboat => 2,
                    }] += 1;
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
        assert!(vessels.iter().sum::<i32>() < 170, "{vessels:?}");
    }
}
