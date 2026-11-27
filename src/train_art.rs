//! Scheduled, depth-tested trains on published railway grades.

use crate::flight::TICKS_PER_SECOND;
use crate::math::{mix, smoothstep};
use crate::rail::{RailLine, RailPoint, RailService};
use crate::rail_corridor::RailCorridor;
use crate::render::{Camera, ScreenProjection, ScreenRect};
use crate::train_follow::TrainMotion;
use crate::world_mesh::WorldMesh;

mod freight_catalog;
mod freight_geography;
mod locomotives;
mod wagons;
pub(crate) use freight_catalog::FreightRoute;
pub(crate) use freight_geography::{contextual_freight_route, freight_route};
use freight_geography::LoadingEnd;
use locomotives::Locomotive;
use wagons::tint;

pub const TRAIN_REACH: f32 = 224.0;
/// Distance budget for the inexpensive distant rendering path only.
pub const TRAIN_RENDER_REACH: f32 = 620.0;
const DISTANT_OPAQUE_REACH: f32 = 470.0;
const OPAQUE_REACH: f32 = 174.0;
const CAR_LENGTH: f32 = 13.6;
const CAR_PITCH: f32 = 15.1;
const FREIGHT_GAP: f32 = 1.12;
const LONGEST_WAGON: f32 = 16.8;
const HALF_WIDTH: f32 = 1.43;
const BODY_TOP: f32 = 3.87;
const BODY_BASE: f32 = 0.62;
const MAX_CARS: usize = 8;
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
    Engine(Locomotive),
    Wagon { color: [u8; 3], variant: u8, loaded: bool },
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
    length: f32,
    opacity: f32,
    distant_opacity: f32,
    detail: bool,
    vehicle: Vehicle,
    lamp: Option<bool>,
}

#[derive(Clone, Copy)]
struct FreightCar {
    length: f32,
    wagon: Option<(u64, u8)>,
}

struct FreightFormation {
    cars: [FreightCar; MAX_CARS],
    half_span: f32,
}

impl FreightFormation {
    fn new(count: usize, route: FreightRoute, key: u64, grain_elevator: bool) -> Self {
        let mut cars = [FreightCar { length: CAR_LENGTH, wagon: None }; MAX_CARS];
        for index in 1..count - 1 {
            let cargo = mix(key ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
            let variant = if grain_elevator {
                freight_catalog::grain_elevator_variant(cargo)
            } else {
                freight_catalog::choose_variant(route, key, index - 1, cargo)
            };
            cars[index] = FreightCar {
                length: freight_catalog::design(variant).body_length(),
                wagon: Some((cargo, variant)),
            };
        }
        let span = cars[..count].iter().map(|car| car.length).sum::<f32>()
            + (count - 1) as f32 * FREIGHT_GAP;
        Self { cars, half_span: span * 0.5 }
    }
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
            trip, key, ticks, camera, dimensions, line.freight_route, line.grain_terminal().is_some(),
            freight_geography::local_loading_end(line),
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
        Some((Timetable::new(line.length(), line.service.into(), line.freight_route, key)?, key))
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
            trip, key, ticks, camera, dimensions, line.freight_route, false,
            freight_geography::regional_loading_end(line),
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
        Some((Timetable::new(line.length(), service, line.freight_route, key)?, key))
    }

    fn on_route(
        trip: Timetable,
        key: u64,
        ticks: u64,
        camera: Camera,
        dimensions: (usize, usize),
        freight_route: FreightRoute,
        grain_elevator: bool,
        loading_end: Option<LoadingEnd>,
        point_at: impl Fn(f32) -> RailPoint,
    ) -> Option<Self> {
        let [center, direction] = trip.position(ticks);
        let engine = Locomotive::for_route(freight_route, trip.high - trip.low, key);
        let freight = (trip.service == TrainService::Freight)
            .then(|| FreightFormation::new(trip.cars, freight_route, key, grain_elevator));
        let cargo_stage = trip.cargo_stage(ticks, loading_end, trip.cars - 2);
        let mut freight_front = center - freight.as_ref().map_or(0.0, |train| train.half_span);
        Self::projected(trip.cars, livery(key.rotate_left(5), false), camera, dimensions, |index| {
            let (a, b, length, wagon) = if let Some(train) = &freight {
                let car = train.cars[index];
                let a = freight_front;
                let b = a + car.length;
                freight_front = b + FREIGHT_GAP;
                (point_at(a), point_at(b), car.length, car.wagon)
            } else {
                let offset = index as f32 - (trip.cars - 1) as f32 * 0.5;
                let middle = center + offset * CAR_PITCH;
                (point_at(middle - CAR_LENGTH * 0.5), point_at(middle + CAR_LENGTH * 0.5), CAR_LENGTH, None)
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
                TrainService::Freight if end => Vehicle::Engine(engine),
                TrainService::Freight => {
                    let (cargo, variant) = wagon?;
                    Vehicle::Wagon {
                        color: livery(cargo, true),
                        variant,
                        loaded: cargo_stage.present(index - 1),
                    }
                }
            };
            Some((a, b, length, vehicle, end.then_some(leading)))
        })
    }

