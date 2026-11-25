use core::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::air::{Air, Lift};
use crate::coast_channel::{ChannelAircraft, CoastChannel};
use crate::controls::{Buttons, Command, Controls, Screen};
use crate::land_stream::LandStream;
use crate::math::{fraction, mix, sin_cos, smoothstep, wrap_angle};
use crate::settlement::Terminus;
use crate::sky::Sky;
use crate::world::{SEA_LEVEL, Surface, World};

pub const TICKS_PER_SECOND: u32 = 30;
const DT: f32 = 1.0 / TICKS_PER_SECOND as f32;
const BLACKBIRD_PITCH: f32 = 0.53;
const CHASE_HEIGHT_RATIO: f32 = 0.28;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Aircraft {
    BushPlane,
    Racer,
    SportJet,
    Trike,
    Glider,
    Fighter,
    Blackbird,
    BaldEagle,
}

impl Aircraft {
    pub const ALL: [Self; 8] = [
        Self::BushPlane, Self::Racer, Self::SportJet, Self::Trike,
        Self::Glider, Self::Fighter, Self::Blackbird, Self::BaldEagle,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::BushPlane => "Bush Plane",
            Self::Racer => "Racer",
            Self::SportJet => "Sport Jet",
            Self::Trike => "Ultralight",
            Self::Glider => "Glider",
            Self::Fighter => "Fighter",
            Self::Blackbird => "Blackbird",
            Self::BaldEagle => "Bald Eagle",
        }
    }

    fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::ALL.len()]
    }

    pub fn ratings(self) -> [u8; 3] {
        let profile = self.profile();
        let bars = |value, thresholds: [f32; 5]| {
            thresholds.iter().filter(|&&threshold| value >= threshold).count() as u8
        };
        [
            bars(profile.speed, [8.2, 14.0, 20.0, 25.0, 46.0]),
            bars(profile.yaw_rate, [0.13, 0.19, 0.25, 0.35, 0.46]),
            bars(profile.clearance, [12.0, 24.0, 44.0, 72.0, 170.0]),
        ]
    }

    fn profile(self) -> Profile {
        const PROFILES: [Profile; 8] = [
            Profile { speed: 15.5, clearance: 31.0, floor: 10.0, yaw_rate: 0.27,
                chase: 12.0, max_bank: 0.46, turn_response: 0.083, climb: 20.0 },
            Profile { speed: 22.0, clearance: 28.0, floor: 13.0, yaw_rate: 0.50,
                chase: 14.0, max_bank: 0.72, turn_response: 0.130, climb: 31.0 },
            Profile { speed: 26.7, clearance: 56.0, floor: 24.0, yaw_rate: 0.26,
                chase: 18.0, max_bank: 0.36, turn_response: 0.078, climb: 31.0 },
            Profile { speed: 10.2, clearance: 20.0, floor: 9.0, yaw_rate: 0.205,
                chase: 11.0, max_bank: 0.31, turn_response: 0.057, climb: 15.0 },
            Profile { speed: 13.0, clearance: 31.0, floor: 14.0, yaw_rate: 0.21,
                chase: 14.0, max_bank: 0.40, turn_response: 0.062, climb: 18.0 },
            Profile { speed: 36.0, clearance: 76.0, floor: 27.0, yaw_rate: 0.38,
                chase: 20.0, max_bank: 0.60, turn_response: 0.100, climb: 41.0 },
            Profile { speed: 57.0, clearance: 321.0, floor: 87.0, yaw_rate: 0.14,
                chase: 26.0, max_bank: 0.27, turn_response: 0.049, climb: 35.0 },
            Profile { speed: 7.4, clearance: 15.0, floor: 7.0, yaw_rate: 0.27,
                chase: 9.0, max_bank: 0.37, turn_response: 0.068, climb: 14.0 },
        ];
        PROFILES[self as usize]
    }
}

#[derive(Clone, Copy)]
struct Profile {
    speed: f32,
    clearance: f32,
    floor: f32,
    yaw_rate: f32,
    chase: f32,
    max_bank: f32,
    turn_response: f32,
    climb: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub x: f32,
    pub z: f32,
    pub altitude: f32,
    pub heading: f32,
    pub pitch: f32,
    pub bank: f32,
}

#[derive(Clone, Copy)]
struct Pose {
    x: f32,
    z: f32,
    altitude: f32,
    heading: f32,
    bank: f32,
    climb_rate: f32,
    view: View,
}

#[derive(Clone)]
pub struct Flight {
    pub aircraft: Aircraft,
    pub ticks: u64,
    pub x: f32,
    pub z: f32,
    pub altitude: f32,
    pub heading: f32,
    pub bank: f32,
    pub view: View,
    pub terrain_recoveries: u32,
    pub air: Air,
    seed: u64,
    energy_altitude: f32,
    thermal_turn: u16,
    thermal_cooldown: u16,
    next_coastal_pass: u64,
    coastal_pass: Option<(u64, u64)>,
    phase: f32,
    turn: f32,
    climb_rate: f32,
    ridge_preview_floor: f32,
    blackbird_cruise_altitude: f32,
    course: f32,
    speed: f32,
    town_approach_staged: bool,
    route_intent: RouteIntent,
    scenic_progress: ScenicProgress,
    train_follow: crate::train_follow::TrainFollow,
    previous_pose: Pose,
}

#[derive(Clone, Copy)]
struct ScenicLeg {
    target: [f32; 2],
    stop: u64,
    water_seconds: u8,
    coast_channel: Option<CoastChannel>,
}

#[derive(Clone)]
struct ScenicProgress {
    // A rolling history of distinct areas survives repeated laps.
    visits: [u16; 48],
    used: usize,
    next: usize,
    last_new: u64,
    local_anchor: [f32; 2],
    local_since: u64,
    retry_at: u64,
    leg: Option<ScenicLeg>,
    // Fifteen 12-second samples cover one candidate lap for these aircraft.
    coastal_lap_turns: [i8; 15],
    coastal_lap_cells: [u16; 15],
}

impl ScenicProgress {
    const fn new() -> Self {
        Self {
            visits: [0; 48], used: 0, next: 0, last_new: 0, local_anchor: [0.0; 2],
            local_since: 0, retry_at: 0, leg: None,
            coastal_lap_turns: [0; 15], coastal_lap_cells: [0; 15],
        }
    }

    fn has_visited(&self, x: f32, z: f32, cell_size: f32) -> bool {
        self.visits[..self.used].contains(&visit_cell(x, z, cell_size))
    }

    fn mark(&mut self, x: f32, z: f32, cell_size: f32, local_radius: f32, age: u64) {
        let dx = x - self.local_anchor[0];
        let dz = z - self.local_anchor[1];
        if self.used == 0 || dx * dx + dz * dz > local_radius * local_radius {
            self.local_anchor = [x, z];
            self.local_since = age;
        }
        let cell = visit_cell(x, z, cell_size);
        if !self.visits[..self.used].contains(&cell) {
            self.visits[self.next] = cell;
            self.next = (self.next + 1) % self.visits.len();
            self.used = (self.used + 1).min(self.visits.len());
            self.last_new = age;
        }
    }

    fn repeated_coastal_lap(&self, x: f32, z: f32, age: u64, max_span_squared: i32) -> bool {
        if self.coastal_lap_turns.iter().copied().sum::<i8>().abs() < 8 {
            return false;
        }
        let current = visit_cell(x, z, 128.0).to_le_bytes();
        let offset = |cell: u16| {
            // Local travel is well within half of the packed grid's wrap span.
            let [x, z] = cell.to_le_bytes();
            [
                i32::from(x.wrapping_sub(current[0]) as i8),
                i32::from(z.wrapping_sub(current[1]) as i8),
            ]
        };
        let oldest = ((age / 12) % self.coastal_lap_cells.len() as u64) as usize;
        let [x, z] = offset(self.coastal_lap_cells[oldest]);
        if x * x + z * z > 36 {
            return false;
        }
        let mut bounds = [0_i32; 4];
        for cell in self.coastal_lap_cells {
            let [x, z] = offset(cell);
            bounds = [
                bounds[0].min(x), bounds[1].min(z), bounds[2].max(x), bounds[3].max(z),
            ];
        }
        let width = bounds[2] - bounds[0];
        let height = bounds[3] - bounds[1];
        width * width + height * height <= max_span_squared
    }

    fn find_exit(
        &self, world: &World, x: f32, z: f32, heading: f32,
        cell_size: f32, range: f32, water_limit: f32, fighter: bool,
    ) -> Option<[f32; 2]> {
        let mut choices = [(f32::NEG_INFINITY, [0.0; 2]); 5];
        let min_coast = if fighter { 0.30 } else { 0.14 };
        for angle in [0.0, -0.58, 0.58, -1.12, 1.12, -1.68, 1.68, -2.24, 2.24, PI] {
            let (sin, cos) = sin_cos(heading + angle);
            for radius in [range * 0.74, range, range * 1.24] {
                let tx = x + sin * radius;
                let tz = z + cos * radius;
                if tx.abs().max(tz.abs()) > 25_100.0 || self.has_visited(tx, tz, cell_size) {
                    continue;
                }
                let coast = world.coast_signal(tx, tz);
                if coast < min_coast {
                    continue;
                }
                let mid = world.coast_signal(x + sin * radius * 0.52, z + cos * radius * 0.52);
                let follow_through = if fighter {
                    // The faster aircraft needs dry terrain beyond the exit to carry on.
                    let coast = [410.0, 810.0].into_iter().map(|runout| {
                        world.coast_signal(tx + sin * runout, tz + cos * runout)
                    }).fold(1.0_f32, f32::min);
                    if coast < 0.085 { continue; }
                    coast.min(0.55) * 0.6
                } else {
                    0.0
                };
                let score = coast.min(0.65) + mid.clamp(-0.35, 0.48) * 0.82
                    - angle.abs() * 0.052 - radius / range * 0.065;
                let score = if fighter { score + follow_through } else { score };
                let index = choices.partition_point(|(old, _)| *old >= score);
                if index < choices.len() {
                    let last = choices.len() - 1;
                    choices.copy_within(index..last, index + 1);
                    choices[index] = (score, [tx, tz]);
                }
            }
        }
        choices.into_iter().filter(|(score, _)| score.is_finite()).find_map(|(_, target)| {
            let [tx, tz] = target;
            let dx = tx - x;
            let dz = tz - z;
            let stride = distance(dx, dz) / 12.0;
            let mut wet = 0.0;
            for step in 1..=12 {
                let fraction = step as f32 / 12.0;
                let coast = world.coast_signal(x + dx * fraction, z + dz * fraction);
                wet = if coast < 0.018 { wet + stride } else { 0.0 };
                if wet > water_limit {
                    return None;
                }
            }
            if fighter {
                let inv_len = 1.0 / distance(dx, dz);
                let side = [-dz * inv_len * 40.0, dx * inv_len * 40.0];
                for step in 1..=16 {
                    let t = step as f32 / 16.0;
                    let (x, z) = (x + dx * t, z + dz * t);
                    if world.coast_signal(x + side[0], z + side[1]) < 0.012
                        || world.coast_signal(x - side[0], z - side[1]) < 0.012
                    {
                        return None;
                    }
                }
            }
            Some(target)
        })
    }
}

