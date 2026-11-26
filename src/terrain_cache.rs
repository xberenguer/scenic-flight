//! World-aligned terrain grids that replenish only newly visible rows.

use crate::math::smoothstep;
use crate::world::{Sample, Surface, World};

const SIDE: usize = 64;
const NEAR_STEP: f32 = 4.0;
const FAR_STEP: f32 = 20.0;
const CRUISE_STEP: f32 = 36.0;
const CRUISE_NEAR_STEP: f32 = 10.0;

const MATERIALS: [Surface; 59] = [
    Surface::DeepWater,
    Surface::Shallows,
    Surface::Sand,
    Surface::Grass,
    Surface::Forest,
    Surface::Rock,
    Surface::Snow,
    Surface::Desert,
    Surface::Pavement,
    Surface::Gravel,
    Surface::Park,
    Surface::TreeCanopy,
    Surface::FieldGreen,
    Surface::FieldGold,
    Surface::FieldSoil,
    Surface::PitchedRoof,
    Surface::FlatRoof,
    Surface::TowerRoof,
    Surface::ShipHull,
    Surface::ShipDeck,
    Surface::ShipCabin,
    Surface::ShipCargo,
    Surface::HarborCrane,
    Surface::CargoOchre,
    Surface::CargoBlue,
    Surface::RiverWater,
    Surface::WetMeadow,
    Surface::GrassRunway,
    Surface::RunwayMarking,
    Surface::RunwayLight,
    Surface::TarnWater,
    Surface::TarnShallows,
    Surface::AlpineMeadow,
    Surface::SaltCrust,
    Surface::DuneSand,
    Surface::ReedMarsh,
    Surface::GlacierIce,
    Surface::GlacierCrevasse,
    Surface::GlacierMoraine,
    Surface::GlacierMeltwater,
    Surface::SolarGlass,
    Surface::Boardwalk,
    Surface::RiverSilt,
    Surface::ScreeEdge,
    Surface::ScreeFine,
    Surface::ScreeCoarse,
    Surface::DryWash,
    Surface::SpruceCanopy,
    Surface::AspenCanopy,
    Surface::MapleCanopy,
    Surface::JuniperCanopy,
    Surface::RailTimber,
    Surface::RailMetal,
    Surface::RailGlass,
    Surface::RailRust,
    Surface::RailApron,
    Surface::IndustryBrick,
    Surface::IndustryConcrete,
    Surface::IndustryMetal,
];
const _: () = {
    let mut index = 0;
    while index < MATERIALS.len() {
        assert!(MATERIALS[index] as usize == index);
        index += 1;
    }
};

#[derive(Clone, Copy)]
struct Cell(u32);

impl Cell {
    const SEA: u32 = 1 << 31;
    const EMPTY: Self = Self(((-127_i8 as u8 as u32) << 24) | ((Surface::DeepWater as u32) << 16));

    fn quantized(sample: Sample, coast: f32) -> Self {
        debug_assert!((sample.surface as usize) < MATERIALS.len());
        let shore = (coast * 118.0) as i8;
        let coast = if coast <= 0.0 { shore.min(-1) } else { shore.max(1) };
        let h = (sample.elevation * 128.0 + 0.5) as u16;
        Self(u32::from(h) | ((sample.surface as u32) << 16) | ((coast as u8 as u32) << 24))
    }

    fn height_units(self) -> f32 {
        f32::from(self.0 as u16)
    }

    fn is_water(self) -> bool {
        self.0 & Self::SEA != 0
    }

    fn surface(self) -> Surface {
        MATERIALS[((self.0 >> 16) & 255) as usize]
    }

    fn signed_height(self) -> f32 {
        if self.is_water() {
            f32::from((self.0 >> 24) as i8) * (19.0 / 118.0)
        } else {
            self.height_units() * (1.0 / 128.0)
        }
    }
}

struct Grid {
    cells: [[Cell; SIDE]; SIDE],
    min_x: i32,
    min_z: i32,
    spacing: f32,
    inverse_spacing: f32,
}

impl Grid {
    const fn new(spacing: f32) -> Self {
        Self {
            cells: [[Cell::EMPTY; SIDE]; SIDE],
            min_x: 0,
            min_z: 0,
            spacing,
            inverse_spacing: 1.0 / spacing,
        }
    }

    fn center(&self, x: f32, z: f32) -> (i32, i32) {
        let half = SIDE as i32 / 2;
        (floor_to_cell(x * self.inverse_spacing) - half, floor_to_cell(z * self.inverse_spacing) - half)
    }

