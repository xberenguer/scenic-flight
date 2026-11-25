//! Original fixed footprints used by seeded coastal landscape plans.
//! Props may move on the beach while those plans retain their site selection.

use super::*;
use core::f32::consts::FRAC_1_SQRT_2;

impl BeachSite {
    pub(crate) fn reserved_footprint(world: &World, seed: u64, tile_x: i32, tile_z: i32) -> Option<Self> {
        const LIMIT: i32 = 123_000;
        if !(-LIMIT..=LIMIT).contains(&tile_x) || !(-LIMIT..=LIMIT).contains(&tile_z) {
            return None;
        }
        let tile_key = (u64::from(tile_x as u32) << 32) | u64::from(tile_z as u32);
        let key = mix(seed ^ tile_key ^ 0x62b2_b84f_bd31_73a7);
        if key & 15 >= 5 {
            return None;
        }
        let origin_x = (tile_x * BEACH_TILE_SIZE) as f32;
        let origin_z = (tile_z * BEACH_TILE_SIZE) as f32;
        let jitter_x = fraction(mix(key ^ 632)) * 7.0;
        let jitter_z = fraction(mix(key ^ 371)) * 7.0;
        const GRID: u32 = 7;
        let mut cell = ((key >> 31) as u32) % (GRID * GRID);
        for index in 0..GRID * GRID {
            let x = origin_x + 15.0 + jitter_x + 15.0 * (cell % GRID) as f32;
            let z = origin_z + 15.0 + jitter_z + 15.0 * (cell / GRID) as f32;
            // A coprime stride visits every candidate in the patch exactly once.
            cell = (cell + 19) % (GRID * GRID);
            let sand = world.sample(x, z);
            if sand.surface != Surface::Sand || sand.elevation < SEA_LEVEL + 0.77 {
                continue;
            }
            let design = mix(key ^ (u64::from(index) << 25));
            let Some(shore) = FixedShore::find(world, x, z, sand.elevation, design) else {
                continue;
            };
            let mut site = Self {
                bounds: [x, z, x, z],
                items: [Piece::EMPTY; PIECES],
                count: 0,
            };
            if site.arrange_reservation(world, shore, design) {
                return Some(site);
            }
        }
        None
    }

    fn arrange_reservation(&mut self, world: &World, shore: FixedShore, key: u64) -> bool {
        if !self.add_reservation(world, &shore, 0.0, 0.0, true, key) {
            return false;
        }
        let side = if key & 16 == 0 { 1.0 } else { -1.0 };
        let fabric = mix(key ^ 467);
        if !self.add_reservation(world, &shore, -1.2, 4.1 * side, false, fabric)
            && !self.add_reservation(world, &shore, -1.7, -4.1 * side, false, fabric)
            && !self.add_reservation(world, &shore, -4.15, 0.0, false, fabric)
        {
            return false;
        }
        if !self.add_reservation(world, &shore, 0.3, -7.7 * side, true, mix(key ^ 71)) {
            self.add_reservation(world, &shore, -0.9, -7.0 * side, true, mix(key ^ 939));
        }
        if key & 3 != 3 {
            self.add_reservation(world, &shore, -1.8, 8.7 * side, true, mix(key ^ 212));
        }
        if key & 32 != 0 {
            self.add_reservation(world, &shore, -3.8, -4.8 * side, false, mix(key ^ 843));
        }
        if key & 64 != 0 {
            self.add_reservation(world, &shore, -2.15, -11.2 * side, false, mix(key ^ 375));
        }
        if key & 128 != 0 {
            self.add_reservation(world, &shore, 1.25, 12.3 * side, false, mix(key ^ 867));
        }
        true
    }

