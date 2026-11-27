//! Infrequent ambient events. The schedule belongs to the world, not the camera.

use core::f32::consts::TAU;

use crate::flight::{Aircraft, TICKS_PER_SECOND};
use crate::math::{fraction, mix, sin_cos, smoothstep};
use crate::sky::Sky;

#[derive(Clone, Copy, Debug)]
pub struct Wingmate {
    pub right: f32,
    pub forward: f32,
    pub above: f32,
    pub bank: f32,
    pub heading_offset: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Formation {
    pub count: usize,
    pub aircraft: [Wingmate; 4],
    pub progress: f32,
    pub pattern: FormationPattern,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormationPattern {
    Wedge,
    LineAbreast,
    Echelon,
}

pub fn formation(aircraft: Aircraft, seed: u64, ticks: u64) -> Option<Formation> {
    if aircraft != Aircraft::Fighter {
        return None;
    }
    let timing = event_time(seed ^ 0x19e7_a839, ticks, 9 * 60, 2, 7, 135, 11.5, 6.75)?;
    let state = timing.state;
    let p = timing.progress;
    let pattern = match (state >> 8) % 3 {
        0 => FormationPattern::Wedge,
        1 => FormationPattern::LineAbreast,
        _ => FormationPattern::Echelon,
    };
    let count = if pattern == FormationPattern::Echelon || state & 1 == 0 { 2 } else { 4 };
    let entry = smoothstep(p / 0.28);
    let tilt = sin_cos(p * TAU + fraction(mix(state)) * TAU).0 * 0.08;
    let order = if state & 4 == 0 { 1.0 } else { -1.0 };
    let aircraft = core::array::from_fn(|index| {
        let echelon = pattern == FormationPattern::Echelon;
        let side = if echelon || index & 1 == 0 { order } else { -order };
        let rank = if echelon { index as f32 } else { (index / 2) as f32 };
        let (right, forward, above) = match pattern {
            FormationPattern::Wedge => (6.8 + rank * 4.2, 7.0 - rank * 4.5, 0.6 + rank * 1.2),
            FormationPattern::LineAbreast => (8.3 + rank * 6.2, 10.6, 0.8),
            FormationPattern::Echelon => (8.2 + rank * 5.2, 5.7 + rank * 7.3, 1.3 + rank * 2.8),
        };
        let peel_delay = if echelon { rank * 0.12 } else { 0.0 };
        let exit = smoothstep((p - 0.66 - peel_delay) / (0.34 - peel_delay));
        let trail = (1.0 - entry) * (27.0 + rank * 8.0);
        let separation = exit * (35.0 + rank * 9.0);
        Wingmate {
            right: side * (right + trail + separation),
            forward: forward + (1.0 - entry) * 10.3 + exit * (22.0 + rank * 6.0),
            above: above + (1.0 - entry) * 2.0 + exit * (7.0 + rank * 2.0),
            bank: tilt + side * (exit * 0.54 - (1.0 - entry) * 0.16),
            heading_offset: side * (exit * 0.47 - (1.0 - entry) * 0.21),
        }
    });
    Some(Formation { count, aircraft, progress: p, pattern })
}

/// Occasional visible reheat while the Blackbird is at cruise.
pub fn blackbird_reheat(seed: u64, ticks: u64) -> f32 {
    let Some(event) = event_time(seed ^ 0x448d_73a9, ticks, 7 * 60, 2, 3, 95, 4.8, 2.8)
    else {
        return 0.0;
    };
    smoothstep(event.progress / 0.18) * (1.0 - smoothstep((event.progress - 0.64) / 0.36))
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Refuel {
    pub progress: f32,
    pub side: f32,
}

impl Refuel {
    pub fn alignment(self) -> f32 {
        smoothstep((self.progress - 0.10) / 0.27)
            * (1.0 - smoothstep((self.progress - 0.72) / 0.23))
    }

    pub fn descent(self) -> f32 {
        smoothstep(self.progress / 0.35) * (1.0 - smoothstep((self.progress - 0.70) / 0.30))
    }

    pub fn speed_match(self) -> f32 {
        smoothstep((self.progress - 0.04) / 0.31)
            * (1.0 - smoothstep((self.progress - 0.78) / 0.22))
    }
}

/// One opportunity in three reheat windows, with the same two-in-three
/// chance per opportunity. The early slot ends before reheat can begin.
pub(crate) fn blackbird_refuel(seed: u64, ticks: u64) -> Option<Refuel> {
    let second = u64::from(TICKS_PER_SECOND);
    let window = 7 * 60 * second;
    let cycle = 3 * window;
    let state = mix(seed ^ 0x6e6f_9a42 ^ mix(ticks / cycle));
    if state % 3 == 2 {
        return None;
    }
    let place = mix(state ^ 0x21bd);
    let start = (place % 3) * window + 16 * second + (place >> 8) % (35 * second);
    let duration = 24 * second + (place >> 28) % (5 * second);
    let age = (ticks % cycle).checked_sub(start)?;
    (age < duration).then_some(Refuel {
        progress: age as f32 / duration as f32,
        side: if place & 4 == 0 { -1.0 } else { 1.0 },
    })
}

/// A short burst of visible reheat on the F/A-18.
pub fn fighter_reheat(seed: u64, ticks: u64) -> f32 {
    event_time(seed ^ 0xcafe_9bf4, ticks, 6 * 60, 1, 2, 75, 4.1, 2.2).map_or(0.0, |event| {
        smoothstep(event.progress / 0.19) * (1.0 - smoothstep((event.progress - 0.56) / 0.44))
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Pass {
    Cross,
    Escort,
    Overhead,
    Breakaway,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Visitor {
    Aircraft(Aircraft),
    Biplane,
    Balloon,
    TowPair,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Effect {
    Motion { bank: f32, pitch: f32, cycles: f32 },
    Visitor(Visitor, Pass, u8),
    Trail { paired: bool, bend: f32, ink: [u8; 3] },
    Lights { span: f32, ahead: f32 },
    Airbrakes,
    Eagle { tuck: f32, flare: f32, talons: f32 },
}

#[derive(Clone, Copy, Debug)]
struct MomentSpec {
    name: &'static str,
    effect: Effect,
}

const fn moment(name: &'static str, effect: Effect) -> MomentSpec {
    MomentSpec { name, effect }
}

const fn motion(name: &'static str, bank: f32, pitch: f32, cycles: f32) -> MomentSpec {
    moment(name, Effect::Motion { bank, pitch, cycles })
}

const fn visitor(name: &'static str, aircraft: Aircraft, pass: Pass, count: u8) -> MomentSpec {
    moment(name, Effect::Visitor(Visitor::Aircraft(aircraft), pass, count))
}

const fn trail(name: &'static str, paired: bool, bend: f32, ink: [u8; 3]) -> MomentSpec {
    moment(name, Effect::Trail { paired, bend, ink })
}

// Indexed by Aircraft's stable discriminants, not the order in the picker.
const MOMENTS: [[MomentSpec; 4]; 8] = [
    [
        moment("Biplane crossing", Effect::Visitor(Visitor::Biplane, Pass::Cross, 1)),
        visitor("Bush-plane escort", Aircraft::BushPlane, Pass::Escort, 1),
        motion("Survey S-turn", 0.21, -0.046, 1.0),
        moment("Landing light pass", Effect::Lights { span: 0.92, ahead: 0.64 }),
    ],
    [
        visitor("Aerobatic pair", Aircraft::Racer, Pass::Breakaway, 2),
        motion("Wing wag", 0.40, 0.02, 2.0),
        trail("Straight smoke pass", false, 0.04, [216, 214, 201]),
        trail("Rising smoke arc", false, 0.91, [226, 165, 121]),
    ],
    [
        visitor("Business jet crossing", Aircraft::SportJet, Pass::Cross, 1),
        trail("Wingtip vapor", true, 0.17, [199, 219, 225]),
        moment("Arrival lights", Effect::Lights { span: 1.42, ahead: 1.42 }),
        motion("Long viewing turn", 0.14, -0.075, 1.0),
    ],
    [
        moment("Hot-air balloon pass", Effect::Visitor(Visitor::Balloon, Pass::Cross, 1)),
        visitor("Ultralight pair", Aircraft::Trike, Pass::Escort, 1),
        visitor("Sailplane passing overhead", Aircraft::Glider, Pass::Overhead, 1),
        motion("Valley lookout turn", 0.25, -0.08, 0.5),
    ],
    [
        moment("Towplane and sailplane crossing", Effect::Visitor(Visitor::TowPair, Pass::Cross, 1)),
        visitor("Sailplane joins alongside", Aircraft::Glider, Pass::Escort, 1),
        moment("Airbrake descent", Effect::Airbrakes),
        motion("Slow sideslip", -0.20, -0.12, 1.0),
    ],
    [
        visitor("Fighter high pass", Aircraft::Fighter, Pass::Overhead, 1),
        visitor("Section break", Aircraft::Fighter, Pass::Breakaway, 2),
        trail("Condensation streamers", true, 0.31, [220, 232, 239]),
        motion("Shallow bank reversal", 0.32, 0.05, 1.0),
    ],
    [
        trail("Twin high-altitude contrails", true, 0.05, [212, 227, 240]),
        visitor("Distant Blackbird crossing", Aircraft::Blackbird, Pass::Cross, 1),
        visitor("Airliner below", Aircraft::SportJet, Pass::Overhead, 1),
        motion("Cruise pitch adjustment", 0.015, 0.082, 0.5),
    ],
    [
        visitor("Bald Eagle crossing", Aircraft::BaldEagle, Pass::Escort, 1),
        moment("Talon stretch", Effect::Eagle { tuck: 0.0, flare: 0.07, talons: 1.0 }),
        moment("Tail fan", Effect::Eagle { tuck: 0.0, flare: 1.0, talons: 0.0 }),
        moment("Ridge wing tuck", Effect::Eagle { tuck: 1.0, flare: 0.10, talons: 0.0 }),
    ],
];

#[derive(Clone, Copy, Debug)]
pub struct FlightMoment {
    slot: usize,
    pub(crate) effect: Effect,
    pub progress: f32,
    pub(crate) side: f32,
}

impl FlightMoment {
    pub fn slot(self) -> usize { self.slot }

    pub(crate) fn ease(self) -> f32 {
        smoothstep(self.progress / 0.23) * (1.0 - smoothstep((self.progress - 0.72) / 0.28))
    }
}

pub fn moment_catalog(aircraft: Aircraft) -> [&'static str; 4] {
    MOMENTS[aircraft as usize].map(|spec| spec.name)
}

/// Rare secondary actions complement the existing airspace encounters. A
/// proposed window is skipped if a tanker or fighter formation owns the view.
pub fn flight_moment(aircraft: Aircraft, seed: u64, ticks: u64) -> Option<FlightMoment> {
    const INTERVAL_SECONDS: u64 = 260;
    let second = u64::from(TICKS_PER_SECOND);
    let timing = event_time(seed ^ 0x6b36_671e ^ aircraft as u64, ticks,
        INTERVAL_SECONDS, 2, 3, 93, 9.5, 5.0)?;
    if matches!(aircraft, Aircraft::Fighter | Aircraft::Blackbird) {
        let start = ticks / (INTERVAL_SECONDS * second) * INTERVAL_SECONDS * second + timing.start;
        let stop = start + timing.duration;
        let mut tick = start;
        while tick <= stop + second {
            let conflict = if aircraft == Aircraft::Blackbird {
                blackbird_refuel(seed, tick).is_some() || blackbird_reheat(seed, tick) > 0.0
            } else {
                formation(aircraft, seed, tick).is_some() || fighter_reheat(seed, tick) > 0.0
            };
            if conflict { return None; }
            tick += second;
        }
    }
    let slot = ((ticks / (INTERVAL_SECONDS * second) + (mix(seed) & 3)) & 3) as usize;
    Some(FlightMoment {
        slot, effect: MOMENTS[aircraft as usize][slot].effect, progress: timing.progress,
        side: if timing.state >> 9 & 1 == 0 { -1.0 } else { 1.0 },
    })
}

#[derive(Clone, Copy, Debug)]
pub struct Meteor {
    pub head: [f32; 3],
    pub tail: [f32; 3],
    pub brightness: f32,
}

pub fn meteor(seed: u64, ticks: u64, sky: &Sky) -> Option<Meteor> {
    let cloud_visibility = 1.0 - smoothstep((sky.cloud_cover - 0.55) / 0.16);
    if sky.daylight > 0.22 || cloud_visibility <= 0.0 {
        return None;
    }
    let timing = event_time(seed ^ 0xf41e_d3f2, ticks, 225, 2, 7, 18, 0.75, 0.65)?;
    let state = timing.state;
    let p = timing.progress;
    let bearing = fraction(mix(state ^ 81272)) * TAU;
    let top = 0.52 + fraction(mix(state ^ 7647)) * 0.63;
    let slant = if state & 1 == 0 { -1.0 } else { 1.0 };
    let arc = 0.42 + fraction(mix(state ^ 2811)) * 0.31;
    let direction = |progress: f32| {
        let (az_sin, az_cos) = sin_cos(bearing + slant * arc * progress);
        let (alt_sin, alt_cos) = sin_cos(top - 0.37 * progress);
        [az_sin * alt_cos, alt_sin, az_cos * alt_cos]
    };
    let visible = smoothstep(p / 0.17) * (1.0 - smoothstep((p - 0.62) / 0.38));
    Some(Meteor {
        head: direction(p),
        tail: direction((p - 0.23).max(0.0)),
        brightness: visible * (1.0 - sky.daylight / 0.22) * cloud_visibility,
    })
}

struct EventTime {
    state: u64,
    progress: f32,
    start: u64,
    duration: u64,
}

fn event_time(
    seed: u64, ticks: u64, interval_seconds: u64, chance: u64, odds: u64,
    start_padding: u64, min_seconds: f32, variation_seconds: f32,
) -> Option<EventTime> {
    let ticks_per_second = u64::from(TICKS_PER_SECOND);
    let interval = interval_seconds * ticks_per_second;
    let state = mix(seed ^ mix(ticks / interval));
    if state % odds >= chance {
        return None;
    }
    let slack = (interval_seconds - start_padding - 20) * ticks_per_second;
    let start = start_padding * ticks_per_second + mix(state ^ 2841) % slack;
    let duration = ((min_seconds + fraction(mix(state ^ 4_679)) * variation_seconds)
        * TICKS_PER_SECOND as f32) as u64;
    let progress = (ticks % interval).checked_sub(start)?;
    (progress < duration).then_some(EventTime {
        state, progress: progress as f32 / duration as f32, start, duration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_aircraft_has_four_schedulable_secondary_events() {
        use std::collections::HashSet;
        let mut titles = HashSet::new();
        let second = u64::from(TICKS_PER_SECOND);
        for aircraft in Aircraft::ALL {
            let mut found = [false; 4];
            for name in moment_catalog(aircraft) {
                assert!(titles.insert(name));
            }
            for seed in 0..12 {
                for tick in (0..second * 2_600).step_by(second as usize) {
                    let Some(event) = flight_moment(aircraft, seed, tick) else { continue; };
                    assert!(tick >= 93 * second);
                    assert!((0.0..1.0).contains(&event.progress));
                    if aircraft == Aircraft::Blackbird {
                        assert!(blackbird_refuel(seed, tick).is_none() && blackbird_reheat(seed, tick) == 0.0);
                    }
                    if aircraft == Aircraft::Fighter {
                        assert!(formation(aircraft, seed, tick).is_none() && fighter_reheat(seed, tick) == 0.0);
                    }
                    found[event.slot()] = true;
                }
                if found.into_iter().all(|seen| seen) { break; }
            }
            assert!(found.into_iter().all(|seen| seen), "{aircraft:?}");
        }
        assert_eq!(titles.len(), 32);
    }

    #[test]
    fn fighter_reheat_is_short_seeded_and_smooth() {
        let hz = u64::from(TICKS_PER_SECOND);
        let mut bursts = 0;
        for seed in 0..12 {
            let mut started = None;
            let mut previous = 0.0;
            for tick in 0..1_800 * hz {
                let current = fighter_reheat(seed, tick);
                assert!((current - previous).abs() < 0.08);
                previous = current;
                if current > 0.0 {
                    assert!(tick % (360 * hz) >= 75 * hz);
                    started.get_or_insert(tick);
                } else if let Some(start) = started.take() {
                    assert!((4 * hz..7 * hz).contains(&(tick - start)));
                    bursts += 1;
                }
            }
        }
        assert!((15..45).contains(&bursts));
    }

    #[test]
    fn blackbird_reheat_is_brief_and_rare() {
        let second = u64::from(TICKS_PER_SECOND);
        let mut bursts = 0;
        for seed in 0..16 {
            let mut start = None;
            let mut active = 0;
            let mut previous = 0.0_f32;
            for tick in 0..3_600 * second {
                let level = blackbird_reheat(seed, tick);
                assert!((level - previous).abs() <= 0.07);
                previous = level;
                if level > 0.0 {
                    assert!(tick >= 95 * second);
                    start.get_or_insert(tick);
                    active += 1;
                } else if let Some(first) = start.take() {
                    assert!((4 * second..8 * second).contains(&(tick - first)));
                    bursts += 1;
                }
            }
            assert!(active < 72 * second);
        }
        assert!(bursts >= 30);
    }

    #[test]
    fn formations_last_for_seconds_and_only_join_the_fighter() {
        let mut total = 0;
        let mut saw = [0; 2];
        let mut patterns = [false; 3];
        for seed in 0..24 {
            let mut started = None;
            for tick in (0..u64::from(TICKS_PER_SECOND) * 3_600).step_by(6) {
                let current = formation(Aircraft::Fighter, seed, tick);
                assert!(formation(Aircraft::Blackbird, seed, tick).is_none());
                if let Some(flight) = current {
                    let start = started.get_or_insert(tick);
                    assert!(*start >= 135 * u64::from(TICKS_PER_SECOND));
                    assert!(flight.count == 2 || flight.count == 4);
                    let [a, b, c, _] = flight.aircraft;
                    match flight.pattern {
                        FormationPattern::Wedge => {
                            assert!(a.right * b.right < 0.0);
                            if (0.3..0.65).contains(&flight.progress) {
                                assert!(a.forward > c.forward);
                            }
                            patterns[0] = true;
                        }
                        FormationPattern::LineAbreast => {
                            assert!(a.right * b.right < 0.0);
                            if (0.3..0.65).contains(&flight.progress) {
                                assert_eq!(a.forward, c.forward);
                            }
                            patterns[1] = true;
                        }
                        FormationPattern::Echelon => {
                            assert_eq!(flight.count, 2);
                            assert!(a.right * b.right > 0.0);
                            assert!(b.right.abs() > a.right.abs() || flight.progress > 0.66);
                            patterns[2] = true;
                        }
                    }
                    saw[flight.count / 2 - 1] += 1;
                } else if let Some(start) = started.take() {
                    let span = (tick - start) as f32 / TICKS_PER_SECOND as f32;
                    assert!((11.2..18.6).contains(&span), "{span}");
                    total += 1;
                }
            }
        }
        assert!((14..65).contains(&total) && saw.iter().all(|count| *count > 100), "{total} {saw:?}");
        assert_eq!(patterns, [true; 3]);
    }

    #[test]
    fn meteors_are_rare_and_excluded_from_daylight() {
        let mut visible = 0;
        let dark = Sky::from_time(15, 0, 0.0);
        let day = Sky::from_time(15, 0, 0.5);
        for tick in 0..u64::from(TICKS_PER_SECOND) * 3_600 {
            assert!(meteor(15, tick, &day).is_none());
            if let Some(event) = meteor(15, tick, &dark) {
                visible += 1;
                assert!((0.0..=1.0).contains(&event.brightness));
                assert!(event.head[1] > 0.0 && event.tail[1] > 0.0);
            }
        }
        assert!((10..400).contains(&visible), "{visible}");
    }
}
