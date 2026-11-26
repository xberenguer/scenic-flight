//! Street-connected districts around the town core. Each block stores only its
//! approved foundation; its building footprints are reconstructed from the seed.

use super::*;

const SIDE: usize = 19;
const ROWS: usize = 17;
const LEFT: i32 = -6;
const NODES: usize = SIDE * ROWS;
const BLOCKS: usize = (SIDE - 1) * (ROWS - 1);
const REAR: i32 = super::ROWS as i32 - 1;
const GROUP_SIDE: usize = 3;
const GROUP_COLS: usize = (SIDE - 1).div_ceil(GROUP_SIDE);
const GROUP_ROWS: usize = (ROWS - 1).div_ceil(GROUP_SIDE);

#[derive(Clone, Copy, Debug, PartialEq)]
struct RoofGroup {
    cells: u16,
    low: u8,
    high: u8,
    roof: u8,
}

impl RoofGroup {
    const EMPTY: Self = Self {
        cells: 0,
        low: u8::MAX,
        high: 0,
        roof: 0,
    };

    fn record(&mut self, position: usize, base: u8, height: f32) {
        self.cells |= 1 << position;
        self.low = self.low.min(base);
        self.high = self.high.max(base);
        self.roof = self.roof.max((f32::from(base) + 4.0 * height) as u8);
    }

