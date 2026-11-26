//! Scheduled, depth-tested trains on published railway grades.

use crate::flight::TICKS_PER_SECOND;
use crate::math::{mix, smoothstep};
use crate::rail::{RailLine, RailPoint, RailService};
use crate::rail_corridor::RailCorridor;
use crate::render::{Camera, ScreenProjection, ScreenRect};
use crate::train_follow::TrainMotion;
use crate::world_mesh::WorldMesh;

mod wagons;
use wagons::tint;

pub const TRAIN_REACH: f32 = 224.0;
const OPAQUE_REACH: f32 = 174.0;
const CAR_LENGTH: f32 = 13.6;
const CAR_PITCH: f32 = 15.1;
const HALF_WIDTH: f32 = 1.43;
const BODY_TOP: f32 = 3.87;
const BODY_BASE: f32 = 0.62;
const MAX_CARS: usize = 5;
const REGIONAL_MIN_LENGTH: f32 = 500.0;
const EXPRESS_MIN_LENGTH: f32 = 850.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrainService {
    Commuter,
    RegionalPassenger,
    Express,
    Freight,
}

impl From<RailService> for TrainService {
    fn from(service: RailService) -> Self {
        match service {
            RailService::Commuter => Self::Commuter,
            RailService::Freight => Self::Freight,
        }
    }
}

#[derive(Clone, Copy)]
enum Vehicle {
    Passenger { cab: bool },
    RegionalPassenger { cab: bool, express: bool },
    Engine,
    Wagon { color: [u8; 3], variant: u8 },
}

fn express_alignment(line: &RailCorridor) -> bool {
    let length = line.length();
    let a = line.position_at(0.0);
    let b = line.position_at(length);
    let dx = b.x - a.x;
    let dz = b.z - a.z;
    dx * dx + dz * dz >= length * length * 0.97 * 0.97
        && (a.elevation - b.elevation).abs() < length * 0.022
}

fn livery(key: u64, cargo: bool) -> [u8; 3] {
    const ACCENT: [[i16; 3]; 5] = [[0, 0, 0], [29, 23, 21], [-19, -12, -8], [17, -10, 9], [-9, 22, -11]];
    let bases = if cargo {
        [[159, 82, 60], [187, 143, 76], [77, 117, 134], [109, 121, 103], [172, 173, 159]]
    } else {
        [[49, 115, 130], [157, 67, 53], [58, 104, 145], [91, 109, 68], [140, 100, 148]]
    };
    let index = (key % 25) as usize;
    core::array::from_fn(|i| (bases[index / 5][i] + ACCENT[index % 5][i]).clamp(0, 255) as u8)
}

#[derive(Clone, Copy)]
struct Car {
    a: RailPoint,
    b: RailPoint,
    opacity: f32,
    detail: bool,
    vehicle: Vehicle,
    lamp: Option<bool>,
}

/// Build the image for one published train once per frame. It can use the
/// resulting small coverage rectangle for both terrain and mesh depth.
pub struct Train {
    cars: [Option<Car>; MAX_CARS],
    region: ScreenRect,
    stripe: [u8; 3],
}

impl Train {
    pub fn line_motion(line: &RailLine, seed: u64, ticks: u64) -> Option<TrainMotion> {
        let (trip, key) = Self::line_timetable(line, seed)?;
        trip.motion(key, ticks, |distance| line.point_at(distance))
    }

    pub fn corridor_motion(line: &RailCorridor, seed: u64, ticks: u64) -> Option<TrainMotion> {
        let (trip, key) = Self::corridor_timetable(line, seed)?;
        trip.motion(key, ticks, |distance| line.position_at(distance))
    }

    pub fn at(
        line: &RailLine,
        seed: u64,
        ticks: u64,
        camera: Camera,
        dimensions: (usize, usize),
    ) -> Option<Self> {
        let (trip, key) = Self::line_timetable(line, seed)?;
        Self::on_route(
            trip, key, ticks, camera, dimensions,
            |distance| line.point_at(distance),
        )
    }

    /// Scheduled center distance along a local line and direction along its station axis.
    pub fn line_position(line: &RailLine, seed: u64, ticks: u64) -> Option<(f32, f32)> {
        let [center, direction] = Self::line_timetable(line, seed)?.0.position(ticks);
        Some((center, direction))
    }

