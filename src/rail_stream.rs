//! Retained railways surveyed away from the frame loop.

use crate::facilities::train_route_near;
use crate::facility_stream::FacilityStream;
use crate::highway_links::{HighwayContact, HighwaySurvey};
use crate::highway_stream::{HighwayPair, HighwayStream};
use crate::land_stream::LandStream;
use crate::local_routes::{LocalRoute, LocalRouteStream};
use crate::math::sin_cos;
use crate::natural_stream::NaturalStream;
use crate::ports::PortKind;
use crate::rail::RailSample;
use crate::rail_art::cover_rail;
use crate::rail_corridor::RailCorridor;
use crate::rail_survey::has_aggregate_transfer;
use crate::render::{view_distance, Camera};
use crate::river_stream::RiverStream;
use crate::train_art::FreightRoute;
use crate::world::{Sample, World};
use crate::world_mesh::{bounds_in_range, bounds_in_view};

type Area = [f32; 4];
const ACTIVE: usize = 2;
const REJECTED: usize = 24;
const CHECKS_PER_SERVICE: usize = 4;
const BUFFER: f32 = 192.0;

#[derive(Clone, Copy)]
pub struct RailRequest {
    pub world: World,
    pub seed: u64,
    pub pair: HighwayPair,
    serial: u32,
}

struct Pending {
    request: RailRequest,
    contacts: [HighwayContact; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RailProgress {
    Idle,
    Checking,
    Rejected,
    Reserved,
    Published,
}

enum State {
    Candidate { checked: usize },
    Staged,
    Published,
}

struct Resident {
    pair: HighwayPair,
    line: RailCorridor,
    state: State,
    aggregate_pending: bool,
}

impl Resident {
    fn reserved_chunks(&self) -> impl Iterator<Item = Area> + '_ {
        let count = match self.state {
            State::Candidate { checked } => checked,
            State::Staged | State::Published => usize::MAX,
        };
        self.line.footprint_chunks().take(count)
    }
}

pub struct RailStream {
    world: World,
    seed: u64,
    serial: u32,
    pending: Option<Pending>,
    rejected: [Option<HighwayPair>; REJECTED],
    residents: [Option<Resident>; ACTIVE],
    quarry_revision: Option<u32>,
}

pub struct RailSampler<'a> {
    lines: [Option<(Area, &'a RailCorridor)>; ACTIVE],
}

impl RailSampler<'_> {
    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        for &([left, near, right, far], line) in self.lines.iter().flatten() {
            if !(left <= x && x <= right && near <= z && z <= far) {
                continue;
            }
            let Some(piece) = line.sample(x, z, ground.elevation) else {
                continue;
            };
            if let Some(material) = piece.material {
                return cover_rail(
                    RailSample {
                        material,
                        elevation: Some(piece.elevation),
                    },
                    ground,
                );
            }
            if let Some(earthwork) = crate::rail_art::cover_rail_earthwork(piece.elevation, ground) {
                return earthwork;
            }
        }
        ground
    }
}

impl RailStream {
    pub const fn new(world: World, seed: u64) -> Self {
        Self {
            world,
            seed,
            serial: 0,
            pending: None,
            rejected: [None; REJECTED],
            residents: [const { None }; ACTIVE],
            quarry_revision: None,
        }
    }

    pub fn reset(&mut self, world: World, seed: u64) {
        let serial = self.serial.wrapping_add(1);
        *self = Self::new(world, seed);
        self.serial = serial;
    }

