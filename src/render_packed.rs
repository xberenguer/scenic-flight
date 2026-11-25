//! Render two upright column bands in the existing terrain buffers.

use super::*;

const SIDE: usize = 466;
const SPLIT: usize = 232;
const RIGHT: usize = SIDE - SPLIT;
const COUNT: usize = SIDE * SIDE;

#[inline(never)]
pub fn render_with_packed_terrain_fork(
    executor: &mut impl TerrainFork,
    mut sample: impl FnMut(f32, f32) -> Sample,
    mut material: impl FnMut(f32, f32, Sample) -> Surface,
    worker_sample: impl FnMut(f32, f32) -> Sample + Send,
    worker_material: impl FnMut(f32, f32, Sample) -> Surface + Send,
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    work: &mut [u16],
    mut depth: Option<(&mut [u16], ScreenRect)>,
    mut checkpoint: impl FnMut(RenderCheckpoint),
    ready_sky_tile: impl FnMut(usize) -> Option<u16>,
    mut ground_horizon: impl FnMut(f32, f32) -> f32,
) -> Result<(), RenderError> {
    let view = View::new(camera, SIDE, SIDE);
    if (view.roll_sin.abs() < 0.000_001 && view.roll_cos > 0.0)
        || (depth.is_some() && work.len() < 2 * COUNT)
    {
        return render_terrain_layer_region_with_horizon(
            (sample, material), ground_horizon, camera, sky, pixels, work, depth,
            (SIDE, SIDE), checkpoint, ready_sky_tile,
        );
    }
    if pixels.len() < COUNT || work.len() < COUNT
        || depth.as_ref().is_some_and(|(buffer, _)| buffer.len() < COUNT)
    {
        return Err(RenderError::BufferTooSmall);
    }
    if !camera_is_finite(camera) {
        return Err(RenderError::InvalidCamera);
    }
    let atmosphere = sky_atmosphere(sky)?;
    let aperture = Aperture::new(SIDE, SIDE);
    let horizon = atmosphere.horizon();
    let upright = View { roll_sin: 0.0, roll_cos: 1.0, ..view };
    let pixels = &mut pixels[..COUNT];
    pixels.fill(0);
    depth = depth.map(|(depth, rect)| {
        let depth = &mut depth[..COUNT];
        depth.fill(u16::MAX);
        (depth, rect.clamped(SIDE, SIDE))
    });
    let (color_work, depth_work) = work.split_at_mut(COUNT);
    color_work.fill(0);
    let (left_color, right_color) = color_work.split_at_mut(SPLIT * SIDE);
    let mut depth_work = depth.as_ref().map(|(_, rect)| {
        let work = &mut depth_work[..COUNT];
        work.fill(u16::MAX);
        let (left, right) = work.split_at_mut(SPLIT * SIDE);
        ([left, right], rect.upright(&view, SIDE, SIDE))
    });
    executor.terrain_checkpoint(TerrainForkPhase::UprightPrepared);
    let (left, right) = depth_work.as_mut().map_or((None, None), |([left, right], rect)| {
        (Some((&mut **left, *rect)), Some((&mut **right, *rect)))
    });
    let mut edge = [SIDE as i16; MAX_BANDS];
    let mut right_edge = edge;
    executor.terrain_checkpoint(TerrainForkPhase::SlabCleared);
    executor.fork(
        || draw_near_bands::<false>(
            &mut sample, &mut material, camera, atmosphere, horizon, left_color, left,
            SIDE, SIDE, &aperture, &upright, &mut edge,
            TerrainBands { first: 0, end: SPLIT, write_from: 0, row_width: SPLIT, origin_x: 0 },
        ),
        || draw_near_bands::<false>(
            worker_sample, worker_material, camera, atmosphere, horizon, right_color, right,
            SIDE, SIDE, &aperture, &upright, &mut right_edge,
            TerrainBands { first: SPLIT - 2, end: SIDE, write_from: SPLIT, row_width: RIGHT, origin_x: SPLIT },
        ),
    );
    executor.terrain_checkpoint(TerrainForkPhase::NearJoined);
    debug_assert_eq!(&edge[SPLIT - 2..SPLIT], &right_edge[SPLIT - 2..SPLIT]);
    edge[SPLIT..SIDE].copy_from_slice(&right_edge[SPLIT..SIDE]);
    executor.terrain_checkpoint(TerrainForkPhase::SlabMerged);
    if edge[..SIDE].iter().any(|&row| row > 0) {
        trace_far_horizon(
            &mut ground_horizon, camera, atmosphere, horizon, &upright, &aperture,
            &mut edge[..SIDE], 1, SIDE, SIDE, |x0, x1, y0, y1, color, z| {
                let (origin, width, index, target) = if x0 < SPLIT {
                    (0, SPLIT, 0, &mut *left_color)
                } else {
                    (SPLIT, RIGHT, 1, &mut *right_color)
                };
                paint_span::<false>(target, width, SIDE, &aperture, &upright,
                    x0 - origin, x1 - origin, y0, y1, color);
                if let Some((planes, rect)) = depth_work.as_mut() {
                    let from = y0.max(rect.top);
                    let to = y1.min(rect.bottom);
                    if to > from && x1 > rect.left && x0 < rect.right {
                        paint_span::<false>(
                            planes[index], width, SIDE, &aperture, &upright,
                            x0.max(rect.left) - origin, x1.min(rect.right) - origin, from, to,
                            encoded_depth(z).min(u16::MAX - 1),
                        );
                    }
                }
            },
        );
    }
    executor.terrain_checkpoint(TerrainForkPhase::HorizonDone);
    checkpoint(RenderCheckpoint::Terrain);
    let first_ground = edge[..SIDE].iter().copied().min().unwrap_or(SIDE as i16) as usize;
    let full = ScreenRect::full((SIDE, SIDE));
    let color_source = [&*left_color, &*right_color];
    if let (Some(([left, right], _)), Some((depth, rect))) = (depth_work, depth) {
        let depth_source = [&*left, &*right];
        if rect.left == 0 && rect.top == 0 && rect.right == SIDE && rect.bottom == SIDE {
            rotate_packed::<true>(color_source, pixels, Some((depth_source, depth)), &aperture, &view, full, first_ground);
        } else {
            rotate_packed::<false>(color_source, pixels, None, &aperture, &view, full, first_ground);
            rotate_packed::<false>(depth_source, depth, None, &aperture, &view, rect, first_ground);
        }
    } else {
        rotate_packed::<false>(color_source, pixels, None, &aperture, &view, full, first_ground);
    }
    checkpoint(RenderCheckpoint::Rotation);
    if atmosphere.clouds > 0.48 {
        paint_ground_clouds(&mut sample, camera, atmosphere, &view, &aperture, pixels, SIDE, SIDE);
    }
    let stars = visible_stars(pixels, SIDE, SIDE, &aperture, &view, atmosphere);
    let moon = MoonPatch::new(pixels, SIDE, SIDE, &aperture, &view, atmosphere);
    const PIXEL_MASK: u32 = (1 << 18) - 1;
    let mut meteor_pixels = [0; 160];
    let mut trail = 0;
    if let Some(event) = atmosphere.meteor {
        streak(event, camera, SIDE, SIDE, |x, y, opacity| {
            let index = y * SIDE + x;
            if pixels[index] == 0 && trail < meteor_pixels.len() {
                let alpha = (opacity.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
                meteor_pixels[trail] = index as u32 | (alpha << 18);
                trail += 1;
            }
        });
    }
    paint_sky(pixels, SIDE, SIDE, &aperture, &view, atmosphere, horizon, ready_sky_tile);
    paint_stars(pixels, &stars, &view, atmosphere);
    if let Some(moon) = moon {
        moon.paint(pixels, SIDE, &view, atmosphere);
    }
    for packed in &meteor_pixels[..trail] {
        let index = (packed & PIXEL_MASK) as usize;
        let opacity = (packed >> 18) as f32 * (1.0 / 255.0);
        pixels[index] = blend_meteor(pixels[index], opacity);
    }
    checkpoint(RenderCheckpoint::Atmosphere);
    Ok(())
}

type Slabs<'a> = [&'a [u16]; 2];

