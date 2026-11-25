//! Rare autopilot passes alongside actually moving, published train services.

use crate::flight::{Aircraft, TICKS_PER_SECOND, bearing};
use crate::math::{mix, smoothstep, wrap_angle};

const SECOND: u64 = TICKS_PER_SECOND as u64;
const OBSERVE_INTERVAL: u64 = 24;

#[derive(Clone, Copy, Debug)]
pub struct TrainMotion {
    pub(crate) key: u64,
    pub(crate) position: [f32; 3],
    pub(crate) direction: [f32; 2],
    pub(crate) speed: f32,
    pub(crate) passenger: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct AircraftPose {
    pub aircraft: Aircraft,
    pub ticks: u64,
    pub x: f32,
    pub z: f32,
    pub altitude: f32,
    pub heading: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrainPassPhase {
    Joining,
    Parallel,
    Departing,
}

#[derive(Clone, Copy)]
enum Pass {
    Along {
        train: TrainMotion,
        start: u64,
        updated: u64,
        end: u64,
        side: f32,
    },
    Away {
        heading: f32,
        side: f32,
        start: u64,
        speed: f32,
    },
}

#[derive(Clone, Copy, Default)]
pub(crate) struct TrainFollow {
    pass: Option<Pass>,
    cooldown: u64,
}

pub(crate) struct Guidance {
    pub heading: f32,
    pub speed: f32,
    pub altitude: Option<f32>,
    pub blend: f32,
}

impl TrainFollow {
    pub fn should_observe(&self, pose: AircraftPose) -> bool {
        matches!(pose.aircraft, Aircraft::Trike | Aircraft::BaldEagle)
            && pose.ticks.is_multiple_of(OBSERVE_INTERVAL)
            && (matches!(self.pass, Some(Pass::Along { .. }))
                || self.pass.is_none() && pose.ticks >= self.cooldown.max(40 * SECOND))
    }

    pub fn score(&self, pose: AircraftPose, train: TrainMotion) -> Option<f32> {
        let cap = match pose.aircraft {
            Aircraft::Trike => 11.5,
            Aircraft::BaldEagle => 8.8,
            _ => return None,
        };
        if !(3.7..=cap).contains(&train.speed) {
            return None;
        }
        let [x, height, z] = train.position;
        let [dx, dz] = train.direction;
        let offset = [x - pose.x, z - pose.z];
        let along = offset[0] * dx + offset[1] * dz;
        let across = offset[0] * dz - offset[1] * dx;
        let vertical = pose.altitude - height;
        if let Some(Pass::Along { train: previous, .. }) = self.pass {
            return (previous.key == train.key && along.abs() < 115.0 && across.abs() < 70.0
                && vertical < 75.0).then_some(0.0);
        }
        if self.pass.is_some() || pose.ticks < self.cooldown.max(40 * SECOND)
            || !(-16.0..=66.0).contains(&along) || across.abs() > 42.0
            || !(8.0..=47.0).contains(&vertical)
            || wrap_angle(bearing(dx, dz) - pose.heading).abs() > 0.88
        {
            return None;
        }
        // A reachable passenger service always wins over a simultaneous freight service.
        Some((along - 17.0).abs() + across.abs() * 0.7 + vertical * 0.2
            + if train.passenger { 0.0 } else { 120.0 })
    }

    pub fn observe(&mut self, pose: AircraftPose, seed: u64, train: Option<TrainMotion>) {
        let Some(train) = train.filter(|&motion| self.score(pose, motion).is_some()) else {
            return;
        };
        if let Some(Pass::Along { start, end, side, .. }) = self.pass {
            self.pass = Some(Pass::Along { train, start, end, side, updated: pose.ticks });
            return;
        }
        // One seeded chance per nearby service and two-minute window.
        let choice = mix(seed ^ train.key ^ (pose.ticks / (120 * SECOND)) ^ 0xa8c7_98ad_3cc1_2972);
        let chance = if train.passenger { 7 } else { 35 };
        if choice % chance != 0 {
            return;
        }
        let [x, _, z] = train.position;
        let [dx, dz] = train.direction;
        let lateral = (pose.x - x) * dz - (pose.z - z) * dx;
        let side = if lateral < 0.0 { -1.0 } else { 1.0 };
        self.pass = Some(Pass::Along {
            train, start: pose.ticks, updated: pose.ticks,
            end: pose.ticks + (17 + choice % 8) * SECOND, side,
        });
        self.cooldown = pose.ticks + (620 + choice % 361) * SECOND;
    }

    pub fn phase(&self, now: u64) -> Option<TrainPassPhase> {
        self.pass.map(|pass| match pass {
            Pass::Along { start, .. } if now - start < 4 * SECOND => TrainPassPhase::Joining,
            Pass::Along { .. } => TrainPassPhase::Parallel,
            Pass::Away { .. } => TrainPassPhase::Departing,
        })
    }

    pub fn guide(&mut self, pose: AircraftPose, normal_speed: f32) -> Option<Guidance> {
        let minimum = normal_speed * if pose.aircraft == Aircraft::Trike { 0.65 } else { 0.82 };
        let maximum = normal_speed * 1.15;
        if let Some(Pass::Along { train, updated, end, side, .. }) = self.pass {
            if pose.ticks >= end || pose.ticks.saturating_sub(updated) > 2 * SECOND {
                self.pass = Some(Pass::Away {
                    heading: pose.heading, side, start: pose.ticks,
                    speed: train.speed.clamp(minimum, maximum),
                });
            }
        }
        match self.pass? {
            Pass::Along { train, start, updated, side, .. } => {
                let [x, height, z] = train.position;
                let [east, north] = train.direction;
                let since = pose.ticks.saturating_sub(updated) as f32 / SECOND as f32;
                let offset = if pose.aircraft == Aircraft::BaldEagle { 11.0 } else { 15.0 };
                let lead = 17.0 + since * train.speed;
                let goal_x = x + east * lead + side * north * offset;
                let goal_z = z + north * lead - side * east * offset;
                let dx = goal_x - pose.x;
                let dz = goal_z - pose.z;
                let along = (x - pose.x) * east + (z - pose.z) * north + since * train.speed;
                Some(Guidance {
                    heading: if dx.abs() + dz.abs() > 0.01 { bearing(dx, dz) } else { pose.heading },
                    speed: (train.speed + (along - 17.0) * 0.075)
                        .clamp(minimum, maximum),
                    altitude: Some(height + if pose.aircraft == Aircraft::BaldEagle { 12.5 } else { 15.5 }),
                    blend: smoothstep((pose.ticks - start) as f32 / (4 * SECOND) as f32),
                })
            }
            Pass::Away { heading, side, start, speed } => {
                let age = pose.ticks.saturating_sub(start);
                if age >= 6 * SECOND {
                    self.pass = None;
                    return None;
                }
                let turn = smoothstep(age as f32 / (3 * SECOND) as f32);
                Some(Guidance {
                    heading: wrap_angle(heading + side * 0.54 * turn),
                    speed,
                    altitude: None,
                    blend: 1.0 - smoothstep(age as f32 / (6 * SECOND) as f32),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eligible_aircraft_join_and_let_a_stopping_train_go() {
        let motion = TrainMotion { key: 32, position: [0.0, 10.0, 17.0], direction: [0.0, 1.0], speed: 6.8, passenger: true };
        let mut pose = AircraftPose {
            aircraft: Aircraft::Trike, ticks: 120 * SECOND, x: 16.0, z: 0.0, altitude: 26.0, heading: 0.0,
        };
        let seed = (0..100).find(|&seed| {
            let mut follow = TrainFollow::default();
            follow.observe(pose, seed, Some(motion));
            follow.phase(pose.ticks) == Some(TrainPassPhase::Joining)
        }).expect("one of the seeds can start a pass");

        let follow = TrainFollow::default();
        let freight = TrainMotion { passenger: false, ..motion };
        assert!(follow.score(pose, motion).unwrap() < follow.score(pose, freight).unwrap());
        let starts = |train| (0..700).filter(|&seed| {
            let mut follow = TrainFollow::default();
            follow.observe(pose, seed, Some(train));
            follow.phase(pose.ticks).is_some()
        }).count();
        assert!(starts(motion) > starts(freight) * 3);

        for aircraft in [Aircraft::Trike, Aircraft::BaldEagle] {
            let mut follow = TrainFollow::default();
            let cruise = if aircraft == Aircraft::Trike { 10.2 } else { 7.4 };
            pose.aircraft = aircraft;
            pose.ticks = 120 * SECOND;
            assert!(follow.should_observe(pose));
            follow.observe(pose, seed, Some(motion));
            assert_eq!(follow.phase(pose.ticks), Some(TrainPassPhase::Joining));
            assert_eq!(follow.guide(pose, cruise).unwrap().blend, 0.0);

            pose.ticks += 4 * SECOND;
            follow.observe(pose, seed, Some(motion));
            assert_eq!(follow.phase(pose.ticks), Some(TrainPassPhase::Parallel));
            assert!((follow.guide(pose, cruise).unwrap().speed - 6.8).abs() < 0.001);

            pose.ticks += 2 * SECOND + 1;
            follow.observe(pose, seed, Some(TrainMotion { speed: 0.0, ..motion }));
            assert!(follow.guide(pose, cruise).unwrap().altitude.is_none());
            assert_eq!(follow.phase(pose.ticks), Some(TrainPassPhase::Departing));
            pose.ticks += 6 * SECOND;
            assert!(follow.guide(pose, cruise).is_none());
            assert!(follow.score(pose, motion).is_none());
        }

        pose.aircraft = Aircraft::Blackbird;
        let mut follow = TrainFollow::default();
        follow.observe(pose, seed, Some(motion));
        assert!(follow.phase(pose.ticks).is_none());
    }
}
