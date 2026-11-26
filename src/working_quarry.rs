//! Road-served open cuts on hillsides and dry gravel terraces.

use crate::ground_details::ParcelSurvey;
use crate::lighthouses::{LIGHTHOUSE_TILE, Lighthouse};
use crate::math::{fraction, mix, smoothstep};
use crate::roads::{ROAD_HALF_WIDTH, ROAD_REGION_SIZE, RoadNetwork};
use crate::world::{Sample, Surface, World};

mod profiles;
pub use profiles::QuarryProfile;

const SPUR_HALF: f32 = 3.05;
const SPUR_SHOULDER: f32 = 4.65;
const MAX_SPUR_GRADE: f32 = 0.105;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuarryForm {
    Hillside,
    GravelPit,
}

#[derive(Clone, Copy, Debug)]
pub struct Quarry {
    junction: [f32; 3],
    yard: [f32; 3],
    axis: [f32; 2],
    length: f32,
    inverse_length: f32,
    curve: f32,
    inverse_width: f32,
    inverse_depth: f32,
    bench: f32,
    bounds: [f32; 4],
    form: QuarryForm,
    profile: u8,
}

impl Quarry {
    pub fn generate(
        world: &World,
        seed: u64,
        region: [i32; 2],
        road: &RoadNetwork,
        mut blocked: impl FnMut(f32, f32) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        let tile = (u64::from(region[0] as u32) << 32) | u64::from(region[1] as u32);
        let key = mix(seed ^ tile ^ 0xcb09_8782_b8e7_64d1);
        if key & 7 >= 3 {
            return None;
        }
        let road_points = road.polyline();
        let available = road_points.len().checked_sub(10)?;
        if available == 0 {
            return None;
        }
        let origin = region.map(|axis| (axis * ROAD_REGION_SIZE) as f32);
        let mut attempts = 0;
        let mut parcels = ParcelSurvey::new();
        for i in 0..available.min(16) {
            pulse();
            let slot = 5 + ((key >> 21) as usize + i) % available;
            let [a, b] = [road_points[slot - 1], road_points[slot]];
            let [dx, dz] = [b.x - a.x, b.z - a.z];
            let distance = root(dx * dx + dz * dz);
            if distance < 14.0 {
                continue;
            }
            let [hx, hz] = [(a.x + b.x) * 0.5, (a.z + b.z) * 0.5];
            let start_h = world.elevation(hx, hz);
            if !(11.0..=65.0).contains(&start_h)
                || !road
                    .proximity(hx, hz)
                    .is_some_and(|part| part.distance_squared < 0.01)
            {
                continue;
            }
            let [tx, tz] = [dx / distance, dz / distance];
            let state = mix(key ^ (slot as u64 + 0x1279));
            let first_side = if state & 2 == 0 { 1.0 } else { -1.0 };
            for side in [first_side, -first_side] {
                for along in [0.0, 0.34, -0.34] {
                    let norm = 1.0 / root(1.0 + along * along);
                    let axis = [
                        (-tz * side + tx * along) * norm,
                        (tx * side + tz * along) * norm,
                    ];
                    let length = 59.0 + 32.0 * fraction(mix(state ^ 0x72b));
                    let j = [hx, hz];
                    let yard = offset(j, axis, length, 0.0);
                    let yard_h = world.elevation(yard[0], yard[1]);
                    if !(0.1..=length * MAX_SPUR_GRADE).contains(&(yard_h - start_h)) {
                        continue;
                    }
                    let at = |forward, transverse| {
                        let [x, z] = offset(yard, axis, forward, transverse);
                        world.elevation(x, z)
                    };
                    let raw_back = at(64.0, 0.0);
                    let face_relief = raw_back - yard_h;
                    if !(21.0..=39.0).contains(&face_relief)
                        || at(30.0, 0.0) < yard_h + 8.0
                        || at(48.0, -17.0).min(at(48.0, 17.0)) < yard_h + 14.0
                    {
                        continue;
                    }
                    let width = 30.0 + 7.5 * fraction(mix(state ^ 0x3faa));
                    let depth = 68.0 + 8.0 * fraction(mix(state ^ 0x873a));
                    let plan = Self {
                        junction: [hx, start_h, hz],
                        yard: [yard[0], yard_h, yard[1]],
                        axis,
                        length,
                        inverse_length: 1.0 / length,
                        curve: (fraction(mix(state ^ 0xaa10)) - 0.5) * 13.0,
                        inverse_width: 1.0 / width,
                        inverse_depth: 1.0 / depth,
                        bench: (face_relief * 0.25).clamp(5.2, 7.2),
                        bounds: combined_bounds(j, yard, axis, width, depth),
                        form: QuarryForm::Hillside,
                        profile: (mix(state ^ 0x7318_2a5d_93e4_461b)
                            % QuarryProfile::ALL.len() as u64)
                            as u8,
                    };
                    if !(0..2).all(|axis| {
                        plan.bounds[axis] >= origin[axis] + 4.0
                            && plan.bounds[axis + 2] < origin[axis] + ROAD_REGION_SIZE as f32 - 4.0
                    }) {
                        continue;
                    }
                    if attempts == 10 {
                        return None;
                    }
                    attempts += 1;
                    if plan.prove(world, seed, road, &mut parcels, &mut blocked, &mut pulse)
                        && plan.clears_navigation(world, seed, &mut pulse)
                    {
                        return Some(plan);
                    }
                }
            }
        }
        None
    }