    /// Ask a detached worker for a dry connection once both destination plans
    /// and their water reservations are known. An empty river plan is ready.
    pub fn request(
        &mut self,
        land: &LandStream<8>,
        facilities: &FacilityStream<8>,
        rivers: &RiverStream<3>,
        camera: Camera,
    ) -> Option<RailRequest> {
        if !valid(camera) {
            return None;
        }
        self.refresh(land, camera);
        if self.pending.is_some() || self.residents.iter().all(Option::is_some) {
            return None;
        }
        let (sin, cos) = sin_cos(camera.heading);
        let ahead = view_distance(camera.altitude) * 1.3;
        let focus = [camera.x + sin * ahead, camera.z + cos * ahead];
        let (pair, _, contacts) = land
            .districts()
            .flat_map(|((x, z), _)| {
                [
                    x.checked_add(1)
                        .and_then(|x1| HighwayPair::new((x, z), (x1, z))),
                    z.checked_add(1)
                        .and_then(|z1| HighwayPair::new((x, z), (x, z1))),
                ]
                .into_iter()
                .flatten()
            })
            .filter_map(|pair| {
                if self.rejected[directory(pair)] == Some(pair)
                    || self
                        .residents
                        .iter()
                        .flatten()
                        .any(|record| record.pair == pair)
                {
                    return None;
                }
                let [a, b] = pair.regions();
                let districts = [land.planned(a)??, land.planned(b)??];
                let facilities = [facilities.planned(a)?, facilities.planned(b)?];
                let survey = survey(pair, land)?;
                for (index, key) in [a, b].into_iter().enumerate() {
                    rivers.planned(key)?;
                    let quarry_transfer = facilities[index].quarry().is_some()
                        && facilities[index ^ 1].ports().any(|port| port.kind() == PortKind::Freight);
                    if districts[index].towns().next().is_none()
                        && facilities[index].ports().next().is_none() && !quarry_transfer
                    {
                        return None;
                    }
                }
                let [a, b] = survey.contacts.map(|contact| contact.point);
                let x = (a.x + b.x) * 0.5 - focus[0];
                let z = (a.z + b.z) * 0.5 - focus[1];
                Some((pair, x * x + z * z, survey.contacts))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))?;

        self.serial = self.serial.wrapping_add(1);
        let request = RailRequest {
            world: self.world,
            seed: self.seed,
            pair,
            serial: self.serial,
        };
        self.pending = Some(Pending { request, contacts });
        Some(request)
    }

    pub fn cancel(&mut self, request: RailRequest) {
        if self.pending_matches(request) {
            self.pending = None;
        }
    }

    fn pending_matches(&self, request: RailRequest) -> bool {
        self.pending.as_ref().is_some_and(|pending| {
            pending.request.serial == request.serial
                && pending.request.seed == request.seed
                && pending.request.pair == request.pair
        })
    }

    /// Retain only replies still owned by the two current regional roads.
    /// A completed worker survey still needs the live footprint check.
    pub fn commit(
        &mut self,
        request: RailRequest,
        line: Option<RailCorridor>,
        land: &LandStream<8>,
        camera: Camera,
    ) -> bool {
        if !self.pending_matches(request) {
            return false;
        }
        let pending = self.pending.take().unwrap();
        if !valid(camera)
            || !survey(request.pair, land).is_some_and(|survey| survey.contacts == pending.contacts)
        {
            return false;
        }
        self.refresh(land, camera);
        let Some(line) = line else {
            self.rejected[directory(request.pair)] = Some(request.pair);
            return true;
        };
        if line.regions().map(|[x, z]| (x, z)) != request.pair.regions() {
            return false;
        }
        let Some(slot) = self.residents.iter_mut().find(|slot| slot.is_none()) else {
            return false;
        };
        *slot = Some(Resident {
            pair: request.pair,
            line,
            state: State::Candidate { checked: 0 },
            aggregate_pending: false,
        });
        self.quarry_revision = None;
        true
    }

    /// Update dispatch after a quarry or corridor is retained. A train already
    /// within reach keeps its manifest until the aircraft leaves the whole corridor.
    pub fn connect_quarries(&mut self, land: &LandStream<8>, facilities: &FacilityStream<8>, camera: Camera) {
        let revision = facilities.quarry_revision();
        if self.quarry_revision != Some(revision) {
            self.quarry_revision = Some(revision);
            for resident in self.residents.iter_mut().flatten() {
                if resident.line.freight_route == FreightRoute::Aggregate || resident.aggregate_pending {
                    continue;
                }
                let [a, b] = resident.pair.regions();
                if let (Some(Some(a)), Some(Some(b)), Some(af), Some(bf)) =
                    (land.planned(a), land.planned(b), facilities.planned(a), facilities.planned(b))
                {
                    resident.aggregate_pending = has_aggregate_transfer(&resident.line, [a, b], [af, bf]);
                }
            }
        }
        if !valid(camera) {
            return;
        }
        for resident in self.residents.iter_mut().flatten().filter(|resident| resident.aggregate_pending) {
            if !matches!(resident.state, State::Published) || !train_route_near(camera, resident.line.bounds()) {
                resident.line.freight_route = FreightRoute::Aggregate;
                resident.aggregate_pending = false;
            }
        }
    }

