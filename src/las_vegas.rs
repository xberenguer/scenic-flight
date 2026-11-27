//! Georeferenced terrain and scenery sampled at one world unit per metre.

use crate::world::{Sample, Surface, World};
use Surface::*;

mod packed;
#[cfg(feature = "landmark-gallery")]
pub mod landmark_gallery;
mod fremont;
mod sphere;
mod stadium;
pub mod native_pack;
use native_pack::{GroundHeader, NativeDetail};
use packed::PaletteTiles;

pub(crate) const DEPARTURE: (f32, f32, f32) = (-52.0, -2_370.0, 0.02);
pub(crate) fn departure(area: u32) -> ((f32, f32, f32), usize) {
    match area {
        1 => ((2_730.0, -4_470.0, 0.0), 12),
        2 => ((2_170.0, 5_250.0, 0.23), 3),
        3 => ((770.0, 565.0, 0.47), TOUR.len() + 5),
        4 => ((-240.0, 135.0, 1.72), TOUR.len()),
        5 => ((815.0, 682.0, -1.65), TOUR.len() + 1),
        6 => ((-310.0, 980.0, 1.35), TOUR.len() + 2),
        7 => ((-510.0, -1_890.0, -2.51), TOUR.len() + 3),
        8 => ((2_140.0, 6_180.0, 0.75), TOUR.len() + 4),
        _ => (DEPARTURE, 0),
    }
}

pub(crate) const TOUR: [[f32; 2]; 14] = [
    [-192.0, 141.0], [810.0, 1_142.0], [1_326.0, 3_632.0], [2_507.0, 6_407.0],
    [8_104.0, 5_938.0], [15_879.0, -1_929.0], [36_373.0, 893.0],
    [39_445.0, -9_593.0], [13_918.0, -8_217.0], [-1_603.0, -4_857.0],
    [-23_838.0, 3_679.0], [-14_748.0, 5_173.0], [2_730.0, -2_260.0], [820.0, -1_680.0],
];

pub(crate) fn tour_waypoint(leg: usize) -> ([f32; 2], usize) {
    const LANDMARK_APPROACHES: [([f32; 2], usize); 6] = [
        ([480.0, 135.0], 1),
        ([-240.0, 682.0], 1),
        ([530.0, 1_170.0], 2),
        ([-1_320.0, -2_850.0], 9),
        ([2_850.0, 6_950.0], 3),
        ([1_120.0, 1_250.0], 1),
    ];
    if let Some(&point) = TOUR.get(leg) {
        (point, (leg + 1) % TOUR.len())
    } else {
        LANDMARK_APPROACHES[leg - TOUR.len()]
    }
}

const STRAT: [f32; 2] = [1_516.0, 3_958.0];
pub(crate) const STRAT_HEIGHT: f32 = 350.2;
const HEADER: usize = 64;
const TILE_SIDE: usize = 16;
const TILE_BYTES: usize = TILE_SIDE * TILE_SIDE * 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappedCity {
    LasVegas,
    LosAngeles,
}

impl MappedCity {
    fn parse(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::LasVegas),
            1 => Some(Self::LosAngeles),
            _ => None,
        }
    }

    pub(crate) fn departure(self, area: u32) -> ((f32, f32, f32), usize) {
        match self {
            Self::LasVegas => departure(area),
            Self::LosAngeles => crate::los_angeles::departure(area),
        }
    }

    pub(crate) fn waypoint(self, leg: usize, world: &World) -> ([f32; 2], usize) {
        match self {
            Self::LasVegas => tour_waypoint(leg),
            Self::LosAngeles => {
                let [x, z] = crate::los_angeles::WIDER_COVERAGE_POINT;
                let [west, south] = crate::los_angeles::COAST_COVERAGE_POINT;
                crate::los_angeles::tour_waypoint(leg, world.city_occupies(x, z), world.city_occupies(west, south))
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct LasVegasData {
    bytes: &'static [u8],
    city: MappedCity,
    ground_origin: [i32; 2],
    ground_cell: i32,
    ground_size: [usize; 2],
    feature_origin: [i32; 2],
    feature_cell: i32,
    feature_size: [usize; 2],
    ground_water: usize,
    feature_index: usize,
    tiles: usize,
    palettes: Option<PaletteTiles>,
    native_detail: Option<&'static dyn NativeDetail>,
    water_bitmap: bool,
    tile_count: usize,
    base_metres: i32,
}

impl core::fmt::Debug for LasVegasData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LasVegasData").field("bytes", &self.bytes.len())
            .field("terrain", &self.ground_size).field("tiles", &self.tile_count).finish()
    }
}

impl PartialEq for LasVegasData {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.bytes, other.bytes)
    }
}

impl Eq for LasVegasData {}

impl LasVegasData {
    pub fn parse(bytes: &'static [u8]) -> Option<Self> {
        if bytes.len() < HEADER { return None; }
        let compact = match &bytes[..4] {
            b"LVS1" => false,
            b"LVS2" => true,
            _ => return None,
        };
        let header = |index: usize| {
            let at = 4 + index * 4;
            i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
        };
        let nz = |index| usize::try_from(header(index)).ok().filter(|&v| v > 0);
        let ground_size = [nz(3)?, nz(4)?];
        let feature_size = [nz(8)?, nz(9)?];
        let tile_count = nz(10)?;
        let ground_cell = header(2);
        let feature_cell = header(7);
        if !matches!(ground_cell, 20..=120) || !matches!(feature_cell, 1..=20) { return None; }
        let ground_count = ground_size[0].checked_mul(ground_size[1])?;
        let ground_water = HEADER.checked_add(ground_count.checked_mul(2)?)?;
        let water_bytes = if compact { ground_count.div_ceil(8) } else { ground_count };
        let feature_index = ground_water.checked_add(water_bytes)?;
        let tiles = feature_index.checked_add(feature_size[0].checked_mul(feature_size[1])?.checked_mul(4)?)?;
        let palettes = if compact {
            Some(PaletteTiles::parse(bytes, tiles, tile_count)?)
        } else {
            if tiles.checked_add(tile_count.checked_mul(TILE_BYTES)?)? != bytes.len() { return None; }
            None
        };
        Some(Self {
            bytes, city: MappedCity::parse(header(14))?,
            ground_origin: [header(0), header(1)], ground_cell, ground_size,
            feature_origin: [header(5), header(6)], feature_cell, feature_size,
            ground_water, feature_index, tiles, palettes, native_detail: None,
            water_bitmap: compact, tile_count, base_metres: header(13),
        })
    }

