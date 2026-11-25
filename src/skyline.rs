//! Distant, depth-tested silhouettes from published town geometry.

use crate::land_stream::LandStream;
use crate::math::{sin_cos, smoothstep};
use crate::render::{
    clear_cruise_visibility, encoded_depth, horizon_distance, view_distance, Camera, ScreenRect,
};
use crate::sky::Sky;

const VISIBLE: usize = 16;
const VISIBLE_ROOFS: usize = 6;
const VISIBLE_NEIGHBORHOODS: usize = 18;
const GLYPHS: usize = VISIBLE + VISIBLE_ROOFS + VISIBLE_NEIGHBORHOODS;
const FADE_TICKS: u8 = 36;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkylineTower {
    pub center: [f32; 2],
    pub footprint: [f32; 2],
    pub direction: [f32; 2],
    pub base: f32,
    pub height: f32,
    pub(crate) profile: u8,
}

/// Each pair gives the roof of a tier and its fraction of the lot width.
const TOWER_TIERS: [[(f32, f32); 3]; 5] = [
    [(0.48, 1.0), (0.79, 0.70), (1.0, 0.44)], // Stepped crown
    [(0.19, 1.0), (0.92, 0.77), (1.0, 0.67)], // Slab over a podium
    [(0.72, 1.0), (0.91, 0.82), (1.0, 0.38)], // Broad shaft and penthouse
    [(0.24, 1.0), (0.81, 0.56), (1.0, 0.24)], // Narrow crown
    [(0.41, 1.0), (0.71, 0.91), (1.0, 0.62)], // Terraced offices
];

pub(crate) fn tower_tiers(profile: u8) -> [(f32, f32); 3] {
    let mut tiers = TOWER_TIERS[usize::from(profile / 5)];
    let offset = f32::from(profile % 5) * 0.018 - 0.036;
    tiers[0].0 += offset;
    tiers[2].1 -= offset;
    tiers
}