    /// Validate a short run of local footprints against stable reservations.
    /// Every checked chunk starts reserving before the next pass can run.
    pub fn service(
        &mut self,
        land: &LandStream<8>,
        camera: Camera,
        mut blocked: impl FnMut(Area) -> bool,
    ) -> RailProgress {
        if !valid(camera) {
            return RailProgress::Idle;
        }
        if self.refresh(land, camera) {
            return RailProgress::Published;
        }
        let Some((slot, resident, checked)) =
            self.residents.iter().enumerate().find_map(|(slot, item)| {
                let resident = item.as_ref()?;
                let State::Candidate { checked } = resident.state else {
                    return None;
                };
                Some((slot, resident, checked))
            })
        else {
            return RailProgress::Idle;
        };
        let (rejected, finished, count) = {
            let mut chunks = resident.line.footprint_chunks().skip(checked);
            let mut count = 0;
            let rejected = chunks.by_ref().take(CHECKS_PER_SERVICE).any(|area| {
                count += 1;
                self.reserves_except(area, Some(slot)) || blocked(area)
            });
            (rejected, chunks.next().is_none(), count)
        };
        if rejected {
            self.rejected[directory(resident.pair)] = Some(resident.pair);
            self.residents[slot] = None;
            return RailProgress::Rejected;
        }
        let resident = self.residents[slot].as_mut().unwrap();
        if !finished {
            resident.state = State::Candidate {
                checked: checked + count,
            };
            return RailProgress::Checking;
        }
        if can_publish(land, camera, resident) {
            resident.state = State::Published;
            RailProgress::Published
        } else {
            resident.state = State::Staged;
            RailProgress::Reserved
        }
    }

    fn refresh(&mut self, land: &LandStream<8>, camera: Camera) -> bool {
        let mut published = false;
        for slot in &mut self.residents {
            let Some(resident) = slot else { continue };
            let held = resident
                .pair
                .regions()
                .into_iter()
                .all(|key| land.planned(key).flatten().is_some());
            if !held {
                let retain = matches!(resident.state, State::Published)
                    && !outside(
                        camera,
                        resident.line.bounds(),
                        view_distance(camera.altitude) * 1.25 + BUFFER,
                    );
                if !retain {
                    *slot = None;
                }
            } else if matches!(resident.state, State::Staged) && can_publish(land, camera, resident)
            {
                resident.state = State::Published;
                published = true;
            }
        }
        published
    }

    fn published(&self) -> impl Iterator<Item = &RailCorridor> {
        self.residents.iter().flatten().filter_map(|resident| {
            matches!(resident.state, State::Published).then_some(&resident.line)
        })
    }

    pub fn rails(&self, camera: Camera, reach: f32) -> impl Iterator<Item = &RailCorridor> {
        let view_reach = view_distance(camera.altitude).max(reach);
        self.published().filter(move |line| {
            let area = line.bounds();
            distance_squared(camera, area) <= reach * reach && bounds_in_range(camera, area, view_reach)
        })
    }

    pub fn reserves(&self, area: Area) -> bool {
        self.reserves_except(area, None)
    }

    fn reserves_except(&self, area: Area, except: Option<usize>) -> bool {
        self.residents.iter().enumerate().any(|(index, resident)| {
            Some(index) != except
                && resident.as_ref().is_some_and(|resident| {
                    overlaps(resident.line.bounds(), area)
                        && resident
                            .reserved_chunks()
                            .any(|chunk| overlaps(chunk, area))
                })
        })
    }