fn fighter_formation_clear(seed: u64, start: u64, stop: u64) -> bool {
    // Formation windows last over 11 seconds. Sampling every six seconds
    // detects even a formation starting during the prospective leg.
    !(start - 8..=stop + 8).step_by(6).any(|second| {
        crate::encounters::formation(
            Aircraft::Fighter, seed, second * u64::from(TICKS_PER_SECOND),
        ).is_some()
    })
}

fn visit_cell(x: f32, z: f32, cell_size: f32) -> u16 {
    let slot = |position: f32| {
        let value = position / cell_size;
        let whole = value as i16;
        (whole - i16::from(value < f32::from(whole))) as u8
    };
    u16::from_le_bytes([slot(x), slot(z)])
}

#[derive(Clone, Default)]
struct RouteIntent {
    recent: [u8; 5],
    course: Option<f32>,
    passed: [Option<[f32; 2]>; 5],
}

impl RouteIntent {
    fn sample(&mut self, world: &World, x: f32, z: f32, heading: f32, speed: f32) {
        // Recent terrain discourages spending the whole flight in one setting.
        for visit in &mut self.recent {
            *visit = visit.saturating_sub(1);
        }
        self.passed.rotate_right(1);
        self.passed[0] = Some([x, z]);
        let Some(terrain) = terrain_context(world, x, z) else {
            self.course = None;
            return;
        };
        self.recent[terrain] = (self.recent[terrain] + 4).min(8);
        let range = (speed * 19.0).clamp(195.0, 860.0);
        let revisit_radius = range * 0.52;
        let score = |offset: f32| {
            let (sin, cos) = sin_cos(heading + offset);
            let mut value = 0.0;
            for (fraction, weight) in [(0.30, 0.37), (0.62, 0.41), (1.0, 0.22)] {
                let distance = fraction * range;
                let at = terrain_context(world, x + sin * distance, z + cos * distance);
                let novelty = at.map_or(-2.8, |class| {
                    [3.1, 2.15, 2.72, 2.95, 3.34][class] - f32::from(self.recent[class]) * 0.51
                });
                value += novelty * weight;
                // Preserve a dry near approach. An inlet may frame the far end.
                if at.is_none() && fraction < 0.7 {
                    return f32::NEG_INFINITY;
                }
            }
            let look_x = x + sin * range;
            let look_z = z + cos * range;
            let revisit = self.passed[1..].iter().flatten().any(|&[old_x, old_z]| {
                let dx = look_x - old_x;
                let dz = look_z - old_z;
                dx * dx + dz * dz < revisit_radius * revisit_radius
            });
            value - offset.abs() * 1.18 - if revisit { 1.35 } else { 0.0 }
        };
        let forward = score(0.0);
        let mut best = (0.0, forward);
        for offset in [-0.72_f32, -0.48, -0.24, 0.24, 0.48, 0.72] {
            let value = score(offset);
            if value > best.1 {
                best = (offset, value);
            }
        }
        self.course = (best.0 != 0.0 && best.1 > forward + 0.32)
            .then_some(wrap_angle(heading + best.0));
    }
}

fn terrain_context(world: &World, x: f32, z: f32) -> Option<usize> {
    let (sample, coast) = world.sample_and_shore(x, z);
    if coast <= 0.019 {
        return None;
    }
    Some(if coast < 0.14 || sample.elevation < 16.0 {
        0
    } else if sample.elevation >= 69.0 {
        4
    } else {
        match sample.surface {
            Surface::Forest => 2,
            Surface::Desert => 3,
            _ => 1,
        }
    })
}

fn coordinated_bank(turn_fraction: f32) -> f32 {
    let fraction = turn_fraction.clamp(-1.0, 1.0);
    let amount = fraction.abs();
    // A very slow turn barely tips the wings. The middle of the range gets
    // some visual emphasis while each aircraft keeps its own roll limit.
    let shoulder = smoothstep((amount - 0.025) / 0.15);
    let headroom = 1.0 - amount;
    fraction * (1.0 + 0.90 * shoulder * headroom * headroom)
}

fn surface(world: &World, x: f32, z: f32) -> f32 {
    world.elevation(x, z).max(SEA_LEVEL)
}

fn ahead(world: &World, x: f32, z: f32, heading: f32, distance: f32) -> f32 {
    let (sin, cos) = sin_cos(heading);
    surface(world, x + distance * sin, z + distance * cos)
}

fn coast_ahead(world: &World, x: f32, z: f32, heading: f32, distance: f32) -> f32 {
    let (sin, cos) = sin_cos(heading);
    world.coast_signal(x + distance * sin, z + distance * cos)
}

fn launch_origin(seed: u64) -> (f32, f32, f32) {
    (
        (fraction(mix(seed ^ 927)) - 0.5) * 4_800.0,
        (fraction(mix(seed ^ 191)) - 0.5) * 4_800.0,
        fraction(mix(seed ^ 763)) * TAU,
    )
}

fn launch_point(world: &World, seed: u64, profile: Profile) -> (f32, f32, f32) {
    let (origin_x, origin_z, angle) = launch_origin(seed);
    let scenery = mix(seed ^ 0xb97e_0b83) % 3;
    let mut best = (origin_x, origin_z, angle);
    let mut best_score = f32::NEG_INFINITY;
    for ring in 0..=9 {
        let distance = ring as f32 * 102.0;
        for face in 0..10 {
            let course = angle + face as f32 * (TAU / 10.0);
            let (sin, cos) = sin_cos(course);
            let x = origin_x + sin * distance;
            let z = origin_z + cos * distance;
            let elevation = surface(world, x, z);
            let close = ahead(world, x, z, course, profile.speed * 3.0);
            let far = ahead(world, x, z, course, 186.0);
            let left = ahead(world, x, z, course - 0.49, 148.0);
            let right = ahead(world, x, z, course + 0.49, 148.0);
            let high = far.max(left).max(right);
            let low = far.min(left).min(right);
            let contrast = (high - low).min(90.0);
            let dry_start = elevation.min(16.0) + close.min(20.0) * 0.36;
            let rise = (close - elevation).max(0.0);
            let view = match scenery {
                0 => {
                    let shoreline = (1.0 - low / 5.0).clamp(0.0, 1.0);
                    let land_in_view = ((left + right + far) / 18.0).clamp(0.0, 1.0);
                    27.0 * shoreline * land_in_view + contrast * 0.16
                }
                1 => contrast * 0.46 + high.min(117.0) * 0.19
                    - (elevation - 61.0).max(0.0) * 0.37,
                _ => contrast * 0.32 + low.min(22.0) * 0.30 + far.min(45.0) * 0.17,
            };
            let score = dry_start + view - rise * 0.63 - distance * 0.012;
            if score > best_score {
                best = (x, z, course);
                best_score = score;
            }
        }
    }
    best
}

fn wider_land_departure(
    world: &World,
    seed: u64,
    aircraft: Aircraft,
    (x, z, heading): (f32, f32, f32),
) -> Option<(f32, f32, f32)> {
    if world.coast_signal(x, z) > 0.0 {
        return None;
    }
    // Preserve offshore openings that already face a credible shoreline.
    for offset in [-0.5, 0.0, 0.5] {
        for range in [130.0, 280.0, 440.0] {
            if coast_ahead(world, x, z, heading + offset, range) > 0.023 {
                return None;
            }
        }
    }

    if aircraft == Aircraft::BaldEagle {
        // Keep the shorter, natural coast approaches of slow flight. Only
        // extend the search from deep water without reachable nearby land.
        if world.sample(x, z).surface != Surface::DeepWater {
            return None;
        }
        for step in 0..12 {
            let (sin, cos) = sin_cos(heading + step as f32 * (TAU / 12.0));
            if [140.0, 280.0, 420.0].into_iter().any(|distance| {
                world.coast_signal(x + sin * distance, z + cos * distance) > 0.035
            }) {
                return None;
            }
        }
    }
    let (origin_x, origin_z, angle) = launch_origin(seed);
    let profile = aircraft.profile();
    let forecast = (profile.speed * 23.0).clamp(220.0, 560.0);
    for radius in [1_170.0, 1_480.0, 1_900.0, 2_420.0, 3_050.0] {
        let mut best = None;
        let mut best_score = f32::NEG_INFINITY;
        for step in 0..16 {
            let bearing = angle + step as f32 * (TAU / 16.0);
            let (sin, cos) = sin_cos(bearing);
            let x = origin_x + sin * radius;
            let z = origin_z + cos * radius;
            let dry = world.coast_signal(x, z);
            if dry < 0.13 {
                continue;
            }
            let elevation = surface(world, x, z);
            for turn in [-0.62, 0.0, 0.62] {
                let course = bearing + turn;
                let (sin, cos) = sin_cos(course);
                let mut minimum = dry;
                for fraction in [0.25, 0.50, 0.78, 1.0] {
                    let distance = forecast * fraction;
                    minimum = minimum.min(world.coast_signal(x + sin * distance, z + cos * distance));
                    if minimum < 0.11 {
                        break;
                    }
                }
                if minimum < 0.11 {
                    continue;
                }
                let rise = (ahead(world, x, z, course, profile.speed * 3.0) - elevation).abs();
                let score = minimum.min(0.46) * 2.0 - rise * 0.013 - turn.abs() * 0.035;
                if score > best_score {
                    best_score = score;
                    best = Some((x, z, wrap_angle(course)));
                }
            }
        }
        if best.is_some() {
            return best;
        }
    }
    None
}