    /// Use losslessly decoded SCG1 terrain with an independently managed detail cache.
    pub fn from_native(ground: &'static [u8], detail: &'static dyn NativeDetail) -> Option<Self> {
        let header = GroundHeader::parse(ground).ok()?;
        if ground.len() != header.ground_len { return None; }
        Some(Self {
            bytes: ground,
            city: MappedCity::parse(i32::from_le_bytes(ground[60..64].try_into().ok()?))?,
            ground_origin: header.ground_origin,
            ground_cell: header.ground_cell,
            ground_size: header.ground_size,
            feature_origin: header.feature_origin,
            feature_cell: header.feature_cell,
            feature_size: header.feature_size,
            ground_water: header.water_offset,
            feature_index: 0,
            tiles: 0,
            palettes: None,
            native_detail: Some(detail),
            water_bitmap: true,
            tile_count: 0,
            base_metres: header.base_metres,
        })
    }

    pub fn city(self) -> MappedCity {
        self.city
    }

    pub(crate) fn datum(self) -> f32 { self.base_metres as f32 }

    pub(crate) fn terrain_weight(self, x: f32, z: f32) -> f32 {
        let half = self.ground_cell as f32 * 0.5;
        let west = self.ground_origin[0] as f32 + half;
        let north = self.ground_origin[1] as f32 - half;
        let east = west + (self.ground_size[0] - 1) as f32 * self.ground_cell as f32;
        let south = north - (self.ground_size[1] - 1) as f32 * self.ground_cell as f32;
        crate::math::smoothstep((x - west).min(east - x).min(z - south).min(north - z) / 6_000.0)
    }

    fn u16(self, offset: usize) -> u16 {
        u16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]])
    }

    fn ground(self, x: f32, z: f32) -> Option<(f32, f32, usize)> {
        let gx = (x - self.ground_origin[0] as f32) / self.ground_cell as f32 - 0.5;
        let gz = (self.ground_origin[1] as f32 - z) / self.ground_cell as f32 - 0.5;
        let [w, h] = self.ground_size;
        if gx < 0.0 || gz < 0.0 || gx >= (w - 1) as f32 || gz >= (h - 1) as f32 {
            return None;
        }
        let col = gx as usize;
        let row = gz as usize;
        let index = row * w + col;
        let at = |n| self.u16(HEADER + n * 2) as f32;
        let fx = gx - col as f32;
        let fz = gz - row as f32;
        let top = at(index) + (at(index + 1) - at(index)) * fx;
        let bottom = at(index + w) + (at(index + w + 1) - at(index + w)) * fx;
        let elevation = (top + (bottom - top) * fz - self.base_metres as f32).max(0.0);
        let nearest = index + usize::from(fx >= 0.5) + w * usize::from(fz >= 0.5);
        Some((elevation, (top - bottom).abs(), nearest))
    }

    pub(crate) fn terrain(self, world: &World, x: f32, z: f32) -> (Sample, f32) {
        let Some((elevation, slope, nearest)) = self.ground(x, z) else {
            return (Sample { elevation: 260.0, surface: Mojave }, 0.55);
        };
        let water = if self.water_bitmap {
            self.bytes[self.ground_water + nearest / 8] & (1 << (nearest & 7)) != 0
        } else {
            self.bytes[self.ground_water + nearest] != 0
        };
        let noise = world.noise(x, z, 0.0074, 0xb652_4309);
        let west_range = x < -17_000.0 && z > -6_500.0 && z < 8_000.0;
        let surface = if self.city == MappedCity::LosAngeles {
            if water {
                if elevation < 2.0 { DeepWater } else { TarnWater }
            } else if slope > 10.0 && noise < 0.36 {
                ScreeFine
            } else if noise < 0.42 {
                Mojave
            } else {
                Grass
            }
        } else if water {
            TarnWater
        } else if west_range && elevation > 660.0 && slope > 8.0 {
            if (elevation as i32 / 44) % 3 == 0 { MesaCap } else { MesaCliff }
        } else if elevation > 1_290.0 {
            Rock
        } else if noise < 0.27 && slope > 10.0 {
            ScreeFine
        } else {
            Mojave
        };
        (Sample { elevation, surface }, if water { -0.14 } else { 0.52 })
    }

    pub(crate) fn detail(self, x: f32, z: f32, base: Sample) -> Sample {
        self.detail_sample(x, z).map_or(base, |sample| Sample {
            elevation: base.elevation + sample.elevation,
            ..sample
        })
    }

    pub(crate) fn scene_sample(self, world: &World, x: f32, z: f32) -> Sample {
        if let Some(detail) = self.detail_sample(x, z) {
            // The mapped surface replaces the underlying biome material.
            let ground = self.ground(x, z).map_or(260.0, |(elevation, _, _)| elevation);
            Sample { elevation: ground + detail.elevation, ..detail }
        } else {
            self.terrain(world, x, z).0
        }
    }

    fn detail_sample(self, x: f32, z: f32) -> Option<Sample> {
        let cell_x = (x - self.feature_origin[0] as f32) / self.feature_cell as f32;
        let cell_z = (z - self.feature_origin[1] as f32) / self.feature_cell as f32;
        let [w, h] = self.feature_size;
        if cell_x < 0.0 || cell_z < 0.0 || cell_x >= (w * TILE_SIDE) as f32 || cell_z >= (h * TILE_SIDE) as f32 {
            return None;
        }
        let [col, row] = [cell_x as usize, cell_z as usize];
        let cell = (row % TILE_SIDE) * TILE_SIDE + (col % TILE_SIDE);
        let tile_col = col / TILE_SIDE;
        let tile_row = row / TILE_SIDE;
        let word = if let Some(detail) = self.native_detail {
            detail.detail_word(tile_col, tile_row, cell)
        } else {
            let off = self.feature_index + (tile_row * w + tile_col) * 4;
            let tile = u32::from_le_bytes(self.bytes[off..off + 4].try_into().unwrap()) as usize;
            if !(1..=self.tile_count).contains(&tile) { return None; }
            match self.palettes {
                Some(palettes) => palettes.sample(self.bytes, tile - 1, cell),
                None => self.u16(self.tiles + (tile - 1) * TILE_BYTES + cell * 2),
            }
        };
        let surface = match word & 127 {
            1 | 6 => Pavement,
            2 => TarnWater,
            3 => Gravel,
            5 => RunwayMarking,
            7 => Park,
            8 => RailMetal,
            10 => Pavement,
            11 => PitchedRoof,
            12 => LandmarkRoof06,
            13 => LandmarkRoof09,
            14 => LandmarkRoof11,
            15 => LandmarkRoof20,
            16 => LandmarkRoof25,
            17 => LandmarkRoof35,
            18 => TowerRoof,
            19 => IndustryMetal,
            20 => PyramidGlass,
            21 => VegasLimestone,
            22 => VegasIvory,
            23 => VegasRose,
            24 => VegasSlateGlass,
            25 => VegasBronzeGlass,
            26 => VegasTerracotta,
            27 => VegasCreamSlate,
            28 => VegasGoldGlass,
            29 => VegasStucco,
            30 => VegasConcrete,
            _ => return None,
        };
        let source_height = (word >> 7) as f32;
        let height = if self.city == MappedCity::LasVegas {
            if word & 127 == 10 || stadium::legacy_prism(x, z, surface, source_height)
                || fremont::legacy_prism(x, z, surface, source_height)
            {
                return Some(Sample { surface: Pavement, elevation: 0.0 });
            }
            strip_landmark_terrain_height(x, z, surface, source_height)
        } else {
            if source_height > 0.0 {
                let scale = self.feature_cell as f32;
                let east = self.feature_origin[0] as f32 + (col as f32 + 0.5) * scale;
                let north = self.feature_origin[1] as f32 + (row as f32 + 0.5) * scale;
                if crate::los_angeles::replaces_raster(east, north, source_height, surface) {
                    return Some(Sample { surface: Pavement, elevation: 0.0 });
                }
            }
            source_height
        };
        Some(Sample { surface, elevation: height })
    }

    pub(crate) fn in_mapped_area(self, x: f32, z: f32) -> bool {
        let east = x - self.feature_origin[0] as f32;
        let north = z - self.feature_origin[1] as f32;
        let scale = self.feature_cell as f32 * TILE_SIDE as f32;
        east >= 0.0 && north >= 0.0
            && east < scale * self.feature_size[0] as f32 && north < scale * self.feature_size[1] as f32
    }
}

