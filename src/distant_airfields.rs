//! Filtered runway silhouettes over the coarse ground backdrop.

use crate::airfield::{AirfieldKind, AirfieldSite};
use crate::math::{sin_cos, smoothstep};
use crate::render::{
    encoded_depth, view_distance, Camera, ScreenProjection, ScreenRect, SCENERY_HORIZON,
};
use crate::sky::Sky;

pub const AIRFIELD_SILHOUETTE_REACH: f32 = 2380.0;
const MAX_STROKES: usize = 4;

#[derive(Clone, Copy)]
struct RunwayStroke {
    start: [f32; 3],
    delta: [f32; 2],
    inverse_square: f32,
    inv_end: f32,
    radius: f32,
    alpha: f32,
    range: f32,
    rect: ScreenRect,
}

pub struct DistantAirfields {
    strokes: [Option<RunwayStroke>; MAX_STROKES],
    dimensions: (usize, usize),
    region: Option<ScreenRect>,
}

impl DistantAirfields {
    /// Only published regional runways have a large, paved silhouette.
    pub fn prepare<'a>(
        fields: impl IntoIterator<Item = &'a AirfieldSite>,
        camera: Camera,
        sky: &Sky,
        dimensions: (usize, usize),
        mut backdrop: impl FnMut(f32, f32) -> f32,
    ) -> Self {
        let mut result = Self {
            strokes: [None; MAX_STROKES],
            dimensions,
            region: None,
        };
        let (width, height) = dimensions;
        if !(1..=512).contains(&width) || !(1..=512).contains(&height) {
            return result;
        }
        let mut fields = fields
            .into_iter()
            .filter(|site| site.kind == AirfieldKind::Regional)
            .peekable();
        if fields.peek().is_none() {
            return result;
        }
        let mut projection = None;
        let (sin, cos) = sin_cos(camera.heading);
        let near = view_distance(camera.altitude);
        for site in fields {
            let (x, z) = site.center();
            let [dx, dz] = [x - camera.x, z - camera.z];
            let forward = dx * sin + dz * cos;
            let cross = dx * cos - dz * sin;
            if forward < near * 0.92
                || forward > AIRFIELD_SILHOUETTE_REACH
                || cross.abs() > forward * 0.69 + 80.0
            {
                continue;
            }
            let ends = site.runway_endpoints().map(|(x, z)| (x, z, backdrop(x, z)));
            let center_y = backdrop(x, z);
            if center_y < 2.0
                || ends
                    .iter()
                    .any(|end| end.2 < 2.0 || (end.2 - center_y).abs() > 8.0)
            {
                continue;
            }
            let projection = projection
                .get_or_insert_with(|| ScreenProjection::new(camera, dimensions).unwrap());
            let [start, finish] =
                ends.map(|(x, z, ground)| projection.backdrop_point([x, ground + 0.16, z]));
            let (Some(start), Some(finish)) = (start, finish) else {
                continue;
            };
            let delta = [finish[0] - start[0], finish[1] - start[1]];
            let square = delta[0] * delta[0] + delta[1] * delta[1];
            if square < 0.81 {
                continue;
            }
            let [e0, e1] = site
                .runway_edges_at_center()
                .map(|(x, z)| projection.backdrop_point([x, center_y + 0.16, z]));
            let (Some(e0), Some(e1)) = (e0, e1) else {
                continue;
            };
            let edge_shift = [e1[0] - e0[0], e1[1] - e0[1]];
            let cross = edge_shift[0] * delta[1] - edge_shift[1] * delta[0];
            let estimate = f32::from_bits((square.to_bits() >> 1) + 0x1fc0_0000);
            let length = 0.5 * (estimate + square / estimate);
            let half_width = 0.5 * cross.abs() / length;
            // The filter footprint keeps a subpixel strip stable as it rotates.
            let radius = half_width + 0.72;
            let alpha = smoothstep((forward - near * 0.92) / (near * 0.34))
                * (1.0 - smoothstep((forward - 1640.0) / (AIRFIELD_SILHOUETTE_REACH - 1640.0)))
                * (0.46 + 0.36 * sky.daylight)
                * (1.0 - 0.45 * sky.haze);
            if alpha < 0.035 {
                continue;
            }
            let rect = ScreenRect {
                left: ((start[0].min(finish[0]) - radius) as usize).min(width),
                right: ((start[0].max(finish[0]) + radius + 1.0) as usize).min(width),
                top: ((start[1].min(finish[1]) - radius) as usize).min(height),
                bottom: ((start[1].max(finish[1]) + radius + 1.0) as usize).min(height),
            };
            if rect.left >= rect.right || rect.top >= rect.bottom {
                continue;
            }
            let slot = result.strokes.iter().position(Option::is_none).or_else(|| {
                let (index, farthest) = result
                    .strokes
                    .iter()
                    .enumerate()
                    .filter_map(|(index, stroke)| {
                        stroke.as_ref().map(|stroke| (index, stroke.range))
                    })
                    .max_by(|a, b| a.1.total_cmp(&b.1))?;
                (forward < farthest).then_some(index)
            });
            if let Some(slot) = slot {
                result.strokes[slot] = Some(RunwayStroke {
                    start,
                    delta,
                    inverse_square: 1.0 / square,
                    inv_end: 1.0 / finish[2],
                    radius,
                    alpha,
                    range: forward,
                    rect,
                });
            }
        }
        result.region = result
            .strokes
            .iter()
            .flatten()
            .map(|stroke| stroke.rect)
            .reduce(ScreenRect::union);
        result
    }

    pub fn depth_region(&self) -> Option<ScreenRect> {
        self.region
    }

    pub fn draw(&self, pixels: &mut [u16], depth: &[u16]) {
        if self.region.is_none() {
            return;
        }
        let (width, height) = self.dimensions;
        let area = width * height;
        if pixels.len() < area || depth.len() < area {
            return;
        }
        let aperture = width.min(height) as i32;
        for stroke in self.strokes.iter().flatten() {
            let [ax, ay, az] = stroke.start;
            let [dx, dy] = stroke.delta;
            let inv_a = 1.0 / az;
            let inv_delta = stroke.inv_end - inv_a;
            for y in stroke.rect.top..stroke.rect.bottom {
                let py = y as f32 - ay;
                let cy = (2 * y + 1) as i32 - height as i32;
                for x in stroke.rect.left..stroke.rect.right {
                    let cx = (2 * x + 1) as i32 - width as i32;
                    if cx * cx + cy * cy > aperture * aperture {
                        continue;
                    }
                    let px = x as f32 - ax;
                    let t = ((px * dx + py * dy) * stroke.inverse_square).clamp(0.0, 1.0);
                    let [ex, ey] = [px - t * dx, py - t * dy];
                    let coverage = (1.0 - (ex * ex + ey * ey) / (stroke.radius * stroke.radius))
                        .clamp(0.0, 1.0);
                    if coverage < 0.045 {
                        continue;
                    }
                    let index = y * width + x;
                    let z = 1.0 / (inv_a + inv_delta * t);
                    // Both sides of the depth band are required: a runway follows
                    // the represented ground and cannot be painted onto sky.
                    let tolerance = z * (1.0 / 32.0)
                        + if z >= SCENERY_HORIZON - 80.0 {
                            76.0
                        } else {
                            6.0
                        };
                    if !(encoded_depth(z - tolerance)..=encoded_depth(z + tolerance))
                        .contains(&depth[index])
                    {
                        continue;
                    }
                    let strength = (coverage * stroke.alpha * 16.0 + 0.5) as u16;
                    let [r, g, b] = [
                        pixels[index] >> 11,
                        (pixels[index] >> 5) & 63,
                        pixels[index] & 31,
                    ];
                    let shade = 32 - strength;
                    pixels[index] = ((r * shade + 16) >> 5) << 11
                        | ((g * shade + 16) >> 5) << 5
                        | ((b * (32 - strength / 2) + 16) >> 5);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facility_stream::FacilityStream;
    use crate::horizon_cache::HorizonCache;
    use crate::land_stream::{LandBudget, LandStream};
    use crate::render::render_terrain_layer_region_with_horizon;
    use crate::terrain_cache::TerrainCache;
    use crate::world::World;

    #[test]
    fn real_published_runway_survives_small_steps_at_two_thousand() {
        const SIDE: usize = 60;
        const AREA: usize = SIDE * SIDE;
        let world = World::new(1);
        let camera = Camera {
            x: 1410.7139,
            z: -3633.8645,
            altitude: 78.0,
            heading: 0.0,
            pitch: 0.055,
            bank: 0.0,
        };
        let key = (1, -2);
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(camera, false));
        let mut facilities = FacilityStream::<8>::new(world, 1);
        facilities.preflight(&land);
        assert!(!land.district_published(key));
        let mut pending = None;
        for frame in 0..1200 {
            if frame % 2 == 0 {
                land.service(
                    camera,
                    LandBudget {
                        expansions: 8,
                        terrain_samples: 32,
                        settlement_jobs: u8::from(frame % 16 == 2),
                    },
                );
            }
            if frame % 4 == 0 {
                if let Some(result) = pending.take() {
                    facilities.commit(result, &land, camera);
                }
                if let Some(request) = facilities.request(&land, camera, true) {
                    pending = Some(request.prepare(|| {}));
                }
            }
            if facilities
                .planned(key)
                .and_then(|owner| owner.airfield())
                .is_some()
            {
                break;
            }
        }
        let site = facilities.planned(key).unwrap().airfield().unwrap();
        assert!(facilities
            .airfields()
            .any(|field| core::ptr::eq(field, site)));
        let (x, z) = site.center();
        assert!((x - camera.x).abs() < 0.001 && ((z - camera.z) - 2000.0).abs() < 0.002);

        let mut horizon = HorizonCache::new();
        horizon.reset(world, camera.x, camera.z);
        let mut terrain = TerrainCache::new();
        terrain.reset_for_altitude(world, camera.x, camera.z, camera.altitude);
        let mut work = [0; AREA];
        let mut counts = std::vec::Vec::new();
        for step in -4..=4 {
            let view = Camera {
                z: camera.z + step as f32 * 5.0,
                ..camera
            };
            let sky = Sky::from_time_at_position(1, 0, 0.43, [view.x, view.altitude, view.z]);
            let far = DistantAirfields::prepare([site], view, &sky, (SIDE, SIDE), |x, z| {
                horizon.backdrop(x, z)
            });
            let region = far.depth_region().unwrap();
            let mut pixels = [0; AREA];
            let mut depth = [u16::MAX; AREA];
            render_terrain_layer_region_with_horizon(
                (|x, z| terrain.sample(x, z), |_, _, sample| sample.surface),
                |x, z| horizon.backdrop(x, z),
                view,
                &sky,
                &mut pixels,
                &mut work,
                Some((&mut depth, region)),
                (SIDE, SIDE),
                |_| {},
                |_| None,
            )
            .unwrap();
            let original = pixels;
            far.draw(&mut pixels, &depth);
            counts.push(
                pixels
                    .iter()
                    .zip(original)
                    .filter(|(new, old)| **new != *old)
                    .count(),
            );
            let mut hidden = original;
            far.draw(&mut hidden, &[encoded_depth(850.0); AREA]);
            assert_eq!(hidden, original);
            far.draw(&mut hidden, &[u16::MAX; AREA]);
            assert_eq!(hidden, original);
        }
        assert!(
            counts.iter().all(|&count| (2..=8).contains(&count)),
            "runway continuity: {counts:?}"
        );
    }
}
