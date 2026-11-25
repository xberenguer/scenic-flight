//! Survey, validate, and publish river cuts and bridge geometry as one region.

use core::cell::RefCell;

use crate::crossings::{Bridge, BridgeStyle};
use crate::facilities::RegionalFacilities;
use crate::facility_stream::FacilityStream;
use crate::floodplain::FloodplainKind;
use crate::land_stream::LandStream;
use crate::math::sin_cos;
use crate::render::{screen_region, view_distance, Camera, ScreenRect};
use crate::river_district::RiverDistrict;
use crate::scenery::DistrictRegion;
use crate::watershed::{WaterCoverage, WaterNeighbors, Watershed, WatershedWork, REGION_SIZE};
use crate::world::{Sample, Surface, World, SEA_LEVEL};
use crate::world_mesh::{bounds_in_view, WorldMesh};

type RegionKey = (i32, i32);
const SIDE: usize = 8;
const RIVER_BAND: f32 = 21.0;
const DRY_FOOTPRINT: Sample = Sample {
    elevation: SEA_LEVEL + 4.0,
    surface: Surface::Grass,
};
const SEA_FOOTPRINT: Sample = Sample {
    elevation: SEA_LEVEL,
    surface: Surface::Shallows,
};

#[derive(Clone, Copy)]
pub struct RiverRequest {
    key: RegionKey,
    revision: u32,
    world: World,
    seed: u64,
    owner: RiverOwner,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RiverOwner {
    District,
    Wilderness,
}

impl RiverRequest {
    pub fn key(self) -> RegionKey {
        self.key
    }

    /// Worker-owned survey. All its scratch is reusable between requests.
    pub fn survey(self, work: &mut WatershedWork) -> RiverSurvey {
        self.survey_cooperatively(work, &mut || {})
    }

    pub fn prepare(
        self,
        work: &mut WatershedWork,
        district: Option<&DistrictRegion>,
        facilities: Option<&RegionalFacilities>,
        mut pulse: impl FnMut(),
    ) -> RiverResult {
        self.survey_cooperatively(work, &mut pulse)
            .certify_cooperatively(district, facilities, |_, _| false, &mut pulse)
    }

    fn survey_cooperatively(self, work: &mut WatershedWork, pulse: &mut impl FnMut()) -> RiverSurvey {
        let mut water = Watershed::new();
        water.rebuild_cooperatively(&self.world, self.seed, self.key.0, self.key.1, work, pulse);
        RiverSurvey {
            request: self,
            water,
        }
    }
}

pub struct RiverSurvey {
    request: RiverRequest,
    water: Watershed,
}

impl RiverSurvey {
    pub fn key(&self) -> RegionKey {
        self.request.key
    }

    /// Certify against the streamed district and its evaluated facility
    /// reservation, which can still be awaiting its first safe publication.
    pub fn certify(
        self,
        district: &DistrictRegion,
        facilities: &RegionalFacilities,
    ) -> RiverResult {
        self.certify_with(district, facilities, |_, _| false)
    }

    pub fn certify_with(
        self,
        district: &DistrictRegion,
        facilities: &RegionalFacilities,
        additionally_reserved: impl FnMut(f32, f32) -> bool,
    ) -> RiverResult {
        self.certify_cooperatively(Some(district), Some(facilities), additionally_reserved, &mut || {})
    }

    pub fn certify_land(
        self,
        district: Option<&DistrictRegion>,
        facilities: Option<&RegionalFacilities>,
    ) -> RiverResult {
        self.certify_cooperatively(district, facilities, |_, _| false, &mut || {})
    }

