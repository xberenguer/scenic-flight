//! Surveyed roadside overlooks, woodland trailheads and rest stops.

use crate::ground_details::ParcelSurvey;
use crate::math::{fraction, mix, smoothstep};
use crate::roads::{RoadNetwork, ROAD_HALF_WIDTH, ROAD_REGION_SIZE};
use crate::world::{Sample, Surface, World};

mod buildings;
mod fixtures;
pub use buildings::RoadsideBuilding;
use fixtures::RoadsideFixture;

const INNER: f32 = ROAD_HALF_WIDTH - 0.22;
const WALL_HALF: f32 = 0.84;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnoutKind {
    Overlook,
    Trailhead,
    RestStop,
}

#[derive(Clone, Copy, Debug)]
pub struct ScenicTurnout {
    center: [f32; 3],
    axis: [f32; 2],
    side: f32,
    grade: f32,
    half_length: f32,
    inverse_half_length: f32,
    width: f32,
    wall: Surface,
    kind: TurnoutKind,
    variant: u8,
    bounds: [f32; 4],
}

impl ScenicTurnout {
    pub(crate) fn survey(
        world: &World,
        seed: u64,
        region: [i32; 2],
        road: &RoadNetwork,
        mut reserved: impl FnMut(f32, f32) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        let tile = (u64::from(region[0] as u32) << 32) | u64::from(region[1] as u32);
        let key = mix(seed ^ tile ^ 0xb793_d280_e4f1_7bd6);
        let mut survey = |overlook| {
            Self::survey_route(world, seed, region, road, key, overlook, &mut reserved, &mut pulse)
        };
        let viewing_area = (key & 3 == 0).then(|| survey(true)).flatten();
        viewing_area.or_else(|| (key & 3 <= 1).then(|| survey(false)).flatten())
    }

