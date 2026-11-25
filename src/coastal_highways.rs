//! Detachable planning for a dry highway with one proven marine crossing.

use crate::crossings::Bridge;
use crate::highway_links::{HighwayContact, HighwaySurvey};
use crate::render::{screen_region, Camera, ScreenRect};
use crate::roads::{RoadNetwork, RoadPoint, ROAD_HALF_WIDTH};
use crate::settlement::SettlementRoadExit;
use crate::world::{Sample, Surface, World, SEA_LEVEL};
use crate::world_mesh::WorldMesh;

type Point = [f32; 2];
type Area = [f32; 4];

const SHOULDER: f32 = ROAD_HALF_WIDTH + 1.5;
const APPROACH_GRADE: f32 = 0.12;
const LANES: usize = 51;
const HEADING_STEPS: usize = 5;
const MIDDLE: usize = LANES / 2;
const OFFSET: f32 = 3.5;
const MAX_STEPS: usize = 116;
const MAX_POINTS: usize = 136;
const TRAIL_ROW: usize = (LANES * HEADING_STEPS).div_ceil(2);
const MAX_BRIDGE_PROOFS: usize = 12;
const MAX_APPROACH_TRIALS: usize = 3;
const MAX_EDGE_PROOFS: u32 = 26_000;
const EMPTY_POINT: RoadPoint = RoadPoint {
    x: 0.0,
    z: 0.0,
    elevation: 0.0,
};

// These guide curves locate a crossing only. Neither dry approach is built
// from them. Its separately surveyed route can move substantially inland.
const GUIDE_OFFSETS: [(f32, f32); 19] = [
    (0.0, 0.0),
    (36.0, 0.0),
    (-36.0, 0.0),
    (70.0, 0.0),
    (-70.0, 0.0),
    (0.0, 120.0),
    (0.0, -120.0),
    (110.0, 0.0),
    (-110.0, 0.0),
    (160.0, 0.0),
    (-160.0, 0.0),
    (55.0, 125.0),
    (55.0, -125.0),
    (-55.0, 125.0),
    (-55.0, -125.0),
    (110.0, 145.0),
    (110.0, -145.0),
    (-110.0, 145.0),
    (-110.0, -145.0),
];

/// External worker scratch. One packed predecessor and two rolling cost rows
/// suffice, so the searches never allocate and the task stack stays small.
pub struct CoastalHighwayWork {
    trail: [[u8; TRAIL_ROW]; MAX_STEPS + 1],
    costs: [[[f32; HEADING_STEPS]; LANES]; 2],
    edge_checks: u32,
}

impl CoastalHighwayWork {
    pub const fn new() -> Self {
        Self {
            trail: [[0; TRAIL_ROW]; MAX_STEPS + 1],
            costs: [[[0.0; HEADING_STEPS]; LANES]; 2],
            edge_checks: 0,
        }
    }
}

/// The same canonical regional endpoints used by a dry highway survey. It
/// contains no borrowed land and remains valid while the worker is detached.
#[derive(Clone, Copy)]
pub struct CoastalHighwaySurvey {
    highway: HighwaySurvey,
    axis: Point,
    distance: f32,
    guide_steps: usize,
}

#[derive(Default)]
struct BridgeSearchBudget {
    proofs: usize,
    approaches: usize,
}

impl CoastalHighwaySurvey {
    pub fn between(a: &RoadNetwork, b: &RoadNetwork) -> Option<Self> {
        Self::from_highway(HighwaySurvey::between(a, b)?)
    }

    pub(crate) fn from_highway(highway: HighwaySurvey) -> Option<Self> {
        if !highway.owns_termini() {
            return None;
        }
        let [a, b] = highway.contacts.map(|contact| xz(contact.point));
        let delta = subtract(b, a);
        let distance = length(delta);
        if !(100.0..=850.0).contains(&distance) || delta[highway.index] < distance * 0.60 {
            return None;
        }
        let axis = scale(delta, 1.0 / distance);
        if dot(axis, highway.outgoing) < 0.50 || dot(axis, highway.inward) < 0.50 {
            return None;
        }
        Some(Self {
            highway,
            axis,
            distance,
            guide_steps: ceil_count(distance * 0.5),
        })
    }