fn rotate_packed<const PAIRED: bool>(
    source: Slabs<'_>,
    pixels: &mut [u16],
    mut other: Option<(Slabs<'_>, &mut [u16])>,
    aperture: &Aperture,
    view: &View,
    rect: ScreenRect,
    first_ground: usize,
) {
    if first_ground >= SIDE { return; }
    let cosine = (view.roll_cos * 65_536.0) as i32;
    let sine = (view.roll_sin * 65_536.0) as i32;
    let copy_runs = rotation_copies_runs(view, aperture);
    let inv_x = 65_536 / (65_536 - cosine).max(1) as u32;
    let inv_y = 65_536 / sine.unsigned_abs().max(1);
    for row in rect.top..rect.bottom {
        let first = usize::from(aperture.left[row]).max(rect.left);
        let last = usize::from(aperture.right[row]).min(rect.right);
        if last <= first { continue; }
        let dx = 2 * first as i32 + 1 - SIDE as i32;
        let dy = 2 * row as i32 + 1 - SIDE as i32;
        let source_x = ((dx * cosine + dy * sine) >> 1) + ((SIDE as i32) << 15);
        let source_y = ((-dx * sine + dy * cosine) >> 1) + ((SIDE as i32) << 15);
        let steps = (last - first - 1) as i32;
        let last_y = source_y - steps * sine;
        if first_ground != 0 && source_y.max(last_y) < ((first_ground as i32) << 16) { continue; }
        let last_x = source_x + steps * cosine;
        let inside = |a: i32, b: i32| (a.min(b) >> 16) >= 0 && (a.max(b) >> 16) < SIDE as i32;
        let inside = inside(source_x, last_x) && inside(source_y, last_y);
        let span = row * SIDE + first..row * SIDE + last;
        let out = &mut pixels[span.clone()];
        let other_row = if PAIRED {
            let (plane, target) = other.as_mut().unwrap();
            Some((*plane, &mut target[span]))
        } else {
            None
        };
        if inside && copy_runs {
            rotate_packed_runs::<PAIRED>(source, out, other_row, source_x, source_y, cosine, sine, inv_x, inv_y);
        } else if inside {
            gather_packed::<PAIRED, false>(source, out, other_row, source_x, source_y, cosine, sine);
        } else {
            gather_packed::<PAIRED, true>(source, out, other_row, source_x, source_y, cosine, sine);
        }
    }
}

fn gather_packed<const PAIRED: bool, const CLAMPED: bool>(
    source: Slabs<'_>,
    output: &mut [u16],
    mut other: Option<(Slabs<'_>, &mut [u16])>,
    mut x: i32,
    mut y: i32,
    cosine: i32,
    sine: i32,
) {
    for (i, out) in output.iter_mut().enumerate() {
        let sx = if CLAMPED { (x >> 16).clamp(0, SIDE as i32 - 1) } else { x >> 16 } as usize;
        let sy = if CLAMPED { (y >> 16).clamp(0, SIDE as i32 - 1) } else { y >> 16 } as usize;
        let (slab, at) = if sx < SPLIT {
            (0, sy * SPLIT + sx)
        } else {
            (1, sy * RIGHT + sx - SPLIT)
        };
        *out = source[slab][at];
        if PAIRED {
            let (plane, depth) = other.as_mut().unwrap();
            depth[i] = plane[slab][at];
        }
        x += cosine;
        y -= sine;
    }
}

fn rotate_packed_runs<const PAIRED: bool>(
    source: Slabs<'_>,
    output: &mut [u16],
    mut other: Option<(Slabs<'_>, &mut [u16])>,
    mut x: i32,
    mut y: i32,
    cosine: i32,
    sine: i32,
    inv_x: u32,
    inv_y: u32,
) {
    let mut offset = 0;
    while offset < output.len() {
        let remaining = output.len() - offset;
        let x_fraction = (x & 65_535) as u32;
        let y_fraction = (y & 65_535) as u32;
        let room_y = if sine < 0 { 65_535 - y_fraction } else { y_fraction };
        let run_x = if cosine == 65_536 { remaining } else { 1 + ((x_fraction * inv_x) >> 16) as usize };
        let run_y = if sine == 0 { remaining } else { 1 + ((room_y * inv_y) >> 16) as usize };
        let sx = (x >> 16) as usize;
        let sy = (y >> 16) as usize;
        let (slab, at, boundary) = if sx < SPLIT {
            (0, sy * SPLIT + sx, SPLIT - sx)
        } else {
            (1, sy * RIGHT + sx - SPLIT, SIDE - sx)
        };
        let count = run_x.min(run_y).min(remaining).min(boundary);
        output[offset..offset + count].copy_from_slice(&source[slab][at..at + count]);
        if PAIRED {
            let (plane, depth) = other.as_mut().unwrap();
            depth[offset..offset + count].copy_from_slice(&plane[slab][at..at + count]);
        }
        offset += count;
        x += cosine * count as i32;
        y -= sine * count as i32;
    }
}
