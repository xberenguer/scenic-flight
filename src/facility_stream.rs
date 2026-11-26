//! Seeded facilities follow prepared highway regions without changing inside the view.

use crate::coastal_landforms::CoastalLandform;
use crate::facilities::{train_route_near, FacilityPhase, RegionalFacilities, RegionalFacilityPlanner, RoadsideSite};
use crate::highway_stream::{HighwayStream, Reservation as HighwayReservation};
use crate::land_stream::LandStream;
use crate::local_routes::LocalRouteStream;
use crate::natural_stream::NaturalStream;
use crate::quayside::Quayside;
use crate::river_district::RiverDistrict;
use crate::river_stream::RiverStream;
use crate::lighthouses::Lighthouse;
use crate::rail::RailLine;
use crate::render::Camera;
use crate::roads::{RoadNetwork, ROAD_REGION_SIZE};
use crate::scenery::{overlay_facilities, DistrictRegion};
use crate::settlement::{Settlement, Terminus};
use crate::solar_sites::SolarSite;
use crate::water_tower::WaterTower;
use crate::working_quarry::Quarry;
use crate::world::{Sample, World};
use crate::world_mesh::{bounds_in_range, bounds_in_view, landmark_reach};

mod coverage;
use coverage::FacilityCoverage;

type RegionKey = (i32, i32);

struct Prepared {
    key: RegionKey,
    facilities: RegionalFacilities,
    bounds: Option<[f32; 4]>,
    coverage: Option<FacilityCoverage>,
    published: bool,
    proposed_roadside: Option<RoadsideSite>,
}

struct Working {
    key: RegionKey,
    planner: RegionalFacilityPlanner,
}

pub struct FacilityStream<const ACTIVE: usize> {
    world: World,
    seed: u64,
    prepared: [Option<Prepared>; ACTIVE],
    working: Option<Working>,
    detached: Option<RegionKey>,
    revision: u32,
    quarry_revision: u32,
}

pub struct FacilityRequest {
    world: World,
    seed: u64,
    key: RegionKey,
    revision: u32,
    road: RoadNetwork,
    towns: [Option<Settlement>; 2],
}

pub struct FacilityResult {
    key: RegionKey,
    revision: u32,
    facilities: RegionalFacilities,
    proposed_roadside: Option<RoadsideSite>,
}

#[derive(Clone, Copy)]
struct SampleSite<'a> {
    bounds: [f32; 4],
    coverage: Option<FacilityCoverage>,
    facilities: &'a RegionalFacilities,
}

pub struct FacilitySampler<'a, const ACTIVE: usize> {
    sites: [Option<SampleSite<'a>>; ACTIVE],
    count: usize,
    standalone_solar: Option<SolarSite>,
}

impl<const ACTIVE: usize> FacilitySampler<'_, ACTIVE> {
    pub fn overlay(&self, x: f32, z: f32, raw: Sample, developed: Sample) -> Sample {
        if let Some(site) = &self.standalone_solar {
            return site.cover(x, z, raw, developed);
        }
        for site in self.sites[..self.count].iter().flatten() {
            let [left, near, right, far] = site.bounds;
            if (left..=right).contains(&x) && (near..=far).contains(&z)
                && site.coverage.as_ref().is_none_or(|coverage| coverage.covers(x, z, site.bounds))
            {
                let applied = overlay_facilities(site.facilities, x, z, raw, developed);
                if applied != developed { return applied; }
            }
        }
        developed
    }
}

impl FacilityRequest {
    pub fn prepare(&self, mut service: impl FnMut()) -> FacilityResult {
        let towns = self.towns.each_ref().map(Option::as_ref);
        let facilities = RegionalFacilities::prepare_cooperatively(
            &self.world, self.seed, &self.road, towns, &mut service,
        );
        let proposed_roadside = prepare_roadside(&self.world, self.seed, self.key, &self.road, towns, &facilities, &mut service);
        FacilityResult { key: self.key, revision: self.revision, facilities, proposed_roadside }
    }
}

impl<const ACTIVE: usize> FacilityStream<ACTIVE> {
    pub const fn new(world: World, seed: u64) -> Self {
        Self { world, seed, prepared: [const { None }; ACTIVE], working: None, detached: None, revision: 0, quarry_revision: 0 }
    }

    pub fn reset(&mut self, world: World, seed: u64) {
        self.world = world;
        self.seed = seed;
        self.clear();
    }

    /// A complete plan also reserves its footprints before it is visible.
    pub fn planned(&self, key: (i32, i32)) -> Option<&RegionalFacilities> {
        self.prepared.iter().flatten().find(|record| record.key == key)
            .map(|record| &record.facilities)
    }

