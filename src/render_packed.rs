//! Render two upright column bands in the existing terrain buffers.

use super::*;

const fn split_column(side: usize) -> usize {
    // The terrain marcher pairs even columns. Keep the seam between pairs.
    (side / 2) & !1
}

#[inline]
pub fn render_with_packed_terrain_fork(
    executor: &mut impl TerrainFork,
    sample: impl FnMut(f32, f32) -> Sample,
    material: impl FnMut(f32, f32, Sample) -> Surface,
    worker_sample: impl FnMut(f32, f32) -> Sample + Send,
    worker_material: impl FnMut(f32, f32, Sample) -> Surface + Send,
    camera: Camera,
    sky: &Sky,
    pixels: &mut [u16],
    work: &mut [u16],
    depth: Option<(&mut [u16], ScreenRect)>,
    checkpoint: impl FnMut(RenderCheckpoint),
    ready_sky_tile: impl FnMut(usize) -> Option<u16>,
    ground_horizon: impl FnMut(f32, f32) -> f32,
) -> Result<(), RenderError> {
    render_with_packed_terrain_fork_sized::<466>(
        executor, sample, material, worker_sample, worker_material, camera, sky, pixels, work,
        depth, checkpoint, ready_sky_tile, ground_horizon,
    )
}

/// Render a compact or native square through the packed terrain fork.
///
/// Depth uses two full planes in `work`. The upright compact views copy
/// the joined rows directly. Upright native frames keep the serial path.
#[inline(never)]
pub fn render_with_packed_terrain_fork_sized<const SIDE: usize>(
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
    if !matches!(SIDE, 36 | 42 | 48 | 54 | 60 | 466) {
        return Err(RenderError::InvalidDimensions);
    }
    let count = SIDE * SIDE;
    let split = split_column(SIDE);
    let right_columns = SIDE - split;
    let view = View::new(camera, SIDE, SIDE);
    let unbanked = view.roll_sin.abs() < 0.000_001 && view.roll_cos > 0.0;
    if (unbanked && SIDE == 466)
        || (depth.is_some() && work.len() < 2 * count)
    {
        return render_terrain_layer_region_with_horizon(
            (sample, material), ground_horizon, camera, sky, pixels, work, depth,
            (SIDE, SIDE), checkpoint, ready_sky_tile,
        );
    }
    if pixels.len() < count || work.len() < count
        || depth.as_ref().is_some_and(|(buffer, _)| buffer.len() < count)
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
    let pixels = &mut pixels[..count];
    pixels.fill(0);
    depth = depth.map(|(depth, rect)| {
        let depth = &mut depth[..count];
        depth.fill(u16::MAX);
        (depth, rect.clamped(SIDE, SIDE))
    });
    let (color_work, depth_work) = work.split_at_mut(count);
    color_work.fill(0);
    let (left_color, right_color) = color_work.split_at_mut(split * SIDE);
    let mut depth_work = depth.as_ref().map(|(_, rect)| {
        let work = &mut depth_work[..count];
        work.fill(u16::MAX);
        let (left, right) = work.split_at_mut(split * SIDE);
        ([left, right], if unbanked { *rect } else { rect.upright(&view, SIDE, SIDE) })
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
            TerrainBands { first: 0, end: split, write_from: 0, row_width: split, origin_x: 0 },
        ),
        || draw_near_bands::<false>(
            worker_sample, worker_material, camera, atmosphere, horizon, right_color, right,
            SIDE, SIDE, &aperture, &upright, &mut right_edge,
            TerrainBands { first: split - 2, end: SIDE, write_from: split, row_width: right_columns, origin_x: split },
        ),
    );
    executor.terrain_checkpoint(TerrainForkPhase::NearJoined);
    debug_assert_eq!(&edge[split - 2..split], &right_edge[split - 2..split]);
    edge[split..SIDE].copy_from_slice(&right_edge[split..SIDE]);
    executor.terrain_checkpoint(TerrainForkPhase::SlabMerged);
    if edge[..SIDE].iter().any(|&row| row > 0) {
        trace_far_horizon(
            &mut ground_horizon, camera, atmosphere, horizon, &upright, &aperture,
            &mut edge[..SIDE], 1, SIDE, SIDE, |x0, x1, y0, y1, color, z| {
                let (origin, width, index, target) = if x0 < split {
                    (0, split, 0, &mut *left_color)
                } else {
                    (split, right_columns, 1, &mut *right_color)
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
    if unbanked {
        copy_upright::<SIDE>(color_source, pixels, &aperture, full, first_ground);
        if let (Some(([left, right], _)), Some((depth, rect))) = (depth_work, depth) {
            copy_upright::<SIDE>([&*left, &*right], depth, &aperture, rect, first_ground);
        }
    } else if let (Some(([left, right], _)), Some((depth, rect))) = (depth_work, depth) {
        let depth_source = [&*left, &*right];
        if rect.left == 0 && rect.top == 0 && rect.right == SIDE && rect.bottom == SIDE {
            rotate_packed::<SIDE, true>(color_source, pixels, Some((depth_source, depth)), &aperture, &view, full, first_ground);
        } else {
            rotate_packed::<SIDE, false>(color_source, pixels, None, &aperture, &view, full, first_ground);
            rotate_packed::<SIDE, false>(depth_source, depth, None, &aperture, &view, rect, first_ground);
        }
    } else {
        rotate_packed::<SIDE, false>(color_source, pixels, None, &aperture, &view, full, first_ground);
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

fn copy_upright<const SIDE: usize>(
    source: Slabs<'_>,
    output: &mut [u16],
    aperture: &Aperture,
    rect: ScreenRect,
    first_ground: usize,
) {
    let split = split_column(SIDE);
    for row in rect.top.max(first_ground)..rect.bottom {
        let left = usize::from(aperture.left[row]).max(rect.left);
        let right = usize::from(aperture.right[row]).min(rect.right);
        if left < right.min(split) {
            let range = left..right.min(split);
            output[row * SIDE + range.start..row * SIDE + range.end]
                .copy_from_slice(&source[0][row * split + range.start..row * split + range.end]);
        }
        if left.max(split) < right {
            let offset = row * (SIDE - split);
            output[row * SIDE + left.max(split)..row * SIDE + right]
                .copy_from_slice(&source[1][offset + left.max(split) - split..offset + right - split]);
        }
    }
}

fn rotate_packed<const SIDE: usize, const PAIRED: bool>(
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
            rotate_packed_runs::<SIDE, PAIRED>(source, out, other_row, source_x, source_y, cosine, sine, inv_x, inv_y);
        } else if inside {
            gather_packed::<SIDE, PAIRED, false>(source, out, other_row, source_x, source_y, cosine, sine);
        } else {
            gather_packed::<SIDE, PAIRED, true>(source, out, other_row, source_x, source_y, cosine, sine);
        }
    }
}

fn gather_packed<const SIDE: usize, const PAIRED: bool, const CLAMPED: bool>(
    source: Slabs<'_>,
    output: &mut [u16],
    mut other: Option<(Slabs<'_>, &mut [u16])>,
    mut x: i32,
    mut y: i32,
    cosine: i32,
    sine: i32,
) {
    let split = split_column(SIDE);
    let right_columns = SIDE - split;
    for (i, out) in output.iter_mut().enumerate() {
        let sx = if CLAMPED { (x >> 16).clamp(0, SIDE as i32 - 1) } else { x >> 16 } as usize;
        let sy = if CLAMPED { (y >> 16).clamp(0, SIDE as i32 - 1) } else { y >> 16 } as usize;
        let (slab, at) = if sx < split {
            (0, sy * split + sx)
        } else {
            (1, sy * right_columns + sx - split)
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

fn rotate_packed_runs<const SIDE: usize, const PAIRED: bool>(
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
    let split = split_column(SIDE);
    let right_columns = SIDE - split;
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
        let (slab, at, boundary) = if sx < split {
            (0, sy * split + sx, split - sx)
        } else {
            (1, sy * right_columns + sx - split, SIDE - sx)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ground_details::GroundDetails,
        horizon_cache::HorizonCache,
        terrain_cache::TerrainCache,
    };

    #[derive(Default)]
    struct TestFork {
        count: usize,
    }

    impl TerrainFork for TestFork {
        fn fork(&mut self, left: impl FnOnce(), right: impl FnOnce() + Send) {
            self.count += 1;
            std::thread::scope(|scope| {
                let right = scope.spawn(right);
                left();
                right.join().unwrap();
            });
        }
    }

    fn check_frame<const SIDE: usize>(
        seed: u64,
        camera: Camera,
        sky: &Sky,
        regions: &[Option<ScreenRect>],
        spare_plane: bool,
    ) {
        let world = World::new(seed);
        let mut terrain = TerrainCache::new();
        terrain.reset_for_altitude(world, camera.x, camera.z, camera.altitude);
        let mut horizon = HorizonCache::new();
        horizon.reset(world, camera.x, camera.z);
        let mut details = GroundDetails::new();
        details.reset(&world, seed, camera.x, camera.z);
        let material = |x, z, sample| details.visible_material(x, z, sample);
        let trees = || details.tree_sampler().for_view(camera.x, camera.altitude, camera.z);

        let count = SIDE * SIDE;
        let mut serial = std::vec![0; count];
        let mut packed = std::vec![0x82a5; count];
        let mut serial_depth = std::vec![0; count];
        let mut packed_depth = std::vec![0; count];
        let mut serial_work = std::vec![0; count];
        let mut packed_work = std::vec![0xad82; count * if spare_plane { 2 } else { 1 }];

        for &region in regions {
            let serial_trees = trees();
            render_terrain_layer_region_with_horizon(
                (
                    |x, z| serial_trees.geometry(x, z, terrain.sample(x, z)),
                    material,
                ),
                |x, z| horizon.backdrop(x, z),
                camera,
                sky,
                &mut serial,
                &mut serial_work,
                region.map(|region| (&mut serial_depth[..], region)),
                (SIDE, SIDE),
                |_| {},
                |_| None,
            )
            .unwrap();

            let mut fork = TestFork::default();
            let left_trees = trees();
            let right_trees = trees();
            let right_terrain = &terrain;
            render_with_packed_terrain_fork_sized::<SIDE>(
                &mut fork,
                |x, z| left_trees.geometry(x, z, terrain.sample(x, z)),
                material,
                move |x, z| right_trees.geometry(x, z, right_terrain.sample(x, z)),
                material,
                camera,
                sky,
                &mut packed,
                &mut packed_work,
                region.map(|region| (&mut packed_depth[..], region)),
                |_| {},
                |_| None,
                |x, z| horizon.backdrop(x, z),
            )
            .unwrap();

            let expect_fork = (SIDE != 466 || camera.bank.abs() > 0.000_001)
                && (spare_plane || region.is_none());
            assert_eq!(
                fork.count,
                usize::from(expect_fork),
                "{SIDE}px, bank {}, region {region:?}",
                camera.bank
            );
            assert!(
                serial == packed,
                "{SIDE}px color differs for seed {seed}, camera {camera:?}, region {region:?}"
            );
            assert!(
                serial_depth == packed_depth,
                "{SIDE}px depth differs for seed {seed}, camera {camera:?}, region {region:?}"
            );
        }
    }

    #[test]
    fn packed_sixty_matches_serial_color_and_depth() {
        check_adaptive_raster::<60>();
    }

    #[test]
    fn packed_fifty_four_matches_serial_color_and_depth() {
        check_adaptive_raster::<54>();
    }

    #[test]
    fn packed_forty_eight_matches_serial_color_and_depth() {
        check_adaptive_raster::<48>();
    }

    #[test]
    fn packed_forty_two_matches_serial_color_and_depth() {
        check_adaptive_raster::<42>();
    }

    #[test]
    fn packed_thirty_six_matches_serial_color_and_depth() {
        check_adaptive_raster::<36>();
    }

    fn check_adaptive_raster<const SIDE: usize>() {
        let regions = [
            None,
            Some(ScreenRect::full((SIDE, SIDE))),
            Some(ScreenRect {
                left: 7, right: SIDE - 7, top: 11, bottom: SIDE - 11,
            }),
            Some(ScreenRect {
                left: SIDE / 2 + 1, right: SIDE - 2, top: 4, bottom: 33,
            }),
        ];

        for seed in [1, 41, 0x74cc_5871_0cfc_2ac4] {
            let world = World::new(seed);
            for (n, bank) in [0.0, 0.03, -0.065, 0.14, 0.19, -0.30, 0.56, -0.46]
                .into_iter()
                .enumerate()
            {
                let x = n as f32 * 88.7 - 155.0;
                let z = 86.0 - n as f32 * 42.3;
                let camera = Camera {
                    x,
                    z,
                    altitude: world.elevation(x, z)
                        + [14.0, 22.0, 61.0, 25.0, 390.0, 33.0, 55.0, 20.0][n],
                    heading: 0.52 + n as f32 * 0.71,
                    pitch: [-0.11, 0.2, 0.4, 0.05, 0.12, 0.13, -0.08, 0.16][n],
                    bank,
                };
                let sky = Sky::from_time_at_position(
                    seed,
                    n as u64 * 261,
                    [0.42, 0.18, 0.63, 0.48][n % 4],
                    [x, camera.altitude, z],
                );
                check_frame::<SIDE>(seed, camera, &sky, &regions, true);

                if n == 6 {
                    // Depth without a spare plane retains the serial path.
                    check_frame::<SIDE>(seed, camera, &sky, &regions[..3], false);
                }
            }
        }
    }

    #[test]
    fn packed_native_matches_serial_in_full_and_clipped_views() {
        const SIDE: usize = 466;
        let regions = [
            Some(ScreenRect::full((SIDE, SIDE))),
            Some(ScreenRect {
                left: 47, right: 421, top: 67, bottom: 368,
            }),
        ];
        for bank in [0.048, -0.71] {
            let camera = Camera {
                x: 154.3, z: 372.4, altitude: 38.8, heading: 3.01, pitch: 0.27, bank,
            };
            let sky = Sky::from_time_at_position(41, 3_480, 0.23, [camera.x, camera.altitude, camera.z]);
            check_frame::<SIDE>(41, camera, &sky, &regions, true);
        }
    }
}
