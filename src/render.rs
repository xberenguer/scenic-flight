//! Terrain and atmosphere rendering into a caller-owned RGB565 framebuffer.

use core::f32::consts::TAU;

use crate::encounters::Meteor;
use crate::math::{sin_cos, smoothstep};
use crate::sky::Sky;
use crate::sky_events::{blend_meteor, streak};
use crate::world::{Sample, Surface, World};

/// A camera in world coordinates. Zero heading looks toward +Z.
/// Positive pitch looks down and positive bank rolls the camera to the right.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: f32,
    pub altitude: f32,
    pub z: f32,
    pub heading: f32,
    pub pitch: f32,
    pub bank: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    InvalidDimensions,
    BufferTooSmall,
    InvalidCamera,
    InvalidSky,
}

#[derive(Clone, Copy, Debug)]
pub enum RenderCheckpoint {
    Terrain,
    Rotation,
    Atmosphere,
}

#[derive(Clone, Copy, Debug)]
pub struct ScreenRect {
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
}

impl ScreenRect {
    pub const fn full(dimensions: (usize, usize)) -> Self {
        Self { left: 0, top: 0, right: dimensions.0, bottom: dimensions.1 }
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left), top: self.top.min(other.top),
            right: self.right.max(other.right), bottom: self.bottom.max(other.bottom),
        }
    }

    fn clamped(self, width: usize, height: usize) -> Self {
        let left = self.left.min(width);
        let top = self.top.min(height);
        Self {
            left, top,
            right: self.right.min(width).max(left), bottom: self.bottom.min(height).max(top),
        }
    }

    fn upright(self, view: &View, width: usize, height: usize) -> Self {
        let points = [(self.left, self.top), (self.right, self.top),
            (self.left, self.bottom), (self.right, self.bottom)].map(|(x, y)| {
                let dx = x as f32 - view.cx;
                let dy = y as f32 - view.cy;
                (view.cx + dx * view.roll_cos + dy * view.roll_sin,
                    view.cy - dx * view.roll_sin + dy * view.roll_cos)
            });
        let [a, b, c, d] = points;
        Self {
            left: (a.0.min(b.0).min(c.0).min(d.0) - 2.0) as usize,
            top: (a.1.min(b.1).min(c.1).min(d.1) - 2.0) as usize,
            right: (a.0.max(b.0).max(c.0).max(d.0) + 3.0) as usize,
            bottom: (a.1.max(b.1).max(c.1).max(d.1) + 3.0) as usize,
        }.clamped(width, height)
    }
}

/// Conservative pixel coverage of a world-space mesh's bounding box.
pub fn screen_region(camera: Camera, dimensions: (usize, usize), bounds: [[f32; 3]; 2]) -> Option<ScreenRect> {
    ScreenProjection::new(camera, dimensions)?.bounds(bounds)
}

/// Reuse the frame projection when collecting depth coverage for many pieces.
pub struct ScreenProjection {
    eye: [f32; 3],
    view: View,
    dimensions: (usize, usize),
}

impl ScreenProjection {
    pub fn new(camera: Camera, (width, height): (usize, usize)) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        Some(Self {
            eye: [camera.x, camera.altitude, camera.z],
            view: View::new(camera, width, height),
            dimensions: (width, height),
        })
    }

    pub(crate) fn backdrop_point(&self, [x, y, z]: [f32; 3]) -> Option<[f32; 3]> {
        let view = &self.view;
        let dx = x - self.eye[0];
        let dz = z - self.eye[2];
        let forward = dx * view.heading_sin + dz * view.heading_cos;
        let right = dx * view.heading_cos - dz * view.heading_sin;
        let squared = if forward > SCENERY_HORIZON {
            backdrop_curvature_squared(forward, right)
        } else {
            dx * dx + dz * dz
        };
        let relief = y - self.eye[1] - view.high_cruise * 0.000040 * squared;
        let down = -view.pitch_sin * forward - view.pitch_cos * relief;
        let depth = view.pitch_cos * forward - view.pitch_sin * relief;
        if depth < 0.8 || !depth.is_finite() {
            return None;
        }
        let scale = view.focal / depth;
        Some([
            view.cx + scale * (right * view.roll_cos - down * view.roll_sin),
            view.cy + scale * (down * view.roll_cos + right * view.roll_sin),
            depth,
        ])
    }

    pub fn bounds(&self, bounds: [[f32; 3]; 2]) -> Option<ScreenRect> {
        let (width, height) = self.dimensions;
        let view = &self.view;
        let curvature = view.high_cruise * 0.000040;
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut near = false;
        let mut count = 0;
        for x in [bounds[0][0], bounds[1][0]] {
            for y in [bounds[0][1], bounds[1][1]] {
                for z in [bounds[0][2], bounds[1][2]] {
                    let dx = x - self.eye[0];
                    let dz = z - self.eye[2];
                    let relief = y - self.eye[1] - curvature * (dx * dx + dz * dz);
                    let along = dx * view.heading_sin + dz * view.heading_cos;
                    let depth = along * view.pitch_cos - relief * view.pitch_sin;
                    if depth < 0.8 {
                        near = true;
                        continue;
                    }
                    count += 1;
                    let horizontal = dx * view.heading_cos - dz * view.heading_sin;
                    let vertical = -along * view.pitch_sin - relief * view.pitch_cos;
                    let scale = view.focal / depth;
                    let sx = view.cx + scale * (horizontal * view.roll_cos - vertical * view.roll_sin);
                    let sy = view.cy + scale * (horizontal * view.roll_sin + vertical * view.roll_cos);
                    min_x = min_x.min(sx);
                    min_y = min_y.min(sy);
                    max_x = max_x.max(sx);
                    max_y = max_y.max(sy);
                }
            }
        }
        if count == 0 { return None; }
        if near { return Some(ScreenRect::full(self.dimensions)); }
        let rect = ScreenRect {
            left: (min_x - 2.0) as usize, top: (min_y - 2.0) as usize,
            right: (max_x + 3.0) as usize, bottom: (max_y + 3.0) as usize,
        }.clamped(width, height);
        (rect.left < rect.right && rect.top < rect.bottom).then_some(rect)
    }
}

const MAX_SIDE: usize = 512;
const MAX_BANDS: usize = MAX_SIDE;
pub const LOW_VIEW_DISTANCE: f32 = 512.0;
pub const HIGH_VIEW_DISTANCE: f32 = 896.0;
const LOW_TERRAIN_DETAIL: f32 = 800.0;
const HIGH_TERRAIN_DETAIL: f32 = 1440.0;
/// Shared outer distance for terrain silhouettes and distant scenery.
pub const SCENERY_HORIZON: f32 = 1920.0;
/// Maximum reach of compact, streamed city silhouettes.
pub const CITY_HORIZON: f32 = 5000.0;
/// Maximum reach of the coarse, terrain-only background.
pub const TERRAIN_BACKDROP_DISTANCE: f32 = 10.0 * SCENERY_HORIZON;
type Color = [f32; 3];

pub(crate) fn encoded_depth(depth: f32) -> u16 {
    let units = if depth <= SCENERY_HORIZON {
        depth * 32.0
    } else {
        SCENERY_HORIZON * 32.0 + (depth - SCENERY_HORIZON) * 0.20
    };
    ((units + 0.5) as u16).min(u16::MAX - 1)
}

pub fn view_distance(altitude: f32) -> f32 {
    LOW_VIEW_DISTANCE + (HIGH_VIEW_DISTANCE - LOW_VIEW_DISTANCE) * high_cruise(altitude)
}

/// The heightfield can show roofs and roads farther away than the near meshes.
pub(crate) fn terrain_detail_distance(altitude: f32) -> f32 {
    LOW_TERRAIN_DETAIL + (HIGH_TERRAIN_DETAIL - LOW_TERRAIN_DETAIL) * aerial_view(altitude)
}

fn aerial_view(altitude: f32) -> f32 {
    smoothstep((altitude - 135.0) / 178.0)
}

fn high_cruise(altitude: f32) -> f32 {
    smoothstep((altitude - 244.0) / 80.0)
}

pub(crate) fn clear_cruise_visibility(altitude: f32, sun_up: f32, haze: f32) -> f32 {
    high_cruise(altitude) * clear_daylight_visibility(sun_up, haze)
}

fn clear_daylight_visibility(sun_up: f32, haze: f32) -> f32 {
    let high_sun = smoothstep((sun_up - 0.18) / 0.25);
    let clear_air = smoothstep((0.29 - haze) / 0.19);
    high_sun * clear_air
}

/// Render a circular viewport in steady daylight. Extra buffer space is preserved.
pub fn render(
    world: &World,
    camera: Camera,
    pixels: &mut [u16],
    width: usize,
    height: usize,
) -> Result<(), RenderError> {
    render_from(|x, z| world.sample(x, z), camera, pixels, width, height)
}

/// Render with time-dependent light and weather.
pub fn render_with_sky(
    world: &World,
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    width: usize,
    height: usize,
) -> Result<(), RenderError> {
    render_from_with_sky(
        |x, z| world.sample(x, z),
        camera,
        sky,
        pixels,
        width,
        height,
    )
}

/// Render from a terrain sampler, such as a world-aligned terrain cache.
pub fn render_from(
    sample: impl FnMut(f32, f32) -> Sample,
    camera: Camera,
    pixels: &mut [u16],
    width: usize,
    height: usize,
) -> Result<(), RenderError> {
    render_inner(sample, unchanged_material, camera, Atmosphere::DAY, pixels, None, None, width, height, |_| {}, |_| None)
}

pub fn render_from_with_sky(
    sample: impl FnMut(f32, f32) -> Sample,
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    width: usize,
    height: usize,
) -> Result<(), RenderError> {
    render_inner(
        sample,
        unchanged_material,
        camera,
        sky_atmosphere(sky)?,
        pixels,
        None,
        None,
        width,
        height,
        |_| {},
        |_| None,
    )
}

/// Render from a cached sampler with an optional full-size RGB565 work surface for bank.
/// Terrain spans are drawn upright first, then rolled with one integer scan of the viewport.
pub fn render_from_with_sky_buffered(
    sample: impl FnMut(f32, f32) -> Sample,
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    terrain_work: &mut [u16],
    width: usize,
    height: usize,
) -> Result<(), RenderError> {
    render_buffered_observed(sample, camera, sky, pixels, terrain_work, (width, height), |_| {})
}

/// Report the CPU boundaries of the same buffered renderer used during ordinary flights.
pub fn render_buffered_observed(
    sample: impl FnMut(f32, f32) -> Sample,
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    terrain_work: &mut [u16],
    dimensions: (usize, usize),
    checkpoint: impl FnMut(RenderCheckpoint),
) -> Result<(), RenderError> {
    render_buffered_with_depth(sample, camera, sky, pixels, terrain_work, None, dimensions, checkpoint)
}

/// Render terrain and retain its visible surface distances for scenery meshes.
pub fn render_with_depth(
    sample: impl FnMut(f32, f32) -> Sample,
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    terrain_work: &mut [u16],
    depth: &mut [u16],
    dimensions: (usize, usize),
) -> Result<(), RenderError> {
    render_buffered_with_depth(sample, camera, sky, pixels, terrain_work, Some(depth), dimensions, |_| {})
}

/// Optional depth lets ordinary flights pay for mesh occlusion only when needed.
pub fn render_buffered_with_depth(
    sample: impl FnMut(f32, f32) -> Sample,
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    terrain_work: &mut [u16],
    depth: Option<&mut [u16]>,
    dimensions: (usize, usize),
    checkpoint: impl FnMut(RenderCheckpoint),
) -> Result<(), RenderError> {
    render_terrain_layers((sample, unchanged_material), camera, sky, pixels, terrain_work, depth, dimensions, checkpoint)
}

/// Apply overlays that only change color after the ray produces a visible span.
pub fn render_terrain_layers(
    (sample, material): (impl FnMut(f32, f32) -> Sample, impl FnMut(f32, f32, Sample) -> Surface),
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    terrain_work: &mut [u16],
    depth: Option<&mut [u16]>,
    dimensions: (usize, usize),
    checkpoint: impl FnMut(RenderCheckpoint),
) -> Result<(), RenderError> {
    render_terrain_layer_region((sample, material), camera, sky, pixels, terrain_work,
        depth.map(|buffer| (buffer, ScreenRect::full(dimensions))), dimensions, checkpoint)
}

/// Retain ground depth only where the current scenery can occupy screen pixels.
pub fn render_terrain_layer_region(
    (sample, material): (impl FnMut(f32, f32) -> Sample, impl FnMut(f32, f32, Sample) -> Surface),
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    terrain_work: &mut [u16],
    depth: Option<(&mut [u16], ScreenRect)>,
    dimensions: (usize, usize),
    checkpoint: impl FnMut(RenderCheckpoint),
) -> Result<(), RenderError> {
    render_terrain_layer_region_with_sky(
        (sample, material), camera, sky, pixels, terrain_work, depth, dimensions, checkpoint,
        |_| None,
    )
}

/// A ready tile callback may return None, in which case the exact ordinary shader runs.
pub fn render_terrain_layer_region_with_sky(
    (sample, material): (impl FnMut(f32, f32) -> Sample, impl FnMut(f32, f32, Sample) -> Surface),
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    terrain_work: &mut [u16],
    depth: Option<(&mut [u16], ScreenRect)>,
    dimensions: (usize, usize),
    checkpoint: impl FnMut(RenderCheckpoint),
    ready_sky_tile: impl FnMut(usize) -> Option<u16>,
) -> Result<(), RenderError> {
    render_inner(
        sample, material, camera, sky_atmosphere(sky)?, pixels, Some(terrain_work), depth,
        dimensions.0, dimensions.1, checkpoint, ready_sky_tile,
    )
}

/// The normal near/depth/scenery pass plus an optional coarse natural horizon.
/// A second frame in `terrain_work` can hold upright depth for full-screen scenery.
#[inline(never)]
pub fn render_terrain_layer_region_with_horizon(
    (sample, material): (
        impl FnMut(f32, f32) -> Sample,
        impl FnMut(f32, f32, Sample) -> Surface,
    ),
    mut ground_horizon: impl FnMut(f32, f32) -> f32,
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    terrain_work: &mut [u16],
    depth: Option<(&mut [u16], ScreenRect)>,
    dimensions: (usize, usize),
    checkpoint: impl FnMut(RenderCheckpoint),
    ready_sky_tile: impl FnMut(usize) -> Option<u16>,
) -> Result<(), RenderError> {
    render_inner_with_horizon(
        sample,
        material,
        camera,
        sky_atmosphere(sky)?,
        pixels,
        Some(terrain_work),
        depth,
        dimensions.0,
        dimensions.1,
        checkpoint,
        ready_sky_tile,
        Some(&mut ground_horizon),
    )
}

pub fn horizon_distance(_altitude: f32) -> f32 {
    SCENERY_HORIZON
}

pub(crate) fn backdrop_curve_distance_squared(forward: f32) -> f32 {
    // Continue the foreground curve into the backdrop, easing its slope.
    let beyond = forward - SCENERY_HORIZON;
    if beyond <= 0.0 {
        forward * forward
    } else {
        SCENERY_HORIZON * SCENERY_HORIZON
            + 2.0 * SCENERY_HORIZON * beyond / (1.0 + beyond / 900.0)
    }
}

/// Match a far landmark to the curve used by the terrain backdrop.
pub(crate) fn backdrop_curvature_squared(forward: f32, side: f32) -> f32 {
    if forward <= SCENERY_HORIZON {
        forward * forward + side * side
    } else {
        let lateral = side / forward;
        backdrop_curve_distance_squared(forward) * (1.0 + lateral * lateral)
    }
}

fn unchanged_material(_: f32, _: f32, sample: Sample) -> Surface {
    sample.surface
}

fn sky_atmosphere(sky: &Sky) -> Result<Atmosphere, RenderError> {
    let inputs = [
        sky.sun_direction[0],
        sky.sun_direction[1],
        sky.sun_direction[2],
        sky.moon_direction[0],
        sky.moon_direction[1],
        sky.moon_direction[2],
        sky.daylight,
        sky.haze,
        sky.cloud_cover,
        sky.cloud_base,
        sky.cloud_offset[0],
        sky.cloud_offset[1],
    ];
    if !inputs.iter().all(|value| value.is_finite()) {
        return Err(RenderError::InvalidSky);
    }
    Ok(Atmosphere {
        sun: sky.sun_direction,
        moon: sky.moon_direction,
        daylight: sky.daylight.clamp(0.0, 1.0),
        haze: sky.haze.clamp(0.0, 1.0),
        clouds: sky.cloud_cover.clamp(0.0, 1.0),
        cloud_base: sky.weather.map_or(0.0, |_| sky.cloud_base.clamp(50.0, 210.0)),
        cloud_offset: sky.cloud_offset,
        meteor: sky.meteor,
        #[cfg(feature = "ground-cover")]
        ground_seed: sky.ground_seed,
    })
}

#[derive(Clone, Copy)]
struct Atmosphere {
    sun: [f32; 3],
    moon: [f32; 3],
    daylight: f32,
    haze: f32,
    clouds: f32,
    cloud_base: f32,
    cloud_offset: [f32; 2],
    meteor: Option<Meteor>,
    #[cfg(feature = "ground-cover")]
    ground_seed: u32,
}

impl Atmosphere {
    const DAY: Self = Self {
        sun: [-0.55, 0.72, 0.42],
        moon: [0.0, -1.0, 0.0],
        daylight: 1.0,
        haze: 0.38,
        clouds: 0.29,
        cloud_base: 0.0,
        cloud_offset: [0.0, 0.0],
        meteor: None,
        #[cfg(feature = "ground-cover")]
        ground_seed: 0,
    };

    fn twilight(self) -> f32 {
        let elevation = self.sun[1];
        let sunrise = smoothstep((elevation + 0.23) / 0.22);
        let sunset = 1.0 - smoothstep((elevation - 0.01) / 0.32);
        sunrise * sunset
    }

    fn horizon(self) -> Color {
        let blue = mix([25.0, 37.0, 63.0], [175.0, 207.0, 232.0], self.daylight);
        mix(blue, [224.0, 145.0, 119.0], self.twilight() * 0.63)
    }

    fn moonlight(self) -> f32 {
        let [x, y, z] = self.moon;
        let phase = ((1.0 - (x * self.sun[0] + y * self.sun[1] + z * self.sun[2])) * 0.5)
            .clamp(0.0, 1.0);
        phase * phase * smoothstep(y / 0.60) * (1.0 - self.daylight)
            * (1.0 - self.clouds * 0.52)
    }
}

#[derive(Clone, Copy)]
struct View {
    focal: f32,
    cx: f32,
    cy: f32,
    heading_sin: f32,
    heading_cos: f32,
    pitch_sin: f32,
    pitch_cos: f32,
    roll_sin: f32,
    roll_cos: f32,
    high_cruise: f32,
    cloud_origin: [f32; 2],
}