    #[inline(never)]
    pub fn preflight(&mut self, land: &LandStream<ACTIVE>) {
        self.clear();
        for (slot, (key, district)) in self.prepared.iter_mut().zip(land.districts()) {
            let Some(road) = district.road() else { continue };
            let facilities = RegionalFacilities::prepare(&self.world, self.seed, road, towns(district));
            let proposed_roadside = prepare_roadside(&self.world, self.seed, key, road, towns(district), &facilities, &mut || {});
            let bounds = facilities.bounds();
            let coverage = FacilityCoverage::new(bounds, &facilities);
            *slot = Some(Prepared { key, bounds, coverage, facilities, published: true, proposed_roadside });
        }
    }

    fn clear(&mut self) {
        self.working = None;
        self.detached = None;
        self.revision = self.revision.wrapping_add(1);
        self.quarry_revision = self.quarry_revision.wrapping_add(1);
        for record in &mut self.prepared {
            *record = None;
        }
    }

    /// Queue an independent survey only after the previous worker reply is collected.
    pub fn request(&mut self, land: &LandStream<ACTIVE>, camera: Camera, worker_ready: bool) -> Option<FacilityRequest> {
        self.refresh(land, camera);
        if !worker_ready || self.detached.is_some() || self.working.is_some() { return None; }
        let (key, district) = self.nearest_missing(land, camera)?;
        let road = district.road()?.clone();
        self.detached = Some(key);
        Some(FacilityRequest {
            world: self.world, seed: self.seed, key, revision: self.revision, road,
            towns: towns(district).map(|town| town.cloned()),
        })
    }

    /// Obsolete replies cannot modify a fresh flight or a district that has been evicted.
    pub fn commit(&mut self, result: FacilityResult, land: &LandStream<ACTIVE>, camera: Camera) -> bool {
        if result.revision != self.revision || self.detached != Some(result.key) { return false; }
        self.detached = None;
        self.refresh(land, camera);
        land.districts().any(|(key, _)| key == result.key)
            && self.retain(result.key, result.facilities, result.proposed_roadside, land, camera)
    }

    /// Advance at most one facility phase after land has had time to prepare.
    pub fn service(&mut self, land: &LandStream<ACTIVE>, camera: Camera) {
        self.refresh(land, camera);
        if self.detached.is_some() { return; }
        if self.working.as_ref().is_some_and(|working| !land.districts().any(|(key, _)| key == working.key)) {
            self.working = None;
        }
        if self.working.is_none() {
            let Some((key, _)) = self.nearest_missing(land, camera) else { return };
            self.working = Some(Working { key, planner: RegionalFacilityPlanner::new(self.world, self.seed) });
        }
        let working = self.working.as_mut().unwrap();
        let (_, district) = land.districts().find(|(key, _)| *key == working.key).unwrap();
        let Some(road) = district.road() else { return };
        if working.planner.step(road, towns(district)) == FacilityPhase::Ready {
            let working = self.working.take().unwrap();
            let facilities = working.planner.finish().unwrap();
            let proposed_roadside = prepare_roadside(&self.world, self.seed, working.key, road, towns(district), &facilities, &mut || {});
            self.retain(working.key, facilities, proposed_roadside, land, camera);
        }
    }

    fn refresh(&mut self, land: &LandStream<ACTIVE>, camera: Camera) {
        for slot in &mut self.prepared {
            let Some(record) = slot else { continue };
            if !land.districts().any(|(key, _)| key == record.key) {
                *slot = None;
            } else if !record.published && land.district_published(record.key)
                && !record.bounds.is_some_and(|area| bounds_in_view(camera, area))
            {
                record.published = true;
            }
        }

    }