#[derive(Clone, Copy)]
pub(crate) struct SkylineRoof {
    pub building: SkylineTower,
    pub pitched: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Form {
    Tower,
    Hall,
    Gable,
    Neighborhood,
}

#[derive(Clone, Copy)]
struct Retained {
    tower: SkylineTower,
    form: Form,
    fade: u8,
    seen: bool,
    fresh: bool,
}

/// Keep a short visual lifetime across land publication and eviction.
/// Does no terrain generation or settlement planning.
pub struct Skyline {
    retained: [Option<Retained>; VISIBLE],
    roofs: [Option<Retained>; VISIBLE_ROOFS],
    neighborhoods: [Option<Retained>; VISIBLE_NEIGHBORHOODS],
    last_tick: u64,
}

impl Skyline {
    pub const fn new() -> Self {
        Self {
            retained: [None; VISIBLE],
            roofs: [None; VISIBLE_ROOFS],
            neighborhoods: [None; VISIBLE_NEIGHBORHOODS],
            last_tick: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn prepare<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        ticks: u64,
        sky: &Sky,
        dimensions: (usize, usize),
        mut far_ground: impl FnMut(f32, f32) -> f32,
    ) -> SkylineFrame {
        if ticks < self.last_tick {
            self.reset();
        }
        let delta = ticks.saturating_sub(self.last_tick).min(6) as u8;
        self.last_tick = ticks;
        let view = Projection::new(camera, dimensions);
        for record in self
            .retained
            .iter_mut()
            .chain(&mut self.roofs)
            .chain(&mut self.neighborhoods)
            .flatten()
        {
            record.seen = false;
        }
        for (key, district) in land.districts() {
            if !land.district_published(key) || !view.district_visible(district.bounds()) {
                continue;
            }
            for town in district.towns() {
                for tower in town.skyline_towers() {
                    if view.potential(tower.center) {
                        retain(&mut self.retained, tower, Form::Tower);
                    }
                }
                for roof in town.skyline_roofs() {
                    if view.roof_potential(roof.building.center) {
                        let form = if roof.pitched {
                            Form::Gable
                        } else {
                            Form::Hall
                        };
                        retain(&mut self.roofs, roof.building, form);
                    }
                }
                for group in town.skyline_neighborhoods() {
                    if view.roof_potential(group.center) {
                        retain(&mut self.neighborhoods, group, Form::Neighborhood);
                    }
                }
            }
        }
        let mut frame = SkylineFrame {
            glyphs: [None; GLYPHS],
            dimensions,
            roll: view.roll,
            rect: None,
            len: 0,
        };
        for slot in self
            .retained
            .iter_mut()
            .chain(&mut self.roofs)
            .chain(&mut self.neighborhoods)
        {
            let Some(entry) = slot else {
                continue;
            };
            if entry.fresh {
                entry.fresh = false;
                continue;
            }
            entry.fade = if entry.seen {
                (entry.fade + delta).min(FADE_TICKS)
            } else {
                entry.fade.saturating_sub(delta)
            };
            if entry.fade == 0 {
                if !entry.seen {
                    *slot = None;
                }
                continue;
            }
            let fade = smoothstep(entry.fade as f32 * (1.0 / FADE_TICKS as f32));
            if let Some(glyph) = view.glyph(entry.tower, entry.form, fade, sky, &mut far_ground) {
                frame.insert(glyph);
            }
        }
        frame.rect = frame.glyphs[..frame.len]
            .iter()
            .flatten()
            .map(Glyph::screen_rect)
            .reduce(ScreenRect::union);
        frame
    }
}

fn retain(slots: &mut [Option<Retained>], tower: SkylineTower, form: Form) {
    if let Some(existing) = slots
        .iter_mut()
        .flatten()
        .find(|entry| entry.tower == tower)
    {
        existing.seen = true;
    } else if let Some(slot) = slots.iter_mut().find(|slot| slot.is_none()) {
        *slot = Some(Retained {
            tower,
            form,
            fade: 0,
            seen: true,
            fresh: true,
        });
    }
}

/// Alpha of the coarse silhouette, also used to retire the original distant
/// mesh before its fixed view boundary. Foreground mesh stays untouched.
pub(crate) fn handoff(forward: f32, near: f32) -> f32 {
    smoothstep((forward - near * 0.78) / (near * 0.20))
}

pub struct SkylineFrame {
    glyphs: [Option<Glyph>; GLYPHS],
    dimensions: (usize, usize),
    roll: [f32; 2],
    rect: Option<ScreenRect>,
    len: usize,
}

impl SkylineFrame {
    pub fn bounds(&self) -> Option<ScreenRect> {
        self.rect
    }
    pub fn len(&self) -> usize {
        self.glyphs[..self.len]
            .iter()
            .flatten()
            .filter(|g| g.form == Form::Tower)
            .count()
    }

    pub fn roof_count(&self) -> usize {
        self.len - self.len() - self.neighborhood_count()
    }

    pub fn neighborhood_count(&self) -> usize {
        self.glyphs[..self.len]
            .iter()
            .flatten()
            .filter(|glyph| glyph.form == Form::Neighborhood)
            .count()
    }

    fn insert(&mut self, glyph: Glyph) {
        // Keep the nearest few silhouettes, drawn back to front. Runtime is
        // bounded even when several retained town regions sit in the wedge.
        let index = self.glyphs[..self.len]
            .partition_point(|entry| entry.is_some_and(|other| other.z <= glyph.z));
        if index == GLYPHS {
            return;
        }
        let last = self.len.min(GLYPHS - 1);
        self.glyphs.copy_within(index..last, index + 1);
        self.glyphs[index] = Some(glyph);
        self.len = (self.len + 1).min(GLYPHS);
    }

