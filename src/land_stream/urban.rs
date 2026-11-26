//! Publication of independently surveyed urban additions.

use super::{Key, LandPaintSampler, LandStream, Resident, Window, CITY_HORIZON, REGION};
use crate::render::{terrain_detail_distance, Camera};
use crate::scenery::{cover_settlement, DistrictExpansion, DistrictExpansionRequest};
use crate::settlement::BoroughPaintCache;
use crate::world::{Sample, Surface, World};
use crate::world_mesh::bounds_in_range;

mod reservation;
pub use reservation::{UrbanAuthorities, UrbanReservations};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Ownership {
    world: World,
    key: Key,
    slot: usize,
    serial: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Claim {
    owner: Ownership,
    authority_revision: u32,
    ticket: u64,
}

pub struct UrbanExpansionJob {
    claim: Claim,
    request: DistrictExpansionRequest,
}

impl UrbanExpansionJob {
    pub fn region(&self) -> [i32; 2] {
        self.request.region()
    }

    pub fn region_bounds(&self) -> [f32; 4] {
        self.request.region_bounds()
    }

    pub fn authority_revision(&self) -> u32 {
        self.claim.authority_revision
    }

    pub fn prepare(
        self,
        reserved: impl Fn(f32, f32) -> bool,
        pulse: impl FnMut(),
    ) -> UrbanExpansionResult {
        UrbanExpansionResult {
            claim: self.claim,
            expansion: self.request.prepare(reserved, pulse),
            reservations: None,
        }
    }

    /// Accept only a complete reservation snapshot for this exact survey.
    pub fn prepare_reserved(
        &self,
        reservations: &UrbanReservations,
        pulse: impl FnMut(),
    ) -> Option<UrbanExpansionResult> {
        reservations.matches(self.region(), self.authority_revision()).then(|| UrbanExpansionResult {
            claim: self.claim,
            expansion: self.request.prepare_with_bounds(
                |x, z| reservations.blocks(x, z),
                |bounds| reservations.overlaps(bounds),
                |x, z| reservations.near_waterway(x, z),
                pulse,
            ),
            reservations: Some(reservations.clone()),
        })
    }
}

pub struct UrbanExpansionResult {
    claim: Claim,
    expansion: DistrictExpansion,
    reservations: Option<UrbanReservations>,
}

impl UrbanExpansionResult {
    pub fn region(&self) -> [i32; 2] {
        self.expansion.region()
    }
}

struct Pending {
    claim: Claim,
    completed: Option<UrbanExpansionResult>,
}

struct Published {
    owner: Ownership,
    expansion: DistrictExpansion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UrbanPublication {
    Idle,
    AwaitingSurvey,
    Deferred,
    Published,
    Stale,
}

/// One outstanding survey and one retained addition per occupied land slot.
/// The original `LandStream` remains the authority for all route surveys.
pub struct UrbanStream<const ACTIVE: usize = 8> {
    published: [Option<Published>; ACTIVE],
    pending: Option<Pending>,
    next_ticket: u64,
}

impl<const ACTIVE: usize> UrbanStream<ACTIVE> {
    pub const fn new() -> Self {
        Self {
            published: [const { None }; ACTIVE],
            pending: None,
            next_ticket: 0,
        }
    }

    /// Release an abandoned survey so the next offscreen owner can be prepared.
    pub fn abort(&mut self) {
        self.pending = None;
    }

    pub fn reset(&mut self) {
        self.abort();
        for slot in &mut self.published {
            *slot = None;
        }
    }

    pub fn pending_region(&self) -> Option<[i32; 2]> {
        self.pending
            .as_ref()
            .map(|pending| [pending.claim.owner.key.x, pending.claim.owner.key.z])
    }

    /// Prefer the nearest surveyed district whose detailed geometry is still out of view.
    /// The publication check handles a visible district if it was the only candidate.
    pub fn upcoming_region(
        &self,
        land: &LandStream<ACTIVE>,
        camera: Camera,
        mut ready: impl FnMut([i32; 2]) -> bool,
    ) -> Option<[i32; 2]> {
        let view = Window::new(camera)?;
        if self.pending.is_some() {
            return None;
        }
        land.districts()
            .filter_map(|((x, z), district)| {
                let key = Key { x, z };
                let region = [x, z];
                let forward = view.intersects_width(view.projected(key), CITY_HORIZON, 0.82);
                (forward
                    && land.district_published((x, z))
                    && district.towns().next().is_some()
                    && self.approved(land, region).is_none()
                    && ready(region))
                    .then(|| (region, view.visible(key), view.distance_squared(key)))
            })
            .min_by(|(_, a_visible, a), (_, b_visible, b)| a_visible.cmp(b_visible).then(a.total_cmp(b)))
            .map(|(region, _, _)| region)
    }

    /// A caller claims a region once all its intersecting survey authorities are settled.
    pub fn claim(
        &mut self,
        land: &LandStream<ACTIVE>,
        region: [i32; 2],
        authority_revision: u32,
    ) -> Option<UrbanExpansionJob> {
        if self.pending.is_some()
            || self
                .approved(land, region)
                .is_some_and(|expansion| expansion.revision() == authority_revision)
        {
            return None;
        }
        let [x, z] = region;
        let (owner, resident) = land.owner(Key { x, z })?;
        let request = resident
            .region
            .expansion_request(land.world, authority_revision)?;
        self.next_ticket = self.next_ticket.wrapping_add(1);
        let claim = Claim {
            owner,
            authority_revision,
            ticket: self.next_ticket,
        };
        self.pending = Some(Pending {
            claim,
            completed: None,
        });
        Some(UrbanExpansionJob { claim, request })
    }

    /// Accept an exact job result. Use `None` for the camera during initial preflight.
    pub fn commit(
        &mut self,
        land: &LandStream<ACTIVE>,
        result: UrbanExpansionResult,
        authority_revision: u32,
        camera: Option<Camera>,
    ) -> UrbanPublication {
        self.stage_result(land, result, authority_revision, camera, None)
    }

    pub fn commit_reserved(
        &mut self,
        land: &LandStream<ACTIVE>,
        result: UrbanExpansionResult,
        current: &UrbanReservations,
        camera: Option<Camera>,
    ) -> UrbanPublication {
        let revision = result.claim.authority_revision;
        self.stage_result(land, result, revision, camera, Some(current))
    }

    fn stage_result(
        &mut self,
        land: &LandStream<ACTIVE>,
        result: UrbanExpansionResult,
        authority_revision: u32,
        camera: Option<Camera>,
        current: Option<&UrbanReservations>,
    ) -> UrbanPublication {
        let Some(pending) = self
            .pending
            .as_mut()
            .filter(|pending| pending.claim == result.claim)
        else {
            return UrbanPublication::Stale;
        };
        if pending.completed.is_some() {
            return UrbanPublication::Stale;
        }
        pending.completed = Some(result);
        self.publish_using(land, authority_revision, camera, current)
    }

    /// Publish a new district before its detailed geometry enters the view.
    /// Retain an existing district until both skyline extents are out of view.
    pub fn publish(
        &mut self,
        land: &LandStream<ACTIVE>,
        authority_revision: u32,
        camera: Option<Camera>,
    ) -> UrbanPublication {
        self.publish_using(land, authority_revision, camera, None)
    }

    pub fn publish_reserved(
        &mut self,
        land: &LandStream<ACTIVE>,
        authority_revision: u32,
        camera: Option<Camera>,
        current: &UrbanReservations,
    ) -> UrbanPublication {
        self.publish_using(land, authority_revision, camera, Some(current))
    }

    fn publish_using(
        &mut self,
        land: &LandStream<ACTIVE>,
        authority_revision: u32,
        camera: Option<Camera>,
        current: Option<&UrbanReservations>,
    ) -> UrbanPublication {
        let Some(pending) = self.pending.as_ref() else {
            return UrbanPublication::Idle;
        };
        let Claim {
            owner,
            authority_revision: surveyed_revision,
            ..
        } = pending.claim;
        if authority_revision != surveyed_revision
            || !land
                .owner(owner.key)
                .is_some_and(|(resident, _)| resident == owner)
        {
            self.abort();
            return UrbanPublication::Stale;
        }
        let Some(completed) = pending.completed.as_ref() else {
            return UrbanPublication::AwaitingSurvey;
        };
        let respected = match (&completed.reservations, current) {
            (Some(captured), Some(current)) => captured.protects(current),
            (None, None) => true,
            _ => false,
        };
        if !respected {
            self.abort();
            return UrbanPublication::Stale;
        }
        if let Some(view) = camera {
            let previous = self.published[owner.slot]
                .as_ref()
                .filter(|record| record.owner == owner)
                .and_then(|record| record.expansion.bounds());
            // A first skyline appearance fades in through the skyline renderer.
            // Replacing already visible geometry must still wait until it is out of view.
            let reach = if previous.is_some() { CITY_HORIZON } else { terrain_detail_distance(view.altitude) };
            if Window::new(view).is_none()
                || [previous, completed.expansion.bounds()]
                    .into_iter()
                    .flatten()
                    .any(|bounds| bounds_in_range(view, bounds, reach))
            {
                return UrbanPublication::Deferred;
            }
        }
        let expansion = self
            .pending
            .take()
            .and_then(|entry| entry.completed)
            .unwrap();
        self.published[owner.slot] = Some(Published { owner, expansion: expansion.expansion });
        UrbanPublication::Published
    }

    /// A committed view is retained until the same owner has an offscreen replacement.
    pub fn approved(
        &self,
        land: &LandStream<ACTIVE>,
        region: [i32; 2],
    ) -> Option<&DistrictExpansion> {
        let [x, z] = region;
        let (owner, _) = land.owner(Key { x, z })?;
        let record = self.published[owner.slot].as_ref()?;
        (record.owner == owner).then_some(&record.expansion)
    }
}

impl<const ACTIVE: usize> Default for UrbanStream<ACTIVE> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UrbanPaintSampler<'a, const ACTIVE: usize> {
    land: LandPaintSampler<'a, ACTIVE>,
    visible: [Option<([f32; 4], &'a DistrictExpansion)>; ACTIVE],
    lots: BoroughPaintCache,
}

impl<const ACTIVE: usize> UrbanStream<ACTIVE> {
    /// Precompute the visible additions once, leaving the main land sampler
    /// and its road cache in charge of established scenery.
    pub fn paint_sampler<'a>(
        &'a self,
        land: &'a LandStream<ACTIVE>,
        view: Camera,
    ) -> UrbanPaintSampler<'a, ACTIVE> {
        let reach = terrain_detail_distance(view.altitude);
        let mut visible = [None; ACTIVE];
        let entries = land.districts().filter_map(|((x, z), _)| {
            let expansion = self.approved(land, [x, z])?;
            let mut bounds = expansion.bounds()?;
            let [left, near] = [x as f32 * REGION, z as f32 * REGION];
            bounds[0] = bounds[0].max(left);
            bounds[1] = bounds[1].max(near);
            bounds[2] = bounds[2].min(left + REGION);
            bounds[3] = bounds[3].min(near + REGION);
            bounds_in_range(view, bounds, reach).then_some((bounds, expansion))
        });
        for (slot, entry) in visible.iter_mut().zip(entries) {
            *slot = Some(entry);
        }
        UrbanPaintSampler { land: land.paint_sampler(), visible, lots: BoroughPaintCache::new() }
    }

    pub fn obstruction(&self, land: &LandStream<ACTIVE>, x: f32, z: f32) -> f32 {
        let key = Key::at(x, z);
        self.approved(land, [key.x, key.z])
            .map_or(0.0, |expansion| expansion.obstruction(x, z))
    }

    pub fn skyline<'a>(&'a self, land: &'a LandStream<ACTIVE>) -> impl Iterator<Item = crate::skyline::SkylineTower> + 'a {
        land.districts()
            .filter_map(|((x, z), _)| self.approved(land, [x, z]))
            .flat_map(DistrictExpansion::skyline)
    }
}

impl<const ACTIVE: usize> UrbanPaintSampler<'_, ACTIVE> {
    pub fn has_expansion(&self) -> bool {
        self.visible.first().is_some_and(Option::is_some)
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let established = self.land.overlay(x, z, ground);
        if established != ground || matches!(ground.surface, Surface::Pavement | Surface::Gravel) {
            return established;
        }
        for &([left, near, right, far], expansion) in self.visible.iter().map_while(Option::as_ref) {
            if x >= left && x < right && z >= near && z < far {
                if let Some(detail) = expansion.sample_within_bounds(x, z, Some(&self.lots)) {
                    return cover_settlement(detail, established);
                }
            }
        }
        established
    }
}

