//! Ready-to-render land scenery. Planning and settlement work stay outside
//! terrain sampling, and a region first becomes visible as a complete unit.

use crate::math::sin_cos;
use crate::render::{CITY_HORIZON, SCENERY_HORIZON, terrain_detail_distance, Camera};
use crate::roads::{RoadNetwork, RoadPaintCache, RoadPlanner, RoadProgress, ROAD_REGION_SIZE};
use crate::scenery::DistrictRegion;
use crate::settlement::{Settlement, Terminus};
use crate::world::{Sample, World};

mod urban;
pub use urban::{UrbanAuthorities, UrbanExpansionJob, UrbanExpansionResult, UrbanPaintSampler, UrbanPublication, UrbanReservations, UrbanStream};

const LOOKAHEAD: f32 = 1024.0;
const REGION: f32 = ROAD_REGION_SIZE as f32;
const INVERSE_REGION: f32 = 1.0 / REGION;
const REGION_HALF: f32 = REGION * 0.5;
// Roads and their endpoint districts stay at least this far inside their tile.
const SCENERY_MARGIN: f32 = 96.0;
const SCENERY_HALF: f32 = REGION_HALF - SCENERY_MARGIN;
const SCAN: i32 = 3;
// A diagonal view can reach across a little more world than its forward cap.
const CITY_SCAN: i32 = (CITY_HORIZON as i32 * 13 + 10 * ROAD_REGION_SIZE - 1) / (10 * ROAD_REGION_SIZE);
const SIDE: usize = ((2 * CITY_SCAN + 1) as usize).next_power_of_two();
const SKYLINE_PRIORITY: u8 = 3;
const SKYLINE_START_REGIONS: usize = 2;
const FOREGROUND_RESERVE: usize = 2;
const SKYLINE_START_ATTEMPTS: usize = 8;
const EMPTY: u8 = 254;
const UNKNOWN: u8 = 255;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LandBudget {
    pub expansions: u8,
    pub terrain_samples: u16,
    /// A separate allowance for complete settlement checks between frames.
    pub settlement_jobs: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LandProgress {
    Idle,
    Working,
    AwaitingSettlement,
    Prepared,
    Deferred,
    Capacity,
}

/// Owned foundation survey for the next unchecked terminus of a pending road.
pub struct SettlementRequest {
    world: World,
    road: RoadNetwork,
    end: Terminus,
    serial: u64,
}

pub struct SettlementResult {
    serial: u64,
    town: Option<Settlement>,
}

impl SettlementRequest {
    pub fn prepare(&self, pulse: impl FnMut()) -> SettlementResult {
        SettlementResult {
            serial: self.serial,
            town: Settlement::generate_with_pulse(&self.world, &self.road, self.end, pulse),
        }
    }
}

pub struct LandStream<const ACTIVE: usize = 8> {
    world: World,
    ready: bool,
    directory: [Record; SIDE * SIDE],
    residents: [Option<Resident>; ACTIVE],
    pending: Option<Pending>,
    next_ownership: u64,
    next_settlement: u64,
    tick: u8,
}

impl<const ACTIVE: usize> LandStream<ACTIVE> {
    pub const fn new() -> Self {
        assert!(ACTIVE <= EMPTY as usize);
        Self {
            world: World::new(0),
            ready: false,
            directory: [Record::EMPTY; SIDE * SIDE],
            residents: [const { None }; ACTIVE],
            pending: None,
            next_ownership: 0,
            next_settlement: 0,
            tick: 0,
        }
    }

    pub fn reset(&mut self, world: World) {
        self.world = world;
        self.ready = true;
        self.directory.fill(Record::EMPTY);
        for resident in &mut self.residents {
            *resident = None;
        }
        self.pending = None;
        self.tick = 0;
    }

    /// Synchronous preparation before the first scene is shown. Passing
    /// `ahead` also prepares the first forward and turn buffers if slots fit.
    #[inline(never)]
    pub fn preflight(&mut self, camera: Camera, ahead: bool) -> bool {
        let Some(view) = Window::new(camera) else {
            return false;
        };
        if !self.ready {
            return false;
        }
        self.pending = None;
        for resident in self.residents.iter_mut().flatten() {
            resident.published = true;
        }
        while let Some((key, _)) = self.next_ordinary_missing(view, !ahead) {
            let Some(road) = RoadNetwork::generate(&self.world, key.x, key.z) else {
                self.set(key, EMPTY);
                continue;
            };
            let Some(slot) = self.available_slot(view, key) else {
                break;
            };
            let towns = [
                Settlement::generate(&self.world, &road, Terminus::Start),
                Settlement::generate(&self.world, &road, Terminus::End),
            ];
            self.write_resident(slot, key, road, towns, true);
        }
        self.visible_ready(camera)
    }

    /// Prepare the skyline down the flight path before the first frame.
    /// Keep two resident slots open for the next foreground surveys.
    pub fn preflight_skyline(&mut self, camera: Camera) {
        let Some(view) = Window::new(camera).filter(|_| self.ready) else {
            return;
        };
        let open = self.residents.iter().filter(|entry| entry.is_none()).count();
        let limit = open.saturating_sub(FOREGROUND_RESERVE).min(SKYLINE_START_REGIONS);
        let mut prepared = 0;
        for _ in 0..SKYLINE_START_ATTEMPTS {
            if prepared == limit {
                return;
            }
            let Some((key, _, _)) = self.next_skyline_missing(view) else {
                return;
            };
            let Some(road) = RoadNetwork::generate(&self.world, key.x, key.z) else {
                self.set(key, EMPTY);
                continue;
            };
            let towns = [
                Settlement::generate(&self.world, &road, Terminus::Start),
                Settlement::generate(&self.world, &road, Terminus::End),
            ];
            let slot = self.residents.iter().position(Option::is_none).unwrap();
            self.write_resident(slot, key, road, towns, true);
            prepared += 1;
        }
    }

    /// Reports when the current view has all of its land regions available.
    /// A late region is held until it can enter the view naturally.
    pub fn visible_ready(&self, camera: Camera) -> bool {
        let Some(view) = Window::new(camera) else {
            return false;
        };
        if !self.ready {
            return false;
        }
        for key in view.keys() {
            if !view.visible(key) {
                continue;
            }
            let entry = self.get(key);
            if entry == UNKNOWN
                || (entry != EMPTY
                    && !self.residents[entry as usize]
                        .as_ref()
                        .is_some_and(|region| region.published))
            {
                return false;
            }
        }
        true
    }

    pub fn has_visible_scenery(&self, camera: Camera) -> bool {
        let Some(view) = Window::new(camera) else { return false };
        self.residents.iter().flatten().any(|region| region.published && view.visible(region.key))
    }

    pub fn districts(&self) -> impl Iterator<Item = ((i32, i32), &DistrictRegion)> {
        self.residents.iter().flatten().map(|resident| {
            ((resident.key.x, resident.key.z), &resident.region)
        })
    }

    /// Survey the detailed districts in viewing order. Far skyline silhouettes
    /// need a road and town plan, with facilities reserved for the approach.
    pub(crate) fn facility_survey_districts(
        &self, camera: Camera,
    ) -> impl Iterator<Item = ((i32, i32), &DistrictRegion, (u8, f32))> {
        let view = Window::new(camera);
        self.residents.iter().flatten().filter_map(move |resident| {
            let rank = view?.rank(resident.key)?;
            (rank.0 < SKYLINE_PRIORITY)
                .then_some(((resident.key.x, resident.key.z), &resident.region, rank))
        })
    }

    /// Natural surveys may use areas where the land plan has already ruled out roads.
    pub(crate) fn nearby_wilderness(&self, camera: Camera) -> impl Iterator<Item = (i32, i32)> + '_ {
        Window::new(camera).into_iter().flat_map(move |view| {
            view.keys().filter_map(move |key| {
                (self.get(key) == EMPTY && view.rank(key).is_some()).then_some((key.x, key.z))
            })
        })
    }

    /// An evaluated region can be empty or prepared but awaiting publication.
    pub fn planned(&self, (x, z): (i32, i32)) -> Option<Option<&DistrictRegion>> {
        match self.get(Key { x, z }) {
            UNKNOWN => None,
            EMPTY => Some(None),
            index => self.residents[index as usize].as_ref().map(|resident| Some(&resident.region)),
        }
    }

    pub fn district_published(&self, (x, z): (i32, i32)) -> bool {
        let key = Key { x, z };
        self.residents.get(self.get(key) as usize).and_then(Option::as_ref)
            .is_some_and(|resident| resident.published)
    }

    pub fn has_distant_towers(&self, camera: Camera) -> bool {
        self.visible_towns(camera).any(|town| town.has_distant_towers(camera))
    }

    pub fn distant_tower_region(&self, camera: Camera, dimensions: (usize, usize)) -> Option<crate::render::ScreenRect> {
        self.visible_towns(camera).filter_map(|town| town.tower_region(camera, dimensions))
            .reduce(crate::render::ScreenRect::union)
    }

    pub fn draw_distant_towers(&self, camera: Camera, mesh: &mut crate::world_mesh::WorldMesh<'_>) {
        for town in self.visible_towns(camera) {
            town.draw_distant_towers(camera, mesh);
        }
    }

    fn visible_towns(&self, camera: Camera) -> impl Iterator<Item = &Settlement> {
        let view = Window::new(camera);
        self.residents.iter().flatten().filter(move |resident| {
            resident.published && view.is_some_and(|view| view.visible(resident.key))
        }).flat_map(|resident| resident.region.towns())
    }

    /// Only immutable, fully prepared scenery can override the ground here.
    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        self.published_region(x, z)
            .map_or(ground, |region| region.overlay_local(x, z, ground))
    }

    fn published_region(&self, x: f32, z: f32) -> Option<&DistrictRegion> {
        let slot = self.residents
            .get(self.get(Key::at(x, z)) as usize)
            .and_then(Option::as_ref)?;
        slot.published.then_some(&slot.region)
    }

    /// Prefer exact bounds for the common case of one or two published districts.
    pub fn sampler(&self) -> LandSampler<'_, ACTIVE> {
        let mut direct = [None, None];
        let mut count = 0;
        for resident in self.residents.iter().flatten().filter(|resident| resident.published) {
            let [left, near, right, far] = resident.region.bounds();
            let [x0, z0] = [resident.key.x as f32 * REGION, resident.key.z as f32 * REGION];
            if count == direct.len() || left < x0 || right >= x0 + REGION
                || near < z0 || far >= z0 + REGION
            {
                return LandSampler::Indexed(self);
            }
            direct[count] = Some(&resident.region);
            count += 1;
        }
        match direct {
            [Some(first), Some(second)] => LandSampler::Two(first, second),
            [Some(first), None] => LandSampler::One(first),
            _ => LandSampler::Empty,
        }
    }

    /// Reuse nearby road geometry while painting one frame.
    pub fn paint_sampler(&self) -> LandPaintSampler<'_, ACTIVE> {
        LandPaintSampler {
            base: self.sampler(),
            roads: RoadPaintCache::new(),
        }
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        self.residents.get(self.get(Key::at(x, z)) as usize).and_then(Option::as_ref)
            .filter(|resident| resident.published).map_or(0.0, |resident| resident.region.obstruction(x, z))
    }

    #[inline(never)]
    pub fn service(&mut self, camera: Camera, mut budget: LandBudget) -> LandProgress {
        let Some(view) = Window::new(camera) else {
            return LandProgress::Idle;
        };
        if !self.ready {
            return LandProgress::Idle;
        }
        self.tick = self.tick.wrapping_add(1);
        for resident in self.residents.iter_mut().flatten() {
            if !resident.published && !view.visible(resident.key) {
                resident.published = true;
            }
        }
        let outdated = self
            .pending
            .as_ref()
            .is_some_and(|pending| {
                let key = pending.key();
                match view.rank(key) {
                    None => true,
                    Some((SKYLINE_PRIORITY, _)) => self.available_slot(view, key).is_none(),
                    _ => false,
                }
            });
        let must_prioritize_visible = self.tick & 7 == 0
            && self.pending.as_ref().is_some_and(|pending| {
                let key = pending.key();
                !view.visible(key) && match pending {
                    Pending::Road { .. } => true,
                    Pending::Towns { next, .. } => *next < 2
                        && view.rank(key).is_some_and(|(priority, _)| priority >= 2),
                }
            })
            && self.next_missing(view, true).is_some_and(|key| self.available_slot(view, key).is_some());
        if outdated || must_prioritize_visible {
            self.pending = None;
        }
        if self.pending.is_none() {
            if budget.terrain_samples < 2 {
                return LandProgress::Idle;
            }
            let Some(key) = self.next_missing(view, false) else {
                return LandProgress::Idle;
            };
            budget.terrain_samples -= 2;
            if let Some(plan) = RoadPlanner::new(&self.world, key.x, key.z) {
                self.pending = Some(Pending::Road { key, plan });
            } else {
                self.set(key, EMPTY);
                return LandProgress::Prepared;
            }
        }

        match self.pending.as_mut() {
            Some(Pending::Road { key, plan }) => {
                let key = *key;
                match plan.advance(budget.expansions, budget.terrain_samples) {
                    RoadProgress::Working => LandProgress::Working,
                    RoadProgress::Unavailable => {
                        self.set(key, EMPTY);
                        self.pending = None;
                        LandProgress::Prepared
                    }
                    RoadProgress::Ready => {
                        let road = plan.take_network().unwrap();
                        self.pending = Some(Pending::Towns {
                            key,
                            road,
                            towns: [None, None],
                            next: 0,
                            queued: None,
                        });
                        LandProgress::AwaitingSettlement
                    }
                }
            }
            Some(Pending::Towns {
                road, towns, next, queued, ..
            }) => {
                if queued.is_some() || (*next < 2 && budget.settlement_jobs == 0) {
                    return LandProgress::AwaitingSettlement;
                }
                for _ in 0..budget.settlement_jobs.min(2) {
                    if *next == 2 {
                        break;
                    }
                    let end = if *next == 0 {
                        Terminus::Start
                    } else {
                        Terminus::End
                    };
                    towns[*next as usize] = Settlement::generate(&self.world, road, end);
                    *next += 1;
                }
                if *next < 2 {
                    LandProgress::AwaitingSettlement
                } else {
                    self.publish_pending(view)
                }
            }
            None => LandProgress::Idle,
        }
    }

    pub fn request_settlement(&mut self) -> Option<SettlementRequest> {
        let Pending::Towns { road, next, queued, .. } = self.pending.as_mut()? else {
            return None;
        };
        if *next >= 2 || queued.is_some() {
            return None;
        }
        let end = if *next == 0 { Terminus::Start } else { Terminus::End };
        let serial = self.next_settlement;
        self.next_settlement = self.next_settlement.wrapping_add(1);
        *queued = Some(serial);
        Some(SettlementRequest { world: self.world, road: road.clone(), end, serial })
    }

    /// A turn or new world can discard the owner while the worker is busy.
    pub fn commit_settlement(&mut self, result: SettlementResult) -> bool {
        let Some(Pending::Towns { towns, next, queued, .. }) = self.pending.as_mut() else {
            return false;
        };
        if *queued != Some(result.serial) {
            return false;
        }
        towns[usize::from(*next)] = result.town;
        *next += 1;
        *queued = None;
        true
    }

    fn publish_pending(&mut self, view: Window) -> LandProgress {
        let key = self.pending.as_ref().unwrap().key();
        let Some(slot) = self.available_slot(view, key) else {
            return LandProgress::Capacity;
        };
        let Some(Pending::Towns { road, towns, .. }) = self.pending.take() else {
            unreachable!()
        };
        let published = !view.visible(key);
        self.write_resident(slot, key, road, towns, published);
        if published {
            LandProgress::Prepared
        } else {
            LandProgress::Deferred
        }
    }

    fn available_slot(&self, view: Window, destination: Key) -> Option<usize> {
        let current = self.get(destination);
        if current < EMPTY {
            return Some(current as usize);
        }
        if let Some(open) = self.residents.iter().position(Option::is_none) {
            return Some(open);
        }
        let wanted = view.rank(destination)?;
        self.residents
            .iter()
            .enumerate()
            .filter_map(|(i, region)| {
                let key = region.as_ref()?.key;
                let rank = view.rank(key);
                let can_release_buffer = rank.is_some_and(|(priority, _)| priority == 2)
                    && (wanted.0 == 0 || wanted.0 == 1 && view.projected(key).0 + view.padding < 0.0);
                let lower_priority = rank.is_none_or(|rank| rank > wanted);
                (!view.visible(key) && lower_priority && (!view.keep(key) || can_release_buffer))
                    .then_some((i, rank.map_or(u8::MAX, |(priority, _)| priority), view.distance_squared(key)))
            })
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| a.2.total_cmp(&b.2)))
            .map(|(i, _, _)| i)
    }

    fn write_resident(
        &mut self,
        slot: usize,
        key: Key,
        road: RoadNetwork,
        towns: [Option<Settlement>; 2],
        published: bool,
    ) {
        if let Some(previous) = &self.residents[slot] {
            let entry = &mut self.directory[previous.key.slot()];
            if entry.key == previous.key && entry.value == slot as u8 {
                *entry = Record::EMPTY;
            }
        }
        self.next_ownership = self.next_ownership.wrapping_add(1);
        self.residents[slot] = Some(Resident {
            key,
            ownership: self.next_ownership,
            region: DistrictRegion::from_prepared(key.x, key.z, road, towns),
            published,
        });
        self.set(key, slot as u8);
    }

    fn next_ordinary_missing(&self, view: Window, visible_only: bool) -> Option<(Key, (u8, f32))> {
        let mut candidate: Option<(Key, (u8, f32))> = None;
        for key in view.keys() {
            let Some(rank) = view.rank(key) else { continue };
            if rank.0 == SKYLINE_PRIORITY || visible_only && rank.0 != 0 || self.get(key) != UNKNOWN {
                continue;
            }
            if candidate.is_none_or(|(_, best)| rank < best) {
                candidate = Some((key, rank));
            }
        }
        candidate
    }

    fn next_missing(&self, view: Window, visible_only: bool) -> Option<Key> {
        let candidate = self.next_ordinary_missing(view, visible_only);
        let ordinary = candidate.map(|(key, _)| key);
        if visible_only || candidate.is_some_and(|(_, (priority, _))| priority < 2)
            || self.residents.iter().all(|slot| slot.as_ref().is_some_and(|region| view.keep(region.key)))
        {
            return ordinary;
        }
        // Preview cities down the current flight path before preparing turn buffers.
        match self.next_skyline_missing(view) {
            Some((key, false, _)) => Some(key),
            Some((key, true, _)) => ordinary.or(Some(key)),
            None => ordinary,
        }
    }

    fn next_skyline_missing(&self, view: Window) -> Option<(Key, bool, f32)> {
        view.keys_within(CITY_SCAN).filter_map(|key| {
            if self.get(key) != UNKNOWN { return None; }
            let (kind, distance) = view.rank(key)?;
            if kind != SKYLINE_PRIORITY || self.available_slot(view, key).is_none() { return None; }
            let (along, across) = view.projected(key);
            let off_axis = across > along * 0.30 + view.padding * 0.5;
            Some((key, off_axis, distance))
        }).min_by(|(_, a, ad), (_, b, bd)| a.cmp(b).then_with(|| ad.total_cmp(bd)))
    }

    fn get(&self, key: Key) -> u8 {
        let entry = self.directory[key.slot()];
        if entry.key == key {
            entry.value
        } else {
            UNKNOWN
        }
    }

    fn set(&mut self, key: Key, value: u8) {
        self.directory[key.slot()] = Record { key, value };
    }
}

