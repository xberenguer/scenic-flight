//! Terrain-following highway connections between already prepared districts.

use crate::coastal_highways::{CoastalHighway, CoastalHighwaySurvey, CoastalHighwayWork};
use crate::facility_stream::FacilityStream;
use crate::highway_links::{HighwayContact, HighwayLink, HighwaySurvey, TOWN_CAP_PAIRS, within_street_cap as within_cap};
use crate::land_stream::LandStream;
use crate::local_routes::LocalRouteStream;
use crate::math::sin_cos;
use crate::render::{CITY_HORIZON, view_distance, Camera, ScreenRect};
use crate::river_district::RiverDistrict;
use crate::river_stream::RiverStream;
use crate::roads::{RoadPoint, ROAD_HALF_WIDTH, ROAD_REGION_SIZE};
use crate::scenery::overlay_highway_link;
use crate::settlement::{Settlement, SettlementSurface, Terminus};
use crate::world::{Sample, Surface, World, SEA_LEVEL};
use crate::world_mesh::{bounds_in_range, bounds_in_view, WorldMesh};

type RegionKey = (i32, i32);
type Area = [f32; 4];
const TILE: f32 = ROAD_REGION_SIZE as f32;
const SHOULDER: f32 = ROAD_HALF_WIDTH + 1.5;
const RETAIN: f32 = 192.0;
const NEIGHBOR_REACH: f32 = 192.0;
const EMPTY_CACHE: usize = 48;
const COAST_QUEUE: usize = 16;

/// Canonical neighboring regions, regardless of flight or survey direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HighwayPair([RegionKey; 2]);

impl HighwayPair {
    pub fn new(a: (i32, i32), b: (i32, i32)) -> Option<Self> {
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        matches!(
            (b.0.checked_sub(a.0), b.1.checked_sub(a.1)),
            (Some(1), Some(0)) | (Some(0), Some(1))
        )
        .then_some(Self([a, b]))
    }

    pub fn regions(self) -> [RegionKey; 2] {
        self.0
    }

    fn survey<const LAND: usize>(self, land: &LandStream<LAND>) -> Option<HighwaySurvey> {
        let [a, b] = self.0;
        let survey = HighwaySurvey::between(land.planned(a)??.road()?, land.planned(b)??.road()?)?;
        let actual = survey
            .contacts
            .map(|contact| (contact.region[0], contact.region[1]));
        (actual == self.0 && survey.owns_termini()).then_some(survey)
    }