pub(crate) fn safety_height(x: f32, z: f32) -> f32 {
    use crate::math::smoothstep;
    let strip = 1.0 - smoothstep(((x - 0.27 * z).abs() - 260.0) / 450.0);
    let corridor = smoothstep((z + 2_600.0) / 950.0) * smoothstep((7_100.0 - z) / 1_900.0);
    strip * corridor * 95.0
}

pub(crate) fn cruise_clearance(aircraft: crate::flight::Aircraft) -> f32 {
    use crate::flight::Aircraft::*;
    match aircraft {
        BaldEagle => 78.0,
        Trike => 105.0,
        Glider => 132.0,
        BushPlane => 135.0,
        Racer => 174.0,
        SportJet => 192.0,
        Fighter => 235.0,
        Blackbird => 560.0,
    }
}

pub(crate) fn strat_clearance(x: f32, z: f32) -> f32 {
    let [east, north] = [x - STRAT[0], z - STRAT[1]];
    if east * east + north * north < 24.0 * 24.0 { STRAT_HEIGHT } else { 0.0 }
}

struct StratTower {
    base: f32,
}

impl StratTower {
    fn draw(&self, mesh: &mut crate::world_mesh::WorldMesh<'_>) {
        let profile = [
            (0.0, 16.0, [175, 177, 181]), (36.0, 10.0, [203, 205, 206]),
            (225.0, 7.7, [210, 213, 217]), (239.0, 12.0, [180, 187, 195]),
            (248.0, 22.8, [80, 99, 114]), (261.0, 23.3, [123, 140, 154]),
            (270.0, 17.6, [185, 193, 196]), (279.0, 7.2, [196, 201, 204]),
            (296.0, 2.5, [164, 144, 140]), (STRAT_HEIGHT, 0.8, [176, 161, 159]),
        ];
        mesh.with_landmark_reach(3_200.0, |mesh| {
            for index in 0..12 {
                let from = crate::math::sin_cos(index as f32 * core::f32::consts::TAU / 12.0);
                let to = crate::math::sin_cos((index + 1) as f32 * core::f32::consts::TAU / 12.0);
                let point = |(height, radius, _), (sin, cos): (f32, f32)| {
                    [STRAT[0] + radius * cos, self.base + height, STRAT[1] + radius * sin]
                };
                for levels in profile.windows(2) {
                    mesh.faded_quad(
                        [point(levels[0], from), point(levels[0], to), point(levels[1], to), point(levels[1], from)],
                        levels[0].2, 1.0,
                    );
                }
            }
        });
    }
}

// OpenStreetMap ways 402543930 and 27831699, in the map pack's metre coordinates.
const HIGH_ROLLER: [f32; 2] = [441.6, 625.3];
const HIGH_ROLLER_HEIGHT: f32 = 167.64;
const HIGH_ROLLER_HUB: f32 = 86.14;
const HIGH_ROLLER_RIM: f32 = 71.5;
const HIGH_ROLLER_CAPSULE_TRACK: f32 = 78.5;
const EIFFEL_TOWER: [f32; 2] = [94.4, 53.8];
const EIFFEL_TOWER_HEIGHT: f32 = 164.90;
// OSM way 364846544; Federal Heath gives 180 ft overall and 72 ft diameter.
const PARIS_BALLOON: [f32; 2] = [60.85, -64.83];
const PARIS_BALLOON_HEIGHT: f32 = 54.864;
const PARIS_BALLOON_RADIUS: f32 = 10.973;
// OSM ways 114697072-114697074 split the piers from the elevated span.
const PARIS_ARC: [f32; 2] = [145.85, -42.43];
const PARIS_ARC_HEIGHT: f32 = 33.0;
const PARIS_ARC_HALF_WIDTH: f32 = 14.8;
const PARIS_ARC_HALF_DEPTH: f32 = 7.6;
const PARIS_ARC_OPENING: f32 = 5.82;
const STRIP_LANDMARK_REACH: f32 = 3_200.0;
const STREET_LANDMARK_REACH: f32 = 1_450.0;