    /// Prospect the raised, dry shelf behind the coast for loose aggregate.
    /// Freight demand motivates this additional survey, but does not grant a site.
    pub(crate) fn gravel_pit(
        world: &World,
        seed: u64,
        region: [i32; 2],
        road: &RoadNetwork,
        mut blocked: impl FnMut(f32, f32) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        let tile = u64::from(region[0] as u32) << 32 | u64::from(region[1] as u32);
        let key = mix(seed ^ tile ^ 0x23a9_5718_896b_b04f);
        let points = road.polyline();
        let spans = points.len().checked_sub(6)?.min(32);
        let origin = region.map(|axis| (axis * ROAD_REGION_SIZE) as f32);
        let mut parcels = ParcelSurvey::new();
        let mut attempts = 0;
        for trial in 0..spans {
            pulse();
            let slot = 3 + ((key >> 25) as usize + trial) % spans;
            let [a, b] = [points[slot - 1], points[slot]];
            let [dx, dz] = [b.x - a.x, b.z - a.z];
            let distance = root(dx * dx + dz * dz);
            if distance < 14.0 {
                continue;
            }
            let j = [(a.x + b.x) * 0.5, (a.z + b.z) * 0.5];
            let start = world.elevation(j[0], j[1]);
            if !(6.5..=21.0).contains(&start) {
                continue;
            }
            let [tx, tz] = [dx / distance, dz / distance];
            let state = mix(key ^ (slot as u64).wrapping_mul(0xa076_1d64_78bd_642f));
            let side = if state & 1 == 0 { 1.0 } else { -1.0 };
            for side in [side, -side] {
                for skew in [0.0, -0.33, 0.33] {
                    let norm = 1.0 / root(1.0 + skew * skew);
                    let axis = [
                        (-tz * side + tx * skew) * norm,
                        (tx * side + tz * skew) * norm,
                    ];
                    let length = 47.0 + 13.0 * fraction(mix(state ^ 0x44dc));
                    let width = 23.0 + 5.0 * fraction(mix(state ^ 0xbc05));
                    let depth = 44.0 + 8.0 * fraction(mix(state ^ 0x52d7));
                    let yard = offset(j, axis, length, 0.0);
                    let (ground, shelf) = world.sample_and_shore(yard[0], yard[1]);
                    if !matches!(ground.surface, Surface::Grass | Surface::Desert)
                        || !(0.12..=0.41).contains(&shelf)
                    {
                        continue;
                    }
                    let floor = ground.elevation - 2.55 - 0.5 * fraction(mix(state ^ 0x880b));
                    if floor < 4.8 || floor > start + 0.4 || start - floor > length * MAX_SPUR_GRADE
                    {
                        continue;
                    }
                    // A broad, continuous terrace must remain dry around and behind the cut.
                    if [
                        (depth * 0.42, -width * 0.8),
                        (depth * 0.42, width * 0.8),
                        (depth, 0.0),
                    ]
                    .into_iter()
                    .any(|(forward, side)| {
                        let [x, z] = offset(yard, axis, forward, side);
                        let h = world.elevation(x, z);
                        !(ground.elevation - 1.6..=ground.elevation + 3.8).contains(&h)
                    }) {
                        continue;
                    }
                    let plan = Self {
                        junction: [j[0], start, j[1]],
                        yard: [yard[0], floor, yard[1]],
                        axis,
                        length,
                        inverse_length: 1.0 / length,
                        curve: (fraction(mix(state ^ 0x8f1c)) - 0.5) * 6.0,
                        inverse_width: 1.0 / width,
                        inverse_depth: 1.0 / depth,
                        bench: 1.0,
                        bounds: combined_bounds(j, yard, axis, width, depth),
                        form: QuarryForm::GravelPit,
                        profile: 0,
                    };
                    if !(0..2).all(|axis| {
                        plan.bounds[axis] >= origin[axis] + 4.0
                            && plan.bounds[axis + 2] < origin[axis] + ROAD_REGION_SIZE as f32 - 4.0
                    }) {
                        continue;
                    }
                    if attempts == 12 {
                        return None;
                    }
                    attempts += 1;
                    if plan.prove(world, seed, road, &mut parcels, &mut blocked, &mut pulse)
                        && plan.clears_navigation(world, seed, &mut pulse)
                    {
                        return Some(plan);
                    }
                }
            }
        }
        None
    }