    fn directory(self) -> usize {
        let [(x, z), (_, other_z)] = self.0;
        (x.wrapping_mul(13)
            .wrapping_add(z.wrapping_mul(39))
            .wrapping_add(i32::from(z != other_z)))
        .rem_euclid(EMPTY_CACHE as i32) as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reservation {
    Pending,
    Clear,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HighwayProgress {
    Idle,
    Reserved,
    Published,
}

pub enum HighwayConnection {
    Ground(HighwayLink),
    Coastal(CoastalHighway),
}

impl HighwayConnection {
    pub fn contacts(&self) -> [HighwayContact; 2] {
        match self {
            Self::Ground(link) => link.contacts(),
            Self::Coastal(link) => link.contacts(),
        }
    }

    pub fn bounds(&self) -> Area {
        match self {
            Self::Ground(link) => link.bounds(),
            Self::Coastal(link) => link.bounds(),
        }
    }

    pub fn intersects_bounds(&self, bounds: Area) -> bool {
        match self {
            Self::Ground(link) => link.intersects_bounds(bounds),
            Self::Coastal(link) => link.intersects_bounds(bounds),
        }
    }

    fn chords(&self) -> impl Iterator<Item = [RoadPoint; 2]> + '_ {
        let (approaches, bridge) = match self {
            Self::Ground(link) => ([link.polyline(), &[][..]], None),
            Self::Coastal(link) => {
                let [start, _, _, end] = *link.bridge().sections();
                let deck = [start, end].map(|[x, elevation, z]| RoadPoint { x, z, elevation });
                (link.approaches(), Some(deck))
            }
        };
        approaches.into_iter().flat_map(|path| path.windows(2).map(|pair| [pair[0], pair[1]]))
            .chain(bridge)
    }

    fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        match self {
            Self::Ground(link) => overlay_highway_link(link, x, z, ground),
            Self::Coastal(link) => link.overlay(x, z, ground),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Search {
    Ground(u8),
    Town { pair: u8, detour: u8 },
    Coastal,
}

#[derive(Clone, Copy)]
pub struct HighwayRequest {
    world: World,
    seed: u64,
    serial: u32,
    pair: HighwayPair,
    survey: HighwaySurvey,
    search: Search,
}

pub struct HighwayResult {
    request: HighwayRequest,
    fit: Fit,
}

/// Only the towns at the two connecting road ends, owned by a detached job.
pub type HighwayTowns = [Option<Settlement>; 2];

enum Fit {
    Candidate { link: HighwayConnection, retry: Option<Search> },
    Retry(Search),
    SurveyCoast,
    Unavailable,
}

impl HighwayRequest {
    pub fn key(self) -> HighwayPair {
        self.pair
    }

    /// Cold terrain and shoulder proof using just the surveyed termini. The
    /// callback can service panel/sky work while the fit is in progress.
    pub fn prepare(&self, pulse: impl FnMut()) -> HighwayResult {
        self.prepare_inner(false, None, [None, None], pulse)
    }

    pub fn prepare_with_coast(&self, work: &mut CoastalHighwayWork, pulse: impl FnMut()) -> HighwayResult {
        self.prepare_inner(true, Some(work), [None, None], pulse)
    }

    pub fn prepare_with_towns(
        &self,
        work: &mut CoastalHighwayWork,
        towns: &HighwayTowns,
        pulse: impl FnMut(),
    ) -> HighwayResult {
        self.prepare_inner(true, Some(work), towns.each_ref().map(Option::as_ref), pulse)
    }

    pub fn prepare_with_land<const LAND: usize>(
        &self,
        work: &mut CoastalHighwayWork,
        land: &LandStream<LAND>,
        pulse: impl FnMut(),
    ) -> HighwayResult {
        self.prepare_inner(true, Some(work), self.nearby_towns(land), pulse)
    }

    pub fn copy_towns<const LAND: usize>(&self, land: &LandStream<LAND>) -> HighwayTowns {
        self.nearby_towns(land).map(|town| town.cloned())
    }

    /// Copy worker inputs into the shared staging slot one town at a time.
    pub fn stage_towns<const LAND: usize>(&self, land: &LandStream<LAND>, towns: &mut HighwayTowns) {
        for (slot, town) in towns.iter_mut().zip(self.nearby_towns(land)) {
            *slot = town.cloned();
        }
    }

    fn nearby_towns<'a, const LAND: usize>(&self, land: &'a LandStream<LAND>) -> [Option<&'a Settlement>; 2] {
        if matches!(self.search, Search::Ground(_)) {
            return [None, None];
        }
        self.survey.contacts.map(|contact| {
            let [x, z] = contact.region;
            land.planned((x, z))??.town(contact_terminus(contact))
        })
    }

    fn prepare_inner(
        &self,
        allow_coast: bool,
        work: Option<&mut CoastalHighwayWork>,
        towns: [Option<&Settlement>; 2],
        mut pulse: impl FnMut(),
    ) -> HighwayResult {
        let fit = match self.search {
            Search::Ground(next) => {
                if let Some((link, next)) = self.survey.fit(&self.world, next, true, |_, _| false, &mut pulse) {
                    Fit::Candidate { link: HighwayConnection::Ground(link), retry: Some(Search::Ground(next)) }
                } else if allow_coast {
                    Fit::Retry(Search::Town { pair: 0, detour: 0 })
                } else {
                    Fit::Unavailable
                }
            }
            Search::Town { pair, detour } => {
                let plan = self.survey.town_cap_plan(&self.world, towns);
                if let Some((index, caps)) = (pair..TOWN_CAP_PAIRS)
                    .find_map(|index| plan.pair(index).map(|caps| (index, caps)))
                {
                    let start = if index == pair { detour } else { 0 };
                    if let Some((link, next)) = self.survey.fit_town_cap_pair(
                        &self.world, towns, caps, start, |_, _| false, &mut pulse,
                    ) {
                        Fit::Candidate {
                            link: HighwayConnection::Ground(link),
                            retry: Some(Search::Town { pair: index, detour: next }),
                        }
                    } else {
                        Fit::Retry(Search::Town { pair: index + 1, detour: 0 })
                    }
                } else if allow_coast {
                    Fit::SurveyCoast
                } else {
                    Fit::Unavailable
                }
            }
            Search::Coastal => match (work, CoastalHighwaySurvey::from_highway(self.survey)) {
                (Some(work), Some(survey)) => {
                    let ends = self.survey.contacts.map(|contact| contact.point);
                    let exits = core::array::from_fn::<_, 2, _>(|i| towns[i].map_or([None; 2], |town| {
                        town.coastal_road_exits(&self.world, ends[i ^ 1])
                    }));
                    let fit = exits[0].into_iter().enumerate().find_map(|(i, west)| {
                        if i > 0 && west.is_none() { return None; }
                        exits[1].into_iter().enumerate().find_map(|(j, east)| {
                            if j > 0 && east.is_none() { return None; }
                            let caps = [west, east];
                            let blocked = |x, z| {
                                towns.into_iter().zip(caps).any(|(town, cap)| {
                                    town.is_some_and(|town| town.sample_transport_reservation(x, z).is_some_and(|surface| {
                                        surface.surface != SettlementSurface::Street
                                            || !cap.is_some_and(|exit| within_cap(exit.point, x, z))
                                    }))
                                })
                            };
                            survey.prepare_with_caps(&self.world, work, caps, blocked, &mut pulse)
                        })
                    });
                    match fit {
                        Some(link) => Fit::Candidate { link: HighwayConnection::Coastal(link), retry: None },
                        None => Fit::Unavailable,
                    }
                }
                _ => Fit::Unavailable,
            },
        };
        HighwayResult { request: *self, fit }
    }
}

enum State {
    Candidate { link: HighwayConnection, retry: Option<Search> },
    Retry(Search),
    Deferred(HighwayConnection),
    Published(HighwayConnection),
}

struct Resident {
    pair: HighwayPair,
    state: State,
}

/// Small resident cache, negative cache, and at most one owned worker job.
pub struct HighwayStream<const ACTIVE: usize = 4> {
    world: World,
    seed: u64,
    serial: u32,
    pending: Option<HighwayRequest>,
    rejected: [Option<HighwayPair>; EMPTY_CACHE],
    awaiting_coast: [Option<HighwayPair>; COAST_QUEUE],
    residents: [Option<Resident>; ACTIVE],
}

pub struct HighwaySampler<'a, const ACTIVE: usize> {
    links: [Option<(Area, &'a HighwayConnection)>; ACTIVE],
    count: usize,
}

impl<const ACTIVE: usize> HighwaySampler<'_, ACTIVE> {
    /// Apply directly after the natural height sample, before forest details.
    /// The grade is already verified, so the link only claims its material.
    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        if !can_cover(ground) {
            return ground;
        }
        self.links[..self.count]
            .iter()
            .flatten()
            .find_map(|&([left, near, right, far], link)| {
                if !(left <= x && x <= right && near <= z && z <= far) {
                    return None;
                }
                let covered = link.overlay(x, z, ground);
                (covered != ground).then_some(covered)
            })
            .unwrap_or(ground)
    }
}

impl<const ACTIVE: usize> HighwayStream<ACTIVE> {
    pub const fn new(world: World, seed: u64) -> Self {
        Self {
            world,
            seed,
            serial: 0,
            pending: None,
            rejected: [None; EMPTY_CACHE],
            awaiting_coast: [None; COAST_QUEUE],
            residents: [const { None }; ACTIVE],
        }
    }

    pub fn reset(&mut self, world: World, seed: u64) {
        self.world = world;
        self.seed = seed;
        self.serial = self.serial.wrapping_add(1);
        self.pending = None;
        self.rejected.fill(None);
        self.awaiting_coast.fill(None);
        for slot in &mut self.residents {
            *slot = None;
        }
    }

    /// Optional startup preparation before rendering the first flight frame.
    /// Pass stable reservations; unresolved layouts remain unpublished.
    pub fn preflight<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        reserve: impl FnMut(HighwayPair, &HighwayConnection) -> Reservation,
        mut pulse: impl FnMut(),
    ) {
        self.preflight_with(land, camera, reserve, false, |request| request.prepare(&mut pulse));
    }

    /// Keep slow marine surveys on the background worker on ordinary launches.
    #[inline(never)]
    pub fn preflight_defer_coast<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        reserve: impl FnMut(HighwayPair, &HighwayConnection) -> Reservation,
        mut pulse: impl FnMut(),
    ) {
        self.preflight_with(land, camera, reserve, false, |request| {
            request.prepare_inner(true, None, [None, None], &mut pulse)
        });
    }