    fn line_timetable(line: &RailLine, seed: u64) -> Option<(Timetable, u64)> {
        let ends = line.stations().map(|station| [station.anchor.x, station.anchor.z]);
        let key = route_key(seed, ends);
        Some((Timetable::new(line.length(), line.service.into(), key)?, key))
    }

    pub fn on_corridor(
        line: &RailCorridor,
        seed: u64,
        ticks: u64,
        camera: Camera,
        dimensions: (usize, usize),
    ) -> Option<Self> {
        let (trip, key) = Self::corridor_timetable(line, seed)?;
        Self::on_route(
            trip, key, ticks, camera, dimensions,
            |distance| line.position_at(distance),
        )
    }

    /// Scheduled center distance along the corridor and direction along its station axis.
    pub fn corridor_position(line: &RailCorridor, seed: u64, ticks: u64) -> Option<(f32, f32)> {
        let [center, direction] = Self::corridor_timetable(line, seed)?.0.position(ticks);
        Some((center, direction))
    }

    fn corridor_timetable(line: &RailCorridor, seed: u64) -> Option<(Timetable, u64)> {
        let key = route_key(seed, line.stations().map(|station| station.stop.entry()));
        // Each corridor has one dedicated trainset. Longer passenger
        // corridors can use regional equipment without adding a train to the track.
        let length = line.length();
        let service = if line.service == RailService::Commuter && length >= EXPRESS_MIN_LENGTH
            && key % 5 == 2 && express_alignment(line)
        {
            TrainService::Express
        } else if line.service == RailService::Commuter && length >= REGIONAL_MIN_LENGTH
            && key % 3 == 1
        {
            TrainService::RegionalPassenger
        } else {
            line.service.into()
        };
        Some((Timetable::new(line.length(), service, key)?, key))
    }

    fn on_route(
        trip: Timetable,
        key: u64,
        ticks: u64,
        camera: Camera,
        dimensions: (usize, usize),
        point_at: impl Fn(f32) -> RailPoint,
    ) -> Option<Self> {
        let projection = ScreenProjection::new(camera, dimensions)?;
        let [center, direction] = trip.position(ticks);
        let mut region = None;
        let mut cars = [None; MAX_CARS];
        for (index, slot) in cars.iter_mut().enumerate().take(trip.cars) {
            let offset = index as f32 - (trip.cars - 1) as f32 * 0.5;
            let middle = center + offset * CAR_PITCH;
            let a = point_at(middle - CAR_LENGTH * 0.5);
            let b = point_at(middle + CAR_LENGTH * 0.5);
            let d = [
                a.x + (b.x - a.x) * 0.5 - camera.x,
                a.elevation + (b.elevation - a.elevation) * 0.5 - camera.altitude,
                a.z + (b.z - a.z) * 0.5 - camera.z,
            ];
            let range = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
            let opacity = smoothstep(
                (TRAIN_REACH * TRAIN_REACH - range)
                    / (TRAIN_REACH * TRAIN_REACH - OPAQUE_REACH * OPAQUE_REACH),
            );
            if opacity <= 0.018 {
                continue;
            }
            let margin = HALF_WIDTH + 0.18;
            let bounds = [
                [
                    a.x.min(b.x) - margin,
                    a.elevation.min(b.elevation),
                    a.z.min(b.z) - margin,
                ],
                [
                    a.x.max(b.x) + margin,
                    a.elevation.max(b.elevation) + BODY_TOP,
                    a.z.max(b.z) + margin,
                ],
            ];
            let Some(cover) = projection.bounds(bounds) else {
                continue;
            };
            let first = index == 0;
            let last = index + 1 == trip.cars;
            let end = first || last;
            let leading = (first && direction < 0.0) || (last && direction > 0.0);
            let vehicle = match trip.service {
                TrainService::Commuter => Vehicle::Passenger { cab: end },
                TrainService::RegionalPassenger | TrainService::Express => Vehicle::RegionalPassenger {
                    cab: end, express: trip.service == TrainService::Express,
                },
                TrainService::Freight if end => Vehicle::Engine,
                TrainService::Freight => {
                    let cargo = key.rotate_left((index * 11) as u32);
                    Vehicle::Wagon {
                        color: livery(cargo, true),
                        variant: (mix(cargo ^ 0x3f32_a93e_42f8_51a7) % 25) as u8,
                    }
                }
            };
            *slot = Some(Car {
                a,
                b,
                opacity,
                detail: range < 112.0 * 112.0,
                vehicle,
                lamp: end.then_some(leading),
            });
            region = Some(region.map_or(cover, |rect: ScreenRect| rect.union(cover)));
        }
        let stripe = livery(key.rotate_left(5), false);
        Some(Self {
            cars,
            region: region?,
            stripe,
        })
    }

