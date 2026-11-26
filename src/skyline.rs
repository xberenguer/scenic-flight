//! Distant, depth-tested silhouettes from published town geometry.

use crate::land_stream::LandStream;
use crate::math::{sin_cos, smoothstep};
use crate::render::{
    CITY_HORIZON, Camera, ScreenRect, backdrop_curvature_squared, clear_cruise_visibility,
    encoded_depth, horizon_distance, view_distance,
};
use crate::sky::Sky;

const VISIBLE: usize = 16;
const VISIBLE_ROOFS: usize = 6;
const VISIBLE_NEIGHBORHOODS: usize = 18;
const GLYPHS: usize = VISIBLE + VISIBLE_ROOFS + VISIBLE_NEIGHBORHOODS;
const FADE_TICKS: u8 = 36;
const CIVIC_ROOF_REACH: f32 = 2240.0;
const CITY_FADE_WIDTH: f32 = 700.0;

pub(crate) const COTTAGE_GROUP: u16 = 4;
pub(crate) const DUPLEX_GROUP: u16 = 5;
pub(crate) const ROWHOUSE_GROUP: u16 = 6;
pub(crate) const INDUSTRIAL_GROUP: u16 = 7;
pub(crate) const SHOP_GROUP: u16 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkylineTower {
    pub center: [f32; 2],
    pub footprint: [f32; 2],
    pub direction: [f32; 2],
    pub base: f32,
    pub height: f32,
    pub(crate) profile: u16,
}

pub(crate) mod tower_forms;

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