    /// Cheap terrain-only gate. It does not claim an inlet or run a bridge
    /// or road search. The worker can call `prepare` directly to avoid doing
    /// this scan twice when its result is already going to be needed.
    pub fn has_marine_candidate(&self, world: &World, mut pulse: impl FnMut()) -> bool {
        GUIDE_OFFSETS
            .into_iter()
            .any(|offsets| self.wet_run(world, offsets, &mut pulse).is_some())
    }

    /// Fit one raised marine span, then independently find and certify a dry
    /// approach at each end. Both callbacks are evaluated during cold work.
    pub fn prepare(
        &self,
        world: &World,
        work: &mut CoastalHighwayWork,
        mut blocked: impl FnMut(f32, f32) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<CoastalHighway> {
        work.edge_checks = 0;
        self.prepare_guided(
            self, world, work, &mut BridgeSearchBudget::default(), &mut blocked, &mut pulse,
        )
    }

    /// Keep the original district-road ownership while taking the dry
    /// approaches from detached, already connected perimeter street caps.
    pub fn prepare_with_caps(
        &self,
        world: &World,
        work: &mut CoastalHighwayWork,
        caps: [Option<SettlementRoadExit>; 2],
        mut blocked: impl FnMut(f32, f32) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<CoastalHighway> {
        if caps == [None, None] {
            return self.prepare(world, work, blocked, pulse);
        }
        for cap in caps.into_iter().flatten() {
            let [x, z] = xz(cap.point);
            if ![x, z, cap.point.elevation].into_iter().all(f32::is_finite)
                || !(0.98..=1.02).contains(&dot(cap.outward, cap.outward))
                || (world.elevation(x, z) - cap.point.elevation).abs() > 0.002
            {
                return None;
            }
        }
        let mut highway = self.highway;
        if let Some(cap) = caps[0] {
            highway.contacts[0].point = cap.point;
            highway.outgoing = cap.outward;
        }
        if let Some(cap) = caps[1] {
            highway.contacts[1].point = cap.point;
            highway.inward = scale(cap.outward, -1.0);
        }
        let adapted = Self::from_highway(highway)?;
        let mut budget = BridgeSearchBudget::default();
        work.edge_checks = 0;
        let mut result = adapted
            .prepare_guided(
                &adapted, world, work, &mut budget, &mut blocked, &mut pulse,
            )
            .or_else(|| {
                adapted.prepare_guided(
                    self, world, work, &mut budget, &mut blocked, &mut pulse,
                )
            })?;
        result.contacts = self.highway.contacts;
        result.street_caps = caps.map(|cap| cap.map(|exit| exit.point));
        Some(result)
    }

    fn prepare_guided(
        &self,
        guide: &Self,
        world: &World,
        work: &mut CoastalHighwayWork,
        budget: &mut BridgeSearchBudget,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<CoastalHighway> {
        for offsets in GUIDE_OFFSETS {
            let Some([wet_first, wet_last]) = guide.wet_run(world, offsets, pulse) else {
                continue;
            };
            for margin in [12, 18, 24] {
                if wet_first < margin || wet_last + margin >= guide.guide_steps {
                    continue;
                }
                if budget.proofs == MAX_BRIDGE_PROOFS
                    || budget.approaches == MAX_APPROACH_TRIALS
                    || work.edge_checks >= MAX_EDGE_PROOFS
                {
                    return None;
                }
                let a = guide.guide(offsets, wet_first - margin);
                let b = guide.guide(offsets, wet_last + margin);
                if length(subtract(b, a)) > 128.0 {
                    continue;
                }
                budget.proofs += 1;
                let Some(bridge) = Bridge::fit_sea_narrows(world, a, b, &mut *blocked, &mut *pulse)
                else {
                    continue;
                };
                budget.approaches += 1;
                if let Some(result) = self.join(world, work, bridge, blocked, pulse) {
                    return Some(result);
                }
            }
        }
        None
    }

    fn guide(&self, (shift, skew): (f32, f32), index: usize) -> Point {
        let [a, b] = self.highway.contacts.map(|contact| xz(contact.point));
        let t = index as f32 / self.guide_steps as f32;
        let u = 1.0 - t;
        let blend = t * t * (3.0 - 2.0 * t);
        let handle = (self.distance * 0.19).min(72.0);
        let near = 3.0 * u * u * t * handle;
        let far = 3.0 * u * t * t * handle;
        let sideways = 16.0 * (shift + skew * (t - u)) * t * t * u * u;
        [
            a[0] + (b[0] - a[0]) * blend + self.highway.outgoing[0] * near
                - self.highway.inward[0] * far
                - self.axis[1] * sideways,
            a[1] + (b[1] - a[1]) * blend + self.highway.outgoing[1] * near
                - self.highway.inward[1] * far
                + self.axis[0] * sideways,
        ]
    }

    fn wet_run(
        &self,
        world: &World,
        offsets: (f32, f32),
        pulse: &mut impl FnMut(),
    ) -> Option<[usize; 2]> {
        let mut start = None;
        let mut run = None;
        for index in 0..=self.guide_steps {
            if index & 15 == 0 {
                pulse();
            }
            let [x, z] = self.guide(offsets, index);
            if !self.highway.inside_seam(x, z) {
                return None;
            }
            if world.coast_signal(x, z) <= 0.0 {
                if run.is_some() {
                    return None;
                }
                if start.is_none() {
                    start = Some(index);
                }
            } else if let Some(first) = start.take() {
                if first < 10
                    || index + 10 >= self.guide_steps
                    || length(subtract([x, z], self.guide(offsets, first))) > 36.5
                {
                    return None;
                }
                run = Some([first, index]);
            }
        }
        if start.is_some() {
            None
        } else {
            run
        }
    }

    fn join(
        &self,
        world: &World,
        work: &mut CoastalHighwayWork,
        bridge: Bridge,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<CoastalHighway> {
        let sections = *bridge.sections();
        let ramps =
            [sections[0], sections[3]].map(|[x, elevation, z]| RoadPoint { x, z, elevation });
        if !sections
            .into_iter()
            .all(|[x, _, z]| self.highway.inside_seam(x, z))
        {
            return None;
        }
        let along = normalized(subtract(xz(ramps[1]), xz(ramps[0])))?;
        let mut points = [EMPTY_POINT; MAX_POINTS];
        let [a, b] = self.highway.contacts.map(|contact| contact.point);
        let first = Search::new(
            world,
            &self.highway,
            a,
            self.highway.outgoing,
            ramps[0],
            along,
        )?;
        let last = Search::new(
            world,
            &self.highway,
            b,
            scale(self.highway.inward, -1.0),
            ramps[1],
            scale(along, -1.0),
        )?;
        let split = first.steps + 3;
        let total = split + last.steps + 3;
        if total > points.len() {
            return None;
        }
        first.solve(work, &mut points[..split], blocked, pulse)?;
        last.solve(work, &mut points[split..total], blocked, pulse)?;
        let paths = [
            Approach::new(&points[..split], first),
            Approach::new(&points[split..total], last),
        ];
        let bounds = union(union(paths[0].bounds, paths[1].bounds), bridge.bounds());
        Some(CoastalHighway {
            contacts: self.highway.contacts,
            street_caps: [None; 2],
            bridge,
            points,
            split: split as u8,
            total: total as u8,
            paths,
            bounds,
        })
    }
}

#[derive(Clone, Copy)]
struct Search<'a> {
    world: &'a World,
    highway: &'a HighwaySurvey,
    road: RoadPoint,
    ramp: RoadPoint,
    depart: Point,
    arrive: Point,
    start: Point,
    end: Point,
    axis: Point,
    distance: f32,
    steps: usize,
}

impl<'a> Search<'a> {
    fn new(
        world: &'a World,
        highway: &'a HighwaySurvey,
        road: RoadPoint,
        depart: Point,
        ramp: RoadPoint,
        arrive: Point,
    ) -> Option<Self> {
        let start = add(xz(road), scale(depart, 4.0));
        let end = subtract(xz(ramp), scale(arrive, 4.0));
        let delta = subtract(end, start);
        let distance = length(delta);
        let steps = ceil_count(distance / 7.0);
        if steps == 0 || steps > MAX_STEPS {
            return None;
        }
        Some(Self {
            world,
            highway,
            road,
            ramp,
            depart,
            arrive,
            start,
            end,
            axis: scale(delta, 1.0 / distance),
            distance,
            steps,
        })
    }

    fn node(self, row: usize, lane: usize) -> Point {
        let p = interpolate(self.start, self.end, row as f32 / self.steps as f32);
        add(
            p,
            scale(perp(self.axis), (lane as f32 - MIDDLE as f32) * OFFSET),
        )
    }

    fn solve(
        self,
        work: &mut CoastalHighwayWork,
        out: &mut [RoadPoint],
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<()> {
        if !self.strip(xz(self.road), self.start, 0.55, blocked, pulse)
            || !self.strip(self.end, xz(self.ramp), 0.55, blocked, pulse)
        {
            return None;
        }
        let spacing = self.distance / self.steps as f32;
        let moves = core::array::from_fn::<_, HEADING_STEPS, _>(|yaw| {
            add(
                scale(self.axis, spacing),
                scale(perp(self.axis), (yaw as f32 - 2.0) * OFFSET),
            )
        });
        let lengths = moves.map(length);
        let directions = core::array::from_fn::<_, HEADING_STEPS, _>(|yaw| {
            scale(moves[yaw], 1.0 / lengths[yaw])
        });
        let mut active = 0;
        work.costs[active].fill([f32::INFINITY; HEADING_STEPS]);
        for (yaw, direction) in directions.into_iter().enumerate() {
            if dot(self.depart, direction) >= 0.866 {
                work.costs[active][MIDDLE][yaw] = 0.0;
            }
        }
        for row in 1..=self.steps {
            pulse();
            let next = active ^ 1;
            work.costs[next].fill([f32::INFINITY; HEADING_STEPS]);
            let radius = (2 * row.min(self.steps - row)).min(MIDDLE);
            let mut found = false;
            for lane in MIDDLE - radius..=MIDDLE + radius {
                let b = self.node(row, lane);
                if !self.highway.inside_seam(b[0], b[1]) {
                    continue;
                }
                for from_lane in lane.saturating_sub(2)..=(lane + 2).min(LANES - 1) {
                    let yaw = (2 + lane as i32 - from_lane as i32) as usize;
                    if row == self.steps && dot(self.arrive, directions[yaw]) < 0.866 {
                        continue;
                    }
                    let previous = &work.costs[active][from_lane];
                    let prior = if row == 1 {
                        yaw
                    } else {
                        (yaw.saturating_sub(1)..=(yaw + 1).min(HEADING_STEPS - 1))
                            .min_by(|&a, &b| previous[a].total_cmp(&previous[b]))?
                    };
                    if !previous[prior].is_finite() {
                        continue;
                    }
                    let a = self.node(row - 1, from_lane);
                    let lateral = lane.abs_diff(MIDDLE) as f32;
                    let cost = previous[prior]
                        + lengths[yaw]
                        + 0.009 * lateral * lateral
                        + 0.70 * from_lane.abs_diff(lane) as f32
                        + 0.4 * prior.abs_diff(yaw) as f32;
                    if cost >= work.costs[next][lane][yaw] {
                        continue;
                    }
                    work.edge_checks += 1;
                    if work.edge_checks > MAX_EDGE_PROOFS {
                        return None;
                    }
                    if !self.strip(a, b, 0.55, blocked, pulse) {
                        continue;
                    }
                    work.costs[next][lane][yaw] = cost;
                    let slot = lane * HEADING_STEPS + yaw;
                    let shift = (slot & 1) * 4;
                    let cell = &mut work.trail[row][slot / 2];
                    *cell = (*cell & !(15 << shift)) | ((prior as u8) << shift);
                    found = true;
                }
            }
            if !found {
                return None;
            }
            active = next;
        }
        let last = work.costs[active][MIDDLE];
        let mut yaw = (0..HEADING_STEPS).min_by(|&a, &b| last[a].total_cmp(&last[b]))?;
        if !last[yaw].is_finite() {
            return None;
        }
        let mut lane = MIDDLE;
        out[0] = self.road;
        out[self.steps + 2] = self.ramp;
        for row in (0..=self.steps).rev() {
            let [x, z] = self.node(row, lane);
            out[row + 1] = RoadPoint {
                x,
                z,
                elevation: self.world.elevation(x, z),
            };
            if row != 0 {
                let slot = lane * HEADING_STEPS + yaw;
                let previous = (work.trail[row][slot / 2] >> ((slot & 1) * 4)) & 15;
                lane = (lane as i32 - (yaw as i32 - 2)) as usize;
                yaw = previous as usize;
            }
        }
        // Recheck the chosen road more densely, including stored cap heights.
        // The bridge fit performs its own proof without relying on this one.
        for edge in out.windows(2) {
            let [a, b] = [edge[0], edge[1]];
            if (b.elevation - a.elevation).abs()
                > length(subtract(xz(b), xz(a))) * APPROACH_GRADE + 0.002
                || !self.strip(xz(a), xz(b), 0.40, blocked, pulse)
            {
                return None;
            }
        }
        Some(())
    }

    fn strip(
        self,
        a: Point,
        b: Point,
        step: f32,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        let delta = subtract(b, a);
        let distance = length(delta);
        if distance < 0.001 {
            return false;
        }
        let across = perp(scale(delta, 1.0 / distance));
        let steps = ceil_count(distance / step);
        let max_change = APPROACH_GRADE * distance / steps as f32 + 0.002;
        let mut previous = [0.0; 5];
        for i in 0..=steps {
            if i & 3 == 0 {
                pulse();
            }
            let p = interpolate(a, b, i as f32 / steps as f32);
            if !self.highway.inside_seam(p[0], p[1]) {
                return false;
            }
            let near_ramp = length(subtract(p, xz(self.ramp)));
            let floor = SEA_LEVEL + 0.14 + 1.46 * ((near_ramp - 8.0) / 48.0).clamp(0.0, 1.0);
            let mut heights = [0.0; 5];
            for (j, offset) in [-SHOULDER, -ROAD_HALF_WIDTH, 0.0, ROAD_HALF_WIDTH, SHOULDER]
                .into_iter()
                .enumerate()
            {
                let [x, z] = add(p, scale(across, offset));
                let y = self.world.elevation(x, z);
                if !(floor..=SEA_LEVEL + 68.0).contains(&y)
                    || (i != 0 && (y - previous[j]).abs() > max_change)
                    || blocked(x, z)
                {
                    return false;
                }
                heights[j] = y;
            }
            let [a, b, c, d, e] = heights;
            if (b - d).abs() > 0.70
                || (c - b).abs() > 0.525
                || (c - d).abs() > 0.525
                || (a - b).abs() > 0.35
                || (d - e).abs() > 0.35
            {
                return false;
            }
            previous = heights;
        }
        true
    }
}

#[derive(Clone, Copy)]
struct Approach {
    bounds: Area,
    origin: Point,
    axis: Point,
    inverse_step: f32,
}

impl Approach {
    fn new(points: &[RoadPoint], search: Search<'_>) -> Self {
        let [x, z] = xz(points[0]);
        let bounds = points.iter().fold([x, z, x, z], |area, point| {
            union(area, [point.x, point.z, point.x, point.z])
        });
        let [left, near, right, far] = bounds;
        Self {
            bounds: [
                left - SHOULDER,
                near - SHOULDER,
                right + SHOULDER,
                far + SHOULDER,
            ],
            origin: search.start,
            axis: search.axis,
            inverse_step: search.steps as f32 / search.distance,
        }
    }

    fn proximity(self, points: &[RoadPoint], query: Point) -> Option<f32> {
        if !contains(self.bounds, query) {
            return None;
        }
        let along = dot(subtract(query, self.origin), self.axis);
        let low = (((along - SHOULDER - 0.01) * self.inverse_step) as usize).min(points.len() - 1);
        let high = ((((along + SHOULDER + 0.01) * self.inverse_step) as usize).saturating_add(3))
            .min(points.len() - 1);
        let mut best = f32::INFINITY;
        for edge in points[low..=high.max(low)].windows(2) {
            best = best.min(segment_distance(query, xz(edge[0]), xz(edge[1])));
        }
        (best <= SHOULDER * SHOULDER).then_some(best)
    }
}

/// A published candidate keeps real road contacts, dry approach grades, and
/// the raised marine mesh together. Approaches are never projected onto water.
pub struct CoastalHighway {
    contacts: [HighwayContact; 2],
    street_caps: [Option<RoadPoint>; 2],
    bridge: Bridge,
    points: [RoadPoint; MAX_POINTS],
    split: u8,
    total: u8,
    paths: [Approach; 2],
    bounds: Area,
}

impl CoastalHighway {
    pub fn contacts(&self) -> [HighwayContact; 2] {
        self.contacts
    }

    pub fn street_caps(&self) -> [Option<RoadPoint>; 2] {
        self.street_caps
    }

    /// Both paths run from an exact highway or connected street cap to the ramp.
    pub fn approaches(&self) -> [&[RoadPoint]; 2] {
        let split = usize::from(self.split);
        [
            &self.points[..split],
            &self.points[split..usize::from(self.total)],
        ]
    }

    pub fn bridge(&self) -> &Bridge {
        &self.bridge
    }

    /// Minimum x/z and maximum x/z, including shoulders and bridge.
    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    /// Reserve the stitched road corridor rather than its large outer box.
    pub fn intersects_bounds(&self, area: [f32; 4]) -> bool {
        if !area.into_iter().all(f32::is_finite)
            || area[0] > area[2]
            || area[1] > area[3]
            || !overlaps(self.bounds, area)
        {
            return false;
        }
        if overlaps(self.bridge.bounds(), area) {
            let ends = self.bridge.sections();
            if segment_intersects(
                [ends[0][0], ends[0][2]],
                [ends[3][0], ends[3][2]],
                self.bridge.width() * 0.5 + 0.35,
                area,
            ) {
                return true;
            }
        }
        self.approaches()
            .into_iter()
            .zip(self.paths)
            .any(|(points, path)| {
                overlaps(path.bounds, area)
                    && points
                        .windows(2)
                        .any(|edge| segment_intersects(xz(edge[0]), xz(edge[1]), SHOULDER, area))
            })
    }

    /// Material only. Compose before adding tree crowns, and keep the
    /// separate water heightfield under the raised bridge.
    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        if ground.elevation <= SEA_LEVEL
            || !matches!(
                ground.surface,
                Surface::Sand
                    | Surface::Grass
                    | Surface::Forest
                    | Surface::Rock
                    | Surface::Snow
                    | Surface::Desert
                    | Surface::Gravel
            )
        {
            return ground;
        }
        if !contains(self.bounds, [x, z]) {
            return ground;
        }
        for (points, path) in self.approaches().into_iter().zip(self.paths) {
            if let Some(distance) = path.proximity(points, [x, z]) {
                let surface = if distance <= ROAD_HALF_WIDTH * ROAD_HALF_WIDTH {
                    Surface::Pavement
                } else {
                    Surface::Gravel
                };
                return Sample { surface, ..ground };
            }
        }
        ground
    }

    pub fn depth_region(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        screen_region(camera, dimensions, self.bridge.mesh_bounds())
    }

    pub fn draw(&self, mesh: &mut WorldMesh<'_>) {
        self.bridge.draw(mesh);
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        if self.bridge.road_height(x, z).is_some() {
            self.bridge.mesh_bounds()[1][1] + 1.0
        } else {
            0.0
        }
    }
}

fn contains([left, near, right, far]: Area, [x, z]: Point) -> bool {
    x >= left && x <= right && z >= near && z <= far
}

fn overlaps(a: Area, b: Area) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

fn union(a: Area, b: Area) -> Area {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ]
}

fn segment_distance(query: Point, a: Point, b: Point) -> f32 {
    let delta = subtract(b, a);
    let from = subtract(query, a);
    let t = (dot(from, delta) / dot(delta, delta)).clamp(0.0, 1.0);
    let offset = subtract(from, scale(delta, t));
    dot(offset, offset)
}

fn segment_intersects(a: Point, b: Point, radius: f32, [left, near, right, far]: Area) -> bool {
    if a[0].min(b[0]) > right + radius
        || a[0].max(b[0]) < left - radius
        || a[1].min(b[1]) > far + radius
        || a[1].max(b[1]) < near - radius
    {
        return false;
    }
    let squared = radius * radius;
    if [a, b].into_iter().any(|[x, z]| {
        let d = [x - x.clamp(left, right), z - z.clamp(near, far)];
        dot(d, d) <= squared
    }) || [[left, near], [left, far], [right, near], [right, far]]
        .into_iter()
        .any(|query| segment_distance(query, a, b) <= squared)
    {
        return true;
    }
    let mut first = 0.0_f32;
    let mut last = 1.0_f32;
    let delta = subtract(b, a);
    for (i, [low, high]) in [[left, right], [near, far]].into_iter().enumerate() {
        if delta[i] == 0.0 {
            if !(low..=high).contains(&a[i]) {
                return false;
            }
        } else {
            let t = (low - a[i]) / delta[i];
            let u = (high - a[i]) / delta[i];
            first = first.max(t.min(u));
            last = last.min(t.max(u));
            if first > last {
                return false;
            }
        }
    }
    true
}

fn xz(point: RoadPoint) -> Point {
    [point.x, point.z]
}
fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1]]
}
fn subtract(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
fn scale(a: Point, k: f32) -> Point {
    [a[0] * k, a[1] * k]
}
fn perp(a: Point) -> Point {
    [-a[1], a[0]]
}
fn dot(a: Point, b: Point) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}
fn interpolate(a: Point, b: Point, t: f32) -> Point {
    add(a, scale(subtract(b, a), t))
}