pub enum LandSampler<'a, const ACTIVE: usize> {
    Empty,
    One(&'a DistrictRegion),
    Two(&'a DistrictRegion, &'a DistrictRegion),
    Indexed(&'a LandStream<ACTIVE>),
}

impl<const ACTIVE: usize> LandSampler<'_, ACTIVE> {
    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        self.region_at(x, z).map_or(ground, |region| region.overlay_local(x, z, ground))
    }

    fn region_at(&self, x: f32, z: f32) -> Option<&DistrictRegion> {
        match *self {
            Self::Empty => None,
            Self::One(region) => Some(region),
            Self::Two(first, second) => {
                let [left, near, right, far] = first.bounds();
                if x >= left && x <= right && z >= near && z <= far {
                    Some(first)
                } else {
                    Some(second)
                }
            }
            Self::Indexed(stream) => stream.published_region(x, z),
        }
    }
}

pub struct LandPaintSampler<'a, const ACTIVE: usize> {
    base: LandSampler<'a, ACTIVE>,
    roads: RoadPaintCache,
}

impl<const ACTIVE: usize> LandPaintSampler<'_, ACTIVE> {
    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        self.base.region_at(x, z).map_or(ground, |region| {
            region.overlay_local_with_road(x, z, ground, |road| self.roads.material(road, x, z))
        })
    }
}