impl View {
    fn new(camera: Camera, width: usize, height: usize) -> Self {
        let (heading_sin, heading_cos) = sin_cos(camera.heading);
        let (pitch_sin, pitch_cos) = sin_cos(camera.pitch.clamp(-1.3, 1.3));
        let (roll_sin, roll_cos) = sin_cos(-camera.bank);
        Self {
            focal: width.min(height) as f32 * 0.73,
            cx: (width - 1) as f32 * 0.5,
            cy: (height - 1) as f32 * 0.5,
            heading_sin,
            heading_cos,
            pitch_sin,
            pitch_cos,
            roll_sin,
            roll_cos,
            high_cruise: high_cruise(camera.altitude),
            cloud_origin: [camera.x * 0.012, camera.z * 0.012],
        }
    }
}

struct Aperture {
    left: [u16; MAX_SIDE],
    right: [u16; MAX_SIDE],
    diameter: usize,
}

impl Aperture {
    fn new(width: usize, height: usize) -> Self {
        let diameter = width.min(height);
        let mut result = Self {
            left: [0; MAX_SIDE],
            right: [0; MAX_SIDE],
            diameter,
        };
        for row in 0..height {
            (result.left[row], result.right[row]) = aperture_row(width, height, diameter, row);
        }
        result
    }
}

fn aperture_row(width: usize, height: usize, diameter: usize, row: usize) -> (u16, u16) {
    let dy = (2 * row + 1) as i32 - height as i32;
    let inside = (diameter * diameter) as i32 - dy * dy;
    if inside < 0 {
        return (0, 0);
    }
    let half = square_root(inside as u32) as usize;
    (((width - half) / 2) as u16, ((width - 1 + half) / 2 + 1) as u16)
}

const fn sky_tile_step(diameter: usize) -> usize {
    let stride = diameter.div_ceil(96);
    if stride < 2 { 2 } else { stride }
}

fn camera_is_finite(camera: Camera) -> bool {
    [
        camera.x,
        camera.altitude,
        camera.z,
        camera.heading,
        camera.pitch,
        camera.bank,
    ]
    .iter()
    .all(|value| value.is_finite())
}

fn render_inner(
    sample: impl FnMut(f32, f32) -> Sample,
    material: impl FnMut(f32, f32, Sample) -> Surface,
    camera: Camera,
    atmosphere: Atmosphere,
    pixels: &mut [u16],
    work: Option<&mut [u16]>,
    depth: Option<(&mut [u16], ScreenRect)>,
    width: usize,
    height: usize,
    checkpoint: impl FnMut(RenderCheckpoint),
    ready_sky_tile: impl FnMut(usize) -> Option<u16>,
) -> Result<(), RenderError> {
    render_inner_with_horizon(
        sample, material, camera, atmosphere, pixels, work, depth, width, height, checkpoint,
        ready_sky_tile, None,
    )
}

#[inline(never)]
fn render_inner_with_horizon(
    mut sample: impl FnMut(f32, f32) -> Sample,
    mut material: impl FnMut(f32, f32, Sample) -> Surface,
    camera: Camera,
    atmosphere: Atmosphere,
    pixels: &mut [u16],
    work: Option<&mut [u16]>,
    mut depth: Option<(&mut [u16], ScreenRect)>,
    width: usize,
    height: usize,
    mut checkpoint: impl FnMut(RenderCheckpoint),
    ready_sky_tile: impl FnMut(usize) -> Option<u16>,
    mut ground_horizon: Option<&mut dyn FnMut(f32, f32) -> f32>,
) -> Result<(), RenderError> {
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(RenderError::InvalidDimensions);
    }
    let count = width * height;
    if pixels.len() < count || work.as_ref().is_some_and(|buffer| buffer.len() < count)
        || depth.as_ref().is_some_and(|(buffer, _)| buffer.len() < count)
    {
        return Err(RenderError::BufferTooSmall);
    }
    if !camera_is_finite(camera) {
        return Err(RenderError::InvalidCamera);
    }

    let pixels = &mut pixels[..count];
    pixels.fill(0);
    depth = depth.map(|(depth, region)| {
        let depth = &mut depth[..count];
        depth.fill(u16::MAX);
        (depth, region.clamped(width, height))
    });
    let aperture = Aperture::new(width, height);
    let view = View::new(camera, width, height);
    let horizon = atmosphere.horizon();

    let rolled = view.roll_sin.abs() >= 0.000_001 || view.roll_cos < 0.0;
    match work.filter(|_| rolled) {
        Some(work) => {
            let (work, spare) = work.split_at_mut(count);
            work.fill(0);
            let upright = View {
                roll_sin: 0.0,
                roll_cos: 1.0,
                ..view
            };
            let mut separate_depth = depth.as_ref().and_then(|(_, rect)| {
                if rect.left == 0 && rect.top == 0 && rect.right == width && rect.bottom == height {
                    spare.get_mut(..count)
                } else {
                    None
                }
            });
            if let Some(scratch) = &mut separate_depth {
                scratch.fill(u16::MAX);
            }
            let upright_depth = depth.as_mut().map(|(buffer, region)| {
                let buffer = separate_depth.as_deref_mut().unwrap_or(&mut **buffer);
                (buffer, region.upright(&view, width, height))
            });
            let upright_region = upright_depth.as_ref().map(|(_, rect)| *rect);
            let first_ground = draw_terrain::<false>(
                &mut sample, &mut material, camera, atmosphere, horizon, work, upright_depth,
                width, height, &aperture, &upright, ground_horizon.take(),
            );
            checkpoint(RenderCheckpoint::Terrain);
            let full = ScreenRect::full((width, height));
            if let (Some(scratch), Some((depth, rect))) = (separate_depth, depth.as_mut()) {
                if rotation_copies_runs(&view, &aperture) {
                    rotate_terrain::<true, true>(
                        work, pixels, width, height, &aperture, &view, *rect, first_ground, Some((scratch, depth)),
                    );
                } else {
                    rotate_terrain::<true, false>(
                        work, pixels, width, height, &aperture, &view, full, first_ground, Some((scratch, depth)),
                    );
                }
            } else {
                rotate_terrain::<false, false>(work, pixels, width, height, &aperture, &view, full, first_ground, None);
                if let (Some((depth, rect)), Some(upright_region)) = (depth, upright_region) {
                    for row in rect.top..rect.bottom {
                        work[row * width + rect.left..row * width + rect.right].fill(u16::MAX);
                    }
                    rotate_terrain::<false, false>(depth, work, width, height, &aperture, &view, rect, first_ground, None);
                    // The rest is still initialized, and the output rectangle
                    // is about to be overwritten by the rotated depth.
                    clear_depth_outside_output(depth, width, upright_region, rect);
                    for row in rect.top..rect.bottom {
                        let span = row * width + rect.left..row * width + rect.right;
                        depth[span.clone()].copy_from_slice(&work[span]);
                    }
                }
            }
        }
        None => {
            draw_terrain::<true>(
                &mut sample, &mut material, camera, atmosphere, horizon, pixels, depth,
                width, height, &aperture, &view,
                ground_horizon.take(),
            );
            checkpoint(RenderCheckpoint::Terrain);
        }
    }
    checkpoint(RenderCheckpoint::Rotation);
    if ground_cloud_presence(atmosphere, camera) > 0.0 {
        paint_ground_clouds(&mut sample, camera, atmosphere, &view, &aperture, pixels, width, height);
    }
    let stars = visible_stars(pixels, width, height, &aperture, &view, atmosphere);
    let moon = MoonPatch::new(pixels, width, height, &aperture, &view, atmosphere);
    const PIXEL_MASK: u32 = (1 << 18) - 1;
    let mut meteor_pixels = [0; 160];
    let mut trail = 0;
    if let Some(event) = atmosphere.meteor {
        streak(event, camera, width, height, |x, y, opacity| {
            let index = y * width + x;
            if pixels[index] == 0 && trail < meteor_pixels.len() {
                let alpha = (opacity.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
                meteor_pixels[trail] = index as u32 | (alpha << 18);
                trail += 1;
            }
        });
    }
    paint_sky(pixels, width, height, &aperture, &view, atmosphere, horizon, ready_sky_tile);
    paint_stars(pixels, &stars, &view, atmosphere);
    if let Some(moon) = moon {
        moon.paint(pixels, width, &view, atmosphere);
    }
    for packed in &meteor_pixels[..trail] {
        let index = (packed & PIXEL_MASK) as usize;
        let opacity = (packed >> 18) as f32 * (1.0 / 255.0);
        pixels[index] = blend_meteor(pixels[index], opacity);
    }
    checkpoint(RenderCheckpoint::Atmosphere);
    Ok(())
}

/// Execute each terrain half to completion before returning. The worker owns
/// its callback and any uniquely borrowed scratch until the join completes.
#[cfg(feature = "terrain-fork")]
#[path = "render_packed.rs"]
mod packed;
#[cfg(feature = "terrain-fork")]
pub use packed::{render_with_packed_terrain_fork, render_with_packed_terrain_fork_sized};

#[cfg(feature = "terrain-fork")]
#[derive(Clone, Copy)]
pub enum TerrainForkPhase { UprightPrepared, SlabCleared, NearJoined, SlabMerged, HorizonDone }

#[cfg(feature = "terrain-fork")]
pub trait TerrainFork {
    fn fork(&mut self, left: impl FnOnce(), right: impl FnOnce() + Send);
    fn terrain_checkpoint(&mut self, _phase: TerrainForkPhase) {}
}

#[cfg(feature = "terrain-fork")]
#[inline(never)]
pub fn render_with_terrain_fork(
    executor: &mut impl TerrainFork,
    mut sample: impl FnMut(f32, f32) -> Sample,
    mut material: impl FnMut(f32, f32, Sample) -> Surface,
    mut worker_sample: impl FnMut(f32, f32) -> Sample + Send,
    mut worker_material: impl FnMut(f32, f32, Sample) -> Surface + Send,
    (right_color, right_depth): (&mut [u16], &mut [u16]),
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    work: Option<&mut [u16]>,
    mut depth: Option<(&mut [u16], ScreenRect)>,
    width: usize,
    height: usize,
    mut checkpoint: impl FnMut(RenderCheckpoint),
    ready_sky_tile: impl FnMut(usize) -> Option<u16>,
    mut ground_horizon: Option<&mut dyn FnMut(f32, f32) -> f32>,
) -> Result<(), RenderError> {
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(RenderError::InvalidDimensions);
    }
    let count = width * height;
    if pixels.len() < count || work.as_ref().is_some_and(|buffer| buffer.len() < count)
        || depth.as_ref().is_some_and(|(buffer, _)| buffer.len() < count)
    {
        return Err(RenderError::BufferTooSmall);
    }
    if !camera_is_finite(camera) {
        return Err(RenderError::InvalidCamera);
    }

    let atmosphere = sky_atmosphere(sky)?;
    let pixels = &mut pixels[..count];
    pixels.fill(0);
    depth = depth.map(|(depth, region)| {
        let depth = &mut depth[..count];
        depth.fill(u16::MAX);
        (depth, region.clamped(width, height))
    });
    let aperture = Aperture::new(width, height);
    let view = View::new(camera, width, height);
    let horizon = atmosphere.horizon();

    let rolled = view.roll_sin.abs() >= 0.000_001 || view.roll_cos < 0.0;
    match work.filter(|_| rolled) {
        Some(work) => {
            let (work, spare) = work.split_at_mut(count);
            work.fill(0);
            let upright = View {
                roll_sin: 0.0,
                roll_cos: 1.0,
                ..view
            };
            let mut separate_depth = depth.as_ref().and_then(|(_, rect)| {
                if rect.left == 0 && rect.top == 0 && rect.right == width && rect.bottom == height {
                    spare.get_mut(..count)
                } else {
                    None
                }
            });
            if let Some(scratch) = &mut separate_depth {
                scratch.fill(u16::MAX);
            }
            let upright_depth = depth.as_mut().map(|(buffer, region)| {
                let buffer = separate_depth.as_deref_mut().unwrap_or(&mut **buffer);
                (buffer, region.upright(&view, width, height))
            });
            let upright_region = upright_depth.as_ref().map(|(_, rect)| *rect);
            executor.terrain_checkpoint(TerrainForkPhase::UprightPrepared);
            let first_ground = if width == 466 && height == 466 {
                draw_joined_bands(
                    executor, &mut sample, &mut material, &mut worker_sample, &mut worker_material,
                    camera, atmosphere, horizon, work, upright_depth, &aperture, &upright,
                    ground_horizon.take(), (right_color, right_depth),
                )
            } else {
                draw_terrain::<false>(
                    &mut sample, &mut material, camera, atmosphere, horizon, work, upright_depth,
                    width, height, &aperture, &upright, ground_horizon.take(),
                )
            };
            checkpoint(RenderCheckpoint::Terrain);
            let full = ScreenRect::full((width, height));
            if let (Some(scratch), Some((depth, rect))) = (separate_depth, depth.as_mut()) {
                if rotation_copies_runs(&view, &aperture) {
                    rotate_terrain::<true, true>(
                        work, pixels, width, height, &aperture, &view, *rect, first_ground, Some((scratch, depth)),
                    );
                } else {
                    rotate_terrain::<true, false>(
                        work, pixels, width, height, &aperture, &view, full, first_ground, Some((scratch, depth)),
                    );
                }
            } else {
                rotate_terrain::<false, false>(work, pixels, width, height, &aperture, &view, full, first_ground, None);
                if let (Some((depth, rect)), Some(upright_region)) = (depth, upright_region) {
                    for row in rect.top..rect.bottom {
                        work[row * width + rect.left..row * width + rect.right].fill(u16::MAX);
                    }
                    rotate_terrain::<false, false>(depth, work, width, height, &aperture, &view, rect, first_ground, None);
                    // The rest is still initialized, and the output rectangle
                    // is about to be overwritten by the rotated depth.
                    clear_depth_outside_output(depth, width, upright_region, rect);
                    for row in rect.top..rect.bottom {
                        let span = row * width + rect.left..row * width + rect.right;
                        depth[span.clone()].copy_from_slice(&work[span]);
                    }
                }
            }
        }
        None => {
            draw_terrain::<true>(
                &mut sample, &mut material, camera, atmosphere, horizon, pixels, depth,
                width, height, &aperture, &view,
                ground_horizon.take(),
            );
            checkpoint(RenderCheckpoint::Terrain);
        }
    }
    checkpoint(RenderCheckpoint::Rotation);
    if ground_cloud_presence(atmosphere, camera) > 0.0 {
        paint_ground_clouds(&mut sample, camera, atmosphere, &view, &aperture, pixels, width, height);
    }
    let stars = visible_stars(pixels, width, height, &aperture, &view, atmosphere);
    let moon = MoonPatch::new(pixels, width, height, &aperture, &view, atmosphere);
    const PIXEL_MASK: u32 = (1 << 18) - 1;
    let mut meteor_pixels = [0; 160];
    let mut trail = 0;
    if let Some(event) = atmosphere.meteor {
        streak(event, camera, width, height, |x, y, opacity| {
            let index = y * width + x;
            if pixels[index] == 0 && trail < meteor_pixels.len() {
                let alpha = (opacity.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
                meteor_pixels[trail] = index as u32 | (alpha << 18);
                trail += 1;
            }
        });
    }
    paint_sky(pixels, width, height, &aperture, &view, atmosphere, horizon, ready_sky_tile);
    paint_stars(pixels, &stars, &view, atmosphere);
    if let Some(moon) = moon {
        moon.paint(pixels, width, &view, atmosphere);
    }
    for packed in &meteor_pixels[..trail] {
        let index = (packed & PIXEL_MASK) as usize;
        let opacity = (packed >> 18) as f32 * (1.0 / 255.0);
        pixels[index] = blend_meteor(pixels[index], opacity);
    }
    checkpoint(RenderCheckpoint::Atmosphere);
    Ok(())
}

#[cfg(feature = "terrain-fork")]
#[inline(never)]
fn draw_joined_bands(
    executor: &mut impl TerrainFork,
    sample: impl FnMut(f32, f32) -> Sample,
    material: impl FnMut(f32, f32, Sample) -> Surface,
    worker_sample: impl FnMut(f32, f32) -> Sample + Send,
    worker_material: impl FnMut(f32, f32, Sample) -> Surface + Send,
    camera: Camera,
    atmosphere: Atmosphere,
    horizon: Color,
    pixels: &mut [u16],
    mut depth: Option<(&mut [u16], ScreenRect)>,
    aperture: &Aperture,
    view: &View,
    ground_horizon: Option<&mut dyn FnMut(f32, f32) -> f32>,
    (right_color, right_depth): (&mut [u16], &mut [u16]),
) -> usize {
    const SIDE: usize = 466;
    const SPLIT: usize = 232;
    const RIGHT: usize = SIDE - SPLIT;
    let mut edge = [SIDE as i16; MAX_BANDS];
    let mut right_edge = edge;
    let rect = depth.as_ref().map(|(_, rect)| *rect);
    let right_color = &mut right_color[..RIGHT * SIDE];
    let right_depth = &mut right_depth[..RIGHT * SIDE];
    right_color.fill(0);
    if rect.is_some() {
        right_depth.fill(u16::MAX);
    }

    executor.terrain_checkpoint(TerrainForkPhase::SlabCleared);
    let color_target = &mut *right_color;
    let depth_target = rect.map(|rect| (&mut *right_depth, rect));
    executor.fork(
        || draw_near_bands::<false>(
            sample, material, camera, atmosphere, horizon, pixels,
            depth.as_mut().map(|(depth, rect)| (&mut **depth, *rect)), SIDE, SIDE,
            aperture, view, &mut edge,
            TerrainBands { first: 0, end: SPLIT, write_from: 0, row_width: SIDE, origin_x: 0 },
        ),
        || draw_near_bands::<false>(
            worker_sample, worker_material, camera, atmosphere, horizon, color_target,
            depth_target, SIDE, SIDE, aperture, view, &mut right_edge,
            TerrainBands { first: SPLIT - 2, end: SIDE, write_from: SPLIT, row_width: RIGHT, origin_x: SPLIT },
        ),
    );

    executor.terrain_checkpoint(TerrainForkPhase::NearJoined);
    debug_assert_eq!(&edge[SPLIT - 2..SPLIT], &right_edge[SPLIT - 2..SPLIT]);
    edge[SPLIT..SIDE].copy_from_slice(&right_edge[SPLIT..SIDE]);
    for (row, source) in pixels.chunks_exact_mut(SIDE).zip(right_color.chunks_exact(RIGHT)) {
        row[SPLIT..].copy_from_slice(source);
    }
    if let Some((depth, _)) = &mut depth {
        for (row, source) in depth.chunks_exact_mut(SIDE).zip(right_depth.chunks_exact(RIGHT)) {
            row[SPLIT..].copy_from_slice(source);
        }
    }
    executor.terrain_checkpoint(TerrainForkPhase::SlabMerged);
    if let Some(ground_horizon) = ground_horizon.filter(|_| edge[..SIDE].iter().any(|&row| row > 0)) {
        draw_far_horizon::<false>(
            ground_horizon, camera, atmosphere, horizon, view, aperture, &mut edge[..SIDE],
            1, pixels, depth, SIDE, SIDE,
        );
    }
    executor.terrain_checkpoint(TerrainForkPhase::HorizonDone);
    edge[..SIDE].iter().copied().min().unwrap_or(SIDE as i16) as usize
}

