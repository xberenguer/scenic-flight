//! Terrain-bound details with a stable position in every seeded world.

use crate::math::{fraction, mix, smoothstep};
use crate::world::{SEA_LEVEL, Sample, Surface, World};

mod field_profiles;
pub use field_profiles::FieldProfile;

const SIDE: i32 = 19;
const TILE: f32 = 64.0;
const TREE_TILE: f32 = 5.5;
const CROWN_BLEND_START: f32 = 54.0 * 54.0;
const CROWN_BLEND_END: f32 = 122.0 * 122.0;
const FARM_DISTRICT_TILES: i32 = 5;

#[derive(Clone, Copy)]
#[repr(u8)]
enum FieldLayout {
    Blocks,
    PlantedPattern,
    ContourAcrossX,
    ContourAcrossZ,
}

#[derive(Clone, Copy)]
struct GroundTile {
    x: i32,
    z: i32,
    fields: bool,
    pattern: u32,
    alignment: i8,
    layout: FieldLayout,
    bend: i8,
}

impl GroundTile {
    const EMPTY: Self = Self {
        x: i32::MIN, z: i32::MIN, fields: false, pattern: 0, alignment: 0,
        layout: FieldLayout::Blocks, bend: 0,
    };

    fn prepare(world: &World, seed: u64, x: i32, z: i32) -> Self {
        let state = key(seed, x, z);
        let [district_x, district_z] = [x, z].map(|tile| tile.div_euclid(FARM_DISTRICT_TILES));
        let district = key(seed ^ 0x17da_e34d_8f0b_2671, district_x, district_z);
        let district_alignment = ((district >> 42) & 31) as i8 - 16;
        let survey = if farm_density(seed, x, z) > 0.32 && state & 15 != 0 {
            survey_farm(world, x, z)
        } else {
            None
        };
        let (layout, alignment, bend) = survey
            .map_or((FieldLayout::Blocks, district_alignment, 0), |profile| {
                fit_field_rows(profile, district_alignment)
            });
        let (layout, pattern) = if survey.is_some()
            && matches!(layout, FieldLayout::Blocks) && (district >> 17) & 3 == 0
        {
            let index = ((district >> 21) % FieldProfile::ALL.len() as u64) as u32;
            let axis = ((district >> 11) & 1) as u32;
            let reverse = ((state >> 7) & 1) as u32;
            (FieldLayout::PlantedPattern, index | (axis << 5) | (reverse << 6))
        } else {
            (layout, (state >> 24) as u32)
        };
        Self {
            x,
            z,
            fields: survey.is_some(),
            pattern,
            alignment,
            layout,
            bend,
        }
    }

    fn surface(self, x: f32, z: f32) -> Surface {
        if !self.fields { return Surface::Grass; }
        let u = x - self.x as f32 * TILE;
        let v = z - self.z as f32 * TILE;
        if !(2.0..=TILE - 2.0).contains(&u) || !(2.0..=TILE - 2.0).contains(&v) {
            return Surface::Grass;
        }
        let shape = self.pattern;
        match self.layout {
            FieldLayout::Blocks => {}
            FieldLayout::PlantedPattern => return self.planted_surface(u, v),
            _ => return self.contour_surface(u, v),
        }
        let grade = self.alignment as f32 * 0.022;
        let a = u + (v - TILE * 0.5) * grade;
        let b = v - (u - TILE * 0.5) * grade;
        let margin = |shift| 3.0 + ((shape >> shift) & 7_u32) as f32 * 0.88;
        if a < margin(0) || a > TILE - margin(3) || b < margin(6) || b > TILE - margin(9) {
            return Surface::Grass;
        }
        // Leave natural ground between blocks and alongside hedgerows.
        let divide_a = 20.0 + ((shape >> 12) & 23) as f32;
        let divide_b = 18.0 + ((shape >> 17) & 23) as f32;
        let layout = (shape >> 22) & 7;
        let split_a = layout & 3 != 0;
        let split_b = layout & 4 != 0 || !split_a && shape & 0x100 != 0;
        if (split_a && (a - divide_a).abs() < 1.25) || (split_b && (b - divide_b).abs() < 1.55) {
            return Surface::Grass;
        }
        let parcel = u32::from(split_a && a >= divide_a) * 2 + u32::from(split_b && b >= divide_b);
        match shape.rotate_right(24 + parcel * 3) & 7 {
            0 => Surface::FieldSoil,
            1 | 5 => Surface::FieldGold,
            2 => Surface::Grass,
            _ => Surface::FieldGreen,
        }
    }