    fn fill_cell(&mut self, world: &World, x: i32, z: i32) {
        let (sample, coast) = world.sample_and_shore(x as f32 * self.spacing, z as f32 * self.spacing);
        self.cells[(z as usize) & (SIDE - 1)][(x as usize) & (SIDE - 1)] =
            Cell::quantized(sample, coast);
    }

    fn fill(&mut self, world: &World, x: i32, z: i32) {
        for gz in z..z + SIDE as i32 {
            for gx in x..x + SIDE as i32 {
                self.fill_cell(world, gx, gz);
            }
        }
        self.min_x = x;
        self.min_z = z;
    }

    fn follow(&mut self, world: &World, x: i32, z: i32) {
        let size = SIDE as i32;
        let dx = x - self.min_x;
        let dz = z - self.min_z;
        if dx == 0 && dz == 0 {
            return;
        }
        if dx.abs() >= size || dz.abs() >= size {
            self.fill(world, x, z);
            return;
        }

        let (first_z, last_z) = if dz >= 0 { (self.min_z + size, z + size) } else { (z, self.min_z) };
        for gz in first_z..last_z {
            for gx in x..x + size {
                self.fill_cell(world, gx, gz);
            }
        }
        let (first_x, last_x) = if dx >= 0 { (self.min_x + size, x + size) } else { (x, self.min_x) };
        for gz in z.max(self.min_z)..(z + size).min(self.min_z + size) {
            for gx in first_x..last_x {
                self.fill_cell(world, gx, gz);
            }
        }
        self.min_x = x;
        self.min_z = z;
    }

    fn sample<const CHECK_BOUNDS: bool>(&self, x: f32, z: f32) -> Option<Sample> {
        let grid_x = x * self.inverse_spacing;
        let grid_z = z * self.inverse_spacing;
        let ix = floor_to_cell(grid_x);
        let iz = floor_to_cell(grid_z);
        let last_x = self.min_x + SIDE as i32 - 1;
        let last_z = self.min_z + SIDE as i32 - 1;
        if CHECK_BOUNDS && (ix < self.min_x || iz < self.min_z || ix >= last_x || iz >= last_z) {
            return None;
        }
        let xf = grid_x - ix as f32;
        let zf = grid_z - iz as f32;
        let left = ix as usize & (SIDE - 1);
        let right = (ix + 1) as usize & (SIDE - 1);
        let near = &self.cells[iz as usize & (SIDE - 1)];
        let far = &self.cells[(iz + 1) as usize & (SIDE - 1)];
        let cells = [near[left], near[right], far[left], far[right]];
        let [a, b, c, d] = cells;
        let (west, east) = if zf >= 0.5 { (c, d) } else { (a, b) };
        let closest = if xf >= 0.5 { east } else { west };
        if (a.0 & b.0 & c.0 & d.0) & Cell::SEA != 0 {
            return Some(Sample { elevation: 0.0, surface: closest.surface() });
        }
        if (a.0 | b.0 | c.0 | d.0) & Cell::SEA != 0 {
            let [a, b, c, d] = cells.map(Cell::signed_height);
            let near = a + (b - a) * xf;
            let far = c + (d - c) * xf;
            let elevation = near + (far - near) * zf;
            if elevation <= 0.0 {
                let surface = if elevation > -2.3 { Surface::Shallows } else { Surface::DeepWater };
                return Some(Sample { elevation: 0.0, surface });
            }
            let surface = if elevation < 2.5 {
                Surface::Sand
            } else if closest.is_water() {
                cells.iter().find(|cell| !cell.is_water()).map_or(Surface::Grass, |cell| cell.surface())
            } else {
                closest.surface()
            };
            return Some(Sample { elevation, surface });
        }
        // All four heights are exactly representable as integer units. Apply
        // the binary scale once, after the same sequence of interpolations.
        let [a, b, c, d] = cells.map(Cell::height_units);
        let near = a + (b - a) * xf;
        let far = c + (d - c) * xf;
        Some(Sample { elevation: (near + (far - near) * zf) * (1.0 / 128.0), surface: closest.surface() })
    }
}

fn floor_to_cell(value: f32) -> i32 {
    let cell = value as i32;
    cell - i32::from(value < cell as f32)
}