    fn certify_cooperatively(
        self, district: Option<&DistrictRegion>, facilities: Option<&RegionalFacilities>,
        mut additionally_reserved: impl FnMut(f32, f32) -> bool, pulse: &mut impl FnMut(),
    ) -> RiverResult {
        let request = self.request;
        let current_district = district
            .and_then(DistrictRegion::road)
            .and_then(|road| road.polyline().first())
            .map(|p| region_at(p.x, p.z));
        let matched = match request.owner {
            RiverOwner::District => current_district == Some(request.key) && facilities.is_some(),
            RiverOwner::Wilderness => district.is_none() && facilities.is_none(),
        };
        let plan = if matched {
            let facility = facilities.and_then(|sites| Some((sites.bounds()?, sites)));
            RiverDistrict::from_prepared_land_cooperatively(&request.world, district, self.water, |x, z| {
                additionally_reserved(x, z)
                    || facility.is_some_and(|(area, facilities)| {
                        contains(area, x, z)
                            && (facilities.sample(x, z, DRY_FOOTPRINT).is_some()
                                || facilities
                                    .ports()
                                    .any(|port| port.sample(x, z, SEA_FOOTPRINT).is_some()))
                    })
            }, pulse)
        } else {
            None
        };
        RiverResult {
            request,
            matched,
            plan,
        }
    }
}

pub struct RiverResult {
    request: RiverRequest,
    matched: bool,
    plan: Option<RiverDistrict>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RiverCommit {
    Published,
    Deferred,
    Rejected,
    Stale,
    Capacity,
}

struct Resident {
    key: RegionKey,
    plan: RiverDistrict,
    geometry: [[f32; 3]; 2],
    published: bool,
}

/// A sparse cache for completed river regions. Land and facility cache sizes
/// can differ from the number of retained watersheds.
pub struct RiverStream<const ACTIVE: usize> {
    world: World,
    seed: u64,
    revision: u32,
    pending: Option<RiverRequest>,
    rejected: [Option<RegionKey>; SIDE * SIDE],
    residents: [Option<Resident>; ACTIVE],
}

#[derive(Clone, Copy)]
struct FrameWater<'a> {
    bounds: [f32; 4],
    coverage: WaterCoverage,
    wetlands: Option<[f32; 4]>,
    point_bar: Option<[f32; 4]>,
    island: Option<[f32; 4]>,
    plan: &'a RiverDistrict,
}

/// Frame-local rejection data. Terrain outside the actual river bands does
/// not visit the resident grid or geometry.
pub struct RiverSampler<'a, const ACTIVE: usize> {
    sites: [Option<FrameWater<'a>>; ACTIVE],
    count: usize,
    neighbors: RefCell<WaterNeighbors>,
}

impl<const ACTIVE: usize> RiverSampler<'_, ACTIVE> {
    #[inline]
    pub fn overlay(&self, x: f32, z: f32, developed: Sample) -> Sample {
        for site in self.sites[..self.count].iter().flatten() {
            if contains(site.bounds, x, z) {
                let query = site.coverage.locate(x, z);
                let floodplain = site.wetlands.is_some_and(|bounds| contains(bounds, x, z))
                    || site.point_bar.is_some_and(|bounds| contains(bounds, x, z))
                    || (query.is_some() && site.island.is_some_and(|bounds| contains(bounds, x, z)));
                if query.is_some() || floodplain {
                    let water = query.and_then(|query| {
                        site.plan.watershed().sample_neighbors(query, &mut self.neighbors.borrow_mut())
                    });
                    let covered = if floodplain {
                        site.plan.overlay_sampled(x, z, developed, water)
                    } else {
                        water.map_or(developed, |water| RiverDistrict::cover(water, developed))
                    };
                    if covered != developed {
                        return covered;
                    }
                }
            }
        }
        developed
    }
}

impl<const ACTIVE: usize> RiverStream<ACTIVE> {
    pub const fn new(world: World, seed: u64) -> Self {
        Self {
            world,
            seed,
            revision: 0,
            pending: None,
            rejected: [None; SIDE * SIDE],
            residents: [const { None }; ACTIVE],
        }
    }

    pub fn reset(&mut self, world: World, seed: u64) {
        self.world = world;
        self.seed = seed;
        self.pending = None;
        self.revision = self.revision.wrapping_add(1);
        self.rejected.fill(None);
        for slot in &mut self.residents {
            *slot = None;
        }
    }