    pub fn conflicts_with(&self, mut blocked: impl FnMut(Area) -> bool) -> bool {
        self.residents
            .iter()
            .flatten()
            .any(|resident| resident.reserved_chunks().any(&mut blocked))
    }

    pub fn sampler(&self) -> RailSampler<'_> {
        let mut lines = [None; ACTIVE];
        for (slot, line) in lines.iter_mut().zip(self.published()) {
            *slot = Some((line.bounds(), line));
        }
        RailSampler { lines }
    }
}

/// Recheck authoritative work that may have settled during the worker survey.
/// The worker separately checks the surveyed roads and station destinations.
pub fn scene_blocks(
    land: &LandStream<8>,
    rivers: &RiverStream<3>,
    routes: &LocalRouteStream<8>,
    natural: &NaturalStream,
    highways: &HighwayStream<4>,
    area: Area,
) -> bool {
    let route_blocks = |route: &LocalRoute| {
        overlaps(route.bounds(), area) && route.ground_conflicts_with(|chunk| overlaps(chunk, area))
    };
    rivers.reserves_scenery(area)
        || highways.reserves(area)
        || natural
            .approved_bounds()
            .any(|bounds| overlaps(bounds, area))
        || land
            .districts()
            .any(|(key, _)| routes.planned(key).flatten().is_some_and(route_blocks))
        || routes
            .routes()
            .any(|(key, route)| land.planned(key).flatten().is_none() && route_blocks(route))
}

fn survey(pair: HighwayPair, land: &LandStream<8>) -> Option<HighwaySurvey> {
    let [a, b] = pair.regions();
    let survey = HighwaySurvey::between(land.planned(a)??.road()?, land.planned(b)??.road()?)?;
    let owners = survey
        .contacts
        .map(|contact| (contact.region[0], contact.region[1]));
    (owners == [a, b] && survey.owns_termini()).then_some(survey)
}

fn can_publish(land: &LandStream<8>, camera: Camera, resident: &Resident) -> bool {
    resident
        .pair
        .regions()
        .into_iter()
        .all(|key| land.district_published(key))
        && outside(camera, resident.line.bounds(), BUFFER)
}

fn directory(pair: HighwayPair) -> usize {
    let [(x, z), (_, other_z)] = pair.regions();
    (x.wrapping_mul(13)
        .wrapping_add(z.wrapping_mul(39))
        .wrapping_add(i32::from(z != other_z)))
    .rem_euclid(REJECTED as i32) as usize
}

fn valid(camera: Camera) -> bool {
    [camera.x, camera.z, camera.heading, camera.altitude]
        .into_iter()
        .all(f32::is_finite)
}

fn distance_squared(camera: Camera, [left, near, right, far]: Area) -> f32 {
    let dx = camera.x - camera.x.clamp(left, right);
    let dz = camera.z - camera.z.clamp(near, far);
    dx * dx + dz * dz
}

fn outside(camera: Camera, area: Area, guard: f32) -> bool {
    distance_squared(camera, area) > guard * guard && !bounds_in_view(camera, area)
}

