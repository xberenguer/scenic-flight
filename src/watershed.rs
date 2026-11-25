//! Seeded drainage plans. Build from the terrain once, then query the saved channels.

use crate::world::{SEA_LEVEL, World};

pub const REGION_SIZE: i32 = 1024;
const STEP: f32 = 32.0;
const COVER_STEP: f32 = 16.0;
const COVER_SIDE: usize = REGION_SIZE as usize / 16;
const SIDE: usize = REGION_SIZE as usize / 32 + 1;
const CELLS: usize = SIDE * SIDE;
const QUERY_SIDE: usize = SIDE - 1;
const QUERY_CELLS: usize = QUERY_SIDE * QUERY_SIDE;
const DIRECTIONS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];
const KNOT_INTERVALS: usize = 8;
const OUTLET_SCALE: f32 = 4096.0;
const HEIGHT_SCALE: f32 = 64.0;
const MAX_STREAMS: usize = 3;
const MAX_LINKS: usize = 96;
const MAX_TRACE: usize = 47;
const MAX_ATTEMPTS: usize = 20;
const MAX_CHECKED_LINKS: usize = 64;
const BANK_REACH: f32 = 14.0;
const CURVE_REACH: f32 = 6.25;
const INVERSE_DIAGONAL: f32 = core::f32::consts::FRAC_1_SQRT_2;
const OPEN_SEA: u8 = u8::MAX;

/// One selected headwater and its verified connection to the sea.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stream {
    pub source: [f32; 2],
    pub outlet: [f32; 2],
    /// Approximate contributing area measured in 32 × 32 world-unit cells.
    pub catchment: u16,
    pub links_to_sea: u8,
}

impl Stream {
    const EMPTY: Self = Self {
        source: [0.0; 2],
        outlet: [0.0; 2],
        catchment: 0,
        links_to_sea: 0,
    };
}

/// Geometry near a saved stream. A positive bank strength is a floodplain hint.
/// Callers may use actual terrain elevation to limit the floodplain effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiverSample {
    /// Surface of the carved channel.
    pub level: f32,
    pub distance: f32,
    pub half_width: f32,
    pub bank_strength: f32,
}

impl RiverSample {
    pub fn is_channel(self) -> bool {
        self.distance <= self.half_width
    }
}

/// A stream reach between two neighboring drainage nodes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reach {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub start_level: f32,
    pub end_level: f32,
    pub reaches_sea: bool,
    bend: [i8; 3],
    normal: [f32; 2],
}

impl Reach {
    /// Position along the saved centerline, including its validated bends.
    pub fn position(self, fraction: f32) -> [f32; 2] {
        curved_point(
            self.start,
            self.end,
            self.normal,
            self.bend,
            fraction.clamp(0.0, 1.0),
        )
    }
}

#[derive(Clone, Copy)]
struct Link {
    cell: u16,
    direction: u8,
    sea: bool,
    fraction: u16,
    bend: [i8; 3],
    widths: [u8; 2],
    levels: [u16; KNOT_INTERVALS + 1],
}

impl Link {
    const EMPTY: Self = Self {
        cell: 0,
        direction: 0,
        sea: false,
        fraction: 0,
        bend: [0; 3],
        widths: [0; 2],
        levels: [0; KNOT_INTERVALS + 1],
    };

    fn local_ends(self) -> ([f32; 2], [f32; 2]) {
        let [x, z] = local(usize::from(self.cell));
        let (dx, dz) = DIRECTIONS[usize::from(self.direction - 1)];
        let length = f32::from(self.fraction) * (STEP / OUTLET_SCALE);
        ([x, z], [x + dx as f32 * length, z + dz as f32 * length])
    }

    fn level(&self, t: f32) -> f32 {
        let offset = t * KNOT_INTERVALS as f32;
        let before = (offset as usize).min(KNOT_INTERVALS - 1);
        let a = f32::from(self.levels[before]);
        let b = f32::from(self.levels[before + 1]);
        SEA_LEVEL + (a + (b - a) * (offset - before as f32)) * (1.0 / HEIGHT_SCALE)
    }

    fn width(&self, t: f32) -> f32 {
        let a = f32::from(self.widths[0]);
        let b = f32::from(self.widths[1]);
        (a + (b - a) * t) * (1.0 / 16.0)
    }

    fn normal(self) -> [f32; 2] {
        let [dx, dz] = unit_direction(self.direction);
        [-dz, dx]
    }

    fn coverage_padding(self, [nx, nz]: [f32; 2]) -> ([f32; 2], f32) {
        let widest = f32::from(self.widths[0].max(self.widths[1])) * (1.0 / 16.0);
        let reach = (widest * 2.0 + 3.0).min(BANK_REACH);
        // t(1-t) is at most 1/4. Bound the whole curved reach,
        // including floating point roundoff near the edge of the bank.
        let bend = f32::from(self.bend[0].abs().max(self.bend[1].abs())) * (1.0 / 16.0)
            + f32::from(self.bend[2].abs()) * 0.25;
        let guard = 1.0 / 32.0;
        ([reach + nx.abs() * bend + guard, reach + nz.abs() * bend + guard], reach + bend + guard)
    }
}

