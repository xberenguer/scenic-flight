//! Independent waterfront settlements connected to the regional road.

use super::*;

const LANE_X: f32 = ENTRY_COL as f32 * SPACING;
const CORE_LOTS: usize = 8;
const BRANCH_LOTS: usize = 12;
const LOTS: usize = CORE_LOTS + 2 * BRANCH_LOTS;
const WALK_STEP: f32 = 3.0;
const WALK_HALF_WIDTH: f32 = 1.6;

mod landmarks;
use landmarks::WalkLandmark;

#[derive(Clone, Copy)]
struct WaterfrontWalk {
    start: [f32; 2],
    direction: u8,
    steps: u8,
    wings: [u8; 2],
}

impl WaterfrontWalk {
    fn direction(self) -> [f32; 2] {
        let (x, z, _, _) = DIRECTIONS[usize::from(self.direction)];
        [x as f32, z as f32]
    }

    fn end(self) -> [f32; 2] {
        walk_point(self.start, self.direction(), self.steps)
    }

    fn strips(self) -> impl Iterator<Item = Area> {
        let [x, z] = self.direction();
        let end = self.end();
        core::iter::once(walk_strip(self.start, end))
            .chain(self.wings.into_iter().zip([[-z, x], [z, -x]])
                .filter(|(steps, _)| *steps != 0)
                .map(move |(steps, direction)| walk_strip(end, walk_point(end, direction, steps))))
    }
}

pub(crate) struct WaterfrontVillage {
    local: Local,
    lanes: [Option<Area>; 3],
    square: Option<Area>,
    walk: Option<WaterfrontWalk>,
    landmarks: [Option<WalkLandmark>; 2],
    bases: [f32; LOTS],
    occupied: u32,
    key: u32,
    extent: Area,
}

