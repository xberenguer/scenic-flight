//! Budgeted preparation of alpine, inland dry, and coastal landforms.


use crate::alpine_glaciers::{AlpineGlacier, GlacierWork};
use crate::alpine_lakes::{ALPINE_TILE_SIZE, AlpineLake};
use crate::caprock_mesas::CaprockMesa;
use crate::coastal_bluffs::BluffSampler;
use crate::dry_landmarks::{DRY_LANDMARK_TILE, DryLandmark, DryLandmarkKind, MAX_DRY_BASINS};
use crate::facility_stream::FacilityStream;
use crate::feature_tiles::FeatureTiles;
use crate::glacial_moraines::GlacialMoraine;
use crate::highway_stream::{HighwayConnection, HighwayPair, HighwayStream, Reservation as HighwayReservation, highway_reservation};
use crate::land_stream::LandStream;
use crate::local_routes::LocalRouteStream;
use crate::outwash_fans::OutwashFan;
use crate::render::{Camera, ScreenRect};
use crate::river_district::RiverDistrict;
use crate::river_stream::RiverStream;
use crate::roads::ROAD_REGION_SIZE;
use crate::scenery::overlay_alpine;
use crate::scree_fans::ScreeFan;
use crate::world::{Sample, Surface, World};
use crate::world_mesh::{bounds_in_view, WorldMesh};

// A settled camera tile keeps three complete tiles on each side. Even the
// smallest tile here leaves more room than the longest terrain ray.
const SIDE: usize = 7;
const ALPINE_CAPACITY: usize = 5;
const DRY_CAPACITY: usize = 6;

