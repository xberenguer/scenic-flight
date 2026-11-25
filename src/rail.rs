//! Seeded rail corridors between real regional destinations.

use crate::math::mix;
use crate::rail_station_art::StationFixtures;
use crate::roads::RoadNetwork;
use crate::world::{World, SEA_LEVEL};

const LANES: usize = 15;
const CENTER: usize = LANES / 2;
const LANE: f32 = 8.0;
const TURNS: usize = 5;
const STRAIGHT: usize = 2;
const STATES: usize = LANES * TURNS;
const MAX_POINTS: usize = 49;
const NOT_REACHED: u16 = u16::MAX;
const TRACK_HALF: f32 = 1.44;
const GRADE: f32 = 0.039;
const BED_GRADE: f32 = 0.097;
const LEVELING: f32 = 1.02;
const DRY: f32 = SEA_LEVEL + 3.25;
const MIN_LENGTH: f32 = 235.0;
const MAX_LENGTH: f32 = 1240.0;
const PLATFORM_FIXTURES: [f32; 2] = [-6.56, 6.56];
const FIXTURE_OUTSIDE: f32 = 1.31;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RailAnchorKind {
    RoadTerminus,
    Town,
    Port,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailAnchor {
    pub x: f32,
    pub z: f32,
    pub kind: RailAnchorKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RailService {
    Commuter,
    Freight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RailMaterial {
    Ballast,
    Sleepers,
    Steel,
    StationPlatform,
    FreightPlatform,
    StationRoof,
    StationAccess,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailSample {
    pub material: RailMaterial,
    pub elevation: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailPoint {
    pub x: f32,
    pub z: f32,
    pub elevation: f32,
}

impl RailPoint {
    const EMPTY: Self = Self {
        x: 0.0,
        z: 0.0,
        elevation: SEA_LEVEL,
    };

    fn at(world: &World, p: Point) -> Self {
        Self {
            x: p.x,
            z: p.z,
            elevation: world.elevation(p.x, p.z),
        }
    }

    fn position(self) -> Point {
        Point::new(self.x, self.z)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailStation {
    pub anchor: RailAnchor,
    pub track_point: RailPoint,
    platform: Platform,
    access: Trail,
    roof_base: f32,
    fixture_heights: [f32; 2],
    fixture_mask: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RailLine {
    pub service: RailService,
    stations: [RailStation; 2],
    axis: Axis,
    intervals: f32,
    points: [RailPoint; MAX_POINTS],
    run: [f32; MAX_POINTS],
    used: u8,
    siding: Option<[RailPoint; 4]>,
    bounds: Bounds,
}

impl RailLine {
    /// Choose an occasional corridor serving the actual ends of a highway.
    pub fn for_road(world: &World, seed: u64, road: &RoadNetwork) -> Option<Self> {
        Self::for_road_with_terminals(world, seed, road, [RailAnchorKind::RoadTerminus; 2])
    }

    /// Classify the two existing road ends after settlements or ports are known.
    /// The rail grade still clears the highway away from station access.
    pub fn for_road_with_terminals(
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        kinds: [RailAnchorKind; 2],
    ) -> Option<Self> {
        Self::for_road_cooperatively(world, seed, road, kinds, &mut || {})
    }

    pub(crate) fn for_road_cooperatively(
        world: &World, seed: u64, road: &RoadNetwork, kinds: [RailAnchorKind; 2],
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let polyline = road.polyline();
        let first = polyline.first()?;
        let last = polyline.last()?;
        let a = RailAnchor {
            x: first.x,
            z: first.z,
            kind: kinds[0],
        };
        let b = RailAnchor {
            x: last.x,
            z: last.z,
            kind: kinds[1],
        };
        let key = anchor_key(seed, a, b);
        if key & 7 >= 3 {
            return None;
        }
        Self::connect(world, key, a, b, Some(road), pulse)
    }

    /// Connect two existing destinations, such as settlements or port aprons.
    /// Reversing the anchors returns exactly the same canonical corridor.
    pub fn between(world: &World, seed: u64, a: RailAnchor, b: RailAnchor) -> Option<Self> {
        let (a, b) = canonical(a, b);
        Self::connect(world, anchor_key(seed, a, b), a, b, None, &mut || {})
    }

    pub fn polyline(&self) -> &[RailPoint] {
        &self.points[..self.used as usize]
    }

    pub(crate) fn length(&self) -> f32 {
        self.run[self.used as usize - 1]
    }

    pub(crate) fn point_at(&self, distance: f32) -> RailPoint {
        let runs = &self.run[..self.used as usize];
        let i = runs.partition_point(|&along| along <= distance)
            .saturating_sub(1).min(runs.len() - 2);
        let t = ((distance - runs[i]) / (runs[i + 1] - runs[i])).clamp(0.0, 1.0);
        let a = self.points[i];
        let b = self.points[i + 1];
        RailPoint {
            x: a.x + (b.x - a.x) * t,
            z: a.z + (b.z - a.z) * t,
            elevation: a.elevation + (b.elevation - a.elevation) * t,
        }
    }

    /// World bounds in minimum-X, minimum-Z, maximum-X, maximum-Z order.
    pub fn bounds(&self) -> [f32; 4] {
        self.bounds.coordinates()
    }

    pub fn stations(&self) -> &[RailStation; 2] {
        &self.stations
    }

    pub fn siding(&self) -> Option<&[RailPoint; 4]> {
        self.siding.as_ref()
    }

    /// Conservative sample areas for prepared facility coverage. Each rail
    /// span uses its swept width, with extra room for world-space rounding.
    pub(crate) fn sample_bounds(&self, mut emit: impl FnMut([f32; 4])) {
        const ROUNDING: f32 = 2.0;
        for pair in self.polyline().windows(2)
            .chain(self.siding().into_iter().flat_map(|points| points.windows(2)))
        {
            let mut bounds = Bounds::at(pair[0].position());
            bounds.include(pair[0].position(), TRACK_HALF + ROUNDING);
            bounds.include(pair[1].position(), TRACK_HALF + ROUNDING);
            emit(bounds.coordinates());
        }
        for station in self.stations {
            let corners = station.platform.corners();
            let mut platform = Bounds::at(corners[0]);
            for point in corners {
                platform.include(point, ROUNDING);
            }
            emit(platform.coordinates());

            let mut access = Bounds::at(station.access.a);
            access.include(station.access.a, 1.25 + ROUNDING);
            access.include(station.access.b, 1.25 + ROUNDING);
            emit(access.coordinates());
        }
    }

    /// The surface query has a bounds rejection and examines at most three
    /// adjacent main-line segments. Terrain probes are confined to planning.
    pub fn sample(&self, x: f32, z: f32) -> Option<RailSample> {
        let point = Point::new(x, z);
        if !self.bounds.contains(point) {
            return None;
        }
        let (along, across) = self.axis.coordinates(point);
        let segments = self.used as usize - 1;
        let estimate = ((along / self.intervals) as usize).min(segments - 1);
        if across.abs() < 70.0 {
            let mut near = None::<TrackHit>;
            for index in estimate.saturating_sub(1)..=(estimate + 1).min(segments - 1) {
                if let Some(hit) = TrackHit::from(
                    point,
                    self.points[index],
                    self.points[index + 1],
                    self.run[index],
                    Some(self.run[index + 1] - self.run[index]),
                ) {
                    if near.is_none_or(|old| hit.distance < old.distance) {
                        near = Some(hit);
                    }
                }
            }
            if let Some(hit) = near {
                return Some(hit.material());
            }
        }
        if let Some(siding) = self.siding {
            let low = siding[0].position();
            let high = siding[3].position();
            if (point.x - low.x).abs().min((point.x - high.x).abs()) < 90.0
                && (point.z - low.z).abs().min((point.z - high.z).abs()) < 90.0
            {
                for pair in siding.windows(2) {
                    if let Some(hit) = TrackHit::from(point, pair[0], pair[1], 0.0, None) {
                        return Some(hit.material());
                    }
                }
            }
        }
        self.stations
            .iter()
            .find_map(|station| station.sample(point, self.service))
    }

    fn connect(
        world: &World,
        key: u64,
        a: RailAnchor,
        b: RailAnchor,
        road: Option<&RoadNetwork>,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let (a, b) = canonical(a, b);
        if !valid_anchor(a) || !valid_anchor(b) {
            return None;
        }
        let start = Point::new(a.x, a.z);
        let end = Point::new(b.x, b.z);
        let length = start.distance_from(end);
        if !(MIN_LENGTH..=MAX_LENGTH).contains(&length) {
            return None;
        }
        let base = Axis::through(start, end);
        let sign = if key & 16 == 0 { 1.0 } else { -1.0 };
        let service = if a.kind == RailAnchorKind::Port || b.kind == RailAnchorKind::Port {
            RailService::Freight
        } else {
            RailService::Commuter
        };
        for offset in [16.0, -16.0, 23.0, -23.0, 30.0, -30.0] {
            pulse();
            let side = offset * sign;
            let axis = Axis {
                origin: base.world(0.0, side),
                ..base
            };
            let Some(first) = station(world, axis, 0.0, 1.0, -side.signum(), a, road) else {
                continue;
            };
            pulse();
            let Some(last) = station(world, axis, length, -1.0, -side.signum(), b, road) else {
                continue;
            };
            let Some(raw) = trace(world, axis, length, road, pulse) else {
                continue;
            };
            let Some(mut line) =
                Self::checked(world, axis, length, raw, [first, last], service, road, pulse)
            else {
                continue;
            };
            if service == RailService::Freight || (length >= 350.0 && key & 8 != 0) {
                line.siding = line.passing_loop(world, side.signum(), road, pulse);
                if let Some(points) = line.siding {
                    for point in points {
                        line.bounds.include(point.position(), TRACK_HALF);
                    }
                }
            }
            for index in 0..line.stations.len() {
                let fixtures = line.stations[index].fixtures();
                line.stations[index].fixture_mask = fixtures.fit(|x, z| {
                    line.sample(x, z).map(|sample| sample.material)
                });
            }
            return Some(line);
        }
        None
    }

    fn checked(
        world: &World,
        axis: Axis,
        length: f32,
        raw: Corridor,
        mut stations: [RailStation; 2],
        service: RailService,
        road: Option<&RoadNetwork>,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let mut points = [RailPoint::EMPTY; MAX_POINTS];
        let step = length / raw.count as f32;
        for (index, entry) in points.iter_mut().enumerate().take(raw.count + 1) {
            let across = (raw.lanes[index] as i32 - CENTER as i32) as f32 * LANE;
            *entry = RailPoint::at(world, axis.world(index as f32 * step, across));
        }
        let used = raw.count + 1;
        for edge in points[..used].windows(2) {
            pulse();
            if !trackbed(world, edge[0].position(), edge[1].position(), road) {
                return None;
            }
        }
        for i in 2..used - 2 {
            pulse();
            let a = points[i - 1];
            let b = points[i];
            let c = points[i + 1];
            let moved = Point::new(
                b.x + (a.x + c.x - 2.0 * b.x) * 0.17,
                b.z + (a.z + c.z - 2.0 * b.z) * 0.17,
            );
            if moved == b.position() {
                continue;
            }
            if turn_allowed(points[i - 2].position(), a.position(), moved)
                && turn_allowed(a.position(), moved, c.position())
                && turn_allowed(moved, c.position(), points[i + 2].position())
                && trackbed(world, a.position(), moved, road)
                && trackbed(world, moved, c.position(), road)
            {
                points[i] = RailPoint::at(world, moved);
            }
        }
        if points[..used]
            .windows(3)
            .any(|p| !turn_allowed(p[0].position(), p[1].position(), p[2].position()))
        {
            return None;
        }
        if !fit_grade(&mut points[..used], 0.16)
            || points[..used]
                .windows(2)
                .any(|p| { pulse(); !firm_grade(world, p[0], p[1]) })
        {
            return None;
        }
        stations[0].track_point = points[0];
        stations[1].track_point = points[used - 1];
        let mut run = [0.0; MAX_POINTS];
        let mut bounds = Bounds::at(points[0].position());
        for i in 0..used {
            let point = points[i].position();
            bounds.include(point, TRACK_HALF);
            if i > 0 {
                run[i] = run[i - 1] + points[i - 1].position().distance_from(point);
            }
        }
        for station in stations {
            for p in station.platform.corners() {
                bounds.include(p, 0.0);
            }
            bounds.include(station.access.a, 1.25);
            bounds.include(station.access.b, 1.25);
        }
        Some(Self {
            service,
            stations,
            axis,
            intervals: step,
            points,
            run,
            used: used as u8,
            siding: None,
            bounds,
        })
    }

    fn passing_loop(
        &self,
        world: &World,
        outward: f32,
        road: Option<&RoadNetwork>,
        pulse: &mut impl FnMut(),
    ) -> Option<[RailPoint; 4]> {
        let route = self.polyline();
        let center = route.len() / 2;
        for first in [center - 2, center, center - 4] {
            let span = route.get(first..first + 4)?;
            let a = span[0];
            let d = span[3];
            let (u1, v1) = self.axis.coordinates(span[1].position());
            let (u2, v2) = self.axis.coordinates(span[2].position());
            for sign in [outward, -outward] {
                pulse();
                let b = RailPoint::at(world, self.axis.world(u1, v1 + sign * 4.3));
                let c = RailPoint::at(world, self.axis.world(u2, v2 + sign * 4.3));
                let mut candidate = [a, b, c, d];
                if candidate
                    .windows(2)
                    .all(|p| trackbed(world, p[0].position(), p[1].position(), road))
                    && candidate
                        .windows(3)
                        .all(|p| turn_allowed(p[0].position(), p[1].position(), p[2].position()))
                {
                    if fit_grade(&mut candidate, 0.0)
                        && candidate.windows(2).all(|p| firm_grade(world, p[0], p[1]))
                    {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    }
}

impl RailStation {
    pub(crate) fn fixtures(&self) -> StationFixtures {
        let axis = self.platform.axis;
        let along = [axis.direction.x, axis.direction.z];
        let side = self.platform.outside;
        let outside = [-along[1] * side, along[0] * side];
        let mounts = core::array::from_fn(|index| {
            let p = axis.world(PLATFORM_FIXTURES[index], side * FIXTURE_OUTSIDE);
            [p.x, self.fixture_heights[index], p.z]
        });
        StationFixtures::new(
            mounts, along, outside, self.anchor.kind == RailAnchorKind::Port, self.fixture_mask,
        )
    }

    pub fn road_contact(&self) -> (f32, f32) {
        (self.anchor.x, self.anchor.z)
    }

    fn sample(self, point: Point, kind: RailService) -> Option<RailSample> {
        let (u, v) = self.platform.axis.coordinates(point);
        if u.abs() <= 4.3 && (v - self.platform.outside * 0.35).abs() <= 0.95 {
            return Some(RailSample {
                material: RailMaterial::StationRoof,
                elevation: Some(self.roof_base + 3.3),
            });
        }
        if u.abs() <= self.platform.half_length && v.abs() <= 1.78 {
            let material = if kind == RailService::Freight {
                RailMaterial::FreightPlatform
            } else {
                RailMaterial::StationPlatform
            };
            return Some(ground(material));
        }
        self.access.contains(point, 1.25)
            .then_some(ground(RailMaterial::StationAccess))
    }
}

fn ground(material: RailMaterial) -> RailSample {
    RailSample {
        material,
        elevation: None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f32,
    z: f32,
}

impl Point {
    const fn new(x: f32, z: f32) -> Self {
        Self { x, z }
    }
    fn difference(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.z - other.z)
    }
    fn squared(self) -> f32 {
        self.x * self.x + self.z * self.z
    }

    // A clamped projection stays between zero and the segment delta on
    // each axis. Use relative coordinates so the guard matches its arithmetic.
    fn outside_segment_box(self, axis: Self, padded_radius: f32) -> bool {
        self.z - axis.z.max(0.0) > padded_radius
            || axis.z.min(0.0) - self.z > padded_radius
            || self.x - axis.x.max(0.0) > padded_radius
            || axis.x.min(0.0) - self.x > padded_radius
    }

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.z * other.z
    }
    fn distance_from(self, other: Self) -> f32 {
        let p = self.difference(other);
        vector_length(p.x, p.z)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Axis {
    origin: Point,
    direction: Point,
}

impl Axis {
    fn through(a: Point, b: Point) -> Self {
        let distance = a.distance_from(b);
        let d = b.difference(a);
        Self {
            origin: a,
            direction: Point::new(d.x / distance, d.z / distance),
        }
    }
    fn coordinates(self, p: Point) -> (f32, f32) {
        let d = p.difference(self.origin);
        (
            d.dot(self.direction),
            d.z * self.direction.x - d.x * self.direction.z,
        )
    }
    fn world(self, x: f32, z: f32) -> Point {
        Point::new(
            self.origin.x + x * self.direction.x - z * self.direction.z,
            self.origin.z + x * self.direction.z + z * self.direction.x,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Platform {
    axis: Axis,
    half_length: f32,
    outside: f32,
}

impl Platform {
    fn corners(self) -> [Point; 4] {
        [
            self.axis.world(-self.half_length, -1.78),
            self.axis.world(self.half_length, -1.78),
            self.axis.world(-self.half_length, 1.78),
            self.axis.world(self.half_length, 1.78),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Trail {
    a: Point,
    b: Point,
}

impl Trail {
    fn contains(self, p: Point, radius: f32) -> bool {
        let axis = self.b.difference(self.a);
        let sample = p.difference(self.a);
        if sample.outside_segment_box(axis, radius + 0.05) {
            return false;
        }
        let t = (sample.dot(axis) / axis.squared()).clamp(0.0, 1.0);
        let distance = Point::new(sample.x - t * axis.x, sample.z - t * axis.z).squared();
        distance <= radius * radius
    }
}

#[derive(Clone, Copy)]
struct TrackHit {
    distance: f32,
    across_squared: f32,
    along: f32,
    elevation: f32,
}

impl TrackHit {
    fn from(p: Point, a: RailPoint, b: RailPoint, run: f32, length: Option<f32>) -> Option<Self> {
        let rise = b.elevation - a.elevation;
        let elevation = a.elevation;
        let a = a.position();
        let b = b.position();
        let axis = b.difference(a);
        let offset = p.difference(a);
        if offset.outside_segment_box(axis, TRACK_HALF + 0.06) {
            return None;
        }
        let inverse = 1.0 / axis.squared();
        let factor = (offset.dot(axis) * inverse).clamp(0., 1.);
        let cross = Point::new(offset.x - axis.x * factor, offset.z - axis.z * factor);
        let distance = cross.squared();
        if distance > TRACK_HALF * TRACK_HALF {
            return None;
        }
        let lateral = cross.z * axis.x - cross.x * axis.z;
        Some(Self {
            distance,
            across_squared: lateral * lateral * inverse,
            along: run + length.unwrap_or_else(|| vector_length(axis.x, axis.z)) * factor,
            elevation: elevation + rise * factor,
        })
    }
    fn material(self) -> RailSample {
        let material = if (0.54 * 0.54..0.90 * 0.90).contains(&self.across_squared) {
            RailMaterial::Steel
        } else if self.distance < 1.16 * 1.16 && self.along % 2.0 < 0.84 {
            RailMaterial::Sleepers
        } else {
            RailMaterial::Ballast
        };
        RailSample {
            material,
            elevation: Some(self.elevation),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    min: Point,
    max: Point,
}

impl Bounds {
    fn at(p: Point) -> Self {
        Self { min: p, max: p }
    }
    fn include(&mut self, p: Point, pad: f32) {
        self.min.x = self.min.x.min(p.x - pad);
        self.min.z = self.min.z.min(p.z - pad);
        self.max.x = self.max.x.max(p.x + pad);
        self.max.z = self.max.z.max(p.z + pad);
    }
    fn coordinates(self) -> [f32; 4] {
        [self.min.x, self.min.z, self.max.x, self.max.z]
    }
    fn contains(self, p: Point) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.z >= self.min.z && p.z <= self.max.z
    }
}

struct Corridor {
    lanes: [u8; MAX_POINTS],
    count: usize,
}

fn trace(
    world: &World, axis: Axis, length: f32, road: Option<&RoadNetwork>, pulse: &mut impl FnMut(),
) -> Option<Corridor> {
    let count = ((length / 26.0) as usize + 1).min(MAX_POINTS - 1);
    let step = length / count as f32;
    let mut parents = [[u8::MAX; STATES]; MAX_POINTS];
    let mut active = [NOT_REACHED; STATES];
    active[CENTER * TURNS + STRAIGHT] = 0;
    for (column, parent_row) in parents.iter_mut().enumerate().take(count + 1).skip(1) {
        let along = (column - 1) as f32 * step;
        let mut next = [NOT_REACHED; STATES];
        for lane in 0..LANES {
            pulse();
            let from_base = lane * TURNS;
            let start = axis.world(along, lane_world(lane));
            for velocity in 0..TURNS {
                let shift = velocity as i32 - STRAIGHT as i32;
                let target = lane as i32 + shift;
                if !(0..LANES as i32).contains(&target) {
                    continue;
                }
                let target = target as usize;
                if (column == 1 || column == count) && target != CENTER {
                    continue;
                }
                if column == count && velocity != STRAIGHT {
                    continue;
                }
                if target.abs_diff(CENTER) > 2 * (count - column) {
                    continue;
                }
                let mut lowest = NOT_REACHED;
                let mut previous = u8::MAX;
                for old in velocity.saturating_sub(1)..=(velocity + 1).min(TURNS - 1) {
                    let before = active[from_base + old];
                    let candidate = before.saturating_add((old.abs_diff(velocity) * 9) as u16);
                    if candidate < lowest {
                        lowest = candidate;
                        previous = old as u8;
                    }
                }
                if lowest == NOT_REACHED {
                    continue;
                }
                let end = axis.world(along + step, lane_world(target));
                let Some(cost) = coarse_grade(world, start, end, road) else {
                    continue;
                };
                let score = lowest.saturating_add(cost + shift.unsigned_abs() as u16 * 3);
                let key = target * TURNS + velocity;
                if score < next[key] {
                    next[key] = score;
                    parent_row[key] = previous;
                }
            }
        }
        if next.iter().all(|&cost| cost == NOT_REACHED) {
            return None;
        }
        active = next;
    }
    let mut corridor = Corridor {
        lanes: [CENTER as u8; MAX_POINTS],
        count,
    };
    let (mut lane, mut velocity) = (CENTER, STRAIGHT);
    for column in (1..=count).rev() {
        corridor.lanes[column] = lane as u8;
        let last = parents[column][lane * TURNS + velocity];
        if last == u8::MAX {
            return None;
        }
        lane = (lane as i32 + STRAIGHT as i32 - velocity as i32) as usize;
        velocity = last as usize;
    }
    Some(corridor)
}

fn lane_world(lane: usize) -> f32 {
    (lane as i32 - CENTER as i32) as f32 * LANE
}

fn height_ok(h: f32) -> bool {
    (DRY..=SEA_LEVEL + 53.0).contains(&h)
}

fn coarse_grade(world: &World, a: Point, b: Point, road: Option<&RoadNetwork>) -> Option<u16> {
    let d = b.difference(a);
    let length = vector_length(d.x, d.z);
    let ha = world.elevation(a.x, a.z);
    let hb = world.elevation(b.x, b.z);
    if !height_ok(ha) || !height_ok(hb) || (ha - hb).abs() > GRADE * length + LEVELING * 1.5 {
        return None;
    }
    let mut previous = ha;
    let mut cost = 0.0;
    for i in 0..=4 {
        let t = i as f32 * 0.25;
        let p = Point::new(a.x + d.x * t, a.z + d.z * t);
        if road.is_some_and(|r| r.proximity(p.x, p.z).is_some()) {
            return None;
        }
        let h = if i == 0 {
            ha
        } else if i == 4 {
            hb
        } else {
            world.elevation(p.x, p.z)
        };
        let rise = (h - previous).abs();
        if !height_ok(h) || rise > BED_GRADE * length * 0.25 + 0.025 {
            return None;
        }
        cost += rise;
        previous = h;
    }
    Some(24 + (cost * 6.4 + (ha.max(hb) - 20.0).max(0.0) * 0.8) as u16)
}

fn trackbed(world: &World, a: Point, b: Point, road: Option<&RoadNetwork>) -> bool {
    let axis = Axis::through(a, b);
    let length = a.distance_from(b);
    let count = (length / 1.35) as usize + 1;
    let delta = length / count as f32;
    let mut previous = [0.0; 3];
    for index in 0..=count {
        let mut next = [0.0; 3];
        for (j, across) in [-TRACK_HALF, 0.0, TRACK_HALF].into_iter().enumerate() {
            let p = axis.world(delta * index as f32, across);
            if road.is_some_and(|r| r.proximity(p.x, p.z).is_some()) {
                return false;
            }
            let height = world.elevation(p.x, p.z);
            if !height_ok(height)
                || (index != 0 && (height - previous[j]).abs() > delta * BED_GRADE + 0.0015)
            {
                return false;
            }
            next[j] = height;
        }
        if (next[0] - next[2]).abs() > 0.70 {
            return false;
        }
        previous = next;
    }
    true
}

/// Keep each level within shallow cut/fill of the ground while propagating
/// the range reachable at the specified train grade from the previous point.
fn fit_grade(points: &mut [RailPoint], endpoint_room: f32) -> bool {
    let last = points.len() - 1;
    let mut low = [0.0_f32; MAX_POINTS];
    let mut high = [0.0_f32; MAX_POINTS];
    low[0] = points[0].elevation - endpoint_room;
    high[0] = points[0].elevation + endpoint_room;
    for i in 1..=last {
        let distance = points[i].position().distance_from(points[i - 1].position());
        let range = distance * GRADE;
        let room = if i == last { endpoint_room } else { LEVELING };
        low[i] = (points[i].elevation - room).max(low[i - 1] - range);
        high[i] = (points[i].elevation + room).min(high[i - 1] + range);
        if low[i] > high[i] {
            return false;
        }
    }
    points[last].elevation = points[last].elevation.clamp(low[last], high[last]);
    for i in (0..last).rev() {
        let distance = points[i].position().distance_from(points[i + 1].position());
        let range = distance * GRADE;
        let lower = low[i].max(points[i + 1].elevation - range);
        let upper = high[i].min(points[i + 1].elevation + range);
        if lower > upper + 0.00005 {
            return false;
        }
        points[i].elevation = points[i].elevation.clamp(lower, upper.max(lower));
    }
    true
}

fn firm_grade(world: &World, a: RailPoint, b: RailPoint) -> bool {
    let axis = Axis::through(a.position(), b.position());
    let length = a.position().distance_from(b.position());
    let count = (length / 1.25) as usize + 1;
    let rise = b.elevation - a.elevation;
    if rise.abs() > GRADE * length + 0.00008 {
        return false;
    }
    for i in 0..=count {
        let t = i as f32 / count as f32;
        for across in [-TRACK_HALF, 0.0, TRACK_HALF] {
            let p = axis.world(t * length, across);
            let raw = world.elevation(p.x, p.z);
            let grade = a.elevation + rise * t;
            if !height_ok(raw) || (grade - raw).abs() > LEVELING + 0.16 {
                return false;
            }
        }
    }
    true
}

fn turn_allowed(a: Point, b: Point, c: Point) -> bool {
    let near = b.difference(a);
    let far = c.difference(b);
    let dot = near.dot(far);
    dot > 0.0 && dot * dot >= near.squared() * far.squared() * 0.9025
}

fn station(
    world: &World,
    axis: Axis,
    along: f32,
    inward: f32,
    side: f32,
    anchor: RailAnchor,
    road: Option<&RoadNetwork>,
) -> Option<RailStation> {
    let half_length = if anchor.kind == RailAnchorKind::Port {
        10.0
    } else {
        8.3
    };
    let platform = Platform {
        axis: Axis {
            origin: axis.world(along + (half_length + 0.55) * inward, side * 3.27),
            ..axis
        },
        half_length,
        outside: side,
    };
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for i in -4..=4 {
        for j in -2..=2 {
            let p = platform
                .axis
                .world(i as f32 * half_length / 4.0, j as f32 * 0.89);
            let h = world.elevation(p.x, p.z);
            if !height_ok(h) || road.is_some_and(|r| r.proximity(p.x, p.z).is_some()) {
                return None;
            }
            low = low.min(h);
            high = high.max(h);
            if high - low > 0.95 {
                return None;
            }
        }
    }
    let access = Trail {
        a: Point::new(anchor.x, anchor.z),
        b: platform.axis.world(0.0, side * 1.55),
    };
    if !path_access(world, access) {
        return None;
    }
    let track = RailPoint::at(world, axis.world(along, 0.0));
    if !height_ok(track.elevation) {
        return None;
    }
    Some(RailStation {
        anchor,
        track_point: track,
        platform,
        access,
        roof_base: high,
        fixture_heights: PLATFORM_FIXTURES.map(|u| {
            let p = platform.axis.world(u, side * FIXTURE_OUTSIDE);
            world.elevation(p.x, p.z)
        }),
        fixture_mask: 0,
    })
}

fn path_access(world: &World, access: Trail) -> bool {
    let axis = Axis::through(access.a, access.b);
    let length = access.a.distance_from(access.b);
    let count = (length / 1.65) as usize + 1;
    let step = length / count as f32;
    let mut previous = [0.0; 3];
    for i in 0..=count {
        for (j, side) in [-1.25, 0.0, 1.25].into_iter().enumerate() {
            let p = axis.world(step * i as f32, side);
            let h = world.elevation(p.x, p.z);
            if h < SEA_LEVEL + 1.8 || (i != 0 && (h - previous[j]).abs() > 0.135 * step + 0.003) {
                return false;
            }
            previous[j] = h;
        }
    }
    true
}

fn anchor_key(seed: u64, a: RailAnchor, b: RailAnchor) -> u64 {
    let (a, b) = canonical(a, b);
    let key = |p: RailAnchor| (u64::from(p.x.to_bits()) << 32) | u64::from(p.z.to_bits());
    mix(seed ^ mix(key(a)) ^ key(b) ^ 0x395d_d3a7_b716_8e0f)
}

fn canonical(a: RailAnchor, b: RailAnchor) -> (RailAnchor, RailAnchor) {
    if (a.x, a.z) <= (b.x, b.z) {
        (a, b)
    } else {
        (b, a)
    }
}

fn valid_anchor(a: RailAnchor) -> bool {
    a.x.abs() <= 16_000_000.0 && a.z.abs() <= 16_000_000.0
}

fn vector_length(x: f32, z: f32) -> f32 {
    let squared = x * x + z * z;
    let mut root = x.abs().max(z.abs());
    if root == 0.0 {
        return 0.0;
    }
    for _ in 0..3 {
        root = (root + squared / root) * 0.5;
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUTES: [(u64, i32, i32); 3] = [(1, -1, -3), (1, -4, -1), (119, -1, 3)];

    #[test]
    fn rail_anchors_are_actual_road_termini_and_repeatable() {
        assert!(core::mem::size_of::<RailLine>() < 1_200);
        for (seed, x, z) in ROUTES {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, x, z).unwrap();
            let rail = RailLine::for_road(&world, seed, &road).unwrap();
            assert_eq!(Some(rail.clone()), RailLine::for_road(&world, seed, &road));
            let road_points = road.polyline();
            for station in rail.stations() {
                let contact = station.road_contact();
                assert!([road_points[0], *road_points.last().unwrap()]
                    .iter()
                    .any(|p| (p.x, p.z) == contact));
                let t = rail.sample(contact.0, contact.1).unwrap();
                assert_eq!(t.material, RailMaterial::StationAccess);
                assert!(path_access(&world, station.access));
                let ramp = station.access.b;
                assert_eq!(
                    rail.sample(ramp.x, ramp.z).unwrap().material,
                    RailMaterial::StationPlatform
                );
                assert!(
                    rail.polyline().first() == Some(&station.track_point)
                        || rail.polyline().last() == Some(&station.track_point)
                );
            }
        }
    }

    #[test]
    fn full_track_profile_is_dry_gentle_and_keeps_clear_of_the_highway() {
        for (seed, x, z) in ROUTES {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, x, z).unwrap();
            let rail = RailLine::for_road(&world, seed, &road).unwrap();
            let points = rail.polyline();
            assert!(points.windows(3).all(|p| turn_allowed(
                p[0].position(),
                p[1].position(),
                p[2].position()
            )));
            for pair in points.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let len = a.position().distance_from(b.position());
                assert!((a.elevation - b.elevation).abs() / len <= GRADE + 0.00001);
                assert!(trackbed(&world, a.position(), b.position(), Some(&road)));
                assert!(firm_grade(&world, a, b));
                let axis = Axis::through(a.position(), b.position());
                for i in 1..16 {
                    let t = i as f32 / 16.0;
                    for v in [-1.1, 0.0, 1.1] {
                        let p = axis.world(t * len, v);
                        assert!(road.proximity(p.x, p.z).is_none());
                        let track = rail.sample(p.x, p.z).unwrap();
                        assert!(matches!(
                            track.material,
                            RailMaterial::Steel | RailMaterial::Sleepers | RailMaterial::Ballast
                        ));
                        let raw = world.elevation(p.x, p.z);
                        assert!(height_ok(raw));
                        assert!((track.elevation.unwrap() - raw).abs() <= LEVELING + 0.18);
                    }
                }
            }
        }
    }

    #[test]
    fn real_coastal_road_ends_support_a_freight_siding() {
        let world = World::new(1);
        let road = RoadNetwork::generate(&world, -1, -3).unwrap();
        let freight =
            RailLine::for_road_with_terminals(&world, 1, &road, [RailAnchorKind::Port; 2]).unwrap();
        assert_eq!(freight.service, RailService::Freight);
        let siding = freight.siding().unwrap();
        assert!(freight.polyline().contains(&siding[0]) && freight.polyline().contains(&siding[3]));
        for edge in siding.windows(2) {
            assert!(trackbed(
                &world,
                edge[0].position(),
                edge[1].position(),
                Some(&road)
            ));
            assert!(firm_grade(&world, edge[0], edge[1]));
        }
    }

    #[test]
    fn connecting_anchors_is_symmetric() {
        let world = World::new(1);
        let road = RoadNetwork::generate(&world, -1, -3).unwrap();
        let rail = RailLine::for_road(&world, 1, &road).unwrap();
        let a = rail.stations()[0].anchor;
        let b = rail.stations()[1].anchor;
        let line = RailLine::between(&world, 1, a, b).unwrap();
        assert_eq!(Some(line), RailLine::between(&world, 1, b, a));
        assert!(RailLine::between(&world, 1, a, a).is_none());
    }
}