impl<const ACTIVE: usize> LandStream<ACTIVE> {
    fn owner(&self, key: Key) -> Option<(Ownership, &Resident)> {
        let slot = usize::from(self.get(key));
        let resident = self
            .residents
            .get(slot)?
            .as_ref()
            .filter(|resident| resident.published)?;
        let owner = Ownership {
            world: self.world,
            key,
            slot,
            serial: resident.ownership,
        };
        Some((owner, resident))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roads::RoadNetwork;
    use crate::settlement::{Settlement, Terminus};
    use crate::world::{Sample, Surface};

    const WORLD: World = World::new(9);
    const REGION: [i32; 2] = [-6, -7];
    const REVISION: u32 = 17;

    fn prepare_owner(land: &mut LandStream<1>) {
        let [x, z] = REGION;
        let road = RoadNetwork::generate(&WORLD, x, z).unwrap();
        let towns =
            [Terminus::Start, Terminus::End].map(|end| Settlement::generate(&WORLD, &road, end));
        land.write_resident(0, Key { x, z }, road, towns, true);
    }

    fn land() -> LandStream<1> {
        let mut land = LandStream::new();
        land.reset(WORLD);
        prepare_owner(&mut land);
        land
    }

    fn look_at(bounds: [f32; 4]) -> Camera {
        let [left, near, right, _] = bounds;
        Camera {
            x: (left + right) * 0.5,
            z: near - 120.0,
            altitude: 52.0,
            heading: 0.0,
            pitch: 0.28,
            bank: 0.0,
        }
    }

    #[test]
    fn a_ready_approach_can_schedule_one_survey_at_a_time() {
        let land = land();
        let mut urban = UrbanStream::new();
        let district = land.planned((REGION[0], REGION[1])).flatten().unwrap();
        let view = look_at(district.bounds());
        assert_eq!(urban.upcoming_region(&land, view, |_| false), None);
        assert_eq!(urban.upcoming_region(&land, view, |_| true), Some(REGION));
        let job = urban.claim(&land, REGION, REVISION).unwrap();
        assert_eq!(urban.upcoming_region(&land, view, |_| true), None);
        let prepared = job.prepare(|_, _| false, || {});
        assert_eq!(urban.commit(&land, prepared, REVISION, None), UrbanPublication::Published);
        assert_eq!(urban.upcoming_region(&land, view, |_| true), None);
    }

    #[test]
    fn urban_job_stages_a_visible_result_until_the_view_turns_away() {
        let land = land();
        let mut urban = UrbanStream::new();
        let job = urban.claim(&land, REGION, REVISION).unwrap();
        assert!(urban.claim(&land, REGION, REVISION).is_none());
        assert_eq!(urban.pending_region(), Some(REGION));
        assert_eq!(
            urban.publish(&land, REVISION, None),
            UrbanPublication::AwaitingSurvey
        );
        let result = job.prepare(|_, _| false, || {});
        let bounds = result.expansion.bounds().unwrap();
        let visible = look_at(bounds);
        assert!(urban.approved(&land, REGION).is_none());
        assert_eq!(
            urban.commit(&land, result, REVISION, Some(visible)),
            UrbanPublication::Deferred
        );
        assert!(urban.approved(&land, REGION).is_none());
        assert_eq!(
            urban.publish(&land, REVISION, Some(visible)),
            UrbanPublication::Deferred
        );

        let distant = Camera {
            z: visible.z - 1200.0,
            ..visible
        };
        assert!(bounds_in_range(distant, bounds, CITY_HORIZON));
        assert!(!bounds_in_range(distant, bounds, terrain_detail_distance(distant.altitude)));
        assert_eq!(
            urban.publish(&land, REVISION, Some(distant)),
            UrbanPublication::Published
        );
        let addition = urban.approved(&land, REGION).unwrap();
        assert_eq!(addition.revision(), REVISION);
        let [left, near, right, far] = addition.bounds().unwrap();
        let [x, z] = REGION;
        let district = land.planned((x, z)).flatten().unwrap();
        let ground = Sample {
            elevation: 10.0,
            surface: Surface::Grass,
        };
        let mut displayed = 0;
        for pz in (near as i32..=far as i32).step_by(7) {
            for px in (left as i32..=right as i32).step_by(7) {
                let [px, pz] = [px as f32, pz as f32];
                let ordinary = land.overlay(px, pz, ground);
                assert_eq!(district.overlay(px, pz, ground), ordinary);
                let expanded = district.overlay_expansion(addition, px, pz, ground);
                if ordinary != ground {
                    assert_eq!(expanded, ordinary);
                } else {
                    displayed += usize::from(expanded != ordinary);
                }
            }
        }
        assert!(displayed > 100);
        assert!(urban.claim(&land, REGION, REVISION).is_none());
    }

    #[test]
    fn detached_mask_keeps_whole_occupied_cells_clear_and_draws_only_published_towns() {
        let land = land();
        let mut urban = UrbanStream::new();
        let job = urban.claim(&land, REGION, REVISION).unwrap();
        let [x, z, _, _] = job.region_bounds();
        let saved = [x + 481.0, z + 481.0, x + 482.0, z + 482.0];
        let mut mask = UrbanReservations::new(REGION, REVISION);
        mask.reserve_area(saved);
        assert!(!mask.advance(128, |_| false));
        assert!(job.prepare_reserved(&mask, || {}).is_none());
        assert!(mask.advance(usize::MAX, |_| false));
        assert!(mask.blocks(x + 480.0, z + 480.5));
        assert!(mask.blocks(x + 495.5, z + 495.5));
        assert!(!mask.blocks(x + 496.5, z + 495.5));
        let result = job.prepare_reserved(&mask, || {}).unwrap();
        let bounds = result.expansion.bounds().unwrap();
        let view = look_at(bounds);
        assert_eq!(urban.commit_reserved(&land, result, &mask, Some(view)), UrbanPublication::Deferred);
        let mut newer = mask.clone();
        newer.reserve_area([x + 512.0, z + 512.0, x + 520.0, z + 520.0]);
        assert_eq!(urban.publish_reserved(&land, REVISION, None, &newer), UrbanPublication::Stale);
        assert!(urban.approved(&land, REGION).is_none());
        let result = urban.claim(&land, REGION, REVISION).unwrap().prepare_reserved(&mask, || {}).unwrap();
        assert_eq!(urban.commit_reserved(&land, result, &mask, None), UrbanPublication::Published);
        let ground = Sample { elevation: 10.0, surface: Surface::Grass };
        let [left, near, right, far] = bounds;
        let mut new_samples = 0;
        let sampler = urban.paint_sampler(&land, view);
        let addition = urban.approved(&land, REGION).unwrap();
        let owner = land.planned((REGION[0], REGION[1])).flatten().unwrap();
        for z in (near as i32..far as i32).step_by(4) {
            for x in (left as i32..right as i32).step_by(4) {
                let (x, z) = (x as f32, z as f32);
                let old = land.overlay(x, z, ground);
                let new = sampler.overlay(x, z, ground);
                assert_eq!(new, owner.overlay_expansion(addition, x, z, ground));
                if old != ground || mask.blocks(x, z) {
                    assert_eq!(old, new);
                } else if new != old {
                    new_samples += 1;
                }
            }
        }
        assert!(new_samples > 50);
    }

    #[test]
    fn real_airport_and_grain_districts_keep_their_surveyed_footprints() {
        use crate::facility_stream::FacilityStream;
        use crate::highway_stream::HighwayStream;
        use crate::local_routes::{route_reservation, LocalRouteStream};
        use crate::natural_stream::NaturalStream;
        use crate::rail_stream::RailStream;
        use crate::river_stream::RiverStream;
        use crate::watershed::WatershedWork;

        for (seed, x, z) in [(9, -6, -7), (165, -7, -1)] {
            let world = World::new(seed);
            let mut land = LandStream::<8>::new();
            land.reset(world);
            let road = RoadNetwork::generate(&world, x, z).unwrap();
            let towns = [Terminus::Start, Terminus::End].map(|end| Settlement::generate(&world, &road, end));
            land.write_resident(0, Key { x, z }, road, towns, true);
            let district = land.planned((x, z)).flatten().unwrap();
            let view = look_at(district.bounds());
            let mut facilities = FacilityStream::new(world, seed);
            let mut rivers = RiverStream::<3>::new(world, seed);
            let mut routes = LocalRouteStream::new(world, seed);
            facilities.preflight(&land);
            let facility = facilities.planned((x, z)).unwrap();
            if seed == 9 {
                assert!(facility.airfield().is_some());
                assert_eq!(facility.ports().count(), 2);
            } else {
                assert!(facility.rail().unwrap().grain_terminal().is_some());
            }
            rivers.preflight(&land, &facilities, view, &mut WatershedWork::new());
            routes.preflight(&land, view, |key, route| route_reservation(&world, &facilities, &rivers, key, route));
            let highways = HighwayStream::new(world, seed);
            let rail = std::boxed::Box::new(RailStream::new(world, seed));
            let natural = NaturalStream::new(world, seed);
            let authorities = UrbanAuthorities {
                facilities: &facilities, rivers: &rivers, routes: &routes, highways: &highways,
                rail: &rail, natural: &natural, summits: None,
            };
            let mut mask = authorities.start([x, z], REVISION).unwrap();
            while !authorities.advance(&mut mask, 64, |_| false) {}
            let mut fast = authorities.start([x, z], REVISION).unwrap();
            authorities.complete(&mut fast);
            assert!(fast.protects(&mask) && mask.protects(&fast));
            let mut urban = UrbanStream::new();
            let job = urban.claim(&land, [x, z], REVISION).unwrap();
            let reference = job.request.prepare_with_bounds(
                |x, z| mask.blocks(x, z),
                |_| true,
                |x, z| mask.near_waterway(x, z),
                || {},
            );
            let result = job.prepare_reserved(&mask, || {}).unwrap();
            assert_eq!(urban.commit_reserved(&land, result, &mask, None), UrbanPublication::Published);
            let addition = urban.approved(&land, [x, z]).unwrap();
            assert_eq!(addition.bounds(), reference.bounds());
            let [left, near, right, far] = addition.bounds().unwrap();
            let mut occupied = 0;
            for z in (near as i32..far as i32).step_by(4) {
                for x in (left as i32..right as i32).step_by(4) {
                    let (x, z) = (x as f32, z as f32);
                    let expected = addition.town_sample(Terminus::Start, x, z)
                        .or_else(|| addition.town_sample(Terminus::End, x, z));
                    assert_eq!(addition.sample(x, z), expected);
                    assert_eq!(addition.sample(x, z), reference.sample(x, z));
                    if expected.is_some() {
                        assert!(facility.sample(x, z, world.sample(x, z)).is_none());
                        occupied += 1;
                    }
                }
            }
            assert!(occupied > 100, "seed {seed} did not retain enough new development");
        }
    }

    #[test]
    fn urban_job_rejects_world_reset_and_same_key_slot_reuse() {
        let mut land = land();
        let mut urban = UrbanStream::new();
        let job = urban.claim(&land, REGION, REVISION).unwrap();
        land.reset(World::new(130));
        let result = job.prepare(|_, _| false, || {});
        assert_eq!(
            urban.commit(&land, result, REVISION, None),
            UrbanPublication::Stale
        );
        assert!(urban.approved(&land, REGION).is_none());

        land.reset(WORLD);
        prepare_owner(&mut land);
        let job = urban.claim(&land, REGION, REVISION).unwrap();
        prepare_owner(&mut land);
        let result = job.prepare(|_, _| false, || {});
        assert_eq!(
            urban.commit(&land, result, REVISION, None),
            UrbanPublication::Stale
        );

        let job = urban.claim(&land, REGION, REVISION).unwrap();
        let fresh = job.prepare(|_, _| false, || {});
        assert_eq!(
            urban.commit(&land, fresh, REVISION, None),
            UrbanPublication::Published
        );
        assert!(urban.approved(&land, REGION).is_some());
        land.reset(WORLD);
        prepare_owner(&mut land);
        assert!(urban.approved(&land, REGION).is_none());
    }

    #[test]
    fn urban_replacement_needs_the_surveyed_revision_and_waits_for_old_view() {
        let land = land();
        let mut urban = UrbanStream::new();
        let job = urban.claim(&land, REGION, REVISION).unwrap();
        let first = job.prepare(|_, _| false, || {});
        let visible = look_at(first.expansion.bounds().unwrap());
        assert_eq!(
            urban.commit(&land, first, REVISION, None),
            UrbanPublication::Published
        );

        let invalid = urban
            .claim(&land, REGION, REVISION + 1)
            .unwrap()
            .prepare(|_, _| true, || {});
        assert_eq!(
            urban.commit(&land, invalid, REVISION + 2, None),
            UrbanPublication::Stale
        );
        assert_eq!(urban.approved(&land, REGION).unwrap().revision(), REVISION);

        let valid = urban
            .claim(&land, REGION, REVISION + 2)
            .unwrap()
            .prepare(|_, _| true, || {});
        assert_eq!(
            urban.commit(&land, valid, REVISION + 2, Some(visible)),
            UrbanPublication::Deferred
        );
        assert!(urban.approved(&land, REGION).unwrap().bounds().is_some());
        let offscreen = Camera {
            z: visible.z + 5000.0,
            ..visible
        };
        assert_eq!(
            urban.publish(&land, REVISION + 2, Some(offscreen)),
            UrbanPublication::Published
        );
        let replacement = urban.approved(&land, REGION).unwrap();
        assert_eq!(replacement.revision(), REVISION + 2);
        assert!(replacement.bounds().is_none());
    }

    #[test]
    fn urban_reset_invalidates_old_job_without_canceling_the_new_claim() {
        let land = land();
        let mut urban = UrbanStream::new();
        let older = urban
            .claim(&land, REGION, REVISION)
            .unwrap()
            .prepare(|_, _| true, || {});
        urban.reset();
        let newer = urban
            .claim(&land, REGION, REVISION)
            .unwrap()
            .prepare(|_, _| false, || {});
        assert_eq!(
            urban.commit(&land, older, REVISION, None),
            UrbanPublication::Stale
        );
        assert_eq!(urban.pending_region(), Some(REGION));
        assert_eq!(
            urban.commit(&land, newer, REVISION, None),
            UrbanPublication::Published
        );
        assert!(urban.approved(&land, REGION).unwrap().bounds().is_some());
    }
}