/// Minimum x, minimum z, maximum x, maximum z.
pub type NaturalBounds = [f32; 4];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NaturalReservation {
    /// Every relevant facility, route, and watershed reservation has been checked.
    Clear,
    /// The surveyed footprint overlaps a protected plan.
    Reserved,
    /// At least one relevant external plan is still being prepared.
    Pending,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NaturalBudget {
    pub alpine_tiles: u8,
    pub dry_tiles: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NaturalLayer {
    Alpine,
    Dry,
}

// The dry planner surveys at most four basins per tile. If the main core
// rejects a footprint immediately, keep that exclusion for the next survey.
#[derive(Clone, Copy)]
struct NaturalJob {
    layer: NaturalLayer,
    tile: [i32; 2],
    rejected: [NaturalBounds; MAX_DRY_BASINS],
    count: u8,
    alpine: AlpinePolicy,
    try_mesa: bool,
}

#[derive(Clone, Copy)]
struct AlpinePolicy {
    moraine: bool,
    outwash: bool,
    glacier: bool,
    scree: bool,
}

enum AlpineSurvey {
    Lake(AlpineLake),
    Moraine(GlacialMoraine),
    Outwash(OutwashFan),
    Glacier(AlpineGlacier),
    Scree(ScreeFan),
    Empty,
}

enum DrySurvey {
    Landmark(Option<DryLandmark>),
    Mesa(CaprockMesa),
}

enum Surveyed {
    Alpine(AlpineSurvey),
    Dry(DrySurvey),
}

/// One immutable survey request. It owns its inputs and may run on another core.
pub struct NaturalRequest {
    world: World,
    seed: u64,
    serial: u32,
    job: NaturalJob,
}

pub struct NaturalResult {
    seed: u64,
    serial: u32,
    tile: [i32; 2],
    surveyed: Surveyed,
}

impl NaturalRequest {
    /// Survey one tile without publishing or reading live scenery reservations.
    /// The callback lets a worker service pending display transfers.
    #[inline(never)]
    pub fn prepare(&self, mut pulse: impl FnMut()) -> NaturalResult {
        let [x, z] = self.job.tile;
        let surveyed = match self.job.layer {
            NaturalLayer::Alpine => Surveyed::Alpine(survey_alpine(
                &self.world, self.seed, x, z, self.job.alpine, &mut pulse,
            )),
            NaturalLayer::Dry => {
                let excluded = &self.job.rejected[..usize::from(self.job.count)];
                let landmark = DryLandmark::generate_with_cooperatively(
                    &self.world, self.seed, x, z, |bounds| excluded.contains(&bounds), &mut pulse,
                );
                Surveyed::Dry(survey_dry(
                    &self.world, self.seed, x, z, landmark, self.job.try_mesa, &mut pulse,
                ))
            }
        };
        NaturalResult {
            seed: self.seed,
            serial: self.serial,
            tile: self.job.tile,
            surveyed,
        }
    }
}

fn survey_alpine(
    world: &World, seed: u64, x: i32, z: i32, policy: AlpinePolicy, pulse: &mut impl FnMut(),
) -> AlpineSurvey {
    if let Some(lake) = AlpineLake::generate_cooperatively(world, seed, x, z, pulse) {
        return AlpineSurvey::Lake(lake);
    }
    if policy.moraine {
        if let Some(moraine) = GlacialMoraine::generate_cooperatively(world, seed, x, z, pulse) {
            return AlpineSurvey::Moraine(moraine);
        }
    }
    if policy.outwash {
        if let Some(fan) = OutwashFan::generate_cooperatively(world, seed, x, z, pulse) {
            return AlpineSurvey::Outwash(fan);
        }
    }
    if policy.glacier {
        if let Some(glacier) = survey_glacier(world, seed, [x, z], pulse) {
            return AlpineSurvey::Glacier(glacier);
        }
    }
    if policy.scree {
        if let Some(fan) = ScreeFan::generate_cooperatively(world, seed, x, z, pulse) {
            return AlpineSurvey::Scree(fan);
        }
    }
    AlpineSurvey::Empty
}

// Keep the reusable height cache out of the older survey branches' stack frames.
#[inline(never)]
fn survey_glacier(
    world: &World, seed: u64, tile: [i32; 2], pulse: &mut impl FnMut(),
) -> Option<AlpineGlacier> {
    AlpineGlacier::generate(world, seed, tile, &mut GlacierWork::new(), |_| false, pulse)
}

fn survey_dry(
    world: &World, seed: u64, x: i32, z: i32, landmark: Option<DryLandmark>,
    try_mesa: bool, pulse: &mut impl FnMut(),
) -> DrySurvey {
    if try_mesa && landmark.as_ref().is_none_or(|plan| plan.kind() == DryLandmarkKind::CoastalBluff) {
        if let Some(mesa) = CaprockMesa::generate_cooperatively(world, seed, x, z, pulse) {
            return DrySurvey::Mesa(mesa);
        }
    }
    DrySurvey::Landmark(landmark)
}

/// Reserve both existing and cold-prepared human and river geometry. Neighboring
/// districts can provide approach ramps or overhead cables across a tile edge.
pub fn scenery_reservation<const LAND: usize, const RIVERS: usize, const ROUTES: usize>(
    land: &LandStream<LAND>,
    facilities: &FacilityStream<LAND>,
    rivers: &RiverStream<RIVERS>,
    routes: &LocalRouteStream<ROUTES>,
    bounds: NaturalBounds,
) -> NaturalReservation {
    let [left, near, right, far] = bounds;
    let tile = ROAD_REGION_SIZE as f32;
    let region = |position: f32| {
        let scaled = position / tile;
        let truncated = scaled as i32;
        truncated - i32::from(scaled < truncated as f32)
    };
    let overlaps = |[a, b, c, d]: NaturalBounds| left <= c && right >= a && near <= d && far >= b;
    let mut pending = false;
    for z in region(near - 128.0)..=region(far + 128.0) {
        for x in region(left - 128.0)..=region(right + 128.0) {
            let key = (x, z);
            let river = rivers.planned(key);
            if river.flatten().is_some_and(|river| river.intersects_scenery(bounds)) {
                return NaturalReservation::Reserved;
            }
            let Some(district) = land.planned(key) else {
                pending = true;
                continue;
            };
            let Some(district) = district else {
                continue;
            };
            if overlaps(district.bounds()) {
                return NaturalReservation::Reserved;
            }
            match (
                facilities.planned(key),
                river,
                routes.planned(key),
            ) {
                (Some(facilities), Some(_), Some(route)) => {
                    if facilities.bounds().is_some_and(overlaps)
                        || route.is_some_and(|route| overlaps(route.bounds()))
                    {
                        return NaturalReservation::Reserved;
                    }
                }
                _ => pending = true,
            }
        }
    }
    if pending {
        NaturalReservation::Pending
    } else {
        NaturalReservation::Clear
    }
}

/// Wait for a nearby highway proposal to resolve before publishing natural scenery.
pub fn scenery_reservation_with_highways<
    const LAND: usize, const RIVERS: usize, const ROUTES: usize, const HIGHWAYS: usize,
>(
    land: &LandStream<LAND>, facilities: &FacilityStream<LAND>, rivers: &RiverStream<RIVERS>,
    routes: &LocalRouteStream<ROUTES>, highways: &HighwayStream<HIGHWAYS>, bounds: NaturalBounds,
) -> NaturalReservation {
    let scenery = scenery_reservation(land, facilities, rivers, routes, bounds);
    if scenery == NaturalReservation::Reserved {
        return scenery;
    }
    match highways.reservation(land, bounds) {
        HighwayReservation::Blocked => NaturalReservation::Reserved,
        HighwayReservation::Pending => NaturalReservation::Pending,
        HighwayReservation::Clear => scenery,
    }
}

/// Protect natural features that have already passed their reservation checks.
pub fn highway_reservation_with_nature<
    const LAND: usize, const RIVERS: usize, const ROUTES: usize,
>(
    land: &LandStream<LAND>, facilities: &FacilityStream<LAND>, rivers: &RiverStream<RIVERS>,
    routes: &LocalRouteStream<ROUTES>, natural: &NaturalStream, pair: HighwayPair, link: &HighwayConnection,
) -> HighwayReservation {
    if natural.approved_bounds().any(|area| link.intersects_bounds(area))
    {
        return HighwayReservation::Blocked;
    }
    highway_reservation(land, facilities, rivers, routes, pair, link)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Publication {
    Pending,
    Ready,
    Active,
    Rejected,
}

struct Site<T> {
    plan: T,
    bounds: NaturalBounds,
    state: Publication,
}

impl<T> Site<T> {
    fn new(plan: T, bounds: NaturalBounds, reservation: NaturalReservation) -> Option<Self> {
        let state = match reservation {
            NaturalReservation::Clear => Publication::Ready,
            NaturalReservation::Pending => Publication::Pending,
            NaturalReservation::Reserved => return None,
        };
        Some(Self {
            plan,
            bounds,
            state,
        })
    }

    fn publish(
        &mut self,
        camera: Camera,
        before_first_frame: bool,
        reserve: &mut impl FnMut(NaturalBounds) -> NaturalReservation,
    ) {
        let state = match self.state {
            Publication::Pending => match reserve(self.bounds) {
                NaturalReservation::Clear => Publication::Ready,
                NaturalReservation::Reserved => Publication::Rejected,
                NaturalReservation::Pending => Publication::Pending,
            },
            state => state,
        };
        self.state = if state == Publication::Ready
            && (before_first_frame || !bounds_in_view(camera, self.bounds))
        {
            Publication::Active
        } else {
            state
        };
    }

    fn active(&self) -> bool {
        self.state == Publication::Active
    }

    fn approved(&self) -> bool {
        matches!(self.state, Publication::Ready | Publication::Active)
    }

    fn held_bounds(&self) -> Option<NaturalBounds> {
        (self.state != Publication::Rejected).then_some(self.bounds)
    }

    fn plan(&self) -> Option<&T> {
        self.active().then_some(&self.plan)
    }
}

struct IsolatedSite<T> {
    tile: [i32; 2],
    site: Site<T>,
}

impl<T> IsolatedSite<T> {
    fn in_window(&self, camera: Camera, tile_size: i32) -> bool {
        let radius = SIDE as i32 / 2;
        [camera.x, camera.z].iter().zip(self.tile).all(|(&position, center)| {
            let lo = ((center - radius) * tile_size) as f32;
            let hi = ((center + radius + 1) * tile_size) as f32;
            (lo..hi).contains(&position)
        })
    }

    fn available(slot: &Option<Self>) -> bool {
        slot.as_ref().is_none_or(|old| old.site.state == Publication::Rejected)
    }

    fn reserve(
        slot: &mut Option<Self>, tile: [i32; 2], plan: T, bounds: NaturalBounds,
        reserve: &mut impl FnMut(NaturalBounds) -> NaturalReservation,
    ) {
        if Self::available(slot) {
            *slot = Site::new(plan, bounds, reserve(bounds)).map(|site| Self { tile, site });
        }
    }
}

impl IsolatedSite<DryLandmark> {
    fn retain(
        cove: &mut Option<Self>, tile: [i32; 2], dry: Site<DryLandmark>,
    ) -> Option<Site<DryLandmark>> {
        if !matches!(dry.plan.kind(), DryLandmarkKind::CoastalDune | DryLandmarkKind::CoastalBluff) {
            return Some(dry);
        }
        // An existing dune survey takes priority until the bluff becomes visible.
        let dune_before_bluff = dry.plan.kind() == DryLandmarkKind::CoastalDune
            && cove.as_ref().is_some_and(|old| old.site.plan.kind() == DryLandmarkKind::CoastalBluff
                && !old.site.active());
        if Self::available(cove) || dune_before_bluff {
            *cove = Some(Self { tile, site: dry });
        }
        None
    }
}

/// Seeded natural layers retained beyond the terrain's full view distance.
///
/// The driver supplies final world-space reservations. Only return `Clear`
/// when neither a current nor a later-published plan can claim that footprint.
/// Use `Pending` while those plans are still unresolved.
pub struct NaturalStream {
    world: World,
    seed: u64,
    lakes: FeatureTiles<Site<AlpineLake>, SIDE, ALPINE_CAPACITY>,
    dry: FeatureTiles<Site<DryLandmark>, SIDE, DRY_CAPACITY>,
    cove: Option<IsolatedSite<DryLandmark>>,
    moraine: Option<IsolatedSite<GlacialMoraine>>,
    outwash: Option<IsolatedSite<OutwashFan>>,
    glacier: Option<IsolatedSite<AlpineGlacier>>,
    scree: Option<IsolatedSite<ScreeFan>>,
    mesa: Option<IsolatedSite<CaprockMesa>>,
    serial: u32,
    detached: Option<NaturalJob>,
    retry: Option<NaturalJob>,
}

const LOCAL_NATURAL_BOUNDS: usize = 4;

/// When only a few landmarks are published, keep their bounds in the frame's
/// local memory and avoid looking up distant samples in the tile directory.
pub struct NaturalSampler<'a> {
    stream: &'a NaturalStream,
    bounds: [NaturalBounds; LOCAL_NATURAL_BOUNDS],
    count: usize,
    only_glacier: Option<&'a AlpineGlacier>,
    only_bluff: Option<BluffSampler>,
}

impl NaturalSampler<'_> {
    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        if let Some(bluff) = &self.only_bluff {
            return bluff.overlay(x, z, ground);
        }
        if self.count > LOCAL_NATURAL_BOUNDS || self.bounds[..self.count]
            .iter().any(|&[left, near, right, far]| left <= x && x <= right && near <= z && z <= far)
        {
            if let Some(glacier) = self.only_glacier {
                // The ordinary alpine precedence check returns NaN ground
                // unchanged before trying any later landform.
                if ground.elevation.is_nan() {
                    ground
                } else {
                    glacier.overlay(x, z, ground)
                }
            } else {
                self.stream.overlay(x, z, ground)
            }
        } else {
            ground
        }
    }
}

impl NaturalStream {
    pub const fn new(world: World, seed: u64) -> Self {
        Self {
            world,
            seed,
            lakes: FeatureTiles::new(ALPINE_TILE_SIZE),
            dry: FeatureTiles::new(DRY_LANDMARK_TILE),
            cove: None,
            moraine: None,
            outwash: None,
            glacier: None,
            scree: None,
            mesa: None,
            serial: 0,
            detached: None,
            retry: None,
        }
    }

    #[inline(never)]
    pub fn reset(&mut self, world: World, seed: u64) {
        let serial = self.serial.wrapping_add(1);
        *self = Self::new(world, seed);
        self.serial = serial;
    }

    /// Prepare a bounded number of tiles before the first frame of a new world.
    /// Only this entry point may publish an already-visible feature.
    #[inline(never)]
    pub fn preflight(
        &mut self,
        camera: Camera,
        budget: NaturalBudget,
        reserve: impl FnMut(NaturalBounds) -> NaturalReservation,
    ) {
        self.advance(camera, budget, true, reserve);
    }