    fn planted_surface(self, u: f32, v: f32) -> Surface {
        let (a, b) = if self.pattern & 32 == 0 { (u, v) } else { (v, u) };
        let shift = (b - TILE * 0.5) * f32::from(self.alignment) * 0.017;
        let across = if self.pattern & 64 == 0 { a + shift } else { TILE - a - shift };
        FieldProfile::ALL[(self.pattern & 31) as usize].sample(across, b)
    }

    fn contour_surface(self, u: f32, v: f32) -> Surface {
        let (along, across) = if matches!(self.layout, FieldLayout::ContourAcrossX) {
            (v, u)
        } else {
            (u, v)
        };
        let offset = along - TILE * 0.5;
        let normalized = offset * (1.0 / 28.0);
        let contour = across + offset * (f32::from(self.alignment) * (1.0 / 64.0))
            + normalized * normalized * (f32::from(self.bend) * 0.125);
        let headland = 5.0 + (self.pattern & 3) as f32;
        if !(headland..TILE - headland).contains(&along) || !(6.0..58.0).contains(&contour) {
            return Surface::Grass;
        }

        // Retain permanent grass bands between the cropped strips.
        let lower = 20.0 + ((self.pattern >> 6) & 3) as f32;
        let upper = 40.0 + ((self.pattern >> 8) & 3) as f32;
        if (contour - lower).abs() < 1.85 || (contour - upper).abs() < 1.85 {
            return Surface::Grass;
        }
        let strip = u32::from(contour > lower) + u32::from(contour > upper);
        match (self.pattern >> (11 + strip * 3)) & 7 {
            0 | 1 => Surface::FieldSoil,
            2..=4 => Surface::FieldGold,
            _ => Surface::FieldGreen,
        }
    }
}

fn farm_density(seed: u64, x: i32, z: i32) -> f32 {
    let lattice = [x, z].map(|tile| tile.div_euclid(FARM_DISTRICT_TILES));
    let [u, v] = [x, z].map(|tile| {
        smoothstep((tile.rem_euclid(FARM_DISTRICT_TILES) as f32 + 0.5) / FARM_DISTRICT_TILES as f32)
    });
    let [i, j] = lattice;
    let weight = |cx, cz| fraction(key(seed ^ 0x8b74_2e65_38d0_19cf, cx, cz));
    let [a, b, c, d] = [weight(i, j), weight(i + 1, j), weight(i, j + 1), weight(i + 1, j + 1)];
    let near = a + (b - a) * u;
    let far = c + (d - c) * u;
    near + (far - near) * v
}

/// Small local cache for footprint surveys. It uses the same field rules as the flight view.
pub(crate) struct ParcelSurvey {
    tiles: [GroundTile; 4],
}

impl ParcelSurvey {
    pub(crate) const fn new() -> Self {
        Self { tiles: [GroundTile::EMPTY; 4] }
    }

    pub(crate) fn occupies_grass(&mut self, world: &World, seed: u64, x: f32, z: f32) -> bool {
        let tx = grid_index(x, 1.0 / TILE);
        let tz = grid_index(z, 1.0 / TILE);
        let slot = (tx as usize & 1) + 2 * (tz as usize & 1);
        let tile = &mut self.tiles[slot];
        if (tile.x, tile.z) != (tx, tz) {
            *tile = GroundTile::prepare(world, seed, tx, tz);
        }
        tile.surface(x, z) != Surface::Grass
    }
}

pub struct GroundDetails {
    seed: u64,
    center: Option<(i32, i32)>,
    tiles: [[GroundTile; SIDE as usize]; SIDE as usize],
}

impl GroundDetails {
    pub const fn new() -> Self {
        Self { seed: 0, center: None, tiles: [[GroundTile::EMPTY; SIDE as usize]; SIDE as usize] }
    }

    pub fn reset(&mut self, world: &World, seed: u64, x: f32, z: f32) {
        self.seed = seed;
        self.center = None;
        self.tiles.fill([GroundTile::EMPTY; SIDE as usize]);
        self.follow(world, x, z);
    }