    pub fn preflight_with_coast<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        work: &mut CoastalHighwayWork,
        reserve: impl FnMut(HighwayPair, &HighwayConnection) -> Reservation,
        mut pulse: impl FnMut(),
    ) {
        self.preflight_with(land, camera, reserve, true, |request| request.prepare_with_land(work, land, &mut pulse));
    }

    fn preflight_with<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        mut reserve: impl FnMut(HighwayPair, &HighwayConnection) -> Reservation,
        include_coast: bool,
        mut prepare: impl FnMut(HighwayRequest) -> HighwayResult,
    ) {
        self.reset(self.world, self.seed);
        // At most two canonical positive-facing seams for every ready district.
        // Each subsequent pass either consumes a seam or advances its detour list.
        for _ in 0..LAND.saturating_mul(2 * (22 + usize::from(TOWN_CAP_PAIRS))) {
            let Some(request) = self.request_filtered(land, camera, true, include_coast) else {
                break;
            };
            let result = prepare(request);
            self.commit(result, land, camera);
            self.check_reservations(land, camera, &mut reserve, true);
        }
    }

    pub fn request<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        worker_ready: bool,
    ) -> Option<HighwayRequest> {
        self.request_filtered(land, camera, worker_ready, true)
    }

    fn request_filtered<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        worker_ready: bool,
        include_coast: bool,
    ) -> Option<HighwayRequest> {
        if !valid_camera(camera) {
            return None;
        }
        self.refresh(land, camera);
        if !worker_ready || self.pending.is_some() {
            return None;
        }
        let retry = self.residents.iter().flatten().filter_map(|record| {
            let State::Retry(search) = record.state else {
                return None;
            };
            if !include_coast && !matches!(search, Search::Ground(_)) {
                return None;
            }
            let survey = record.pair.survey(land)?;
            Some((record.pair, survey, search))
        });
        let room = self.residents.iter().any(Option::is_none);
        let new = neighboring_pairs(land).filter_map(|pair| {
            if !room
                || self.rejected[pair.directory()] == Some(pair)
                || self
                    .residents
                    .iter()
                    .flatten()
                    .any(|record| record.pair == pair)
            {
                return None;
            }
            let search = if self.awaiting_coast.contains(&Some(pair)) {
                if !include_coast {
                    return None;
                }
                Search::Coastal
            } else {
                Search::Ground(0)
            };
            Some((pair, pair.survey(land)?, search))
        });
        let (pair, survey, search) = retry
            .chain(new)
            .min_by(|a, b| priority(a.1, camera).total_cmp(&priority(b.1, camera)))?;
        self.serial = self.serial.wrapping_add(1);
        let request = HighwayRequest {
            world: self.world,
            seed: self.seed,
            serial: self.serial,
            pair,
            survey,
            search,
        };
        self.pending = Some(request);
        Some(request)
    }

    /// Store a completed fit without letting it alter the current frame.
    pub fn commit<const LAND: usize>(
        &mut self,
        result: HighwayResult,
        land: &LandStream<LAND>,
        camera: Camera,
    ) -> bool {
        let request = result.request;
        let Some(pending) = self.pending else {
            return false;
        };
        if pending.serial != request.serial
            || pending.seed != request.seed
            || pending.pair != request.pair
        {
            return false;
        }
        self.pending = None;
        if valid_camera(camera) {
            self.refresh(land, camera);
        }
        let Some(actual) = request.pair.survey(land) else {
            return false;
        };
        if actual.contacts != request.survey.contacts {
            return false;
        }
        let slot = self.residents.iter().position(|slot| {
            slot.as_ref()
                .is_some_and(|value| value.pair == request.pair)
        });
        let directory = request.pair.directory();
        if request.search == Search::Coastal {
            if let Some(queued) = self.awaiting_coast.iter_mut().find(|entry| **entry == Some(request.pair)) {
                *queued = None;
            }
        }
        let state = match result.fit {
            Fit::Candidate { link, retry } => State::Candidate { link, retry },
            Fit::Retry(search) => State::Retry(search),
            Fit::SurveyCoast => {
                if let Some(slot) = slot {
                    self.residents[slot] = None;
                }
                if let Some(queued) = self.awaiting_coast.iter_mut().find(|entry| entry.is_none()) {
                    *queued = Some(request.pair);
                } else {
                    self.rejected[directory] = Some(request.pair);
                }
                return true;
            }
            Fit::Unavailable => {
                if let Some(slot) = slot {
                    self.residents[slot] = None;
                }
                self.rejected[directory] = Some(request.pair);
                return true;
            }
        };
        let Some(slot) = slot.or_else(|| self.residents.iter().position(Option::is_none)) else {
            return false;
        };
        self.residents[slot] = Some(Resident {
            pair: request.pair,
            state,
        });
        true
    }

    /// No geometry generation. Apply at most one ready reservation decision.
    pub fn service_with_reservations<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        allow_reservations: bool,
        mut reserve: impl FnMut(HighwayPair, &HighwayConnection) -> Reservation,
    ) -> HighwayProgress {
        if !valid_camera(camera) {
            return HighwayProgress::Idle;
        }
        if self.refresh(land, camera) {
            return HighwayProgress::Published;
        }
        if allow_reservations {
            self.check_reservations(land, camera, &mut reserve, false)
        } else {
            HighwayProgress::Idle
        }
    }

    fn check_reservations<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        reserve: &mut impl FnMut(HighwayPair, &HighwayConnection) -> Reservation,
        initial: bool,
    ) -> HighwayProgress {
        for slot in &mut self.residents {
            let Some(resident) = slot else { continue; };
            let State::Candidate { link, retry } = &resident.state else {
                continue;
            };
            let check = reserve(resident.pair, link);
            if check == Reservation::Pending {
                continue;
            }
            let retry = *retry;
            let State::Candidate { link, .. } =
                core::mem::replace(&mut resident.state, State::Retry(Search::Ground(0)))
            else {
                unreachable!()
            };
            if check == Reservation::Blocked {
                if let Some(search) = retry {
                    resident.state = State::Retry(search);
                } else {
                    self.rejected[resident.pair.directory()] = Some(resident.pair);
                    *slot = None;
                }
                return HighwayProgress::Reserved;
            }
            let publish = published_owners(land, resident.pair)
                && (initial || outside(camera, link.bounds(), RETAIN));
            resident.state = if publish {
                State::Published(link)
            } else {
                State::Deferred(link)
            };
            return if publish {
                HighwayProgress::Published
            } else {
                HighwayProgress::Reserved
            };
        }
        HighwayProgress::Idle
    }

    fn refresh<const LAND: usize>(&mut self, land: &LandStream<LAND>, camera: Camera) -> bool {
        let mut changed = false;
        let mut retained_far_crossing = false;
        for queued in &mut self.awaiting_coast {
            if queued.is_some_and(|pair| pair.0.into_iter().any(|key| land.planned(key).flatten().is_none())) {
                *queued = None;
            }
        }
        for slot in &mut self.residents {
            let Some(record) = slot else {
                continue;
            };
            let held = record
                .pair
                .0
                .into_iter()
                .all(|key| land.planned(key).flatten().is_some());
            if !held {
                let retain = matches!(&record.state, State::Published(link)
                    if !outside(camera, link.bounds(), view_distance(camera.altitude) * 1.25 + RETAIN));
                // A distant bridge can outlast its town tiles. Keep one such
                // link in view and leave the other slots for current surveys.
                let retain_far = !retain && !retained_far_crossing && matches!(
                    &record.state, State::Published(HighwayConnection::Coastal(link))
                        if crate::bridge_silhouette::has_extended_range(link.bridge())
                            && bounds_in_range(camera, link.bridge().bounds(), CITY_HORIZON)
                );
                retained_far_crossing |= retain_far;
                if !retain && !retain_far {
                    *slot = None;
                }
                continue;
            }
            if let State::Deferred(link) = &record.state {
                if published_owners(land, record.pair) && outside(camera, link.bounds(), RETAIN) {
                    let State::Deferred(link) =
                        core::mem::replace(&mut record.state, State::Retry(Search::Ground(0)))
                    else {
                        unreachable!()
                    };
                    record.state = State::Published(link);
                    changed = true;
                }
            }
        }
        changed
    }

    fn connections(&self) -> impl Iterator<Item = (HighwayPair, &HighwayConnection)> {
        self.residents
            .iter()
            .flatten()
            .filter_map(|resident| match &resident.state {
                State::Published(link) => Some((resident.pair, link)),
                _ => None,
            })
    }

    pub fn links(&self) -> impl Iterator<Item = (HighwayPair, &HighwayLink)> {
        self.connections().filter_map(|(pair, connection)| match connection {
            HighwayConnection::Ground(link) => Some((pair, link)),
            HighwayConnection::Coastal(_) => None,
        })
    }

    pub fn coastal_links(&self) -> impl Iterator<Item = (HighwayPair, &CoastalHighway)> {
        self.connections().filter_map(|(pair, connection)| match connection {
            HighwayConnection::Ground(_) => None,
            HighwayConnection::Coastal(link) => Some((pair, link)),
        })
    }

    pub fn crossing_region(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        self.coastal_links().filter_map(|(_, link)| link.depth_region(camera, dimensions))
            .reduce(ScreenRect::union)
    }

    pub fn draw_crossings(&self, camera: Camera, dimensions: (usize, usize), mesh: &mut WorldMesh<'_>) {
        self.draw_crossings_fading(camera, dimensions, mesh, |_| 1.0);
    }

    pub fn draw_crossings_fading(&self, camera: Camera, dimensions: (usize, usize), mesh: &mut WorldMesh<'_>,
        opacity: impl Fn(&crate::crossings::Bridge) -> f32,
    ) {
        for (_, link) in self.coastal_links() {
            if link.depth_region(camera, dimensions).is_some() {
                let bridge = link.bridge();
                bridge.draw_fading(mesh, opacity(bridge));
            }
        }
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        self.coastal_links().map(|(_, link)| link.obstruction(x, z)).fold(0.0, f32::max)
    }

    /// Evaluated corridors reserve their own footprint before first display.
    pub fn planned(&self, key: HighwayPair) -> Option<Option<&HighwayConnection>> {
        if let Some(resident) = self
            .residents
            .iter()
            .flatten()
            .find(|record| record.pair == key)
        {
            match &resident.state {
                State::Deferred(link) | State::Published(link) => Some(Some(link)),
                State::Candidate { .. } | State::Retry(_) => None,
            }
        } else {
            (self.rejected[key.directory()] == Some(key)).then_some(None)
        }
    }

    pub fn sampler(&self) -> HighwaySampler<'_, ACTIVE> {
        let mut links = [None; ACTIVE];
        let mut count = 0;
        for (slot, (_, link)) in links.iter_mut().zip(self.connections()) {
            *slot = Some((link.bounds(), link));
            count += 1;
        }
        HighwaySampler { links, count }
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        self.sampler().overlay(x, z, ground)
    }

    pub fn has_visible_links(&self, camera: Camera) -> bool {
        self.connections()
            .any(|(_, link)| bounds_in_view(camera, link.bounds()))
    }

    /// Accepted links reserve the full shoulder, even before publication.
    pub fn reserves(&self, bounds: [f32; 4]) -> bool {
        self.residents
            .iter()
            .flatten()
            .any(|record| match &record.state {
                State::Published(link) | State::Deferred(link) => link.intersects_bounds(bounds),
                _ => false,
            })
    }

    pub fn reserves_river(&self, river: &RiverDistrict) -> bool {
        self.residents.iter().flatten().any(|resident| match &resident.state {
            State::Deferred(link) | State::Published(link) => river_blocks_link(river, link),
            _ => false,
        })
    }

    /// Query current and cold-prepared highway plans before placing a later
    /// feature. Unsettled pairs that could reach this bound return Pending.
    pub fn reservation<const LAND: usize>(
        &self,
        land: &LandStream<LAND>,
        area: [f32; 4],
    ) -> Reservation {
        if self.reserves(area) {
            return Reservation::Blocked;
        }
        let mut pending = false;
        for pair in neighboring_pairs(land) {
            if self.planned(pair).is_some() {
                continue;
            }
            if let Some(survey) = pair.survey(land) {
                let [a, b] = survey.contacts.map(|contact| contact.point);
                let reach = 277.0;
                pending |= overlaps(
                    area,
                    [
                        a.x.min(b.x) - reach,
                        a.z.min(b.z) - reach,
                        a.x.max(b.x) + reach,
                        a.z.max(b.z) + reach,
                    ],
                );
            }
        }
        if pending {
            Reservation::Pending
        } else {
            Reservation::Clear
        }
    }
}

