//! Sparse stone forms on surveyed shallow shelves beside rocky promontories.

use crate::beach::{BEACH_TILE_SIZE, BeachSite};
use crate::maritime::{ANCHORAGE_TILE, Anchorage, Vessel};
use crate::math::{fraction, mix, sin_cos};
use crate::ports::{Port, PortKind};
use crate::world::{SEA_LEVEL, Surface, World};

mod profiles;
pub use profiles::CoastalProfile;
use profiles::Silhouette;

pub const COASTAL_LANDFORM_TILE: i32 = 256;
const CACHE_SIDE: usize = 11;
const LANDMARK_LIMIT: usize = 24;
const NO_LANDMARK: u8 = u8::MAX;

#[derive(Clone, Copy)]
struct RegionSlot {
    tile: [i32; 2],
    landmark_index: u8,
}

impl RegionSlot {
    const EMPTY: Self = Self {
        tile: [i32::MIN; 2],
        landmark_index: NO_LANDMARK,
    };
}

/// An eleven-tile window leaves a preparation margin around the highest aircraft's view.
#[derive(Clone, Copy)]
pub struct CoastalLandmarks {
    slots: [RegionSlot; CACHE_SIDE * CACHE_SIDE],
    landmarks: [Option<CoastalLandform>; LANDMARK_LIMIT],
    center: [i32; 2],
    next: usize,
}

impl CoastalLandmarks {
    pub const fn new() -> Self {
        Self {
            slots: [RegionSlot::EMPTY; CACHE_SIDE * CACHE_SIDE],
            landmarks: [None; LANDMARK_LIMIT],
            center: [0; 2],
            next: 0,
        }
    }

    pub fn reset(&mut self, world: &World, seed: u64, x: f32, z: f32) {
        self.slots.fill(RegionSlot::EMPTY);
        self.landmarks.fill(None);
        self.next = 0;
        self.follow(world, seed, x, z, CACHE_SIDE * CACHE_SIDE);
    }

    pub fn follow(&mut self, world: &World, seed: u64, x: f32, z: f32, tile_budget: usize) {
        let center = [x, z].map(|value| tile_of(value, COASTAL_LANDFORM_TILE));
        if self.center != center {
            self.center = center;
            self.next = 0;
        }
        let half = CACHE_SIDE as i32 / 2;
        let mut updated = 0;
        while self.next < self.slots.len() && updated < tile_budget {
            let tx = center[0] + (self.next % CACHE_SIDE) as i32 - half;
            let tz = center[1] + (self.next / CACHE_SIDE) as i32 - half;
            let slot = (tz.rem_euclid(CACHE_SIDE as i32) as usize) * CACHE_SIDE
                + tx.rem_euclid(CACHE_SIDE as i32) as usize;
            self.next += 1;
            if self.slots[slot].tile == [tx, tz] {
                continue;
            }
            if let Some(previous) = self
                .landmarks
                .get_mut(self.slots[slot].landmark_index as usize)
            {
                *previous = None;
            }
            let mut landmark_index = NO_LANDMARK;
            if let Some(form) = CoastalLandform::generate(world, seed, tx, tz) {
                if let Some(index) = self.landmarks.iter().position(Option::is_none) {
                    self.landmarks[index] = Some(form);
                    landmark_index = index as u8;
                }
            }
            self.slots[slot] = RegionSlot {
                tile: [tx, tz],
                landmark_index,
            };
            updated += 1;
        }
    }

    pub fn forms(&self) -> impl Iterator<Item = &CoastalLandform> {
        self.landmarks.iter().flatten()
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        let [tx, tz] = [x, z].map(|value| tile_of(value, COASTAL_LANDFORM_TILE));
        let slot = self.slots[(tz.rem_euclid(CACHE_SIDE as i32) as usize) * CACHE_SIDE
            + tx.rem_euclid(CACHE_SIDE as i32) as usize];
        if slot.tile != [tx, tz] {
            return SEA_LEVEL;
        }
        let Some(form) = self
            .landmarks
            .get(slot.landmark_index as usize)
            .and_then(Option::as_ref)
        else {
            return SEA_LEVEL;
        };
        let [min_x, min_z, max_x, max_z] = form.bounds();
        let dx = (min_x - x).max(x - max_x).max(0.0);
        let dz = (min_z - z).max(z - max_z).max(0.0);
        let fade = (dx * dx + dz * dz) * 0.01;
        if fade >= 1.0 {
            return SEA_LEVEL;
        }
        let height = match form.shape {
            Shape::Stack(rock) => 3.0 + rock.height,
            Shape::Skerries(rocks) => {
                3.0 + rocks.iter().map(|rock| rock.height).fold(0.0, f32::max)
            }
            Shape::Arch(arch) => 3.0 + arch.height,
        };
        SEA_LEVEL + height * (1.0 - crate::math::smoothstep(fade))
    }
}