    pub fn follow(&mut self, world: &World, x: f32, z: f32) {
        let tx = grid_index(x, 1.0 / TILE);
        let tz = grid_index(z, 1.0 / TILE);
        if self.center == Some((tx, tz)) {
            return;
        }
        self.center = Some((tx, tz));
        for iz in tz - SIDE / 2..=tz + SIDE / 2 {
            for ix in tx - SIDE / 2..=tx + SIDE / 2 {
                let slot = &mut self.tiles[wrap(iz)][wrap(ix)];
                if (slot.x, slot.z) == (ix, iz) {
                    continue;
                }
                *slot = GroundTile::prepare(world, self.seed, ix, iz);
            }
        }
    }

    pub fn sample(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let geometry = self.geometry(x, z, ground);
        Sample { surface: self.visible_material(x, z, geometry), ..geometry }
    }

    pub fn geometry(&self, x: f32, z: f32, ground: Sample) -> Sample {
        if ground.surface == Surface::Forest { self.tree(x, z, ground) } else { ground }
    }

    pub fn visible_material(&self, x: f32, z: f32, geometry: Sample) -> Surface {
        if geometry.surface == Surface::Grass { self.field(x, z) } else { geometry.surface }
    }

    /// Retain the cultivated headlands and rows around a grain facility together.
    pub fn grain_field_bounds(&self, [x, _, z]: [f32; 3]) -> impl Iterator<Item = [f32; 4]> + '_ {
        self.tiles.iter().flatten().filter_map(move |tile| {
            tile.fields.then(|| grain_tile_bounds(tile.x, tile.z, x, z)).flatten()
        })
    }

    /// Survey just the plots in range of a terminal outside the flight cache.
    pub fn survey_grain_fields(
        world: &World, seed: u64, [x, _, z]: [f32; 3],
    ) -> impl Iterator<Item = [f32; 4]> + '_ {
        let [tx, tz] = [x, z].map(|position| grid_index(position, 1.0 / TILE));
        (tz - 2..=tz + 2).flat_map(move |row| {
            (tx - 2..=tx + 2).filter_map(move |column| {
                let bounds = grain_tile_bounds(column, row, x, z)?;
                GroundTile::prepare(world, seed, column, row).fields.then_some(bounds)
            })
        })
    }

    /// Inspect the cultivated profile in the same cache used to shade the terrain.
    pub fn field_profile_at(&self, x: f32, z: f32) -> Option<FieldProfile> {
        let tx = grid_index(x, 1.0 / TILE);
        let tz = grid_index(z, 1.0 / TILE);
        let tile = self.tiles[wrap(tz)][wrap(tx)];
        (tile.x == tx && tile.z == tz && tile.fields && matches!(tile.layout, FieldLayout::PlantedPattern))
            .then(|| FieldProfile::ALL[(tile.pattern & 31) as usize])
    }

    fn field(&self, x: f32, z: f32) -> Surface {
        let tx = grid_index(x, 1.0 / TILE);
        let tz = grid_index(z, 1.0 / TILE);
        let tile = self.tiles[wrap(tz)][wrap(tx)];
        if (tile.x, tile.z) == (tx, tz) { tile.surface(x, z) } else { Surface::Grass }
    }

    fn tree(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let tx = grid_index(x, 1.0 / TREE_TILE);
        let tz = grid_index(z, 1.0 / TREE_TILE);
        TreeCrown::survey(self.seed ^ 0x27a4_2dec, tx, tz).overlay(x, z, ground)
    }

    pub fn tree_sampler(&self) -> TreeSampler {
        TreeSampler {
            seed: self.seed ^ 0x27a4_2dec, last: core::cell::Cell::new(None), view: None,
        }
    }
}

fn grain_tile_bounds(column: i32, row: i32, x: f32, z: f32) -> Option<[f32; 4]> {
    let [left, near] = [column as f32 * TILE, row as f32 * TILE];
    ((left + TILE * 0.5 - x).abs().max((near + TILE * 0.5 - z).abs()) <= 96.0)
        .then_some([left, near, left + TILE, near + TILE])
}

/// Retains the prepared crown while consecutive rays sample its world tile.
pub struct TreeSampler {
    seed: u64,
    last: core::cell::Cell<Option<TreeCrown>>,
    view: Option<[f32; 3]>,
}

