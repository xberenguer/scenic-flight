//! Continuous regional climate with 2,500 addressable profiles.

use crate::world::World;

pub const PROFILE_COUNT: u16 = 2500;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Climate {
    heat: f32,
    moisture: f32,
    relief: f32,
    wind: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Conditions {
    pub canopy: f32,
    pub blooms: f32,
    pub exposed_stone: f32,
    pub cloud_bias: f32,
    pub dust: f32,
}

impl Climate {
    /// Landform, coastal influence, and regional weather potential are sampled
    /// independently. The camera altitude does not change the local climate.
    pub fn at(world: &World, x: f32, z: f32) -> Self {
        Self::over_terrain(world, x, z, world.coast_signal(x, z), world.elevation(x, z))
    }

    pub(crate) fn over_terrain(world: &World, x: f32, z: f32, coast: f32, altitude: f32) -> Self {
        let relief = world.noise(x, z, 0.00131, 0xaf02_9e31);
        let wet = world.noise(x, z, 0.00071, 0xbb14_06e7);
        let heat = world.noise(x, z, 0.00051, 0x933a_8f1c);
        let wind = world.noise(x, z, 0.00102, 0xc31d_a821);
        Self {
            heat: (0.61 + heat * 0.87 - altitude * 0.0023).clamp(0.0, 1.0),
            moisture: (0.51 + wet * 0.86 - coast * 0.15).clamp(0.0, 1.0),
            relief: (0.47 + relief * 0.86 + altitude * 0.0014).clamp(0.0, 1.0),
            wind: (0.50 + wind * 0.84 - coast * 0.07).clamp(0.0, 1.0),
        }
    }

    pub fn axes(self) -> [f32; 4] {
        [self.heat, self.moisture, self.relief, self.wind]
    }

    /// Stable atlas identifiers are for reporting. The scene uses continuous axes.
    pub fn id(self) -> u16 {
        let band = |value: f32, count: u16| ((value * f32::from(count)) as u16).min(count - 1);
        ((band(self.heat, 10) * 10 + band(self.moisture, 10)) * 5 + band(self.relief, 5)) * 5
            + band(self.wind, 5)
    }

    pub fn profile(id: u16) -> Option<Self> {
        if id >= PROFILE_COUNT {
            return None;
        }
        Some(Self {
            heat: (f32::from(id / 250) + 0.5) / 10.0,
            moisture: (f32::from(id / 25 % 10) + 0.5) / 10.0,
            relief: (f32::from(id / 5 % 5) + 0.5) / 5.0,
            wind: (f32::from(id % 5) + 0.5) / 5.0,
        })
    }

    pub fn conditions(self) -> Conditions {
        let [heat, wet, relief, wind] = self.axes();
        let growing = crate::math::smoothstep((heat - 0.10) / 0.40);
        let spring = (1.0 - (heat - 0.58).abs() * 1.6).max(0.0);
        Conditions {
            canopy: wet * growing * (1.0 - relief * 0.56) * (1.0 - wind * 0.42),
            blooms: spring * (1.0 - (wet - 0.55).abs() * 1.45) * (1.0 - wind * 0.50),
            exposed_stone: (relief * 0.69 + wind * 0.19 + (1.0 - wet) * 0.12).clamp(0.0, 1.0),
            cloud_bias: wet * 0.46 + relief * 0.17 + (1.0 - heat) * 0.15 - wind * 0.08,
            dust: heat * (1.0 - wet) * (0.27 + 0.73 * wind),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_profile_round_trips_and_changes_physical_conditions() {
        use std::collections::HashSet;
        let mut distinct = HashSet::new();
        for id in 0..PROFILE_COUNT {
            let c = Climate::profile(id).unwrap();
            assert_eq!(c.id(), id);
            let p = c.conditions();
            distinct.insert([p.canopy, p.blooms, p.exposed_stone, p.cloud_bias, p.dust].map(f32::to_bits));
        }
        assert_eq!(distinct.len(), usize::from(PROFILE_COUNT));
        assert!(Climate::profile(PROFILE_COUNT).is_none());
    }

    #[test]
    fn neighbors_blend_and_world_seeds_keep_distinct_regional_climates() {
        let world = World::new(17);
        let other = World::new(18);
        let a = Climate::at(&world, -800.0, 992.0);
        let b = Climate::at(&world, -799.9, 991.9);
        assert!(a.axes().into_iter().zip(b.axes()).all(|(a,b)| (a-b).abs() < 0.01));
        assert_ne!(a, Climate::at(&other, -800.0, 992.0));
    }
}
