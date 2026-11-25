//! A paired regional railway survey. Both stations and the shared grade are
//! planned together, using an accepted dry road as a geographic reference.

use crate::highway_links::HighwayLink;
use crate::math::smoothstep;
use crate::ports::{Port, PortSurface};
use crate::rail::{RailAnchorKind, RailMaterial, RailPoint, RailService};
use crate::rail_station_art::StationFixtures;
use crate::roads::{ROAD_HALF_WIDTH, ROAD_REGION_SIZE};
use crate::settlement::{Settlement, SettlementSurface};
use crate::world::{World, SEA_LEVEL};

const STEP: f32 = 24.0;
const LANE: f32 = 6.0;
const LANES: usize = 43;
const CENTER: usize = LANES / 2;
const DIRECTIONS: usize = 5;
const STRAIGHT: usize = DIRECTIONS / 2;
const STATES: usize = LANES * DIRECTIONS;
const POINTS: usize = 66;
const GUIDE_POINTS: usize = 97;
const DRY: f32 = SEA_LEVEL + 1.8;
const MAX_GRADE: f32 = 0.039;
const ROOM: f32 = 3.25;
const BED: f32 = 1.5;
const EDGE: f32 = 6.5;
const PLATFORM_INNER: f32 = 2.15;
const PLATFORM_OUTER: f32 = 4.75;
const PLATFORM_END: f32 = 14.0;
const ACCESS_HALF: f32 = 1.2;
const ACCESS_AT: f32 = 5.0;
const EMPTY: RailPoint = RailPoint {
    x: 0.0,
    z: 0.0,
    elevation: 0.0,
};

type Xz = [f32; 2];

/// A copy of the accepted road's reference geometry, for detached surveys.
/// The caller retains authoritative road/town/facility reservations separately.
#[derive(Clone)]
pub struct CorridorGuide {
    regions: [[i32; 2]; 2],
    points: [Xz; GUIDE_POINTS],
    progress: [f32; GUIDE_POINTS],
    axis: Axis,
    bounds: [f32; 4],
    used: usize,
}

impl CorridorGuide {
    pub fn from_highway(link: &HighwayLink) -> Option<Self> {
        let road = link.polyline();
        if road.len() < 2 || road.len() > GUIDE_POINTS {
            return None;
        }
        let start = road[0];
        let finish = *road.last()?;
        let delta = [finish.x - start.x, finish.z - start.z];
        let axis = Axis {
            origin: [start.x, start.z],
            fwd: scale(delta, 1.0 / norm(delta)),
        };
        let mut points = [[0.0; 2]; GUIDE_POINTS];
        let mut progress = [0.0; GUIDE_POINTS];
        for ((slot, along), point) in points.iter_mut().zip(&mut progress).zip(road) {
            *slot = [point.x, point.z];
            *along = axis.project(*slot).0;
        }
        Some(Self {
            regions: link.contacts().map(|contact| contact.region),
            points,
            progress,
            axis,
            bounds: link.bounds(),
            used: road.len(),
        })
    }

    pub fn regions(&self) -> [[i32; 2]; 2] {
        self.regions
    }