    /// Prepare at most the given number of new tiles per layer. May also
    /// reconsider already-surveyed footprints after reservations become ready.
    pub fn follow(
        &mut self,
        camera: Camera,
        budget: NaturalBudget,
        reserve: impl FnMut(NaturalBounds) -> NaturalReservation,
    ) {
        self.advance(camera, budget, false, reserve);
    }

    /// Select one still-needed tile on the main core. A known dry alternative
    /// is tried before starting another tile. The tile stays unknown in flight.
    pub fn request(&mut self, camera: Camera, layer: NaturalLayer) -> Option<NaturalRequest> {
        if self.detached.is_some() || !valid_camera(camera) {
            return None;
        }
        self.recenter(camera);
        let job = if let Some(job) = self.retry.take().filter(|job| self.wants(*job)) {
            job
        } else {
            let tile = match layer {
                NaturalLayer::Alpine => self.lakes.next_unknown_centered(),
                NaturalLayer::Dry => self.dry.next_unknown_centered(),
            }?;
            NaturalJob {
                layer,
                tile,
                rejected: [[0.0; 4]; MAX_DRY_BASINS],
                count: 0,
                alpine: AlpinePolicy {
                    moraine: IsolatedSite::available(&self.moraine),
                    outwash: IsolatedSite::available(&self.outwash),
                    glacier: IsolatedSite::available(&self.glacier),
                    scree: IsolatedSite::available(&self.scree),
                },
                try_mesa: IsolatedSite::available(&self.mesa),
            }
        };
        self.serial = self.serial.wrapping_add(1);
        self.detached = Some(job);
        Some(NaturalRequest {
            world: self.world,
            seed: self.seed,
            serial: self.serial,
            job,
        })
    }

    /// Release a request the worker could not accept. Its tile can be retried.
    pub fn cancel(&mut self, request: &NaturalRequest) -> bool {
        if request.serial != self.serial
            || request.seed != self.seed
            || !self
                .detached
                .is_some_and(|job| job.tile == request.job.tile && job.layer == request.job.layer)
        {
            return false;
        }
        let Some(job) = self.detached.take() else {
            return false;
        };
        self.retry = Some(job);
        true
    }

    /// Consume one worker result against current plans and the current camera.
    /// Results from an old launch or a tile outside the new window are ignored.
    #[inline(never)]
    pub fn commit(
        &mut self,
        result: NaturalResult,
        camera: Camera,
        mut reserve: impl FnMut(NaturalBounds) -> NaturalReservation,
    ) -> bool {
        if result.serial != self.serial
            || result.seed != self.seed
            || !self.detached.is_some_and(|job| {
                job.tile == result.tile
                    && matches!(
                        (job.layer, &result.surveyed),
                        (NaturalLayer::Alpine, Surveyed::Alpine(_))
                            | (NaturalLayer::Dry, Surveyed::Dry(_))
                    )
            })
        {
            return false;
        }
        let Some(mut job) = self.detached.take() else {
            return false;
        };
        if !valid_camera(camera) {
            return false;
        }
        self.recenter(camera);
        if !self.wants(job) {
            return false;
        }
        match result.surveyed {
            Surveyed::Alpine(plan) => {
                let site = match plan {
                    AlpineSurvey::Lake(lake) => {
                        let bounds = lake.bounds();
                        Site::new(lake, bounds, reserve(bounds))
                    }
                    AlpineSurvey::Moraine(moraine) => {
                        IsolatedSite::reserve(&mut self.moraine, job.tile, moraine, moraine.bounds(), &mut reserve);
                        None
                    }
                    AlpineSurvey::Outwash(fan) => {
                        IsolatedSite::reserve(&mut self.outwash, job.tile, fan, fan.bounds(), &mut reserve);
                        None
                    }
                    AlpineSurvey::Glacier(glacier) => {
                        IsolatedSite::reserve(&mut self.glacier, job.tile, glacier, glacier.bounds(), &mut reserve);
                        None
                    }
                    AlpineSurvey::Scree(fan) => {
                        IsolatedSite::reserve(&mut self.scree, job.tile, fan, fan.bounds(), &mut reserve);
                        None
                    }
                    AlpineSurvey::Empty => None,
                };
                self.lakes.complete(job.tile, site);
            }
            Surveyed::Dry(DrySurvey::Mesa(plan)) => {
                IsolatedSite::reserve(&mut self.mesa, job.tile, plan, plan.bounds(), &mut reserve);
                self.dry.complete(job.tile, None);
            }
            Surveyed::Dry(DrySurvey::Landmark(plan)) => {
                let site = if let Some(plan) = plan {
                    let bounds = plan.bounds();
                    let policy = reserve(bounds);
                    if policy == NaturalReservation::Reserved {
                        job.rejected[usize::from(job.count)] = bounds;
                        job.count += 1;
                        if usize::from(job.count) < MAX_DRY_BASINS {
                            self.retry = Some(job);
                            return true;
                        }
                    }
                    Site::new(plan, bounds, policy)
                } else {
                    None
                };
                let inland = site.and_then(|site| IsolatedSite::retain(&mut self.cove, job.tile, site));
                self.dry.complete(job.tile, inland);
            }
        }
        self.publish(camera, false, &mut reserve);
        true
    }

    fn recenter(&mut self, camera: Camera) {
        self.lakes.recenter(camera.x, camera.z);
        self.dry.recenter(camera.x, camera.z);
        if let Some(cove) = self.cove.as_ref().filter(|cove| !cove.in_window(camera, DRY_LANDMARK_TILE)) {
            self.dry.forget_empty(cove.tile);
            self.cove = None;
        }
        if let Some(moraine) = self.moraine.as_ref().filter(|moraine| !moraine.in_window(camera, ALPINE_TILE_SIZE)) {
            self.lakes.forget_empty(moraine.tile);
            self.moraine = None;
        }
        if let Some(outwash) = self.outwash.as_ref().filter(|outwash| !outwash.in_window(camera, ALPINE_TILE_SIZE)) {
            self.lakes.forget_empty(outwash.tile);
            self.outwash = None;
        }
        if let Some(glacier) = self.glacier.as_ref().filter(|glacier| !glacier.in_window(camera, ALPINE_TILE_SIZE)) {
            self.lakes.forget_empty(glacier.tile);
            self.glacier = None;
        }
        if let Some(scree) = self.scree.as_ref().filter(|scree| !scree.in_window(camera, ALPINE_TILE_SIZE)) {
            self.lakes.forget_empty(scree.tile);
            self.scree = None;
        }
        if let Some(mesa) = self.mesa.as_ref().filter(|mesa| !mesa.in_window(camera, DRY_LANDMARK_TILE)) {
            self.dry.forget_empty(mesa.tile);
            self.mesa = None;
        }
    }

    fn wants(&self, job: NaturalJob) -> bool {
        match job.layer {
            NaturalLayer::Alpine => self.lakes.wants(job.tile),
            NaturalLayer::Dry => self.dry.wants(job.tile),
        }
    }