/// Generation workspace. Keep one instance and reuse it across regions.
pub struct WatershedWork {
    heights: [u16; CELLS],
    order: [u16; CELLS],
    sea_links: [u8; CELLS],
    flow: [u16; CELLS],
    directions: [u8; CELLS],
    rejected: [bool; CELLS],
    key: u32,
}

impl WatershedWork {
    pub const fn new() -> Self {
        Self {
            heights: [0; CELLS],
            order: [0; CELLS],
            sea_links: [0; CELLS],
            flow: [0; CELLS],
            directions: [0; CELLS],
            rejected: [false; CELLS],
            key: 0,
        }
    }
}

impl Default for WatershedWork {
    fn default() -> Self {
        Self::new()
    }
}

/// The hot rejection data can be copied to the frame stack while the
/// immutable channel geometry stays with its retained region.
#[derive(Clone, Copy)]
pub(crate) struct WaterCoverage {
    origin: [f32; 2],
    rows: [[u32; 2]; COVER_SIDE],
}

impl WaterCoverage {
    const EMPTY: Self = Self {
        origin: [0.0; 2],
        rows: [[0; 2]; COVER_SIDE],
    };

    #[inline]
    pub(crate) fn locate(&self, x: f32, z: f32) -> Option<WaterQuery> {
        let [px, pz] = [x - self.origin[0], z - self.origin[1]];
        if !(0.0..REGION_SIZE as f32).contains(&px) || !(0.0..REGION_SIZE as f32).contains(&pz) {
            return None;
        }
        let gx = (px * (1.0 / COVER_STEP)) as usize;
        let gz = (pz * (1.0 / COVER_STEP)) as usize;
        if self.rows[gz][gx >> 5] & (1 << (gx & 31)) == 0 {
            return None;
        }
        Some(WaterQuery { px, pz, gx: gx >> 1, gz: gz >> 1 })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct WaterQuery {
    px: f32,
    pz: f32,
    gx: usize,
    gz: usize,
}

#[derive(Clone, Copy)]
struct PreparedEdge {
    bounds: [f32; 4],
    ax: f32,
    az: f32,
    dx: f32,
    dz: f32,
    nx: f32,
    nz: f32,
    scale: f32,
    width: f32,
    width_delta: f32,
    bend: [i8; 3],
    index: u8,
    full: bool,
}

impl PreparedEdge {
    const EMPTY: Self = Self {
        bounds: [0.0; 4],
        ax: 0.0,
        az: 0.0,
        dx: 0.0,
        dz: 0.0,
        nx: 0.0,
        nz: 0.0,
        scale: 0.0,
        width: 0.0,
        width_delta: 0.0,
        bend: [0; 3],
        index: 0,
        full: false,
    };
}

/// One neighborhood in fast frame memory. Create a fresh cache after rebuilding
/// a watershed or reusing its storage.
pub(crate) struct WaterNeighbors {
    owner: usize,
    cell: [usize; 2],
    edges: [PreparedEdge; 16],
    count: usize,
}

impl WaterNeighbors {
    pub(crate) const fn new() -> Self {
        Self {
            owner: 0,
            cell: [usize::MAX; 2],
            edges: [PreparedEdge::EMPTY; 16],
            count: 0,
        }
    }
}

/// A compact set of channels for one world-aligned region.
#[derive(Clone, Copy)]
pub struct Watershed {
    streams: [Stream; MAX_STREAMS],
    stream_count: usize,
    links: [Link; MAX_LINKS],
    link_count: usize,
    link_at: [u8; CELLS],
    candidates: [u16; QUERY_CELLS],
    coverage: WaterCoverage,
}

impl Watershed {
    pub const fn new() -> Self {
        Self {
            streams: [Stream::EMPTY; MAX_STREAMS],
            stream_count: 0,
            links: [Link::EMPTY; MAX_LINKS],
            link_count: 0,
            link_at: [0; CELLS],
            candidates: [0; QUERY_CELLS],
            coverage: WaterCoverage::EMPTY,
        }
    }

    /// Build from the visible, unmodified heights. Dry portions stay inside the
    /// tile, and every accepted path ends at an observed sea crossing.
    pub fn rebuild(
        &mut self,
        world: &World,
        seed: u64,
        region_x: i32,
        region_z: i32,
        work: &mut WatershedWork,
    ) {
        self.rebuild_cooperatively(world, seed, region_x, region_z, work, &mut || {});
    }

    pub(crate) fn rebuild_cooperatively(
        &mut self, world: &World, seed: u64, region_x: i32, region_z: i32,
        work: &mut WatershedWork, pulse: &mut impl FnMut(),
    ) {
        self.stream_count = 0;
        self.link_count = 0;
        self.link_at.fill(0);
        self.candidates.fill(0);
        self.coverage.rows.fill([0; 2]);
        // Keep sub-grid bank and shoreline probes precise even far from launch.
        if !(-2048..2048).contains(&region_x) || !(-2048..2048).contains(&region_z) {
            return;
        }
        self.coverage.origin = [
            (region_x * REGION_SIZE) as f32,
            (region_z * REGION_SIZE) as f32,
        ];
        work.sample_grid(world, self.coverage.origin, pulse);
        if !work.heights.contains(&0) || work.heights.iter().all(|&h| h == 0) {
            return;
        }
        let key = seed as u32
            ^ (seed >> 32) as u32
            ^ (region_x as u32).wrapping_mul(0x9e37_79b1)
            ^ (region_z as u32).wrapping_mul(0x85eb_ca77);
        work.route_grid(key, pulse);
        let mut check_budget = MAX_CHECKED_LINKS;
        let mut attempted = [0; MAX_ATTEMPTS];
        for attempt in 0..MAX_ATTEMPTS {
            if self.stream_count == MAX_STREAMS || self.link_count == MAX_LINKS || check_budget == 0
            {
                break;
            }
            let Some(head) = self.best_head(work, key, &attempted[..attempt], pulse) else {
                break;
            };
            attempted[attempt] = head;
            self.trace(world, work, head, &mut check_budget, pulse);
        }
    }

    pub fn streams(&self) -> &[Stream] {
        &self.streams[..self.stream_count]
    }

    pub fn reaches(&self) -> impl Iterator<Item = Reach> + '_ {
        self.links[..self.link_count].iter().map(|&link| {
            let (a, b) = link.local_ends();
            Reach {
                start: world_point(self.coverage.origin, a),
                end: world_point(self.coverage.origin, b),
                start_level: link.level(0.0),
                end_level: link.level(1.0),
                reaches_sea: link.sea,
                bend: link.bend,
                normal: link.normal(),
            }
        })
    }

    /// Query an existing plan without regenerating any terrain.
    pub fn sample(&self, x: f32, z: f32) -> Option<RiverSample> {
        self.sample_covered(self.coverage.locate(x, z)?)
    }

    pub(crate) fn coverage(&self) -> &WaterCoverage {
        &self.coverage
    }

    pub(crate) fn sample_covered(&self, query: WaterQuery) -> Option<RiverSample> {
        self.sample_edges(query, self.nearby_edges(query))
    }

    pub(crate) fn sample_neighbors(&self, query: WaterQuery, work: &mut WaterNeighbors) -> Option<RiverSample> {
        let owner = self as *const Self as usize;
        let cell = [query.gx, query.gz];
        if work.owner != owner || work.cell != cell {
            work.owner = owner;
            work.cell = cell;
            work.count = 0;
            for edge in self.nearby_edges(query) {
                work.edges[work.count] = edge;
                work.count += 1;
            }
        }
        self.sample_edges(query, work.edges[..work.count].iter().copied())
    }

    fn nearby_edges(&self, query: WaterQuery) -> impl Iterator<Item = PreparedEdge> + '_ {
        let first_x = query.gx.saturating_sub(1);
        let first_z = query.gz.saturating_sub(1);
        let mut remaining = self.candidates[query.gz * QUERY_SIDE + query.gx];
        core::iter::from_fn(move || {
            if remaining == 0 {
                return None;
            }
            // Lower bits preserve the original row-major priority at confluences.
            let bit = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            let sx = first_x + (bit & 3);
            let sz = first_z + (bit >> 2);
            let index = self.link_at[sz * SIDE + sx] - 1;
            let link = &self.links[usize::from(index)];
            let ax = sx as f32 * STEP;
            let az = sz as f32 * STEP;
            let (ix, iz) = DIRECTIONS[usize::from(link.direction - 1)];
            let length = f32::from(link.fraction) * (STEP / OUTLET_SCALE);
            let bx = ax + ix as f32 * length;
            let bz = az + iz as f32 * length;
            let dx = bx - ax;
            let dz = bz - az;
            let diagonal = ix != 0 && iz != 0;
            let normal = if diagonal { INVERSE_DIAGONAL } else { 1.0 };
            let full = link.fraction == OUTLET_SCALE as u16;
            let scale = if full {
                // Full lattice reaches have exact power-of-two lengths squared.
                if diagonal { 1.0 / 2048.0 } else { 1.0 / 1024.0 }
            } else {
                dx * dx + dz * dz
            };
            let width = f32::from(link.widths[0]);
            let [nx, nz] = [-(iz as f32 * normal), ix as f32 * normal];
            let ([xpad, zpad], _) = link.coverage_padding([nx, nz]);
            Some(PreparedEdge {
                bounds: [ax.min(bx) - xpad, az.min(bz) - zpad, ax.max(bx) + xpad, az.max(bz) + zpad],
                ax, az, dx, dz, nx, nz,
                scale, width, width_delta: f32::from(link.widths[1]) - width,
                bend: link.bend, index, full,
            })
        })
    }

