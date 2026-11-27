//! Georeferenced terrain and scenery sampled at one world unit per metre.

use crate::world::{Sample, Surface, World};
use Surface::*;

pub(crate) const DEPARTURE: (f32, f32, f32) = (-52.0, -2_370.0, 0.02);
pub(crate) fn departure(area: u32) -> ((f32, f32, f32), usize) {
    match area {
        1 => ((2_730.0, -4_470.0, 0.0), 12),
        2 => ((2_170.0, 5_250.0, 0.23), 3),
        3 => ((500.0, 660.0, 0.63), 1),
        _ => (DEPARTURE, 0),
    }
}

pub(crate) const TOUR: [[f32; 2]; 14] = [
    [-192.0, 141.0], [810.0, 1_142.0], [1_326.0, 3_632.0], [2_507.0, 6_407.0],
    [8_104.0, 5_938.0], [15_879.0, -1_929.0], [36_373.0, 893.0],
    [39_445.0, -9_593.0], [13_918.0, -8_217.0], [-1_603.0, -4_857.0],
    [-23_838.0, 3_679.0], [-14_748.0, 5_173.0], [2_730.0, -2_260.0], [820.0, -1_680.0],
];

const STRAT: [f32; 2] = [1_516.0, 3_958.0];
pub(crate) const STRAT_HEIGHT: f32 = 350.2;
const HEADER: usize = 64;
const TILE_SIDE: usize = 16;
const TILE_BYTES: usize = TILE_SIDE * TILE_SIDE * 2;

#[derive(Clone, Copy)]
pub struct LasVegasData {
    bytes: &'static [u8],
    ground_origin: [i32; 2],
    ground_cell: i32,
    ground_size: [usize; 2],
    feature_origin: [i32; 2],
    feature_cell: i32,
    feature_size: [usize; 2],
    ground_water: usize,
    feature_index: usize,
    tiles: usize,
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
        if bytes.len() < HEADER || &bytes[..4] != b"LVS1" { return None; }
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
        let ground_water = HEADER.checked_add(ground_size[0].checked_mul(ground_size[1])?.checked_mul(2)?)?;
        let feature_index = ground_water.checked_add(ground_size[0].checked_mul(ground_size[1])?)?;
        let tiles = feature_index.checked_add(feature_size[0].checked_mul(feature_size[1])?.checked_mul(4)?)?;
        if tiles.checked_add(tile_count.checked_mul(TILE_BYTES)?)? != bytes.len() { return None; }
        Some(Self {
            bytes, ground_origin: [header(0), header(1)], ground_cell, ground_size,
            feature_origin: [header(5), header(6)], feature_cell, feature_size,
            ground_water, feature_index, tiles, tile_count, base_metres: header(13),
        })
    }

    fn u16(self, offset: usize) -> u16 {
        u16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]])
    }

    pub(crate) fn terrain(self, world: &World, x: f32, z: f32) -> (Sample, f32) {
        let gx = (x - self.ground_origin[0] as f32) / self.ground_cell as f32 - 0.5;
        let gz = (self.ground_origin[1] as f32 - z) / self.ground_cell as f32 - 0.5;
        let [w, h] = self.ground_size;
        if gx < 0.0 || gz < 0.0 || gx >= (w - 1) as f32 || gz >= (h - 1) as f32 {
            return (Sample { elevation: 260.0, surface: Mojave }, 0.55);
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
        let water = self.bytes[self.ground_water + nearest] != 0;
        let noise = world.noise(x, z, 0.0074, 0xb652_4309);
        let west_range = x < -17_000.0 && z > -6_500.0 && z < 8_000.0;
        let surface = if water {
            TarnWater
        } else if west_range && elevation > 660.0 && (top - bottom).abs() > 8.0 {
            if (elevation as i32 / 44) % 3 == 0 { MesaCap } else { MesaCliff }
        } else if elevation > 1_290.0 {
            Rock
        } else if noise < 0.27 && (top - bottom).abs() > 10.0 {
            ScreeFine
        } else {
            Mojave
        };
        (Sample { elevation, surface }, if water { -0.14 } else { 0.52 })
    }

    pub(crate) fn detail(self, x: f32, z: f32, base: Sample) -> Sample {
        let cell_x = (x - self.feature_origin[0] as f32) / self.feature_cell as f32;
        let cell_z = (z - self.feature_origin[1] as f32) / self.feature_cell as f32;
        let [w, h] = self.feature_size;
        if cell_x < 0.0 || cell_z < 0.0 || cell_x >= (w * TILE_SIDE) as f32 || cell_z >= (h * TILE_SIDE) as f32 {
            return base;
        }
        let [x, z] = [cell_x as usize, cell_z as usize];
        let location = (z / TILE_SIDE) * w + (x / TILE_SIDE);
        let off = self.feature_index + location * 4;
        let tile = u32::from_le_bytes(self.bytes[off..off + 4].try_into().unwrap()) as usize;
        if !(1..=self.tile_count).contains(&tile) { return base; }
        let cell = (z % TILE_SIDE) * TILE_SIDE + (x % TILE_SIDE);
        let word = self.u16(self.tiles + (tile - 1) * TILE_BYTES + cell * 2);
        let surface = match word & 127 {
            1 | 6 => Pavement,
            2 => TarnWater,
            3 => Gravel,
            5 => RunwayMarking,
            7 => Park,
            8 => RailMetal,
            10 => SphereShell,
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
            _ => return base,
        };
        Sample { surface, elevation: base.elevation + (word >> 7) as f32 }
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

pub struct StratTower {
    base: f32,
}

impl StratTower {
    pub fn from_world(world: &World) -> Option<Self> {
        world.is_las_vegas().then(|| Self { base: world.elevation(STRAT[0], STRAT[1]) })
    }

    pub fn bounds(
        &self, camera: crate::render::Camera, dimensions: (usize, usize),
    ) -> Option<crate::render::ScreenRect> {
        let [x, z] = STRAT;
        if (camera.x - x).abs().max((camera.z - z).abs()) > 3_200.0 { return None; }
        crate::render::ScreenProjection::new(camera, dimensions)?.bounds([
            [x - 25.0, self.base, z - 25.0],
            [x + 25.0, self.base + STRAT_HEIGHT, z + 25.0],
        ])
    }

    pub fn draw(&self, mesh: &mut crate::world_mesh::WorldMesh<'_>) {
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
