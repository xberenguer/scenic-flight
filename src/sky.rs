use core::f32::consts::{FRAC_PI_2, TAU};

use crate::encounters::{Meteor, meteor};
use crate::flight::TICKS_PER_SECOND;
use crate::math::{fraction, mix, sin_cos, smoothstep};

pub const SECONDS_PER_DAY: u32 = 24 * 60;
const DAY_TICKS: u64 = SECONDS_PER_DAY as u64 * TICKS_PER_SECOND as u64;
const LUNATION_TICKS: u64 = 59 * DAY_TICKS / 2;
const WEATHER_TICKS: u64 = 210 * TICKS_PER_SECOND as u64;

#[derive(Clone, Copy, Debug)]
pub struct Sky {
    pub sun_direction: [f32; 3],
    pub moon_direction: [f32; 3],
    pub daylight: f32,
    pub cloud_cover: f32,
    pub haze: f32,
    pub cloud_base: f32,
    pub cloud_offset: [f32; 2],
    pub meteor: Option<Meteor>,
    pub(crate) weather: Option<WeatherField>,
    pub(crate) sun_rising: bool,
    #[cfg(feature = "ground-cover")]
    pub(crate) ground_seed: u32,
}

impl Sky {
    pub fn at(seed: u64, ticks: u64) -> Self {
        Self::build(seed, ticks, Self::day_at(seed, ticks), None)
    }

    /// Keep the astronomical clock global while following the air mass over
    /// the aircraft's actual location. Position is [east, height, north].
    pub fn at_position(seed: u64, ticks: u64, position: [f32; 3]) -> Self {
        Self::from_time_at_position(seed, ticks, Self::day_at(seed, ticks), position)
    }

    pub fn from_time_at_position(
        seed: u64, ticks: u64, day_fraction: f32, position: [f32; 3],
    ) -> Self {
        Self::build(seed, ticks, day_fraction, Some(position))
    }

    fn day_at(seed: u64, ticks: u64) -> f32 {
        let launch_time = (0.27 + fraction(mix(seed ^ 7)) * 0.49) * DAY_TICKS as f32;
        ((ticks % DAY_TICKS) as f32 + launch_time) / DAY_TICKS as f32
    }

    pub fn from_time(seed: u64, ticks: u64, day_fraction: f32) -> Self {
        Self::build(seed, ticks, day_fraction, None)
    }

    fn build(seed: u64, ticks: u64, day_fraction: f32, position: Option<[f32; 3]>) -> Self {
        let latitude = 0.43 + fraction(mix(seed ^ 23)) * 0.39;
        let declination = (fraction(mix(seed ^ 49)) - 0.50) * 0.68;
        let hour = (day_fraction - 0.5) * TAU;
        let (lat_sin, lat_cos) = sin_cos(latitude);
        let (turn_sin, turn_cos) = sin_cos(fraction(mix(seed ^ 83)) * TAU);
        let direction = |(hour_sin, hour_cos), declination| {
            let (dec_sin, dec_cos) = sin_cos(declination);
            let up = lat_sin * dec_sin + lat_cos * dec_cos * hour_cos;
            let east = -dec_cos * hour_sin;
            let north = lat_cos * dec_sin - lat_sin * dec_cos * hour_cos;
            [east * turn_cos + north * turn_sin, up, north * turn_cos - east * turn_sin]
        };
        let solar_hour = sin_cos(hour);
        let sun_direction = direction(solar_hour, declination);
        let daylight = smoothstep((sun_direction[1] + 0.19) / 0.49);
        let phase = (fraction(mix(seed ^ 0x6e7f_ea73))
            + (ticks % LUNATION_TICKS) as f32 / LUNATION_TICKS as f32) * TAU;
        // The moon drifts east of the sun as the month advances. A small
        // orbital tilt keeps most conjunctions and oppositions out of eclipse.
        let (_, phase_cos) = sin_cos(phase);
        let (tilt, _) = sin_cos(phase + fraction(mix(seed ^ 0xac81_f74d)) * TAU);
        let moon_direction = direction(sin_cos(hour - phase), declination * phase_cos + 0.089 * tilt);
        let field = WeatherField::new(seed, ticks);
        let local = position.map_or_else(|| field.background(), |[x, _, z]| field.over(x, z));
        let haze = position.map_or(local.haze, |[_, altitude, _]| {
            let clear_above = smoothstep((altitude - local.cloud_base) / 95.0);
            local.haze * (1.0 - 0.44 * clear_above)
        });
        let mut sky = Self {
            sun_direction, moon_direction, daylight,
            cloud_cover: local.cloud_cover, cloud_base: local.cloud_base, haze,
            cloud_offset: field.wind.map(|dir| field.seconds * dir * 0.18),
            meteor: None, weather: position.map(|_| field), sun_rising: solar_hour.0 < 0.0,
            #[cfg(feature = "ground-cover")]
            ground_seed: (seed ^ seed.rotate_right(32)) as u32,
        };
        sky.meteor = meteor(seed, ticks, &sky);
        sky
    }