    pub fn junction(&self) -> [f32; 3] {
        self.junction
    }

    pub fn yard(&self) -> [f32; 3] {
        self.yard
    }

    pub fn axis(&self) -> [f32; 2] {
        self.axis
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    fn spur(&self, t: f32) -> (f32, f32) {
        let side = self.curve * 4.0 * t * (1.0 - t);
        let h = self.junction[1] + (self.yard[1] - self.junction[1]) * t;
        (side, h)
    }

    pub fn profile(&self) -> Option<&'static QuarryProfile> {
        (self.form == QuarryForm::Hillside).then(|| &QuarryProfile::ALL[usize::from(self.profile)])
    }

    fn excavation(&self, forward: f32, transverse: f32) -> Option<(f32, f32)> {
        self.excavation_profile(forward, transverse, self.profile())
    }

    fn excavation_profile(
        &self,
        forward: f32,
        transverse: f32,
        profile: Option<&QuarryProfile>,
    ) -> Option<(f32, f32)> {
        let u = transverse * self.inverse_width;
        let v = forward * self.inverse_depth;
        if !(-0.16..=1.0).contains(&v) || u.abs() >= 1.0 {
            return None;
        }
        let (back, wing) = if let Some(plan) = profile {
            let advance = v.max(0.0);
            let slip = u - plan.drift * advance * advance;
            let opening = 1.0 - plan.throat * (1.0 - advance) * (1.0 - advance);
            let back = v + plan.flank * slip * slip + plan.skew * slip * advance;
            (back, (slip.abs() / opening).max(u.abs()))
        } else {
            (v + 0.24 * u * u, u.abs())
        };
        let edge = smoothstep((1.0 - wing) * 4.8)
            * smoothstep((1.0 - back) * 7.0)
            * smoothstep((v + 0.16) * 7.0);
        if edge <= 0.0 {
            return None;
        }
        let rise = if let Some(plan) = profile {
            let [lower, middle, top] = plan.terraces;
            smoothstep((back - lower) * 23.0)
                + smoothstep((back - middle) * 26.0)
                + smoothstep((back - top) * 26.0)
        } else {
            smoothstep((back - 0.66) * 10.0)
        };
        Some((
            self.yard[1] + 0.65 * back.max(0.0) + self.bench * rise,
            edge,
        ))
    }

