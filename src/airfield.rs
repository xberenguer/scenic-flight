//! Sparse, seeded airfields fitted to the existing terrain and roads.

use crate::math::{fraction, mix, sin_cos};
use crate::roads::{ROAD_REGION_SIZE, RoadNetwork, RoadPoint};
use crate::world::{SEA_LEVEL, Surface, World};
use core::f32::consts::{FRAC_PI_2, PI};

pub const AIRFIELD_TILE_SIZE: i32 = ROAD_REGION_SIZE;
const COUNTRY_GRADE: f32 = 0.049;
const REGIONAL_GRADE: f32 = 0.038;
const CLEARANCE: f32 = SEA_LEVEL + 4.0;
const ACCESS_HALF_WIDTH: f32 = 1.65;

mod aprons;
mod buildings;
mod runways;
pub use aprons::ApronProfile;
pub use runways::RunwayProfile;
const BUILDING_SLOTS: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AirfieldKind {
    Country,
    Regional,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AirfieldMaterial {
    GrassRunway,
    PavedRunway,
    Taxiway,
    Apron,
    AccessRoad,
    RunwayMarking,
    EdgeLight,
    HangarRoof,
    PitchedHangarRoof,
    GlazedAirfieldRoof,
    ApronStand,
    ApronMarking,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirfieldSample {
    pub material: AirfieldMaterial,
    /// Runway and pavement follow the existing terrain. Hangars have roofs.
    pub elevation: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirfieldSite {
    pub kind: AirfieldKind,
    local: Basis,
    runway: Rect,
    apron: Rect,
    taxiway: Rect,
    buildings: [Option<Building>; BUILDING_SLOTS],
    access: [Option<AccessLeg>; 2],
    bounds: Bounds,
    apron_profile: u8,
    runway_profile: u8,
    runway_number: u8,
}

impl AirfieldSite {
    /// Find one isolated country strip, when a tile has suitable open ground.
    pub fn generate(world: &World, seed: u64, tile_x: i32, tile_z: i32) -> Option<Self> {
        Self::generate_country(
            world,
            seed,
            Tile::new(tile_x, tile_z)?,
            |_| true,
            &mut || {},
        )
    }

    pub(crate) fn country_for_road(
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let tile = Tile::at(Point::from_road(*road.polyline().first()?))?;
        Self::generate_country(
            world,
            seed,
            tile,
            |p| road.proximity(p.x, p.z).is_none(),
            pulse,
        )
    }

    fn generate_country(
        world: &World,
        seed: u64,
        tile: Tile,
        clear: impl Fn(Point) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let layout = country_layout(seed, tile);
        if layout & 15 >= 3 {
            return None;
        }
        for attempt in 0..16 {
            pulse();
            let key = mix(layout ^ attempt);
            let location = Point {
                x: tile.x + 129.0 + fraction(key) * 766.0,
                z: tile.z + 129.0 + fraction(mix(key)) * 766.0,
            };
            if !open_ground(world, location) {
                continue;
            }
            let half_length = 31.0 + 11.0 * fraction(mix(key ^ 23));
            let rotation = fraction(mix(key ^ 61)) * PI;
            for index in 0..8 {
                let dir = Direction::at(rotation + index as f32 * (PI / 8.0));
                let side = if (key ^ index) & 1 == 0 { -1.0 } else { 1.0 };
                let shape = Layout::new(AirfieldKind::Country, half_length, side);
                let basis = Basis {
                    origin: location,
                    direction: dir,
                };
                if let Some(site) = shape.place(world, basis, tile, key, None, &clear, pulse) {
                    return Some(site);
                }
            }
        }
        None
    }

    /// Find at most one regional airport with an actual link to this road.
    pub fn for_road(world: &World, seed: u64, road: &RoadNetwork) -> Option<Self> {
        Self::for_road_cooperatively(world, seed, road, &mut || {})
    }

    pub(crate) fn for_road_cooperatively(
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let points = road.polyline();
        let termini = [
            (*points.first()?, *points.get(1)?),
            (*points.last()?, *points.get(points.len().checked_sub(2)?)?),
        ];
        let start = Point::from_road(termini[0].0);
        let tile = Tile::at(start)?;
        // The sparse country allocation and regional allocation are disjoint.
        if country_layout(seed, tile) & 15 < 3 {
            return None;
        }
        let layout = mix(seed ^ tile.key() ^ 0x478b_358d_b91b_0a67);
        if layout & 7 >= 5 {
            return None;
        }
        let first = ((layout >> 8) & 1) as usize;
        let half_length = 57.0 + 13.0 * fraction(mix(layout >> 16));
        for choice in [first, 1 - first] {
            let (point, approach) = termini[choice];
            let anchor = Point::from_road(point);
            let outward = Direction::between(Point::from_road(approach), anchor);
            let seed_side = if (layout >> (40 + choice)) & 1 == 0 {
                -1.0
            } else {
                1.0
            };
            for attempt in 0..26 {
                pulse();
                let key = mix(layout ^ (attempt + choice as u64 * 18));
                let side = if attempt & 1 == 0 {
                    seed_side
                } else {
                    -seed_side
                };
                let setback = 28.0 + 96.0 * fraction(key);
                let separation = 44.0 + 96.0 * fraction(mix(key));
                let apron = anchor.offset(outward, -setback, separation * side);
                if !grade_height(world.elevation(apron.x, apron.z)) {
                    continue;
                }
                let geometry = Layout::new(AirfieldKind::Regional, half_length, -side);
                for rotation in [
                    FRAC_PI_2,
                    FRAC_PI_2 - 0.30,
                    FRAC_PI_2 + 0.30,
                    FRAC_PI_2 - 0.62,
                    FRAC_PI_2 + 0.62,
                    FRAC_PI_2 - 0.95,
                    FRAC_PI_2 + 0.95,
                    FRAC_PI_2 - 1.25,
                    FRAC_PI_2 + 1.25,
                ] {
                    pulse();
                    let direction = outward.rotated(rotation);
                    let Rect { along, across, .. } = geometry.apron;
                    let origin = apron.offset(direction, -along, -across);
                    let basis = Basis { origin, direction };
                    let access = RoadLink {
                        road,
                        anchor,
                        outward,
                        apron,
                    };
                    if let Some(site) =
                        geometry.place(world, basis, tile, key, Some(access), |_| true, pulse)
                    {
                        return Some(site);
                    }
                }
            }
        }
        None
    }

    pub fn center(&self) -> (f32, f32) {
        (self.local.origin.x, self.local.origin.z)
    }

    pub fn runway_endpoints(&self) -> [(f32, f32); 2] {
        let a = self.local.world(-self.runway.half_length, 0.0);
        let b = self.local.world(self.runway.half_length, 0.0);
        [(a.x, a.z), (b.x, b.z)]
    }

    pub(crate) fn runway_edges_at_center(&self) -> [(f32, f32); 2] {
        [-self.runway.half_width, self.runway.half_width].map(|across| {
            let point = self.local.world(0.0, across);
            (point.x, point.z)
        })
    }

    pub fn apron_profile(&self) -> ApronProfile {
        ApronProfile::ALL[usize::from(self.apron_profile)]
    }

    pub fn runway_profile(&self) -> RunwayProfile {
        RunwayProfile::ALL[usize::from(self.runway_profile)]
    }

    /// Threshold numbers in the same order as the surveyed runway endpoints.
    pub fn runway_numbers(&self) -> [u8; 2] {
        [self.runway_number, runways::reciprocal(self.runway_number)]
    }

    pub fn apron_center(&self) -> [f32; 2] {
        let point = self.local.world(self.apron.along, self.apron.across);
        [point.x, point.z]
    }

    pub fn road_contact(&self) -> Option<(f32, f32)> {
        self.access[0].map(|leg| (leg.start.x, leg.start.z))
    }

    pub fn sample(&self, x: f32, z: f32) -> Option<AirfieldSample> {
        let point = Point { x, z };
        if !self.bounds.contains(point) {
            return None;
        }
        let (along, across) = self.local.coordinates(point);
        if self.runway.contains(along, across) {
            return Some(mark_ground(self.runway_material(along, across)));
        }
        if self.taxiway.contains(along, across) {
            return Some(mark_ground(AirfieldMaterial::Taxiway));
        }
        if self.apron.contains(along, across) {
            let u = (along - self.apron.along) / self.apron.half_length;
            let v =
                (across - self.apron.across) * self.apron.across.signum() / self.apron.half_width;
            return Some(mark_ground(self.apron_profile().sample(u, v)));
        }
        for building in self.buildings.iter().flatten() {
            if building.area.contains(along, across) {
                let u = (along - building.area.along) / building.area.half_length;
                let v = (across - building.area.across) / building.area.half_width;
                if let Some((roof, rise)) =
                    buildings::sample(building.design, u, v * self.apron.across.signum())
                {
                    let material = match roof {
                        buildings::Roof::Flat => AirfieldMaterial::HangarRoof,
                        buildings::Roof::Pitched => AirfieldMaterial::PitchedHangarRoof,
                        buildings::Roof::Glazed => AirfieldMaterial::GlazedAirfieldRoof,
                    };
                    return Some(AirfieldSample {
                        material,
                        elevation: Some(building.base + building.height * rise),
                    });
                }
            }
        }
        self.access
            .iter()
            .flatten()
            .any(|leg| leg.contains(point))
            .then_some(mark_ground(AirfieldMaterial::AccessRoad))
    }

    fn runway_material(&self, along: f32, across: f32) -> AirfieldMaterial {
        let end = usize::from(along > 0.0);
        self.runway_profile().sample(
            self.runway.half_length - along.abs(),
            across * if end == 0 { 1.0 } else { -1.0 },
            self.runway.half_width,
            self.runway_numbers()[end],
        )
    }
}

fn country_layout(seed: u64, tile: Tile) -> u64 {
    mix(seed ^ tile.key() ^ 0x71af_db5d_0341_7d79)
}

fn mark_ground(material: AirfieldMaterial) -> AirfieldSample {
    AirfieldSample {
        material,
        elevation: None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f32,
    z: f32,
}

impl Point {
    fn from_road(point: RoadPoint) -> Self {
        Self {
            x: point.x,
            z: point.z,
        }
    }

    fn offset(self, dir: Direction, along: f32, across: f32) -> Self {
        Self {
            x: self.x + dir.fx * along + dir.fz * across,
            z: self.z + dir.fz * along - dir.fx * across,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Direction {
    fx: f32,
    fz: f32,
}

impl Direction {
    fn at(radians: f32) -> Self {
        let (fx, fz) = sin_cos(radians);
        Self { fx, fz }
    }

    fn between(a: Point, b: Point) -> Self {
        let (dx, dz) = (b.x - a.x, b.z - a.z);
        let distance = length(dx, dz);
        Self {
            fx: dx / distance,
            fz: dz / distance,
        }
    }

    fn rotated(self, angle: f32) -> Self {
        let (sin, cos) = sin_cos(angle);
        Self {
            fx: self.fx * cos + self.fz * sin,
            fz: self.fz * cos - self.fx * sin,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Basis {
    origin: Point,
    direction: Direction,
}

impl Basis {
    fn world(self, along: f32, across: f32) -> Point {
        self.origin.offset(self.direction, along, across)
    }

    fn coordinates(self, point: Point) -> (f32, f32) {
        let (dx, dz) = (point.x - self.origin.x, point.z - self.origin.z);
        (
            dx * self.direction.fx + dz * self.direction.fz,
            dx * self.direction.fz - dz * self.direction.fx,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    along: f32,
    across: f32,
    half_length: f32,
    half_width: f32,
}

impl Rect {
    fn contains(self, along: f32, across: f32) -> bool {
        (along - self.along).abs() <= self.half_length
            && (across - self.across).abs() <= self.half_width
    }

    fn corners(self, basis: Basis) -> [Point; 4] {
        [
            basis.world(self.along - self.half_length, self.across - self.half_width),
            basis.world(self.along + self.half_length, self.across - self.half_width),
            basis.world(self.along + self.half_length, self.across + self.half_width),
            basis.world(self.along - self.half_length, self.across + self.half_width),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Building {
    area: Rect,
    base: f32,
    height: f32,
    design: u8,
}

#[derive(Clone, Copy)]
enum BuildingRole {
    CountryHangar = 0,
    RegionalHangar = 1,
    Terminal = 2,
    Operations = 3,
    Service = 4,
}

#[derive(Clone, Copy)]
struct BuildingLot {
    area: Rect,
    height: f32,
    role: BuildingRole,
}

fn design_for(role: BuildingRole, key: u64, slot: usize) -> u8 {
    role as u8 * 5 + (mix(key ^ ((slot as u64 + 1) << 33)) % 5) as u8
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
}

impl Bounds {
    fn at(p: Point) -> Self {
        Self {
            min_x: p.x,
            max_x: p.x,
            min_z: p.z,
            max_z: p.z,
        }
    }

    fn include(&mut self, p: Point, halo: f32) {
        self.min_x = self.min_x.min(p.x - halo);
        self.max_x = self.max_x.max(p.x + halo);
        self.min_z = self.min_z.min(p.z - halo);
        self.max_z = self.max_z.max(p.z + halo);
    }

    fn contains(self, p: Point) -> bool {
        p.x >= self.min_x && p.x <= self.max_x && p.z >= self.min_z && p.z <= self.max_z
    }
}

#[derive(Clone, Copy)]
struct Tile {
    gx: i32,
    gz: i32,
    x: f32,
    z: f32,
}

impl Tile {
    fn new(gx: i32, gz: i32) -> Option<Self> {
        const LIMIT: i32 = 15_000;
        if !(-LIMIT..=LIMIT).contains(&gx) || !(-LIMIT..=LIMIT).contains(&gz) {
            return None;
        }
        Some(Self {
            gx,
            gz,
            x: (gx * AIRFIELD_TILE_SIZE) as f32,
            z: (gz * AIRFIELD_TILE_SIZE) as f32,
        })
    }

    fn at(p: Point) -> Option<Self> {
        let axis = |a: f32| {
            let scaled = a / AIRFIELD_TILE_SIZE as f32;
            let i = scaled as i32;
            i.saturating_sub(i32::from(scaled < i as f32))
        };
        Self::new(axis(p.x), axis(p.z))
    }

    fn key(self) -> u64 {
        (u64::from(self.gx as u32) << 32) | u64::from(self.gz as u32)
    }

    fn contains(self, p: Point) -> bool {
        let margin = 5.0;
        p.x >= self.x + margin
            && p.x <= self.x + AIRFIELD_TILE_SIZE as f32 - margin
            && p.z >= self.z + margin
            && p.z <= self.z + AIRFIELD_TILE_SIZE as f32 - margin
    }
}

#[derive(Clone, Copy)]
struct RoadLink<'a> {
    road: &'a RoadNetwork,
    anchor: Point,
    outward: Direction,
    apron: Point,
}

impl RoadLink<'_> {
    fn keep_clear(self, point: Point) -> bool {
        let position = Basis {
            origin: self.anchor,
            direction: self.outward,
        }
        .coordinates(point);
        position.0 < -5.0 && self.road.proximity(point.x, point.z).is_none()
    }

    fn access(
        self,
        world: &World,
        tile: Tile,
        basis: Basis,
        runway: Rect,
        taxiway: Rect,
        pulse: &mut impl FnMut(),
    ) -> Option<[Option<AccessLeg>; 2]> {
        let mut clear = |leg: AccessLeg| {
            pulse();
            !leg.intersects(basis, runway)
                && !leg.intersects(basis, taxiway)
                && leg.clear(world, tile, pulse)
        };
        let direct = AccessLeg::new(self.anchor, self.apron);
        if clear(direct) {
            return Some([Some(direct), None]);
        }
        let joint = Point {
            x: (self.anchor.x + self.apron.x) * 0.5,
            z: (self.anchor.z + self.apron.z) * 0.5,
        };
        for offset in [-9.0, 9.0, -17.0, 17.0, -27.0, 27.0, -38.0, 38.0] {
            let knee = joint.offset(self.outward, offset, 0.0);
            let first = AccessLeg::new(self.anchor, knee);
            let second = AccessLeg::new(knee, self.apron);
            if clear(first) && clear(second) {
                return Some([Some(first), Some(second)]);
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct AccessLeg {
    start: Point,
    end: Point,
    direction: Direction,
    length: f32,
}

impl AccessLeg {
    fn new(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
            direction: Direction::between(start, end),
            length: length(end.x - start.x, end.z - start.z),
        }
    }

    fn contains(self, p: Point) -> bool {
        let (u, v) = Basis {
            origin: self.start,
            direction: self.direction,
        }
        .coordinates(p);
        let edge = u - u.clamp(0.0, self.length);
        edge * edge + v * v <= ACCESS_HALF_WIDTH * ACCESS_HALF_WIDTH
    }

    fn intersects(self, basis: Basis, rect: Rect) -> bool {
        let (ax, az) = basis.coordinates(self.start);
        let (bx, bz) = basis.coordinates(self.end);
        let mut entry = 0.0_f32;
        let mut exit = 1.0_f32;
        for (a, b, center, half) in [
            (ax, bx, rect.along, rect.half_length),
            (az, bz, rect.across, rect.half_width),
        ] {
            let half = half + ACCESS_HALF_WIDTH;
            let low = center - half;
            let high = center + half;
            let delta = b - a;
            if delta.abs() <= 0.0001 {
                if a < low || a > high {
                    return false;
                }
            } else {
                let p = (low - a) / delta;
                let q = (high - a) / delta;
                entry = entry.max(p.min(q));
                exit = exit.min(p.max(q));
                if entry > exit {
                    return false;
                }
            }
        }
        true
    }

    fn clear(self, world: &World, tile: Tile, pulse: &mut impl FnMut()) -> bool {
        let basis = Basis {
            origin: self.start,
            direction: self.direction,
        };
        let strip = Rect {
            along: self.length * 0.5,
            across: 0.0,
            half_length: self.length * 0.5,
            half_width: ACCESS_HALF_WIDTH,
        };
        strip
            .corners(basis)
            .into_iter()
            .all(|point| tile.contains(point))
            && footprint(world, basis, strip, 0.119, 0.18, false, |_| true, pulse).is_some()
    }
}

struct Layout {
    kind: AirfieldKind,
    runway: Rect,
    apron: Rect,
    taxiway: Rect,
    buildings: [Option<BuildingLot>; BUILDING_SLOTS],
}

impl Layout {
    fn new(kind: AirfieldKind, half_length: f32, side: f32) -> Self {
        let regional = kind == AirfieldKind::Regional;
        let half_width = if regional { 5.5 } else { 3.7 };
        let taxi_length = if regional { 9.0 } else { 5.0 };
        let apron_width = if regional { 8.0 } else { 5.0 };
        let apron_length = if regional { 13.0 } else { 8.5 };
        let apron_position = -half_length * 0.28;
        let apron_across = side * (half_width + taxi_length + apron_width);
        let building_setback = if regional { 4.0 } else { 3.35 };
        let building_side = apron_across + side * (apron_width + building_setback);
        let lot = |along, across, half_length, half_width, height, role| {
            Some(BuildingLot {
                area: Rect {
                    along: apron_position + along,
                    across,
                    half_length,
                    half_width,
                },
                height,
                role,
            })
        };
        use BuildingRole::*;
        Self {
            kind,
            runway: Rect {
                along: 0.0,
                across: 0.0,
                half_length,
                half_width,
            },
            apron: Rect {
                along: apron_position,
                across: apron_across,
                half_length: apron_length,
                half_width: apron_width,
            },
            taxiway: Rect {
                along: apron_position + 2.3,
                across: side * (half_width + taxi_length * 0.5),
                half_length: if regional { 2.1 } else { 1.65 },
                half_width: taxi_length * 0.5 + 0.55,
            },
            buildings: if regional {
                [
                    lot(-6.2, building_side, 4.6, 3.5, 6.1, RegionalHangar),
                    lot(6.4, building_side, 4.0, 3.2, 5.5, Terminal),
                    lot(6.0, building_side + side * 7.0, 2.5, 2.5, 7.9, Operations),
                    lot(-5.2, building_side + side * 7.0, 3.8, 2.6, 4.5, Service),
                ]
            } else {
                [
                    lot(-2.0, building_side, 3.0, 3.1, 4.7, CountryHangar),
                    lot(5.3, building_side, 2.45, 2.35, 3.4, Service),
                    None,
                    None,
                ]
            },
        }
    }

    fn place(
        &self,
        world: &World,
        basis: Basis,
        tile: Tile,
        key: u64,
        road: Option<RoadLink<'_>>,
        clear: impl Fn(Point) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<AirfieldSite> {
        let check = |p| tile.contains(p) && clear(p) && road.is_none_or(|link| link.keep_clear(p));
        let mut bounds = Bounds::at(basis.origin);
        for rect in [self.runway, self.taxiway, self.apron] {
            for corner in rect.corners(basis) {
                if !check(corner) {
                    return None;
                }
                bounds.include(corner, 0.0);
            }
        }
        let paved = self.kind == AirfieldKind::Regional;
        let runway_grade = if paved { REGIONAL_GRADE } else { COUNTRY_GRADE };
        // A country strip uses a natural clearing. A regional field may
        // clear existing vegetation, but its whole site still needs good grade.

        footprint(
            world,
            basis,
            self.runway,
            runway_grade,
            0.042,
            !paved,
            check,
            pulse,
        )?;
        footprint(
            world,
            basis,
            self.taxiway,
            0.082,
            0.093,
            !paved,
            check,
            pulse,
        )?;
        footprint(world, basis, self.apron, 0.058, 0.066, !paved, check, pulse)?;
        let access = if let Some(link) = road {
            link.access(world, tile, basis, self.runway, self.taxiway, pulse)?
        } else {
            [None, None]
        };
        for leg in access.iter().flatten() {
            bounds.include(leg.start, ACCESS_HALF_WIDTH);
            bounds.include(leg.end, ACCESS_HALF_WIDTH);
        }
        let mut buildings = [None; BUILDING_SLOTS];
        for (index, lot) in self.buildings.into_iter().enumerate() {
            let Some(BuildingLot { area, height, role }) = lot else {
                continue;
            };
            if access
                .iter()
                .flatten()
                .any(|leg| leg.intersects(basis, area))
            {
                continue;
            }
            if let Some(base) = footprint(world, basis, area, 0.11, 0.11, !paved, check, pulse) {
                for corner in area.corners(basis) {
                    bounds.include(corner, 0.0);
                }
                let design = design_for(role, key, index);
                buildings[index] = Some(Building {
                    area,
                    base,
                    height,
                    design,
                });
            }
        }
        Some(AirfieldSite {
            kind: self.kind,
            local: basis,
            runway: self.runway,
            apron: self.apron,
            taxiway: self.taxiway,
            buildings,
            access,
            bounds,
            apron_profile: (mix(key ^ 0x292e_b327_291c_fda1) % ApronProfile::ALL.len() as u64)
                as u8,
            runway_profile: RunwayProfile::choose(self.kind, mix(key ^ 0x98bb_0761_f21d_4307)),
            runway_number: runways::number(basis.direction),
        })
    }
}

fn footprint(
    world: &World,
    basis: Basis,
    rect: Rect,
    grade: f32,
    cross_grade: f32,
    clear_surface: bool,
    check: impl Fn(Point) -> bool,
    pulse: &mut impl FnMut(),
) -> Option<f32> {
    let longitudinal = 2.0 * rect.half_length;
    let lateral = 2.0 * rect.half_width;
    let begin = rect.along - rect.half_length;
    let across = rect.across - rect.half_width;
    let mut top = f32::NEG_INFINITY;
    let mut average = [0.0; 3];
    for (index, row) in [0., 0.5, 1.].into_iter().enumerate() {
        for step in 0..=2 {
            let point = basis.world(
                begin + longitudinal * step as f32 * 0.5,
                across + lateral * row,
            );
            let terrain = world.elevation(point.x, point.z);
            if !grade_height(terrain) || !check(point) {
                return None;
            }
            if step == 0 {
                average[index] = terrain;
            } else if (terrain - average[index]).abs()
                > grade * longitudinal * step as f32 * 0.5 + 0.01
            {
                return None;
            }
        }
    }
    let length_steps = (longitudinal / 1.6) as usize + 1;
    let cross_steps = (lateral / 1.6) as usize + 1;
    let du = longitudinal / length_steps as f32;
    let dv = lateral / cross_steps as f32;
    const MAX_LANES: usize = 20;
    if cross_steps >= MAX_LANES {
        return None;
    }
    let mut prior = [0.0; MAX_LANES];
    for step in 0..=length_steps {
        pulse();
        let mut last = 0.0;
        for (lane, old) in prior.iter_mut().enumerate().take(cross_steps + 1) {
            let point = basis.world(begin + du * step as f32, across + dv * lane as f32);
            if !check(point) {
                return None;
            }
            let terrain = if clear_surface {
                let sample = world.sample(point.x, point.z);
                if !matches!(sample.surface, Surface::Grass | Surface::Desert) {
                    return None;
                }
                sample.elevation
            } else {
                world.elevation(point.x, point.z)
            };
            if !grade_height(terrain)
                || (step != 0 && (terrain - *old).abs() > grade * du + 0.003)
                || (lane != 0 && (terrain - last).abs() > cross_grade * dv + 0.003)
            {
                return None;
            }
            *old = terrain;
            last = terrain;
            top = top.max(terrain);
        }
    }
    Some(top)
}

fn grade_height(height: f32) -> bool {
    (CLEARANCE..=SEA_LEVEL + 52.0).contains(&height)
}

fn open_ground(world: &World, point: Point) -> bool {
    let sample = world.sample(point.x, point.z);
    grade_height(sample.elevation) && matches!(sample.surface, Surface::Grass | Surface::Desert)
}

fn length(dx: f32, dz: f32) -> f32 {
    let squared = dx * dx + dz * dz;
    let mut root = dx.abs().max(dz.abs());
    for _ in 0..3 {
        root = (root + squared / root) * 0.5;
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    const COUNTRY: [(u64, i32, i32); 2] = [(1, -2, -4), (31, 1, -1)];
    const REGIONAL: [(u64, i32, i32); 2] = [(1, 1, -2), (7, 1, -4)];

    #[test]
    fn seeded_country_and_regional_tiles_are_repeatable_and_disjoint() {
        for (seed, x, z) in COUNTRY {
            let world = World::new(seed);
            let site = AirfieldSite::generate(&world, seed, x, z).unwrap();
            assert_eq!(site, AirfieldSite::generate(&world, seed, x, z).unwrap());
            assert_eq!(site.kind, AirfieldKind::Country);
            if let Some(road) = RoadNetwork::generate(&world, x, z) {
                assert!(AirfieldSite::for_road(&world, seed, &road).is_none());
            }
        }
        for (seed, x, z) in REGIONAL {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, x, z).unwrap();
            let site = AirfieldSite::for_road(&world, seed, &road).unwrap();
            assert_eq!(site, AirfieldSite::for_road(&world, seed, &road).unwrap());
            assert_eq!(site.kind, AirfieldKind::Regional);
            assert!(site.runway.half_length > 55.0);
            assert!(AirfieldSite::generate(&world, seed, x, z).is_none());
        }
        let world = World::new(1);
        let mut strips = 0;
        for z in -6..=6 {
            for x in -6..=6 {
                strips += usize::from(AirfieldSite::generate(&world, 1, x, z).is_some());
            }
        }
        assert!((1..12).contains(&strips));
    }

    #[test]
    fn runway_pavement_is_dry_and_gently_graded_at_fine_spacing() {
        for (kind, fixtures) in [
            (AirfieldKind::Country, COUNTRY),
            (AirfieldKind::Regional, REGIONAL),
        ] {
            for (seed, x, z) in fixtures {
                let world = World::new(seed);
                let site = if kind == AirfieldKind::Country {
                    AirfieldSite::generate(&world, seed, x, z)
                } else {
                    AirfieldSite::for_road(
                        &world,
                        seed,
                        &RoadNetwork::generate(&world, x, z).unwrap(),
                    )
                }
                .unwrap();
                let grade = if kind == AirfieldKind::Country {
                    COUNTRY_GRADE
                } else {
                    REGIONAL_GRADE
                };
                let rect = site.runway;
                let across = (rect.half_width * 4.0) as usize + 1;
                let along = (rect.half_length * 4.0) as usize + 1;
                let du = 2.0 * rect.half_length / along as f32;
                let dv = 2.0 * rect.half_width / across as f32;
                for i in 0..=across {
                    let mut prior = 0.0;
                    let v = -rect.half_width + dv * i as f32;
                    for j in 0..=along {
                        let u = -rect.half_length + du * j as f32;
                        let p = site.local.world(u, v);
                        let ground = world.elevation(p.x, p.z);
                        assert!(grade_height(ground));
                        if j != 0 {
                            assert!((ground - prior).abs() / du < grade + 0.008);
                        }
                        if i != 0 {
                            let side = site.local.world(u, v - dv);
                            let dh = (world.elevation(side.x, side.z) - ground).abs();
                            assert!(dh / dv < 0.051);
                        }
                        prior = ground;
                    }
                }
                assert!(
                    footprint(
                        &world,
                        site.local,
                        site.taxiway,
                        0.082,
                        0.093,
                        false,
                        |_| true,
                        &mut || {},
                    )
                    .is_some()
                );
                assert!(
                    footprint(
                        &world,
                        site.local,
                        site.apron,
                        0.058,
                        0.066,
                        false,
                        |_| true,
                        &mut || {}
                    )
                    .is_some()
                );
            }
        }
    }

    #[test]
    fn all_airfield_plans_are_distinct_and_surveyed_buildings_stay_off_pavement() {
        use std::collections::HashSet;

        let mut signatures = HashSet::new();
        let mut selected = HashSet::new();
        for (slot, role) in [
            BuildingRole::CountryHangar,
            BuildingRole::RegionalHangar,
            BuildingRole::Terminal,
            BuildingRole::Operations,
            BuildingRole::Service,
        ]
        .into_iter()
        .enumerate()
        {
            selected.extend((0..128).map(|key| design_for(role, key, slot)));
        }
        assert_eq!(selected.len(), 25);
        for design in selected {
            let mut shape = [0; 21 * 21];
            for (i, value) in shape.iter_mut().enumerate() {
                let along = (i % 21) as f32 * 0.1 - 1.0;
                let across = (i / 21) as f32 * 0.1 - 1.0;
                if let Some((_, height)) = buildings::sample(design, along, across) {
                    assert!((0.0..=1.0001).contains(&height));
                    *value = (height * 100.0) as u8;
                }
            }
            assert!(shape.iter().filter(|&&value| value > 0).count() > 90);
            assert!(
                signatures.insert(shape),
                "duplicated airfield design {design}"
            );
        }
        let intersects = |a: Rect, b: Rect| {
            (a.along - b.along).abs() < a.half_length + b.half_length
                && (a.across - b.across).abs() < a.half_width + b.half_width
        };
        let mut roles = HashSet::new();
        for ((seed, x, z), regional) in COUNTRY
            .map(|fixture| (fixture, false))
            .into_iter()
            .chain(REGIONAL.map(|fixture| (fixture, true)))
        {
            let world = World::new(seed);
            let site = if regional {
                let road = RoadNetwork::generate(&world, x, z).unwrap();
                AirfieldSite::for_road(&world, seed, &road).unwrap()
            } else {
                AirfieldSite::generate(&world, seed, x, z).unwrap()
            };
            for (slot, building) in site.buildings.iter().enumerate() {
                let Some(building) = building else {
                    continue;
                };
                assert!(
                    [site.runway, site.taxiway, site.apron]
                        .into_iter()
                        .all(|paved| !intersects(paved, building.area))
                );
                assert!(
                    site.access
                        .iter()
                        .flatten()
                        .all(|leg| !leg.intersects(site.local, building.area))
                );
                assert!(
                    site.buildings[slot + 1..]
                        .iter()
                        .flatten()
                        .all(|other| !intersects(other.area, building.area))
                );
                let mut sampled_roof = false;
                for row in 0..=8 {
                    for col in 0..=8 {
                        let u = col as f32 * 0.25 - 1.0;
                        let v = row as f32 * 0.25 - 1.0;
                        let pos = site.local.world(
                            building.area.along + building.area.half_length * u * 0.98,
                            building.area.across + building.area.half_width * v * 0.98,
                        );
                        if let Some(elevation) =
                            site.sample(pos.x, pos.z).and_then(|part| part.elevation)
                        {
                            assert!(elevation > world.elevation(pos.x, pos.z));
                            assert!(elevation <= building.base + building.height + 0.001);
                            sampled_roof = true;
                        }
                    }
                }
                assert!(
                    sampled_roof,
                    "No roof pixels for design {}",
                    building.design
                );
                roles.insert(building.design / 5);
            }
        }
        assert_eq!(
            roles.len(),
            5,
            "Published airfield building roles: {roles:?}"
        );
    }

    #[test]
    fn taxiway_joins_runway_apron_and_exact_highway_contact() {
        for (seed, x, z) in REGIONAL {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, x, z).unwrap();
            let site = AirfieldSite::for_road(&world, seed, &road).unwrap();
            let contact = site.road_contact().unwrap();
            let first = road.polyline()[0];
            let last = *road.polyline().last().unwrap();
            assert!(contact == (first.x, first.z) || contact == (last.x, last.z));
            assert!(road.proximity(contact.0, contact.1).is_some());
            assert_eq!(
                site.sample(contact.0, contact.1).unwrap().material,
                AirfieldMaterial::AccessRoad
            );
            let mut met_taxiway = false;
            let mut met_apron = false;
            for i in 0..=27 {
                let local = site
                    .local
                    .world(site.taxiway.along, site.apron.across * i as f32 / 27.0);
                let tag = site.sample(local.x, local.z).unwrap().material;
                met_taxiway |= tag == AirfieldMaterial::Taxiway;
                met_apron |= matches!(
                    tag,
                    AirfieldMaterial::Apron
                        | AirfieldMaterial::ApronStand
                        | AirfieldMaterial::ApronMarking
                );
            }
            assert!(met_taxiway && met_apron);
            for leg in site.access.iter().flatten() {
                assert!(leg.clear(&world, Tile::new(x, z).unwrap(), &mut || {}));
                for i in 0..=30 {
                    let at = leg
                        .start
                        .offset(leg.direction, leg.length * i as f32 / 30.0, 0.0);
                    assert!(matches!(
                        site.sample(at.x, at.z).unwrap().material,
                        AirfieldMaterial::AccessRoad
                            | AirfieldMaterial::Apron
                            | AirfieldMaterial::ApronStand
                            | AirfieldMaterial::ApronMarking
                    ));
                }
            }
        }
    }
}