    fn sample_edges(&self, query: WaterQuery, edges: impl Iterator<Item = PreparedEdge>) -> Option<RiverSample> {
        let WaterQuery { px, pz, .. } = query;
        let mut nearest = None;
        let mut best = f32::MAX;
        let mut bank = 0.0;
        for edge in edges {
            let [left, near, right, far] = edge.bounds;
            if px < left || px > right || pz < near || pz > far {
                continue;
            }
            let vx = px - edge.ax;
            let vz = pz - edge.az;
            let projected = vx * edge.dx + vz * edge.dz;
            let t = if edge.full { projected * edge.scale } else { projected / edge.scale }.clamp(0.0, 1.0);
            let offset = lateral_bend(edge.bend, t);
            let (rx, rz) = (vx - edge.dx * t - edge.nx * offset, vz - edge.dz * t - edge.nz * offset);
            let squared = rx * rx + rz * rz;
            let width = (edge.width + edge.width_delta * t) * (1.0 / 16.0);
            let reach = (width * 2.0 + 3.0).min(BANK_REACH);
            if squared <= reach * reach {
                let distance = root(squared);
                let separation = distance - width;
                let priority = if separation <= 0.0 { distance - BANK_REACH } else { separation };
                if priority < best {
                    best = priority;
                    bank = reach;
                    nearest = Some((edge.index, t, distance, width, separation));
                }
            }
        }
        let (index, t, distance, width, separation) = nearest?;
        Some(RiverSample {
            level: self.links[usize::from(index)].level(t),
            distance,
            half_width: width,
            bank_strength: if separation <= 0.0 {
                1.0
            } else {
                (1.0 - separation / (bank - width)).clamp(0.0, 1.0)
            },
        })
    }

}