    /// Before the first frame, certify prepared districts and survey the nearest
    /// wilderness area. More remote natural streams use the background worker.
    #[inline(never)]
    pub fn preflight<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        facilities: &FacilityStream<LAND>,
        camera: Camera,
        work: &mut WatershedWork,
    ) {
        let mut wilderness_budget = true;
        while let Some(request) = self.request_near(land, facilities, camera, wilderness_budget) {
            let district = land.planned(request.key).flatten();
            let reservation = facilities.planned(request.key);
            if request.owner == RiverOwner::Wilderness {
                wilderness_budget = false;
            }
            let result = request.survey(work).certify_land(district, reservation);
            if matches!(
                self.insert(result, land, camera, true),
                RiverCommit::Capacity | RiverCommit::Stale
            ) {
                break;
            }
        }
    }

    /// Survey prepared roads and their facilities first, then confirmed wilderness
    /// near and ahead of the camera. A district awaiting facilities stays pending.
    pub fn request<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        facilities: &FacilityStream<LAND>,
        camera: Camera,
    ) -> Option<RiverRequest> {
        self.request_near(land, facilities, camera, true)
    }

    fn request_near<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        facilities: &FacilityStream<LAND>,
        camera: Camera,
        allow_wilderness: bool,
    ) -> Option<RiverRequest> {
        self.follow(land, camera);
        if self.pending.is_some() || !valid(camera) || self.slot(camera).is_none() {
            return None;
        }
        let (sin, cos) = sin_cos(camera.heading);
        let available = |key| {
            self.rejected[directory(key)] != Some(key)
                && self.residents.iter().flatten().all(|record| record.key != key)
        };
        let closest = |a: &RegionKey, b: &RegionKey| {
            let score = |key| {
                let [x, z] = tile_center(key);
                let dx = x - camera.x;
                let dz = z - camera.z;
                let ahead = dx * sin + dz * cos;
                let behind = ahead.min(0.0);
                dx * dx + dz * dz + behind * behind * 0.38
            };
            score(*a).total_cmp(&score(*b))
        };
        let (key, owner) = land.districts()
            .filter_map(|(key, _)| (available(key) && facilities.planned(key).is_some()).then_some(key))
            .min_by(closest)
            .map(|key| (key, RiverOwner::District))
            .or_else(|| {
                if !allow_wilderness { return None; }
                land.nearby_wilderness(camera)
                    .filter(|&key| available(key))
                    .min_by(closest)
                    .map(|key| (key, RiverOwner::Wilderness))
            })?;
        self.revision = self.revision.wrapping_add(1);
        let request = RiverRequest {
            key,
            revision: self.revision,
            world: self.world,
            seed: self.seed,
            owner,
        };
        self.pending = Some(request);
        Some(request)
    }

    /// Cancel a request whose worker could not usefully complete it.
    /// Any result arriving after cancellation is stale.
    pub fn cancel(&mut self, request: RiverRequest) {
        if self.matches(request) {
            self.pending = None;
        }
    }

    pub fn commit<const LAND: usize>(
        &mut self,
        result: RiverResult,
        land: &LandStream<LAND>,
        camera: Camera,
    ) -> RiverCommit {
        self.insert(result, land, camera, false)
    }

    pub fn commit_avoiding<const LAND: usize>(
        &mut self,
        mut result: RiverResult,
        land: &LandStream<LAND>,
        camera: Camera,
        occupied: impl FnOnce(&RiverDistrict) -> bool,
    ) -> RiverCommit {
        if self.matches(result.request) && result.plan.as_ref().is_some_and(occupied) {
            result.plan = None;
        }
        self.insert(result, land, camera, false)
    }

    /// Reveal ready regions only while their cuts and bridges are out of
    /// view. Once revealed, a change of heading cannot cancel them.
    pub fn follow<const LAND: usize>(&mut self, land: &LandStream<LAND>, camera: Camera) {
        if !valid(camera) {
            return;
        }
        if self
            .pending
            .is_some_and(|job| owner_at(land, job.key) != Some(job.owner))
        {
            self.pending = None;
        }
        for resident in self.residents.iter_mut().flatten() {
            if !resident.published
                && owner_is_ready(land, resident.key)
                && !visible(&resident.plan, camera)
            {
                resident.published = true;
            }
        }
    }

    pub fn river(&self, key: RegionKey) -> Option<&RiverDistrict> {
        self.residents
            .iter()
            .flatten()
            .find(|record| record.key == key && record.published)
            .map(|record| &record.plan)
    }

    /// Cheap frame gate before sampling the terrain layer.
    pub fn has_visible_rivers(&self, camera: Camera) -> bool {
        self.residents
            .iter()
            .flatten()
            .any(|record| record.published && bounds_in_view(camera, record.plan.bounds()))
    }

    /// Keep a new transport route out of both placed and pending riverbanks.
    pub fn reserves_scenery(&self, bounds: [f32; 4]) -> bool {
        self.residents
            .iter()
            .flatten()
            .any(|record| record.plan.intersects_scenery(bounds))
    }

    pub fn wetland_sites(&self, camera: Camera) -> impl Iterator<Item = FloodplainKind> + '_ {
        self.residents.iter().flatten().filter(|record| record.published)
            .flat_map(|record| record.plan.floodplain().sites())
            .filter_map(move |(kind, area)| bounds_in_view(camera, area).then_some(kind))
    }

    /// The evaluated reservation, including plans awaiting safe publication.
    pub fn planned(&self, key: (i32, i32)) -> Option<Option<&RiverDistrict>> {
        if let Some(record) = self.residents.iter().flatten().find(|record| record.key == key) {
            Some(Some(&record.plan))
        } else if self.rejected[directory(key)] == Some(key) {
            Some(None)
        } else {
            None
        }
    }

    /// Copy published bounds and compact raster gates into the caller's frame
    /// storage. The region plans stay borrowed until the renderer is finished.
    pub fn sampler(&self) -> RiverSampler<'_, ACTIVE> {
        let mut sites = [None; ACTIVE];
        let mut count = 0;
        for (slot, resident) in sites.iter_mut().zip(self.residents.iter()
                .flatten().filter(|resident| resident.published)) {
            let water = resident.plan.watershed();
            *slot = Some(FrameWater {
                bounds: resident.plan.bounds(),
                coverage: *water.coverage(),
                wetlands: resident.plan.floodplain().wetland_bounds(),
                point_bar: resident.plan.floodplain().bar_bounds(),
                island: resident.plan.floodplain().island_bounds(),
                plan: &resident.plan,
            });
            count += 1;
        }
        RiverSampler { sites, count, neighbors: RefCell::new(WaterNeighbors::new()) }
    }

    /// Supply the already developed natural and road sample.
    pub fn overlay(&self, x: f32, z: f32, developed: Sample) -> Sample {
        self.river(region_at(x, z))
            .map_or(developed, |plan| plan.overlay(x, z, developed))
    }

    /// Bounds of visible bridge geometry for partial terrain-depth rendering.
    pub fn bridge_region(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        self.residents
            .iter()
            .flatten()
            .filter(|record| record.published)
            .filter_map(|record| {
                if record.plan.crossings().bridges().is_empty() { return None; }
                let [lo, hi] = record.geometry;
                bounds_in_view(camera, [lo[0], lo[2], hi[0], hi[2]])
                    .then(|| screen_region(camera, dimensions, [lo, hi]))
                    .flatten()
            })
            .reduce(ScreenRect::union)
    }

    /// Actual published crossings, including those beyond the near-mesh window.
    pub fn bridges(&self) -> impl Iterator<Item = &crate::crossings::Bridge> {
        self.residents.iter().flatten().filter(|record| record.published)
            .flat_map(|record| record.plan.crossings().bridges())
    }

    /// Draw after the matching water overlay and its terrain depth.
    pub fn draw(&self, camera: Camera, mesh: &mut WorldMesh<'_>) {
        self.draw_fading(camera, mesh, |_| 1.0);
    }

    pub fn draw_fading(&self, camera: Camera, mesh: &mut WorldMesh<'_>,
        opacity: impl Fn(&crate::crossings::Bridge) -> f32,
    ) {
        for bridge in self.bridges().filter(|b| bounds_in_view(camera, b.bounds())) {
            bridge.draw_fading(mesh, opacity(bridge));
        }
    }

    /// Absolute obstacle height for the flight clearance sampler.
    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        self.river(region_at(x, z))
            .into_iter()
            .flat_map(|plan| plan.crossings().bridges())
            .filter_map(|bridge| {
                bridge.road_height(x, z).map(|height| {
                    let top = match bridge.style {
                        BridgeStyle::LowGirder => 0.60,
                        BridgeStyle::SmallTruss => (bridge.span() * 0.10).clamp(1.35, 2.55) + 0.15,
                    };
                    height + top
                })
            })
            .fold(0.0, f32::max)
    }

    fn matches(&self, request: RiverRequest) -> bool {
        self.pending.is_some_and(|job| {
            job.revision == request.revision && job.key == request.key && job.seed == request.seed
        })
    }

    fn insert<const LAND: usize>(
        &mut self,
        result: RiverResult,
        land: &LandStream<LAND>,
        camera: Camera,
        before_first_frame: bool,
    ) -> RiverCommit {
        let request = result.request;
        if !self.matches(request) {
            return RiverCommit::Stale;
        }
        self.pending = None;
        let key = request.key;
        if !result.matched || !valid(camera) || owner_at(land, key) != Some(request.owner) {
            return RiverCommit::Stale;
        }
        let Some(plan) = result.plan else {
            self.rejected[directory(key)] = Some(key);
            return RiverCommit::Rejected;
        };
        let Some(index) = self.slot(camera) else {
            return RiverCommit::Capacity;
        };
        let geometry = geometry_bounds(plan.crossings().bridges());
        let published =
            owner_is_ready(land, key) && (before_first_frame || !visible(&plan, camera));
        self.residents[index] = Some(Resident {
            key,
            plan,
            geometry,
            published,
        });
        if published {
            RiverCommit::Published
        } else {
            RiverCommit::Deferred
        }
    }

    fn slot(&self, camera: Camera) -> Option<usize> {
        if let Some(empty) = self.residents.iter().position(|slot| slot.is_none()) {
            return Some(empty);
        }
        let keep = view_distance(camera.altitude) * 1.28 + 128.0;
        self.residents
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                let plan = &record.as_ref()?.plan;
                let distance = planar_distance_squared(plan.bounds(), camera);
                (distance > keep * keep && !visible(plan, camera)).then_some((index, distance))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }
}

