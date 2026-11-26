//! Surveyed playas, dunes, badlands, and coastal bluffs.

use crate::coastal_bluffs::{BluffSampler, CoastalBluff};
use crate::cove_dunes::CoveDune;
use crate::math::{fraction, mix, sin_cos, smoothstep};
use crate::world::{Sample, Surface, World, SEA_LEVEL};

pub const DRY_LANDMARK_TILE: i32 = 512;
pub(crate) const MAX_DRY_BASINS: usize = 4;
const SIDE: usize = 31;
const CENTER: usize = SIDE / 2;
const STEP: f32 = 4.0;
const RADIUS: f32 = CENTER as f32 * STEP;
const EDGE_MARGIN: f32 = RADIUS + 10.0;
const LOWLAND: core::ops::RangeInclusive<f32> = SEA_LEVEL + 6.0..=SEA_LEVEL + 49.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DryLandmarkKind {
    SaltPan,
    Dunes,
    Badlands,
    CoastalDune,
    CoastalBluff,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Wind {
    axis: [f32; 2],
    frequency: f32,
    phase: f32,
    bow: f32,
    relief: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    SaltPan([[u32; 2]; SIDE]),
    Dunes(Wind, [[u32; 2]; SIDE]),
    Badlands(Ravine),
    CoastalDune(CoveDune),
    CoastalBluff(CoastalBluff),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DryLandmark {
    center: [f32; 2],
    bounds: [f32; 4],
    floor: f32,
    rim: f32,
    shape: Shape,
}

impl DryLandmark {
    pub fn generate(world: &World, seed: u64, tile_x: i32, tile_z: i32) -> Option<Self> {
        Self::generate_with(world, seed, tile_x, tile_z, |_| false)
    }

    /// Reject footprints reserved by other prepared scenery before publishing a site.
    pub fn generate_with(
        world: &World,
        seed: u64,
        tile_x: i32,
        tile_z: i32,
        excluded: impl FnMut([f32; 4]) -> bool,
    ) -> Option<Self> {
        Self::generate_with_cooperatively(world, seed, tile_x, tile_z, excluded, &mut || {})
    }

    pub(crate) fn generate_with_cooperatively(
        world: &World,
        seed: u64,
        tile_x: i32,
        tile_z: i32,
        mut excluded: impl FnMut([f32; 4]) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        pulse();
        if !(-4096..4096).contains(&tile_x) || !(-4096..4096).contains(&tile_z) {
            return None;
        }
        let tile = (u64::from(tile_x as u32) << 32) | u64::from(tile_z as u32);
        let key = mix(seed ^ tile ^ 0x4c63_2c5d_45ff_b19b);
        let origin = [
            (tile_x * DRY_LANDMARK_TILE) as f32,
            (tile_z * DRY_LANDMARK_TILE) as f32,
        ];
        Self::generate_basin(world, origin, key, &mut excluded, pulse)
            .or_else(|| {
                Ravine::survey(
                    world,
                    origin,
                    mix(seed ^ tile ^ 0xf2bc_c260_5d91_3644),
                    &mut excluded,
                    pulse,
                )
            })
            .or_else(|| {
                let mut result = None;
                CoveDune::survey(
                    world,
                    seed,
                    origin,
                    mix(seed ^ tile ^ 0x7d94_34fe_855b_8432),
                    |center, bounds, plan| {
                        if excluded(bounds) {
                            return false;
                        }
                        let floor = world.elevation(center[0], center[1]);
                        result = Some(Self {
                            center,
                            bounds,
                            floor,
                            rim: floor,
                            shape: Shape::CoastalDune(plan),
                        });
                        true
                    },
                    pulse,
                );
                result
            })
            .or_else(|| {
                let mut result = None;
                CoastalBluff::survey(
                    world, seed, origin, mix(seed ^ tile ^ 0xf902_627a_e1ec_5c81),
                    |center, bounds, plan| {
                        if excluded(bounds) {
                            return false;
                        }
                        let floor = world.elevation(center[0], center[1]);
                        result = Some(Self { center, bounds, floor, rim: floor, shape: Shape::CoastalBluff(plan) });
                        true
                    }, pulse,
                );
                result
            })
    }

    fn generate_basin(
        world: &World,
        origin: [f32; 2],
        key: u64,
        mut excluded: impl FnMut([f32; 4]) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        if key % 5 >= 3 {
            return None;
        }
        let sweep = ((key >> 21) & 15) as usize;
        let mut surveyed = 0;
        for index in 0..16 {
            pulse();
            let candidate = (sweep + index * 7) & 15;
            let state = mix(key ^ (index as u64 + 900));
            let start = [
                origin[0] + 81.0 + (candidate % 4) as f32 * 111.0 + 16.0 * fraction(state),
                origin[1] + 81.0 + (candidate / 4) as f32 * 111.0 + 16.0 * fraction(mix(state)),
            ];
            let Some(base) = arid_floor(world, start) else {
                continue;
            };
            let (center, floor) = descend(world, start, base, pulse);
            let low = DRY_LANDMARK_TILE as f32 - EDGE_MARGIN;
            if !(EDGE_MARGIN..=low).contains(&(center[0] - origin[0]))
                || !(EDGE_MARGIN..=low).contains(&(center[1] - origin[1]))
                || arid_floor(world, center).is_none()
            {
                continue;
            }
            let rim = floor + 0.58 + fraction(mix(state ^ 0xe899)) * 0.72;
            surveyed += 1;
            if let Some(plan) = surveyed_basin(world, center, rim, pulse)
                .and_then(|basin| Self::from_basin(world, center, floor, rim, basin, state, pulse))
                .filter(|plan| !excluded(plan.bounds))
            {
                return Some(plan);
            }
            if surveyed == MAX_DRY_BASINS {
                break;
            }
        }
        None
    }

    fn from_basin(
        world: &World,
        center: [f32; 2],
        floor: f32,
        rim: f32,
        basin: Basin,
        state: u64,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        if basin.size < 32
            || basin.arid * 20 < basin.size * 19
            || basin.minimum < floor - 0.42
            || basin.max_grade > 0.23
        {
            return None;
        }
        // Verify a second higher contour, protecting a margin above the crust.
        surveyed_basin(world, center, rim + 0.33, pulse)?;
        let raw = basin.bounds;
        let bounds = [
            center[0] + grid_offset(raw[0]) - STEP,
            center[1] + grid_offset(raw[1]) - STEP,
            center[0] + grid_offset(raw[2]) + STEP,
            center[1] + grid_offset(raw[3]) + STEP,
        ];
        if (bounds[2] - bounds[0]).min(bounds[3] - bounds[1]) < 26.0 {
            return None;
        }
        let shape = if state & 3 == 0 {
            Shape::SaltPan(encode_footprint(basin.rows, false))
        } else {
            let (sine, cosine) = sin_cos(fraction(mix(state ^ 0xa902)) * core::f32::consts::TAU);
            let size = fraction(mix(state ^ 0x2bb1));
            let wind = Wind {
                axis: [cosine, sine],
                frequency: 1.0 / (22.0 + size * 8.5),
                phase: fraction(mix(state ^ 0x1185)),
                bow: if state & 8 == 0 { 0.010 } else { -0.008 },
                relief: 2.15 + size * 0.55,
            };
            Shape::Dunes(wind, encode_footprint(basin.rows, true))
        };
        Some(Self {
            center,
            bounds,
            floor,
            rim,
            shape,
        })
    }

    pub fn kind(&self) -> DryLandmarkKind {
        match self.shape {
            Shape::SaltPan(_) => DryLandmarkKind::SaltPan,
            Shape::Dunes(..) => DryLandmarkKind::Dunes,
            Shape::Badlands(_) => DryLandmarkKind::Badlands,
            Shape::CoastalDune(_) => DryLandmarkKind::CoastalDune,
            Shape::CoastalBluff(_) => DryLandmarkKind::CoastalBluff,
        }
    }

    pub fn center(&self) -> [f32; 2] {
        self.center
    }

    pub(crate) fn bluff_sampler(&self) -> Option<BluffSampler> {
        let Shape::CoastalBluff(plan) = self.shape else { return None; };
        Some(BluffSampler { center: self.center, bounds: self.bounds, plan })
    }

    /// Bounds as minimum x, minimum z, maximum x, maximum z.
    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub fn floor_elevation(&self) -> f32 {
        self.floor
    }

    /// Direction from the head of a ravine towards the dry outlet.
    pub fn downhill(&self) -> Option<[f32; 2]> {
        match self.shape {
            Shape::Badlands(ravine) => Some(ravine.axis),
            _ => None,
        }
    }

    /// Direction from a coastal landmark towards the open water.
    pub fn seaward(&self) -> Option<[f32; 2]> {
        match self.shape {
            Shape::CoastalDune(plan) => Some(plan.seaward()),
            Shape::CoastalBluff(plan) => Some(plan.seaward()),
            _ => None,
        }
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let [left, near, right, far] = self.bounds;
        if ground.elevation <= SEA_LEVEL || !(left..right).contains(&x) || !(near..far).contains(&z)
        {
            return ground;
        }
        let [dx, dz] = [x - self.center[0], z - self.center[1]];
        match &self.shape {
            Shape::Badlands(ravine) => ravine.overlay([dx, dz], ground),
            Shape::CoastalDune(plan) => plan.overlay([dx, dz], ground),
            Shape::CoastalBluff(plan) => plan.overlay([dx, dz], ground),
            _ if ground.surface != Surface::Desert => ground,
            Shape::SaltPan(footprint) => {
                let strength = Self::weight(footprint, dx, dz);
                let surface = if strength > 0.63 && ground.elevation <= self.rim {
                    Surface::SaltCrust
                } else if strength > 0.28 && ground.elevation <= self.rim + 0.10 {
                    Surface::Gravel
                } else {
                    ground.surface
                };
                Sample { surface, ..ground }
            }
            Shape::Dunes(wind, footprint) => {
                let strength = Self::weight(footprint, dx, dz);
                let across = dx * wind.axis[1] - dz * wind.axis[0];
                let along = dx * wind.axis[0] + dz * wind.axis[1];
                let distance = (along + wind.bow * across * across) * wind.frequency + wind.phase;
                let whole = distance as i32 - i32::from(distance < distance as i32 as f32);
                let phase = distance - whole as f32;
                let crest = (phase * (1.0 / 0.66)).min((1.0 - phase) * (1.0 / 0.34));
                let shoulder = smoothstep((self.rim + 0.95 - ground.elevation) * 0.85);
                let rise = wind.relief * strength * shoulder * crest * (0.75 + 0.25 * crest);
                Sample {
                    elevation: ground.elevation + rise,
                    surface: if rise > 0.30 {
                        Surface::DuneSand
                    } else {
                        ground.surface
                    },
                }
            }
        }
    }

    fn weight(footprint: &[[u32; 2]; SIDE], dx: f32, dz: f32) -> f32 {
        let [x, z] = [(dx + RADIUS) * (1.0 / STEP), (dz + RADIUS) * (1.0 / STEP)];
        if !(0.0..(SIDE - 1) as f32).contains(&x) || !(0.0..(SIDE - 1) as f32).contains(&z) {
            return 0.0;
        }
        let [i, j] = [x as usize, z as usize];
        let [fx, fz] = [x - i as f32, z - j as f32];
        let row_weight = |row: [u32; 2]| {
            let a = ((row[i >> 4] >> ((i & 15) * 2)) & 3) as f32;
            let next = i + 1;
            let b = ((row[next >> 4] >> ((next & 15) * 2)) & 3) as f32;
            a + (b - a) * fx
        };
        let near = row_weight(footprint[j]);
        let far = row_weight(footprint[j + 1]);
        (near + (far - near) * fz) * (1.0 / 3.0)
    }
}

#[derive(Clone, Copy)]
struct Basin {
    rows: [u32; SIDE],
    bounds: [usize; 4],
    size: usize,
    arid: usize,
    minimum: f32,
    max_grade: f32,
}

/// Flood uphill only to the attempted fill level. Reaching an open edge, sea,
/// or channel falsifies enclosure even if the surrounding terrain is dry.
fn surveyed_basin(
    world: &World,
    center: [f32; 2],
    rim: f32,
    pulse: &mut impl FnMut(),
) -> Option<Basin> {
    pulse();
    let mut basin = Basin {
        rows: [0; SIDE],
        bounds: [CENTER; 4],
        size: 1,
        arid: 1,
        minimum: world.elevation(center[0], center[1]),
        max_grade: 0.0,
    };
    let mut seen = [0u32; SIDE];
    let mut queue = [0u16; SIDE * SIDE];
    queue[0] = (CENTER * SIDE + CENTER) as u16;
    let mut front = 0;
    basin.rows[CENTER] = 1 << CENTER;
    seen[CENTER] = 1 << CENTER;
    while front < basin.size {
        pulse();
        let cell = usize::from(queue[front]);
        let (x, z) = (cell % SIDE, cell / SIDE);
        front += 1;
        let height = world.elevation(center[0] + grid_offset(x), center[1] + grid_offset(z));
        for j in z - 1..=z + 1 {
            for i in x - 1..=x + 1 {
                let bit = 1 << i;
                if seen[j] & bit != 0 {
                    continue;
                }
                seen[j] |= bit;
                let sample = world.sample(center[0] + grid_offset(i), center[1] + grid_offset(j));
                if sample.elevation > rim {
                    continue;
                }
                if i == 0
                    || j == 0
                    || i == SIDE - 1
                    || j == SIDE - 1
                    || sample.elevation <= SEA_LEVEL
                    || matches!(
                        sample.surface,
                        Surface::RiverWater | Surface::TarnWater | Surface::TarnShallows
                    )
                {
                    return None;
                }
                basin.rows[j] |= bit;
                basin.bounds[0] = basin.bounds[0].min(i);
                basin.bounds[1] = basin.bounds[1].min(j);
                basin.bounds[2] = basin.bounds[2].max(i);
                basin.bounds[3] = basin.bounds[3].max(j);
                basin.arid += usize::from(sample.surface == Surface::Desert);
                basin.minimum = basin.minimum.min(sample.elevation);
                let run = if i != x && j != z {
                    STEP * core::f32::consts::SQRT_2
                } else {
                    STEP
                };
                basin.max_grade = basin.max_grade.max((sample.elevation - height).abs() / run);
                queue[basin.size] = (j * SIDE + i) as u16;
                basin.size += 1;
            }
        }
    }
    Some(basin)
}

fn encode_footprint(outer: [u32; SIDE], dunes: bool) -> [[u32; 2]; SIDE] {
    let erode = |mask: [u32; SIDE]| {
        core::array::from_fn(|z: usize| {
            if z == 0 || z + 1 == SIDE {
                return 0;
            }
            let common = mask[z - 1] & mask[z] & mask[z + 1];
            common & (common << 1) & (common >> 1)
        })
    };
    let middle = if dunes { erode(outer) } else { outer };
    let inner = if dunes { erode(middle) } else { outer };
    core::array::from_fn(|z| {
        let mut row = [0u32; 2];
        for x in 0..SIDE {
            let level = ((outer[z] >> x) & 1) + ((middle[z] >> x) & 1) + ((inner[z] >> x) & 1);
            row[x >> 4] |= level << ((x & 15) * 2);
        }
        row
    })
}

fn grid_offset(cell: usize) -> f32 {
    (cell as i32 - CENTER as i32) as f32 * STEP
}

fn arid_floor(world: &World, [x, z]: [f32; 2]) -> Option<f32> {
    let (ground, coast) = world.sample_and_shore(x, z);
    (ground.surface == Surface::Desert && coast > 0.16 && LOWLAND.contains(&ground.elevation))
        .then_some(ground.elevation)
}

fn descend(
    world: &World,
    mut center: [f32; 2],
    mut height: f32,
    pulse: &mut impl FnMut(),
) -> ([f32; 2], f32) {
    for step in [22.0, 14.0, 8.0, 4.0, 2.0] {
        pulse();
        let from = center;
        for [a, b] in [
            [-1.0, -1.0],
            [0.0, -1.0],
            [1.0, -1.0],
            [-1.0, 0.0],
            [1.0, 0.0],
            [-1.0, 1.0],
            [0.0, 1.0],
            [1.0, 1.0],
        ] {
            let next = [from[0] + step * a, from[1] + step * b];
            if let Some(low) = arid_floor(world, next).filter(|&low| low < height) {
                center = next;
                height = low;
            }
        }
    }
    (center, height)
}

const RAVINE_HEIGHT: core::ops::RangeInclusive<f32> = 17.0..=59.0;
const RAVINE_AXES: [[f32; 2]; 8] = [
    [1.0, 0.0],
    [0.707_106_8, 0.707_106_8],
    [0.0, 1.0],
    [-0.707_106_8, 0.707_106_8],
    [-1.0, 0.0],
    [-0.707_106_8, -0.707_106_8],
    [0.0, -1.0],
    [0.707_106_8, -0.707_106_8],
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct Gully {
    source: f32,
    offset: f32,
    height: f32,
    outlet: f32,
    inverse_length: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Ravine {
    axis: [f32; 2],
    length: f32,
    inverse_length: f32,
    width: f32,
    inverse_width: f32,
    bend: f32,
    profile: [f32; 9],
    banks: [[u8; 2]; 9],
    layers: f32,
    tributary: Option<Gully>,
}

impl Ravine {
    fn survey(
        world: &World,
        origin: [f32; 2],
        key: u64,
        mut excluded: impl FnMut([f32; 4]) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<DryLandmark> {
        let mut proofs = 0;
        for attempt in 0..9 {
            pulse();
            let state = mix(key ^ (0x741d + attempt));
            let center = [
                origin[0] + 106.0 + 300.0 * fraction(state),
                origin[1] + 106.0 + 300.0 * fraction(state.rotate_left(26)),
            ];
            let Some(middle) = ravine_ground(world, center) else {
                continue;
            };
            let length = 104.0 + 37.0 * fraction(mix(state));
            let width = 23.0 + 8.0 * fraction(state.rotate_left(11));
            let bend = (fraction(state.rotate_left(37)) - 0.5) * 18.0;
            for i in 0..8 {
                pulse();
                let axis = RAVINE_AXES[(i + (state & 7) as usize) & 7];
                let Some(head) =
                    ravine_ground(world, ravine_point(center, axis, -0.5 * length, 0.0))
                else {
                    continue;
                };
                let Some(outlet) =
                    ravine_ground(world, ravine_point(center, axis, 0.5 * length, 0.0))
                else {
                    continue;
                };
                let drop = head - outlet;
                if !(8.0..=25.0).contains(&drop) || !(4.0..=20.0).contains(&(middle - outlet)) {
                    continue;
                }
                let early_cut = 2.5 + 2.7 * fraction(state.rotate_left(48));
                let profile = core::array::from_fn(|i| {
                    let t = i as f32 * 0.125;
                    outlet + drop * (1.0 - t) / (1.0 + early_cut * t)
                });
                let banks = core::array::from_fn(|i| {
                    let h = mix(state ^ (0x99da + i as u64));
                    [((h >> 18) & 127) as u8, ((h >> 35) & 127) as u8]
                });
                let mut ravine = Self {
                    axis,
                    length,
                    inverse_length: 1.0 / length,
                    width,
                    inverse_width: 1.0 / width,
                    bend,
                    profile,
                    banks,
                    layers: 0.5 + 2.7 * fraction(state.rotate_left(8)),
                    tributary: None,
                };
                if middle - ravine.floor(0.5) < 3.5 {
                    continue;
                }
                let mut banks = 0;
                for side in [-1.0, 1.0] {
                    for t in [0.23, 0.52, 0.76] {
                        let at = center_at(center, ravine, t, width * side * 0.76);
                        if ravine_ground(world, at).is_some_and(|h| h - ravine.floor(t) >= 4.5) {
                            banks += 1;
                        }
                    }
                }
                if banks < 4 {
                    continue;
                }
                let mut bounds = ravine.bounds(center, width + 0.50 * bend.abs());
                if !in_dry_tile(origin, bounds) || excluded(bounds) {
                    continue;
                }
                proofs += 1;
                if !ravine.prove_wash(world, center, pulse) {
                    if proofs >= 4 {
                        return None;
                    }
                    continue;
                }
                if let Some(tributary) = ravine.fit_tributary(world, center, state, pulse) {
                    let extended =
                        ravine.bounds(center, tributary.offset.abs() + 7.0 + 0.50 * bend.abs());
                    if in_dry_tile(origin, extended) && !excluded(extended) {
                        ravine.tributary = Some(tributary);
                        bounds = extended;
                    } else if excluded(bounds) {
                        continue;
                    }
                }
                return Some(DryLandmark {
                    center,
                    bounds,
                    floor: ravine.floor(0.5),
                    rim: head,
                    shape: Shape::Badlands(ravine),
                });
            }
        }
        None
    }

    fn offset(&self, t: f32) -> f32 {
        self.bend * (4.0 * t * (1.0 - t)) * (2.0 * t - 1.0)
    }

    fn floor(&self, t: f32) -> f32 {
        let progress = t * 8.0;
        let i = (progress as usize).min(7);
        let base = self.profile[i];
        base + (self.profile[i + 1] - base) * (progress - i as f32)
    }

    fn bounds(self, center: [f32; 2], width: f32) -> [f32; 4] {
        let [x, z] = center;
        let [dx, dz] = self.axis.map(f32::abs);
        let rx = 0.5 * self.length * dx + width * dz;
        let rz = 0.5 * self.length * dz + width * dx;
        [x - rx, z - rz, x + rx, z + rz]
    }

    fn prove_wash(self, world: &World, center: [f32; 2], pulse: &mut impl FnMut()) -> bool {
        let steps = (self.length * 0.67) as usize + 1;
        let mut last = self.profile[0];
        let mut lowest = last;
        let mut deepest = 0.0_f32;
        for i in 0..=steps {
            if i & 3 == 0 {
                pulse();
            }
            let t = i as f32 / steps as f32;
            let Some(h) = ravine_ground(world, center_at(center, self, t, 0.0)) else {
                return false;
            };
            let cut = (h - self.floor(t)).clamp(0.0, 13.0) * end_fade(t);
            let bed = h - cut;
            if bed > last + 0.10 || bed > lowest + 0.16 {
                return false;
            }
            deepest = deepest.max(cut);
            lowest = lowest.min(bed);
            last = bed;
            if i % 3 == 0 {
                for width in [-1.0, -0.75, -0.50, -0.25, 0.25, 0.50, 0.75, 1.0] {
                    pulse();
                    if ravine_ground(world, center_at(center, self, t, self.width * width))
                        .is_none()
                    {
                        return false;
                    }
                }
            }
        }
        for distance in [2.0, 5.0, 9.0, 15.0, 22.0] {
            pulse();
            let at = ravine_point(center, self.axis, 0.5 * self.length + distance, 0.0);
            let Some(h) = ravine_ground(world, at) else {
                return false;
            };
            if h > last + 0.13 {
                return false;
            }
            last = h;
        }
        deepest >= 4.5 && self.floor(1.0) - last >= 0.6
    }

    fn fit_tributary(
        self,
        world: &World,
        center: [f32; 2],
        key: u64,
        pulse: &mut impl FnMut(),
    ) -> Option<Gully> {
        let join = 0.54 + 0.13 * fraction(key.rotate_left(14));
        let source = join - 0.31;
        let first = if key & (1 << 12) == 0 { -1.0 } else { 1.0 };
        for side in [first, -first] {
            let offset = side * (self.width + 2.6);
            let Some(height) = ravine_ground(world, center_at(center, self, source, offset)) else {
                continue;
            };
            let gully = Gully {
                source,
                offset,
                height,
                outlet: self.floor(join),
                inverse_length: 1.0 / (join - source),
            };
            let mut last = height;
            let mut deepest = 0.0_f32;
            let mut valid = true;
            for i in 0..=30 {
                pulse();
                let s = i as f32 * (1.0 / 30.0);
                let t = source + s * (join - source);
                let cross = gully.cross(s);
                let Some(h) = ravine_ground(world, center_at(center, self, t, cross)) else {
                    valid = false;
                    break;
                };
                let main = self.overlay_at(t, cross, h, None);
                let bed = self.overlay_at(t, cross, h, Some(gully));
                if bed > last + 0.06 {
                    valid = false;
                    break;
                }
                if i & 3 == 0
                    && [-7.0, 7.0].into_iter().any(|margin| {
                        ravine_ground(world, center_at(center, self, t, cross + margin)).is_none()
                    })
                {
                    valid = false;
                    break;
                }
                deepest = deepest.max(main - bed);
                last = bed;
            }
            if valid && deepest >= 1.6 {
                return Some(gully);
            }
        }
        None
    }

    fn overlay(&self, [dx, dz]: [f32; 2], ground: Sample) -> Sample {
        if !matches!(
            ground.surface,
            Surface::Desert | Surface::Grass | Surface::Rock
        ) || !(SEA_LEVEL + 8.0..=64.0).contains(&ground.elevation)
        {
            return ground;
        }
        let [ax, az] = self.axis;
        let t = (dx * ax + dz * az) * self.inverse_length + 0.5;
        if !(0.0..1.0).contains(&t) {
            return ground;
        }
        let cross = dz * ax - dx * az - self.offset(t);
        let outer = self.width + if self.tributary.is_some() { 11.1 } else { 0.0 };
        if cross.abs() >= outer {
            return ground;
        }
        let elevation = self.overlay_at(t, cross, ground.elevation, self.tributary);
        let relief = ground.elevation - elevation;
        if relief <= 0.0 {
            return ground;
        }
        let surface = if relief < 1.05 {
            ground.surface
        } else if cross.abs() < self.width * 0.26 && end_fade(t) > 0.48 {
            Surface::DryWash
        } else {
            let layer = ((elevation + self.layers) * 0.31) as i32;
            match layer.rem_euclid(5) {
                0 => Surface::Gravel,
                1 | 2 => Surface::DuneSand,
                _ => Surface::Desert,
            }
        };
        Sample { elevation, surface }
    }

    fn overlay_at(&self, t: f32, cross: f32, ground: f32, branch: Option<Gully>) -> f32 {
        let progress = t * 8.0;
        let i = (progress as usize).min(7);
        let fraction = progress - i as f32;
        let floor = self.profile[i];
        let center_floor = floor + (self.profile[i + 1] - floor) * fraction;
        let side = usize::from(cross >= 0.0);
        let a = f32::from(self.banks[i][side]);
        let b = f32::from(self.banks[i + 1][side]);
        let taper = a + (b - a) * smoothstep(fraction);
        let sides = (cross.abs() * self.inverse_width) * (1.01 + taper * (0.51 / 127.0));
        let fade = end_fade(t);
        let end = fade * (1.0 - 0.55 * sides.min(1.0) * (1.0 - fade));
        let inner = smoothstep((0.48 - sides) * 3.1);
        let outer = smoothstep((1.0 - sides) * 2.25);
        let wash = (0.73 * inner + 0.27 * outer) * end;
        let elevation = ground - (ground - center_floor).clamp(0.0, 13.0) * wash;
        let Some(branch) = branch else {
            return elevation;
        };
        let s = (t - branch.source) * branch.inverse_length;
        if !(0.0..1.0).contains(&s) {
            return elevation;
        }
        let cross = (cross - branch.cross(s)).abs();
        if cross >= 7.0 {
            return elevation;
        }
        let floor = branch.height + (branch.outlet - branch.height) * s;
        let fade = smoothstep(s * 6.0) * smoothstep((1.0 - s) * 6.0);
        let cut = (ground - floor.max(center_floor)).clamp(0.0, 8.0);
        elevation.min(ground - cut * smoothstep((7.0 - cross) * 0.19) * fade)
    }
}

impl Gully {
    fn cross(self, t: f32) -> f32 {
        self.offset * (1.0 - t)
    }
}

fn end_fade(t: f32) -> f32 {
    smoothstep(t * 6.0) * smoothstep((1.0 - t) * 7.5)
}

fn center_at(center: [f32; 2], ravine: Ravine, t: f32, cross: f32) -> [f32; 2] {
    ravine_point(
        center,
        ravine.axis,
        (t - 0.5) * ravine.length,
        ravine.offset(t) + cross,
    )
}

fn ravine_point([x, z]: [f32; 2], [ax, az]: [f32; 2], along: f32, across: f32) -> [f32; 2] {
    [x + ax * along - az * across, z + az * along + ax * across]
}

fn ravine_ground(world: &World, [x, z]: [f32; 2]) -> Option<f32> {
    let base = world.sample(x, z);
    (base.surface == Surface::Desert && RAVINE_HEIGHT.contains(&base.elevation))
        .then_some(base.elevation)
}

fn in_dry_tile([x, z]: [f32; 2], [left, near, right, far]: [f32; 4]) -> bool {
    let margin = 5.0;
    let upper = DRY_LANDMARK_TILE as f32 - margin;
    x + margin <= left && z + margin <= near && right <= x + upper && far <= z + upper
}

#[cfg(test)]
mod tests {

    #[test]
    fn forested_cove_dunes_keep_existing_water_and_a_contiguous_dry_crest() {
        for (seed, tile) in [(1, [3, -11]), (1, [6, -5]), (2, [-7, -8])] {
            let world = World::new(seed);
            let dune = DryLandmark::generate(&world, seed, tile[0], tile[1]).unwrap();
            let Shape::CoastalDune(shape) = dune.shape else { panic!("expected a coastal dune") };
            let [left, near, right, far] = dune.bounds();
            let mut dry = 0;
            let mut raised = 0.0_f32;
            let mut crest = 0;
            for row in -10..=((far - near) * 2.0) as i32 + 10 {
                for col in -10..=((right - left) * 2.0) as i32 + 10 {
                    let x = left + col as f32 * 0.5;
                    let z = near + row as f32 * 0.5;
                    let before = world.sample(x, z);
                    let after = dune.overlay(x, z, before);
                    if after != before {
                        dry += 1;
                        assert!(before.elevation >= SEA_LEVEL + 0.65);
                        assert!(after.elevation >= before.elevation);
                        raised = raised.max(after.elevation - before.elevation);
                    }
                    crest += usize::from(after.surface == Surface::DuneSand);
                    if matches!(before.surface, Surface::Shallows | Surface::DeepWater)
                        || x <= left || x >= right || z <= near || z >= far
                    {
                        assert_eq!(after, before);
                    }
                }
            }
            assert!(dry > 5000 && raised > 1.7 && raised <= 3.8 && crest > 600);
            let sea = shape.seaward();
            let [x, z] = dune.center();
            let shoreline_dry = world.sample(x + sea[0] * 20.0, z + sea[1] * 20.0);
            assert!(shoreline_dry.elevation > SEA_LEVEL);
            let shore = world.sample(x + sea[0] * 56.0, z + sea[1] * 56.0);
            assert_eq!(shore.surface, Surface::Shallows);
        }
    }

    use super::*;

    const FIXTURES: [(i32, i32, DryLandmarkKind); 2] = [
        (-2, 11, DryLandmarkKind::SaltPan),
        (-6, -13, DryLandmarkKind::Dunes),
    ];

    #[test]
    fn badlands_follow_the_dry_outlet_and_preserve_their_borders() {
        for (seed, tx, tz) in [(1, 3, 9), (2, -6, 4)] {
            let world = World::new(seed);
            let site = DryLandmark::generate(&world, seed, tx, tz).unwrap();
            let Shape::Badlands(ravine) = site.shape else {
                panic!("expected a ravine");
            };
            assert_eq!(DryLandmark::generate(&world, seed, tx, tz), Some(site));
            assert!(DryLandmark::generate_with(&world, seed, tx, tz, |_| true).is_none());
            assert!(in_dry_tile(
                [
                    (tx * DRY_LANDMARK_TILE) as f32,
                    (tz * DRY_LANDMARK_TILE) as f32
                ],
                site.bounds()
            ));
            let mut lowest = f32::INFINITY;
            let mut deepest = 0.0_f32;
            let mut wash_floor = 0;
            for i in 0..=480 {
                let t = i as f32 / 480.0;
                let [x, z] = center_at(site.center, ravine, t, 0.0);
                let base = world.sample(x, z);
                let grade = site.overlay(x, z, base);
                assert_eq!(base.surface, Surface::Desert);
                assert!(grade.elevation <= base.elevation && grade.elevation > SEA_LEVEL + 5.0);
                assert!(grade.elevation <= lowest + 0.18);
                lowest = lowest.min(grade.elevation);
                deepest = deepest.max(base.elevation - grade.elevation);
                if grade.surface == Surface::DryWash {
                    assert!(base.elevation - grade.elevation >= 1.05);
                    wash_floor += 1;
                }
                if i == 0 || i == 480 {
                    assert_eq!(grade, base);
                }
            }
            assert!(deepest >= 7.0 && deepest <= 13.001);
            assert!(wash_floor > 180);
            let [x, z] = ravine_point(site.center, ravine.axis, 0.5 * ravine.length + 22.0, 0.0);
            let outlet = world.sample(x, z);
            assert_eq!(site.overlay(x, z, outlet), outlet);
            assert!(outlet.elevation + 0.45 < lowest);

            for surface in [
                Surface::Snow,
                Surface::DeepWater,
                Surface::RiverWater,
                Surface::Pavement,
                Surface::FieldSoil,
            ] {
                let base = Sample {
                    elevation: site.floor + 4.0,
                    surface,
                };
                assert_eq!(site.overlay(site.center[0], site.center[1], base), base);
            }
            let [a, b, c, d] = site.bounds();
            for i in 0..=40 {
                let t = i as f32 / 40.0;
                for (x, z) in [
                    (a - 0.002, b + t * (d - b)),
                    (c + 0.002, b + t * (d - b)),
                    (a + t * (c - a), b - 0.002),
                    (a + t * (c - a), d + 0.002),
                ] {
                    let base = world.sample(x, z);
                    assert_eq!(site.overlay(x, z, base), base);
                }
            }
        }
    }

    #[test]
    fn seeded_sites_keep_their_ground_and_respect_reservations() {
        let world = World::new(0);
        for (tx, tz, kind) in FIXTURES {
            let site = DryLandmark::generate(&world, 0, tx, tz).unwrap();
            assert_eq!(site.kind(), kind);
            assert_eq!(DryLandmark::generate(&world, 0, tx, tz), Some(site));
            assert!(DryLandmark::generate_with(&world, 0, tx, tz, |_| true).is_none());
            let [left, near, right, far] = site.bounds();
            let mut affected = 0;
            for z in near as i32..far as i32 {
                for x in left as i32..right as i32 {
                    let (x, z) = (x as f32, z as f32);
                    let ground = world.sample(x, z);
                    let sample = site.overlay(x, z, ground);
                    if sample != ground {
                        assert_eq!(ground.surface, Surface::Desert);
                        let rise = sample.elevation - ground.elevation;
                        if kind == DryLandmarkKind::SaltPan {
                            assert_eq!(rise, 0.0);
                        } else {
                            assert!((0.0..=2.70).contains(&rise));
                        }
                        affected += 1;
                    }
                }
            }
            assert!(affected >= 500);
            let [x, z] = site.center();
            for surface in [Surface::Pavement, Surface::RiverWater, Surface::FieldSoil] {
                let existing = Sample {
                    elevation: site.floor,
                    surface,
                };
                assert_eq!(site.overlay(x, z, existing), existing);
            }
        }
    }

    #[test]
    fn both_native_fixtures_remain_enclosed_on_a_finer_grid() {
        const CHECK_SIDE: usize = 61;
        const HALF: usize = CHECK_SIDE / 2;
        let world = World::new(0);
        for (tx, tz, _) in FIXTURES {
            let site = DryLandmark::generate(&world, 0, tx, tz).unwrap();
            let [x, z] = site.center();
            let mut seen = [0u64; CHECK_SIDE];
            let mut queue = [0u16; CHECK_SIDE * CHECK_SIDE];
            queue[0] = (HALF * CHECK_SIDE + HALF) as u16;
            seen[HALF] |= 1 << HALF;
            let mut count = 1;
            let mut front = 0;
            while front < count {
                let cell = usize::from(queue[front]);
                front += 1;
                let (cx, cz) = (cell % CHECK_SIDE, cell / CHECK_SIDE);
                for j in cz - 1..=cz + 1 {
                    for i in cx - 1..=cx + 1 {
                        if seen[j] & (1 << i) != 0 {
                            continue;
                        }
                        seen[j] |= 1 << i;
                        let a = x + (i as f32 - HALF as f32) * 2.0;
                        let b = z + (j as f32 - HALF as f32) * 2.0;
                        if world.elevation(a, b) <= site.rim + 0.10 {
                            assert!(i > 0 && j > 0 && i < CHECK_SIDE - 1 && j < CHECK_SIDE - 1);
                            queue[count] = (j * CHECK_SIDE + i) as u16;
                            count += 1;
                        }
                    }
                }
            }
            assert!(count > 120);
        }
    }
}
