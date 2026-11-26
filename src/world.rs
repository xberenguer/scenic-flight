//! Seeded terrain and surface materials in world coordinates.

pub const SEA_LEVEL: f32 = 0.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Surface {
    DeepWater,
    Shallows,
    Sand,
    Grass,
    Forest,
    Rock,
    Snow,
    Desert,
    Pavement,
    Gravel,
    Park,
    TreeCanopy,
    FieldGreen,
    FieldGold,
    FieldSoil,
    PitchedRoof,
    FlatRoof,
    TowerRoof,
    ShipHull,
    ShipDeck,
    ShipCabin,
    ShipCargo,
    HarborCrane,
    CargoOchre,
    CargoBlue,
    RiverWater,
    WetMeadow,
    GrassRunway,
    RunwayMarking,
    RunwayLight,
    TarnWater,
    TarnShallows,
    AlpineMeadow,
    SaltCrust,
    DuneSand,
    ReedMarsh,
    GlacierIce,
    GlacierCrevasse,
    GlacierMoraine,
    GlacierMeltwater,
    SolarGlass,
    Boardwalk,
    RiverSilt,
    ScreeEdge,
    ScreeFine,
    ScreeCoarse,
    DryWash,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub elevation: f32,
    pub surface: Surface,
}

#[derive(Clone, Copy, Debug)]
pub struct World {
    key: [u32; 2],
}