impl WatershedWork {
    fn sample_grid(&mut self, world: &World, origin: [f32; 2], pulse: &mut impl FnMut()) {
        for (cell, height) in self.heights.iter_mut().enumerate() {
            if cell & 15 == 0 { pulse(); }
            let [x, z] = world_point(origin, local(cell));
            let elevation = world.elevation(x, z);
            *height = if elevation <= SEA_LEVEL {
                0
            } else {
                quantize_height(elevation).max(1)
            };
        }
    }

    fn mark_open_sea(&mut self, pulse: &mut impl FnMut()) {
        self.sea_links.fill(0);
        let mut end = 0;
        for cell in 0..CELLS {
            if cell & 63 == 0 { pulse(); }
            if self.heights[cell] == 0 && !inside(cell) {
                self.sea_links[cell] = OPEN_SEA;
                self.order[end] = cell as u16;
                end += 1;
            }
        }
        let mut head = 0;
        while head < end {
            if head & 63 == 0 { pulse(); }
            let cell = usize::from(self.order[head]);
            head += 1;
            let (x, z) = (cell % SIDE, cell / SIDE);
            for (nx, nz) in [
                (x.wrapping_sub(1), z),
                (x + 1, z),
                (x, z.wrapping_sub(1)),
                (x, z + 1),
            ] {
                if nx >= SIDE || nz >= SIDE {
                    continue;
                }
                let next = nz * SIDE + nx;
                if self.heights[next] == 0 && self.sea_links[next] == 0 {
                    self.sea_links[next] = OPEN_SEA;
                    self.order[end] = next as u16;
                    end += 1;
                }
            }
        }
    }

    fn route_grid(&mut self, key: u32, pulse: &mut impl FnMut()) {
        self.key = key;
        self.mark_open_sea(pulse);
        self.directions.fill(0);
        self.flow.fill(0);
        self.rejected.fill(false);
        for (cell, entry) in self.order.iter_mut().enumerate() {
            *entry = cell as u16;
        }
        self.order
            .sort_unstable_by_key(|&cell| (self.heights[usize::from(cell)], cell));
        for (index, &cell) in self.order.iter().enumerate() {
            if index & 63 == 0 { pulse(); }
            let cell = usize::from(cell);
            let height = self.heights[cell];
            if height == 0 || !inside(cell) {
                continue;
            }
            let shift = (mix_key(cell as u32 ^ key) & 7) as usize;
            let mut best = 0;
            for i in 0..DIRECTIONS.len() {
                let direction = ((i + shift) & 7) as u8 + 1;
                let next = downstream(cell, direction);
                if self.sea_links[next] == 0 || self.heights[next] >= height {
                    continue;
                }
                let (dx, dz) = DIRECTIONS[usize::from(direction - 1)];
                let diagonal = dx != 0 && dz != 0;
                let drop =
                    u32::from(height - self.heights[next]) * if diagonal { 181 } else { 256 };
                if drop <= best || drop > 15 * 64 * 256 {
                    continue;
                }
                if diagonal {
                    // The other diagonal can drain a different basin at a
                    // different elevation. Do not allow a crossing.
                    let horizontal = cell.wrapping_add_signed(dx as isize);
                    let vertical = cell.wrapping_add_signed(dz as isize * SIDE as isize);
                    if (self.directions[horizontal] != 0
                        && downstream(horizontal, self.directions[horizontal]) == vertical)
                        || (self.directions[vertical] != 0
                            && downstream(vertical, self.directions[vertical]) == horizontal)
                    {
                        continue;
                    }
                }
                let distance = if self.sea_links[next] == OPEN_SEA {
                    1
                } else {
                    self.sea_links[next] + 1
                };
                if usize::from(distance) <= MAX_TRACE {
                    best = drop;
                    self.directions[cell] = direction;
                    self.sea_links[cell] = distance;
                }
            }
        }
        for (index, &cell) in self.order.iter().rev().enumerate() {
            if index & 63 == 0 { pulse(); }
            let cell = usize::from(cell);
            if self.directions[cell] != 0 {
                self.flow[cell] += 1;
                let next = downstream(cell, self.directions[cell]);
                self.flow[next] += self.flow[cell];
            }
        }
    }

