//! A sparse regional highway on the world's dry, gently graded terrain.

use crate::world::{Surface, World, SEA_LEVEL};

pub const ROAD_REGION_SIZE: i32 = 1024;
pub const ROAD_HALF_WIDTH: f32 = 3.0;

const SIDE: usize = 25;
const CELLS: usize = SIDE * SIDE;
const STEP: f32 = 24.0;
const INTERIOR_MARGIN: i32 = (ROAD_REGION_SIZE - (SIDE as i32 - 1) * STEP as i32) / 2;
const MAX_POINTS: usize = 65;
const LOW_CLEARANCE: f32 = SEA_LEVEL + 1.6;
const MAX_ROAD_HEIGHT: f32 = SEA_LEVEL + 68.0;
const HIGHWAY_GRADE: f32 = 0.12;
const SHOULDER_FALL: f32 = 0.70;
const PROXIMITY_LIMIT: f32 = ROAD_HALF_WIDTH + 1.5;
// The lookup visits up to nine road nodes with at most two incident edges each.
const NEARBY_EDGES: usize = 3 * 3 * 2;
const UNREACHABLE: u16 = u16::MAX;
const DIRECTIONS: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadPoint {
    pub x: f32,
    pub z: f32,
    pub elevation: f32,
}

impl RoadPoint {
    const EMPTY: Self = Self {
        x: 0.0,
        z: 0.0,
        elevation: 0.0,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoadClass {
    Highway,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadProximity {
    pub class: RoadClass,
    pub distance_squared: f32,
}

/// An immutable corridor. Retain it while its region is visible to the camera.
/// Expensive terrain proofs run in `generate`, never in a proximity query.
#[derive(Clone)]
pub struct RoadNetwork {
    grid: Grid,
    points: [RoadPoint; MAX_POINTS],
    count: u8,
    // Occupied lattice cells map to route points in row-major order.
    occupied_rows: [u32; SIDE],
    row_start: [u8; SIDE],
    point_indices: [u8; MAX_POINTS],
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
    coverage: [[u32; 2]; ROAD_COVER_SIDE],
}

const ROAD_COVER_STEP: f32 = 16.0;
const ROAD_COVER_SIDE: usize = 40;

#[derive(Clone, Copy)]
struct PreparedRoadEdge {
    x: f32,
    z: f32,
    dx: f32,
    dz: f32,
    length_squared: f32,
}

impl PreparedRoadEdge {
    const EMPTY: Self = Self { x: 0.0, z: 0.0, dx: 0.0, dz: 0.0, length_squared: 1.0 };

    fn new(a: RoadPoint, b: RoadPoint) -> Self {
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        Self { x: a.x, z: a.z, dx, dz, length_squared: dx * dx + dz * dz }
    }

    fn distance_squared(self, x: f32, z: f32) -> f32 {
        let projected = (x - self.x) * self.dx + (z - self.z) * self.dz;
        let factor = (projected / self.length_squared).clamp(0.0, 1.0);
        let ex = x - (self.x + self.dx * factor);
        let ez = z - (self.z + self.dz * factor);
        ex * ex + ez * ez
    }
}

struct RoadNeighbors {
    owner: usize,
    cell: [i32; 2],
    edges: [PreparedRoadEdge; NEARBY_EDGES],
    count: usize,
}

/// Frame-local edge geometry shared by nearby terrain samples.
pub(crate) struct RoadPaintCache(core::cell::RefCell<RoadNeighbors>);

impl RoadPaintCache {
    pub(crate) const fn new() -> Self {
        Self(core::cell::RefCell::new(RoadNeighbors {
            owner: 0,
            cell: [i32::MIN; 2],
            edges: [PreparedRoadEdge::EMPTY; NEARBY_EDGES],
            count: 0,
        }))
    }

    pub(crate) fn material(&self, road: &RoadNetwork, x: f32, z: f32) -> Option<Surface> {
        let cell = road.covered_cell(x, z)?;
        let mut neighbors = self.0.borrow_mut();
        let owner = road as *const RoadNetwork as usize;
        if neighbors.owner != owner || neighbors.cell != cell {
            neighbors.owner = owner;
            neighbors.cell = cell;
            neighbors.count = 0;
            let points = road.polyline();
            let mut seen = [0_u32; MAX_POINTS.div_ceil(32)];
            road.visit_edges(cell, |edge| {
                let word = edge >> 5;
                let bit = 1 << (edge & 31);
                if seen[word] & bit == 0 {
                    seen[word] |= bit;
                    let index = neighbors.count;
                    neighbors.edges[index] = PreparedRoadEdge::new(points[edge], points[edge + 1]);
                    neighbors.count += 1;
                }
            });
        }

        let mut gravel = false;
        for edge in &neighbors.edges[..neighbors.count] {
            let distance = edge.distance_squared(x, z);
            if distance <= ROAD_HALF_WIDTH * ROAD_HALF_WIDTH {
                return Some(Surface::Pavement);
            }
            gravel |= distance <= PROXIMITY_LIMIT * PROXIMITY_LIMIT;
        }
        gravel.then_some(Surface::Gravel)
    }
}

impl RoadNetwork {
    pub fn generate(world: &World, region_x: i32, region_z: i32) -> Option<Self> {
        let (grid, layout) = route_layout(world, region_x, region_z)?;
        let mut planner = Planner::new(world, grid);
        let prefer_north = layout & 8 != 0;
        for north in [prefer_north, !prefer_north] {
            let transverse = ((layout >> 4) % 9 + 8) as usize;
            let starts = planner.termini(north, false, transverse);
            let ends = planner.termini(north, true, transverse);
            for (a, b) in [(0, 0), (1, 0), (0, 1)] {
                if starts[a] == UNREACHABLE || ends[b] == UNREACHABLE {
                    continue;
                }
                if let Some(road) = planner.connect(starts[a], ends[b]) {
                    return Some(road);
                }
            }
        }
        None
    }

    /// A continuous graded route from its western or northern terminus.
    /// The first and last points are on land with room to build a settlement.
    pub fn polyline(&self) -> &[RoadPoint] {
        &self.points[..self.count as usize]
    }

    pub(crate) fn bounds(&self) -> [f32; 4] {
        [self.min_x, self.min_z, self.max_x, self.max_z]
    }

    /// Finds pavement or its immediate margin without evaluating terrain.
    pub fn proximity(&self, x: f32, z: f32) -> Option<RoadProximity> {
        let cell = self.covered_cell(x, z)?;
        let points = self.polyline();
        let mut nearest = PROXIMITY_LIMIT * PROXIMITY_LIMIT;
        let mut found = false;
        self.visit_edges(cell, |edge| {
            let distance = segment_distance_squared(x, z, points[edge], points[edge + 1]);
            if distance <= nearest {
                nearest = distance;
                found = true;
            }
        });
        found.then_some(RoadProximity {
            class: RoadClass::Highway,
            distance_squared: nearest,
        })
    }

    fn covered_cell(&self, x: f32, z: f32) -> Option<[i32; 2]> {
        if !(x >= self.min_x && x <= self.max_x && z >= self.min_z && z <= self.max_z) {
            return None;
        }
        let coverage_x = ((x - self.min_x) * (1.0 / ROAD_COVER_STEP)) as usize;
        let coverage_z = ((z - self.min_z) * (1.0 / ROAD_COVER_STEP)) as usize;
        if coverage_x < ROAD_COVER_SIDE && coverage_z < ROAD_COVER_SIDE
            && self.coverage[coverage_z][coverage_x >> 5] & (1 << (coverage_x & 31)) == 0
        {
            return None;
        }
        Some([((x - self.grid.x) / STEP) as i32, ((z - self.grid.z) / STEP) as i32])
    }

    fn visit_edges(&self, [col, row]: [i32; 2], mut visit: impl FnMut(usize)) {
        // Each segment joins adjacent lattice nodes. The bend offsets and
        // road width fit within one cell of either endpoint.
        let left = (col - 1).clamp(0, SIDE as i32);
        let right = (col + 2).clamp(0, SIDE as i32);
        let before = !(u32::MAX << left);
        let columns = !before & !(u32::MAX << right);
        let last_point = self.polyline().len() - 1;
        for gz in (row - 1).max(0)..=(row + 1).min(SIDE as i32 - 1) {
            let occupied = self.occupied_rows[gz as usize];
            let mut candidates = occupied & columns;
            if candidates == 0 {
                continue;
            }
            let mut slot = usize::from(self.row_start[gz as usize])
                + (occupied & before).count_ones() as usize;
            while candidates != 0 {
                candidates &= candidates - 1;
                let index = usize::from(self.point_indices[slot]);
                slot += 1;
                let start = index.saturating_sub(1);
                let end = (index + 1).min(last_point);
                for edge in start..end {
                    visit(edge);
                }
            }
        }
    }
}

/// Progress for a road plan advanced outside the renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoadProgress {
    Working,
    Ready,
    Unavailable,
}

/// A single regional plan with bounded terrain probes and A* expansions.
/// Keep this in scenery state while it works, then retain only the network.
pub struct RoadPlanner {
    planner: Planner,
    surveyed: [u32; CELLS.div_ceil(32)],
    terrain_loaded: bool,
    phase: Phase,
    layout: u32,
    orientation: u8,
    pair: usize,
    attempt: u8,
    start: u16,
    finish: u16,
    termini: [Termini; 2],
    path: GridPath,
    strip: Option<StripProbe>,
    points: [RoadPoint; MAX_POINTS],
    bend: RoadPoint,
}

#[derive(Clone, Copy)]
enum Phase {
    Terrain(usize),
    Termini(usize),
    Pair,
    Search(u16, usize),
    Proof(usize, bool),
    Points(usize),
    Ease(usize, bool),
    Ready,
    Unavailable,
}

struct Budget {
    expansions: u8,
    samples: u16,
}

impl Budget {
    fn sample(&mut self, count: u16) -> bool {
        if self.samples < count {
            return false;
        }
        self.samples -= count;
        true
    }
}

impl RoadPlanner {
    /// Sparse regions can be rejected immediately, using two layout probes.
    pub fn new(world: &World, region_x: i32, region_z: i32) -> Option<Self> {
        let (grid, layout) = route_layout(world, region_x, region_z)?;
        Some(Self {
            planner: Planner::empty(world, grid),
            surveyed: [0; CELLS.div_ceil(32)],
            terrain_loaded: false,
            phase: Phase::Termini(0),
            layout,
            orientation: 0,
            pair: 0,
            attempt: 0,
            start: UNREACHABLE,
            finish: UNREACHABLE,
            termini: [Termini::EMPTY; 2],
            path: GridPath {
                nodes: [0; MAX_POINTS],
                count: 0,
            },
            strip: None,
            points: [RoadPoint::EMPTY; MAX_POINTS],
            bend: RoadPoint::EMPTY,
        })
    }

    /// Reserve limits for this call. Endpoint, edge, and pavement checks
    /// use groups of up to nine terrain samples, even when a test exits early.
    /// A call with 32 terrain samples and four expansions can make progress
    /// at any stage. A smaller call may defer the next complete check.
    pub fn advance(&mut self, max_expansions: u8, max_terrain_samples: u16) -> RoadProgress {
        let mut budget = Budget {
            expansions: max_expansions,
            samples: max_terrain_samples,
        };
        loop {
            let continued = match self.phase {
                Phase::Terrain(index) => self.prepare(index, &mut budget),
                Phase::Termini(candidate) => self.find_termini(candidate, &mut budget),
                Phase::Pair => {
                    self.select_pair();
                    true
                }
                Phase::Search(node, heading) => self.scan_search(node, heading, &mut budget),
                Phase::Proof(index, rejected) => self.prove_path(index, rejected, &mut budget),
                Phase::Points(index) => self.prepare_points(index, &mut budget),
                Phase::Ease(index, far) => self.ease_path(index, far, &mut budget),
                Phase::Ready => return RoadProgress::Ready,
                Phase::Unavailable => return RoadProgress::Unavailable,
            };
            if !continued {
                return RoadProgress::Working;
            }
        }
    }

    pub fn into_network(self) -> Option<RoadNetwork> {
        if !matches!(self.phase, Phase::Ready) {
            return None;
        }
        Some(assemble_road(self.planner.grid, &self.path, self.points))
    }

    /// Release a finished road without moving the planning workspace.
    pub fn take_network(&mut self) -> Option<RoadNetwork> {
        if !matches!(self.phase, Phase::Ready) {
            return None;
        }
        self.phase = Phase::Unavailable;
        Some(assemble_road(self.planner.grid, &self.path, self.points))
    }

    fn prepare(&mut self, index: usize, budget: &mut Budget) -> bool {
        if index == CELLS {
            self.terrain_loaded = true;
            self.restart_search();
            return true;
        }
        if !self.sample_node(index, budget) {
            return false;
        }
        self.phase = Phase::Terrain(index + 1);
        true
    }

    fn sample_node(&mut self, index: usize, budget: &mut Budget) -> bool {
        let word = &mut self.surveyed[index / 32];
        let bit = 1 << (index % 32);
        if *word & bit == 0 {
            if !budget.sample(1) {
                return false;
            }
            self.planner.load(index);
            *word |= bit;
        }
        true
    }

    fn find_termini(&mut self, cursor: usize, budget: &mut Budget) -> bool {
        let side = cursor / TERMINUS_CANDIDATES;
        if side == 2 || side == 1 && self.termini[0].indices[0] == UNREACHABLE {
            self.pair = 0;
            self.phase = Phase::Pair;
            return true;
        }
        let prefer_north = self.layout & 8 != 0;
        let north = prefer_north != (self.orientation != 0);
        let transverse = ((self.layout >> 4) % 9 + 8) as usize;
        let candidate = cursor % TERMINUS_CANDIDATES;
        let [along, cross] = terminus_coordinates(side != 0, candidate);
        if !self.sample_node(terminus_index(north, along, cross), budget) {
            return false;
        }
        let result = &mut self.termini[side];
        if let Some((index, trend)) = self.planner.terminus_candidate(
            north,
            side != 0,
            transverse,
            candidate,
            result.scores[1],
        ) {
            if !budget.sample(9) {
                return false;
            }
            self.planner.consider_terminus(index, trend, result);
        }
        self.phase = Phase::Termini(cursor + 1);
        true
    }

    fn select_pair(&mut self) {
        const PAIRS: [(usize, usize); 3] = [(0, 0), (1, 0), (0, 1)];
        while let Some(&(a, b)) = PAIRS.get(self.pair) {
            self.pair += 1;
            let start = self.termini[0].indices[a];
            let finish = self.termini[1].indices[b];
            if start != UNREACHABLE && finish != UNREACHABLE {
                self.start = start;
                self.finish = finish;
                self.attempt = 0;
                // Regions without a viable pair never need an interior grid.
                if self.terrain_loaded {
                    self.restart_search();
                } else {
                    self.phase = Phase::Terrain(0);
                }
                return;
            }
        }
        if self.orientation == 0 {
            self.orientation = 1;
            self.termini = [Termini::EMPTY; 2];
            self.phase = Phase::Termini(0);
        } else {
            self.phase = Phase::Unavailable;
        }
    }

    fn restart_search(&mut self) {
        self.attempt += 1;
        self.planner.start_search(self.start);
        self.phase = Phase::Search(UNREACHABLE, 0);
    }

    fn scan_search(&mut self, node: u16, heading: usize, budget: &mut Budget) -> bool {
        if node == UNREACHABLE {
            if budget.expansions == 0 {
                return false;
            }
            budget.expansions -= 1;
            let Some(index) = self.planner.minimum_open(self.finish) else {
                self.phase = Phase::Pair;
                return true;
            };
            if index == self.finish {
                if let Some(path) = self.planner.retrace(self.start, self.finish) {
                    self.path = path;
                    self.phase = Phase::Proof(0, false);
                } else {
                    self.phase = Phase::Pair;
                }
                return true;
            }
            self.planner.open[index as usize].closed = true;
            self.phase = Phase::Search(index, 0);
            return true;
        }
        if heading == DIRECTIONS.len() {
            self.phase = Phase::Search(UNREACHABLE, 0);
            return true;
        }
        if let Some((next, length)) = self.planner.pending_edge(node, heading) {
            if !budget.sample(3) {
                return false;
            }
            self.planner.relax_edge(node, next, heading, length);
        }
        self.phase = Phase::Search(node, heading + 1);
        true
    }

    fn prove_path(&mut self, index: usize, rejected: bool, budget: &mut Budget) -> bool {
        if index + 1 == self.path.count {
            if !rejected {
                self.phase = Phase::Points(0);
            } else if self.attempt < 3 {
                self.restart_search();
            } else {
                self.phase = Phase::Pair;
            }
            return true;
        }
        let a = self.path.nodes[index];
        let b = self.path.nodes[index + 1];
        if self.strip.is_none() {
            if !budget.sample(2) {
                return false;
            }
            self.strip = Some(StripProbe::new(
                self.planner.position(a),
                self.planner.position(b),
            ));
        }
        if !budget.sample(3) {
            return false;
        }
        if let Some(clear) = self.strip.as_mut().unwrap().advance(&self.planner.world) {
            self.strip = None;
            if !clear {
                reject_link(&mut self.planner.rejected_edges, a, b);
            }
            self.phase = Phase::Proof(index + 1, rejected || !clear);
        }
        true
    }

    fn prepare_points(&mut self, index: usize, budget: &mut Budget) -> bool {
        if index == self.path.count {
            self.phase = Phase::Ease(1, false);
            return true;
        }
        if !budget.sample(1) {
            return false;
        }
        self.points[index] = self.planner.position(self.path.nodes[index]);
        self.phase = Phase::Points(index + 1);
        true
    }

    fn ease_path(&mut self, index: usize, far: bool, budget: &mut Budget) -> bool {
        if index + 1 == self.path.count {
            self.phase = Phase::Ready;
            return true;
        }
        if self.strip.is_none() {
            let (ax, az) = self.planner.grid.position(self.path.nodes[index - 1]);
            let b = self.points[index];
            let c = self.points[index + 1];
            let (x, z) = bend_position(ax, az, b, c);
            if x == b.x && z == b.z {
                self.phase = Phase::Ease(index + 1, false);
                return true;
            }
            if !budget.sample(1) {
                return false;
            }
            self.bend = RoadPoint {
                x,
                z,
                elevation: self.planner.world.elevation(x, z),
            };
            self.strip = Some(StripProbe::new(self.points[index - 1], self.bend));
        }
        if !budget.sample(3) {
            return false;
        }
        if let Some(clear) = self.strip.as_mut().unwrap().advance(&self.planner.world) {
            self.strip = None;
            if clear && !far {
                self.strip = Some(StripProbe::new(self.bend, self.points[index + 1]));
                self.phase = Phase::Ease(index, true);
            } else {
                if clear {
                    self.points[index] = self.bend;
                }
                self.phase = Phase::Ease(index + 1, false);
            }
        }
        true
    }
}

#[derive(Clone, Copy)]
struct Grid {
    x: f32,
    z: f32,
}

impl Grid {
    fn for_region(rx: i32, rz: i32) -> Option<Self> {
        // Room for the entire window while retaining world-unit precision.
        const LIMIT: i32 = 16_000;
        if !(-LIMIT..=LIMIT).contains(&rx) || !(-LIMIT..=LIMIT).contains(&rz) {
            return None;
        }
        Some(Self {
            x: (rx * ROAD_REGION_SIZE + INTERIOR_MARGIN) as f32,
            z: (rz * ROAD_REGION_SIZE + INTERIOR_MARGIN) as f32,
        })
    }

    fn position(self, index: u16) -> (f32, f32) {
        let (x, z) = lattice_position(index);
        (self.x + x as f32 * STEP, self.z + z as f32 * STEP)
    }
}

#[derive(Clone, Copy)]
struct SearchCell {
    cost: u16,
    approach: u8,
    closed: bool,
}

impl SearchCell {
    const UNVISITED: Self = Self {
        cost: UNREACHABLE,
        approach: 0,
        closed: false,
    };
}

struct Planner {
    world: World,
    grid: Grid,
    // Heights at the lattice nodes, in sixteenths of a world unit.
    elevation: [u16; CELLS],
    open: [SearchCell; CELLS],
    rejected_edges: [u8; CELLS],
}

impl Planner {
    fn empty(world: &World, grid: Grid) -> Self {
        Self {
            world: *world,
            grid,
            elevation: [0; CELLS],
            open: [SearchCell::UNVISITED; CELLS],
            rejected_edges: [0; CELLS],
        }
    }

    fn new(world: &World, grid: Grid) -> Self {
        let mut plan = Self::empty(world, grid);
        for index in 0..CELLS {
            plan.load(index);
        }
        plan
    }

    fn load(&mut self, index: usize) {
        let (x, z) = self.grid.position(index as u16);
        self.elevation[index] = ((self.world.elevation(x, z) - SEA_LEVEL) * 16.0) as u16;
    }

    fn termini(&self, north: bool, far: bool, look_at: usize) -> [u16; 2] {
        let mut result = Termini::EMPTY;
        for candidate in 0..TERMINUS_CANDIDATES {
            if let Some((index, trend)) =
                self.terminus_candidate(north, far, look_at, candidate, result.scores[1])
            {
                self.consider_terminus(index, trend, &mut result);
            }
        }
        result.indices
    }

    fn terminus_candidate(
        &self,
        north: bool,
        far: bool,
        look_at: usize,
        candidate: usize,
        score_limit: f32,
    ) -> Option<(u16, f32)> {
        let [along, cross] = terminus_coordinates(far, candidate);
        let index = terminus_index(north, along, cross);
        let height = self.elevation[index] as f32 / 16.0;
        if !(4.8..=33.0).contains(&height) {
            return None;
        }
        let trend = (along as f32 - if far { 19.0 } else { 5.0 }).abs() * 1.65
            + (cross as f32 - look_at as f32).abs() * 0.82
            + (height - 13.0).abs() * 0.21;
        (trend < score_limit).then_some((index as u16, trend))
    }

    fn consider_terminus(&self, index: u16, trend: f32, result: &mut Termini) {
        let (x, z) = self.grid.position(index);
        let Some(grade) = terminus_grade(&self.world, x, z) else {
            return;
        };
        let score = trend + grade * 1.9;
        if score < result.scores[0] {
            result.scores[1] = result.scores[0];
            result.indices[1] = result.indices[0];
            result.scores[0] = score;
            result.indices[0] = index;
        } else if score < result.scores[1] {
            result.scores[1] = score;
            result.indices[1] = index;
        }
    }

    fn connect(&mut self, start: u16, finish: u16) -> Option<RoadNetwork> {
        // Rejected footprint edges are retained for the few bounded retries.
        for _ in 0..3 {
            let path = self.search(start, finish)?;
            let mut invalid = false;
            for link in path.nodes[..path.count].windows(2) {
                let a = self.position(link[0]);
                let b = self.position(link[1]);
                if !dry_strip(&self.world, a, b) {
                    reject_link(&mut self.rejected_edges, link[0], link[1]);
                    invalid = true;
                }
            }
            if !invalid {
                return Some(self.to_network(path));
            }
        }
        None
    }

    fn search(&mut self, start: u16, finish: u16) -> Option<GridPath> {
        self.start_search(start);
        for _ in 0..CELLS {
            let index = self.minimum_open(finish)?;
            if index == finish {
                return self.retrace(start, finish);
            }
            self.open[index as usize].closed = true;
            for direction in 0..DIRECTIONS.len() {
                if let Some((next, length)) = self.pending_edge(index, direction) {
                    self.relax_edge(index, next, direction, length);
                }
            }
        }
        None
    }

    fn start_search(&mut self, start: u16) {
        self.open.fill(SearchCell::UNVISITED);
        self.open[start as usize].cost = 0;
    }

    fn pending_edge(&self, index: u16, direction: usize) -> Option<(u16, u16)> {
        let (x, z) = lattice_position(index);
        let (dx, dz) = DIRECTIONS[direction];
        let next = lattice_index(x + dx, z + dz)?;
        let to = self.open[next as usize];
        let length = if direction & 1 == 0 { 24 } else { 34 };
        if to.closed
            || self.rejected_edges[index as usize] & (1 << direction) != 0
            || self.open[index as usize].cost + length >= to.cost
        {
            return None;
        }
        Some((next, length))
    }

    fn relax_edge(&mut self, index: u16, next: u16, direction: usize, length: u16) {
        let Some(cost) = self.edge_cost(index, next, length) else {
            return;
        };
        let cost = self.open[index as usize].cost + cost;
        if cost < self.open[next as usize].cost {
            self.open[next as usize] = SearchCell {
                cost,
                approach: ((direction + 4) & 7) as u8,
                closed: false,
            };
        }
    }

    fn minimum_open(&self, finish: u16) -> Option<u16> {
        let mut best = UNREACHABLE;
        let mut index = None;
        for (i, cell) in self.open.iter().enumerate() {
            if cell.closed || cell.cost >= best {
                continue;
            }
            let score = cell.cost.saturating_add(estimate(i as u16, finish));
            if score < best {
                best = score;
                index = Some(i as u16);
            }
        }
        index
    }

    fn edge_cost(&self, from: u16, to: u16, length: u16) -> Option<u16> {
        let (start_x, start_z) = self.grid.position(from);
        let (end_x, end_z) = self.grid.position(to);
        let end_height = SEA_LEVEL + self.elevation[to as usize] as f32 / 16.0;
        let start_height = SEA_LEVEL + self.elevation[from as usize] as f32 / 16.0;
        let distance = length as f32;
        if !road_ground(end_height) || (end_height - start_height).abs() > distance * HIGHWAY_GRADE
        {
            return None;
        }
        let mut climb = 0.0;
        let mut previous = start_height;
        for sample in 1..=4 {
            let height = if sample == 4 {
                end_height
            } else {
                let t = sample as f32 * 0.25;
                self.world.elevation(
                    start_x + (end_x - start_x) * t,
                    start_z + (end_z - start_z) * t,
                )
            };
            let slope = (height - previous).abs();
            if !road_ground(height) || slope > distance * HIGHWAY_GRADE * 0.25 + 0.09 {
                return None;
            }
            climb += slope;
            previous = height;
        }
        let higher = start_height.max(end_height);
        Some(length + (climb * 2.6 + (higher - 24.0).max(0.0) * 0.16) as u16)
    }

    fn retrace(&self, start: u16, finish: u16) -> Option<GridPath> {
        let mut path = GridPath {
            nodes: [0; MAX_POINTS],
            count: 0,
        };
        let mut distance = 0;
        let mut index = finish;
        loop {
            if path.count == MAX_POINTS {
                return None;
            }
            path.nodes[path.count] = index;
            path.count += 1;
            if index == start {
                break;
            }
            let direction = self.open[index as usize].approach as usize;
            let (dx, dz) = DIRECTIONS[direction];
            let (x, z) = lattice_position(index);
            index = lattice_index(x + dx, z + dz)?;
            distance += if direction & 1 == 0 { 24 } else { 34 };
        }
        if distance > estimate(start, finish) * 7 / 4 {
            return None;
        }
        path.nodes[..path.count].reverse();
        Some(path)
    }

    fn position(&self, index: u16) -> RoadPoint {
        let (x, z) = self.grid.position(index);
        RoadPoint {
            x,
            z,
            elevation: self.world.elevation(x, z),
        }
    }

    fn to_network(&self, path: GridPath) -> RoadNetwork {
        let mut points = [RoadPoint::EMPTY; MAX_POINTS];
        for (i, &index) in path.nodes[..path.count].iter().enumerate() {
            points[i] = self.position(index);
        }
        // Ease corners only after both chords pass the full footprint proof.
        for i in 1..path.count - 1 {
            let a = points[i - 1];
            let b = points[i];
            let c = points[i + 1];
            let (ax, az) = self.grid.position(path.nodes[i - 1]);
            let (x, z) = bend_position(ax, az, b, c);
            if x == b.x && z == b.z {
                continue;
            }
            let eased = RoadPoint {
                x,
                z,
                elevation: self.world.elevation(x, z),
            };
            if dry_strip(&self.world, a, eased) && dry_strip(&self.world, eased, c) {
                points[i] = eased;
            }
        }
        assemble_road(self.grid, &path, points)
    }
}

fn bend_position(ax: f32, az: f32, b: RoadPoint, c: RoadPoint) -> (f32, f32) {
    (
        b.x + ((ax + c.x) * 0.5 - b.x) * 0.34,
        b.z + ((az + c.z) * 0.5 - b.z) * 0.34,
    )
}

fn assemble_road(grid: Grid, path: &GridPath, points: [RoadPoint; MAX_POINTS]) -> RoadNetwork {
    let mut occupied = [0; CELLS];
    for (i, &index) in path.nodes[..path.count].iter().enumerate() {
        occupied[usize::from(index)] = (i + 1) as u8;
    }
    let mut occupied_rows = [0; SIDE];
    let mut row_start = [0; SIDE];
    let mut point_indices = [0; MAX_POINTS];
    let mut slot = 0;
    for (row, cells) in occupied.chunks_exact(SIDE).enumerate() {
        row_start[row] = slot as u8;
        for (column, &index) in cells.iter().enumerate() {
            if index != 0 {
                occupied_rows[row] |= 1 << column;
                point_indices[slot] = index - 1;
                slot += 1;
            }
        }
    }
    let (mut min_x, mut max_x, mut min_z, mut max_z) =
        (points[0].x, points[0].x, points[0].z, points[0].z);
    for point in &points[1..path.count] {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_z = min_z.min(point.z);
        max_z = max_z.max(point.z);
    }
    // A conservative mask of the road and its shoulders. Each marked cell
    // intersects the padded bounds of a measured centerline segment.
    let pad = PROXIMITY_LIMIT + 0.03125;
    let cover = |value: f32, base: f32| {
        (((value - base) * (1.0 / ROAD_COVER_STEP)) as usize).min(ROAD_COVER_SIDE - 1)
    };
    let mut coverage = [[0; 2]; ROAD_COVER_SIDE];
    for segment in points[..path.count].windows(2) {
        let [a, b] = [segment[0], segment[1]];
        let x0 = cover(a.x.min(b.x) - pad, min_x - PROXIMITY_LIMIT);
        let x1 = cover(a.x.max(b.x) + pad, min_x - PROXIMITY_LIMIT);
        let z0 = cover(a.z.min(b.z) - pad, min_z - PROXIMITY_LIMIT);
        let z1 = cover(a.z.max(b.z) + pad, min_z - PROXIMITY_LIMIT);
        for row in &mut coverage[z0..=z1] {
            for x in x0..=x1 {
                row[x >> 5] |= 1 << (x & 31);
            }
        }
    }
    RoadNetwork {
        grid,
        points,
        count: path.count as u8,
        occupied_rows,
        row_start,
        point_indices,
        min_x: min_x - PROXIMITY_LIMIT,
        max_x: max_x + PROXIMITY_LIMIT,
        min_z: min_z - PROXIMITY_LIMIT,
        max_z: max_z + PROXIMITY_LIMIT,
        coverage,
    }
}

fn route_layout(world: &World, region_x: i32, region_z: i32) -> Option<(Grid, u32)> {
    let grid = Grid::for_region(region_x, region_z)?;
    let middle_x = grid.x + STEP * (SIDE / 2) as f32;
    let middle_z = grid.z + STEP * (SIDE / 2) as f32;
    let layout = region_pattern(world, middle_x, middle_z, region_x, region_z);
    // Large natural tracts remain between designated road regions.
    (layout & 7 < 3).then_some((grid, layout))
}

const TERMINUS_WIDTH: usize = 17;
const TERMINUS_CANDIDATES: usize = 6 * TERMINUS_WIDTH;

fn terminus_coordinates(far: bool, candidate: usize) -> [usize; 2] {
    [(if far { 16 } else { 3 }) + candidate / TERMINUS_WIDTH, 4 + candidate % TERMINUS_WIDTH]
}

fn terminus_index(north: bool, along: usize, cross: usize) -> usize {
    if north { along * SIDE + cross } else { cross * SIDE + along }
}

#[derive(Clone, Copy)]
struct Termini {
    indices: [u16; 2],
    scores: [f32; 2],
}

impl Termini {
    const EMPTY: Self = Self {
        indices: [UNREACHABLE; 2],
        scores: [f32::INFINITY; 2],
    };
}

struct GridPath {
    nodes: [u16; MAX_POINTS],
    count: usize,
}

fn region_pattern(world: &World, x: f32, z: f32, rx: i32, rz: i32) -> u32 {
    let geology = world
        .regional_pattern_height(x - 129.0, z - 147.0)
        .to_bits()
        ^ world
            .regional_pattern_height(x + 117.0, z + 151.0)
            .to_bits()
            .rotate_left(13);
    let mut key = geology
        ^ (rx as u32).wrapping_mul(0xab55_e701)
        ^ (rz as u32).wrapping_mul(0x50b1_f0e9)
        ^ 0x51b7_0a43;
    key = (key ^ (key >> 16)).wrapping_mul(0x2c1b_3c6d);
    key = (key ^ (key >> 13)).wrapping_mul(0x297a_2d39);
    key ^ (key >> 16)
}

fn lattice_position(index: u16) -> (i32, i32) {
    (
        (index as usize % SIDE) as i32,
        (index as usize / SIDE) as i32,
    )
}

fn lattice_index(x: i32, z: i32) -> Option<u16> {
    let side = SIDE as i32;
    if (0..side).contains(&x) && (0..side).contains(&z) {
        Some((z * side + x) as u16)
    } else {
        None
    }
}

fn estimate(i: u16, g: u16) -> u16 {
    let (ax, az) = lattice_position(i);
    let (bx, bz) = lattice_position(g);
    let dx = ax.abs_diff(bx) as u16;
    let dz = az.abs_diff(bz) as u16;
    24 * dx.max(dz) + 10 * dx.min(dz)
}

fn reject_link(edges: &mut [u8; CELLS], a: u16, b: u16) {
    let (ax, az) = lattice_position(a);
    let (bx, bz) = lattice_position(b);
    if let Some(direction) = DIRECTIONS.iter().position(|&d| d == (bx - ax, bz - az)) {
        edges[a as usize] |= 1 << direction;
        edges[b as usize] |= 1 << ((direction + 4) & 7);
    }
}

fn terminus_grade(world: &World, x: f32, z: f32) -> Option<f32> {
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for dz in [-8.0, 0.0, 8.0] {
        for dx in [-8.0, 0.0, 8.0] {
            let height = world.elevation(x + dx, z + dz);
            if height < SEA_LEVEL + 3.6 {
                return None;
            }
            low = low.min(height);
            high = high.max(height);
        }
    }
    (high - low <= 3.0).then_some(high - low)
}

fn road_ground(elevation: f32) -> bool {
    (LOW_CLEARANCE..=MAX_ROAD_HEIGHT).contains(&elevation)
}

fn road_distance(dx: f32, dz: f32) -> f32 {
    let squared = dx * dx + dz * dz;
    let mut distance = dx.abs().max(dz.abs());
    // A nonzero segment starts within a factor of sqrt(2) of its length.
    for _ in 0..3 {
        distance = (distance + squared / distance) * 0.5;
    }
    distance
}

fn dry_strip(world: &World, a: RoadPoint, b: RoadPoint) -> bool {
    let mut strip = StripProbe::new(a, b);
    loop {
        if let Some(clear) = strip.advance(world) {
            return clear;
        }
    }
}

struct StripProbe {
    start: RoadPoint,
    dx: f32,
    dz: f32,
    count: usize,
    index: usize,
    grade: f32,
    shoulder_x: f32,
    shoulder_z: f32,
    previous: [f32; 3],
}

impl StripProbe {
    fn new(a: RoadPoint, b: RoadPoint) -> Self {
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let length = road_distance(dx, dz);
        let count = (length / 1.25) as usize + 1;
        Self {
            start: a,
            dx,
            dz,
            count,
            index: 0,
            grade: length * HIGHWAY_GRADE / count as f32 + 0.002,
            shoulder_x: -dz * ROAD_HALF_WIDTH / length,
            shoulder_z: dx * ROAD_HALF_WIDTH / length,
            previous: [a.elevation; 3],
        }
    }

    fn advance(&mut self, world: &World) -> Option<bool> {
        let t = self.index as f32 / self.count as f32;
        let x = self.start.x + self.dx * t;
        let z = self.start.z + self.dz * t;
        let middle = world.elevation(x, z);
        let left = world.elevation(x + self.shoulder_x, z + self.shoulder_z);
        let right = world.elevation(x - self.shoulder_x, z - self.shoulder_z);
        if (left - right).abs() > SHOULDER_FALL
            || (middle - left).abs() > SHOULDER_FALL * 0.75
            || (middle - right).abs() > SHOULDER_FALL * 0.75
        {
            return Some(false);
        }
        let strip = [left, middle, right];
        for (height, prior) in strip.iter().zip(self.previous) {
            if !road_ground(*height) || (self.index != 0 && (*height - prior).abs() > self.grade) {
                return Some(false);
            }
        }
        self.previous = strip;
        self.index += 1;
        (self.index > self.count).then_some(true)
    }
}

fn segment_distance_squared(x: f32, z: f32, a: RoadPoint, b: RoadPoint) -> f32 {
    PreparedRoadEdge::new(a, b).distance_squared(x, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORRIDORS: [(u64, i32, i32); 3] = [(1, -1, -3), (7, 2, -5), (31, 4, -5)];

    #[test]
    fn cached_road_paint_matches_proximity_at_edges_and_junctions() {
        let roads = CORRIDORS.map(|(seed, x, z)| RoadNetwork::generate(&World::new(seed), x, z).unwrap());
        let cache = RoadPaintCache::new();
        for road in roads.iter().chain(roads.iter().rev()) {
            for points in road.polyline().windows(2) {
                let [a, b] = [points[0], points[1]];
                let (dx, dz) = (b.x - a.x, b.z - a.z);
                let length = (dx * dx + dz * dz).sqrt();
                for t in [0.0, 0.13, 0.5, 0.87, 1.0] {
                    for offset in [0.0, 2.999, 3.0, 3.001, 4.499, 4.5, 4.501, 7.0] {
                        for sign in [-1.0, 1.0] {
                            let side = offset * sign / length;
                            let (x, z) = (a.x + dx * t - dz * side, a.z + dz * t + dx * side);
                            let expected = road.proximity(x, z).map(|sample| {
                                if sample.distance_squared <= ROAD_HALF_WIDTH * ROAD_HALF_WIDTH {
                                    Surface::Pavement
                                } else {
                                    Surface::Gravel
                                }
                            });
                            assert_eq!(cache.material(road, x, z), expected);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn incremental_planning_matches_sync_under_different_budgets() {
        assert!(core::mem::size_of::<RoadPlanner>() < 5600);
        for (seed, x, z) in CORRIDORS {
            let world = World::new(seed);
            let expected = RoadNetwork::generate(&world, x, z).unwrap();
            for alternate in [false, true] {
                let mut plan = RoadPlanner::new(&world, x, z).unwrap();
                assert_eq!(plan.advance(0, 0), RoadProgress::Working);
                assert!(matches!(plan.phase, Phase::Termini(0)));
                let mut steps = 0;
                let actual = loop {
                    steps += 1;
                    assert!(steps < 4000);
                    let (expansions, probes) = if alternate && steps % 2 == 0 {
                        (1, 9)
                    } else {
                        (4, 32)
                    };
                    match plan.advance(expansions, probes) {
                        RoadProgress::Working => {}
                        RoadProgress::Unavailable => panic!("verified corridor disappeared"),
                        RoadProgress::Ready => break plan.into_network().unwrap(),
                    }
                };
                assert_eq!(actual.polyline(), expected.polyline());
            }
        }
    }

    #[test]
    fn an_unsuitable_region_can_finish_without_sampling_the_interior() {
        let world = World::new(1);
        for z in -4..=4 {
            for x in -4..=4 {
                let Some(mut plan) = RoadPlanner::new(&world, x, z) else { continue; };
                let outcome = loop {
                    let progress = plan.advance(8, 32);
                    if progress != RoadProgress::Working {
                        break progress;
                    }
                };
                if outcome == RoadProgress::Unavailable && !plan.terrain_loaded {
                    let samples = plan.surveyed.iter().map(|word| word.count_ones()).sum::<u32>() as usize;
                    assert!(samples < CELLS);
                    assert!(RoadNetwork::generate(&world, x, z).is_none());
                    return;
                }
            }
        }
        panic!("expected a region without a viable endpoint pair");
    }

    #[test]
    fn corridors_repeat_and_link_two_different_termini() {
        assert!(core::mem::size_of::<RoadNetwork>() <= 1500);
        for (seed, rx, rz) in CORRIDORS {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, rx, rz).unwrap();
            let second = RoadNetwork::generate(&World::new(seed), rx, rz).unwrap();
            let points = road.polyline();
            assert_eq!(points, second.polyline());
            assert!(points.len() >= 10 && points.len() <= MAX_POINTS);
            let a = points[0];
            let b = points[points.len() - 1];
            assert!(terminus_grade(&world, a.x, a.z).is_some());
            assert!(terminus_grade(&world, b.x, b.z).is_some());
            assert!((a.x - b.x).abs().max((a.z - b.z).abs()) >= 8.0 * STEP);
            for edge in points.windows(2) {
                let (dx, dz) = (edge[0].x - edge[1].x, edge[0].z - edge[1].z);
                assert!((12.0..=43.0).contains(&road_distance(dx, dz)));
            }
            assert!(road.proximity(a.x + 700.0, a.z).is_none());
        }
    }

    #[test]
    fn full_pavement_is_dry_at_subcell_resolution() {
        for (seed, rx, rz) in CORRIDORS {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, rx, rz).unwrap();
            for link in road.polyline().windows(2) {
                let (a, b) = (link[0], link[1]);
                assert_eq!(a.elevation, world.elevation(a.x, a.z));
                assert!(dry_strip(&world, a, b));
                let dx = b.x - a.x;
                let dz = b.z - a.z;
                let distance = road_distance(dx, dz);
                let count = (distance * 2.5) as usize + 1;
                let step = distance / count as f32;
                let normal = (
                    -dz * ROAD_HALF_WIDTH / distance,
                    dx * ROAD_HALF_WIDTH / distance,
                );
                let mut previous = [0.0; 3];
                for i in 0..=count {
                    let t = i as f32 / count as f32;
                    let x = a.x + dx * t;
                    let z = a.z + dz * t;
                    for (band, side) in [-1.0, 0.0, 1.0].into_iter().enumerate() {
                        let height = world.elevation(x + side * normal.0, z + side * normal.1);
                        assert!(road_ground(height));
                        if i != 0 {
                            assert!((height - previous[band]).abs() / step < 0.126);
                        }
                        previous[band] = height;
                    }
                }
            }
        }
    }

    #[test]
    fn nearby_segments_match_a_bidirectional_brute_force_query() {
        for (seed, rx, rz) in CORRIDORS {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, rx, rz).unwrap();
            let points = road.polyline();
            for link in points.windows(2) {
                for i in 0..=12 {
                    let t = i as f32 / 12.0;
                    for dx in [-6.5, -2.5, 0.0, 2.5, 6.5] {
                        for dz in [-6.5, -2.5, 0.0, 2.5, 6.5] {
                            let x = link[0].x + (link[1].x - link[0].x) * t + dx;
                            let z = link[0].z + (link[1].z - link[0].z) * t + dz;
                            let best = points
                                .windows(2)
                                .rev()
                                .map(|pair| segment_distance_squared(x, z, pair[1], pair[0]))
                                .fold(f32::INFINITY, f32::min);
                            if best <= PROXIMITY_LIMIT * PROXIMITY_LIMIT {
                                let actual = road.proximity(x, z).unwrap();
                                assert_eq!(actual.class, RoadClass::Highway);
                                assert!((actual.distance_squared - best).abs() < 0.006);
                            } else {
                                assert!(road.proximity(x, z).is_none());
                            }
                        }
                    }
                }
            }
        }
    }
}