impl Flight {
    pub fn start(world: &World, seed: u64, aircraft: Aircraft) -> Self {
        let profile = aircraft.profile();
        let mut approach = launch_point(world, seed, profile);
        if matches!(aircraft, Aircraft::BushPlane | Aircraft::Racer | Aircraft::SportJet
            | Aircraft::Trike | Aircraft::Glider | Aircraft::BaldEagle | Aircraft::Blackbird)
        {
            approach = wider_land_departure(world, seed, aircraft, approach).unwrap_or(approach);
        }
        Self::start_at(world, seed, aircraft, approach)
    }

    fn start_at(world: &World, seed: u64, aircraft: Aircraft, (x, z, heading): (f32, f32, f32)) -> Self {
        let profile = aircraft.profile();
        let terrain = surface(world, x, z);
        let ahead = ahead(world, x, z, heading, profile.speed * 2.0);
        let altitude = if aircraft == Aircraft::Blackbird {
            profile.clearance.max(terrain.max(ahead) + profile.floor)
        } else {
            terrain.max(ahead) + profile.clearance
        };
        let (sin, cos) = sin_cos(heading);
        let view_x = x - sin * profile.chase;
        let view_z = z - cos * profile.chase;
        let view = View {
            x: view_x,
            z: view_z,
            altitude: (altitude + profile.chase * CHASE_HEIGHT_RATIO)
                .max(surface(world, view_x, view_z) + 6.0),
            heading,
            pitch: if aircraft == Aircraft::Blackbird { BLACKBIRD_PITCH } else { 0.15 },
            bank: 0.0,
        };
        Self {
            aircraft, ticks: 0, x, z, altitude, heading, bank: 0.0, view,
            terrain_recoveries: 0, air: Air::CALM, seed, energy_altitude: 0.0,
            thermal_turn: 0, thermal_cooldown: 0,
            next_coastal_pass: (104 + mix(seed ^ 7_583) % 110) * u64::from(TICKS_PER_SECOND),
            coastal_pass: None, phase: fraction(mix(seed ^ 4_697)) * TAU,
            turn: 0.0, climb_rate: 0.0, ridge_preview_floor: 0.0, blackbird_cruise_altitude: altitude, course: heading, speed: profile.speed,
            town_approach_staged: false,
            route_intent: RouteIntent::default(),
            scenic_progress: ScenicProgress::new(),
            train_follow: crate::train_follow::TrainFollow::default(),
            previous_pose: Pose { x, z, altitude, heading, bank: 0.0, climb_rate: 0.0, view },
        }
    }

    /// Occasionally start on approach to a real town in the prepared launch
    /// area. Call before the first simulation tick and camera-dependent caches.
    pub fn stage_town_approach<const N: usize>(&mut self, world: &World, land: &LandStream<N>) -> bool {
        if self.town_approach_staged || self.ticks != 0 || self.aircraft == Aircraft::Blackbird
            || mix(self.seed ^ 0x894e_47b3_daf0_8211) & 1 != 0
        {
            return false;
        }
        let profile = self.aircraft.profile();
        let distance_to_cap = (profile.speed * 6.2).clamp(92.0, 179.0);
        let mut best = None;
        let mut best_score = f32::NEG_INFINITY;
        for (_, district) in land.districts() {
            let Some(road) = district.road() else { continue };
            for end in [Terminus::Start, Terminus::End] {
                let Some(town) = district.town(end) else { continue };
                if town.building_count() < 14 || town.street_count() < 16 {
                    continue;
                }
                let points = road.polyline();
                let point = |index| {
                    points[if end == Terminus::Start { index } else { points.len() - 1 - index }]
                };
                let mut remaining = distance_to_cap;
                for i in 1..points.len() {
                    let a = point(i - 1);
                    let b = point(i);
                    let dx: f32 = b.x - a.x;
                    let dz: f32 = b.z - a.z;
                    let squared = dx * dx + dz * dz;
                    if squared < 0.0001 {
                        continue;
                    }
                    let length = distance(dx, dz);
                    if remaining > length {
                        remaining -= length;
                        continue;
                    }
                    let t = remaining / length;
                    let x = a.x + t * dx;
                    let z = a.z + t * dz;
                    let [left, near, right, far] = town.bounds();
                    let target_x = (left + right) * 0.5;
                    let target_z = (near + far) * 0.5;
                    let east = target_x - x;
                    let north = target_z - z;
                    let toward = east * east + north * north;
                    let drift_x = self.x - x;
                    let drift_z = self.z - z;
                    let travel = drift_x * drift_x + drift_z * drift_z;
                    if !(75.0 * 75.0..=245.0 * 245.0).contains(&toward)
                        || travel > 920.0 * 920.0
                        || -(east * dx + north * dz) < 0.55 * length * distance(east, north)
                    {
                        break;
                    }
                    let ground = surface(world, x, z);
                    let safe = (1..=8).all(|step| {
                        let phase = step as f32 / 8.0;
                        let height = surface(world, x + east * phase, z + north * phase);
                        height >= 1.4 && height < ground + 14.0 + phase * 12.0
                    });
                    if safe {
                        let score = f32::from(town.building_count()) * 0.40
                            + f32::from(town.tower_count()) * 1.5
                            - travel * 0.000018;
                        if score > best_score {
                            best = Some((x, z, bearing(east, north)));
                            best_score = score;
                        }
                    }
                    break;
                }
            }
        }
        let Some(approach) = best else { return false; };
        *self = Self::start_at(world, self.seed, self.aircraft, approach);
        self.town_approach_staged = true;
        true
    }