/// Remove old map prisms where the detailed models need visible openings.
pub(crate) fn strip_landmark_terrain_height(x: f32, z: f32, surface: Surface, height: f32) -> f32 {
    if surface == LandmarkRoof11 && height == 165.0
        && (x - EIFFEL_TOWER[0]).abs() < 46.0 && (z - EIFFEL_TOWER[1]).abs() < 37.0
    {
        10.0
    } else if (surface == PitchedRoof && height == 46.0 && paris_balloon_site(x, z))
        || (surface == LandmarkRoof09 && height == 22.0 && paris_arc_site(x, z))
        || stadium::legacy_prism(x, z, surface, height)
        || fremont::legacy_prism(x, z, surface, height)
    {
        0.0
    } else {
        height
    }
}

fn paris_balloon_site(x: f32, z: f32) -> bool {
    let dx = x - PARIS_BALLOON[0];
    let dz = z - PARIS_BALLOON[1];
    dx * dx + dz * dz < 13.0 * 13.0
}

fn paris_arc_site(x: f32, z: f32) -> bool {
    (x - PARIS_ARC[0]).abs() < 9.0 && (z - PARIS_ARC[1]).abs() < 17.0
}

pub(crate) fn strip_landmark_clearance(x: f32, z: f32) -> f32 {
    let dx = x - sphere::CENTER[0];
    let dz = z - sphere::CENTER[1];
    if dx * dx + dz * dz <= sphere::RADIUS * sphere::RADIUS {
        return sphere::HEIGHT;
    }
    let strat = strat_clearance(x, z);
    let eiffel = (x - EIFFEL_TOWER[0]).abs() < 46.0 && (z - EIFFEL_TOWER[1]).abs() < 38.0;
    let [wheel_x, wheel_z] = [x - HIGH_ROLLER[0], z - HIGH_ROLLER[1]];
    let wheel = wheel_x.abs() < 18.0 && wheel_z.abs() < 82.0;
    let brace = (14.0..60.0).contains(&wheel_x) && wheel_z.abs() < 4.0;
    if eiffel {
        EIFFEL_TOWER_HEIGHT
    } else if wheel {
        HIGH_ROLLER_HEIGHT
    } else if brace {
        HIGH_ROLLER_HUB
    } else if paris_balloon_site(x, z) {
        PARIS_BALLOON_HEIGHT
    } else if paris_arc_site(x, z) {
        PARIS_ARC_HEIGHT
    } else if stadium::contains(x, z) {
        stadium::HEIGHT
    } else if fremont::contains(x, z) {
        fremont::HEIGHT
    } else {
        strat
    }
}

/// Depth coverage and terrain elevation for visible Strip structures.
pub struct StripLandmarks {
    origin: [f32; 3],
    strat: Option<StratTower>,
    wheel_base: Option<f32>,
    eiffel_base: Option<f32>,
    balloon_base: Option<f32>,
    arc_base: Option<f32>,
    sphere: Option<sphere::Sphere>,
    stadium: Option<stadium::Stadium>,
    fremont: Option<fremont::Canopy>,
    bounds: crate::render::ScreenRect,
}

impl StripLandmarks {
    pub fn from_world(world: &World, mut camera: crate::render::Camera, dimensions: (usize, usize)) -> Option<Self> {
        use crate::render::{ScreenProjection, ScreenRect};

        let (local, origin) = world.local_city(MappedCity::LasVegas)?;
        let world = &local;
        camera.x -= origin[0];
        camera.altitude -= origin[1];
        camera.z -= origin[2];
        let projection = ScreenProjection::new(camera, dimensions)?;
        let mut bounds: Option<ScreenRect> = None;
        let mut visible = |[x, z]: [f32; 2], side: [f32; 2], height, reach| {
            if (camera.x - x).abs().max((camera.z - z).abs()) > reach { return None; }
            let base = world.elevation(x, z);
            let region = projection.bounds([
                [x - side[0], base, z - side[1]], [x + side[0], base + height, z + side[1]],
            ])?;
            bounds = Some(bounds.map_or(region, |previous| previous.union(region)));
            Some(base)
        };
        let strat = visible(STRAT, [25.0, 25.0], STRAT_HEIGHT, STRIP_LANDMARK_REACH)
            .map(|base| StratTower { base });
        let wheel_base = visible(HIGH_ROLLER, [65.0, 84.0], HIGH_ROLLER_HEIGHT, STRIP_LANDMARK_REACH);
        let eiffel_base = visible(EIFFEL_TOWER, [45.0, 38.0], EIFFEL_TOWER_HEIGHT, STRIP_LANDMARK_REACH);
        let balloon_base = visible(PARIS_BALLOON, [12.0, 12.0], PARIS_BALLOON_HEIGHT, STREET_LANDMARK_REACH);
        let arc_base = visible(PARIS_ARC, [9.0, 17.0], PARIS_ARC_HEIGHT, STREET_LANDMARK_REACH);
        let stadium = visible(stadium::CENTER, [160.0, 301.0], stadium::HEIGHT, stadium::REACH)
            .map(|base| stadium::Stadium::new(world, base));
        let fremont = visible(fremont::CENTER, [197.0, 117.0], fremont::HEIGHT + 8.0, fremont::REACH)
            .map(|base| fremont::Canopy::new(world, camera.altitude, base));
        let [x, z] = sphere::CENTER;
        let sphere = if (camera.x - x).abs().max((camera.z - z).abs()) <= sphere::REACH {
            sphere::Sphere::at(world.elevation(x, z), &projection)
        } else {
            None
        };
        if let Some(sphere) = &sphere {
            let region = sphere.bounds();
            bounds = Some(bounds.map_or(region, |previous| previous.union(region)));
        }
        Some(Self { origin, strat, wheel_base, eiffel_base, balloon_base, arc_base, sphere, stadium, fremont, bounds: bounds? })
    }

    pub fn bounds(&self) -> crate::render::ScreenRect { self.bounds }

    pub fn draw(&self, mesh: &mut crate::world_mesh::WorldMesh<'_>, ticks: u64) {
        self.draw_with_phase(mesh, ticks, 0);
    }