    /// Cross-fade only a legacy tower with a corresponding ready silhouette.
    pub fn draw_legacy<const LAND: usize>(
        &self,
        land: &LandStream<LAND>,
        camera: Camera,
        mesh: &mut crate::world_mesh::WorldMesh<'_>,
    ) {
        for (key, district) in land.districts() {
            if !land.district_published(key) {
                continue;
            }
            for town in district.towns() {
                town.draw_distant_towers_faded(camera, mesh, |x, z| {
                    self.glyphs[..self.len]
                        .iter()
                        .flatten()
                        .find(|glyph| glyph.form == Form::Tower && glyph.center == [x, z])
                        .map_or(1.0, |glyph| 1.0 - glyph.crossfade)
                });
            }
        }
    }

    /// Draw after the terrain and before nearer world meshes. The depth slice
    /// must contain terrain coverage for `bounds()` (blank sky is u16::MAX).
    pub fn draw(&self, pixels: &mut [u16], depth: &[u16]) {
        let (width, height) = self.dimensions;
        let Some(count) = width.checked_mul(height) else {
            return;
        };
        if pixels.len() < count || depth.len() < count || width > 512 || height > 512 {
            return;
        }
        let radius_sq = (width.min(height) * width.min(height)) as i32;
        for glyph in self.glyphs[..self.len].iter().rev().flatten() {
            let rect = glyph.screen_rect();
            let opacity = if glyph.form == Form::Tower {
                glyph.alpha
            } else {
                glyph.alpha * footing(glyph, self.roll, width, depth)
            };
            if opacity < 0.015 {
                continue;
            }
            for y in rect.top..rect.bottom {
                let py = y as f32 - glyph.origin[1];
                let dy = (2 * y + 1) as i32 - height as i32;
                for x in rect.left..rect.right {
                    let dx = (2 * x + 1) as i32 - width as i32;
                    let index = y * width + x;
                    if dx * dx + dy * dy > radius_sq || depth[index] <= glyph.z {
                        continue;
                    }
                    let px = x as f32 - glyph.origin[0];
                    let u = px * self.roll[1] + py * self.roll[0];
                    let v = py * self.roll[1] - px * self.roll[0];
                    let value = glyph.coverage(u, v);
                    let alpha = (value * opacity * 64.0 + 0.5) as u32;
                    if alpha != 0 {
                        pixels[index] = blend(pixels[index], glyph.color, alpha.min(64));
                    }
                }
            }
        }
    }
}

fn footing(glyph: &Glyph, roll: [f32; 2], width: usize, depth: &[u16]) -> f32 {
    // The coarse horizon may miss a small isolated rise. Fade a roof if the
    // rendered terrain provides no support just below its projected footing.
    let rect = glyph.screen_rect();
    let x = glyph.origin[0] - roll[0] * 0.85;
    let y = glyph.origin[1] + roll[1] * 0.85;
    if x < 0.0 || y < 0.0 {
        return 0.0;
    }
    let ix = x as usize;
    let iy = y as usize;
    let fx = x - ix as f32;
    let fy = y - iy as f32;
    let mut support = 0.0;
    for (sy, wy) in [(iy, 1.0 - fy), (iy + 1, fy)] {
        for (sx, wx) in [(ix, 1.0 - fx), (ix + 1, fx)] {
            if (rect.left..rect.right).contains(&sx)
                && (rect.top..rect.bottom).contains(&sy)
                && depth[sy * width + sx] <= glyph.z.saturating_add(1024).min(u16::MAX - 1)
            {
                support += wx * wy;
            }
        }
    }
    smoothstep(support * 2.0)
}

#[derive(Clone, Copy)]
struct Glyph {
    form: Form,
    profile: u8,
    center: [f32; 2],
    crossfade: f32,
    origin: [f32; 2],
    half: f32,
    height: f32,
    alpha: f32,
    z: u16,
    color: u16,
    rect: [u16; 4],
}

impl Glyph {
    fn screen_rect(&self) -> ScreenRect {
        let [left, top, right, bottom] = self.rect.map(usize::from);
        ScreenRect {
            left,
            top,
            right,
            bottom,
        }
    }

