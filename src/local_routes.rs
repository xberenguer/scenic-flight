//! Resident road shortcuts and gondolas attached to prepared districts.

use crate::cableways::Cableway;
use crate::facility_stream::FacilityStream;
use crate::land_stream::LandStream;
use crate::math::sin_cos;
use crate::render::{Camera, ScreenRect, screen_region, view_distance};
use crate::river_stream::RiverStream;
use crate::roads::{ROAD_REGION_SIZE, RoadNetwork};
use crate::scenery::DistrictRegion;
use crate::settlement::{Settlement, SettlementSurface, Terminus};
use crate::tunnel_art::{draw_tunnel, open_portals, overlay_approach};
use crate::tunnels::RoadTunnel;
use crate::world::{Sample, Surface, World, SEA_LEVEL};
use crate::world_mesh::{WorldMesh, bounds_in_view};

pub type RegionKey = (i32, i32);
type Area = [f32; 4];
type Point = [f32; 3];

const TILE: f32 = ROAD_REGION_SIZE as f32;
const TURN_MARGIN: f32 = 192.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reservation {
    /// Facilities and water in the candidate corridor are still being prepared.
    Pending,
    /// The prepared layout leaves this corridor free. The answer must stay
    /// stable while the district is resident, including when off screen.
    Clear,
    Blocked,
}

pub fn route_reservation<const LAND: usize, const RIVERS: usize>(
    world: &World,
    facilities: &FacilityStream<LAND>,
    rivers: &RiverStream<RIVERS>,
    key: RegionKey,
    route: &LocalRoute,
) -> Reservation {
    let (Some(facilities), Some(river)) = (facilities.planned(key), rivers.planned(key)) else {
        return Reservation::Pending;
    };
    if let Some(bounds) = facilities.bounds() {
        let dry = Sample { elevation: SEA_LEVEL + 4.0, surface: Surface::Grass };
        let sea = Sample { elevation: SEA_LEVEL, surface: Surface::Shallows };
        if route.conflicts_with(|area| area_has_sample(area, bounds, |x, z| {
            facilities.sample(x, z, dry).is_some()
                || facilities.ports().any(|port| port.sample(x, z, sea).is_some())
        })) {
            return Reservation::Blocked;
        }
    }
    if let Some(river) = river {
        if route.conflicts_with(|area| {
            river.crossings().bridges().iter().any(|bridge| areas_overlap(area, bridge.bounds()))
        }) {
            return Reservation::Blocked;
        }
        let water = |area| area_has_sample(area, river.bounds(), |x, z| {
            let ground = world.sample(x, z);
            river.overlay(x, z, ground) != ground
        });
        let blocked = match route {
            LocalRoute::Tunnel(_) => route.conflicts_with(water),
            LocalRoute::Cableway(_) => route.ground_conflicts_with(water),
        };
        if blocked {
            return Reservation::Blocked;
        }
    }
    Reservation::Clear
}

fn areas_overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

