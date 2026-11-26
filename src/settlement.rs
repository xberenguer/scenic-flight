//! Compact settlements growing out of the actual termini of regional roads.

use crate::roads::{RoadNetwork, RoadPoint, ROAD_HALF_WIDTH, ROAD_REGION_SIZE};
use crate::render::{screen_region, view_distance, Camera, ScreenRect};
use crate::world::{World, SEA_LEVEL};
use crate::world_mesh::{bounds_in_view, WorldMesh};

mod neighborhoods;
mod building_forms;
mod expansion;
pub mod town_landmarks;
pub(crate) use expansion::{BoroughPaintCache, TownExpansion, WaterfrontVillage};

use neighborhoods::Neighborhoods;
use town_landmarks::TownLandmark;

const COLS: usize = 7;
const CORE_ROWS: usize = 6;
const ROWS: usize = CORE_ROWS + 1;
const JUNCTIONS: usize = COLS * ROWS;
const NO_ROOF: u8 = u8::MAX;
const BLOCKS: usize = (COLS - 1) * (ROWS - 1);
const ENTRY_COL: usize = COLS / 2;
const SPACING: f32 = 18.0;
const STREET_HALF_WIDTH: f32 = 2.1;
const STREET_GRADE: f32 = 0.175;
const DRY_HEIGHT: f32 = SEA_LEVEL + 1.7;
const EAST: u8 = 1;
const FORWARD: u8 = 2;
const WEST: u8 = 4;
const BACK: u8 = 8;
const DIRECTIONS: [(i32, i32, u8, u8); 4] = [
    (1, 0, EAST, WEST),
    (0, 1, FORWARD, BACK),
    (-1, 0, WEST, EAST),
    (0, -1, BACK, FORWARD),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Terminus {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SettlementSurface {
    Street,
    Park,
    WaterfrontPlaza,
    RiverWalk,
    ParkCanopy,
    PitchedRoof,
    FlatRoof,
    TowerRoof,
    IndustryBrick,
    IndustryConcrete,
    IndustryMetal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettlementSample {
    pub surface: SettlementSurface,
    /// Roof height. Streets and open space retain the terrain elevation.
    pub elevation: Option<f32>,
}

/// A connected perimeter street cap and the direction it leaves the town.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettlementRoadExit {
    pub point: RoadPoint,
    pub outward: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settlement {
    local: Local,
    streets: [u8; JUNCTIONS],
    parcels: [Parcel; BLOCKS],
    street_count: u8,
    building_count: u8,
    tower_count: u8,
    skyline_roofs: [u8; 4],
    neighborhoods: Neighborhoods,
}

impl Settlement {
    pub fn generate(world: &World, road: &RoadNetwork, end: Terminus) -> Option<Self> {
        Self::generate_with_pulse(world, road, end, || {})
    }

    pub(crate) fn generate_with_pulse(
        world: &World, road: &RoadNetwork, end: Terminus, mut pulse: impl FnMut(),
    ) -> Option<Self> {
        let points = road.polyline();
        let first = *points.first()?;
        let last = *points.last()?;
        let (anchor, approach) = match end {
            Terminus::Start => (first, *points.get(1)?),
            Terminus::End => (last, *points.get(points.len().checked_sub(2)?)?),
        };
        let local = Local::at_end(anchor, approach);
        let layout = town_key(world, local);
        // Reserve occasional road termini for scenery or future uses.
        if layout & 7 == 0 {
            return None;
        }
        let (streets, street_count) = grow_streets(world, local, layout, &mut pulse);
        if street_count < 7 || streets[ENTRY_COL] & FORWARD == 0 {
            return None;
        }
        let mut town = Self {
            local,
            streets,
            parcels: [Parcel::EMPTY; BLOCKS],
            street_count,
            building_count: 0,
            tower_count: 0,
            skyline_roofs: [NO_ROOF; 4],
            neighborhoods: Neighborhoods::EMPTY,
        };
        town.site_blocks(world, road, layout, &mut pulse);
        if town.building_count < 3 {
            return None;
        }
        town.shape_streets(world, layout, &mut pulse);
        town.extend_hillside(world, road, layout, &mut pulse);
        town.select_skyline_roofs();
        town.neighborhoods = Neighborhoods::survey(world, road, &town, layout, &mut pulse);
        Some(town)
    }

    /// Rank at most two perimeter caps with a continuous street route from
    /// the original highway entrance and room to continue along that tangent.
    pub fn coastal_road_exits(
        &self, world: &World, toward: RoadPoint,
    ) -> [Option<SettlementRoadExit>; 2] {
        let mut parent = [usize::MAX; JUNCTIONS];
        let mut queue = [0; JUNCTIONS];
        parent[ENTRY_COL] = ENTRY_COL;
        queue[0] = ENTRY_COL;
        let mut head = 0;
        let mut tail = 1;
        while head < tail {
            let node = queue[head];
            head += 1;
            for next in street_neighbors(&self.streets, node) {
                if parent[next] == usize::MAX {
                    parent[next] = node;
                    queue[tail] = next;
                    tail += 1;
                }
            }
        }

        let mut best = [(f32::NEG_INFINITY, None); 2];
        let origin = [toward.x - self.local.x, toward.z - self.local.z];
        let origin_length = horizontal_length(origin[0], origin[1]);
        if !origin_length.is_finite() || origin_length < 1.0 {
            return best.map(|(_, exit)| exit);
        }
        for node in 0..COLS * CORE_ROWS {
            let col = node % COLS;
            let row = node / COLS;
            if row == 0 || parent[node] == usize::MAX {
                continue;
            }
            for (boundary, back, outward) in [
                (row == CORE_ROWS - 1, node - COLS, [self.local.fx, self.local.fz]),
                (col == 0, node + 1, [-self.local.fz, self.local.fx]),
                (col == COLS - 1, node - 1, [self.local.fz, -self.local.fx]),
            ] {
                if !boundary || parent[back] == usize::MAX
                    || !street_neighbors(&self.streets, back).any(|next| next == node)
                {
                    continue;
                }
                let mut cursor = back;
                while cursor != ENTRY_COL && cursor != node {
                    cursor = parent[cursor];
                }
                if cursor != ENTRY_COL {
                    continue;
                }
                let (x, z) = self.local.world(col as f32 * SPACING, row as f32 * SPACING);
                let delta = [toward.x - x, toward.z - z];
                let distance = horizontal_length(delta[0], delta[1]);
                if !distance.is_finite() || distance < 1.0 {
                    continue;
                }
                let alignment = (delta[0] * outward[0] + delta[1] * outward[1]) / distance;
                if alignment < 0.50 {
                    continue;
                }
                let elevation = world.elevation(x, z);
                if elevation < DRY_HEIGHT {
                    continue;
                }
                let score = origin_length - distance + alignment * 36.0;
                let exit = Some(SettlementRoadExit { point: RoadPoint { x, z, elevation }, outward });
                if score > best[0].0 {
                    best[1] = best[0];
                    best[0] = (score, exit);
                } else if score > best[1].0 {
                    best[1] = (score, exit);
                }
            }
        }
        best.map(|(_, exit)| exit)
    }

    pub fn street_count(&self) -> u8 {
        self.street_count
    }

    pub fn building_count(&self) -> u8 {
        self.building_count
    }

    pub fn tower_count(&self) -> u8 {
        self.tower_count
    }

    pub fn town_landmarks(&self) -> impl Iterator<Item = ([f32; 2], TownLandmark)> + '_ {
        self.parcels.iter().flat_map(|parcel| parcel.buildings()).filter_map(|building| {
            let Form::TownLandmark(_, design) = building.form else { return None; };
            let (x, z) = self.local.world(building.area.x, building.area.z);
            Some(([x, z], design))
        })
    }

    pub fn skyline_towers(&self) -> impl Iterator<Item = crate::skyline::SkylineTower> + '_ {
        self.parcels.iter().take(if self.tower_count==0 {0}else{self.parcels.len()})
            .flat_map(|parcel| parcel.buildings()).filter(|building| matches!(building.form, Form::Tower(_)))
            .map(|building| {
                let area=building.area;
                let (x,z)=self.local.world(area.x,area.z);
                crate::skyline::SkylineTower {
                    center:[x,z],footprint:[area.half_width,area.half_depth],
                    direction:[self.local.fx,self.local.fz],base:building.base,height:building.height,
                    profile: building.tower_profile(),
                }
            })
    }

    fn select_skyline_roofs(&mut self) {
        // Keep one existing silhouette from each quarter. The frame loop
        // reads four packed building indices without scanning the parcels.
        let mut scores = [0.0; 4];
        for (block, parcel) in self.parcels.iter().enumerate() {
            let col = block % (COLS - 1);
            let row = block / (COLS - 1);
            let quarter = usize::from(col >= (COLS - 1) / 2)
                + 2 * usize::from(row >= (ROWS - 1) / 2);
            for (index, building) in parcel.buildings().iter().enumerate() {
                if matches!(building.form, Form::Tower(_)) {
                    continue;
                }
                let area = building.area;
                let height = match building.form {
                    Form::Civic(_) => building.height * 0.60,
                    Form::House => building.height + 0.85,
                    _ => building.height,
                };
                let score = (area.half_width + area.half_depth) * height;
                if score > scores[quarter] {
                    scores[quarter] = score;
                    self.skyline_roofs[quarter] = (2 * block + index) as u8;
                }
            }
        }
    }

    pub(crate) fn skyline_roofs(&self) -> impl Iterator<Item = crate::skyline::SkylineRoof> + '_ {
        use crate::skyline::Form as SkylineForm;
        self.skyline_roofs.into_iter().filter(|&id| id != NO_ROOF).map(|id| {
            self.parcels[usize::from(id) / 2].buildings[usize::from(id) % 2]
        }).map(|building| {
            let area = building.area;
            let (x, z) = self.local.world(area.x, area.z);
            let mut direction = [self.local.fx, self.local.fz];
            let mut footprint = [area.half_width, area.half_depth];
            let (height, form, profile) = match building.form {
                Form::House => (building.height + 1.7, SkylineForm::Gable, 0),
                Form::Civic(_) => (building.height * 0.60, SkylineForm::Gable, 0),
                Form::TownLandmark(frontage, design) => {
                    let [east, north] = direction;
                    direction = match frontage {
                        Frontage::Near => [-east, -north],
                        Frontage::Far => [east, north],
                        Frontage::Left => [-north, east],
                        Frontage::Right => [north, -east],
                    };
                    if matches!(frontage, Frontage::Left | Frontage::Right) { footprint.swap(0, 1); }
                    (building.height, SkylineForm::TownLandmark, design.id())
                }
                _ => (building.height, SkylineForm::Hall, 0),
            };
            crate::skyline::SkylineRoof {
                building: crate::skyline::SkylineTower {
                    center: [x, z],
                    footprint,
                    direction,
                    base: building.base,
                    height,
                    profile,
                },
                form,
            }
        })
    }

    pub(crate) fn skyline_neighborhoods(&self) -> impl Iterator<Item = crate::skyline::SkylineTower> + '_ {
        self.neighborhoods.roof_groups(self.local)
    }

    pub(crate) fn bounds(&self) -> [f32; 4] {
        let min = -STREET_HALF_WIDTH;
        let width = (COLS - 1) as f32 * SPACING + STREET_HALF_WIDTH;
        let last_row = if self.streets[COLS * CORE_ROWS..].iter().any(|&street| street != 0) {
            ROWS - 1
        } else {
            CORE_ROWS - 1
        };
        let length = last_row as f32 * SPACING + STREET_HALF_WIDTH;
        let [left, near, right, far] = self.neighborhoods.extent().map_or(
            [min, min, width, length],
            |[l, n, r, f]| [min.min(l - STREET_HALF_WIDTH), min.min(n - STREET_HALF_WIDTH),
                width.max(r + STREET_HALF_WIDTH), length.max(f + STREET_HALF_WIDTH)],
        );
        let [a, b, c, d] = [(left, near), (right, near), (right, far), (left, far)]
            .map(|(x, z)| self.local.world(x, z));
        [a.0.min(b.0).min(c.0).min(d.0), a.1.min(b.1).min(c.1).min(d.1),
            a.0.max(b.0).max(c.0).max(d.0), a.1.max(b.1).max(c.1).max(d.1)]
    }

    fn visible_towers(&self, camera: Camera) -> impl Iterator<Item = (Building, f32)> + '_ {
        self.parcels.iter().flat_map(|parcel| parcel.buildings().iter().copied())
            .filter(|building| matches!(building.form, Form::Tower(_)))
            .filter_map(move |building| {
                let (x, z) = self.local.world(building.area.x, building.area.z);
                let radius = building.area.half_width + building.area.half_depth;
                if !bounds_in_view(camera, [x - radius, z - radius, x + radius, z + radius]) {
                    return None;
                }
                let distance = (x - camera.x).abs().max((z - camera.z).abs());
                let far = view_distance(camera.altitude);
                let fade = crate::math::smoothstep((distance - 92.0) / 80.0)
                    * (1.0 - crate::math::smoothstep((distance - 0.85 * far) / (0.15 * far)));
                (fade > 0.02).then_some((building, fade))
            })
    }

    pub(crate) fn has_distant_towers(&self, camera: Camera) -> bool {
        self.tower_count > 0 && self.visible_towers(camera).next().is_some()
    }

    pub(crate) fn tower_region(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        if self.tower_count == 0 { return None; }
        self.visible_towers(camera).filter_map(|(tower, _)| {
            let area = tower.area;
            let radius = area.half_width + area.half_depth;
            let (x, z) = self.local.world(area.x, area.z);
            screen_region(camera, dimensions, [[x - radius, tower.base, z - radius],
                [x + radius, tower.base + tower.height, z + radius]])
        }).reduce(ScreenRect::union)
    }

    pub(crate) fn draw_distant_towers(&self, camera: Camera, mesh: &mut WorldMesh<'_>) {
        self.draw_distant_towers_faded(camera, mesh, |_, _| 1.0);
    }

    pub(crate) fn draw_distant_towers_faded(
        &self, camera: Camera, mesh: &mut WorldMesh<'_>, fade_factor: impl Fn(f32, f32) -> f32,
    ) {
        if self.tower_count == 0 { return; }
        for (tower, fade) in self.visible_towers(camera) {
            let area = tower.area;
            let (x, z) = self.local.world(area.x, area.z);
            let fade = fade * fade_factor(x, z);
            if fade < 0.015 { continue; }
            let profile = tower.tower_profile();
            let landmark = crate::skyline::tower_forms::landmark(profile);
            let (masses, wall) = if let Some(form) = landmark {
                (form.masses, form.wall)
            } else {
                let form = crate::skyline::tower_forms::design(profile);
                (form.tiers.as_slice(), form.wall)
            };
            for &tier in masses {
                let lower = tower.base + tower.height * tier.bottom;
                let upper = tower.base + tower.height * tier.top;
                let ring = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]].map(|[x, z]| {
                    self.local.world(
                        area.x + (tier.center[0] + x * tier.half[0]) * area.half_width,
                        area.z + (tier.center[1] + z * tier.half[1]) * area.half_depth,
                    )
                });
                let [a, b, c, d] = ring.map(|(x, z)| [x, upper, z]);
                let roof_color = wall.map(|channel| channel.saturating_add(24));
                mesh.faded_triangle([a, b, c], roof_color, fade);
                mesh.faded_triangle([a, c, d], roof_color, fade);
                if landmark.is_some() && tier.bottom > 0.0 {
                    let [a, b, c, d] = ring.map(|(x, z)| [x, lower, z]);
                    let underside = wall.map(|channel| channel.saturating_sub(26));
                    mesh.faded_triangle([a, c, b], underside, fade);
                    mesh.faded_triangle([a, d, c], underside, fade);
                }
                for side in 0..4 {
                    let (x0, z0) = ring[side];
                    let (x1, z1) = ring[(side + 1) % 4];
                    let shade = if side % 2 == 0 { 0 } else { 19 };
                    let color = wall.map(|channel| channel.saturating_sub(shade));
                    let a = [x0, lower, z0];
                    let b = [x0, upper, z0];
                    let c = [x1, upper, z1];
                    let d = [x1, lower, z1];
                    mesh.faded_triangle([a, b, c], color, fade);
                    mesh.faded_triangle([a, c, d], color, fade);
                }
            }
        }
    }

    pub(crate) fn obstruction(&self, x: f32, z: f32) -> f32 {
        const PAD: f32 = 23.0;
        let (across, ahead) = self.local.grid(x, z);
        let width = (COLS - 1) as f32 * SPACING;
        let length = (ROWS - 1) as f32 * SPACING;
        let neighborhood_peak = self.neighborhoods.obstruction(across, ahead);
        if across < -PAD || ahead < -PAD || across > width + PAD || ahead > length + PAD {
            return neighborhood_peak;
        }
        let first_col = ((across - PAD) / SPACING) as usize;
        let first_row = ((ahead - PAD) / SPACING) as usize;
        let last_col = (((across + PAD) / SPACING) as usize).min(COLS - 2);
        let last_row = (((ahead + PAD) / SPACING) as usize).min(ROWS - 2);
        let mut peak = neighborhood_peak;
        for row in first_row..=last_row {
            for col in first_col..=last_col {
                let parcel = &self.parcels[block_index(col, row)];
                for building in parcel.buildings() {
                    let dx = ((across - building.area.x).abs() - building.area.half_width).max(0.0);
                    let dz = ((ahead - building.area.z).abs() - building.area.half_depth).max(0.0);
                    let fraction = (dx * dx + dz * dz) * (1.0 / (PAD * PAD));
                    if fraction < 1.0 {
                        let weight = 1.0 - crate::math::smoothstep(fraction);
                        peak = peak.max((building.base + building.height + 2.0) * weight);
                    }
                }
            }
        }
        peak
    }

    pub fn sample(&self, x: f32, z: f32) -> Option<SettlementSample> {
        let (across, ahead) = self.local.grid(x, z);
        self.sample_existing(across, ahead)
    }

    /// Regional routes reserve the established town core. New neighborhoods
    /// yield to the transport network, which can cross or serve these districts.
    pub(crate) fn sample_transport_reservation(&self, x: f32, z: f32) -> Option<SettlementSample> {
        let (across, ahead) = self.local.grid(x, z);
        self.sample_core(across, ahead)
    }

    fn sample_existing(&self, across: f32, ahead: f32) -> Option<SettlementSample> {
        self.sample_core(across, ahead).or_else(|| self.neighborhoods.sample(across, ahead))
    }

    fn sample_core(&self, across: f32, ahead: f32) -> Option<SettlementSample> {
        let width = (COLS - 1) as f32 * SPACING;
        let length = (ROWS - 1) as f32 * SPACING;
        if !(across >= -STREET_HALF_WIDTH
            && across <= width + STREET_HALF_WIDTH
            && ahead >= -STREET_HALF_WIDTH
            && ahead <= length + STREET_HALF_WIDTH)
        {
            return None;
        }
        if self.on_street(across, ahead) {
            return Some(SettlementSample {
                surface: SettlementSurface::Street,
                elevation: None,
            });
        }
        if !(0.0..width).contains(&across) || !(0.0..length).contains(&ahead) {
            return None;
        }
        let col = (across / SPACING) as usize;
        let row = (ahead / SPACING) as usize;
        self.parcels[block_index(col, row)].sample(across, ahead)
    }

    fn on_street(&self, x: f32, z: f32) -> bool {
        let gx = x / SPACING;
        let gz = z / SPACING;
        let col = (gx + 0.5) as usize;
        let row = (gz + 0.5) as usize;
        let dx = (x - col as f32 * SPACING).abs();
        let dz = (z - row as f32 * SPACING).abs();
        let point = self.streets[row * COLS + col];
        if point != 0 && dx * dx + dz * dz <= STREET_HALF_WIDTH * STREET_HALF_WIDTH {
            return true;
        }
        if dz <= STREET_HALF_WIDTH && gx >= 0.0 && gx < (COLS - 1) as f32 {
            let link = self.streets[row * COLS + gx as usize];
            if link & EAST != 0 {
                return true;
            }
        }
        if dx <= STREET_HALF_WIDTH && gz >= 0.0 && gz < (ROWS - 1) as f32 {
            let link = self.streets[gz as usize * COLS + col];
            if link & FORWARD != 0 {
                return true;
            }
        }
        false
    }

    fn shape_streets(&mut self, world: &World, layout: u32, pulse: &mut impl FnMut()) {
        let budget = (self.street_count / 10).min(6);
        if budget < 2 {
            return;
        }
        let pattern = StreetPattern::for_site(world, self.local, layout);
        let mut frontages_in_use = [0_u8; JUNCTIONS];
        for (block, parcel) in self.parcels.iter().enumerate() {
            if parcel.is_empty() {
                continue;
            }
            let (col, row) = (block % (COLS - 1), block / (COLS - 1));
            let key = block_key(layout, block);
            let frontage = choose_frontage(frontages(&self.streets, col, row), key);
            let (start, direction) = frontage.edge(col, row);
            frontages_in_use[start] |= direction;
        }

        for _ in 0..budget {
            pulse();
            let removable = (0..JUNCTIONS).flat_map(|node| {
                [EAST, FORWARD].into_iter().map(move |direction| (node, direction))
            }).filter_map(|(node, direction)| {
                let (col, row) = (node % COLS, node / COLS);
                if self.streets[node] & direction == 0 || frontages_in_use[node] & direction != 0
                    || !keeps_perimeter_access(col, row, direction)
                    || !has_block_detour(&self.streets, node, direction)
                {
                    return None;
                }
                let rank = pattern.rank(col, row, direction, layout)?;
                let tie = shuffle(layout ^ (node as u32 * 4 + u32::from(direction)).wrapping_mul(0x65d3_6d27));
                Some(((rank, tie), node, direction))
            }).min_by_key(|candidate| candidate.0);
            let Some((_, node, direction)) = removable else {
                break;
            };
            let (next, reverse) = if direction == EAST { (node + 1, WEST) } else { (node + COLS, BACK) };
            self.streets[node] &= !direction;
            self.streets[next] &= !reverse;
            self.street_count -= 1;
        }
    }

    /// Grow a quiet housing row on a dry bench beyond a dense town.
    fn extend_hillside(&mut self, world: &World, road: &RoadNetwork, layout: u32, pulse: &mut impl FnMut()) {
        const LOTS: usize = 3;
        const FIRST_REAR_ROW: usize = CORE_ROWS - 1;
        const LAST_ROW: usize = ROWS - 1;
        if self.tower_count < 2 {
            return;
        }
        let inner_z = FIRST_REAR_ROW as f32 * SPACING;
        let outer_z = LAST_ROW as f32 * SPACING;
        let downtown = self.local.ground(world, ENTRY_COL as f32 * SPACING, 2.5 * SPACING);
        let tile_size = ROAD_REGION_SIZE as f32;
        let tile_origin = |value: f32| {
            let ratio = value / tile_size;
            let whole = ratio as i32;
            (whole - i32::from(ratio < whole as f32)) as f32 * tile_size
        };
        let tile_x = tile_origin(self.local.x);
        let tile_z = tile_origin(self.local.z);
        // Keep the added street inside the land stream's regional scenery margin.
        let [a, b] = [-STREET_HALF_WIDTH, (COLS - 1) as f32 * SPACING + STREET_HALF_WIDTH]
            .map(|x| self.local.world(x, outer_z + STREET_HALF_WIDTH));
        if [a, b].into_iter().any(|(x, z)| {
            !(tile_x + 96.0..tile_x + tile_size - 96.0).contains(&x)
                || !(tile_z + 96.0..tile_z + tile_size - 96.0).contains(&z)
        }) {
            return;
        }

        let starts = COLS - LOTS;
        let first = (layout >> 21) as usize % starts;
        for offset in 0..starts {
            pulse();
            let start = (first + offset) % starts;
            let middle = (start as f32 + LOTS as f32 * 0.5) * SPACING;
            let rise = self.local.ground(world, middle, outer_z) - downtown;
            if !(2.5..=13.0).contains(&rise) {
                continue;
            }

            let order = if layout & 0x4 == 0 { [1, 2] } else { [2, 1] };
            let Some(feeder) = order.into_iter().map(|i| start + i).find(|&col| {
                self.streets[FIRST_REAR_ROW * COLS + col] != 0
                    && street_fits(world, self.local, col as f32 * SPACING, inner_z, 0, 1)
            }) else {
                continue;
            };
            if !(start..start + LOTS).all(|col| {
                street_fits(world, self.local, col as f32 * SPACING, outer_z, 1, 0)
            }) {
                continue;
            }

            let lots = core::array::from_fn::<_, LOTS, _>(|i| {
                let col = start + i;
                let key = block_key(layout, block_index(col, FIRST_REAR_ROW));
                let along = ((key >> 9 & 7) as f32 - 3.5) * 0.41;
                let lot = Frontage::Far.lot(
                    (col as f32 + 0.5) * SPACING, inner_z + 0.5 * SPACING,
                    along, 0.9, 3.1, 2.6,
                );
                let base = Frontage::Far.site(world, road, self.local, lot, 1.75)?;
                let height = 4.4 + (key >> 19 & 7) as f32 * 0.28;
                Some(Building::new(lot, base, height, Form::House))
            });
            let buildings = lots.iter().flatten().count() as u8;
            if buildings < 2 {
                continue;
            }

            self.streets[FIRST_REAR_ROW * COLS + feeder] |= FORWARD;
            self.streets[LAST_ROW * COLS + feeder] |= BACK;
            for (i, building) in lots.into_iter().enumerate() {
                let node = LAST_ROW * COLS + start + i;
                self.streets[node] |= EAST;
                self.streets[node + 1] |= WEST;
                if let Some(building) = building {
                    self.parcels[block_index(start + i, FIRST_REAR_ROW)].add(building);
                }
            }
            self.street_count += 1 + LOTS as u8;
            self.building_count += buildings;
            return;
        }
    }

    fn site_blocks(&mut self, world: &World, road: &RoadNetwork, layout: u32, pulse: &mut impl FnMut()) {
        let metropolitan = self.street_count >= 32 && layout & 0x30 != 0;
        let mut civic_available = layout & 0x1800_0000 != 0;
        let mut landmark_count = 0;
        let mut industry_count = 0;
        let mut parks = 0;
        let mut waterfront = false;
        for row in 0..CORE_ROWS - 1 {
            for col in 0..COLS - 1 {
                pulse();
                let sides = frontages(&self.streets, col, row);
                if sides == 0 {
                    continue;
                }
                let index = block_index(col, row);
                let key = block_key(layout, index);
                let x = (col as f32 + 0.5) * SPACING;
                let z = (row as f32 + 0.5) * SPACING;
                let frontage = choose_frontage(sides, key);
                let middle = (2..=3).contains(&col) && (1..=3).contains(&row);
                let (lx, lz) = self.local.world(x, z);
                let ground = world.elevation(lx, lz);
                let mut parcel = Parcel::EMPTY;
                if !waterfront
                    && ground < SEA_LEVEL + 10.0
                    && key & 3 != 0
                    && shore_near(world, lx, lz, ground, pulse)
                {
                    let plaza = frontage.lot(x, z, 0.0, 1.8, 6.0, 4.8);
                    if frontage.site(world, road, self.local, plaza, 2.2).is_some() {
                        parcel = Parcel::with_open(plaza, SettlementSurface::WaterfrontPlaza);
                        waterfront = true;
                    }
                }
                if parcel.open().is_none() && parks < 2 && key & 15 < 2 {
                    let park = frontage.lot(x, z, 0.0, 1.5, 6.0, 5.2);
                    if frontage.site(world, road, self.local, park, 3.0).is_some() {
                        parcel = Parcel::with_open(park, SettlementSurface::Park);
                        parks += 1;
                    }
                }
                if parcel.open().is_some() {
                    self.parcels[index] = parcel;
                    continue;
                }

                if metropolitan
                    && middle
                    && sides.count_ones() >= 2
                    && self.tower_count < 4
                    && (key & 3 != 0 || self.tower_count == 0)
                {
                    use crate::skyline::tower_forms;
                    let regular = tower_forms::select_regular(layout, key, self.tower_count);
                    let design = tower_forms::design(regular);
                    let profile = tower_forms::select(layout, key, self.tower_count);
                    let site = |size: [f32; 2], height, profile| {
                        let tower = frontage.lot(x, z, 0.0, 0.5, 5.15 * size[0], 4.85 * size[1]);
                        let base = frontage.site(world, road, self.local, tower, 1.35)?;
                        Some((tower, base, height, profile))
                    };
                    let landmark = tower_forms::landmark(profile)
                        .and_then(|form| site(form.size, form.height, profile));
                    if let Some((tower, base, height, profile)) =
                        landmark.or_else(|| site(design.size, design.height, regular))
                    {
                        parcel.add(Building::new(tower, base, height, Form::Tower(profile)));
                        self.tower_count += 1;
                    }
                }
                let height = 7.0 + (key >> 19 & 7) as f32;
                let civic_candidate = middle && sides.count_ones() >= 2 && height >= 10.0;
                if parcel.buildings().is_empty() && !middle && row >= CORE_ROWS - 3
                    && industry_count < 2 && key >> 23 & 7 < 3
                {
                    let design = TownLandmark::industry(key.rotate_right(13));
                    let [width, depth, height] = design.dimensions();
                    let lot = frontage.lot(x, z, 0.0, 0.15, width, depth);
                    if let Some(base) = frontage.site(world, road, self.local, lot, 1.95) {
                        parcel.add(Building::new(lot, base, height, Form::TownLandmark(frontage, design)));
                        industry_count += 1;
                    }
                }
                if parcel.buildings().is_empty()
                    && landmark_count < 2
                    && key >> 25 & 7 < 2
                    && !(civic_available && civic_candidate)
                {
                    let family = if !middle && row >= CORE_ROWS - 3 {
                        3 + (key >> 17 & 1) as usize
                    } else if middle {
                        ((key >> 12 & 1) * 2) as usize
                    } else {
                        ((layout >> 21) + (key >> 15 & 1)) as usize % 3
                    };
                    let design = TownLandmark::select(family, key >> 6);
                    let [width, depth, height] = design.dimensions();
                    let lot = frontage.lot(x, z, 0.0, 0.20, width, depth);
                    if let Some(base) = frontage.site(world, road, self.local, lot, 1.95) {
                        parcel.add(Building::new(lot, base, height, Form::TownLandmark(frontage, design)));
                        landmark_count += 1;
                    }
                }
                if parcel.buildings().is_empty() && (middle || key & 7 < 3) {
                    let hall = frontage.lot(x, z, 0.0, 1.25, 5.4, 3.7);
                    if let Some(base) = frontage.site(world, road, self.local, hall, 1.9) {
                        let form = if civic_available && civic_candidate {
                            civic_available = false;
                            Form::Civic(frontage)
                        } else {
                            let variant = (key >> 10) as u8;
                            match (middle, row >= CORE_ROWS - 3, (key >> 7) & 3) {
                                (true, _, 0) => Form::Hall,
                                (true, _, 1) => Form::Rowhouse(frontage, variant),
                                (true, _, _) => Form::Shop(frontage, variant),
                                (false, true, 0..=2) => Form::Industrial(frontage, variant),
                                (false, _, 0..=1) => Form::Rowhouse(frontage, variant),
                                (false, _, _) => Form::Shop(frontage, variant),
                            }
                        };
                        parcel.add(Building::new(hall, base, height, form));
                    }
                }
                if parcel.buildings().is_empty() {
                    let setback = 2.25 + (key >> 15 & 3) as f32 * 0.27;
                    for (i, edge) in [-1.0, 1.0].into_iter().enumerate() {
                        let lot = frontage.lot(x, z, edge * 3.45, setback, 2.55, 3.05);
                        if let Some(base) = frontage.site(world, road, self.local, lot, 2.15) {
                            let height = 4.6 + (key >> (20 + 3 * i) & 7) as f32 * 0.39;
                            let variant = (key >> (4 + i * 8)) as u8;
                            let form = if row < 2 && variant & 3 == 0 {
                                Form::Duplex(variant >> 2)
                            } else {
                                Form::Cottage(variant)
                            };
                            parcel.add(Building::new(lot, base, height + 0.8, form));
                        }
                    }
                }
                self.building_count += parcel.buildings().len() as u8;
                self.parcels[index] = parcel;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Local {
    x: f32,
    z: f32,
    fx: f32,
    fz: f32,
}

impl Local {
    fn at_end(point: RoadPoint, approach: RoadPoint) -> Self {
        let dx = point.x - approach.x;
        let dz = point.z - approach.z;
        let length = horizontal_length(dx, dz);
        Self {
            x: point.x,
            z: point.z,
            fx: dx / length,
            fz: dz / length,
        }
    }

    fn world(self, across: f32, ahead: f32) -> (f32, f32) {
        let across = across - ENTRY_COL as f32 * SPACING;
        (
            self.x + self.fz * across + self.fx * ahead,
            self.z - self.fx * across + self.fz * ahead,
        )
    }

    fn grid(self, x: f32, z: f32) -> (f32, f32) {
        let dx = x - self.x;
        let dz = z - self.z;
        (
            dx * self.fz - dz * self.fx + ENTRY_COL as f32 * SPACING,
            dx * self.fx + dz * self.fz,
        )
    }

    fn ground(self, world: &World, across: f32, ahead: f32) -> f32 {
        let (x, z) = self.world(across, ahead);
        world.elevation(x, z)
    }
}

fn horizontal_length(x: f32, z: f32) -> f32 {
    let square = x * x + z * z;
    let mut length = x.abs().max(z.abs());
    if length == 0.0 {
        return length;
    }
    for _ in 0..3 {
        length = (length + square / length) * 0.5;
    }
    length
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Area {
    x: f32,
    z: f32,
    half_width: f32,
    half_depth: f32,
}

impl Area {
    const EMPTY: Self = Self {
        x: 0.0,
        z: 0.0,
        half_width: 0.0,
        half_depth: 0.0,
    };

    fn contains(self, x: f32, z: f32) -> bool {
        (x - self.x).abs() <= self.half_width && (z - self.z).abs() <= self.half_depth
    }

    #[cfg(test)]
    fn interior_samples(self) -> impl Iterator<Item = [f32; 2]> {
        (-4..=4).flat_map(move |row| {
            (-4..=4).map(move |col| [
                self.x + col as f32 * 0.20 * self.half_width,
                self.z + row as f32 * 0.20 * self.half_depth,
            ])
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Form {
    House,
    Hall,
    Tower(u16),
    Civic(Frontage),
    TownLandmark(Frontage, TownLandmark),
    Terrace(Frontage),
    CourtWing(Frontage),
    Workshop(Frontage),
    Cottage(u8),
    Duplex(u8),
    Rowhouse(Frontage, u8),
    Industrial(Frontage, u8),
    Shop(Frontage, u8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Building {
    area: Area,
    base: f32,
    height: f32,
    form: Form,
}

impl Building {
    const EMPTY: Self = Self::new(Area::EMPTY, 0.0, 0.0, Form::House);

    const fn new(area: Area, base: f32, height: f32, form: Form) -> Self {
        Self {
            area,
            base,
            height,
            form,
        }
    }

    fn tower_profile(self) -> u16 {
        let Form::Tower(profile) = self.form else { unreachable!() };
        profile
    }

    fn detached_axes(self, x: f32, z: f32, wide: bool) -> [f32; 2] {
        let [u, v] = [(x - self.area.x) / self.area.half_width, (z - self.area.z) / self.area.half_depth];
        if (self.area.half_width >= self.area.half_depth) == wide { [u, v] } else { [v, u] }
    }

    fn sample(self, x: f32, z: f32) -> Option<SettlementSample> {
        if !self.area.contains(x, z) {
            return None;
        }
        let across = (x - self.area.x).abs() / self.area.half_width;
        let along = (z - self.area.z).abs() / self.area.half_depth;
        let (surface, height) = match self.form {
            Form::House => {
                let span = if self.area.half_width < self.area.half_depth {
                    across
                } else {
                    along
                };
                (
                    SettlementSurface::PitchedRoof,
                    self.height + 1.7 * (1.0 - span),
                )
            }
            Form::Hall => (SettlementSurface::FlatRoof, self.height),
            Form::TownLandmark(frontage, design) => {
                let [face, mut side] = frontage.roof_axes(self.area, x, z);
                if matches!(frontage, Frontage::Near | Frontage::Right) { side = -side; }
                let (surface, profile) = design.roof([side, face])?;
                (surface, self.height * profile)
            }
            Form::Cottage(variant) => {
                let (surface, profile) = building_forms::cottage(variant, self.detached_axes(x, z, false))?;
                (surface, self.height * profile)
            }
            Form::Duplex(variant) => {
                let (surface, profile) = building_forms::duplex(variant, self.detached_axes(x, z, true))?;
                (surface, self.height * profile)
            }
            Form::Rowhouse(frontage, variant) => {
                let (surface, profile) = building_forms::rowhouse(variant, frontage.roof_axes(self.area, x, z))?;
                (surface, self.height * profile)
            }
            Form::Industrial(frontage, variant) => {
                let (surface, profile) = building_forms::industrial(variant, frontage.roof_axes(self.area, x, z))?;
                (surface, self.height * profile)
            }
            Form::Shop(frontage, variant) => {
                let (surface, profile) = building_forms::shop(variant, frontage.roof_axes(self.area, x, z))?;
                (surface, self.height * profile)
            }
            Form::Terrace(frontage) => {
                let [face, row] = frontage.roof_axes(self.area, x, z);
                let along = ((row + 1.0) * 1.5).min(2.9999);
                let bay = along as u8;
                let gable = 1.0 - (2.0 * (along - f32::from(bay)) - 1.0).abs();
                // The center house reaches the street first. Its neighbors
                // share the wall behind their shallower front gardens.
                if bay != 1 && face > 0.76 {
                    return None;
                }
                let cap = [0.88, 1.0, 0.93][usize::from(bay)];
                (SettlementSurface::PitchedRoof, self.height * cap * (0.70 + 0.30 * gable))
            }
            Form::CourtWing(frontage) => {
                let [face, side] = frontage.roof_axes(self.area, x, z);
                let side = side.abs();
                // Both higher tiers retreat from the green and leave a lower
                // terrace across the two ends of the apartment wing.
                if face < 0.12 && side < 0.63 {
                    (SettlementSurface::TowerRoof, self.height)
                } else if face < 0.64 && side < 0.84 {
                    (SettlementSurface::FlatRoof, self.height * 0.77)
                } else {
                    (SettlementSurface::FlatRoof, self.height * 0.56)
                }
            }
            Form::Workshop(frontage) => {
                let [face, side] = frontage.roof_axes(self.area, x, z);
                let along = ((face + 1.0) * 1.5).min(2.9999);
                let rise = along - f32::from(along as u8);
                // Three roof bays rise toward their clerestories. A shallower
                // roof at the yard edge gives the loading front its own line.
                if side.abs() > 0.94 || face > 0.91 {
                    (SettlementSurface::FlatRoof, self.height * 0.69)
                } else if rise > 0.78 {
                    (SettlementSurface::TowerRoof, self.height * (0.72 + 0.28 * rise))
                } else {
                    (SettlementSurface::FlatRoof, self.height * (0.72 + 0.28 * rise))
                }
            }
            Form::Civic(frontage) => {
                let [front, side] = frontage.roof_axes(self.area, x, z);
                let side = side.abs();
                // The entrance tower shares the hall's walls and faces its connected street.
                if front >= 0.15 && side <= 0.52 {
                    let cap = (side / 0.52).max(((front - 0.575) / 0.425).abs());
                    (SettlementSurface::TowerRoof, self.height * (1.0 - cap * 0.14))
                } else {
                    (SettlementSurface::PitchedRoof, self.height * (0.52 + 0.19 * (1.0 - front.abs())))
                }
            }
            Form::Tower(profile) => {
                let [u, v] = [
                    (x - self.area.x) / self.area.half_width,
                    (z - self.area.z) / self.area.half_depth,
                ];
                let within = |tier: &crate::skyline::tower_forms::Tier| {
                    (u - tier.center[0]).abs() < tier.half[0]
                        && (v - tier.center[1]).abs() < tier.half[1]
                };
                let roof = if let Some(design) = crate::skyline::tower_forms::landmark(profile) {
                    let height = design.masses.iter().filter(|tier| within(tier))
                        .map(|tier| tier.top).fold(0.0_f32, f32::max);
                    if height == 0.0 {
                        return Some(SettlementSample { surface: SettlementSurface::Street, elevation: None });
                    }
                    height
                } else {
                    let [podium, shaft, crown] = crate::skyline::tower_forms::design(profile).tiers;
                    if within(&crown) { crown.top }
                    else if within(&shaft) { shaft.top }
                    else { podium.top }
                };
                (SettlementSurface::TowerRoof, self.height * roof)
            }
        };
        Some(SettlementSample {
            surface,
            elevation: Some(self.base + height),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Parcel {
    buildings: [Building; 2],
    count: u8,
    open: Option<SettlementSurface>,
}

impl Parcel {
    const EMPTY: Self = Self { buildings: [Building::EMPTY; 2], count: 0, open: None };

    fn with_open(area: Area, surface: SettlementSurface) -> Self {
        // Parks and plazas share the space reserved for the first building.
        Self {
            buildings: [Building { area, ..Building::EMPTY }, Building::EMPTY],
            open: Some(surface),
            ..Self::EMPTY
        }
    }

    fn with_yard(building: Building, area: Area) -> Self {
        Self {
            buildings: [building, Building { area, ..Building::EMPTY }],
            count: 1,
            open: Some(SettlementSurface::Street),
        }
    }

    fn with_courtyard(buildings: [Building; 2]) -> Self {
        Self { buildings, count: 2, open: Some(SettlementSurface::Park) }
    }

    fn add(&mut self, building: Building) {
        self.buildings[self.count as usize] = building;
        self.count += 1;
    }

    fn is_empty(&self) -> bool {
        self.count == 0 && self.open.is_none()
    }

    fn buildings(&self) -> &[Building] {
        &self.buildings[..self.count as usize]
    }

    fn open(&self) -> Option<(Area, SettlementSurface)> {
        let surface = self.open?;
        let [a, b] = self.buildings.map(|building| building.area);
        let area = match self.count {
            0 => a,
            1 => b,
            _ => {
                let [dx, dz] = [(a.x - b.x).abs(), (a.z - b.z).abs()];
                Area {
                    x: (a.x + b.x) * 0.5,
                    z: (a.z + b.z) * 0.5,
                    half_width: if dx > dz {
                        (dx - a.half_width - b.half_width) * 0.5
                    } else {
                        a.half_width.min(b.half_width) * 0.88
                    },
                    half_depth: if dx > dz {
                        a.half_depth.min(b.half_depth) * 0.88
                    } else {
                        (dz - a.half_depth - b.half_depth) * 0.5
                    },
                }
            }
        };
        Some((area, surface))
    }

    fn sample(&self, x: f32, z: f32) -> Option<SettlementSample> {
        for &building in self.buildings() {
            if let Some(roof) = building.sample(x, z) {
                return Some(roof);
            }
        }
        let (area, surface) = self.open()?;
        area.contains(x, z).then_some(SettlementSample { surface, elevation: None })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Frontage {
    Near,
    Far,
    Left,
    Right,
}

impl Frontage {
    fn roof_axes(self, area: Area, x: f32, z: f32) -> [f32; 2] {
        let [u, v] = [(x - area.x) / area.half_width, (z - area.z) / area.half_depth];
        match self {
            Self::Near => [-v, u],
            Self::Far => [v, u],
            Self::Left => [-u, v],
            Self::Right => [u, v],
        }
    }

    fn edge(self, col: usize, row: usize) -> (usize, u8) {
        let node = row * COLS + col;
        match self {
            Self::Near => (node, EAST),
            Self::Far => (node + COLS, EAST),
            Self::Left => (node, FORWARD),
            Self::Right => (node + 1, FORWARD),
        }
    }

    fn site(
        self,
        world: &World,
        road: &RoadNetwork,
        local: Local,
        area: Area,
        tolerance: f32,
    ) -> Option<f32> {
        if highway_overlaps(road, local, area) {
            return None;
        }
        let height = foundation(world, local, area, tolerance)?;
        let block_x = (area.x / SPACING) as i32 as f32 * SPACING;
        let block_z = (area.z / SPACING) as i32 as f32 * SPACING;
        let (face_x, face_z, street_x, street_z) = match self {
            Self::Near => (area.x, area.z - area.half_depth, area.x, block_z),
            Self::Far => (area.x, area.z + area.half_depth, area.x, block_z + SPACING),
            Self::Left => (area.x - area.half_width, area.z, block_x, area.z),
            Self::Right => (area.x + area.half_width, area.z, block_x + SPACING, area.z),
        };
        let mut previous = local.ground(world, face_x, face_z);
        let max_change = (street_x - face_x).abs().max((street_z - face_z).abs()) * 0.25 / 5.0;
        for i in 1..=5 {
            let t = i as f32 / 5.0;
            let ground = local.ground(
                world,
                face_x + (street_x - face_x) * t,
                face_z + (street_z - face_z) * t,
            );
            if ground < DRY_HEIGHT || (ground - previous).abs() > max_change {
                return None;
            }
            previous = ground;
        }
        Some(height)
    }

    fn lot(self, x: f32, z: f32, along: f32, offset: f32, width: f32, depth: f32) -> Area {
        let (dx, dz, half_width, half_depth) = match self {
            Self::Near => (along, -offset, width, depth),
            Self::Far => (along, offset, width, depth),
            Self::Left => (-offset, along, depth, width),
            Self::Right => (offset, along, depth, width),
        };
        Area {
            x: x + dx,
            z: z + dz,
            half_width,
            half_depth,
        }
    }
}

fn block_index(col: usize, row: usize) -> usize {
    row * (COLS - 1) + col
}

fn street_neighbors(streets: &[u8; JUNCTIONS], node: usize) -> impl Iterator<Item = usize> + '_ {
    DIRECTIONS.into_iter().filter_map(move |(dx, dz, exit, entry)| {
        let x = (node % COLS) as i32 + dx;
        let z = (node / COLS) as i32 + dz;
        if !(0..COLS as i32).contains(&x) || !(0..ROWS as i32).contains(&z) {
            return None;
        }
        let next = z as usize * COLS + x as usize;
        (streets[node] & exit != 0 && streets[next] & entry != 0).then_some(next)
    })
}

fn frontages(streets: &[u8; JUNCTIONS], col: usize, row: usize) -> u8 {
    let index = row * COLS + col;
    u8::from(streets[index] & EAST != 0)
        | (u8::from(streets[index + COLS] & EAST != 0) << 1)
        | (u8::from(streets[index] & FORWARD != 0) << 2)
        | (u8::from(streets[index + 1] & FORWARD != 0) << 3)
}

fn choose_frontage(mask: u8, key: u32) -> Frontage {
    let sides = [
        Frontage::Near,
        Frontage::Far,
        Frontage::Left,
        Frontage::Right,
    ];
    let mut order = (key >> 7) % mask.count_ones();
    for (index, side) in sides.into_iter().enumerate() {
        if mask & (1 << index) != 0 {
            if order == 0 {
                return side;
            }
            order -= 1;
        }
    }
    Frontage::Near
}

enum StreetPattern {
    Waterfront { parallel: u8 },
    Terrace { parallel: u8 },
    Junction { cross_row: usize },
}

impl StreetPattern {
    fn for_site(world: &World, local: Local, layout: u32) -> Self {
        let right = (COLS - 1) as f32 * SPACING;
        let far = (CORE_ROWS - 1) as f32 * SPACING;
        let wet = |along: [f32; 3], side: f32, across: bool| {
            along.into_iter().filter(|&offset| {
                let (x, z) = if across { (side, offset) } else { (offset, side) };
                local.ground(world, x, z) <= SEA_LEVEL
            }).count()
        };
        let along_shore = wet([18.0, 45.0, 72.0], -14.0, true)
            .max(wet([18.0, 45.0, 72.0], right + 14.0, true));
        let across_shore = wet([18.0, 54.0, 90.0], -14.0, false)
            .max(wet([18.0, 54.0, 90.0], far + 14.0, false));
        if along_shore.max(across_shore) >= 2 && along_shore != across_shore {
            return Self::Waterfront { parallel: if along_shore > across_shore { FORWARD } else { EAST } };
        }

        let across_rise = (local.ground(world, 90.0, 45.0) - local.ground(world, 18.0, 45.0)).abs();
        let along_rise = (local.ground(world, 54.0, 81.0) - local.ground(world, 54.0, 9.0)).abs();
        if (across_rise - along_rise).abs() >= 5.0 {
            return Self::Terrace { parallel: if across_rise < along_rise { EAST } else { FORWARD } };
        }
        Self::Junction { cross_row: 1 + ((layout >> 15) as usize % (CORE_ROWS - 2)) }
    }

    fn rank(&self, col: usize, row: usize, direction: u8, layout: u32) -> Option<u8> {
        let lane = if direction == EAST { row } else { col };
        match *self {
            Self::Waterfront { parallel } => {
                let run = if direction == EAST { CORE_ROWS - 2 } else { COLS - 2 };
                let divide = 1 + ((layout >> 10) as usize % run);
                (direction != parallel).then_some(u8::from(lane != divide))
            }
            Self::Terrace { parallel } => {
                let alternate = ((layout >> 11) & 1) as usize;
                (direction != parallel).then_some(u8::from(lane % 2 != alternate))
            }
            Self::Junction { cross_row } => {
                if direction == EAST && row == cross_row {
                    return None;
                }
                let west = col < ENTRY_COL;
                let aligned = (direction == EAST) == (west ^ (layout & 0x80 != 0));
                Some(u8::from(aligned) + u8::from(row.abs_diff(cross_row) > 1))
            }
        }
    }
}

fn keeps_perimeter_access(col: usize, row: usize, direction: u8) -> bool {
    if direction == EAST {
        row > 0 && col > 0 && col + 2 < COLS
    } else {
        col != ENTRY_COL && row > 0 && row + 2 < CORE_ROWS
    }
}

/// Remove only a link with a three-edge alternate route around a dry block.
fn has_block_detour(streets: &[u8; JUNCTIONS], node: usize, direction: u8) -> bool {
    let (col, row) = (node % COLS, node / COLS);
    if direction == EAST {
        (row > 0 && streets[node - COLS] & EAST != 0
            && streets[node] & streets[node + 1] & BACK != 0)
            || (row + 1 < CORE_ROWS && streets[node + COLS] & EAST != 0
                && streets[node] & streets[node + 1] & FORWARD != 0)
    } else {
        (col > 0 && streets[node - 1] & FORWARD != 0
            && streets[node] & streets[node + COLS] & WEST != 0)
            || (col + 1 < COLS && streets[node + 1] & FORWARD != 0
                && streets[node] & streets[node + COLS] & EAST != 0)
    }
}

fn grow_streets(world: &World, local: Local, layout: u32, pulse: &mut impl FnMut()) -> ([u8; JUNCTIONS], u8) {
    let mut street = [0; JUNCTIONS];
    let mut tested = [0; JUNCTIONS];
    let mut seen = [false; JUNCTIONS];
    let mut queue = [0u8; JUNCTIONS];
    let mut viable = [false; JUNCTIONS];
    for (i, usable) in viable.iter_mut().enumerate() {
        pulse();
        let col = i % COLS;
        let row = i / COLS;
        // Let the outer blocks taper where a district reaches open country.
        let side = col.min(COLS - 1 - col);
        if row >= CORE_ROWS {
            continue;
        }
        let rear = row == CORE_ROWS - 1;
        if side == 0 && (row == 0 || rear) {
            continue;
        }
        if side == 1 && rear && (layout >> (8 + col)) & 1 != 0 {
            continue;
        }
        let x = col as f32 * SPACING;
        let z = row as f32 * SPACING;
        let junction = Area {
            x,
            z,
            half_width: STREET_HALF_WIDTH,
            half_depth: STREET_HALF_WIDTH,
        };
        *usable = foundation(world, local, junction, 0.9).is_some();
    }
    if !viable[ENTRY_COL] {
        return (street, 0);
    }
    queue[0] = ENTRY_COL as u8;
    seen[ENTRY_COL] = true;
    let (mut start, mut end) = (0, 1);
    let mut count = 0;
    while start < end {
        pulse();
        let index = queue[start] as usize;
        start += 1;
        let col = (index % COLS) as i32;
        let row = (index / COLS) as i32;
        for (dc, dr, direction, reverse) in DIRECTIONS {
            let c = col + dc;
            let r = row + dr;
            if !(0..COLS as i32).contains(&c)
                || !(0..CORE_ROWS as i32).contains(&r)
                || tested[index] & direction != 0
            {
                continue;
            }
            let next = r as usize * COLS + c as usize;
            tested[index] |= direction;
            tested[next] |= reverse;
            if !viable[next]
                || !street_fits(
                    world,
                    local,
                    col as f32 * SPACING,
                    row as f32 * SPACING,
                    dc,
                    dr,
                )
            {
                continue;
            }
            street[index] |= direction;
            street[next] |= reverse;
            count += 1;
            if !seen[next] {
                seen[next] = true;
                queue[end] = next as u8;
                end += 1;
            }
        }
    }
    (street, count)
}

fn street_fits(world: &World, local: Local, x: f32, z: f32, dc: i32, dr: i32) -> bool {
    street_fits_with_grade(world, local, [x, z], [dc, dr], STREET_GRADE)
}

fn street_fits_with_grade(
    world: &World, local: Local, [x, z]: [f32; 2], [dc, dr]: [i32; 2], grade: f32,
) -> bool {
    const STEPS: usize = 12;
    let dx = dc as f32;
    let dz = dr as f32;
    let mut previous = [0.0; 3];
    for step in 0..=STEPS {
        let d = step as f32 * (SPACING / STEPS as f32);
        let across = x + dx * d;
        let ahead = z + dz * d;
        let mut band = [0.0; 3];
        for (index, side) in [-STREET_HALF_WIDTH, 0.0, STREET_HALF_WIDTH]
            .into_iter()
            .enumerate()
        {
            let h = local.ground(world, across - dz * side, ahead + dx * side);
            if !(DRY_HEIGHT..=SEA_LEVEL + 51.0).contains(&h)
                || (step != 0
                    && (h - previous[index]).abs() > grade * SPACING / STEPS as f32)
            {
                return false;
            }
            band[index] = h;
        }
        if (band[0] - band[2]).abs() > 0.85 {
            return false;
        }
        previous = band;
    }
    true
}

fn foundation(world: &World, local: Local, area: Area, tolerance: f32) -> Option<f32> {
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for nz in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        for nx in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let x = area.x + (area.half_width + 0.35) * nx;
            let z = area.z + (area.half_depth + 0.35) * nz;
            let h = local.ground(world, x, z);
            if !(DRY_HEIGHT..=SEA_LEVEL + 48.0).contains(&h) {
                return None;
            }
            low = low.min(h);
            high = high.max(h);
            if high - low > tolerance {
                return None;
            }
        }
    }
    Some(high)
}

fn highway_overlaps(road: &RoadNetwork, local: Local, area: Area) -> bool {
    let margin = ROAD_HALF_WIDTH + 0.35;
    let (min_x, max_x) = (
        area.x - area.half_width - margin,
        area.x + area.half_width + margin,
    );
    let (min_z, max_z) = (
        area.z - area.half_depth - margin,
        area.z + area.half_depth + margin,
    );
    road.polyline().windows(2).any(|segment| {
        let (ax, az) = local.grid(segment[0].x, segment[0].z);
        let (bx, bz) = local.grid(segment[1].x, segment[1].z);
        ax.min(bx) <= max_x && ax.max(bx) >= min_x && az.min(bz) <= max_z && az.max(bz) >= min_z
    })
}

fn shore_near(world: &World, x: f32, z: f32, ground: f32, pulse: &mut impl FnMut()) -> bool {
    const DIAGONAL: f32 = core::f32::consts::FRAC_1_SQRT_2;
    const OUTWARD: [(f32, f32); 8] = [
        (1.0, 0.0),
        (DIAGONAL, DIAGONAL),
        (0.0, 1.0),
        (-DIAGONAL, DIAGONAL),
        (-1.0, 0.0),
        (-DIAGONAL, -DIAGONAL),
        (0.0, -1.0),
        (DIAGONAL, -DIAGONAL),
    ];
    for (dx, dz) in OUTWARD {
        pulse();
        let mut previous = ground;
        for step in 1..=10 {
            let distance = step as f32 * 6.0;
            let height = world.elevation(x + dx * distance, z + dz * distance);
            if height > ground + 0.6 || (height - previous).abs() > 1.25 {
                break;
            }
            if height == SEA_LEVEL {
                let far = distance + 13.0;
                let offshore_x = x + dx * far;
                let offshore_z = z + dz * far;
                if world.coast_signal(offshore_x - dz * 6.0, offshore_z + dx * 6.0) < -0.022
                    && world.coast_signal(offshore_x + dz * 6.0, offshore_z - dx * 6.0) < -0.022
                {
                    return true;
                }
                break;
            }
            previous = height;
        }
    }
    false
}

fn block_key(layout: u32, block: usize) -> u32 {
    shuffle(layout ^ (block as u32).wrapping_mul(0x9f81_695d))
}

fn town_key(world: &World, local: Local) -> u32 {
    let ground = world.elevation(local.x + 44.0, local.z - 30.0).to_bits();
    shuffle(ground ^ local.x.to_bits() ^ local.z.to_bits().rotate_left(13))
}

fn shuffle(key: u32) -> u32 {
    let key = (key ^ (key >> 15)).wrapping_mul(0x31ed_493d);
    let key = (key ^ (key >> 14)).wrapping_mul(0x9fd6_411b);
    key ^ (key >> 15)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: [(u64, i32, i32, Terminus); 3] = [
        (1, -1, 0, Terminus::Start),
        (1, -1, -3, Terminus::Start),
        (119, -5, 0, Terminus::Start),
    ];

    #[test]
    fn industrial_landmarks_have_distinct_roofs_and_surveyed_frontages() {
        use std::collections::HashSet;

        let mut silhouettes = HashSet::new();
        for index in 0..TownLandmark::INDUSTRY_COUNT {
            let design = TownLandmark::industry_from_index(index).unwrap();
            let silhouette: std::vec::Vec<_> = (0..25).flat_map(|z| (0..25).map(move |x| {
                let p = |i| (i as f32 + 0.5) * 0.08 - 1.0;
                design.roof([p(x), p(z)]).map_or(0, |(_, h)| (h * 10.0).ceil() as u8)
            })).collect();
            assert!(silhouette.iter().any(|&height| height > 0));
            assert!(silhouettes.insert(silhouette), "Repeated roof for {}", design.name());
        }

        let mut placed = HashSet::new();
        let mut skyline = HashSet::new();
        'worlds: for seed in 0..16 {
            let world = World::new(seed);
            for z in -8..=8 {
                for x in -8..=8 {
                    let Some(road) = RoadNetwork::generate(&world, x, z) else { continue; };
                    for end in [Terminus::Start, Terminus::End] {
                        let Some(town) = Settlement::generate(&world, &road, end) else { continue; };
                        for (index, parcel) in town.parcels.iter().enumerate() {
                            for building in parcel.buildings() {
                                let Form::TownLandmark(_, design) = building.form else { continue; };
                                if !design.is_industry() { continue; }
                                assert!(index / (COLS - 1) >= CORE_ROWS - 3);
                                assert_ne!(frontages(&town.streets, index % (COLS - 1), index / (COLS - 1)), 0);
                                assert_eq!(foundation(&world, town.local, building.area, 1.95), Some(building.base));
                                assert!(!highway_overlaps(&road, town.local, building.area));
                                placed.insert(design.id());
                            }
                        }
                        skyline.extend(town.skyline_roofs().filter(|roof| roof.form == crate::skyline::Form::TownLandmark)
                            .map(|roof| roof.building.profile)
                            .filter(|&profile| TownLandmark::from_index(profile as usize).unwrap().is_industry()));
                        if placed.len() == TownLandmark::INDUSTRY_COUNT && skyline.len() >= 20 { break 'worlds; }
                    }
                }
            }
        }
        assert_eq!(placed.len(), TownLandmark::INDUSTRY_COUNT);
        assert!(skyline.len() >= 20, "Industrial skyline profiles: {skyline:?}");
    }

    #[test]
    fn every_street_connects_to_the_regional_road() {
        assert!(core::mem::size_of::<Settlement>() <= 3000);
        for (seed, rx, rz, end) in FIXTURES {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, rx, rz).unwrap();
            let town = Settlement::generate(&world, &road, end).unwrap();
            assert_eq!(
                town,
                Settlement::generate(&World::new(seed), &road, end).unwrap()
            );
            let anchor = road.polyline()[0];
            assert!(road.proximity(anchor.x, anchor.z).is_some());
            assert_eq!(
                town.sample(anchor.x, anchor.z).unwrap().surface,
                SettlementSurface::Street
            );
            assert!(town.sample(anchor.x + 240.0, anchor.z + 240.0).is_none());

            let mut reached = [false; JUNCTIONS];
            reached[ENTRY_COL] = true;
            for _ in 0..JUNCTIONS {
                for i in 0..JUNCTIONS {
                    if !reached[i] {
                        continue;
                    }
                    let col = (i % COLS) as i32;
                    let row = (i / COLS) as i32;
                    for (dc, dr, direction, reverse) in DIRECTIONS {
                        if town.streets[i] & direction == 0 {
                            continue;
                        }
                        let c = col + dc;
                        let r = row + dr;
                        assert!((0..COLS as i32).contains(&c) && (0..ROWS as i32).contains(&r));
                        let j = r as usize * COLS + c as usize;
                        assert_ne!(town.streets[j] & reverse, 0);
                        reached[j] = true;
                    }
                }
            }
            assert!(town
                .streets
                .iter()
                .zip(reached)
                .all(|(&street, connected)| street == 0 || connected));
            let mut links = 0;
            for i in 0..JUNCTIONS {
                let x = (i % COLS) as f32 * SPACING;
                let z = (i / COLS) as f32 * SPACING;
                for (dc, dr, direction, _) in DIRECTIONS[..2].iter().copied() {
                    if town.streets[i] & direction == 0 {
                        continue;
                    }
                    links += 1;
                    assert!(street_fits(&world, town.local, x, z, dc, dr));
                    for offset in [0.0, 6.0, 12.0, SPACING] {
                        let (wx, wz) = town
                            .local
                            .world(x + dc as f32 * offset, z + dr as f32 * offset);
                        assert_eq!(
                            town.sample(wx, wz).unwrap().surface,
                            SettlementSurface::Street
                        );
                    }
                }
            }
            assert_eq!(town.street_count(), links);
        }
    }

    #[test]
    fn street_shaping_keeps_original_parcels_and_frontages() {
        let mut shaped = 0;
        for (seed, rx, rz, end) in FIXTURES {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, rx, rz).unwrap();
            let town = Settlement::generate(&world, &road, end).unwrap();
            let key = town_key(&world, town.local);
            let (streets, street_count) = grow_streets(&world, town.local, key, &mut || {});
            let mut original = Settlement {
                local: town.local, streets, street_count,
                parcels: [Parcel::EMPTY; BLOCKS], building_count: 0, tower_count: 0,
                skyline_roofs: [NO_ROOF; 4],
                neighborhoods: Neighborhoods::EMPTY,
            };
            original.site_blocks(&world, &road, key, &mut || {});
            assert_eq!(town.parcels[..block_index(0, CORE_ROWS - 1)], original.parcels[..block_index(0, CORE_ROWS - 1)]);
            let core_links = town.streets[..COLS * CORE_ROWS].iter().enumerate()
                .map(|(node, flags)| u8::from(flags & EAST != 0)
                    + u8::from(node / COLS < CORE_ROWS - 1 && flags & FORWARD != 0))
                .sum::<u8>();
            assert!(core_links <= original.street_count);
            shaped += original.street_count - core_links;
            for (node, (new, old)) in town.streets.into_iter().zip(original.streets)
                .take(COLS * CORE_ROWS).enumerate()
            {
                let core = if node / COLS == CORE_ROWS - 1 { new & !FORWARD } else { new };
                assert_eq!(core & old, core);
                assert_eq!(core != 0, old != 0);
            }
            for (block, parcel) in town.parcels.iter().enumerate().take(block_index(0, CORE_ROWS - 1)) {
                if parcel.is_empty() {
                    continue;
                }
                let (col, row) = (block % (COLS - 1), block / (COLS - 1));
                let frontage = choose_frontage(frontages(&original.streets, col, row), block_key(key, block));
                let (start, direction) = frontage.edge(col, row);
                assert_ne!(town.streets[start] & direction, 0);
            }
        }
        assert!(shaped > 0);
    }

    #[test]
    fn hillside_homes_join_a_dry_street_on_a_measured_rise() {
        let world = World::new(11);
        let road = RoadNetwork::generate(&world, -1, 0).unwrap();
        let town = Settlement::generate(&world, &road, Terminus::Start).unwrap();
        let core = CORE_ROWS - 1;
        let outer = ROWS - 1;
        let rear = &town.streets[core * COLS..outer * COLS];
        let feeders = rear.iter().filter(|&&node| node & FORWARD != 0).count();
        assert_eq!(feeders, 1);
        for (col, &node) in town.streets[outer * COLS..].iter().enumerate() {
            let x = col as f32 * SPACING;
            if node & BACK != 0 {
                assert_ne!(rear[col] & FORWARD, 0);
                assert!(street_fits(&world, town.local, x, core as f32 * SPACING, 0, 1));
            }
            if node & EAST != 0 {
                assert!(street_fits(&world, town.local, x, outer as f32 * SPACING, 1, 0));
            }
        }
        let houses = &town.parcels[block_index(0, core)..];
        assert_eq!(houses.iter().map(|p| p.buildings().len()).sum::<usize>(), 3);
        for (col, parcel) in houses.iter().enumerate() {
            for building in parcel.buildings() {
                assert_ne!(town.streets[outer * COLS + col] & EAST, 0);
                assert_eq!(foundation(&world, town.local, building.area, 1.75), Some(building.base));
                assert!(!highway_overlaps(&road, town.local, building.area));
                assert_eq!(building.form, Form::House);
            }
        }
    }

    #[test]
    fn parcels_are_dry_and_have_connected_frontages() {
        let mut street_styles = 0;
        for (seed, rx, rz, end) in FIXTURES {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, rx, rz).unwrap();
            let town = Settlement::generate(&world, &road, end).unwrap();
            let (mut towers, mut structures) = (0, 0);
            for (index, parcel) in town.parcels.iter().enumerate() {
                if !parcel.is_empty() {
                    assert_ne!(
                        frontages(&town.streets, index % (COLS - 1), index / (COLS - 1)),
                        0
                    );
                }
                for building in parcel.buildings() {
                    structures += 1;
                    let tolerance = match building.form {
                        Form::House | Form::Cottage(_) | Form::Duplex(_) => 2.15,
                        Form::Hall | Form::Civic(_) | Form::Rowhouse(_, _)
                        | Form::Shop(_, _) | Form::Industrial(_, _) => 1.9,
                        Form::TownLandmark(_, _) => 1.95,
                        Form::Tower(_) => {
                            towers += 1;
                            1.35
                        }
                        _ => panic!("unexpected form in core: {:?}", building.form),
                    };
                    street_styles |= match building.form {
                        Form::Cottage(_) => 1,
                        Form::Duplex(_) => 2,
                        Form::Rowhouse(_, _) => 4,
                        Form::Shop(_, _) => 8,
                        Form::Industrial(_, _) => 16,
                        _ => 0,
                    };
                    let area = building.area;
                    assert_eq!(
                        foundation(&world, town.local, area, tolerance),
                        Some(building.base)
                    );
                    assert!(!highway_overlaps(&road, town.local, area));
                    let roof_height = area.interior_samples().filter_map(|[x, z]| {
                        let (x, z) = town.local.world(x, z);
                        Some(town.sample(x, z)?.elevation? - world.elevation(x, z))
                    }).fold(0.0_f32, f32::max);
                    assert!(roof_height > 4.0);
                }
            }
            assert_eq!(town.building_count(), structures);
            assert_eq!(town.tower_count(), towers);
        }
        assert_eq!(street_styles, 31);
    }

    #[test]
    fn civic_hall_faces_its_street_inside_the_original_parcel() {
        let (seed, rx, rz, end) = FIXTURES[0];
        let world = World::new(seed);
        let road = RoadNetwork::generate(&world, rx, rz).unwrap();
        let town = Settlement::generate(&world, &road, end).unwrap();
        let mut civic = town.parcels.iter().enumerate().flat_map(|(index, parcel)| {
            parcel.buildings().iter().map(move |building| (index, building))
        }).filter(|(_, building)| matches!(building.form, Form::Civic(_)));
        let (index, &building) = civic.next().unwrap();
        assert!(civic.next().is_none());
        let (col, row) = (index % (COLS - 1), index / (COLS - 1));
        assert!((2..=3).contains(&col) && (1..=3).contains(&row));
        assert_eq!(building.form, Form::Civic(Frontage::Far));
        assert_ne!(frontages(&town.streets, col, row) & 2, 0);

        let Area { x, z, half_width, half_depth } = building.area;
        let tower = building.sample(x, z + half_depth * 0.575).unwrap();
        let hall = building.sample(x, z).unwrap();
        assert_eq!(tower.surface, SettlementSurface::TowerRoof);
        assert_eq!(hall.surface, SettlementSurface::PitchedRoof);
        assert!(tower.elevation.unwrap() - hall.elevation.unwrap() > 3.0);
        let flat = Building { form: Form::Hall, ..building };
        for i in -6..=6 {
            for j in -6..=6 {
                let point = (x + half_width * i as f32 / 5.0, z + half_depth * j as f32 / 5.0);
                let civic = building.sample(point.0, point.1);
                let old = flat.sample(point.0, point.1);
                assert_eq!(civic.is_some(), old.is_some());
                if let (Some(civic), Some(old)) = (civic, old) {
                    assert!(civic.elevation.unwrap() <= old.elevation.unwrap());
                }
            }
        }
    }

    #[test]
    fn coastal_district_has_open_space_and_a_mixed_skyline() {
        let (seed, rx, rz, end) = FIXTURES[0];
        let world = World::new(seed);
        let road = RoadNetwork::generate(&world, rx, rz).unwrap();
        let town = Settlement::generate(&world, &road, end).unwrap();
        let mut roof_count = [0; 3];
        let (mut park_count, mut plazas) = (0, 0);
        for (index, parcel) in town.parcels.iter().enumerate() {
            for building in parcel.buildings() {
                let kind = match building.form {
                    Form::House | Form::Cottage(_) | Form::Duplex(_) => 0,
                    Form::Hall | Form::Civic(_) | Form::Rowhouse(_, _)
                    | Form::Shop(_, _) | Form::Industrial(_, _) | Form::TownLandmark(_, _) => 1,
                    Form::Tower(_) => 2,
                    _ => panic!("unexpected form in core: {:?}", building.form),
                };
                roof_count[kind] += 1;
            }
            if let Some((area, surface)) = parcel.open() {
                let (x, z) = town.local.world(area.x, area.z);
                assert_eq!(
                    town.sample(x, z),
                    Some(SettlementSample {
                        surface,
                        elevation: None
                    })
                );
                match surface {
                    SettlementSurface::Park => park_count += 1,
                    SettlementSurface::WaterfrontPlaza => {
                        let cx = ((index % (COLS - 1)) as f32 + 0.5) * SPACING;
                        let cz = ((index / (COLS - 1)) as f32 + 0.5) * SPACING;
                        let (x, z) = town.local.world(cx, cz);
                        assert!(shore_near(&world, x, z, world.elevation(x, z), &mut || {}));
                        plazas += 1;
                    }
                    _ => panic!("unexpected open-space surface"),
                }
            }
        }
        assert!(roof_count[0] > roof_count[2] && roof_count[1] > 0 && roof_count[2] >= 2);
        assert!((1..=2).contains(&park_count));
        assert_eq!(plazas, 1);
    }
}