fn distant_stride_growth(diameter: usize) -> f32 {
    if cfg!(feature = "small-raster-ray-budget") && diameter <= 112 {
        if diameter <= 36 {
            // In the emergency tier, the distant terrain needs fewer
            // samples per pixel. The near pass and far horizon stay intact.
            2.20
        } else if diameter <= 48 {
            1.05
        } else if diameter == 54 {
            0.875
        } else {
            0.70 + ((diameter as f32 - 64.0) / 24.0).clamp(0.0, 0.35)
        }
    } else {
        0.0
    }
}

#[inline]
fn next_terrain_distance(distance: f32, far: f32, high_cruise: f32, stride_growth: f32) -> f32 {
    let wide = ((far - LOW_TERRAIN_DETAIL) / (HIGH_TERRAIN_DETAIL - LOW_TERRAIN_DETAIL))
        .clamp(0.0, 1.0);
    // At altitude spend the recovered foreground samples on streets and
    // roof geometry just below the aircraft. Widen the steps past that band.
    let outer = smoothstep((distance - 455.0) / 220.0);
    let low_outer = smoothstep((distance - 390.0) / 195.0);
    let stride = (1.8 + distance * (0.064 + high_cruise * 0.018))
        * (1.0 + wide * outer * 1.40 + (1.0 - wide) * low_outer * 0.90)
        * (1.0 + stride_growth * smoothstep((distance - 60.0) / 60.0)
            * (1.0 - high_cruise * 0.35 * (1.0 - outer)));
    (distance + stride).min(far)
}

fn first_terrain_distance(altitude: f32) -> f32 {
    // Even the tallest modeled roofs are well below the high-cruise camera
    // at this distance. Lower aircraft still sample the foreground.
    5.0 + 28.0 * high_cruise(altitude)
}

#[cfg(feature = "terrain-fork")]
#[derive(Clone, Copy)]
struct TerrainBands {
    first: usize,
    end: usize,
    write_from: usize,
    row_width: usize,
    origin_x: usize,
}

#[cfg(feature = "terrain-fork")]
#[inline(never)]
fn draw_near_bands<const CLIPPED: bool>(
    mut sample: impl FnMut(f32, f32) -> Sample,
    mut material: impl FnMut(f32, f32, Sample) -> Surface,
    camera: Camera,
    atmosphere: Atmosphere,
    horizon: Color,
    pixels: &mut [u16],
    mut depth: Option<(&mut [u16], ScreenRect)>,
    width: usize,
    height: usize,
    aperture: &Aperture,
    view: &View,
    edge: &mut [i16; MAX_BANDS],
    work: TerrainBands,
) {
    let left = (width - aperture.diameter) / 2;
    let right = left + aperture.diameter;
    let top = ((height - aperture.diameter) / 2) as i32;
    let bottom = top + aperture.diameter as i32;
    let stride = aperture.diameter.div_ceil(MAX_BANDS);
    let mut visible_rows = [[0_i16; 2]; MAX_BANDS];
    if !CLIPPED {
        // Nearest-neighbor rotation can read at most one extra source pixel
        // beyond the circle. Include two for fixed-point and angle error.
        let radius = aperture.diameter as i32 + 4;
        let cx2 = width as i32 - 1;
        let cy2 = height as i32 - 1;
        for (band, rows) in visible_rows.iter_mut().enumerate().take(work.end).skip(work.first) {
            let x0 = (left + band * stride) as i32;
            let x1 = (left + (band + 1) * stride).min(right) as i32;
            let dx = (2 * x0 - cx2).max(cx2 - 2 * (x1 - 1)).max(0);
            let reach = square_root((radius * radius - dx * dx) as u32) as i32;
            *rows = [((cy2 - reach + 1) / 2).max(top) as i16,
                ((cy2 + reach) / 2 + 1).min(bottom) as i16];
        }
    }
    let mut slope_height = [0.0; MAX_BANDS];
    let mut slope_center = [0_u16; MAX_BANDS];
    let mut open = work.end - work.first;
    let stride_growth = distant_stride_growth(aperture.diameter);
    let mut distance = first_terrain_distance(camera.altitude);
    let mut previous = 0.0;
    let sun_forward = atmosphere.sun[0] * view.heading_sin + atmosphere.sun[2] * view.heading_cos;
    let sun_right = atmosphere.sun[0] * view.heading_cos - atmosphere.sun[2] * view.heading_sin;
    let sun_up = atmosphere.sun[1];
    let glint = if sun_forward > 0.40 && (0.0..0.38).contains(&sun_up) {
        smoothstep(sun_up / 0.08) * (1.0 - smoothstep((sun_up - 0.14) / 0.24)) * 0.43
    } else { 0.0 };
    let reflection = if glint > 0.0 { sun_up / sun_forward } else { 0.0 };
    let moonlight = atmosphere.moonlight();
    let [light_x, light_up, light_z] = mix(atmosphere.sun, atmosphere.moon, moonlight);
    let light_forward = light_x * view.heading_sin + light_z * view.heading_cos;
    let light_right = light_x * view.heading_cos - light_z * view.heading_sin;
    let ambient = 0.26 + atmosphere.daylight * 0.74 + moonlight * 0.16;
    let far = terrain_detail_distance(camera.altitude);
    let clear_contrast = 0.145 * clear_cruise_visibility(camera.altitude, atmosphere.sun[1], atmosphere.haze);
    let curvature = view.high_cruise * 0.000040;
    let curved = curvature != 0.0;
    let inverse_focal = 1.0 / view.focal;

    while distance <= far && open != 0 {
        let forward_x = camera.x + view.heading_sin * distance;
        let forward_z = camera.z + view.heading_cos * distance;
        let spacing = distance / view.focal;
        let step_x = view.heading_cos * spacing;
        let step_z = -view.heading_sin * spacing;
        let along_up = view.pitch_sin * distance;
        let along_camera = view.pitch_cos * distance;
        let sea_depth = along_camera - view.pitch_sin * -camera.altitude;
        let sea_row = if curved {
            -1
        } else if sea_depth < 0.5 {
            bottom
        } else {
            let sea_up = view.pitch_cos * -camera.altitude + along_up;
            ((view.cy - view.focal * sea_up / sea_depth + 0.5) as i32).max(top)
        };
        let inverse_step = 1.0 / (distance - previous);
        let previous_spacing = previous * inverse_focal;
        let distance_fraction = (distance / far).min(1.0);
        let detail_fade = (1.0 - distance / 150.0).clamp(0.0, 1.0);
        let curve = curvature * distance * distance;
        let glint_width = if glint > 0.0 { 1.0 / (3.0 + distance * 0.047) } else { 0.0 };
        // Clear air above the weather shows more surface color on distant ground.
        let fog = (distance_fraction * (0.42 + atmosphere.haze * 0.64)
            + distance_fraction * distance_fraction * 0.38).min(0.97)
            - clear_contrast * smoothstep((distance_fraction - 0.44) / 0.56);
        let mut neighbor = None;
        let mut next_band = work.first;
        let merge_distance = if view.high_cruise > 0.57 { 116.0 } else { 147.0 };
        while next_band < work.end {
            let band = next_band;
            // A distant, nearly level silhouette can share a sample across two
            // narrow columns. Keep individual columns at steep edges.
            let paired = band.is_multiple_of(2)
                && distance > merge_distance
                && band + 1 < work.end
                && edge[band].abs_diff(edge[band + 1]) <= 1;
            next_band += if paired { 2 } else { 1 };
            let limit = i32::from(edge[band].max(edge[next_band - 1]));
            if limit == top {
                neighbor = None;
                continue;
            }
            let x0 = left + band * stride;
            let x1 = (left + next_band * stride).min(right);
            let center = (x0 + x1 - 1) as u16;
            let u = f32::from(center) * 0.5 - view.cx;
            let world_x = forward_x + step_x * u;
            let world_z = forward_z + step_z * u;
            let ground = sample(world_x, world_z);
            let elevation = ground.elevation;
            if !elevation.is_finite() {
                neighbor = None;
                continue;
            }
            let prior = [(f32::from(slope_center[band]) * 0.5 - view.cx) * previous_spacing,
                slope_height[band]];
            slope_height[band..next_band].fill(elevation);
            slope_center[band..next_band].fill(center);
            let current = [u * spacing, elevation];
            let adjacent = neighbor.replace(current);

            let (y, camera_depth) = if sea_row >= 0 && elevation == 0.0 {
                (sea_row, sea_depth)
            } else {
                let relief = if curved {
                    let lateral = u * inverse_focal;
                    elevation - camera.altitude - curve * (1.0 + lateral * lateral)
                } else {
                    elevation - camera.altitude
                };
                let camera_depth = along_camera - view.pitch_sin * relief;
                if camera_depth < 0.5 {
                    continue;
                }
                let up = view.pitch_cos * relief + along_up;
                let projected_up = view.focal * up;
                // Samples safely below the current skyline cannot draw a span.
                let cutoff = (view.cy - limit as f32 + 0.499) * camera_depth;
                if projected_up < cutoff && camera_depth.is_finite() {
                    continue;
                }
                // Negative scanlines clamp to top, so truncation has the same visible result.
                (((view.cy - projected_up / camera_depth + 0.5) as i32).max(top), camera_depth)
            };
            if y >= limit {
                continue;
            }

            let common_limit = i32::from(edge[band].min(edge[next_band - 1])) as usize;
            let long_left = x0 + usize::from(edge[band] < edge[next_band - 1]) * stride;
            let long_right = (long_left + stride).min(x1);
            for bottom in &mut edge[band..next_band] {
                if y < i32::from(*bottom) {
                    *bottom = y as i16;
                    if y == top {
                        open -= 1;
                    }
                }
            }
            if band < work.write_from {
                continue;
            }
            // Keep skyline and slope state even when no rotated output pixel
            // can sample this span. Only skip its paint and material work.
            if !CLIPPED {
                let [min0, max0] = visible_rows[band];
                let [min1, max1] = visible_rows[next_band - 1];
                if y >= i32::from(max0.max(max1)) || limit <= i32::from(min0.min(min1)) {
                    continue;
                }
            }

            let grade = || terrain_grade(current, prior, adjacent,
                if previous > 0.0 { inverse_step } else { 0.0 });
            let world_position = [world_x, elevation, world_z];
            let surface = material(world_x, world_z, ground);
            let light = GroundLight {
                daylight: atmosphere.daylight, ambient, light_up,
                #[cfg(feature = "ground-cover")]
                ground_seed: atmosphere.ground_seed,
                horizon, fog, light_forward, light_right,
                lateral: u * inverse_focal,
                reflection: [camera.altitude - distance * reflection, glint_width, glint,
                    sun_right, sun_forward],
            };
            let (color, rock_phase) = ground_color(surface, world_position, grade, detail_fade, light, &camera);
            let paint = |from: usize, to: usize, color: u16| {
                let together = to.min(common_limit);
                if from < together {
                    paint_span::<CLIPPED>(pixels, work.row_width, height, aperture, view,
                        x0 - work.origin_x, x1 - work.origin_x, from, together, color);
                }
                if to > common_limit {
                    paint_span::<CLIPPED>(
                        pixels, work.row_width, height, aperture, view,
                        long_left - work.origin_x, long_right - work.origin_x, from.max(together), to, color,
                    );
                }
            };
            paint_visible_surface(
                surface, world_position, color, y as usize..limit as usize,
                distance, camera_depth, view, light, rock_phase, paint,
            );
            if let Some((depth, rect)) = depth.as_mut() {
                let from = (y as usize).max(rect.top);
                let to = (limit as usize).min(rect.bottom);
                if to > from && x1 > rect.left && x0 < rect.right {
                    let value = encoded_depth(camera_depth);
                    let together = to.min(common_limit);
                    if from < together {
                        paint_span::<CLIPPED>(depth, work.row_width, height, aperture, view,
                            x0.max(rect.left) - work.origin_x, x1.min(rect.right) - work.origin_x, from, together, value);
                    }
                    if to > common_limit {
                        paint_span::<CLIPPED>(depth, work.row_width, height, aperture, view,
                            long_left.max(rect.left) - work.origin_x, long_right.min(rect.right) - work.origin_x, from.max(together), to, value);
                    }
                }
            }

        }

        if distance == far {
            break;
        }
        previous = distance;
        distance = next_terrain_distance(distance, far, view.high_cruise, stride_growth);
    }
}

fn draw_terrain<const CLIPPED: bool>(
    mut sample: impl FnMut(f32, f32) -> Sample,
    mut material: impl FnMut(f32, f32, Sample) -> Surface,
    camera: Camera,
    atmosphere: Atmosphere,
    horizon: Color,
    pixels: &mut [u16],
    mut depth: Option<(&mut [u16], ScreenRect)>,
    width: usize,
    height: usize,
    aperture: &Aperture,
    view: &View,
    ground_horizon: Option<&mut dyn FnMut(f32, f32) -> f32>,
) -> usize {
    let left = (width - aperture.diameter) / 2;
    let right = left + aperture.diameter;
    let top = ((height - aperture.diameter) / 2) as i32;
    let bottom = top + aperture.diameter as i32;
    let stride = aperture.diameter.div_ceil(MAX_BANDS);
    let bands = aperture.diameter.div_ceil(stride);
    let mut visible_rows = [[0_i16; 2]; MAX_BANDS];
    if !CLIPPED {
        // Nearest-neighbor rotation can read at most one extra source pixel
        // beyond the circle. Include two for fixed-point and angle error.
        let radius = aperture.diameter as i32 + 4;
        let cx2 = width as i32 - 1;
        let cy2 = height as i32 - 1;
        for (band, rows) in visible_rows.iter_mut().take(bands).enumerate() {
            let x0 = (left + band * stride) as i32;
            let x1 = (left + (band + 1) * stride).min(right) as i32;
            let dx = (2 * x0 - cx2).max(cx2 - 2 * (x1 - 1)).max(0);
            let reach = square_root((radius * radius - dx * dx) as u32) as i32;
            *rows = [((cy2 - reach + 1) / 2).max(top) as i16,
                ((cy2 + reach) / 2 + 1).min(bottom) as i16];
        }
    }
    let mut edge = [bottom as i16; MAX_BANDS];
    let mut slope_height = [0.0; MAX_BANDS];
    let mut slope_center = [0_u16; MAX_BANDS];
    let mut open = bands;
    let stride_growth = distant_stride_growth(aperture.diameter);
    let mut distance = first_terrain_distance(camera.altitude);
    let mut previous = 0.0;
    let sun_forward = atmosphere.sun[0] * view.heading_sin + atmosphere.sun[2] * view.heading_cos;
    let sun_right = atmosphere.sun[0] * view.heading_cos - atmosphere.sun[2] * view.heading_sin;
    let sun_up = atmosphere.sun[1];
    let glint = if sun_forward > 0.40 && (0.0..0.38).contains(&sun_up) {
        smoothstep(sun_up / 0.08) * (1.0 - smoothstep((sun_up - 0.14) / 0.24)) * 0.43
    } else { 0.0 };
    let reflection = if glint > 0.0 { sun_up / sun_forward } else { 0.0 };
    let moonlight = atmosphere.moonlight();
    let [light_x, light_up, light_z] = mix(atmosphere.sun, atmosphere.moon, moonlight);
    let light_forward = light_x * view.heading_sin + light_z * view.heading_cos;
    let light_right = light_x * view.heading_cos - light_z * view.heading_sin;
    let ambient = 0.26 + atmosphere.daylight * 0.74 + moonlight * 0.16;
    let far = terrain_detail_distance(camera.altitude);
    let clear_contrast = 0.145 * clear_cruise_visibility(camera.altitude, atmosphere.sun[1], atmosphere.haze);
    let curvature = view.high_cruise * 0.000040;
    let curved = curvature != 0.0;
    let inverse_focal = 1.0 / view.focal;

    while distance <= far && open != 0 {
        let forward_x = camera.x + view.heading_sin * distance;
        let forward_z = camera.z + view.heading_cos * distance;
        let spacing = distance / view.focal;
        let step_x = view.heading_cos * spacing;
        let step_z = -view.heading_sin * spacing;
        let along_up = view.pitch_sin * distance;
        let along_camera = view.pitch_cos * distance;
        let sea_depth = along_camera - view.pitch_sin * -camera.altitude;
        let sea_row = if curved {
            -1
        } else if sea_depth < 0.5 {
            bottom
        } else {
            let sea_up = view.pitch_cos * -camera.altitude + along_up;
            ((view.cy - view.focal * sea_up / sea_depth + 0.5) as i32).max(top)
        };
        let inverse_step = 1.0 / (distance - previous);
        let previous_spacing = previous * inverse_focal;
        let distance_fraction = (distance / far).min(1.0);
        let detail_fade = (1.0 - distance / 150.0).clamp(0.0, 1.0);
        let curve = curvature * distance * distance;
        let glint_width = if glint > 0.0 { 1.0 / (3.0 + distance * 0.047) } else { 0.0 };
        // Clear air above the weather shows more surface color on distant ground.
        let fog = (distance_fraction * (0.42 + atmosphere.haze * 0.64)
            + distance_fraction * distance_fraction * 0.38).min(0.97)
            - clear_contrast * smoothstep((distance_fraction - 0.44) / 0.56);
        let mut neighbor = None;
        let mut next_band = 0;
        let merge_distance = if view.high_cruise > 0.57 { 116.0 } else { 147.0 };
        while next_band < bands {
            let band = next_band;
            // A distant, nearly level silhouette can share a sample across two
            // narrow columns. Keep individual columns at steep edges.
            let paired = band.is_multiple_of(2)
                && distance > merge_distance
                && band + 1 < bands
                && edge[band].abs_diff(edge[band + 1]) <= 1;
            next_band += if paired { 2 } else { 1 };
            let limit = i32::from(edge[band].max(edge[next_band - 1]));
            if limit == top {
                neighbor = None;
                continue;
            }
            let x0 = left + band * stride;
            let x1 = (left + next_band * stride).min(right);
            let center = (x0 + x1 - 1) as u16;
            let u = f32::from(center) * 0.5 - view.cx;
            let world_x = forward_x + step_x * u;
            let world_z = forward_z + step_z * u;
            let ground = sample(world_x, world_z);
            let elevation = ground.elevation;
            if !elevation.is_finite() {
                neighbor = None;
                continue;
            }
            let prior = [(f32::from(slope_center[band]) * 0.5 - view.cx) * previous_spacing,
                slope_height[band]];
            slope_height[band..next_band].fill(elevation);
            slope_center[band..next_band].fill(center);
            let current = [u * spacing, elevation];
            let adjacent = neighbor.replace(current);

            let (y, camera_depth) = if sea_row >= 0 && elevation == 0.0 {
                (sea_row, sea_depth)
            } else {
                let relief = if curved {
                    let lateral = u * inverse_focal;
                    elevation - camera.altitude - curve * (1.0 + lateral * lateral)
                } else {
                    elevation - camera.altitude
                };
                let camera_depth = along_camera - view.pitch_sin * relief;
                if camera_depth < 0.5 {
                    continue;
                }
                let up = view.pitch_cos * relief + along_up;
                let projected_up = view.focal * up;
                // Samples safely below the current skyline cannot draw a span.
                let cutoff = (view.cy - limit as f32 + 0.499) * camera_depth;
                if projected_up < cutoff && camera_depth.is_finite() {
                    continue;
                }
                // Negative scanlines clamp to top, so truncation has the same visible result.
                (((view.cy - projected_up / camera_depth + 0.5) as i32).max(top), camera_depth)
            };
            if y >= limit {
                continue;
            }

            let common_limit = i32::from(edge[band].min(edge[next_band - 1])) as usize;
            let long_left = x0 + usize::from(edge[band] < edge[next_band - 1]) * stride;
            let long_right = (long_left + stride).min(x1);
            for bottom in &mut edge[band..next_band] {
                if y < i32::from(*bottom) {
                    *bottom = y as i16;
                    if y == top {
                        open -= 1;
                    }
                }
            }
            // Keep skyline and slope state even when no rotated output pixel
            // can sample this span. Only skip its paint and material work.
            if !CLIPPED {
                let [min0, max0] = visible_rows[band];
                let [min1, max1] = visible_rows[next_band - 1];
                if y >= i32::from(max0.max(max1)) || limit <= i32::from(min0.min(min1)) {
                    continue;
                }
            }

            let grade = || terrain_grade(current, prior, adjacent,
                if previous > 0.0 { inverse_step } else { 0.0 });
            let world_position = [world_x, elevation, world_z];
            let surface = material(world_x, world_z, ground);
            let light = GroundLight {
                daylight: atmosphere.daylight, ambient, light_up,
                #[cfg(feature = "ground-cover")]
                ground_seed: atmosphere.ground_seed,
                horizon, fog, light_forward, light_right,
                lateral: u * inverse_focal,
                reflection: [camera.altitude - distance * reflection, glint_width, glint,
                    sun_right, sun_forward],
            };
            let (color, rock_phase) = ground_color(surface, world_position, grade, detail_fade, light, &camera);
            let paint = |from: usize, to: usize, color: u16| {
                let together = to.min(common_limit);
                if from < together {
                    paint_span::<CLIPPED>(pixels, width, height, aperture, view,
                        x0, x1, from, together, color);
                }
                if to > common_limit {
                    paint_span::<CLIPPED>(
                        pixels, width, height, aperture, view,
                        long_left, long_right, from.max(together), to, color,
                    );
                }
            };
            paint_visible_surface(
                surface, world_position, color, y as usize..limit as usize,
                distance, camera_depth, view, light, rock_phase, paint,
            );
            if let Some((depth, rect)) = depth.as_mut() {
                let from = (y as usize).max(rect.top);
                let to = (limit as usize).min(rect.bottom);
                if to > from && x1 > rect.left && x0 < rect.right {
                    let value = encoded_depth(camera_depth);
                    let together = to.min(common_limit);
                    if from < together {
                        paint_span::<CLIPPED>(depth, width, height, aperture, view,
                            x0.max(rect.left), x1.min(rect.right), from, together, value);
                    }
                    if to > common_limit {
                        paint_span::<CLIPPED>(depth, width, height, aperture, view,
                            long_left.max(rect.left), long_right.min(rect.right), from.max(together), to, value);
                    }
                }
            }

        }

        if distance == far {
            break;
        }
        previous = distance;
        distance = next_terrain_distance(distance, far, view.high_cruise, stride_growth);
    }
    if let Some(ground_horizon) = ground_horizon.filter(|_| open > 0) {
        draw_far_horizon::<CLIPPED>(
            ground_horizon, camera, atmosphere, horizon, view, aperture, &mut edge[..bands],
            stride, pixels, depth, width, height,
        );
    }

    // The skyline also tracks culled spans, so it may start earlier than
    // the first actual write. Both terrain planes remain clear above it.
    if CLIPPED {
        0
    } else {
        edge[..bands].iter().copied().min().unwrap_or(bottom as i16) as usize
    }
}