    #[inline(never)]
    fn advance(
        &mut self,
        camera: Camera,
        budget: NaturalBudget,
        before_first_frame: bool,
        mut reserve: impl FnMut(NaturalBounds) -> NaturalReservation,
    ) {
        if !valid_camera(camera) {
            return;
        }
        self.recenter(camera);
        let world = &self.world;
        let seed = self.seed;
        let moraine = &mut self.moraine;
        let outwash = &mut self.outwash;
        let glacier = &mut self.glacier;
        let scree = &mut self.scree;
        self.lakes.follow_centered(
            camera.x,
            camera.z,
            usize::from(budget.alpine_tiles),
            |x, z| {
                let policy = AlpinePolicy {
                    moraine: IsolatedSite::available(moraine),
                    outwash: IsolatedSite::available(outwash),
                    glacier: IsolatedSite::available(glacier),
                    scree: IsolatedSite::available(scree),
                };
                match survey_alpine(world, seed, x, z, policy, &mut || {}) {
                    AlpineSurvey::Lake(lake) => {
                        let bounds = lake.bounds();
                        Site::new(lake, bounds, reserve(bounds))
                    }
                    AlpineSurvey::Moraine(plan) => {
                        IsolatedSite::reserve(moraine, [x, z], plan, plan.bounds(), &mut reserve);
                        None
                    }
                    AlpineSurvey::Outwash(fan) => {
                        IsolatedSite::reserve(outwash, [x, z], fan, fan.bounds(), &mut reserve);
                        None
                    }
                    AlpineSurvey::Glacier(plan) => {
                        IsolatedSite::reserve(glacier, [x, z], plan, plan.bounds(), &mut reserve);
                        None
                    }
                    AlpineSurvey::Scree(fan) => {
                        IsolatedSite::reserve(scree, [x, z], fan, fan.bounds(), &mut reserve);
                        None
                    }
                    AlpineSurvey::Empty => None,
                }
            },
        );
        let cove = &mut self.cove;
        let mesa = &mut self.mesa;
        self.dry
            .follow_centered(camera.x, camera.z, usize::from(budget.dry_tiles), |x, z| {
                let mut reservation = NaturalReservation::Reserved;
                let landmark = DryLandmark::generate_with(world, seed, x, z, |bounds| {
                    reservation = reserve(bounds);
                    reservation == NaturalReservation::Reserved
                });
                match survey_dry(world, seed, x, z, landmark, IsolatedSite::available(mesa), &mut || {}) {
                    DrySurvey::Landmark(landmark) => {
                        let plan = landmark?;
                        let site = Site::new(plan, plan.bounds(), reservation)?;
                        IsolatedSite::retain(cove, [x, z], site)
                    }
                    DrySurvey::Mesa(plan) => {
                        IsolatedSite::reserve(mesa, [x, z], plan, plan.bounds(), &mut reserve);
                        None
                    }
                }
            });
        self.publish(camera, before_first_frame, &mut reserve);
    }

    fn publish(
        &mut self,
        camera: Camera,
        before_first_frame: bool,
        reserve: &mut impl FnMut(NaturalBounds) -> NaturalReservation,
    ) {
        for site in self.lakes.sites_mut() {
            site.publish(camera, before_first_frame, reserve);
        }
        for site in self.dry.sites_mut().chain(self.cove.iter_mut().map(|cove| &mut cove.site)) {
            site.publish(camera, before_first_frame, reserve);
        }
        if let Some(moraine) = self.moraine.as_mut() {
            moraine.site.publish(camera, before_first_frame, reserve);
        }
        if let Some(outwash) = self.outwash.as_mut() {
            outwash.site.publish(camera, before_first_frame, reserve);
        }
        if let Some(mesa) = self.mesa.as_mut() {
            mesa.site.publish(camera, before_first_frame, reserve);
        }
        if let Some(bounds) = self.glacier_site().map(|site| site.bounds) {
            let overlaps_legacy = self.legacy_nature_intersects(bounds);
            let site = &mut self.glacier.as_mut().unwrap().site;
            if overlaps_legacy {
                // A legacy landform keeps precedence. Retire an already
                // visible glacier when its footprint is out of view.
                if !site.active() || !bounds_in_view(camera, site.bounds) {
                    site.state = Publication::Rejected;
                }
            } else {
                site.publish(camera, before_first_frame, reserve);
            }
        }
        if let Some(bounds) = self.scree_site().map(|site| site.bounds) {
            let overlaps_ice = self.glacier_site().and_then(Site::held_bounds)
                .is_some_and(|glacier| overlapping_bounds(bounds, glacier));
            let overlaps_legacy = self.legacy_nature_intersects(bounds);
            let site = &mut self.scree.as_mut().unwrap().site;
            if overlaps_ice || overlaps_legacy {
                if !site.active() || !bounds_in_view(camera, site.bounds) {
                    site.state = Publication::Rejected;
                }
            } else {
                site.publish(camera, before_first_frame, reserve);
            }
        }
    }

    fn legacy_nature_intersects(&self, bounds: NaturalBounds) -> bool {
        self.lakes.sites().filter_map(Site::held_bounds)
            .chain(self.dry_sites().filter_map(Site::held_bounds))
            .chain(self.moraine_site().and_then(Site::held_bounds))
            .chain(self.outwash_site().and_then(Site::held_bounds))
            .chain(self.mesa_site().and_then(Site::held_bounds))
            .any(|other| overlapping_bounds(bounds, other))
    }

    fn cove_site(&self) -> Option<&Site<DryLandmark>> {
        self.cove.as_ref().map(|cove| &cove.site)
    }

    fn moraine_site(&self) -> Option<&Site<GlacialMoraine>> {
        self.moraine.as_ref().map(|moraine| &moraine.site)
    }

    pub fn moraine(&self) -> Option<&GlacialMoraine> {
        self.moraine_site().and_then(Site::plan)
    }

    fn outwash_site(&self) -> Option<&Site<OutwashFan>> {
        self.outwash.as_ref().map(|fan| &fan.site)
    }

    pub fn outwash(&self) -> Option<&OutwashFan> {
        self.outwash_site().and_then(Site::plan)
    }

    fn glacier_site(&self) -> Option<&Site<AlpineGlacier>> {
        self.glacier.as_ref().map(|glacier| &glacier.site)
    }

    pub fn glacier(&self) -> Option<&AlpineGlacier> {
        self.glacier_site().and_then(Site::plan)
    }

    fn scree_site(&self) -> Option<&Site<ScreeFan>> {
        self.scree.as_ref().map(|fan| &fan.site)
    }

    pub fn scree_fan(&self) -> Option<&ScreeFan> {
        self.scree_site().and_then(Site::plan)
    }

    fn mesa_site(&self) -> Option<&Site<CaprockMesa>> {
        self.mesa.as_ref().map(|mesa| &mesa.site)
    }

    pub fn mesa(&self) -> Option<&CaprockMesa> {
        self.mesa_site().and_then(Site::plan)
    }

    fn dry_sites(&self) -> impl Iterator<Item = &Site<DryLandmark>> {
        self.dry.sites().chain(self.cove_site())
    }

    pub fn lakes(&self) -> impl Iterator<Item = &AlpineLake> {
        self.lakes.sites().filter_map(Site::plan)
    }

    pub fn dry_landmarks(&self) -> impl Iterator<Item = &DryLandmark> {
        self.dry_sites().filter_map(Site::plan)
    }

    /// Screen coverage of the retained natural meshes. Uses the existing
    /// waterfall depth pass so a static cliff does not require another pass.
    pub fn waterfall_region(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        self.lakes()
            .filter_map(AlpineLake::waterfall)
            .filter_map(|fall| fall.screen_region(camera, dimensions))
            .chain(self.scree_fan().and_then(ScreeFan::headwall)
                .and_then(|wall| wall.screen_region(camera, dimensions)))
            .reduce(ScreenRect::union)
    }

    pub fn draw_waterfalls(&self, camera: Camera, ticks: u64, mesh: &mut WorldMesh<'_>) {
        for fall in self.lakes().filter_map(AlpineLake::waterfall) {
            fall.draw(camera, ticks, mesh);
        }
        if let Some(wall) = self.scree_fan().and_then(ScreeFan::headwall) {
            wall.draw(camera, mesh);
        }
    }