fn area_has_sample(
    a: [f32; 4],
    b: [f32; 4],
    mut occupied: impl FnMut(f32, f32) -> bool,
) -> bool {
    if !areas_overlap(a, b) {
        return false;
    }
    let min_x = a[0].max(b[0]);
    let min_z = a[1].max(b[1]);
    let width = a[2].min(b[2]) - min_x;
    let length = a[3].min(b[3]) - min_z;
    let columns = (width / 0.7) as usize + 1;
    let rows = (length / 0.7) as usize + 1;
    let dx = width / columns as f32;
    let dz = length / rows as f32;
    (0..=rows).any(|row| {
        let z = min_z + row as f32 * dz;
        (0..=columns).any(|column| occupied(min_x + column as f32 * dx, z))
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocalRoute {
    Tunnel(RoadTunnel),
    Cableway(Cableway),
}

impl LocalRoute {
    /// Minimum x, minimum z, maximum x, maximum z.
    pub fn bounds(&self) -> [f32; 4] {
        match self {
            Self::Tunnel(tunnel) => {
                let (left, near, right, far) = tunnel.bounds();
                [left, near, right, far]
            }
            Self::Cableway(line) => line.bounds(),
        }
    }

    /// Test overlapping reservations in short conservative world-space boxes.
    /// The caller should consult prepared plans, including unpublished ones.
    pub fn conflicts_with(&self, mut reserved: impl FnMut([f32; 4]) -> bool) -> bool {
        match self {
            Self::Tunnel(tunnel) => {
                let [a, b] = tunnel.junctions();
                corridor([a.x, a.z], [b.x, b.z], 4.2, &mut reserved)
            }
            Self::Cableway(line) => {
                let [a, b] = line.stations().map(|station| station.platform);
                self.ground_conflicts_with(&mut reserved)
                    || corridor(xz(a), xz(b), 4.5, &mut reserved)
            }
        }
    }

    /// Test the approaches and physical supports. An overhead cable may
    /// span water between dry supports, with its separate height proof.
    pub fn ground_conflicts_with(&self, mut reserved: impl FnMut([f32; 4]) -> bool) -> bool {
        match self {
            Self::Tunnel(tunnel) => {
                let junctions = tunnel.junctions();
                tunnel
                    .mouths()
                    .into_iter()
                    .zip(junctions)
                    .any(|(mouth, junction)| {
                        corridor(
                            [junction.x, junction.z],
                            [mouth.center.x, mouth.center.z],
                            4.2,
                            &mut reserved,
                        )
                    })
            }
            Self::Cableway(line) => {
                line.stations()
                    .iter()
                    .any(|station| reserved(point_area(station.platform, 7.0)))
                    || line
                        .pylons()
                        .iter()
                        .any(|pylon| reserved(point_area(pylon.base, 2.05)))
                    || line
                        .access_path()
                        .windows(2)
                        .any(|edge| corridor(xz(edge[0]), xz(edge[1]), 1.3, &mut reserved))
            }
        }
    }

    fn screen_bounds(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        let [left, near, right, far] = self.bounds();
        let (floor, crown) = match self {
            Self::Tunnel(tunnel) => {
                let [a, b] = tunnel.junctions();
                (
                    a.elevation.min(b.elevation) - 0.3,
                    a.elevation.max(b.elevation) + 5.5,
                )
            }
            Self::Cableway(line) => {
                let [a, b] = line.stations();
                let mut floor = a.platform[1].min(b.platform[1]);
                for point in line.access_path() {
                    floor = floor.min(point[1]);
                }
                let mut crown = a.anchor[1].max(b.anchor[1]);
                for tower in line.pylons() {
                    floor = floor.min(tower.base[1]);
                    crown = crown.max(tower.anchor[1]);
                }
                (floor - 5.0, crown + 2.0)
            }
        };
        screen_region(
            camera,
            dimensions,
            [[left, floor, near], [right, crown, far]],
        )
    }

    fn obstruction(&self, x: f32, z: f32) -> f32 {
        let Self::Cableway(line) = self else {
            return 0.0;
        };
        let [a, b] = line.stations();
        for station in [a, b] {
            if inside(point_area(station.platform, 6.7), x, z) {
                return station.anchor[1] + 1.5;
            }
        }
        for span in line.spans() {
            let dx = span.to[0] - span.from[0];
            let dz = span.to[2] - span.from[2];
            let t = ((x - span.from[0]) * dx + (z - span.from[2]) * dz) / (dx * dx + dz * dz);
            if !(0.0..=1.0).contains(&t) {
                continue;
            }
            let [cx, y, cz] = span.point(t);
            if (x - cx) * (x - cx) + (z - cz) * (z - cz) <= 4.4 * 4.4 {
                return y + 1.5;
            }
        }
        0.0
    }

    fn clears_town(&self, town: &Settlement, pulse: &mut impl FnMut()) -> bool {
        if !overlaps(self.bounds(), town.bounds()) {
            return true;
        }
        match self {
            Self::Tunnel(_) => {
                !self.conflicts_with(|area| occupied_by_town(town, area, true, pulse))
            }
            Self::Cableway(line) => {
                line.stations().iter().all(|station| {
                    !occupied_by_town(town, point_area(station.platform, 6.0), true, pulse)
                }) && line
                    .pylons()
                    .iter()
                    .all(|pylon| !occupied_by_town(town, point_area(pylon.base, 2.05), true, pulse))
                    && !line.access_path().windows(2).any(|edge| {
                        corridor(xz(edge[0]), xz(edge[1]), 1.3, &mut |area| {
                            occupied_by_town(town, area, false, pulse)
                        })
                    })
                    && line.clears_cooperatively(
                        |x, z| {
                            town.sample(x, z)
                                .and_then(|sample| sample.elevation)
                                .unwrap_or(0.0)
                        },
                        pulse,
                    )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteProgress {
    Idle,
    Surveyed,
    Reserved,
    Published,
}

#[derive(Clone, Copy)]
enum Plan {
    Tunnel,
    Cableway,
}

#[derive(Clone, Copy)]
enum State {
    Survey(Plan),
    Candidate(LocalRoute),
    Deferred(LocalRoute),
    Published(LocalRoute),
    Empty,
}

#[derive(Clone, Copy)]
struct Resident {
    key: RegionKey,
    state: State,
}

#[derive(Clone, Copy)]
struct Detached {
    key: RegionKey,
    slot: usize,
}

/// An owned survey for a worker. It cannot read or modify frame state.
pub struct RouteRequest {
    world: World,
    seed: u64,
    key: RegionKey,
    revision: u32,
    plan: Plan,
    road: RoadNetwork,
    towns: [Option<Settlement>; 2],
}

pub struct RouteResult {
    key: RegionKey,
    seed: u64,
    revision: u32,
    state: State,
}

impl RouteRequest {
    /// Run one deterministic route phase. The callback may yield to the panel
    /// or service other work without changing the candidate or its proof.
    pub fn prepare(&self, mut pulse: impl FnMut()) -> RouteResult {
        let state = SurveyInput {
            world: &self.world,
            seed: self.seed,
            key: self.key,
            road: &self.road,
            towns: self.towns.each_ref().map(Option::as_ref),
        }
        .run(self.plan, &mut pulse);
        RouteResult {
            key: self.key,
            seed: self.seed,
            revision: self.revision,
            state,
        }
    }
}

/// One connected feature per district, with a fixed resident/negative cache.
/// Terrain samples and drawing are read-only over the same published state.
pub struct LocalRouteStream<const ACTIVE: usize = 8> {
    world: World,
    seed: u64,
    residents: [Option<Resident>; ACTIVE],
    detached: Option<Detached>,
    revision: u32,
}

impl<const ACTIVE: usize> LocalRouteStream<ACTIVE> {
    pub const fn new(world: World, seed: u64) -> Self {
        Self {
            world,
            seed,
            residents: [None; ACTIVE],
            detached: None,
            revision: 0,
        }
    }

    pub fn reset(&mut self, world: World, seed: u64) {
        self.world = world;
        self.seed = seed;
        self.clear();
    }

    /// Call before showing the first frame. Planning runs synchronously over
    /// each already prepared district that fits in the route cache.
    #[inline(never)]
    pub fn preflight<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        reserve: impl FnMut(RegionKey, &LocalRoute) -> Reservation,
    ) {
        self.preflight_cooperatively(land, camera, reserve, || {});
    }

    pub fn preflight_cooperatively<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        mut reserve: impl FnMut(RegionKey, &LocalRoute) -> Reservation,
        mut pulse: impl FnMut(),
    ) {
        self.clear();
        for _ in 0..ACTIVE {
            let Some((key, district)) = self.next_district(land, camera) else {
                break;
            };
            let mut state = State::Survey(Plan::Tunnel);
            for _ in 0..2 {
                let State::Survey(plan) = state else { break };
                state = self.survey(key, district, plan, &mut pulse);
                if let State::Candidate(route) = state {
                    state = match reserve(key, &route) {
                        Reservation::Clear if land.district_published(key) => {
                            State::Published(route)
                        }
                        Reservation::Clear => State::Deferred(route),
                        Reservation::Pending => State::Candidate(route),
                        Reservation::Blocked => after_conflict(route),
                    };
                }
            }
            let slot = self
                .residents
                .iter_mut()
                .find(|slot| slot.is_none())
                .unwrap();
            *slot = Some(Resident { key, state });
        }
    }

    /// Advance the original synchronous planner by one phase.
    pub fn service<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        allow_survey: bool,
        reserve: impl FnMut(RegionKey, &LocalRoute) -> Reservation,
    ) -> RouteProgress {
        let progress = self.service_with_reservations(land, camera, allow_survey, reserve);
        if progress != RouteProgress::Idle
            || !allow_survey
            || self.detached.is_some()
            || !valid_camera(camera)
        {
            return progress;
        }
        let Some((slot, key, district, plan)) = self.next_survey(land, camera) else {
            return RouteProgress::Idle;
        };
        self.residents[slot] = Some(Resident {
            key,
            state: self.survey(key, district, plan, &mut || {}),
        });
        RouteProgress::Surveyed
    }

    /// Refresh lifetime and advance one ready reservation without generating
    /// terrain. Use this phase on the main thread with a detached worker.
    pub fn service_with_reservations<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        allow_reservations: bool,
        mut reserve: impl FnMut(RegionKey, &LocalRoute) -> Reservation,
    ) -> RouteProgress {
        if !valid_camera(camera) {
            return RouteProgress::Idle;
        }
        if self.refresh(land, camera) {
            return RouteProgress::Published;
        }
        if !allow_reservations {
            return RouteProgress::Idle;
        }
        for record in self.residents.iter_mut().flatten() {
            let State::Candidate(route) = record.state else {
                continue;
            };
            record.state = match reserve(record.key, &route) {
                Reservation::Pending => continue,
                Reservation::Blocked => after_conflict(route),
                Reservation::Clear
                    if land.district_published(record.key) && can_enter(camera, route.bounds()) =>
                {
                    State::Published(route)
                }
                Reservation::Clear => State::Deferred(route),
            };
            return if matches!(record.state, State::Published(_)) {
                RouteProgress::Published
            } else {
                RouteProgress::Reserved
            };
        }

        RouteProgress::Idle
    }

    /// Take the next survey for a worker once the previous reply is collected.
    pub fn request<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        worker_ready: bool,
    ) -> Option<RouteRequest> {
        if !valid_camera(camera) {
            return None;
        }
        self.refresh(land, camera);
        if !worker_ready || self.detached.is_some() {
            return None;
        }
        let (slot, key, district, plan) = self.next_survey(land, camera)?;
        let road = district.road()?.clone();
        self.revision = self.revision.wrapping_add(1);
        self.detached = Some(Detached { key, slot });
        Some(RouteRequest {
            world: self.world,
            seed: self.seed,
            key,
            revision: self.revision,
            plan,
            road,
            towns: district_towns(district).map(|town| town.cloned()),
        })
    }

    /// Collect a completed survey as a candidate. Reservations and publication
    /// still run separately, and an evicted district cannot acquire a route.
    pub fn commit<const LAND: usize>(
        &mut self,
        result: RouteResult,
        land: &LandStream<LAND>,
        camera: Camera,
    ) -> bool {
        if self.revision != result.revision || self.seed != result.seed {
            return false;
        }
        let Some(detached) = self.detached.filter(|job| job.key == result.key) else {
            return false;
        };
        self.detached = None;
        if valid_camera(camera) {
            self.refresh(land, camera);
        }
        if !land.districts().any(|(key, _)| key == result.key) {
            return false;
        }
        self.residents[detached.slot] = Some(Resident {
            key: result.key,
            state: result.state,
        });
        true
    }

    fn clear(&mut self) {
        self.residents.fill(None);
        self.detached = None;
        self.revision = self.revision.wrapping_add(1);
    }

    fn refresh<const LAND: usize>(&mut self, land: &LandStream<LAND>, camera: Camera) -> bool {
        let mut published = false;
        for slot in &mut self.residents {
            let Some(record) = slot else { continue };
            let held = land.districts().any(|(key, _)| key == record.key);
            if !held {
                let keep = matches!(record.state, State::Published(route)
                    if stay_resident(camera, route.bounds()));
                if !keep {
                    *slot = None;
                }
            } else if let State::Deferred(route) = record.state {
                if land.district_published(record.key) && can_enter(camera, route.bounds()) {
                    record.state = State::Published(route);
                    published = true;
                }
            }
        }
        published
    }

    fn next_survey<'a, const LAND: usize>(
        &self,
        land: &'a LandStream<LAND>,
        camera: Camera,
    ) -> Option<(usize, RegionKey, &'a DistrictRegion, Plan)> {
        let retry = self
            .residents
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                let Some(Resident {
                    key,
                    state: State::Survey(plan),
                }) = *record
                else {
                    return None;
                };
                let (_, district) = land.districts().find(|(candidate, _)| *candidate == key)?;
                Some((index, key, district, plan))
            })
            .min_by(|a, b| priority(a.1, camera).total_cmp(&priority(b.1, camera)));
        let missing = self
            .residents
            .iter()
            .position(Option::is_none)
            .and_then(|slot| {
                self.next_district(land, camera)
                    .map(|(key, district)| (slot, key, district, Plan::Tunnel))
            });
        [retry, missing]
            .into_iter()
            .flatten()
            .min_by(|a, b| priority(a.1, camera).total_cmp(&priority(b.1, camera)))
    }

    pub fn routes(&self) -> impl Iterator<Item = (RegionKey, &LocalRoute)> {
        self.residents
            .iter()
            .flatten()
            .filter_map(|record| match &record.state {
                State::Published(route) => Some((record.key, route)),
                _ => None,
            })
    }

    pub fn planned(&self, key: RegionKey) -> Option<Option<&LocalRoute>> {
        let record = self.residents.iter().flatten().find(|record| record.key == key)?;
        match &record.state {
            State::Empty => Some(None),
            State::Deferred(route) | State::Published(route) => Some(Some(route)),
            State::Survey(_) | State::Candidate(_) => None,
        }
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        self.routes()
            .find(|(_, route)| inside(route.bounds(), x, z))
            .map_or(ground, |(_, route)| match route {
                LocalRoute::Tunnel(tunnel) => overlay_approach(tunnel, x, z, ground),
                LocalRoute::Cableway(line) => access_surface(line, x, z, ground),
            })
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        self.routes()
            .filter(|(_, route)| inside(route.bounds(), x, z))
            .map(|(_, route)| route.obstruction(x, z))
            .fold(0.0, f32::max)
    }

    pub fn has_visible_routes(&self, camera: Camera) -> bool {
        self.visible(camera).next().is_some()
    }

    /// Terrain-depth coverage for this frame. Pass the same published set to
    /// `open_portals` and `draw` immediately after composing terrain.
    pub fn depth_region(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        self.visible(camera)
            .filter_map(|route| route.screen_bounds(camera, dimensions))
            .reduce(ScreenRect::union)
    }

    pub fn open_portals(
        &self,
        camera: Camera,
        pixels: &mut [u16],
        depth: &mut [u16],
        dimensions: (usize, usize),
    ) -> usize {
        self.visible(camera)
            .filter_map(|route| match route {
                LocalRoute::Tunnel(tunnel) => {
                    Some(open_portals(tunnel, camera, pixels, depth, dimensions))
                }
                LocalRoute::Cableway(_) => None,
            })
            .sum()
    }

    pub fn draw(&self, camera: Camera, ticks: u64, mesh: &mut WorldMesh<'_>) {
        for route in self.visible(camera) {
            match route {
                LocalRoute::Tunnel(tunnel) => draw_tunnel(tunnel, mesh),
                LocalRoute::Cableway(line) => line.draw(mesh, ticks),
            }
        }
    }

    fn visible(&self, camera: Camera) -> impl Iterator<Item = &LocalRoute> {
        self.routes()
            .map(|(_, route)| route)
            .filter(move |route| bounds_in_view(camera, route.bounds()))
    }

    fn next_district<'a, const LAND: usize>(
        &self,
        land: &'a LandStream<LAND>,
        camera: Camera,
    ) -> Option<(RegionKey, &'a DistrictRegion)> {
        land.districts()
            .filter(|(key, _)| {
                self.residents
                    .iter()
                    .flatten()
                    .all(|record| record.key != *key)
            })
            .min_by(|a, b| priority(a.0, camera).total_cmp(&priority(b.0, camera)))
    }

    fn survey(
        &self,
        key: RegionKey,
        district: &DistrictRegion,
        plan: Plan,
        pulse: &mut impl FnMut(),
    ) -> State {
        let Some(road) = district.road() else {
            return State::Empty;
        };
        SurveyInput {
            world: &self.world,
            seed: self.seed,
            key,
            road,
            towns: district_towns(district),
        }
        .run(plan, pulse)
    }
}

