//! Sparse lookout sites sited on actual mountain crests and high benches.

use crate::math::mix;
use crate::world::World;

pub const SUMMIT_TILE_SIZE: i32 = 768;
const FOOTPRINT: f32 = 7.0;
const EDGE: usize = 9;
const AXES: [[f32; 2]; 8] = {
    let diagonal = core::f32::consts::FRAC_1_SQRT_2;
    [
        [0.0, 1.0],
        [diagonal, diagonal],
        [1.0, 0.0],
        [diagonal, -diagonal],
        [0.0, -1.0],
        [-diagonal, -diagonal],
        [-1.0, 0.0],
        [-diagonal, diagonal],
    ]
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SummitKind {
    Watchtower,
    SummitHut,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SummitSite {
    center: [f32; 3],
    heading: f32,
    kind: SummitKind,
    variant: u32,
}

impl SummitSite {
    /// Survey up to two local ridges in a sparse, world-aligned set of tiles.
    /// Reservations must be treated as fixed for the duration of one survey.
    pub fn survey(
        world: &World,
        seed: u64,
        tile: [i32; 2],
        mut blocked: impl FnMut([f32; 4]) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        if tile
            .iter()
            .any(|coordinate| !(-4096..=4096).contains(coordinate))
        {
            return None;
        }
        let id = (u64::from(tile[0] as u32) << 32) | u64::from(tile[1] as u32);
        let key = mix(seed ^ id ^ 0xafb4_3e9c_66bd_f005);
        if key % 5 != 0 {
            return None;
        }
        let corner = tile.map(|axis| (axis * SUMMIT_TILE_SIZE + 64) as f32);
        let at =
            |col: usize, row: usize| [corner[0] + col as f32 * 80.0, corner[1] + row as f32 * 80.0];
        let mut grid = [[0.0; EDGE]; EDGE];
        for (row, samples) in grid.iter_mut().enumerate() {
            pulse();
            for (col, sample) in samples.iter_mut().enumerate() {
                *sample = elevation(world, at(col, row));
            }
        }
        let mut peaks = [(88.0, [0.0; 2]); 2];
        for row in 1..EDGE - 1 {
            for col in 1..EDGE - 1 {
                let height = grid[row][col];
                if height <= peaks[1].0
                    || (row - 1..=row + 1).any(|r| {
                        (col - 1..=col + 1).any(|c| (r != row || c != col) && grid[r][c] >= height)
                    })
                {
                    continue;
                }
                let candidate = (height, at(col, row));
                if height > peaks[0].0 {
                    peaks[1] = peaks[0];
                    peaks[0] = candidate;
                } else {
                    peaks[1] = candidate;
                }
            }
        }
        for (initial, point) in peaks {
            if initial <= 88.0 {
                break;
            }
            let (height, crest) = refine(world, initial, point, &mut pulse);
            if height < 101.0 || !mountain_relief(world, crest, height) {
                continue;
            }
            // Try the surveyed crown first, then two increasingly broad
            // rings for a bench directly below rough or protected crowns.
            let first_axis = (key >> 17) as usize & 7;
            let benches = [0.0, 12.0, 24.0].into_iter().flat_map(|radius| {
                (0..if radius == 0.0 { 1 } else { 8 })
                    .map(move |axis| offset(crest, AXES[(first_axis + axis) & 7], radius))
            });
            for point in benches {
                pulse();
                let grade = elevation(world, point);
                if grade < 94.0 || grade < height - 10.0 || blocked(bounds(point)) {
                    continue;
                }
                let Some(direction) = approach(world, point, grade) else {
                    continue;
                };
                if !prove_footing(world, point, grade, &mut pulse) {
                    continue;
                }
                let [x, z] = point;
                let variant =
                    mix(key ^ u64::from(x.to_bits()) ^ (u64::from(z.to_bits()) << 32)) as u32;
                let kind = if key & 3 == 0 && hut_apron(world, point, grade) {
                    SummitKind::SummitHut
                } else {
                    SummitKind::Watchtower
                };
                return Some(Self {
                    center: [x, grade, z],
                    heading: direction as f32 * core::f32::consts::FRAC_PI_4,
                    kind,
                    variant,
                });
            }
        }
        None
    }

    pub fn center(&self) -> [f32; 3] {
        self.center
    }

    pub fn heading(&self) -> f32 {
        self.heading
    }

    pub fn kind(&self) -> SummitKind {
        self.kind
    }

    pub fn variant(&self) -> u32 {
        self.variant
    }

    pub fn bounds(&self) -> [f32; 4] {
        bounds([self.center[0], self.center[2]])
    }
}

fn bounds([x, z]: [f32; 2]) -> [f32; 4] {
    const BUFFER: f32 = FOOTPRINT + 2.0;
    [x - BUFFER, z - BUFFER, x + BUFFER, z + BUFFER]
}

fn elevation(world: &World, [x, z]: [f32; 2]) -> f32 {
    world.elevation(x, z)
}

fn offset([x, z]: [f32; 2], [dx, dz]: [f32; 2], distance: f32) -> [f32; 2] {
    [x + dx * distance, z + dz * distance]
}

fn refine(
    world: &World,
    height: f32,
    center: [f32; 2],
    pulse: &mut impl FnMut(),
) -> (f32, [f32; 2]) {
    let mut peak = (height, center);
    for step in [16.0, 5.0] {
        let [x, z] = peak.1;
        for row in -2..=2 {
            pulse();
            for col in -2..=2 {
                let point = [x + col as f32 * step, z + row as f32 * step];
                let grade = elevation(world, point);
                if grade > peak.0 {
                    peak = (grade, point);
                }
            }
        }
    }
    peak
}

fn mountain_relief(world: &World, crest: [f32; 2], height: f32) -> bool {
    // A single low ravine is not enough. Look into several different valleys
    // and reject a bench sitting in the shadow of a distinctly higher crest.
    let near = AXES.map(|axis| height - elevation(world, offset(crest, axis, 58.0)));
    let far = AXES.map(|axis| height - elevation(world, offset(crest, axis, 104.0)));
    near.iter().filter(|&&drop| drop >= 8.0).count() >= 3
        && near.iter().filter(|&&drop| drop < -4.0).count() <= 1
        && far.iter().filter(|&&drop| drop >= 19.0).count() >= 4
        && far.iter().sum::<f32>() >= 8.0 * 19.0
}

fn approach(world: &World, center: [f32; 2], grade: f32) -> Option<usize> {
    let mut best = None;
    let mut least = f32::INFINITY;
    for (index, axis) in AXES.into_iter().enumerate() {
        let first = elevation(world, offset(center, axis, 10.0));
        let middle = elevation(world, offset(center, axis, 22.0));
        let last = elevation(world, offset(center, axis, 38.0));
        let drop = [grade - first, first - middle, middle - last];
        if !(-1.25..=3.8).contains(&drop[0])
            || !(-1.0..=6.5).contains(&drop[1])
            || !(-1.25..=9.0).contains(&drop[2])
            || last > grade - 2.0
        {
            continue;
        }
        let score =
            (drop[0] - 1.4).abs() + (drop[1] - 3.5).abs() * 0.5 + (drop[2] - 5.0).abs() * 0.28;
        if score < least {
            least = score;
            best = Some(index);
        }
    }
    best
}

fn prove_footing(world: &World, center: [f32; 2], grade: f32, pulse: &mut impl FnMut()) -> bool {
    let stable = |point| (-2.33..=0.68).contains(&(elevation(world, point) - grade));
    for row in -3..=3 {
        pulse();
        for col in -3..=3 {
            let radial_squared = (row * row + col * col) * 4;
            if radial_squared <= 49 && !stable(offset(center, [col as f32, row as f32], 2.0)) {
                return false;
            }
        }
    }
    if !AXES
        .into_iter()
        .all(|axis| stable(offset(center, axis, FOOTPRINT)))
    {
        return false;
    }
    // Check the exact low-level render grids as well as the procedural grade.
    // The broad grid can otherwise round off a crest beneath the foundations.
    for axis in [[0.0, 0.0], AXES[0], AXES[2], AXES[4], AXES[6]] {
        pulse();
        let point = offset(center, axis, 4.0);
        if !(-2.40..=0.95).contains(&(display_grade(world, point, 4.0) - grade)) {
            return false;
        }
    }
    // Within one broad cell, bilinear interpolation stays between the four
    // corner heights. Split the footing square wherever it meets a grid line.
    let split = |axis: f32| {
        let lo = axis - FOOTPRINT;
        let hi = axis + FOOTPRINT;
        let boundary = (grid_floor(lo * (1.0 / 20.0)) + 1) as f32 * 20.0;
        [lo, boundary.clamp(lo, hi), hi]
    };
    let columns = split(center[0]);
    for z in split(center[1]) {
        pulse();
        for x in columns {
            if !(-2.85..=1.1).contains(&(display_grade(world, [x, z], 20.0) - grade)) {
                return false;
            }
        }
    }
    true
}

fn display_grade(world: &World, point: [f32; 2], spacing: f32) -> f32 {
    let inverse = 1.0 / spacing;
    let grid = point.map(|axis| axis * inverse);
    let cell = grid.map(grid_floor);
    let vertex = |x: i32, z: i32| {
        let height = world.elevation(x as f32 * spacing, z as f32 * spacing);
        f32::from((height * 128.0 + 0.5) as u16)
    };
    let [x, z] = cell;
    let [u, v] = [grid[0] - x as f32, grid[1] - z as f32];
    let [a, b, c, d] = [
        vertex(x, z),
        vertex(x + 1, z),
        vertex(x, z + 1),
        vertex(x + 1, z + 1),
    ];
    let near = a + (b - a) * u;
    let far = c + (d - c) * u;
    (near + (far - near) * v) * (1.0 / 128.0)
}

fn grid_floor(axis: f32) -> i32 {
    let integer = axis as i32;
    integer - i32::from(axis < integer as f32)
}

fn hut_apron(world: &World, point: [f32; 2], grade: f32) -> bool {
    AXES.into_iter()
        .all(|axis| (-3.4..=1.3).contains(&(elevation(world, offset(point, axis, 11.5)) - grade)))
}