    pub fn draw_for_flight(&self, mesh: &mut crate::world_mesh::WorldMesh<'_>, ticks: u64, seed: u64) {
        let first_show = crate::math::mix(seed ^ 0xe805_2416_cd77_9a39) % sphere::SHOWS;
        self.draw_with_phase(mesh, ticks, first_show * sphere::SHOW_TICKS);
    }

    fn draw_with_phase(&self, mesh: &mut crate::world_mesh::WorldMesh<'_>, ticks: u64, phase: u64) {
        mesh.with_origin(self.origin, |mesh| self.draw_local(mesh, ticks, phase));
    }

    fn draw_local(&self, mesh: &mut crate::world_mesh::WorldMesh<'_>, ticks: u64, phase: u64) {
        if let Some(strat) = &self.strat { strat.draw(mesh); }
        if let Some(sphere) = &self.sphere { sphere.draw(mesh, ticks.wrapping_add(phase)); }
        mesh.with_landmark_reach(STRIP_LANDMARK_REACH, |mesh| {
            if let Some(base) = self.wheel_base { draw_high_roller(mesh, base, ticks); }
            if let Some(base) = self.eiffel_base { draw_eiffel_tower(mesh, base); }
        });
        mesh.with_landmark_reach(STREET_LANDMARK_REACH, |mesh| {
            if let Some(base) = self.balloon_base { draw_paris_balloon(mesh, base, ticks); }
            if let Some(base) = self.arc_base { draw_paris_arc(mesh, base); }
        });
        if let Some(stadium) = &self.stadium {
            mesh.with_landmark_reach(stadium::REACH, |mesh| stadium.draw(mesh));
        }
        if let Some(fremont) = &self.fremont {
            mesh.with_landmark_reach(fremont::REACH, |mesh| fremont.draw(mesh, ticks));
        }
    }
}

fn draw_high_roller(mesh: &mut crate::world_mesh::WorldMesh<'_>, base: f32, ticks: u64) {
    use crate::math::sin_cos;
    use core::f32::consts::TAU;

    // Arup's dimensions give a 143 m rim and a 30 m hub. Capsules sit outside the rim.
    const PANELS: usize = 56;
    const REVOLUTION: u64 = crate::flight::TICKS_PER_SECOND as u64 * 30 * 60;
    const STEEL: [u8; 3] = [204, 220, 225];
    const GLASS: [u8; 3] = [64, 131, 157];
    let phase = (ticks % REVOLUTION) as f32 * (TAU / REVOLUTION as f32);
    let light_cycle = (ticks % (9 * crate::flight::TICKS_PER_SECOND as u64)) as f32
        * (TAU / (9 * crate::flight::TICKS_PER_SECOND) as f32);
    let (yaw_sin, yaw_cos) = sin_cos(-0.025);
    let point = |east, up, north| [
        HIGH_ROLLER[0] + east * yaw_cos + north * yaw_sin,
        base + up,
        HIGH_ROLLER[1] + north * yaw_cos - east * yaw_sin,
    ];
    let hub = |east| point(east, HIGH_ROLLER_HUB, 0.0);

    // Two A-frames in the wheel plane and a single east brace outside it.
    for side in [-1.0, 1.0] {
        for end in [-1.0, 1.0] {
            mesh.faded_segment([point(side * 10.0, 1.0, end * 27.0), hub(side * 14.0)], 3.0, STEEL, 1.0);
        }
    }
    mesh.faded_segment([point(57.0, 1.0, 0.0), hub(15.0)], 3.4, STEEL, 1.0);
    mesh.faded_segment([hub(-15.0), hub(15.0)], 5.0, STEEL, 1.0);

    let arc = |sin, cos, radius| point(0.0, HIGH_ROLLER_HUB + sin * radius, cos * radius);
    let mut prior = sin_cos(phase);
    for panel in 1..=PANELS {
        let (sin, cos) = sin_cos(phase + panel as f32 * (TAU / PANELS as f32));
        let rim = arc(sin, cos, HIGH_ROLLER_RIM);
        let chase = sin_cos(light_cycle + panel as f32 * (TAU / PANELS as f32) * 2.0).0;
        let glow = (48.0 + 82.0 * crate::math::smoothstep((chase + 0.25) / 1.25)) as u8;
        let rim_color = [104 + glow, 121 + glow, 230 + glow / 6];
        mesh.luminous_segment([arc(prior.0, prior.1, HIGH_ROLLER_RIM), rim], 2.2, rim_color);
        if panel % 2 == 0 {
            // Every cabin has a distinct outline. The much thinner cables are decimated at this raster scale.
            mesh.faded_segment([hub(15.0), rim], 0.15, STEEL, 0.60);
            mesh.faded_segment([hub(-15.0), rim], 0.15, STEEL, 0.60);
            let [x, y, z] = arc(sin, cos, HIGH_ROLLER_CAPSULE_TRACK);
            mesh.faded_segment([rim, [x, y, z]], 0.8, STEEL, 0.88);
            let belt = [[x - 3.0, y, z], [x, y, z + 3.0], [x + 3.0, y, z], [x, y, z - 3.0]];
            for edge in 0..4 {
                let a = belt[edge];
                let b = belt[(edge + 1) % 4];
                mesh.lit_triangle([[x, y + 3.0, z], a, b], GLASS);
                mesh.lit_triangle([[x, y - 3.0, z], b, a], STEEL);
            }
        }
        prior = (sin, cos);
    }
}