    fn coverage(&self, u: f32, v: f32) -> f32 {
        match self.form {
            Form::Tower => {
                let mut value = 0.0;
                let mut from = 0.0;
                for (fraction, scale) in tower_tiers(self.profile) {
                    let half = self.half * scale;
                    let across = overlap(u, -half, half);
                    let down = overlap(v, -self.height * fraction, -self.height * from);
                    value += across * down;
                    from = fraction;
                }
                value
            }
            Form::Hall => overlap(u, -self.half, self.half) * overlap(v, -self.height, 0.0),
            Form::Neighborhood => {
                // Break the mass into offset rooflines. Gaps keep a row of
                // blocks legible at a distance without drawing each house.
                let roofs = match self.profile {
                    0 => [0.57, 0.91, 0.66],
                    1 => [0.83, 0.63, 0.71],
                    2 => [0.61, 0.71, 0.89],
                    _ => [0.72, 0.84, 0.55],
                };
                let step = self.half * (2.0 / 3.0);
                roofs
                    .into_iter()
                    .enumerate()
                    .map(|(index, height)| {
                        let start = -self.half + index as f32 * step;
                        overlap(u, start + step * 0.06, start + step * 0.92)
                            * overlap(v, -self.height * height, 0.0)
                    })
                    .sum()
            }
            Form::Gable => {
                let eave = self.height * 0.73;
                let wall = overlap(u, -self.half, self.half) * overlap(v, -eave, 0.0);
                let roof = overlap(v, -self.height, -eave);
                let roof_center = v.clamp(-self.height, -eave);
                let half = self.half * (roof_center + self.height) / (self.height - eave);
                wall + roof * overlap(u, -half, half)
            }
        }
    }
}

struct Projection {
    camera: Camera,
    heading: [f32; 2],
    pitch: [f32; 2],
    roll: [f32; 2],
    near: f32,
    far: f32,
    curve: f32,
    dimensions: (usize, usize),
    focal: f32,
}

impl Projection {
    fn new(camera: Camera, dimensions: (usize, usize)) -> Self {
        let near = view_distance(camera.altitude);
        Self {
            camera,
            heading: sin_cos(camera.heading).into(),
            pitch: sin_cos(camera.pitch.clamp(-1.3, 1.3)).into(),
            roll: sin_cos(-camera.bank).into(),
            near,
            far: near + (horizon_distance(camera.altitude) - near) * 0.83,
            curve: (near - 512.0) * (0.000040 / 384.0),
            dimensions,
            focal: dimensions.0.min(dimensions.1) as f32 * 0.73,
        }
    }

    fn relative(&self, p: [f32; 2]) -> [f32; 2] {
        let x = p[0] - self.camera.x;
        let z = p[1] - self.camera.z;
        [
            x * self.heading[1] - z * self.heading[0],
            x * self.heading[0] + z * self.heading[1],
        ]
    }

    fn district_visible(&self, b: [f32; 4]) -> bool {
        let [side, along] = self.relative([(b[0] + b[2]) * 0.5, (b[1] + b[3]) * 0.5]);
        let radius = (b[2] - b[0] + b[3] - b[1]) * 0.5;
        along + radius > self.near * 0.69
            && along - radius < self.far + 80.0
            && side.abs() - radius < (along + radius) * 0.82 + 80.0
    }

    fn potential(&self, center: [f32; 2]) -> bool {
        let [side, forward] = self.relative(center);
        forward > self.near * 0.69
            && forward < self.far + 80.0
            && side.abs() < forward * 0.82 + 80.0
    }

    fn roof_potential(&self, center: [f32; 2]) -> bool {
        let [side, forward] = self.relative(center);
        forward > self.near * 0.58
            && forward < self.far + 80.0
            && side.abs() < forward * 0.82 + 32.0
    }

