//! Prepared river corridors with protected banks and certified highway crossings.

use crate::crossings::{Bridge, Crossings};
use crate::floodplain::Floodplain;
use crate::render::Camera;
use crate::roads::{RoadNetwork, ROAD_HALF_WIDTH};
use crate::scenery::{cover_river_sample, DistrictRegion};
use crate::settlement::{Settlement, Terminus};
use crate::watershed::{Reach, RiverSample, Watershed, WatershedWork, REGION_SIZE};
use crate::world::{Sample, Surface, World, SEA_LEVEL};
use crate::world_mesh::{bounds_in_view, WorldMesh};

// The same road shoulder margin as RoadNetwork::proximity.
const SHOULDER_EDGE: f32 = ROAD_HALF_WIDTH + 1.5;
const BANK_REACH: f32 = 14.0;
const REACH_HALO: f32 = BANK_REACH + 6.25;
const FOOTPRINT_STEP: f32 = 0.58;
// Reject channels that could clip an unspanned pavement edge between samples.
const CHANNEL_MARGIN: f32 = 0.38;
const PAVEMENT_MARGIN: f32 = 0.12;
// Clearance from the deck surface, including the girder's thickness.
const DECK_CLEARANCE: f32 = 0.80;

pub struct RiverDistrict {
    water: Watershed,
    crossings: Crossings,
    floodplain: Floodplain,
    bounds: [f32; 4],
}

impl RiverDistrict {
    /// Prepare the watershed in the region containing this highway. Keep the
    /// reusable watershed scratch and the placement checks outside drawing.
    pub fn prepare(
        world: &World,
        seed: u64,
        district: &DistrictRegion,
        work: &mut WatershedWork,
        blocked: impl FnMut(f32, f32) -> bool,
    ) -> Option<Self> {
        let first = district.road()?.polyline().first()?;
        let mut water = Watershed::new();
        water.rebuild(world, seed, region(first.x), region(first.z), work);
        Self::from_survey_with_detour(world, Some(district), &mut water, work, blocked, &mut || {})
    }

    /// Preserve the established design if any surveyed channel is usable.
    /// In developed regions a failed design gets one terrain-guided detour.
    #[inline(never)]
    pub(crate) fn from_survey_with_detour(
        world: &World,
        district: Option<&DistrictRegion>,
        water: &mut Watershed,
        work: &mut WatershedWork,
        mut blocked: impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let (crossings, bounds) = if let Some(certified) = Self::certify_channels(world, district, water, &mut blocked, pulse) {
            certified
        } else {
            // Reuse the failed survey's storage. The worker also needs room
            // for certification scratch and a result containing the channels.
            let district = district.filter(|_| !water.streams().is_empty())?;
            Self::reroute(world, district, water, work, &mut blocked, pulse)?;
            Self::certify_channels(world, Some(district), water, &mut blocked, pulse)?
        };
        Some(Self::finish(world, district, water, crossings, bounds, &mut blocked, pulse))
    }