    fn stockpile(&self, forward: f32, transverse: f32) -> Option<(f32, Surface)> {
        let along = forward * self.inverse_depth;
        let across = transverse * self.inverse_width;
        if !(0.23..0.82).contains(&along) || !(0.29..0.77).contains(&across.abs()) {
            return None;
        }
        // The seeded spur curvature also mirrors and staggers the loading bays.
        let across = across * if self.curve < 0.0 { -1.0 } else { 1.0 };
        let along = along - 0.015 * self.curve;
        for (side, center, width, depth, cap, material) in [
            (0.52, 0.39, 0.21, 0.15, 1.65, Surface::ScreeCoarse),
            (-0.53, 0.54, 0.22, 0.17, 1.45, Surface::ScreeFine),
            (0.48, 0.66, 0.17, 0.11, 1.10, Surface::ScreeFine),
        ] {
            let x = (across - side) / width;
            let z = (along - center) / depth;
            let mound = 1.0 - x * x - z * z;
            if mound > 0.0 {
                return Some((cap * mound * mound, material));
            }
        }
        None
    }

    fn prove(
        &self,
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        parcels: &mut ParcelSurvey,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        let [jx, _, jz] = self.junction;
        let spans = (self.length / 1.15) as usize + 1;
        let mut last: Option<[f32; 3]> = None;
        for i in 0..=spans {
            pulse();
            let t = i as f32 / spans as f32;
            let (center, grade) = self.spur(t);
            let [x, z] = offset([jx, jz], self.axis, self.length * t, center);
            let height = world.elevation(x, z);
            let on_trunk = road
                .proximity(x, z)
                .is_some_and(|point| point.distance_squared <= ROAD_HALF_WIDTH * ROAD_HALF_WIDTH);
            let road_height = if on_trunk {
                height
            } else {
                height + (grade - height) * join_weight(self.length * t)
            };
            if let Some([px, py, pz]) = last {
                let step = root((x - px) * (x - px) + (z - pz) * (z - pz));
                if (road_height - py).abs() > 0.114 * step {
                    return false;
                }
            }
            last = Some([x, road_height, z]);
            for side in [-SPUR_SHOULDER, -SPUR_HALF, 0.0, SPUR_HALF, SPUR_SHOULDER] {
                let [x, z] = offset([jx, jz], self.axis, self.length * t, center + side);
                let g = world.sample(x, z);
                let own_contact = t * self.length < 2.0 * ROAD_HALF_WIDTH + 3.0;
                if !natural(g.surface)
                    || g.elevation < 4.0
                    || (g.elevation - grade).abs() > if own_contact { 1.25 } else { 3.5 }
                    || blocked(x, z)
                    || (!own_contact && road.proximity(x, z).is_some())
                    || (g.surface == Surface::Grass && parcels.occupies_grass(world, seed, x, z))
                {
                    return false;
                }
            }
        }
        let [qx, _, qz] = self.yard;
        let mut working = 0;
        for row in -3..=21 {
            pulse();
            let forward = row as f32 * 3.25;
            for column in -13..=13 {
                let transverse = column as f32 * 3.0;
                let Some((h, weight)) = self.excavation(forward, transverse) else {
                    continue;
                };
                let [x, z] = offset([qx, qz], self.axis, forward, transverse);
                let g = world.sample(x, z);
                if !natural(g.surface)
                    || g.elevation < 4.0
                    || g.elevation - h > 24.0
                    || blocked(x, z)
                    || road.proximity(x, z).is_some()
                    || (g.surface == Surface::Grass && parcels.occupies_grass(world, seed, x, z))
                {
                    return false;
                }
                working += usize::from(weight > 0.78 && g.elevation - h > 2.0);
            }
        }
        working
            > if self.form == QuarryForm::GravelPit {
                88
            } else {
                140
            }
    }

