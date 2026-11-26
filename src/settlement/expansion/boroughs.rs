//! Coarse surveyed collector streets with small buildings on the outer blocks.

use super::*;
use core::cell::Cell;

const SIDE: usize = 17;
const NODES: usize = SIDE * SIDE;
const SIDE_BLOCKS: usize = SIDE - 1;
const BLOCKS: usize = SIDE_BLOCKS * SIDE_BLOCKS;
const LEFT: i32 = -7;
const NEAR: i32 = -6;
const MARGIN: f32 = 99.0;
const ALLEYS: u8 = 1;
const FRONT_STRIP: u8 = 2;
const ALTERNATE_STRIP: u8 = 3;
const BASE_MASK: u8 = 63;
const FEEDERS: usize = 6;
const DIAGONALS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
struct DiagonalBlock {
    block: u8,
    rising: bool,
    foundations: [u8; 2],
}

impl DiagonalBlock {
    fn lot(self, borough: &Boroughs, side: usize) -> Area {
        let [x, z] = borough.node_point(block_node(usize::from(self.block)));
        let span = borough.span();
        let along = [0.24, 0.76][side];
        let across = if self.rising { 1.0 - along } else { along };
        Area {
            x: x + along * span,
            z: z + across * span,
            half_width: span * 0.11,
            half_depth: span * 0.12,
        }
    }

    fn building(self, borough: &Boroughs, side: usize) -> Option<Building> {
        let base = self.foundations[side];
        if base == 0 {
            return None;
        }
        let key = block_key(borough.key ^ 0x960e_68b7, usize::from(self.block) * 2 + side);
        let form = if key & 3 == 0 {
            let frontage = if self.rising == (side == 0) { Frontage::Near } else { Frontage::Far };
            Form::Shop(frontage, (key >> 8) as u8)
        } else {
            Form::Cottage((key >> 8) as u8)
        };
        let height = 5.0 + ((key >> 17) & 7) as f32 * 0.34;
        Some(Building::new(self.lot(borough, side), pad_base(base), height, form))
    }

