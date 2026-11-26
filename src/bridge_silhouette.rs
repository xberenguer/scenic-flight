//! Subpixel bridge silhouettes following the exact fitted deck and approaches.

use crate::crossings::{Bridge, BridgeStyle, BridgeWater};
use crate::math::{sin_cos, smoothstep};
use crate::render::{
    CITY_HORIZON, Camera, ScreenRect, backdrop_curvature_squared, encoded_depth, horizon_distance,
    view_distance,
};
use crate::sky::Sky;
use crate::world::SEA_LEVEL;

const LIMIT: usize = 8;
const FADE: u64 = 32;

pub(crate) fn has_extended_range(bridge: &Bridge) -> bool {
    bridge.water == BridgeWater::SeaNarrows && bridge.span() >= 44.0
}

#[derive(Clone, Copy)]
struct Retained {
    bridge: Bridge,
    age: u64,
    seen: bool,
    fresh: bool,
}

/// Publication and eviction are eased without creating any additional plans.
pub struct BridgeSilhouettes {
    retained: [Option<Retained>; LIMIT],
    last_tick: u64,
}

impl BridgeSilhouettes {
    pub const fn new() -> Self {
        Self {
            retained: [None; LIMIT],
            last_tick: 0,
        }
    }
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn prepare<'a>(
        &mut self,
        camera: Camera,
        ticks: u64,
        sky: &Sky,
        dimensions: (usize, usize),
        bridges: impl IntoIterator<Item = &'a Bridge>,
        mut far_ground: impl FnMut(f32, f32) -> f32,
    ) -> BridgeSilhouetteFrame {
        if ticks < self.last_tick {
            self.reset();
        }
        let delta = ticks.saturating_sub(self.last_tick).min(6);
        self.last_tick = ticks;
        let view = View::new(camera, dimensions);
        for entry in self.retained.iter_mut().flatten() {
            entry.seen = false;
        }
        for bridge in bridges {
            if !view.potential(bridge) {
                continue;
            }
            if let Some(entry) = self
                .retained
                .iter_mut()
                .flatten()
                .find(|entry| entry.bridge == *bridge)
            {
                entry.seen = true;
            } else {
                let mut free = self.retained.iter_mut().filter(|r| r.is_none());
                let Some(slot) = free.next() else { continue };
                // Reserve two slots for crossings entering the detailed view.
                if view.outer(bridge) && free.nth(1).is_none() {
                    continue;
                }
                *slot = Some(Retained {
                    bridge: *bridge,
                    age: 0,
                    seen: true,
                    fresh: true,
                });
            }
        }
        let mut frame = BridgeSilhouetteFrame {
            projected: [None; LIMIT],
            dimensions,
            len: 0,
            bounds: None,
        };
        for slot in &mut self.retained {
            let Some(entry) = slot else {
                continue;
            };
            if entry.fresh {
                entry.fresh = false;
                continue;
            }
            entry.age = if entry.seen {
                (entry.age + delta).min(FADE)
            } else {
                entry.age.saturating_sub(delta)
            };
            if entry.age == 0 {
                if !entry.seen {
                    *slot = None;
                }
                continue;
            }
            let maturity = smoothstep(entry.age as f32 * (1.0 / FADE as f32));
            if let Some(projected) = view.bridge(entry.bridge, maturity, sky, &mut far_ground) {
                let i = frame.projected[..frame.len]
                    .partition_point(|value| value.is_some_and(|p| p.depth <= projected.depth));
                frame.projected.copy_within(i..frame.len, i + 1);
                frame.projected[i] = Some(projected);
                frame.len += 1;
            }
        }
        frame.bounds = frame.projected[..frame.len]
            .iter()
            .flatten()
            .map(|bridge| bridge.bounds)
            .reduce(ScreenRect::union);
        frame
    }
}

pub struct BridgeSilhouetteFrame {
    projected: [Option<Projected>; LIMIT],
    dimensions: (usize, usize),
    len: usize,
    bounds: Option<ScreenRect>,
}