/// Protect authoritative plans even while they are waiting to publish. Only
/// the two actual regional road caps may share ground with the extension.
pub fn highway_reservation<const LAND: usize, const RIVERS: usize, const ROUTES: usize>(
    land: &LandStream<LAND>,
    facilities: &FacilityStream<LAND>,
    rivers: &RiverStream<RIVERS>,
    routes: &LocalRouteStream<ROUTES>,
    pair: HighwayPair,
    link: &HighwayConnection,
) -> Reservation {
    let [left, near, right, far] = link.bounds();
    let mut pending = false;
    for z in region_at(near - NEIGHBOR_REACH)..=region_at(far + NEIGHBOR_REACH) {
        for x in region_at(left - NEIGHBOR_REACH)..=region_at(right + NEIGHBOR_REACH) {
            let key = (x, z);
            let Some(district) = land.planned(key) else {
                pending = true;
                continue;
            };
            let river = rivers.planned(key);
            if river.flatten().is_some_and(|river| river_blocks_link(river, link)) {
                return Reservation::Blocked;
            }
            let Some(district) = district else {
                continue;
            };
            let contact = link
                .contacts()
                .into_iter()
                .find(|c| (c.region[0], c.region[1]) == key);
            if let Some(road) = district.road() {
                if occupied(link, road.bounds(), |x, z| {
                    let shared_cap = contact.is_some_and(|join| within_cap(join.point, x, z));
                    !shared_cap && road.proximity(x, z).is_some()
                }) {
                    return Reservation::Blocked;
                }
            }
            let caps = match link {
                HighwayConnection::Ground(ground) => ground.street_caps(),
                HighwayConnection::Coastal(crossing) => crossing.street_caps(),
            };
            let shared_street = link.contacts().into_iter().zip(caps)
                .find(|(contact, _)| (contact.region[0], contact.region[1]) == key)
                .and_then(|(contact, cap)| Some((district.town(contact_terminus(contact))?, cap?)));
            let overlaps_town = district.towns().any(|town| {
                occupied(link, town.bounds(), |x, z| {
                    let Some(sample) = town.sample_transport_reservation(x, z) else { return false; };
                    let exact_junction = sample.surface == SettlementSurface::Street
                        && shared_street.is_some_and(|(owner, cap)| {
                            core::ptr::eq(owner, town) && within_cap(cap, x, z)
                        });
                    !exact_junction
                })
            });
            if overlaps_town {
                return Reservation::Blocked;
            }
            if let Some(facilities) = facilities.planned(key) {
                const DRY: Sample = Sample {
                    elevation: SEA_LEVEL + 4.0,
                    surface: Surface::Grass,
                };
                const WATER: Sample = Sample { elevation: SEA_LEVEL, surface: Surface::Shallows };
                if facilities.bounds().is_some_and(|area| {
                    occupied(link, area, |x, z| facilities.sample(x, z, DRY).is_some()
                        || (matches!(link, HighwayConnection::Coastal(_))
                            && facilities.ports().any(|port| port.sample(x, z, WATER).is_some())))
                }) {
                    return Reservation::Blocked;
                }
            } else {
                pending = true;
            }
            if river.is_none() {
                pending = true;
            }
            if pair.0.contains(&key) {
                match routes.planned(key) {
                    Some(Some(route))
                        if route.ground_conflicts_with(|area| link.intersects_bounds(area)) =>
                    {
                        return Reservation::Blocked
                    }
                    Some(_) => {}
                    None => pending = true,
                }
            }
        }
    }
    if pending {
        Reservation::Pending
    } else {
        Reservation::Clear
    }
}

