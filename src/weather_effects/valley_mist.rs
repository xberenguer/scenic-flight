//! Sheltered valley fog built from the terrain and lit by the local dawn.

use crate::flight::TICKS_PER_SECOND;
use crate::math::{fraction, mix, sin_cos, smoothstep};
use crate::render::{Camera, ScreenRect};
use crate::sky::Sky;
use crate::world::{Surface, World};

use super::{Projection, blend_alpha, rgb565, sqrt};

const TILE: f32 = 512.0;
const SLOTS: usize = 9;
const REACH: f32 = 424.0;

#[derive(Clone, Copy)]
struct Bank {
    center: [f32; 3],
    axis: (f32, f32),
    radii: [f32; 3],
    density: f32,
}

impl Bank {
    fn survey(world: &World, seed: u64, cell: [i32; 2]) -> Option<Self> {
        let tile = (u64::from(cell[0] as u32) << 32) | u64::from(cell[1] as u32);
        let key = mix(seed ^ mix(tile ^ 0x80d9_54e1_7d8a_3013));
        if key % 7 >= 5 {
            return None;
        }
        let mut best = None;
        let mut best_score = 0.0;
        for attempt in 0..7 {
            let random = mix(key ^ attempt);
            let x = (cell[0] as f32 + 0.25 + fraction(random) * 0.5) * TILE;
            let z = (cell[1] as f32 + 0.25 + fraction(mix(random ^ 39)) * 0.5) * TILE;
            let floor = world.sample(x, z);
            if !(4.0..=100.0).contains(&floor.elevation)
                || !matches!(floor.surface, Surface::Grass | Surface::Forest)
            {
                continue;
            }
            let angle = fraction(mix(random ^ 55)) * core::f32::consts::FRAC_PI_4;
            for aim in 0..4 {
                let axis = sin_cos(angle + aim as f32 * core::f32::consts::FRAC_PI_4);
                let (east, north) = axis;
                let height = |along: f32, across: f32| {
                    world.elevation(x + east * along + north * across, z + north * along - east * across)
                        - floor.elevation
                };
                let banks = height(0.0, -64.0).min(height(0.0, 64.0));
                if banks < 9.0 {
                    continue;
                }
                let a = height(-54.0, 0.0);
                let b = height(54.0, 0.0);
                if a.min(b) < -9.0 || a.max(b) > 12.0 {
                    continue;
                }
                let inner = height(0.0, -32.0).min(height(0.0, 32.0));
                let relief = banks.min(28.0);
                let score = relief - (a.abs() + b.abs()) * 0.30 - inner.abs() * 0.35;
                if score <= best_score {
                    continue;
                }
                let thickness = (relief * 0.42).clamp(4.0, 11.0);
                best = Some(Self {
                    center: [x, floor.elevation + thickness * 0.19 + (a + b) * 0.12, z],
                    axis,
                    radii: [70.0 + relief * 2.1, thickness, (59.0 - inner * 0.85).clamp(32.0, 64.0)],
                    density: 0.62 + fraction(mix(random ^ 601)) * 0.35,
                });
                best_score = score;
            }
        }
        best
    }

    fn screen_bounds(self, view: &Projection, dimensions: (usize, usize)) -> Option<ScreenRect> {
        let (width, height) = dimensions;
        let [x, y, z] = self.center;
        let (east, north) = self.axis;
        let [length, rise, cross] = self.radii;
        let center = view.direction([x - view.camera.x, y - view.camera.altitude, z - view.camera.z]);
        let local_axes = [
            view.direction([east * length, 0.0, north * length]),
            view.direction([0.0, rise, 0.0]),
            view.direction([north * cross, 0.0, -east * cross]),
        ];
        let component = |axis| local_axes.map(|direction| direction[axis]);
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let depths = component(2);
        let radius_squared = dot(depths, depths);
        let radius = sqrt(radius_squared);
        if center[2] + radius <= 2.0 { return None; }
        if center[2] - radius <= 2.0 { return Some(ScreenRect::full(dimensions)); }
        // Tangents to the volume bound its perspective image more tightly
        // than projecting a rectangular box around the whole valley.
        let divisor = center[2] * center[2] - radius_squared;
        let limits = |axis| {
            let values = component(axis);
            let offset = center[axis] * center[2] - dot(values, depths);
            let edge = center[axis] * center[axis] - dot(values, values);
            let span = sqrt((offset * offset - divisor * edge).max(0.0));
            let scale = view.focal / divisor;
            [
                view.center[axis] + (offset - span) * scale,
                view.center[axis] + (offset + span) * scale,
            ]
        };
        let [left, right] = limits(0);
        let [top, bottom] = limits(1);
        let rect = ScreenRect {
            left: ((left - 2.0) as usize).min(width),
            right: ((right + 3.0) as usize).min(width),
            top: ((top - 2.0) as usize).min(height),
            bottom: ((bottom + 3.0) as usize).min(height),
        };
        (rect.left < rect.right && rect.top < rect.bottom).then_some(rect)
    }
}