    pub(crate) fn approved_bounds(&self) -> impl Iterator<Item = NaturalBounds> + '_ {
        self.lakes.sites().filter(|site| site.approved()).map(|site| site.bounds)
            .chain(self.dry_sites().filter(|site| site.approved()).map(|site| site.bounds))
            .chain(self.moraine_site().filter(|site| site.approved()).map(|site| site.bounds))
            .chain(self.outwash_site().filter(|site| site.approved()).map(|site| site.bounds))
            .chain(self.glacier_site().filter(|site| site.approved()).map(|site| site.bounds))
            .chain(self.scree_site().filter(|site| site.approved()).map(|site| site.bounds))
            .chain(self.mesa_site().filter(|site| site.approved()).map(|site| site.bounds))
    }

    fn active_bounds(&self) -> impl Iterator<Item = NaturalBounds> + '_ {
        self.lakes
            .sites()
            .filter(|site| site.active())
            .map(|site| site.bounds)
            .chain(
                self.dry_sites()
                    .filter(|site| site.active())
                    .map(|site| site.bounds),
            )
            .chain(self.moraine_site().filter(|site| site.active()).map(|site| site.bounds))
            .chain(self.outwash_site().filter(|site| site.active()).map(|site| site.bounds))
            .chain(self.glacier_site().filter(|site| site.active()).map(|site| site.bounds))
            .chain(self.scree_site().filter(|site| site.active()).map(|site| site.bounds))
            .chain(self.mesa_site().filter(|site| site.active()).map(|site| site.bounds))
    }

    pub fn has_visible_features(&self, camera: Camera) -> bool {
        self.active_bounds().any(|bounds| bounds_in_view(camera, bounds))
    }

    pub fn sampler(&self) -> NaturalSampler<'_> {
        let mut bounds = [[0.0; 4]; LOCAL_NATURAL_BOUNDS];
        let mut count = 0;
        for area in self.active_bounds() {
            if count == LOCAL_NATURAL_BOUNDS {
                count += 1;
                break;
            }
            bounds[count] = area;
            count += 1;
        }
        let only_glacier = if count == 1 { self.glacier() } else { None };
        let only_bluff = if count == 1 {
            self.cove_site().and_then(Site::plan).and_then(DryLandmark::bluff_sampler)
        } else {
            None
        };
        NaturalSampler { stream: self, bounds, count, only_glacier, only_bluff }
    }

    /// Lets a later planner protect an already-published natural site.
    pub fn reserves(&self, bounds: NaturalBounds) -> bool {
        let [left, near, right, far] = bounds;
        self.active_bounds()
            .any(|[a, b, c, d]| left <= c && right >= a && near <= d && far >= b)
    }

    pub fn reserves_river(&self, river: &RiverDistrict) -> bool {
        self.approved_bounds().any(|bounds| river.intersects_scenery(bounds))
    }

    /// The active layers sample the supplied terrain without opening new jobs.
    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let dry = || {
            let established = self.dry.at(x, z).and_then(Site::plan)
                .or_else(|| self.cove_site().and_then(Site::plan))
                .map_or(ground, |plan| plan.overlay(x, z, ground));
            if established == ground {
                self.mesa().map_or(ground, |plan| plan.overlay(x, z, ground))
            } else {
                established
            }
        };
        match ground.surface {
            Surface::Grass | Surface::Forest | Surface::Rock | Surface::Snow => {
                let alpine = self.lakes.at(x, z).and_then(Site::plan)
                    .map_or(ground, |lake| overlay_alpine(lake, x, z, ground));
                // A coarser cache cell can carry the material of a neighboring
                // slope. A surveyed dry wash still owns its exact footprint.
                if alpine != ground {
                    return alpine;
                }
                let established = if ground.elevation <= 64.0
                    && matches!(ground.surface, Surface::Grass | Surface::Forest | Surface::Rock)
                {
                    dry()
                } else {
                    self.moraine().map_or(ground, |moraine| moraine.overlay(x, z, ground))
                };
                let established = if established == ground {
                    self.outwash().map_or(ground, |fan| fan.overlay(x, z, ground))
                } else {
                    established
                };
                let established = if established == ground {
                    self.glacier().map_or(ground, |plan| plan.overlay(x, z, ground))
                } else {
                    established
                };
                if established == ground {
                    self.scree_fan().map_or(ground, |fan| fan.overlay(x, z, ground))
                } else {
                    established
                }
            }
            Surface::Desert | Surface::Sand => dry(),
            _ => ground,
        }
    }

    /// Raised water and dune crests can also support normal terrain clearance.
    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        let contains = |[left, near, right, far]: NaturalBounds| {
            (left..=right).contains(&x) && (near..=far).contains(&z)
        };
        if !self
            .lakes
            .at(x, z)
            .is_some_and(|site| site.active() && contains(site.bounds))
            && !self
                .dry
                .at(x, z)
                .is_some_and(|site| site.active() && contains(site.bounds))
            && !self.cove_site().is_some_and(|site| site.active() && contains(site.bounds))
            && !self.moraine_site().is_some_and(|site| site.active() && contains(site.bounds))
            && !self.mesa_site().is_some_and(|site| site.active() && contains(site.bounds))
            && !self.outwash_site().is_some_and(|site| site.active() && contains(site.bounds))
            && !self.glacier_site().is_some_and(|site| site.active() && contains(site.bounds))
            && !self.scree_site().is_some_and(|site| site.active() && contains(site.bounds))
        {
            return 0.0;
        }
        let raw = self.world.sample(x, z);
        let elevation = self.overlay(x, z, raw).elevation;
        let crest = self.scree_fan().and_then(ScreeFan::headwall)
            .map_or(0.0, |wall| wall.obstruction(x, z));
        if elevation > raw.elevation {
            elevation.max(crest)
        } else {
            crest
        }
    }
}

fn overlapping_bounds([left, near, right, far]: NaturalBounds, [a, b, c, d]: NaturalBounds) -> bool {
    left <= c && right >= a && near <= d && far >= b
}