// Only unfilled screen rows are eligible; near terrain, overlays, and their
// depth remain exactly as produced by the normal march.
fn draw_far_horizon<const CLIPPED: bool>(
    sample: &mut dyn FnMut(f32, f32) -> f32,
    camera: Camera,
    atmosphere: Atmosphere,
    horizon: Color,
    view: &View,
    aperture: &Aperture,
    edge: &mut [i16],
    stride: usize,
    pixels: &mut [u16],
    mut depth: Option<(&mut [u16], ScreenRect)>,
    width: usize,
    height: usize,
) {
    trace_far_horizon(
        sample, camera, atmosphere, horizon, view, aperture, edge, stride, width, height,
        |x0, x1, row_start, bottom, color, distance_to_eye| {
            paint_span::<CLIPPED>(
                pixels, width, height, aperture, view, x0, x1, row_start, bottom,
                color,
            );
            if let Some((depth, rect)) = depth.as_mut() {
                let from = (row_start).max(rect.top);
                let to = bottom.min(rect.bottom);
                if to > from && x1 > rect.left && x0 < rect.right {
                    paint_span::<CLIPPED>(
                        depth,
                        width,
                        height,
                        aperture,
                        view,
                        x0.max(rect.left),
                        x1.min(rect.right),
                        from,
                        to,
                        encoded_depth(distance_to_eye).min(u16::MAX - 1),
                    );
                }
            }
        },
    );
}

fn trace_far_horizon(
    sample: &mut dyn FnMut(f32, f32) -> f32,
    camera: Camera,
    atmosphere: Atmosphere,
    horizon: Color,
    view: &View,
    aperture: &Aperture,
    edge: &mut [i16],
    stride: usize,
    width: usize,
    height: usize,
    mut paint: impl FnMut(usize, usize, usize, usize, u16, f32),
) {
    let near_far = terrain_detail_distance(camera.altitude);
    let far = horizon_distance(camera.altitude);
    let left = (width - aperture.diameter) / 2;
    let right = left + aperture.diameter;
    let top = (height - aperture.diameter) as i32 / 2;
    let curve = view.high_cruise * 0.00004;
    let inverse_focal = 1.0 / view.focal;
    let old_fog = (0.80 + atmosphere.haze * 0.64).min(0.97);
    let clear_contrast = 0.145 * clear_cruise_visibility(camera.altitude, atmosphere.sun[1], atmosphere.haze);
    let city_band_contrast = 0.06 * clear_daylight_visibility(atmosphere.sun[1], atmosphere.haze);
    let land_tint = mix(
        [25.0, 34.0, 45.0],
        [91.0, 116.0, 102.0],
        atmosphere.daylight,
    );
    let high_tint = mix(
        [30.0, 37.0, 50.0],
        [150.0, 147.0, 135.0],
        atmosphere.daylight,
    );
    let water_tint = mix(
        [15.0, 29.0, 48.0],
        [76.0, 121.0, 141.0],
        atmosphere.daylight,
    );
    let mut distance = (near_far * 1.08).min(far);
    loop {
        let front_x = camera.x + view.heading_sin * distance;
        let front_z = camera.z + view.heading_cos * distance;
        let spacing = distance * inverse_focal;
        let step_x = view.heading_cos * spacing;
        let step_z = -view.heading_sin * spacing;
        let along_up = view.pitch_sin * distance;
        let along_camera = view.pitch_cos * distance;
        let drop = curve * backdrop_curve_distance_squared(distance);
        let mut ring_fog = None;
        let mut ring_water = None;
        let mut i = 0;
        while i < edge.len() {
            let end = if i + 1 < edge.len() && edge[i].abs_diff(edge[i + 1]) <= 1 {
                i + 2
            } else {
                i + 1
            };
            let limit = i32::from(edge[i].max(edge[end - 1]));
            if limit > top {
                let x0 = left + i * stride;
                let x1 = (left + end * stride).min(right);
                let u = (x0 + x1 - 1) as f32 * 0.5 - view.cx;
                let lateral = u * inverse_focal;
                let elevation = sample(front_x + step_x * u, front_z + step_z * u).max(0.0);
                let relief = elevation - camera.altitude - drop * (1.0 + lateral * lateral);
                let distance_to_eye = along_camera - view.pitch_sin * relief;
                let up = view.focal * (view.pitch_cos * relief + along_up);
                if distance_to_eye > 0.5 && up >= (view.cy - limit as f32 + 0.499) * distance_to_eye
                {
                    let y = ((view.cy - up / distance_to_eye + 0.5) as i32).max(top);
                    if y < limit {
                        // Covered strata need height tests, but no color.
                        let fog = *ring_fog.get_or_insert_with(|| {
                            let progress = ((distance - near_far) / (far - near_far)).min(1.0);
                            let distant = (0.81 + atmosphere.haze * 0.12 + 0.14 * progress).min(0.98);
                            let fog = old_fog + (distant - old_fog) * smoothstep(progress * 4.0)
                                - clear_contrast * (1.0 - smoothstep(progress));
                            let beyond = (distance - far).max(0.0);
                            let outer_fade = smoothstep(beyond / (TERRAIN_BACKDROP_DISTANCE - far));
                            // Keep enough land contrast to read the city silhouettes in fair air.
                            let city_band = smoothstep((distance - far * 0.62) / (far * 0.38))
                                * (1.0 - smoothstep((distance - CITY_HORIZON) * (3.0 / CITY_HORIZON)));
                            fog + (0.98 - fog) * outer_fade - city_band_contrast * city_band
                        });
                        let color = if elevation == 0.0 {
                            *ring_water.get_or_insert_with(|| rgb565(mix(water_tint, horizon, fog)))
                        } else {
                            let tint =
                                mix(land_tint, high_tint, (elevation * (1.0 / 150.0)).min(1.0));
                            rgb565(mix(tint, horizon, fog))
                        };
                        for (band, edge) in edge.iter_mut().enumerate().take(end).skip(i) {
                            if y >= i32::from(*edge) {
                                continue;
                            }
                            let bottom = *edge as usize;
                            let x0 = left + band * stride;
                            let x1 = (x0 + stride).min(right);
                            paint(x0, x1, y as usize, bottom, color, distance_to_eye);
                            *edge = y as i16;
                        }
                    }
                }
            }
            i = end;
        }
        if distance == TERRAIN_BACKDROP_DISTANCE {
            break;
        }
        distance = if distance < far {
            (distance * 1.105).min(far)
        } else if distance < CITY_HORIZON {
            // Keep track of intervening ridges in the city band. Beyond the
            // first six rings, the smaller projection needs fewer samples.
            let spacing = if distance < SCENERY_HORIZON + 960.0 { 160.0 } else { 320.0 };
            (distance + spacing).min(CITY_HORIZON)
        } else {
            (distance * 1.38).min(TERRAIN_BACKDROP_DISTANCE)
        };
    }
}

fn clear_depth_outside_output(
    depth: &mut [u16],
    width: usize,
    upright: ScreenRect,
    output: ScreenRect,
) {
    if output.top <= upright.top && output.bottom >= upright.bottom
        && output.left <= upright.left && output.right >= upright.right
    {
        return;
    }
    let keep_left = output.left.clamp(upright.left, upright.right);
    let keep_right = output.right.clamp(keep_left, upright.right);
    for row in upright.top..upright.bottom {
        let at = row * width;
        if (output.top..output.bottom).contains(&row) {
            depth[at + upright.left..at + keep_left].fill(u16::MAX);
            depth[at + keep_right..at + upright.right].fill(u16::MAX);
        } else {
            depth[at + upright.left..at + upright.right].fill(u16::MAX);
        }
    }
}

fn rotation_copies_runs(view: &View, aperture: &Aperture) -> bool {
    let cosine = (view.roll_cos * 65_536.0) as i32;
    let sine = (view.roll_sin * 65_536.0) as i32;
    (65_000..=65_536).contains(&cosine)
        && (aperture.diameter < 384 || sine.unsigned_abs() <= 4_096)
}

fn rotate_terrain<const PAIRED: bool, const PAIRED_RUNS: bool>(
    source: &[u16],
    pixels: &mut [u16],
    width: usize,
    height: usize,
    aperture: &Aperture,
    view: &View,
    rect: ScreenRect,
    first_ground: usize,
    mut other_plane: Option<(&[u16], &mut [u16])>,
) {
    let cosine = (view.roll_cos * 65_536.0) as i32;
    let sine = (view.roll_sin * 65_536.0) as i32;
    // At native panel size, short copies across larger banks cost more
    // than gathering directly. Keep bulk copies for long, shallow runs.
    let copy_runs = !PAIRED && rotation_copies_runs(view, aperture);
    let x_deficit = (65_536 - cosine).max(1) as u32;
    let inv_x = 65_536 / x_deficit;
    let inv_y = 65_536 / sine.unsigned_abs().max(1);
    let w = width as i32;
    let h = height as i32;
    let min_x = (width - aperture.diameter) as i32 / 2;
    let min_y = (height - aperture.diameter) as i32 / 2;
    let max_x = min_x + aperture.diameter as i32 - 1;
    let max_y = min_y + aperture.diameter as i32 - 1;
    let first_ground = first_ground as i32;
    if first_ground > max_y { return; }
    let terrain_boundary = if first_ground > min_y { first_ground << 16 } else { i32::MIN };
    for row in rect.top..rect.bottom {
        let first = (aperture.left[row] as usize).max(rect.left);
        let last = (aperture.right[row] as usize).min(rect.right);
        if last <= first { continue; }
        let dx = 2 * first as i32 + 1 - w;
        let dy = 2 * row as i32 + 1 - h;
        let mut source_x = ((dx * cosine + dy * sine) >> 1) + (w << 15);
        let mut source_y = ((-dx * sine + dy * cosine) >> 1) + (h << 15);
        let steps = (last - first - 1) as i32;
        let last_y = source_y - steps * sine;
        // Source rows are monotone. When their maximum is above the
        // first possible terrain row, the initialized output is exact.
        if source_y.max(last_y) < terrain_boundary { continue; }
        let span = row * width + first..row * width + last;
        let target = &mut pixels[span.clone()];
        let inside = |first: i32, last: i32, min: i32, max: i32| {
            (first.min(last) >> 16) >= min && (first.max(last) >> 16) <= max
        };
        // Each projected coordinate is monotone across this row. Checking
        // the endpoints also certifies every intermediate nearest sample.
        let inside = inside(source_x, source_x + steps * cosine, min_x, max_x)
            && inside(source_y, last_y, min_y, max_y);
        if PAIRED {
            let (depth_source, depth) = other_plane.as_mut().unwrap();
            let depth_target = &mut depth[span];
            if PAIRED_RUNS && inside {
                rotate_contiguous_pair(
                    (source, depth_source), (target, depth_target), width,
                    source_x, source_y, cosine, sine, inv_x, inv_y,
                );
            } else if inside {
                for (pixel, depth) in target.iter_mut().zip(depth_target) {
                    let x = (source_x >> 16) as usize;
                    let y = (source_y >> 16) as usize;
                    let index = y * width + x;
                    *pixel = source[index];
                    *depth = depth_source[index];
                    source_x += cosine;
                    source_y -= sine;
                }
            } else {
                for (pixel, depth) in target.iter_mut().zip(depth_target) {
                    let x = (source_x >> 16).clamp(min_x, max_x) as usize;
                    let y = (source_y >> 16).clamp(min_y, max_y) as usize;
                    let index = y * width + x;
                    *pixel = source[index];
                    *depth = depth_source[index];
                    source_x += cosine;
                    source_y -= sine;
                }
            }
        } else if inside {
            if copy_runs {
                rotate_contiguous(source, target, width, source_x, source_y, cosine, sine, inv_x, inv_y);
            } else {
                for pixel in target {
                    let x = (source_x >> 16) as usize;
                    let y = (source_y >> 16) as usize;
                    *pixel = source[y * width + x];
                    source_x += cosine;
                    source_y -= sine;
                }
            }
        } else {
            for pixel in target {
                let x = (source_x >> 16).clamp(min_x, max_x) as usize;
                let y = (source_y >> 16).clamp(min_y, max_y) as usize;
                *pixel = source[y * width + x];
                source_x += cosine;
                source_y -= sine;
            }
        }
    }
}

// In a short run, each nearest-neighbor lookup advances one source column
// and stays on one row. The fixed-point reciprocals round down, so the
// computed run can only be shorter than the original exact source run.
fn rotate_contiguous(
    source: &[u16],
    mut output: &mut [u16],
    width: usize,
    mut x: i32,
    mut y: i32,
    cosine: i32,
    sine: i32,
    inv_x: u32,
    inv_y: u32,
) {
    while !output.is_empty() {
        let x_fraction = (x & 65_535) as u32;
        let y_fraction = (y & 65_535) as u32;
        let room_y = if sine < 0 { 65_535 - y_fraction } else { y_fraction };
        let run_x = if cosine == 65_536 { output.len() } else { 1 + ((x_fraction * inv_x) >> 16) as usize };
        let run_y = if sine == 0 { output.len() } else { 1 + ((room_y * inv_y) >> 16) as usize };
        let count = run_x.min(run_y).min(output.len());
        let at = (y >> 16) as usize * width + (x >> 16) as usize;
        let (next, rest) = output.split_at_mut(count);
        next.copy_from_slice(&source[at..at + count]);
        output = rest;
        x += cosine * count as i32;
        y -= sine * count as i32;
    }
}

fn rotate_contiguous_pair(
    (color, depth): (&[u16], &[u16]),
    (mut color_output, mut depth_output): (&mut [u16], &mut [u16]),
    width: usize,
    mut x: i32,
    mut y: i32,
    cosine: i32,
    sine: i32,
    inv_x: u32,
    inv_y: u32,
) {
    while !color_output.is_empty() {
        let x_fraction = (x & 65_535) as u32;
        let y_fraction = (y & 65_535) as u32;
        let room_y = if sine < 0 { 65_535 - y_fraction } else { y_fraction };
        let run_x = if cosine == 65_536 { color_output.len() } else { 1 + ((x_fraction * inv_x) >> 16) as usize };
        let run_y = if sine == 0 { color_output.len() } else { 1 + ((room_y * inv_y) >> 16) as usize };
        let count = run_x.min(run_y).min(color_output.len());
        let at = (y >> 16) as usize * width + (x >> 16) as usize;
        let (next_color, rest_color) = color_output.split_at_mut(count);
        let (next_depth, rest_depth) = depth_output.split_at_mut(count);
        next_color.copy_from_slice(&color[at..at + count]);
        next_depth.copy_from_slice(&depth[at..at + count]);
        color_output = rest_color;
        depth_output = rest_depth;
        x += cosine * count as i32;
        y -= sine * count as i32;
    }
}

#[derive(Clone, Copy)]
struct GroundLight {
    daylight: f32,
    #[cfg(feature = "ground-cover")]
    ground_seed: u32,
    ambient: f32,
    light_up: f32,
    horizon: Color,
    fog: f32,
    light_forward: f32,
    light_right: f32,
    lateral: f32,
    reflection: [f32; 5],
}

#[inline]
fn terrain_grade(
    [side, height]: [f32; 2], [past_side, past_height]: [f32; 2],
    adjacent: Option<[f32; 2]>, inverse_step: f32,
) -> [f32; 2] {
    // The prior ray sample has a different lateral offset as its column
    // widens with distance or joins its neighbor. Remove that component
    // before treating its height difference as a forward grade.
    let across = adjacent.map_or(0.0, |[left, left_height]| {
        (height - left_height) / (side - left)
    });
    let forward = (height - past_height - across * (side - past_side)) * inverse_step;
    [forward, across]
}