    pub(crate) fn on_metro(
        center: f32,
        direction: f32,
        count: usize,
        stripe: [u8; 3],
        camera: Camera,
        dimensions: (usize, usize),
        point_at: impl Fn(f32) -> RailPoint,
    ) -> Option<Self> {
        Self::projected(count, stripe, camera, dimensions, |index| {
            let middle = center + (index as f32 - (count - 1) as f32 * 0.5) * CAR_PITCH;
            let a = point_at(middle - CAR_LENGTH * 0.5);
            let b = point_at(middle + CAR_LENGTH * 0.5);
            let first = index == 0;
            let last = index + 1 == count;
            let cab = first || last;
            let leading = (first && direction < 0.0) || (last && direction > 0.0);
            Some((a, b, CAR_LENGTH, Vehicle::Passenger { cab }, cab.then_some(leading)))
        })
    }

    fn projected(
        count: usize,
        stripe: [u8; 3],
        camera: Camera,
        dimensions: (usize, usize),
        mut car_at: impl FnMut(usize) -> Option<(RailPoint, RailPoint, f32, Vehicle, Option<bool>)>,
    ) -> Option<Self> {
        let projection = ScreenProjection::new(camera, dimensions)?;
        let mut region = None;
        let mut cars = [None; MAX_CARS];
        for (index, slot) in cars.iter_mut().enumerate().take(count) {
            let (a, b, length, vehicle, lamp) = car_at(index)?;
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
            let distant_opacity = (1.0 - opacity) * smoothstep(
                (TRAIN_RENDER_REACH * TRAIN_RENDER_REACH - range)
                    / (TRAIN_RENDER_REACH * TRAIN_RENDER_REACH - DISTANT_OPAQUE_REACH * DISTANT_OPAQUE_REACH),
            );
            if opacity <= 0.018 && distant_opacity <= 0.018 { continue; }
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
            let car = Car {
                a,
                b,
                length,
                opacity: if opacity > 0.018 { opacity } else { 0.0 },
                distant_opacity,
                detail: range < 112.0 * 112.0,
                vehicle,
                lamp,
            };
            let cover = if car.opacity > 0.0 { projection.bounds(bounds) }
                else { car.distant_region(&projection, dimensions) };
            let Some(cover) = cover else { continue; };
            *slot = Some(car);
            region = Some(region.map_or(cover, |rect: ScreenRect| rect.union(cover)));
        }
        Some(Self { cars, region: region?, stripe })
    }

    pub fn depth_region(&self) -> ScreenRect {
        self.region
    }

    /// Keep close trains ahead of silhouettes when only a few trains fit the frame budget.
    pub fn is_near(&self) -> bool {
        self.cars.iter().flatten().any(|car| car.opacity > 0.0)
    }

    pub fn visible_pair(candidates: impl IntoIterator<Item = Self>) -> [Option<Self>; 2] {
        let mut near = [None, None];
        let mut distant = [None, None];
        for train in candidates {
            let slots = if train.is_near() { &mut near } else { &mut distant };
            if let Some(slot) = slots.iter_mut().find(|slot| slot.is_none()) {
                *slot = Some(train);
            }
            if near[1].is_some() { return near; }
        }
        let mut distant = distant.into_iter().flatten();
        near.map(|slot| slot.or_else(|| distant.next()))
    }