struct SurveyInput<'a> {
    world: &'a World,
    seed: u64,
    key: RegionKey,
    road: &'a RoadNetwork,
    towns: [Option<&'a Settlement>; 2],
}

impl SurveyInput<'_> {
    fn run(&self, plan: Plan, pulse: &mut impl FnMut()) -> State {
        pulse();
        let candidate = match plan {
            Plan::Tunnel => {
                RoadTunnel::generate_cooperatively(self.world, self.seed, self.road, pulse)
                    .map(LocalRoute::Tunnel)
            }
            Plan::Cableway => {
                Cableway::generate_cooperatively(self.world, self.seed, self.road, None, pulse)
                    .map(LocalRoute::Cableway)
            }
        };
        let clear = candidate.filter(|route| {
            let [left, near, right, far] = route.bounds();
            let x = self.key.0 as f32 * TILE;
            let z = self.key.1 as f32 * TILE;
            left >= x + 4.0
                && right <= x + TILE - 4.0
                && near >= z + 4.0
                && far <= z + TILE - 4.0
                && self
                    .towns
                    .into_iter()
                    .flatten()
                    .all(|town| route.clears_town(town, pulse))
        });
        match (clear, plan) {
            (Some(route), _) => State::Candidate(route),
            (None, Plan::Tunnel) => State::Survey(Plan::Cableway),
            (None, Plan::Cableway) => State::Empty,
        }
    }
}