fn normalized(delta: Point) -> Option<Point> {
    let distance = length(delta);
    (distance > 0.001).then(|| scale(delta, 1.0 / distance))
}

fn length(delta: Point) -> f32 {
    let area = dot(delta, delta);
    let mut estimate = delta[0].abs().max(delta[1].abs());
    if estimate != 0.0 {
        for _ in 0..3 {
            estimate = 0.5 * (estimate + area / estimate);
        }
    }
    estimate
}

fn ceil_count(value: f32) -> usize {
    let integer = value as usize;
    integer + usize::from((integer as f32) < value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settlement::{Settlement, SettlementSurface, Terminus};

    #[test]
    fn perimeter_streets_connect_the_original_roads_without_displacing_roofs() {
        let world = World::new(3418);
        let roads = [(4, 4), (5, 4)]
            .map(|(x, z)| RoadNetwork::generate(&world, x, z).unwrap());
        let original = [*roads[0].polyline().last().unwrap(), roads[1].polyline()[0]];
        let [west, east] = core::array::from_fn(|i| {
            Settlement::generate(&world, &roads[i], [Terminus::End, Terminus::Start][i]).unwrap()
        });
        let exits = [
            west.coastal_road_exits(&world, original[1])[0],
            east.coastal_road_exits(&world, original[0])[0],
        ];
        let caps = exits.map(|exit| exit.unwrap().point);
        assert_ne!(caps, original);
        let occupied = |x: f32, z: f32| {
            [&west, &east].into_iter().zip(caps).any(|(town, cap)| {
                town.sample_transport_reservation(x, z).is_some_and(|sample| {
                    sample.surface != SettlementSurface::Street
                        || length(subtract([x, z], xz(cap))) > 5.3
                })
            })
        };
        let survey = CoastalHighwaySurvey::between(&roads[0], &roads[1]).unwrap();
        let mut work = CoastalHighwayWork::new();
        let route = survey
            .prepare_with_caps(&world, &mut work, exits, occupied, || {})
            .unwrap();
        assert_eq!(route.contacts().map(|contact| contact.point), original);
        assert_eq!(route.street_caps(), caps.map(Some));
        assert_eq!(route.approaches().map(|path| path[0]), caps);
        for path in route.approaches() {
            for edge in path.windows(2) {
                let [a, b] = [xz(edge[0]), xz(edge[1])];
                let normal = perp(normalized(subtract(b, a)).unwrap());
                let steps = ceil_count(length(subtract(b, a)) / 0.4);
                for step in 0..=steps {
                    let center = interpolate(a, b, step as f32 / steps as f32);
                    for side in [-4.5, -3.0, -1.5, 0.0, 1.5, 3.0, 4.5] {
                        let [x, z] = add(center, scale(normal, side));
                        assert!(!occupied(x, z));
                    }
                }
            }
        }
    }

    #[test]
    fn surveyed_coastal_route_joins_exact_caps_and_does_not_replace_seawater() {
        let world = World::new(67);
        let west = RoadNetwork::generate(&world, -2, 7).unwrap();
        let east = RoadNetwork::generate(&world, -1, 7).unwrap();
        let survey = CoastalHighwaySurvey::between(&east, &west).unwrap();
        let mut work = CoastalHighwayWork::new();
        let mut pulses = 0;
        assert!(survey.has_marine_candidate(&world, || pulses += 1));
        let result = survey
            .prepare(&world, &mut work, |_, _| false, || pulses += 1)
            .unwrap();
        assert!(pulses > 100);
        assert_eq!(result.contacts()[0].point, *west.polyline().last().unwrap());
        assert_eq!(result.contacts()[1].point, east.polyline()[0]);

        let [first, _, _, last] = *result.bridge().sections();
        for (i, (points, section)) in result
            .approaches()
            .into_iter()
            .zip([first, last])
            .enumerate()
        {
            assert_eq!(points[0], result.contacts()[i].point);
            let [x, elevation, z] = section;
            assert_eq!(*points.last().unwrap(), RoadPoint { x, elevation, z });
            let path = result.paths[i];
            for edge in points.windows(2) {
                let [a, b] = [xz(edge[0]), xz(edge[1])];
                let across = perp(normalized(subtract(b, a)).unwrap());
                for fraction in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    for offset in [-4.6, -3.0, 0.0, 3.0, 4.6] {
                        let query = add(interpolate(a, b, fraction), scale(across, offset));
                        let exact = points.windows(2).fold(f32::INFINITY, |nearest, pair| {
                            nearest.min(segment_distance(query, xz(pair[0]), xz(pair[1])))
                        });
                        assert_eq!(
                            path.proximity(points, query),
                            (exact <= SHOULDER * SHOULDER).then_some(exact)
                        );
                        if offset == 0.0 {
                            let [x, z] = query;
                            let sample = world.sample(x, z);
                            assert!(sample.elevation > SEA_LEVEL);
                            assert_eq!(result.overlay(x, z, sample).surface, Surface::Pavement);
                        }
                    }
                }
            }
        }

        let [x, z] = interpolate([first[0], first[2]], [last[0], last[2]], 0.50);
        let water = world.sample(x, z);
        assert_eq!(water.elevation, SEA_LEVEL);
        assert_eq!(result.overlay(x, z, water), water);
        assert!(result.obstruction(x, z) > SEA_LEVEL);
        assert!(result.intersects_bounds([x - 0.1, z - 0.1, x + 0.1, z + 0.1]));

        let cap = xz(result.contacts()[0].point);
        assert!(survey
            .prepare(
                &world,
                &mut work,
                |x, z| length(subtract([x, z], cap)) < 8.0,
                || {}
            )
            .is_none());
    }
}