    fn clears_navigation(&self, world: &World, seed: u64, pulse: &mut impl FnMut()) -> bool {
        let [left, near, right, far] = self.bounds;
        let tile = |value: f32| {
            let scaled = value / LIGHTHOUSE_TILE as f32;
            let integer = scaled as i32;
            integer - i32::from(scaled < integer as f32)
        };
        for z in tile(near)..=tile(far) {
            for x in tile(left)..=tile(right) {
                pulse();
                if Lighthouse::generate(world, seed, x, z).is_some_and(|light| {
                    let [a, b, c, d] = light.bounds();
                    left <= c && right >= a && near <= d && far >= b
                }) {
                    return false;
                }
            }
        }
        true
    }

    fn coordinates(&self, x: f32, z: f32) -> [f32; 2] {
        let [dx, dz] = [x - self.junction[0], z - self.junction[2]];
        let [ax, az] = self.axis;
        [dx * ax + dz * az, dz * ax - dx * az]
    }

    pub(crate) fn occupies(&self, x: f32, z: f32) -> bool {
        let [left, near, right, far] = self.bounds;
        if !(left..right).contains(&x) || !(near..far).contains(&z) {
            return false;
        }
        let [along, across] = self.coordinates(x, z);
        let t = (along * self.inverse_length).clamp(0.0, 1.0);
        let (center, _) = self.spur(t);
        ((0.0..=self.length + 4.0).contains(&along) && (across - center).abs() < SPUR_SHOULDER)
            || self.excavation(along - self.length, across).is_some()
    }

    pub(crate) fn overlay_developed(
        &self,
        x: f32,
        z: f32,
        raw: Sample,
        developed: Sample,
    ) -> Sample {
        let ground = if developed.surface.is_tree_canopy() {
            raw
        } else {
            developed
        };
        let placed = self.overlay(x, z, ground);
        if placed == ground { developed } else { placed }
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let [left, near, right, far] = self.bounds;
        if !(left..right).contains(&x) || !(near..far).contains(&z) {
            return ground;
        }
        let [along, across] = self.coordinates(x, z);
        if (0.0..=self.length + 4.0).contains(&along)
            && (natural(ground.surface) || (ground.surface == Surface::Gravel && along < 9.0))
        {
            let t = (along * self.inverse_length).min(1.0);
            let (center, grade) = self.spur(t);
            let edge = (across - center).abs();
            if edge < SPUR_SHOULDER {
                let strength =
                    smoothstep((SPUR_SHOULDER - edge) * (1.0 / 1.6)) * join_weight(along);
                return Sample {
                    elevation: ground.elevation
                        + (grade - ground.elevation).clamp(-3.5, 3.5) * strength,
                    surface: if edge < SPUR_HALF && along < self.length + 1.0 {
                        Surface::Pavement
                    } else {
                        Surface::Gravel
                    },
                };
            }
        }
        if !natural(ground.surface) {
            return ground;
        }
        let forward = along - self.length;
        if let Some((h, strength)) = self.excavation(forward, across) {
            let cut = (ground.elevation - h).max(0.0) * strength;
            if cut > 0.18 {
                if self.form == QuarryForm::GravelPit && strength > 0.76 && cut > 0.9 {
                    if let Some((mound, surface)) = self.stockpile(forward, across) {
                        return Sample {
                            elevation: ground.elevation - cut + mound.min(cut * 0.7),
                            surface,
                        };
                    }
                }
                return Sample {
                    elevation: ground.elevation - cut,
                    surface: if strength > 0.33 {
                        Surface::Gravel
                    } else {
                        ground.surface
                    },
                };
            }
            if strength > 0.81 && (0.0..18.0).contains(&forward) {
                return Sample {
                    surface: Surface::Gravel,
                    ..ground
                };
            }
        }
        ground
    }
}