impl BridgeSilhouetteFrame {
    pub fn bounds(&self) -> Option<ScreenRect> {
        self.bounds
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn detail_opacity(&self, bridge: &Bridge) -> f32 {
        self.projected[..self.len]
            .iter()
            .flatten()
            .find(|record| record.join_key == join_key(bridge))
            .map_or(1.0, |record| 1.0 - record.handoff)
    }

    /// Paint after the terrain, with its depth valid over the saved bounds.
    pub fn draw(&self, pixels: &mut [u16], depth: &[u16]) {
        let (width, height) = self.dimensions;
        let Some(total) = width.checked_mul(height) else {
            return;
        };
        if pixels.len() < total || depth.len() < total {
            return;
        }
        let diameter = width.min(height) as i32;
        for structure in self.projected[..self.len].iter().rev().flatten() {
            let rect = structure.bounds;
            for y in rect.top..rect.bottom {
                let dy = (2 * y + 1) as i32 - height as i32;
                for x in rect.left..rect.right {
                    let dx = (2 * x + 1) as i32 - width as i32;
                    if dx * dx + dy * dy > diameter * diameter {
                        continue;
                    }
                    let i = y * width + x;
                    // Choose one stroke at coincident joins. This avoids a
                    // dark seam when the deck and its ramp share a pixel.
                    let mut strongest = 0.0_f32;
                    let mut color = 0;
                    for stroke in structure.strokes.iter().flatten() {
                        let Some(coverage) = stroke.coverage([x as f32, y as f32], depth[i]) else {
                            continue;
                        };
                        if coverage > strongest {
                            strongest = coverage;
                            color = stroke.color;
                        }
                    }
                    let alpha = (strongest * 64.0 + 0.5) as u32;
                    if alpha != 0 {
                        pixels[i] = blend(pixels[i], color, alpha.min(64));
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Projected {
    join_key: [u32; 4],
    strokes: [Option<Stroke>; 6],
    bounds: ScreenRect,
    depth: f32,
    handoff: f32,
}

#[derive(Clone, Copy)]
struct Vertex {
    p: [f32; 2],
    inverse_z: f32,
}

#[derive(Clone, Copy)]
struct Stroke {
    start: Vertex,
    end: Vertex,
    delta: [f32; 2],
    inverse_length_squared: f32,
    radius: f32,
    opacity: f32,
    color: u16,
}

impl Stroke {
    fn coverage(self, p: [f32; 2], terrain_z: u16) -> Option<f32> {
        let ray = sub(p, self.start.p);
        let t = (dot(ray, self.delta) * self.inverse_length_squared).clamp(0.0, 1.0);
        let normal = sub(ray, scale(self.delta, t));
        let distance = dot(normal, normal);
        let outer = self.radius + 0.55;
        if distance >= outer * outer {
            return None;
        }
        let z = encoded_depth(1.0 / mix(self.start.inverse_z, self.end.inverse_z, t));
        if z >= terrain_z {
            return None;
        }
        let inner = (self.radius - 0.5).max(0.0);
        let coverage =
            ((outer * outer - distance) / (outer * outer - inner * inner)).clamp(0.0, 1.0);
        Some(coverage * self.opacity * (self.radius * 2.0).min(1.0))
    }
}

struct View {
    camera: Camera,
    dimensions: (usize, usize),
    heading: [f32; 2],
    pitch: [f32; 2],
    roll: [f32; 2],
    near: f32,
    far: f32,
    focal: f32,
    curvature: f32,
}

impl View {
    fn new(camera: Camera, dimensions: (usize, usize)) -> Self {
        let near = view_distance(camera.altitude);
        Self {
            camera,
            dimensions,
            heading: sin_cos(camera.heading).into(),
            pitch: sin_cos(camera.pitch.clamp(-1.3, 1.3)).into(),
            roll: sin_cos(-camera.bank).into(),
            near,
            far: horizon_distance(camera.altitude),
            focal: dimensions.0.min(dimensions.1) as f32 * 0.73,
            curvature: (near - 512.0) * (0.000040 / 384.0),
        }
    }

    fn relative(&self, x: f32, z: f32) -> [f32; 2] {
        let dx = x - self.camera.x;
        let dz = z - self.camera.z;
        [
            dx * self.heading[1] - dz * self.heading[0],
            dx * self.heading[0] + dz * self.heading[1],
        ]
    }

    fn center(&self, bridge: &Bridge) -> [f32; 2] {
        let [a, _, _, b] = *bridge.sections();
        self.relative((a[0] + b[0]) * 0.5, (a[2] + b[2]) * 0.5)
    }

    fn reach(&self, bridge: &Bridge) -> f32 {
        if has_extended_range(bridge) {
            CITY_HORIZON
        } else {
            self.far
        }
    }

    fn outer(&self, bridge: &Bridge) -> bool {
        self.center(bridge)[1] > self.far
    }

    fn potential(&self, bridge: &Bridge) -> bool {
        let [side, along] = self.center(bridge);
        along > self.near * 0.61
            && along < self.reach(bridge) + 100.0
            && side.abs() < along * 0.80 + 120.0
    }

    fn vertex(&self, [x, y, z]: [f32; 3]) -> Option<Vertex> {
        let [side, ahead] = self.relative(x, z);
        let relief =
            y - self.camera.altitude - self.curvature * backdrop_curvature_squared(ahead, side);
        let z = ahead * self.pitch[1] - relief * self.pitch[0];
        if z <= 2.0 || !z.is_finite() {
            return None;
        }
        let k = 1.0 / z;
        let u = side * self.focal * k;
        let v = (-ahead * self.pitch[0] - relief * self.pitch[1]) * self.focal * k;
        let (w, h) = self.dimensions;
        Some(Vertex {
            p: [
                (w - 1) as f32 * 0.5 + u * self.roll[1] - v * self.roll[0],
                (h - 1) as f32 * 0.5 + u * self.roll[0] + v * self.roll[1],
            ],
            inverse_z: k,
        })
    }

    fn stroke(
        &self,
        a: [f32; 3],
        b: [f32; 3],
        physical_width: f32,
        opacity: f32,
        color: u16,
        axis: [f32; 2],
        side_depth: f32,
    ) -> Option<(Stroke, ScreenRect)> {
        let start = self.vertex(a)?;
        let end = self.vertex(b)?;
        let delta = sub(end.p, start.p);
        let len2 = dot(delta, delta);
        let a_edge = self.vertex([
            a[0] - axis[1] * physical_width,
            a[1],
            a[2] + axis[0] * physical_width,
        ])?;
        let offset = sub(a_edge.p, start.p);
        let projected_casing = side_depth * self.focal * (start.inverse_z + end.inverse_z) * 0.5;
        let radius = if len2 > 0.004 {
            let square = (offset[0] * delta[1] - offset[1] * delta[0]).abs();
            square * inverse_root(len2) + projected_casing
        } else {
            dot(offset, offset) * inverse_root(dot(offset, offset).max(0.002)) + projected_casing
        }
        .clamp(0.075, 3.8);
        let (w, h) = self.dimensions;
        let pad = radius + 1.0;
        let bounds = ScreenRect {
            left: ((start.p[0].min(end.p[0]) - pad) as usize).min(w),
            top: ((start.p[1].min(end.p[1]) - pad) as usize).min(h),
            right: ((start.p[0].max(end.p[0]) + pad + 1.0) as usize).min(w),
            bottom: ((start.p[1].max(end.p[1]) + pad + 1.0) as usize).min(h),
        };
        if bounds.left >= bounds.right || bounds.top >= bounds.bottom {
            return None;
        }
        Some((
            Stroke {
                start,
                end,
                delta,
                inverse_length_squared: 1.0 / len2.max(0.004),
                radius,
                opacity,
                color,
            },
            bounds,
        ))
    }

    fn bridge(
        &self,
        bridge: Bridge,
        age: f32,
        sky: &Sky,
        ground: &mut impl FnMut(f32, f32) -> f32,
    ) -> Option<Projected> {
        let (w, h) = self.dimensions;
        if w == 0 || h == 0 || w > 512 || h > 512 {
            return None;
        }
        let [a, b, c, d] = *bridge.sections();
        let [side, along] = self.relative((a[0] + d[0]) * 0.5, (a[2] + d[2]) * 0.5);
        if side.abs() > along * 0.78 + 55.0 {
            return None;
        }
        let handoff_range = match bridge.water {
            BridgeWater::River => 0.93,
            BridgeWater::SeaNarrows => 1.07,
        };
        let near_fade =
            smoothstep((along - self.near * 0.74) / (self.near * (handoff_range - 0.74)));
        let reach = self.reach(&bridge);
        let far_fade = 1.0 - smoothstep((along - reach * 0.80) / (reach * 0.20));
        let fade = age * near_fade * far_fade;
        if fade < 0.015 {
            return None;
        }
        // Keep certified heights. Where coarse terrain would visually detach
        // a dry ramp, fade the whole span. Never replace or lower the water.
        let mismatch = [a, d]
            .into_iter()
            .map(|[x, y, z]| (y - ground(x, z).max(SEA_LEVEL)).max(0.0))
            .fold(0.0, f32::max);
        let grounded = 1.0
            - smoothstep((mismatch - 2.2) / 6.8)
                * smoothstep((along - self.near * 0.95) / (self.near * 0.20));
        let alpha = fade * grounded * (0.85 - sky.haze * 0.36);
        if alpha < 0.015 {
            return None;
        }
        let dxz = [d[0] - a[0], d[2] - a[2]];
        let axis = scale(dxz, inverse_root(dot(dxz, dxz)));
        let daylight = sky.daylight.clamp(0.0, 1.0);
        let road = rgb([
            37.0 + 89.0 * daylight,
            50.0 + 92.0 * daylight,
            64.0 + 96.0 * daylight,
        ]);
        let deck = rgb([
            30.0 + 76.0 * daylight,
            43.0 + 92.0 * daylight,
            57.0 + 103.0 * daylight,
        ]);
        let truss = rgb(bridge
            .sidework_color()
            .map(|c| 26.0 + f32::from(c) * daylight * 0.76));
        let width = bridge.width() * 0.5;
        let mut strokes = [
            self.stroke(a, b, width, alpha * 0.66, road, axis, 0.15),
            self.stroke(b, c, width, alpha, deck, axis, 0.47),
            self.stroke(c, d, width, alpha * 0.66, road, axis, 0.15),
            None,
            None,
            None,
        ];
        if bridge.style == BridgeStyle::SmallTruss {
            for (i, [from, to]) in [[0.0, 1.0 / 3.0], [1.0 / 3.0, 2.0 / 3.0], [2.0 / 3.0, 1.0]]
                .into_iter()
                .enumerate()
            {
                strokes[3 + i] = self.stroke(
                    bridge.sidework_outline(from),
                    bridge.sidework_outline(to),
                    width * 0.38,
                    alpha * 0.73,
                    truss,
                    axis,
                    0.23,
                );
            }
        }
        let bounds = strokes
            .iter()
            .flatten()
            .map(|&(_, bounds)| bounds)
            .reduce(ScreenRect::union)?;
        Some(Projected {
            join_key: join_key(&bridge),
            strokes: strokes.map(|stroke| stroke.map(|(stroke, _)| stroke)),
            bounds,
            depth: along,
            handoff: age * near_fade * grounded,
        })
    }
}

fn join_key(bridge: &Bridge) -> [u32; 4] {
    // Published plans reserve their entire approach, so two bridges cannot
    // occupy the same pair of exact dry road joins.
    let [a, _, _, b] = bridge.sections();
    [
        a[0].to_bits(),
        a[2].to_bits(),
        b[0].to_bits(),
        b[2].to_bits(),
    ]
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}
fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn scale(a: [f32; 2], m: f32) -> [f32; 2] {
    [a[0] * m, a[1] * m]
}
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn inverse_root(squared: f32) -> f32 {
    let mut y = f32::from_bits(0x5f37_5a86 - (squared.to_bits() >> 1));
    for _ in 0..2 {
        y *= 1.5 - 0.5 * squared * y * y;
    }
    y
}
fn rgb([r, g, b]: [f32; 3]) -> u16 {
    ((r * (31.0 / 255.0) + 0.5) as u16) << 11
        | ((g * (63.0 / 255.0) + 0.5) as u16) << 5
        | ((b * (31.0 / 255.0) + 0.5) as u16)
}
fn blend(a: u16, b: u16, t: u32) -> u16 {
    let ch = |shift: usize, mask: u32| {
        let a = (u32::from(a) >> shift) & mask;
        let b = (u32::from(b) >> shift) & mask;
        ((a * (64 - t) + b * t + 32) >> 6) as u16
    };
    ch(11, 31) << 11 | ch(5, 63) << 5 | ch(0, 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crossings::Crossings;
    use crate::roads::RoadNetwork;
    use crate::watershed::{Watershed, WatershedWork};
    use crate::world::World;

    const SIDE: usize = 112;
    const PIXELS: usize = SIDE * SIDE;
    const SIZE: (usize, usize) = (SIDE, SIDE);

    fn camera(bridge: &Bridge, distance: f32) -> Camera {
        let [_, b, c, _] = bridge.sections();
        let dx = c[0] - b[0];
        let dz = c[2] - b[2];
        let heading = dz.atan2(-dx);
        let (sin, cos) = sin_cos(heading);
        Camera {
            x: (b[0] + c[0]) * 0.5 - sin * distance,
            z: (b[2] + c[2]) * 0.5 - cos * distance,
            altitude: b[1] + 38.0,
            heading,
            pitch: 0.10,
            bank: 0.035,
        }
    }

    fn verify(world: World, bridge: &Bridge) {
        let mut state = BridgeSilhouettes::new();
        let far = camera(bridge, 660.0);
        let sky = Sky::from_time_at_position(67, 0, 0.33, [far.x, far.altitude, far.z]);
        let ground = |x, z| world.elevation(x, z);
        let empty_depth = [u16::MAX; PIXELS];
        let nearer_depth = [encoded_depth(200.0); PIXELS];
        let background = [u16::MAX; PIXELS];
        let render = |frame: &BridgeSilhouetteFrame, depth| {
            let mut image = background;
            frame.draw(&mut image, depth);
            image
        };
        assert!(
            state
                .prepare(far, 0, &sky, SIZE, [bridge], ground)
                .is_empty()
        );
        for tick in 1..=32 {
            state.prepare(far, tick, &sky, SIZE, [bridge], ground);
        }
        let frame = state.prepare(far, 33, &sky, SIZE, [bridge], ground);
        assert_eq!(frame.len(), 1);
        let region = frame.bounds().unwrap();
        assert!(frame.detail_opacity(bridge) < 0.01);
        let pixels = render(&frame, &empty_depth);
        let changed: std::vec::Vec<_> = pixels
            .iter()
            .zip(background)
            .enumerate()
            .filter_map(|(index, (&pixel, drawn))| (pixel != drawn).then_some(index))
            .collect();
        assert!(!changed.is_empty());
        for i in changed {
            let (x, y) = (i % SIDE, i / SIDE);
            assert!((region.left..region.right).contains(&x));
            assert!((region.top..region.bottom).contains(&y));
        }
        assert_eq!(render(&frame, &nearer_depth), background);

        let mut last_delta = 0u32;
        for tick in 34..=65 {
            let frame = state.prepare(far, tick, &sky, SIZE, [], ground);
            let faded = render(&frame, &empty_depth);
            let difference = faded.iter().map(|&p| u32::from(u16::MAX - p)).sum();
            if tick != 34 {
                assert!(difference <= last_delta);
            }
            last_delta = difference;
        }
        assert_eq!(last_delta, 0);

        let near = camera(bridge, 260.0);
        assert_eq!(
            state
                .prepare(near, 0, &sky, SIZE, [bridge], ground)
                .detail_opacity(bridge),
            1.0
        );
        let [a, b, c, d] = *bridge.sections();
        let view = View::new(far, SIZE);
        let projected = view.bridge(*bridge, 1.0, &sky, &mut |_, _| b[1]).unwrap();
        let [ramp, deck, exit, ..] = projected.strokes;
        for (stroke, start, end) in [(ramp, a, b), (deck, b, c), (exit, c, d)] {
            let stroke = stroke.unwrap();
            assert_eq!(stroke.start.p, view.vertex(start).unwrap().p);
            assert_eq!(stroke.end.p, view.vertex(end).unwrap().p);
        }
    }

    #[test]
    fn fitted_coastal_profile_is_continuous_faded_and_depth_tested() {
        let world = World::new(67);
        let bridge = Bridge::fit_sea_narrows(
            &world,
            [-991.8888, 7583.4556],
            [-895.02423, 7558.3716],
            |_, _| false,
            || {},
        )
        .unwrap();
        let far = camera(&bridge, 660.0);
        let projected = View::new(far, SIZE)
            .bridge(bridge, 1.0, &Sky::from_time(67, 0, 0.33), &mut |x, z| {
                world.elevation(x, z)
            })
            .unwrap();
        assert!(projected.strokes[3].is_some());
        verify(world, &bridge);
    }

    #[test]
    fn watershed_crossing_keeps_both_exact_dry_joins() {
        let world = World::new(98);
        let road = RoadNetwork::generate(&world, 1, 2).unwrap();
        let mut water = Watershed::new();
        water.rebuild(&world, 98, 1, 2, &mut WatershedWork::new());
        let plans = Crossings::fit(&world, &road, Some(&water), false);
        let bridge = plans.bridges().first().unwrap();
        verify(world, bridge);
    }
}