impl<const ACTIVE: usize> Default for LandStream<ACTIVE> {
    fn default() -> Self {
        Self::new()
    }
}

struct Resident {
    key: Key,
    ownership: u64,
    region: DistrictRegion,
    published: bool,
}

// Only one regional work item is staged at a time. A completed road is held
// here until both towns are checked so no half-drawn district can escape.
enum Pending {
    Road {
        key: Key,
        plan: RoadPlanner,
    },
    Towns {
        key: Key,
        road: RoadNetwork,
        towns: [Option<Settlement>; 2],
        next: u8,
        queued: Option<u64>,
    },
}

impl Pending {
    fn key(&self) -> Key {
        match self {
            Self::Road { key, .. } | Self::Towns { key, .. } => *key,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Key {
    x: i32,
    z: i32,
}

impl Key {
    fn at(x: f32, z: f32) -> Self {
        Self {
            x: region_axis(x),
            z: region_axis(z),
        }
    }

    fn center(self) -> (f32, f32) {
        (
            (self.x as f32 + 0.5) * REGION,
            (self.z as f32 + 0.5) * REGION,
        )
    }

    fn slot(self) -> usize {
        (self.z as usize & (SIDE - 1)) * SIDE + (self.x as usize & (SIDE - 1))
    }
}

fn region_axis(value: f32) -> i32 {
    let scaled = value * INVERSE_REGION;
    let integral = scaled as i32;
    integral.saturating_sub(i32::from(scaled < integral as f32))
}

#[derive(Clone, Copy)]
struct Record {
    key: Key,
    value: u8,
}

impl Record {
    const EMPTY: Self = Self {
        key: Key {
            x: i32::MIN,
            z: i32::MIN,
        },
        value: UNKNOWN,
    };
}

#[derive(Clone, Copy)]
struct Window {
    x: f32,
    z: f32,
    sin: f32,
    cos: f32,
    horizon: f32,
    padding: f32,
    center: Key,
}

impl Window {
    fn new(camera: Camera) -> Option<Self> {
        if !camera.heading.is_finite()
            || !camera.altitude.is_finite()
            || !(camera.x.abs() <= 16_000_000.0 && camera.z.abs() <= 16_000_000.0)
        {
            return None;
        }
        let (sin, cos) = sin_cos(camera.heading);
        Some(Self {
            x: camera.x,
            z: camera.z,
            sin,
            cos,
            horizon: terrain_detail_distance(camera.altitude),
            padding: SCENERY_HALF * (sin.abs() + cos.abs()),
            center: Key::at(camera.x, camera.z),
        })
    }

    fn keys(self) -> impl Iterator<Item = Key> {
        self.keys_within(SCAN)
    }

    fn keys_within(self, scan: i32) -> impl Iterator<Item = Key> {
        (self.center.z - scan..=self.center.z + scan).flat_map(move |z| {
            (self.center.x - scan..=self.center.x + scan).map(move |x| Key { x, z })
        })
    }

    fn projected(self, key: Key) -> (f32, f32) {
        let (x, z) = key.center();
        let x = x - self.x;
        let z = z - self.z;
        (
            x * self.sin + z * self.cos,
            (x * self.cos - z * self.sin).abs(),
        )
    }

    fn intersects(self, pos: (f32, f32), reach: f32) -> bool {
        self.intersects_width(pos, reach, 0.72)
    }

    fn intersects_width(self, pos: (f32, f32), reach: f32, width: f32) -> bool {
        let near = pos.0 - self.padding;
        let far = pos.0 + self.padding;
        near <= reach && far >= 0.0 && pos.1 - self.padding <= far.min(reach) * width
    }

    fn visible(self, key: Key) -> bool {
        self.intersects(self.projected(key), self.horizon)
    }

    fn distance_squared(self, key: Key) -> f32 {
        let (x, z) = key.center();
        let dx = (x - self.x).abs() - SCENERY_HALF;
        let dz = (z - self.z).abs() - SCENERY_HALF;
        dx.max(0.0) * dx.max(0.0) + dz.max(0.0) * dz.max(0.0)
    }

    fn keep(self, key: Key) -> bool {
        let reach = self.horizon * 1.24 + 112.0;
        self.intersects(self.projected(key), SCENERY_HORIZON) || self.distance_squared(key) <= reach * reach
    }

    fn rank(self, key: Key) -> Option<(u8, f32)> {
        let projected = self.projected(key);
        if self.intersects(projected, self.horizon) {
            return Some((0, self.distance_squared(key)));
        }
        if self.intersects(projected, SCENERY_HORIZON.max(self.horizon + LOOKAHEAD)) {
            let entry = (projected.0 - self.padding).max(0.0);
            let across = (projected.1 - self.padding).max(0.0);
            return Some((1, entry + across * 0.48));
        }
        if self.keep(key) {
            return Some((2, self.distance_squared(key)));
        }
        self.intersects_width(projected, CITY_HORIZON, 0.82)
            .then(|| (SKYLINE_PRIORITY, self.distance_squared(key)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Surface;

    const WORK: LandBudget = LandBudget {
        expansions: 12,
        terrain_samples: 96,
        settlement_jobs: 2,
    };

    fn camera(x: f32, z: f32, altitude: f32, heading: f32) -> Camera {
        Camera {
            x,
            z,
            altitude,
            heading,
            pitch: 0.19,
            bank: 0.0,
        }
    }

    #[test]
    fn outer_scenery_is_planned_for_low_and_high_aircraft() {
        let distant = Key { x: 0, z: 2 };
        let skyline = Key { x: 0, z: 3 };
        let far_skyline = Key { x: 0, z: 4 };
        let extended_skyline = Key { x: 0, z: 5 };
        let beyond = Key { x: 0, z: 6 };
        for altitude in [22.0, 320.0] {
            let window = Window::new(camera(512.0, 900.0, altitude, 0.0)).unwrap();
            assert!(!window.visible(distant));
            assert_eq!(window.rank(distant).map(|rank| rank.0), Some(1));
            assert!(window.keep(distant));
            assert_eq!(window.rank(skyline).map(|rank| rank.0), Some(SKYLINE_PRIORITY));
            assert!(!window.keep(skyline));
            assert_eq!(window.rank(far_skyline).map(|rank| rank.0), Some(SKYLINE_PRIORITY));
            assert!(window.keys_within(CITY_SCAN).any(|key| key == far_skyline));
            assert!(!window.keys().any(|key| key == far_skyline));
            assert_eq!(window.rank(extended_skyline).map(|rank| rank.0), Some(SKYLINE_PRIORITY));
            assert!(window.keys_within(CITY_SCAN).any(|key| key == extended_skyline));
            assert_eq!(window.rank(beyond), None);
            assert_eq!(crate::render::horizon_distance(altitude), SCENERY_HORIZON);
        }
    }

    #[test]
    fn distant_city_planning_respects_priority_and_publishes_the_complete_district() {
        let world = World::new(1);
        let city = Key { x: 3, z: -3 };
        let view = camera(-160.0, -2560.0, 55.0, core::f32::consts::FRAC_PI_2);
        let window = Window::new(view).unwrap();
        assert_eq!(window.projected(city).0, 3744.0);
        assert!(window.intersects_width(window.projected(city), CITY_HORIZON, 0.82));
        assert!(!window.intersects_width(window.projected(city), 2880.0, 0.82));
        let mut stream = LandStream::<8>::new();
        stream.reset(world);
        let nearby = window.center;
        for key in window.keys_within(CITY_SCAN).filter(|&key| key != city && key != nearby) {
            stream.set(key, EMPTY);
        }
        assert_eq!(stream.next_missing(window, false), Some(nearby));
        stream.set(nearby, EMPTY);
        assert_eq!(stream.next_missing(window, true), None);
        assert_eq!(stream.next_missing(window, false), Some(city));
        let key = (city.x, city.z);
        for _ in 0..128 {
            stream.service(view, WORK);
            if stream.district_published(key) { break; }
        }
        assert!(stream.district_published(key));
        let published = stream.planned(key).flatten().unwrap();
        assert!(published.towns().next().is_some());
        let expected = DistrictRegion::build(&world, key.0, key.1);
        assert_eq!(published.road().unwrap().polyline(), expected.road().unwrap().polyline());
        assert_eq!(stream.facility_survey_districts(view).count(), 0);
    }

    #[test]
    fn launch_preflight_can_prepare_a_city_past_three_thousand_without_using_foreground_slots() {
        let world = World::new(1);
        let city = Key { x: 3, z: -3 };
        let view = camera(-160.0, -2560.0, 55.0, core::f32::consts::FRAC_PI_2);
        let window = Window::new(view).unwrap();
        let mut stream = LandStream::<3>::new();
        stream.reset(world);
        for key in window.keys_within(CITY_SCAN).filter(|&key| key != city) {
            stream.set(key, EMPTY);
        }
        assert!(stream.preflight(view, false));
        assert!(!stream.district_published((city.x, city.z)));
        stream.preflight_skyline(view);
        assert!(stream.district_published((city.x, city.z)));
        assert!(stream.planned((city.x, city.z)).flatten().unwrap().towns().next().is_some());
        assert_eq!(stream.residents.iter().filter(|slot| slot.is_none()).count(), 2);
        assert!(stream.visible_ready(view));
    }

    #[test]
    fn launch_preflight_prepares_two_distant_regions_with_two_slots_in_reserve() {
        let world = World::new(1);
        let destinations = [Key { x: 3, z: -3 }, Key { x: 3, z: -2 }];
        let view = camera(-160.0, -2560.0, 55.0, core::f32::consts::FRAC_PI_2);
        let window = Window::new(view).unwrap();
        let mut stream = LandStream::<4>::new();
        stream.reset(world);
        for key in window.keys_within(CITY_SCAN).filter(|key| !destinations.contains(key)) {
            stream.set(key, EMPTY);
        }
        assert!(stream.preflight(view, false));
        stream.preflight_skyline(view);
        for key in destinations {
            let pair = (key.x, key.z);
            assert!(stream.district_published(pair), "missing launch region {key:?}");
            assert!(stream.planned(pair).flatten().unwrap().towns().next().is_some());
        }
        assert_eq!(stream.residents.iter().filter(|slot| slot.is_none()).count(), 2);
        assert!(stream.visible_ready(view));
    }

    #[test]
    fn launch_preflight_finds_a_side_city_when_the_centerline_is_empty() {
        let world = World::new(1);
        let city = Key { x: 3, z: -2 };
        let view = camera(-160.0, -3200.0, 55.0, core::f32::consts::FRAC_PI_2);
        let window = Window::new(view).unwrap();
        let mut stream = LandStream::<3>::new();
        stream.reset(world);
        for key in window.keys_within(CITY_SCAN).filter(|&key| key != city) {
            stream.set(key, EMPTY);
        }
        assert!(matches!(stream.next_skyline_missing(window), Some((key, true, _)) if key == city));
        stream.preflight_skyline(view);
        assert!(stream.district_published((city.x, city.z)));
        assert_eq!(stream.residents.iter().filter(|slot| slot.is_none()).count(), 2);
    }

    #[test]
    fn centerline_city_is_ready_before_turn_buffers_under_the_board_budget() {
        let world = World::new(1);
        let view = Camera {
            x: -2790.6943, z: 2695.9517, altitude: 328.28,
            heading: 2.36, pitch: 0.53, bank: 0.0,
        };
        let mut stream = LandStream::<8>::new();
        stream.reset(world);
        assert!(stream.preflight(view, false));
        let near = (-2, 1);
        let city = (-1, 0);
        assert!(!stream.district_published(city));

        let city_ready = (0..=840).step_by(2).find(|frame| {
            stream.service(view, LandBudget {
                expansions: 8,
                terrain_samples: 32,
                settlement_jobs: u8::from(frame % 16 == 2),
            });
            stream.district_published(city)
        });
        assert!(city_ready.is_some(), "centerline city missed the streaming window");
        assert!(stream.district_published(near));
        assert!(stream.visible_ready(view));
        assert!(stream.planned(city).flatten().unwrap().towns().next().is_some());
    }

    #[test]
    fn approach_surveys_publish_a_farther_harbor_before_a_closer_district_behind() {
        use crate::facility_stream::FacilityStream;

        let seed = 518;
        let world = World::new(seed);
        let behind = Key { x: 5, z: 3 };
        let ahead = Key { x: 5, z: 5 };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        for (slot, key) in [behind, ahead].into_iter().enumerate() {
            let road = RoadNetwork::generate(&world, key.x, key.z).unwrap();
            let towns = [Terminus::Start, Terminus::End]
                .map(|end| Settlement::generate(&world, &road, end));
            land.write_resident(slot, key, road, towns, true);
        }
        for altitude in [55.0, 320.0] {
            let view = camera(5632.0, 4150.0, altitude, 0.0);
            let window = Window::new(view).unwrap();
            assert_eq!(window.projected(behind).0, -566.0);
            assert_eq!(window.projected(ahead).0, 1482.0);
            assert_eq!(window.rank(behind).unwrap().0, 2);
            assert_eq!(window.rank(ahead).unwrap().0, 1);

            let mut facilities = FacilityStream::<8>::new(world, seed);
            let result = facilities.request(&land, view, true).unwrap().prepare(|| {});
            assert!(facilities.commit(result, &land, view));
            assert!(facilities.planned((behind.x, behind.z)).is_none());
            let port = facilities.planned((ahead.x, ahead.z)).unwrap().ports().next().unwrap();
            let [x, z] = port.shore_origin();
            let [dx, dz] = port.seaward();
            assert!((-18..=18).any(|meters| {
                let x = x + dx * meters as f32;
                let z = z + dz * meters as f32;
                let raw = world.sample(x, z);
                port.sample(x, z, raw).is_some() && facilities.overlay(x, z, raw, raw) != raw
            }));
        }
    }

    #[test]
    fn preflight_uses_the_exact_road_and_settlement_overlays() {
        let world = World::new(1);
        let synchronous = DistrictRegion::build(&world, 3, -3);
        let road = synchronous.road().unwrap();
        let anchor = road.polyline()[0];
        let view = camera(
            anchor.x - 78.0,
            anchor.z - 10.0,
            55.0,
            core::f32::consts::FRAC_PI_2,
        );
        let mut stream = LandStream::<8>::new();
        assert!(core::mem::size_of::<LandStream<8>>() < 72_000);
        stream.reset(world);
        assert!(stream.preflight(view, false));
        assert!(stream.visible_ready(view));
        let paint = stream.paint_sampler();
        let road_sample = stream.overlay(anchor.x, anchor.z, world.sample(anchor.x, anchor.z));
        assert_eq!(road_sample.surface, Surface::Pavement);
        for dz in (-120..100).step_by(3) {
            for dx in (-110..120).step_by(3) {
                let x = anchor.x + dx as f32;
                let z = anchor.z + dz as f32;
                if Key::at(x, z) != (Key { x: 3, z: -3 }) {
                    continue;
                }
                let ground = world.sample(x, z);
                let expected = synchronous.overlay(x, z, ground);
                assert_eq!(stream.overlay(x, z, ground), expected);
                assert_eq!(paint.overlay(x, z, ground), expected);
            }
        }
    }

    #[test]
    fn a_late_region_waits_until_it_can_enter_view_naturally() {
        let world = World::new(1);
        let road = RoadNetwork::generate(&world, 3, -3).unwrap();
        let anchor = road.polyline()[0];
        let key = Key { x: 3, z: -3 };
        let incoming = camera(
            anchor.x - 78.0,
            anchor.z - 10.0,
            55.0,
            core::f32::consts::FRAC_PI_2,
        );
        assert!(Window::new(incoming).unwrap().visible(key));
        let natural = world.sample(anchor.x, anchor.z);
        let mut stream = LandStream::<8>::new();
        stream.reset(world);
        stream.pending = Some(Pending::Towns {
            key,
            road,
            towns: [None, None],
            next: 0,
            queued: None,
        });
        assert_eq!(
            stream.service(incoming, LandBudget::default()),
            LandProgress::AwaitingSettlement
        );
        assert_eq!(stream.service(incoming, WORK), LandProgress::Deferred);
        for _ in 0..8 {
            stream.service(incoming, LandBudget::default());
            assert_eq!(stream.overlay(anchor.x, anchor.z, natural), natural);
        }
        let outgoing = camera(4600.0, -2560.0, 55.0, core::f32::consts::FRAC_PI_2);
        assert!(!Window::new(outgoing).unwrap().visible(key));
        stream.service(outgoing, LandBudget::default());
        assert_eq!(
            stream.overlay(anchor.x, anchor.z, natural).surface,
            Surface::Pavement
        );
        stream.service(incoming, LandBudget::default());
        assert_eq!(
            stream.overlay(anchor.x, anchor.z, natural).surface,
            Surface::Pavement
        );
    }

    #[test]
    fn worker_foundations_match_direct_survey_and_reject_previous_world() {
        let world = World::new(1);
        let key = Key { x: 3, z: -3 };
        let road = RoadNetwork::generate(&world, key.x, key.z).unwrap();
        let mut stream = LandStream::<8>::new();
        let pending = || Pending::Towns {
            key, road: road.clone(), towns: [None, None], next: 0, queued: None,
        };

        stream.reset(world);
        stream.pending = Some(pending());
        let stale = stream.request_settlement().unwrap().prepare(|| {});
        assert!(stream.request_settlement().is_none());

        stream.reset(world);
        stream.pending = Some(pending());
        let request = stream.request_settlement().unwrap();
        assert!(!stream.commit_settlement(stale));
        let mut yields = 0;
        let first = request.prepare(|| yields += 1);
        assert_eq!(first.town, Settlement::generate(&world, &road, Terminus::Start));
        assert!(yields > 10);
        assert!(stream.commit_settlement(first));
        let second = stream.request_settlement().unwrap().prepare(|| {});
        assert_eq!(second.town, Settlement::generate(&world, &road, Terminus::End));
        assert!(stream.commit_settlement(second));
        assert!(stream.request_settlement().is_none());
    }

    #[test]
    fn sufficient_work_keeps_regions_ready_across_signed_boundaries() {
        for (seed, altitude, dz) in [(1, 54.0, -2440.0), (31, 320.0, 429.0)] {
            let mut stream = LandStream::<8>::new();
            stream.reset(World::new(seed));
            let heading = core::f32::consts::FRAC_PI_2;
            let initial = camera(-2800.0, dz, altitude, heading);
            assert!(stream.preflight(initial, true));
            let mut visible = 0;
            for i in 0..=400 {
                let view = camera(-2800.0 + i as f32 * 13.0, dz, altitude, heading);
                // Offer several bounded calls here to exercise window behavior
                // independently of any frame-rate claim for a specific board.
                for _ in 0..9 {
                    stream.service(view, WORK);
                }
                assert!(
                    stream.visible_ready(view),
                    "seed {seed} camera {:?}",
                    (view.x, view.z)
                );
                let window = Window::new(view).unwrap();
                visible += window
                    .keys()
                    .filter(|&key| window.visible(key) && stream.get(key) < EMPTY)
                    .count();
            }
            assert!(visible > 20);
        }
    }

    #[test]
    fn renderer_samples_in_scenery_footprints_stay_inside_the_visibility_window() {
        let world = World::new(31);
        let mut image = [0; 112 * 112];
        for (heading, altitude) in [(0.17, 55.0), (2.65, 143.0), (4.1, 282.0), (5.73, 360.0)] {
            let mut eye = camera(1984.0, -2059.0, altitude, heading);
            eye.bank = 0.17;
            let window = Window::new(eye).unwrap();
            let mut interior = 0;
            crate::render::render_from(
                |x, z| {
                    let key = Key::at(x, z);
                    let (cx, cz) = key.center();
                    if (x - cx).abs() <= SCENERY_HALF && (z - cz).abs() <= SCENERY_HALF {
                        assert!(
                            window.visible(key),
                            "visible camera sample omitted for {key:?}"
                        );
                        interior += 1;
                    }
                    world.sample(x, z)
                },
                eye,
                &mut image,
                112,
                112,
            )
            .unwrap();
            assert!(interior > 100);
        }
    }

    #[test]
    fn a_forward_region_can_take_a_slot_from_behind_the_plane() {
        let world = World::new(1);
        let old = Key { x: 3, z: -3 };
        let incoming = Key { x: 3, z: -1 };
        let mut stream = LandStream::<1>::new();
        stream.reset(world);
        let road = RoadNetwork::generate(&world, old.x, old.z).unwrap();
        stream.write_resident(0, old, road, [None, None], true);
        let view = Window::new(camera(3584.0, -2000.0, 55.0, 0.0)).unwrap();
        assert!(view.keep(old));
        assert!(!view.visible(old));
        assert!(view.projected(old).0 + view.padding < 0.0);
        assert_eq!(view.rank(incoming).unwrap().0, 1);
        assert_eq!(stream.available_slot(view, incoming), Some(0));
    }

    #[test]
    fn a_turn_can_retarget_a_queued_offscreen_town_survey() {
        let world = World::new(1);
        let behind = Key { x: 3, z: -3 };
        let ahead = Key { x: 3, z: -2 };
        let camera = camera(3584.0, -2000.0, 55.0, 0.0);
        let view = Window::new(camera).unwrap();
        let mut stream = LandStream::<2>::new();
        stream.reset(world);
        for key in view.keys_within(CITY_SCAN).filter(|&key| key != ahead && key != behind) {
            stream.set(key, EMPTY);
        }
        let road = RoadNetwork::generate(&world, behind.x, behind.z).unwrap();
        stream.pending = Some(Pending::Towns {
            key: behind, road, towns: [None, None], next: 0, queued: None,
        });
        let late = stream.request_settlement().unwrap().prepare(|| {});
        stream.tick = 7;
        stream.service(camera, LandBudget { expansions: 0, terrain_samples: 2, settlement_jobs: 0 });
        assert!(matches!(stream.pending.as_ref(), Some(work) if work.key() == ahead));
        assert!(!stream.commit_settlement(late));
    }

    #[test]
    fn full_capacity_does_not_evict_a_visible_region() {
        let mut stream = LandStream::<1>::new();
        let world = World::new(1);
        stream.reset(world);
        let old = Key { x: 3, z: -3 };
        let road = RoadNetwork::generate(&world, old.x, old.z).unwrap();
        stream.write_resident(0, old, road, [None, None], true);
        let view =
            Window::new(camera(3830.0, -2640.0, 320.0, core::f32::consts::FRAC_PI_2)).unwrap();
        let other = Key { x: 4, z: -3 };
        assert!(view.visible(old) && view.visible(other));
        assert!(stream.available_slot(view, other).is_none());
        assert_eq!(stream.get(old), 0);
    }
}
