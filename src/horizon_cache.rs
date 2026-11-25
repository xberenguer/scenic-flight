//! Budgeted, world-aligned elevation for the distant ground silhouette.

use crate::world::World;

const SIDE: usize = 64;
const EDGE: i32 = SIDE as i32;
const SPACING: f32 = 80.0;
const INVERSE_SPACING: f32 = 1.0 / SPACING;
const SCALE: f32 = 64.0;

struct Strip {
    axis: usize,
    direction: i32,
    cursor: usize,
}

/// A third, height-only grid that does not change the foreground samplers.
/// Negative values retain the coastline through interpolation.
pub struct HorizonCache {
    elevation: [[i16; SIDE]; SIDE],
    world: World,
    origin: [i32; 2],
    strip: Option<Strip>,
    ready: bool,
}

impl HorizonCache {
    pub const fn new() -> Self {
        Self {
            elevation: [[0; SIDE]; SIDE],
            world: World::new(0),
            origin: [0; 2],
            strip: None,
            ready: false,
        }
    }

    pub fn reset(&mut self, world: World, x: f32, z: f32) {
        self.reset_cooperatively(world, x, z, || {});
    }

    pub fn reset_cooperatively(&mut self, world: World, x: f32, z: f32, mut pulse: impl FnMut()) {
        self.world = world;
        self.origin = origin(x, z);
        self.strip = None;
        let [left, near] = self.origin;
        for z in near..near + EDGE {
            pulse();
            for x in left..left + EDGE {
                self.place(x, z);
            }
        }
        self.ready = true;
    }

    /// Replenish no more than `budget` cells. The coordinate of an unfinished
    /// strip remains unavailable until every cell has its new value.
    pub fn follow(&mut self, x: f32, z: f32, budget: usize) -> usize {
        if !self.ready {
            return 0;
        }
        let target = origin(x, z);
        let mut spent = 0;
        while spent < budget {
            let delta = [target[0] - self.origin[0], target[1] - self.origin[1]];
            if self.strip.is_none() {
                let axis = usize::from(delta[1].abs() > delta[0].abs());
                let direction = delta[axis].signum();
                if direction == 0 {
                    break;
                }
                self.strip = Some(Strip {
                    axis,
                    direction,
                    cursor: 0,
                });
            }
            let Strip {
                axis,
                direction,
                cursor,
            } = self.strip.take().unwrap();
            let count = (SIDE - cursor).min(budget - spent);
            let mut cell = self.origin;
            cell[axis] += if direction > 0 { EDGE } else { -1 };
            for offset in cursor..cursor + count {
                cell[axis ^ 1] = self.origin[axis ^ 1] + offset as i32;
                self.place(cell[0], cell[1]);
            }
            spent += count;
            if cursor + count == SIDE {
                self.origin[axis] += direction;
            } else {
                self.strip = Some(Strip {
                    axis,
                    direction,
                    cursor: cursor + count,
                });
            }
        }
        spent
    }

    /// Sample the same world-aligned lattice even beyond the retained grid.
    /// Missing vertices cost a procedural query; they never change the shape.
    #[inline]
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        let gx = x * INVERSE_SPACING;
        let gz = z * INVERSE_SPACING;
        let ix = floor(gx);
        let iz = floor(gz);
        let u = gx - ix as f32;
        let v = gz - iz as f32;
        let [lo, hi] = self.retained();
        let [a, b, c, d] =
            if self.ready && ix >= lo[0] && ix + 1 < hi[0] && iz >= lo[1] && iz + 1 < hi[1] {
                let near = &self.elevation[slot(iz)];
                let far = &self.elevation[slot(iz + 1)];
                let [left, right] = [slot(ix), slot(ix + 1)];
                [near[left], near[right], far[left], far[right]]
            } else {
                [[ix, iz], [ix + 1, iz], [ix, iz + 1], [ix + 1, iz + 1]]
                    .map(|[x, z]| relief(&self.world, x, z))
            }
            .map(f32::from);
        let near = a + (b - a) * u;
        let far = c + (d - c) * u;
        (near + (far - near) * v) * (1.0 / SCALE)
    }

    pub fn covers(&self, x: f32, z: f32) -> bool {
        let x = floor(x * INVERSE_SPACING);
        let z = floor(z * INVERSE_SPACING);
        let [lo, hi] = self.retained();
        self.ready && x >= lo[0] && x + 1 < hi[0] && z >= lo[1] && z + 1 < hi[1]
    }

    fn retained(&self) -> [[i32; 2]; 2] {
        let mut lo = self.origin;
        let mut hi = [lo[0] + EDGE, lo[1] + EDGE];
        if let Some(strip) = &self.strip {
            if strip.direction > 0 {
                lo[strip.axis] += 1;
            } else {
                hi[strip.axis] -= 1;
            }
        }
        [lo, hi]
    }

    fn place(&mut self, x: i32, z: i32) {
        self.elevation[slot(z)][slot(x)] = relief(&self.world, x, z);
    }
}

fn relief(world: &World, x: i32, z: i32) -> i16 {
    let h = world.signed_horizon_height(x as f32 * SPACING, z as f32 * SPACING);
    (h * SCALE + h.signum() * 0.5) as i16
}

fn slot(cell: i32) -> usize {
    (cell as usize) & (SIDE - 1)
}
fn floor(value: f32) -> i32 {
    let n = value as i32;
    n - i32::from(value < n as f32)
}
fn origin(x: f32, z: f32) -> [i32; 2] {
    [x, z].map(|v| floor(v * INVERSE_SPACING) - EDGE / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_strips_and_fast_repositions_keep_the_same_lattice() {
        let world = World::new(7911);
        let mut cache = HorizonCache::new();
        cache.reset(world, -73.0, 45.0);
        let uncached = HorizonCache {
            world,
            ..HorizonCache::new()
        };
        for [x, z] in [
            [129.0, -151.0],
            [-210.0, 79.0],
            [6430.0, -8220.0],
            [6451.0, -8173.0],
        ] {
            for budget in [0, 7, 16, 16, 32, 16, 64] {
                assert!(cache.follow(x, z, budget) <= budget);
                for dz in [-2345.5, -1015.3, -74.25, 0.0, 792.5, 2335.1] {
                    for dx in [-2401.3, -833.2, -31.4, 0.0, 802.1, 2380.7] {
                        assert_eq!(
                            cache.sample(x + dx, z + dz).to_bits(),
                            uncached.sample(x + dx, z + dz).to_bits()
                        );
                    }
                }
            }
        }
        cache.reset(world, 6451.0, -8173.0);
        assert!(cache.covers(6451.0 - 2290.0, -8173.0 + 2290.0));
        assert!(core::mem::size_of::<HorizonCache>() < 8300);
    }
}