    fn survey_route(
        world: &World,
        seed: u64,
        region: [i32; 2],
        road: &RoadNetwork,
        key: u64,
        overlook: bool,
        reserved: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let points = road.polyline();
        let count = points.len().checked_sub(6)?;
        let origin = region.map(|axis| axis as f32 * ROAD_REGION_SIZE as f32);
        let mut parcels = ParcelSurvey::new();
        for candidate in 0..count.min(14) {
            pulse();
            let index = 3 + ((key >> 21) as usize + candidate) % count;
            let [a, b] = [points[index - 1], points[index]];
            let dx = b.x - a.x;
            let dz = b.z - a.z;
            let length = distance(dx, dz);
            if length < 20.0 || (a.elevation - b.elevation).abs() > length * 0.082 {
                continue;
            }
            let [x, z] = [(a.x + b.x) * 0.5, (a.z + b.z) * 0.5];
            let h = world.elevation(x, z);
            let minimum = if overlook { 13.0 } else { 7.5 };
            if !(minimum..=64.0).contains(&h) {
                continue;
            }
            let [ax, az] = [dx / length, dz / length];
            let half_length = (length * 0.46).min(16.0);
            let layout = mix(key ^ index as u64);
            let width = if overlook { 7.1 } else { 9.4 } + 2.8 * fraction(layout);
            let wall = if key & 16 == 0 {
                Surface::Rock
            } else {
                Surface::ScreeCoarse
            };
            for side in [-1.0, 1.0] {
                let downhill = |span| world.elevation(x - az * side * span, z + ax * side * span);
                let kind = if overlook {
                    let near = downhill(30.0);
                    let far = downhill(76.0);
                    if h - near < 2.2 || h - far < 8.0 || far > near - 2.0 {
                        continue;
                    }
                    TurnoutKind::Overlook
                } else {
                    let terrain = world.sample(x - az * side * 13.0, z + ax * side * 13.0);
                    if (h - terrain.elevation).abs() > 2.8 {
                        continue;
                    }
                    match terrain.surface {
                        Surface::Forest => TurnoutKind::Trailhead,
                        Surface::Grass | Surface::Rock | Surface::Desert => TurnoutKind::RestStop,
                        _ => continue,
                    }
                };
                let mut plan = Self {
                    center: [x, h, z],
                    axis: [ax, az],
                    side,
                    grade: (b.elevation - a.elevation) / length,
                    half_length,
                    inverse_half_length: 1.0 / half_length,
                    width,
                    wall,
                    kind,
                    variant: ((layout >> 30) % 250) as u8,
                    bounds: [
                        f32::INFINITY,
                        f32::INFINITY,
                        f32::NEG_INFINITY,
                        f32::NEG_INFINITY,
                    ],
                };
                for u in [-half_length, half_length] {
                    for v in [INNER, INNER + width + 2.0] {
                        let [px, pz] = plan.place(u, v);
                        plan.bounds[0] = plan.bounds[0].min(px - 0.2);
                        plan.bounds[1] = plan.bounds[1].min(pz - 0.2);
                        plan.bounds[2] = plan.bounds[2].max(px + 0.2);
                        plan.bounds[3] = plan.bounds[3].max(pz + 0.2);
                    }
                }
                if (0..2).all(|axis| {
                    plan.bounds[axis] >= origin[axis] + 4.0
                        && plan.bounds[axis + 2] < origin[axis] + ROAD_REGION_SIZE as f32 - 4.0
                }) && plan.fits(world, seed, road, &mut parcels, reserved, pulse)
                {
                    return Some(plan);
                }
            }
        }
        None
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub fn kind(&self) -> TurnoutKind {
        self.kind
    }

    pub fn road_contact(&self) -> [f32; 3] {
        self.center
    }

    pub fn roadside_building(&self) -> Option<(RoadsideBuilding, [f32; 3])> {
        let [along, across, ..] = self.building_pad()?;
        let [x, z] = self.place(along, across);
        Some((RoadsideBuilding::select(self.kind, self.variant), [x, self.deck(along), z]))
    }

    pub fn downhill(&self) -> [f32; 2] {
        [-self.axis[1] * self.side, self.axis[0] * self.side]
    }

    fn place(&self, along: f32, across: f32) -> [f32; 2] {
        let [x, _, z] = self.center;
        let [dx, dz] = self.downhill();
        [
            x + self.axis[0] * along + dx * across,
            z + self.axis[1] * along + dz * across,
        ]
    }

    fn outer(&self, along: f32) -> f32 {
        let t = along * self.inverse_half_length;
        INNER + self.width * (1.0 - t * t).max(0.0)
    }

    fn deck(&self, along: f32) -> f32 {
        self.center[1] + self.grade * along - 0.08
    }

    fn fits(
        &self,
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        parcels: &mut ParcelSurvey,
        reserved: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        for row in -14..=14 {
            pulse();
            let along = self.half_length * row as f32 / 14.0;
            let outer = self.outer(along);
            let deck = self.deck(along);
            for col in 0..=14 {
                let across = INNER + (outer + 1.65 - INNER) * col as f32 / 14.0;
                let [x, z] = self.place(along, across);
                let raw = world.sample(x, z);
                if raw.elevation < 6.0
                    || !matches!(
                        raw.surface,
                        Surface::Grass | Surface::Forest | Surface::Rock | Surface::Desert
                    )
                    || !(-2.3..=1.45).contains(&(raw.elevation - deck))
                    || reserved(x, z)
                    || (across > ROAD_HALF_WIDTH + 1.4
                        && road.proximity(x, z).is_some_and(|hit| {
                            hit.distance_squared < ROAD_HALF_WIDTH * ROAD_HALF_WIDTH
                        }))
                    || (raw.surface == Surface::Grass && parcels.occupies_grass(world, seed, x, z))
                {
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn sample(&self, x: f32, z: f32, raw: Sample) -> Option<Sample> {
        let [left, near, right, far] = self.bounds;
        if !(left..=right).contains(&x) || !(near..=far).contains(&z) || raw.elevation < 6.0 {
            return None;
        }
        let [dx, dz] = [x - self.center[0], z - self.center[2]];
        let [ax, az] = self.axis;
        let along = dx * ax + dz * az;
        let across = self.side * (dz * ax - dx * az);
        if along.abs() >= self.half_length || across < INNER {
            return None;
        }
        let end = along.abs() * self.inverse_half_length;
        let edge = across - self.outer(along);
        let deck = self.deck(along);
        if let Some(amenity) = self.amenity(along, across, edge, deck) {
            return Some(amenity);
        }
        if self.kind == TurnoutKind::Overlook && end < 0.68 && edge.abs() < WALL_HALF {
            let cap = 0.88 * smoothstep((0.68 - end) * 11.0);
            return Some(Sample {
                elevation: deck + cap,
                surface: self.wall,
            });
        }
        if edge <= 0.0 {
            let grade = smoothstep((across - INNER) * 0.58);
            return Some(Sample {
                elevation: raw.elevation + (deck - raw.elevation) * grade,
                surface: if across < INNER + 1.9 || end > 0.73 {
                    Surface::Pavement
                } else {
                    Surface::Gravel
                },
            });
        }
        if self.kind == TurnoutKind::Overlook && end < 0.69 && edge < 1.65 {
            let taper = smoothstep((1.65 - edge) * 1.24) * smoothstep((0.69 - end) * 12.0);
            return Some(Sample {
                elevation: raw.elevation + (deck - raw.elevation).max(0.0) * taper,
                surface: Surface::ScreeCoarse,
            });
        }
        None
    }

    fn fixture_pad(&self) -> Option<[f32; 4]> {
        let overlook = self.kind == TurnoutKind::Overlook;
        let side = if overlook || self.variant & 1 != 0 { -1.0 } else { 1.0 };
        let along = self.half_length * side * if overlook { 0.35 } else { 0.45 };
        let half_length = (self.half_length * 0.17).clamp(1.64, 2.38);
        let half_depth = if overlook { 0.82 } else { 1.14 };
        let max_across = self.outer(along.abs() + half_length) - half_depth - 0.64;
        let across = (INNER + if overlook { 4.18 } else { 5.62 }).min(max_across);
        (across - half_depth >= INNER + 3.0).then_some([along, across, half_length, half_depth])
    }

    fn building_pad(&self) -> Option<[f32; 4]> {
        let overlook = self.kind == TurnoutKind::Overlook;
        // Place the main building on the other side of the bay from its
        // smaller fixture and keep the approach tapers and wall clear.
        let side = if overlook || self.variant & 1 != 0 { 1.0 } else { -1.0 };
        let along = self.half_length * side * 0.22;
        let reach = (self.half_length * 0.20).clamp(1.78, 3.18);
        let depth = if overlook { 1.15 } else { 1.62 };
        let margin = if overlook { WALL_HALF + 0.15 } else { 0.56 };
        let edge = self.outer(along.abs() + reach) - depth - margin;
        let across = (INNER + if overlook { 5.18 } else { 6.13 }).min(edge);
        (across - depth >= INNER + 3.04).then_some([along, across, reach, depth])
    }

    fn amenity(&self, along: f32, across: f32, edge: f32, deck: f32) -> Option<Sample> {
        // Leave the through lane and both tapers clear. All furnishings stay
        // inside the strip whose foundations the turnout survey approved.
        if edge > -0.4 || across < INNER + 3.0 {
            return None;
        }
        let building = self.building_pad();
        if let Some([center_u, center_v, reach, depth]) = building {
            let u = along - center_u;
            let v = across - center_v;
            if u.abs() <= reach && v.abs() <= depth {
                return RoadsideBuilding::select(self.kind, self.variant).profile([u / reach, v / depth])
                    .map(|(height, surface)| Sample { elevation: deck + height, surface });
            }
        }
        let fixture = self.fixture_pad();
        if let Some([center_u, center_v, reach, depth]) = fixture {
            let u = along - center_u;
            let v = across - center_v;
            if u.abs() <= reach && v.abs() <= depth {
                if let Some((height, surface)) = RoadsideFixture::select(self.kind, self.variant)
                    .profile([u / reach, v / depth])
                {
                    return Some(Sample { elevation: deck + height, surface });
                }
            }
        }
        let width_style = f32::from(self.variant % 5);
        let roof_style = f32::from(self.variant / 5 % 5);
        let direction = if self.variant & 1 == 0 { -1.0 } else { 1.0 };
        let u = along - direction * self.half_length * 0.13;
        let v = across - (INNER + 5.5);
        let strip = |height, surface| Some(Sample { elevation: deck + height, surface });
        match self.kind {
            TurnoutKind::Overlook => {
                if (along - self.half_length * 0.31).abs() < 1.2 + width_style * 0.1
                    && (across - (INNER + 3.55)).abs() < 0.34
                {
                    strip(0.48 + roof_style * 0.03, Surface::Boardwalk)
                } else {
                    None
                }
            }
            _ if building.is_some() => None,
            _ => stop_roof(self.kind, self.variant, u, v)
                .and_then(|(height, surface)| strip(height, surface))
                .or_else(|| {
                    (fixture.is_none() && (along + direction * self.half_length * 0.43).abs() < 0.8 && v.abs() < 0.46)
                        .then(|| Sample { elevation: deck + 1.35 + roof_style * 0.06, surface: Surface::Boardwalk })
                }),
        }
    }
}

/// Vary the footprint and ridge of the shelters independently within each use.
fn stop_roof(kind: TurnoutKind, variant: u8, along: f32, across: f32) -> Option<(f32, Surface)> {
    let family = variant / 25;
    let longer = f32::from(variant % 5);
    let taller = f32::from(variant / 5 % 5);
    let trailhead = kind == TurnoutKind::Trailhead;
    let length = if trailhead { 2.2 } else { 2.1 } + longer * 0.17;
    let depth = if trailhead { 1.48 } else { 1.30 };
    let u = along / length;
    let v = across / depth;
    if u.abs() > 1.0 || v.abs() > 1.0 {
        return None;
    }
    let ridge = 1.0 - v.abs();
    let end = 1.0 - u.abs();
    let gable = Surface::PitchedRoof;
    let flat = Surface::FlatRoof;
    let (rise, ink) = match family {
        0 if trailhead => (1.15 * ridge, gable), // Gable
        0 => (if u > 0.46 { 0.42 } else { 0.0 }, flat), // Stepped parapet
        1 => (1.2 * ridge.min(end * 1.8), gable), // Hip
        2 => (0.3 + 0.8 * (v + 1.0) * 0.5, gable), // Shed
        3 if v < -0.12 => (0.48 * end, gable), // Double shed and clerestory
        3 => (0.65 + 0.43 * end, gable),
        4 if u.abs() > 0.83 && v.abs() > 0.68 => return None, // Corner setbacks
        4 => (if u.abs() < 0.40 && v.abs() < 0.64 { 0.65 } else { 0.16 }, flat),
        5 if u > 0.30 && v < -0.12 => return None, // L-shaped shelter
        5 => (if u > 0.30 { 0.52 * end } else { 1.05 * ridge }, gable),
        6 if u.abs() < 0.11 => return None, // Two stalls separated by a footpath
        6 => (if u < 0.0 { 0.48 } else { 0.10 }, flat),
        7 if v.abs() < 0.23 && u.abs() < 0.75 => (1.05, flat), // Raised ventilation ridge
        7 => (0.42 * ridge, gable),
        8 if v > 0.58 => (0.17 + 0.22 * end, gable), // Low porch and pitched main roof
        8 => (1.1 * (1.0 - ((v + 0.22) / 0.78).abs()).max(0.0), gable),
        _ if u > 0.65 && v < -0.08 => return None, // Offset service bay
        _ => (if u > 0.63 { 0.17 } else if v < 0.2 { 0.46 } else { 0.72 }, flat),
    };
    Some((2.6 + taller * 0.17 + rise, ink))
}

fn distance(x: f32, z: f32) -> f32 {
    let squared = x * x + z * z;
    let mut root = x.abs().max(z.abs());
    for _ in 0..3 {
        root = 0.5 * (root + squared / root);
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surveyed_stops_fit_their_landscape_and_keep_the_through_road_clear() {
        let seed = 4;
        let world = World::new(seed);
        for (region, kind) in [
            ([4, -4], TurnoutKind::Overlook),
            ([1, -2], TurnoutKind::Trailhead),
            ([-3, -4], TurnoutKind::RestStop),
        ] {
            let road = RoadNetwork::generate(&world, region[0], region[1]).unwrap();
            let site = ScenicTurnout::survey(&world, seed, region, &road, |_, _| false, || {}).unwrap();
            assert_eq!(site.kind(), kind);
            let [x, h, z] = site.road_contact();
            assert!(road.proximity(x, z).is_some());
            assert!(site.sample(x, z, world.sample(x, z)).is_none());
            let [along, across, half, depth] = site.building_pad().expect("surveyed building pad");
            let design = RoadsideBuilding::select(kind, site.variant);
            let [u, v] = [-0.72, -0.3, 0.0, 0.3, 0.72].into_iter()
                .flat_map(|u| [-0.5, 0.0, 0.5].into_iter().map(move |v| [u, v]))
                .find(|point| design.profile(*point).is_some_and(|(height, _)| height > 2.0))
                .expect("roof on building pad");
            let [px, pz] = site.place(along + u * half, across + v * depth);
            let raw = world.sample(px, pz);
            let property = site.sample(px, pz, raw).unwrap();
            assert_eq!(property.surface, design.profile([u, v]).unwrap().1);
            assert!(property.elevation > h && raw.elevation > 6.0);
        }
    }

    #[test]
    fn thirty_fixture_designs_fit_real_turnout_shoulders() {
        use std::collections::HashSet;

        let seed = 4;
        let world = World::new(seed);
        let mut forms = HashSet::new();
        let mut masks = HashSet::new();
        for (region, kind) in [
            ([4, -4], TurnoutKind::Overlook),
            ([1, -2], TurnoutKind::Trailhead),
            ([-3, -4], TurnoutKind::RestStop),
        ] {
            let road = RoadNetwork::generate(&world, region[0], region[1]).unwrap();
            let mut site = ScenicTurnout::survey(&world, seed, region, &road, |_, _| false, || {}).unwrap();
            assert_eq!(site.kind, kind);
            for variant in 0..10 {
                site.variant = variant * 25;
                let form = RoadsideFixture::select(kind, site.variant);
                assert!(forms.insert(form));
                let [along, across, reach, depth] = site.fixture_pad().expect("approved fixture strip");
                let mut silhouette = [0_u8; 19 * 19];
                let mut occupied = 0;
                for (i, cell) in silhouette.iter_mut().enumerate() {
                    let u = ((i % 19) as f32 + 0.5) * (2.0 / 19.0) - 1.0;
                    let v = ((i / 19) as f32 + 0.5) * (2.0 / 19.0) - 1.0;
                    let [x,z] = site.place(along + u * reach, across + v * depth);
                    // Sample the profile at the actual world-coordinate point.
                    // A rounded point on a stair riser can fall on either step.
                    let [dx,dz] = [x - site.center[0],z - site.center[2]];
                    let [ax,az] = site.axis;
                    let sample_along = dx * ax + dz * az;
                    let sample_across = site.side * (dz * ax - dx * az);
                    let Some((height, material)) = form.profile([
                        (sample_along-along)/reach, (sample_across-across)/depth,
                    ]) else { continue };
                    assert!((0.18..=2.75).contains(&height), "{form:?} height {height}");
                    let raw = world.sample(x,z);
                    let observed = site.sample(x,z,raw).expect("drawn fixture");
                    assert_eq!(observed.surface, material, "{form:?}");
                    assert!((observed.elevation - site.deck(sample_along) - height).abs() < 0.0001, "{form:?}");
                    assert!(road.proximity(x, z).is_none_or(|hit|
                        hit.distance_squared >= ROAD_HALF_WIDTH * ROAD_HALF_WIDTH));
                    *cell = (height * 30.0) as u8;
                    occupied += 1;
                }
                assert!(occupied > 20, "{form:?} has too little projected geometry");
                assert!(masks.insert(silhouette), "{form:?} duplicates a previous model");
            }
        }
        assert_eq!(forms.len(), 30);
    }

    #[test]
    fn twenty_five_buildings_fit_surveyed_turnouts_and_have_distinct_roofs() {
        use std::collections::HashSet;

        let seed = 4;
        let world = World::new(seed);
        let mut shapes = HashSet::new();
        for (region, kind, count) in [
            ([4, -4], TurnoutKind::Overlook, 5),
            ([1, -2], TurnoutKind::Trailhead, 10),
            ([-3, -4], TurnoutKind::RestStop, 10),
        ] {
            let road = RoadNetwork::generate(&world, region[0], region[1]).unwrap();
            let mut site = ScenicTurnout::survey(&world, seed, region, &road, |_, _| false, || {}).unwrap();
            assert_eq!(site.kind(), kind);
            for index in 0..count {
                site.variant = index * (250 / count);
                let [along, across, half, depth] = site.building_pad().expect("surveyed building pad");
                if let Some([fixture, _, reach, _]) = site.fixture_pad() {
                    assert!((along - fixture).abs() > half + reach);
                }
                let form = RoadsideBuilding::select(kind, site.variant);
                let mut shape = [0_u8; 21 * 21];
                for (index, height_bucket) in shape.iter_mut().enumerate() {
                    let point = |i| (i as f32 + 0.5) * (2.0 / 21.0) - 1.0;
                    let [x, z] = site.place(along + point(index % 21) * half, across + point(index / 21) * depth);
                    let [dx, dz] = [x - site.center[0], z - site.center[2]];
                    let [ax, az] = site.axis;
                    let local_u = dx * ax + dz * az;
                    let local_v = site.side * (dz * ax - dx * az);
                    let Some((height, material)) = form.profile([(local_u - along) / half, (local_v - across) / depth]) else { continue; };
                    assert!(road.proximity(x, z).is_none_or(|hit| hit.distance_squared >= ROAD_HALF_WIDTH * ROAD_HALF_WIDTH));
                    let observed = site.sample(x, z, world.sample(x, z)).expect("published roof");
                    assert_eq!(observed.surface, material);
                    assert!((observed.elevation - site.deck(local_u) - height).abs() < 0.001);
                    *height_bucket = (height * 20.0) as u8;
                }
                assert!(shape.iter().any(|&sample| sample >= 45));
                assert!(shapes.insert(shape), "duplicated building {}", form.name());
            }
        }
        assert_eq!(shapes.len(), RoadsideBuilding::COUNT);
    }

    #[test]
    fn roof_catalog_has_250_distinct_forms_per_stop_type() {
        for kind in [TurnoutKind::Trailhead, TurnoutKind::RestStop] {
            let mut designs = std::collections::BTreeSet::new();
            for variant in 0..250 {
                let signature: std::vec::Vec<_> = (-16..=16).flat_map(|across| {
                    (-32..=32).map(move |along| {
                        stop_roof(kind, variant, along as f32 / 10.0, across as f32 / 10.0)
                            .map(|(height, surface)| ((height * 40.0) as u16, surface as u8))
                    })
                }).collect();
                assert!(designs.insert(signature), "duplicate roof {kind:?} {variant}");
            }
        }
    }
}