    /// A presentation pose one simulation step behind, with wrap-safe attitude.
    pub fn display_pose(&self, fraction: f32) -> Self {
        let blend = fraction.clamp(0.0, 1.0);
        if blend >= 1.0 { return self.clone(); }
        let line = |from: f32, to: f32| from + (to - from) * blend;
        let angle = |from: f32, to: f32| {
            if blend == 0.0 { from } else { wrap_angle(from + wrap_angle(to - from) * blend) }
        };
        let before = self.previous_pose;
        let mut visible = self.clone();
        visible.x = line(before.x, self.x);
        visible.z = line(before.z, self.z);
        visible.altitude = line(before.altitude, self.altitude);
        visible.heading = angle(before.heading, self.heading);
        visible.bank = line(before.bank, self.bank);
        visible.climb_rate = line(before.climb_rate, self.climb_rate);
        visible.view = View {
            x: line(before.view.x, self.view.x),
            z: line(before.view.z, self.view.z),
            altitude: line(before.view.altitude, self.view.altitude),
            heading: angle(before.view.heading, self.view.heading),
            pitch: line(before.view.pitch, self.view.pitch),
            bank: line(before.view.bank, self.view.bank),
        };
        visible
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn refueling_progress(&self) -> Option<f32> {
        (self.aircraft == Aircraft::Blackbird)
            .then(|| crate::encounters::blackbird_refuel(self.seed, self.ticks))
            .flatten()
            .map(|event| event.progress)
    }

    pub(crate) fn climb_rate(&self) -> f32 {
        self.climb_rate
    }

    pub(crate) fn seed(&self) -> u64 {
        self.seed
    }

    pub fn turn_rate(&self) -> f32 {
        self.turn
    }

    pub fn flying_coastal_pass(&self) -> bool {
        self.coastal_pass.is_some()
    }

    pub fn following_scenic_leg(&self) -> bool {
        self.scenic_progress.leg.is_some()
    }

    fn train_pose(&self) -> crate::train_follow::AircraftPose {
        crate::train_follow::AircraftPose {
            aircraft: self.aircraft, ticks: self.ticks, x: self.x, z: self.z,
            altitude: self.altitude, heading: self.heading,
        }
    }

    pub fn should_observe_trains(&self) -> bool {
        self.train_follow.should_observe(self.train_pose())
    }

    pub fn train_score(&self, train: crate::train_follow::TrainMotion) -> Option<f32> {
        self.train_follow.score(self.train_pose(), train)
    }

    pub fn train_pass_phase(&self) -> Option<crate::train_follow::TrainPassPhase> {
        self.train_follow.phase(self.ticks)
    }

    pub fn observe_train(&mut self, train: Option<crate::train_follow::TrainMotion>) {
        self.train_follow.observe(self.train_pose(), self.seed, train);
    }

    /// True from the surveyed crossing's approach through its dry landfall.
    pub fn following_fighter_crossing(&self) -> bool {
        self.aircraft == Aircraft::Fighter
            && self.scenic_progress.leg.is_some_and(|leg| leg.coast_channel.is_some())
    }

    /// True from the surveyed crossing's approach through its dry landfall.
    pub fn following_racer_crossing(&self) -> bool {
        self.aircraft == Aircraft::Racer
            && self.scenic_progress.leg.is_some_and(|leg| leg.coast_channel.is_some())
    }

    pub fn update(&mut self, world: &World) {
        self.update_with_obstacles(world, |_, _| 0.0);
    }

    pub fn update_with_obstacles(&mut self, world: &World, obstruction: impl Fn(f32, f32) -> f32) {
        self.previous_pose = Pose { x: self.x, z: self.z, altitude: self.altitude,
            heading: self.heading, bank: self.bank, climb_rate: self.climb_rate, view: self.view };
        let profile = self.aircraft.profile();
        let ridge_guidance = !matches!(self.aircraft, Aircraft::BaldEagle | Aircraft::Blackbird);
        self.ticks += 1;
        let blackbird = self.aircraft == Aircraft::Blackbird;
        let refuel = if blackbird { crate::encounters::blackbird_refuel(self.seed, self.ticks) } else { None };
        let long_cycle = (self.ticks % 21_600) as f32 / 21_600.0 * TAU;
        let meander = sin_cos(long_cycle * 5.0 + self.phase).0 * 0.24
            + sin_cos(long_cycle * 13.0 + self.phase * 2.0).0 * 0.10;
        // Keep the route in a large coordinate range where subpixel movement stays smooth.
        // At the edge the course turns inward gradually while the landscape stays continuous.
        let edge = self.x.abs().max(self.z.abs());
        let inward = if edge > 26_000.0 {
            let (sin, cos) = sin_cos(self.heading);
            (sin * self.z - cos * self.x) / edge * smoothstep((edge - 26_000.0) / 4_000.0)
        } else {
            0.0
        };
        if self.ticks.is_multiple_of(60) {
            let distance = 60.0 + profile.speed * 2.2;
            let left = ahead(world, self.x, self.z, self.heading - 0.42, distance);
            let right = ahead(world, self.x, self.z, self.heading + 0.42, distance);
            let center = ahead(world, self.x, self.z, self.heading, distance * 1.35);
            let toward_land = (1.0 - center / 8.0).clamp(0.0, 1.0);
            // At sea favor the nearest coast. Inland, let gentler slopes draw the route.
            let valley = (left - right).clamp(-45.0, 45.0) * 0.003;
            self.course = wrap_angle(self.course + meander * (0.12 + profile.yaw_rate * 0.30)
                + valley * (1.0 - 1.8 * toward_land) + inward * 0.68);
            self.track_terrain_variety(world, profile, edge);
            if self.aircraft == Aircraft::BushPlane && edge < 24_000.0 {
                self.track_bush_pass(world);
            }
            if !self.steer_scenic_leg(world, profile, edge) {
                let coast_before = self.course;
                self.track_coast(world, profile, edge);
                if matches!(self.aircraft, Aircraft::Fighter | Aircraft::Racer) {
                    let turns = &mut self.scenic_progress.coastal_lap_turns;
                    let bucket = ((self.ticks / 360) % turns.len() as u64) as usize;
                    if self.ticks.is_multiple_of(360) {
                        turns[bucket] = 0;
                        self.scenic_progress.coastal_lap_cells[bucket] =
                            visit_cell(self.x, self.z, 128.0);
                    }
                    let delta = wrap_angle(self.course - coast_before);
                    if delta.abs() > 0.70 {
                        turns[bucket] += if self.aircraft == Aircraft::Racer || delta > 0.0 { 1 } else { -1 };
                    }
                }
            }
        }
        let train = self.train_follow.guide(self.train_pose(), profile.speed);
        if ridge_guidance && self.aircraft != Aircraft::BushPlane && self.ticks.is_multiple_of(60)
            && edge < 24_000.0 && train.is_none() && self.scenic_progress.leg.is_none()
        {
            self.track_mountain_pass(world, profile);
        }
        if let Some(guide) = &train {
            let correction = wrap_angle(guide.heading - self.course).clamp(-0.025, 0.025);
            self.course = wrap_angle(self.course + correction * guide.blend);
        }
        let ordinary_turn = (wrap_angle(self.course - self.heading) * 0.72)
            .clamp(-profile.yaw_rate, profile.yaw_rate);
        let thermalling = self.update_air(world);
        let alignment = refuel.map_or(0.0, |event| event.alignment());
        let desired_turn = thermalling.unwrap_or(ordinary_turn) * (1.0 - alignment);
        self.turn += (desired_turn - self.turn) * profile.turn_response;
        self.heading = wrap_angle(self.heading + self.turn * DT);
        // Turn rate already has inertia. Derive roll from that same state so
        // starts and reversals do not leave the wings behind the turn.
        let (bank_limit, turn_limit) = if thermalling.is_some() {
            (0.58, 0.58 / 0.88)
        } else {
            (profile.max_bank, profile.yaw_rate)
        };
        self.bank = bank_limit * coordinated_bank(self.turn / turn_limit);

        let (sin, cos) = sin_cos(self.heading);
        let old_surface = surface(world, self.x, self.z);
        let terrain_and_objects = |x, z| surface(world, x, z).max(obstruction(x, z));
        let near = terrain_and_objects(self.x + sin * profile.speed, self.z + cos * profile.speed);
        let mid = terrain_and_objects(self.x + sin * profile.speed * 2.4, self.z + cos * profile.speed * 2.4);
        let far = terrain_and_objects(self.x + sin * profile.speed * 4.0, self.z + cos * profile.speed * 4.0);

        let mut envelope = old_surface.max(near).max(mid - profile.speed * 0.21)
            .max(far - profile.speed * 0.54);
        if self.turn.abs() > 0.09 {
            // During a stronger bank the future track is curved. Check both
            // corridors so a plane has time to climb before reaching a ridge.
            let predicted = |seconds, distance| {
                let (sin, cos) = sin_cos(self.heading + self.turn * seconds);
                terrain_and_objects(self.x + sin * distance, self.z + cos * distance)
            };
            let inner = predicted(0.65, profile.speed * 1.4);
            let outer = predicted(1.20, profile.speed * 2.7);
            envelope = envelope.max(inner - profile.speed * 0.12).max(outer - profile.speed * 0.29);
        }
        if ridge_guidance && train.is_none() {
            if self.ticks == 1 || self.ticks.is_multiple_of(15) {
                self.ridge_preview_floor = if self.aircraft == Aircraft::BushPlane {
                    self.bush_terrain_preview(world, profile.speed)
                } else {
                    self.ridge_terrain_preview(world, profile, old_surface)
                };
            }
            envelope = envelope.max(self.ridge_preview_floor);
        }
        let coastal_pass = if self.aircraft == Aircraft::SportJet {
            self.coastal_pass(world, old_surface, near, mid)
        } else {
            0.0
        };
        let desired = if self.aircraft == Aircraft::Blackbird {
            (profile.clearance + sin_cos(long_cycle).0 * 10.0).max(envelope + profile.floor + 10.0)
        } else {
            envelope + profile.clearance + self.energy_altitude - coastal_pass * 15.0
        };
        let desired = if let Some(guide) = &train {
            guide.altitude.map_or(desired, |height| {
                let safe = height.max(envelope + profile.floor + 1.5);
                desired + (safe - desired) * guide.blend
            })
        } else {
            desired
        };
        if blackbird {
            self.altitude = self.blackbird_cruise_altitude;
        }
        let max_climb = profile.climb;
        let climbing = ((desired - self.altitude) * 0.85).clamp(-max_climb * 0.58, max_climb);
        self.climb_rate += (climbing - self.climb_rate) * 0.12;
        self.altitude += self.climb_rate * DT;
        let circle_speed = if thermalling.is_some() { 0.91 } else { 1.0 };
        let energy_speed = (profile.speed * circle_speed - self.climb_rate * 0.17)
            .max(profile.speed * 0.78);
        let energy_speed = train.map_or(energy_speed, |guide| {
            energy_speed + (guide.speed - energy_speed) * guide.blend
        });
        self.speed += (energy_speed - self.speed) * 0.07;
        self.x += sin * self.speed * DT;
        self.z += cos * self.speed * DT;

        let floor = terrain_and_objects(self.x, self.z) + profile.floor;
        if self.altitude < floor {
            self.terrain_recoveries += 1;
            self.altitude = floor;
            self.climb_rate = self.climb_rate.max(0.0);
        }
        let mut visual_climb = self.climb_rate;
        if blackbird {
            // Keep the cruise controller independent of the descent so it
            // smoothly resumes its usual altitude after the rendezvous.
            self.blackbird_cruise_altitude = self.altitude;
            let drop = refuel.map_or(0.0, |event| 91.0 * event.descent());
            self.altitude -= drop.min((self.altitude - (envelope + profile.floor).max(floor) - 10.0).max(0.0));
            if drop > 0.0 {
                visual_climb = (self.altitude - self.previous_pose.altitude) / DT;
            }
        }
        self.update_view(world, profile, obstruction, visual_climb);
    }

    fn track_bush_pass(&mut self, world: &World) {
        let ground = surface(world, self.x, self.z);
        if ground < 6.0 || wrap_angle(self.course - self.heading).abs() > 0.68 {
            return;
        }
        let elevations = |course| {
            [55.0, 101.0, 146.0].map(|distance| ahead(world, self.x, self.z, course, distance))
        };
        let forward = elevations(self.course).into_iter().fold(ground, f32::max);
        if forward < ground + 31.0 || forward < self.altitude + 8.0 {
            return;
        }
        let mut best = (0.0_f32, forward);
        for offset in [-0.35_f32, 0.35, -0.62, 0.62] {
            let heading = self.course + offset;
            let lane = elevations(heading);
            if lane.into_iter().any(|height| height < 4.0) {
                continue;
            }
            let rim = [heading - 0.08, heading + 0.08].map(|direction| {
                ahead(world, self.x, self.z, direction, 111.0)
            });
            if rim.into_iter().any(|height| height < 4.0) {
                continue;
            }
            let score = lane.into_iter().chain(rim).fold(0.0, f32::max) + offset.abs() * 13.0;
            if score < best.1 - 9.0 {
                best = (offset, score);
            }
        }
        if best.1 + 15.0 < forward {
            self.course = wrap_angle(self.course + best.0.clamp(-0.22, 0.22));
        }
    }

    fn track_mountain_pass(&mut self, world: &World, profile: Profile) {
        let ground = surface(world, self.x, self.z);
        let desired_turn = wrap_angle(self.course - self.heading);
        if ground < 14.0 || desired_turn.abs() > 0.64 || self.thermal_turn != 0
            || world.coast_signal(self.x, self.z) < 0.24
        {
            return;
        }

        let speed = self.speed.max(profile.speed * 0.8);
        let corridor = |turn: f32| [3.5, 5.5, 7.5, 9.5].map(|seconds| {
            let blend = smoothstep((seconds - 1.0) / 8.5);
            ahead(world, self.x, self.z, self.heading + turn * blend, speed * seconds)
        });
        let forward = corridor(desired_turn).into_iter().fold(ground, f32::max);
        if forward < ground + 36.0 || forward + profile.floor < self.altitude + 13.0
            || !self.inland_ridge(world, profile.speed)
        {
            return;
        }
        let mut best = (desired_turn, forward);
        for offset in [-0.48_f32, 0.48, -0.77, 0.77] {
            let turn = (desired_turn + offset).clamp(-0.95, 0.95);
            let lane = corridor(turn);
            if lane.into_iter().any(|height| height < 4.0) {
                continue;
            }
            let side = ahead(world, self.x, self.z, self.heading + turn * 0.77, speed * 7.5);
            let peak = lane.into_iter().fold(side, f32::max);
            let score = peak + offset.abs() * 12.0;
            if side >= 4.0 && score < best.1 {
                best = (turn, score);
            }
        }
        if best.1 + 12.0 < forward {
            let correction = (best.0 - desired_turn).clamp(-0.32, 0.32);
            self.course = wrap_angle(self.course + correction);
        }
    }

    fn ridge_terrain_preview(&self, world: &World, profile: Profile, ground: f32) -> f32 {
        if ground < 14.0 || world.coast_signal(self.x, self.z) < 0.24 {
            return 0.0;
        }
        let turn = wrap_angle(self.course - self.heading).clamp(-0.72, 0.72);
        let speed = self.speed.max(profile.speed * 0.8);
        let preview = [(5.0, 0.48), (6.5, 0.66), (8.0, 0.80), (9.5, 0.90)].into_iter()
            .map(|(seconds, blend)| {
                let height = ahead(world, self.x, self.z, self.heading + turn * blend, speed * seconds);
                let rise = smoothstep((height - ground - 22.0) / 15.0);
                let projected = height - (seconds - 2.5) * (profile.climb * 0.20).clamp(2.8, 5.0);
                if rise > 0.0 { ground + rise * (projected - ground).max(0.0) } else { 0.0 }
            })
            .fold(0.0, f32::max);
        if preview < ground + 12.0 || !self.inland_ridge(world, profile.speed) {
            0.0
        } else {
            preview
        }
    }

    fn inland_ridge(&self, world: &World, speed: f32) -> bool {
        // A nearby coast keeps authority over its limited dry flight lanes.
        // Check only after the cheaper terrain look-ahead has found a ridge.
        const AXES: [(f32, f32); 8] = [
            (1.0, 0.0), (0.707, 0.707), (0.0, 1.0), (-0.707, 0.707),
            (-1.0, 0.0), (-0.707, -0.707), (0.0, -1.0), (0.707, -0.707),
        ];
        let outer = if self.aircraft == Aircraft::Fighter { speed * 48.0 } else { speed * 35.0 };
        [360.0, outer.max(660.0)].into_iter().all(|radius| AXES.into_iter().all(|(dx, dz)| {
            world.coast_signal(self.x + dx * radius, self.z + dz * radius) > 0.07
        }))
    }

    fn bush_terrain_preview(&self, world: &World, speed: f32) -> f32 {
        // The close obstacle envelope is still authoritative. A distant ridge
        // can start a climb while the Cub has room, with more allowance at range.
        let turn = wrap_angle(self.course - self.heading).clamp(-0.55, 0.55);
        [(5.5, 0.54), (7.5, 0.67), (9.5, 0.78)].into_iter()
            .map(|(seconds, blend)| {
                let ground = ahead(world, self.x, self.z, self.heading + turn * blend, speed * seconds);
                ground - (seconds - 2.25) * 4.25
            })
            .fold(0.0, f32::max)
    }

    fn coastal_pass(&mut self, world: &World, ground: f32, near: f32, mid: f32) -> f32 {
        if let Some((start, length)) = self.coastal_pass {
            let age = self.ticks - start;
            if age < length {
                let progress = age as f32 / length as f32;
                return smoothstep(progress / 0.31) * (1.0 - smoothstep((progress - 0.63) / 0.37));
            }
            self.coastal_pass = None;
        }
        if self.ticks < self.next_coastal_pass {
            return 0.0;
        }
        let second = u64::from(TICKS_PER_SECOND);
        self.next_coastal_pass = self.ticks + 9 * second;
        if ground > 28.0 || (near - ground).abs() > 10.0 || (mid - ground).abs() > 14.0 {
            return 0.0;
        }
        let left = coast_ahead(world, self.x, self.z, self.heading - 0.73, 134.0);
        let right = coast_ahead(world, self.x, self.z, self.heading + 0.73, 134.0);
        let ahead = coast_ahead(world, self.x, self.z, self.heading, 168.0);
        if left.min(right) < -0.08 && left.max(right) > 0.16 && ahead.abs() < 0.27 {
            let state = mix(self.seed ^ self.ticks);
            self.coastal_pass = Some((self.ticks, (14 + state % 6) * second));
            self.next_coastal_pass = self.ticks + (185 + state % 177) * second;
        }
        0.0
    }

    fn update_air(&mut self, world: &World) -> Option<f32> {
        if !matches!(self.aircraft, Aircraft::Glider | Aircraft::BaldEagle) {
            return None;
        }
        if self.ticks == 1 || self.ticks.is_multiple_of(12) {
            let sky = Sky::at_position(self.seed, self.ticks, [self.x, self.altitude, self.z]);
            let direct_light = sky.daylight * (1.0 - 0.60 * smoothstep((sky.cloud_cover - 0.43) / 0.42));
            self.air = Air::at(world, self.seed, self.x, self.z, self.altitude, direct_light);
        }
        let sink = if self.air.upward < 0.0 { 1.5 } else { 0.85 };
        let cap = if self.aircraft == Aircraft::Glider { 61.0 } else { 23.0 };
        let lift = if self.aircraft == Aircraft::Glider { 0.88 } else { 0.42 };
        self.energy_altitude = (self.energy_altitude + (self.air.upward * lift - sink) * DT)
            .clamp(0.0, cap);

        if self.aircraft != Aircraft::Glider {
            return None;
        }
        self.thermal_cooldown = self.thermal_cooldown.saturating_sub(1);
        let core = if self.air.lift == Lift::Thermal { self.air.thermal_center } else { None };
        if self.thermal_turn == 0 && self.thermal_cooldown == 0
            && core.is_some() && self.air.upward >= 3.1
            && self.altitude - surface(world, self.x, self.z) >= 29.0
        {
            self.thermal_turn = 465;
            self.thermal_cooldown = 2_700;
        }
        if self.thermal_turn == 0 {
            return None;
        }
        self.thermal_turn -= 1;
        let Some((cx, cz)) = core else { return None };
        let dx = self.x - cx;
        let dz = self.z - cz;
        let radial = ((dx * dx + dz * dz) / (22.0 * 22.0) - 1.0).clamp(-1.0, 1.0);
        let preferred = if self.seed & 2 == 0 { 1.0 } else { -1.0 };
        let orbit_x = preferred * dz - dx * radial;
        let orbit_z = -preferred * dx - dz * radial;
        let (sin, cos) = sin_cos(self.heading);
        let along = orbit_x * sin + orbit_z * cos;
        let lateral = orbit_x * cos - orbit_z * sin;
        let target = if along > 0.0 {
            0.63 * lateral / (lateral.abs() + along + 0.001)
        } else {
            0.60 * preferred
        };
        Some(target)
    }

    fn track_terrain_variety(&mut self, world: &World, profile: Profile, edge: f32) {
        if matches!(self.aircraft, Aircraft::SportJet | Aircraft::Fighter | Aircraft::Blackbird)
            || self.ticks < 46 * u64::from(TICKS_PER_SECOND)
        {
            return;
        }
        if edge > 24_000.0 || self.thermal_turn > 0 {
            self.route_intent.course = None;
            return;
        }
        let seconds = self.ticks / u64::from(TICKS_PER_SECOND);
        let stagger = self.seed % 7 * 2;
        if (seconds + stagger).is_multiple_of(14) {
            // One sample here plus at most three per bearing, every 14 seconds.
            self.route_intent.sample(world, self.x, self.z, self.heading, profile.speed);
        }
        if let Some(course) = self.route_intent.course {
            let shift = wrap_angle(course - self.course).clamp(-0.13, 0.13);
            self.course = wrap_angle(self.course + shift);
        }
    }

    /// Surveyed crossings guide the aircraft over approved water spans.
    /// Dry scenic legs retain ordinary coastal steering.
    fn steer_scenic_leg(&mut self, world: &World, profile: Profile, edge: f32) -> bool {
        let (cell_size, range, water_limit, stale, cadence, arrival, confinement, confined_for) =
            match self.aircraft {
                Aircraft::BaldEagle => (384.0, 760.0, 0.0, 112, 14, 76.0, 740.0, 0),
                Aircraft::Blackbird => (960.0, 3_070.0, 630.0, 120, 8, 290.0, 1_130.0, 104),
                Aircraft::SportJet if !self.flying_coastal_pass() => {
                    (480.0, 1_460.0, 0.0, 96, 10, 140.0, 900.0, 120)
                }
                // A thermal keeps control of the glider until its turn ends.
                Aircraft::Glider if self.thermal_turn == 0 => {
                    (384.0, 960.0, 0.0, 224, 14, 112.0, 1_050.0, 196)
                }
                Aircraft::Fighter => (576.0, 1_960.0, 0.0, 216, 12, 180.0, 2_050.0, 180),
                Aircraft::Racer => (384.0, 1_380.0, 0.0, 540, 12, 112.0, 1_290.0, 360),
                _ => return false,
            };
        let age = self.ticks / u64::from(TICKS_PER_SECOND);
        let progress = &mut self.scenic_progress;
        if age.is_multiple_of(cadence) {
            progress.mark(self.x, self.z, cell_size, confinement, age);
            if age > 180 && age >= progress.retry_at && age - progress.last_new >= stale
                && age - progress.local_since >= confined_for
                && progress.leg.is_none() && edge < 25_100.0
                && (self.aircraft != Aircraft::Fighter
                    || progress.repeated_coastal_lap(self.x, self.z, age, 576))
                && (self.aircraft != Aircraft::Racer
                    || (self.turn.abs() < 0.16
                        && progress.repeated_coastal_lap(self.x, self.z, age, 100)))
                && (self.aircraft == Aircraft::Blackbird || world.coast_signal(self.x, self.z) > 0.025)
            {
                progress.retry_at = age + 2 * cadence;
                let dry_target = progress.find_exit(
                    world, self.x, self.z, self.heading, cell_size, range, water_limit,
                    self.aircraft == Aircraft::Fighter,
                );
                if let Some(target) = dry_target.filter(|_| self.aircraft != Aircraft::Racer) {
                    let seconds = distance(target[0] - self.x, target[1] - self.z) / profile.speed;
                    let stop = age + seconds as u64 + 42;
                    if self.aircraft != Aircraft::Fighter || fighter_formation_clear(self.seed, age, stop) {
                        progress.leg = Some(ScenicLeg {
                            target, stop, water_seconds: 0, coast_channel: None,
                        });
                    }
                } else if dry_target.is_none() {
                    let channel_aircraft = match self.aircraft {
                        Aircraft::Fighter if age - progress.last_new >= 396
                            && progress.repeated_coastal_lap(self.x, self.z, age, 64)
                            && fighter_formation_clear(
                                self.seed, age, age + ChannelAircraft::Fighter.max_leg_seconds(),
                            ) => Some(ChannelAircraft::Fighter),
                        Aircraft::Racer => Some(ChannelAircraft::Racer),
                        _ => None,
                    };
                    if let Some(aircraft) = channel_aircraft {
                        if let Some(channel) = CoastChannel::scout(world, [self.x, self.z], self.heading, aircraft) {
                            progress.leg = Some(ScenicLeg {
                                target: channel.inland_target,
                                stop: age + aircraft.max_leg_seconds(),
                                water_seconds: 0,
                                coast_channel: Some(channel),
                            });
                        }
                    }
                }
            }
        }
        let Some(leg) = &mut progress.leg else { return false; };
        if let Some(channel) = leg.coast_channel {
            let guidance = channel.guidance();
            let [sin, cos] = channel.forward;
            let x = self.x - channel.origin[0];
            let z = self.z - channel.origin[1];
            let along = x * sin + z * cos;
            let lateral = x * cos - z * sin;
            let over_water = world.coast_signal(self.x, self.z) <= 0.0;
            if over_water {
                leg.water_seconds += 2;
            }
            let near_water = (channel.wet_span[0] - guidance.near_water[0]
                ..=channel.wet_span[1] + guidance.near_water[1]).contains(&along);
            let lateral_limit = if near_water {
                guidance.near_water_lateral_limit
            } else {
                guidance.dry_lateral_limit
            };
            let unexpected_water = over_water && !(channel.wet_span[0] - guidance.wet_margin
                ..=channel.wet_span[1] + guidance.wet_margin).contains(&along);
            if age >= leg.stop || edge >= 25_100.0 || lateral.abs() > lateral_limit
                || leg.water_seconds > guidance.max_water_seconds || unexpected_water
            {
                progress.leg = None;
                progress.retry_at = age + 28;
                return false;
            }
            if along < channel.inland_handoff {
                let goal = bearing(
                    guidance.follow_distance * sin - lateral * cos,
                    guidance.follow_distance * cos + lateral * sin,
                );
                let correction = wrap_angle(goal - self.course)
                    .clamp(-guidance.max_course_change, guidance.max_course_change);
                self.course = wrap_angle(self.course + correction);
                return true;
            }
            leg.coast_channel = None;
            leg.water_seconds = 0;
        }
        let dx = leg.target[0] - self.x;
        let dz = leg.target[1] - self.z;
        if world.coast_signal(self.x, self.z) <= 0.0 {
            leg.water_seconds += 2;
        } else {
            leg.water_seconds = 0;
        }
        if age >= leg.stop || dx * dx + dz * dz <= arrival * arrival
            || leg.water_seconds > (water_limit / profile.speed) as u8 + 8
            || edge > 25_500.0
        {
            progress.leg = None;
            progress.retry_at = age + 28;
            return false;
        }
        let goal = bearing(dx, dz);
        let correction = wrap_angle(goal - self.course).clamp(-0.36, 0.36);
        self.course = wrap_angle(self.course + correction);
        self.aircraft == Aircraft::Blackbird
    }

    fn track_coast(&mut self, world: &World, profile: Profile, edge: f32) {
        let speed = profile.speed;
        let near = (speed * (9.0 + speed * 0.095)).max(73.0);
        let ok = coast_ahead(world, self.x, self.z, self.course, 0.55 * near)
            .min(coast_ahead(world, self.x, self.z, self.course, near));
        if ok > 0.018 {
            return;
        }
        let here = world.coast_signal(self.x, self.z);
        // Bank toward a coast early enough for the aircraft's speed and turn radius.
        let score = |direction: f32| {
            let (sin, cos) = sin_cos(direction);
            let point = |distance: f32| (self.x + sin * distance, self.z + cos * distance);
            let (x, z) = point(near * 0.58);
            let short = world.coast_signal(x, z);
            let (x, z) = point(near);
            let mid = world.coast_signal(x, z);
            let (x, z) = point(near * 1.68);
            let far = world.coast_signal(x, z);
            let edging = if edge > 25_000.0 {
                (x.abs().max(z.abs()) - edge).max(0.0) * 0.00063
            } else {
                0.0
            };
            let dryness = short.min(0.24) * 0.50 + mid.min(0.27) * 0.36 + far.min(0.31) * 0.14;
            dryness - edging - wrap_angle(direction - self.course).abs() * 0.025
        };
        let mut best_direction = self.course;
        let mut best_score = score(self.course);
        for turn in [-1.45_f32, -0.93, -0.51, 0.51, 0.93, 1.45] {
            let direction = self.heading + turn;
            let candidate = score(direction);
            if candidate > best_score + 0.012 {
                best_score = candidate;
                best_direction = direction;
            }
        }
        if here < 0.035 && best_score < 0.040 {
            for turn in [-2.05_f32, 2.05, -2.58, 2.58, core::f32::consts::PI] {
                let direction = self.heading + turn;
                let candidate = score(direction);
                if candidate > best_score + 0.018 {
                    best_score = candidate;
                    best_direction = direction;
                }
            }
        }
        self.course = wrap_angle(best_direction);
    }

    fn update_view(&mut self, world: &World, profile: Profile, obstruction: impl Fn(f32, f32) -> f32, visual_climb: f32) {
        self.view.heading = wrap_angle(self.view.heading
            + wrap_angle(self.heading - self.view.heading) * 0.095);
        let (sin, cos) = sin_cos(self.view.heading);
        self.view.x = self.x - sin * profile.chase;
        self.view.z = self.z - cos * profile.chase;
        let desired_height = self.altitude + profile.chase * CHASE_HEIGHT_RATIO;
        self.view.altitude += (desired_height - self.view.altitude) * 0.14;
        let ground = surface(world, self.view.x, self.view.z).max(obstruction(self.view.x, self.view.z));
        self.view.altitude = self.view.altitude.max(ground + 5.0);
        let base_pitch = if self.aircraft == Aircraft::Blackbird { BLACKBIRD_PITCH } else { 0.15 };
        self.view.pitch += (base_pitch - visual_climb * 0.002 - self.view.pitch) * 0.12;
        self.view.bank += (self.bank * 0.24 - self.view.bank) * 0.086;
    }
}

pub struct Game {
    seed_state: u64,
    controls: Controls,
    aircraft: Aircraft,
    world_seed: u64,
    world: Option<World>,
    flight: Option<Flight>,
    flight_info: bool,
    startup_untouched: bool,
}

fn distance(east: f32, north: f32) -> f32 {
    let squared = east * east + north * north;
    let mut estimate = east.abs().max(north.abs());
    for _ in 0..3 {
        estimate = 0.5 * (estimate + squared / estimate);
    }
    estimate
}

pub(crate) fn bearing(east: f32, north: f32) -> f32 {
    let x = east.abs();
    let z = north.abs();
    let ratio = x.min(z) / x.max(z);
    let acute = ratio / (1.0 + 0.28 * ratio * ratio);
    let angle = if x <= z { acute } else { FRAC_PI_2 - acute };
    let angle = if north >= 0.0 { angle } else { PI - angle };
    angle.copysign(east)
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Self {
            seed_state: seed,
            controls: Controls::default(),
            aircraft: Aircraft::BushPlane,
            world_seed: 0,
            world: None,
            flight: None,
            flight_info: false,
            startup_untouched: true,
        }
    }

