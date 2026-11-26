//! Terrain-bound details with a stable position in every seeded world.

use crate::math::{fraction, mix, smoothstep};
use crate::world::{SEA_LEVEL, Sample, Surface, World};

const SIDE: i32 = 19;
const TILE: f32 = 64.0;
const TREE_TILE: f32 = 5.5;
const FARM_DISTRICT_TILES: i32 = 5;

#[derive(Clone, Copy)]
struct GroundTile {
    x: i32,
    z: i32,
    fields: bool,
    pattern: u32,
    alignment: i8,
}

impl GroundTile {
    const EMPTY: Self = Self {
        x: i32::MIN, z: i32::MIN, fields: false, pattern: 0, alignment: 0,
    };

    fn prepare(world: &World, seed: u64, x: i32, z: i32) -> Self {
        let state = key(seed, x, z);
        let [district_x, district_z] = [x, z].map(|tile| tile.div_euclid(FARM_DISTRICT_TILES));
        let layout = key(seed ^ 0x17da_e34d_8f0b_2671, district_x, district_z);
        Self {
            x,
            z,
            fields: farm_density(seed, x, z) > 0.32 && state & 15 != 0 && cultivable(world, x, z),
            pattern: (state >> 24) as u32,
            alignment: ((layout >> 42) & 31) as i8 - 16,
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
        TreeSampler { seed: self.seed ^ 0x27a4_2dec, last: core::cell::Cell::new(None) }
    }
}

/// Retains the prepared crown while consecutive rays sample its world tile.
pub struct TreeSampler {
    seed: u64,
    last: core::cell::Cell<Option<TreeCrown>>,
}

impl TreeSampler {
    #[inline]
    pub fn geometry(&self, x: f32, z: f32, ground: Sample) -> Sample {
        if ground.surface != Surface::Forest {
            return ground;
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
        crown.overlay(x, z, ground)
    }
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
        Self::with_variant((tx, tz), center, (state >> 20) as u8)
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
        Self {
            tile,
            center,
            inverse_span: SPAN[(variant >> 4) & 3],
            height: HEIGHT[variant >> 6],
            form: FORM[variant & 15],
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
            surface: Surface::TreeCanopy,
        }
    }
}

impl Default for GroundDetails {
    fn default() -> Self {
        Self::new()
    }
}

fn cultivable(world: &World, tx: i32, tz: i32) -> bool {
    let origin = [tx as f32 * TILE, tz as f32 * TILE];
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for z in [4.0, 18.0, 32.0, 46.0, 60.0] {
        for x in [4.0, 18.0, 32.0, 46.0, 60.0] {
            let sample = world.sample(origin[0] + x, origin[1] + z);
            if !matches!(sample.surface, Surface::Grass)
                || !(SEA_LEVEL + 4.0..=SEA_LEVEL + 50.0).contains(&sample.elevation)
            {
                return false;
            }
            low = low.min(sample.elevation);
            high = high.max(sample.elevation);
            if high - low > 5.2 {
                return false;
            }
        }
    }
    true
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
    use std::collections::HashSet;

    const FLAT_FOREST: Sample = Sample { elevation: 0.0, surface: Surface::Forest };

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
                if sample.surface == Surface::TreeCanopy {
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
                    crowns += usize::from(expected.surface == Surface::TreeCanopy);
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
                    if enriched.surface == Surface::TreeCanopy {
                        trees += 1;
                        assert_eq!(natural.surface, Surface::Forest);
                        assert!((0.0..=1.40).contains(&(enriched.elevation - natural.elevation)));
                    }
                    if matches!(enriched.surface, Surface::FieldGreen | Surface::FieldGold | Surface::FieldSoil) {
                        fields += 1;
                        assert_eq!(natural.surface, Surface::Grass);
                        assert!(cultivable(&world, grid_index(x, 1.0 / TILE), grid_index(z, 1.0 / TILE)));
                    }
                    details.follow(&world, x + 64.0, z - 64.0);
                    assert_eq!(enriched, details.sample(x, z, natural));
                }
            }
        }
        assert!(fields > 5 && trees > 5, "{fields} {trees}");
    }
}