    pub fn depth_region(&self) -> ScreenRect {
        self.region
    }

    pub fn draw(&self, mesh: &mut WorldMesh<'_>, daylight: f32) {
        let dusk = smoothstep((0.44 - daylight) / 0.38);
        for (index, car) in self.cars.iter().enumerate() {
            if let Some(car) = car {
                car.draw(mesh, self.stripe, dusk, index == 0);
            }
        }
    }
}

fn route_key(seed: u64, [a, b]: [[f32; 2]; 2]) -> u64 {
    mix(seed
        ^ (u64::from(a[0].to_bits()) << 32 | u64::from(a[1].to_bits()))
        ^ (u64::from(b[0].to_bits()) << 32 | u64::from(b[1].to_bits())).rotate_left(21)
        ^ 0xac27_31dc_6958_f24d)
}

struct Timetable {
    low: f32,
    high: f32,
    trip: u64,
    layover_a: u64,
    layover_b: u64,
    offset: u64,
    cars: usize,
    service: TrainService,
}

impl Timetable {
    fn motion(&self, key: u64, ticks: u64, point_at: impl Fn(f32) -> RailPoint) -> Option<TrainMotion> {
        let [center, direction] = self.position(ticks);
        if self.position(ticks.saturating_add(1))[0] == center {
            return None;
        }
        let step = u64::from(TICKS_PER_SECOND) / 2;
        let from = self.position(ticks.saturating_sub(step))[0];
        let to = self.position(ticks.saturating_add(step))[0];
        let speed = (to - from) * direction;
        if speed < 2.0 {
            return None;
        }
        let point = point_at(center);
        let ahead = point_at(center + direction * 5.0);
        let dx = ahead.x - point.x;
        let dz = ahead.z - point.z;
        let tangent = crate::flight::bearing(dx, dz);
        let (sin, cos) = crate::math::sin_cos(tangent);
        Some(TrainMotion { key, position: [point.x, point.elevation, point.z], direction: [sin, cos], speed,
            passenger: self.service != TrainService::Freight })
    }

    fn new(length: f32, service: TrainService, key: u64) -> Option<Self> {
        let (cars, speed, layover) = match service {
            TrainService::Commuter => (3, 7.8 + (key % 6) as f32 * 0.32, 43),
            TrainService::RegionalPassenger => (4, 15.8, 58),
            TrainService::Express => (5, 24.0, 74),
            TrainService::Freight => (5, 6.8, 88),
        };
        let low = (cars - 1) as f32 * CAR_PITCH * 0.5 + CAR_LENGTH * 0.5 + 2.3;
        let high = length - low;
        let distance = high - low;
        if distance < 40.0 {
            return None;
        }
        let hz = u64::from(TICKS_PER_SECOND);
        let trip = ((distance / speed + 4.0) * hz as f32) as u64;
        let layover_a = (layover + key % 67) * hz;
        let layover_b = (layover + key.rotate_left(17) % 97) * hz;
        Some(Self {
            low,
            high,
            trip,
            layover_a,
            layover_b,
            cars,
            service,
            offset: key.rotate_left(32) % (2 * trip + layover_a + layover_b),
        })
    }

    fn position(&self, ticks: u64) -> [f32; 2] {
        let period = 2 * self.trip + self.layover_a + self.layover_b;
        let phase = (ticks % period + self.offset) % period;
        if phase < self.layover_a {
            return [self.low, 1.0];
        }
        let away = phase - self.layover_a;
        if away < self.trip {
            return [self.low + (self.high - self.low) * self.progress(away), 1.0];
        }
        let back = away - self.trip;
        if back < self.layover_b {
            return [self.high, -1.0];
        }
        [
            self.high - (self.high - self.low) * self.progress(back - self.layover_b),
            -1.0,
        ]
    }