    fn glyph(
        &self,
        tower: SkylineTower,
        form: Form,
        appearance: f32,
        sky: &Sky,
        ground: &mut impl FnMut(f32, f32) -> f32,
    ) -> Option<Glyph> {
        let (width, height) = self.dimensions;
        if width == 0 || height == 0 || width > 512 || height > 512 {
            return None;
        }
        let [side, along] = self.relative(tower.center);
        let near_fade = if form == Form::Tower {
            handoff(along, self.near)
        } else if form == Form::Neighborhood {
            smoothstep((along - self.near * 0.77) / (self.near * 0.19))
        } else {
            smoothstep((along - self.near * 0.65) / (self.near * 0.24))
        };
        let fade = appearance
            * near_fade
            * (1.0 - smoothstep((along - self.far * 0.80) / (self.far * 0.20)));
        if fade < 0.015 || side.abs() > along * 0.82 + 12.0 {
            return None;
        }
        let [x, z] = tower.center;
        let coarse = ground(x, z).max(0.0);
        let gap = (tower.base - coarse).abs();
        let shape = if form == Form::Tower {
            1.0 - smoothstep((gap - 10.0) / 18.0)
        } else {
            let coarse_weight = smoothstep((along - self.near * 0.91) / (self.near * 0.18));
            let dry_footing = smoothstep(coarse / 1.5);
            let alignment = if form == Form::Neighborhood {
                (1.0 - smoothstep((gap - 4.0) / 8.0)) * dry_footing
            } else {
                (1.0 - smoothstep((gap - 2.0) / 5.0)) * dry_footing
            };
            1.0 - coarse_weight * (1.0 - alignment)
        };
        let alpha = fade * shape * (0.75 - sky.haze * 0.32);
        if alpha < 0.015 {
            return None;
        }
        // Blend height anchoring over the LOD overlap. A town outside the
        // original terrain mesh is planted on the same coarse ground grid.
        let shift = smoothstep((along - self.near * 0.91) / (self.near * 0.19));
        let adjustment = coarse - tower.base;
        let base = tower.base
            + if form == Form::Tower {
                adjustment
            } else {
                adjustment.min(0.0)
            } * shift;
        let curve = self.curve * (side * side + along * along);
        let relief = base - self.camera.altitude - curve;
        let camera_z = along * self.pitch[1] - relief * self.pitch[0];
        let projection = self.focal / camera_z;
        let screen = [
            side * projection,
            (-along * self.pitch[0] - relief * self.pitch[1]) * projection,
        ];
        let origin = [
            (width - 1) as f32 * 0.5 + screen[0] * self.roll[1] - screen[1] * self.roll[0],
            (height - 1) as f32 * 0.5 + screen[0] * self.roll[0] + screen[1] * self.roll[1],
        ];
        let dir = tower.direction;
        let right = dir[1] * self.heading[1] + dir[0] * self.heading[0];
        let model_side = dir[0] * self.heading[1] - dir[1] * self.heading[0];
        let half =
            (tower.footprint[0] * right.abs() + tower.footprint[1] * model_side.abs()) * projection;
        let h =
            tower.height * projection * (self.pitch[1] - screen[1] * self.pitch[0] / self.focal);
        if !(camera_z > 2.0 && half > 0.0 && h > 0.0) {
            return None;
        }
        let top = [origin[0] + h * self.roll[0], origin[1] - h * self.roll[1]];
        let pad = half + 1.0;
        let rect = ScreenRect {
            left: ((origin[0].min(top[0]) - pad) as usize).min(width),
            top: ((origin[1].min(top[1]) - pad) as usize).min(height),
            right: ((origin[0].max(top[0]) + pad + 1.0) as usize).min(width),
            bottom: ((origin[1].max(top[1]) + pad + 1.0) as usize).min(height),
        };
        if rect.left >= rect.right || rect.top >= rect.bottom {
            return None;
        }
        let dusk = sky.daylight.clamp(0.0, 1.0);
        let wall = match form {
            Form::Tower => [32.0 + 92.0 * dusk, 44.0 + 104.0 * dusk, 60.0 + 111.0 * dusk],
            Form::Hall => [41.0 + 117.0 * dusk, 49.0 + 107.0 * dusk, 60.0 + 97.0 * dusk],
            Form::Gable => [46.0 + 115.0 * dusk, 42.0 + 86.0 * dusk, 49.0 + 66.0 * dusk],
            Form::Neighborhood => [48.0 + 103.0 * dusk, 48.0 + 97.0 * dusk, 54.0 + 91.0 * dusk],
        };
        let visibility =
            clear_cruise_visibility(self.camera.altitude, sky.sun_direction[1], sky.haze);
        let color = rgb(wall.map(|channel| channel * (1.0 - 0.18 * visibility)));
        // Use the farther side of the footprint for smaller shapes so their
        // blended pixels cannot slip in front of a close hill or roof.
        let depth = if form == Form::Tower {
            camera_z
        } else {
            let extent = tower.footprint[0] + tower.footprint[1];
            camera_z
                + if form == Form::Neighborhood {
                    extent * 0.5
                } else {
                    extent
                }
        };
        Some(Glyph {
            form,
            profile: tower.profile,
            center: tower.center,
            crossfade: appearance * handoff(along, self.near),
            origin,
            half,
            height: h,
            alpha,
            z: encoded_depth(depth).min(u16::MAX - 1),
            color,
            rect: [
                rect.left as u16,
                rect.top as u16,
                rect.right as u16,
                rect.bottom as u16,
            ],
        })
    }
}

fn overlap(center: f32, lo: f32, hi: f32) -> f32 {
    ((center + 0.5).min(hi) - (center - 0.5).max(lo)).max(0.0)
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

    #[test]
    fn all_25_tower_profiles_have_distinct_continuous_tiers() {
        let mut seen = std::collections::HashSet::new();
        for variant in 0..25 {
            let tiers = tower_tiers(variant);
            assert_eq!(tiers[0].1, 1.0);
            assert_eq!(tiers[2].0, 1.0);
            assert!(tiers.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 > w[1].1));
            assert!(seen.insert(tiers.map(|(height, width)| [height.to_bits(), width.to_bits()])));
        }
    }
    use crate::horizon_cache::HorizonCache;
    use crate::world::World;
    use crate::world_mesh::WorldMesh;

