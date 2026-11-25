//! Detached input and planning for a regional railway. Copy only retained
//! owner plans across cores; regenerate immutable water geometry on the worker.

use crate::facilities::{FacilitySample, RegionalFacilities};
use crate::highway_links::HighwaySurvey;
use crate::highway_stream::{HighwayPair, HighwayStream};
use crate::ports::PortSurface;
use crate::rail_corridor::{CorridorGuide, CorridorStop, CorridorUse, CorridorWork, RailCorridor};
use crate::roads::{RoadNetwork, RoadPoint};
use crate::scenery::DistrictRegion;
use crate::settlement::{SettlementSurface, Terminus};
use crate::watershed::{Watershed, WatershedWork};
use crate::world::World;

const STOPS_PER_DISTRICT: usize = 6;
const MAX_TRIALS: usize = 12;

/// A single job's authoritative inputs. Put this in the PSRAM mailbox and
/// populate it in place, rather than moving it through the command enum.
pub struct RailSurveyData {
    survey: Option<HighwaySurvey>,
    districts: [Option<DistrictRegion>; 2],
    facilities: [Option<RegionalFacilities>; 2],
    guide: Option<CorridorGuide>,
}

/// Only the survey core mutates this persistent worker buffer.
#[derive(Clone, Copy)]
pub struct RailSurveyWork {
    railway: CorridorWork,
    watersheds: [Watershed; 2],
}

impl RailSurveyData {
    pub const fn new() -> Self {
        Self {
            survey: None,
            districts: [None, None],
            facilities: [None, None],
            guide: None,
        }
    }

    /// Load corresponding district/facility owners in either order. A stale
    /// or mismatched request invalidates the previously loaded job.
    pub fn load(
        &mut self,
        districts: [&DistrictRegion; 2],
        facilities: [&RegionalFacilities; 2],
        regional_roads: &HighwayStream<4>,
        pair: HighwayPair,
    ) -> bool {
        self.survey = None;
        let [Some(a), Some(b)] = districts.map(DistrictRegion::road) else {
            return false;
        };
        let Some(survey) = HighwaySurvey::between(a, b) else {
            return false;
        };
        let regions = survey
            .contacts
            .map(|contact| (contact.region[0], contact.region[1]));
        if pair.regions() != regions || !survey.owns_termini() {
            return false;
        }
        let contacts = survey.contacts.map(|contact| contact.point);
        let order = if owns_road_end(a, contacts[0]) && owns_road_end(b, contacts[1]) {
            [0, 1]
        } else if owns_road_end(b, contacts[0]) && owns_road_end(a, contacts[1]) {
            [1, 0]
        } else {
            return false;
        };
        for (to, from) in order.into_iter().enumerate() {
            self.districts[to] = Some(districts[from].clone());
            self.facilities[to] = Some(facilities[from].clone());
        }
        self.guide = regional_roads.links().find_map(|(key, link)| {
            (key == pair)
                .then(|| CorridorGuide::from_highway(link))
                .flatten()
        });
        self.survey = Some(survey);
        true
    }