impl World {
    pub const fn new(seed: u64) -> Self {
        let mut key = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        key = (key ^ (key >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        key = (key ^ (key >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        key ^= key >> 31;
        Self {
            key: [key as u32, (key >> 32) as u32],
        }
    }

    /// Height of the visible terrain or water surface.
    pub fn elevation(&self, x: f32, z: f32) -> f32 {
        self.elevation_variant::<true>(x, z)
    }

    pub(crate) fn signed_horizon_height(&self, x: f32, z: f32) -> f32 {
        let coast = self.coast(x, z);
        if coast <= 0.0 {
            coast * 19.0
        } else {
            self.land_height::<true>(x, z, coast)
        }
    }

    /// Fixed terrain signal for selecting regional layouts. Collisions and
    /// every placement check still use the shaped terrain.
    pub(crate) fn regional_pattern_height(&self, x: f32, z: f32) -> f32 {
        self.elevation_variant::<false>(x, z)
    }

    fn elevation_variant<const SHAPED: bool>(&self, x: f32, z: f32) -> f32 {
        let coast = self.coast(x, z);
        if coast <= 0.0 {
            SEA_LEVEL
        } else {
            SEA_LEVEL + self.land_height::<SHAPED>(x, z, coast)
        }
    }

    pub fn sample(&self, x: f32, z: f32) -> Sample {
        self.sample_and_shore(x, z).0
    }

    pub(crate) fn sample_and_shore(&self, x: f32, z: f32) -> (Sample, f32) {
        let coast = self.coast(x, z);
        if coast <= 0.0 {
            let surface = if coast < -0.105 {
                Surface::DeepWater
            } else {
                Surface::Shallows
            };
            return (Sample { elevation: SEA_LEVEL, surface }, coast);
        }

        let height = self.land_height::<true>(x, z, coast);
        #[cfg(feature = "regional-climate")]
        let region = crate::climate::Climate::over_terrain(self, x, z, coast, height);
        let surface = if height < 2.5 {
            Surface::Sand
        } else if height >= 88.0 {
            let climate = self.noise(x, z, 0.0018, 0x58fe_b14d);
            let snow_line = 129.0 + climate * 24.0;
            #[cfg(feature = "regional-climate")]
            let snow_line = snow_line + (region.axes()[0] - 0.45) * 19.0;
            if height > snow_line {
                Surface::Snow
            } else {
                Surface::Rock
            }
        } else {
            let regional = self.noise(x, z, 0.0019, 0xca34_7b59);
            let local_moisture = self.noise(z, -x, 0.0063, 0x8cb9_2e21);
            let moisture = regional * 0.76 + local_moisture * 0.24;
            #[cfg(feature = "regional-climate")]
            let moisture = {
                let [heat, wet, relief, wind] = region.axes();
                moisture + (wet - 0.51) * 0.31 - (heat - 0.5) * 0.16
                    - relief * wind * 0.07
            };
            if moisture < -0.31 && height < 64.0 && coast > 0.12 {
                Surface::Desert
            } else if moisture > 0.16 && height < 76.0 {
                Surface::Forest
            } else {
                Surface::Grass
            }
        };
        (Sample { elevation: SEA_LEVEL + height, surface }, coast)
    }

    /// A signed, continuous coast field for route planning. Land is positive.
    pub(crate) fn coast_signal(&self, x: f32, z: f32) -> f32 {
        self.coast(x, z)
    }

    // The sign of this field alone decides whether the ground is dry.
    // Inlets shape the continental edge while relief grows inland.
    fn coast(&self, x: f32, z: f32) -> f32 {
        let u = x * 0.8 + z * 0.6;
        let v = z * 0.8 - x * 0.6;
        let continent = self.noise(x, z, 0.00135, 0x730d_31db);
        let shelf = self.noise(u, v, 0.0039, 0x3f91_8b27);
        let inlet = self.noise(u, v, 0.0127, 0x982d_74f1);
        0.11 + continent * 0.79 + shelf * 0.23 + inlet * 0.058
    }

    fn land_height<const SHAPED: bool>(&self, x: f32, z: f32, coast: f32) -> f32 {
        let rise = coast * (18.0 + 12.0 * coast);
        let foothills = smooth_range(0.035, 0.37, coast);
        if foothills == 0.0 {
            return rise;
        }

        let roll = self.noise(x, z, 0.0095, 0xf037_eb15);
        let hills = rise + foothills * (10.0 + 8.0 * roll);
        let inland = smooth_range(0.17, 0.63, coast);
        if inland == 0.0 {
            return hills;
        }

        let u = x * 0.8 - z * 0.6;
        let v = x * 0.6 + z * 0.8;
        let belt = self.noise(u, v, 0.0026, 0x17da_09b5);
        let mountain = inland * smooth_range(-0.28, 0.57, belt);
        if mountain == 0.0 {
            return hills;
        }

        let fold = self.noise(u, v, 0.0087, 0x491e_8a39);
        let crest = 1.0 - fold * fold;
        let scale = 100.0 + 11.0 * roll;
        let base = hills + mountain * (23.0 + scale * crest * crest * crest);
        if !SHAPED || base <= 68.0 {
            return base;
        }
        let relief = smooth_range(68.0, 110.0, base);
        let distance = (fold + 0.08 * roll).abs().min(1.0);
        // Round only the top of each ridge, retaining a clear shoulder.
        let shoulder = if distance < 0.16 {
            0.08 + distance * distance * 3.125
        } else {
            distance
        };
        let ridge = 1.0 - shoulder;
        let summit = hills + mountain * (37.0 + scale * ridge * (0.52 + 0.48 * ridge));
        let face = self.noise(
            u * 1.20 + v * 0.18,
            v * 0.72 - u * 0.10,
            0.0117,
            0x5af2_9037,
        );
        let slope = fold.abs();
        let valley_flank =
            smooth_range(0.10, 0.29, slope) * (1.0 - smooth_range(0.61, 0.85, slope));
        let cirque = 1.0 - smooth_range(-0.44, 0.50, face + fold * 0.16);
        let incision = mountain * (2.0 + 13.0 * cirque * cirque) * valley_flank;
        let basin = self.alpine_basin(u, v);
        let shaped = base + relief * (0.58 * (summit - base) - incision - basin);
        #[cfg(feature = "regional-climate")]
        let shaped = {
            // Stony dry hillsides retain hard ribs between narrow channels.
            // Confine incision to the flank so it connects to the existing valleys.
            let moisture = self.noise(x, z, 0.00071, 0xbb14_06e7);
            let dry = 1.0 - smooth_range(-0.46, 0.05, moisture);
            let fluting = self.noise(u * 1.54 + v * 0.17, v * 0.62, 0.028, 0x4ab2_e5c9);
            let gullies = 1.0 - smooth_range(0.07, 0.58, fluting.abs());
            shaped - relief * mountain * valley_flank * dry * gullies * 4.5
        };
        shaped.max(68.0 + (base - 68.0) * 0.23)
    }

    // In some massifs, an elongated glacial trough cuts a broad floor.
    // The largest outline fits inside its seed cell, so seams cannot cut it.
    fn alpine_basin(&self, u: f32, v: f32) -> f32 {
        const CELL: f32 = 384.0;
        let a = u / CELL;
        let b = v / CELL;
        let ix = lattice_floor(a);
        let iz = lattice_floor(b);
        let design = lattice(ix, iz, self.key[0] ^ 0x14d9_87ba);
        if design < 0.30 {
            return 0.0;
        }
        let cross = a - ix as f32 - 0.49;
        let along = b - iz as f32 - 0.50;
        let tilt = (design - 0.63) * 0.59;
        let waist = (cross - along * tilt) / (0.18 + design * 0.047);
        let reach = along / (0.30 + design * 0.085);
        let edge = waist * waist + reach * reach;
        let bowl = 1.0 - smooth_range(0.13, 1.0, edge);
        (14.0 + 13.0 * design) * bowl
    }

    pub(crate) fn noise(&self, x: f32, z: f32, scale: f32, lane: u32) -> f32 {
        let x = x * scale;
        let z = z * scale;
        let ix = lattice_floor(x);
        let iz = lattice_floor(z);
        let tx = ease((x - ix as f32).clamp(0.0, 1.0));
        let tz = ease((z - iz as f32).clamp(0.0, 1.0));
        let next_x = ix.wrapping_add(1);
        let next_z = iz.wrapping_add(1);
        let key = self.key[(lane >> 31) as usize] ^ lane;
        let lower = mix(lattice(ix, iz, key), lattice(next_x, iz, key), tx);
        let upper = mix(lattice(ix, next_z, key), lattice(next_x, next_z, key), tx);
        mix(lower, upper, tz)
    }
}

fn lattice_floor(value: f32) -> i32 {
    let integer = value as i32;
    if value < integer as f32 {
        integer.saturating_sub(1)
    } else {
        integer
    }
}

fn lattice(x: i32, z: i32, key: u32) -> f32 {
    let mut bits =
        key ^ (x as u32).wrapping_mul(0x9e37_79b1) ^ (z as u32).wrapping_mul(0x85eb_ca77);
    bits = (bits ^ (bits >> 16)).wrapping_mul(0x7feb_352d);
    bits = (bits ^ (bits >> 15)).wrapping_mul(0x846c_a68b);
    bits ^= bits >> 16;
    (bits >> 8) as f32 * (2.0 / 16_777_215.0) - 1.0
}

fn mix(a: f32, b: f32, weight: f32) -> f32 {
    a + (b - a) * weight
}

fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn smooth_range(low: f32, high: f32, value: f32) -> f32 {
    ease(((value - low) / (high - low)).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_and_subcell_positions_are_repeatable() {
        let a = World::new(31);
        let b = World::new(31);
        let c = World::new(31 | (1_u64 << 52));
        let mut different = 0;
        for index in -64..64 {
            let x = index as f32 * 17.25 + 0.375;
            let z = 233.5 - index as f32 * 11.75;
            let sample = a.sample(x, z);
            assert_eq!(sample, b.sample(x, z));
            assert_eq!(sample.elevation, a.elevation(x, z));
            different += usize::from(sample != c.sample(x, z));
        }
        assert!(different > 80);
    }

    #[test]
    fn lattice_and_world_heights_are_continuous_across_zero() {
        for boundary in -8..=8 {
            let world = World::new(2);
            let x = boundary as f32;
            assert_eq!(lattice_floor(x), boundary);
            assert_eq!(lattice_floor(x - 0.001), boundary - 1);
            let left = world.noise(x - 0.001, -34.25, 1.0, 27);
            let right = world.noise(x + 0.001, -34.25, 1.0, 27);
            assert!((left - right).abs() < 0.0001);
        }

        for seed in [2, 119, 8745] {
            let world = World::new(seed);
            for z in (-1600..1600).step_by(23) {
                let z = z as f32 + 0.37;
                assert!((world.elevation(-0.01, z) - world.elevation(0.01, z)).abs() < 0.15);
                assert!((world.elevation(z, -0.01) - world.elevation(z, 0.01)).abs() < 0.15);
            }
        }
    }

    #[test]
    fn new_alpine_relief_preserves_the_lowland_profile() {
        let world = World::new(31);
        for z in (-1700..=1700).step_by(31) {
            for x in (-1700..=1700).step_by(31) {
                let x = x as f32;
                let z = z as f32;
                let original = world.regional_pattern_height(x, z);
                let shaped = world.elevation(x, z);
                if original <= 68.0 {
                    assert_eq!(shaped, original);
                } else {
                    assert!(shaped > 68.0);
                }
            }
        }
        let ridge = world.regional_pattern_height(190.0, 624.0);
        let trough = world.elevation(190.0, 624.0);
        assert!(ridge - trough > 20.0);
    }

    #[test]
    fn landscape_has_all_surface_bands_and_bounded_grades() {
        let mut counts = [0_usize; 8];
        for seed in [1, 7, 31] {
            let world = World::new(seed);
            for z in (-2800..=2800).step_by(37) {
                for x in (-2800..=2800).step_by(37) {
                    let x = x as f32 + 0.125;
                    let z = z as f32 + 0.625;
                    let sample = world.sample(x, z);
                    let h = sample.elevation;
                    assert!((0.0..=200.0).contains(&h));
                    assert!((world.elevation(x + 1.0, z) - h).abs() < 4.2);
                    assert!((world.elevation(x, z + 1.0) - h).abs() < 4.2);
                    if matches!(sample.surface, Surface::DeepWater | Surface::Shallows) {
                        assert_eq!(h, SEA_LEVEL);
                    }
                    if sample.surface == Surface::Snow {
                        assert!(h > 104.0);
                    }
                    counts[sample.surface as usize] += 1;
                }
            }
        }
        assert!(counts.iter().all(|&count| count > 35), "{counts:?}");
    }
}