#[derive(Clone, Copy)]
struct TileBank {
    cell: [i32; 2],
    survey: Option<(u64, Option<Bank>)>,
}

impl TileBank {
    const EMPTY: Self = Self { cell: [i32::MIN; 2], survey: None };
}

/// Survey at most one new tile per frame, during the approach to sunrise.
pub struct ValleyMist {
    region: Option<(u64, [i32; 2])>,
    tiles: [TileBank; SLOTS],
}

impl Default for ValleyMist {
    fn default() -> Self { Self::new() }
}

impl ValleyMist {
    pub const fn new() -> Self {
        Self { region: None, tiles: [TileBank::EMPTY; SLOTS] }
    }

    pub fn observe(
        &mut self, world: &World, seed: u64, ticks: u64, camera: Camera, sky: &Sky,
        dimensions: (usize, usize),
    ) -> MistView {
        let mut visible = MistView { patches: [None; SLOTS], region: None, tint: 0 };
        let sun = sky.sun_direction[1];
        if !sky.sun_rising || !(-0.31..0.33).contains(&sun) || camera.altitude > 172.0 {
            return visible;
        }
        let cell_at = |value: f32| {
            let value = value * (1.0 / TILE);
            let whole = value as i32;
            whole.saturating_sub(i32::from(value < whole as f32))
        };
        let center = [cell_at(camera.x), cell_at(camera.z)];
        if self.region != Some((seed, center)) {
            let old = self.tiles;
            let same_world = self.region.is_some_and(|(previous, _)| previous == seed);
            const OFFSETS: [[i32; 2]; SLOTS] = [
                [0, 0], [-1, 0], [0, -1], [1, 0], [0, 1], [-1, -1], [1, 1], [-1, 1], [1, -1],
            ];
            self.tiles = OFFSETS.map(|[x, z]| {
                let cell = [center[0].saturating_add(x), center[1].saturating_add(z)];
                let survey = if same_world {
                    old.iter().find(|entry| entry.cell == cell).and_then(|entry| entry.survey)
                } else {
                    None
                };
                TileBank { cell, survey }
            });
            self.region = Some((seed, center));
        }
        if let Some(tile) = self.tiles.iter_mut().find(|entry| entry.survey.is_none()) {
            tile.survey = Some((ticks, Bank::survey(world, seed, tile.cell)));
        }
        let daylight = smoothstep((sun + 0.23) / 0.15) * (1.0 - smoothstep((sun - 0.09) / 0.24));
        let mut projection = None;
        let tint = rgb565([154.0 + 68.0 * sky.daylight, 165.0 + 52.0 * sky.daylight, 180.0 + 33.0 * sky.daylight]);
        for (slot, tile) in visible.patches.iter_mut().zip(&self.tiles) {
            let Some((published, Some(bank))) = tile.survey else { continue; };
            let [x, y, z] = bank.center;
            let squared = (x - camera.x) * (x - camera.x) + (z - camera.z) * (z - camera.z);
            if squared >= REACH * REACH {
                continue;
            }
            let clouds = sky.weather_over(x, z).cloud_cover;
            let clearing = 1.0 - smoothstep((clouds - 0.54) / 0.23);
            let distance = smoothstep((REACH * REACH - squared) / 51_000.0);
            let fade = smoothstep(ticks.saturating_sub(published) as f32 / (2 * TICKS_PER_SECOND) as f32);
            let density = daylight * clearing * distance * fade * bank.density
                * (1.0 - smoothstep((camera.altitude - y - 61.0) / 55.0))
                * smoothstep((172.0 - camera.altitude) / 30.0);
            if density < 0.009 {
                continue;
            }
            let projection = projection.get_or_insert_with(|| Projection::new(camera, dimensions.0, dimensions.1));
            let Some(rect) = bank.screen_bounds(projection, dimensions) else { continue; };
            *slot = Some(ProjectedBank { bank, density, rect });
            visible.region = Some(visible.region.map_or(rect, |prior| prior.union(rect)));
        }
        visible.tint = tint;
        visible
    }
}