    pub fn prepare(
        &self,
        world: &World,
        seed: u64,
        work: &mut RailSurveyWork,
        water: &mut WatershedWork,
        mut pulse: impl FnMut(),
    ) -> Option<RailCorridor> {
        let survey = self.survey?;
        let [Some(a), Some(b)] = self.districts.each_ref() else {
            return None;
        };
        let [Some(af), Some(bf)] = self.facilities.each_ref() else {
            return None;
        };
        let road = [a.road()?, b.road()?];
        let districts = [a, b];
        let facilities = [af, bf];
        let stops = core::array::from_fn::<_, 2, _>(|i| {
            let contact = survey.contacts[i];
            stop_candidates(
                world,
                districts[i],
                facilities[i],
                contact.region,
                survey.contacts[i ^ 1].point,
                &mut pulse,
            )
        });
        if stops.iter().any(|sites| sites[0].is_none()) {
            return None;
        }
        for (watershed, contact) in work.watersheds.iter_mut().zip(survey.contacts) {
            let [x, z] = contact.region;
            watershed.rebuild_cooperatively(world, seed, x, z, water, &mut pulse);
        }
        let wet = |x, z| {
            work.watersheds.iter().any(|watershed| {
                watershed
                    .sample(x, z)
                    .is_some_and(|reach| reach.is_channel())
            })
        };
        let raw_guide;
        let guide = if let Some(published) = &self.guide {
            published
        } else {
            // The same dry geographic band may remain useful after the road
            // publisher rejects its own distinct land-use footprint.
            let (candidate, _) = survey.fit(world, 0, false, wet, &mut pulse)?;
            raw_guide = CorridorGuide::from_highway(&candidate)?;
            &raw_guide
        };
        let facility_bounds = facilities.map(RegionalFacilities::bounds);
        let blocked = |x, z, kind| {
            let access = kind == CorridorUse::Access;
            if wet(x, z)
                || (!access
                    && (guide.covers_road(x, z)
                        || road.iter().any(|network| network.proximity(x, z).is_some())))
                || districts
                    .iter()
                    .flat_map(|district| district.towns())
                    .any(|town| {
                        in_bounds(town.bounds(), x, z)
                            && town.sample_transport_reservation(x, z).is_some_and(|piece| {
                                !access || piece.surface != SettlementSurface::Street
                            })
                    })
            {
                return true;
            }
            facilities
                .into_iter()
                .zip(facility_bounds)
                .any(|(current, bounds)| {
                    bounds.is_some_and(|bounds| in_bounds(bounds, x, z))
                        && (current
                            .rail()
                            .is_some_and(|rail| rail.sample(x, z).is_some())
                            || current
                                .sample(x, z, world.sample(x, z))
                                .is_some_and(|piece| match piece {
                                    FacilitySample::Port(port) => {
                                        !access
                                            || !matches!(
                                                port.surface,
                                                PortSurface::AccessRoad | PortSurface::Apron
                                            )
                                    }
                                    FacilitySample::Airfield(_) | FacilitySample::Rail(_)
                                    | FacilitySample::Quarry | FacilitySample::Solar(_)
                                    | FacilitySample::Quayside(_) | FacilitySample::WaterTower => true,
                                }))
                })
        };
        let mut tried = 0_u64;
        for _ in 0..MAX_TRIALS {
            let (index, pair) = next_pair(stops, tried)?;
            tried |= 1 << index;
            if let Some(line) = RailCorridor::survey_detached_cooperatively(
                world,
                guide,
                pair,
                &mut work.railway,
                blocked,
                &mut pulse,
            ) {
                return Some(line);
            }
        }
        None
    }
}