fn overlaps(a: Area, b: Area) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highway_stream::highway_reservation;
    use crate::local_routes::route_reservation;
    use crate::rail_survey::{RailSurveyData, RailSurveyWork};
    use crate::watershed::WatershedWork;
    use crate::world::Surface;
    use std::vec::Vec;

    #[test]
    fn real_neighbor_rail_reserves_then_publishes_outside_view() {
        let seed = 7911;
        let world = World::new(seed);
        let pair = HighwayPair::new((-2, 3), (-2, 4)).unwrap();
        let visible = Camera {
            x: -1510.0,
            z: 3970.0,
            altitude: 72.0,
            heading: -0.16,
            pitch: 0.12,
            bank: 0.0,
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(visible, true));
        let mut facilities = FacilityStream::<8>::new(world, seed);
        let mut rivers = RiverStream::<3>::new(world, seed);
        let mut rails = RailStream::new(world, seed);
        assert!(rails
            .request(&land, &facilities, &rivers, visible)
            .is_none());
        facilities.preflight(&land);
        assert!(rails
            .request(&land, &facilities, &rivers, visible)
            .is_none());
        let mut water_work = WatershedWork::new();
        rivers.preflight(&land, &facilities, visible, &mut water_work);
        let mut routes = LocalRouteStream::<8>::new(world, seed);
        routes.preflight(&land, visible, |key, route| {
            route_reservation(&world, &facilities, &rivers, key, route)
        });
        let mut highways = HighwayStream::<4>::new(world, seed);
        highways.preflight(
            &land,
            visible,
            |key, link| highway_reservation(&land, &facilities, &rivers, &routes, key, link),
            || {},
        );
        assert!(!highways.links().any(|(key, _)| key == pair));
        let nature = NaturalStream::new(world, seed);

        let stale = rails.request(&land, &facilities, &rivers, visible).unwrap();
        assert_eq!(stale.pair, pair);
        rails.reset(world, seed);
        let request = rails.request(&land, &facilities, &rivers, visible).unwrap();
        assert_eq!(request.pair, pair);
        assert!(!rails.commit(stale, None, &land, visible));
        let owners = pair.regions();
        let mut data = std::boxed::Box::new(RailSurveyData::new());
        let mut work = std::boxed::Box::new(RailSurveyWork::new());
        assert!(data.load(
            owners.map(|key| land.planned(key).flatten().unwrap()),
            owners.map(|key| facilities.planned(key).unwrap()),
            &highways,
            pair,
        ));
        let line = data
            .prepare(&world, seed, &mut work, &mut water_work, || {})
            .unwrap();
        assert!((line.length() - 533.03).abs() < 0.1);
        let track = line.position_at(line.length() * 0.5);
        let base = world.sample(track.x, track.z);
        let bounds = [track.x, track.z, track.x, track.z];
        let chunk_count = line.footprint_chunks().count();

        let mut rejected = RailStream::new(world, seed);
        let probe = rejected
            .request(&land, &facilities, &rivers, visible)
            .unwrap();
        assert!(rejected.commit(probe, Some(line.clone()), &land, visible));
        let first_chunk = line.footprint_chunks().next().unwrap();
        assert_eq!(
            rejected.service(&land, visible, |_| false),
            RailProgress::Checking
        );
        assert!(rejected.reserves(first_chunk));
        assert_eq!(
            rejected.service(&land, visible, |_| true),
            RailProgress::Rejected
        );
        assert!(!rejected.reserves(first_chunk));

        assert!(rails.commit(request, Some(line.clone()), &land, visible));
        assert!(!rails.commit(request, None, &land, visible));
        assert!(!rails.reserves(bounds));

        let mut checked = 0;
        let mut checked_chunks = Vec::new();
        while checked < chunk_count {
            let before = checked;
            rails.service(&land, visible, |area| {
                checked += 1;
                checked_chunks.push(area);
                assert!(!scene_blocks(
                    &land, &rivers, &routes, &nature, &highways, area
                ));
                false
            });
            assert!((1..=CHECKS_PER_SERVICE).contains(&(checked - before)));
            assert!(checked_chunks.iter().all(|area| rails.reserves(*area)));
            let mut conflicts = Vec::new();
            rails.conflicts_with(|area| {
                conflicts.push(area);
                false
            });
            assert_eq!(conflicts, checked_chunks);
            assert_eq!(rails.rails(visible, 512.0).count(), 0);
            assert_eq!(rails.sampler().overlay(track.x, track.z, base), base);
        }
        assert_eq!(checked, chunk_count);
        assert!(rails.reserves(bounds));
        assert!(rails.conflicts_with(|area| overlaps(area, bounds)));
        let safe = Camera {
            z: line.bounds()[1] - 400.0,
            heading: core::f32::consts::PI,
            ..visible
        };
        assert_eq!(
            rails.service(&land, safe, |_| panic!()),
            RailProgress::Published
        );
        assert_eq!(rails.rails(safe, 512.0).count(), 0);
        assert_eq!(rails.rails(visible, 512.0).next(), Some(&line));
        assert_eq!(rails.rails(visible, 2.0).next(), Some(&line));
        let [left, near, right, _] = line.bounds();
        let approach = Camera { x: (left + right) * 0.5, z: near - 615.0, heading: 0.0, ..visible };
        assert!(!bounds_in_view(approach, line.bounds()));
        assert_eq!(rails.rails(approach, crate::train_art::TRAIN_RENDER_REACH).next(), Some(&line));

        // Exercise a complete published grade with its depot-change state queued.
        // The camera can turn away from the track while still close enough for a train.
        rails.residents.iter_mut().flatten().next().unwrap().aggregate_pending = true;
        let turned = Camera { heading: core::f32::consts::PI, ..visible };
        rails.connect_quarries(&land, &facilities, turned);
        let resident = rails.residents.iter().flatten().next().unwrap();
        assert!(resident.aggregate_pending);
        assert_eq!(resident.line.freight_route, line.freight_route);
        rails.connect_quarries(&land, &facilities, safe);
        let resident = rails.residents.iter().flatten().next().unwrap();
        assert!(!resident.aggregate_pending);
        assert_eq!(resident.line.freight_route, FreightRoute::Aggregate);
        for point in line.polyline() {
            let raw = world.sample(point.x, point.z);
            let piece = line.sample(point.x, point.z, raw.elevation).unwrap();
            let expected = cover_rail(
                RailSample {
                    material: piece.material.unwrap(),
                    elevation: Some(piece.elevation),
                },
                raw,
            );
            assert_eq!(rails.sampler().overlay(point.x, point.z, raw), expected);
            let river = Sample {
                surface: Surface::RiverWater,
                ..raw
            };
            assert_eq!(rails.sampler().overlay(point.x, point.z, river), river);
        }
        let a = line.position_at(line.length() * 0.5 - 0.5);
        let b = line.position_at(line.length() * 0.5 + 0.5);
        let x = track.x - 3.0 * (b.z - a.z);
        let z = track.z + 3.0 * (b.x - a.x);
        let forest = Sample {
            surface: Surface::Forest,
            ..world.sample(x, z)
        };
        let shoulder = line.sample(x, z, forest.elevation).unwrap();
        assert!(shoulder.material.is_none());
        assert_eq!(
            rails.sampler().overlay(x, z, forest),
            crate::rail_art::cover_rail_earthwork(shoulder.elevation, forest).unwrap(),
        );
        let mut rock_cut = false;
        let mut gravel_fill = false;
        let sampler = rails.sampler();
        for distance in (32..(line.length() as usize - 32)).step_by(3) {
            let distance = distance as f32;
            let point = line.position_at(distance);
            let a = line.position_at(distance - 0.5);
            let b = line.position_at(distance + 0.5);
            for across in [-5.4, -3.7, -2.3, 2.3, 3.7, 5.4] {
                let x = point.x - across * (b.z - a.z);
                let z = point.z + across * (b.x - a.x);
                let raw = world.sample(x, z);
                let shaped = sampler.overlay(x, z, raw);
                let Some(piece) = line.sample(x, z, raw.elevation) else { continue };
                if piece.material.is_some() || shaped == raw {
                    continue;
                }
                assert_eq!(shaped.elevation.to_bits(), piece.elevation.to_bits());
                rock_cut |= shaped.surface == Surface::Rock && raw.surface != Surface::Rock
                    && shaped.elevation < raw.elevation;
                gravel_fill |= shaped.surface == Surface::Gravel && raw.surface != Surface::Gravel
                    && shaped.elevation > raw.elevation;
            }
        }
        assert!(rock_cut && gravel_fill, "published route must show both cuts and fills");

        land.reset(world);
        rails.service(&land, visible, |_| panic!());
        assert!(rails.reserves(bounds));
        let departed = Camera {
            x: visible.x + 4096.0,
            ..visible
        };
        rails.service(&land, departed, |_| panic!());
        assert!(!rails.reserves(bounds));
        assert_eq!(rails.sampler().overlay(track.x, track.z, base), base);
    }
}