    fn progress(&self, ticks: u64) -> f32 {
        let ramp = (4 * TICKS_PER_SECOND) as f32;
        let time = ticks as f32;
        let duration = self.trip as f32;
        let area = duration - ramp;
        if time < ramp {
            time * time / (2.0 * ramp * area)
        } else if time > duration - ramp {
            let left = duration - time;
            1.0 - left * left / (2.0 * ramp * area)
        } else {
            (time - ramp * 0.5) / area
        }
    }
}

impl Car {
    fn point(self, t: f32, s: f32, h: f32) -> [f32; 3] {
        let dx = self.b.x - self.a.x;
        let dz = self.b.z - self.a.z;
        let cross = s / CAR_LENGTH;
        [
            self.a.x + dx * t - dz * cross,
            self.a.elevation + (self.b.elevation - self.a.elevation) * t + h,
            self.a.z + dz * t + dx * cross,
        ]
    }

    fn draw(self, mesh: &mut WorldMesh<'_>, stripe: [u8; 3], dusk: f32, first: bool) {
        let (body, frame, roof, cap) = match self.vehicle {
            Vehicle::Passenger { .. } => ([210, 210, 192], stripe, [157, 164, 161], BODY_TOP),
            Vehicle::RegionalPassenger { express: true, .. } => ([228, 233, 231], stripe, [83, 101, 123], BODY_TOP - 0.43),
            Vehicle::RegionalPassenger { .. } => ([148, 189, 196], stripe, [48, 70, 87], BODY_TOP - 0.17),
            Vehicle::Engine => ([175, 89, 56], [55, 70, 73], [117, 122, 115], BODY_TOP),
            Vehicle::Wagon { color, variant } => {
                return wagons::draw(
                    |t, side, height| self.point(t, side, height),
                    |corners, color| quad_triangles(mesh, corners, color, self.opacity, 0.0),
                    color, variant, self.detail,
                );
            }
        };
        let point = |t, side, h| self.point(t, side, h);
        let mut face =
            |quad, color, emission| quad_triangles(mesh, quad, color, self.opacity, emission);
        let regional = matches!(self.vehicle, Vehicle::RegionalPassenger { .. });
        let inset = 0.014;
        let [roof_start, roof_end] = match self.vehicle {
            Vehicle::RegionalPassenger { cab: true, express } if first => [inset + if express { 0.23 } else { 0.14 }, 1.0 - inset],
            Vehicle::RegionalPassenger { cab: true, express } => [inset, 1.0 - inset - if express { 0.23 } else { 0.14 }],
            _ => [inset, 1.0 - inset],
        };
        let wall_top = cap - 0.52;
        let wall_base: f32 = 0.94;
        let left = -HALF_WIDTH;
        let right = HALF_WIDTH;
        let inner = HALF_WIDTH * 0.63;
        face(
            [
                point(roof_start, -inner, cap),
                point(roof_end, -inner, cap),
                point(roof_end, inner, cap),
                point(roof_start, inner, cap),
            ],
            roof,
            0.0,
        );
        for (side, light) in [(left, 0.80), (right, 1.0)] {
            let panel = |a, b, bottom, top| {
                [
                    point(a, side, bottom),
                    point(b, side, bottom),
                    point(b, side, top),
                    point(a, side, top),
                ]
            };
            let roof_edge = inner.copysign(side);
            face(
                [
                    point(inset, side, wall_top),
                    point(1.0 - inset, side, wall_top),
                    point(roof_end, roof_edge, cap),
                    point(roof_start, roof_edge, cap),
                ],
                tint(roof, 0.90 * light),
                0.0,
            );
            face(panel(0.0, 1.0, BODY_BASE, 1.19), tint(frame, light), 0.0);
            face(
                panel(inset, 1.0 - inset, (wall_base + 0.09).max(1.19), wall_top),
                tint(body, light),
                0.0,
            );
            if self.detail {
                for (a, b) in [(0.13, 0.27), (0.73, 0.87)] {
                    let axle = side * 0.67;
                    face(
                        [
                            point(a, axle, 0.25),
                            point(b, axle, 0.25),
                            point(b, axle, BODY_BASE + 0.12),
                            point(a, axle, BODY_BASE + 0.12),
                        ],
                        [37, 47, 50],
                        0.0,
                    );
                }
            }
            match self.vehicle {
                Vehicle::Passenger { .. } | Vehicle::RegionalPassenger { .. } => {
                    let pane = |a, b| {
                        [
                            point(a, side * 1.009, 2.25),
                            point(b, side * 1.009, 2.25),
                            point(b, side * 1.009, 3.06),
                            point(a, side * 1.009, 3.06),
                        ]
                    };
                    let cold = tint(if regional { [37, 55, 75] } else { [49, 68, 80] }, light);
                    let warm = tint([233, 172, 94], 0.88 + 0.12 * light);
                    let glass = core::array::from_fn(|channel| {
                        let start = f32::from(cold[channel]);
                        (start + (f32::from(warm[channel]) - start) * dusk + 0.5) as u8
                    });
                    let panes = if regional {
                        [(0.09, 0.27), (0.29, 0.47), (0.51, 0.69), (0.71, 0.89)]
                    } else {
                        [(0.11, 0.27), (0.31, 0.47), (0.51, 0.67), (0.71, 0.87)]
                    };
                    for (a, b) in panes {
                        face(pane(a, b), glass, dusk * 0.72);
                    }
                }
                Vehicle::Engine if self.detail => {
                    let offset = side * 1.009;
                    let (a, b) = if first { (0.09, 0.33) } else { (0.67, 0.91) };
                    face(
                        [
                            point(a, offset, 2.17),
                            point(b, offset, 2.17),
                            point(b, offset, 3.14),
                            point(a, offset, 3.14),
                        ],
                        [45, 66, 70],
                        0.0,
                    );
                }
                _ => {}
            }
        }
        for (t, roof_t) in [(inset, roof_start), (1.0 - inset, roof_end)] {
            face(
                [
                    point(t, left, wall_base),
                    point(t, right, wall_base),
                    point(t, right, wall_top),
                    point(t, left, wall_top),
                ],
                tint(body, 0.72),
                0.0,
            );
            face(
                [
                    point(t, left, wall_top),
                    point(t, right, wall_top),
                    point(roof_t, inner, cap),
                    point(roof_t, -inner, cap),
                ],
                tint(roof, 0.74),
                0.0,
            );
        }
        if let Some(leading) = self.lamp {
            let outer = if first { -0.003 } else { 1.003 };
            if matches!(
                self.vehicle,
                Vehicle::Passenger { cab: true } | Vehicle::RegionalPassenger { cab: true, .. } | Vehicle::Engine
            ) {
                face(
                    [
                        point(outer, -0.99, 2.33),
                        point(outer, 0.99, 2.33),
                        point(outer, 0.99, 3.21),
                        point(outer, -0.99, 3.21),
                    ],
                    [49, 66, 76],
                    0.0,
                );
            }
            if dusk > 0.02 {
                let color = if leading {
                    [255, 219, 142]
                } else {
                    [207, 63, 48]
                };
                let face_at = if first { outer - 0.005 } else { outer + 0.005 };
                for side in [-0.83, 0.83] {
                    face(
                        [
                            point(face_at, side - 0.24, 1.35),
                            point(face_at, side + 0.24, 1.35),
                            point(face_at, side + 0.24, 1.82),
                            point(face_at, side - 0.24, 1.82),
                        ],
                        color,
                        dusk * if leading { 0.93 } else { 0.65 },
                    );
                }
            }
        }
    }
}