fn draw_eiffel_tower(mesh: &mut crate::world_mesh::WorldMesh<'_>, base: f32) {
    const CORNERS: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
    const STEEL: [u8; 3] = [179, 139, 98];
    const DECK: [u8; 3] = [191, 162, 120];
    // Open lattice geometry follows the replica's flared legs, deck bands and narrowing spire.
    let profile = [
        (0.0, 27.0, 3.4), (13.0, 22.0, 3.2), (24.0, 18.7, 2.8),
        (32.0, 16.0, 2.5), (38.0, 14.4, 2.4), (46.0, 12.4, 2.0),
        (60.0, 10.2, 1.8), (73.0, 8.4, 1.5), (87.0, 7.0, 1.4),
        (101.0, 5.8, 1.2), (114.0, 4.7, 1.1), (127.0, 3.8, 1.0), (137.0, 3.2, 0.9),
    ];
    let (sin, cos) = crate::math::sin_cos(0.76);
    let point = |height: f32, radius: f32, [east, north]: [f32; 2]| {
        [EIFFEL_TOWER[0] + radius * 1.02 * (east * cos - north * sin), base + height,
         EIFFEL_TOWER[1] + radius * 0.86 * (east * sin + north * cos)]
    };
    for face in 0..4 {
        let a = CORNERS[face];
        let b = CORNERS[(face + 1) % 4];
        for pair in profile.windows(2) {
            let [(lo, broad, width), (hi, narrow, _)] = [pair[0], pair[1]];
            mesh.faded_segment([point(lo, broad, a), point(hi, narrow, a)], width, STEEL, 1.0);
            if lo >= 32.0 {
                let bottom = [point(lo, broad, a), point(lo, broad, b)];
                let top = [point(hi, narrow, a), point(hi, narrow, b)];
                mesh.faded_segment(bottom, width * 0.65, STEEL, 0.9);
                mesh.faded_segment([bottom[0], top[1]], width * 0.51, STEEL, 0.9);
                mesh.faded_segment([bottom[1], top[0]], width * 0.51, STEEL, 0.9);
            }
        }

        // The arch leaves the opening between each pair of legs visible.
        let along = |t| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
        let arch = [(13.0, 22.0, 0.0), (25.0, 18.0, 0.19), (30.0, 16.4, 0.5),
                    (25.0, 18.0, 0.81), (13.0, 22.0, 1.0)];
        for pair in arch.windows(2) {
            let ends = [pair[0], pair[1]].map(|(height, radius, t)| point(height, radius, along(t)));
            mesh.faded_segment(ends, 2.2, STEEL, 1.0);
        }
        for (bottom, top, radius) in [(32.0, 36.5, 16.8), (59.0, 62.0, 10.9), (137.0, 144.0, 4.4)] {
            mesh.faded_quad([point(bottom, radius, a), point(bottom, radius, b),
                point(top, radius, b), point(top, radius, a)], DECK, 1.0);
        }
        mesh.faded_segment([point(143.0, 2.9, a), point(156.5, 0.95, a)], 1.1, STEEL, 1.0);
    }
    mesh.faded_segment([point(156.0, 0.0, [0.0, 0.0]), point(EIFFEL_TOWER_HEIGHT, 0.0, [0.0, 0.0])],
        1.15, DECK, 1.0);
}

fn draw_paris_balloon(mesh: &mut crate::world_mesh::WorldMesh<'_>, base: f32, ticks: u64) {
    use crate::math::{sin_cos, smoothstep};
    use core::f32::consts::{FRAC_PI_2, TAU};

    const GOLD: [u8; 3] = [225, 174, 54];
    const RED: [u8; 3] = [202, 46, 52];
    const BLUE: [u8; 3] = [40, 78, 175];
    const BRONZE: [u8; 3] = [166, 128, 62];
    const GONDOLA_DISPLAY: [u8; 3] = [54, 81, 101];
    let daylight = mesh.daylight();
    let neon = smoothstep((0.58 - daylight) / 0.58);
    let period = 8 * u64::from(crate::flight::TICKS_PER_SECOND);
    let phase = (ticks % period) as f32 * TAU / period as f32;
    // UNLV records neon in the six ribs and horizontal bands. This chase is deliberately slow.
    let trim = |angle: f32, height: f32| {
        let (wave, _) = sin_cos(angle - phase + height * 0.08);
        let strength = 0.48 + 0.48 * smoothstep(0.5 + wave * 0.5);
        let ambient = 0.28 + daylight * 0.72;
        let tube = [249_u8, 190, 64];
        core::array::from_fn(|channel| {
            let paint = f32::from(GOLD[channel]) * ambient;
            let glow = f32::from(tube[channel]) * strength;
            ((paint + (glow - paint) * neon) / 0.96).min(255.0) as u8
        })
    };
    let point = |east: f32, height: f32, north: f32| {
        [PARIS_BALLOON[0] + east, base + height, PARIS_BALLOON[1] + north]
    };
    let skin = |angle, height, radius| {
        let (sin, cos) = sin_cos(angle);
        point(radius * cos, height, radius * sin)
    };
    let profile = [
        (23.0, 1.6, GOLD), (25.6, 3.5, BLUE), (28.7, 5.9, RED),
        (29.8, 6.8, GOLD), (31.1, 7.6, BLUE), (38.3, 10.5, BLUE),
        (43.1, PARIS_BALLOON_RADIUS, BLUE), (48.4, 9.5, GOLD), (49.0, 9.1, BLUE),
        (52.4, 6.4, RED), (53.0, 5.7, GOLD), (PARIS_BALLOON_HEIGHT, 0.0, GOLD),
    ];
    let tones = [0.74, 0.79, 0.82, 0.91, 0.93, 1.0, 0.95, 0.84, 0.80, 0.75, 0.70, 0.70];
    for (side, tone) in tones.into_iter().enumerate() {
        let start = side as f32 * TAU / 12.0;
        let end = (side + 1) as f32 * TAU / 12.0;
        for levels in profile.windows(2) {
            let [(lo, lower_radius, color), (hi, upper_radius, _)] = [levels[0], levels[1]];
            let shade = color.map(|channel| (f32::from(channel) * tone) as u8);
            mesh.faded_quad(
                [skin(start, lo, lower_radius), skin(end, lo, lower_radius),
                 skin(end, hi, upper_radius), skin(start, hi, upper_radius)],
                shade, 1.0,
            );
            if side % 2 == 0 && upper_radius > 1.0 {
                let ends = [skin(start, lo, lower_radius + 0.2), skin(start, hi, upper_radius + 0.2)];
                if neon > 0.0 {
                    mesh.luminous_segment(ends, 0.48 + neon * 0.22, trim(start, (lo + hi) * 0.5));
                } else {
                    mesh.faded_segment(ends, 0.48, GOLD, 0.86);
                }
            }
            if neon > 0.0 && matches!(lo, 23.0 | 29.8 | 48.4) {
                let height = (lo + hi) * 0.5;
                let radius = (lower_radius + upper_radius) * 0.5 + 0.16;
                let band = trim((start + end) * 0.5, height);
                mesh.luminous_segment([skin(start, height, radius), skin(end, height, radius)],
                    neon * 0.5, band);
            }
        }
    }

    // The sign is fixed on a pylon. The separate square box carries panel faces and corner ropes.
    landmark_box(mesh, point, [-1.45, 0.0, -1.45], [1.45, 14.5, 1.45], [BRONZE, BRONZE, GOLD]);
    landmark_box(mesh, point, [-4.0, 12.0, -4.0], [4.0, 18.8, 4.0],
        [GONDOLA_DISPLAY, GONDOLA_DISPLAY, BRONZE]);
    for east in [-4.05, 4.05] {
        for north in [-4.05, 4.05] {
            mesh.faded_segment([point(east, 11.9, north), point(east, 19.0, north)], 0.7, GOLD, 1.0);
            mesh.faded_segment([point(east, 18.7, north), point(east * 0.60, 26.0, north * 0.60)],
                0.28, BRONZE, 0.9);
        }
    }

    // Letter strokes wrap around the north and south faces, as in the UNLV sign survey.
    const PARIS: [&[[u8; 4]]; 5] = [
        &[[0, 0, 0, 4], [0, 4, 2, 4], [2, 4, 2, 2], [2, 2, 0, 2]],
        &[[0, 0, 0, 2], [0, 2, 2, 2], [2, 2, 2, 0], [2, 0, 0, 0], [2, 2, 2, 3]],
        &[[0, 0, 0, 3], [0, 2, 1, 3], [1, 3, 2, 3]],
        &[[1, 0, 1, 2], [1, 3, 1, 4]],
        &[[2, 3, 0, 3], [0, 3, 0, 2], [0, 2, 2, 1], [2, 1, 2, 0], [2, 0, 0, 0]],
    ];
    for face in [-FRAC_PI_2, FRAC_PI_2] {
        for (letter, strokes) in PARIS.iter().enumerate() {
            let point = |column: u8, row: u8| {
                let offset = (letter as f32 * 3.0 + f32::from(column) - 7.0) * 0.083;
                let height = 34.0 + f32::from(row) * 0.93;
                let radius = 9.04 + (height - 34.0) * 0.40;
                skin(face + offset, height, radius)
            };
            for &[x0, y0, x1, y1] in *strokes {
                mesh.faded_segment([point(x0, y0), point(x1, y1)], 0.76, [245, 71, 63], 1.0);
            }
        }
    }
}