impl TreeSampler {
    /// Use individual crowns near the eye and a continuous canopy farther out.
    pub fn for_view(mut self, x: f32, altitude: f32, z: f32) -> Self {
        self.view = Some([x, altitude, z]);
        self
    }

    #[inline]
    pub fn geometry(&self, x: f32, z: f32, ground: Sample) -> Sample {
        if ground.surface != Surface::Forest {
            return ground;
        }
        let distance = self.view.map_or(0.0, |[eye_x, eye_y, eye_z]| {
            let [dx, dy, dz] = [x - eye_x, ground.elevation - eye_y, z - eye_z];
            dx * dx + dy * dy + dz * dz
        });
        if distance >= CROWN_BLEND_END {
            return Sample { elevation: ground.elevation + stand_height(ground.elevation), ..ground };
        }
        let tx = grid_index(x, 1.0 / TREE_TILE);
        let tz = grid_index(z, 1.0 / TREE_TILE);
        let crown = match self.last.get() {
            Some(crown) if crown.tile == (tx, tz) => crown,
            _ => {
                let crown = TreeCrown::survey(self.seed, tx, tz);
                self.last.set(Some(crown));
                crown
            }
        };
        let tree = crown.overlay(x, z, ground);
        if distance <= CROWN_BLEND_START {
            return tree;
        }
        let weight = smoothstep((distance - CROWN_BLEND_START) * (1.0 / (CROWN_BLEND_END - CROWN_BLEND_START)));
        let stand = ground.elevation + stand_height(ground.elevation);
        Sample { elevation: tree.elevation + (stand - tree.elevation) * weight, ..tree }
    }
}

// At low altitude distant woods read as a canopy above the ground.
// It thins as the terrain rises toward the tree line.
fn stand_height(elevation: f32) -> f32 {
    (0.66 - (elevation - 38.0).max(0.0) * (0.43 / 38.0)).max(0.23)
}

#[derive(Clone, Copy)]
struct TreeCrown {
    tile: (i32, i32),
    center: [f32; 2],
    inverse_span: [f32; 2],
    height: f32,
    form: CrownForm,
}

#[derive(Clone, Copy)]
enum CrownForm {
    Bare,
    Rounded,
    Pointed,
    Leaning,
    Forked,
    Spire,
    Umbrella,
    Vase,
    Fan,
    Cluster,
    Crescent,
    Tiered,
    WindSwept,
    ThreeLobed,
    CutCrown,
    SplitRidge,
    Cushion,
}

fn grove_variant(seed: u64, tx: i32, tz: i32, state: u64) -> u8 {
    let variant = (state >> 20) as u8;
    // Keep occasional volunteers from all five families between the stands.
    if (state >> 32) & 15 < 3 {
        return variant;
    }
    // Oblique coordinates prevent alignment with the crown pitch. Small
    // per-tree offsets interleave the groves at their edges.
    let jitter_u = ((state >> 48) & 7) as i32 - 3;
    let jitter_v = ((state >> 54) & 15) as i32 - 7;
    let grove_x = tx.wrapping_mul(3).wrapping_add(tz).wrapping_add(jitter_u) >> 5;
    let grove_z = tz.wrapping_mul(4).wrapping_sub(tx).wrapping_add(jitter_v) >> 6;
    let grove = key(seed ^ 0x7690_edd1_7f4c_1a53, grove_x, grove_z);
    const MIXES: [[usize; 2]; 8] = [
        [0, 3], [1, 2], [2, 1], [3, 0], [4, 0], [1, 4], [0, 4], [3, 2],
    ];
    // These are the existing catalog's oak, spruce, aspen, maple and juniper forms.
    const FORMS: [[u8; 4]; 5] = [
        [0, 5, 8, 15], [1, 4, 10, 1], [6, 7, 14, 6], [3, 12, 13, 3], [2, 9, 11, 2],
    ];
    let pair = MIXES[(grove & 7) as usize];
    let family = pair[usize::from((state >> 41) & 7 == 0)];
    let form = FORMS[family][(variant & 3) as usize];
    (variant & 0xf0) | form
}