fn owner_at<const LAND: usize>(land: &LandStream<LAND>, key: RegionKey) -> Option<RiverOwner> {
    land.planned(key).map(|district| {
        if district.is_some() { RiverOwner::District } else { RiverOwner::Wilderness }
    })
}

fn owner_is_ready<const LAND: usize>(land: &LandStream<LAND>, key: RegionKey) -> bool {
    owner_at(land, key) == Some(RiverOwner::Wilderness) || land.district_published(key)
}

fn valid(camera: Camera) -> bool {
    camera.heading.is_finite()
        && camera.altitude.is_finite()
        && camera.x.abs() <= 16_000_000.0
        && camera.z.abs() <= 16_000_000.0
}

fn region_at(x: f32, z: f32) -> RegionKey {
    let axis = |value: f32| {
        let scaled = value * (1.0 / REGION_SIZE as f32);
        let whole = scaled as i32;
        whole.saturating_sub(i32::from(scaled < whole as f32))
    };
    (axis(x), axis(z))
}

fn tile_center((x, z): RegionKey) -> [f32; 2] {
    [
        (x as f32 + 0.5) * REGION_SIZE as f32,
        (z as f32 + 0.5) * REGION_SIZE as f32,
    ]
}

fn directory((x, z): RegionKey) -> usize {
    (z as usize & (SIDE - 1)) * SIDE + (x as usize & (SIDE - 1))
}

