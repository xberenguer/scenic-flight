//! Ready-to-render land scenery. Planning and settlement work stay outside
//! terrain sampling, and a region first becomes visible as a complete unit.

use crate::math::sin_cos;
use crate::render::{view_distance, Camera};
use crate::roads::{RoadNetwork, RoadPaintCache, RoadPlanner, RoadProgress, ROAD_REGION_SIZE};
use crate::scenery::DistrictRegion;
use crate::settlement::{Settlement, Terminus};
use crate::world::{Sample, World};

const LOOKAHEAD: f32 = 1024.0;
const REGION: f32 = ROAD_REGION_SIZE as f32;
const INVERSE_REGION: f32 = 1.0 / REGION;
const REGION_HALF: f32 = REGION * 0.5;
// Roads and their endpoint districts stay at least this far inside their tile.
const SCENERY_MARGIN: f32 = 96.0;
const SCENERY_HALF: f32 = REGION_HALF - SCENERY_MARGIN;
const SIDE: usize = 8;
const SCAN: i32 = 3;
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

pub struct LandStream<const ACTIVE: usize = 8> {
    world: World,
    ready: bool,
    directory: [Record; SIDE * SIDE],
    residents: [Option<Resident>; ACTIVE],
    pending: Option<Pending>,
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
        while let Some(key) = self.next_missing(view, !ahead) {
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
            .is_some_and(|pending| view.rank(pending.key()).is_none());
        let must_prioritize_visible = self.tick & 7 == 0
            && matches!(&self.pending, Some(Pending::Road { key, .. }) if !view.visible(*key))
            && self.next_missing(view, true).is_some();
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
                        });
                        LandProgress::AwaitingSettlement
                    }
                }
            }
            Some(Pending::Towns {
                road, towns, next, ..
            }) => {
                if *next < 2 && budget.settlement_jobs == 0 {
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
                let lower_priority = view.rank(key).is_none_or(|rank| rank > wanted);
                (!view.keep(key) && lower_priority).then_some((i, view.distance_squared(key)))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
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
        self.residents[slot] = Some(Resident {
            key,
            region: DistrictRegion::from_prepared(key.x, key.z, road, towns),
            published,
        });
        self.set(key, slot as u8);
    }

    fn next_missing(&self, view: Window, visible_only: bool) -> Option<Key> {
        let mut candidate: Option<(Key, (u8, f32))> = None;
        for key in view.keys() {
            let Some(rank) = view.rank(key) else { continue };
            if visible_only && rank.0 != 0 || self.get(key) != UNKNOWN {
                continue;
            }
            if candidate.is_none_or(|(_, best)| rank < best) {
                candidate = Some((key, rank));
            }
        }
        candidate.map(|(key, _)| key)
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
            horizon: view_distance(camera.altitude),
            padding: SCENERY_HALF * (sin.abs() + cos.abs()),
            center: Key::at(camera.x, camera.z),
        })
    }

    fn keys(self) -> impl Iterator<Item = Key> {
        (self.center.z - SCAN..=self.center.z + SCAN).flat_map(move |z| {
            (self.center.x - SCAN..=self.center.x + SCAN).map(move |x| Key { x, z })
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
        let near = pos.0 - self.padding;
        let far = pos.0 + self.padding;
        near <= reach && far >= 0.0 && pos.1 - self.padding <= far.min(reach) * 0.72
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
        self.visible(key) || self.distance_squared(key) <= reach * reach
    }

    fn rank(self, key: Key) -> Option<(u8, f32)> {
        let projected = self.projected(key);
        if self.intersects(projected, self.horizon) {
            return Some((0, self.distance_squared(key)));
        }
        if self.intersects(projected, self.horizon + LOOKAHEAD) {
            let entry = (projected.0 - self.padding).max(0.0);
            let across = (projected.1 - self.padding).max(0.0);
            return Some((1, entry + across * 0.48));
        }
        self.keep(key).then(|| (2, self.distance_squared(key)))
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