    pub fn screen(&self) -> Screen {
        if self.flight.is_some() { Screen::Flight } else { Screen::AircraftPicker }
    }

    pub fn selected(&self) -> Aircraft {
        self.aircraft
    }

    pub fn stage_town_approach<const N: usize>(&mut self, land: &LandStream<N>) -> bool {
        let (Some(world), Some(flight)) = (&self.world, &mut self.flight) else { return false; };
        flight.stage_town_approach(world, land)
    }

    pub fn active(&self) -> Option<(&World, &Flight)> {
        Some((self.world.as_ref()?, self.flight.as_ref()?))
    }

    pub fn world_seed(&self) -> Option<u64> {
        self.flight.as_ref().map(|_| self.world_seed)
    }

    pub fn flight_info_visible(&self) -> bool {
        self.flight_info
    }

    pub fn new_world_progress(&self, now: u32) -> Option<f32> {
        self.controls.new_world_progress(now)
    }

    pub fn reset_buttons(&mut self) {
        self.controls = Controls::default();
        self.startup_untouched = false;
    }

    /// Launch once from the boot picker if neither physical button was touched.
    pub fn launch_if_boot_idle(&mut self, elapsed_ms: u64) -> bool {
        if !self.startup_untouched || self.flight.is_some() || elapsed_ms < 10_000 {
            return false;
        }
        let choice = mix(self.seed_state ^ 0x300c_730f_5543_9593) as usize % Aircraft::ALL.len();
        self.aircraft = Aircraft::ALL[choice];
        self.start_flight();
        true
    }