impl RailSurveyWork {
    pub const fn new() -> Self {
        Self {
            railway: CorridorWork::new(),
            watersheds: [Watershed::new(), Watershed::new()],
        }
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    stop: CorridorStop,
    cost: f32,
}

type Stops = [Option<Candidate>; STOPS_PER_DISTRICT];

fn stop_candidates(
    world: &World,
    district: &DistrictRegion,
    facilities: &RegionalFacilities,
    region: [i32; 2],
    toward: RoadPoint,
    pulse: &mut impl FnMut(),
) -> Stops {
    let mut result = [None; STOPS_PER_DISTRICT];
    let Some(road) = district.road() else {
        return result;
    };
    let points = road.polyline();
    for (end, contact) in [Terminus::Start, Terminus::End]
        .into_iter()
        .zip([points[0], points[points.len() - 1]])
    {
        pulse();
        let entry = [contact.x, contact.z];
        if let Some(port) = facilities.port(end) {
            insert(
                &mut result,
                CorridorStop::port(world, region, entry, port),
                toward,
                0.985,
            );
        }
        if let Some(town) = district.town(end) {
            for exit in town.coastal_road_exits(world, toward).into_iter().flatten() {
                let pos = [exit.point.x, exit.point.z];
                insert(
                    &mut result,
                    CorridorStop::town(region, pos, town),
                    toward,
                    1.0,
                );
            }
            insert(
                &mut result,
                CorridorStop::town(region, entry, town),
                toward,
                1.0,
            );
        }
    }
    result
}

fn insert(stops: &mut Stops, stop: Option<CorridorStop>, toward: RoadPoint, bias: f32) {
    let Some(stop) = stop else {
        return;
    };
    if stops
        .iter()
        .flatten()
        .any(|old| old.stop.entry() == stop.entry())
    {
        return;
    }
    let [x, z] = stop.entry();
    let sum = (x - toward.x) * (x - toward.x) + (z - toward.z) * (z - toward.z);
    let entry = Candidate {
        stop,
        cost: bias * sum,
    };
    if let Some(i) = stops
        .iter()
        .position(|old| old.is_none_or(|value| entry.cost < value.cost))
    {
        stops[i..].rotate_right(1);
        stops[i] = Some(entry);
    }
}

fn next_pair(stops: [Stops; 2], tried: u64) -> Option<(usize, [CorridorStop; 2])> {
    let mut best: Option<(usize, [CorridorStop; 2], f32)> = None;
    for (i, a) in stops[0].into_iter().enumerate() {
        let Some(a) = a else {
            break;
        };
        for (j, b) in stops[1].into_iter().enumerate() {
            let Some(b) = b else {
                break;
            };
            let index = i * STOPS_PER_DISTRICT + j;
            if tried & (1 << index) != 0 {
                continue;
            }
            let cost = a.cost + b.cost;
            if best.is_none_or(|(_, _, old)| cost < old) {
                best = Some((index, [a.stop, b.stop], cost));
            }
        }
    }
    best.map(|(index, pair, _)| (index, pair))
}

fn owns_road_end(road: &RoadNetwork, point: RoadPoint) -> bool {
    let points = road.polyline();
    points.first() == Some(&point) || points.last() == Some(&point)
}

fn in_bounds([left, near, right, far]: [f32; 4], x: f32, z: f32) -> bool {
    (left..=right).contains(&x) && (near..=far).contains(&z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facility_stream::FacilityStream;
    use crate::highway_stream::highway_reservation;
    use crate::land_stream::LandStream;
    use crate::local_routes::{route_reservation, LocalRouteStream};
    use crate::render::Camera;
    use crate::river_stream::RiverStream;

    #[test]
    fn detached_survey_loads_both_retained_owners_and_keeps_the_river_clear() {
        let seed = 7911;
        let world = World::new(seed);
        let pair = HighwayPair::new((-2, 3), (-2, 4)).unwrap();
        let owners = pair.regions();
        let camera = Camera {
            x: -1510.0,
            z: 3970.0,
            altitude: 72.0,
            heading: -0.16,
            pitch: 0.12,
            bank: 0.0,
        };
        let mut land: LandStream<8> = LandStream::new();
        land.reset(world);
        land.preflight(camera, true);
        let mut facilities = FacilityStream::new(world, seed);
        facilities.preflight(&land);
        let mut water_work = WatershedWork::new();
        let mut rivers: RiverStream<3> = RiverStream::new(world, seed);
        rivers.preflight(&land, &facilities, camera, &mut water_work);
        let mut routes: LocalRouteStream<8> = LocalRouteStream::new(world, seed);
        routes.preflight(&land, camera, |key, route| {
            route_reservation(&world, &facilities, &rivers, key, route)
        });
        let mut highways = HighwayStream::new(world, seed);
        highways.preflight(
            &land,
            camera,
            |key, link| highway_reservation(&land, &facilities, &rivers, &routes, key, link),
            || {},
        );
        assert!(rivers.planned(owners[0]).flatten().is_some());
        assert!(highways.planned(pair).is_some_and(|link| link.is_none()));
        let land = owners.map(|key| land.planned(key).flatten().unwrap());
        let facilities = owners.map(|key| facilities.planned(key).unwrap());
        let mut data = std::boxed::Box::new(RailSurveyData::new());
        let mut work = std::boxed::Box::new(RailSurveyWork::new());
        assert!(data.load(
            [land[1], land[0]],
            [facilities[1], facilities[0]],
            &highways,
            pair
        ));
        let line = data
            .prepare(&world, seed, &mut work, &mut water_work, || {})
            .unwrap();
        assert!((line.length() - 533.0256).abs() < 0.02);
        assert_eq!(
            line.stations().map(|s| s.stop.entry()),
            [[-1506.0, 3842.0], [-1605.3381, 4348.2563]]
        );
        assert!(line.footprint_chunks().all(|bounds| owners
            .into_iter()
            .filter_map(|key| rivers.planned(key).flatten())
            .all(|river| !river.intersects_scenery(bounds))));
        let wrong = HighwayPair::new((-2, 4), (-2, 5)).unwrap();
        assert!(!data.load(land, facilities, &highways, wrong));
        assert!(data
            .prepare(&world, seed, &mut work, &mut water_work, || {})
            .is_none());
    }
}