#[derive(Clone, Copy)]
struct ProjectedBank {
    bank: Bank,
    density: f32,
    rect: ScreenRect,
}

pub struct MistView {
    patches: [Option<ProjectedBank>; SLOTS],
    region: Option<ScreenRect>,
    tint: u16,
}

impl MistView {
    pub fn depth_region(&self) -> Option<ScreenRect> { self.region }

    pub fn paint(
        &self, camera: Camera, dimensions: (usize, usize), pixels: &mut [u16], depth: &[u16],
    ) {
        let (width, height) = dimensions;
        if self.region.is_none() || width == 0 || height == 0 || width > 512 || height > 512
            || pixels.len() < width * height || depth.len() < width * height
        {
            return;
        }
        let view = Projection::new(camera, width, height);
        for patch in self.patches.iter().flatten() {
            patch.paint(&view, dimensions, self.tint, pixels, depth);
        }
    }
}

impl ProjectedBank {
    fn paint(
        &self, view: &Projection, (width, height): (usize, usize), tint: u16,
        pixels: &mut [u16], depth: &[u16],
    ) {
        let [x, y, z] = self.bank.center;
        let (east, north) = self.bank.axis;
        let [length, rise, cross] = self.bank.radii.map(|radius| 1.0 / radius);
        let local = |[x, y, z]: [f32; 3]| {
            [(x * east + z * north) * length, y * rise, (x * north - z * east) * cross]
        };
        let origin = local([view.camera.x - x, view.camera.altitude - y, view.camera.z - z]);
        let (hs, hc) = view.heading;
        let (ps, pc) = view.pitch;
        let (bs, bc) = view.bank;
        let focal = 1.0 / view.focal;
        let flat = local([hs * pc, -ps, hc * pc]);
        let right = local([hc, 0.0, -hs]);
        let down = local([-hs * ps, -pc, -hc * ps]);
        let step_x: [f32; 3] = core::array::from_fn(|i| (right[i] * bc - down[i] * bs) * focal);
        let step_y: [f32; 3] = core::array::from_fn(|i| (right[i] * bs + down[i] * bc) * focal);
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let c = dot(origin, origin) - 1.0;
        let aperture = (width.min(height) * width.min(height)) as i32;
        let screen = self.rect;
        let first_column = screen.left & !1;
        for row in ((screen.top & !1)..screen.bottom).step_by(2) {
            let sy = row as f32 + 0.5 - view.center[1];
            let sx = first_column as f32 + 0.5 - view.center[0];
            let left: [f32; 3] = core::array::from_fn(|i| flat[i] + step_x[i] * sx + step_y[i] * sy);
            for column in (first_column..screen.right).step_by(2) {
                let offset = (column - first_column) as f32;
                let ray = [
                    left[0] + step_x[0] * offset,
                    left[1] + step_x[1] * offset,
                    left[2] + step_x[2] * offset,
                ];
                let a = dot(ray, ray);
                let b = dot(origin, ray);
                let discriminant = b * b - a * c;
                if discriminant <= 0.0 {
                    continue;
                }
                let inverse = 1.0 / a;
                let spread = sqrt(discriminant);
                let start = ((-b - spread) * inverse).max(2.0);
                let end = ((-b + spread) * inverse).min(512.0);
                if end <= start {
                    continue;
                }
                // Depth is stored in world units times 32. Convert optical
                // density to fixed-point alpha once for the whole square.
                let gain = (self.density * (discriminant * inverse).min(1.0) * (0.023 * 2048.0) + 0.5) as u32;
                if gain == 0 { continue; }
                let entry = (start * 32.0 + 0.5) as u16;
                let exit = (end * 32.0 + 0.5) as u16;
                let cutoff = ((end + 18.0) * 32.0) as u16;
                // Share the fog ray within a two-pixel square. Each pixel still
                // uses its own terrain depth, so a nearer ridge masks the bank.
                for y in row.max(screen.top)..(row + 2).min(screen.bottom) {
                    let dy = 2 * y as i32 + 1 - height as i32;
                    let above = y.saturating_sub(1) * width;
                    let below = (y + 1).min(height - 1) * width;
                    for x in column.max(screen.left)..(column + 2).min(screen.right) {
                        let dx = 2 * x as i32 + 1 - width as i32;
                        if dx * dx + dy * dy > aperture { continue; }
                        let index = y * width + x;
                        let foreground = depth[index];
                        if foreground <= entry { continue; }
                        let surface = if foreground < cutoff {
                            let tolerance = 128 + foreground / 8;
                            let adjacent = |address| {
                                let neighbor: u16 = depth[address];
                                if neighbor > entry && neighbor.abs_diff(foreground) < tolerance {
                                    neighbor
                                } else {
                                    foreground
                                }
                            };
                            ((u32::from(foreground) * 2 + u32::from(adjacent(above + x))
                                + u32::from(adjacent(below + x)) + 2) >> 2) as u16
                        } else {
                            foreground
                        };
                        let thickness = u32::from(exit.min(surface).saturating_sub(entry));
                        let alpha = ((thickness * gain + 128) >> 8).min(184);
                        if alpha != 0 {
                            pixels[index] = blend_alpha(pixels[index], tint, alpha);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_with_depth;

    const SIDE: usize = 112;
    const DIMENSIONS: (usize, usize) = (SIDE, SIDE);
    const PIXELS: usize = SIDE * SIDE;
    const CAMERA: Camera = Camera {
        x: -2304.0, z: -1442.0, altitude: 109.39075, heading: 0.027, pitch: 0.21, bank: 0.023,
    };

    fn sky(day: f32) -> Sky {
        Sky::from_time_at_position(3, 180, day, [CAMERA.x, CAMERA.altitude, CAMERA.z])
    }

    #[test]
    fn fog_grows_gradually_in_a_surveyed_valley_at_dawn() {
        let world = World::new(3);
        let mut mist = ValleyMist::new();
        assert!(mist.observe(&world, 3, 0, CAMERA, &sky(0.277), DIMENSIONS).depth_region().is_none());
        for tick in 1..9 {
            mist.observe(&world, 3, tick, CAMERA, &sky(0.277), DIMENSIONS);
        }
        assert!(mist.observe(&world, 3, 180, CAMERA, &sky(0.277), DIMENSIONS).depth_region().is_some());
        for phase in [0.50, 0.723] {
            assert!(mist.observe(&world, 3, 180, CAMERA, &sky(phase), DIMENSIONS).depth_region().is_none());
        }
    }

    #[test]
    fn fog_respects_terrain_depth_and_the_round_aperture() {
        let world = World::new(3);
        let sky = sky(0.277);
        let mut mist = ValleyMist::new();
        for tick in 0..9 {
            mist.observe(&world, 3, tick, CAMERA, &sky, DIMENSIONS);
        }
        let view = mist.observe(&world, 3, 180, CAMERA, &sky, DIMENSIONS);
        let mut original = [0; PIXELS];
        let mut work = [0; PIXELS];
        let mut depth = [0; PIXELS];
        render_with_depth(
            |x, z| world.sample(x, z), CAMERA, &sky, &mut original, &mut work, &mut depth, DIMENSIONS,
        ).unwrap();
        let mut with_fog = original;
        view.paint(CAMERA, DIMENSIONS, &mut with_fog, &depth);
        let mut changed = 0;
        for (index, (before, after)) in original.iter().zip(with_fog).enumerate() {
            if *before != after {
                changed += 1;
                let x = 2 * (index % SIDE) as i32 + 1 - SIDE as i32;
                let y = 2 * (index / SIDE) as i32 + 1 - SIDE as i32;
                assert!(x * x + y * y <= (SIDE * SIDE) as i32);
            }
        }
        assert!(changed > 1000);
        let mut in_front = original;
        view.paint(CAMERA, DIMENSIONS, &mut in_front, &[32; PIXELS]);
        assert_eq!(in_front, original);
    }
}