fn district_towns(district: &DistrictRegion) -> [Option<&Settlement>; 2] {
    [Terminus::Start, Terminus::End].map(|end| district.town(end))
}

fn valid_camera(camera: Camera) -> bool {
    [camera.x, camera.z, camera.altitude, camera.heading]
        .into_iter()
        .all(f32::is_finite)
}

fn after_conflict(route: LocalRoute) -> State {
    match route {
        LocalRoute::Tunnel(_) => State::Survey(Plan::Cableway),
        LocalRoute::Cableway(_) => State::Empty,
    }
}

fn priority((x, z): RegionKey, camera: Camera) -> f32 {
    let (sin, cos) = sin_cos(camera.heading);
    let ahead = view_distance(camera.altitude) + 270.0;
    let dx = (x as f32 + 0.5) * TILE - camera.x - sin * ahead;
    let dz = (z as f32 + 0.5) * TILE - camera.z - cos * ahead;
    dx * dx + dz * dz
}

fn can_enter(camera: Camera, area: Area) -> bool {
    distance_squared(camera, area) > TURN_MARGIN * TURN_MARGIN && !bounds_in_view(camera, area)
}

fn stay_resident(camera: Camera, area: Area) -> bool {
    let reach = view_distance(camera.altitude) * 1.25 + TURN_MARGIN;
    distance_squared(camera, area) <= reach * reach || bounds_in_view(camera, area)
}