    fn tangent(&self, cell: usize) -> [f32; 2] {
        let [x, z] = unit_direction(self.directions[cell]);
        let mut biggest = 0;
        let mut incoming = [x, z];
        for (index, &(dx, dz)) in DIRECTIONS.iter().enumerate() {
            let donor = cell.wrapping_add_signed(dx as isize + dz as isize * SIDE as isize);
            if self.flow[donor] > biggest
                && self.directions[donor] != 0
                && downstream(donor, self.directions[donor]) == cell
            {
                biggest = self.flow[donor];
                let [dx, dz] = unit_direction(index as u8 + 1);
                incoming = [-dx, -dz];
            }
        }
        [x * 0.54 + incoming[0] * 0.46, z * 0.54 + incoming[1] * 0.46]
    }
}

impl Watershed {
    fn best_head(&self, work: &WatershedWork, key: u32, attempted: &[usize], pulse: &mut impl FnMut()) -> Option<usize> {
        let mut best = None;
        let mut best_score = 0;
        for cell in 0..CELLS {
            if cell & 31 == 0 { pulse(); }
            let steps = usize::from(work.sea_links[cell]);
            if self.link_at[cell] != 0
                || !(3..=MAX_TRACE).contains(&steps)
                || attempted.contains(&cell)
            {
                continue;
            }
            let mut cursor = cell;
            let mut mouth = cell;
            let mut usable = true;
            for _ in 0..steps {
                if work.rejected[cursor] {
                    usable = false;
                    break;
                }
                mouth = cursor;
                cursor = downstream(cursor, work.directions[cursor]);
            }
            if !usable || work.flow[mouth] < 5 {
                continue;
            }
            let score = u32::from(work.flow[mouth]) * 4
                + steps as u32 * 22
                + u32::from(work.flow[cell]).min(12)
                + (mix_key(key ^ cell as u32) & 15);
            if score > best_score {
                best_score = score;
                best = Some(cell);
            }
        }
        best
    }

    fn trace(&mut self, world: &World, work: &mut WatershedWork, head: usize, budget: &mut usize, pulse: &mut impl FnMut()) {
        let steps = usize::from(work.sea_links[head]);
        let mut pending = [0; MAX_TRACE];
        let mut count = 0;
        let mut cell = head;
        let mut mouth = head;
        for _ in 0..steps {
            if self.link_at[cell] == 0 {
                pending[count] = cell;
                count += 1;
            }
            mouth = cell;
            cell = downstream(cell, work.directions[cell]);
        }
        let shares_trunk = count < steps;
        let minimum = if shares_trunk { 2 } else { 3 };
        if count < minimum {
            return;
        }
        let mut accepted = 0;
        for &cell in pending[..count].iter().rev() {
            if *budget == 0 || self.link_count + accepted == MAX_LINKS {
                break;
            }
            *budget -= 1;
            if let Some(link) = self.validate_link(world, work, cell, pulse) {
                self.links[self.link_count + accepted] = link;
                accepted += 1;
            } else {
                work.rejected[cell] = true;
                break;
            }
        }
        if accepted < minimum {
            return;
        }
        let first = self.link_count;
        self.link_count += accepted;
        for index in first..self.link_count {
            let link = self.links[index];
            self.link_at[usize::from(link.cell)] = index as u8 + 1;
            self.add_coverage(link);
        }
        let mouth_link = self.links[usize::from(self.link_at[mouth]) - 1];
        let (_, outlet) = mouth_link.local_ends();
        self.streams[self.stream_count] = Stream {
            source: world_point(self.coverage.origin, local(pending[count - accepted])),
            outlet: world_point(self.coverage.origin, outlet),
            catchment: work.flow[mouth],
            links_to_sea: (steps - (count - accepted)) as u8,
        };
        self.stream_count += 1;
    }

    fn add_coverage(&mut self, link: Link) {
        let ([ax, az], [bx, bz]) = link.local_ends();
        let [nx, nz] = link.normal();
        let ([xpad, zpad], side_limit) = link.coverage_padding([nx, nz]);
        let fine = |value: f32| ((value * (1.0 / COVER_STEP)) as usize).min(COVER_SIDE - 1);

        // Keep the original 32-unit envelope as the outer mask. This also
        // preserves its behavior exactly when guard reaches a cell edge.
        let coarse = |value: f32| ((value * (1.0 / STEP)) as usize).min(SIDE - 2);
        let old = BANK_REACH + if link.bend == [0; 3] { 0.0 } else { CURVE_REACH };
        let xmin = fine(ax.min(bx) - xpad).max(2 * coarse(ax.min(bx) - old));
        let xmax = fine(ax.max(bx) + xpad).min(2 * coarse(ax.max(bx) + old) + 1);
        let zmin = fine(az.min(bz) - zpad).max(2 * coarse(az.min(bz) - old));
        let zmax = fine(az.max(bz) + zpad).min(2 * coarse(az.max(bz) + old) + 1);
        let half_box = (nx.abs() + nz.abs()) * (COVER_STEP * 0.5);
        let cell = usize::from(link.cell);
        let (sx, sz) = (cell % SIDE, cell / SIDE);
        for z in zmin..=zmax {
            let rz = (z as f32 + 0.5) * COVER_STEP - az;
            for x in xmin..=xmax {
                let rx = (x as f32 + 0.5) * COVER_STEP - ax;
                // Any hit is inside the reach's perpendicular band. A
                // diagonal cell can be outside this band despite its AABB.
                if (rx * nx + rz * nz).abs() <= side_limit + half_box {
                    self.coverage.rows[z][x >> 5] |= 1 << (x & 31);
                    // Index each reach by its conservative fine coverage.
                    // The four fine cells share a mask in the query lattice.
                    let (gx, gz) = (x >> 1, z >> 1);
                    let (first_x, first_z) = (gx.saturating_sub(1), gz.saturating_sub(1));
                    if (first_x..=gx + 2).contains(&sx) && (first_z..=gz + 2).contains(&sz) {
                        let bit = (sz - first_z) * 4 + sx - first_x;
                        self.candidates[gz * QUERY_SIDE + gx] |= 1 << bit;
                    }
                }
            }
        }
    }