fn quad_triangles(
    mesh: &mut WorldMesh<'_>,
    [a, b, c, d]: [[f32; 3]; 4],
    color: [u8; 3],
    opacity: f32,
    emission: f32,
) {
    for triangle in [[a, b, c], [a, c, d]] {
        if opacity >= 0.985 && emission > 0.0 {
            mesh.luminous_triangle(triangle, color, emission);
        } else {
            mesh.faded_triangle(triangle, color, opacity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn express_cruises_faster_and_both_livery_banks_have_25_colors() {
        let length = 1100.0;
        let express = Timetable::new(length, TrainService::Express, 21).unwrap();
        let regional = Timetable::new(length, TrainService::RegionalPassenger, 21).unwrap();
        assert!(express.trip < regional.trip);
        let period = 2 * express.trip + express.layover_a + express.layover_b;
        let tick = (period - express.offset) % period + express.layover_a + express.trip / 2;
        let step = (express.position(tick + 1)[0] - express.position(tick)[0]).abs();
        assert!((23.8..24.2).contains(&(step * TICKS_PER_SECOND as f32)));
        for cargo in [true, false] {
            let colors: std::collections::HashSet<_> = (0..25).map(|key| livery(key, cargo)).collect();
            assert_eq!(colors.len(), 25);
        }
    }
    use crate::rail::RailAnchorKind;
    use crate::rail_art::overlay_rail;
    use crate::render::render_terrain_layer_region;
    use crate::roads::RoadNetwork;
    use crate::sky::Sky;
    use crate::world::World;

    #[test]
    fn trains_stop_and_reverse_without_jumping_between_endpoints() {
        for service in [RailService::Commuter, RailService::Freight] {
            let trip = Timetable::new(480.0, service.into(), 719_391).unwrap();
            let period = 2 * trip.trip + trip.layover_a + trip.layover_b;
            let mut previous = trip.position(0)[0];
            let mut stopped = 0;
            let mut moved = 0;
            for tick in 1..=period {
                let [distance, _] = trip.position(tick);
                assert!((trip.low..=trip.high).contains(&distance));
                let motion = (distance - previous).abs();
                assert!(motion <= 0.34, "{tick} {motion}");
                stopped += usize::from(motion == 0.0);
                moved += usize::from(motion > 0.0);
                previous = distance;
            }
            assert!(stopped > 1_000 && moved > 1_000);
            assert_eq!(trip.position(0), trip.position(period));
            assert!(trip.position(u64::MAX)[0].is_finite());
        }
    }

    #[test]
    fn regional_train_has_faster_cruise_and_two_stationary_dwells() {
        let key = 729_123;
        let train = Timetable::new(533.0, TrainService::RegionalPassenger, key).unwrap();
        let local = Timetable::new(533.0, TrainService::Commuter, key).unwrap();
        let period = 2 * train.trip + train.layover_a + train.layover_b;
        let phase_zero = (period - train.offset) % period;
        let depart_a = phase_zero + train.layover_a;
        let arrive_b = depart_a + train.trip;
        let depart_b = arrive_b + train.layover_b;
        let arrive_a = depart_b + train.trip;
        assert_eq!(train.cars, 4);
        assert!(CAR_PITCH > CAR_LENGTH);
        assert!(train.trip < local.trip * 2 / 3);
        for tick in phase_zero..depart_a {
            assert_eq!(train.position(tick), [train.low, 1.0]);
        }
        for tick in arrive_b..depart_b {
            assert_eq!(train.position(tick), [train.high, -1.0]);
        }
        assert_eq!(train.position(arrive_a), [train.low, 1.0]);
        assert_eq!(train.position(depart_a), [train.low, 1.0]);
        assert_eq!(train.position(depart_b), [train.high, -1.0]);
        let step = |tick| (train.position(tick + 1)[0] - train.position(tick)[0]).abs();
        let hz = u64::from(TICKS_PER_SECOND);
        assert!(step(depart_a + 1) < step(depart_a + 2 * hz));
        assert!(step(arrive_b - 2) < step(arrive_b - 2 * hz));
        assert!(step(depart_b + 1) < step(depart_b + 2 * hz));
        assert!(step(arrive_a - 2) < step(arrive_a - 2 * hz));
        assert!((15.7..15.9).contains(&(step(depart_a + 5 * hz) * hz as f32)));
        assert!((depart_a..arrive_a).all(|tick| step(tick) < 0.54));
        assert!(train.position(u64::MAX)[0].is_finite());
    }

    #[test]
    fn motion_matches_the_train_and_excludes_stationary_dwells() {
        let timetable = Timetable::new(480.0, RailService::Freight.into(), 719_391).unwrap();
        let period = 2 * timetable.trip + timetable.layover_a + timetable.layover_b;
        let start = (period - timetable.offset) % period;
        let station = |z| RailPoint { x: 2.0, z, elevation: 12.0 };
        assert!(timetable.motion(11, start, station).is_none());
        let cruise = start + timetable.layover_a + 8 * u64::from(TICKS_PER_SECOND);
        let motion = timetable.motion(11, cruise, station).unwrap();
        assert_eq!(motion.position, [2.0, 12.0, timetable.position(cruise)[0]]);
        assert!((motion.speed - 6.8).abs() < 0.01);
        assert_eq!(motion.direction, [0.0, 1.0]);
        let arrival = start + timetable.layover_a + timetable.trip;
        assert!(timetable.motion(11, arrival, station).is_none());
    }

    #[test]
    fn moving_train_respects_the_track_grade_and_clipped_depth() {
        let world = World::new(1);
        let road = RoadNetwork::generate(&world, -1, -3).unwrap();
        let line =
            RailLine::for_road_with_terminals(&world, 1, &road, [RailAnchorKind::Port; 2]).unwrap();
        let start = line.polyline()[0];
        let end = *line.polyline().last().unwrap();
        assert_eq!(line.point_at(0.0), start);
        assert_eq!(line.point_at(line.length()), end);
        let side = 112;
        let dimensions = (side, side);
        let a = line.point_at(line.length() * 0.5);
        let camera = Camera {
            x: a.x - 6.0,
            z: a.z - 62.0,
            altitude: a.elevation + 24.0,
            heading: 0.1,
            pitch: 0.25,
            bank: -0.12,
        };
        let ticks = (0..25_000)
            .step_by(21)
            .find(|&ticks| {
                Train::at(&line, 1, ticks, camera, dimensions)
                    .is_some_and(|train| train.cars.iter().flatten().count() >= 4)
            })
            .unwrap();
        let train = Train::at(&line, 1, ticks, camera, dimensions).unwrap();
        let center = Train::line_position(&line, 1, ticks).unwrap().0;
        for (index, car) in train.cars.iter().enumerate().filter_map(|(index, car)| Some((index, car.as_ref()?))) {
            let middle = center + (index as f32 - 2.0) * CAR_PITCH;
            assert_eq!(car.a, line.point_at(middle - CAR_LENGTH * 0.5));
            assert_eq!(car.b, line.point_at(middle + CAR_LENGTH * 0.5));
            for point in [car.a, car.b] {
                let track = line.sample(point.x, point.z).unwrap();
                assert!((track.elevation.unwrap() - point.elevation).abs() < 0.01);
            }
        }
        let sky = Sky::from_time(1, ticks, 0.43);
        let render = |region| {
            let mut pixels = std::vec![0; side * side];
            let mut work = pixels.clone();
            let mut depth = pixels.clone();
            render_terrain_layer_region(
                (
                    |x, z| overlay_rail(&line, x, z, world.sample(x, z)),
                    |_, _, sample: crate::world::Sample| sample.surface,
                ),
                camera,
                &sky,
                &mut pixels,
                &mut work,
                Some((&mut depth, region)),
                dimensions,
                |_| {},
            )
            .unwrap();
            (pixels, depth)
        };
        let (base, depth) = render(ScreenRect::full(dimensions));
        let mut full = base.clone();
        let mut full_depth = depth.clone();
        train.draw(
            &mut WorldMesh::new(&mut full, &mut full_depth, dimensions, camera, &sky).unwrap(),
            sky.daylight,
        );
        assert!(full.iter().zip(&base).filter(|(a, b)| a != b).count() > 18);
        let (mut clipped, mut clipped_depth) = render(train.depth_region());
        train.draw(
            &mut WorldMesh::new(&mut clipped, &mut clipped_depth, dimensions, camera, &sky)
                .unwrap(),
            sky.daylight,
        );
        assert_eq!(clipped, full);
        let mut occluded = base.clone();
        let mut occluding_depth = std::vec![0; side * side];
        train.draw(
            &mut WorldMesh::new(
                &mut occluded,
                &mut occluding_depth,
                dimensions,
                camera,
                &sky,
            )
            .unwrap(),
            sky.daylight,
        );
        assert_eq!(occluded, base);
    }
}