    const SIZE: (usize, usize) = (112, 112);
    const PIXELS: usize = SIZE.0 * SIZE.1;

    fn camera(x: f32, z: f32, altitude: f32, heading: f32, pitch: f32) -> Camera {
        Camera {
            x,
            z,
            altitude,
            heading,
            pitch,
            bank: 0.09,
        }
    }

    fn sky_at(camera: Camera) -> Sky {
        Sky::from_time_at_position(1, 0, 0.31, [camera.x, camera.altitude, camera.z])
    }

    fn strength(image: &[u16; PIXELS], clear: u16) -> u32 {
        image
            .iter()
            .map(|&value| {
                [11, 5, 0]
                    .into_iter()
                    .zip([31, 63, 31])
                    .map(|(shift, mask)| {
                        u32::from((value >> shift) & mask)
                            .abs_diff(u32::from((clear >> shift) & mask))
                    })
                    .sum::<u32>()
            })
            .sum()
    }

    #[test]
    fn published_towers_appear_gently_and_remain_behind_foreground_depth() {
        let world = World::new(1);
        let mut horizon = HorizonCache::new();
        for view in [
            camera(129.28485, 662.7953, 36.0, 4.18879, 0.15),
            camera(-989.20355, 1316.074, 335.6, 2.6179938, 0.53),
        ] {
            let sky = sky_at(view);
            let mut land = LandStream::<8>::new();
            land.reset(world);
            land.preflight(view, true);
            assert!(land.district_published((-1, 0)));
            let first_tower = land
                .planned((-1, 0))
                .flatten()
                .unwrap()
                .towns()
                .flat_map(|town| town.skyline_towers())
                .next()
                .unwrap();
            let [x, z] = first_tower.center;
            assert!(!crate::world_mesh::bounds_in_view(
                view,
                [x - 10.0, z - 10.0, x + 10.0, z + 10.0]
            ));
            horizon.reset(world, view.x, view.z);
            let mut skyline = Skyline::new();
            let empty = LandStream::<8>::new();
            let depth = [u16::MAX; PIXELS];
            let mountain = [encoded_depth(210.0); PIXELS];
            let clear = 0xadff;
            let mut level = 0;
            for tick in 100..137 {
                let frame =
                    skyline.prepare(&land, view, tick, &sky, SIZE, |x, z| horizon.sample(x, z));
                let mut image = [clear; PIXELS];
                frame.draw(&mut image, &depth);
                let visible = strength(&image, clear);
                assert!(visible >= level);
                if tick == 100 {
                    assert_eq!(visible, 0);
                }
                level = visible;
                let mut behind = [clear; PIXELS];
                frame.draw(&mut behind, &mountain);
                assert_eq!(behind, [clear; PIXELS]);
            }
            assert!(level > 0);
            for tick in 137..173 {
                let frame =
                    skyline.prepare(&empty, view, tick, &sky, SIZE, |x, z| horizon.sample(x, z));
                let mut image = [clear; PIXELS];
                frame.draw(&mut image, &depth);
                let visible = strength(&image, clear);
                assert!(visible <= level);
                level = visible;
            }
            assert_eq!(level, 0);
        }
    }