impl CrownForm {
    fn foliage(self) -> (Surface, f32) {
        match self {
            Self::Pointed | Self::Spire | Self::Tiered => (Surface::SpruceCanopy, 1.58),
            Self::Vase | Self::Fan | Self::SplitRidge => (Surface::AspenCanopy, 1.33),
            Self::Forked | Self::ThreeLobed | Self::CutCrown => (Surface::MapleCanopy, 1.08),
            Self::Leaning | Self::Crescent | Self::WindSwept => (Surface::JuniperCanopy, 0.82),
            _ => (Surface::TreeCanopy, 1.0),
        }
    }
}

impl TreeCrown {
    fn survey(seed: u64, tx: i32, tz: i32) -> Self {
        let state = key(seed, tx, tz);
        if state & 15 < 4 {
            return Self {
                tile: (tx, tz), center: [0.0; 2], inverse_span: [0.0; 2],
                height: 0.0, form: CrownForm::Bare,
            };
        }
        let center = [
            (tx as f32 + 0.36 + fraction(state) * 0.28) * TREE_TILE,
            (tz as f32 + 0.36 + fraction(state.rotate_left(11)) * 0.28) * TREE_TILE,
        ];
        Self::with_variant((tx, tz), center, grove_variant(seed, tx, tz, state))
    }

    fn with_variant(tile: (i32, i32), center: [f32; 2], variant: u8) -> Self {
        // Four footprint proportions and four height levels for each
        // silhouette. A crown's edge fades before it reaches its tile seam.
        const SPAN: [[f32; 2]; 4] = [
            [1.0 / 1.30, 1.0 / 1.30],
            [1.0 / 1.70, 1.0 / 1.20],
            [1.0 / 1.20, 1.0 / 1.69],
            [1.0 / 1.71, 1.0 / 1.66],
        ];
        const HEIGHT: [f32; 4] = [0.64, 0.86, 1.11, 1.38];
        const FORM: [CrownForm; 16] = [
            CrownForm::Rounded,
            CrownForm::Pointed,
            CrownForm::Leaning,
            CrownForm::Forked,
            CrownForm::Spire,
            CrownForm::Umbrella,
            CrownForm::Vase,
            CrownForm::Fan,
            CrownForm::Cluster,
            CrownForm::Crescent,
            CrownForm::Tiered,
            CrownForm::WindSwept,
            CrownForm::ThreeLobed,
            CrownForm::CutCrown,
            CrownForm::SplitRidge,
            CrownForm::Cushion,
        ];
        let variant = usize::from(variant);
        let form = FORM[variant & 15];
        Self {
            tile,
            center,
            inverse_span: SPAN[(variant >> 4) & 3],
            height: HEIGHT[variant >> 6] * form.foliage().1,
            form,
        }
    }

    fn overlay(self, x: f32, z: f32, ground: Sample) -> Sample {
        let u = (x - self.center[0]) * self.inverse_span[0];
        let v = (z - self.center[1]) * self.inverse_span[1];
        let (a, b) = (u.abs(), v.abs());
        if a >= 1.12 || b >= 1.12 {
            return ground;
        }
        let square = |value: f32| value * value;
        let squared = match self.form {
            CrownForm::Bare => return ground,
            CrownForm::Rounded => square(u) + square(v),
            CrownForm::Pointed => square(a + b),
            CrownForm::Leaning => square(u - v * 0.30) + square(v),
            CrownForm::Forked => square((a - 0.40) * (1.0 / 0.60)) + square(v),
            CrownForm::Spire => (a * 1.15 + b * 0.9).max(square(u * 1.25) + square(v * 0.90)),
            CrownForm::Umbrella => square(square(u)) + square(square(v * 1.09)),
            CrownForm::Vase => (square(u * 1.23) + square(v)) * if v < 0.0 { 1.3 } else { 0.7 },
            CrownForm::Fan => square(u * (1.2 - 0.35 * v)) + square(v * 1.03),
            CrownForm::Cluster => (square((u + 0.32) * 1.55) + square(v + 0.13)).min(square((u - 0.38) * 1.8) + square((v - 0.27) * 1.28)),
            CrownForm::Crescent => (square(a * 0.96) + square(v)).max(1.4 - (square(u * 1.4) + square((v - 0.75) * 2.4))),
            CrownForm::Tiered => (square(u) + square(v * 1.05)).max((a * 0.85 + b * 0.75) * 0.68),
            CrownForm::WindSwept => square(u * 1.02 - v * 0.38) + square(v * 1.26) + a * 0.2,
            CrownForm::ThreeLobed => (square(u * 1.28) + square((v + 0.36) * 1.58)).min(square((a - 0.43) * 1.83) + square((v - 0.31) * 1.57)),
            CrownForm::CutCrown => square(u * 0.94) + square(v) + (a + v - 0.75).max(0.0) * 1.5,
            CrownForm::SplitRidge => square(u * 1.13) + square((b - 0.38) * 1.52),
            CrownForm::Cushion => (a * 1.05).max(b * 1.05) * 0.78 + (square(u) + square(v)) * 0.12,
        };
        let border = (a.max(b) - 1.03).max(0.0) * (0.95 / 0.09);
        let squared = squared.max(border);
        if squared >= 0.95 {
            return ground;
        }
        let shape = 1.0 - squared;
        Sample {
            elevation: ground.elevation + self.height * shape * shape * (3.0 - 2.0 * shape),
            surface: self.form.foliage().0,
        }
    }
}