    pub fn covers_road(&self, x: f32, z: f32) -> bool {
        let [left, near, right, far] = self.bounds;
        if !(left..=right).contains(&x) || !(near..=far).contains(&z) {
            return false;
        }
        let p = [x, z];
        let along = self.axis.project(p).0;
        let progress = &self.progress[..self.used];
        let road_margin = ROAD_HALF_WIDTH + 1.5;
        let reach = road_margin + 0.005;
        let first = progress
            .partition_point(|&v| v < along - reach)
            .saturating_sub(1);
        let last = progress
            .partition_point(|&v| v <= along + reach)
            .min(self.used - 1);
        (first..last)
            .any(|i| foot(p, self.points[i], self.points[i + 1]).1 <= road_margin * road_margin)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CorridorStop {
    region: [i32; 2],
    entry: Xz,
    kind: RailAnchorKind,
}

impl CorridorStop {
    /// `entry` can be the highway approach cell or one of the town's existing
    /// connected edge streets. No new street is inferred from coordinates.
    pub fn town(region: [i32; 2], entry: Xz, town: &Settlement) -> Option<Self> {
        (town.sample(entry[0], entry[1])?.surface == SettlementSurface::Street
            && inside(region, entry))
        .then_some(Self {
            region,
            entry,
            kind: RailAnchorKind::Town,
        })
    }

    /// A contact on a retained port's dry access lane or apron.
    pub fn port(world: &World, region: [i32; 2], entry: Xz, port: &Port) -> Option<Self> {
        let [x, z] = entry;
        (inside(region, entry)
            && matches!(
                port.sample(x, z, world.sample(x, z))?.surface,
                PortSurface::AccessRoad | PortSurface::Apron
            ))
        .then_some(Self {
            region,
            entry,
            kind: RailAnchorKind::Port,
        })
    }

    pub fn entry(self) -> Xz {
        self.entry
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorridorUse {
    Trackbed,
    Platform,
    Access,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CorridorStation {
    pub stop: CorridorStop,
    pub track: RailPoint,
    /// Unit direction from the end of the tracks into the platform length.
    pub inward: Xz,
    pub side: f32,
    pub walk_end: Xz,
    pub grade: f32,
    access_start_height: f32,
    fixture_mask: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CorridorSample {
    pub elevation: f32,
    /// An earthwork shoulder changes elevation while retaining native cover.
    pub material: Option<RailMaterial>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RailCorridor {
    pub service: RailService,
    stations: [CorridorStation; 2],
    points: [RailPoint; POINTS],
    run: [f32; POINTS],
    axis: Axis,
    step: f32,
    used: usize,
    bounds: [f32; 4],
}

/// Owned worker storage. Keep this out of the hot sampler and task stack.
#[derive(Clone, Copy)]
pub struct CorridorWork {
    raw: [[f32; LANES]; POINTS],
    back: [[u8; STATES]; POINTS],
    active: [Reach; STATES],
    next: [Reach; STATES],
}

#[derive(Clone, Copy)]
struct Reach {
    cost: f32,
    low: f32,
    high: f32,
}
const UNREACHED: Reach = Reach {
    cost: f32::INFINITY,
    low: 0.0,
    high: 0.0,
};

impl CorridorWork {
    pub const fn new() -> Self {
        Self {
            raw: [[0.0; LANES]; POINTS],
            back: [[u8::MAX; STATES]; POINTS],
            active: [UNREACHED; STATES],
            next: [UNREACHED; STATES],
        }
    }
}

impl RailCorridor {
    /// Survey one dry, approved neighboring highway pair. The outer highway
    /// shoulder, mixed terrain overlays and all authoritative reservations are
    /// provided by the cold caller. Access may meet an existing street surface.
    pub fn survey_cooperatively(
        world: &World,
        guide: &HighwayLink,
        stops: [CorridorStop; 2],
        work: &mut CorridorWork,
        blocked: impl FnMut(f32, f32, CorridorUse) -> bool,
        pulse: impl FnMut(),
    ) -> Option<Self> {
        Self::survey_detached_cooperatively(
            world,
            &CorridorGuide::from_highway(guide)?,
            stops,
            work,
            blocked,
            pulse,
        )
    }

    pub fn survey_detached_cooperatively(
        world: &World,
        guide: &CorridorGuide,
        mut stops: [CorridorStop; 2],
        work: &mut CorridorWork,
        mut blocked: impl FnMut(f32, f32, CorridorUse) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        if stops[0].region != guide.regions[0] {
            stops.swap(0, 1);
        }
        if [stops[0].region, stops[1].region] != guide.regions {
            return None;
        }
        let start = stops[0].entry;
        let finish = stops[1].entry;
        let length = distance(start, finish);
        if !(175.0..=1500.0).contains(&length) {
            return None;
        }
        let axis = Axis {
            origin: start,
            fwd: scale(sub(finish, start), 1.0 / length),
        };
        let count = ((length / STEP) as usize + 1).min(POINTS - 1);
        let step = length / count as f32;
        let line = Survey {
            axis,
            step,
            count,
            stops,
        };
        let mut sites = [[None; LANES]; 2];
        let mut guide_bias = [0.0_f32; POINTS];
        for index in 0..=count {
            pulse();
            let t = index as f32 * step;
            guide_bias[index] = guide_offset(guide, axis, t);
            for lane in 0..LANES {
                let p = line.point(index, lane);
                let h = world.elevation(p[0], p[1]);
                work.raw[index][lane] = if line.inside(p)
                    && h >= DRY
                    && h < 80.0
                    && !blocked(p[0], p[1], CorridorUse::Trackbed)
                {
                    h
                } else {
                    f32::NAN
                };
                if lane.abs_diff(CENTER) > 9 || lane.abs_diff(CENTER) < 2 {
                    continue;
                }
                if index == 0 || index == count {
                    let end = usize::from(index != 0);
                    sites[end][lane] = line.station(world, end, p, h, &mut blocked, &mut pulse);
                }
            }
        }
        if sites.iter().any(|ends| ends.iter().all(Option::is_none)) {
            return None;
        }
        // A coarse path occasionally fails its finer earthwork proof. Ban the
        // failing edge band and let the same bounded survey choose another one.
        for _ in 0..4 {
            let (mut lane, mut direction) = line.trace(work, &sites, &guide_bias, &mut pulse)?;
            let mut points = [EMPTY; POINTS];
            let mut lanes = [0_usize; POINTS];
            for i in (0..=count).rev() {
                let p = line.point(i, lane);
                points[i] = point(p, work.raw[i][lane]);
                lanes[i] = lane;
                if i != 0 {
                    let before = work.back[i][lane * DIRECTIONS + direction];
                    lane = (lane as i32 + STRAIGHT as i32 - direction as i32) as usize;
                    direction = before as usize;
                }
            }
            let mut stations = [sites[0][lanes[0]]?, sites[1][lanes[count]]?];
            let validation = line
                .profile(world, &mut points[..=count], &mut blocked, &mut pulse)
                .and_then(|()| {
                    for (station, (end, inside)) in
                        stations.iter_mut().zip([(0, 1), (count, count - 1)])
                    {
                        station.track = points[end];
                        station.grade = (points[inside].elevation - points[end].elevation)
                            / distance(position(points[end]), position(points[inside]));
                        if !station.connects_platform(world, &mut pulse) {
                            return Err(end);
                        }
                    }
                    Ok(())
                });
            if let Err(row) = validation {
                let lane = lanes[row];
                let first = lane.saturating_sub(1);
                let last = (lane + 1).min(LANES - 1);
                for i in first..=last {
                    work.raw[row][i] = f32::NAN;
                }
                continue;
            }
            let mut run = [0.0; POINTS];
            let mut bounds = [start[0], start[1], start[0], start[1]];
            for i in 0..=count {
                if i > 0 {
                    run[i] = run[i - 1] + distance(position(points[i - 1]), position(points[i]));
                }
                include(&mut bounds, position(points[i]), EDGE);
            }
            for station in stations {
                include(&mut bounds, station.stop.entry, ACCESS_HALF);
                include(&mut bounds, station.walk_end, ACCESS_HALF);
                include(
                    &mut bounds,
                    position(station.track),
                    PLATFORM_END + PLATFORM_OUTER,
                );
            }
            let service = if stops.iter().any(|stop| stop.kind == RailAnchorKind::Port) {
                RailService::Freight
            } else {
                RailService::Commuter
            };
            let mut route = Self {
                service,
                stations,
                points,
                run,
                axis,
                step,
                used: count + 1,
                bounds,
            };
            for index in 0..route.stations.len() {
                let fixtures = route.stations[index].fixtures();
                route.stations[index].fixture_mask = fixtures.fit(|x, z| {
                    route.sample(x, z, world.elevation(x, z))
                        .and_then(|sample| sample.material)
                });
            }
            return Some(route);
        }
        None
    }

    pub fn regions(&self) -> [[i32; 2]; 2] {
        self.stations.map(|station| station.stop.region)
    }
    pub fn polyline(&self) -> &[RailPoint] {
        &self.points[..self.used]
    }
    pub fn stations(&self) -> &[CorridorStation; 2] {
        &self.stations
    }
    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }
    pub fn length(&self) -> f32 {
        self.run[self.used - 1]
    }

    /// Local conservative areas for a cheap publication-time veto against
    /// already committed terrain overlays. Survey sampling is more precise.
    pub fn footprint_chunks(&self) -> impl Iterator<Item = [f32; 4]> + '_ {
        self.polyline()
            .windows(2)
            .map(|pair| span_bounds(position(pair[0]), position(pair[1]), EDGE))
            .chain(self.stations.into_iter().flat_map(|station| {
                let axis = station.axis();
                let [a, b, c, d] = [
                    (0.5, PLATFORM_INNER),
                    (0.5, PLATFORM_OUTER),
                    (PLATFORM_END, PLATFORM_INNER),
                    (PLATFORM_END, PLATFORM_OUTER),
                ]
                .map(|(u, v)| axis.at(u, v * station.side));
                let mut platform = span_bounds(a, b, 0.0);
                include(&mut platform, c, 0.0);
                include(&mut platform, d, 0.0);
                [
                    platform,
                    span_bounds(station.stop.entry, station.walk_end, ACCESS_HALF),
                ]
            }))
    }

    pub fn position_at(&self, distance: f32) -> RailPoint {
        let d = distance.clamp(0.0, self.length());
        let i = self.run[..self.used]
            .partition_point(|p| *p <= d)
            .saturating_sub(1)
            .min(self.used - 2);
        let t = (d - self.run[i]) / (self.run[i + 1] - self.run[i]);
        let a = self.points[i];
        let b = self.points[i + 1];
        point(
            lerp(position(a), position(b), t),
            blend(a.elevation, b.elevation, t),
        )
    }

    pub fn sample(&self, x: f32, z: f32, raw: f32) -> Option<CorridorSample> {
        let [left, near, right, far] = self.bounds;
        if !(left..=right).contains(&x) || !(near..=far).contains(&z) || raw <= SEA_LEVEL {
            return None;
        }
        let p = [x, z];
        let along = self.axis.project(p).0;
        let index = ((along / self.step) as usize).min(self.used - 2);
        let mut best = (EDGE * EDGE, 0.0, 0.0, 0.0);
        for i in index.saturating_sub(1)..=(index + 1).min(self.used - 2) {
            let a = self.points[i];
            let b = self.points[i + 1];
            let (t, r2, cross) = foot(p, position(a), position(b));
            if r2 < best.0 {
                best = (
                    r2,
                    blend(a.elevation, b.elevation, t),
                    blend(self.run[i], self.run[i + 1], t),
                    cross,
                );
            }
        }
        if best.0 <= BED * BED {
            let material = if (0.54 * 0.54..=0.9 * 0.9).contains(&best.3) {
                RailMaterial::Steel
            } else if best.2 % 2.0 < 0.85 && best.0 <= 1.16 * 1.16 {
                RailMaterial::Sleepers
            } else {
                RailMaterial::Ballast
            };
            return Some(CorridorSample {
                elevation: best.1,
                material: Some(material),
            });
        }
        for station in self.stations {
            let axis = station.axis();
            let (u, v) = axis.project(p);
            let side = v * station.side;
            if (0.5..=PLATFORM_END).contains(&u)
                && (PLATFORM_INNER..=PLATFORM_OUTER).contains(&side)
            {
                let material = if (4.5..=10.0).contains(&u) && (3.1..=4.3).contains(&side) {
                    RailMaterial::StationRoof
                } else if self.service == RailService::Freight {
                    RailMaterial::FreightPlatform
                } else {
                    RailMaterial::StationPlatform
                };
                return Some(CorridorSample {
                    elevation: station.floor(u)
                        + if material == RailMaterial::StationRoof {
                            2.9
                        } else {
                            0.0
                        },
                    material: Some(material),
                });
            }
            let (access, d2, _) = foot(p, station.stop.entry, station.walk_end);
            if d2 <= ACCESS_HALF * ACCESS_HALF {
                let grade = blend(
                    station.access_start_height,
                    station.floor(ACCESS_AT),
                    access,
                );
                let across = smoothstep(1.0 - d2 / (ACCESS_HALF * ACCESS_HALF));
                return Some(CorridorSample {
                    elevation: blend(raw, grade, across),
                    material: Some(RailMaterial::StationAccess),
                });
            }
        }
        let weight = smoothstep((EDGE * EDGE - best.0) / (EDGE * EDGE - BED * BED));
        (weight > 0.0).then_some(CorridorSample {
            elevation: blend(raw, best.1, weight),
            material: None,
        })
    }
}

impl CorridorStation {
    pub(crate) fn fixtures(self) -> StationFixtures {
        let axis = self.axis();
        let outside = [-self.inward[1] * self.side, self.inward[0] * self.side];
        let mounts = [2.21, 12.23].map(|along| {
            let [x, z] = axis.at(along, 4.28 * self.side);
            [x, self.floor(along), z]
        });
        StationFixtures::new(
            mounts, self.inward, outside, self.stop.kind == RailAnchorKind::Port, self.fixture_mask,
        )
    }

    fn axis(self) -> Axis {
        Axis {
            origin: position(self.track),
            fwd: self.inward,
        }
    }

    fn floor(self, along: f32) -> f32 {
        self.track.elevation + along * self.grade + 0.28
    }

    fn connects_platform(self, world: &World, pulse: &mut impl FnMut()) -> bool {
        let axis = self.axis();
        for row in 0..=14 {
            pulse();
            let along = 0.5 + row as f32 * (13.5 / 14.0);
            for side in [PLATFORM_INNER, 3.45, PLATFORM_OUTER] {
                let [x, z] = axis.at(along, self.side * side);
                if (self.floor(along) - world.elevation(x, z)).abs() > 1.55 {
                    return false;
                }
            }
        }
        let access = distance(self.stop.entry, self.walk_end);
        let target = self.floor(ACCESS_AT);
        if (self.access_start_height - target).abs() > access * 0.20 {
            return false;
        }
        for j in 0..=16 {
            let t = j as f32 / 16.0;
            let [x, z] = lerp(self.stop.entry, self.walk_end, t);
            let raw = world.elevation(x, z);
            if (raw - blend(self.access_start_height, target, t)).abs() > 0.9 {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Axis {
    origin: Xz,
    fwd: Xz,
}
impl Axis {
    fn at(self, along: f32, across: f32) -> Xz {
        [
            self.origin[0] + self.fwd[0] * along - self.fwd[1] * across,
            self.origin[1] + self.fwd[1] * along + self.fwd[0] * across,
        ]
    }
    fn project(self, p: Xz) -> (f32, f32) {
        let d = sub(p, self.origin);
        (dot(d, self.fwd), d[1] * self.fwd[0] - d[0] * self.fwd[1])
    }
}

struct Survey {
    axis: Axis,
    step: f32,
    count: usize,
    stops: [CorridorStop; 2],
}
impl Survey {
    fn point(&self, row: usize, lane: usize) -> Xz {
        self.axis.at(
            row as f32 * self.step,
            (lane as i32 - CENTER as i32) as f32 * LANE,
        )
    }
    fn inside(&self, p: Xz) -> bool {
        self.stops.iter().any(|stop| inside(stop.region, p))
    }

    fn station(
        &self,
        world: &World,
        end: usize,
        cap: Xz,
        h: f32,
        blocked: &mut impl FnMut(f32, f32, CorridorUse) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<CorridorStation> {
        if !h.is_finite() {
            return None;
        }
        let stop = self.stops[end];
        let inward = scale(self.axis.fwd, if end == 0 { 1.0 } else { -1.0 });
        let axis = Axis {
            origin: cap,
            fwd: inward,
        };
        let side = axis.project(stop.entry).1.signum();
        let walk_end = axis.at(ACCESS_AT, PLATFORM_OUTER * side);
        let width = distance(stop.entry, walk_end);
        if !(6.0..=62.0).contains(&width) {
            return None;
        }
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for row in 0..=14 {
            pulse();
            for v in [PLATFORM_INNER, 3.45, PLATFORM_OUTER] {
                let p = axis.at(0.5 + row as f32 * (13.5 / 14.0), side * v);
                let z = world.elevation(p[0], p[1]);
                if !inside(stop.region, p) || z < DRY || blocked(p[0], p[1], CorridorUse::Platform)
                {
                    return None;
                }
                lo = lo.min(z);
                hi = hi.max(z);
            }
            if hi - lo > 1.35 {
                return None;
            }
        }
        let walk_axis = Axis {
            origin: stop.entry,
            fwd: scale(sub(walk_end, stop.entry), 1.0 / width),
        };
        let rows = width as usize + 1;
        let step = width / rows as f32;
        let mut prev = [f32::NAN; 3];
        for i in 0..=rows {
            pulse();
            for (j, v) in [-ACCESS_HALF, 0.0, ACCESS_HALF].into_iter().enumerate() {
                let p = walk_axis.at(i as f32 * step, v);
                let h = world.elevation(p[0], p[1]);
                if !inside(stop.region, p)
                    || h < DRY
                    || (h - prev[j]).abs() > step * 0.17 + 0.002
                    || blocked(p[0], p[1], CorridorUse::Access)
                {
                    return None;
                }
                prev[j] = h;
            }
        }
        Some(CorridorStation {
            stop,
            track: point(cap, h),
            inward,
            side,
            walk_end,
            grade: 0.0,
            access_start_height: world.elevation(stop.entry[0], stop.entry[1]),
            fixture_mask: 0,
        })
    }

    fn trace(
        &self,
        work: &mut CorridorWork,
        sites: &[[Option<CorridorStation>; LANES]; 2],
        guide: &[f32; POINTS],
        pulse: &mut impl FnMut(),
    ) -> Option<(usize, usize)> {
        work.active.fill(UNREACHED);
        for (lane, site) in sites[0].iter().enumerate() {
            if let Some(station) = site {
                let h = station.track.elevation;
                if work.raw[0][lane].is_finite() {
                    work.active[lane * DIRECTIONS + STRAIGHT] = Reach {
                        cost: (lane.abs_diff(CENTER) as f32) * 0.15,
                        low: h - 0.5,
                        high: h + 0.5,
                    };
                }
            }
        }
        for i in 1..=self.count {
            work.next.fill(UNREACHED);
            work.back[i].fill(u8::MAX);
            let terminal = i == self.count;
            let room = if terminal { 0.50 } else { ROOM - 0.36 };
            for lane in 0..LANES {
                pulse();
                let h = work.raw[i][lane];
                if !h.is_finite() || (terminal && sites[1][lane].is_none()) {
                    continue;
                }
                let across = (lane as i32 - CENTER as i32) as f32 * LANE;
                let bias = (across - guide[i]).abs() * 0.014;
                for direction in 0..DIRECTIONS {
                    if (i == 1 || terminal) && direction != STRAIGHT {
                        continue;
                    }
                    let stride = direction as i32 - STRAIGHT as i32;
                    let from = lane as i32 - stride;
                    if !(0..LANES as i32).contains(&from) {
                        continue;
                    }
                    let from = from as usize;
                    let old_h = work.raw[i - 1][from];
                    if !old_h.is_finite() {
                        continue;
                    }
                    let reach = MAX_GRADE * norm([self.step, stride as f32 * LANE]);
                    let key = lane * DIRECTIONS + direction;
                    for prev in direction.saturating_sub(1)..=(direction + 1).min(DIRECTIONS - 1) {
                        let old = work.active[from * DIRECTIONS + prev];
                        if !old.cost.is_finite() {
                            continue;
                        }
                        let low = (h - room).max(old.low - reach).max(DRY);
                        let high = (h + room).min(old.high + reach);
                        let score = old.cost
                            + (h - old_h).abs() * 0.60
                            + bias
                            + stride.unsigned_abs() as f32 * 0.12
                            + direction.abs_diff(prev) as f32 * 0.47
                            + (h - (low + high) * 0.5).abs() * 0.31;
                        if low <= high && score < work.next[key].cost {
                            work.next[key] = Reach {
                                cost: score,
                                low,
                                high,
                            };
                            work.back[i][key] = prev as u8;
                        }
                    }
                }
            }
            core::mem::swap(&mut work.active, &mut work.next);
        }
        work.active
            .iter()
            .enumerate()
            .filter(|(_, v)| v.cost.is_finite())
            .min_by(|(_, a), (_, b)| a.cost.total_cmp(&b.cost))
            .map(|(index, _)| (index / DIRECTIONS, index % DIRECTIONS))
    }

    fn profile(
        &self,
        world: &World,
        points: &mut [RailPoint],
        blocked: &mut impl FnMut(f32, f32, CorridorUse) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Result<(), usize> {
        let end = points.len() - 1;
        let mut ranges = [[0.0; 2]; POINTS];
        for i in 1..=end {
            pulse();
            let a = position(points[i - 1]);
            let b = position(points[i]);
            let len = distance(a, b);
            let axis = Axis {
                origin: a,
                fwd: scale(sub(b, a), 1.0 / len),
            };
            let rows = len as usize + 1;
            // Check actual support, water, and published reservations across
            // the whole cut/fill footprint at approximately one world unit.
            for j in 0..=rows {
                let t = j as f32 / rows as f32;
                for v in [-EDGE, -BED, 0.0, BED, EDGE] {
                    let p = axis.at(len * t, v);
                    let raw = world.elevation(p[0], p[1]);
                    if raw < DRY || !self.inside(p) || blocked(p[0], p[1], CorridorUse::Trackbed) {
                        return Err(i);
                    }
                }
            }
            let rise = len * MAX_GRADE;
            let room = if i == end { 0.50 } else { ROOM - 0.30 };
            let h = points[i].elevation;
            let now = if i == 1 {
                [points[0].elevation - 0.5, points[0].elevation + 0.5]
            } else {
                ranges[i - 1]
            };
            if i == 1 {
                ranges[0] = now;
            }
            ranges[i] = [
                (h - room).max(now[0] - rise).max(DRY),
                (h + room).min(now[1] + rise),
            ];
            if ranges[i][0] > ranges[i][1] {
                return Err(i);
            }
        }
        points[end].elevation = points[end].elevation.clamp(ranges[end][0], ranges[end][1]);
        for i in (0..end).rev() {
            let rise = distance(position(points[i]), position(points[i + 1])) * MAX_GRADE;
            let h = points[i + 1].elevation;
            let low = ranges[i][0].max(h - rise);
            let high = ranges[i][1].min(h + rise);
            if low > high + 0.00003 {
                return Err(i + 1);
            }
            points[i].elevation = points[i].elevation.clamp(low, high.max(low));
        }
        for (i, pair) in points.windows(2).enumerate() {
            pulse();
            let a = position(pair[0]);
            let b = position(pair[1]);
            let len = distance(a, b);
            let axis = Axis {
                origin: a,
                fwd: scale(sub(b, a), 1.0 / len),
            };
            let rows = len as usize + 1;
            for j in 0..=rows {
                let t = j as f32 / rows as f32;
                let design = blend(pair[0].elevation, pair[1].elevation, t);
                for v in [-BED, 0.0, BED] {
                    let p = axis.at(len * t, v);
                    if (world.elevation(p[0], p[1]) - design).abs() > ROOM {
                        return Err(i + 1);
                    }
                }
            }
        }
        Ok(())
    }
}

fn guide_offset(guide: &CorridorGuide, axis: Axis, u: f32) -> f32 {
    let mut nearest = (f32::INFINITY, 0.0);
    for pair in guide.points[..guide.used].windows(2) {
        let a = axis.project(pair[0]);
        let b = axis.project(pair[1]);
        let low = a.0.min(b.0);
        let high = a.0.max(b.0);
        let d = (u - u.clamp(low, high)).abs();
        if d < nearest.0 {
            let t = if (b.0 - a.0).abs() > 0.0001 {
                ((u - a.0) / (b.0 - a.0)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            nearest = (d, blend(a.1, b.1, t));
        }
    }
    nearest.1
}

fn inside(region: [i32; 2], p: Xz) -> bool {
    p.into_iter().zip(region).all(|(v, r)| {
        (r as f32 * ROAD_REGION_SIZE as f32..(r + 1) as f32 * ROAD_REGION_SIZE as f32).contains(&v)
    })
}
fn foot(p: Xz, a: Xz, b: Xz) -> (f32, f32, f32) {
    let edge = sub(b, a);
    let ray = sub(p, a);
    let norm = dot(edge, edge);
    let t = (dot(ray, edge) / norm).clamp(0.0, 1.0);
    let d = sub(ray, scale(edge, t));
    let cross = d[1] * edge[0] - d[0] * edge[1];
    (t, dot(d, d), cross * cross / norm)
}
fn point(p: Xz, elevation: f32) -> RailPoint {
    RailPoint {
        x: p[0],
        z: p[1],
        elevation,
    }
}
fn position(p: RailPoint) -> Xz {
    [p.x, p.z]
}
fn sub(a: Xz, b: Xz) -> Xz {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: Xz, b: Xz) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}
fn scale(p: Xz, f: f32) -> Xz {
    [p[0] * f, p[1] * f]
}
fn norm(p: Xz) -> f32 {
    let s = dot(p, p);
    let mut x = p[0].abs().max(p[1].abs());
    if x > 0.0 {
        for _ in 0..3 {
            x = (x + s / x) * 0.5;
        }
    }
    x
}
fn distance(a: Xz, b: Xz) -> f32 {
    norm(sub(a, b))
}
fn blend(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn lerp(a: Xz, b: Xz, t: f32) -> Xz {
    [blend(a[0], b[0], t), blend(a[1], b[1], t)]
}
fn include(bounds: &mut [f32; 4], p: Xz, pad: f32) {
    bounds[0] = bounds[0].min(p[0] - pad);
    bounds[1] = bounds[1].min(p[1] - pad);
    bounds[2] = bounds[2].max(p[0] + pad);
    bounds[3] = bounds[3].max(p[1] + pad);
}

fn span_bounds(a: Xz, b: Xz, pad: f32) -> [f32; 4] {
    let mut bounds = [a[0], a[1], a[0], a[1]];
    include(&mut bounds, a, pad);
    include(&mut bounds, b, pad);
    bounds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roads::RoadNetwork;
    use crate::settlement::Terminus;
    use crate::watershed::{Watershed, WatershedWork};

    #[test]
    fn neighboring_towns_share_a_continuous_graded_railway() {
        let seed = 7911;
        let world = World::new(seed);
        let regions = [[-2, 3], [-2, 4]];
        let roads = regions.map(|[x, z]| RoadNetwork::generate(&world, x, z).unwrap());
        let towns = roads.each_ref().map(|road| {
            [Terminus::Start, Terminus::End].map(|end| Settlement::generate(&world, road, end))
        });
        let mut water = [Watershed::new(), Watershed::new()];
        let mut terrain_work = WatershedWork::new();
        for (water, [x, z]) in water.iter_mut().zip(regions) {
            water.rebuild(&world, seed, x, z, &mut terrain_work);
        }
        let wet = |x, z| {
            water
                .iter()
                .any(|plan| plan.sample(x, z).is_some_and(|s| s.is_channel()))
        };
        let road_link = HighwayLink::fit_avoiding(&world, &roads[0], &roads[1], wet).unwrap();
        let guide = CorridorGuide::from_highway(&road_link).unwrap();
        let contacts = road_link.contacts();
        let entries = [[-1506.0, 3842.0], [-1605.3381, 4348.2563]];
        let stops = core::array::from_fn(|i| {
            towns[i]
                .iter()
                .flatten()
                .find_map(|town| {
                    town.coastal_road_exits(&world, contacts[i ^ 1].point)
                        .into_iter()
                        .flatten()
                        .any(|p| [p.point.x, p.point.z] == entries[i])
                        .then(|| CorridorStop::town(regions[i], entries[i], town))
                        .flatten()
                })
                .unwrap()
        });
        let reserved = |x, z, kind| {
            wet(x, z)
                || (kind != CorridorUse::Access
                    && (guide.covers_road(x, z)
                        || roads.iter().any(|r| r.proximity(x, z).is_some())))
                || towns.iter().flatten().flatten().any(|town| {
                    town.sample_transport_reservation(x, z).is_some_and(|s| {
                        kind != CorridorUse::Access || s.surface != SettlementSurface::Street
                    })
                })
        };
        let mut work = std::boxed::Box::new(CorridorWork::new());
        let route = RailCorridor::survey_detached_cooperatively(
            &world,
            &guide,
            stops,
            &mut work,
            reserved,
            || {},
        )
        .unwrap();
        assert_eq!(
            RailCorridor::survey_detached_cooperatively(
                &world,
                &guide,
                [stops[1], stops[0]],
                &mut work,
                reserved,
                || {}
            ),
            Some(route.clone())
        );
        assert_eq!(route.regions(), regions);
        assert!((route.length() - 533.03).abs() < 0.1);
        assert_eq!(route.position_at(0.0), route.polyline()[0]);
        assert_eq!(
            route.position_at(route.length()),
            *route.polyline().last().unwrap()
        );
        for station in route.stations() {
            let [x, z] = station.stop.entry();
            let ground = world.elevation(x, z);
            let contact = route.sample(x, z, ground).unwrap();
            assert_eq!(contact.material, Some(RailMaterial::StationAccess));
            assert_eq!(contact.elevation.to_bits(), ground.to_bits());
            let [r, s] = station.walk_end;
            let platform = route.sample(r, s, world.elevation(r, s)).unwrap();
            assert!((platform.elevation - station.floor(ACCESS_AT)).abs() < 0.003);
        }
        for pair in route.polyline().windows(2) {
            let a = pair[0];
            let b = pair[1];
            let length = distance(position(a), position(b));
            let axis = Axis {
                origin: position(a),
                fwd: scale(sub(position(b), position(a)), 1.0 / length),
            };
            assert!((a.elevation - b.elevation).abs() <= MAX_GRADE * length + 0.00003);
            for j in 0..(length * 3.0) as usize {
                let t = j as f32 / (length * 3.0);
                let [x, z] = axis.at(t * length, 0.0);
                let h = world.elevation(x, z);
                let grade = blend(a.elevation, b.elevation, t);
                let sample = route.sample(x, z, h).unwrap();
                assert!((sample.elevation - grade).abs() < 0.003);
                assert!((h - grade).abs() < ROOM + 0.015);
                assert!(!reserved(x, z, CorridorUse::Trackbed));
            }
        }
        assert!(core::mem::size_of::<CorridorWork>() < 32 * 1024);
        assert!(core::mem::size_of::<RailCorridor>() < 1280);
    }
}