    #[test]
    fn prepared_low_roofs_fade_in_and_yield_to_terrain_and_near_geometry() {
        use crate::render::render_terrain_layer_region_with_horizon;
        use crate::terrain_cache::TerrainCache;

        let world = World::new(7_911);
        let view = camera(-1078.7152, 3325.965, 36.0, 5.4977875, 0.15);
        let sky = Sky::from_time_at_position(7_911, 0, 0.31, [view.x, view.altitude, view.z]);
        let mut land = LandStream::<8>::new();
        land.reset(world);
        land.preflight(view, true);
        let mut horizon = HorizonCache::new();
        horizon.reset(world, view.x, view.z);
        let mut terrain = TerrainCache::new();
        terrain.reset_for_altitude(world, view.x, view.z, view.altitude);
        let mut skyline = Skyline::new();
        let model = |x, z| horizon.sample(x, z);
        let opening = skyline.prepare(&land, view, 0, &sky, SIZE, model);
        assert!(opening.glyphs.iter().all(Option::is_none));
        for tick in 1..36 {
            skyline.prepare(&land, view, tick, &sky, SIZE, model);
        }
        let ready = skyline.prepare(&land, view, 36, &sky, SIZE, model);
        let mut roofs = SkylineFrame {
            glyphs: [None; GLYPHS],
            dimensions: SIZE,
            roll: ready.roll,
            rect: None,
            len: 0,
        };
        for &glyph in ready
            .glyphs
            .iter()
            .flatten()
            .filter(|g| matches!(g.form, Form::Hall | Form::Gable))
        {
            roofs.insert(glyph);
        }
        assert!((1..=VISIBLE_ROOFS).contains(&roofs.len));
        let mut clear = [0; PIXELS];
        let mut work = [0; PIXELS];
        let mut depth = [u16::MAX; PIXELS];
        render_terrain_layer_region_with_horizon(
            (
                |x, z| land.overlay(x, z, terrain.sample(x, z)),
                |_, _, p| p.surface,
            ),
            model,
            view,
            &sky,
            &mut clear,
            &mut work,
            Some((&mut depth, ScreenRect::full(SIZE))),
            SIZE,
            |_| {},
            |_| None,
        )
        .unwrap();
        let mut image = clear;
        roofs.draw(&mut image, &depth);
        assert!(image.iter().zip(clear).filter(|(a, b)| **a != *b).count() >= 4);
        image = clear;
        roofs.draw(&mut image, &[encoded_depth(210.0); PIXELS]);
        assert_eq!(image, clear);
        roofs.draw(&mut image, &[u16::MAX; PIXELS]);
        assert_eq!(image, clear);

        let roof = land
            .districts()
            .filter(|(key, _)| land.district_published(*key))
            .flat_map(|(_, district)| district.towns())
            .find_map(|town| town.skyline_roofs().next())
            .unwrap();
        let [x, z] = roof.building.center;
        let nearby = Projection::new(camera(x, z - 120.0, 36.0, 0.0, 0.15), SIZE);
        assert!(nearby
            .glyph(roof.building, Form::Hall, 1.0, &sky, &mut |_, _| roof
                .building
                .base)
            .is_none());
    }