/// Use one instance per active view and reset it for each new world.
/// Distant terrain uses a coarser grid to keep CPU work bounded.
pub struct TerrainCache {
    world: World,
    view_x: f32,
    view_z: f32,
    blend_start: f32,
    blend_end: f32,
    blend_scale: f32,
    near: Grid,
    far: Grid,
    ready: bool,
}

impl TerrainCache {
    pub const fn new() -> Self {
        Self {
            world: World::new(0),
            view_x: 0.0,
            view_z: 0.0,
            blend_start: 78.0,
            blend_end: 111.0,
            blend_scale: 1.0 / 33.0,
            near: Grid::new(NEAR_STEP),
            far: Grid::new(FAR_STEP),
            ready: false,
        }
    }

    pub fn reset(&mut self, world: World, x: f32, z: f32) {
        self.reset_for_altitude(world, x, z, 0.0);
    }

    #[inline(never)]
    pub fn reset_for_altitude(&mut self, world: World, x: f32, z: f32, altitude: f32) {
        self.world = world;
        let high = altitude > 259.0;
        self.far.spacing = if high { CRUISE_STEP } else { FAR_STEP };
        self.far.inverse_spacing = 1.0 / self.far.spacing;
        self.near.spacing = if high { CRUISE_NEAR_STEP } else { NEAR_STEP };
        self.near.inverse_spacing = 1.0 / self.near.spacing;
        self.blend_start = self.near.spacing * 19.5;
        self.blend_end = self.near.spacing * 27.75;
        self.blend_scale = 1.0 / (self.blend_end - self.blend_start);
        let near = self.near.center(x, z);
        let far = self.far.center(x, z);
        self.near.fill(&self.world, near.0, near.1);
        self.far.fill(&self.world, far.0, far.1);
        self.view_x = x;
        self.view_z = z;
        self.ready = true;
    }

    #[inline(never)]
    pub fn follow(&mut self, x: f32, z: f32) {
        if !self.ready {
            self.reset(self.world, x, z);
            return;
        }
        let near = self.near.center(x, z);
        let far = self.far.center(x, z);
        self.near.follow(&self.world, near.0, near.1);
        self.far.follow(&self.world, far.0, far.1);
        self.view_x = x;
        self.view_z = z;
    }

    pub fn sample(&self, x: f32, z: f32) -> Sample {
        if !self.ready {
            return self.world.sample(x, z);
        }
        let distance = (x - self.view_x).abs().max((z - self.view_z).abs());
        // The near blend ends less than 28 cells from the center of a 64-cell grid.
        let near = if distance < self.blend_end { self.near.sample::<false>(x, z) } else { None };
        if let Some(local) = near {
            if distance <= self.blend_start {
                return local;
            }
        }
        let far = self.far.sample::<true>(x, z).unwrap_or_else(|| self.world.sample(x, z));
        match near {
            Some(local) => {
                let blend = smoothstep((distance - self.blend_start) * self.blend_scale);
                Sample {
                    elevation: local.elevation + (far.elevation - local.elevation) * blend,
                    surface: if blend < 0.5 { local.surface } else { far.surface },
                }
            }
            None => far,
        }
    }
}

impl Default for TerrainCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepped_grids_agree_with_fresh_grids_along_signed_paths() {
        let world = World::new(9762);
        let mut streaming = TerrainCache::new();
        let mut reference = TerrainCache::new();
        streaming.reset(world, 6.0, 7.0);
        for (x, z) in [(5.0, -14.0), (-29.0, 79.0), (31.0, -188.0), (-11.4, 28.9), (1315.0, -814.0)] {
            streaming.follow(x, z);
            reference.reset(world, x, z);
            for dz in -48..=48 {
                for dx in -48..=48 {
                    let sx = x + dx as f32 * 12.4;
                    let sz = z + dz as f32 * 11.7;
                    assert_eq!(streaming.sample(sx, sz), reference.sample(sx, sz), "{sx} {sz}");
                }
            }
        }
    }

    #[test]
    fn aligned_near_cells_reconstruct_height_and_material() {
        let world = World::new(31);
        let mut cache = TerrainCache::new();
        cache.reset(world, 50.0, -50.0);
        for z in -24..=-2 {
            for x in 2..24 {
                let x = x as f32 * NEAR_STEP;
                let z = z as f32 * NEAR_STEP;
                let exact = world.sample(x, z);
                let cached = cache.sample(x, z);
                assert_eq!(exact.surface, cached.surface);
                assert!((exact.elevation - cached.elevation).abs() <= 1.0 / 256.0);
            }
        }
    }
}