// Carry the native highway's grade through its paved and gravel collar
// before easing into the surveyed spur profile.
fn join_weight(distance: f32) -> f32 {
    smoothstep((distance - 6.0) * (1.0 / 14.0))
}

fn natural(surface: Surface) -> bool {
    matches!(
        surface,
        Surface::Grass | Surface::Forest | Surface::Rock | Surface::Desert
    )
}

fn offset(point: [f32; 2], [ax, az]: [f32; 2], along: f32, side: f32) -> [f32; 2] {
    [
        point[0] + ax * along - az * side,
        point[1] + az * along + ax * side,
    ]
}

fn combined_bounds(a: [f32; 2], b: [f32; 2], axis: [f32; 2], width: f32, depth: f32) -> [f32; 4] {
    let [xe, ze] = axis.map(f32::abs);
    let end = offset(b, axis, depth * 0.42, 0.0);
    let qx = xe * depth * 0.59 + ze * width;
    let qz = ze * depth * 0.59 + xe * width;
    [
        (a[0].min(b[0]) - 13.0).min(end[0] - qx),
        (a[1].min(b[1]) - 13.0).min(end[1] - qz),
        (a[0].max(b[0]) + 13.0).max(end[0] + qx),
        (a[1].max(b[1]) + 13.0).max(end[1] + qz),
    ]
}

fn root(value: f32) -> f32 {
    let seed = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    let first = (seed + value / seed) * 0.5;
    (first + value / first) * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenery::DistrictRegion;

    #[test]
    fn the_spur_continues_from_actual_pavement_to_the_open_working_floor() {
        for region in [[3, 2], [-4, 4]] {
            let world = World::new(3);
            let district = DistrictRegion::build(&world, region[0], region[1]);
            let road = district.road().unwrap();
            let site = Quarry::generate(
                &world,
                3,
                region,
                road,
                |x, z| district.towns().any(|town| town.sample(x, z).is_some()),
                || {},
            )
            .unwrap();
            let [jx, _, jz] = site.junction();
            assert!(
                road.proximity(jx, jz)
                    .is_some_and(|point| point.distance_squared < 0.001)
            );
            let mut prev: Option<[f32; 3]> = None;
            let mut max_grade = 0.0_f32;
            for i in 0..=240 {
                let t = i as f32 * (1.0 / 240.0);
                let (offset_side, _) = site.spur(t);
                let [x, z] = offset([jx, jz], site.axis, site.length * t, offset_side);
                let old = district.overlay(x, z, world.sample(x, z));
                let next = site.overlay(x, z, old);
                assert_eq!(next.surface, Surface::Pavement, "{region:?} sample {i}");
                if i == 0 {
                    assert_eq!(old, next);
                }
                if let Some([a, y, b]) = prev {
                    let step = root((x - a) * (x - a) + (z - b) * (z - b));
                    max_grade = max_grade.max((next.elevation - y).abs() / step);
                }
                prev = Some([x, next.elevation, z]);
            }
            assert!(max_grade <= 0.12, "{region:?}: {max_grade}");
            let [qx, h, qz] = site.yard();
            let actual = site.overlay(qx, qz, world.sample(qx, qz));
            assert!((actual.elevation - h).abs() <= 0.01);
            let mut cuts = 0;
            let mut deepest = 0.0_f32;
            for i in 0..=50 {
                for j in -29..=29 {
                    let [x, z] = offset([qx, qz], site.axis, i as f32 * 1.4, j as f32 * 1.3);
                    let old = district.overlay(x, z, world.sample(x, z));
                    let new = site.overlay(x, z, old);
                    if new.elevation < old.elevation - 0.20 {
                        assert!(natural(old.surface));
                        assert!(new.elevation > 4.0);
                        deepest = deepest.max(old.elevation - new.elevation);
                        cuts += 1;
                    }
                }
            }
            assert!(
                deepest > 4.8 && deepest <= 24.0 && cuts > 500,
                "{region:?}: {deepest} {cuts}"
            );
        }
    }
}