fn ground_color(
    surface: Surface, position: [f32; 3], grade: impl FnOnce() -> [f32; 2],
    detail_fade: f32, lighting: GroundLight, eye: &Camera,
) -> (u16, f32) {
    let GroundLight {
        daylight, ambient, light_up, horizon, fog, light_forward, light_right, lateral,
        #[cfg(feature = "ground-cover")]
        ground_seed,
        reflection: [reflection_height, reflection_width, glint, sun_right, sun_forward],
    } = lighting;
    let woodland = || {
        #[cfg(feature = "ground-cover")]
        { crate::ground_cover::woodland_color(position, ground_seed) }
        #[cfg(not(feature = "ground-cover"))]
        { [67.0, 114.0, 68.0] }
    };
    let (color, water) = match surface {
        Surface::DeepWater => ([34.0, 94.0, 141.0], true),
        Surface::Shallows => ([45.0, 120.0, 148.0], true),
        Surface::Sand => (
            mix([198.0, 177.0, 136.0], [126.0, 150.0, 86.0],
                smoothstep((position[1] - 0.80) / 1.69)),
            false,
        ),
        Surface::Grass => (grass_color(position[1]), false),
        Surface::Forest => (forest_color(woodland(), position[1]), false),
        Surface::Rock => (alpine_rock(position[1]), false),
        Surface::Snow => (snow_color(position[1]), false),
        Surface::Desert => (desert_color(position[1]), false),
        Surface::Mojave => ([170.0, 153.0, 119.0], false),
        Surface::PyramidGlass => ([49.0, 58.0, 73.0], false),
        Surface::SphereShell => {
            let ribs = surface_noise(position[0] * 0.039, position[2] * 0.041);
            (mix([30.0, 82.0, 141.0], [71.0, 181.0, 228.0], ribs), false)
        },
        Surface::Pavement => ([80.0, 87.0, 92.0], false),
        Surface::Gravel => ([163.0, 149.0, 118.0], false),
        Surface::Park => ([87.0, 135.0, 70.0], false),
        Surface::UrbanLot => (
            mix([87.0, 135.0, 70.0], [136.0, 134.0, 124.0], aerial_view(eye.altitude)),
            false,
        ),
        Surface::TreeCanopy | Surface::SpruceCanopy | Surface::AspenCanopy
        | Surface::MapleCanopy | Surface::JuniperCanopy => {
            let [dx, dy, dz] = [position[0] - eye.x, position[1] - eye.altitude, position[2] - eye.z];
            let close = 1.0 - smoothstep((dx * dx + dy * dy + dz * dz - 54.0 * 54.0)
                * (1.0 / (112.0 * 112.0 - 54.0 * 54.0)));
            let pigment = surface.canopy_color().unwrap_or([61, 117, 55]).map(f32::from);
            // The same seeded woodland palette colors adjoining crowns and the canopy behind them.
            let foliage = mix(woodland(), pigment, 0.16 * close);
            (forest_color(foliage, position[1]), false)
        }
        Surface::FieldGreen => ([111.0, 137.0, 69.0], false),
        Surface::FieldGold => ([183.0, 164.0, 86.0], false),
        Surface::FieldSoil => ([152.0, 109.0, 77.0], false),
        Surface::PitchedRoof => ([171.0, 85.0, 61.0], false),
        Surface::FlatRoof => ([137.0, 141.0, 150.0], false),
        Surface::TowerRoof => ([157.0, 175.0, 188.0], false),
        Surface::IndustryBrick => ([135.0, 92.0, 76.0], false),
        Surface::IndustryConcrete => ([190.0, 182.0, 160.0], false),
        Surface::IndustryMetal => ([117.0, 145.0, 149.0], false),
        Surface::MesaCap | Surface::MesaCliff | Surface::MesaApron => {
            (surface.sandstone_color().unwrap().map(f32::from), false)
        }
        Surface::RailTimber => ([174.0, 99.0, 52.0], false),
        Surface::RailMetal => ([226.0, 231.0, 212.0], false),
        Surface::RailGlass => ([58.0, 123.0, 167.0], false),
        Surface::RailRust => ([120.0, 59.0, 37.0], false),
        Surface::RailApron => ([151.0, 124.0, 101.0], false),
        Surface::ShipHull => ([58.0, 68.0, 86.0], false),
        Surface::ShipDeck => ([162.0, 129.0, 91.0], false),
        Surface::ShipCabin => ([233.0, 230.0, 216.0], false),
        Surface::ShipCargo => ([193.0, 91.0, 56.0], false),
        Surface::HarborCrane => ([213.0, 146.0, 72.0], false),
        Surface::CargoOchre => ([186.0, 157.0, 83.0], false),
        Surface::CargoBlue => ([95.0, 142.0, 169.0], false),
        Surface::RiverWater => ([54.0, 119.0, 139.0], true),
        Surface::WetMeadow => ([89.0, 131.0, 77.0], false),
        Surface::ReedMarsh => ([75.0, 105.0, 61.0], false),
        Surface::Boardwalk => ([155.0, 102.0, 65.0], false),
        Surface::RiverSilt => ([194.0, 169.0, 121.0], false),
        Surface::SolarGlass => ([39.0, 80.0, 135.0], false),
        Surface::GrassRunway => ([139.0, 158.0, 105.0], false),
        Surface::RunwayMarking => ([211.0, 215.0, 201.0], false),
        Surface::AirfieldGuide => ([216.0, 180.0, 72.0], false),
        Surface::RunwayLight => ([120.0, 142.0, 154.0], false),
        Surface::TarnWater => ([38.0, 96.0, 133.0], true),
        Surface::TarnShallows => ([62.0, 126.0, 137.0], true),
        Surface::AlpineMeadow => ([127.0, 145.0, 91.0], false),
        Surface::SaltCrust => ([214.0, 198.0, 167.0], false),
        Surface::DuneSand => ([186.0, 147.0, 98.0], false),
        Surface::DryWash => ([174.0, 163.0, 146.0], false),
        Surface::GlacierIce => ([175.0, 212.0, 223.0], false),
        Surface::GlacierCrevasse => ([66.0, 118.0, 142.0], false),
        Surface::GlacierMoraine => ([110.0, 118.0, 121.0], false),
        Surface::GlacierMeltwater => ([87.0, 143.0, 151.0], true),
        Surface::ScreeEdge => (mix(grass_color(position[1]), [135.0, 128.0, 116.0], 0.44), false),
        Surface::ScreeFine => (mix(alpine_rock(position[1]), [172.0, 166.0, 151.0], 0.70), false),
        Surface::ScreeCoarse => (mix(alpine_rock(position[1]), [155.0, 149.0, 135.0], 0.70), false),
        _ => (surface.building_palette().unwrap().roof.map(f32::from), false),
    };
    let slopes = if water { [0.0; 2] } else { grade() };
    let light = if water {
        0.80 + light_up.max(0.0) * 0.19
    } else {
        let [slope, across] = slopes;
        let steepness = slope.abs() + across.abs();
        let diffuse = light_up - slope * light_forward - across * light_right;
        let incidence = diffuse * (1.0 - steepness.min(3.8) * 0.13);
        0.56 + incidence.clamp(0.0, 1.0) * 0.58
    };
    #[cfg(feature = "ground-cover")]
    let (color, roughness, rock_phase) = match crate::ground_cover::paint(surface, position, ground_seed, color, detail_fade, slopes) {
        Some((covered, phase)) => (covered, 0.0, phase),
        None => (color, surface_detail(surface, position, detail_fade), 0.0),
    };
    #[cfg(not(feature = "ground-cover"))]
    let (roughness, rock_phase) = (surface_detail(surface, position, detail_fade), 0.0);
    let exposure = light * (1.0 + roughness) * ambient;
    let twilight = (1.0 - daylight) * 16.0;
    let color = [
        color[0] * exposure + twilight * 0.15,
        color[1] * exposure + twilight * 0.43,
        color[2] * exposure + twilight,
    ];
    let color = if water && glint > 0.0 {
        let azimuth = (sun_right - sun_forward * lateral).abs();
        let slope = (reflection_height - position[1]).abs() * reflection_width;
        let flicker = (0.72 + roughness * 2.0).clamp(0.50, 0.92);
        let glint = (1.0 - smoothstep(azimuth / 0.18)) * (1.0 - smoothstep(slope))
            * glint * flicker;
        mix(color, [233.0, 167.0, 109.0], glint)
    } else {
        color
    };
    let city_surface = matches!(surface, Surface::Pavement | Surface::UrbanLot | Surface::PitchedRoof
        | Surface::FlatRoof | Surface::TowerRoof | Surface::IndustryBrick | Surface::IndustryConcrete
        | Surface::IndustryMetal) || surface.building_palette().is_some();
    let city_visibility = if city_surface { aerial_view(eye.altitude) * daylight } else { 0.0 };
    let material = if surface == Surface::RunwayLight {
        let evening = 1.0 - smoothstep((daylight - 0.08) / 0.41);
        mix(color, [192.0, 209.0, 236.0], evening * (1.0 - fog))
    } else if surface == Surface::SphereShell {
        mix(color, [43.0, 137.0, 225.0], (0.40 + (1.0 - daylight) * 0.46) * (1.0 - fog))
    } else if city_visibility > 0.0 {
        let gain = if surface == Surface::Pavement { -0.17 } else { 0.14 };
        color.map(|channel| channel * (1.0 + gain * city_visibility))
    } else {
        color
    };
    (rgb565(mix(material, horizon, fog * (1.0 - 0.29 * city_visibility))).max(1), rock_phase)
}

const FOOTHILL: Color = [139.0, 148.0, 96.0];
const ALPINE_TURF: Color = [139.0, 143.0, 116.0];
const TREELINE_TURF: Color = [139.0, 145.5, 106.0];

fn grass_color(height: f32) -> Color {
    const LOWLAND: Color = [114.0, 151.0, 78.0];
    if height <= 51.0 { return LOWLAND; }
    if height < 64.0 {
        return mix(LOWLAND, FOOTHILL, smoothstep((height - 51.0) * (1.0 / 13.0)));
    }
    mix(FOOTHILL, ALPINE_TURF, smoothstep((height - 64.0) * (1.0 / 24.0)))
}

fn forest_color(woodland: Color, height: f32) -> Color {
    if height <= 57.0 { return woodland; }
    if height >= 76.0 { return grass_color(height); }
    mix(woodland, TREELINE_TURF, smoothstep((height - 57.0) * (1.0 / 19.0)))
}

fn alpine_rock(height: f32) -> Color {
    mix(ALPINE_TURF, [154.0, 150.0, 146.0], smoothstep((height - 88.0) * (1.0 / 37.0)))
}

fn snow_color(height: f32) -> Color {
    mix([147.0, 146.0, 141.0], [226.0, 234.0, 243.0],
        smoothstep((height - 109.0) * (1.0 / 47.0)))
}

fn desert_color(height: f32) -> Color {
    const HIGH_DESERT: Color = [185.0, 129.0, 86.0];
    if height <= 51.0 {
        return mix([151.0, 145.0, 96.0], HIGH_DESERT,
            smoothstep((height - 14.0) / 37.0));
    }
    mix(HIGH_DESERT, FOOTHILL, smoothstep((height - 51.0) * (1.0 / 13.0)))
}

#[derive(Clone, Copy)]
struct CloudLight {
    exposure: f32,
    opacity: f32,
}

impl CloudLight {
    const CLEAR: Self = Self { exposure: 1.0, opacity: 0.0 };
}

fn ground_cloud_presence(sky: Atmosphere, camera: Camera) -> f32 {
    let overhead = smoothstep((camera.altitude - 225.0) / 72.0);
    smoothstep((sky.clouds - (0.48 - 0.20 * overhead)) / (0.24 + 0.07 * overhead))
}

#[derive(Clone, Copy)]
struct GroundClouds {
    camera: [f32; 3],
    altitude: f32,
    threshold: f32,
    shade_offset: [f32; 2],
    wind: [f32; 2],
    presence: f32,
    overhead: f32,
}

impl GroundClouds {
    fn new(sky: Atmosphere, camera: Camera) -> Self {
        let altitude = if sky.cloud_base > 0.0 {
            sky.cloud_base
        } else {
            107.0 + sky.clouds * 28.0
        };
        let sun_height = sky.sun[1].max(0.25);
        let shade_distance = (altitude / sun_height).min(270.0);
        let overhead = smoothstep((camera.altitude - 225.0) / 72.0);
        Self {
            camera: [camera.x, camera.altitude, camera.z],
            altitude, threshold: 0.67 - sky.clouds * 0.40 + overhead * (0.06 + (1.0 - sky.clouds) * 0.05),
            shade_offset: [sky.sun[0] * shade_distance, sky.sun[2] * shade_distance],
            wind: sky.cloud_offset, presence: ground_cloud_presence(sky, camera), overhead,
        }
    }

    fn over(self, [x, y, z]: [f32; 3], distance: f32, cells: &mut [CloudCell; 2]) -> CloudLight {
        if self.presence == 0.0 {
            return CloudLight::CLEAR;
        }
        let cx = self.camera[0];
        let cy = self.camera[1];
        let cz = self.camera[2];
        let mut density = |x: f32, z: f32| {
            let x = x * 0.0105 + self.wind[0] * 0.056;
            let z = z * 0.0105 + self.wind[1] * 0.056;
            let mut field = surface_noise(x, z);
            if self.overhead > 0.0 {
                let body = cells[0].sample((x + z * 0.32) * 0.57 + 19.6, z * 0.78 - 10.7);
                let billows = cells[1].sample(x * 1.84 - 15.3, z * 1.92 + 6.8);
                let shape = body * 0.76 + billows * 0.24;
                field += (shape - field) * self.overhead;
            }
            smoothstep((field - self.threshold) / (0.34 - 0.16 * self.overhead))
        };
        let below = self.altitude - y;
        let plane_clearance = cy - self.altitude;
        let near_layer = 1.0 - smoothstep((plane_clearance.abs() - 3.0) / 31.0);
        let veil = if near_layer > 0.01 {
            density(cx + (x - cx) * 0.25, cz + (z - cz) * 0.25)
                * near_layer * smoothstep(distance / 190.0) * 0.46
        } else {
            0.0
        };
        if plane_clearance > 0.0 && below > 0.0 {
            let fraction = plane_clearance / (cy - y);
            let thickness = density(cx + (x - cx) * fraction, cz + (z - cz) * fraction);
            let cloud_tops = thickness * smoothstep(plane_clearance / 49.0)
                * smoothstep(below / 20.0) * (0.67 + self.overhead * 0.19);
            let sunlit_ground = if self.overhead > 0.0 {
                let factor = (below / self.altitude).clamp(0.0, 1.0);
                1.0 - density(x + self.shade_offset[0] * factor, z + self.shade_offset[1] * factor)
                    * self.overhead * self.presence * 0.10
            } else {
                1.0
            };
            return CloudLight {
                exposure: sunlit_ground,
                opacity: cloud_tops.max(veil) * self.presence,
            };
        }
        let ground_shade = if below > 0.0 && distance < 270.0 {
            let factor = (below / self.altitude).clamp(0.0, 1.0);
            density(x + self.shade_offset[0] * factor, z + self.shade_offset[1] * factor)
                * smoothstep((270.0 - distance) / 80.0)
                * smoothstep(below / 18.0)
                * smoothstep(-plane_clearance / 29.0)
                * 0.17
        } else {
            0.0
        };
        CloudLight { exposure: 1.0 - ground_shade * self.presence, opacity: veil * self.presence }
    }
}

fn paint_ground_clouds(
    sample: &mut impl FnMut(f32, f32) -> Sample,
    camera: Camera,
    sky: Atmosphere,
    view: &View,
    aperture: &Aperture,
    pixels: &mut [u16],
    width: usize,
    height: usize,
) {
    const PATCHES: usize = 16;
    const EDGE: usize = PATCHES + 1;
    let stride = width.max(height).div_ceil(PATCHES).next_power_of_two().max(4);
    let x_steps = width.div_ceil(stride);
    let y_steps = height.div_ceil(stride);
    let mut opacity = [[0_u8; EDGE]; EDGE];
    let mut shade = [[0_u8; EDGE]; EDGE];
    let clouds = GroundClouds::new(sky, camera);
    let mut cells = [CloudCell::START; 2];
    let valley = sample(camera.x, camera.z).elevation.min(camera.altitude - 8.0);
    let inverse_focal = 1.0 / view.focal;
    let far = view_distance(camera.altitude)
        + (terrain_detail_distance(camera.altitude) - view_distance(camera.altitude)) * clouds.overhead;
    for (row, (opacity_row, shade_row)) in opacity.iter_mut().zip(shade.iter_mut())
        .enumerate().take(y_steps + 1)
    {
        let sy = (row * stride) as f32 - view.cy;
        for (column, (opacity, shade)) in opacity_row.iter_mut().zip(shade_row.iter_mut())
            .enumerate().take(x_steps + 1)
        {
            let sx = (column * stride) as f32 - view.cx;
            let u = (sx * view.roll_cos + sy * view.roll_sin) * inverse_focal;
            let v = (sy * view.roll_cos - sx * view.roll_sin) * inverse_focal;
            let down = view.pitch_sin + v * view.pitch_cos;
            if down <= 0.02 {
                continue;
            }
            let along = view.pitch_cos - v * view.pitch_sin;
            let rx = view.heading_sin * along + view.heading_cos * u;
            let rz = view.heading_cos * along - view.heading_sin * u;
            let march = ((camera.altitude - valley) / down).clamp(2.0, far);
            let terrain = sample(camera.x + rx * march, camera.z + rz * march).elevation;
            let march = ((camera.altitude - terrain) / down).clamp(2.0, far);
            let ray_height = terrain + (camera.altitude - down * march - terrain).max(0.0) * clouds.overhead;
            let point = [camera.x + rx * march, ray_height, camera.z + rz * march];
            let air = clouds.over(point, march, &mut cells);
            *opacity = (air.opacity * 64.0 + 0.5) as u8;
            *shade = ((1.0 - air.exposure) * 64.0 + 0.5) as u8;
        }
    }

    let tint = rgb565(mix([32.0, 47.0, 76.0], [202.0, 218.0, 233.0], sky.daylight));
    let shadow = rgb565([22.0, 34.0, 46.0]);
    let shift = stride.trailing_zeros();
    for y in 0..height {
        let grid_y = y >> shift;
        let yf = y & (stride - 1);
        let mut x = aperture.left[y] as usize;
        let last = aperture.right[y] as usize;
        while x < last {
            let grid_x = x >> shift;
            let xf = (x & (stride - 1)) as i32;
            let end = ((grid_x + 1) * stride).min(last);
            let interpolate = |layer: &[[u8; EDGE]; EDGE], column: usize| {
                ((usize::from(layer[grid_y][column]) * (stride - yf)
                    + usize::from(layer[grid_y + 1][column]) * yf) >> shift) as i32
            };
            let s0 = interpolate(&shade, grid_x);
            let s1 = interpolate(&shade, grid_x + 1);
            let a0 = interpolate(&opacity, grid_x);
            let a1 = interpolate(&opacity, grid_x + 1);
            let sd = s1 - s0;
            let ad = a1 - a0;
            if s0 == 0 && s1 == 0 && a0 == 0 && a1 == 0 {
                x = end;
                continue;
            }
            let mut sh = (s0 << shift) + sd * xf;
            let mut alpha = (a0 << shift) + ad * xf;
            for pixel in &mut pixels[y * width + x..y * width + end] {
                if *pixel != 0 {
                    let shade = (sh >> shift) as u32;
                    let opacity = (alpha >> shift) as u32;
                    if shade != 0 {
                        *pixel = blend565(*pixel, shadow, shade).max(1);
                    }
                    if opacity != 0 {
                        *pixel = blend565(*pixel, tint, opacity).max(1);
                    }
                }
                sh += sd;
                alpha += ad;
            }
            x = end;
        }
    }
}