    pub fn draw(&self, mesh: &mut WorldMesh<'_>, daylight: f32) {
        for car in self.cars.iter().flatten().filter(|car| car.distant_opacity > 0.018) {
            mesh.faded_segment(car.distant_span(0.27), 2.76, [112, 102, 89], car.distant_opacity * 0.70);
            let (height, paint) = car.distant_profile(self.stripe);
            mesh.faded_segment(car.distant_span(height), 2.35, paint, car.distant_opacity);
        }
        let dusk = smoothstep((0.44 - daylight) / 0.38);
        for (index, car) in self.cars.iter().enumerate() {
            if let Some(car) = car.filter(|car| car.opacity > 0.0) {
                if let Some(next) = self.cars.get(index + 1).copied().flatten().filter(|next| next.opacity > 0.0) {
                    car.draw_connection(next, mesh);
                }
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

#[derive(Clone, Copy)]
enum CargoStage {
    InTransit(bool),
    AtTerminal {
        loading: bool,
        processed: usize,
        wagons: usize,
        end: LoadingEnd,
    },
}

impl CargoStage {
    fn present(self, wagon: usize) -> bool {
        match self {
            Self::InTransit(loaded) => loaded,
            Self::AtTerminal { loading, processed, wagons, end } => {
                let served = match end {
                    LoadingEnd::First => wagon < processed,
                    LoadingEnd::Second => wagon >= wagons - processed,
                };
                served == loading
            }
        }
    }
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

    fn new(length: f32, service: TrainService, freight: FreightRoute, key: u64) -> Option<Self> {
        let (cars, speed, layover) = match service {
            TrainService::Commuter => (3, 7.8 + (key % 6) as f32 * 0.32, 43),
            TrainService::RegionalPassenger => (4, 15.8, 58),
            TrainService::Express => (5, 24.0, 74),
            TrainService::Freight => freight_timing(freight, length),
        };
        let low = if service == TrainService::Freight {
            // Leave room at both terminals for the longest eligible freight equipment.
            let span = 2.0 * CAR_LENGTH + (cars - 2) as f32 * LONGEST_WAGON
                + (cars - 1) as f32 * FREIGHT_GAP;
            span * 0.5 + 2.3
        } else {
            (cars - 1) as f32 * CAR_PITCH * 0.5 + CAR_LENGTH * 0.5 + 2.3
        };
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
        let phase = self.phase(ticks);
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

    fn phase(&self, ticks: u64) -> u64 {
        let period = 2 * self.trip + self.layover_a + self.layover_b;
        (ticks % period + self.offset) % period
    }

    fn cargo_stage(&self, ticks: u64, source: Option<LoadingEnd>, wagons: usize) -> CargoStage {
        let Some(source) = source else { return CargoStage::InTransit(true) };
        let phase = self.phase(ticks);
        if phase < self.layover_a {
            return self.service_cargo(source, LoadingEnd::First, phase, self.layover_a, wagons);
        }
        let away = phase - self.layover_a;
        if away < self.trip {
            return CargoStage::InTransit(source == LoadingEnd::First);
        }
        let back = away - self.trip;
        if back < self.layover_b {
            return self.service_cargo(source, LoadingEnd::Second, back, self.layover_b, wagons);
        }
        CargoStage::InTransit(source == LoadingEnd::Second)
    }

    fn service_cargo(&self, source: LoadingEnd, end: LoadingEnd, elapsed: u64, dwell: u64, wagons: usize) -> CargoStage {
        // Settle at the terminal before servicing cars. Leave the last quarter
        // of the dwell for departure preparation instead of changing in motion.
        let settling = dwell / 4;
        let service = (dwell / 2).max(1);
        let work = elapsed.saturating_sub(settling).min(service);
        let processed = (work * wagons as u64 / service) as usize;
        CargoStage::AtTerminal { loading: source == end, processed, wagons, end }
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

fn freight_timing(route: FreightRoute, length: f32) -> (usize, f32, u64) {
    use FreightRoute::*;

    let long = length >= 720.0;
    let (cars, speed, dwell) = match route {
        Intermodal | Perishable => (7, 9.4, 108),
        Automotive => (6, 8.4, 122),
        Agriculture => (8, 6.4, 126),
        Quarry | Aggregate => (8, 5.6, 102),
        Mill | Liquid => (6, 6.0, 136),
        Forest => (6, 6.4, 112),
        Mixed => (7, 6.8, 88),
    };
    (if long { cars } else { 5 }, speed, dwell)
}

impl Car {
    fn point(self, t: f32, s: f32, h: f32) -> [f32; 3] {
        let dx = self.b.x - self.a.x;
        let dz = self.b.z - self.a.z;
        let cross = s / self.length;
        [
            self.a.x + dx * t - dz * cross,
            self.a.elevation + (self.b.elevation - self.a.elevation) * t + h,
            self.a.z + dz * t + dx * cross,
        ]
    }

    fn distant_profile(self, stripe: [u8; 3]) -> (f32, [u8; 3]) {
        use freight_catalog::Family;
        match self.vehicle {
            Vehicle::Passenger { .. } => (2.88, [203, 203, 182]),
            Vehicle::RegionalPassenger { express: true, .. } => (2.94, [219, 224, 216]),
            Vehicle::RegionalPassenger { .. } => (2.85, [148, 189, 196]),
            Vehicle::Engine(_) => (2.58, stripe),
            Vehicle::Wagon { color, variant, loaded } => {
                let height = match freight_catalog::design(variant).family {
                    Family::Well | Family::Deck if !loaded => 1.27,
                    Family::Well | Family::Deck | Family::OpenBulk | Family::Coil => 1.91,
                    Family::CoveredHopper | Family::Tank | Family::Centerbeam => 2.63,
                    Family::Boxcar | Family::Autorack | Family::Reefer => 2.95,
                };
                (height, color)
            }
        }
    }

    fn distant_span(self, height: f32) -> [[f32; 3]; 2] {
        [self.point(0.0, 0.0, height), self.point(1.0, 0.0, height)]
    }

    fn distant_region(self, projection: &ScreenProjection, (width, height): (usize, usize)) -> Option<ScreenRect> {
        let (roof, _) = self.distant_profile([0; 3]);
        let mut limits = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
        for point in self.distant_span(roof).into_iter().chain(self.distant_span(0.27)) {
            let [x, y, _] = projection.backdrop_point(point)?;
            limits = [limits[0].min(x), limits[1].min(y), limits[2].max(x), limits[3].max(y)];
        }
        let [left, top, right, bottom] = limits;
        let rect = ScreenRect {
            left: ((left - 2.0) as usize).min(width),
            top: ((top - 2.0) as usize).min(height),
            right: ((right + 3.0) as usize).min(width),
            bottom: ((bottom + 3.0) as usize).min(height),
        };
        (rect.left < rect.right && rect.top < rect.bottom).then_some(rect)
    }

    fn draw_connection(self, next: Self, mesh: &mut WorldMesh<'_>) {
        if !matches!(self.vehicle, Vehicle::Engine(_) | Vehicle::Wagon { .. }) {
            return;
        }
        // Span from the actual endpoints so the coupler stays connected on curves.
        let h = 0.74;
        let [a, b, c, d] = [
            self.point(0.98, -0.17, h), self.point(0.98, 0.17, h),
            next.point(0.02, 0.17, h), next.point(0.02, -0.17, h),
        ];
        quad_triangles(mesh, [a, b, c, d], [59, 63, 61], self.opacity.min(next.opacity), 0.0);
    }

    fn draw(self, mesh: &mut WorldMesh<'_>, stripe: [u8; 3], dusk: f32, first: bool) {
        let (body, frame, roof, cap) = match self.vehicle {
            Vehicle::Passenger { .. } => ([210, 210, 192], stripe, [157, 164, 161], BODY_TOP),
            Vehicle::RegionalPassenger { express: true, .. } => ([228, 233, 231], stripe, [83, 101, 123], BODY_TOP - 0.43),
            Vehicle::RegionalPassenger { .. } => ([148, 189, 196], stripe, [48, 70, 87], BODY_TOP - 0.17),
            Vehicle::Engine(locomotive) => {
                return locomotives::draw(
                    |t, side, height| {
                        if first {
                            self.point(t, side, height)
                        } else {
                            self.point(1.0 - t, -side, height)
                        }
                    },
                    |corners, color, emission| quad_triangles(mesh, corners, color, self.opacity, emission),
                    locomotive, self.detail, self.lamp.unwrap_or(false), dusk,
                );
            }
            Vehicle::Wagon { color, variant, loaded } => {
                return wagons::draw(
                    |t, side, height| self.point(t, side, height),
                    |corners, color| quad_triangles(mesh, corners, color, self.opacity, 0.0),
                    color, variant, self.detail, loaded,
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
                Vehicle::Passenger { cab: true } | Vehicle::RegionalPassenger { cab: true, .. }
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
    points: [[f32; 3]; 4],
    color: [u8; 3],
    opacity: f32,
    emission: f32,
) {
    if opacity >= 0.985 && emission > 0.0 {
        let [a, b, c, d] = points;
        mesh.luminous_triangle([a, b, c], color, emission);
        mesh.luminous_triangle([a, c, d], color, emission);
    } else {
        mesh.faded_quad(points, color, opacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distant_freight_stays_inside_its_banked_depth_region() {
        let trip = Timetable::new(640.0, TrainService::Freight, FreightRoute::Mixed, 71).unwrap();
        let ticks = 11_400;
        let center = trip.position(ticks)[0];
        let point = |x| RailPoint { x, z: 0.0, elevation: 12.0 };
        for side in [48, 60] {
            let dimensions = (side, side);
            let camera = Camera { x: center, z: -442.0, altitude: 108.0,
                heading: 0.0, pitch: 0.215, bank: -0.49 };
            let trip = Timetable::new(640.0, TrainService::Freight, FreightRoute::Mixed, 71).unwrap();
            let train = Train::on_route(trip, 71, ticks, camera, dimensions, FreightRoute::Mixed, false, None, point).unwrap();
            assert!(!train.is_near());
            let sky = crate::sky::Sky::from_time(165, ticks, 0.45);
            let region = train.depth_region();
            let count = side * side;
            let mut pixels = std::vec![0x86b7; count];
            let mut depth = std::vec![u16::MAX; count];
            let render = |pixels: &mut [u16], depth: &mut [u16]| {
                train.draw(&mut WorldMesh::new(pixels, depth, dimensions, camera, &sky).unwrap(), sky.daylight);
            };
            render(&mut pixels, &mut depth);
            let mut count = 0;
            for (index, &distance) in depth.iter().enumerate() {
                if distance != u16::MAX {
                    assert!((region.left..region.right).contains(&(index % side)));
                    assert!((region.top..region.bottom).contains(&(index / side)));
                    count += 1;
                }
            }
            assert!(count >= 4);
            pixels.fill(0x86b7);
            depth.fill(0);
            render(&mut pixels, &mut depth);
            assert!(pixels.iter().all(|&pixel| pixel == 0x86b7));
            assert!(depth.iter().all(|&distance| distance == 0));
        }
    }

    #[test]
    fn visible_pair_prioritizes_near_trains_without_reordering_them() {
        let trip = Timetable::new(640.0, TrainService::Freight, FreightRoute::Mixed, 71).unwrap();
        let ticks = 11_400;
        let center = trip.position(ticks)[0];
        let point = |x| RailPoint { x, z: 0.0, elevation: 12.0 };
        let train_at = |distance: f32, key: u64| {
            let trip = Timetable::new(640.0, TrainService::Freight, FreightRoute::Mixed, 71).unwrap();
            let camera = Camera { x: center, z: -distance, altitude: 38.0,
                heading: 0.0, pitch: 0.19, bank: 0.0 };
            Train::on_route(trip, key, ticks, camera, (60, 60), FreightRoute::Mixed, false, None, point).unwrap()
        };
        let far = train_at(401.0, 1);
        let close = train_at(116.0, 2);
        assert!(!far.is_near() && close.is_near());
        let near_stripe = close.stripe;
        let farther_stripe = far.stripe;
        let [first, second] = Train::visible_pair([far, close]);
        assert_eq!(first.unwrap().stripe, near_stripe);
        assert_eq!(second.unwrap().stripe, farther_stripe);
        let [first, second] = Train::visible_pair([train_at(401.0, 3), train_at(112.0, 4), train_at(124.0, 5)]);
        assert_eq!(first.unwrap().stripe, train_at(112.0, 4).stripe);
        assert_eq!(second.unwrap().stripe, train_at(124.0, 5).stripe);
    }

    #[test]
    fn express_cruises_faster_and_both_livery_banks_have_25_colors() {
        let length = 1100.0;
        let express = Timetable::new(length, TrainService::Express, FreightRoute::Mixed, 21).unwrap();
        let regional = Timetable::new(length, TrainService::RegionalPassenger, FreightRoute::Mixed, 21).unwrap();
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
            let trip = Timetable::new(480.0, service.into(), FreightRoute::Mixed, 719_391).unwrap();
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
        let train = Timetable::new(533.0, TrainService::RegionalPassenger, FreightRoute::Mixed, key).unwrap();
        let local = Timetable::new(533.0, TrainService::Commuter, FreightRoute::Mixed, key).unwrap();
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
        let timetable = Timetable::new(480.0, RailService::Freight.into(), FreightRoute::Mixed, 719_391).unwrap();
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
    fn fast_intermodal_and_long_heavy_freight_have_distinct_timetables() {
        let timetable = |length, route| Timetable::new(length, TrainService::Freight, route, 7907).unwrap();
        let containers = timetable(800.0, FreightRoute::Intermodal);
        let rock = timetable(800.0, FreightRoute::Aggregate);
        let branch = timetable(180.0, FreightRoute::Aggregate);
        assert!(containers.trip < rock.trip);
        assert_eq!((containers.cars, rock.cars, branch.cars), (7, MAX_CARS, 5));
        assert!(rock.low > containers.low && rock.high < containers.high);
        assert!(branch.high > branch.low);
    }

    #[test]
    fn dedicated_bulk_cars_change_cargo_during_terminal_dwell() {
        let trip = Timetable::new(800.0, TrainService::Freight, FreightRoute::Aggregate, 7907)
            .unwrap();
        let period = 2 * trip.trip + trip.layover_a + trip.layover_b;
        let second_arrival = trip.layover_a + trip.trip;
        let second_departure = second_arrival + trip.layover_b;
        let cargo = |source, phase| {
            let ticks = (phase + period - trip.offset) % period;
            let stage = trip.cargo_stage(ticks, source, 6);
            core::array::from_fn::<_, 6, _>(|wagon| stage.present(wagon))
        };

        for source in [LoadingEnd::First, LoadingEnd::Second] {
            let loaded_outbound = source == LoadingEnd::First;
            let initial = [!loaded_outbound; 6];
            let outbound = [loaded_outbound; 6];
            let servicing = [loaded_outbound, loaded_outbound, loaded_outbound,
                             !loaded_outbound, !loaded_outbound, !loaded_outbound];
            let cargo = |phase| cargo(Some(source), phase);
            assert_eq!(cargo(0), initial);
            assert_eq!(cargo(trip.layover_a / 2), servicing);
            assert_eq!(cargo(trip.layover_a - 1), outbound);
            assert_eq!(cargo(trip.layover_a + trip.trip / 2), outbound);
            assert_eq!(cargo(second_arrival), outbound);
            assert_eq!(cargo(second_arrival + trip.layover_b / 2), servicing);
            assert_eq!(cargo(second_departure - 1), initial);
            assert_eq!(cargo(second_departure + trip.trip / 2), initial);
            assert_eq!(cargo(period), initial);
        }

        for phase in [0, trip.layover_a / 2, second_arrival, second_departure, period] {
            assert_eq!(cargo(None, phase), [true; 6]);
        }
    }

    #[test]
    fn freight_clearance_covers_short_and_long_equipment() {
        use FreightRoute::*;

        for route in [Mixed, Intermodal, Agriculture, Liquid, Mill, Forest, Automotive, Perishable, Quarry, Aggregate] {
            for key in 0..128 {
                let timetable = Timetable::new(900.0, TrainService::Freight, route, key).unwrap();
                let formation = FreightFormation::new(timetable.cars, route, key, false);
                assert!(formation.half_span + 2.29 <= timetable.low);
            }
        }
        for variant in 0..freight_catalog::DESIGN_COUNT {
            assert!((10.6..=LONGEST_WAGON).contains(&freight_catalog::design(variant).body_length()));
        }
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
        for (index, car) in train.cars.iter().enumerate().filter_map(|(index, car)| Some((index, car.as_ref()?))) {
            let expected_length = match car.vehicle {
                Vehicle::Wagon { variant, .. } => freight_catalog::design(variant).body_length(),
                _ => CAR_LENGTH,
            };
            assert_eq!(car.length, expected_length);
            if let Some(next) = train.cars.get(index + 1).copied().flatten() {
                let gap_x = car.b.x - next.a.x;
                let gap_y = car.b.elevation - next.a.elevation;
                let gap_z = car.b.z - next.a.z;
                let gap_squared = gap_x * gap_x + gap_y * gap_y + gap_z * gap_z;
                assert!(((FREIGHT_GAP - 0.05).powi(2)..=(FREIGHT_GAP + 0.05).powi(2)).contains(&gap_squared));
            }
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