impl Default for GroundDetails {
    fn default() -> Self {
        Self::new()
    }
}

fn survey_farm(world: &World, tx: i32, tz: i32) -> Option<[f32; 5]> {
    let origin = [tx as f32 * TILE, tz as f32 * TILE];
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    let mut heights = [[0.0; 5]; 5];
    let offsets = [4.0, 18.0, 32.0, 46.0, 60.0];
    for (row, z) in offsets.into_iter().enumerate() {
        for (column, x) in offsets.into_iter().enumerate() {
            let sample = world.sample(origin[0] + x, origin[1] + z);
            if !matches!(sample.surface, Surface::Grass)
                || !(SEA_LEVEL + 4.0..=SEA_LEVEL + 50.0).contains(&sample.elevation)
            {
                return None;
            }
            heights[row][column] = sample.elevation;
            low = low.min(sample.elevation);
            high = high.max(sample.elevation);
            if high - low > 5.2 {
                return None;
            }
        }
    }
    Some([heights[0][2], heights[2][0], heights[2][2], heights[2][4], heights[4][2]])
}

fn fit_field_rows(
    [north, west, center, east, south]: [f32; 5],
    district_alignment: i8,
) -> (FieldLayout, i8, i8) {
    let dx = east - west;
    let dz = south - north;
    let (layout, across, along, curve) = if dx.abs() > dz.abs() {
        (FieldLayout::ContourAcrossX, dx, dz, (north + south) * 0.5 - center)
    } else {
        (FieldLayout::ContourAcrossZ, dz, dx, (west + east) * 0.5 - center)
    };
    if across.abs() < 1.45 {
        return (FieldLayout::Blocks, district_alignment, 0);
    }
    let reciprocal = 1.0 / across;
    let alignment = ((along * reciprocal).clamp(-0.75, 0.75) * 64.0) as i8;
    let bend = ((curve * reciprocal * 56.0).clamp(-7.0, 7.0) * 8.0) as i8;
    (layout, alignment, bend)
}

fn key(seed: u64, x: i32, z: i32) -> u64 {
    mix(seed ^ (u64::from(x as u32) << 32) ^ u64::from(z as u32))
}

fn grid_index(value: f32, inverse: f32) -> i32 {
    let scaled = value * inverse;
    let index = scaled as i32;
    index - i32::from(scaled < index as f32)
}