fn blend565(bottom: u16, top: u16, alpha: u32) -> u16 {
    let a = u32::from(bottom);
    let b = u32::from(top);
    let back = 64 - alpha;
    let rb = ((a & 0xf81f) * back + (b & 0xf81f) * alpha) >> 6;
    let g = ((a & 0x07e0) * back + (b & 0x07e0) * alpha) >> 6;
    ((rb & 0xf81f) | (g & 0x07e0)) as u16
}

fn surface_noise(x: f32, z: f32) -> f32 {
    // Vary a broad patch in world space, with a continuous neutral value at its boundaries.
    let ix = floor_i32(x);
    let iz = floor_i32(z);
    let sx = x - ix as f32;
    let sz = z - iz as f32;
    let taper = 16.0 * sx * (1.0 - sx) * sz * (1.0 - sz);
    0.5 + (cloud_hash(ix, iz) - 0.5) * smoothstep(taper)
}

fn paint_visible_surface(
    surface: Surface, position: [f32; 3], color: u16, rows: core::ops::Range<usize>,
    distance: f32, depth: f32, view: &View, light: GroundLight,
    _rock_phase: f32, mut paint: impl FnMut(usize, usize, u16),
) {
    let resort = surface.vegas_facade();
    if rows.len() >= 3 && (resort.is_some()
        || distance < 160.0 && (surface.is_station_roof() || surface.is_industry()
            || surface.building_palette().is_some()
            || matches!(surface, Surface::PitchedRoof | Surface::FlatRoof | Surface::TowerRoof | Surface::ShipCabin)))
    {
        paint_facade(surface, position, color, rows, view.focal / depth, light, paint);
        return;
    }
    #[cfg(feature = "ground-cover")]
    if surface == Surface::Rock && rows.len() >= 5 && distance < 118.0 {
        let projected = view.focal * view.pitch_cos + (view.cy - rows.start as f32) * view.pitch_sin;
        let elevation_per_row = depth / projected.max(view.focal * 0.45);
        let fade = (1.0 - distance / 118.0).clamp(0.0, 1.0);
        paint_rock_span(_rock_phase, color, rows, elevation_per_row, fade, paint);
        return;
    }
    paint(rows.start, rows.end, color);
}

#[cfg(feature = "ground-cover")]
fn paint_rock_span(
    phase: f32, color: u16, rows: core::ops::Range<usize>, elevation_per_row: f32,
    fade: f32, mut paint: impl FnMut(usize, usize, u16),
) {
    use crate::ground_cover::strata_phase_tone;

    let top = strata_phase_tone(phase);
    let span = rows.len();
    let band_size = span.div_ceil(3).max(2);
    let phase_per_row = 0.083 * elevation_per_row;
    let strength = fade * 59.0;
    let r = i32::from(color >> 11);
    let g = i32::from((color >> 5) & 63);
    let b = i32::from(color & 31);
    let mut first = 0;
    while first < span {
        let end = (first + band_size).min(span);
        let phase_drop = ((first + end) as f32 * 0.5 * phase_per_row).min(0.996);
        let tone = strata_phase_tone(phase - phase_drop);
        let scale = 256 + ((tone - top) * strength) as i32;
        let r = ((r * scale + 128) >> 8).min(31);
        let g = ((g * scale + 128) >> 8).min(63);
        let b = ((b * scale + 128) >> 8).min(31);
        let shaded = ((r << 11) | (g << 5) | b).max(1) as u16;
        paint(rows.start + first, rows.start + end, shaded);
        first = end;
    }
}

fn paint_facade(
    surface: Surface, [x, _, z]: [f32; 3], roof: u16, rows: core::ops::Range<usize>,
    pixel_scale: f32, light: GroundLight, mut paint: impl FnMut(usize, usize, u16),
) {
    let roof_depth = match surface {
        Surface::PitchedRoof | Surface::RailTimber => 1.45,
        Surface::RailMetal => 0.44,
        Surface::RailGlass | Surface::RailRust => 1.01,
        _ => 0.76,
    };
    let cap = ((roof_depth * pixel_scale) as usize).max(1).min(rows.len());
    let roof_end = rows.start + cap;
    let facade = facade_color(surface, light.daylight, light.horizon, light.fog);
    paint(rows.start, roof_end, roof);
    let column = floor_i32(x * 0.31 + z * 0.19);
    let resort = surface.vegas_facade();
    let distant_resort = resort.is_some() && pixel_scale < 0.50;
    if surface.is_industry() || matches!(surface, Surface::RailTimber | Surface::RailMetal | Surface::RailRust)
        || (!distant_resort && (pixel_scale < 0.50 || column.rem_euclid(5) >= 3))
        || roof_end + 2 >= rows.end
    {
        paint(roof_end, rows.end, facade);
        return;
    }
    let reflective = resort.map_or_else(
        || mix([73.0, 89.0, 103.0], [104.0, 149.0, 174.0], light.daylight),
        |finish| mix([37.0, 43.0, 57.0], finish.glass.map(f32::from), light.daylight),
    );
    let night = smoothstep((0.39 - light.daylight) / 0.39);
    let lighted = distant_resort || (column ^ floor_i32(z * 0.25)).rem_euclid(7) >= 2;
    let windows = if lighted { mix(reflective, [240.0, 165.0, 87.0], night) } else { reflective };
    let glass = rgb565(mix(windows, light.horizon, light.fog)).max(1);
    // Group the floors at a distance so individual window rows do not alias.
    let glass = if distant_resort {
        let coverage = smoothstep((pixel_scale - 0.10) / 0.40);
        blend565(facade, glass, (coverage * 27.0) as u32).max(1)
    } else {
        glass
    };
    let floor_height = resort.map_or(2.85, |finish| finish.floor_height);
    let floor = if distant_resort { 3 } else { ((floor_height * pixel_scale) as usize).clamp(2, 9) };
    let mut top = roof_end;
    while top < rows.end {
        let glass_end = (top + floor / 2).min(rows.end);
        let floor_end = (top + floor).min(rows.end);
        paint(top, glass_end, glass);
        if glass_end < floor_end {
            paint(glass_end, floor_end, facade);
        }
        top = floor_end;
    }
}

// Keeping the wall palette outside the inlined span painter makes flat-wall
// shading stable in the Xtensa release build.
#[inline(never)]
fn facade_color(surface: Surface, daylight: f32, horizon: Color, fog: f32) -> u16 {
    let [red, green, blue]: [u8; 3] = match surface {
        Surface::PitchedRoof => [173, 144, 113],
        Surface::FlatRoof => [137, 139, 147],
        Surface::TowerRoof => [119, 142, 161],
        Surface::IndustryBrick => [161, 110, 81],
        Surface::IndustryConcrete => [161, 151, 126],
        Surface::IndustryMetal => [113, 130, 129],
        Surface::RailTimber => [162, 119, 74],
        Surface::RailMetal => [184, 199, 196],
        Surface::RailGlass => [86, 139, 156],
        Surface::RailRust => [111, 85, 69],
        Surface::ShipCabin => [199, 202, 193],
        _ => surface.building_palette().map_or([131, 137, 143], |palette| palette.wall),
    };
    let ambient = 0.32 + daylight * 0.57;
    let subdued = [
        f32::from(red) * ambient,
        f32::from(green) * ambient,
        f32::from(blue) * ambient + 7.0,
    ];
    rgb565(mix(subdued, horizon, fog)).max(1)
}

fn surface_detail(surface: Surface, [x, y, z]: [f32; 3], fade: f32) -> f32 {
    if fade <= 0.0 {
        return 0.0;
    }
    let variation = match surface {
        Surface::DeepWater | Surface::Shallows | Surface::RiverWater
        | Surface::TarnWater | Surface::TarnShallows => {
            let streak_u = (x * 0.21 + z * 0.08) * 0.43;
            let streak_v = (z * 0.19 - x * 0.08) * 0.61;
            let waves = surface_noise(streak_u, streak_v);
            (waves - 0.5) * if surface == Surface::Shallows { 0.24 } else { 0.15 }
        }
        Surface::Grass | Surface::Forest | Surface::Park | Surface::TreeCanopy | Surface::SpruceCanopy
        | Surface::AspenCanopy | Surface::MapleCanopy | Surface::JuniperCanopy
        | Surface::AlpineMeadow => {
            let meadow = surface_noise(x * 0.085 + z * 0.029, z * 0.095 - x * 0.024);
            let fine = cloud_hash(floor_i32(x * 0.38), floor_i32(z * 0.38));
            0.19 * (meadow - 0.40) + 0.075 * (fine - 0.48)
        }
        Surface::ReedMarsh => {
            let stands = surface_noise(x * 0.17 + z * 0.09, z * 0.21 - x * 0.06);
            let reed_color = cloud_hash(floor_i32(x * 0.54), floor_i32(z * 0.34));
            0.28 * (stands - 0.45) + 0.12 * (reed_color - 0.42)
        }
        Surface::WetMeadow => {
            let swales = surface_noise(x * 0.077 + z * 0.021, z * 0.058 - x * 0.018);
            let tussocks = cloud_hash(floor_i32(x * 0.38), floor_i32(z * 0.38));
            0.27 * (swales - 0.48) + 0.09 * (tussocks - 0.49)
        }
        Surface::RiverSilt => {
            let ripples = surface_noise(x * 0.18 + z * 0.04, z * 0.068 - x * 0.016);
            0.23 * (ripples - 0.49)
        }
        Surface::ScreeFine | Surface::ScreeCoarse => {
            let coarse = surface == Surface::ScreeCoarse;
            let scale = if coarse { 0.17 } else { 0.31 };
            let chips = surface_noise((x + y * 0.19) * scale, (z - y * 0.26) * scale);
            let beds = surface_noise(x * 0.056 + y * 0.09, z * 0.069 - y * 0.07);
            let grain_relief = if coarse { 0.34 } else { 0.19 };
            (chips - 0.49) * grain_relief + (beds - 0.51) * 0.14
        }
        Surface::GlacierIce | Surface::GlacierCrevasse => {
            let bands = surface_noise(x * 0.042 + y * 0.14 + z * 0.017, z * 0.063 - x * 0.013 - y * 0.062);
            // Wide blue-white bands stay readable without turning distant crevasses into noise.
            (bands - 0.51) * if surface == Surface::GlacierIce { 0.32 } else { 0.19 }
        }
        Surface::MesaCliff => {
            let height = y * 0.28 + x * 0.013 - z * 0.009;
            let strata = height - floor_i32(height) as f32;
            0.31 * (smoothstep((strata - 0.14) * 4.0) * smoothstep((1.0 - strata) * 5.0) - 0.51)
        }
        Surface::Snow => {
            let drifts = surface_noise(x * 0.075 + y * 0.081 + z * 0.021, z * 0.046 - y * 0.067);
            (drifts - 0.46) * 0.21
        }
        Surface::Rock | Surface::Desert | Surface::Mojave | Surface::Sand
        | Surface::SaltCrust | Surface::DuneSand | Surface::DryWash
        | Surface::GlacierMoraine | Surface::MesaCap | Surface::MesaApron => {
            // Samples are anchored to the terrain so the bands do not crawl under the camera.
            let layers = surface_noise(x * 0.049 + y * 0.13, z * 0.072 - y * 0.04);
            let grain = cloud_hash(floor_i32(x * 0.45), floor_i32(z * 0.45));
            (layers - 0.48) * 0.24 + (grain - 0.46) * 0.055
        }
        _ => 0.0,
    };
    variation * fade
}

struct SkyShader {
    view: View,
    atmosphere: Atmosphere,
    horizon: Color,
    twilight: f32,
    zenith: Color,
    sun_position: (f32, f32),
    cloud_color: Color,
    inverse_focal: f32,
    halo_strength: f32,
    cloud: [CloudCell; 2],
}

impl SkyShader {
    fn new(view: View, atmosphere: Atmosphere, horizon: Color) -> Self {
        let twilight = atmosphere.twilight();
        let blue = mix([7.0, 15.0, 42.0], [43.0, 109.0, 181.0], atmosphere.daylight);
        let pale_zenith = mix(blue, [61.0, 67.0, 129.0], twilight * 0.30);
        let zenith = mix(pale_zenith, [18.0, 34.0, 83.0], view.high_cruise * 0.36);
        let sun_forward =
            atmosphere.sun[0] * view.heading_sin + atmosphere.sun[2] * view.heading_cos;
        let sun_right = atmosphere.sun[0] * view.heading_cos - atmosphere.sun[2] * view.heading_sin;
        let sun_depth = sun_forward * view.pitch_cos - atmosphere.sun[1] * view.pitch_sin;
        let sun_position = if sun_depth > 0.08 {
            (
                sun_right / sun_depth,
                -(atmosphere.sun[1] * view.pitch_cos + sun_forward * view.pitch_sin) / sun_depth,
            )
        } else {
            (20.0, 20.0)
        };
        let cloud_base = mix(
            [25.0, 37.0, 66.0],
            [235.0, 238.0, 232.0],
            atmosphere.daylight,
        );
        let cloud_color = mix(cloud_base, [214.0, 146.0, 128.0], twilight * 0.53);
        let inverse_focal = 1.0 / view.focal;

        let halo_strength = 0.09 + atmosphere.daylight * 0.19 + twilight * 0.28;
        Self {
            view,
            atmosphere,
            horizon,
            twilight,
            zenith,
            sun_position,
            cloud_color,
            inverse_focal,
            halo_strength,
            cloud: [CloudCell::START; 2],
        }
    }

    fn shade(&mut self, x_center: f32, y_center: f32) -> u16 {
        let Self {
            view,
            atmosphere,
            horizon,
            twilight,
            zenith,
            sun_position,
            cloud_color,
            inverse_focal,
            halo_strength,
            ..
        } = *self;
        let xx = x_center - view.cx;
        let yy = y_center - view.cy;
        let u = (xx * view.roll_cos + yy * view.roll_sin) * inverse_focal;
        let v = (-xx * view.roll_sin + yy * view.roll_cos) * inverse_focal;
        let altitude = (-view.pitch_sin - v * view.pitch_cos).max(0.0);
        let height_mix = (altitude * (1.21 + twilight * 0.62)).clamp(0.0, 1.0);
        let mut color = mix(horizon, zenith, height_mix * (2.0 - height_mix));
        let sun_x = u - sun_position.0;
        let sun_y = v - sun_position.1;
        let sun_r2 = sun_x * sun_x + sun_y * sun_y;
        if sun_r2 < 1.0 / 7.0 {
            let bloom = (1.0 - sun_r2 * 7.0).clamp(0.0, 1.0);
            color = mix(color, [255.0, 188.0, 121.0], bloom * bloom * halo_strength);
        }
        if sun_r2 < 0.00085 {
            color = mix(
                color,
                [255.0, 235.0, 173.0],
                (1.0 - sun_r2 / 0.00085).clamp(0.0, 1.0),
            );
        }
        let opacity = cloud_opacity(u, v, altitude, &view, atmosphere, &mut self.cloud);
        rgb565(mix(color, cloud_color, opacity))
    }
}

/// Raster-order sky colors for the exact circular aperture, independent of the landscape.
/// One call to next shades at most one sky group, for cooperative hardware scheduling.
pub struct SkyTileRaster {
    shader: SkyShader,
    width: usize,
    height: usize,
    diameter: usize,
    step: usize,
    row: usize,
    x: usize,
    end: usize,
    y_center: f32,
}

impl SkyTileRaster {
    /// Upper bound for a circular, square framebuffer's tile count.
    pub const fn square_tile_capacity(side: usize) -> usize {
        side.div_ceil(2) * side.div_ceil(sky_tile_step(side))
    }

    pub fn new(
        camera: Camera,
        sky: &Sky,
        (width, height): (usize, usize),
    ) -> Result<Self, RenderError> {
        if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
            return Err(RenderError::InvalidDimensions);
        }
        if !camera_is_finite(camera) {
            return Err(RenderError::InvalidCamera);
        }
        let atmosphere = sky_atmosphere(sky)?;
        let diameter = width.min(height);
        let mut raster = Self {
            shader: SkyShader::new(
                View::new(camera, width, height),
                atmosphere,
                atmosphere.horizon(),
            ),
            width,
            height,
            diameter,
            step: sky_tile_step(diameter),
            row: 0,
            x: 0,
            end: 0,
            y_center: 0.0,
        };
        // Skip empty letterbox pairs during setup or row advancement without shading.
        raster.start_row();
        Ok(raster)
    }

    fn start_row(&mut self) {
        // Most calls cross one row pair. For portrait view, jump the leading
        // empty pairs without probing each of them.
        let first_pixel_row = (self.height - self.diameter).saturating_sub(1) / 2;
        self.row = self.row.max(first_pixel_row & !1);
        if self.row >= self.height {
            self.x = 0;
            self.end = 0;
            return;
        }
        let lower = (self.row + 1).min(self.height - 1);
        let (left0, right0) = aperture_row(self.width, self.height, self.diameter, self.row);
        let (left1, right1) = aperture_row(self.width, self.height, self.diameter, lower);
        self.x = usize::from(left0.min(left1));
        self.end = usize::from(right0.max(right1));
        self.y_center = (self.row + lower) as f32 * 0.5;
    }
}

impl Iterator for SkyTileRaster {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= self.end {
            self.row += 2;
            self.start_row();
            if self.x >= self.end {
                return None;
            }
        }
        let x = self.x;
        let end = (x + self.step).min(self.end);
        self.x = end;
        Some(self.shader.shade((x + end - 1) as f32 * 0.5, self.y_center))
    }
}

fn paint_sky(
    pixels: &mut [u16],
    width: usize,
    height: usize,
    aperture: &Aperture,
    view: &View,
    atmosphere: Atmosphere,
    horizon: Color,
    mut ready_sky_tile: impl FnMut(usize) -> Option<u16>,
) {
    let step = sky_tile_step(aperture.diameter);
    let mut fallback = None;
    let mut index = 0;
    for row in (0..height).step_by(2) {
        let lower = (row + 1).min(height - 1);
        let start = usize::from(aperture.left[row].min(aperture.left[lower]));
        let end = usize::from(aperture.right[row].max(aperture.right[lower]));
        let row_is_ground = |y: usize| {
            let left = y * width + usize::from(aperture.left[y]);
            let right = y * width + usize::from(aperture.right[y]);
            pixels[left..right].iter().all(|&pixel| pixel != 0)
        };
        if row_is_ground(row) && row_is_ground(lower) {
            // The worker numbers tiles by aperture, including those hidden by land.
            index += (end - start).div_ceil(step);
            continue;
        }
        let y_center = (row + lower) as f32 * 0.5;
        for x in (start..end).step_by(step) {
            let tile = index;
            index += 1;
            let limit = (x + step).min(end);
            let spans = |y: usize| {
                let from = x.max(usize::from(aperture.left[y]));
                let to = limit.min(usize::from(aperture.right[y]));
                y * width + from..y * width + from.max(to)
            };
            let first = spans(row);
            let second = spans(lower);
            if pixels[first.clone()].iter().all(|&pixel| pixel != 0)
                && pixels[second.clone()].iter().all(|&pixel| pixel != 0)
            {
                continue;
            }
            let shade = ready_sky_tile(tile).unwrap_or_else(|| {
                fallback
                    .get_or_insert_with(|| SkyShader::new(*view, atmosphere, horizon))
                    .shade((x + limit - 1) as f32 * 0.5, y_center)
            });
            for span in [first, second] {
                for pixel in &mut pixels[span] {
                    if *pixel == 0 {
                        *pixel = shade;
                    }
                }
            }
        }
    }
}