fn distance_squared(camera: Camera, [left, near, right, far]: Area) -> f32 {
    let x = camera.x - camera.x.clamp(left, right);
    let z = camera.z - camera.z.clamp(near, far);
    x * x + z * z
}

fn inside([left, near, right, far]: Area, x: f32, z: f32) -> bool {
    x >= left && z >= near && x <= right && z <= far
}

fn access_surface(line: &Cableway, x: f32, z: f32, ground: Sample) -> Sample {
    if !matches!(
        ground.surface,
        Surface::Grass
            | Surface::Forest
            | Surface::Rock
            | Surface::Sand
            | Surface::Desert
            | Surface::Snow
    ) {
        return ground;
    }
    let on_walkway = line.access_path().windows(2).any(|edge| {
        let [dx, dz] = [edge[1][0] - edge[0][0], edge[1][2] - edge[0][2]];
        let [x, z] = [x - edge[0][0], z - edge[0][2]];
        let length = dx * dx + dz * dz;
        let along = x * dx + z * dz;
        let across = x * dz - z * dx;
        along >= 0.0 && along <= length && across * across <= length * 0.82 * 0.82
    });
    if on_walkway {
        Sample {
            surface: Surface::Gravel,
            ..ground
        }
    } else {
        ground
    }
}

fn overlaps(a: Area, b: Area) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