impl Default for CoastalLandmarks {
    fn default() -> Self {
        Self::new()
    }
}
const HEXAGON: [[f32; 2]; 6] = [
    [1.0, 0.0],
    [0.5, 0.866_025_4],
    [-0.5, 0.866_025_4],
    [-1.0, 0.0],
    [-0.5, -0.866_025_4],
    [0.5, -0.866_025_4],
];
const SHELF: core::ops::RangeInclusive<f32> = -0.099..=-0.002;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LandformKind {
    Stack,
    Skerries,
    Arch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stone {
    Slate,
    Sandstone,
    GrayRock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stratum {
    WaveCut,
    Face,
    Cap,
}

/// A world-space triangle. Vertices are x, elevation, z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RockFace {
    pub vertices: [[f32; 3]; 3],
    pub stone: Stone,
    pub stratum: Stratum,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Shore {
    edge: [f32; 2],
    out: [f32; 2],
    relief: f32,
    head: f32,
}

impl Shore {
    fn point(self, along: f32, out: f32) -> [f32; 2] {
        [
            self.edge[0] - self.out[1] * along + self.out[0] * out,
            self.edge[1] + self.out[0] * along + self.out[1] * out,
        ]
    }

    fn find(world: &World, candidate: [f32; 2]) -> Option<Self> {
        let [x, z] = candidate;
        if world.coast_signal(x, z).abs() > 0.24 {
            return None;
        }
        let dx = world.coast_signal(x - 9.0, z) - world.coast_signal(x + 9.0, z);
        let dz = world.coast_signal(x, z - 9.0) - world.coast_signal(x, z + 9.0);
        let strength = root(dx * dx + dz * dz);
        if strength < 0.023 {
            return None;
        }
        let out = [dx / strength, dz / strength];
        let point = |distance| [x + out[0] * distance, z + out[1] * distance];
        let mut dry = -82.0;
        let mut wet = 82.0;
        if coast(world, point(dry)) <= 0.0 || coast(world, point(wet)) > 0.0 {
            return None;
        }
        for _ in 0..10 {
            let middle = (dry + wet) * 0.5;
            if coast(world, point(middle)) > 0.0 {
                dry = middle;
            } else {
                wet = middle;
            }
        }
        let mut shore = Self {
            edge: point((dry + wet) * 0.5),
            out,
            relief: 0.0,
            head: 0.0,
        };
        let a = coast(world, shore.point(-34.0, 0.0));
        let b = coast(world, shore.point(34.0, 0.0));
        // Flanks that are still dry at the middle's shore identify a cove.
        shore.head = -(a + b) * 0.5;
        if shore.head < 0.011 || a.max(b) > 0.03 {
            return None;
        }
        let lower = elevation(world, shore.point(0.0, -42.0));
        let shoulder = elevation(world, shore.point(0.0, -94.0));
        let crown = elevation(world, shore.point(0.0, -155.0));
        if shoulder < 9.0 || shoulder - lower < 5.3 || crown < 20.0 {
            return None;
        }
        let left = elevation(world, shore.point(-25.0, -120.0));
        let right = elevation(world, shore.point(25.0, -120.0));
        if left.min(right) < 6.0 {
            return None;
        }
        shore.relief = crown.max(left).max(right);
        Some(shore)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StoneSpur {
    center: [f32; 2],
    axis: [f32; 2],
    scale: [f32; 2],
    lean: [f32; 2],
    height: f32,
    sides: [u8; 6],
    top_scale: f32,
    shoulder: f32,
}

impl StoneSpur {
    const EMPTY: Self = Self {
        center: [0.0; 2],
        axis: [1.0, 0.0],
        scale: [0.0; 2],
        lean: [0.0; 2],
        height: 0.0,
        sides: [0; 6],
        top_scale: 0.0,
        shoulder: 0.0,
    };

    fn new(
        center: [f32; 2],
        axis: [f32; 2],
        scale: [f32; 2],
        height: f32,
        lean: [f32; 2],
        key: u64,
    ) -> Self {
        Self {
            center,
            axis,
            scale,
            lean,
            height,
            sides: core::array::from_fn(|side| 83 + (mix(key ^ (side as u64 * 1151)) % 28) as u8),
            top_scale: 0.35 + fraction(mix(key ^ 0x71e4)) * 0.24,
            shoulder: 0.85,
        }
    }

    fn cap_radius(self) -> f32 {
        1.11 * self.scale[0].max(self.scale[1]) + root(dot(self.lean, self.lean))
    }

    fn radius(self, level: f32) -> f32 {
        if level < 0.62 {
            1.0 + (self.shoulder - 1.0) * (level / 0.62)
        } else {
            self.shoulder + (self.top_scale - self.shoulder) * ((level - 0.62) / 0.38)
        }
    }

    fn ring(self, level: f32, band: usize) -> [[f32; 3]; 6] {
        let center = offset(self.center, self.lean, level);
        let radius = self.radius(level);
        let height = SEA_LEVEL - 0.42 + (self.height + 0.42) * level;
        core::array::from_fn(|index| {
            let [u, v] = HEXAGON[index];
            let flank = f32::from(self.sides[index]) * 0.01;
            let fracture = f32::from(self.sides[(index + band * 2) % 6]) * 0.01;
            let scale = radius * (flank + (fracture - flank) * level * 0.55);
            let tip = (fracture - 0.965) * self.height * level * level * 0.95;
            let [x, z] = local_point(
                center,
                self.axis,
                u * self.scale[0] * scale,
                v * self.scale[1] * scale,
            );
            [x, height + tip, z]
        })
    }

    fn contains_ring(self, x: f32, z: f32, level: f32) -> bool {
        let [u, v] = subtract([x, z], offset(self.center, self.lean, level));
        let scale = self.radius(level);
        let a = (u * self.axis[0] + v * self.axis[1]) / (self.scale[0] * scale);
        let b = (-u * self.axis[1] + v * self.axis[0]) / (self.scale[1] * scale);
        // A cheap inner envelope of the six faceted sides.
        a * a + b * b <= 0.70
    }

    fn height_at(self, x: f32, z: f32) -> Option<f32> {
        if self.height <= 0.0 || !within(self.center, [x, z], self.cap_radius()) {
            return None;
        }
        let mut upper = 1.0;
        let mut lower = [1.0, 0.81, 0.62, 0.39, 0.19, 0.0]
            .into_iter()
            .find(|&level| {
                if self.contains_ring(x, z, level) {
                    true
                } else {
                    upper = level;
                    false
                }
            })?;
        for _ in 0..5 {
            let level = (lower + upper) * 0.5;
            if self.contains_ring(x, z, level) {
                lower = level;
            } else {
                upper = level;
            }
        }
        let height = SEA_LEVEL - 0.42 + (self.height + 0.42) * lower;
        (height > SEA_LEVEL).then_some(height)
    }

    fn faces(self, stone: Stone, emit: &mut impl FnMut(RockFace)) {
        if self.height == 0.0 {
            return;
        }
        let mut previous = self.ring(0.0, 0);
        for (band, (level, stratum)) in [
            (0.19, Stratum::WaveCut),
            (0.62, Stratum::Face),
            (1.0, Stratum::Face),
        ]
        .into_iter()
        .enumerate()
        {
            let ring = self.ring(level, band + 1);
            for side in 0..6 {
                let next = (side + 1) % 6;
                quad(
                    [previous[side], ring[side], ring[next], previous[next]],
                    stone,
                    stratum,
                    emit,
                );
            }
            previous = ring;
        }
        for i in 1..5 {
            emit(RockFace {
                vertices: [previous[0], previous[i + 1], previous[i]],
                stone,
                stratum: Stratum::Cap,
            });
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StoneArch {
    span: f32,
    gap: f32,
    thickness: f32,
    height: f32,
    clearance: f32,
    asymmetry: f32,
    crown: f32,
    fractures: [u8; 9],
    axis: [f32; 2],
}

impl StoneArch {
    fn feet(self, center: [f32; 2]) -> [([f32; 2], [f32; 2]); 2] {
        let separation = (self.span + self.gap) * 0.5;
        let width = (self.span - self.gap) * 0.5;
        [-1.0, 1.0].map(|side| {
            (
                offset(center, self.axis, side * separation),
                [width, self.thickness],
            )
        })
    }

    fn feet_height(self, center: [f32; 2], x: f32, z: f32) -> Option<f32> {
        self.feet(center)
            .into_iter()
            .find_map(|(heel, [width, depth])| {
                let p = subtract([x, z], heel);
                let u = dot(p, self.axis) / width;
                let v = dot(p, perpendicular(self.axis)) / depth;
                (u * u + v * v <= 0.76).then_some(SEA_LEVEL + 1.22 + (1.0 - u.abs()) * 0.82)
            })
    }

    fn faces(self, center: [f32; 2], stone: Stone, emit: &mut impl FnMut(RockFace)) {
        const PROFILE: [[f32; 2]; 9] = [
            [-1.0, 0.0],
            [-0.91, 0.42],
            [-0.71, 0.72],
            [-0.42, 0.91],
            [0.0, 1.0],
            [0.43, 0.93],
            [0.74, 0.68],
            [0.92, 0.39],
            [1.0, 0.0],
        ];
        let ring = |index: usize| {
            let [u, rise] = PROFILE[index];
            let rise = rise / (self.crown + (1.0 - self.crown) * rise);
            let fault = self.fractures[index];
            let cut = (f32::from(fault & 15) - 7.5) * (1.0 / 7.5);
            let strata = (f32::from(fault >> 4) - 7.5) * (1.0 / 7.5);
            let drift = (self.asymmetry + 0.52 * cut) * rise;
            let width = self.thickness * (1.0 + (0.17 * cut - 0.43) * rise);
            let inner = (self.clearance + 0.46 * cut) * rise - 0.54;
            let outer = (self.height + 1.6 * u * self.asymmetry + 1.17 * strata) * rise - 0.54;
            [
                (
                    self.span * u + drift,
                    -width * (1.0 + 0.16 * strata * rise),
                    outer,
                ),
                (
                    self.span * u + drift,
                    width * 0.80,
                    outer + (0.43 - 0.97 * cut) * rise,
                ),
                (self.gap * u + drift, width * 0.98, inner),
                (self.gap * u + drift, -width * 0.83, inner - 0.32 * rise),
            ]
            .map(|(x, z, y)| {
                let [x, z] = local_point(center, self.axis, x, z);
                [x, SEA_LEVEL + y, z]
            })
        };
        let mut a = ring(0);
        for index in 0..PROFILE.len() - 1 {
            let b = ring(index + 1);
            for side in 0..4 {
                let next = (side + 1) % 4;
                let stratum = if side == 0 {
                    Stratum::Cap
                } else if index == 0 || index == 7 || side == 2 {
                    Stratum::WaveCut
                } else {
                    Stratum::Face
                };
                quad([a[side], a[next], b[next], b[side]], stone, stratum, emit);
            }
            a = b;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    Stack(StoneSpur),
    Skerries([StoneSpur; 3]),
    Arch(StoneArch),
}

/// Immutable geometry tied to a particular shallow shelf and headland.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoastalLandform {
    center: [f32; 2],
    radius: f32,
    shore: Shore,
    stone: Stone,
    profile: CoastalProfile,
    shape: Shape,
}

impl CoastalLandform {
    pub fn generate(world: &World, seed: u64, tile_x: i32, tile_z: i32) -> Option<Self> {
        Self::generate_with(world, seed, tile_x, tile_z, |_| false)
    }

    /// Use the same regional reservations whenever a tile is prepared. The
    /// callback can reject plans that conflict with a port or another feature.
    pub fn generate_with(
        world: &World,
        seed: u64,
        tile_x: i32,
        tile_z: i32,
        mut excluded: impl FnMut(&Self) -> bool,
    ) -> Option<Self> {
        if !(-8192..8192).contains(&tile_x) || !(-8192..8192).contains(&tile_z) {
            return None;
        }
        let tile = (u64::from(tile_x as u32) << 32) | u64::from(tile_z as u32);
        let key = mix(seed ^ tile ^ 0xbb8a_e712_510b_0cdd);
        if key & 7 >= 4 {
            return None;
        }
        let kind = match (key >> 12) % 9 {
            0 => LandformKind::Arch,
            1..=4 => LandformKind::Stack,
            _ => LandformKind::Skerries,
        };
        let origin = [
            (tile_x * COASTAL_LANDFORM_TILE) as f32,
            (tile_z * COASTAL_LANDFORM_TILE) as f32,
        ];
        let shift = [
            fraction(mix(key ^ 93)) * 12.0,
            fraction(mix(key ^ 352)) * 12.0,
        ];
        let first_cell = (key >> 28) as usize & 15;
        for attempt in 0..16 {
            let cell = (first_cell + 5 * attempt) & 15;
            let candidate = [
                origin[0] + 40.0 + (cell % 4) as f32 * 55.0 + shift[0],
                origin[1] + 40.0 + (cell / 4) as f32 * 55.0 + shift[1],
            ];
            let Some(shore) = Shore::find(world, candidate) else {
                continue;
            };
            let design = mix(key ^ (attempt as u64 + 4477));
            let preferred = if (design >> 31) % 7 == 0 {
                LandformKind::Arch
            } else {
                kind
            };
            for choice in [Some(preferred), (preferred != kind).then_some(kind)]
                .into_iter()
                .flatten()
            {
                let Some(plan) = Self::arrange(shore, choice, design) else {
                    continue;
                };
                let [lx, lz, hx, hz] = plan.bounds();
                let margin = 10.0;
                if lx >= origin[0] + margin
                    && lz >= origin[1] + margin
                    && hx <= origin[0] + COASTAL_LANDFORM_TILE as f32 - margin
                    && hz <= origin[1] + COASTAL_LANDFORM_TILE as f32 - margin
                    && plan.supported_by(world)
                    && !excluded(&plan)
                    && plan.clears_anchorages(world, seed)
                    && plan.clears_beaches(world, seed)
                {
                    return Some(plan);
                }
            }
        }
        None
    }

    fn arrange(shore: Shore, kind: LandformKind, key: u64) -> Option<Self> {
        let variety = fraction(mix(key ^ 0xec19));
        let (sine, cosine) = sin_cos((fraction(mix(key ^ 0x11b4)) - 0.5) * 0.65);
        let tangent = perpendicular(shore.out);
        let axis = [
            tangent[0] * cosine + shore.out[0] * sine,
            tangent[1] * cosine + shore.out[1] * sine,
        ];
        let out = match kind {
            LandformKind::Skerries => 15.5 + variety * 6.0,
            LandformKind::Stack => 19.5 + variety * 10.0,
            LandformKind::Arch => 15.0 + variety * 6.2,
        };
        let center = shore.point((fraction(mix(key ^ 0x5f6d)) - 0.5) * 13.0, out);
        let stone = match (key >> 16) % 5 {
            0 => Stone::Slate,
            1 | 2 if shore.relief < 75.0 => Stone::Sandstone,
            _ => Stone::GrayRock,
        };
        let profile = CoastalProfile::choose(kind, mix(key ^ 0x40dc));
        Self::build_profile(shore, center, axis, stone, profile, key)
    }

    fn build_profile(
        shore: Shore,
        center: [f32; 2],
        axis: [f32; 2],
        stone: Stone,
        profile: CoastalProfile,
        key: u64,
    ) -> Option<Self> {
        let variation = 0.93 + fraction(mix(key ^ 0xa493)) * 0.14;
        let (shape, radius) = match profile.silhouette() {
            Silhouette::Stack([width, depth, rise, crown, shoulder, lean])
                if shore.relief >= 24.0 =>
            {
                let height = (9.5 + shore.relief * 0.24).clamp(13.0, 25.5) * rise * variation;
                let mut rock = StoneSpur::new(
                    center,
                    axis,
                    [width * variation, depth * variation],
                    height,
                    shore.out.map(|v| v * lean),
                    key,
                );
                rock.top_scale = crown;
                rock.shoulder = shoulder;
                (Shape::Stack(rock), rock.cap_radius())
            }
            Silhouette::Skerries(layout) => {
                let mut radius = 0.0_f32;
                let rocks = core::array::from_fn(|index| {
                    let [along, out, width, depth, height] = layout[index];
                    if height == 0.0 {
                        return StoneSpur::EMPTY;
                    }
                    let seed = mix(key ^ (index as u64 + 77));
                    let position = offset(offset(center, axis, along), shore.out, out);
                    let rock = StoneSpur::new(
                        position,
                        axis,
                        [width * variation, depth * variation],
                        height * variation,
                        axis.map(|v| v * (fraction(seed) - 0.5)),
                        seed,
                    );
                    let delta = subtract(position, center);
                    radius = radius.max(root(dot(delta, delta)) + rock.cap_radius());
                    rock
                });
                (Shape::Skerries(rocks), radius)
            }
            Silhouette::Arch([span, gap, depth, height, clearance, asymmetry, crown])
                if shore.relief >= 29.5 && shore.head >= 0.013 =>
            {
                let height = height * variation;
                let arch = StoneArch {
                    span,
                    gap: span * gap,
                    thickness: depth,
                    height,
                    clearance: height * clearance,
                    asymmetry,
                    crown,
                    fractures: core::array::from_fn(|station| {
                        mix(key ^ (station as u64 * 9299 + 7073)) as u8
                    }),
                    axis,
                };
                (Shape::Arch(arch), span + depth + 1.0)
            }
            _ => return None,
        };
        Some(Self {
            center,
            radius,
            shore,
            stone,
            profile,
            shape,
        })
    }

    /// The same mesh used in flight, centered for the artwork gallery.
    pub fn artwork(profile: CoastalProfile) -> Self {
        let shore = Shore {
            edge: [0.0; 2],
            out: [0.0, 1.0],
            relief: 52.0,
            head: 0.02,
        };
        Self::build_profile(shore, [0.0; 2], [1.0, 0.0], Stone::GrayRock, profile, 89)
            .expect("The artwork shelf supports all rock profiles")
    }

    pub fn profile(&self) -> CoastalProfile {
        self.profile
    }

    pub fn center(&self) -> [f32; 2] {
        self.center
    }

    pub fn shore(&self) -> [f32; 2] {
        self.shore.edge
    }

    pub fn kind(&self) -> LandformKind {
        match self.shape {
            Shape::Stack(_) => LandformKind::Stack,
            Shape::Skerries(_) => LandformKind::Skerries,
            Shape::Arch(_) => LandformKind::Arch,
        }
    }

    pub fn stone(&self) -> Stone {
        self.stone
    }

    /// Bounds as minimum x, minimum z, maximum x, maximum z.
    pub fn bounds(&self) -> [f32; 4] {
        let [x, z] = self.center;
        [
            x - self.radius,
            z - self.radius,
            x + self.radius,
            z + self.radius,
        ]
    }

    /// Surface height for rock forms and the submerged-to-low exposed footings
    /// of an arch. Render the arch itself from `faces` to retain its opening.
    pub fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        if !within(self.center, [x, z], self.radius) {
            return None;
        }
        match self.shape {
            Shape::Stack(rock) => rock.height_at(x, z),
            Shape::Skerries(rocks) => rocks
                .into_iter()
                .filter_map(|rock| rock.height_at(x, z))
                .reduce(f32::max),
            Shape::Arch(arch) => arch.feet_height(self.center, x, z),
        }
    }

    pub fn faces(&self, mut emit: impl FnMut(RockFace)) {
        match self.shape {
            Shape::Stack(rock) => rock.faces(self.stone, &mut emit),
            Shape::Skerries(rocks) => {
                for rock in rocks {
                    rock.faces(self.stone, &mut emit);
                }
            }
            Shape::Arch(arch) => arch.faces(self.center, self.stone, &mut emit),
        }
    }

    pub fn draw(&self, mesh: &mut crate::world_mesh::WorldMesh<'_>) {
        self.faces(|face| {
            let palette = match face.stone {
                Stone::Slate => [[71, 76, 80], [113, 120, 121], [126, 138, 123]],
                Stone::Sandstone => [[105, 83, 68], [173, 139, 104], [193, 166, 122]],
                Stone::GrayRock => [[81, 90, 92], [128, 131, 125], [151, 150, 137]],
            };
            let color = match face.stratum {
                Stratum::WaveCut => palette[0],
                Stratum::Face => palette[1],
                Stratum::Cap => palette[2],
            };
            mesh.lit_triangle(face.vertices, color);
        });
    }

    /// Protect laid-out port infrastructure, moorings, and the outward route
    /// used to approach the docks. The outer route widens away from shore.
    pub fn clears_port(&self, port: &Port) -> bool {
        let [x, z] = self.center;
        let [lx, lz, hx, hz] = port.bounds();
        if within(
            self.center,
            [x.clamp(lx, hx), z.clamp(lz, hz)],
            self.radius + 6.0,
        ) {
            return false;
        }
        if port.vessels().iter().any(|boat| {
            within(
                self.center,
                [boat.x, boat.z],
                self.radius + boat.length * 0.54 + 8.0,
            )
        }) {
            return false;
        }
        let out = port.seaward();
        let delta = subtract(self.center, port.shore_origin());
        let ahead = dot(delta, out);
        let across = dot(delta, perpendicular(out)).abs();
        let (length, width) = match port.kind() {
            PortKind::Marina => (84.0, 16.0),
            PortKind::Ferry => (110.0, 20.0),
            PortKind::Freight => (144.0, 24.0),
        };
        ahead < -self.radius
            || ahead > length + self.radius
            || across > width + 0.16 * ahead.max(0.0) + self.radius
    }

    fn supported_by(&self, world: &World) -> bool {
        let rock = |rock: StoneSpur| {
            rock.height == 0.0
                || shelf_support(
                    world,
                    rock.center,
                    rock.axis,
                    rock.scale.map(|axis| axis * 1.13),
                )
        };
        match self.shape {
            Shape::Stack(stack) => rock(stack),
            Shape::Skerries(rocks) => rocks.into_iter().all(rock),
            Shape::Arch(arch) => {
                arch.feet(self.center).into_iter().all(|(center, size)| {
                    [-1.0, 0.0, 1.0].into_iter().all(|a| {
                        [-1.0, 0.0, 1.0].into_iter().all(|b| {
                            shallow(
                                world,
                                local_point(
                                    center,
                                    arch.axis,
                                    a * (size[0] + 0.48),
                                    b * (size[1] + 0.48),
                                ),
                            )
                        })
                    })
                }) && [-0.75, -0.30, 0.0, 0.30, 0.75].into_iter().all(|along| {
                    [-0.56, 0.0, 0.56].into_iter().all(|across| {
                        shallow(
                            world,
                            local_point(
                                self.center,
                                arch.axis,
                                arch.gap * along,
                                arch.thickness * across,
                            ),
                        )
                    })
                })
            }
        }
    }

    fn clears_anchorages(&self, world: &World, seed: u64) -> bool {
        let [tile_x, tile_z] = self.center.map(|value| tile_of(value, ANCHORAGE_TILE));
        for tz in tile_z - 1..=tile_z + 1 {
            for tx in tile_x - 1..=tile_x + 1 {
                if let Some(boat) = Anchorage::generate(world, seed, tx, tz) {
                    let position = [boat.x, boat.z];
                    let (clearance, width) = match boat.vessel {
                        Vessel::Freighter => (35.0, 14.0),
                        Vessel::Ferry => (27.0, 12.0),
                        Vessel::Sailboat => (18.0, 8.0),
                        Vessel::FishingBoat => (21.0, 9.0),
                        Vessel::Tug => (18.0, 9.0),
                        Vessel::Service => {
                            let [length, beam] = boat.half_extents();
                            (length + 12.0, beam + 6.0)
                        }
                    };
                    let delta = subtract(self.center, position);
                    let ahead = dot(delta, self.shore.out);
                    let across = dot(delta, perpendicular(self.shore.out)).abs();
                    if within(self.center, position, self.radius + clearance)
                        || (0.0..=92.0 + self.radius).contains(&ahead)
                            && across <= width + self.radius
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn clears_beaches(&self, world: &World, seed: u64) -> bool {
        let [tile_x, tile_z] = self.shore.edge.map(|value| tile_of(value, BEACH_TILE_SIZE));
        for tz in tile_z - 1..=tile_z + 1 {
            for tx in tile_x - 1..=tile_x + 1 {
                if BeachSite::reserved_footprint(world, seed, tx, tz).is_some_and(|beach| {
                    beach.conflicts_with(|x, z| within(self.shore.edge, [x, z], 36.0 + self.radius))
                }) {
                    return false;
                }
            }
        }
        true
    }
}

fn shallow(world: &World, p: [f32; 2]) -> bool {
    SHELF.contains(&coast(world, p))
        && matches!(world.sample(p[0], p[1]).surface, Surface::Shallows)
}

fn shelf_support(world: &World, center: [f32; 2], axis: [f32; 2], scale: [f32; 2]) -> bool {
    shallow(world, center)
        && HEXAGON.into_iter().all(|[a, b]| {
            shallow(world, local_point(center, axis, a * scale[0], b * scale[1]))
                && shallow(
                    world,
                    local_point(center, axis, a * scale[0] * 0.60, b * scale[1] * 0.60),
                )
        })
        && [-1.0, 1.0]
            .into_iter()
            .all(|sign| shallow(world, local_point(center, axis, 0.0, sign * scale[1])))
}

fn tile_of(coordinate: f32, side: i32) -> i32 {
    let scaled = coordinate / side as f32;
    let cell = scaled as i32;
    cell - i32::from(scaled < cell as f32)
}

fn quad(vertices: [[f32; 3]; 4], stone: Stone, stratum: Stratum, emit: &mut impl FnMut(RockFace)) {
    let [a, b, c, d] = vertices;
    emit(RockFace {
        vertices: [a, b, c],
        stone,
        stratum,
    });
    emit(RockFace {
        vertices: [a, c, d],
        stone,
        stratum,
    });
}

fn coast(world: &World, [x, z]: [f32; 2]) -> f32 {
    world.coast_signal(x, z)
}

fn elevation(world: &World, [x, z]: [f32; 2]) -> f32 {
    world.elevation(x, z)
}

fn subtract(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn within(a: [f32; 2], b: [f32; 2], radius: f32) -> bool {
    let delta = subtract(a, b);
    dot(delta, delta) <= radius * radius
}

fn perpendicular(axis: [f32; 2]) -> [f32; 2] {
    [-axis[1], axis[0]]
}

fn offset(point: [f32; 2], axis: [f32; 2], distance: f32) -> [f32; 2] {
    [point[0] + axis[0] * distance, point[1] + axis[1] * distance]
}

fn local_point(point: [f32; 2], axis: [f32; 2], along: f32, across: f32) -> [f32; 2] {
    offset(offset(point, axis, along), perpendicular(axis), across)
}

fn root(value: f32) -> f32 {
    if value == 0.0 {
        return 0.0;
    }
    let start = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    let next = 0.5 * (start + value / start);
    0.5 * (next + value / next)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: [(u64, i32, i32, LandformKind); 3] = [
        (0, -3, -4, LandformKind::Stack),
        (2, 7, 2, LandformKind::Skerries),
        (2, -5, 7, LandformKind::Arch),
    ];

    #[test]
    fn three_kinds_keep_supported_footings_and_clearance() {
        for (seed, tx, tz, kind) in FIXTURES {
            let world = World::new(seed);
            let landform = CoastalLandform::generate(&world, seed, tx, tz).unwrap();
            assert_eq!(landform.kind(), kind);
            assert_eq!(
                CoastalLandform::generate(&world, seed, tx, tz),
                Some(landform)
            );
            assert!(CoastalLandform::generate_with(&world, seed, tx, tz, |_| true).is_none());
            assert!(landform.shore.head > 0.01);
            assert!(landform.supported_by(&world));
            assert!(landform.clears_beaches(&world, seed));
            assert!(landform.clears_anchorages(&world, seed));
            let [lx, lz, hx, hz] = landform.bounds();
            let mut faces = 0;
            landform.faces(|face| {
                faces += 1;
                for [x, y, z] in face.vertices {
                    assert!((lx..=hx).contains(&x) && (lz..=hz).contains(&z));
                    if y <= SEA_LEVEL + 0.1 {
                        assert_eq!(world.sample(x, z).surface, Surface::Shallows);
                    }
                }
            });
            assert!(faces >= 40);
        }
    }

    #[test]
    fn natural_arch_keeps_a_real_opening_above_water() {
        let world = World::new(2);
        let plan = CoastalLandform::generate(&world, 2, -5, 7).unwrap();
        let Shape::Arch(arch) = plan.shape else {
            panic!("expected an arch")
        };
        assert_eq!(plan.height_at(plan.center[0], plan.center[1]), None);
        for ([x, z], _) in arch.feet(plan.center) {
            assert!(plan.height_at(x, z).unwrap() > SEA_LEVEL + 1.0);
        }
        let mut opening = false;
        let mut crown = false;
        plan.faces(|face| {
            let points = face
                .vertices
                .map(|[x, y, z]| [dot(subtract([x, z], plan.center), arch.axis), y]);
            let edge = |a: [f32; 2], b: [f32; 2], p: [f32; 2]| {
                (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
            };
            let [a, b, c] = points;
            let area = edge(a, b, c);
            if area.abs() < 0.01 {
                return;
            }
            let covers = |p| {
                edge(a, b, p) * area >= 0.0
                    && edge(b, c, p) * area >= 0.0
                    && edge(c, a, p) * area >= 0.0
            };
            opening |= covers([0.0, SEA_LEVEL + arch.clearance * 0.51]);
            crown |= covers([0.0, SEA_LEVEL + (arch.clearance + arch.height) * 0.5 - 0.6]);
        });
        assert!(!opening && crown);
    }

    #[test]
    fn port_reservation_guards_its_seaward_approach() {
        let world = World::new(1);
        let port = Port::generate(&world, 1, -368.0, -2584.0).unwrap();
        let mut candidate = CoastalLandform::generate(&World::new(0), 0, -3, -4).unwrap();
        candidate.center = offset(port.shore_origin(), port.seaward(), 74.0);
        assert!(!candidate.clears_port(&port));
        candidate.center = offset(candidate.center, perpendicular(port.seaward()), 122.0);
        assert!(candidate.clears_port(&port));
    }
}