fn cloud_opacity(
    u: f32, v: f32, altitude: f32, view: &View, atmosphere: Atmosphere, cloud: &mut [CloudCell; 2],
) -> f32 {
    if atmosphere.clouds <= 0.03 || altitude <= 0.03 {
        return 0.0;
    }
    let forward = view.pitch_cos - v * view.pitch_sin;
    let cloud_x = view.cloud_origin[0] + atmosphere.cloud_offset[0] * 0.27
        + 6.2 * (view.heading_sin * forward + view.heading_cos * u);
    let cloud_z = view.cloud_origin[1] + atmosphere.cloud_offset[1] * 0.27
        + 6.2 * (view.heading_cos * forward - view.heading_sin * u)
        + altitude * 3.1;
    let large = cloud[0].sample(cloud_x, cloud_z);
    let fine = cloud[1].sample(cloud_x * 2.07 + 7.1, cloud_z * 2.07 - 5.3);
    let shape = large * 0.78 + fine * 0.22;
    let fade = ((altitude - 0.03) * 5.5).min(1.0);
    ((shape - (0.72 - atmosphere.clouds * 0.35)) * 4.25).clamp(0.0, 0.85) * fade
}

struct MoonPatch {
    left: usize,
    top: usize,
    sky: [u16; 16],
    center: [f32; 2],
    radius: f32,
    light: [f32; 3],
    strength: f32,
    low_horizon: f32,
}

impl MoonPatch {
    fn new(
        pixels: &[u16],
        width: usize,
        height: usize,
        aperture: &Aperture,
        view: &View,
        atmosphere: Atmosphere,
    ) -> Option<Self> {
        let altitude = atmosphere.moon[1];
        if altitude < -0.025 {
            return None;
        }
        let project = |[x, y, z]: [f32; 3]| {
            let forward = x * view.heading_sin + z * view.heading_cos;
            let right = x * view.heading_cos - z * view.heading_sin;
            let down = -(y * view.pitch_cos + forward * view.pitch_sin);
            [right, down, forward * view.pitch_cos - y * view.pitch_sin]
        };
        let [right, down, depth] = project(atmosphere.moon);
        if depth < 0.10 {
            return None;
        }
        let u = right / depth;
        let v = down / depth;
        let center = [
            view.cx + view.focal * (u * view.roll_cos - v * view.roll_sin),
            view.cy + view.focal * (u * view.roll_sin + v * view.roll_cos),
        ];
        // Slightly enlarge the disc so its phase reads at the watch resolution.
        let radius = (view.focal * 0.018).clamp(2.1, 6.9);
        let edge = radius + 0.48;
        if !(-edge..width as f32 + edge).contains(&center[0])
            || !(-edge..height as f32 + edge).contains(&center[1])
        {
            return None;
        }
        let left = floor_i32(center[0] - edge).max(0) as usize;
        let top = floor_i32(center[1] - edge).max(0) as usize;
        let right = (floor_i32(center[0] + edge) + 1).clamp(0, width as i32) as usize;
        let bottom = (floor_i32(center[1] + edge) + 1).clamp(0, height as i32) as usize;
        if left >= right || top >= bottom {
            return None;
        }

        // Illumination across a sphere comes from the sun's direction, with
        // the terminator facing the sun on either side of the lunar cycle.
        let sun = atmosphere.sun;
        let [mx, my, mz] = atmosphere.moon;
        let toward = (-(mx * sun[0] + my * sun[1] + mz * sun[2])).clamp(-1.0, 1.0);
        if toward < -0.99 {
            return None;
        }
        let [sx, sy, sz] = project(sun);
        let lit_x = sx - u * sz;
        let lit_y = sy - v * sz;
        let tangent_length = moon_sqrt(lit_x * lit_x + lit_y * lit_y);
        let weight = moon_sqrt(1.0 - toward * toward) / tangent_length.max(0.001);
        let light = [
            (lit_x * view.roll_cos - lit_y * view.roll_sin) * weight,
            (lit_x * view.roll_sin + lit_y * view.roll_cos) * weight,
            toward,
        ];
        let mut sky = [0; 16];
        for y in top..bottom {
            let start = left.max(usize::from(aperture.left[y]));
            let end = right.min(usize::from(aperture.right[y]));
            for x in start..end {
                if pixels[y * width + x] == 0 {
                    sky[y - top] |= 1 << (x - left);
                }
            }
        }
        if sky == [0; 16] {
            return None;
        }
        let strength = (0.46 + (1.0 - atmosphere.daylight) * 0.43)
            * smoothstep((altitude + 0.025) / 0.075);
        let low_horizon = 1.0 - smoothstep(altitude / 0.21);
        Some(Self { left, top, sky, center, radius, light, strength, low_horizon })
    }

    fn paint(self, pixels: &mut [u16], width: usize, view: &View, atmosphere: Atmosphere) {
        let mut cloud = [CloudCell::START; 2];
        let lit_color = mix([219.0, 231.0, 248.0], [243.0, 211.0, 170.0], self.low_horizon);
        for (row, mut columns) in self.sky.into_iter().enumerate() {
            let y = self.top + row;
            while columns != 0 {
                let column = columns.trailing_zeros() as usize;
                columns &= columns - 1;
                let x = self.left + column;
                let nx = (x as f32 - self.center[0]) / self.radius;
                let ny = (y as f32 - self.center[1]) / self.radius;
                let distance = nx * nx + ny * ny;
                let limb = ((1.0 - distance) * self.radius * 0.64 + 0.5).clamp(0.0, 1.0);
                if limb == 0.0 {
                    continue;
                }
                let nz = moon_sqrt((1.0 - distance).max(0.0));
                let sun = nx * self.light[0] + ny * self.light[1] + nz * self.light[2];
                let day_side = ((sun * self.radius * 1.2) + 0.5).clamp(0.0, 1.0);
                let xx = x as f32 - view.cx;
                let yy = y as f32 - view.cy;
                let u = (xx * view.roll_cos + yy * view.roll_sin) / view.focal;
                let v = (-xx * view.roll_sin + yy * view.roll_cos) / view.focal;
                let altitude = (-view.pitch_sin - v * view.pitch_cos).max(0.0);
                let clear = 1.0 - cloud_opacity(u, v, altitude, view, atmosphere, &mut cloud);
                let alpha = limb * self.strength * clear * clear * clear;
                let pixel = &mut pixels[y * width + x];
                let background = [
                    f32::from(*pixel >> 11) * (255.0 / 31.0),
                    f32::from((*pixel >> 5) & 63) * (255.0 / 63.0),
                    f32::from(*pixel & 31) * (255.0 / 31.0),
                ];
                let dim_side = mix(background, [74.0, 87.0, 108.0], 0.13 * (1.0 - atmosphere.daylight));
                *pixel = rgb565(mix(background, mix(dim_side, lit_color, day_side), alpha));
            }
        }
    }
}