    fn endpoints(self, borough: &Boroughs) -> [[f32; 2]; 2] {
        let node = block_node(usize::from(self.block));
        let [first, last] = if self.rising { [node, node + SIDE + 1] } else { [node + 1, node + SIDE] };
        [first, last].map(|corner| borough.node_point(corner))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Feeder {
    node: u16,
    contact: [i16; 2],
}

impl Feeder {
    fn endpoint(self) -> [f32; 2] {
        self.contact.map(|position| f32::from(position) * 0.25)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Boroughs {
    links: [u8; NODES.div_ceil(2)],
    pads: [u8; BLOCKS],
    diagonals: [Option<DiagonalBlock>; DIAGONALS],
    diagonal_rows: [u16; SIDE_BLOCKS],
    pad_uses: [Use; BLOCKS],
    bounds: [u8; 4],
    paint_bounds: Option<[f32; 4]>,
    stride: u8,
    inverse_span: f32,
    offset: [i8; 2],
    key: u32,
    feeders: [Option<Feeder>; FEEDERS],
    feeder_cells: [[u8; SIDE]; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum Use {
    Homes,
    Terrace,
    Warehouse,
    Works,
    Shops,
    Park,
    RiverNear,
    RiverFar,
    RiverLeft,
    RiverRight,
}

impl Use {
    const RIVER_PARKS: [Self; 4] = [Self::RiverNear, Self::RiverFar, Self::RiverLeft, Self::RiverRight];

    fn river_frontage(self) -> Option<Frontage> {
        match self {
            Self::RiverNear => Some(Frontage::Near),
            Self::RiverFar => Some(Frontage::Far),
            Self::RiverLeft => Some(Frontage::Left),
            Self::RiverRight => Some(Frontage::Right),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct RiverPark {
    frontage: Frontage,
    center: [f32; 2],
    half_span: f32,
}

impl RiverPark {
    fn feature(self, along: f32, setback: f32, half_width: f32, half_depth: f32) -> Area {
        self.frontage.lot(self.center[0], self.center[1], along,
            self.half_span - setback, half_width, half_depth)
    }

    fn footprint(self) -> Area {
        self.feature(0.0, 7.2, self.half_span * 0.73, 5.2)
    }

    fn by_bank(self, near_river: impl Fn(f32, f32) -> bool) -> bool {
        [2.0, 12.4].into_iter().all(|setback| {
            [-0.70, 0.0, 0.70].into_iter().all(|along| {
                let probe = self.feature(along * self.half_span, setback, 0.0, 0.0);
                near_river(probe.x, probe.z)
            })
        })
    }

    fn sample(self, x: f32, z: f32, base: f32, key: u32) -> Option<SettlementSample> {
        if !self.footprint().contains(x, z) {
            return None;
        }
        let width = self.half_span * 0.73;
        let walk = self.feature(0.0, 10.8, width, 1.55);
        let access = [-0.66, 0.66].into_iter().any(|side| {
            self.feature(side * width, 6.6, 1.2, 4.65).contains(x, z)
        });
        if walk.contains(x, z) || access {
            return Some(open(SettlementSurface::RiverWalk));
        }

        let middle = self.feature(0.0, 6.2, 3.15, 2.65);
        if middle.contains(x, z) {
            return Some(open(SettlementSurface::WaterfrontPlaza));
        }
        for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
            let spread = ((key >> (i * 3 + 6)) & 7) as f32;
            let size = 1.65 + spread * 0.11;
            let crown = self.feature(side * width * (0.34 + spread * 0.025), 6.5, size, size);
            let dx = (x - crown.x) / crown.half_width;
            let dz = (z - crown.z) / crown.half_depth;
            let radius = dx * dx + dz * dz;
            if radius < 1.0 {
                return Some(SettlementSample {
                    surface: SettlementSurface::ParkCanopy,
                    elevation: Some(base + 2.5 + (1.0 - radius) * (1.1 + spread * 0.15)),
                });
            }
        }
        Some(open(SettlementSurface::Park))
    }
}

#[derive(Clone, Copy)]
struct ResidentialLot {
    parcel: Parcel,
    yard: Area,
}

impl ResidentialLot {
    fn sample(self, x: f32, z: f32) -> Option<SettlementSample> {
        self.parcel.sample(x, z)
            .or_else(|| self.yard.contains(x, z).then_some(open(SettlementSurface::Park)))
    }
}

#[derive(Clone, Copy)]
struct CachedLot {
    borough: usize,
    plot: [usize; 3],
    lot: ResidentialLot,
}

pub(crate) struct BoroughPaintCache {
    lots: [Cell<Option<CachedLot>>; 16],
}

impl BoroughPaintCache {
    pub(crate) fn new() -> Self {
        Self { lots: [const { Cell::new(None) }; 16] }
    }

    fn residential(
        &self,
        borough: &Boroughs,
        plot: [usize; 3],
        prepare: impl FnOnce() -> ResidentialLot,
    ) -> ResidentialLot {
        let owner = core::ptr::from_ref(borough).addr();
        let [block, col, row] = plot;
        let slot = &self.lots[(block * 5 + col + row * 3) & (self.lots.len() - 1)];
        if let Some(entry) = slot.get() {
            if entry.borough == owner && entry.plot == plot {
                return entry.lot;
            }
        }
        let lot = prepare();
        slot.set(Some(CachedLot { borough: owner, plot, lot }));
        lot
    }
}

impl Boroughs {
    pub(super) const EMPTY: Self = Self {
        links: [0; NODES.div_ceil(2)],
        pads: [0; BLOCKS],
        diagonals: [None; DIAGONALS],
        diagonal_rows: [0; SIDE_BLOCKS],
        pad_uses: [Use::Homes; BLOCKS],
        bounds: [u8::MAX, u8::MAX, 0, 0],
        paint_bounds: None,
        stride: 3,
        inverse_span: 1.0 / (3.0 * SPACING),
        offset: [0; 2],
        key: 0,
        feeders: [None; FEEDERS],
        feeder_cells: [[0; SIDE]; 2],
    };

    #[inline(never)]
    pub(super) fn survey(
        world: &World,
        road: &RoadNetwork,
        town: &Settlement,
        claim: TownClaim,
        key: u32,
        reservation: &Reservation<'_, impl Fn(f32, f32) -> bool>,
        near_waterway: &dyn Fn(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Self {
        let mut best = Self::survey_at(world, road, town, claim, key, (3, [0, 0]), reservation, near_waterway, pulse);
        if town.building_count <= 13 && town.tower_count < 2 {
            let compact = Self::survey_at(world, road, town, claim, key, (2, [0, 0]), reservation, near_waterway, pulse);
            if compact.covered_estimate() > best.covered_estimate() {
                best = compact;
            }
        }
        if best.covered_estimate() < 30_000.0 {
            for grid in [(2, [0, 0]), (2, [9, 9]), (3, [18, 18]), (3, [-18, 18]), (3, [0, -18])] {
                let candidate = Self::survey_at(world, road, town, claim, key, grid, reservation, near_waterway, pulse);
                if candidate.covered_estimate() > best.covered_estimate() {
                    best = candidate;
                }
            }
        }
        best
    }

    fn covered_estimate(&self) -> f32 {
        let span = self.span();
        let edge_count = self
            .links
            .iter()
            .map(|flags| flags.count_ones())
            .sum::<u32>()
            / 2;
        let pad_area = self
            .pads
            .iter()
            .filter(|&&pad| pad != 0)
            .map(|pad| {
                if pad >> 6 >= FRONT_STRIP {
                    span * SPACING
                } else {
                    span * span
                }
            })
            .sum::<f32>();
        pad_area * 0.62 + edge_count as f32 * span * STREET_HALF_WIDTH * 2.0
    }

    fn span(&self) -> f32 {
        f32::from(self.stride) * SPACING
    }

    fn origin_x(&self) -> f32 {
        (ENTRY_COL % usize::from(self.stride)) as f32 * SPACING + f32::from(self.offset[0])
    }

    fn node_point(&self, node: usize) -> [f32; 2] {
        let span = self.span();
        [
            ((node % SIDE) as i32 + LEFT) as f32 * span + self.origin_x(),
            ((node / SIDE) as i32 + NEAR) as f32 * span + f32::from(self.offset[1]),
        ]
    }

    fn survey_at(
        world: &World,
        road: &RoadNetwork,
        town: &Settlement,
        claim: TownClaim,
        key: u32,
        (stride, offset): (u8, [i8; 2]),
        reservation: &Reservation<'_, impl Fn(f32, f32) -> bool>,
        near_waterway: &dyn Fn(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Self {
        let local = town.local;
        let points = road.polyline();
        let origin = [
            (crate::scenery::region_axis(local.x, ROAD_REGION_SIZE) * ROAD_REGION_SIZE) as f32,
            (crate::scenery::region_axis(local.z, ROAD_REGION_SIZE) * ROAD_REGION_SIZE) as f32,
        ];
        let owns = |x, z| {
            let (wx, wz) = local.world(x, z);
            let dry = world.elevation(wx, wz);
            (MARGIN..ROAD_REGION_SIZE as f32 - MARGIN).contains(&(wx - origin[0]))
                && (MARGIN..ROAD_REGION_SIZE as f32 - MARGIN).contains(&(wz - origin[1]))
                && claim.permits(wx, wz)
                && (DRY_HEIGHT..=SEA_LEVEL + 49.0).contains(&dry)
        };
        let river_near = |x, z| {
            let (wx, wz) = local.world(x, z);
            near_waterway(wx, wz)
        };
        let waterfront = |x, z| {
            let (wx, wz) = local.world(x, z);
            near_waterway(wx, wz) || (0.0..0.12).contains(&world.coast_signal(wx, wz))
        };
        let mut area = Self {
            key,
            stride,
            inverse_span: 1.0 / (f32::from(stride) * SPACING),
            offset,
            ..Self::EMPTY
        };
        let span = area.span();
        let steps = usize::from(stride) * 6;
        let mut queue = [0u16; NODES];
        let mut state = [0u8; NODES];
        // Two seeded growth directions make a long, uneven fringe. Waterfront
        // sites can continue past it, but still need dry, connected streets.
        let angle = (key >> 16 & 7) as usize;
        let [axis_x, axis_z] = [
            [1.0, 0.0], [0.924, 0.383], [0.707, 0.707], [0.383, 0.924],
            [0.0, 1.0], [-0.383, 0.924], [-0.707, 0.707], [-0.924, 0.383],
        ][angle];
        let valid = core::array::from_fn::<_, NODES, _>(|node| {
            pulse();
            let [x, z] = area.node_point(node);
            if !owns(x, z) || reservation.blocks(x, z, true) {
                return false;
            }
            let [east, north] = [x - 54.0, z - 54.0];
            if east.abs().max(north.abs()) <= 170.0 {
                return true;
            }
            let along = east * axis_x + north * axis_z;
            let across = north * axis_x - east * axis_z;
            let reach = if along < 0.0 { 335.0 } else { 425.0 };
            let (wx, wz) = local.world(x, z);
            let edge = world.noise(wx, wz, 0.011, key ^ 0xb832_278f) * 54.0;
            let water = if waterfront(x, z) { 92.0 } else { 0.0 };
            along.abs() + across.abs() * 0.81 < reach + edge + water
        });
        let mut tail = 0;
        let mut nearby_highway = [(f32::INFINITY, None); NODES];
        const CONTACT: f32 = ROAD_HALF_WIDTH + 0.6;
        for node in 0..NODES {
            pulse();
            if !valid[node] {
                continue;
            }
            let [x, z] = area.node_point(node);
            let (wx, wz) = local.world(x, z);
            let on_highway = road
                .proximity(wx, wz)
                .is_some_and(|road| road.distance_squared <= CONTACT * CONTACT);
            if on_highway
                || town
                    .sample_existing(x, z)
                    .is_some_and(|sample| sample.surface == SettlementSurface::Street)
            {
                queue[tail] = node as u16;
                tail += 1;
                state[node] = 16;
            } else if let Some((contact, distance)) = nearest_contact(points, local, [x, z]) {
                if owns(contact[0], contact[1]) {
                    nearby_highway[node] = (distance, Some(contact));
                }
            }
        }
        let mut connected = 0;
        for _ in 0..FEEDERS * 4 {
            pulse();
            if connected == FEEDERS {
                break;
            }
            let candidate = nearby_highway
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.1.is_some())
                .min_by(|(_, a), (_, b)| a.0.total_cmp(&b.0));
            let Some((node, &(length, Some(contact)))) = candidate else {
                break;
            };
            nearby_highway[node].1 = None;
            let start = area.node_point(node);
            let direction = [
                (contact[0] - start[0]) / length,
                (contact[1] - start[1]) / length,
            ];
            let conflict = area.feeders.iter().flatten().any(|feeder| {
                let [px, pz] = feeder.endpoint();
                (contact[0] - px).abs() < 60.0 && (contact[1] - pz).abs() < 60.0
            });
            let steps = (length / 2.0) as usize + 1;
            let touches_building = (0..=steps).any(|step| {
                let distance = step as f32 * length / steps as f32;
                town.sample_existing(
                    start[0] + direction[0] * distance,
                    start[1] + direction[1] * distance,
                )
                .is_some_and(|spot| spot.elevation.is_some())
            });
            if conflict
                || touches_building
                || !street_segment(world, local, start, direction, length, steps)
                || !reservation.clear_road(start, direction, length)
            {
                continue;
            }
            let feeder = Feeder {
                node: node as u16,
                contact: contact.map(|coord| (coord * 4.0 + coord.signum() * 0.5) as i16),
            };
            area.feeders[connected] = Some(feeder);
            connected += 1;
            area.include(node);
            let [gx, gz] = [
                (contact[0] - area.origin_x()) / span - LEFT as f32,
                (contact[1] - f32::from(area.offset[1])) / span - NEAR as f32,
            ];
            if gx >= 0.0 && gz >= 0.0 && gx < SIDE_BLOCKS as f32 && gz < SIDE_BLOCKS as f32 {
                let corner = gz as usize * SIDE + gx as usize;
                area.include(corner);
                area.include(corner + SIDE + 1);
            }
            queue[tail] = node as u16;
            tail += 1;
            state[node] = 16;
        }
        let mut head = 0;
        let mut links = 0;
        let limit = 194 + 3 * usize::from(town.building_count).min(44);
        while head < tail && links < limit {
            pulse();
            let node = usize::from(queue[head]);
            head += 1;
            let [x, z] = area.node_point(node);
            let col = (node % SIDE) as i32;
            let row = (node / SIDE) as i32;
            let shuffle = block_key(key ^ 0xe84a_590b, node) as usize;
            for i in 0..4 {
                let (dc, dr, flag, reverse) = DIRECTIONS[(i + shuffle) & 3];
                let c = col + dc;
                let r = row + dr;
                if !(0..SIDE as i32).contains(&c)
                    || !(0..SIDE as i32).contains(&r)
                    || state[node] & flag != 0
                {
                    continue;
                }
                let next = r as usize * SIDE + c as usize;
                state[node] |= flag;
                state[next] |= reverse;
                // Keep every newly reached node connected. On the outskirts,
                // omit most cross streets between existing branches.
                let edge_style = block_key(key ^ 0x74ad_2241, node.min(next) * 2 + usize::from(dc == 0));
                let [nx, nz] = area.node_point(next);
                let east = (x + nx) * 0.5 - 54.0;
                let north = (z + nz) * 0.5 - 54.0;
                let fringe = east * east + north * north > 165.0 * 165.0;
                if fringe && state[next] & 16 != 0 && edge_style & 7 < 4
                    && !waterfront(east + 54.0, north + 54.0) {
                    continue;
                }
                pulse();
                if !valid[next]
                    || !street_segment(world, local, [x, z], [dc as f32, dr as f32], span, steps)
                    || !reservation.clear_road([x, z], [dc as f32, dr as f32], span)
                {
                    continue;
                }
                area.connect(node, next, flag, reverse);
                links += 1;
                if state[next] & 16 == 0 {
                    state[next] |= 16;
                    queue[tail] = next as u16;
                    tail += 1;
                }
            }
        }

        let mut diagonals = 0;
        let mut river_parks = 0;
        for index in 0..BLOCKS {
            pulse();
            let node = block_node(index);
            if area.frontages(node) == 0 {
                continue;
            }
            let [x, z] = area.node_point(node);
            let lot = Area {
                x: x + span * 0.5,
                z: z + span * 0.5,
                half_width: span * 0.5 - STREET_HALF_WIDTH,
                half_depth: span * 0.5 - STREET_HALF_WIDTH,
            };
            let frontage_count = area.frontages(node).count_ones();
            let shore = waterfront(lot.x, lot.z);
            let east = lot.x - 54.0;
            let north = lot.z - 54.0;
            let fringe = east * east + north * north > 180.0 * 180.0;
            let profile = block_key(key ^ 0xb132_f45b, index);
            // Favor narrow frontages at the ends of road branches and keep
            // small connected pockets along riverbanks and the coast.
            let ribbon = fringe && frontage_count <= 2 && (shore || profile & 3 != 0);
            let use_kind = if shore && profile & 3 == 0 { Use::Shops } else { area.use_at(index) };
            let clear = |shape: Area| {
                [-1.0, 1.0].into_iter().all(|a| {
                    [-1.0, 1.0].into_iter().all(|b| {
                        owns(
                            shape.x + a * shape.half_width,
                            shape.z + b * shape.half_depth,
                        )
                    })
                }) && !highway_overlaps(road, local, shape)
                    && reservation.clear_lot(shape)
                    && !area.feeders.iter().flatten().any(|feeder| {
                        let start = area.node_point(usize::from(feeder.node));
                        let end = feeder.endpoint();
                        let padded = Area {
                            half_width: shape.half_width + 1.3,
                            half_depth: shape.half_depth + 1.3,
                            ..shape
                        };
                        (1..8).any(|i| {
                            let t = i as f32 * 0.125;
                            padded.contains(
                                start[0] + t * (end[0] - start[0]),
                                start[1] + t * (end[1] - start[1]),
                            )
                        })
                    })
                    && !overlaps_existing(town, shape)
            };
            if river_parks < 4 && river_near(lot.x, lot.z) && profile & 3 != 0 {
                let frontages = area.frontages(node);
                let park = Use::RIVER_PARKS.into_iter().enumerate().find_map(|(side, plan)| {
                    if frontages & (1 << side) == 0 {
                        return None;
                    }
                    let frontage = plan.river_frontage()?;
                    let park = area.river_park(index, frontage);
                    let strip = park.footprint();
                    if !park.by_bank(river_near) || !clear(strip) {
                        return None;
                    }
                    foundation(world, local, strip, 3.7).map(|base| (base, plan))
                });
                if let Some((base, plan)) = park {
                    area.pads[index] = (base + 1.999) as u8 | (FRONT_STRIP << 6);
                    area.pad_uses[index] = plan;
                    area.include(node);
                    area.include(node + SIDE + 1);
                    river_parks += 1;
                    continue;
                }
            }
            let saved = if !ribbon && clear(lot) {
                broad_pad(world, local, lot).map(|base| {
                    let streets_fit = matches!(use_kind, Use::Homes | Use::Terrace | Use::Shops)
                        && (1..stride).all(|i| {
                            let delta = f32::from(i) * SPACING;
                            [([x + delta, z], [0.0, 1.0]), ([x, z + delta], [1.0, 0.0])]
                                .into_iter()
                                .all(|(start, dir)| {
                                    pulse();
                                    street_segment(world, local, start, dir, span, steps)
                                })
                        });
                    (base, u8::from(streets_fit))
                })
            } else {
                None
            };
            let saved = saved.or_else(|| {
                [FRONT_STRIP, ALTERNATE_STRIP].into_iter().find_map(|mode| {
                    let strip = area.front_strip(index, mode);
                    if !clear(strip) {
                        return None;
                    }
                    foundation(world, local, strip, 3.95).map(|base| (base, mode))
                })
            });
            let Some((base, mode)) = saved else {
                if diagonals < DIAGONALS && fringe && (shore || profile & 3 == 0) {
                    let first_rising = profile & 4 == 0;
                    let diagonal = [first_rising, !first_rising].into_iter().find_map(|rising| {
                        let diagonal = DiagonalBlock { block: index as u8, rising, foundations: [0; 2] };
                        let corners = if rising { [node, node + SIDE + 1] } else { [node + 1, node + SIDE] };
                        if corners.into_iter().any(|corner| area.directions(corner) == 0) {
                            return None;
                        }
                        let foundations = [0, 1].map(|side| {
                            pulse();
                            let lot = diagonal.lot(&area, side);
                            let base = clear(lot).then(|| foundation(world, local, lot, 2.45)).flatten();
                            base.map_or(0, |height| (height + 1.999) as u8)
                        });
                        if foundations == [0, 0] {
                            return None;
                        }
                        let [start, end] = diagonal.endpoints(&area);
                        let length = span * core::f32::consts::SQRT_2;
                        let direction = [(end[0] - start[0]) / length, (end[1] - start[1]) / length];
                        for fraction in [0.25, 0.5, 0.75] {
                            if !owns(start[0] + (end[0] - start[0]) * fraction,
                                     start[1] + (end[1] - start[1]) * fraction)
                            {
                                return None;
                            }
                        }
                        if !street_segment(world, local, start, direction, length, (length / 2.0) as usize + 1)
                            || !reservation.clear_road(start, direction, length)
                        {
                            return None;
                        }
                        Some(DiagonalBlock { foundations, ..diagonal })
                    });
                    if let Some(diagonal) = diagonal {
                        area.diagonals[diagonals] = Some(diagonal);
                        area.diagonal_rows[index / SIDE_BLOCKS] |= 1 << (index % SIDE_BLOCKS);
                        diagonals += 1;
                    }
                }
                continue;
            };
            area.pads[index] = (base + 1.999) as u8 | (mode << 6);
            area.pad_uses[index] = use_kind;
            area.include(node);
            area.include(node + SIDE + 1);
        }
        area.index_feeders();
        area.paint_bounds = area.extent();
        area
    }

    fn index_feeders(&mut self) {
        let span = self.span();
        let origin = self.node_point(0);
        // The sampler rounds to the nearest collector node on each axis.
        // Include any cell the full street-width segment can enter.
        let padding = span * 0.5 + STREET_HALF_WIDTH + 0.01;
        for (index, feeder) in self.feeders.iter().enumerate() {
            let Some(feeder) = feeder else { continue };
            let start = self.node_point(usize::from(feeder.node));
            let end = feeder.endpoint();
            for axis in 0..2 {
                let lower = start[axis].min(end[axis]) - padding;
                let upper = start[axis].max(end[axis]) + padding;
                for (cell, flags) in self.feeder_cells[axis].iter_mut().enumerate() {
                    if (lower..=upper).contains(&(origin[axis] + cell as f32 * span)) {
                        *flags |= 1 << index;
                    }
                }
            }
        }
    }

    fn connect(&mut self, node: usize, next: usize, flag: u8, reverse: u8) {
        self.links[node / 2] |= flag << ((node & 1) * 4);
        self.links[next / 2] |= reverse << ((next & 1) * 4);
        self.include(node);
        self.include(next);
    }

    fn directions(&self, node: usize) -> u8 {
        (self.links[node / 2] >> ((node & 1) * 4)) & 15
    }

    fn frontages(&self, node: usize) -> u8 {
        u8::from(self.directions(node) & EAST != 0)
            | (u8::from(self.directions(node + SIDE) & EAST != 0) << 1)
            | (u8::from(self.directions(node) & FORWARD != 0) << 2)
            | (u8::from(self.directions(node + 1) & FORWARD != 0) << 3)
    }

    fn river_park(&self, index: usize, frontage: Frontage) -> RiverPark {
        let [x, z] = self.node_point(block_node(index));
        let half_span = self.span() * 0.5;
        RiverPark { frontage, center: [x + half_span, z + half_span], half_span }
    }

    fn front_strip(&self, index: usize, mode: u8) -> Area {
        let node = block_node(index);
        let mask = self.frontages(node);
        let key = block_key(self.key, index);
        let first = choose_frontage(mask, key);
        let first_bit = match first {
            Frontage::Near => 1,
            Frontage::Far => 2,
            Frontage::Left => 4,
            Frontage::Right => 8,
        };
        let rest = mask & !first_bit;
        let side = if mode == ALTERNATE_STRIP && rest != 0 {
            choose_frontage(rest, key.rotate_left(9))
        } else {
            first
        };
        let [x, z] = self.node_point(node);
        side.lot(
            x + self.span() * 0.5,
            z + self.span() * 0.5,
            0.0,
            self.span() * 0.5 - SPACING * 0.5,
            self.span() * 0.5 - STREET_HALF_WIDTH,
            SPACING * 0.5 - 1.72,
        )
    }

    fn include(&mut self, node: usize) {
        let col = (node % SIDE) as u8;
        let row = (node / SIDE) as u8;
        self.bounds[0] = self.bounds[0].min(col);
        self.bounds[1] = self.bounds[1].min(row);
        self.bounds[2] = self.bounds[2].max(col);
        self.bounds[3] = self.bounds[3].max(row);
    }

    pub(super) fn extent(&self) -> Option<[f32; 4]> {
        let [l, n, r, f] = self.bounds;
        (l != u8::MAX).then(|| {
            [
                (f32::from(l) + LEFT as f32) * self.span() + self.origin_x(),
                (f32::from(n) + NEAR as f32) * self.span() + f32::from(self.offset[1]),
                (f32::from(r) + LEFT as f32) * self.span() + self.origin_x(),
                (f32::from(f) + NEAR as f32) * self.span() + f32::from(self.offset[1]),
            ]
        })
    }

    pub(super) fn footprints(&self, mut emit: impl FnMut([f32; 4])) {
        let span = self.span();
        for (index, &pad) in self.pads.iter().enumerate() {
            if pad != 0 {
                if let Some(frontage) = self.pad_uses[index].river_frontage() {
                    let strip = self.river_park(index, frontage).footprint();
                    emit([strip.x - strip.half_width, strip.z - strip.half_depth,
                          strip.x + strip.half_width, strip.z + strip.half_depth]);
                } else {
                    let [x, z] = self.node_point(block_node(index));
                    emit([x, z, x + span, z + span]);
                }
            }
        }
        let w = STREET_HALF_WIDTH;
        for node in 0..NODES {
            let flags = self.directions(node);
            if flags != 0 {
                let [x, z] = self.node_point(node);
                let far_x = if flags & EAST != 0 { x + span } else { x };
                let far_z = if flags & FORWARD != 0 { z + span } else { z };
                if flags & EAST != 0 {
                    emit([x - w, z - w, far_x + w, z + w]);
                }
                if flags & FORWARD != 0 {
                    emit([x - w, z - w, x + w, far_z + w]);
                }
                if flags & (EAST | FORWARD) == 0 {
                    emit([x - w, z - w, x + w, z + w]);
                }
            }
        }
        for diagonal in self.diagonals.iter().flatten() {
            let [start, end] = diagonal.endpoints(self);
            let point = |i| core::array::from_fn::<_, 2, _>(|axis| {
                start[axis] + (end[axis] - start[axis]) * (i as f32 / 8.0)
            });
            for step in 0..8 {
                let ([x, z], [tx, tz]) = (point(step), point(step + 1));
                emit([x.min(tx) - w, z.min(tz) - w, x.max(tx) + w, z.max(tz) + w]);
            }
            for side in 0..2 {
                if diagonal.foundations[side] != 0 {
                    let lot = diagonal.lot(self, side);
                    emit([lot.x - lot.half_width, lot.z - lot.half_depth,
                          lot.x + lot.half_width, lot.z + lot.half_depth]);
                }
            }
        }
        for feeder in self.feeders.iter().flatten() {
            let [x, z] = self.node_point(usize::from(feeder.node));
            let [tx, tz] = feeder.endpoint();
            emit([x.min(tx) - w, z.min(tz) - w, x.max(tx) + w, z.max(tz) + w]);
        }
    }

    pub(super) fn sample(&self, x: f32, z: f32) -> Option<SettlementSample> {
        self.sample_with_cache(x, z, None)
    }

    pub(super) fn sample_with_cache(
        &self,
        x: f32,
        z: f32,
        cache: Option<&BoroughPaintCache>,
    ) -> Option<SettlementSample> {
        let [l, n, r, f] = self.paint_bounds?;
        if x < l - STREET_HALF_WIDTH
            || x > r + STREET_HALF_WIDTH
            || z < n - STREET_HALF_WIDTH
            || z > f + STREET_HALF_WIDTH
        {
            return None;
        }
        let gx = (x - self.origin_x()) * self.inverse_span - LEFT as f32;
        let gz = (z - f32::from(self.offset[1])) * self.inverse_span - NEAR as f32;
        let col = (gx + 0.5) as usize;
        let row = (gz + 0.5) as usize;
        let mut nearby = self.feeder_cells[0][col] & self.feeder_cells[1][row];
        while nearby != 0 {
            let index = nearby.trailing_zeros() as usize;
            nearby &= nearby - 1;
            let feeder = self.feeders[index].unwrap();
            let start = self.node_point(usize::from(feeder.node));
            let end = feeder.endpoint();
            let [px, pz] = [x - start[0], z - start[1]];
            if px.abs() > 36.0 || pz.abs() > 36.0 {
                continue;
            }
            let [dx, dz] = [end[0] - start[0], end[1] - start[1]];
            let t = ((dx * px + dz * pz) / (dx * dx + dz * dz)).clamp(0.0, 1.0);
            let [rx, rz] = [px - t * dx, pz - t * dz];
            if rx * rx + rz * rz <= STREET_HALF_WIDTH * STREET_HALF_WIDTH {
                return Some(open(SettlementSurface::Street));
            }
        }
        let dx = (gx - col as f32).abs() * self.span();
        let dz = (gz - row as f32).abs() * self.span();
        let joined = self.directions(row * SIDE + col) != 0;
        if joined && dx * dx + dz * dz <= STREET_HALF_WIDTH * STREET_HALF_WIDTH
            || dx <= STREET_HALF_WIDTH
                && (0.0..SIDE_BLOCKS as f32).contains(&gz)
                && self.directions(gz as usize * SIDE + col) & FORWARD != 0
            || dz <= STREET_HALF_WIDTH
                && (0.0..SIDE_BLOCKS as f32).contains(&gx)
                && self.directions(row * SIDE + gx as usize) & EAST != 0
        {
            return Some(open(SettlementSurface::Street));
        }
        if !(0.0..SIDE_BLOCKS as f32).contains(&gx) || !(0.0..SIDE_BLOCKS as f32).contains(&gz) {
            return None;
        }
        let index = gz as usize * SIDE_BLOCKS + gx as usize;
        let pad = self.pads[index];
        if pad == 0 {
            if self.diagonal_rows[gz as usize] & (1 << gx as usize) == 0 {
                return None;
            }
            let diagonal = *self.diagonals.iter().flatten().find(|diagonal| usize::from(diagonal.block) == index)?;
            let [ax, az] = self.node_point(block_node(index));
            let [u, v] = [x - ax, z - az];
            let across = if diagonal.rising { u - v } else { u + v - self.span() };
            if across.abs() <= STREET_HALF_WIDTH * core::f32::consts::SQRT_2 {
                return Some(open(SettlementSurface::Street));
            }
            return (0..2).filter_map(|side| diagonal.building(self, side))
                .find_map(|building| building.sample(x, z));
        }
        let plan = self.pad_uses[index];
        if let Some(frontage) = plan.river_frontage() {
            return self.river_park(index, frontage).sample(x, z, pad_base(pad), block_key(self.key, index));
        }
        let [ax, az] = self.node_point(block_node(index));
        let [u, v] = [x - ax, z - az];
        let cell_x = (u * (1.0 / SPACING)) as usize;
        let cell_z = (v * (1.0 / SPACING)) as usize;
        let mode = pad >> 6;
        if mode >= FRONT_STRIP && !self.front_strip(index, mode).contains(x, z) {
            return None;
        }
        if mode == ALLEYS
            && [u, v]
                .into_iter()
                .any(|pos| (pos - SPACING).abs().min((pos - 2.0 * SPACING).abs()) <= 1.55)
        {
            return Some(open(SettlementSurface::Street));
        }
        let use_kind = if mode >= FRONT_STRIP && matches!(plan, Use::Warehouse | Use::Works) {
            Use::Shops
        } else {
            plan
        };
        let base = pad_base(pad);
        if matches!(use_kind, Use::Warehouse | Use::Works) {
            return self.industrial_lot(index, use_kind, base, [ax, az], [x, z]);
        }
        let prepare = || self.residential_lot(index, use_kind, base, [ax, az], [cell_x, cell_z]);
        let lot = match cache {
            Some(cache) => cache.residential(self, [index, cell_x, cell_z], prepare),
            None => prepare(),
        };
        lot.sample(x, z)
    }

    fn use_at(&self, index: usize) -> Use {
        let [x, z] = self.node_point(block_node(index));
        let [c, r] = [index % SIDE_BLOCKS, index / SIDE_BLOCKS];
        let style = block_key(self.key ^ 0x13fa_8751, (c / 2) + (r / 2) * SIDE);
        let distance = (x - 54.0).abs().max((z - 54.0).abs());
        if distance > 135.0 && style & 7 <= 1 {
            return if style & 32 == 0 {
                Use::Warehouse
            } else {
                Use::Works
            };
        }
        if style & 15 >= 13 {
            return Use::Park;
        }
        if distance < 180.0 && style & 3 == 2 {
            return Use::Terrace;
        }
        if style & 7 == 4 {
            Use::Shops
        } else {
            Use::Homes
        }
    }

    fn industrial_lot(
        &self,
        index: usize,
        use_kind: Use,
        base: f32,
        [ax, az]: [f32; 2],
        [x, z]: [f32; 2],
    ) -> Option<SettlementSample> {
        let part = block_key(self.key, index);
        let frontage = choose_frontage(self.frontages(block_node(index)), part);
        let [cx, cz] = [ax + self.span() * 0.5, az + self.span() * 0.5];
        let factor = self.span() * (1.0 / 54.0);
        let width = (14.0 + (part & 7) as f32 * 0.65) * factor;
        let height = 6.4 + (part >> 3 & 7) as f32 * 0.48;
        let shed = frontage.lot(cx, cz, 0.0, -5.0 * factor, width, 11.3 * factor);
        if shed.contains(x, z) {
            let design = (part >> 8) as usize;
            let style = if use_kind == Use::Warehouse {
                const WAREHOUSES: [u8; 10] = [0, 1, 2, 3, 5, 8, 11, 12, 14, 16];
                WAREHOUSES[design % WAREHOUSES.len()]
            } else {
                const WORKS: [u8; 7] = [4, 6, 7, 9, 10, 13, 15];
                WORKS[design % WORKS.len()]
            };
            return Building::new(shed, base, height, Form::Industrial(frontage, style))
            .sample(x, z);
        }
        let yard = frontage.lot(cx, cz, 0.0, 13.0 * factor, 20.0 * factor, 6.4 * factor);
        if yard.contains(x, z) {
            return Some(open(SettlementSurface::Street));
        }
        if use_kind == Use::Works {
            let outbuilding = frontage.lot(
                cx,
                cz,
                20.3 * factor,
                -6.0 * factor,
                3.2 * factor,
                9.1 * factor,
            );
            if outbuilding.contains(x, z) {
                return Building::new(outbuilding, base, height - 2.0, Form::Hall).sample(x, z);
            }
        }
        None
    }

    fn residential_lot(
        &self,
        index: usize,
        use_kind: Use,
        base: f32,
        [ax, az]: [f32; 2],
        [col, row]: [usize; 2],
    ) -> ResidentialLot {
        let key = block_key(self.key ^ 0x67c0_3465, index * 9 + row * 3 + col);
        let x0 = ax + (col as f32 + 0.5) * SPACING;
        let z0 = az + (row as f32 + 0.5) * SPACING;
        let yard = Area {
            x: x0,
            z: z0,
            half_width: 6.8,
            half_depth: 6.8,
        };
        if use_kind == Use::Park || key & 15 == 0 {
            return ResidentialLot { parcel: Parcel::EMPTY, yard };
        }
        let east_west = key & 32 == 0;
        let frontage = if east_west {
            Frontage::Near
        } else {
            Frontage::Left
        };
        let shift = ((key >> 9 & 7) as f32 - 3.5) * 0.31;
        let mut parcel = Parcel::EMPTY;
        if use_kind == Use::Terrace {
            let lot = frontage.lot(x0, z0, shift, 0.0, 5.8, 4.1);
            parcel.add(Building::new(
                lot,
                base,
                5.3 + (key >> 15 & 7) as f32 * 0.52,
                Form::Rowhouse(frontage, (key >> 20) as u8),
            ));
        } else if use_kind == Use::Shops || key & 7 == 2 {
            let lot = frontage.lot(x0, z0, shift, 0.9, 4.9, 3.9);
            parcel.add(Building::new(
                lot,
                base,
                4.7 + (key >> 14 & 7) as f32 * 0.41,
                Form::Shop(frontage, (key >> 19) as u8),
            ));
        } else if key & 3 == 0 {
            let lot = frontage.lot(x0, z0, shift, 0.8, 5.1, 3.75);
            parcel.add(Building::new(
                lot,
                base,
                3.75 + (key >> 23 & 7) as f32 * 0.42,
                Form::Duplex((key >> 14) as u8),
            ));
        } else {
            for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
                let width = 2.1 + (key >> (i * 4 + 10) & 7) as f32 * 0.14;
                let depth = 2.7 + (key >> (i * 3 + 19) & 7) as f32 * 0.24;
                let lot = frontage.lot(x0, z0, side * 3.48, shift * side, width, depth);
                parcel.add(Building::new(
                    lot,
                    base,
                    3.2 + (key >> (i * 3 + 24) & 7) as f32 * 0.39,
                    Form::Cottage((key >> (16 + i * 5)) as u8),
                ));
            }
        }
        ResidentialLot { parcel, yard }
    }

    pub(super) fn obstruction(&self, x: f32, z: f32) -> f32 {
        self.sample(x, z)
            .and_then(|spot| spot.elevation)
            .map_or(0.0, |height| height + 4.0)
    }

    pub(super) fn roof_groups(
        &self,
        local: Local,
    ) -> impl Iterator<Item = crate::skyline::SkylineTower> + '_ {
        const GROUPS: usize = SIDE_BLOCKS / 2;
        (0..GROUPS * GROUPS).filter_map(move |group| {
            let row = group / GROUPS * 2;
            let col = group % GROUPS * 2;
            let first = row * SIDE_BLOCKS + col;
            let index = [
                first,
                first + 1,
                first + SIDE_BLOCKS,
                first + SIDE_BLOCKS + 1,
            ]
            .into_iter()
            .filter(|&i| self.pads[i] != 0 && self.pad_uses[i] != Use::Park && self.pad_uses[i].river_frontage().is_none())
            .min_by_key(|&i| self.pads[i] >> 6)?;
            let pad = self.pads[index];
            let mode = pad >> 6;
            let [x, z] = self.node_point(block_node(index));
            let lot = if mode >= FRONT_STRIP {
                self.front_strip(index, mode)
            } else {
                Area {
                    x: x + self.span() * 0.5,
                    z: z + self.span() * 0.5,
                    half_width: self.span() * (1.0 / 3.0),
                    half_depth: self.span() * 0.31,
                }
            };
            use crate::skyline::{
                BOROUGH_ALT_OFFSET, COTTAGE_GROUP, DUPLEX_GROUP, INDUSTRIAL_GROUP, ROWHOUSE_GROUP, SHOP_GROUP,
            };
            let use_kind = self.pad_uses[index];
            let (profile, height) = match use_kind {
                Use::Terrace => (ROWHOUSE_GROUP, 8.7),
                Use::Warehouse | Use::Works if mode < FRONT_STRIP => (INDUSTRIAL_GROUP, 8.8),
                Use::Warehouse | Use::Works | Use::Shops => (SHOP_GROUP, 7.2),
                Use::Homes if group & 3 == 0 => (DUPLEX_GROUP, 6.0),
                _ => (COTTAGE_GROUP, 5.15),
            };
            let profile = profile + if block_key(self.key, index) >> 25 & 1 == 0 {
                BOROUGH_ALT_OFFSET
            } else {
                0
            };
            let (x, z) = local.world(lot.x, lot.z);
            Some(crate::skyline::SkylineTower {
                center: [x, z],
                footprint: [lot.half_width, lot.half_depth],
                direction: [local.fx, local.fz],
                base: pad_base(pad),
                height,
                profile,
            })
        }).chain(self.diagonals.iter().flatten().flat_map(move |diagonal| {
            (0..2).filter_map(move |side| {
                let building = diagonal.building(self, side)?;
                let (x, z) = local.world(building.area.x, building.area.z);
                let profile = if matches!(building.form, Form::Shop(_, _)) {
                    crate::skyline::SHOP_GROUP
                } else {
                    crate::skyline::COTTAGE_GROUP
                };
                Some(crate::skyline::SkylineTower {
                    center: [x, z],
                    footprint: [building.area.half_width, building.area.half_depth],
                    direction: [local.fx, local.fz],
                    base: building.base,
                    height: building.height,
                    profile,
                })
            })
        }))
    }
}

fn open(surface: SettlementSurface) -> SettlementSample {
    SettlementSample {
        surface,
        elevation: None,
    }
}

fn block_node(index: usize) -> usize {
    index / SIDE_BLOCKS * SIDE + index % SIDE_BLOCKS
}

fn nearest_contact(
    points: &[RoadPoint],
    local: Local,
    [x, z]: [f32; 2],
) -> Option<([f32; 2], f32)> {
    let (wx, wz) = local.world(x, z);
    let mut nearest = 34.0 * 34.0;
    let mut point = None;
    for pair in points.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        let dx = b.x - a.x;
        let dz = b.z - a.z;
        let squared = dx * dx + dz * dz;
        if squared < 0.05 {
            continue;
        }
        let t = (((wx - a.x) * dx + (wz - a.z) * dz) / squared).clamp(0.0, 1.0);
        let [px, pz] = [a.x + dx * t, a.z + dz * t];
        let [ox, oz] = [wx - px, wz - pz];
        let distance = ox * ox + oz * oz;
        if distance < nearest {
            nearest = distance;
            point = Some(local.grid(px, pz));
        }
    }
    let (x0, z0) = point?;
    let actual = horizontal_length(x0 - x, z0 - z);
    (actual >= 2.4).then_some(([x0, z0], actual))
}

fn pad_base(value: u8) -> f32 {
    f32::from((value & BASE_MASK) - 1)
}

fn overlaps_existing(town: &Settlement, area: Area) -> bool {
    // Use actual older lots, including the dry gaps around the streets.
    let checks = [-0.90, -0.68, -0.45, -0.23, 0.0, 0.23, 0.45, 0.68, 0.90];
    checks.into_iter().any(|dz| {
        checks.into_iter().any(|dx| {
            town.sample_existing(area.x + dx * area.half_width, area.z + dz * area.half_depth)
                .is_some_and(|sample| sample.surface != SettlementSurface::Street)
        })
    })
}

fn broad_pad(world: &World, local: Local, area: Area) -> Option<f32> {
    let best = foundation(world, local, area, 5.0)?;
    let mut peak = best;
    for row in 0..3 {
        for col in 0..3 {
            let x = area.x + (col as f32 - 1.0) * SPACING;
            let z = area.z + (row as f32 - 1.0) * SPACING;
            let h = local.ground(world, x, z);
            if h < DRY_HEIGHT || (h - best).abs() > 5.0 {
                return None;
            }
            peak = peak.max(h);
        }
    }
    Some(peak)
}

fn street_segment(
    world: &World,
    local: Local,
    [x, z]: [f32; 2],
    [dx, dz]: [f32; 2],
    length: f32,
    steps: usize,
) -> bool {
    let step_length = length / steps as f32;
    let grade = STREET_GRADE * step_length;
    let mut last = [0.0; 3];
    for step in 0..=steps {
        let d = step as f32 * step_length;
        for (i, s) in [-STREET_HALF_WIDTH, 0.0, STREET_HALF_WIDTH]
            .into_iter()
            .enumerate()
        {
            let height = local.ground(world, x + dx * d - dz * s, z + dz * d + dx * s);
            if !(DRY_HEIGHT..=SEA_LEVEL + 51.0).contains(&height)
                || step > 0 && (height - last[i]).abs() > grade
            {
                return false;
            }
            last[i] = height;
        }
        if (last[0] - last[2]).abs() > 0.85 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_streets_link_two_dry_junctions_and_leave_room_for_corner_roofs() {
        let mut links = 0;
        let mut roofs = 0;
        for (seed, rx, rz) in [(9, -6, -7), (49, 0, 3), (165, -7, -1)] {
            let world = World::new(seed);
            let Some(road) = RoadNetwork::generate(&world, rx, rz) else { continue; };
            let towns = [Terminus::Start, Terminus::End]
                .map(|end| Settlement::generate(&world, &road, end));
            for (index, end) in [Terminus::Start, Terminus::End].into_iter().enumerate() {
                let Some(town) = towns[index].as_ref() else { continue; };
                let expansion = TownExpansion::survey(
                    &world, &road, end, town, towns[1 - index].as_ref(),
                    &|_, _| false, &|_| false, &|_, _| false, &mut || {},
                );
                let borough = &expansion.boroughs;
                for diagonal in borough.diagonals.iter().flatten() {
                    let [start, end] = diagonal.endpoints(borough);
                    for corner in [start, end] {
                        assert_eq!(borough.sample(corner[0], corner[1]), Some(open(SettlementSurface::Street)));
                    }
                    for step in 1..16 {
                        let t = step as f32 / 16.0;
                        let [x, z] = core::array::from_fn(|axis| start[axis] + (end[axis] - start[axis]) * t);
                        assert!(town.local.ground(&world, x, z) >= DRY_HEIGHT);
                        assert_eq!(borough.sample(x, z), Some(open(SettlementSurface::Street)));
                    }
                    for side in 0..2 {
                        let Some(building) = diagonal.building(borough, side) else { continue; };
                        let area = building.area;
                        assert!(foundation(&world, town.local, area, 2.45).is_some());
                        assert!(!highway_overlaps(&road, town.local, area));
                        assert!(area.interior_samples().any(|[x, z]| {
                            borough.sample(x, z).and_then(|sample| sample.elevation).is_some()
                        }));
                        roofs += 1;
                    }
                    links += 1;
                }
            }
        }
        assert!(links >= 2 && roofs >= links, "{links} links and {roofs} roofs");
    }
}