impl WaterfrontVillage {
    pub(crate) fn survey(
        world: &World,
        road: &RoadNetwork,
        original: [Option<&Settlement>; 2],
        reserved: &impl Fn(f32, f32) -> bool,
        may_reserve: &dyn Fn([f32; 4]) -> bool,
        near_waterway: &dyn Fn(f32, f32) -> bool,
        waterway_cell: &dyn Fn(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let points = road.polyline();
        let mut best: Option<(usize, Self)> = None;
        // Highway junctions themselves are never moved. Look between them for
        // an independent landing village, leaving space around both old towns.
        for segment in points.windows(2).skip(2).take(points.len().saturating_sub(5)) {
            pulse();
            let [a, b] = [segment[0], segment[1]];
            let [x, z] = [(a.x + b.x) * 0.5, (a.z + b.z) * 0.5];
            if original.iter().flatten().any(|town| {
                (town.local.x - x).abs() < 106.0 && (town.local.z - z).abs() < 106.0
            }) {
                continue;
            }
            let point = RoadPoint { x, z, elevation: (a.elevation + b.elevation) * 0.5 };
            for sign in [-1.0, 1.0] {
                pulse();
                let local = Local::at_end(point, RoadPoint {
                    x: x - (b.z - a.z) * sign, z: z + (b.x - a.x) * sign, ..point
                });
                let river = [9.0, 27.0, 45.0].into_iter().any(|ahead| {
                    let (x, z) = local.world(LANE_X, ahead);
                    near_waterway(x, z)
                });
                let coast = [(-34.0, 25.0), (34.0, 25.0), (0.0, 62.0), (0.0, 86.0)]
                    .into_iter().any(|(across, ahead)| {
                        let (x, z) = local.world(LANE_X + across, ahead);
                        world.coast_signal(x, z) < -0.012
                    });
                if !river && !coast {
                    continue;
                }
                let reservation = Reservation {
                    local, original, original_bounds: original.map(|town| town.map(Settlement::bounds)),
                    neighborhood: None, neighborhood_bounds: None, reserved, may_reserve,
                };
                let mut length = 0.0;
                for _ in 0..3 {
                    if !street_fits(world, local, LANE_X, length, 0, 1)
                        || !reservation.clear_road([LANE_X, length], [0.0, 1.0], SPACING)
                    {
                        break;
                    }
                    length += SPACING;
                }
                if length < 2.0 * SPACING {
                    continue;
                }
                let lane = Area { x: LANE_X, z: length * 0.5, half_width: STREET_HALF_WIDTH, half_depth: length * 0.5 };
                let mut village = Self {
                    local, lanes: [Some(lane), None, None], square: None, walk: None,
                    landmarks: [None; 2],
                    bases: [0.0; LOTS], occupied: 0, key: town_key(world, local),
                    extent: Area::EMPTY,
                };
                for id in 0..2 * (length / 12.0) as usize {
                    pulse();
                    let (area, _, _) = village.design(id);
                    if highway_overlaps(road, local, area) || !reservation.clear_lot(area) {
                        continue;
                    }
                    if let Some(base) = foundation(world, local, area, 1.8) {
                        village.add(id, base);
                    }
                }
                let count = village.occupied.count_ones() as usize;
                if count < 4 {
                    continue;
                }
                let score = count * 2 + usize::from(river) * 3;
                if best.as_ref().is_none_or(|(old, _)| score > *old) {
                    best = Some((score, village));
                }
            }
        }
        let (_, mut village) = best?;
        let reservation = Reservation {
            local: village.local, original, original_bounds: original.map(|town| town.map(Settlement::bounds)),
            neighborhood: None, neighborhood_bounds: None, reserved, may_reserve,
        };
        village.grow(world, road, &reservation, pulse);
        village.walk = village.survey_walk(world, road, &reservation, waterway_cell, pulse);
        if let Some(walk) = village.walk {
            village.site_landmarks(walk, world, road, &reservation, pulse);
        }
        village.extent = village.areas().reduce(enclose).unwrap();
        Some(village)
    }

    fn survey_walk(
        &self,
        world: &World,
        road: &RoadNetwork,
        reservation: &Reservation<'_, impl Fn(f32, f32) -> bool>,
        waterway_cell: &dyn Fn(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<WaterfrontWalk> {
        let local = self.local;
        let water_distance = |[x, z]: [f32; 2]| {
            (1..=5).find(|&step| {
                let d = step as f32 * 8.0;
                DIRECTIONS.into_iter().any(|(a, b, _, _)| {
                    let (wx, wz) = local.world(x + a as f32 * d, z + b as f32 * d);
                    waterway_cell(wx, wz) || world.coast_signal(wx, wz) <= 0.0
                })
            })
        };
        let clear_step = |a, b| {
            let strip = walk_strip(a, b);
            if highway_overlaps(road, local, strip)
                || self.square.is_some_and(|square| overlaps(square, strip))
                || self.lots().any(|id| overlaps(self.design(id).0, strip))
                || !reservation.clear_lot(strip)
            {
                return false;
            }
            let x_step = b[0] - a[0];
            let z_step = b[1] - a[1];
            let side = if x_step == 0.0 { [WALK_HALF_WIDTH, 0.0] } else { [0.0, WALK_HALF_WIDTH] };
            [-1.0, 0.0, 1.0].into_iter().all(|offset| {
                let x = a[0] + offset * side[0];
                let z = a[1] + offset * side[1];
                let heights = [0.0, 0.5, 1.0].map(|t| local.ground(world, x + x_step * t, z + z_step * t));
                heights.iter().all(|&height| height >= SEA_LEVEL + 1.0)
                    && heights.windows(2).all(|pair| (pair[1] - pair[0]).abs() <= 0.48)
            })
        };
        let mut best = None;
        for (index, lane) in self.lanes.iter().enumerate().filter_map(|(i, lane)| lane.map(|lane| (i, lane))) {
            // Try beyond the street end and from two points along each side.
            let starts = if index == 0 {
                [([lane.x, lane.z + lane.half_depth], 1), ([lane.x, lane.z], 0),
                    ([lane.x, lane.z], 2), ([lane.x, lane.z + lane.half_depth - 8.0], 0),
                    ([lane.x, lane.z + lane.half_depth - 8.0], 2)]
            } else {
                let direction = if lane.x > LANE_X { 0 } else { 2 };
                let end = if direction == 0 { lane.x + lane.half_width } else { lane.x - lane.half_width };
                [([end, lane.z], direction), ([lane.x, lane.z], 1), ([lane.x, lane.z], 3),
                    ([end, lane.z], 1), ([end, lane.z], 3)]
            };
            for (start, direction) in starts {
                pulse();
                let mut walk = WaterfrontWalk { start, direction, steps: 0, wings: [0; 2] };
                let [x, z] = walk.direction();
                let start_distance = water_distance(start).unwrap_or(6);
                let mut best_end = None;
                for step in 1..=18 {
                    pulse();
                    if !clear_step(walk.end(), walk_point(start, [x, z], step)) {
                        break;
                    }
                    walk.steps = step;
                    if step >= 3 && step % 2 == 0 {
                        if let Some(distance) = water_distance(walk.end()) {
                            if distance <= 4 && distance <= start_distance
                                && best_end.is_none_or(|(old, _)| distance <= old)
                            {
                                best_end = Some((distance, step));
                            }
                        }
                    }
                }
                let Some((distance, steps)) = best_end else { continue; };
                walk.steps = steps;
                let end = walk.end();
                for (slot, direction) in [[-z, x], [z, -x]].into_iter().enumerate() {
                    for step in 1..=6 {
                        pulse();
                        let next = walk_point(end, direction, step);
                        if water_distance(next).is_none_or(|dist| dist > distance.max(3))
                            || !clear_step(walk_point(end, direction, step - 1), next)
                        {
                            break;
                        }
                        walk.wings[slot] = step;
                    }
                }
                let wings = u32::from(walk.wings[0]) + u32::from(walk.wings[1]);
                if wings < 2 {
                    continue;
                }
                let score = (6 - distance) * 12 + wings + u32::from(walk.steps) / 2;
                if best.as_ref().is_none_or(|(old, _)| score > *old) {
                    best = Some((score, walk));
                }
            }
        }
        best.map(|(_, walk)| walk)
    }

    fn grow(
        &mut self,
        world: &World,
        road: &RoadNetwork,
        reservation: &Reservation<'_, impl Fn(f32, f32) -> bool>,
        pulse: &mut impl FnMut(),
    ) {
        let local = self.local;
        let spine = self.lanes[0].unwrap();
        let length = spine.half_depth * 2.0;
        let mut extended = length;
        for _ in 0..2 {
            pulse();
            if !street_fits(world, local, LANE_X, extended, 0, 1)
                || !reservation.clear_road([LANE_X, extended], [0.0, 1.0], SPACING)
            {
                break;
            }
            extended += SPACING;
        }

        let district_side = if self.key & 1 == 0 { -1 } else { 1 };
        for (slot, side) in [district_side, -district_side].into_iter().enumerate() {
            let start = if slot == 1 && extended >= length + SPACING { extended - SPACING } else { extended };
            // A headland can cut off the end of the spine. Look back for a
            // dry parallel bench before giving up on the whole neighborhood.
            let mut best = (start, 0.0);
            for junction in [start, length, length - 8.0, length - 16.0, length - 24.0] {
                pulse();
                let mut reach = 0.0;
                for _ in 0..4 {
                    let x = LANE_X + reach * side as f32;
                    let next = x + SPACING * side as f32;
                    let strip = Area {
                        x: (x + next) * 0.5, z: junction,
                        half_width: SPACING * 0.5, half_depth: STREET_HALF_WIDTH,
                    };
                    if self.lots().any(|id| overlaps(self.design(id).0, strip))
                        || !street_fits(world, local, x, junction, side, 0)
                        || !reservation.clear_road([x, junction], [side as f32, 0.0], SPACING)
                    {
                        break;
                    }
                    reach += SPACING;
                    pulse();
                }
                if reach > best.1 {
                    best = (junction, reach);
                }
                if reach == 4.0 * SPACING {
                    break;
                }
            }
            let (junction, reach) = best;
            if reach < SPACING {
                continue;
            }
            let before = self.occupied;
            let branch = Area {
                x: LANE_X + reach * side as f32 * 0.5, z: junction,
                half_width: reach * 0.5, half_depth: STREET_HALF_WIDTH,
            };
            self.lanes[slot + 1] = Some(branch);
            let first = CORE_LOTS + slot * BRANCH_LOTS;
            let end = first + 2 * (reach / 12.0) as usize;
            for id in first..end {
                pulse();
                let (area, _, _) = self.design(id);
                let access = Area {
                    z: extended * 0.5, half_depth: extended * 0.5, ..spine
                };
                if overlaps(access, area)
                    || self.areas().any(|other| overlaps(other, area))
                    || highway_overlaps(road, local, area)
                    || !reservation.clear_lot(area)
                {
                    continue;
                }
                let Some(base) = foundation(world, local, area, 2.1) else { continue; };
                if id == CORE_LOTS + 1 {
                    self.square = Some(area);
                    continue;
                }
                self.add(id, base);
            }
            if (self.occupied ^ before).count_ones() < 2 {
                self.occupied = before;
                self.lanes[slot + 1] = None;
                if slot == 0 {
                    self.square = None;
                }
                continue;
            }
            let length = self.lanes[0].unwrap().half_depth * 2.0;
            if junction > length {
                self.lanes[0] = Some(Area {
                    z: junction * 0.5, half_depth: junction * 0.5, ..spine
                });
            }
        }
    }

    // Foundations are the surveyed part of each lot. All other geometry is
    // derived from the street, slot and seed to keep streamed towns compact.
    fn add(&mut self, id: usize, base: f32) {
        self.occupied |= 1 << id;
        self.bases[id] = base;
    }

    fn lots(&self) -> impl Iterator<Item = usize> {
        let mut remaining = self.occupied;
        core::iter::from_fn(move || {
            if remaining == 0 {
                return None;
            }
            let id = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            Some(id)
        })
    }

    fn design(&self, id: usize) -> (Area, Form, f32) {
        if id < CORE_LOTS {
            let variant = block_key(self.key, id);
            let sign = if id & 1 == 0 { -1.0 } else { 1.0 };
            let area = Area {
                x: LANE_X + sign * (6.5 + (variant & 7) as f32 * 0.32),
                z: 8.0 + (id / 2) as f32 * 12.0 + (variant >> 3 & 3) as f32 * 0.55,
                half_width: 2.2 + (variant >> 5 & 3) as f32 * 0.48,
                half_depth: 2.6 + (variant >> 7 & 3) as f32 * 0.47,
            };
            let style = (variant >> 16) as u8;
            let face = if sign < 0.0 { Frontage::Right } else { Frontage::Left };
            let form = if id < 2 {
                Form::Shop(face, style)
            } else if variant & 8 == 0 {
                Form::Duplex(style)
            } else {
                Form::Cottage(style)
            };
            return (area, form, 4.4 + (variant >> 10 & 7) as f32 * 0.38);
        }

        let branch_id = id - CORE_LOTS;
        let branch = self.lanes[1 + branch_id / BRANCH_LOTS].unwrap();
        let row = branch_id % BRANCH_LOTS / 2;
        let variant = block_key(self.key ^ 0x34d9_269f, branch_id);
        let center = branch_id < 2;
        let offset = if branch_id & 1 == 0 { -1.0 } else { 1.0 };
        let side = if branch.x < LANE_X { -1.0 } else { 1.0 };
        let area = Area {
            x: LANE_X + side * (7.5 + row as f32 * 12.0),
            z: branch.z + offset * (7.0 + (variant & 3) as f32 * 0.6),
            half_width: if center { 5.0 } else { 2.8 + (variant >> 2 & 3) as f32 * 0.58 },
            half_depth: 2.8 + (variant >> 4 & 3) as f32 * 0.56,
        };
        let face = if offset < 0.0 { Frontage::Far } else { Frontage::Near };
        let style = (variant >> 16) as u8;
        let (form, height) = if center {
            (Form::Civic(face), 9.5)
        } else if row == 1 && branch_id < BRANCH_LOTS {
            (Form::Shop(face, style), 6.6)
        } else if variant & 3 == 0 {
            (Form::Rowhouse(face, style), 6.0)
        } else if variant & 3 == 1 {
            (Form::Duplex(style), 5.1)
        } else {
            (Form::Cottage(style), 4.4)
        };
        (area, form, height)
    }

    fn areas(&self) -> impl Iterator<Item = Area> + '_ {
        self.lanes.iter().flatten().chain(self.square.iter()).copied()
            .chain(self.walk.into_iter().flat_map(WaterfrontWalk::strips))
            .chain(self.landmarks.iter().flatten().flat_map(WalkLandmark::areas))
            .chain(self.lots().map(|id| self.design(id).0))
    }

    pub(crate) fn footprints(&self, mut emit: impl FnMut([f32; 4])) {
        for area in self.areas() {
            let (x, z) = self.local.world(area.x, area.z);
            let [fx, fz] = [self.local.fx.abs(), self.local.fz.abs()];
            let dx = area.half_width * fz + area.half_depth * fx;
            let dz = area.half_width * fx + area.half_depth * fz;
            emit([x - dx, z - dz, x + dx, z + dz]);
        }
    }

    pub(crate) fn bounds(&self) -> [f32; 4] {
        let mut bounds = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
        self.footprints(|[x, z, u, v]| {
            bounds = [bounds[0].min(x), bounds[1].min(z), bounds[2].max(u), bounds[3].max(v)];
        });
        bounds
    }

    pub(crate) fn reserves(&self, x: f32, z: f32) -> bool {
        let (x, z) = self.local.grid(x, z);
        self.extent.contains(x, z) && self.areas().any(|area| area.contains(x, z))
    }

    pub(crate) fn sample(&self, x: f32, z: f32) -> Option<SettlementSample> {
        let (x, z) = self.local.grid(x, z);
        if !self.extent.contains(x, z) {
            return None;
        }
        if self.lanes.iter().flatten().any(|lane| lane.contains(x, z)) {
            return Some(SettlementSample { surface: SettlementSurface::Street, elevation: None });
        }
        if self.square.is_some_and(|square| square.contains(x, z)) {
            return Some(SettlementSample { surface: SettlementSurface::WaterfrontPlaza, elevation: None });
        }
        if self.walk.is_some_and(|walk| walk.strips().any(|strip| strip.contains(x, z))) {
            return Some(SettlementSample { surface: SettlementSurface::RiverWalk, elevation: None });
        }
        if let Some(sample) = self.landmarks.iter().flatten().find_map(|landmark| landmark.sample(x, z)) {
            return Some(sample);
        }
        self.lots().find_map(|id| {
            let (area, form, height) = self.design(id);
            Building::new(area, self.bases[id], height, form).sample(x, z)
        })
    }

    pub(crate) fn obstruction(&self, x: f32, z: f32) -> f32 {
        self.sample(x, z).and_then(|sample| sample.elevation).map_or(0.0, |height| height + 4.0)
    }

    pub(crate) fn roof_groups(&self) -> impl Iterator<Item = crate::skyline::SkylineTower> + '_ {
        self.lots().map(|id| {
            use crate::skyline::{SkylineTower, COTTAGE_GROUP, DUPLEX_GROUP, ROWHOUSE_GROUP, SHOP_GROUP};
            let (area, form, height) = self.design(id);
            let (x, z) = self.local.world(area.x, area.z);
            SkylineTower {
                center: [x, z],
                footprint: [area.half_width, area.half_depth],
                direction: [self.local.fx, self.local.fz],
                base: self.bases[id],
                height,
                profile: match form {
                    Form::Shop(_, _) | Form::Civic(_) => SHOP_GROUP,
                    Form::Duplex(_) => DUPLEX_GROUP,
                    Form::Rowhouse(_, _) => ROWHOUSE_GROUP,
                    _ => COTTAGE_GROUP,
                },
            }
        }).chain(self.landmarks.iter().flatten().map(|landmark| landmark.skyline(self.local)))
    }
}

fn walk_point([x, z]: [f32; 2], [dx, dz]: [f32; 2], steps: u8) -> [f32; 2] {
    let length = steps as f32 * WALK_STEP;
    [x + dx * length, z + dz * length]
}

fn walk_strip([x, z]: [f32; 2], [u, v]: [f32; 2]) -> Area {
    Area {
        x: (x + u) * 0.5, z: (z + v) * 0.5,
        half_width: if x == u { WALK_HALF_WIDTH } else { (x - u).abs() * 0.5 + 0.05 },
        half_depth: if z == v { WALK_HALF_WIDTH } else { (z - v).abs() * 0.5 + 0.05 },
    }
}

fn overlaps(a: Area, b: Area) -> bool {
    (a.x - b.x).abs() <= a.half_width + b.half_width
        && (a.z - b.z).abs() <= a.half_depth + b.half_depth
}

fn enclose(a: Area, b: Area) -> Area {
    let left = (a.x - a.half_width).min(b.x - b.half_width);
    let right = (a.x + a.half_width).max(b.x + b.half_width);
    let near = (a.z - a.half_depth).min(b.z - b.half_depth);
    let far = (a.z + a.half_depth).max(b.z + b.half_depth);
    Area {
        x: (left + right) * 0.5, z: (near + far) * 0.5,
        half_width: (right - left) * 0.5 + 0.001, half_depth: (far - near) * 0.5 + 0.001,
    }
}