#[derive(Clone, Copy)]
struct Candidate {
    tower: SkylineTower,
    form: Form,
    priority: f32,
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
        far_ground: impl FnMut(f32, f32) -> f32,
    ) -> SkylineFrame {
        self.prepare_with_additions(land, camera, ticks, sky, dimensions, core::iter::empty(), far_ground)
    }

    #[inline(never)]
    pub fn prepare_with_additions<const LAND: usize>(
        &mut self,
        land: &LandStream<LAND>,
        camera: Camera,
        ticks: u64,
        sky: &Sky,
        dimensions: (usize, usize),
        additions: impl Iterator<Item = SkylineTower>,
        mut far_ground: impl FnMut(f32, f32) -> f32,
    ) -> SkylineFrame {
        if ticks < self.last_tick {
            self.reset();
        }
        let delta = ticks.saturating_sub(self.last_tick).min(6) as u8;
        self.last_tick = ticks;
        let view = Projection::new(camera, dimensions);
        let mut towers = [None; VISIBLE];
        let mut roofs = [None; VISIBLE_ROOFS];
        let mut neighborhoods = [None; VISIBLE_NEIGHBORHOODS];
        for (key, district) in land.districts() {
            if !land.district_published(key) || !view.district_visible(district.bounds()) {
                continue;
            }
            for town in district.towns() {
                for tower in town.skyline_towers() {
                    if view.potential(tower.center) {
                        nominate(&mut towers, &self.retained, tower, Form::Tower, &view);
                    }
                }
                for roof in town.skyline_roofs() {
                    if view.roof_potential(roof.building.center) {
                        let form = if roof.pitched {
                            Form::Gable
                        } else {
                            Form::Hall
                        };
                        nominate(&mut roofs, &self.roofs, roof.building, form, &view);
                    }
                }
                for group in town.skyline_neighborhoods() {
                    if view.potential(group.center) {
                        nominate(&mut neighborhoods, &self.neighborhoods, group, Form::Neighborhood, &view);
                    }
                }
            }
        }
        for group in additions {
            if view.potential(group.center) {
                nominate(&mut neighborhoods, &self.neighborhoods, group, Form::Neighborhood, &view);
            }
        }
        choose(&mut self.retained, &towers);
        choose(&mut self.roofs, &roofs);
        choose(&mut self.neighborhoods, &neighborhoods);
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

fn nominate(
    candidates: &mut [Option<Candidate>],
    slots: &[Option<Retained>],
    tower: SkylineTower,
    form: Form,
    view: &Projection,
) {
    let [side, distance] = view.relative(tower.center);
    let availability = view.range_fade(form, distance);
    if availability < 0.02 {
        return;
    }
    let center_weight = 1.0 - 0.55 * (side.abs() / (distance * 0.82 + 80.0)).min(1.0);
    let height = tower.height + (tower.footprint[0] + tower.footprint[1]).min(48.0) * 0.22;
    let raw = height * center_weight * (0.35 + 0.65 * availability) / distance;
    // Existing silhouettes get a small advantage so gentle turns don't
    // repeatedly swap similar candidates near the capacity limit.
    let previous = slots.iter().flatten().any(|entry| entry.tower == tower && entry.seen);
    let priority = if previous { raw * 1.12 } else { raw };
    let index = candidates.partition_point(|candidate| candidate.is_some_and(|entry| entry.priority >= priority));
    if index == candidates.len() || candidates.iter().flatten().any(|entry| entry.tower == tower) {
        return;
    }
    candidates.copy_within(index..candidates.len() - 1, index + 1);
    candidates[index] = Some(Candidate { tower, form, priority });
}

fn choose(slots: &mut [Option<Retained>], candidates: &[Option<Candidate>]) {
    for entry in slots.iter_mut().flatten() {
        entry.seen = false;
    }
    for candidate in candidates.iter().flatten() {
        if let Some(previous) = slots.iter_mut().flatten().find(|entry| entry.tower == candidate.tower) {
            previous.seen = true;
        } else if let Some(slot) = slots.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(Retained {
                tower: candidate.tower, form: candidate.form, fade: 0, seen: true, fresh: true,
            });
        }
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
            let opacity = if glyph.form == Form::Tower && !glyph.backdrop {
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
                && depth[sy * width + sx] <= glyph.support_depth
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
    profile: u16,
    center: [f32; 2],
    crossfade: f32,
    origin: [f32; 2],
    half: f32,
    tier_halves: [f32; 2],
    tier_centers: [f32; 2],
    height: f32,
    alpha: f32,
    z: u16,
    support_depth: u16,
    backdrop: bool,
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
                if let Some(design) = tower_forms::landmark(self.profile) {
                    return design.masses.iter().fold(0.0_f32, |coverage, mass| {
                        let half = self.tier_halves[0] * mass.half[0] + self.tier_halves[1] * mass.half[1];
                        let center = self.tier_centers[0] * mass.center[0] + self.tier_centers[1] * mass.center[1];
                        coverage.max(overlap(u, center - half, center + half)
                            * overlap(v, -self.height * mass.top, 0.0))
                    });
                }
                let mut value = 0.0;
                let mut from = 0.0;
                let [shaft, crown] = self.tier_halves;
                let [shaft_center, crown_center] = self.tier_centers;
                let tiers = &tower_forms::design(self.profile).tiers;
                for ((tier, half), center) in tiers.iter().zip([self.half, shaft, crown])
                    .zip([0.0, shaft_center, crown_center])
                {
                    let across = overlap(u, center - half, center + half);
                    let down = overlap(v, -self.height * tier.top, -self.height * from);
                    value += across * down;
                    from = tier.top;
                }
                value
            }
            Form::Hall => overlap(u, -self.half, self.half) * overlap(v, -self.height, 0.0),
            Form::Neighborhood if (COTTAGE_GROUP..=SHOP_GROUP).contains(&self.profile) => {
                borough_roof_coverage(u, v, self.half, self.height, self.profile)
            }
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

// Left and right limits, eave and ridge heights, and position of the ridge within each mass.
// The same three small silhouettes fit the existing neighborhood glyph budget.
const fn roof_mass(left: f32, right: f32, eave: f32, ridge: f32, crest: f32) -> [f32; 6] {
    [left, right, eave, ridge, crest, 1.0 / (ridge - eave)]
}

const BOROUGH_ROOFS: [[[f32; 6]; 3]; 5] = [
    [roof_mass(-0.96, -0.41, 0.49, 0.68, 0.47), roof_mass(-0.31, 0.27, 0.61, 0.90, 0.50), roof_mass(0.39, 0.90, 0.52, 0.76, 0.54)],
    [roof_mass(-0.96, -0.40, 0.57, 0.81, 0.57), roof_mass(-0.39, 0.16, 0.65, 0.91, 0.44), roof_mass(0.30, 0.93, 0.59, 0.80, 0.49)],
    [roof_mass(-0.97, -0.34, 0.69, 0.78, 0.47), roof_mass(-0.32, 0.32, 0.78, 0.92, 0.56), roof_mass(0.33, 0.96, 0.64, 0.76, 0.52)],
    [roof_mass(-0.98, -0.35, 0.57, 0.86, 0.84), roof_mass(-0.34, 0.33, 0.57, 0.86, 0.84), roof_mass(0.34, 0.98, 0.53, 0.81, 0.85)],
    [roof_mass(-0.96, -0.42, 0.58, 0.68, 0.45), roof_mass(-0.40, 0.25, 0.70, 0.85, 0.53), roof_mass(0.34, 0.97, 0.72, 0.79, 0.41)],
];

#[inline(never)]
fn borough_roof_coverage(u: f32, v: f32, half: f32, height: f32, profile: u16) -> f32 {
    let shape = &BOROUGH_ROOFS[usize::from(profile - COTTAGE_GROUP)];
    let elevation = -v / height;
    let mut amount = 0.0;
    for &[left, right, eave, ridge, crest, inverse_rise] in shape {
        let left = left * half;
        let right = right * half;
        let slope = ((elevation - eave) * inverse_rise).clamp(0.0, 1.0);
        let eave = -eave * height;
        let ridge = -ridge * height;
        let eaves = overlap(u, left, right) * overlap(v, eave, 0.0);
        let crest = left + (right - left) * crest;
        let roof_left = left + (crest - left) * slope;
        let roof_right = right + (crest - right) * slope;
        amount += eaves + overlap(u, roof_left, roof_right) * overlap(v, ridge, eave);
    }
    amount
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
            far: horizon_distance(camera.altitude),
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
            && along - radius < CITY_HORIZON + 80.0
            && side.abs() - radius < (along + radius) * 0.82 + 80.0
    }

    fn potential(&self, center: [f32; 2]) -> bool {
        let [side, forward] = self.relative(center);
        forward > self.near * 0.69
            && forward < CITY_HORIZON + 80.0
            && side.abs() < forward * 0.82 + 80.0
    }

    fn roof_potential(&self, center: [f32; 2]) -> bool {
        let [side, forward] = self.relative(center);
        forward > self.near * 0.58
            && forward < CIVIC_ROOF_REACH + 80.0
            && side.abs() < forward * 0.82 + 32.0
    }

    #[cfg(test)]
    fn outer(&self, center: [f32; 2]) -> bool {
        self.relative(center)[1] > self.far
    }

    fn range_fade(&self, form: Form, along: f32) -> f32 {
        let near_fade = if form == Form::Tower {
            handoff(along, self.near)
        } else if form == Form::Neighborhood {
            smoothstep((along - self.near * 0.77) / (self.near * 0.19))
        } else {
            smoothstep((along - self.near * 0.65) / (self.near * 0.24))
        };
        let (reach, fade_width) = if matches!(form, Form::Tower | Form::Neighborhood) {
            (CITY_HORIZON, CITY_FADE_WIDTH)
        } else {
            (CIVIC_ROOF_REACH, CIVIC_ROOF_REACH * 0.20)
        };
        near_fade * (1.0 - smoothstep((along - (reach - fade_width)) / fade_width))
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
        let fade = appearance * self.range_fade(form, along);
        if fade < 0.015 || side.abs() > along * 0.82 + 12.0 {
            return None;
        }
        let [x, z] = tower.center;
        let coarse = ground(x, z).max(0.0);
        if along > self.far && coarse <= 0.5 {
            return None;
        }
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
        let curve = self.curve * backdrop_curvature_squared(along, side);
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
        let landmark = if form == Form::Tower { tower_forms::landmark(tower.profile) } else { None };
        // Preserve landmark gaps as they approach one pixel. The horizontal
        // assist fades in with the distant layer and is capped by the real footprint.
        let width_scale = if landmark.is_some() {
            let span = (tower.footprint[0] * right.abs() + tower.footprint[1] * model_side.abs()) * projection;
            1.0 + 0.55 * handoff(along, self.near) * (1.0 - smoothstep((span - 0.55) / 0.85))
        } else {
            1.0
        };
        let x_signed = tower.footprint[0] * right * projection * width_scale;
        let z_signed = tower.footprint[1] * model_side * projection * width_scale;
        let x_width = x_signed.abs();
        let z_width = z_signed.abs();
        let half = x_width + z_width;
        let (tier_halves, tier_centers) = if landmark.is_some() {
            ([x_width, z_width], [x_signed, z_signed])
        } else if form == Form::Tower {
            let [_, shaft, crown] = tower_forms::design(tower.profile).tiers;
            (
                [shaft, crown].map(|tier| x_width * tier.half[0] + z_width * tier.half[1]),
                [shaft, crown].map(|tier| projection * (
                    tower.footprint[0] * right * tier.center[0]
                    + tower.footprint[1] * model_side * tier.center[1]
                )),
            )
        } else {
            ([0.0; 2], [0.0; 2])
        };
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
            Form::Tower => landmark.map_or_else(|| tower_forms::design(tower.profile).wall, |form| form.wall).map(|value| {
                27.0 + f32::from(value) * (0.20 + 0.58 * dusk)
            }),
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
            tier_halves,
            tier_centers,
            height: h,
            alpha,
            z: encoded_depth(depth),
            support_depth: encoded_depth(depth + 32.0),
            backdrop: along > self.far,
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
    fn all_250_tower_profiles_have_distinct_geometry_and_grounded_tiers() {
        let mut seen = std::collections::HashSet::new();
        for id in 0..tower_forms::TOWER_DESIGN_COUNT {
            let design = tower_forms::design(id as u16);
            assert_eq!(design.tiers[0].half, [1.0; 2]);
            assert_eq!(design.tiers[2].top, 1.0);
            for pair in design.tiers.windows(2) {
                let [lower, upper] = [pair[0], pair[1]];
                assert!(lower.top < upper.top);
                for axis in 0..2 {
                    assert!(
                        (upper.center[axis] - lower.center[axis]).abs() + upper.half[axis]
                            <= lower.half[axis] + 0.0001
                    );
                }
            }
            let signature = design.tiers.map(|tier| [
                (tier.top * design.height * 4.0).round() as i32,
                (tier.half[0] * design.size[0] * 24.0).round() as i32,
                (tier.half[1] * design.size[1] * 24.0).round() as i32,
                (tier.center[0] * design.size[0] * 24.0).round() as i32,
                (tier.center[1] * design.size[1] * 24.0).round() as i32,
            ]);
            assert!(seen.insert(signature), "duplicated geometry for tower {id}");
        }
    }
    #[test]
    fn landmarks_remain_distinct_at_the_waveshare_distant_raster_size() {
        const SIDE: usize = 60;
        let mut signatures = std::collections::HashSet::new();
        let mut selected = std::collections::HashSet::new();
        for (index, design) in tower_forms::landmarks::LANDMARKS.iter().enumerate() {
            let profile = (tower_forms::TOWER_DESIGN_COUNT + index) as u16;
            selected.insert(tower_forms::select((index as u32) << 14, 1, 0));
            assert!(design.masses.iter().any(|mass| mass.top == 1.0), "{}", design.name);
            for mass in design.masses {
                assert!(mass.top > 0.0 && mass.top <= 1.0, "{}", design.name);
                for axis in 0..2 {
                    assert!(mass.half[axis] > 0.0 && mass.half[axis] + mass.center[axis].abs() <= 1.001,
                        "{}", design.name);
                }
            }
            let view = Camera { x: 0.0, z: 0.0, altitude: 38.0, heading: 0.0, pitch: 0.0, bank: 0.0 };
            let sky = sky_at(view);
            let projection = Projection::new(view, (SIDE, SIDE));
            let mut signature = std::vec::Vec::new();
            for distance in [620.0, 900.0] {
                for direction in [[0.0, 1.0], [0.60, 0.80], [1.0, 0.0]] {
                    let tower = SkylineTower {
                        center: [0.0, distance], footprint: [5.15 * design.size[0], 4.85 * design.size[1]],
                        direction, base: 10.0, height: design.height, profile,
                    };
                    let mut glyph = projection.glyph(tower, Form::Tower, 1.0, &sky, &mut |_, _| 10.0).unwrap();
                    glyph.color = u16::MAX; // Compare geometry independent of material.
                    let mut frame = SkylineFrame {
                        glyphs: [None; GLYPHS], dimensions: (SIDE, SIDE), roll: projection.roll,
                        rect: Some(glyph.screen_rect()), len: 0,
                    };
                    frame.insert(glyph);
                    let mut pixels = [0; SIDE * SIDE];
                    frame.draw(&mut pixels, &[u16::MAX; SIDE * SIDE]);
                    assert!(pixels.iter().any(|&pixel| pixel != 0), "{}", design.name);
                    signature.extend(pixels);
                }
            }
            assert!(signatures.insert(signature), "{} duplicates another distant silhouette", design.name);
        }
        assert_eq!(selected.len(), 25);
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

    #[test]
    fn a_tower_slab_is_broader_from_the_side_than_from_the_end() {
        let view = camera(0.0, 0.0, 36.0, 0.0, 0.15);
        let projection = Projection::new(view, SIZE);
        let sky = sky_at(view);
        let tower = SkylineTower {
            center: [0.0, 700.0], footprint: [5.0; 2], direction: [0.0, 1.0],
            base: 20.0, height: 35.0, profile: 200,
        };
        let broad = projection.glyph(tower, Form::Tower, 1.0, &sky, &mut |_, _| 20.0).unwrap();
        let narrow = projection.glyph(SkylineTower { direction: [1.0, 0.0], ..tower },
            Form::Tower, 1.0, &sky, &mut |_, _| 20.0).unwrap();
        assert_eq!(broad.half, narrow.half);
        assert!(broad.tier_halves[0] > narrow.tier_halves[0] * 1.7);
        assert!(broad.tier_halves[1] > narrow.tier_halves[1] * 1.7);
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
    fn visible_neighborhood_can_replace_distant_candidates_without_a_hard_cut() {
        let view = camera(0.0, 0.0, 48.0, 0.0, 0.15);
        let sky = sky_at(view);
        let land = LandStream::<1>::new();
        let building = |x, z| SkylineTower {
            center: [x, z], footprint: [12.0, 8.0], direction: [0.0, 1.0],
            base: 10.0, height: 12.0, profile: SHOP_GROUP,
        };
        let outer = core::array::from_fn::<_, VISIBLE_NEIGHBORHOODS, _>(|index| {
            building(index as f32 * 6.0, 1_500.0 + index as f32 * 18.0)
        });
        let visible = building(0.0, 900.0);
        // The foreground terrain already owns this roof.
        let foreground = building(0.0, 350.0);
        let mut skyline = Skyline::new();
        for tick in (0..=36).step_by(6) {
            skyline.prepare_with_additions(&land, view, tick, &sky, SIZE, outer.into_iter(), |_, _| 10.0);
        }
        let sample = |skyline: &mut Skyline, tick| {
            skyline.prepare_with_additions(&land, view, tick, &sky, SIZE,
                outer.into_iter().chain([foreground, visible]), |_, _| 10.0)
        };
        let first = sample(&mut skyline, 42);
        assert_eq!(first.neighborhood_count(), VISIBLE_NEIGHBORHOODS);
        assert!(!first.glyphs.iter().flatten().any(|entry| entry.center == visible.center));
        for tick in (48..=114).step_by(6) {
            sample(&mut skyline, tick);
        }
        let final_frame = sample(&mut skyline, 120);
        assert!(final_frame.glyphs.iter().flatten().any(|entry| entry.center == visible.center));
        assert!(!skyline.neighborhoods.iter().flatten().any(|entry| entry.tower == foreground));
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
    fn streamed_city_beyond_the_old_horizon_appears_at_low_and_cruise_altitudes() {
        use crate::land_stream::LandBudget;
        use crate::render::render_terrain_layer_region_with_horizon;
        use crate::terrain_cache::TerrainCache;

        const RASTER: (usize, usize) = (60, 60);
        const AREA: usize = 60 * 60;
        let world = World::new(1);
        let owner = (-1, 0);
        for (altitude, pitch) in [(36.0, 0.15), (335.6, 0.23)] {
            let view = camera(-1526.7035, -1563.4604, altitude, 0.5235988, pitch);
            let sky = sky_at(view);
            let projection = Projection::new(view, RASTER);
            let mut land = std::boxed::Box::new(LandStream::<8>::new());
            land.reset(world);
            assert!(land.preflight(view, false));
            assert!(!land.district_published(owner));
            for frame in 0..450 {
                land.service(view, LandBudget {
                    expansions: 8,
                    terrain_samples: 32,
                    settlement_jobs: u8::from(frame % 16 == 2),
                });
                if land.district_published(owner) {
                    break;
                }
            }
            assert!(land.district_published(owner));

            let mut horizon = HorizonCache::new();
            horizon.reset(world, view.x, view.z);
            let far = |x, z| horizon.backdrop(x, z);
            let mut skyline = Skyline::new();
            for tick in 0..36 {
                skyline.prepare(&land, view, tick, &sky, RASTER, far);
            }
            let mut frame = skyline.prepare(&land, view, 36, &sky, RASTER, far);
            for entry in &mut frame.glyphs {
                if entry.is_some_and(|glyph| !projection.outer(glyph.center)) {
                    *entry = None;
                }
            }
            assert!(frame.len() > 0);

            let mut terrain = TerrainCache::new();
            terrain.reset_for_altitude(world, view.x, view.z, view.altitude);
            let mut pixels = [0; AREA];
            let mut scratch = [0; AREA];
            let mut depth = [u16::MAX; AREA];
            render_terrain_layer_region_with_horizon(
                (|x, z| land.overlay(x, z, terrain.sample(x, z)), |_, _, p| p.surface),
                far, view, &sky, &mut pixels, &mut scratch,
                Some((&mut depth, ScreenRect::full(RASTER))), RASTER, |_| {}, |_| None,
            ).unwrap();
            let baseline = pixels;
            frame.draw(&mut pixels, &depth);
            assert!(pixels != baseline, "the extended city was obscured at altitude {altitude}");
            pixels = baseline;
            frame.draw(&mut pixels, &[encoded_depth(1_800.0); AREA]);
            assert_eq!(pixels, baseline);
        }
    }

    #[test]
    fn high_cruise_keeps_the_extended_city_attached_to_rendered_terrain() {
        assert_city_blends_with_terrain_at(
            (1, (-1, 0)),
            camera(-2625.7314, -1814.3259, 328.28, 0.79, 0.53),
            0.39,
            3_000.0..3_500.0,
            Some(4),
        );
    }

    #[test]
    fn real_skyline_paints_beyond_four_thousand_units() {
        assert_city_blends_with_terrain_at(
            (46, (-5, -3)),
            Camera { bank: 0.0, ..camera(-1430.4126, 305.29858, 328.28, 3.93, 0.53) },
            0.44,
            4_050.0..4_300.0,
            None,
        );
    }

    fn assert_city_blends_with_terrain_at(
        (seed, owner): (u64, (i32, i32)),
        view: Camera,
        phase: f32,
        tower_range: core::ops::Range<f32>,
        minimum_towers: Option<usize>,
    ) {
        use crate::land_stream::LandBudget;
        use crate::render::render_terrain_layer_region_with_horizon;
        use crate::terrain_cache::TerrainCache;

        const RASTER: (usize, usize) = (60, 60);
        const AREA: usize = 60 * 60;
        let world = World::new(seed);
        let sky = Sky::from_time_at_position(seed, 0, phase, [view.x, view.altitude, view.z]);
        let projection = Projection::new(view, RASTER);
        let mut land = std::boxed::Box::new(LandStream::<8>::new());
        land.reset(world);
        land.preflight(view, false);
        for frame in 0..1600 {
            land.service(view, LandBudget {
                expansions: 8,
                terrain_samples: 32,
                settlement_jobs: u8::from(frame % 16 == 2),
            });
            if land.district_published(owner) {
                break;
            }
        }
        assert!(land.district_published(owner));

        let mut horizon = HorizonCache::new();
        horizon.reset(world, view.x, view.z);
        let far = |x, z| horizon.backdrop(x, z);
        let mut skyline = Skyline::new();
        for tick in 0..36 {
            skyline.prepare(&land, view, tick, &sky, RASTER, far);
        }
        let mut frame = skyline.prepare(&land, view, 36, &sky, RASTER, far);
        for entry in &mut frame.glyphs {
            if entry.is_some_and(|glyph| {
                let [x, z] = glyph.center;
                let tile = (
                    crate::scenery::region_axis(x, crate::roads::ROAD_REGION_SIZE),
                    crate::scenery::region_axis(z, crate::roads::ROAD_REGION_SIZE),
                );
                !tower_range.contains(&projection.relative(glyph.center)[1])
                    || tile != owner
                    || (minimum_towers.is_some() && glyph.form != Form::Tower)
            }) {
                *entry = None;
            }
        }
        assert!(frame.len() >= minimum_towers.unwrap_or(1));

        let mut terrain = TerrainCache::new();
        terrain.reset_for_altitude(world, view.x, view.z, view.altitude);
        let mut pixels = [0; AREA];
        let mut scratch = [0; AREA * 2];
        let mut depth = [u16::MAX; AREA];
        render_terrain_layer_region_with_horizon(
            (|x, z| land.overlay(x, z, terrain.sample(x, z)), |_, _, p| p.surface),
            far, view, &sky, &mut pixels, &mut scratch,
            Some((&mut depth, ScreenRect::full(RASTER))), RASTER, |_| {}, |_| None,
        ).unwrap();
        let terrain_only = pixels;
        frame.draw(&mut pixels, &depth);
        assert!(pixels != terrain_only, "no skyline pixels cleared the foreground at {view:?}");
        pixels = terrain_only;
        frame.draw(&mut pixels, &[u16::MAX; AREA]);
        assert_eq!(pixels, terrain_only);
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