    fn validate_link(&self, world: &World, work: &WatershedWork, cell: usize, pulse: &mut impl FnMut()) -> Option<Link> {
        pulse();
        let direction = work.directions[cell];
        let next = downstream(cell, direction);
        let sea = work.heights[next] == 0;
        let fraction = if sea {
            self.find_outlet(world, cell, direction)?
        } else {
            OUTLET_SCALE as u16
        };
        let mut link = Link {
            cell: cell as u16,
            direction,
            sea,
            fraction,
            bend: [0; 3],
            widths: [width_at(work.flow[cell]), width_at(work.flow[next])],
            levels: [0; KNOT_INTERVALS + 1],
        };
        let (dx, dz) = DIRECTIONS[usize::from(direction - 1)];
        let length = f32::from(fraction)
            * (STEP / OUTLET_SCALE)
            * if dx != 0 && dz != 0 {
                core::f32::consts::SQRT_2
            } else {
                1.0
            };
        if length < 0.10 {
            return None;
        }
        let [ux, uz] = unit_direction(direction);
        let [nx, nz] = link.normal();
        let tangent_offset = |[tx, tz]: [f32; 2]| {
            let slope = (tx * nx + tz * nz) / (tx * ux + tz * uz).max(0.1);
            let quarter_units = slope.clamp(-0.42, 0.42) * length * 4.0;
            (quarter_units + 0.5_f32.copysign(quarter_units)) as i8
        };
        let salt = mix_key(work.key ^ (cell as u32).wrapping_mul(0x51ed_270b));
        link.bend = [
            tangent_offset(work.tangent(cell)),
            if sea {
                0
            } else {
                -tangent_offset(work.tangent(next))
            },
            ((salt % 11) as i8 - 5) * i8::from(length >= 14.0),
        ];
        self.validate_shape(world, work, link, length, next, pulse)
            .or_else(|| {
                if link.bend == [0; 3] {
                    return None;
                }
                link.bend = [0; 3];
                self.validate_shape(world, work, link, length, next, pulse)
            })
    }

    fn validate_shape(
        &self,
        world: &World,
        work: &WatershedWork,
        mut link: Link,
        length: f32,
        next: usize,
        pulse: &mut impl FnMut(),
    ) -> Option<Link> {
        let cell = usize::from(link.cell);
        let (a, b) = link.local_ends();
        let normal = link.normal();
        let point = |t| world_point(self.coverage.origin, curved_point(a, b, normal, link.bend, t));
        let first = work.heights[cell].saturating_sub(bed_cut(work.flow[cell]));
        let last = work.heights[next].saturating_sub(bed_cut(work.flow[next]));
        link.levels[0] = first;
        link.levels[KNOT_INTERVALS] = last;
        let mut previous = first;
        for knot in 1..KNOT_INTERVALS {
            let t = knot as f32 * (1.0 / KNOT_INTERVALS as f32);
            let [x, z] = point(t);
            let elevation = world.elevation(x, z);
            if elevation <= SEA_LEVEL {
                return None;
            }
            let flow = work.flow[cell] as f32 + f32::from(work.flow[next] - work.flow[cell]) * t;
            let level = quantize_height(elevation).saturating_sub(bed_cut(flow as u16));
            previous = level.clamp(last, previous);
            link.levels[knot] = previous;
        }

        let [nx, nz] = normal;
        // Validate the piecewise water surface and shoulders at finer spacing
        // than the saved profile. A coarse D8 downhill result is insufficient.
        let probes = (length * 0.85) as usize + 1;
        let spacing = length / probes as f32;
        let mut previous_ground = f32::from(work.heights[cell]) * (1.0 / HEIGHT_SCALE) + SEA_LEVEL;
        for i in 0..=probes {
            if i & 7 == 0 { pulse(); }
            let t = i as f32 / probes as f32;
            let [x, z] = point(t);
            let ground = world.elevation(x, z);
            let level = link.level(t);
            let cut = ground - level;
            let reaches_water_here = link.sea && i == probes;
            if (ground <= SEA_LEVEL) != reaches_water_here
                || !(-0.03..=2.35).contains(&cut)
                || ground - previous_ground > 0.105 + spacing * 0.11
                || previous_ground - ground > 0.08 + spacing * 0.54
            {
                return None;
            }
            previous_ground = ground;
            if link.sea && ground < SEA_LEVEL + 0.36 {
                continue;
            }
            let offset = link.width(t) + 0.82;
            let west = world.elevation(x + nx * offset, z + nz * offset);
            let east = world.elevation(x - nx * offset, z - nz * offset);
            if west.min(east) < level.max(SEA_LEVEL) + 0.12 {
                return None;
            }
        }
        Some(link)
    }