fn wrap(index: i32) -> usize {
    index.rem_euclid(SIDE) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_grain_survey_matches_the_full_ground_cache() {
        for seed in [9, 49, 165] {
            let world = World::new(seed);
            for origin in [[-384.0, 20.0, 256.0], [22.3, 46.0, 1150.75], [1904.5, 8.0, -610.0]] {
                let mut cache = GroundDetails::new();
                cache.reset(&world, seed, origin[0], origin[2]);
                let expected: std::vec::Vec<_> = cache.grain_field_bounds(origin).collect();
                let found: std::vec::Vec<_> = GroundDetails::survey_grain_fields(&world, seed, origin).collect();
                assert_eq!(found.len(), expected.len());
                assert!(found.iter().all(|bounds| expected.contains(bounds)));
            }
        }
    }
    use std::collections::HashSet;

    const FLAT_FOREST: Sample = Sample { elevation: 0.0, surface: Surface::Forest };

    #[test]
    fn stand_canopy_preserves_near_crowns_and_blends_smoothly_with_distance() {
        let world = World::new(49);
        let mut details = GroundDetails::new();
        details.reset(&world, 49, 0.0, 0.0);
        let [x, z] = [-3.7, 5.1];
        let local = details.tree_sampler().geometry(x, z, FLAT_FOREST);
        let far = details.tree_sampler().for_view(x + 160.0, 0.0, z).geometry(x, z, FLAT_FOREST);
        let far_other = details.tree_sampler().for_view(x, 160.0, z).geometry(x, z, FLAT_FOREST);
        let height_at = |offset| details.tree_sampler().for_view(x + offset, 0.0, z)
            .geometry(x, z, FLAT_FOREST).elevation;
        assert_eq!(height_at(50.0), local.elevation);
        assert_eq!(far, far_other);
        assert_eq!(far.surface, Surface::Forest);
        assert!((0.24..=1.49).contains(&far.elevation));
        assert!((height_at(54.01) - height_at(53.99)).abs() < 0.0001);
        assert!((height_at(122.01) - height_at(121.99)).abs() < 0.0001);
        let rock = Sample { surface: Surface::Rock, ..FLAT_FOREST };
        assert_eq!(details.tree_sampler().for_view(200.0, 200.0, 0.0).geometry(x, z, rock), rock);
    }

    #[test]
    fn all_256_crowns_have_distinct_sampled_geometry() {
        let mut signatures = HashSet::new();
        for variant in u8::MIN..=u8::MAX {
            let crown = TreeCrown::with_variant((0, 0), [0.0; 2], variant);
            let mut signature = [0; 17 * 17];
            for (index, cell) in signature.iter_mut().enumerate() {
                let x = (index % 17) as f32 * 0.25 - 2.0;
                let z = (index / 17) as f32 * 0.25 - 2.0;
                let sample = crown.overlay(x, z, FLAT_FOREST);
                if sample.surface.is_tree_canopy() {
                    *cell = 1 + (sample.elevation / 0.12) as u8;
                }
            }
            assert!(signatures.insert(signature), "repeated crown geometry for variant {variant}");
        }
    }

    #[test]
    fn seeded_signed_tiles_produce_every_crown_variant() {
        let mut geometries = HashSet::new();
        for seed in [11, 162, 4329, 7911] {
            for tx in -23..=23 {
                for tz in -23..=23 {
                    let crown = TreeCrown::survey(seed, tx, tz);
                    if !matches!(crown.form, CrownForm::Bare) {
                        geometries.insert((crown.form as u8, crown.inverse_span.map(f32::to_bits), crown.height.to_bits()));
                    }
                }
            }
        }
        assert_eq!(geometries.len(), 256);
    }

    #[test]
    fn seeded_groves_cluster_families_without_losing_mixed_woodland() {
        let mut families = std::collections::BTreeSet::new();
        let mut local = [0; 2];
        let mut distant = [0; 2];
        let family = |seed, x, z| {
            let crown = TreeCrown::survey(seed, x, z);
            (!matches!(crown.form, CrownForm::Bare)).then(|| {
                crown.form.foliage().0.canopy_color().unwrap()
            })
        };
        for seed in [11, 162, 4329, 7911] {
            for x in -34..=34 {
                for z in -35..=35 {
                    let Some(here) = family(seed, x, z) else { continue };
                    families.insert(here);
                    if let Some(next) = family(seed, x + 1, z) {
                        local[0] += usize::from(here == next);
                        local[1] += 1;
                    }
                    if let Some(next) = family(seed, x + 57, z - 31) {
                        distant[0] += usize::from(here == next);
                        distant[1] += 1;
                    }
                }
            }
        }
        let rate = |[same, pairs]: [usize; 2]| same as f32 / pairs as f32;
        let (near, far) = (rate(local), rate(distant));
        assert_eq!(families.len(), 5);
        assert!(near > 0.45 && near < 0.80 && near - far > 0.20, "{near} vs {far}");
    }

    #[test]
    fn widest_crowns_leave_clearance_at_signed_tile_seams() {
        for tx in [-4621, -1, 0, 4621] {
            for tz in [-4621, -1, 0, 4621] {
                let x = tx as f32 * TREE_TILE;
                let z = tz as f32 * TREE_TILE;
                for cx in [0.36, 0.64] {
                    for cz in [0.36, 0.64] {
                        let center = [x + TREE_TILE * cx, z + TREE_TILE * cz];
                        for form in 0..16 {
                            let crown = TreeCrown::with_variant((tx, tz), center, 240 + form);
                            for step in 0..=22 {
                                let offset = step as f32 * 0.25;
                                for seam in [0.0, TREE_TILE] {
                                    assert_eq!(crown.overlay(x + seam, z + offset, FLAT_FOREST), FLAT_FOREST);
                                    assert_eq!(crown.overlay(x + offset, z + seam, FLAT_FOREST), FLAT_FOREST);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn crown_cache_matches_uncached_geometry_across_tile_edges() {
        let mut details = GroundDetails::new();
        for seed in [11, 162] {
            details.reset(&World::new(seed), seed, -120.0, 140.0);
            let sampler = details.tree_sampler();
            let mut crowns = 0;
            for row in -40..=40 {
                for column in -80..=80 {
                    let x = column as f32 * 0.78;
                    let z = row as f32 * 1.31;
                    let ground = Sample {
                        elevation: 18.0 + x * 0.07,
                        surface: if column % 9 == 0 { Surface::Grass } else { Surface::Forest },
                    };
                    let expected = details.geometry(x, z, ground);
                    assert_eq!(sampler.geometry(x, z, ground), expected);
                    crowns += usize::from(expected.surface.is_tree_canopy());
                }
            }
            assert!(crowns > 100);
        }
    }

    #[test]
    fn features_stay_fixed_and_fields_stay_on_low_flat_ground() {
        let mut details = GroundDetails::new();
        let mut fields = 0;
        let mut trees = 0;
        for seed in [1, 31, 819, 7911] {
            let world = World::new(seed);
            details.reset(&world, seed, -220.0, 450.0);
            for iz in -12..=12 {
                for ix in -12..=12 {
                    let x = ix as f32 * 96.0;
                    let z = iz as f32 * 96.0;
                    details.follow(&world, x, z);
                    let natural = world.sample(x, z);
                    let enriched = details.sample(x, z, natural);
                    if enriched.surface.is_tree_canopy() {
                        trees += 1;
                        assert_eq!(natural.surface, Surface::Forest);
                        assert!((0.0..=2.20).contains(&(enriched.elevation - natural.elevation)));
                    }
                    if matches!(enriched.surface, Surface::FieldGreen | Surface::FieldGold | Surface::FieldSoil) {
                        fields += 1;
                        assert_eq!(natural.surface, Surface::Grass);
                        assert!(survey_farm(&world, grid_index(x, 1.0 / TILE), grid_index(z, 1.0 / TILE)).is_some());
                    }
                    details.follow(&world, x + 64.0, z - 64.0);
                    assert_eq!(enriched, details.sample(x, z, natural));
                }
            }
        }
        assert!(fields > 5 && trees > 5, "{fields} {trees}");
    }

    #[test]
    fn contour_buffers_follow_the_survey_and_leave_dry_headlands() {
        let (layout, alignment, bend) = fit_field_rows([21.0, 19.0, 21.0, 25.0, 23.0], 0);
        assert!(matches!(layout, FieldLayout::ContourAcrossX));
        assert!(alignment > 0 && bend > 0);
        let field = GroundTile { fields: true, x: 0, z: 0, layout, alignment, bend, ..GroundTile::EMPTY };
        for v in [12.0_f32, 24.0, 32.0, 42.0, 52.0] {
            let along = v - TILE * 0.5;
            let t = along * (1.0 / 28.0);
            let across = 20.0 - along * (f32::from(alignment) / 64.0)
                - t * t * f32::from(bend) / 8.0;
            assert_eq!(field.surface(across, v), Surface::Grass);
            assert_eq!(field.surface(across + 5.0, v), Surface::FieldSoil);
        }
        assert_eq!(field.surface(26.0, 3.0), Surface::Grass);
        assert!(matches!(fit_field_rows([21.0; 5], 0).0, FieldLayout::Blocks));
    }
}