fn moon_sqrt(value: f32) -> f32 {
    square_root((value * 262_144.0) as u32) as f32 / 512.0
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Star {
    pixel: usize,
    u: f32,
    v: f32,
    intensity: f32,
}

impl Star {
    const EMPTY: Self = Self {
        pixel: 0,
        u: 0.0,
        v: 0.0,
        intensity: 0.0,
    };
}

struct Stars {
    visible: [Star; 48],
    count: usize,
}

// Directions and intrinsic brightness do not depend on the camera or sky.
// Use the same math to build them once rather than hashing and recomputing
// two angles for each star on every twilight and night frame.
static STAR_CATALOGUE: [[f32; 4]; 240] = {
    let mut stars = [[0.0; 4]; 240];
    let mut i = 0;
    while i < stars.len() {
        let (azimuth_sin, azimuth_cos) = sin_cos(cloud_hash(i as i32, 1181) * TAU);
        let (up, radial) = sin_cos(0.09 + cloud_hash(i as i32, 271) * 1.43);
        let shine = cloud_hash(i as i32, 801);
        stars[i] = [azimuth_sin * radial, up, azimuth_cos * radial, 0.10 + shine * shine * 0.60];
        i += 1;
    }
    stars
};

fn visible_stars(
    pixels: &[u16],
    width: usize,
    height: usize,
    aperture: &Aperture,
    view: &View,
    atmosphere: Atmosphere,
) -> Stars {
    let mut stars = Stars {
        visible: [Star::EMPTY; 48],
        count: 0,
    };
    let fade = smoothstep((0.80 - atmosphere.daylight) / 0.72);
    if fade <= 0.002 {
        return stars;
    }

    for &[x, up, z, brightness] in &STAR_CATALOGUE {
        let forward = x * view.heading_sin + z * view.heading_cos;
        let camera_depth = forward * view.pitch_cos - up * view.pitch_sin;
        if camera_depth < 0.10 {
            continue;
        }
        let u = (x * view.heading_cos - z * view.heading_sin) / camera_depth;
        let v = -(up * view.pitch_cos + forward * view.pitch_sin) / camera_depth;
        let screen_x =
            floor_i32(view.cx + view.focal * (u * view.roll_cos - v * view.roll_sin) + 0.5);
        let screen_y =
            floor_i32(view.cy + view.focal * (u * view.roll_sin + v * view.roll_cos) + 0.5);
        if screen_y < 0 || screen_y >= height as i32 {
            continue;
        }
        let row = screen_y as usize;
        if screen_x < i32::from(aperture.left[row]) || screen_x >= i32::from(aperture.right[row]) {
            continue;
        }
        let pixel = row * width + screen_x as usize;
        if pixels[pixel] != 0 {
            continue;
        }

        let intensity = brightness * fade;
        stars.visible[stars.count] = Star {
            pixel,
            u,
            v,
            intensity,
        };
        stars.count += 1;
        if stars.count == stars.visible.len() {
            break;
        }
    }
    stars
}

fn paint_stars(pixels: &mut [u16], stars: &Stars, view: &View, atmosphere: Atmosphere) {
    let mut cloud = [CloudCell::START; 2];
    for &star in &stars.visible[..stars.count] {
        let pixel = &mut pixels[star.pixel];
        let color = [
            f32::from(*pixel >> 11) * (255.0 / 31.0),
            f32::from((*pixel >> 5) & 63) * (255.0 / 63.0),
            f32::from(*pixel & 31) * (255.0 / 31.0),
        ];
        let altitude = (-view.pitch_sin - star.v * view.pitch_cos).max(0.0);
        let opacity = cloud_opacity(star.u, star.v, altitude, view, atmosphere, &mut cloud);
        let clear = 1.0 - opacity;
        let alpha = star.intensity * clear * clear;
        *pixel = rgb565(mix(color, [219.0, 233.0, 255.0], alpha));
    }
}

fn paint_span<const CLIPPED: bool>(
    pixels: &mut [u16],
    width: usize,
    height: usize,
    aperture: &Aperture,
    view: &View,
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
    color: u16,
) {
    if !CLIPPED || (view.roll_sin.abs() < 0.000_001 && view.roll_cos > 0.0) {
        if y1 > y0 && x1 > x0 && x1 - x0 <= 2 {
            // In the round aperture the rows between the endpoints can only
            // be wider than the narrower endpoint.
            let inside = !CLIPPED || (x0 >= usize::from(aperture.left[y0].max(aperture.left[y1 - 1]))
                && x1 <= usize::from(aperture.right[y0].min(aperture.right[y1 - 1])));
            if inside {
                let first = y0 * width + x0;
                let end = (y1 - 1) * width + x1;
                let columns = &mut pixels[first..end];
                if x1 == x0 + 1 {
                    for pixel in columns.iter_mut().step_by(width) {
                        *pixel = color;
                    }
                } else {
                    for row in columns.chunks_mut(width) {
                        row[..2].fill(color);
                    }
                }
                return;
            }
        }
        for y in y0..y1 {
            let (first, last) = if CLIPPED {
                (
                    x0.max(aperture.left[y] as usize),
                    x1.min(aperture.right[y] as usize),
                )
            } else {
                (x0, x1)
            };
            if first < last {
                pixels[y * width + first..y * width + last].fill(color);
            }
        }
        return;
    }

    // Scan across the dominant screen direction of a rolled vertical band.
    // Using the inverse transform for pixel membership leaves no seams between bands.
    let u0 = x0 as f32 - view.cx - 0.5;
    let u1 = x1 as f32 - view.cx - 0.5;
    let v0 = y0 as f32 - view.cy - 0.5;
    let v1 = y1 as f32 - view.cy - 0.5;
    let s = view.roll_sin;
    let c = view.roll_cos;
    let rows = c.abs() >= s.abs();
    let (axis_u, axis_v, center, extent) = if rows {
        (s, c, view.cy, height)
    } else {
        (c, -s, view.cx, width)
    };
    let a = axis_u * u0;
    let b = axis_u * u1;
    let d = axis_v * v0;
    let e = axis_v * v1;
    let first = floor_i32(center + a.min(b) + d.min(e)).max(0) as usize;
    let last = (floor_i32(center + a.max(b) + d.max(e)) + 1).clamp(0, extent as i32) as usize;
    let inverse = 1.0 / if rows { c } else { s };
    for major in first..last {
        let offset = major as f32 - center;
        let (minor_center, minor_extent, contribution) = if rows {
            (view.cx, width, s * offset)
        } else {
            (view.cy, height, c * offset)
        };
        let near = minor_center + (u0 - contribution) * inverse;
        let far = minor_center + (u1 - contribution) * inverse;
        let lo = floor_i32(near.min(far)).max(0) as usize;
        let hi = (floor_i32(near.max(far)) + 1).clamp(0, minor_extent as i32) as usize;
        for minor in lo..hi {
            let (x, y) = if rows { (minor, major) } else { (major, minor) };
            if x < aperture.left[y] as usize || x >= aperture.right[y] as usize {
                continue;
            }
            let xx = x as f32 - view.cx;
            let yy = y as f32 - view.cy;
            let u = c * xx + s * yy;
            let v = -s * xx + c * yy;
            if u >= u0 && u < u1 && v >= v0 && v < v1 {
                pixels[y * width + x] = color;
            }
        }
    }
}

fn mix(from: Color, to: Color, amount: f32) -> Color {
    [
        from[0] + (to[0] - from[0]) * amount,
        from[1] + (to[1] - from[1]) * amount,
        from[2] + (to[2] - from[2]) * amount,
    ]
}

fn rgb565(c: Color) -> u16 {
    let red = (c[0].clamp(0.0, 255.0) * (31.0 / 255.0) + 0.5) as u16;
    let green = (c[1].clamp(0.0, 255.0) * (63.0 / 255.0) + 0.5) as u16;
    let blue = (c[2].clamp(0.0, 255.0) * (31.0 / 255.0) + 0.5) as u16;
    (red << 11) | (green << 5) | blue
}

#[derive(Clone, Copy)]
struct CloudCell {
    ix: i32,
    iy: i32,
    corners: [f32; 4],
}

impl CloudCell {
    const START: Self = Self::new(0, 0);

    const fn new(ix: i32, iy: i32) -> Self {
        let right = ix.wrapping_add(1);
        let lower = iy.wrapping_add(1);
        Self { ix, iy, corners: [
            cloud_hash(ix, iy), cloud_hash(right, iy),
            cloud_hash(ix, lower), cloud_hash(right, lower),
        ] }
    }

    fn sample(&mut self, x: f32, y: f32) -> f32 {
        let ix = floor_i32(x);
        let iy = floor_i32(y);
        if (ix, iy) != (self.ix, self.iy) {
            *self = Self::new(ix, iy);
        }
        let fx = ease(x - ix as f32);
        let fy = ease(y - iy as f32);
        let [a, b, c, d] = self.corners;
        let top = a + (b - a) * fx;
        let bottom = c + (d - c) * fx;
        top + (bottom - top) * fy
    }
}

fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

const fn cloud_hash(x: i32, y: i32) -> f32 {
    let k = (x as u32).wrapping_mul(0x85eb_ca77) ^ (y as u32).wrapping_mul(0x9e37_79b1);
    let mixed = (k ^ (k >> 15)).wrapping_mul(0xc2b2_ae3d);
    ((mixed ^ (mixed >> 16)) >> 16) as f32 * (1.0 / 65535.0)
}

pub(crate) fn floor_i32(value: f32) -> i32 {
    let integer = value as i32;
    if value < integer as f32 {
        integer.saturating_sub(1)
    } else {
        integer
    }
}

fn square_root(mut value: u32) -> u32 {
    let mut result = 0;
    let mut place = 1 << 18;
    while place > value {
        place >>= 2;
    }
    while place != 0 {
        if value >= result + place {
            value -= result + place;
            result = (result >> 1) + place;
        } else {
            result >>= 1;
        }
        place >>= 2;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_cruise_covers_more_ground_without_spending_more_rings() {
        let measure = |altitude: f32, far: f32, stride_growth: f32, legacy: bool| {
            let boost = high_cruise(altitude);
            let mut distance = if legacy { 5.0 } else { first_terrain_distance(altitude) };
            let mut rings = 0;
            let mut city_rings = 0;
            while distance < far {
                rings += 1;
                city_rings += usize::from((150.0..=550.0).contains(&distance));
                distance = if legacy {
                    let step = (1.8 + distance * (0.064 + boost * 0.043))
                        * (1.0 + stride_growth * smoothstep((distance - 60.0) / 60.0));
                    (distance + step).min(far)
                } else {
                    next_terrain_distance(distance, far, boost, stride_growth)
                };
            }
            (rings, city_rings)
        };
        assert_eq!(terrain_detail_distance(45.0), 800.0);
        assert!(terrain_detail_distance(225.0) > 800.0);
        assert_eq!(terrain_detail_distance(325.0), 1440.0);
        for stride in [0.0, 0.70, 1.05, 2.20] {
            let low_before = measure(45.0, 640.0, stride, true);
            let low = measure(45.0, terrain_detail_distance(45.0), stride, false);
            assert!(low.0 <= low_before.0 + 1, "{low_before:?} vs {low:?}");
            let old = measure(325.0, 896.0, stride, true);
            let new = measure(325.0, terrain_detail_distance(325.0), stride, false);
            assert!(new.0 <= old.0 + 1, "{old:?} vs {new:?}");
            assert!(new.1 >= old.1 + 2, "{old:?} vs {new:?}");
        }
    }

    #[test]
    fn nearby_crowns_keep_subtle_species_color_within_one_woodland_palette() {
        let position = [12.0, 32.0, -9.0];
        let eye = Camera { x: 12.0, z: -9.0, altitude: 45.0, heading: 0.0, pitch: 0.2, bank: 0.0 };
        let light = GroundLight {
            daylight: 1.0, ambient: 1.0, light_up: 0.6, horizon: [0.0; 3], fog: 0.0,
            light_forward: 0.0, light_right: 0.0, lateral: 0.0, reflection: [0.0; 5],
            #[cfg(feature = "ground-cover")]
            ground_seed: 43_719,
        };
        let species = [Surface::TreeCanopy, Surface::SpruceCanopy, Surface::AspenCanopy,
            Surface::MapleCanopy, Surface::JuniperCanopy];
        let paint = |surface, camera| ground_color(surface, position, || [0.0; 2], 0.0, light, camera).0;
        let near = species.map(|surface| paint(surface, &eye));
        let channels = near.map(|color| [color >> 11, (color >> 5) & 63, color & 31]);
        for (channel, limit) in [3, 3, 2].into_iter().enumerate() {
            let values = channels.map(|color| color[channel]);
            assert!(values.iter().max().unwrap() - values.iter().min().unwrap() <= limit);
        }
        assert!(near.iter().any(|color| color != &near[0]));

        let distant = Camera { z: eye.z - 160.0, ..eye };
        let far = species.map(|surface| paint(surface, &distant));
        assert!(far.iter().all(|color| color == &far[0]));
    }

    #[cfg(feature = "ground-cover")]
    #[test]
    fn long_rock_spans_sample_multiple_beds_and_fill_every_row() {
        let rows = 12..25;
        let base = rgb565([164.0, 147.0, 121.0]);
        let mut shaded = [0_u16; 13];
        let phase = crate::ground_cover::strata_phase([32.0, 112.0, -88.0], 11);
        paint_rock_span(phase, base, rows.clone(), 0.91, 1.0,
            |a, b, color| {
                let span = &mut shaded[a - rows.start..b - rows.start];
                assert!(span.iter().all(|&pixel| pixel == 0));
                span.fill(color);
            });
        assert!(shaded.iter().all(|&pixel| pixel != 0));
        assert!(shaded.windows(2).any(|pair| pair[0] != pair[1]));

        paint_rock_span(phase, base, rows.clone(), 0.91, 0.0,
            |_, _, color| assert_eq!(color, base));
    }

    #[test]
    fn planar_slope_lighting_is_invariant_to_column_width_and_spread() {
        let elevation = |forward: f32, side: f32| 48.0 + 0.4 * forward - 0.6 * side;
        for [side, prior, left] in [[5.0, 4.0, 4.0], [5.5, 4.0, 3.5], [-8.0, -7.0, -10.5]] {
            let actual = terrain_grade(
                [side, elevation(30.0, side)], [prior, elevation(25.0, prior)],
                Some([left, elevation(30.0, left)]), 0.2,
            );
            assert!((actual[0] - 0.4).abs() < 0.000_01);
            assert!((actual[1] + 0.6).abs() < 0.000_01);
        }
    }

    #[test]
    fn compressed_backdrop_depth_orders_distant_skylines_and_terrain() {
        assert_eq!(encoded_depth(896.0), 896 * 32);
        assert_eq!(encoded_depth(SCENERY_HORIZON), 1_920 * 32);
        for [near, far] in [[1_920.0, 2_000.0], [2_200.0, 2_240.0], [2_880.0, 6_000.0],
                            [6_000.0, TERRAIN_BACKDROP_DISTANCE]] {
            assert!(encoded_depth(near) < encoded_depth(far));
        }
        assert!(encoded_depth(TERRAIN_BACKDROP_DISTANCE) < u16::MAX - 1);
        assert!(encoded_depth(f32::INFINITY) < u16::MAX);
    }

    #[test]
    fn low_flight_terrain_retains_roofs_beyond_the_near_mesh_limit() {
        const SIDE: usize = 60;
        let camera = Camera {
            x: 0.0, z: 0.0, altitude: 56.0, heading: 0.0, pitch: 0.18, bank: 0.0,
        };
        let sky = Sky::from_time(49, 0, 0.44);
        let flat = Sample { elevation: 12.0, surface: Surface::Grass };
        let mut work = [0; SIDE * SIDE];
        let mut plain = [0; SIDE * SIDE];
        let mut city = plain;
        render_from_with_sky_buffered(|_, _| flat, camera, &sky, &mut plain, &mut work, SIDE, SIDE).unwrap();
        render_from_with_sky_buffered(
            |x, z| {
                if x.abs() < 48.0 && (690.0..=789.0).contains(&z) {
                    Sample { elevation: 42.0, surface: Surface::FlatRoof }
                } else {
                    flat
                }
            },
            camera, &sky, &mut city, &mut work, SIDE, SIDE,
        ).unwrap();
        assert_eq!(view_distance(camera.altitude), 512.0);
        assert_eq!(terrain_detail_distance(camera.altitude), 800.0);
        assert!(city.iter().zip(plain).filter(|(roof, ground)| **roof != *ground).count() >= 12);
    }

    #[test]
    fn a_far_ridge_reaches_the_display_at_low_and_cruise_altitudes() {
        const SIDE: usize = 60;
        let aperture = Aperture::new(SIDE, SIDE);
        let atmosphere = Atmosphere::DAY;
        for altitude in [28.0, 140.0, 350.0] {
            // A faint silhouette can be narrower than one raster pixel.
            // Check several subpixel alignments instead of fixing a row.
            let visible = (0..8).any(|alignment| {
                let camera = Camera {
                    x: 0.0, z: 0.0, altitude, heading: 0.0,
                    pitch: 0.13 + alignment as f32 * 0.004,
                    bank: 0.0,
                };
                let view = View::new(camera, SIDE, SIDE);
                let capture = |ridge| {
                    let mut visible = [SIDE as i16; SIDE];
                    trace_far_horizon(
                        &mut |_, z| if ridge && z > 12_000.0 { 140.0 } else { 0.0 },
                        camera, atmosphere, atmosphere.horizon(), &view, &aperture, &mut visible,
                        1, SIDE, SIDE, |_, _, _, _, _, _| {},
                    );
                    visible
                };
                capture(true).iter().zip(capture(false)).any(|(&land, water)| land < water)
            });
            assert!(visible, "distant ridge should break the horizon at altitude {altitude}");
        }
    }

    #[test]
    fn blank_rotation_rows_match_when_the_source_is_clamped() {
        for (width, height) in [(52, 113), (113, 52), (466, 466)] {
            let aperture = Aperture::new(width, height);
            for first_ground in [height / 3, height / 2, height] {
                let mut source = std::vec![u16::MAX; width * height];
                for (index, depth) in source.iter_mut().enumerate().skip(width * first_ground) {
                    *depth = ((index * 31) % 65534) as u16;
                }
                for bank in [-2.8, -0.19, 0.035, 0.5] {
                    let camera = Camera {
                        x: 0.0, z: 0.0, altitude: 30.0, heading: 0.0, pitch: 0.0, bank,
                    };
                    let view = View::new(camera, width, height);
                    let rect = ScreenRect {
                        left: 3, top: 1, right: width - 2, bottom: height - 3,
                    };
                    let mut reference = std::vec![u16::MAX; width * height];
                    let mut candidate = reference.clone();
                    rotate_terrain::<false, false>(
                        &source, &mut reference, width, height, &aperture, &view, rect, 0, None,
                    );
                    rotate_terrain::<false, false>(
                        &source, &mut candidate, width, height, &aperture, &view, rect, first_ground, None,
                    );
                    assert_eq!(candidate, reference);
                }
            }
        }
    }

    #[test]
    fn dedicated_upright_depth_matches_the_inline_scratch_path() {
        const DIMENSIONS: (usize, usize) = (466, 466);
        let count = DIMENSIONS.0 * DIMENSIONS.1;
        let world = World::new(41);
        let sky = Sky::at(41, 3_480);
        let mut regular = std::vec![0; count];
        let mut separated = regular.clone();
        let mut regular_work = std::vec![0; count];
        let mut separated_work = std::vec![0x7531; count * 2];
        let mut regular_depth = std::vec![0; count];
        let mut separated_depth = regular_depth.clone();

        for bank in [0.035, 0.25, -0.58, 0.0] {
            let camera = Camera { x: 5_412.34, z: -3_511.42, altitude: 29.14, heading: 2.914, pitch: 0.15, bank };
            let sample = |x, z| world.sample(x, z);
            for region in [ScreenRect::full(DIMENSIONS), ScreenRect { left: 101, top: 100, right: 396, bottom: 427 }] {
                render_terrain_layer_region((sample, unchanged_material), camera, &sky, &mut regular,
                    &mut regular_work, Some((&mut regular_depth, region)), DIMENSIONS, |_| {}).unwrap();
                render_terrain_layer_region((sample, unchanged_material), camera, &sky, &mut separated,
                    &mut separated_work, Some((&mut separated_depth, region)), DIMENSIONS, |_| {}).unwrap();
                assert_eq!(regular, separated);
                assert_eq!(regular_depth, separated_depth);
            }
        }
    }

    #[test]
    fn short_terrain_spans_match_the_row_clipped_reference() {
        for (width, height) in [(466, 466), (449, 466), (466, 449), (113, 52)] {
            let aperture = Aperture::new(width, height);
            let camera = Camera { x: 0.0, z: 0.0, altitude: 30.0, heading: 0.0, pitch: 0.0, bank: 0.0 };
            let view = View::new(camera, width, height);
            for clipped in [false, true] {
                let mut actual = std::vec![u16::MAX; width * height];
                let mut expected = actual.clone();
                for n in 0..2048 {
                    let x0 = (3 + n * 313) % width;
                    let x1 = (x0 + n % 4).min(width);
                    let a = (23 + n * 131) % height;
                    let b = (7 + n * 23) % height;
                    let (y0, y1) = (a.min(b), a.max(b) + 1);
                    let color = (n * 947) as u16;
                    if clipped {
                        paint_span::<true>(&mut actual, width, height, &aperture, &view, x0, x1, y0, y1, color);
                    } else {
                        paint_span::<false>(&mut actual, width, height, &aperture, &view, x0, x1, y0, y1, color);
                    }
                    for y in y0..y1 {
                        let mut columns = x0..x1;
                        if clipped {
                            columns.start = columns.start.max(aperture.left[y] as usize);
                            columns.end = columns.end.min(aperture.right[y] as usize);
                        }
                        for x in columns {
                            expected[y * width + x] = color;
                        }
                    }
                    if n % 256 == 0 {
                        assert!(actual == expected, "terrain span {n} at {width}×{height} clipped={clipped}");
                    }
                }
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn unclamped_rotation_matches_scalar_sampling_for_round_and_clipped_views() {
        for (width, height) in [(466, 466), (449, 466), (146, 113)] {
            let aperture = Aperture::new(width, height);
            let min_x = ((width - aperture.diameter) / 2) as i32;
            let min_y = ((height - aperture.diameter) / 2) as i32;
            let max_x = min_x + aperture.diameter as i32 - 1;
            let max_y = min_y + aperture.diameter as i32 - 1;
            let source = (0..width * height).map(|i| (i as u32).wrapping_mul(0x5137_07ed) as u16)
                .collect::<std::vec::Vec<_>>();
            let mut actual = std::vec![0; width * height];
            let mut expected = actual.clone();
            for bank in [0.0, 0.041, -0.067, 0.112, -0.18, 0.25, -0.80, 1.57, 3.12] {
                let camera = Camera { x: 0.0, z: 0.0, altitude: 30.0, heading: 0.0, pitch: 0.0, bank };
                let view = View::new(camera, width, height);
                let cosine = (view.roll_cos * 65_536.0) as i32;
                let sine = (view.roll_sin * 65_536.0) as i32;
                for rect in [ScreenRect::full((width,height)), ScreenRect {
                    left: width / 5, top: height / 6, right: width * 4 / 5, bottom: height * 5 / 6,
                }] {
                    actual.fill(u16::MAX);
                    expected.fill(u16::MAX);
                    rotate_terrain::<false, false>(&source, &mut actual, width, height, &aperture, &view, rect, 0, None);
                    for row in rect.top..rect.bottom {
                        let first = rect.left.max(aperture.left[row] as usize);
                        let last = rect.right.min(aperture.right[row] as usize);
                        let dy = 2 * row as i32 + 1 - height as i32;
                        for column in first..last {
                            let dx = 2 * column as i32 + 1 - width as i32;
                            let x = (((dx * cosine + dy * sine) >> 1) + ((width as i32) << 15)) >> 16;
                            let y = (((-dx * sine + dy * cosine) >> 1) + ((height as i32) << 15)) >> 16;
                            expected[row * width + column] = source[y.clamp(min_y, max_y) as usize * width
                                + x.clamp(min_x, max_x) as usize];
                        }
                    }
                    assert_eq!(actual, expected, "bank {bank}, viewport {width}×{height}, {rect:?}");
                }
            }
        }
    }

    #[test]
    fn moon_disc_follows_illumination_and_respects_terrain_occlusion() {
        let side = 112;
        let camera = Camera { x: 0.0, z: 0.0, altitude: 35.0, heading: 0.0, pitch: 0.0, bank: 0.0 };
        let view = View::new(camera, side, side);
        let aperture = Aperture::new(side, side);
        let atmosphere = Atmosphere {
            sun: [-1.0, 0.0, 0.0],
            moon: [0.0, 0.22, 0.9755],
            daylight: 0.0,
            clouds: 0.0,
            ..Atmosphere::DAY
        };
        let mut pixels = std::vec![0; side * side];
        let hidden = 37 * side + 54;
        pixels[hidden] = 0x6328;
        let patch = MoonPatch::new(&pixels, side, side, &aperture, &view, atmosphere).unwrap();
        let clear = 0x08c5;
        for pixel in &mut pixels {
            if *pixel == 0 {
                *pixel = clear;
            }
        }
        patch.paint(&mut pixels, side, &view, atmosphere);
        assert_eq!(pixels[hidden], 0x6328);
        assert!(pixels[36 * side + 54] > pixels[36 * side + 57]);
        assert_eq!(pixels[36 * side + 60], clear);
    }

    #[test]
    fn moonlight_depends_on_elevation_phase_and_weather() {
        let clear = Atmosphere {
            sun: [0.0, -0.75, -0.6614],
            moon: [0.0, 0.75, 0.6614],
            daylight: 0.0,
            clouds: 0.0,
            ..Atmosphere::DAY
        };
        assert!(clear.moonlight() > 0.95);
        assert!(Atmosphere { clouds: 0.8, ..clear }.moonlight() < clear.moonlight() * 0.65);
        assert_eq!(Atmosphere { daylight: 1.0, ..clear }.moonlight(), 0.0);
        assert_eq!(Atmosphere { moon: clear.sun, ..clear }.moonlight(), 0.0);
    }

    #[test]
    fn cloud_layer_blends_through_the_crest_and_into_clear_weather() {
        let mut sky = Atmosphere::DAY;
        sky.clouds = 0.78;
        let location = [139.0, 23.0, 300.0];
        let mut camera = Camera { x: 80.0, z: 32.0, altitude: 82.0, heading: 0.0, pitch: 0.25, bank: 0.0 };
        let mut cells = [CloudCell::START; 2];
        let mut prior = GroundClouds::new(sky, camera).over(location, 218.0, &mut cells);
        let mut max_opacity = 0.0_f32;
        for step in 1..=450 {
            camera.altitude = 82.0 + step as f32 * 0.30;
            let next = GroundClouds::new(sky, camera).over(location, 218.0, &mut cells);
            assert!((next.opacity - prior.opacity).abs() < 0.012,
                "altitude {} opacity {} -> {}", camera.altitude, prior.opacity, next.opacity);
            assert!((next.exposure - prior.exposure).abs() < 0.012,
                "altitude {} exposure {} -> {}", camera.altitude, prior.exposure, next.exposure);
            max_opacity = max_opacity.max(next.opacity);
            prior = next;
        }
        assert!(max_opacity > 0.02);
        sky.clouds = 0.48;
        assert_eq!(GroundClouds::new(sky, camera).over(location, 218.0, &mut cells).opacity, 0.0);
    }

    #[test]
    fn broken_cruise_clouds_have_clear_gaps_and_drift_across_the_land() {
        let camera = Camera {
            x: 0.0, z: 0.0, altitude: 320.0, heading: 0.0, pitch: 0.55, bank: 0.0,
        };
        let mut sky = Atmosphere { clouds: 0.55, ..Atmosphere::DAY };
        let mut cells = [CloudCell::START; 2];
        let clouds = GroundClouds::new(sky, camera);
        let mut samples = 0;
        let mut clear = 0;
        let mut visible = 0;
        let mut movement = 0.0;
        sky.cloud_offset = [15.0, 9.0];
        let shifted = GroundClouds::new(sky, camera);
        for x in (-400..=400).step_by(20) {
            for z in (110..=850).step_by(20) {
                let point = [x as f32, 20.0, z as f32];
                let base = clouds.over(point, z as f32, &mut cells).opacity;
                let moved = shifted.over(point, z as f32, &mut cells).opacity;
                clear += usize::from(base < 0.015);
                visible += usize::from(base > 0.18);
                movement += (base - moved).abs();
                samples += 1;
            }
        }
        assert!(clear > samples / 3 && visible > samples / 20);
        assert!(movement > samples as f32 * 0.025);
        sky.clouds = 0.25;
        assert_eq!(ground_cloud_presence(sky, camera), 0.0);
    }

    #[test]
    fn drifting_clouds_leave_star_coordinates_fixed() {
        let camera = Camera {
            x: 0.0,
            altitude: 24.0,
            z: 0.0,
            heading: 0.6,
            pitch: 0.02,
            bank: 0.27,
        };
        let view = View::new(camera, 112, 112);
        let aperture = Aperture::new(112, 112);
        let mut early = [0; 112 * 112];
        let mut late = [0; 112 * 112];
        let night = Atmosphere {
            daylight: 0.0,
            clouds: 0.68,
            ..Atmosphere::DAY
        };
        let cloudier = Atmosphere {
            cloud_offset: [19.3, -27.7],
            ..night
        };

        let before = visible_stars(&early, 112, 112, &aperture, &view, night);
        let after = visible_stars(&late, 112, 112, &aperture, &view, cloudier);
        assert!(before.count > 8);
        assert_eq!(before.count, after.count);
        assert_eq!(before.visible[..before.count], after.visible[..after.count]);
        let twilight = Atmosphere {
            daylight: 0.45,
            ..night
        };
        let dusk = visible_stars(&early, 112, 112, &aperture, &view, twilight);
        assert_eq!(before.count, dusk.count);
        for (full, faded) in before.visible[..before.count]
            .iter()
            .zip(&dusk.visible[..dusk.count])
        {
            assert_eq!(full.pixel, faded.pixel);
            assert!(faded.intensity < full.intensity);
        }
        assert_eq!(
            visible_stars(&early, 112, 112, &aperture, &view, Atmosphere::DAY).count,
            0
        );
        paint_sky(
            &mut early,
            112,
            112,
            &aperture,
            &view,
            night,
            night.horizon(),
            |_| None,
        );
        paint_sky(
            &mut late,
            112,
            112,
            &aperture,
            &view,
            cloudier,
            cloudier.horizon(),
            |_| None,
        );
        assert_ne!(early, late);
        paint_stars(&mut early, &before, &view, night);
        paint_stars(&mut late, &after, &view, cloudier);
        assert!(early.iter().any(|&pixel| pixel > 0x528a));
    }
}