    fn find_outlet(&self, world: &World, cell: usize, direction: u8) -> Option<u16> {
        let [ax, az] = local(cell);
        let (dx, dz) = DIRECTIONS[usize::from(direction - 1)];
        let wet = |fraction: u16| {
            let step = f32::from(fraction) * (STEP / OUTLET_SCALE);
            let [x, z] = world_point(self.coverage.origin, [ax + dx as f32 * step, az + dz as f32 * step]);
            world.elevation(x, z) <= SEA_LEVEL
        };
        let mut last_dry = 0;
        let mut first_wet = None;
        for fraction in (256..=OUTLET_SCALE as u16).step_by(256) {
            if wet(fraction) {
                first_wet.get_or_insert(fraction);
            } else if first_wet.is_some() {
                return None;
            } else {
                last_dry = fraction;
            }
        }
        let mut first_wet = first_wet?;
        while first_wet - last_dry > 1 {
            let midpoint = (first_wet + last_dry) / 2;
            if wet(midpoint) {
                first_wet = midpoint;
            } else {
                last_dry = midpoint;
            }
        }
        Some(first_wet)
    }
}

impl Default for Watershed {
    fn default() -> Self {
        Self::new()
    }
}

fn local(cell: usize) -> [f32; 2] {
    [(cell % SIDE) as f32 * STEP, (cell / SIDE) as f32 * STEP]
}

fn world_point(origin: [f32; 2], local: [f32; 2]) -> [f32; 2] {
    [origin[0] + local[0], origin[1] + local[1]]
}

fn unit_direction(direction: u8) -> [f32; 2] {
    let (x, z) = DIRECTIONS[usize::from(direction - 1)];
    let scale = if x != 0 && z != 0 {
        INVERSE_DIAGONAL
    } else {
        1.0
    };
    [x as f32 * scale, z as f32 * scale]
}

fn lateral_bend(bend: [i8; 3], t: f32) -> f32 {
    let [start, end, sway] = bend.map(f32::from);
    let cross = t * (1.0 - t);
    // The first two terms round the ends. The last bows the reach without
    // changing its endpoint headings.
    cross * (0.25 * (start * (1.0 - t) + end * t) + 4.0 * cross * sway)
}

fn curved_point(a: [f32; 2], b: [f32; 2], normal: [f32; 2], bend: [i8; 3], t: f32) -> [f32; 2] {
    let offset = lateral_bend(bend, t);
    [
        a[0] + (b[0] - a[0]) * t + normal[0] * offset,
        a[1] + (b[1] - a[1]) * t + normal[1] * offset,
    ]
}

fn inside(cell: usize) -> bool {
    let (x, z) = (cell % SIDE, cell / SIDE);
    x > 0 && z > 0 && x < SIDE - 1 && z < SIDE - 1
}

fn downstream(cell: usize, direction: u8) -> usize {
    let (dx, dz) = DIRECTIONS[usize::from(direction - 1)];
    cell.wrapping_add_signed(dx as isize + dz as isize * SIDE as isize)
}

fn mix_key(state: u32) -> u32 {
    let hash = (state ^ (state >> 16)).wrapping_mul(0x7feb_352d);
    (hash ^ (hash >> 15)).wrapping_mul(0x846c_a68b)
}

fn quantize_height(height: f32) -> u16 {
    ((height - SEA_LEVEL) * HEIGHT_SCALE + 0.5) as u16
}

fn width_at(flow: u16) -> u8 {
    26 + (flow.min(84) * 2 / 3) as u8
}

fn bed_cut(flow: u16) -> u16 {
    47 + flow.min(72) * 2 / 3
}