fn river_blocks_link(river: &RiverDistrict, link: &HighwayConnection) -> bool {
    if !river.intersects_scenery(link.bounds()) {
        return false;
    }
    river.crossings().bridges().iter().any(|bridge| link.intersects_bounds(bridge.bounds()))
        || river.floodplain().sites().any(|(_, bounds)| link.intersects_bounds(bounds))
        || occupied(link, river.bounds(), |x, z| {
            river.watershed().sample(x, z).is_some_and(|water| {
                water.is_channel() || water.bank_strength >= 0.63
            })
        })
}

fn neighboring_pairs<const LAND: usize>(
    land: &LandStream<LAND>,
) -> impl Iterator<Item = HighwayPair> + '_ {
    land.districts().flat_map(|((x, z), _)| {
        [
            x.checked_add(1)
                .map(|east| HighwayPair([(x, z), (east, z)])),
            z.checked_add(1)
                .map(|north| HighwayPair([(x, z), (x, north)])),
        ]
        .into_iter()
        .flatten()
    })
}

fn published_owners<const LAND: usize>(land: &LandStream<LAND>, pair: HighwayPair) -> bool {
    pair.0.into_iter().all(|key| land.district_published(key))
}

fn valid_camera(camera: Camera) -> bool {
    [camera.x, camera.z, camera.altitude, camera.heading]
        .into_iter()
        .all(f32::is_finite)
}

fn priority(survey: HighwaySurvey, camera: Camera) -> f32 {
    let [a, b] = survey.contacts.map(|contact| contact.point);
    let (sin, cos) = sin_cos(camera.heading);
    let ahead = view_distance(camera.altitude) * 1.3;
    let x = (a.x + b.x) * 0.5 - camera.x - sin * ahead;
    let z = (a.z + b.z) * 0.5 - camera.z - cos * ahead;
    x * x + z * z
}

fn outside(camera: Camera, area: Area, radius: f32) -> bool {
    let dx = camera.x - camera.x.clamp(area[0], area[2]);
    let dz = camera.z - camera.z.clamp(area[1], area[3]);
    dx * dx + dz * dz > radius * radius && !bounds_in_view(camera, area)
}

fn can_cover(sample: Sample) -> bool {
    sample.elevation > SEA_LEVEL
        && matches!(
            sample.surface,
            Surface::Sand
                | Surface::Grass
                | Surface::Forest
                | Surface::Rock
                | Surface::Snow
                | Surface::Desert
                | Surface::Gravel
        )
}

fn overlaps(a: Area, b: Area) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

fn contact_terminus(contact: HighwayContact) -> Terminus {
    if contact.final_endpoint { Terminus::End } else { Terminus::Start }
}

fn region_at(value: f32) -> i32 {
    let scaled = value / TILE;
    let truncated = scaled as i32;
    truncated - i32::from(scaled < truncated as f32)
}