    fn add_reservation(
        &mut self,
        world: &World,
        shore: &FixedShore,
        seaward: f32,
        across: f32,
        umbrella: bool,
        key: u64,
    ) -> bool {
        if self.count as usize == PIECES {
            return false;
        }
        let x = shore.x + shore.fx * seaward - shore.fz * across;
        let z = shore.z + shore.fz * seaward + shore.fx * across;
        let (sin, cos) = sin_cos(fraction(mix(key)) * TAU);
        let mut piece = Piece {
            x,
            z,
            sin,
            cos,
            umbrella,
            ground: SEA_LEVEL,
            fabric: Fabric::from_key(key),
            trim: key & 128 == 0,
        };
        let radius = piece.extent();
        for other in self.items() {
            let dx = x - other.x;
            let dz = z - other.z;
            let gap = radius + other.extent() - 0.4;
            if dx * dx + dz * dz < gap * gap {
                return false;
            }
        }
        let Some(ground) = original_sand_pad(world, x, z, radius) else {
            return false;
        };
        piece.ground = ground;
        self.bounds[0] = self.bounds[0].min(x - radius);
        self.bounds[1] = self.bounds[1].min(z - radius);
        self.bounds[2] = self.bounds[2].max(x + radius);
        self.bounds[3] = self.bounds[3].max(z + radius);
        self.items[self.count as usize] = piece;
        self.count += 1;
        true
    }
}

struct FixedShore {
    x: f32,
    z: f32,
    fx: f32,
    fz: f32,
}

impl FixedShore {
    fn find(world: &World, x: f32, z: f32, height: f32, key: u64) -> Option<Self> {
        const DIR: [(f32, f32); 8] = [
            (1.0, 0.0),
            (FRAC_1_SQRT_2, FRAC_1_SQRT_2),
            (0.0, 1.0),
            (-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
            (-1.0, 0.0),
            (-FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
            (0.0, -1.0),
            (FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
        ];
        let rotation = (key >> 38) as usize & 7;
        for i in 0..DIR.len() {
            let (fx, fz) = DIR[(i + rotation) & 7];
            let shore = Self { x, z, fx, fz };
            if world.coast_signal(x + fx * 49.0, z + fz * 49.0) >= -0.008 {
                continue;
            }
            if shore.walkable(world, height) && shore.open_water(world) {
                return Some(shore);
            }
        }
        None
    }

    fn walkable(&self, world: &World, height: f32) -> bool {
        let mut previous = height;
        for d in [4.0, 9.0, 15.0, 22.0] {
            let x = self.x - d * self.fx;
            let z = self.z - d * self.fz;
            let ground = world.elevation(x, z);
            if ground <= SEA_LEVEL + 0.60
                || ground < height - 0.30
                || (ground - previous).abs() > 0.8
            {
                return false;
            }
            previous = ground;
        }
        true
    }

    fn open_water(&self, world: &World) -> bool {
        let coast = |along: f32, cross: f32| {
            world.coast_signal(
                self.x + self.fx * along - self.fz * cross,
                self.z + self.fz * along + self.fx * cross,
            )
        };
        let mut land = coast(4.0, 0.0);
        if land <= 0.0 {
            return false;
        }
        for distance in [10.0, 17.0, 24.0, 31.0, 38.0, 45.0, 52.0] {
            let next = coast(distance, 0.0);
            if next > land + 0.018 {
                return false;
            }
            if next <= 0.0 {
                return next > -0.105
                    && coast(distance + 15.0, 0.0) < -0.018
                    && coast(distance + 19.0, -10.0) < -0.015
                    && coast(distance + 19.0, 10.0) < -0.015;
            }
            land = next;
        }
        false
    }
}

fn original_sand_pad(world: &World, x: f32, z: f32, radius: f32) -> Option<f32> {
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for dz in [-1.0, 0.0, 1.0] {
        for dx in [-1.0, 0.0, 1.0] {
            let ground = world.sample(x + dx * radius, z + dz * radius);
            if ground.surface != Surface::Sand || ground.elevation < SEA_LEVEL + 0.66 {
                return None;
            }
            low = low.min(ground.elevation);
            high = high.max(ground.elevation);
        }
    }
    (high - low <= 0.52).then(|| world.elevation(x, z))
}

