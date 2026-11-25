//! Prepared coastal layouts tied to real road termini and the seeded shoreline.

use core::f32::consts::TAU;

use crate::beach::{BeachSite, BEACH_TILE_SIZE};
use crate::ground_details::ParcelSurvey;
use crate::math::{fraction, mix, sin_cos};
use crate::world::{Sample, Surface, World, SEA_LEVEL};

const LAND_LIMIT: usize = 14;
const VESSEL_LIMIT: usize = 3;
const ACCESS_HALF_WIDTH: f32 = 2.1;
const CARGO_COLORS: [CargoColor; 4] = [
    CargoColor::Ochre, CargoColor::Blue, CargoColor::Rust, CargoColor::Pale,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortKind {
    Marina,
    Ferry,
    Freight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CargoColor {
    Ochre,
    Blue,
    Rust,
    Pale,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortSurface {
    AccessRoad,
    Quay,
    Apron,
    Terminal,
    Warehouse,
    Crane,
    Cargo(CargoColor),
    BoatHull,
    BoatDeck,
    BoatCabin,
    BoatSail,
    BoatCargo,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortSample {
    pub surface: PortSurface,
    pub elevation: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoatKind {
    Launch,
    Sailboat,
    Ferry,
    Freighter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MooredVessel {
    pub kind: BoatKind,
    pub x: f32,
    pub z: f32,
    pub length: f32,
    pub beam: f32,
    forward: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    left: f32,
    right: f32,
    rear: f32,
    front: f32,
}

impl Rect {
    const EMPTY: Self = Self {
        left: 0.0,
        right: 0.0,
        rear: 0.0,
        front: 0.0,
    };

    fn around(x: f32, z: f32, width: f32, depth: f32) -> Self {
        Self {
            left: x - width / 2.0,
            right: x + width / 2.0,
            rear: z - depth / 2.0,
            front: z + depth / 2.0,
        }
    }

    fn contains(self, x: f32, z: f32) -> bool {
        x >= self.left && x <= self.right && z >= self.rear && z <= self.front
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pier {
    walkway: Rect,
    berth: Rect,
    deck: f32,
}

impl Pier {
    fn contains(self, along: f32, out: f32) -> bool {
        self.walkway.contains(along, out) || self.berth.contains(along, out)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LandPiece {
    rect: Rect,
    sample: PortSample,
}

impl LandPiece {
    const EMPTY: Self = Self {
        rect: Rect::EMPTY,
        sample: PortSample {
            surface: PortSurface::Apron,
            elevation: SEA_LEVEL,
        },
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Shore {
    origin: [f32; 2],
    seaward: [f32; 2],
    road_distance: f32,
}

impl Shore {
    fn world(self, along: f32, out: f32) -> [f32; 2] {
        [
            self.origin[0] - self.seaward[1] * along + self.seaward[0] * out,
            self.origin[1] + self.seaward[0] * along + self.seaward[1] * out,
        ]
    }

    fn local(self, x: f32, z: f32) -> [f32; 2] {
        let dx = x - self.origin[0];
        let dz = z - self.origin[1];
        [
            dz * self.seaward[0] - dx * self.seaward[1],
            dx * self.seaward[0] + dz * self.seaward[1],
        ]
    }

    fn height(self, world: &World, along: f32, out: f32) -> f32 {
        let [x, z] = self.world(along, out);
        world.elevation(x, z)
    }

    fn signal(self, world: &World, along: f32, out: f32) -> f32 {
        let [x, z] = self.world(along, out);
        world.coast_signal(x, z)
    }

    fn find(world: &World, endpoint: [f32; 2], angle: f32) -> Option<Self> {
        let (sx, sz) = sin_cos(angle);
        let point = |distance: f32| [endpoint[0] + sx * distance, endpoint[1] + sz * distance];
        let wet = |distance| {
            let [x, z] = point(distance);
            world.coast_signal(x, z) <= 0.0
        };
        let mut dry = 0.0;
        for step in 1..=14 {
            let mut sea = step as f32 * 9.0;
            if wet(sea) {
                for _ in 0..7 {
                    let middle = (dry + sea) * 0.5;
                    if wet(middle) {
                        sea = middle;
                    } else {
                        dry = middle;
                    }
                }
                let road_distance = (dry + sea) * 0.5;
                if road_distance < 14.0 {
                    return None;
                }
                return Some(Self {
                    origin: point(road_distance),
                    seaward: [sx, sz],
                    road_distance,
                });
            }
            dry = sea;
        }
        None
    }
}

/// An immutable plan. World surveys run only at preparation; drawing uses local bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Port {
    kind: PortKind,
    shore: Shore,
    bounds: [f32; 4],
    apron: Rect,
    landing: Rect,
    pier: Option<Pier>,
    buildings: [LandPiece; LAND_LIMIT],
    buildings_count: usize,
    vessels: [MooredVessel; VESSEL_LIMIT],
    vessel_count: usize,
}

impl Port {
    pub fn generate(
        world: &World,
        seed: u64,
        route_endpoint_x: f32,
        route_endpoint_z: f32,
    ) -> Option<Self> {
        Self::generate_cooperatively(world, seed, route_endpoint_x, route_endpoint_z, &mut || {})
    }

    pub(crate) fn generate_cooperatively(
        world: &World, seed: u64, route_endpoint_x: f32, route_endpoint_z: f32,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        if !route_endpoint_x.is_finite() || !route_endpoint_z.is_finite() {
            return None;
        }
        let endpoint = [route_endpoint_x, route_endpoint_z];
        let height = world.elevation(route_endpoint_x, route_endpoint_z);
        if !(SEA_LEVEL + 0.7..=SEA_LEVEL + 15.0).contains(&height) {
            return None;
        }
        let cell = (u64::from(index(route_endpoint_x, 192.0) as u32) << 32)
            | u64::from(index(route_endpoint_z, 192.0) as u32);
        let key = mix(seed ^ cell ^ 0x2a07_8f41_617c_448e);
        // Leave most eligible coastline empty between developed places.
        if key & 7 >= 4 {
            return None;
        }
        let rotation = fraction(mix(key)) * TAU;
        let directions: [Option<Shore>; 16] = core::array::from_fn(|i| {
            pulse();
            Shore::find(world, endpoint, rotation + i as f32 * (TAU / 16.0))
        });
        let kinds = [PortKind::Marina, PortKind::Ferry, PortKind::Freight];
        let first_kind = ((key >> 7) % 3) as usize;
        for offset in 0..kinds.len() {
            let kind = kinds[(first_kind + offset) % kinds.len()];
            for shore in directions.iter().flatten().copied() {
                if !eligible_water(world, shore, kind) {
                    continue;
                }
                for frontage in [-3.0, -7.0, -12.0, -17.0] {
                    pulse();
                    let Some(mut plan) = Self::arrange(world, shore, kind, frontage, key) else {
                        continue;
                    };
                    if plan.land_use_clear(world, seed, pulse) {
                        if let Some(pier) = plan.fit_pier(world) {
                            plan.pier = Some(pier);
                            plan.bounds = plan.calculate_bounds();
                        }
                        return Some(plan);
                    }
                }
            }
        }
        None
    }

    /// Actual paved edge contacts where a dry quay extension can depart.
    pub(crate) fn quayside_contacts(&self) -> [([f32; 2], f32); 4] {
        let left = self.apron.left + 0.44;
        let right = self.apron.right - 0.44;
        let near = self.apron.front - 1.8;
        let far = (self.apron.rear + near) * 0.5;
        [
            (self.shore.world(left, near), -1.0),
            (self.shore.world(left, far), -1.0),
            (self.shore.world(right, near), 1.0),
            (self.shore.world(right, far), 1.0),
        ]
    }

    pub fn kind(&self) -> PortKind {
        self.kind
    }

    /// Outer bounds as minimum x, minimum z, maximum x, maximum z.
    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub fn shore_origin(&self) -> [f32; 2] {
        self.shore.origin
    }

    pub fn seaward(&self) -> [f32; 2] {
        self.shore.seaward
    }

    pub fn vessels(&self) -> &[MooredVessel] {
        &self.vessels[..self.vessel_count]
    }

    /// Sample against underlying terrain. Existing field and ship materials take precedence.
    pub fn sample(&self, x: f32, z: f32, ground: Sample) -> Option<PortSample> {
        let [min_x, min_z, max_x, max_z] = self.bounds;
        if !(x >= min_x && x <= max_x && z >= min_z && z <= max_z) {
            return None;
        }
        let [along, out] = self.shore.local(x, z);
        let pier = self.pier.filter(|pier| pier.contains(along, out)).map(|pier| PortSample {
            surface: PortSurface::Quay,
            elevation: pier.deck.max(ground.elevation),
        });
        if ground.elevation == SEA_LEVEL {
            return match ground.surface {
                Surface::Shallows | Surface::DeepWater => {
                    self.vessels().iter().find_map(|boat| boat.sample(x, z)).or(pier)
                }
                _ => None,
            };
        }
        if pier.is_some() {
            return pier;
        }
        if ground.elevation < SEA_LEVEL + 0.15
            || !matches!(
                ground.surface,
                Surface::Sand | Surface::Grass | Surface::Forest | Surface::Desert
            )
        {
            return None;
        }
        if let Some(piece) = self.buildings[..self.buildings_count]
            .iter()
            .find(|piece| piece.rect.contains(along, out))
        {
            return Some(piece.sample);
        }
        let surface = if self.landing.contains(along, out)
            || (self.apron.contains(along, out) && out >= self.apron.front - 3.0)
        {
            PortSurface::Quay
        } else if self.apron.contains(along, out) {
            PortSurface::Apron
        } else if self.access_contains(along, out) {
            PortSurface::AccessRoad
        } else {
            return None;
        };
        Some(PortSample {
            surface,
            elevation: ground.elevation,
        })
    }

    fn arrange(
        world: &World,
        shore: Shore,
        kind: PortKind,
        frontage: f32,
        key: u64,
    ) -> Option<Self> {
        let (half_width, depth, landing_width) = match kind {
            PortKind::Marina => (11.0, 9.0, 3.8),
            PortKind::Ferry => (16.0, 13.0, 5.0),
            PortKind::Freight => (25.0, 18.0, 9.0),
        };
        let apron = Rect {
            left: -half_width,
            right: half_width,
            rear: frontage - depth,
            front: frontage,
        };
        if shore.road_distance < -apron.rear + 1.0 {
            return None;
        }
        let (low, high) = ground_range(world, shore, apron)?;
        if low < SEA_LEVEL + 0.24 || high > SEA_LEVEL + 7.0 || high - low > 2.2 {
            return None;
        }
        let landing = Rect {
            left: -landing_width,
            right: landing_width,
            rear: frontage,
            front: -0.25,
        };
        let mut port = Self {
            kind,
            shore,
            apron,
            landing,
            pier: None,
            bounds: [0.0; 4],
            buildings: [LandPiece::EMPTY; LAND_LIMIT],
            buildings_count: 0,
            vessels: [MooredVessel::EMPTY; VESSEL_LIMIT],
            vessel_count: 0,
        };
        if !port.prove_access(world) || !port.build_yard(world, key) || !port.moor(world, key) {
            return None;
        }
        port.bounds = port.calculate_bounds();
        Some(port)
    }

    fn fit_pier(&self, world: &World) -> Option<Pier> {
        let moored = self.vessels().first()?;
        let [along, out] = self.shore.local(moored.x, moored.z);
        let (walkway_half, berth_depth, height) = match self.kind {
            PortKind::Marina => (1.65, 1.8, 0.84),
            PortKind::Ferry => (2.6, 2.8, 1.22),
            PortKind::Freight => (3.8, 4.0, 1.44),
        };
        let edge = out - moored.beam * 0.5 - 0.76;
        let berth = Rect {
            left: (along - moored.length * 0.43).min(-walkway_half),
            right: (along + moored.length * 0.43).max(walkway_half),
            rear: edge - berth_depth,
            front: edge,
        };
        if berth.rear < 1.4 {
            return None;
        }
        for side in [berth.left, berth.left * 0.5, 0.0, berth.right * 0.5, berth.right] {
            for depth in [berth.rear, berth.front] {
                if self.shore.height(world, side, depth) > SEA_LEVEL {
                    return None;
                }
            }
        }
        let walkway = Rect {
            left: -walkway_half,
            right: walkway_half,
            rear: self.landing.front - 2.5,
            front: berth.rear,
        };
        // Keep the access corridor and deck clear of every other mooring.
        for boat in self.vessels() {
            let [x, z] = self.shore.local(boat.x, boat.z);
            for deck in [walkway, berth] {
                if deck.left < x + boat.length * 0.5 && deck.right > x - boat.length * 0.5
                    && deck.rear < z + boat.beam * 0.5 && deck.front > z - boat.beam * 0.5
                {
                    return None;
                }
            }
        }
        Some(Pier { walkway, berth, deck: SEA_LEVEL + height })
    }

    fn access_contains(&self, x: f32, z: f32) -> bool {
        x.abs() <= ACCESS_HALF_WIDTH
            && z >= -self.shore.road_distance - 0.35
            && z <= self.apron.rear
    }

    fn prove_access(&self, world: &World) -> bool {
        let start = -self.shore.road_distance;
        let length = self.apron.rear - start;
        let steps = (length / 3.0) as usize + 1;
        let mut last = self.shore.height(world, 0.0, start);
        for step in 0..=steps {
            let out = start + length * step as f32 / steps as f32;
            let height = self.shore.height(world, 0.0, out);
            if (last - height).abs() > length / steps as f32 * 0.15 + 0.04 {
                return false;
            }
            for x in [-ACCESS_HALF_WIDTH, 0.0, ACCESS_HALF_WIDTH] {
                let edge = self.shore.height(world, x, out);
                if edge < SEA_LEVEL + 0.25 || (edge - height).abs() > 0.68 {
                    return false;
                }
            }
            last = height;
        }
        // Keep the lane's connection to water, while letting its tip follow the real coast.
        for point in [0.0, -0.5, 0.5] {
            let along = point * self.landing.right;
            if self.shore.height(world, along, self.landing.front - 2.5) <= SEA_LEVEL + 0.025 {
                return false;
            }
        }
        true
    }

    fn build_yard(&mut self, world: &World, key: u64) -> bool {
        let frontage = self.apron.front;
        match self.kind {
            PortKind::Marina => self.add_piece(
                world,
                Rect::around(-4.8, frontage - 5.1, 5.5, 3.5),
                2.2,
                PortSurface::Terminal,
            ),
            PortKind::Ferry => self.add_piece(
                world,
                Rect::around(-7.6, frontage - 7.4, 11.0, 5.5),
                3.6,
                PortSurface::Terminal,
            ),
            PortKind::Freight => {
                if !self.add_piece(
                    world,
                    Rect::around(0.0, self.apron.rear + 4.0, 23.0, 6.5),
                    4.8,
                    PortSurface::Warehouse,
                ) {
                    return false;
                }
                self.stack_containers(world, key);
                for x in [-17.0, 15.0] {
                    self.add_piece(
                        world,
                        Rect::around(x, frontage - 1.8, 2.4, 2.3),
                        6.7,
                        PortSurface::Crane,
                    );
                }
                self.buildings[..self.buildings_count]
                    .iter()
                    .filter(|piece| matches!(piece.sample.surface, PortSurface::Cargo(_)))
                    .count()
                    >= 4
            }
        }
    }

    fn stack_containers(&mut self, world: &World, key: u64) {
        // Long bays and staggered stacks leave different access lanes on the apron.
        let slots: &[[f32; 4]] = match (key >> 36) % 3 {
            1 => &[
                [-17.0, 5.3, 8.5, 2.6], [-5.5, 5.3, 8.5, 2.6],
                [5.5, 5.3, 8.5, 2.6], [17.0, 5.3, 8.5, 2.6],
                [-17.0, 8.8, 8.5, 2.5], [-5.5, 8.8, 8.5, 2.5],
                [5.5, 8.8, 8.5, 2.5], [17.0, 8.8, 8.5, 2.5],
            ],
            2 => &[
                [-19.0, 5.1, 7.0, 3.0], [-10.0, 5.8, 7.0, 3.0],
                [3.5, 5.1, 7.0, 3.0], [12.5, 5.8, 7.0, 3.0],
                [21.0, 5.1, 5.0, 3.0], [-14.5, 9.0, 7.4, 2.4],
                [8.0, 9.0, 7.4, 2.4], [19.0, 9.0, 7.4, 2.4],
            ],
            _ => &[],
        };
        let frontage = self.apron.front;
        let start = self.buildings_count;
        if !slots.is_empty()
            && slots.iter().enumerate().all(|(i, &[x, setback, width, depth])| {
                let color = CARGO_COLORS[((key >> (12 + i * 2)) & 3) as usize];
                let height = 1.6 + ((key >> (23 + i)) & 1) as f32 * 0.9;
                self.add_piece(
                    world,
                    Rect::around(x, frontage - setback, width, depth),
                    height,
                    PortSurface::Cargo(color),
                )
            })
        {
            return;
        }
        // Try the original compact stacks when the larger surveyed pads do not fit.
        self.buildings[start..self.buildings_count].fill(LandPiece::EMPTY);
        self.buildings_count = start;
        for (i, x) in [-17.0, -8.5, 0.0, 8.5, 17.0].into_iter().enumerate() {
            let color = CARGO_COLORS[((key >> (12 + i * 2)) & 3) as usize];
            let height = 1.6 + ((key >> (23 + i)) & 1) as f32 * 0.9;
            self.add_piece(
                world,
                Rect::around(x, frontage - 5.8, 6.0, 2.6),
                height,
                PortSurface::Cargo(color),
            );
            if i & 1 == 0 {
                self.add_piece(
                    world,
                    Rect::around(x, frontage - 9.6, 5.5, 2.1),
                    1.55,
                    PortSurface::Cargo(CARGO_COLORS[(i + 1) % 4]),
                );
            }
        }
    }

    fn add_piece(&mut self, world: &World, rect: Rect, rise: f32, surface: PortSurface) -> bool {
        if self.buildings_count == LAND_LIMIT
            || !self.apron.contains(rect.left, rect.rear)
            || !self.apron.contains(rect.right, rect.front)
        {
            return false;
        }
        let Some((low, high)) = ground_range(world, self.shore, rect) else {
            return false;
        };
        if low < SEA_LEVEL + 0.30 || high - low > 1.1 {
            return false;
        }
        self.buildings[self.buildings_count] = LandPiece {
            rect,
            sample: PortSample {
                surface,
                elevation: high + rise,
            },
        };
        self.buildings_count += 1;
        true
    }

    fn moor(&mut self, world: &World, key: u64) -> bool {
        let mut add = |along, out, length, beam, kind| {
            let [x, z] = self.shore.world(along, out);
            let direction = if key & 0x400 != 0 { -1.0 } else { 1.0 };
            let vessel = MooredVessel {
                kind,
                x,
                z,
                length,
                beam,
                forward: [
                    -direction * self.shore.seaward[1],
                    direction * self.shore.seaward[0],
                ],
            };
            if vessel.has_water_clearance(world) {
                self.vessels[self.vessel_count] = vessel;
                self.vessel_count += 1;
            }
        };
        match self.kind {
            PortKind::Marina => {
                add(-7.0, 7.3, 8.0, 3.35, BoatKind::Sailboat);
                add(3.0, 9.0, 6.1, 2.85, BoatKind::Launch);
                add(11.5, 14.3, 8.5, 3.2, BoatKind::Sailboat);
            }
            PortKind::Ferry => {
                add(0.0, 12.0, 24.0, 6.1, BoatKind::Ferry);
                add(-9.5, 23.0, 7.2, 2.9, BoatKind::Launch);
            }
            PortKind::Freight => {
                add(0.0, 17.5, 37.0, 8.0, BoatKind::Freighter);
                add(-16.5, 30.0, 9.0, 3.7, BoatKind::Launch);
            }
        }
        self.vessels().iter().any(|vessel| match self.kind {
            PortKind::Marina => true,
            PortKind::Ferry => vessel.kind == BoatKind::Ferry,
            PortKind::Freight => vessel.kind == BoatKind::Freighter,
        })
    }

    fn land_use_clear(&self, world: &World, seed: u64, pulse: &mut impl FnMut()) -> bool {
        let [min_x, min_z, max_x, max_z] = self.bounds;
        let mut details = ParcelSurvey::new();
        // Search the road and apron at sub-parcel spacing. Every apron feature is on the apron.
        for along in self.apron.left as i32..=self.apron.right as i32 {
            pulse();
            for out in -(self.shore.road_distance as i32)..=self.landing.front as i32 {
                if !self.dry_footprint(along as f32, out as f32) {
                    continue;
                }
                let [x, z] = self.shore.world(along as f32, out as f32);
                let ground = world.sample(x, z);
                if ground.surface == Surface::Grass
                    && details.occupies_grass(world, seed, x, z)
                {
                    return false;
                }
            }
        }
        let tile = BEACH_TILE_SIZE as f32;
        for tz in index(min_z - 3.0, tile)..=index(max_z + 3.0, tile) {
            for tx in index(min_x - 3.0, tile)..=index(max_x + 3.0, tile) {
                pulse();
                if let Some(beach) = BeachSite::reserved_footprint(world, seed, tx, tz) {
                    if beach.conflicts_with(|x, z| {
                        let [along, out] = self.shore.local(x, z);
                        self.dry_footprint(along, out)
                    }) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn dry_footprint(&self, along: f32, out: f32) -> bool {
        self.access_contains(along, out)
            || self.apron.contains(along, out)
            || self.landing.contains(along, out)
    }

    fn calculate_bounds(&self) -> [f32; 4] {
        let mut bounds = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        let mut expand = |x: f32, z: f32| {
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].min(z);
            bounds[2] = bounds[2].max(x);
            bounds[3] = bounds[3].max(z);
        };
        for along in [self.apron.left, self.apron.right] {
            for out in [-self.shore.road_distance - 0.4, self.landing.front] {
                let [x, z] = self.shore.world(along, out);
                expand(x, z);
            }
        }
        if let Some(pier) = self.pier {
            for deck in [pier.walkway, pier.berth] {
                for along in [deck.left, deck.right] {
                    for out in [deck.rear, deck.front] {
                        let [x, z] = self.shore.world(along, out);
                        expand(x, z);
                    }
                }
            }
        }
        for vessel in self.vessels() {
            let radius = (vessel.length + vessel.beam) * 0.5;
            expand(vessel.x - radius, vessel.z - radius);
            expand(vessel.x + radius, vessel.z + radius);
        }
        bounds
    }
}

fn eligible_water(world: &World, shore: Shore, kind: PortKind) -> bool {
    let (half, inner, outer) = match kind {
        PortKind::Marina => (10.0, 7.0, 28.0),
        PortKind::Ferry => (14.0, 10.0, 54.0),
        PortKind::Freight => (21.0, 15.0, 84.0),
    };
    for x in [-half, 0.0, half] {
        for out in [inner, (inner + outer) * 0.5, outer] {
            if shore.signal(world, x, out) >= -0.004 {
                return false;
            }
        }
    }
    match kind {
        PortKind::Marina => {
            let sheltered = [-58.0, 58.0]
                .into_iter()
                .any(|x| shore.signal(world, x, 21.0) > 0.01);
            sheltered && shore.signal(world, 0.0, 15.0) >= -0.105
        }
        PortKind::Ferry => shore.signal(world, 0.0, 54.0) < -0.035,
        // The world supplies a deep-water class, rather than simulated soundings.
        PortKind::Freight => [-21.0, 0.0, 21.0]
            .into_iter()
            .all(|x| shore.signal(world, x, 84.0) < -0.105),
    }
}

fn ground_range(world: &World, shore: Shore, rect: Rect) -> Option<(f32, f32)> {
    let columns = ((rect.right - rect.left) / 3.0) as usize + 1;
    let rows = ((rect.front - rect.rear) / 3.0) as usize + 1;
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for row in 0..=rows {
        let out = rect.rear + (rect.front - rect.rear) * row as f32 / rows as f32;
        for col in 0..=columns {
            let along = rect.left + (rect.right - rect.left) * col as f32 / columns as f32;
            let height = shore.height(world, along, out);
            if height <= SEA_LEVEL + 0.14 {
                return None;
            }
            low = low.min(height);
            high = high.max(height);
        }
    }
    Some((low, high))
}

impl MooredVessel {
    const EMPTY: Self = Self {
        kind: BoatKind::Launch,
        x: 0.0,
        z: 0.0,
        length: 0.0,
        beam: 0.0,
        forward: [0.0, 1.0],
    };

    pub fn heading(&self) -> [f32; 2] {
        self.forward
    }

    fn has_water_clearance(self, world: &World) -> bool {
        let [fx, fz] = self.forward;
        let rows = (self.length / 2.0) as usize + 1;
        let columns = (self.beam / 2.0) as usize + 1;
        for row in 0..=rows {
            let along = (row as f32 / rows as f32 - 0.5) * (self.length + 2.0);
            for col in 0..=columns {
                let across = (col as f32 / columns as f32 - 0.5) * (self.beam + 2.0);
                let x = self.x + fx * along - fz * across;
                let z = self.z + fz * along + fx * across;
                if world.coast_signal(x, z) > -0.006 {
                    return false;
                }
            }
        }
        true
    }

    fn sample(self, x: f32, z: f32) -> Option<PortSample> {
        let dx = x - self.x;
        let dz = z - self.z;
        let radius = (self.length + self.beam) * 0.5;
        if dx.abs() > radius || dz.abs() > radius {
            return None;
        }
        let [fx, fz] = self.forward;
        let along = 2.0 * (dx * fx + dz * fz) / self.length;
        let across = 2.0 * (dz * fx - dx * fz).abs() / self.beam;
        let beam = 1.0 - 1.45 * (along.abs() - 0.44).max(0.0);
        if along.abs() >= 1.0 || across >= beam {
            return None;
        }
        let (surface, height) = if across >= beam - 0.16 {
            (
                PortSurface::BoatHull,
                if self.length > 10.0 { 1.2 } else { 0.48 },
            )
        } else {
            match self.kind {
                BoatKind::Sailboat if (-0.21..=0.24).contains(&along) && across < 0.28 => {
                    (PortSurface::BoatSail, 2.9 - along * 1.7)
                }
                BoatKind::Launch if (-0.48..=0.12).contains(&along) && across < 0.64 => {
                    (PortSurface::BoatCabin, 1.6)
                }
                BoatKind::Ferry if along.abs() < 0.58 && across < 0.67 => {
                    (PortSurface::BoatCabin, if along > 0.12 { 4.1 } else { 2.9 })
                }
                BoatKind::Freighter if (-0.72..=-0.42).contains(&along) && across < 0.65 => {
                    (PortSurface::BoatCabin, 5.6)
                }
                BoatKind::Freighter
                    if (-0.32..=0.55).contains(&along)
                        && across < 0.72
                        && (along + 0.4) * 8.0 % 1.0 < 0.82 =>
                {
                    (PortSurface::BoatCargo, 3.45)
                }
                _ => (
                    PortSurface::BoatDeck,
                    if self.length > 10.0 { 1.62 } else { 0.78 },
                ),
            }
        };
        Some(PortSample {
            surface,
            elevation: SEA_LEVEL + height,
        })
    }
}

fn index(value: f32, scale: f32) -> i32 {
    let scaled = value / scale;
    let whole = scaled as i32;
    whole.saturating_sub(i32::from(scaled < whole as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground_details::GroundDetails;
    use crate::roads::RoadNetwork;

    const FIXTURES: [(u64, [i32; 2], [f32; 2], PortKind); 3] = [
        (1, [-1, -3], [-368.0, -2584.0], PortKind::Marina),
        (1, [-1, -3], [-680.0, -2464.0], PortKind::Ferry),
        (82, [0, 2], [392.0, 2752.0], PortKind::Freight),
    ];

    #[test]
    fn seeded_ports_join_real_roads_and_reserve_dry_ground() {
        for (seed, [rx, rz], [x, z], kind) in FIXTURES {
            let world = World::new(seed);
            let route = RoadNetwork::generate(&world, rx, rz).unwrap();
            let points = route.polyline();
            assert!([points[0], points[points.len() - 1]]
                .iter()
                .any(|point| (point.x, point.z) == (x, z)));
            let port = Port::generate(&world, seed, x, z).unwrap();
            assert_eq!(port.kind(), kind);
            assert_eq!(Port::generate(&world, seed, x, z), Some(port));
            assert!(eligible_water(&world, port.shore, kind));
            assert!(port.land_use_clear(&world, seed, &mut || {}));

            let mut details = GroundDetails::new();
            details.reset(&world, seed, x, z);
            let [center_x, center_z] = port.shore_origin();
            details.follow(&world, center_x, center_z);
            let lane_length = port.shore.road_distance + port.apron.rear;
            for i in 0..=lane_length as usize * 4 {
                let out = i as f32 * 0.25 - port.shore.road_distance;
                let [x, z] = port.shore.world(0.0, out);
                assert!(matches!(
                    port.sample(x, z, world.sample(x, z)),
                    Some(PortSample {
                        surface: PortSurface::AccessRoad | PortSurface::Apron,
                        ..
                    })
                ));
            }

            let mut roof = 0;
            let mut cargo = 0;
            let mut quay = 0;
            for out in (2.0 * port.apron.rear) as i32..=0 {
                for along in (2.0 * port.apron.left) as i32..=(2.0 * port.apron.right) as i32 {
                    let [x, z] = port.shore.world(along as f32 * 0.5, out as f32 * 0.5);
                    let ground = world.sample(x, z);
                    let Some(sample) = port.sample(x, z, ground) else {
                        continue;
                    };
                    if ground.elevation == SEA_LEVEL {
                        assert_eq!(sample.surface, PortSurface::Quay);
                        assert!(port.pier.is_some_and(|pier| {
                            pier.contains(along as f32 * 0.5, out as f32 * 0.5)
                        }));
                        continue;
                    }
                    assert!(sample.elevation >= ground.elevation);
                    assert!(!matches!(
                        details.sample(x, z, ground).surface,
                        Surface::FieldGreen | Surface::FieldGold | Surface::FieldSoil
                    ));
                    match sample.surface {
                        PortSurface::Terminal | PortSurface::Warehouse => roof += 1,
                        PortSurface::Cargo(_) => cargo += 1,
                        PortSurface::Quay => quay += 1,
                        _ => (),
                    }
                }
            }
            assert!(roof > 30 && quay > 25, "{kind:?} {roof} {quay}");
            if kind == PortKind::Freight {
                assert!(cargo > 60);
                let pieces = &port.buildings[..port.buildings_count];
                let is_cargo = |piece: &&LandPiece| matches!(piece.sample.surface, PortSurface::Cargo(_));
                assert!(pieces.iter().filter(is_cargo).any(|p| p.rect.right - p.rect.left >= 7.0));
                for (i, first) in pieces.iter().enumerate().filter(|(_, piece)| is_cargo(piece)) {
                    assert!(pieces[i + 1..].iter().filter(is_cargo).all(|second| {
                        first.rect.right <= second.rect.left || second.rect.right <= first.rect.left
                            || first.rect.front <= second.rect.rear || second.rect.front <= first.rect.rear
                    }));
                }
                assert!(
                    port.buildings[..port.buildings_count]
                        .iter()
                        .filter(|piece| piece.sample.surface == PortSurface::Crane)
                        .count()
                        >= 1
                );
            }
        }
    }

    #[test]
    fn freight_layouts_leave_distinct_lanes_on_the_same_surveyed_apron() {
        let world = World::new(82);
        let port = Port::generate(&world, 82, 392.0, 2752.0).unwrap();
        let mut variants = [port; 3];
        for (layout, yard) in variants.iter_mut().enumerate() {
            yard.buildings.fill(LandPiece::EMPTY);
            yard.buildings_count = 0;
            assert!(yard.build_yard(&world, (layout as u64) << 36));
            assert_eq!(yard.buildings_count, 11);
        }
        for (i, first) in variants.iter().enumerate() {
            for second in &variants[i + 1..] {
                assert_ne!(first.buildings.map(|piece| piece.rect), second.buildings.map(|piece| piece.rect));
            }
        }
    }

    #[test]
    fn moored_hulls_fit_real_water_and_do_not_overlap_each_other() {
        for (seed, _, [x, z], kind) in FIXTURES {
            let world = World::new(seed);
            let port = Port::generate(&world, seed, x, z).unwrap();
            let mut small = 0;
            for (index, boat) in port.vessels().iter().copied().enumerate() {
                small += usize::from(boat.length < 10.0);
                assert!(boat.has_water_clearance(&world));
                let half = ((boat.length + boat.beam) * 0.5) as i32 + 1;
                for dz in -2 * half..=2 * half {
                    for dx in -2 * half..=2 * half {
                        let x = boat.x + dx as f32 * 0.5;
                        let z = boat.z + dz as f32 * 0.5;
                        if let Some(sample) = boat.sample(x, z) {
                            let natural = world.sample(x, z);
                            assert_eq!(natural.elevation, SEA_LEVEL);
                            assert!(matches!(
                                natural.surface,
                                Surface::Shallows | Surface::DeepWater
                            ));
                            assert_eq!(port.sample(x, z, natural), Some(sample));
                            assert!(port.vessels()[index + 1..]
                                .iter()
                                .all(|other| other.sample(x, z).is_none()));
                        }
                    }
                }
            }
            assert!(small > 0, "{kind:?}");
        }
    }

    #[test]
    fn pier_runs_from_the_landing_to_a_dry_deck_next_to_each_primary_vessel() {
        for (seed, _, [x, z], kind) in FIXTURES {
            let world = World::new(seed);
            let port = Port::generate(&world, seed, x, z).unwrap();
            let pier = port.pier.unwrap_or_else(|| panic!("missing {kind:?} pier"));
            let at = |along, out| {
                let [x, z] = port.shore.world(along, out);
                let raw = world.sample(x, z);
                (raw, port.sample(x, z, raw))
            };
            let (dry, _) = at(0.0, pier.walkway.rear + 0.1);
            let (wet, _) = at(0.0, pier.berth.front - 0.1);
            assert!(dry.elevation > SEA_LEVEL);
            assert_eq!(wet.elevation, SEA_LEVEL);
            for step in 0..=50 {
                let out = pier.walkway.rear + 0.02
                    + (pier.berth.front - pier.walkway.rear - 0.04) * step as f32 / 50.0;
                let (ground, deck) = at(0.0, out);
                let deck = deck.expect("continuous gangway and berth");
                assert_eq!(deck.surface, PortSurface::Quay);
                assert!(deck.elevation >= ground.elevation && deck.elevation > SEA_LEVEL);
            }
            let moored = port.vessels()[0];
            let [along, out] = port.shore.local(moored.x, moored.z);
            let clearance = out - moored.beam * 0.5 - pier.berth.front;
            assert!((0.75..0.77).contains(&clearance));
            assert!(pier.berth.contains(along, pier.berth.front - 0.06));
        }
    }
}
