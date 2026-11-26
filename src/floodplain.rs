//! Small prepared low terraces, anchored to measured river reaches.

use crate::math::{mix, smoothstep};
use crate::point_bars::PointBar;
use crate::river_islands::{IslandProfile, RiverIsland};
use crate::watershed::{Reach, RiverSample, Watershed, REGION_SIZE};
use crate::world::{Sample, Surface, World, SEA_LEVEL};

mod oxbow;

type Area = [f32; 4];

const MAX_SITES: usize = 6;
const MAX_TRIALS: u8 = 24;
const PROBE_STEP: f32 = 1.15;
const MAX_RISE: f32 = 4.9;
const MIN_RISE: f32 = 0.30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloodplainKind {
    WetMeadow,
    ReedMarsh,
    PointBar,
    RiverIsland,
    OxbowLake,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Site {
    center: [f32; 2],
    axis: [f32; 2],
    inverse_size: [f32; 2],
    curve: f32,
    water_level: f32,
    level_slope: f32,
    bounds: Area,
    kind: FloodplainKind,
}

/// A few contiguous terraces whose survey is completed before publication.
pub struct Floodplain {
    sites: [Option<Site>; MAX_SITES],
    count: usize,
    bounds: Option<Area>,
    bar: Option<PointBar>,
    island: Option<RiverIsland>,
}

impl Floodplain {
    pub const fn new() -> Self {
        Self { sites: [None; MAX_SITES], count: 0, bounds: None, bar: None, island: None }
    }

    pub(crate) fn prepare(
        world: &World,
        water: &Watershed,
        mut reserved: impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Self {
        let mut result = Self::new();
        if let Some(oxbow) = oxbow::survey(world, water, &mut reserved, pulse) {
            result.bounds = Some(oxbow.bounds);
            result.sites[0] = Some(oxbow);
            result.count = 1;
        }
        let mut trials = 0;
        for reach in water.reaches() {
            pulse();
            if trials == MAX_TRIALS || result.count == MAX_SITES {
                break;
            }
            if reach.reaches_sea {
                continue;
            }
            let key = reach_key(reach);
            let first = if key & 1 == 0 { -1.0 } else { 1.0 };
            for side in [first, -first] {
                let Some(mut site) = Site::near_reach(world, water, reach, side, key) else {
                    continue;
                };
                if result.intersects(site.bounds) {
                    continue;
                }
                trials += 1;
                if site.prove(world, water, &mut reserved, pulse) {
                    result.bounds = Some(result.bounds.map_or(site.bounds, |a| union(a, site.bounds)));
                    result.sites[result.count] = Some(site);
                    result.count += 1;
                    break;
                }
                if trials == MAX_TRIALS {
                    break;
                }
            }
        }
        if result.count < MAX_SITES {
            result.bar = PointBar::survey(world, water, &mut reserved, |area| result.intersects(area), pulse);
        }
        result.island = RiverIsland::survey(world, water, &mut reserved, |area| result.intersects(area), pulse);
        result
    }

    pub fn bounds(&self) -> Option<Area> {
        [self.bounds, self.bar_bounds(), self.island_bounds()].into_iter().flatten().reduce(union)
    }

    pub(crate) fn wetland_bounds(&self) -> Option<Area> {
        self.bounds
    }

    pub(crate) fn bar_bounds(&self) -> Option<Area> {
        self.bar.as_ref().map(PointBar::bounds)
    }

    pub(crate) fn island_bounds(&self) -> Option<Area> {
        self.island.as_ref().map(RiverIsland::bounds)
    }

    pub fn island_profile(&self) -> Option<(IslandProfile, [f32; 2])> {
        self.island.as_ref().map(|site| (site.profile(), site.center()))
    }

    pub fn sites(&self) -> impl Iterator<Item = (FloodplainKind, Area)> + '_ {
        self.sites[..self.count].iter().flatten().map(|site| (site.kind, site.bounds))
            .chain(self.bar_bounds().map(|bounds| (FloodplainKind::PointBar, bounds)))
            .chain(self.island_bounds().map(|bounds| (FloodplainKind::RiverIsland, bounds)))
    }

    pub fn intersects(&self, area: Area) -> bool {
        self.bar_bounds().is_some_and(|bounds| overlaps(bounds, area))
            || self.island_bounds().is_some_and(|bounds| overlaps(bounds, area))
            || (self.bounds.is_some_and(|bounds| overlaps(bounds, area))
                && self.sites[..self.count].iter().flatten().any(|site| overlaps(site.bounds, area)))
    }

    pub(crate) fn overlay_point_bar(
        &self, x: f32, z: f32, developed: Sample, bank: Sample, water: Option<RiverSample>,
    ) -> Sample {
        self.bar.as_ref().map_or(bank, |bar| bar.overlay(x, z, developed, bank, water))
    }

    pub(crate) fn overlay_island(&self, x: f32, z: f32, developed: Sample, covered: Sample) -> Sample {
        self.island.as_ref().map_or(covered, |island| island.overlay(x, z, developed, covered))
    }

    pub(crate) fn overlay(
        &self, x: f32, z: f32, ground: Sample, water: Option<RiverSample>,
    ) -> Sample {
        if !self.bounds.is_some_and(|area| contains(area, x, z))
            || !natural_cover(ground.surface)
            || water.is_some_and(RiverSample::is_channel)
            || ground.elevation < SEA_LEVEL + MIN_RISE
        {
            return ground;
        }
        // The original gravel strip starts at strength 0.63. Ease the
        // terrace grade back to the old bank profile before that boundary.
        let bank_grade = water.map_or(1.0, |sample| {
            1.0 - smoothstep((sample.bank_strength - 0.34) * (1.0 / 0.27))
        });
        self.sites[..self.count]
            .iter()
            .flatten()
            .find_map(|site| site.overlay(x, z, ground, bank_grade))
            .unwrap_or(ground)
    }
}

impl Default for Floodplain {
    fn default() -> Self {
        Self::new()
    }
}

impl Site {
    fn near_reach(world: &World, water: &Watershed, reach: Reach, side: f32, key: u64) -> Option<Self> {
        let middle = reach.position(0.5);
        let a = reach.position(0.125);
        let b = reach.position(0.875);
        let length = length(b[0] - a[0], b[1] - a[1]);
        if length < 18.0 {
            return None;
        }
        let river = water.sample(middle[0], middle[1])?;
        if !river.is_channel() {
            return None;
        }
        let axis = [(b[0] - a[0]) / length, (b[1] - a[1]) / length];
        let half_length = 18.0 + ((key >> 5) & 7) as f32;
        let half_width = 6.8 + ((key >> 9) & 3) as f32;
        let offset = side * (river.half_width + half_width + 1.6);
        let center = [middle[0] - axis[1] * offset, middle[1] + axis[0] * offset];
        let ground = world.sample(center[0], center[1]);
        let rise = ground.elevation - river.level;
        if !natural_cover(ground.surface) || !(MIN_RISE..=MAX_RISE - 0.6).contains(&rise) {
            return None;
        }
        let curve = (((key >> 14) & 7) as f32 - 3.5) * 0.045;
        let normal_extent = half_width * (1.0 + curve.abs());
        let x = axis[0].abs() * half_length + axis[1].abs() * normal_extent;
        let z = axis[1].abs() * half_length + axis[0].abs() * normal_extent;
        let bounds = [center[0] - x, center[1] - z, center[0] + x, center[1] + z];
        let region = reach.start.map(region_origin);
        if bounds[0] < region[0] || bounds[1] < region[1]
            || bounds[2] >= region[0] + REGION_SIZE as f32
            || bounds[3] >= region[1] + REGION_SIZE as f32
        {
            return None;
        }
        let level_slope = ((reach.end_level - reach.start_level) * half_length
            / (length / 0.75)).clamp(-1.4, 1.4);
        Some(Self {
            center,
            axis,
            inverse_size: [1.0 / half_length, 1.0 / half_width],
            curve,
            water_level: river.level,
            level_slope,
            bounds,
            kind: if rise <= 2.65 && (key & 4 != 0 || rise <= 1.75) {
                FloodplainKind::ReedMarsh
            } else {
                FloodplainKind::WetMeadow
            },
        })
    }

    fn point(&self, along: f32, across: f32) -> [f32; 2] {
        let u = along / self.inverse_size[0];
        let v = (across + self.curve * along * along) / self.inverse_size[1];
        [
            self.center[0] + self.axis[0] * u - self.axis[1] * v,
            self.center[1] + self.axis[1] * u + self.axis[0] * v,
        ]
    }

    fn level(&self, along: f32) -> f32 {
        (self.water_level + self.level_slope * along).max(SEA_LEVEL)
    }

    fn prove(
        &mut self,
        world: &World,
        water: &Watershed,
        reserved: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        let rows = (2.0 / (self.inverse_size[0] * PROBE_STEP)) as usize + 1;
        let cols = (2.0 / (self.inverse_size[1] * PROBE_STEP)) as usize + 1;
        let mut low_core = f32::INFINITY;
        let mut high_core = f32::NEG_INFINITY;
        for row in 0..=rows {
            pulse();
            let u = 2.0 * row as f32 / rows as f32 - 1.0;
            for col in 0..=cols {
                let v = 2.0 * col as f32 / cols as f32 - 1.0;
                let footprint = u * u + v * v;
                if footprint >= 1.0 {
                    continue;
                }
                let [x, z] = self.point(u, v);
                let ground = world.elevation(x, z);
                let channel = water.sample(x, z);
                if ground <= SEA_LEVEL + 0.20
                    || channel.is_some_and(RiverSample::is_channel)
                    || reserved(x, z)
                {
                    return false;
                }
                let rise = ground - self.level(u);
                let max_rise = if footprint < 0.58 { MAX_RISE - 0.45 } else { MAX_RISE + 0.7 };
                if !(MIN_RISE..=max_rise).contains(&rise) {
                    return false;
                }
                if footprint < 0.38 {
                    low_core = low_core.min(rise);
                    high_core = high_core.max(rise);
                }
            }
        }
        if high_core - low_core > 2.4 {
            return false;
        }
        if high_core > 3.15 {
            self.kind = FloodplainKind::WetMeadow;
        }
        true
    }

    fn overlay(&self, x: f32, z: f32, ground: Sample, bank_grade: f32) -> Option<Sample> {
        if !contains(self.bounds, x, z) {
            return None;
        }
        let dx = x - self.center[0];
        let dz = z - self.center[1];
        let along = (dx * self.axis[0] + dz * self.axis[1]) * self.inverse_size[0];
        let across = (dz * self.axis[0] - dx * self.axis[1]) * self.inverse_size[1];
        if self.kind == FloodplainKind::OxbowLake {
            return oxbow::overlay(*self, along, across, ground);
        }
        let across = across - self.curve * along * along;
        let footprint = along * along + across * across;
        if footprint >= 1.0 {
            return None;
        }
        let rise = ground.elevation - self.level(along);
        if !(MIN_RISE..MAX_RISE).contains(&rise) {
            return None;
        }
        let fade = smoothstep((1.0 - footprint) * 2.6)
            * smoothstep((rise - MIN_RISE) * 2.5)
            * smoothstep((MAX_RISE - rise) * 0.85);
        let elevation = ground.elevation - (rise - 0.75).clamp(0.0, 0.82) * fade * bank_grade;
        let surface = if fade >= 0.46 && self.kind == FloodplainKind::ReedMarsh && footprint < 0.62 {
            Surface::ReedMarsh
        } else if fade >= 0.27 {
            Surface::WetMeadow
        } else {
            ground.surface
        };
        Some(Sample { elevation, surface })
    }
}

fn natural_cover(surface: Surface) -> bool {
    matches!(surface, Surface::Grass | Surface::Sand | Surface::Forest | Surface::WetMeadow)
}

fn reach_key(reach: Reach) -> u64 {
    let [x, z] = reach.start;
    mix(u64::from(x.to_bits()) | (u64::from(z.to_bits()) << 32))
}

fn region_origin(value: f32) -> f32 {
    let scaled = value / REGION_SIZE as f32;
    let whole = scaled as i32;
    ((whole - i32::from(scaled < whole as f32)) * REGION_SIZE) as f32
}

fn contains(area: Area, x: f32, z: f32) -> bool {
    area[0] <= x && x <= area[2] && area[1] <= z && z <= area[3]
}

fn overlaps(a: Area, b: Area) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

fn union(a: Area, b: Area) -> Area {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

fn length(x: f32, z: f32) -> f32 {
    let squared = x * x + z * z;
    let mut root = x.abs().max(z.abs());
    if root == 0.0 {
        return root;
    }
    for _ in 0..3 {
        root = (root + squared / root) * 0.5;
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::river_district::RiverDistrict;
    use crate::watershed::WatershedWork;

    #[test]
    fn bank_grade_blends_before_the_gravel_boundary() {
        let seed = 0xf7ef_2732_41e5_45f7;
        let world = World::new(seed);
        let mut water = Watershed::new();
        water.rebuild(&world, seed, -1, -3, &mut WatershedWork::new());
        let plan = RiverDistrict::from_prepared_land_cooperatively(
            &world, None, water, |_, _| false, &mut || {},
        ).unwrap();
        let z = -2733.4358;
        let terrace_x = -518.9207;
        let gravel_x = -518.4207;
        let old_terrace = world.sample(terrace_x, z);
        let old_gravel = world.sample(gravel_x, z);
        let terrace = plan.overlay(terrace_x, z, old_terrace);
        let gravel = plan.overlay(gravel_x, z, old_gravel);
        assert_eq!(terrace.surface, Surface::WetMeadow);
        assert_eq!(gravel, Sample { surface: Surface::Gravel, ..old_gravel });
        let bank_rise = gravel.elevation - terrace.elevation;
        assert!((0.0..=0.08).contains(&bank_rise), "{bank_rise}");

        let bank_x = -518.53;
        let raw = world.sample(bank_x, z);
        let bank = plan.watershed().sample(bank_x, z).unwrap();
        assert!((0.61..0.63).contains(&bank.bank_strength));
        assert_eq!(plan.overlay(bank_x, z, raw), Sample { surface: Surface::WetMeadow, ..raw });

        let mut gravel_count = 0;
        let mut wet_count = 0;
        for step in 0..75 {
            let x = gravel_x + step as f32 * 0.125;
            let raw = world.sample(x, z);
            let water = plan.watershed().sample(x, z).unwrap();
            let after = plan.overlay(x, z, raw);
            if water.is_channel() {
                assert_eq!(after, Sample { surface: Surface::RiverWater, elevation: water.level });
                wet_count += 1;
            } else if water.bank_strength >= 0.63 && (0.0..2.0).contains(&(raw.elevation - water.level)) {
                assert_eq!(after, Sample { surface: Surface::Gravel, ..raw });
                gravel_count += 1;
            }
        }
        assert!(gravel_count > 10 && wet_count > 10);
    }

    #[test]
    fn terraces_protect_the_existing_water_and_reserved_footprint() {
        let world = World::new(14_763);
        let mut water = Watershed::new();
        water.rebuild(&world, 14_763, 2, 0, &mut WatershedWork::new());
        let flood = Floodplain::prepare(&world, &water, |_, _| false, &mut || {});
        let reed = flood.sites[..flood.count].iter().flatten()
            .find(|site| site.kind == FloodplainKind::ReedMarsh).unwrap();
        let [x, z] = reed.center;
        let land = world.sample(x, z);
        let channel = water.sample(x, z);
        assert_eq!(flood.overlay(x, z, land, channel).surface, Surface::ReedMarsh);
        let restricted = Floodplain::prepare(&world, &water, |px, pz| {
            (x - px).abs() < 2.0 && (z - pz).abs() < 2.0
        }, &mut || {});
        assert_eq!(restricted.overlay(x, z, land, channel), land);

        for surface in [Surface::Pavement, Surface::Gravel, Surface::PitchedRoof, Surface::GrassRunway] {
            let occupied = Sample { surface, ..land };
            assert_eq!(flood.overlay(x, z, occupied, channel), occupied);
        }
        for site in flood.sites[..flood.count].iter().flatten() {
            let area = site.bounds;
            for row in 0..=60 {
                for col in 0..=80 {
                    let x = area[0] + (area[2] - area[0]) * col as f32 / 80.0;
                    let z = area[1] + (area[3] - area[1]) * row as f32 / 60.0;
                    let ground = world.sample(x, z);
                    let water = water.sample(x, z);
                    let sample = flood.overlay(x, z, ground, water);
                    if ground.elevation <= SEA_LEVEL || water.is_some_and(RiverSample::is_channel) {
                        assert_eq!(sample, ground);
                    }
                    assert!(sample.elevation <= ground.elevation);
                    assert!(ground.elevation - sample.elevation <= 0.821);
                    if sample != ground {
                        assert!(sample.elevation > SEA_LEVEL);
                    }
                }
            }
        }
    }
}