fn contains([left, front, right, back]: [f32; 4], x: f32, z: f32) -> bool {
    (left..=right).contains(&x) && (front..=back).contains(&z)
}

fn planar_distance_squared([left, front, right, back]: [f32; 4], camera: Camera) -> f32 {
    let dx = (left - camera.x).max(camera.x - right).max(0.0);
    let dz = (front - camera.z).max(camera.z - back).max(0.0);
    dx * dx + dz * dz
}

fn visible(plan: &RiverDistrict, camera: Camera) -> bool {
    bounds_in_view(camera, plan.bounds())
        && (plan
            .crossings()
            .bridges()
            .iter()
            .any(|bridge| bounds_in_view(camera, bridge.bounds()))
            || plan.watershed().reaches().any(|reach| {
                let [x, z] = reach.start;
                let [a, b] = reach.end;
                bounds_in_view(
                    camera,
                    [
                        x.min(a) - RIVER_BAND,
                        z.min(b) - RIVER_BAND,
                        x.max(a) + RIVER_BAND,
                        z.max(b) + RIVER_BAND,
                    ],
                )
            }))
}

fn geometry_bounds(bridges: &[Bridge]) -> [[f32; 3]; 2] {
    let mut low = [f32::INFINITY; 3];
    let mut high = [f32::NEG_INFINITY; 3];
    for bridge in bridges {
        bridge.triangles(|triangle| {
            for point in triangle.corners {
                for axis in 0..3 {
                    low[axis] = low[axis].min(point[axis]);
                    high[axis] = high[axis].max(point[axis]);
                }
            }
        });
    }
    [low, high]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::land_stream::LandBudget;
    use crate::render::render_terrain_layer_region;
    use crate::sky::Sky;

    const SEED: u64 = 1;
    const KEY: RegionKey = (1, -1);
    const WET_ROAD: (f32, f32) = (1680.0, -560.0);

    fn camera() -> Camera {
        Camera {
            x: 1673.0,
            altitude: 19.0,
            z: -592.0,
            heading: 0.29,
            pitch: 0.32,
            bank: 0.0,
        }
    }

    fn setup() -> (World, LandStream<8>, FacilityStream<8>) {
        let world = World::new(SEED);
        let mut land = LandStream::new();
        land.reset(world);
        assert!(land.preflight(camera(), true));
        assert!(land.district_published(KEY));
        (world, land, FacilityStream::new(world, SEED))
    }

    fn certify(
        request: RiverRequest,
        land: &LandStream<8>,
        facilities: &FacilityStream<8>,
    ) -> RiverResult {
        request.survey(&mut WatershedWork::new()).certify_land(
            land.planned(request.key()).unwrap(),
            facilities.planned(request.key()),
        )
    }

    #[test]
    fn wilderness_keeps_real_channels_without_requiring_a_road_or_bridge() {
        let seed = 14_692;
        let key = (2, 0);
        let look = Camera {
            x: 2_407.0,
            z: 875.0,
            altitude: 32.0,
            heading: core::f32::consts::FRAC_PI_2,
            pitch: 0.2,
            bank: 0.0,
        };
        let world = World::new(seed);
        let mut land = LandStream::<8>::new();
        land.reset(world);
        land.preflight(look, true);
        assert!(matches!(land.planned(key), Some(None)));
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        assert!(facilities.planned(key).is_none());
        let mut rivers = RiverStream::<8>::new(world, seed);
        rivers.preflight(&land, &facilities, look, &mut WatershedWork::new());
        let river = rivers.river(key).unwrap();
        assert!(river.crossings().bridges().is_empty());
        assert!(!river.watershed().streams().is_empty());
        assert!(rivers.has_visible_rivers(look));

        let sampler = rivers.sampler();
        let mut channel_points = 0;
        for reach in river.watershed().reaches() {
            for index in 1..8 {
                let [x, z] = reach.position(index as f32 / 8.0);
                let ground = world.sample(x, z);
                let covered = river.overlay(x, z, ground);
                assert_eq!(sampler.overlay(x, z, ground), covered);
                if covered.surface == Surface::RiverWater {
                    assert!(covered.elevation <= ground.elevation + 0.01);
                    channel_points += 1;
                }
            }
        }
        assert!(channel_points > 40);

        let mut wet_points = 0;
        for (_, [left, near, right, far]) in river.floodplain().sites() {
            for row in 0..=16 {
                for col in 0..=16 {
                    let x = left + (right - left) * col as f32 / 16.0;
                    let z = near + (far - near) * row as f32 / 16.0;
                    let ground = world.sample(x, z);
                    let covered = river.overlay(x, z, ground);
                    assert_eq!(sampler.overlay(x, z, ground), covered);
                    wet_points += usize::from(matches!(covered.surface, Surface::ReedMarsh | Surface::WetMeadow));
                }
            }
        }
        assert!(wet_points > 100);
    }

    #[test]
    fn near_late_result_keeps_road_until_water_and_bridge_can_publish_together() {
        let (world, land, mut facilities) = setup();
        let mut rivers = RiverStream::<2>::new(world, SEED);
        if let Some(unrelated) = rivers.request(&land, &facilities, camera()) {
            assert_ne!(unrelated.key(), KEY);
            rivers.cancel(unrelated);
        }
        facilities.preflight(&land);
        let request = rivers.request(&land, &facilities, camera()).unwrap();
        assert_eq!(request.key(), KEY);
        assert!(rivers.request(&land, &facilities, camera()).is_none());
        let result = certify(request, &land, &facilities);
        assert_eq!(
            rivers.commit(result, &land, camera()),
            RiverCommit::Deferred
        );

        let (x, z) = WET_ROAD;
        let pavement = land.overlay(x, z, world.sample(x, z));
        assert_eq!(pavement.surface, Surface::Pavement);
        assert_eq!(rivers.overlay(x, z, pavement), pavement);
        assert!(rivers.bridge_region(camera(), (112, 112)).is_none());
        assert_eq!(rivers.obstruction(x, z), 0.0);

        let offscreen = Camera {
            x: 880.0,
            z: -590.0,
            heading: -1.56,
            ..camera()
        };
        rivers.follow(&land, offscreen);
        rivers.follow(&land, camera());
        let channel = rivers.overlay(x, z, pavement);
        assert_eq!(channel.surface, Surface::RiverWater);
        assert!(rivers.bridge_region(camera(), (112, 112)).is_some());
        assert!(rivers.has_visible_rivers(camera()));
        assert!(rivers.obstruction(x, z) > channel.elevation + 0.8);
    }

    #[test]
    fn prepared_ahead_launches_before_the_visible_river() {
        let world = World::new(SEED);
        let start = Camera {
            x: 1650.0,
            z: -1740.0,
            heading: 0.0,
            ..camera()
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(start, true));
        assert!(land.district_published(KEY));
        let mut facilities = FacilityStream::new(world, SEED);
        facilities.preflight(&land);
        let mut rivers = RiverStream::<3>::new(world, SEED);
        while let Some(request) = rivers.request(&land, &facilities, start) {
            let result = certify(request, &land, &facilities);
            let published = rivers.commit(result, &land, start);
            if request.key() == KEY {
                assert_eq!(published, RiverCommit::Published);
                break;
            }
        }
        assert!(rivers.river(KEY).is_some());
        assert!(!rivers.has_visible_rivers(start));
        assert!(rivers.bridge_region(start, (112, 112)).is_none());
        for i in 0..=26 {
            let z = start.z + (camera().z - start.z) * i as f32 / 26.0;
            let view = Camera { z, ..start };
            rivers.follow(&land, view);
            assert!(rivers.river(KEY).is_some());
        }
        assert!(rivers.has_visible_rivers(camera()));
        assert!(rivers.bridge_region(camera(), (112, 112)).is_some());
    }

    #[test]
    fn retained_river_survives_live_land_churn_and_abort_invalidates_worker() {
        let (world, mut land, mut facilities) = setup();
        facilities.preflight(&land);
        let mut rivers = RiverStream::<2>::new(world, SEED);
        rivers.preflight(&land, &facilities, camera(), &mut WatershedWork::new());
        assert!(rivers.river(KEY).is_some());
        let original = rivers.river(KEY).unwrap().crossings().bridges()[0];

        let work = LandBudget {
            expansions: 26,
            terrain_samples: 120,
            settlement_jobs: 2,
        };
        for i in 0..=26 {
            let view = Camera {
                x: camera().x + i as f32 * 30.0,
                z: camera().z + i as f32 * 9.0,
                heading: if i < 12 { 0.7 } else { 2.8 },
                ..camera()
            };
            for _ in 0..=5 {
                land.service(view, work);
                facilities.service(&land, view);
            }
            rivers.follow(&land, view);
            assert_eq!(
                rivers.river(KEY).unwrap().crossings().bridges()[0],
                original
            );
        }
        let destination = Camera {
            x: 6429.0,
            z: 3616.0,
            heading: 1.56,
            ..camera()
        };
        for _ in 0..240 {
            land.service(destination, work);
            facilities.service(&land, destination);
            rivers.follow(&land, destination);
        }
        assert!(land.districts().any(|(key, _)| key == (6, 3)));
        assert_eq!(
            rivers.river(KEY).unwrap().crossings().bridges()[0],
            original
        );

        let old = Camera {
            x: 1673.0,
            z: -592.0,
            ..camera()
        };
        let (_, land, mut facilities) = setup();
        facilities.preflight(&land);
        let mut rivers = RiverStream::<2>::new(world, SEED);
        let request = rivers.request(&land, &facilities, old).unwrap();
        let result = certify(request, &land, &facilities);
        rivers.reset(World::new(5), 5);
        assert_eq!(rivers.commit(result, &land, old), RiverCommit::Stale);
        assert!(rivers.river(KEY).is_none());
    }

    #[test]
    fn native_bridge_rectangle_covers_the_exact_visible_mesh() {
        const SIDE: usize = 112;
        const COUNT: usize = SIDE * SIDE;
        let (world, land, mut facilities) = setup();
        facilities.preflight(&land);
        let mut river = RiverStream::<2>::new(world, SEED);
        river.preflight(&land, &facilities, camera(), &mut WatershedWork::new());

        let sky = Sky::from_time(SEED, 0, 0.51);
        for bank in [0.0, 0.22, -0.37] {
            let camera = Camera { bank, ..camera() };
            let dims = (SIDE, SIDE);
            let clip = river.bridge_region(camera, dims).unwrap();
            assert!((clip.right - clip.left) * (clip.bottom - clip.top) < COUNT / 2);
            let make_frame = |rect: ScreenRect| {
                let mut pixels = [0; COUNT];
                let mut terrain = [0; COUNT];
                let mut depth = [u16::MAX; COUNT];
                render_terrain_layer_region(
                    (
                        |x, z| river.overlay(x, z, land.overlay(x, z, world.sample(x, z))),
                        |_, _, terrain: Sample| terrain.surface,
                    ),
                    camera,
                    &sky,
                    &mut pixels,
                    &mut terrain,
                    Some((&mut depth, rect)),
                    dims,
                    |_| {},
                )
                .unwrap();
                let before = pixels;
                river.draw(
                    camera,
                    &mut WorldMesh::new(&mut pixels, &mut depth, dims, camera, &sky).unwrap(),
                );
                assert!(
                    pixels
                        .iter()
                        .zip(before)
                        .filter(|(&after, old)| after != *old)
                        .count()
                        >= 180
                );
                pixels
            };
            assert_eq!(make_frame(clip), make_frame(ScreenRect::full(dims)));
        }
    }
}