    fn start_flight(&mut self) {
        self.startup_untouched = false;
        self.seed_state = self.seed_state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        self.world_seed = mix(self.seed_state);
        let world = World::new(self.world_seed);
        self.flight = Some(Flight::start(&world, self.world_seed, self.aircraft));
        self.world = Some(world);
        self.flight_info = false;
    }

    pub fn read_buttons(&mut self, now: u32, buttons: Buttons) -> Option<Command> {
        if buttons.small || buttons.large {
            self.startup_untouched = false;
        }
        let command = self.controls.update(now, buttons, self.screen())?;
        match command {
            Command::NextAircraft => self.aircraft = self.aircraft.next(),
            Command::Fly => self.start_flight(),
            Command::ToggleFlightInfo => self.flight_info = !self.flight_info,
            Command::ReturnToPicker => {
                self.flight = None;
                self.world = None;
                self.flight_info = false;
            }
        }
        Some(command)
    }

    pub fn tick(&mut self) {
        self.tick_with_obstacles(|_, _| 0.0);
    }

    pub fn observe_train(&mut self, train: Option<crate::train_follow::TrainMotion>) {
        if let Some(flight) = &mut self.flight {
            flight.observe_train(train);
        }
    }

    pub fn tick_with_obstacles(&mut self, obstruction: impl Fn(f32, f32) -> f32) {
        if let (Some(world), Some(flight)) = (&self.world, &mut self.flight) {
            flight.update_with_obstacles(world, obstruction);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_idle_launches_once_and_a_raw_press_cancels_it() {
        let mut choices = [false; 8];
        for seed in 0..40 {
            let mut game = Game::new(seed);
            assert!(!game.launch_if_boot_idle(9_999));
            assert!(game.launch_if_boot_idle(10_000));
            choices[game.selected() as usize] = true;
            let hold = Buttons { small: true, large: false };
            for at in [10_100, 10_140, 11_400] { game.read_buttons(at, hold); }
            assert_eq!(game.screen(), Screen::AircraftPicker);
            assert!(!game.launch_if_boot_idle(30_000));
        }
        assert!(choices.into_iter().all(|seen| seen));
        for buttons in [Buttons { small: true, large: false }, Buttons { small: false, large: true }] {
            let mut game = Game::new(41);
            game.read_buttons(9_999, buttons);
            assert!(!game.launch_if_boot_idle(10_000));
            assert_eq!(game.screen(), Screen::AircraftPicker);
        }
    }

    #[test]
    fn glider_takes_a_dry_exit_after_revisiting_the_same_area() {
        let world = World::new(33);
        let mut flight = Flight::start(&world, 33, Aircraft::Glider);
        let hz = u64::from(TICKS_PER_SECOND);
        let target = (0..520 * hz).find_map(|_| {
            let thermalling = flight.thermal_turn > 0;
            flight.update(&world);
            flight.scenic_progress.leg.map(|leg| {
                assert!(!thermalling);
                leg.target
            })
        });
        let [x, z] = target.expect("repeated coastal terrain should have a surveyed exit");
        assert!((430..500).contains(&(flight.ticks / hz)));
        assert!(world.coast_signal(x, z) > 0.14);
        let [start_x, start_z] = [flight.x, flight.z];
        for _ in 0..90 * hz {
            flight.update(&world);
        }
        assert!(distance(flight.x - start_x, flight.z - start_z) > 500.0);
        assert_eq!(flight.terrain_recoveries, 0);
    }

    #[test]
    fn eagle_changes_only_departures_without_nearby_land() {
        let seed = 13_647_215_125_184_110_592;
        let world = World::new(seed);
        let eagle = Aircraft::BaldEagle;
        let (x, z, _) = launch_point(&world, seed, eagle.profile());
        assert_eq!(world.sample(x, z).surface, Surface::DeepWater);
        let flight = Flight::start(&world, seed, eagle);
        assert!(world.elevation(flight.x, flight.z) > SEA_LEVEL);

        // This heading points away from a nearby coast. It still has land
        // within the eagle's local reach, though not in the forward wedge.
        let coast = (-630.0, -1596.0, 0.0);
        assert!(wider_land_departure(&world, seed, Aircraft::BushPlane, coast).is_some());
        assert!(wider_land_departure(&world, seed, eagle, coast).is_none());
    }

    #[test]
    fn isolated_open_water_launch_finds_a_broad_dry_departure() {
        let seed = 13_647_215_125_184_110_592;
        let world = World::new(seed);
        for aircraft in [
            Aircraft::BushPlane, Aircraft::Racer, Aircraft::SportJet,
            Aircraft::Trike, Aircraft::Glider, Aircraft::Blackbird,
        ] {
            let profile = aircraft.profile();
            let (x, z, _) = launch_point(&world, seed, profile);
            assert_eq!(surface(&world, x, z), SEA_LEVEL);
            let mut flight = Flight::start(&world, seed, aircraft);
            assert!(world.coast_signal(flight.x, flight.z) >= 0.13);
            for distance in [110.0, 220.0] {
                assert!(coast_ahead(&world, flight.x, flight.z, flight.heading, distance) >= 0.11);
            }
            for _ in 0..2 * TICKS_PER_SECOND {
                flight.update(&world);
            }
            assert_eq!(flight.terrain_recoveries, 0);
        }
        let ordinary = launch_point(&world, seed, Aircraft::Fighter.profile());
        let fighter = Flight::start(&world, seed, Aircraft::Fighter);
        assert_eq!((fighter.x, fighter.z, fighter.heading), ordinary);
    }

    #[test]
    fn blackbird_descends_for_refueling_and_resumes_cruise() {
        let seed = 1;
        let world = World::new(seed);
        let mut flight = Flight::start(&world, seed, Aircraft::Blackbird);
        let mut lowest = f32::INFINITY;
        let mut refueled = false;
        for _ in 0..15_500 {
            flight.update(&world);
            if let Some(progress) = flight.refueling_progress() {
                assert_eq!(crate::encounters::blackbird_reheat(seed, flight.ticks), 0.0);
                assert!(flight.speed() > 56.0);
                assert!(flight.altitude > world.elevation(flight.x, flight.z) + 87.0);
                lowest = lowest.min(flight.altitude);
                if (0.46..0.65).contains(&progress) {
                    assert!(flight.turn_rate().abs() < 0.002 && flight.bank.abs() < 0.005);
                    assert!(flight.view.bank.abs() < 0.024);
                }
                refueled |= progress > 0.65;
            }
        }
        assert!(refueled && lowest < 245.0);
        assert!(flight.altitude > 310.0);
        assert_eq!(flight.terrain_recoveries, 0);
    }

    #[test]
    fn blackbird_leaves_a_repeated_coastal_loop_in_a_real_world() {
        // Refueling straightens the route, so the former seed no longer
        // reaches the same confined coastal loop during the test window.
        let seed = 41;
        let world = World::new(seed);
        let mut flight = Flight::start(&world, seed, Aircraft::Blackbird);
        let hz = u64::from(TICKS_PER_SECOND);
        let target = (0..4900 * hz).find_map(|_| {
            flight.update(&world);
            flight.scenic_progress.leg.map(|leg| leg.target)
        });
        let [x, z] = target.expect("repeated coast should have a surveyed exit");
        assert!(flight.following_scenic_leg());
        assert!((4_700..4_900).contains(&(flight.ticks / hz)));
        assert!(world.coast_signal(x, z) > 0.14);

        let origin = [flight.x, flight.z];
        for _ in 0..75 * hz {
            flight.update(&world);
            assert!((310.999..=331.001).contains(&flight.altitude));
        }
        let dx = flight.x - origin[0];
        let dz = flight.z - origin[1];
        assert!(dx * dx + dz * dz > 1_100.0 * 1_100.0);
        assert_eq!(flight.terrain_recoveries, 0);
    }

    #[test]
    fn racer_leaves_an_isolated_island_with_clearance() {
        let seed = 0x310b_e5cb_306a_57a0;
        let profile = Aircraft::Racer.profile();
        let world = World::new(seed);
        let mut flight = Flight::start(&world, seed, Aircraft::Racer);
        let mut crossing_start = None;
        let mut wet_ticks = 0;
        for _ in 0..35_000 {
            flight.update(&world);
            if flight.following_racer_crossing() {
                crossing_start.get_or_insert(flight.ticks);
                assert!(flight.altitude >= world.elevation(flight.x, flight.z) + profile.floor);
                wet_ticks += u32::from(world.coast_signal(flight.x, flight.z) <= 0.0);
            } else if crossing_start.is_some() {
                assert!(world.coast_signal(flight.x, flight.z) > 0.025);
                break;
            }
        }
        assert_eq!(crossing_start, Some(30_240));
        assert!((180..=360).contains(&wet_ticks));
        assert!(!flight.following_racer_crossing());
        assert_eq!(flight.terrain_recoveries, 0);
    }

    #[test]
    fn bank_scales_from_a_slight_lean_to_each_aircraft_limit() {
        let mut previous = 0.0;
        for sample in 0..=100 {
            let turn = sample as f32 * 0.01;
            let current = coordinated_bank(turn);
            assert!(current >= previous && current <= 1.0);
            assert_eq!(coordinated_bank(-turn), -current);
            previous = current;
        }
        for aircraft in Aircraft::ALL {
            let limit = aircraft.profile().max_bank;
            assert!(limit * coordinated_bank(0.025) < 0.02);
        }
        let sport = Aircraft::SportJet.profile();
        assert!(sport.max_bank.to_degrees() < 21.0);
        assert!(sport.yaw_rate.to_degrees() < 15.0);
    }

    #[test]
    fn shallow_turns_bank_with_the_heading_in_both_directions() {
        let world = World::new(11);
        for aircraft in Aircraft::ALL {
            let profile = aircraft.profile();
            let mut flight = Flight::start(&world, 11, aircraft);
            for sign in [-1.0, 1.0] {
                flight.course = wrap_angle(flight.heading + sign * profile.yaw_rate * (0.13 / 0.72));
                for _ in 0..28 {
                    let previous_heading = flight.heading;
                    flight.update(&world);
                    let yaw = wrap_angle(flight.heading - previous_heading);
                    assert_eq!(yaw.signum(), flight.bank.signum(), "{}", aircraft.name());
                }
                assert!(flight.bank * sign > profile.max_bank * 0.05, "{}", aircraft.name());
                assert!(flight.view.bank.abs() < flight.bank.abs());
            }
        }
    }

    #[test]
    fn racer_gains_room_before_reaching_an_inland_ridge() {
        let world = World::new(7);
        let mut flight = Flight::start(&world, 7, Aircraft::Racer);
        let starting_altitude = flight.altitude;
        let starting_heading = flight.heading;
        for _ in 0..90 {
            flight.update(&world);
        }
        assert!(flight.altitude > starting_altitude + 35.0);
        assert!(wrap_angle(flight.heading - starting_heading).abs() > 0.16);
        assert_eq!(flight.terrain_recoveries, 0);
    }

    #[test]
    fn cub_climbs_and_turns_early_on_a_real_ridge_approach() {
        let seed = 691;
        let world = World::new(seed);
        let mut cub = Flight::start(&world, seed, Aircraft::BushPlane);
        let start = cub.altitude;
        for _ in 0..120 {
            cub.update(&world);
        }
        let forward_peak = [47.0, 64.0, 87.0, 113.0].into_iter()
            .map(|distance| ahead(&world, cub.x, cub.z, cub.heading, distance))
            .fold(0.0_f32, f32::max);
        assert!(cub.altitude >= start + 30.0);
        assert!(cub.altitude > forward_peak + 10.0);
        assert!(cub.heading < -0.2);
        assert_eq!(cub.terrain_recoveries, 0);
    }

    #[test]
    fn fighter_leaves_a_repeated_coastal_lap_without_interrupting_formation() {
        let seed = 312;
        let aircraft = Aircraft::Fighter;
        let world = World::new(seed);
        let mut flight = Flight::start(&world, seed, aircraft);
        let hz = u64::from(TICKS_PER_SECOND);
        let target = (0..6_200 * hz).find_map(|_| {
            flight.update(&world);
            flight.scenic_progress.leg.map(|leg| leg.target)
        });
        let [x, z] = target.expect("the repeated Fighter lap should have a dry exit");
        assert!((6_000..6_200).contains(&(flight.ticks / hz)));
        assert!(world.coast_signal(x, z) > 0.30);

        let origin = [flight.x, flight.z];
        for _ in 0..130 * hz {
            if !flight.following_scenic_leg() {
                break;
            }
            assert!(crate::encounters::formation(aircraft, seed, flight.ticks).is_none());
            assert!(world.coast_signal(flight.x, flight.z) > 0.0);
            flight.update(&world);
        }
        assert!(!flight.following_scenic_leg());
        assert!(distance(flight.x - origin[0], flight.z - origin[1]) > 850.0);
        assert_eq!(flight.terrain_recoveries, 0);
    }

    #[test]
    fn fighter_leaves_an_island_through_one_short_channel() {
        let seed = 0xfd85_920c_ec67_fd5b;
        let world = World::new(seed);
        let mut flight = Flight::start(&world, seed, Aircraft::Fighter);
        let mut first_crossing = None;
        let mut water_ticks = 0;
        let mut landed = false;
        for _ in 0..68_000 {
            flight.update(&world);
            if flight.following_fighter_crossing() {
                first_crossing.get_or_insert(flight.ticks);
                assert!(crate::encounters::formation(flight.aircraft, seed, flight.ticks).is_none());
                water_ticks += u32::from(world.coast_signal(flight.x, flight.z) <= 0.0);
            } else if first_crossing.is_some() {
                landed = world.coast_signal(flight.x, flight.z) > 0.025;
                break;
            }
        }
        assert_eq!(first_crossing, Some(64_800));
        assert!((30..=150).contains(&water_ticks));
        assert!(landed);
        assert_eq!(flight.terrain_recoveries, 0);
    }

    #[test]
    fn long_passive_flight_only_surveys_after_the_opening() {
        let seed = 5_574_532_911_583_637_595;
        let world = World::new(seed);
        let mut flight = Flight::start(&world, seed, Aircraft::BushPlane);
        let mut high = Flight::start(&world, seed, Aircraft::Blackbird);
        let hz = u64::from(TICKS_PER_SECOND);
        for _ in 0..45 * hz {
            flight.update(&world);
            high.update(&world);
            assert!(flight.route_intent.passed[0].is_none());
        }
        let mut surveys = 0;
        for _ in 45 * hz..120 * hz {
            let previous = flight.route_intent.passed[0];
            flight.update(&world);
            high.update(&world);
            if flight.route_intent.passed[0] != previous {
                assert!((flight.ticks / hz + seed % 7 * 2).is_multiple_of(14));
                surveys += 1;
            }
        }
        assert!((5..=6).contains(&surveys));
        assert!(high.route_intent.passed[0].is_none());
        assert_eq!(flight.terrain_recoveries, 0);
    }

    #[test]
    fn staged_approach_uses_a_prepared_town_road_once() {
        let mut game = Game::new(0);
        let fly = Buttons { small: false, large: true };
        game.read_buttons(0, fly);
        assert_eq!(game.read_buttons(30, fly), Some(Command::Fly));
        let seed = game.world_seed().unwrap();
        let (world, flight) = game.active().unwrap();
        let world = *world;
        let v = flight.view;
        let original = (flight.x, flight.z);
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(!game.stage_town_approach(&land));
        assert!(land.preflight(crate::render::Camera {
            x: v.x, z: v.z, altitude: v.altitude, heading: v.heading, pitch: v.pitch, bank: v.bank,
        }, false));
        assert!(game.stage_town_approach(&land));
        let (_, flight) = game.active().unwrap();
        let approach = (flight.x, flight.z);
        assert_ne!(approach, original);
        assert_eq!(game.world_seed(), Some(seed));
        assert_eq!(flight.ticks, 0);
        assert!(land.districts().any(|(_, district)| district.towns().next().is_some()
            && district.road().is_some_and(|road| road.proximity(approach.0, approach.1).is_some())));
        assert!(!game.stage_town_approach(&land));
        let mut high = Flight::start(&world, seed, Aircraft::Blackbird);
        assert!(!high.stage_town_approach(&world, &land));
    }

    #[test]
    fn presentation_follows_the_short_turn_across_the_heading_boundary() {
        let world = World::new(167);
        let mut flight = Flight::start(&world, 167, Aircraft::BaldEagle);
        flight.previous_pose.view.heading = 3.10;
        flight.previous_pose.view.x = 12.0;
        flight.view.heading = -3.10;
        flight.view.x = 18.0;
        let mid = flight.display_pose(0.5);
        assert!((mid.view.heading.abs() - core::f32::consts::PI).abs() < 0.0001);
        assert_eq!(mid.view.x, 15.0);
        assert_eq!(flight.display_pose(0.0).view.heading, 3.10);
        assert_eq!(flight.display_pose(1.0).view.heading, -3.10);
    }

    #[test]
    fn fresh_flight_keeps_selection_and_generates_a_new_seed() {
        let mut game = Game::new(2277);
        let next = Buttons { small: true, large: false };
        let fly = Buttons { small: false, large: true };
        let up = Buttons::default();
        game.read_buttons(0, next);
        game.read_buttons(30, next);
        assert_eq!(game.selected(), Aircraft::Racer);
        game.read_buttons(60, fly);
        game.read_buttons(90, fly);
        let old_seed = game.world_seed().unwrap();
        game.read_buttons(130, up);
        game.read_buttons(160, up);
        game.read_buttons(700, next);
        game.read_buttons(740, next);
        assert_eq!(game.read_buttons(1900, next), Some(Command::ReturnToPicker));
        assert_eq!(game.selected(), Aircraft::Racer);
        assert_eq!(game.read_buttons(1950, next), None);
        game.read_buttons(2000, fly);
        game.read_buttons(2030, fly);
        assert_eq!(game.active().unwrap().1.aircraft, Aircraft::Racer);
        assert_ne!(game.world_seed().unwrap(), old_seed);
        assert!(!game.flight_info_visible());
    }

    #[test]
    fn ratings_reflect_the_actual_flight_profiles() {
        let speeds = [
            Aircraft::BaldEagle, Aircraft::Trike, Aircraft::Glider, Aircraft::BushPlane,
            Aircraft::Racer, Aircraft::SportJet, Aircraft::Fighter, Aircraft::Blackbird,
        ];
        for pair in speeds.windows(2) {
            assert!(pair[0].profile().speed < pair[1].profile().speed);
            assert!(pair[0].ratings()[0] <= pair[1].ratings()[0]);
        }
        assert_eq!(Aircraft::BaldEagle.ratings()[0], 0);
        assert_eq!(Aircraft::Racer.ratings()[1], 5);
        assert_eq!(Aircraft::Blackbird.ratings(), [5, 1, 5]);
        assert!(Aircraft::SportJet.profile().clearance < Aircraft::Fighter.profile().clearance);
    }

    #[test]
    fn varied_aircraft_follow_terrain_safely() {
        for seed in [9, 714, 0x0371_9041, 0xfa8e_a155_cba7_4daa] {
            let world = World::new(seed);
            for aircraft in Aircraft::ALL {
                let mut flight = Flight::start(&world, seed, aircraft);
                for _ in 0..5_400 {
                    flight.update(&world);
                    assert!(flight.altitude - surface(&world, flight.x, flight.z) >= aircraft.profile().floor - 0.001);
                    assert!(flight.view.altitude >= surface(&world, flight.view.x, flight.view.z) + 4.999);
                }
                assert!(flight.terrain_recoveries < 12, "{seed:x} {aircraft:?} {}", flight.terrain_recoveries);
            }
        }
    }
}