    fn nearest_missing<'a>(&self, land: &'a LandStream<ACTIVE>, camera: Camera) -> Option<(RegionKey, &'a DistrictRegion)> {
        land.districts().filter(|(key, _)| self.planned(*key).is_none()).min_by(|(a, _), (b, _)| {
            distance_to_region(*a, camera).total_cmp(&distance_to_region(*b, camera))
        })
    }

    fn retain(&mut self, key: RegionKey, facilities: RegionalFacilities, proposed_roadside: Option<RoadsideSite>, land: &LandStream<ACTIVE>, camera: Camera) -> bool {
        let bounds = facilities.bounds();
        let published = land.district_published(key)
            && !bounds.is_some_and(|area| bounds_in_view(camera, area));
        if let Some(slot) = self.prepared.iter_mut().find(|slot| slot.is_none()) {
            let coverage = FacilityCoverage::new(bounds, &facilities);
            *slot = Some(Prepared { key, facilities, bounds, coverage, published, proposed_roadside });
            true
        } else {
            false
        }
    }

    /// Publish only after other retained layers agree that the proposed
    /// roadside footprint is clear. Cold preparation can publish in view once.
    pub fn publish_roadside_sites(
        &mut self, camera: Camera, before_first_frame: bool,
        mut reserve: impl FnMut([f32; 4]) -> RoadsideReservation,
    ) {
        if !self.prepared.iter().flatten().any(|record| record.proposed_roadside.is_some()) {
            return;
        }
        let existing = self.prepared.each_ref().map(|record| {
            record.as_ref().and_then(|record| record.bounds.map(|bounds| (record.key, bounds)))
        });
        for record in self.prepared.iter_mut().flatten() {
            let Some(site) = record.proposed_roadside.as_ref().filter(|_| record.published) else { continue };
            let bounds = site.bounds();
            let in_view = if matches!(site, RoadsideSite::WaterTower(_)) {
                bounds_in_range(camera, bounds, landmark_reach(camera.altitude))
            } else {
                bounds_in_view(camera, bounds)
            };
            if !before_first_frame {
                let train_near = matches!(site, RoadsideSite::Quarry { local_transfer: true, .. })
                    && record.facilities.rail().is_some_and(|rail| train_route_near(camera, rail.bounds()));
                if in_view || train_near {
                    continue;
                }
            }
            let blocked = existing.iter().flatten().any(|&(key, area)| key != record.key && overlaps(area, bounds));
            match if blocked { RoadsideReservation::Reserved } else { reserve(bounds) } {
                RoadsideReservation::Pending => {}
                RoadsideReservation::Reserved => record.proposed_roadside = None,
                RoadsideReservation::Clear => {
                    if matches!(site, RoadsideSite::Quarry { .. }) {
                        self.quarry_revision = self.quarry_revision.wrapping_add(1);
                    }
                    record.facilities.install_roadside(record.proposed_roadside.take().unwrap());
                    record.bounds = record.facilities.bounds();
                    record.coverage = FacilityCoverage::new(record.bounds, &record.facilities);
                }
            }
        }
    }

    pub fn quarries(&self) -> impl Iterator<Item = &Quarry> {
        self.prepared.iter().flatten().filter_map(|record| record.facilities.quarry())
    }

    pub(crate) fn quarry_revision(&self) -> u32 {
        self.quarry_revision
    }

    pub fn solar_sites(&self) -> impl Iterator<Item = &SolarSite> {
        self.prepared.iter().flatten().filter_map(|record| record.facilities.solar_site())
    }

    pub fn quaysides(&self) -> impl Iterator<Item = &Quayside> {
        self.prepared.iter().flatten().filter_map(|record| record.facilities.quayside())
    }

    pub fn water_towers(&self) -> impl Iterator<Item = &WaterTower> {
        self.prepared.iter().flatten().filter_map(|record| record.facilities.water_tower())
    }

    fn roadside_sites(&self) -> impl Iterator<Item = &RoadsideSite> {
        self.prepared.iter().flatten().filter_map(|record| record.facilities.roadside())
    }

    pub fn reserves_roadside_site(&self, bounds: [f32; 4]) -> bool {
        self.roadside_sites().any(|site| overlaps(site.bounds(), bounds))
    }

    pub fn reserves_roadside_river(&self, river: &RiverDistrict) -> bool {
        self.roadside_sites().any(|site| river.intersects_scenery(site.bounds()))
    }

    pub fn has_visible_facilities(&self, camera: Camera) -> bool {
        self.prepared.iter().flatten().any(|record| {
            record.published && record.bounds.is_some_and(|bounds| bounds_in_view(camera, bounds))
        })
    }

    /// Keep the resident bounds and any standalone solar layout close to the
    /// terrain sampler. The larger facility layouts stay in external memory.
    pub fn sampler(&self) -> FacilitySampler<'_, ACTIVE> {
        let mut sampler = FacilitySampler { sites: [None; ACTIVE], count: 0, standalone_solar: None };
        for record in self.prepared.iter().flatten().filter(|record| record.published) {
            if let Some(bounds) = record.bounds {
                sampler.sites[sampler.count] = Some(SampleSite {
                    bounds,
                    coverage: record.coverage,
                    facilities: &record.facilities,
                });
                sampler.count += 1;
            }
        }
        if sampler.count == 1 {
            let facility = sampler.sites[0].unwrap().facilities;
            if !facility.has_primary() {
                sampler.standalone_solar = facility.solar_site().copied();
            }
        }
        sampler
    }

    pub fn overlay(&self, x: f32, z: f32, raw: Sample, developed: Sample) -> Sample {
        for record in self.prepared.iter().flatten() {
            if !record.published {
                continue;
            }
            let Some([left, near, right, far]) = record.bounds else { continue };
            if (left..=right).contains(&x) && (near..=far).contains(&z) {
                let applied = overlay_facilities(&record.facilities, x, z, raw, developed);
                if applied != developed {
                    return applied;
                }
            }
        }
        developed
    }

    pub fn rails(&self, camera: Camera, reach: f32) -> impl Iterator<Item = &RailLine> {
        self.prepared.iter().flatten().filter_map(move |resident| {
            let line = resident.published.then(|| resident.facilities.rail()).flatten()?;
            let [left, near, right, far] = line.bounds();
            ((left - reach..=right + reach).contains(&camera.x)
                && (near - reach..=far + reach).contains(&camera.z)
                && bounds_in_view(camera, line.bounds())).then_some(line)
        })
    }

    pub fn retains_landmark(&self, form: &CoastalLandform) -> bool {
        self.prepared.iter().flatten().filter(|region| region.published)
            .flat_map(|region| region.facilities.ports()).all(|port| form.clears_port(port))
    }

    pub fn retains_lighthouse(&self, light: &Lighthouse) -> bool {
        self.prepared.iter().flatten().filter(|region| region.published)
            .flat_map(|region| region.facilities.ports()).all(|port| light.clears_port(port))
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        let water_tower = self.water_towers().map(|tower| tower.obstruction(x, z)).fold(0.0, f32::max);
        if !self.prepared.iter().flatten().any(|region| {
            region.published && region.bounds.is_some_and(|[left, near, right, far]| {
                (left..=right).contains(&x) && (near..=far).contains(&z)
            })
        }) {
            return water_tower;
        }
        let raw = self.world.sample(x, z);
        let placed = self.overlay(x, z, raw, raw);
        if placed.elevation > raw.elevation + 0.5 {
            water_tower.max(placed.elevation + 1.5)
        } else {
            water_tower
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoadsideReservation {
    Pending,
    Reserved,
    Clear,
}

/// Consult every nearby prepared network. The approved junction to the parent
/// road was already proved by the roadside survey.
pub fn roadside_reservation<const LAND: usize, const RIVERS: usize, const ROUTES: usize, const HIGHWAYS: usize>(
    land: &LandStream<LAND>, rivers: &RiverStream<RIVERS>, routes: &LocalRouteStream<ROUTES>,
    highways: &HighwayStream<HIGHWAYS>, natural: &NaturalStream, bounds: [f32; 4],
) -> RoadsideReservation {
    if natural.approved_bounds().any(|area| overlaps(bounds, area)) {
        return RoadsideReservation::Reserved;
    }
    let [left, near, right, far] = bounds;
    let tile = |value: f32| {
        let scaled = value / ROAD_REGION_SIZE as f32;
        let index = scaled as i32;
        index - i32::from(scaled < index as f32)
    };
    let mut pending = false;
    for z in tile(near - 128.0)..=tile(far + 128.0) {
        for x in tile(left - 128.0)..=tile(right + 128.0) {
            let key = (x, z);
            match rivers.planned(key) {
                Some(Some(river)) if river.intersects_scenery(bounds) => return RoadsideReservation::Reserved,
                Some(_) => {}
                None => pending = true,
            }
            match land.planned(key) {
                Some(Some(_)) => match routes.planned(key) {
                    Some(Some(route)) if route.conflicts_with(|area| overlaps(area, bounds)) => {
                        return RoadsideReservation::Reserved;
                    }
                    Some(_) => {}
                    None => pending = true,
                },
                Some(None) => {}
                None => pending = true,
            }
        }
    }
    match highways.reservation(land, bounds) {
        HighwayReservation::Blocked => RoadsideReservation::Reserved,
        HighwayReservation::Pending => RoadsideReservation::Pending,
        HighwayReservation::Clear if pending => RoadsideReservation::Pending,
        HighwayReservation::Clear => RoadsideReservation::Clear,
    }
}

fn prepare_roadside(
    world: &World, seed: u64, key: RegionKey, road: &RoadNetwork, towns: [Option<&Settlement>; 2],
    facilities: &RegionalFacilities, pulse: &mut impl FnMut(),
) -> Option<RoadsideSite> {
    let bounds = facilities.bounds();
    let primary = |x, z| {
        bounds.is_some_and(|[left, near, right, far]| {
            (left..=right).contains(&x) && (near..=far).contains(&z)
                && facilities.sample(x, z, world.sample(x, z)).is_some()
        })
    };
    let blocked = |x, z| {
        towns.into_iter().flatten().any(|town| town.sample(x, z).is_some()) || primary(x, z)
    };
    Quarry::generate(world, seed, [key.0, key.1], road, blocked, &mut *pulse)
        .map(|site| facilities.roadside_quarry(site, road))
        .or_else(|| {
            let rail = facilities.rail()?;
            if rail.service != crate::rail::RailService::Freight
                || rail.stations().iter().all(|station| station.anchor.kind.is_port())
            {
                return None;
            }
            let quarry = Quarry::gravel_pit(world, seed, [key.0, key.1], road, blocked, &mut *pulse)?;
            let site = facilities.roadside_quarry(quarry, road);
            matches!(site, RoadsideSite::Quarry { local_transfer: true, .. }).then_some(site)
        })
        .or_else(|| {
            SolarSite::generate(world, seed, key, road, blocked, &mut *pulse).map(RoadsideSite::Solar)
        })
        .or_else(|| facilities.ports().find_map(|port| {
            Quayside::plan(world, seed, port, |x, z| {
                let ground = world.sample(x, z);
                road.proximity(x, z).is_some()
                    || towns.into_iter().flatten().any(|town| town.sample(x, z).is_some())
                    || facilities.airfield().is_some_and(|field| field.sample(x, z).is_some())
                    || facilities.rail().is_some_and(|line| line.sample(x, z).is_some())
                    || facilities.ports().any(|other| other != port && other.sample(x, z, ground).is_some())
            }, &mut *pulse).map(RoadsideSite::Quayside)
        }))
        .or_else(|| {
            WaterTower::plan(world, seed, [key.0, key.1], road, towns, primary, &mut *pulse)
                .map(RoadsideSite::WaterTower)
        })
}

fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

fn towns(region: &DistrictRegion) -> [Option<&crate::settlement::Settlement>; 2] {
    [region.town(Terminus::Start), region.town(Terminus::End)]
}

fn distance_to_region((x, z): RegionKey, camera: Camera) -> f32 {
    let side = ROAD_REGION_SIZE as f32;
    let dx = (x as f32 + 0.5) * side - camera.x;
    let dz = (z as f32 + 0.5) * side - camera.z;
    dx * dx + dz * dz
}

#[cfg(test)]
mod quarry_tests {
    use super::*;
    use crate::local_routes::route_reservation;
    use crate::natural_stream::{highway_reservation_with_nature, scenery_reservation_with_highways, NaturalBudget};
    use crate::watershed::WatershedWork;

    #[test]
    fn dry_gravel_pit_supplies_a_retained_freight_rail_to_the_cargo_dock() {
        use crate::ports::PortKind;
        use crate::rail::{RailMaterial, RailService};
        use crate::train_art::{FreightRoute, Train};
        use crate::world::Surface;

        let seed = 518;
        let key = (5, 5);
        let world = World::new(seed);
        let view = Camera { x: 5516.0, z: 5570.0, altitude: 49.0, heading: 0.53, pitch: 0.32, bank: 0.0 };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(view, false));
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        let owner = facilities.planned(key).unwrap();
        let base = owner.rail().unwrap();
        assert_eq!(base.service, RailService::Freight);
        assert_ne!(base.freight_route, FreightRoute::Aggregate);
        let retained_track = base.polyline().to_vec();
        let proposal = facilities.prepared.iter().flatten().find(|record| record.key == key)
            .and_then(|record| record.proposed_roadside.as_ref()).unwrap();
        let RoadsideSite::Quarry { site, local_transfer: true } = proposal else {
            panic!("no road-connected gravel pit");
        };
        let [jx, _, jz] = site.junction();
        let [x, floor, z] = site.yard();
        let bounds = site.bounds();
        assert_eq!([jx, jz], [5572.0, 5632.0]);
        assert!(world.elevation(x, z) > floor + 2.5 && floor > 4.8);
        assert!(owner.quarry().is_none());

        let mut rivers = RiverStream::<3>::new(world, seed);
        let mut routes = LocalRouteStream::<8>::new(world, seed);
        let mut highways = HighwayStream::<4>::new(world, seed);
        let mut nature = NaturalStream::new(world, seed);
        rivers.preflight(&land, &facilities, view, &mut WatershedWork::new());
        routes.preflight(&land, view, |key, route| route_reservation(&world, &facilities, &rivers, key, route));
        highways.preflight_defer_coast(&land, view, |pair, link| {
            highway_reservation_with_nature(&land, &facilities, &rivers, &routes, &nature, pair, link)
        }, || {});
        nature.preflight(view, NaturalBudget { alpine_tiles: 9, dry_tiles: 9 }, |bounds| {
            scenery_reservation_with_highways(&land, &facilities, &rivers, &routes, &highways, bounds)
        });
        let reserve = |bounds| roadside_reservation(&land, &rivers, &routes, &highways, &nature, bounds);
        assert_eq!(reserve(bounds), RoadsideReservation::Clear);
        facilities.publish_roadside_sites(view, true, reserve);

        let owner = facilities.planned(key).unwrap();
        assert_eq!(owner.quarry().unwrap().bounds(), bounds);
        let line = owner.rail().unwrap();
        assert_eq!(line.polyline(), retained_track.as_slice());
        assert_eq!(line.freight_route, FreightRoute::Aggregate);
        let road = land.planned(key).flatten().unwrap().road().unwrap();
        let cargo = [Terminus::Start, Terminus::End].into_iter()
            .find(|end| owner.port(*end).is_some_and(|port| port.kind() == PortKind::Freight)).unwrap();
        let (x, z) = owner.port_transfer(cargo, road).unwrap();
        assert_eq!(line.sample(x, z).unwrap().material, RailMaterial::StationAccess);
        let junction = world.sample(jx, jz);
        let pavement = land.overlay(jx, jz, junction);
        assert_eq!(pavement.surface, Surface::Pavement);
        assert_eq!(facilities.sampler().overlay(jx, jz, junction, pavement), pavement);
        let pit = owner.quarry().unwrap();
        let [x, _, z] = pit.yard();
        let yard = world.sample(x, z);
        let sampler = facilities.sampler();
        assert!((sampler.overlay(x, z, yard, land.overlay(x, z, yard)).elevation - floor).abs() < 0.01);
        let [ax, az] = pit.axis();
        let mut materials = [0, 0];
        for forward in 0..48 {
            for across in -24_i32..=24 {
                let sx = x + ax * forward as f32 - az * across as f32;
                let sz = z + az * forward as f32 + ax * across as f32;
                let terrain = world.sample(sx, sz);
                let placed = sampler.overlay(sx, sz, terrain, land.overlay(sx, sz, terrain));
                let stone = match placed.surface {
                    Surface::ScreeCoarse => 0,
                    Surface::ScreeFine => 1,
                    _ => continue,
                };
                assert!(across.abs() >= 7 && forward > 11);
                assert!(placed.elevation <= terrain.elevation && placed.elevation >= floor);
                materials[stone] += 1;
            }
        }
        assert!(materials.into_iter().all(|area| area > 30));
        assert!((0..12_000).step_by(17).any(|tick| Train::at(line, seed, tick, view, (128, 128)).is_some()));
    }

    #[test]
    fn water_tower_uses_the_existing_off_screen_authorities() {
        let seed = 31;
        let key = (1, 1);
        let world = World::new(seed);
        let view = Camera { x: 1382.8467, z: 1687.6177, altitude: 52.63, heading: 0.0, pitch: 0.18, bank: 0.0 };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(view, false));
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        let record = facilities.prepared.iter().flatten().find(|record| record.key == key).unwrap();
        let Some(RoadsideSite::WaterTower(tower)) = record.proposed_roadside else {
            panic!("expected water tower proposal at the river town");
        };
        let bounds = tower.bounds();
        assert!(bounds_in_view(view, bounds));
        let mut rivers = RiverStream::<3>::new(world, seed);
        let mut routes = LocalRouteStream::<8>::new(world, seed);
        let mut highways = HighwayStream::<4>::new(world, seed);
        let mut nature = NaturalStream::new(world, seed);
        let reservation = roadside_reservation(&land, &rivers, &routes, &highways, &nature, bounds);
        assert_eq!(reservation, RoadsideReservation::Pending);
        facilities.publish_roadside_sites(view, true, |_| RoadsideReservation::Pending);
        assert!(facilities.planned(key).unwrap().water_tower().is_none());

        rivers.preflight(&land, &facilities, view, &mut WatershedWork::new());
        routes.preflight(&land, view, |key, route| route_reservation(&world, &facilities, &rivers, key, route));
        highways.preflight_defer_coast(&land, view, |pair, link| {
            highway_reservation_with_nature(&land, &facilities, &rivers, &routes, &nature, pair, link)
        }, || {});
        nature.preflight(view, NaturalBudget { alpine_tiles: 9, dry_tiles: 9 }, |bounds| {
            scenery_reservation_with_highways(&land, &facilities, &rivers, &routes, &highways, bounds)
        });
        let reserve = |bounds| roadside_reservation(&land, &rivers, &routes, &highways, &nature, bounds);
        assert_eq!(reserve(bounds), RoadsideReservation::Clear);

        facilities.publish_roadside_sites(view, false, reserve);
        assert!(facilities.planned(key).unwrap().water_tower().is_none());
        let away = Camera { heading: core::f32::consts::PI, ..view };
        assert!(!bounds_in_view(away, bounds));
        facilities.publish_roadside_sites(away, false, reserve);
        let published = facilities.planned(key).unwrap().water_tower().unwrap();
        assert_eq!(published, &tower);
        assert!(facilities.reserves_roadside_site(bounds));
        let at = tower.access_road()[2];
        let raw = world.sample(at.x, at.z);
        assert_eq!(facilities.sampler().overlay(at.x, at.z, raw, raw).surface, crate::world::Surface::Gravel);
        let tree = Sample { surface: crate::world::Surface::TreeCanopy, elevation: raw.elevation + 7.0 };
        let grounded = facilities.sampler().overlay(at.x, at.z, raw, tree);
        assert_eq!(grounded.elevation, raw.elevation);
        assert_eq!(grounded.surface, crate::world::Surface::Gravel);
        assert!(facilities.obstruction(at.x, at.z) > tower.mesh_bounds()[1][1]);
    }

    #[test]
    fn quayside_keeps_its_apron_and_publishes_through_startup_authorities() {
        let seed = 0;
        let world = World::new(seed);
        let key = (3, -5);
        let view = Camera {
            x: 3283.7344,
            z: -4772.783,
            altitude: 55.0,
            heading: 0.53620315,
            pitch: 0.41822433,
            bank: 0.025,
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(view, false));
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        let record = facilities.prepared.iter().flatten().find(|record| record.key == key).unwrap();
        let Some(RoadsideSite::Quayside(quay)) = record.proposed_roadside else {
            panic!("expected shoreline promenade proposal");
        };
        let bounds = quay.bounds();
        assert!(bounds_in_view(view, bounds));
        let [x, z] = quay.contact();
        let raw = world.sample(x, z);
        let apron = facilities.sampler().overlay(x, z, raw, raw);
        assert_ne!(apron, raw);
        assert!(matches!(apron.surface, crate::world::Surface::Pavement | crate::world::Surface::Gravel));

        let mut rivers = RiverStream::<3>::new(world, seed);
        let mut routes = LocalRouteStream::<8>::new(world, seed);
        let mut highways = HighwayStream::<4>::new(world, seed);
        let mut nature = NaturalStream::new(world, seed);
        facilities.publish_roadside_sites(view, true, |bounds| {
            roadside_reservation(&land, &rivers, &routes, &highways, &nature, bounds)
        });
        assert!(facilities.planned(key).unwrap().quayside().is_none());

        rivers.preflight(&land, &facilities, view, &mut WatershedWork::new());
        routes.preflight(&land, view, |key, route| {
            route_reservation(&world, &facilities, &rivers, key, route)
        });
        highways.preflight_defer_coast(&land, view, |pair, link| {
            highway_reservation_with_nature(&land, &facilities, &rivers, &routes, &nature, pair, link)
        }, || {});
        nature.preflight(view, NaturalBudget { alpine_tiles: 9, dry_tiles: 9 }, |bounds| {
            scenery_reservation_with_highways(&land, &facilities, &rivers, &routes, &highways, bounds)
        });
        let reserve = |bounds| roadside_reservation(&land, &rivers, &routes, &highways, &nature, bounds);
        assert_eq!(reserve(bounds), RoadsideReservation::Clear);
        facilities.publish_roadside_sites(view, false, reserve);
        assert!(facilities.planned(key).unwrap().quayside().is_none());
        facilities.publish_roadside_sites(view, true, reserve);
        let published = facilities.planned(key).unwrap().quayside().unwrap();
        assert_eq!(published.bounds(), bounds);
        assert_eq!(published.shops(), 4);
        assert!(facilities.reserves_roadside_site(bounds));
        assert_eq!(facilities.sampler().overlay(x, z, raw, raw), apron);
    }

    #[test]
    fn solar_site_publishes_from_ordinary_startup_and_reserves_its_footprint() {
        let seed = 2;
        let world = World::new(seed);
        let key = (1, 0);
        let view = Camera {
            x: 1596.0,
            z: 513.5032,
            altitude: 80.33295,
            heading: core::f32::consts::PI,
            pitch: 0.53303415,
            bank: 0.035,
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(view, false));
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        let proposal = facilities.prepared.iter().flatten()
            .find(|record| record.key == key)
            .and_then(|record| record.proposed_roadside).unwrap();
        let RoadsideSite::Solar(site) = proposal else { panic!("expected solar proposal"); };
        let bounds = site.bounds();
        assert!(bounds_in_view(view,bounds));

        let (cols, rows) = site.rows();
        let [gx, _, gz] = site.gate();
        let [ax, az] = site.direction();
        let along = 13.0 + 12.0 * (f32::from(rows) * 0.5 - 0.5);
        let across = 22.0 * (0.5 - f32::from(cols) * 0.5);
        let [x, z] = [gx + ax * along + az * across, gz + az * along - ax * across];
        let ground = world.sample(x, z);
        assert_eq!(facilities.sampler().overlay(x, z, ground, ground), ground);

        let mut rivers = RiverStream::<3>::new(world, seed);
        let mut routes = LocalRouteStream::<8>::new(world, seed);
        let mut highways = HighwayStream::<4>::new(world, seed);
        let mut nature = NaturalStream::new(world, seed);
        facilities.publish_roadside_sites(view, true, |bounds| {
            roadside_reservation(&land, &rivers, &routes, &highways, &nature, bounds)
        });
        assert!(facilities.planned(key).unwrap().solar_site().is_none());

        rivers.preflight(&land, &facilities, view, &mut WatershedWork::new());
        routes.preflight(&land, view, |key, route| {
            route_reservation(&world, &facilities, &rivers, key, route)
        });
        highways.preflight_defer_coast(&land, view, |pair, link| {
            highway_reservation_with_nature(&land, &facilities, &rivers, &routes, &nature, pair, link)
        }, || {});
        nature.preflight(view, NaturalBudget { alpine_tiles: 9, dry_tiles: 9 }, |bounds| {
            scenery_reservation_with_highways(&land, &facilities, &rivers, &routes, &highways, bounds)
        });
        let reserve = |bounds| roadside_reservation(&land, &rivers, &routes, &highways, &nature, bounds);
        assert_eq!(reserve(bounds), RoadsideReservation::Clear);
        facilities.publish_roadside_sites(view, false, reserve);
        assert!(facilities.planned(key).unwrap().solar_site().is_none());
        facilities.publish_roadside_sites(view, true, reserve);
        let published = facilities.planned(key).unwrap().solar_site().unwrap();
        assert_eq!(published.bounds(), bounds);
        let covered = facilities.sampler().overlay(x, z, ground, ground);
        assert_eq!(covered.surface, crate::world::Surface::SolarGlass);
        assert!(facilities.obstruction(x, z) > covered.elevation);
        assert!(facilities.reserves_roadside_site(bounds));

        let [x, _, z] = site.trunk();
        let ground = world.sample(x, z);
        let pavement = land.overlay(x, z, ground);
        assert_eq!(pavement.surface, crate::world::Surface::Pavement);
        assert_eq!(facilities.sampler().overlay(x, z, ground, pavement), pavement);

        let sampler = facilities.sampler();
        assert!(sampler.standalone_solar.is_some());
        let [left, near, right, far] = bounds;
        let mut changed = 0;
        for row in 0..=256 {
            let z = near + (far - near) * row as f32 / 256.0;
            for column in 0..=256 {
                let x = left + (right - left) * column as f32 / 256.0;
                let raw = world.sample(x, z);
                let developed = land.overlay(x, z, raw);
                let reference = facilities.overlay(x, z, raw, developed);
                let fast = sampler.overlay(x, z, raw, developed);
                assert_eq!(fast.elevation.to_bits(), reference.elevation.to_bits());
                assert_eq!(fast.surface, reference.surface);
                changed += usize::from(fast != developed);
            }
        }
        assert!(changed > 10_000);
    }

    #[test]
    fn streamed_quarry_waits_for_complete_authority_before_changing_the_view() {
        let seed = 3;
        let world = World::new(seed);
        let key = (-4, 4);
        let view = Camera {
            x: -3666.8782,
            z: 4594.154,
            altitude: 67.73687,
            heading: 2.325461,
            pitch: 0.4636476,
            bank: 0.035,
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(view, false));
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        let unaccepted = facilities.quarry_revision();
        let proposal = facilities.prepared.iter().flatten().find(|record| record.key == key)
            .and_then(|record| record.proposed_roadside.as_ref()).unwrap();
        let bounds = proposal.bounds();
        assert!(bounds_in_view(view, bounds));
        assert!(facilities.quarries().next().is_none());

        let mut rivers = RiverStream::<3>::new(world, seed);
        let mut routes = LocalRouteStream::<8>::new(world, seed);
        let mut highways = HighwayStream::<4>::new(world, seed);
        let mut nature = NaturalStream::new(world, seed);
        assert_eq!(roadside_reservation(&land, &rivers, &routes, &highways, &nature, bounds), RoadsideReservation::Pending);
        facilities.publish_roadside_sites(view, true, |_| RoadsideReservation::Pending);
        assert!(facilities.quarries().next().is_none());
        assert_eq!(facilities.quarry_revision(), unaccepted);

        rivers.preflight(&land, &facilities, view, &mut WatershedWork::new());
        routes.preflight(&land, view, |key, route| route_reservation(&world, &facilities, &rivers, key, route));
        highways.preflight_defer_coast(&land, view, |pair, link| {
            highway_reservation_with_nature(&land, &facilities, &rivers, &routes, &nature, pair, link)
        }, || {});
        nature.preflight(view, NaturalBudget { alpine_tiles: 9, dry_tiles: 9 }, |bounds| {
            scenery_reservation_with_highways(&land, &facilities, &rivers, &routes, &highways, bounds)
        });
        let reserve = |bounds| roadside_reservation(&land, &rivers, &routes, &highways, &nature, bounds);
        assert_eq!(reserve(bounds), RoadsideReservation::Clear);
        facilities.publish_roadside_sites(view, false, reserve);
        assert!(facilities.quarries().next().is_none());
        assert_eq!(facilities.quarry_revision(), unaccepted);
        facilities.publish_roadside_sites(view, true, reserve);
        assert_eq!(facilities.quarry_revision(), unaccepted.wrapping_add(1));
        let quarry = facilities.planned(key).unwrap().quarry().unwrap();
        assert_eq!(quarry.bounds(), bounds);
        assert!(facilities.reserves_roadside_site(bounds));
        let [x, _, z] = quarry.junction();
        let ground = world.sample(x, z);
        let pavement = land.overlay(x, z, ground);
        assert_eq!(pavement.surface, crate::world::Surface::Pavement);
        assert_eq!(facilities.sampler().overlay(x, z, ground, pavement), pavement);
    }
}