fn occupied_by_town(
    town: &Settlement,
    area: Area,
    reserve_streets: bool,
    pulse: &mut impl FnMut(),
) -> bool {
    let bounds = town.bounds();
    if !overlaps(bounds, area) {
        return false;
    }
    let left = area[0].max(bounds[0]);
    let near = area[1].max(bounds[1]);
    let width = area[2].min(bounds[2]) - left;
    let length = area[3].min(bounds[3]) - near;
    let cols = width as usize + 1;
    let rows = length as usize + 1;
    (0..=rows).any(|row| {
        pulse();
        let z = near + length * row as f32 / rows as f32;
        (0..=cols).any(|col| {
            let x = left + width * col as f32 / cols as f32;
            town.sample(x, z).is_some_and(|sample| {
                sample.elevation.is_some()
                    || reserve_streets
                        && matches!(
                            sample.surface,
                            SettlementSurface::Street | SettlementSurface::WaterfrontPlaza
                        )
            })
        })
    })
}

fn point_area(point: Point, radius: f32) -> Area {
    [
        point[0] - radius,
        point[2] - radius,
        point[0] + radius,
        point[2] + radius,
    ]
}

fn xz(point: Point) -> [f32; 2] {
    [point[0], point[2]]
}

fn corridor(
    a: [f32; 2],
    b: [f32; 2],
    radius: f32,
    reserved: &mut impl FnMut(Area) -> bool,
) -> bool {
    let [dx, dz] = [b[0] - a[0], b[1] - a[1]];
    let steps = (dx.abs().max(dz.abs()) / 7.0) as usize + 1;
    let inverse = 1.0 / steps as f32;
    let mut previous = a;
    for step in 1..=steps {
        let t = step as f32 * inverse;
        let next = [a[0] + dx * t, a[1] + dz * t];
        if reserved([
            previous[0].min(next[0]) - radius,
            previous[1].min(next[1]) - radius,
            previous[0].max(next[0]) + radius,
            previous[1].max(next[1]) + radius,
        ]) {
            return true;
        }
        previous = next;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roads::RoadNetwork;

    fn camera(x: f32, z: f32, altitude: f32, heading: f32) -> Camera {
        Camera {
            x,
            z,
            altitude,
            heading,
            pitch: 0.2,
            bank: 0.0,
        }
    }

    #[test]
    fn a_late_route_waits_and_then_survives_a_district_boundary() {
        let world = World::new(8);
        let key = (0, 0);
        let road = RoadNetwork::generate(&world, key.0, key.1).unwrap();
        let tunnel = RoadTunnel::generate(&world, 8, &road).unwrap();
        let mouth = tunnel.mouths()[0];
        let look_in = mouth.inward_x.atan2(mouth.inward_z);
        let on_approach = |back: f32| {
            [
                mouth.center.x - back * mouth.inward_x,
                mouth.center.z - back * mouth.inward_z,
            ]
        };
        let [x, z] = on_approach(24.0);
        let approach = camera(x, z, 31.0, look_in);
        let area = LocalRoute::Tunnel(tunnel).bounds();
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(approach, true));

        let mut routes = LocalRouteStream::<8>::new(world, 8);
        routes.preflight(&land, approach, |_, _| Reservation::Pending);
        assert!(routes.depth_region(approach, (112, 112)).is_none());
        assert_eq!(
            routes.service(&land, approach, false, |_, _| panic!()),
            RouteProgress::Idle
        );
        assert_eq!(
            routes.service(&land, approach, true, |candidate, _| {
                if candidate == key {
                    Reservation::Clear
                } else {
                    Reservation::Pending
                }
            }),
            RouteProgress::Reserved
        );
        assert_eq!(routes.routes().count(), 0);

        let [x, z] = on_approach(390.0);
        let away = camera(x, z, 44.0, look_in + core::f32::consts::PI);
        assert!(can_enter(away, area));
        assert_eq!(
            routes.service(&land, away, false, |_, _| panic!()),
            RouteProgress::Published
        );
        assert_eq!(
            routes.routes().find(|(id, _)| *id == key),
            Some((key, &LocalRoute::Tunnel(tunnel)))
        );

        let [x, z] = on_approach(0.7);
        let ground = world.sample(x, z);
        let approach_sample = routes.overlay(x, z, ground);
        assert_eq!(approach_sample.surface, Surface::Pavement);

        let over_boundary = camera(
            mouth.center.x + 5.0,
            TILE + 3.0,
            50.0,
            core::f32::consts::PI,
        );
        land.reset(world);
        routes.service(&land, over_boundary, false, |_, _| panic!());
        assert_eq!(routes.overlay(x, z, ground), approach_sample);
        assert!(routes.depth_region(over_boundary, (112, 112)).is_some());

        routes.service(
            &land,
            camera(9000.0, -9000.0, 50.0, 0.0),
            false,
            |_, _| panic!(),
        );
        assert_eq!(routes.routes().count(), 0);
        assert_eq!(routes.overlay(x, z, ground), ground);
    }

    #[test]
    fn the_seeded_gondola_keeps_ground_and_cable_reservations_distinct() {
        let seed = 1;
        let world = World::new(seed);
        let camera = camera(1512.0, -1470.0, 50.0, 0.2);
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(camera, true));
        let mut routes = LocalRouteStream::<8>::new(world, seed);
        routes.preflight(&land, camera, |key, _| {
            if key == (1, -2) {
                Reservation::Clear
            } else {
                Reservation::Blocked
            }
        });
        let (key, route) = routes.routes().next().unwrap();
        let LocalRoute::Cableway(line) = route else {
            panic!()
        };
        assert_eq!(key, (1, -2));
        let [_, access, _] = *line.access_path();
        let dry = Sample {
            elevation: access[1],
            surface: Surface::Grass,
        };
        assert_eq!(
            routes.overlay(access[0], access[2], dry),
            Sample {
                surface: Surface::Gravel,
                ..dry
            }
        );
        let [x, height, z] = line.spans()[1].point(0.5);
        let between_supports = [x - 0.4, z - 0.4, x + 0.4, z + 0.4];
        assert!(route.conflicts_with(|area| overlaps(area, between_supports)));
        assert!(!route.ground_conflicts_with(|area| overlaps(area, between_supports)));
        assert!(routes.obstruction(x, z) > height);
    }

    #[test]
    fn a_worker_and_synchronous_surveys_publish_the_same_routes() {
        for (seed, start) in [
            (8, camera(267.68, 652.71, 27.77, 0.8)),
            (1, camera(1512.0, -1470.0, 50.0, 0.2)),
        ] {
            let world = World::new(seed);
            let mut land = LandStream::<8>::new();
            land.reset(world);
            assert!(land.preflight(start, true));
            let mut baseline = LocalRouteStream::<8>::new(world, seed);
            let mut cooperative = LocalRouteStream::<8>::new(world, seed);
            baseline.preflight(&land, start, |_, _| Reservation::Clear);
            let mut pulses = 0;
            cooperative.preflight_cooperatively(
                &land,
                start,
                |_, _| Reservation::Clear,
                || pulses += 1,
            );
            assert!(pulses > 20);
            for (key, route) in baseline.routes() {
                assert_eq!(
                    cooperative.routes().find(|(other, _)| *other == key),
                    Some((key, route))
                );
            }

            let away = camera(start.x - 3000.0, start.z - 3000.0, start.altitude, -2.35);
            let mut detached = LocalRouteStream::<8>::new(world, seed);
            for _ in 0..32 {
                detached.service_with_reservations(&land, away, true, |_, _| Reservation::Clear);
                let Some(request) = detached.request(&land, away, true) else {
                    break;
                };
                let result = std::thread::spawn(move || request.prepare(std::thread::yield_now))
                    .join()
                    .unwrap();
                assert!(detached.commit(result, &land, away));
            }
            assert!(baseline.routes().count() >= 1);
            assert_eq!(baseline.routes().count(), detached.routes().count());
            for (key, route) in baseline.routes() {
                assert_eq!(
                    detached.routes().find(|(other, _)| *other == key),
                    Some((key, route))
                );
            }
        }
    }

    #[test]
    fn stale_worker_replies_cannot_replace_a_current_survey() {
        let world = World::new(8);
        let camera = camera(267.68, 652.71, 27.77, 0.8);
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(camera, true));
        let mut stream = LocalRouteStream::<8>::new(world, 8);
        assert!(stream.request(&land, camera, false).is_none());
        let old = stream.request(&land, camera, true).unwrap();
        assert!(stream.request(&land, camera, true).is_none());
        stream.reset(world, 8);
        let current = stream.request(&land, camera, true).unwrap();
        assert_eq!(old.key, current.key);
        assert!(!stream.commit(old.prepare(|| {}), &land, camera));
        assert!(stream.commit(current.prepare(|| {}), &land, camera));
        assert!(!stream.commit(current.prepare(|| {}), &land, camera));
        stream.reset(world, 8);
        let pending = stream.request(&land, camera, true).unwrap();
        land.reset(world);
        assert!(!stream.commit(pending.prepare(|| {}), &land, camera));
        assert_eq!(stream.routes().count(), 0);
    }
}