fn draw_paris_arc(mesh: &mut crate::world_mesh::WorldMesh<'_>, base: f32) {
    use core::f32::consts::PI;

    const STONE: [u8; 3] = [215, 196, 164];
    const SIDE: [u8; 3] = [189, 174, 150];
    const CAP: [u8; 3] = [229, 215, 190];
    const RECESS: [u8; 3] = [158, 143, 122];
    const SPRING: f32 = 9.4;
    const SPANDREL: f32 = 21.9;
    let (sin, cos) = crate::math::sin_cos(-0.018);
    let point = |along: f32, height: f32, depth: f32| [
        PARIS_ARC[0] + depth * cos + along * sin,
        base + height,
        PARIS_ARC[1] + along * cos - depth * sin,
    ];
    let block = |mesh: &mut crate::world_mesh::WorldMesh<'_>, width: f32, lo: f32, hi: f32, depth: f32, colors| {
        landmark_box(mesh, point, [-width, lo, -depth], [width, hi, depth], colors);
    };
    let [width, depth, open] = [PARIS_ARC_HALF_WIDTH, PARIS_ARC_HALF_DEPTH, PARIS_ARC_OPENING];
    for (left, right) in [(-width + 0.4, -open), (open, width - 0.4)] {
        landmark_box(mesh, point, [left, 0.0, -depth + 0.38], [right, SPANDREL, depth - 0.38],
            [STONE, SIDE, CAP]);
        landmark_box(mesh, point, [left - 0.26, 0.0, -depth + 0.1], [right + 0.26, 1.55, depth - 0.1],
            [SIDE, SIDE, CAP]);
    }

    for panel in 0..8 {
        let arc_point = |index: usize, side: f32| {
            let (sin, cos) = crate::math::sin_cos(PI * index as f32 / 8.0);
            point(open * cos, SPRING + open * sin, side)
        };
        let front = -depth + 0.38;
        let back = depth - 0.38;
        mesh.faded_quad(
            [arc_point(panel, front), arc_point(panel + 1, front),
             arc_point(panel + 1, back), arc_point(panel, back)],
            RECESS, 1.0,
        );
        for face in [front, back] {
            let a = arc_point(panel, face);
            let b = arc_point(panel + 1, face);
            let (a_sin, a_cos) = crate::math::sin_cos(PI * panel as f32 / 8.0);
            let (b_sin, b_cos) = crate::math::sin_cos(PI * (panel + 1) as f32 / 8.0);
            mesh.faded_quad(
                [a, b, point(open * b_cos, SPANDREL, face), point(open * a_cos, SPANDREL, face)],
                STONE, 1.0,
            );
            let trim_face = face * 1.012;
            mesh.faded_segment(
                [point((open + 0.65) * a_cos, SPRING + (open + 0.65) * a_sin, trim_face),
                 point((open + 0.65) * b_cos, SPRING + (open + 0.65) * b_sin, trim_face)],
                1.05, CAP, 1.0,
            );
        }
    }

    block(mesh, width, 21.9, 23.0, depth, [CAP, STONE, CAP]);
    block(mesh, width - 0.38, 23.0, 27.4, depth - 0.45, [STONE, SIDE, CAP]);
    block(mesh, width, 27.4, 28.8, depth, [CAP, STONE, CAP]);
    block(mesh, width - 0.46, 28.8, PARIS_ARC_HEIGHT, depth - 0.49, [STONE, SIDE, CAP]);
    for face in [-1.0, 1.0] {
        let facade = face * (depth - 0.32);
        for center in [-10.25, 10.25] {
            mesh.faded_quad(
                [point(center - 2.3, 3.8, facade), point(center + 2.3, 3.8, facade),
                 point(center + 2.0, 10.4, facade), point(center - 2.0, 10.4, facade)],
                SIDE, 1.0,
            );
            mesh.triangle(
                [point(center - 1.7, 4.6, facade + face * 0.12), point(center + 1.9, 4.6, facade + face * 0.12),
                 point(center + 0.45, 9.55, facade + face * 0.12)],
                CAP,
            );
        }
    }
}