fn occupied(link: &HighwayConnection, area: Area, mut intersects: impl FnMut(f32, f32) -> bool) -> bool {
    if !link.intersects_bounds(area) {
        return false;
    }
    for [a, b] in link.chords() {
        let ax = b.x - a.x;
        let az = b.z - a.z;
        let inverse_length = 1.0 / (ax * ax + az * az);
        let segment = [
            a.x.min(b.x) - SHOULDER,
            a.z.min(b.z) - SHOULDER,
            a.x.max(b.x) + SHOULDER,
            a.z.max(b.z) + SHOULDER,
        ];
        if !overlaps(area, segment) {
            continue;
        }
        let left = area[0].max(segment[0]);
        let near = area[1].max(segment[1]);
        let width = area[2].min(segment[2]) - left;
        let length = area[3].min(segment[3]) - near;
        let columns = (width / 0.70) as usize + 1;
        let rows = (length / 0.70) as usize + 1;
        let dx = width / columns as f32;
        let dz = length / rows as f32;
        for row in 0..=rows {
            for col in 0..=columns {
                let x = left + col as f32 * dx;
                let z = near + row as f32 * dz;
                let px = x - a.x;
                let pz = z - a.z;
                let t = ((px * ax + pz * az) * inverse_length).clamp(0.0, 1.0);
                let sx = px - ax * t;
                let sz = pz - az * t;
                if sx * sx + sz * sz <= SHOULDER * SHOULDER && intersects(x, z) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_routes::route_reservation;
    use crate::roads::RoadNetwork;
    use crate::watershed::WatershedWork;

    const SEED: u64 = 31;
    const WEST: RegionKey = (-5, -17);
    const EAST: RegionKey = (-4, -17);

    fn camera(x: f32, z: f32, heading: f32) -> Camera {
        Camera {
            x,
            z,
            altitude: 140.0,
            heading,
            pitch: 0.26,
            bank: -0.17,
        }
    }

    fn scene() -> (World, LandStream<8>, HighwayPair, Camera) {
        let world = World::new(SEED);
        let pair = HighwayPair::new(WEST, EAST).unwrap();
        let look = camera(-4_146.0, -17_035.0, core::f32::consts::FRAC_PI_2);
        let mut land = LandStream::<8>::new();
        land.reset(world);
        land.preflight(look, true);
        assert!(published_owners(&land, pair));
        (world, land, pair, look)
    }

    #[test]
    fn accepted_highway_reserves_its_shoulder_before_revealing_and_crosses_the_real_seam() {
        let (world, mut land, pair, look) = scene();
        let mut stream = HighwayStream::<4>::new(world, SEED);
        let request = stream.request(&land, look, true).unwrap();
        assert_eq!(request.key(), pair);
        let result = std::thread::spawn(move || request.prepare(std::thread::yield_now))
            .join()
            .unwrap();
        assert!(stream.commit(result, &land, look));
        assert_eq!(stream.links().count(), 0);
        assert_eq!(
            stream.service_with_reservations(&land, look, true, |_, _| Reservation::Pending),
            HighwayProgress::Idle
        );
        let mut middle = None;
        assert_eq!(
            stream.service_with_reservations(&land, look, true, |key, link| {
                assert_eq!(key, pair);
                let [a, b] = link.contacts();
                for contact in [a, b] {
                    let [x, z] = contact.region;
                    let road = land.planned((x, z)).unwrap().unwrap().road().unwrap();
                    let points = road.polyline();
                    let endpoint = if contact.final_endpoint {
                        points[points.len() - 1]
                    } else {
                        points[0]
                    };
                    assert_eq!(endpoint, contact.point);
                }
                let seam = EAST.0 as f32 * TILE;
                let HighwayConnection::Ground(link) = link else { panic!("expected a dry connector"); };
                let edge = link
                    .polyline()
                    .windows(2)
                    .find(|edge| edge[0].x <= seam && edge[1].x >= seam)
                    .unwrap();
                let t = (seam - edge[0].x) / (edge[1].x - edge[0].x);
                let z = edge[0].z + (edge[1].z - edge[0].z) * t;
                middle = Some((seam, z));
                Reservation::Clear
            }),
            HighwayProgress::Reserved
        );
        let (x, z) = middle.unwrap();
        let area = [x - 2.5, z - 2.5, x + 2.5, z + 2.5];
        assert!(stream.reserves(area));
        assert_eq!(stream.reservation(&land, area), Reservation::Blocked);
        let base = world.sample(x, z);
        assert_eq!(stream.overlay(x, z, base), base);

        let away = camera(x - 1_300.0, z, -core::f32::consts::FRAC_PI_2);
        assert_eq!(
            stream.service_with_reservations(&land, away, false, |_, _| panic!()),
            HighwayProgress::Published
        );
        for dx in [-1.0, 0.0, 1.0] {
            let raw = world.sample(x + dx, z);
            let paved = stream.sampler().overlay(x + dx, z, raw);
            assert_eq!(
                paved,
                Sample {
                    surface: Surface::Pavement,
                    ..raw
                }
            );
            let protected = Sample {
                surface: Surface::RiverWater,
                ..raw
            };
            assert_eq!(stream.overlay(x + dx, z, protected), protected);
        }
        land.reset(world);
        stream.service_with_reservations(&land, look, false, |_, _| panic!());
        assert!(stream.has_visible_links(look));
        let distant = camera(x + 4_000.0, z, core::f32::consts::FRAC_PI_2);
        stream.service_with_reservations(&land, distant, false, |_, _| panic!());
        assert!(!stream.reserves(area));
        assert_eq!(stream.overlay(x, z, base), base);
    }

    #[test]
    fn rejected_detours_advance_and_stale_worker_replies_cannot_replace_the_pair() {
        let (world, mut land, pair, look) = scene();
        let mut stream = HighwayStream::<4>::new(world, SEED);
        let old = stream.request(&land, look, true).unwrap();
        assert!(stream.request(&land, look, true).is_none());
        stream.reset(world, SEED);
        let current = stream.request(&land, look, true).unwrap();
        assert!(!stream.commit(old.prepare(|| {}), &land, look));
        assert!(stream.commit(current.prepare(|| {}), &land, look));
        assert!(!stream.commit(current.prepare(|| {}), &land, look));
        let mut previous = 0;
        let mut offered = 0;
        loop {
            stream.service_with_reservations(&land, look, true, |key, _| {
                assert_eq!(key, pair);
                offered += 1;
                Reservation::Blocked
            });
            let Some(request) = stream.request(&land, look, true) else {
                break;
            };
            let Search::Ground(next) = request.search else { panic!("unexpected coastal survey"); };
            assert!(next > previous && next <= 19);
            previous = next;
            assert!(stream.commit(request.prepare(|| {}), &land, look));
        }
        assert!(offered >= 2 && offered <= 19);
        assert!(matches!(stream.planned(pair), Some(None)));
        assert_eq!(stream.links().count(), 0);

        stream.reset(world, SEED);
        let outdated = stream.request(&land, look, true).unwrap();
        land.reset(world);
        assert!(!stream.commit(outdated.prepare(|| {}), &land, look));
    }

    #[test]
    fn coastal_fallback_stays_off_the_launch_thread_and_respects_reservations() {
        let seed = 67;
        let world = World::new(seed);
        let pair = HighwayPair::new((-2, 7), (-1, 7)).unwrap();
        let look = Camera {
            altitude: 335.0,
            pitch: 0.53,
            ..camera(-1_301.313_4, 7_609.695_3, 1.709_436)
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        land.preflight(look, true);
        assert!(published_owners(&land, pair));
        let mut stream = HighwayStream::<4>::new(world, seed);
        stream.preflight_defer_coast(&land, look, |_, _| Reservation::Blocked, || {});
        assert!(stream.coastal_links().next().is_none());

        let mut work = CoastalHighwayWork::new();
        let mut encountered = false;
        while let Some(request) = stream.request(&land, look, true) {
            if request.key() == pair {
                assert!(matches!(request.search, Search::Town { .. } | Search::Coastal));
                encountered |= request.search == Search::Coastal;
            }
            assert!(stream.commit(request.prepare_with_coast(&mut work, || {}), &land, look));
            stream.service_with_reservations(&land, look, true, |key, link| {
                if key == pair {
                    let HighwayConnection::Coastal(crossing) = link else {
                        panic!("expected a coastal crossing");
                    };
                    assert_eq!(crossing.approaches()[0].first(), Some(&link.contacts()[0].point));
                }
                Reservation::Blocked
            });
        }
        assert!(encountered);
        assert!(matches!(stream.planned(pair), Some(None)));
        assert_eq!(stream.obstruction(-943.12, 7_570.83), 0.0);
    }

    #[test]
    fn two_town_streets_join_through_the_authoritative_dry_highway_survey() {
        let seed = 93;
        let world = World::new(seed);
        let pair = HighwayPair::new((-2, 0), (-2, 1)).unwrap();
        let look = Camera { x: -1431.8726, z: 970.2853, altitude: 140.0,
            heading: -0.10283643, pitch: 0.13, bank: 0.0 };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(look, true));
        land.preflight_skyline(look);
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        let mut rivers = RiverStream::<3>::new(world, seed);
        rivers.preflight(&land, &facilities, look, &mut WatershedWork::new());
        let mut routes = LocalRouteStream::<8>::new(world, seed);
        routes.preflight(&land, look, |key, route| {
            route_reservation(&world, &facilities, &rivers, key, route)
        });
        let mut stream = HighwayStream::<4>::new(world, seed);
        let reserve = |key, link: &HighwayConnection| {
            highway_reservation(&land, &facilities, &rivers, &routes, key, link)
        };
        stream.preflight_with_coast(&land, look, &mut CoastalHighwayWork::new(), reserve, || {});
        let (_, link) = stream.links().find(|(key, _)| *key == pair)
            .expect("a dry road joining the surveyed town streets");
        let [start, finish] = link.street_caps().map(Option::unwrap);
        assert_eq!(link.polyline().first(), Some(&start));
        assert_eq!(link.polyline().last(), Some(&finish));
        assert!(start.z < ROAD_REGION_SIZE as f32 && finish.z > ROAD_REGION_SIZE as f32);
        assert!(link.town_alignment().is_some());
        for (contact, cap) in link.contacts().into_iter().zip([start, finish]) {
            let [x, z] = contact.region;
            let town = land.planned((x, z)).flatten().unwrap().town(contact_terminus(contact)).unwrap();
            assert_eq!(town.sample(cap.x, cap.z).unwrap().surface, SettlementSurface::Street);
            assert_ne!(contact.point, cap);
            assert_eq!(stream.overlay(cap.x, cap.z, world.sample(cap.x, cap.z)).surface, Surface::Pavement);
        }
    }

    #[test]
    fn real_town_exits_publish_a_coastal_highway_through_authoritative_reservations() {
        let seed = 4_329;
        let world = World::new(seed);
        let pair = HighwayPair::new((-7, -10), (-7, -9)).unwrap();
        let look = Camera {
            x: -6_706.787_6, z: -9_505.119, altitude: 335.0, heading: -0.143_583_4,
            pitch: 0.53, bank: 0.0,
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(look, true));
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        let mut rivers = RiverStream::<3>::new(world, seed);
        rivers.preflight(&land, &facilities, look, &mut WatershedWork::new());
        let mut routes = LocalRouteStream::<8>::new(world, seed);
        routes.preflight(&land, look, |key, route| {
            route_reservation(&world, &facilities, &rivers, key, route)
        });
        let mut stream = HighwayStream::<4>::new(world, seed);
        let reserve = |key, link: &HighwayConnection| {
            highway_reservation(&land, &facilities, &rivers, &routes, key, link)
        };
        stream.preflight_with_coast(&land, look, &mut CoastalHighwayWork::new(), reserve, || {});
        let (_, crossing) = stream.coastal_links().find(|(key, _)| *key == pair).unwrap();
        for ((contact, cap), approach) in crossing.contacts().into_iter()
            .zip(crossing.street_caps()).zip(crossing.approaches())
        {
            let [x, z] = contact.region;
            let cap = cap.unwrap();
            let town = land.planned((x, z)).unwrap().unwrap().town(contact_terminus(contact)).unwrap();
            assert_eq!(town.sample(cap.x, cap.z).unwrap().surface, SettlementSurface::Street);
            assert_eq!(approach[0], cap);
        }
        assert_eq!(crossing.bridge().water, crate::crossings::BridgeWater::SeaNarrows);
        assert!(stream.obstruction(-6_797.73, -9_141.43) > 0.9);

        stream.preflight_defer_coast(&land, look, reserve, || {});
        assert!(stream.coastal_links().next().is_none());
        let mut work = CoastalHighwayWork::new();
        for _ in 0..32 {
            let Some(request) = stream.request(&land, look, true) else { break; };
            let reply = request.prepare_with_towns(&mut work, &request.copy_towns(&land), || {});
            assert!(stream.commit(reply, &land, look));
            stream.service_with_reservations(&land, look, true, reserve);
        }
        assert!(matches!(stream.planned(pair), Some(Some(HighwayConnection::Coastal(_)))));
        let beyond = Camera { x: look.x + 3_500.0, ..look };
        stream.service_with_reservations(&land, beyond, false, reserve);
        let bridge = *stream.coastal_links().find(|(key, _)| *key == pair).unwrap().1.bridge();

        // At cruise altitude this real span should survive beyond its town
        // tiles, and its pixels should still respect the terrain depth.
        let cruise = Camera {
            x: -7_305.035_6, z: -6_958.678_7, altitude: 335.0, heading: 2.913_061_6,
            pitch: 0.189, bank: 0.04,
        };
        land.reset(world);
        assert!(!bounds_in_view(cruise, bridge.bounds()));
        stream.service_with_reservations(&land, cruise, false, |_, _| panic!());
        assert!(stream.coastal_links().any(|(key, _)| key == pair));

        use crate::bridge_silhouette::BridgeSilhouettes;
        use crate::horizon_cache::HorizonCache;
        use crate::render::{ScreenRect, encoded_depth, render_terrain_layer_region_with_horizon};
        use crate::sky::Sky;
        use crate::terrain_cache::TerrainCache;

        const RASTER: (usize, usize) = (60, 60);
        const AREA: usize = RASTER.0 * RASTER.1;
        let mut horizon = HorizonCache::new();
        horizon.reset(world, cruise.x, cruise.z);
        let backdrop = |x, z| horizon.backdrop(x, z);
        let sky = Sky::from_time_at_position(seed, 0, 0.34, [cruise.x, cruise.altitude, cruise.z]);
        let spans = || stream.coastal_links().map(|(_, link)| link.bridge());
        let mut silhouettes = BridgeSilhouettes::new();
        assert!(silhouettes.prepare(cruise, 0, &sky, RASTER, spans(), backdrop).is_empty());
        for tick in 1..=32 {
            silhouettes.prepare(cruise, tick, &sky, RASTER, spans(), backdrop);
        }
        let frame = silhouettes.prepare(cruise, 33, &sky, RASTER, spans(), backdrop);
        assert_eq!(frame.len(), 1);

        let mut terrain = TerrainCache::new();
        terrain.reset_for_altitude(world, cruise.x, cruise.z, cruise.altitude);
        let mut pixels = [0; AREA];
        let mut depth = [u16::MAX; AREA];
        let mut scratch = [0; AREA];
        render_terrain_layer_region_with_horizon(
            (|x, z| terrain.sample(x, z), |_, _, sample| sample.surface),
            backdrop, cruise, &sky, &mut pixels, &mut scratch,
            Some((&mut depth, ScreenRect::full(RASTER))), RASTER, |_| {}, |_| None,
        ).unwrap();
        let clear = pixels;
        frame.draw(&mut pixels, &depth);
        assert!(pixels != clear);
        pixels = clear;
        frame.draw(&mut pixels, &[encoded_depth(1_800.0); AREA]);
        assert_eq!(pixels, clear);
    }

    #[test]
    fn narrow_area_tests_use_the_actual_link_and_retain_every_sampled_shoulder() {
        let world = World::new(SEED);
        let first = RoadNetwork::generate(&world, WEST.0, WEST.1).unwrap();
        let last = RoadNetwork::generate(&world, EAST.0, EAST.1).unwrap();
        let link = HighwayLink::fit(&world, &first, &last).unwrap();
        for edge in link.polyline().windows(2) {
            let dx = edge[1].x - edge[0].x;
            let dz = edge[1].z - edge[0].z;
            let len = dx.hypot(dz);
            for t in [0.0, 0.18, 0.39, 0.60, 0.77, 1.0] {
                for offset in [-4.49, -2.8, 0.0, 2.8, 4.49] {
                    let x = edge[0].x + dx * t - dz / len * offset;
                    let z = edge[0].z + dz * t + dx / len * offset;
                    assert!(link.intersects_bounds([x, z, x, z]));
                }
            }
        }
        let [left, near, right, far] = link.bounds();
        assert!([(left, near), (left, far), (right, near), (right, far)]
            .into_iter()
            .any(|(x, z)| !link.intersects_bounds([x - 0.1, z - 0.1, x + 0.1, z + 0.1])));
    }

    #[test]
    fn authoritative_regional_layers_allow_vacant_contacts_and_reject_an_occupied_corridor() {
        for (seed, a, b, expected) in [
            (2114, (35, -19), (35, -18), true),
            (503_821, (-19, 35), (-19, 36), true),
            (31, (1, 5), (2, 5), false),
        ] {
            let world = World::new(seed);
            let pair = HighwayPair::new(a, b).unwrap();
            let roads = [a, b].map(|(x, z)| RoadNetwork::generate(&world, x, z).unwrap());
            let dry_link = HighwayLink::fit(&world, &roads[0], &roads[1]).unwrap();
            let [a, b] = dry_link.contacts().map(|contact| contact.point);
            let look = camera(
                (a.x + b.x) * 0.5,
                (a.z + b.z) * 0.5,
                (b.x - a.x).atan2(b.z - a.z),
            );
            let mut land = LandStream::<8>::new();
            land.reset(world);
            land.preflight(look, true);
            assert!(published_owners(&land, pair));
            let mut facilities = FacilityStream::<8>::new(world, seed);
            facilities.preflight(&land);
            let mut rivers = RiverStream::<3>::new(world, seed);
            rivers.preflight(&land, &facilities, look, &mut WatershedWork::new());
            let mut routes = LocalRouteStream::<8>::new(world, seed);
            routes.preflight(&land, look, |key, route| {
                route_reservation(&world, &facilities, &rivers, key, route)
            });
            let mut highways = HighwayStream::<4>::new(world, seed);
            highways.preflight(
                &land,
                look,
                |key, link| highway_reservation(&land, &facilities, &rivers, &routes, key, link),
                || {},
            );
            assert_eq!(
                highways.links().any(|(key, _)| key == pair),
                expected,
                "seed {seed}"
            );
            if !expected {
                assert!(matches!(highways.planned(pair), Some(None)));
            }
        }
    }
}