    pub(crate) fn weather_over(&self, x: f32, z: f32) -> LocalWeather {
        self.weather.map_or(LocalWeather {
            cloud_cover: self.cloud_cover, haze: self.haze,
            cloud_base: 107.0 + self.cloud_cover * 28.0,
        }, |field| field.over(x, z))
    }

    /// In a shower, scattering softens distant views. Nearby terrain,
    /// the astronomical clock, and flight controls keep their usual scale.
    pub fn with_shower(mut self, strength: f32) -> Self {
        let strength = strength.clamp(0.0, 1.0);
        self.haze = (self.haze + strength * 0.35).min(0.67);
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WeatherField {
    seed: u64,
    wind: [f32; 2],
    seconds: f32,
    background_clouds: f32,
    background_haze: f32,
}

pub(crate) struct LocalWeather {
    pub cloud_cover: f32,
    pub haze: f32,
    pub cloud_base: f32,
}

impl WeatherField {
    fn new(seed: u64, ticks: u64) -> Self {
        let stage = ticks / WEATHER_TICKS;
        let fade = smoothstep((ticks % WEATHER_TICKS) as f32 / WEATHER_TICKS as f32);
        let climate = |stage| {
            let state = mix(seed ^ mix(stage));
            (fraction(state), fraction(mix(state ^ 0xab09)))
        };
        let (a, b) = climate(stage);
        let (c, d) = climate(stage + 1);
        let (wind_x, wind_z) = sin_cos(fraction(mix(seed ^ 148)) * TAU + FRAC_PI_2);
        Self {
            seed, wind: [wind_x, wind_z], seconds: ticks as f32 / TICKS_PER_SECOND as f32,
            background_clouds: a + (c - a) * fade,
            background_haze: b + (d - b) * fade,
        }
    }

    fn background(self) -> LocalWeather {
        let cloud_cover = 0.11 + 0.75 * self.background_clouds;
        LocalWeather {
            cloud_cover,
            haze: (0.10 + 0.36 * self.background_haze) * (0.62 + 0.38 * cloud_cover),
            cloud_base: 107.0 + cloud_cover * 28.0,
        }
    }

    fn over(self, x: f32, z: f32) -> LocalWeather {
        let [east, north] = self.wind;
        // Wide bands extend across the wind. Their grid follows the same
        // prevailing flow as the clouds rather than following the camera.
        let along = x * east + z * north + self.seconds * 0.88;
        let across = z * east - x * north;
        let bend = weather_noise(self.seed ^ 0xec55_1249, along / 1_780.0, across / 2_420.0);
        let field = weather_noise(
            self.seed ^ 0x2677_b0c1, (along + 240.0 * (bend - 0.5)) / 620.0, across / 1_310.0,
        );
        let wet = smoothstep((field - 0.22) / 0.58);
        let cover = (wet * 0.70 + self.background_clouds * 0.30).clamp(0.0, 1.0);
        #[cfg(not(feature = "regional-climate"))]
        let ceiling = smoothstep((cover - 0.45) / 0.47);
        #[cfg(feature = "regional-climate")]
        let (cover, ceiling, dust) = {
            let world = crate::world::World::new(self.seed);
            let climate = crate::climate::Climate::at(&world, x, z).conditions();
            let cover = (cover + (climate.cloud_bias - 0.32) * 0.37).clamp(0.0, 1.0);
            (cover, smoothstep((cover - 0.45) / 0.47), climate.dust * 0.11)
        };
        #[cfg(not(feature = "regional-climate"))]
        let dust = 0.0;
        LocalWeather {
            cloud_cover: 0.11 + cover * 0.75,
            haze: 0.075 + 0.105 * self.background_haze + wet * 0.09 + ceiling * 0.14 + dust,
            cloud_base: 136.0 + (bend - 0.5) * 18.0 - ceiling * 46.0,
        }
    }
}

fn weather_noise(seed: u64, x: f32, z: f32) -> f32 {
    let floor = |value: f32| {
        let index = value as i32;
        index.saturating_sub(i32::from(value < index as f32))
    };
    let nx = floor(x);
    let nz = floor(z);
    let u = smoothstep(x - nx as f32);
    let v = smoothstep(z - nz as f32);
    let cell = |ix: i32, iz: i32| {
        let tile = (u64::from(ix as u32) << 32) | u64::from(iz as u32);
        fraction(mix(seed ^ mix(tile)))
    };
    let x1 = nx.saturating_add(1);
    let z1 = nz.saturating_add(1);
    let a = cell(nx, nz);
    let b = cell(nx, z1);
    let near = a + (cell(x1, nz) - a) * u;
    let far = b + (cell(x1, z1) - b) * u;
    near + (far - near) * v
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_rises_crosses_and_sets_in_order() {
        for seed in [4, 9_805, 47, 72, 664] {
            let early = Sky::from_time(seed, 0, 0.0);
            let day = Sky::from_time(seed, 0, 0.5);
            assert!(early.sun_direction[1] < -0.18);
            assert!(day.sun_direction[1] > 0.40);
            assert_eq!(early.daylight, 0.0);
            assert_eq!(day.daylight, 1.0);
            let mut last_up = early.sun_direction[1];
            let mut rises = 0;
            let mut sets = 0;
            for step in 1..=1_440 {
                let sky = Sky::from_time(seed, 0, step as f32 / 1_440.0);
                let up = sky.sun_direction[1];
                assert!((up - last_up).abs() < 0.005);
                rises += usize::from(last_up < 0.0 && up >= 0.0);
                sets += usize::from(last_up >= 0.0 && up < 0.0);
                last_up = up;
            }
            assert_eq!((rises, sets), (1, 1));
        }
    }

    #[test]
    fn weather_crossfades_at_epoch_boundary() {
        for stage in 1..100 {
            let before = Sky::at(383, stage * WEATHER_TICKS - 1);
            let after = Sky::at(383, stage * WEATHER_TICKS);
            assert!((before.cloud_cover - after.cloud_cover).abs() < 0.00001);
            assert!((before.haze - after.haze).abs() < 0.00001);
        }
    }

    #[test]
    fn regional_weather_changes_with_place_and_clears_above_the_deck() {
        let seed = 38;
        let ticks = 5_471;
        let clock = Sky::at(seed, ticks);
        let mut driest = 1.0_f32;
        let mut wettest = 0.0_f32;
        let mut previous = None::<Sky>;
        for step in -1_000..=1_000 {
            let point = [step as f32 * 3.0, 28.0, -159.0];
            let low = Sky::at_position(seed, ticks, point);
            let high = Sky::at_position(seed, ticks, [point[0], 340.0, point[2]]);
            assert_eq!(low.sun_direction, clock.sun_direction);
            assert_eq!(low.moon_direction, clock.moon_direction);
            assert_eq!(low.cloud_cover, high.cloud_cover);
            assert!(high.haze < low.haze * 0.57);
            driest = driest.min(low.cloud_cover);
            wettest = wettest.max(low.cloud_cover);
            if let Some(past) = previous {
                assert!((past.cloud_cover - low.cloud_cover).abs() < 0.017);
                assert!((past.cloud_base - low.cloud_base).abs() < 0.9);
            }
            previous = Some(low);
        }
        assert!(wettest - driest > 0.34);
    }

    #[test]
    fn moon_opposes_the_sun_at_full_and_moves_continuously_between_days() {
        for seed in [4, 19, 383, 9_805] {
            let initial_phase = fraction(mix(seed ^ 0x6e7f_ea73));
            let phase_tick = |target: f32| {
                (((target - initial_phase + 1.0) % 1.0) * LUNATION_TICKS as f32) as u64
            };
            let angle = |sky: Sky| {
                sky.sun_direction.into_iter().zip(sky.moon_direction)
                    .map(|(sun, moon)| sun * moon).sum::<f32>()
            };
            let full = phase_tick(0.50);
            assert!(angle(Sky::from_time(seed, phase_tick(0.0), 0.5)) > 0.994);
            assert!(angle(Sky::from_time(seed, full, 0.5)) < -0.994);
            assert!(angle(Sky::from_time(seed, phase_tick(0.25), 0.5)).abs() < 0.04);
            assert!(Sky::from_time(seed, full, 0.0).moon_direction[1] > 0.40);
            assert!(Sky::from_time(seed, full, 0.5).moon_direction[1] < -0.40);
            let before = Sky::at(seed, DAY_TICKS - 1).moon_direction;
            let after = Sky::at(seed, DAY_TICKS).moon_direction;
            assert!(before.into_iter().zip(after).all(|(a, b)| (a - b).abs() < 0.001));
        }
    }
}