fn valid_camera(camera: Camera) -> bool {
    [camera.x, camera.z, camera.altitude, camera.heading]
        .into_iter()
        .all(f32::is_finite)
        && camera.x.abs() <= 16_000_000.0
        && camera.z.abs() <= 16_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const SWEEP: NaturalBudget = NaturalBudget {
        alpine_tiles: 49,
        dry_tiles: 49,
    };
    const EMPTY: NaturalBudget = NaturalBudget {
        alpine_tiles: 0,
        dry_tiles: 0,
    };
    const CLEAR: fn(NaturalBounds) -> NaturalReservation = |_| NaturalReservation::Clear;

    fn toward(x: f32, z: f32, heading: f32) -> Camera {
        Camera {
            x,
            z,
            heading,
            altitude: 100.0,
            pitch: 0.15,
            bank: 0.0,
        }
    }

    fn request_tile(
        stream: &mut NaturalStream,
        camera: Camera,
        layer: NaturalLayer,
        tile: [i32; 2],
    ) -> NaturalRequest {
        for _ in 0..SIDE * SIDE {
            let request = stream.request(camera, layer).unwrap();
            if request.job.tile == tile {
                return request;
            }
            assert!(stream.commit(request.prepare(|| {}), camera, CLEAR));
        }
        panic!("the fixture tile was outside the current window");
    }

    #[test]
    fn glacier_jobs_preserve_the_ice_until_it_can_publish_offscreen() {
        use core::f32::consts::PI;
        for (seed, tile, mut camera) in [
            (47_711, [9, 12], toward(3592.9207, 4971.1797, 2.7416172)),
            (8, [-9, -12], toward(-3473.2876, -4265.4126, 2.7416172)),
        ] {
            let world = World::new(seed);
            camera.altitude = 185.0;
            let fixture = survey_glacier(&world, seed, tile, &mut || {}).unwrap();
            let (sine, cosine) = crate::math::sin_cos(camera.heading);
            let hidden = Camera {
                x: camera.x - 200.0 * sine,
                z: camera.z - 200.0 * cosine,
                heading: camera.heading + PI,
                ..camera
            };
            assert!(bounds_in_view(camera, fixture.bounds()));
            assert!(!bounds_in_view(hidden, fixture.bounds()));
            let mut stream = NaturalStream::new(world, seed);
            let request = request_tile(&mut stream, camera, NaturalLayer::Alpine, tile);
            let mut pulses = 0;
            let result = request.prepare(|| pulses += 1);
            assert!(pulses >= 196);
            assert!(matches!(&result.surveyed, Surveyed::Alpine(AlpineSurvey::Glacier(plan))
                if plan.bounds() == fixture.bounds()));
            assert!(stream.commit(result, camera, |_| NaturalReservation::Pending));
            assert!(stream.glacier().is_none());
            stream.follow(camera, EMPTY, CLEAR);
            assert!(stream.glacier().is_none());
            stream.follow(hidden, EMPTY, CLEAR);
            let glacier = stream.glacier().unwrap();
            assert_eq!(glacier.bounds(), fixture.bounds());
            let [x, _, z, _] = glacier.profile().nth(7).unwrap();
            let raw = world.sample(x, z);
            let painted = stream.sampler().overlay(x, z, raw);
            assert!(matches!(painted.surface, Surface::GlacierIce | Surface::GlacierCrevasse));
            assert_eq!(stream.obstruction(x, z), painted.elevation);
            assert!(stream.reserves(glacier.bounds()));
        }
    }

    #[test]
    fn alpine_scree_waits_for_a_clear_out_of_view_footprint() {
        let seed = 10;
        let tile = [-6, -7];
        let world = World::new(seed);
        let camera = Camera {
            x: -2206.968, z: -2367.3643, altitude: 127.969826,
            heading: 2.3561945, pitch: 0.28, bank: 0.0,
        };
        let away = Camera { x: camera.x - 160.0, z: camera.z + 160.0, heading: -0.7853982, ..camera };
        let expected = ScreeFan::generate(&world, seed, tile[0], tile[1]).unwrap();
        assert!(bounds_in_view(camera, expected.bounds()));
        assert!(!bounds_in_view(away, expected.bounds()));

        let mut stream = NaturalStream::new(world, seed);
        let request = request_tile(&mut stream, camera, NaturalLayer::Alpine, tile);
        let result = request.prepare(|| {});
        assert!(matches!(&result.surveyed, Surveyed::Alpine(AlpineSurvey::Scree(fan)) if *fan == expected));
        assert!(stream.commit(result, camera, |_| NaturalReservation::Pending));
        stream.follow(camera, EMPTY, CLEAR);
        assert!(stream.scree_fan().is_none());
        assert!(stream.approved_bounds().any(|bounds| bounds == expected.bounds()));

        stream.follow(away, EMPTY, CLEAR);
        assert_eq!(stream.scree_fan(), Some(&expected));
        let [x, z] = expected.center();
        let [dx, dz] = expected.downhill();
        let [sample_x, sample_z] = [x + 26.0 * dx, z + 26.0 * dz];
        let raw = world.sample(sample_x, sample_z);
        let terrain = stream.sampler().overlay(sample_x, sample_z, raw);
        assert_eq!(terrain.surface, Surface::ScreeCoarse);
        assert_eq!(stream.obstruction(sample_x, sample_z), terrain.elevation);
        assert!(stream.reserves(expected.bounds()));

        let far = Camera { x: camera.x + 5.0 * ALPINE_TILE_SIZE as f32, ..camera };
        stream.follow(far, EMPTY, CLEAR);
        assert!(stream.scree_fan().is_none());
        let request = request_tile(&mut stream, away, NaturalLayer::Alpine, tile);
        assert!(stream.commit(request.prepare(|| {}), away, |_| NaturalReservation::Reserved));
        assert!(stream.scree_fan().is_none());
        assert!(!stream.reserves(expected.bounds()));
    }

    #[test]
    fn surveyed_columnar_headwall_reserves_native_rock_and_draws_at_sixty_pixels() {
        use crate::render::render_terrain_layer_region;
        use crate::sky::Sky;

        let seed = 11;
        let world = World::new(seed);
        let expected = ScreeFan::generate(&world, seed, 12, -3).unwrap();
        let wall = expected.headwall().unwrap();
        let camera = Camera {
            x: 4771.6357, z: -1009.48, altitude: 136.31148,
            heading: -core::f32::consts::FRAC_PI_2, pitch: 0.22, bank: 0.0,
        };
        let preparation = Camera { x: 4800.0, z: -960.0, ..camera };
        let budget = NaturalBudget { alpine_tiles: 9, dry_tiles: 0 };
        let mut blocked = NaturalStream::new(world, seed);
        blocked.preflight(preparation, budget, |bounds| {
            if bounds == expected.bounds() { NaturalReservation::Reserved } else { NaturalReservation::Clear }
        });
        assert_ne!(blocked.scree_fan().and_then(ScreeFan::headwall), Some(wall));

        let mut nature = NaturalStream::new(world, seed);
        nature.preflight(preparation, budget, CLEAR);
        assert_eq!(nature.scree_fan().and_then(ScreeFan::headwall), Some(wall));
        assert!(nature.reserves(wall.bounds()));
        let [left, near, right, far] = wall.bounds();
        let mut grounded = 0;
        for row in 0..=20 {
            let z = near + (far - near) * row as f32 / 20.0;
            for column in 0..=20 {
                let x = left + (right - left) * column as f32 / 20.0;
                let native = world.sample(x, z);
                let obstruction = nature.obstruction(x, z);
                if obstruction > native.elevation + 3.0 {
                    assert_eq!(native.surface, Surface::Rock);
                    grounded += 1;
                }
            }
        }
        assert!(grounded > 45);

        const SIDE: usize = 60;
        let size = (SIDE, SIDE);
        let sampler = nature.sampler();
        let sample = |x, z| sampler.overlay(x, z, world.sample(x, z));
        let sky = Sky::from_time_at_position(seed, 0, 0.42, [camera.x, camera.altitude, camera.z]);
        let coverage = nature.waterfall_region(camera, size).unwrap();
        let mut pixels = [0; SIDE * SIDE];
        let mut scratch = pixels;
        let mut depth = [u16::MAX; SIDE * SIDE];
        render_terrain_layer_region(
            (&sample, |_, _, terrain| terrain.surface), camera, &sky, &mut pixels,
            &mut scratch, Some((&mut depth, coverage)), size, |_| {},
        ).unwrap();
        let terrain = pixels;
        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, size, camera, &sky).unwrap();
        nature.draw_waterfalls(camera, 0, &mut mesh);
        let changed = pixels.into_iter().zip(terrain).filter(|(old, new)| old != new).count();
        assert!(changed >= 150, "only {changed} visible headwall pixels");
    }

    #[test]
    fn cove_dune_waits_for_reservations_and_an_offscreen_view() {
        use core::f32::consts::PI;

        let seed = 1;
        let world = World::new(seed);
        let tile = [3, -11];
        let dune = DryLandmark::generate(&world, seed, tile[0], tile[1]).unwrap();
        assert_eq!(dune.kind(), DryLandmarkKind::CoastalDune);
        let [x, z] = dune.center();
        let sea = dune.seaward().unwrap();
        let look = crate::world_mesh::bounds_in_view;
        let camera = toward(x + sea[0] * 140.0, z + sea[1] * 140.0, (-sea[0]).atan2(-sea[1]));
        assert!(look(camera, dune.bounds()));
        let mut stream = NaturalStream::new(world, seed);
        let request = request_tile(&mut stream, camera, NaturalLayer::Dry, tile);
        assert!(stream.commit(request.prepare(|| {}), camera, |_| NaturalReservation::Pending));
        assert!(!stream.dry_landmarks().any(|site| *site == dune));

        stream.follow(camera, EMPTY, CLEAR);
        assert!(!stream.dry_landmarks().any(|site| *site == dune));
        let away = Camera { heading: camera.heading + PI, ..camera };
        assert!(!look(away, dune.bounds()));
        stream.follow(away, EMPTY, CLEAR);
        assert!(stream.dry_landmarks().any(|site| *site == dune));
        let ground = world.sample(x, z);
        let raised = stream.sampler().overlay(x, z, ground);
        assert!(raised.elevation > ground.elevation + 0.7);
        assert_eq!(stream.obstruction(x, z), raised.elevation);

        let far = Camera { x: camera.x + 4.0 * DRY_LANDMARK_TILE as f32, ..camera };
        stream.follow(far, EMPTY, CLEAR);
        assert!(!stream.dry_landmarks().any(|site| *site == dune));
        let return_plan = request_tile(&mut stream, away, NaturalLayer::Dry, tile);
        assert!(stream.commit(return_plan.prepare(|| {}), away, CLEAR));
        assert!(stream.dry_landmarks().any(|site| *site == dune));
    }

    #[test]
    fn coastal_bluff_uses_shared_coastal_priority_and_publication() {
        let world = World::new(0);
        let tile = [-3, -14];
        let bluff = DryLandmark::generate(&world, 0, tile[0], tile[1]).unwrap();
        assert_eq!(bluff.kind(), DryLandmarkKind::CoastalBluff);
        let camera = Camera {
            x: -1401.3779, z: -6921.1323, altitude: 31.0,
            heading: -3.6243296, pitch: 0.43911266, bank: 0.035,
        };
        let mut stream = NaturalStream::new(world, 0);
        let request = request_tile(&mut stream, camera, NaturalLayer::Dry, tile);
        assert!(stream.commit(request.prepare(|| {}), camera, |_| NaturalReservation::Pending));
        assert!(!stream.dry_landmarks().any(|site| *site == bluff));
        stream.follow(camera, EMPTY, CLEAR);
        assert!(stream.approved_bounds().any(|bounds| bounds == bluff.bounds()));
        assert!(!stream.dry_landmarks().any(|site| *site == bluff));

        let other = [3, -11];
        let dune = DryLandmark::generate(&World::new(1), 1, other[0], other[1]).unwrap();
        assert_eq!(dune.kind(), DryLandmarkKind::CoastalDune);
        let dune_site = || Site::new(dune, dune.bounds(), NaturalReservation::Clear).unwrap();
        let mut before_visible = Some(IsolatedSite { tile, site: Site::new(bluff, bluff.bounds(), NaturalReservation::Clear).unwrap() });
        assert!(IsolatedSite::retain(&mut before_visible, other, dune_site()).is_none());
        assert_eq!(before_visible.unwrap().site.plan, dune);

        let away = Camera { z: camera.z + 210.0, heading: 0.0, ..camera };
        assert!(!bounds_in_view(away, bluff.bounds()));
        stream.follow(away, EMPTY, CLEAR);
        assert!(stream.dry_landmarks().any(|site| *site == bluff));
        assert!(stream.reserves(bluff.bounds()));
        let sampler = stream.sampler();
        assert!(sampler.only_bluff.is_some());
        let [left, near, right, far] = bluff.bounds();
        for row in 0..=42 {
            let z = near + (far - near) * (row as f32 / 40.0);
            for column in 0..=42 {
                let x = left + (right - left) * (column as f32 / 40.0);
                for surface in [Surface::Sand, Surface::Grass, Surface::Forest, Surface::Rock] {
                    for elevation in [0.0, 0.11, 0.5, 6.5, 22.99, 23.0, 64.01, f32::NAN] {
                        let ground = Sample { elevation, surface };
                        let reference = stream.overlay(x, z, ground);
                        let fast = sampler.overlay(x, z, ground);
                        assert_eq!(fast.surface, reference.surface);
                        assert_eq!(fast.elevation.to_bits(), reference.elevation.to_bits());
                    }
                }
            }
        }
        assert!(IsolatedSite::retain(&mut stream.cove, other, dune_site()).is_none());
        assert!(stream.dry_landmarks().any(|site| *site == bluff));
    }

    #[test]
    fn outwash_waits_for_authority_and_returns_after_leaving_its_window() {
        let seed = 8;
        let tile = [3, 3];
        let world = World::new(seed);
        let plan = OutwashFan::generate(&world, seed, tile[0], tile[1]).unwrap();
        assert!(AlpineLake::generate(&world, seed, tile[0], tile[1]).is_none());
        assert!(GlacialMoraine::generate(&world, seed, tile[0], tile[1]).is_none());
        let [x, z] = plan.center();
        let camera = toward(x, z - 251.0, 0.0);
        let away = Camera { heading: core::f32::consts::PI, ..camera };
        assert!(bounds_in_view(camera, plan.bounds()));
        assert!(!bounds_in_view(away, plan.bounds()));
        let mut stream = NaturalStream::new(world, seed);
        let request = request_tile(&mut stream, camera, NaturalLayer::Alpine, tile);
        let result = request.prepare(|| {});
        assert!(matches!(&result.surveyed, Surveyed::Alpine(AlpineSurvey::Outwash(fan)) if *fan == plan));
        assert!(stream.commit(result, camera, |_| NaturalReservation::Pending));
        stream.follow(camera, EMPTY, CLEAR);
        assert!(stream.outwash().is_none());
        assert!(stream.approved_bounds().any(|bounds| bounds == plan.bounds()));

        stream.follow(away, EMPTY, CLEAR);
        assert_eq!(stream.outwash(), Some(&plan));
        let mut raised = 0;
        for sample in -38..=38 {
            let x = x + sample as f32;
            let terrain = world.sample(x, z);
            let altered = stream.sampler().overlay(x, z, terrain);
            if altered.elevation > terrain.elevation {
                assert_eq!(stream.obstruction(x, z), altered.elevation);
                raised += 1;
            }
        }
        assert!(raised >= 8);

        let far = Camera { x: camera.x + 4.0 * ALPINE_TILE_SIZE as f32, ..camera };
        stream.follow(far, EMPTY, CLEAR);
        assert!(stream.outwash().is_none());
        let back = request_tile(&mut stream, away, NaturalLayer::Alpine, tile);
        assert!(stream.commit(back.prepare(|| {}), away, CLEAR));
        assert_eq!(stream.outwash(), Some(&plan));
    }

    #[test]
    fn mesa_waits_until_offscreen_and_coexists_with_an_older_foredune() {
        let seed = 1;
        let world = World::new(seed);
        let tile = [1, -1];
        let plan = CaprockMesa::generate(&world, seed, tile[0], tile[1]).unwrap();
        assert!(DryLandmark::generate(&world, seed, tile[0], tile[1]).is_none());
        let [x, z] = plan.centers()[0];
        let camera = toward(x, z - 245.0, 0.0);
        let away = Camera { heading: core::f32::consts::PI, ..camera };
        assert!(bounds_in_view(camera, plan.bounds()));
        assert!(!bounds_in_view(away, plan.bounds()));
        let mut stream = NaturalStream::new(world, seed);
        let request = request_tile(&mut stream, camera, NaturalLayer::Dry, tile);
        let result = request.prepare(|| {});
        assert!(matches!(&result.surveyed, Surveyed::Dry(DrySurvey::Mesa(mesa)) if *mesa == plan));
        assert!(stream.commit(result, camera, |_| NaturalReservation::Pending));
        stream.follow(camera, EMPTY, CLEAR);
        assert!(stream.mesa().is_none());
        assert!(stream.approved_bounds().any(|area| area == plan.bounds()));
        stream.follow(away, EMPTY, CLEAR);
        assert_eq!(stream.mesa(), Some(&plan));

        let cove = DryLandmark::generate(&world, seed, 3, 1).unwrap();
        assert_eq!(cove.kind(), DryLandmarkKind::CoastalDune);
        let mut cove_site = Site::new(cove, cove.bounds(), NaturalReservation::Clear).unwrap();
        cove_site.publish(camera, true, &mut |_| NaturalReservation::Clear);
        stream.cove = Some(IsolatedSite { tile: [3, 1], site: cove_site });
        let ground = world.sample(x, z);
        let sample = stream.sampler().overlay(x, z, ground);
        assert_eq!(sample, plan.overlay(x, z, ground));
        assert!(sample.elevation > ground.elevation + 5.0);
        assert_eq!(sample.elevation, stream.obstruction(x, z));

        let far = Camera { x: camera.x + 4.0 * DRY_LANDMARK_TILE as f32, ..camera };
        stream.follow(far, EMPTY, CLEAR);
        assert!(stream.mesa().is_none());
        let back = request_tile(&mut stream, away, NaturalLayer::Dry, tile);
        assert!(stream.commit(back.prepare(|| {}), away, CLEAR));
        assert_eq!(stream.mesa(), Some(&plan));
    }

    #[test]
    fn moraine_retains_an_independent_slot_and_waits_until_offscreen() {
        let seed = 1;
        let tile = [2, -1];
        let world = World::new(seed);
        let plan = GlacialMoraine::generate(&world, seed, tile[0], tile[1]).unwrap();
        let [x, z] = plan.center();
        let camera = toward(x, z - 240.0, 0.0);
        let away = Camera { heading: core::f32::consts::PI, ..camera };
        assert!(bounds_in_view(camera, plan.bounds()));
        assert!(!bounds_in_view(away, plan.bounds()));
        let mut stream = NaturalStream::new(world, seed);
        let request = request_tile(&mut stream, camera, NaturalLayer::Alpine, tile);
        let result = request.prepare(|| {});
        assert!(matches!(&result.surveyed, Surveyed::Alpine(AlpineSurvey::Moraine(site)) if *site == plan));

        assert!(stream.commit(result, camera, |_| NaturalReservation::Pending));
        assert!(stream.moraine().is_none());
        stream.follow(camera, EMPTY, CLEAR);
        assert!(stream.moraine().is_none());
        assert!(stream.approved_bounds().any(|area| area == plan.bounds()));

        stream.follow(away, EMPTY, CLEAR);
        assert_eq!(stream.moraine(), Some(&plan));
        let mut changed = 0;
        for d in -55..=55 {
            let bx = x + d as f32;
            let terrain = world.sample(bx, z);
            let overlay = stream.sampler().overlay(bx, z, terrain);
            if overlay.elevation > terrain.elevation {
                assert_eq!(stream.obstruction(bx, z), overlay.elevation);
                changed += 1;
            }
        }
        assert!(changed > 12);

        let far = Camera { x: camera.x + 4.0 * ALPINE_TILE_SIZE as f32, ..camera };
        stream.follow(far, EMPTY, CLEAR);
        assert!(stream.moraine().is_none());
        let back = request_tile(&mut stream, away, NaturalLayer::Alpine, tile);
        assert!(stream.commit(back.prepare(|| {}), away, CLEAR));
        assert_eq!(stream.moraine(), Some(&plan));
    }

    #[test]
    fn detached_survey_yields_and_only_publishes_after_leaving_view() {
        let seed = 7;
        let world = World::new(seed);
        let lake = AlpineLake::generate(&world, seed, 0, -6).unwrap();
        let [x, _, z] = lake.floor();
        let camera = toward(x, z - 80.0, 0.0);
        let ground = world.sample(x, z);
        let mut stream = NaturalStream::new(world, seed);
        let request = request_tile(&mut stream, camera, NaturalLayer::Alpine, [0, -6]);
        assert!(stream.request(camera, NaturalLayer::Dry).is_none());
        let (result, pulses) = std::thread::spawn(move || {
            let mut pulses = 0;
            let result = request.prepare(|| pulses += 1);
            (result, pulses)
        })
        .join()
        .unwrap();
        assert!(pulses > 100);
        assert!(matches!(&result.surveyed, Surveyed::Alpine(AlpineSurvey::Lake(plan)) if *plan == lake));
        assert!(stream.commit(result, camera, CLEAR));
        assert_eq!(stream.overlay(x, z, ground), ground);
        stream.follow(toward(x, z - 220.0, core::f32::consts::PI), EMPTY, CLEAR);
        assert_eq!(
            stream.overlay(x, z, ground),
            overlay_alpine(&lake, x, z, ground)
        );
    }

    #[test]
    fn rejected_dry_footprints_retry_with_a_native_alternative() {
        let seed = 0;
        let world = World::new(seed);
        let tile = [7, -18];
        let first = DryLandmark::generate(&world, seed, tile[0], tile[1]).unwrap();
        let excluded = first.bounds();
        let expected =
            DryLandmark::generate_with(&world, seed, tile[0], tile[1], |bounds| bounds == excluded)
                .unwrap();
        let [x, z] = first.center();
        let camera = toward(x, z - 210.0, core::f32::consts::PI);
        let mut stream = NaturalStream::new(world, seed);
        let reserve = |bounds| {
            if bounds == excluded {
                NaturalReservation::Reserved
            } else {
                NaturalReservation::Clear
            }
        };
        let request = request_tile(&mut stream, camera, NaturalLayer::Dry, tile);
        let mut pulses = 0;
        assert!(stream.commit(request.prepare(|| pulses += 1), camera, reserve));
        assert!(pulses > 100);
        assert!(!stream.dry_landmarks().any(|plan| plan.bounds() == excluded));

        let retry = stream.request(camera, NaturalLayer::Alpine).unwrap();
        assert_eq!(retry.job.layer, NaturalLayer::Dry);
        assert_eq!(retry.job.tile, tile);
        assert!(stream.commit(retry.prepare(|| {}), camera, reserve));
        assert!(stream.dry_landmarks().any(|plan| *plan == expected));
    }

    #[test]
    fn canceled_old_and_out_of_window_replies_cannot_publish() {
        let world = World::new(0);
        let camera = toward(0.0, 0.0, 0.0);
        let elsewhere = toward(6400.0, -6112.0, 0.2);
        let mut stream = NaturalStream::new(world, 0);
        let canceled = stream.request(camera, NaturalLayer::Dry).unwrap();
        let stale = canceled.prepare(|| {});
        assert!(stream.cancel(&canceled));
        let retry = stream.request(camera, NaturalLayer::Dry).unwrap();
        assert_eq!(canceled.job.tile, retry.job.tile);
        assert!(!stream.commit(stale, camera, CLEAR));
        assert!(!stream.commit(retry.prepare(|| {}), elsewhere, CLEAR));

        let old = stream.request(elsewhere, NaturalLayer::Dry).unwrap();
        stream.reset(world, 0);
        let current = stream.request(elsewhere, NaturalLayer::Dry).unwrap();
        assert!(!stream.commit(old.prepare(|| {}), elsewhere, CLEAR));
        assert!(stream.commit(current.prepare(|| {}), elsewhere, CLEAR));
        assert!(!stream.commit(current.prepare(|| {}), elsewhere, CLEAR));
    }

    #[test]
    fn real_tarn_matches_unchanged_native_feature_and_waits_for_a_safe_view() {
        let seed = 7;
        let world = World::new(seed);
        let lake = AlpineLake::generate(&world, seed, 0, -6).unwrap();
        let [x, _, z] = lake.floor();
        let camera = toward(x, z - 92.0, 0.0);
        let mut features = NaturalStream::new(world, seed);
        let ground = world.sample(x, z);
        let painted = overlay_alpine(&lake, x, z, ground);
        assert_ne!(painted, ground);

        features.preflight(camera, SWEEP, |_| NaturalReservation::Pending);
        assert_eq!(features.overlay(x, z, ground), ground);
        features.follow(camera, EMPTY, CLEAR);
        assert_eq!(features.overlay(x, z, ground), ground);
        assert!(!features.reserves(lake.bounds()));
        assert!(features.approved_bounds().any(|bounds| bounds == lake.bounds()));

        features.follow(toward(x, z - 200.0, core::f32::consts::PI), EMPTY, CLEAR);
        assert_eq!(features.overlay(x, z, ground), painted);
        assert!(features.has_visible_features(camera));
        assert!(features.reserves(lake.bounds()));
        assert_eq!(features.obstruction(x, z), lake.level());
        let road = Sample {
            surface: Surface::Pavement,
            ..ground
        };
        assert_eq!(features.overlay(x, z, road), road);

        features.reset(World::new(19), 19);
        assert!(features.lakes().next().is_none());
        assert_eq!(features.overlay(x, z, ground), ground);
        assert_eq!(features.obstruction(x, z), 0.0);
    }

    #[test]
    fn real_salt_and_dune_sites_share_exact_native_geometry_only_when_clear() {
        let world = World::new(0);
        for (tx, tz) in [(-2, 11), (-6, -13)] {
            let plan = DryLandmark::generate(&world, 0, tx, tz).unwrap();
            let [x, z] = plan.center();
            let camera = toward(x, z - 70.0, 0.0);
            let mut stream = NaturalStream::new(world, 0);
            stream.preflight(camera, EMPTY, |_| panic!("zero budget started a survey"));
            assert!(stream.dry_landmarks().next().is_none());
            stream.preflight(camera, SWEEP, CLEAR);
            assert!(stream.dry_landmarks().any(|site| *site == plan));
            let [left, near, right, far] = plan.bounds();
            let mut touched = 0;
            for iz in near as i32..far as i32 {
                for ix in left as i32..right as i32 {
                    let (px, pz) = (ix as f32, iz as f32);
                    let raw = world.sample(px, pz);
                    let expected = plan.overlay(px, pz, raw);
                    assert_eq!(stream.overlay(px, pz, raw), expected);
                    touched += usize::from(expected != raw);
                }
            }
            assert!(touched >= 500);

            stream.reset(world, 0);
            stream.preflight(camera, SWEEP, |_| NaturalReservation::Reserved);
            assert!(stream.dry_landmarks().next().is_none());
            assert_eq!(stream.overlay(x, z, world.sample(x, z)), world.sample(x, z));
        }
    }

    #[test]
    fn active_tarn_survives_signed_tile_crossing_and_full_camera_turn() {
        let world = World::new(82);
        let seed = 82;
        let lake = AlpineLake::generate(&world, seed, 2, 7).unwrap();
        let [x, _, z] = lake.floor();
        let mut stream = NaturalStream::new(world, seed);
        let raw = world.sample(x, z);
        let expected = overlay_alpine(&lake, x, z, raw);
        assert_ne!(raw, expected);
        stream.preflight(toward(x, z, 0.0), SWEEP, CLEAR);
        for (dx, heading) in [(-390.0, 0.2), (-42.0, 1.7), (180.0, 4.7), (401.0, 6.0)] {
            stream.follow(toward(x + dx, z, heading), SWEEP, CLEAR);
            assert_eq!(stream.overlay(x, z, raw), expected);
        }
    }
}