    #[test]
    fn outer_neighborhoods_are_visible_with_terrain_depth_at_a_distance() {
        use crate::render::render_terrain_layer_region_with_horizon;
        use crate::terrain_cache::TerrainCache;

        const RASTER: (usize, usize) = (60, 60);
        let world = World::new(11);
        let view = camera(-512.0, 930.0, 68.0, core::f32::consts::PI, 0.1);
        let sky = Sky::from_time(11, 0, 0.39);
        let mut land = std::boxed::Box::new(LandStream::<8>::new());
        land.reset(world);
        land.preflight(view, true);
        let mut horizon = HorizonCache::new();
        horizon.reset(world, view.x, view.z);
        let mut terrain = TerrainCache::new();
        terrain.reset_for_altitude(world, view.x, view.z, view.altitude);
        let far = |x, z| horizon.sample(x, z);
        let mut skyline = Skyline::new();
        for tick in 0..36 {
            skyline.prepare(&land, view, tick, &sky, RASTER, far);
        }
        let mut frame = skyline.prepare(&land, view, 36, &sky, RASTER, far);
        assert!(frame.neighborhood_count() >= 5);
        frame.glyphs.iter_mut().for_each(|slot| {
            if slot.is_some_and(|glyph| glyph.form != Form::Neighborhood) {
                *slot = None;
            }
        });
        let mut pixels = [0; 3600];
        let mut scratch = [0; 3600];
        let mut depth = [u16::MAX; 3600];
        render_terrain_layer_region_with_horizon(
            (
                |x, z| land.overlay(x, z, terrain.sample(x, z)),
                |_, _, p| p.surface,
            ),
            far,
            view,
            &sky,
            &mut pixels,
            &mut scratch,
            Some((&mut depth, ScreenRect::full(RASTER))),
            RASTER,
            |_| {},
            |_| None,
        )
        .unwrap();
        let baseline = pixels;
        frame.draw(&mut pixels, &depth);
        assert!(
            pixels
                .iter()
                .zip(baseline)
                .filter(|(a, b)| **a != *b)
                .count()
                >= 10
        );
        pixels = baseline;
        frame.draw(&mut pixels, &[encoded_depth(120.0); 3600]);
        assert_eq!(pixels, baseline);
    }

    #[test]
    fn uncrossed_foreground_keeps_its_original_tower_mesh() {
        let world = World::new(1);
        let view = camera(-426.7035, 141.79535, 36.0, 0.0, 0.15);
        let sky = sky_at(view);
        let mut land = LandStream::<8>::new();
        land.reset(world);
        land.preflight(view, true);
        let mut skyline = Skyline::new();
        let frame = skyline.prepare(&land, view, 0, &sky, SIZE, |x, z| world.elevation(x, z));
        let mut a = [0xadff; PIXELS];
        let mut b = a;
        let mut ad = [u16::MAX; PIXELS];
        let mut bd = ad;
        let mut original = WorldMesh::new(&mut a, &mut ad, SIZE, view, &sky).unwrap();
        land.draw_distant_towers(view, &mut original);
        let mut new = WorldMesh::new(&mut b, &mut bd, SIZE, view, &sky).unwrap();
        frame.draw_legacy(&land, view, &mut new);
        assert!(a.iter().any(|&v| v != 0xadff));
        assert_eq!(a, b);
        assert_eq!(ad, bd);
    }
}