    fn finish(self) -> u16 {
        if self.cells.count_ones() < 3 || self.high - self.low > 32 {
            return 0;
        }
        let mut min = [GROUP_SIDE; 2];
        let mut max = [0; 2];
        let mut cells = self.cells;
        while cells != 0 {
            let slot = cells.trailing_zeros() as usize;
            cells &= cells - 1;
            let position = [slot % GROUP_SIDE, slot / GROUP_SIDE];
            for axis in 0..2 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        if min[0] == max[0] || min[1] == max[1] {
            return 0;
        }
        let interval = |axis: usize| {
            if max[axis] - min[axis] == 2 {
                0
            } else {
                1 + min[axis] as u16
            }
        };
        let height = ((u16::from(self.roof - self.low) + 2) / 4).clamp(4, 15);
        // Foundation in quarter units, roof height in whole units, then
        // two interval codes describing which part of the 3x3 block is built.
        u16::from(self.low) | (height << 8) | (interval(0) << 12) | (interval(1) << 14)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Neighborhoods {
    streets: [u8; NODES],
    foundations: [u8; BLOCKS],
    roof_groups: [u16; GROUP_COLS * GROUP_ROWS],
    bounds: [u8; 4],
    key: u32,
}

impl Neighborhoods {
    pub(super) const EMPTY: Self = Self {
        streets: [0; NODES],
        foundations: [0; BLOCKS],
        roof_groups: [0; GROUP_COLS * GROUP_ROWS],
        bounds: [u8::MAX, u8::MAX, 0, 0],
        key: 0,
    };

    pub(super) fn survey(world: &World, road: &RoadNetwork, town: &Settlement, key: u32) -> Self {
        if town.tower_count < 2 {
            return Self::EMPTY;
        }
        let mut district = Self { key, ..Self::EMPTY };
        let local = town.local;
        let rx = crate::scenery::region_axis(local.x, ROAD_REGION_SIZE);
        let rz = crate::scenery::region_axis(local.z, ROAD_REGION_SIZE);
        let inside_region = |x: f32, z: f32| {
            let side = ROAD_REGION_SIZE as f32;
            let (x, z) = local.world(x, z);
            (160.0..side - 160.0).contains(&(x - rx as f32 * side))
                && (160.0..side - 160.0).contains(&(z - rz as f32 * side))
        };
        let mut queue = [0_u16; NODES];
        let mut tail = 0;
        let mut visited = [false; NODES];
        let mut rejected = [0_u8; NODES];
        for row in 0..super::ROWS {
            for col in 0..COLS {
                if town.streets[row * COLS + col] == 0
                    || (col != 0 && col + 1 != COLS && row < REAR as usize - 1)
                {
                    continue;
                }
                let node = row * SIDE + (col as i32 - LEFT) as usize;
                visited[node] = true;
                queue[tail] = node as u16;
                tail += 1;
            }
        }

        let mut head = 0;
        while head < tail {
            let node = usize::from(queue[head]);
            head += 1;
            let col = node as i32 % SIDE as i32 + LEFT;
            let row = (node / SIDE) as i32;
            let (x, z) = (col as f32 * SPACING, row as f32 * SPACING);
            if !inside_region(x, z) {
                continue;
            }
            for (dc, dr, flag, reverse) in DIRECTIONS {
                let (c, r) = (col + dc, row + dr);
                if !(LEFT..LEFT + SIDE as i32).contains(&c) || !(0..ROWS as i32).contains(&r) {
                    continue;
                }
                if core_node(c, r) && core_node(col, row) {
                    continue;
                }
                let next = r as usize * SIDE + (c - LEFT) as usize;
                if (district.streets[node] | rejected[node]) & flag != 0
                    || core_node(c, r) && !visited[next]
                    || !inside_region(c as f32 * SPACING, r as f32 * SPACING)
                {
                    continue;
                }
                if !street_fits(world, local, x, z, dc, dr) {
                    // The grade check samples the same bands in either direction.
                    rejected[node] |= flag;
                    rejected[next] |= reverse;
                    continue;
                }
                district.streets[node] |= flag;
                district.streets[next] |= reverse;
                district.include((col - LEFT) as usize, row as usize);
                district.include((c - LEFT) as usize, r as usize);
                if !visited[next] {
                    visited[next] = true;
                    queue[tail] = next as u16;
                    tail += 1;
                }
            }
        }
        let mut groups = [RoofGroup::EMPTY; GROUP_COLS * GROUP_ROWS];
        for block in 0..BLOCKS {
            let col = block % (SIDE - 1);
            let row = block / (SIDE - 1);
            let core_col = col as i32 + LEFT;
            if (0..COLS as i32 - 1).contains(&core_col) && row < REAR as usize {
                continue;
            }
            let node = row * SIDE + col;
            let street = district.streets[node] & (EAST | FORWARD) != 0
                || district.streets[node + 1] & FORWARD != 0
                || district.streets[node + SIDE] & EAST != 0;
            if !street {
                continue;
            }
            let lot = district.lot(block);
            if highway_overlaps(road, local, lot) {
                continue;
            }
            let Some(base) = foundation(world, local, lot, 2.6) else {
                continue;
            };
            district.foundations[block] = (base * 4.0 + 1.999) as u8;
            district.include(col, row);
            district.include(col + 1, row + 1);
            let height = district
                .parcel(block, 0.0)
                .buildings()
                .iter()
                .map(|building| building.height)
                .fold(0.0, f32::max);
            if height > 0.0 {
                let group = row / GROUP_SIDE * GROUP_COLS + col / GROUP_SIDE;
                let slot = row % GROUP_SIDE * GROUP_SIDE + col % GROUP_SIDE;
                groups[group].record(slot, district.foundations[block], height);
            }
        }
        district.roof_groups = groups.map(RoofGroup::finish);
        district
    }

    fn include(&mut self, col: usize, row: usize) {
        let (col, row) = (col as u8, row as u8);
        self.bounds[0] = self.bounds[0].min(col);
        self.bounds[1] = self.bounds[1].min(row);
        self.bounds[2] = self.bounds[2].max(col);
        self.bounds[3] = self.bounds[3].max(row);
    }

    pub(super) fn extent(&self) -> Option<[f32; 4]> {
        let [left, near, right, far] = self.bounds;
        (left != u8::MAX).then(|| {
            [
                (f32::from(left) + LEFT as f32) * SPACING,
                f32::from(near) * SPACING,
                (f32::from(right) + LEFT as f32) * SPACING,
                f32::from(far) * SPACING,
            ]
        })
    }

    pub(super) fn sample(&self, x: f32, z: f32) -> Option<SettlementSample> {
        let [left, near, right, far] = self.extent()?;
        if x < left - STREET_HALF_WIDTH
            || x > right + STREET_HALF_WIDTH
            || z < near - STREET_HALF_WIDTH
            || z > far + STREET_HALF_WIDTH
        {
            return None;
        }
        let gx = x / SPACING - LEFT as f32;
        let gz = z / SPACING;
        let col = (gx + 0.5) as usize;
        let row = (gz + 0.5) as usize;
        let dx = (gx - col as f32).abs() * SPACING;
        let dz = (gz - row as f32).abs() * SPACING;
        let node = self.streets[row * SIDE + col];
        let vertical = dx <= STREET_HALF_WIDTH
            && (0.0..(ROWS - 1) as f32).contains(&gz)
            && self.streets[gz as usize * SIDE + col] & FORWARD != 0;
        let horizontal = dz <= STREET_HALF_WIDTH
            && (0.0..(SIDE - 1) as f32).contains(&gx)
            && self.streets[row * SIDE + gx as usize] & EAST != 0;
        if vertical
            || horizontal
            || node != 0 && dx * dx + dz * dz <= STREET_HALF_WIDTH * STREET_HALF_WIDTH
        {
            return Some(SettlementSample {
                surface: SettlementSurface::Street,
                elevation: None,
            });
        }
        if !(0.0..(SIDE - 1) as f32).contains(&gx) || !(0.0..(ROWS - 1) as f32).contains(&gz) {
            return None;
        }
        self.block(gz as usize * (SIDE - 1) + gx as usize)?
            .sample(x, z)
    }

    pub(super) fn roof_groups(
        &self,
        local: Local,
    ) -> impl Iterator<Item = crate::skyline::SkylineTower> + '_ {
        self.roof_groups
            .iter()
            .enumerate()
            .filter_map(move |(index, &group)| {
                if group == 0 {
                    return None;
                }
                let extent = |shift: u32| match (group >> shift) & 3 {
                    1 => (1.0, SPACING - 2.5),
                    2 => (2.0, SPACING - 2.5),
                    _ => (1.5, 1.5 * SPACING - 2.5),
                };
                let (across, half_width) = extent(12);
                let (ahead, half_depth) = extent(14);
                let x = (index % GROUP_COLS * GROUP_SIDE) as f32 + LEFT as f32 + across;
                let z = (index / GROUP_COLS * GROUP_SIDE) as f32 + ahead;
                let (x, z) = local.world(x * SPACING, z * SPACING);
                Some(crate::skyline::SkylineTower {
                    center: [x, z],
                    footprint: [half_width, half_depth],
                    direction: [local.fx, local.fz],
                    base: f32::from((group & 255) - 1) * 0.25,
                    height: f32::from((group >> 8) & 15),
                    profile: (index & 3) as u8,
                })
            })
    }

    fn block(&self, index: usize) -> Option<Parcel> {
        let base = self.foundations[index];
        (base != 0).then(|| self.parcel(index, f32::from(base - 1) * 0.25))
    }

    fn lot(&self, index: usize) -> Area {
        let col = (index % (SIDE - 1)) as i32 + LEFT;
        let row = index / (SIDE - 1);
        Area {
            x: (col as f32 + 0.5) * SPACING,
            z: (row as f32 + 0.5) * SPACING,
            half_width: 7.1,
            half_depth: 7.1,
        }
    }

    fn parcel(&self, index: usize, base: f32) -> Parcel {
        let key = block_key(self.key, index);
        let area = self.lot(index);
        let distance = (area.x - ENTRY_COL as f32 * SPACING)
            .abs()
            .max((area.z - 45.0).abs());
        let district = block_key(
            self.key,
            index / ((SIDE - 1) * 3) * 6 + index % (SIDE - 1) / 3,
        );
        let style = key & 15;
        if style == 0 || style == 1 && distance > 120.0 {
            return Parcel::with_open(
                Area {
                    half_width: 6.8,
                    half_depth: 6.8,
                    ..area
                },
                SettlementSurface::Park,
            );
        }
        let mut parcel = Parcel::EMPTY;
        if style >= 5 && distance < 122.0 || district & 3 == 0 && style >= 9 {
            // Mid-rise apartments, office slabs and industrial sheds.
            let (half_width, half_depth, floors) = match district & 3 {
                0 => (6.75, 5.85, 2),
                1 => (4.65, 6.65, 5),
                2 => (6.45, 3.9, 4),
                _ => (5.25, 5.9, 3),
            };
            let offset = (key >> 8 & 3) as f32 * 0.28 - 0.42;
            parcel.add(Building::new(
                Area {
                    x: area.x + offset,
                    half_width: half_width - (key >> 10 & 3) as f32 * 0.34,
                    half_depth: half_depth - (key >> 12 & 3) as f32 * 0.35,
                    ..area
                },
                base,
                floors as f32 * 2.5 + (key >> 16 & 7) as f32 * 0.55,
                Form::Hall,
            ));
        } else {
            // Paired rows share a street block but vary in depth and roofline.
            for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
                let size = (key >> (8 + i * 4) & 15) as f32;
                let shift = (size - 7.5) * 0.11;
                let (x, z) = if district & 4 == 0 {
                    (side * 3.55, shift)
                } else {
                    (shift, side * 3.55)
                };
                let (half_width, half_depth) = if district & 4 == 0 {
                    (2.7, 4.8 + size * 0.075)
                } else {
                    (4.8 + size * 0.075, 2.7)
                };
                let form = if style >= 12 { Form::Hall } else { Form::House };
                parcel.add(Building::new(
                    Area {
                        x: area.x + x,
                        z: area.z + z,
                        half_width,
                        half_depth,
                    },
                    base,
                    4.6 + size * 0.36,
                    form,
                ));
            }
        }
        parcel
    }

    pub(super) fn obstruction(&self, x: f32, z: f32) -> f32 {
        let Some([left, near, right, far]) = self.extent() else {
            return 0.0;
        };
        const PAD: f32 = 14.0;
        if x < left - PAD || z < near - PAD || x > right + PAD || z > far + PAD {
            return 0.0;
        }
        let gx = x / SPACING - LEFT as f32;
        let gz = z / SPACING;
        let radius = PAD / SPACING;
        let mut peak = 0.0_f32;
        for row in (gz - radius) as usize..=((gz + radius) as usize).min(ROWS - 2) {
            for col in (gx - radius) as usize..=((gx + radius) as usize).min(SIDE - 2) {
                let Some(parcel) = self.block(row * (SIDE - 1) + col) else {
                    continue;
                };
                for building in parcel.buildings() {
                    let dx = ((x - building.area.x).abs() - building.area.half_width).max(0.0);
                    let dz = ((z - building.area.z).abs() - building.area.half_depth).max(0.0);
                    let factor = 1.0 - crate::math::smoothstep((dx * dx + dz * dz) / (PAD * PAD));
                    peak = peak.max((building.base + building.height + 4.0) * factor);
                }
            }
        }
        peak
    }
}

fn core_node(col: i32, row: i32) -> bool {
    (0..COLS as i32).contains(&col) && row < REAR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighborhood_grades_are_independent_of_traversal_direction() {
        for (seed, rx, rz) in [(1, -1, 0), (11, -1, 0), (119, -5, 0)] {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, rx, rz).unwrap();
            let town = Settlement::generate(&world, &road, Terminus::Start).unwrap();
            for row in 0..ROWS as i32 {
                for col in LEFT..LEFT + SIDE as i32 {
                    for (dc, dr) in [(1, 0), (0, 1)] {
                        let (c, r) = (col + dc, row + dr);
                        if c >= LEFT + SIDE as i32 || r >= ROWS as i32 {
                            continue;
                        }
                        assert_eq!(
                            street_fits(&world, town.local, col as f32 * SPACING, row as f32 * SPACING, dc, dr),
                            street_fits(&world, town.local, c as f32 * SPACING, r as f32 * SPACING, -dc, -dr),
                            "seed {seed}, edge ({col}, {row}) to ({c}, {r})",
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn large_districts_have_dry_blocks_on_connected_streets() {
        let mut largest = 0;
        for (seed, rx, rz) in [
            (1, -1, 0),
            (1, -1, -3),
            (11, -1, 0),
            (119, -5, 0),
            (41, 0, 0),
        ] {
            let world = World::new(seed);
            let Some(road) = RoadNetwork::generate(&world, rx, rz) else {
                continue;
            };
            for end in [Terminus::Start, Terminus::End] {
                let Some(town) = Settlement::generate(&world, &road, end) else {
                    continue;
                };
                let district = &town.neighborhoods;
                let mut reached = [false; NODES];
                let mut pending = std::collections::VecDeque::new();
                for row in 0..super::super::ROWS {
                    for col in 0..COLS {
                        if town.streets[row * COLS + col] != 0 {
                            let node = row * SIDE + (col as i32 - LEFT) as usize;
                            reached[node] = true;
                            pending.push_back(node);
                        }
                    }
                }
                while let Some(node) = pending.pop_front() {
                    for (dc, dr, flag, reverse) in DIRECTIONS {
                        if district.streets[node] & flag == 0 {
                            continue;
                        }
                        let next = (node as i32 + dc + dr * SIDE as i32) as usize;
                        assert_ne!(district.streets[next] & reverse, 0);
                        if !reached[next] {
                            reached[next] = true;
                            pending.push_back(next);
                        }
                    }
                }
                assert!(district
                    .streets
                    .iter()
                    .zip(reached)
                    .all(|(&links, connected)| links == 0 || connected));
                let mut buildings = 0;
                for index in 0..BLOCKS {
                    let Some(block) = district.block(index) else {
                        continue;
                    };
                    for building in block.buildings() {
                        let area = building.area;
                        let (x, z) = town.local.world(area.x, area.z);
                        let roof = town.sample(x, z).unwrap().elevation.unwrap();
                        assert!(roof > world.elevation(x, z) + 3.0);
                        assert!(!highway_overlaps(&road, town.local, area));
                        assert!(town.obstruction(x, z) >= roof);
                        buildings += 1;
                    }
                }
                largest = largest.max(buildings);
                std::eprintln!("seed={seed} region={rx},{rz} end={end:?} core={} neighborhood={buildings} bounds={:?}",
                    town.building_count(), district.extent());
            }
        }
        assert!(largest >= 90, "only {largest} buildings beyond the core");
    }
}