fn landmark_box(
    mesh: &mut crate::world_mesh::WorldMesh<'_>,
    point: impl Fn(f32, f32, f32) -> [f32; 3],
    [left, bottom, front]: [f32; 3],
    [right, top, back]: [f32; 3],
    colors: [[u8; 3]; 3],
) {
    let face = [
        point(left, bottom, front), point(right, bottom, front),
        point(right, top, front), point(left, top, front),
        point(left, bottom, back), point(right, bottom, back),
        point(right, top, back), point(left, top, back),
    ];
    for (indices, color) in [
        ([0, 1, 2, 3], colors[0]), ([4, 5, 6, 7], colors[0]),
        ([0, 3, 7, 4], colors[1]), ([1, 2, 6, 5], colors[1]),
        ([2, 3, 7, 6], colors[2]), ([0, 1, 5, 4], colors[1]),
    ] {
        mesh.faded_quad(indices.map(|index| face[index]), color, 1.0);
    }
}

#[cfg(test)]
mod landmark_tests {
    use super::*;

    #[test]
    fn paris_lattice_only_replaces_the_original_tower_prism() {
        let [x, z] = EIFFEL_TOWER;
        assert_eq!(strip_landmark_terrain_height(x, z, LandmarkRoof11, 165.0), 10.0);
        assert_eq!(strip_landmark_terrain_height(x, z, LandmarkRoof11, 48.0), 48.0);
        assert_eq!(strip_landmark_terrain_height(x + 50.0, z, LandmarkRoof11, 165.0), 165.0);
        assert_eq!(strip_landmark_clearance(x, z), EIFFEL_TOWER_HEIGHT);
    }

    #[test]
    fn paris_balloon_chase_changes_night_frames_only() {
        use crate::{render::Camera, sky::Sky, world_mesh::WorldMesh};

        const SIDE: usize = 120;
        let camera = Camera {
            x: PARIS_BALLOON[0] - 85.0,
            z: PARIS_BALLOON[1],
            altitude: 132.0,
            heading: core::f32::consts::FRAC_PI_2,
            pitch: 0.0,
            bank: 0.0,
        };
        let frame = |day_fraction, ticks| {
            let mut pixels = [0x2367; SIDE * SIDE];
            let mut depth = [u16::MAX; SIDE * SIDE];
            let sky = Sky::from_time(1, 0, day_fraction);
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
            draw_paris_balloon(&mut mesh, 100.0, ticks);
            pixels
        };
        let quarter_cycle = 2 * u64::from(crate::flight::TICKS_PER_SECOND);
        assert!(frame(0.5, 0) == frame(0.5, quarter_cycle));
        assert!(frame(0.0, 0) != frame(0.0, quarter_cycle));
    }

    #[test]
    fn paris_street_models_keep_open_passages_in_older_packs() {
        let [x, z] = PARIS_ARC;
        assert_eq!(strip_landmark_terrain_height(x, z, LandmarkRoof09, 22.0), 0.0);
        assert_eq!(strip_landmark_terrain_height(x, z, LandmarkRoof09, 10.0), 10.0);
        assert_eq!(strip_landmark_clearance(x, z), PARIS_ARC_HEIGHT);
        let [x, z] = PARIS_BALLOON;
        assert_eq!(strip_landmark_terrain_height(x, z, PitchedRoof, 46.0), 0.0);
        assert_eq!(strip_landmark_terrain_height(x + 15.0, z, PitchedRoof, 46.0), 46.0);
        assert_eq!(strip_landmark_clearance(x, z), PARIS_BALLOON_HEIGHT);
    }

    #[test]
    fn paris_arc_renders_open_space_between_its_piers() {
        use crate::{render::Camera, sky::Sky, world_mesh::WorldMesh};

        const SIDE: usize = 144;
        const BACKGROUND: u16 = 0x2367;
        let mut pixels = [BACKGROUND; SIDE * SIDE];
        let mut depth = [u16::MAX; SIDE * SIDE];
        let sky = Sky::from_time(1, 0, 0.5);
        let camera = Camera {
            x: PARIS_ARC[0] - 58.0,
            z: PARIS_ARC[1],
            altitude: 110.0,
            heading: core::f32::consts::FRAC_PI_2,
            pitch: 0.0,
            bank: 0.0,
        };
        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
        draw_paris_arc(&mut mesh, 100.0);
        assert_eq!(pixels[72 * SIDE + 72], BACKGROUND);
        assert_ne!(pixels[72 * SIDE + 54], BACKGROUND);
        assert_ne!(pixels[72 * SIDE + 90], BACKGROUND);
        assert_ne!(pixels[50 * SIDE + 72], BACKGROUND);
    }

    #[test]
    fn high_roller_keeps_an_open_silhouette_and_rotates() {
        use crate::{render::Camera, sky::Sky, world_mesh::WorldMesh};

        const SIDE: usize = 144;
        const BACKGROUND: u16 = 0x2367;
        let sky = Sky::from_time(1, 0, 0.5);
        let dimensions = (SIDE, SIDE);
        let camera = Camera {
            x: HIGH_ROLLER[0] - 240.0,
            z: HIGH_ROLLER[1],
            altitude: 100.0 + HIGH_ROLLER_HUB,
            heading: core::f32::consts::FRAC_PI_2,
            pitch: 0.0,
            bank: 0.0,
        };
        let frame = |ticks| {
            let mut pixels = [BACKGROUND; SIDE * SIDE];
            let mut depth = [u16::MAX; SIDE * SIDE];
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, dimensions, camera, &sky).unwrap();
            draw_high_roller(&mut mesh, 100.0, ticks);
            pixels
        };
        let first = frame(0);
        let changed = first.iter().filter(|&&pixel| pixel != BACKGROUND).count();
        assert!(changed > 240, "wheel and spokes should render as a visible structure");
        assert!(first[56 * SIDE + 64..56 * SIDE + 80].contains(&BACKGROUND));
        assert!(first != frame(crate::flight::TICKS_PER_SECOND as u64 * 225));
    }
}