fn root(value: f32) -> f32 {
    if value == 0.0 {
        return 0.0;
    }
    // A short Newton refinement is enough for channel-width comparisons.
    let start = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    let next = 0.5 * (start + value / start);
    0.5 * (next + value / next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_geometry_matches_original_bits_at_bank_and_cell_boundaries() {
        // Digests from the original row-major sampler before the decoded cache.
        for (seed, rx, rz, original) in [
            (0x24a9_0ba8_bb59_ee70, 0, -1, 0x7cef_3764_acf9_8e55),
            (14_763, 2, 0, 0xdf0a_144d_a7a6_de37),
            (14_692, -2, 2, 0xbec6_f3e4_067d_c9f6),
            (98, 1, 2, 0x8add_87da_002d_e87c),
        ] {
            let mut water = Watershed::new();
            water.rebuild(&World::new(seed), seed, rx, rz, &mut WatershedWork::new());
            let mut cache = WaterNeighbors::new();
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            let bits = |sample: RiverSample| {
                [sample.level, sample.distance, sample.half_width, sample.bank_strength].map(f32::to_bits)
            };
            let mut emit = |x, z| {
                let cached = water.coverage.locate(x, z)
                    .and_then(|query| water.sample_neighbors(query, &mut cache))
                    .map(bits);
                assert_eq!(cached, water.sample(x, z).map(bits), "seed {seed}, x={x}, z={z}");
                if let Some(values) = cached {
                    for value in values {
                        hash = (hash ^ u64::from(value)).wrapping_mul(0x100_0000_01b3);
                    }
                } else {
                    hash = (hash ^ 0xffff_ffff).wrapping_mul(0x100_0000_01b3);
                }
            };
            const EPS: f32 = 1.0 / 64.0;
            let [ox, oz] = [rx, rz].map(|value| (value * REGION_SIZE) as f32);
            for j in 0..=COVER_SIDE {
                for i in 0..=COVER_SIDE {
                    for [dx, dz] in [[0.0, 0.0], [-EPS, EPS], [EPS, -EPS], [EPS, EPS], [-EPS, -EPS]] {
                        emit(ox + i as f32 * COVER_STEP + dx, oz + j as f32 * COVER_STEP + dz);
                    }
                }
            }
            for reach in water.reaches() {
                let dx = reach.end[0] - reach.start[0];
                let dz = reach.end[1] - reach.start[1];
                let length = (dx * dx + dz * dz).sqrt();
                for along in [0.0, 1.0/64.0, 0.125, 0.375, 0.5, 0.625, 0.875, 63.0/64.0, 1.0] {
                    let [x, z] = reach.position(along);
                    for side in -56..=56 {
                        let d = (side as f32 * 0.25) / length;
                        emit(x - dz * d, z + dx * d);
                    }
                }
            }
            assert_eq!(hash, original, "seed {seed}");
        }
    }

    #[test]
    fn cached_neighborhood_is_refilled_when_its_owner_changes() {
        let mut work = WatershedWork::new();
        let mut first = Watershed::new();
        let mut second = Watershed::new();
        first.rebuild(&World::new(1), 1, 0, 0, &mut work);
        second.rebuild(&World::new(1), 1, 0, 0, &mut work);
        // Keep both coverage maps identical while changing the cached geometry.
        for link in &mut second.links[..second.link_count] {
            link.widths = link.widths.map(|width| width - 3);
        }
        let mut cache = WaterNeighbors::new();
        let [x, z] = first.reaches().next().unwrap().position(0.5);
        let a = first.coverage.locate(x, z).unwrap();
        let b = second.coverage.locate(x, z).unwrap();
        let left = first.sample_covered(a).unwrap();
        let right = second.sample_covered(b).unwrap();
        assert_ne!(left, right);
        assert_eq!(first.sample_neighbors(a, &mut cache), Some(left));
        assert_eq!(second.sample_neighbors(b, &mut cache), Some(right));
        assert_eq!(first.sample_neighbors(a, &mut cache), Some(left));
    }

    #[test]
    fn reused_planner_produces_the_same_connected_channels() {
        let mut work = WatershedWork::new();
        let mut plan = Watershed::new();
        let world = World::new(0);
        let elsewhere = World::new(2);
        let mut reference = Watershed::new();
        reference.rebuild(&world, 0, 0, 0, &mut work);
        plan.rebuild(&elsewhere, 2, -3, -1, &mut work);
        assert!(!plan.streams().is_empty());
        assert!(plan.streams().iter().all(|stream| stream.source[0] < 0.0));
        plan.rebuild(&world, 0, 0, 0, &mut work);
        assert!(!plan.streams().is_empty());
        assert_eq!(plan.streams(), reference.streams());
        assert!(plan.reaches().eq(reference.reaches()));
        for stream in plan.streams() {
            let mut point = stream.source;
            let mut water = f32::INFINITY;
            for _ in 0..stream.links_to_sea {
                let reach = plan.reaches().find(|reach| reach.start == point).unwrap();
                assert!(reach.end_level <= reach.start_level && reach.start_level <= water);
                water = reach.end_level;
                point = reach.end;
            }
            assert_eq!(point, stream.outlet);
            assert_eq!(water, SEA_LEVEL);
            assert_eq!(world.elevation(point[0], point[1]), SEA_LEVEL);
        }
    }

    #[test]
    fn dense_cross_sections_preserve_water_and_dry_banks() {
        let world = World::new(1);
        let mut work = WatershedWork::new();
        let mut plan = Watershed::new();
        plan.rebuild(&world, 1, 0, 0, &mut work);
        let mut checked = 0;
        for reach in plan.reaches() {
            let [ax, az] = reach.start;
            let [bx, bz] = reach.end;
            let (dx, dz) = (bx - ax, bz - az);
            let length = root(dx * dx + dz * dz);
            for step in 6..43 {
                let t = f32::from(step as u8) * (1.0 / 48.0);
                let [x, z] = reach.position(t);
                let water = plan.sample(x, z).unwrap();
                assert!(water.is_channel());
                let ground = world.elevation(x, z);
                assert!(water.level <= ground + 0.03);
                if ground >= SEA_LEVEL + 0.38 {
                    let offset = (water.half_width + 0.82) / length;
                    let left = world.elevation(x - dz * offset, z + dx * offset);
                    let right = world.elevation(x + dz * offset, z - dx * offset);
                    assert!(left.min(right) > water.level);
                    checked += 1;
                }
            }
        }
        assert!(checked >= 370);
    }
}