    #[inline(never)]
    fn reroute(
        world: &World,
        district: &DistrictRegion,
        water: &mut Watershed,
        work: &mut WatershedWork,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<()> {
        let road = district.road()?;
        let first = road.polyline().first()?;
        let road_bounds = road.bounds();
        let towns = [Terminus::Start, Terminus::End].map(|end| {
            district.town(end).map(|town| (town, town.bounds()))
        });
        water.rebuild_routed(world, water.seed(), (region(first.x), region(first.z)), work, |a, b| {
            let Some(axis) = Axis::between(a, b) else { return false; };
            for distance in [0.0, 0.25, 0.5, 0.75, 1.0] {
                for side in [0.0, -5.0, 5.0] {
                    let [x, z] = axis.point(axis.length * distance, side);
                    if (contains(road_bounds, x, z) && road.proximity(x, z).is_some())
                        || towns.iter().flatten().any(|(town, area)| {
                            contains(*area, x, z) && town.sample(x, z).is_some()
                        })
                        || blocked(x, z)
                    {
                        return false;
                    }
                }
            }
            true
        }, pulse);
        Some(())
    }

    /// Check a retained water plan against the actual highway and towns.
    /// `blocked` reserves additional grounded facilities and infrastructure.
    pub fn from_prepared(
        world: &World,
        district: &DistrictRegion,
        water: Watershed,
        blocked: impl FnMut(f32, f32) -> bool,
    ) -> Option<Self> {
        Self::from_prepared_cooperatively(world, district, water, blocked, &mut || {})
    }

    pub(crate) fn from_prepared_cooperatively(
        world: &World, district: &DistrictRegion, water: Watershed,
        blocked: impl FnMut(f32, f32) -> bool, pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        Self::from_prepared_land_cooperatively(world, Some(district), water, blocked, pulse)
    }

    /// Certify a real watershed against any roads, towns, and reserved facilities.
    pub fn from_prepared_land_cooperatively(
        world: &World,
        district: Option<&DistrictRegion>,
        mut water: Watershed,
        mut blocked: impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let (crossings, bounds) = Self::certify_channels(world, district, &mut water, &mut blocked, pulse)?;
        Some(Self::finish(world, district, &water, crossings, bounds, &mut blocked, pulse))
    }

    #[inline(never)]
    fn certify_channels(
        world: &World,
        district: Option<&DistrictRegion>,
        water: &mut Watershed,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<(Crossings, [f32; 4])> {
        if water.streams().is_empty() {
            return None;
        }
        if let Some(certified) = Self::certify(world, district, water, blocked, pulse) {
            return Some(certified);
        }
        let (mask, crossings, bounds) = Self::certify_subset(world, district, water, blocked, pulse)?;
        *water = water.with_streams(mask);
        Some((crossings, bounds))
    }

    #[inline(never)]
    fn finish(
        world: &World,
        district: Option<&DistrictRegion>,
        water: &Watershed,
        crossings: Crossings,
        bounds: [f32; 4],
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Self {
        let road = district.and_then(DistrictRegion::road);
        let mut proof = Proof {
            world,
            road,
            water,
            crossings: &crossings,
            towns: [Terminus::Start, Terminus::End].map(|end| {
                let town = district?.town(end)?;
                Some((town, town.bounds()))
            }),
            blocked,
        };
        let floodplain = Floodplain::prepare(world, water, |x, z| {
            proof.floodplain_reserved(x, z)
        }, pulse);
        let bounds = floodplain.bounds().map_or(bounds, |area| union(bounds, area));
        Self { water: *water, crossings, floodplain, bounds }
    }

    fn certify_subset(
        world: &World,
        district: Option<&DistrictRegion>,
        water: &Watershed,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<(usize, Crossings, [f32; 4])> {
        let stream_count = water.streams().len();
        let full = (1_usize << stream_count) - 1;
        // Prefer the largest safe network, with original stream order breaking ties.
        for count in (1..stream_count).rev() {
            for mask in 1..full {
                if mask.count_ones() as usize != count {
                    continue;
                }
                pulse();
                let subset = water.with_streams(mask);
                if let Some((crossings, bounds)) = Self::certify(world, district, &subset, blocked, pulse) {
                    return Some((mask, crossings, bounds));
                }
            }
        }
        None
    }

    fn certify(
        world: &World,
        district: Option<&DistrictRegion>,
        water: &Watershed,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<(Crossings, [f32; 4])> {
        let road = district.and_then(DistrictRegion::road);
        let crossings = road.map_or_else(Crossings::new, |road| {
            Crossings::fit_cooperatively(world, road, Some(water), false, pulse)
        });
        let mut proof = Proof {
            world,
            road,
            water,
            crossings: &crossings,
            towns: [Terminus::Start, Terminus::End].map(|end| {
                let town = district?.town(end)?;
                Some((town, town.bounds()))
            }),
            blocked,
        };
        let mut bounds = None;
        for bridge in crossings.bridges() {
            if !proof.bridge_clear(bridge, pulse) {
                return None;
            }
            bounds = Some(bounds.map_or(bridge.bounds(), |b| union(b, bridge.bounds())));
        }
        for reach in water.reaches() {
            if !proof.reach_clear(reach, pulse) {
                return None;
            }
            let area = [
                reach.start[0].min(reach.end[0]) - REACH_HALO,
                reach.start[1].min(reach.end[1]) - REACH_HALO,
                reach.start[0].max(reach.end[0]) + REACH_HALO,
                reach.start[1].max(reach.end[1]) + REACH_HALO,
            ];
            bounds = Some(bounds.map_or(area, |b| union(b, area)));
        }
        Some((crossings, bounds?))
    }

    pub fn watershed(&self) -> &Watershed {
        &self.water
    }

    pub fn crossings(&self) -> &Crossings {
        &self.crossings
    }

    pub fn floodplain(&self) -> &Floodplain {
        &self.floodplain
    }

    /// Minimum x, minimum z, maximum x, maximum z. Includes dry approaches.
    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    /// Compose over prepared dry roads and towns. Let water pass below its
    /// separate bridge mesh and keep the road material on all dry shoulders.
    pub fn overlay(&self, x: f32, z: f32, developed: Sample) -> Sample {
        if !contains(self.bounds, x, z) {
            return developed;
        }
        self.overlay_sampled(x, z, developed, self.water.sample(x, z))
    }

    /// Accept the frame sampler's existing channel query and compose the bank.
    #[inline]
    pub fn overlay_sampled(
        &self, x: f32, z: f32, developed: Sample, water: Option<RiverSample>,
    ) -> Sample {
        let covered = water.map_or(developed, |water| Self::cover(water, developed));
        let bank = self.floodplain.overlay(x, z, covered, water);
        let bank = self.floodplain.overlay_point_bar(x, z, developed, bank, water);
        self.floodplain.overlay_island(x, z, developed, bank)
    }

    /// Conservatively reserve individual river reaches and wet terraces.
    pub fn intersects_scenery(&self, area: [f32; 4]) -> bool {
        if !area.into_iter().all(f32::is_finite) || area[0] > area[2] || area[1] > area[3]
            || !overlaps(self.bounds, area)
        {
            return false;
        }
        self.floodplain.intersects(area)
            || self.crossings.bridges().iter().any(|bridge| overlaps(bridge.bounds(), area))
            || self.bank_chords().any(|(a, b)| crosses_expanded_area(a, b, area, BANK_REACH + 1.0))
    }

    pub(crate) fn scenery_rectangles(&self) -> impl Iterator<Item = [f32; 4]> + '_ {
        self.floodplain.sites().map(|(_, bounds)| bounds)
            .chain(self.crossings.bridges().iter().map(|bridge| bridge.bounds()))
    }

    pub(crate) fn bank_chords(&self) -> impl Iterator<Item = ([f32; 2], [f32; 2])> + '_ {
        self.water.reaches().flat_map(|reach| {
            (1..=8).scan(reach.start, move |previous, step| {
                let next = reach.position(step as f32 / 8.0);
                Some((core::mem::replace(previous, next), next))
            })
        })
    }

    pub(crate) fn bank_intersects(&self, a: [f32; 2], b: [f32; 2], area: [f32; 4]) -> bool {
        overlaps(self.bounds, area) && crosses_expanded_area(a, b, area, BANK_REACH + 1.0)
    }

    #[inline]
    pub(crate) fn cover(water: RiverSample, developed: Sample) -> Sample {
        let river = cover_river_sample(water, developed);
        if river.surface != Surface::RiverWater
            && matches!(developed.surface, Surface::Pavement | Surface::Gravel)
        {
            return developed;
        }
        river
    }

    /// Vehicle surface or clearance including the short dry approaches.
    pub fn deck_height(&self, x: f32, z: f32) -> Option<f32> {
        self.crossings
            .bridges()
            .iter()
            .find_map(|bridge| bridge.road_height(x, z))
    }

    /// Draw after the water and natural ground have established terrain depth.
    pub fn draw(&self, camera: Camera, mesh: &mut WorldMesh<'_>) {
        for bridge in self.crossings.bridges() {
            if bounds_in_view(camera, bridge.bounds()) {
                bridge.draw(mesh);
            }
        }
    }
}

struct Proof<'a, Blocked> {
    world: &'a World,
    road: Option<&'a RoadNetwork>,
    water: &'a Watershed,
    crossings: &'a Crossings,
    towns: [Option<(&'a Settlement, [f32; 4])>; 2],
    blocked: Blocked,
}

impl<Blocked: FnMut(f32, f32) -> bool> Proof<'_, Blocked> {
    fn reserved(&mut self, x: f32, z: f32) -> bool {
        (self.blocked)(x, z)
            || self
                .towns
                .iter()
                .flatten()
                .any(|(town, bounds)| contains(*bounds, x, z) && town.sample(x, z).is_some())
    }

    fn floodplain_reserved(&mut self, x: f32, z: f32) -> bool {
        self.road.is_some_and(|road| road.proximity(x, z).is_some())
            || self.crossings.bridges().iter().any(|bridge| contains(bridge.bounds(), x, z))
            || self.reserved(x, z)
    }

    fn bridge_clear(&mut self, bridge: &Bridge, pulse: &mut impl FnMut()) -> bool {
        let Some(road) = self.road else { return false };
        let [a, b, c, d] = *bridge.sections();
        let Some(axis) = Axis::between([a[0], a[2]], [d[0], d[2]]) else {
            return false;
        };
        let Some(span) = Axis::between([b[0], b[2]], [c[0], c[2]]) else {
            return false;
        };
        // Keep the abutments, guardrails and ramps out of protected land use.
        let width = bridge.width() * 0.5 + 0.34;
        let steps = steps(axis.length);
        let lanes = steps_for_width(width);
        for row in 0..=steps {
            pulse();
            let along = axis.length * row as f32 / steps as f32;
            for side in -lanes..=lanes {
                let [x, z] = axis.point(along, side as f32 * width / lanes as f32);
                if self.reserved(x, z) {
                    return false;
                }
            }
        }
        // Keep the entire bridge aligned to the retained pavement where it
        // joins the terrain, with dry shoulders reaching both dry ramps.
        let [dx, dz] = span.unit;
        for [x, y, z] in [a, d] {
            if road.proximity(x, z)
                .is_none_or(|road| road.distance_squared > 0.01)
                || (self.world.elevation(x, z) - y).abs() > 0.01
            {
                return false;
            }
            for side in [-SHOULDER_EDGE, 0.0, SHOULDER_EDGE] {
                let sx = x - side * dz;
                let sz = z + side * dx;
                if self.world.elevation(sx, sz) <= SEA_LEVEL
                    || self
                        .water
                        .sample(sx, sz)
                        .is_some_and(|river| river.is_channel())
                {
                    return false;
                }
            }
        }
        true
    }

    fn reach_clear(&mut self, reach: Reach, pulse: &mut impl FnMut()) -> bool {
        let Some(axis) = Axis::between(reach.start, reach.end) else {
            return false;
        };
        // Use the same linear parameter and normal as Watershed's sampler.
        // Rounded caps cover the ground beyond a reach's two endpoints.
        let rows = steps(axis.length + 2.0 * BANK_REACH);
        let dz = (axis.length + 2.0 * BANK_REACH) / rows as f32;
        let lanes = steps_for_width(BANK_REACH);
        let dx = BANK_REACH / lanes as f32;
        let road_bounds = self.road.map(RoadNetwork::bounds);
        let [fx, fz] = axis.unit;
        let decks = self.crossings.bridges();
        for row in 0..=rows {
            pulse();
            let along = row as f32 * dz - BANK_REACH;
            let t = (along / axis.length).clamp(0.0, 1.0);
            let tail = along - t * axis.length;
            let [mx, mz] = reach.position(t);
            let px = mx + fx * tail;
            let pz = mz + fz * tail;
            for lane in -lanes..=lanes {
                let side = lane as f32 * dx;
                if side * side + tail * tail > (BANK_REACH + 0.01) * (BANK_REACH + 0.01) {
                    continue;
                }
                let x = px - side * fz;
                let z = pz + side * fx;
                let road_bounds_hit = road_bounds.is_some_and(|area| contains(area, x, z));
                let reserved = self.reserved(x, z);
                if !road_bounds_hit && !reserved {
                    continue;
                }
                let Some(river) = self.water.sample(x, z) else {
                    continue;
                };
                let wet = river.is_channel();
                if reserved && (wet || river.bank_strength >= 0.35) {
                    let ground = self.world.elevation(x, z);
                    if ground > SEA_LEVEL && (wet || (0.0..4.0).contains(&(ground - river.level))) {
                        return false;
                    }
                }
                if !road_bounds_hit || river.distance > river.half_width + CHANNEL_MARGIN {
                    continue;
                }
                let Some(lane) = self.road.and_then(|road| road.proximity(x, z)) else {
                    continue;
                };
                let wide_pavement = ROAD_HALF_WIDTH + PAVEMENT_MARGIN;
                let pavement = lane.distance_squared <= wide_pavement * wide_pavement;
                if !decks
                    .iter()
                    .any(|bridge| deck_covers(bridge, x, z, pavement, river.level))
                {
                    return false;
                }
            }
        }
        true
    }
}

fn deck_covers(bridge: &Bridge, x: f32, z: f32, pavement: bool, water: f32) -> bool {
    let [_, a, b, _] = *bridge.sections();
    let [dx, dz] = [b[0] - a[0], b[2] - a[2]];
    let [px, pz] = [x - a[0], z - a[2]];
    let along = px * dx + pz * dz;
    let length_squared = dx * dx + dz * dz;
    if !(0.0..=length_squared).contains(&along) {
        return false;
    }
    if pavement {
        bridge
            .road_height(x, z)
            .is_some_and(|deck| deck - water >= DECK_CLEARANCE)
    } else {
        let side = px * dz - pz * dx;
        // Gravel ends at the bank beside the bridge instead of drawing
        // shoulders across the open channel.
        side * side <= (SHOULDER_EDGE + 0.02) * (SHOULDER_EDGE + 0.02) * length_squared
    }
}

struct Axis {
    origin: [f32; 2],
    unit: [f32; 2],
    length: f32,
}

impl Axis {
    fn between(a: [f32; 2], b: [f32; 2]) -> Option<Self> {
        let [dx, dz] = [b[0] - a[0], b[1] - a[1]];
        let mut length = dx.abs().max(dz.abs());
        if length <= 0.01 || !length.is_finite() {
            return None;
        }
        let squared = dx * dx + dz * dz;
        for _ in 0..4 {
            length = (length + squared / length) * 0.5;
        }
        Some(Self {
            origin: a,
            unit: [dx / length, dz / length],
            length,
        })
    }

    fn point(&self, along: f32, across: f32) -> [f32; 2] {
        let [x, z] = self.origin;
        let [dx, dz] = self.unit;
        [x + dx * along - dz * across, z + dz * along + dx * across]
    }
}

fn region(value: f32) -> i32 {
    let scaled = value / REGION_SIZE as f32;
    let whole = scaled as i32;
    whole.saturating_sub(i32::from(scaled < whole as f32))
}

fn steps(extent: f32) -> usize {
    (extent / FOOTPRINT_STEP) as usize + 1
}

fn steps_for_width(extent: f32) -> i32 {
    (extent / FOOTPRINT_STEP) as i32 + 1
}

fn contains([min_x, min_z, max_x, max_z]: [f32; 4], x: f32, z: f32) -> bool {
    (min_x..=max_x).contains(&x) && (min_z..=max_z).contains(&z)
}

fn overlaps(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

fn crosses_expanded_area(a: [f32; 2], b: [f32; 2], area: [f32; 4], pad: f32) -> bool {
    let mut first = 0.0_f32;
    let mut last = 1.0_f32;
    for axis in 0..2 {
        let min = area[axis] - pad;
        let max = area[axis + 2] + pad;
        let length = b[axis] - a[axis];
        if length == 0.0 {
            if !(min..=max).contains(&a[axis]) {
                return false;
            }
            continue;
        }
        let begin = (min - a[axis]) / length;
        let end = (max - a[axis]) / length;
        first = first.max(begin.min(end));
        last = last.min(begin.max(end));
        if first > last {
            return false;
        }
    }
    true
}

fn union(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crossings::BridgeWater;
    use crate::render::render_with_depth;
    use crate::sky::Sky;

    fn fixture(seed: u64, rx: i32, rz: i32) -> (World, DistrictRegion, RiverDistrict) {
        let world = World::new(seed);
        let district = DistrictRegion::build(&world, rx, rz);
        let river = RiverDistrict::prepare(
            &world,
            seed,
            &district,
            &mut WatershedWork::new(),
            |_, _| false,
        )
        .unwrap();
        (world, district, river)
    }

    #[test]
    fn unsafe_tributaries_do_not_remove_a_clear_river_by_town() {
        let seed = 49;
        let world = World::new(seed);
        let district = DistrictRegion::build(&world, 0, 3);
        let mut complete = Watershed::new();
        complete.rebuild(&world, seed, 0, 3, &mut WatershedWork::new());
        assert_eq!(complete.streams().len(), 3);
        assert!(district.towns().next().is_some());
        assert!(RiverDistrict::certify(&world, Some(&district), &complete, &mut |_, _| false, &mut || {}).is_none());

        let plan = RiverDistrict::from_prepared(&world, &district, complete, |_, _| false)
            .expect("the conflicting paths should leave a safe source-to-outlet river");
        let kept = plan.watershed();
        assert_eq!(kept.streams().len(), 1);
        assert!(complete.streams().contains(&kept.streams()[0]));
        assert_eq!(kept.reaches().count(), usize::from(kept.streams()[0].links_to_sea));
        assert_eq!(kept.reaches().filter(|reach| reach.reaches_sea).count(), 1);
        for reach in kept.reaches() {
            let [x, z] = reach.position(0.5);
            assert!(kept.sample(x, z).is_some_and(RiverSample::is_channel));
        }
        for stream in complete.streams().iter().filter(|stream| !kept.streams().contains(stream)) {
            let [x, z] = stream.source;
            assert!(kept.sample(x, z).is_none());
        }
    }

    #[test]
    fn a_conflicted_river_can_find_a_downhill_route_around_town() {
        let seed = 49;
        let world = World::new(seed);
        let district = DistrictRegion::build(&world, 0, 1);
        let mut work = WatershedWork::new();
        let mut original = Watershed::new();
        original.rebuild(&world, seed, 0, 1, &mut work);
        assert!(!original.streams().is_empty());
        assert!(RiverDistrict::from_prepared(&world, &district, original, |_, _| false).is_none());

        let plan = RiverDistrict::prepare(&world, seed, &district, &mut work, |_, _| false).unwrap();
        let water = plan.watershed();
        assert!(!water.streams().is_empty());
        for outlet in water.streams().iter().map(|stream| stream.outlet) {
            assert!(world.elevation(outlet[0], outlet[1]) <= SEA_LEVEL);
        }
        for reach in water.reaches() {
            assert!(reach.end_level <= reach.start_level);
            for step in 0..=16 {
                let [x, z] = reach.position(step as f32 / 16.0);
                let ground = world.sample(x, z);
                assert_eq!(district.overlay(x, z, ground), ground);
                assert!(water.sample(x, z).is_some_and(RiverSample::is_channel));
            }
        }
    }

    #[test]
    fn natural_rivers_publish_without_bridges_and_keep_their_channel_profile() {
        let seed = 14_692;
        let world = World::new(seed);
        let mut work = WatershedWork::new();
        for (rx, rz, has_road) in [(2, 0, false), (-6, -5, true)] {
            let district = DistrictRegion::build(&world, rx, rz);
            assert_eq!(district.road().is_some(), has_road);
            let mut water = Watershed::new();
            water.rebuild(&world, seed, rx, rz, &mut work);
            let plan = RiverDistrict::from_prepared_land_cooperatively(
                &world, has_road.then_some(&district), water, |_, _| false, &mut || {},
            ).unwrap();
            assert!(plan.crossings().bridges().is_empty());
            assert!(plan.floodplain().sites().any(|(kind, _)|
                kind == crate::floodplain::FloodplainKind::ReedMarsh));
            for (_, [left, near, right, far]) in plan.floodplain().sites() {
                assert_eq!([region(left), region(near)], [rx, rz]);
                assert_eq!([region(right), region(far)], [rx, rz]);
            }
            for reach in plan.watershed().reaches() {
                for i in 1..16 {
                    let [x, z] = reach.position(i as f32 / 16.0);
                    let raw = world.sample(x, z);
                    let ground = district.overlay(x, z, raw);
                    let water = plan.watershed().sample(x, z).unwrap();
                    assert!(water.is_channel());
                    assert!(plan.intersects_scenery([x, z, x, z]));
                    let covered = RiverDistrict::cover(water, ground);
                    let sample = plan.overlay(x, z, ground);
                    if plan.floodplain().island_bounds().is_some_and(|bounds| contains(bounds, x, z))
                        && sample != covered
                    {
                        assert_eq!(covered.surface, Surface::RiverWater);
                        assert!(matches!(sample.surface, Surface::RiverSilt | Surface::WetMeadow | Surface::ReedMarsh));
                        assert!((covered.elevation..covered.elevation + 0.96).contains(&sample.elevation));
                    } else {
                        assert_eq!(sample, covered);
                    }
                }
            }
            if let Some(road) = district.road() {
                for pair in road.polyline().windows(2) {
                    let a = pair[0];
                    let b = pair[1];
                    let axis = Axis::between([a.x, a.z], [b.x, b.z]).unwrap();
                    for step in 0..=steps(axis.length) {
                        let distance = axis.length * step as f32 / steps(axis.length) as f32;
                        for side in [-4.4, -3.0, 0.0, 3.0, 4.4] {
                            let [x, z] = axis.point(distance, side);
                            let ground = district.overlay(x, z, world.sample(x, z));
                            if matches!(ground.surface, Surface::Pavement | Surface::Gravel) {
                                assert_eq!(plan.overlay(x, z, ground), ground);
                            }
                        }
                    }
                }
            }
            let area = plan.bounds();
            assert!((1..8).any(|j| (1..8).any(|i| {
                let x = area[0] + (area[2] - area[0]) * i as f32 * 0.125;
                let z = area[1] + (area[3] - area[1]) * j as f32 * 0.125;
                !plan.intersects_scenery([x, z, x, z])
            })));
        }
        assert!(RiverDistrict::from_prepared_land_cooperatively(
            &world, None, Watershed::new(), |_, _| false, &mut || {},
        ).is_none());
    }

    #[test]
    fn dense_road_ribbon_has_water_under_deck_and_dry_connected_shoulders() {
        for (seed, rx, rz, count) in [(1, 1, -1, 1), (98, 1, 2, 2)] {
            let (world, district, river) = fixture(seed, rx, rz);
            let road = district.road().unwrap();
            assert_eq!(river.crossings().bridges().len(), count);
            let mut wet_pavement = 0;
            let mut wet_shoulder = 0;
            let mut check = |x, z| {
                let Some(lane) = road.proximity(x, z) else {
                    return;
                };
                let raw = world.sample(x, z);
                let developed = district.overlay(x, z, raw);
                let overlay = river.overlay(x, z, developed);
                if overlay.surface != Surface::RiverWater {
                    assert_eq!(overlay, developed);
                    return;
                }
                let pavement = lane.distance_squared <= ROAD_HALF_WIDTH * ROAD_HALF_WIDTH;
                assert!(river
                    .crossings()
                    .bridges()
                    .iter()
                    .any(|bridge| { deck_covers(bridge, x, z, pavement, overlay.elevation) }));
                if pavement {
                    assert!(river.deck_height(x, z).unwrap() > overlay.elevation + 0.8);
                    wet_pavement += 1;
                } else {
                    wet_shoulder += 1;
                }
            };
            for segment in road.polyline().windows(2) {
                let [a, b] = [segment[0], segment[1]];
                let axis = Axis::between([a.x, a.z], [b.x, b.z]).unwrap();
                let samples = (axis.length * 4.0) as usize + 1;
                for i in 0..=samples {
                    let d = axis.length * i as f32 / samples as f32;
                    for side in -18..=18 {
                        let [x, z] = axis.point(d, side as f32 * 0.25);
                        check(x, z);
                    }
                }
            }
            for point in road.polyline() {
                for x in -18..=18 {
                    for z in -18..=18 {
                        check(point.x + x as f32 * 0.25, point.z + z as f32 * 0.25);
                    }
                }
            }
            assert!(wet_pavement >= count * 240);
            assert!(wet_shoulder >= count * 80);
            for bridge in river.crossings().bridges() {
                assert_eq!(bridge.water, BridgeWater::River);
                let [a, _, _, b] = *bridge.sections();
                for [x, height, z] in [a, b] {
                    assert_eq!(
                        district.overlay(x, z, world.sample(x, z)).surface,
                        Surface::Pavement
                    );
                    assert!((river.deck_height(x, z).unwrap() - height).abs() < 0.001);
                    assert!((world.elevation(x, z) - height).abs() < 0.001);
                }
            }
        }
    }

    #[test]
    fn external_land_can_exclude_water_or_bridge_work() {
        let (world, district, river) = fixture(1, 1, -1);
        let mut work = WatershedWork::new();
        let bridge = river.crossings().bridges()[0].sections()[1];
        let [rx, rz] = river.watershed().streams()[0].source;
        for [x, z] in [[bridge[0], bridge[2]], [rx, rz]] {
            let reserved = |px: f32, pz: f32| (px - x).abs() <= 1.2 && (pz - z).abs() <= 1.2;
            if let Some(retained) = RiverDistrict::prepare(&world, 1, &district, &mut work, reserved) {
                assert!(retained.watershed().streams().len() < river.watershed().streams().len());
                for dz in [-1.0, 0.0, 1.0] {
                    for dx in [-1.0, 0.0, 1.0] {
                        let (px, pz) = (x + dx, z + dz);
                        let ground = district.overlay(px, pz, world.sample(px, pz));
                        assert_eq!(retained.overlay(px, pz, ground), ground);
                        assert!(retained.deck_height(px, pz).is_none());
                    }
                }
            }
        }
        let far = RiverDistrict::prepare(&world, 1, &district, &mut work, |px, pz| {
            (px - 200.0).abs() < 8.0 && (pz + 5000.0).abs() < 8.0
        })
        .unwrap();
        assert_eq!(far.bounds(), river.bounds());
        assert_eq!(far.crossings(), river.crossings());
    }

    #[test]
    fn bridge_plan_can_still_be_rejected_for_an_unspanned_shoulder() {
        let seed = 16;
        let world = World::new(seed);
        let district = DistrictRegion::build(&world, 9, -3);
        let road = district.road().unwrap();
        let mut water = Watershed::new();
        water.rebuild(&world, seed, 9, -3, &mut WatershedWork::new());
        let crossing = Crossings::fit(&world, road, Some(&water), false);
        assert_eq!(crossing.bridges().len(), 1);
        let [x, z] = [9872.0, -2732.5];
        assert!(water.sample(x, z).unwrap().is_channel());
        assert!(road.proximity(x, z).unwrap().distance_squared > ROAD_HALF_WIDTH * ROAD_HALF_WIDTH);
        assert!(!deck_covers(&crossing.bridges()[0], x, z, false, 2.9));
        assert!(RiverDistrict::from_prepared(&world, &district, water, |_, _| false).is_none());
    }

    #[test]
    fn native_bridge_mesh_writes_only_closer_depth() {
        const SIDE: usize = 112;
        const PIXELS: usize = SIDE * SIDE;
        let (world, district, river) = fixture(1, 1, -1);
        let camera = Camera {
            x: 1673.0,
            altitude: 19.0,
            z: -592.0,
            heading: 0.29,
            pitch: 0.32,
            bank: 0.0,
        };
        let sky = Sky::from_time(1, 0, 0.51);
        let mut pixels = [0; PIXELS];
        let mut depth = [0; PIXELS];
        let mut terrain = [0; PIXELS];
        render_with_depth(
            |x, z| river.overlay(x, z, district.overlay(x, z, world.sample(x, z))),
            camera,
            &sky,
            &mut pixels,
            &mut terrain,
            &mut depth,
            (SIDE, SIDE),
        )
        .unwrap();
        let before_color = pixels;
        let before_depth = depth;
        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
        river.draw(camera, &mut mesh);
        let changed = pixels
            .iter()
            .zip(before_color)
            .filter(|(&color, old)| color != *old)
            .count();
        assert!(changed >= 40, "only {changed} native bridge pixels");
        assert!(depth.iter().zip(before_depth).all(|(&a, b)| a <= b));
    }
}
