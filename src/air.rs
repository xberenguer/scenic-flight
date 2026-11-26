//! Local rising and sinking air linked to terrain, sun and wind.

use core::f32::consts::TAU;

use crate::math::{fraction, mix, sin_cos, smoothstep};
use crate::world::{SEA_LEVEL, Surface, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lift {
    Calm,
    Thermal,
    Ridge,
    LeeWave,
    Convergence,
}

#[derive(Clone, Copy, Debug)]
pub struct Air {
    pub lift: Lift,
    pub upward: f32,
    pub thermal_center: Option<(f32, f32)>,
}

impl Air {
    pub const CALM: Self = Self { lift: Lift::Calm, upward: 0.0, thermal_center: None };

    pub fn at(world: &World, seed: u64, x: f32, z: f32, altitude: f32, daylight: f32) -> Self {
        let terrain = world.elevation(x, z).max(SEA_LEVEL);
        let above = (altitude - terrain).max(0.0);
        let ([wind_x, wind_z], wind) = wind_field(seed);
        let height_at = |along: f32| world.elevation(x + along * wind_x, z + along * wind_z);
        let upstream = height_at(-27.0);
        let downstream = height_at(27.0);
        let ridge = ridge_updraft(upstream, downstream, wind, above);

        let far_upstream = height_at(-118.0);
        let lee_shape = smoothstep((far_upstream - terrain - 19.0) / 46.0)
            * smoothstep((above - 18.0) / 32.0)
            * (1.0 - smoothstep((above - 105.0) / 102.0));
        let phase = (x * wind_x + z * wind_z) * 0.044 + fraction(mix(seed ^ 271)) * TAU;
        let lee_wave = sin_cos(phase).0 * 3.8 * lee_shape;

        let shore = world.coast_signal(x, z);
        let inland = world.coast_signal(x + wind_x * 49.0, z + wind_z * 49.0);
        let convergence = daylight * 2.8 * smoothstep((inland - shore - 0.12) * 3.3)
            * (1.0 - smoothstep((shore.abs() - 0.03) / 0.078))
            * (1.0 - smoothstep((above - 38.0) / 60.0));

        let thermal = warm_thermal(world, seed, x, z, above, daylight);
        let mut strongest = (0.42, Lift::Calm);
        for candidate in [
            (ridge, Lift::Ridge),
            (lee_wave, Lift::LeeWave),
            (convergence, Lift::Convergence),
            (thermal.map_or(0.0, |t| t.0), Lift::Thermal),
        ] {
            if candidate.0 > strongest.0 {
                strongest = candidate;
            }
        }
        Self {
            lift: strongest.1,
            upward: (ridge + lee_wave + convergence + thermal.map_or(0.0, |t| t.0)).clamp(-5.0, 9.5),
            thermal_center: thermal.map(|(_, center)| center),
        }
    }
}

pub(crate) fn wind_field(seed: u64) -> ([f32; 2], f32) {
    let (x, z) = sin_cos(fraction(mix(seed ^ 0x9c20b3)) * TAU);
    ([x, z], 6.0 + fraction(mix(seed ^ 0x7d61e)) * 5.5)
}

pub(crate) fn ridge_updraft(upstream: f32, downstream: f32, wind: f32, above: f32) -> f32 {
    ((downstream - upstream) * wind * (1.0 / 54.0)).clamp(-4.0, 5.5)
        * (1.0 - smoothstep((above - 22.0) / 85.0))
}

const CELL: f32 = 128.0;

fn thermal_tile(position: f32) -> i32 {
    let cell = position / CELL;
    let index = cell as i32;
    index - i32::from(cell < index as f32)
}

fn warm_thermal(
    world: &World, seed: u64, x: f32, z: f32, above: f32, daylight: f32,
) -> Option<(f32, (f32, f32))> {
    let sun = smoothstep((daylight - 0.42) / 0.42);
    if sun == 0.0 || above > 176.0 || world.coast_signal(x, z) < 0.06 {
        return None;
    }
    let tx = thermal_tile(x);
    let tz = thermal_tile(z);
    let key = mix(seed ^ (u64::from(tx as u32) << 32) ^ u64::from(tz as u32) ^ 0x8faa_791c);
    if key & 7 >= 3 {
        return None;
    }
    let cx = tx as f32 * CELL + 38.0 + fraction(mix(key ^ 18)) * 52.0;
    let cz = tz as f32 * CELL + 38.0 + fraction(mix(key ^ 53)) * 52.0;
    let dx = cx - x;
    let dz = cz - z;
    let radius = 20.0 + fraction(mix(key ^ 0x351)) * 14.0;
    let core = 1.0 - (dx * dx + dz * dz) / (radius * radius);
    if core <= 0.0 {
        return None;
    }
    let ground = world.sample(cx, cz);
    let warmth = match ground.surface {
        Surface::Desert | Surface::Rock => 1.0,
        Surface::Grass => 0.72,
        Surface::Forest => 0.30,
        _ => 0.0,
    };
    let cap = (1.0 - smoothstep((above - 71.0) / 105.0)) * smoothstep((above + 7.0) / 25.0);
    let upward = core * core * warmth * cap * sun * 8.6;
    (upward > 0.03).then_some((upward, (cx, cz)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solar_thermals_only_form_over_warming_dry_ground() {
        let seeds = [2, 31, 817, 8377, 14809];
        let mut daytime = 0;
        let mut terrain_lift = 0;
        for seed in seeds {
            let world = World::new(seed);
            for z in -29..=29 {
                for x in -29..=29 {
                    let x = x as f32 * 24.0;
                    let z = z as f32 * 24.0;
                    let ground = world.elevation(x, z);
                    let at_noon = Air::at(&world, seed, x, z, ground + 36.0, 1.0);
                    let at_night = Air::at(&world, seed, x, z, ground + 36.0, 0.0);
                    assert!(at_night.thermal_center.is_none());
                    assert!((-5.0..=9.5).contains(&at_noon.upward));
                    terrain_lift += usize::from(matches!(at_night.lift, Lift::Ridge | Lift::LeeWave));
                    if at_noon.thermal_center.is_some() {
                        daytime += 1;
                        assert!(ground > SEA_LEVEL);
                        assert!(at_noon.upward > at_night.upward || at_noon.upward == 9.5);
                    }
                }
            }
        }
        assert!(daytime > 40 && terrain_lift > 40, "{daytime} {terrain_lift}");
    }
}
