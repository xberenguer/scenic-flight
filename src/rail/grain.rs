//! Road-served country elevators next to an already surveyed rail grade.

use super::{ground, vector_length, RailAnchorKind, RailLine, RailMaterial, RailSample};
use crate::ground_details::ParcelSurvey;
use crate::roads::{RoadNetwork, ROAD_HALF_WIDTH};
use crate::world::{Surface, World};

const ACCESS_HALF: f32 = 2.0;
const GALLERY_HALF: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrainTerminal {
    pub origin: [f32; 3],
    pub along: [f32; 2],
    pub outward: [f32; 2],
    pub track: [f32; 3],
    contact: [f32; 2],
    apron: [f32; 2],
    bounds: [f32; 4],
}

impl GrainTerminal {
    pub const HALF_LENGTH: f32 = 13.0;
    pub const HALF_WIDTH: f32 = 6.0;
    pub const MAX_HEIGHT: f32 = 11.5;

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub fn road_contact(&self) -> [f32; 2] {
        self.contact
    }

    fn ground_point(&self, along: f32, outward: f32) -> [f32; 2] {
        [
            self.origin[0] + along * self.along[0] + outward * self.outward[0],
            self.origin[2] + along * self.along[1] + outward * self.outward[1],
        ]
    }

    fn coordinates(&self, x: f32, z: f32) -> (f32, f32) {
        let [dx, dz] = [x - self.origin[0], z - self.origin[2]];
        (
            dx * self.along[0] + dz * self.along[1],
            dx * self.outward[0] + dz * self.outward[1],
        )
    }

    fn close(&self, x: f32, z: f32) -> bool {
        let [left, near, right, far] = self.bounds;
        (left..=right).contains(&x) && (near..=far).contains(&z)
    }

    pub(crate) fn sample(&self, x: f32, z: f32) -> Option<RailSample> {
        if !self.close(x, z) {
            return None;
        }
        let (along, across) = self.coordinates(x, z);
        if along.abs() <= Self::HALF_LENGTH && across.abs() <= Self::HALF_WIDTH {
            return Some(RailSample {
                material: RailMaterial::FreightPlatform,
                elevation: Some(self.origin[1]),
            });
        }
        let d = [
            self.apron[0] - self.contact[0],
            self.apron[1] - self.contact[1],
        ];
        let offset = [x - self.contact[0], z - self.contact[1]];
        let t = dot(offset, d) / dot(d, d);
        let lateral = [offset[0] - t * d[0], offset[1] - t * d[1]];
        ((0.0..=1.0).contains(&t) && dot(lateral, lateral) <= ACCESS_HALF * ACCESS_HALF)
            .then_some(ground(RailMaterial::StationAccess))
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        if !self.close(x, z) {
            return 0.0;
        }
        let (along, across) = self.coordinates(x, z);
        if along.abs() <= Self::HALF_LENGTH + 1.0 && across.abs() <= Self::HALF_WIDTH + 1.0 {
            return self.origin[1] + Self::MAX_HEIGHT + 1.5;
        }
        let (_, track_across) = self.coordinates(self.track[0], self.track[2]);
        if along.abs() <= GALLERY_HALF && across >= track_across - 1.0 && across <= 0.0 {
            return self.origin[1].max(self.track[1]) + Self::MAX_HEIGHT + 1.5;
        }
        0.0
    }

    fn update_bounds(&mut self) {
        let [x, _, z] = self.track;
        self.bounds = [x - 2.0, z - 2.0, x + 2.0, z + 2.0];
        for [x, z] in [
            self.contact,
            self.apron,
            self.ground_point(-Self::HALF_LENGTH, -Self::HALF_WIDTH),
            self.ground_point(-Self::HALF_LENGTH, Self::HALF_WIDTH),
            self.ground_point(Self::HALF_LENGTH, -Self::HALF_WIDTH),
            self.ground_point(Self::HALF_LENGTH, Self::HALF_WIDTH),
        ] {
            self.bounds[0] = self.bounds[0].min(x - 2.2);
            self.bounds[1] = self.bounds[1].min(z - 2.2);
            self.bounds[2] = self.bounds[2].max(x + 2.2);
            self.bounds[3] = self.bounds[3].max(z + 2.2);
        }
    }
}

pub(super) fn survey(
    line: &RailLine,
    world: &World,
    seed: u64,
    road: &RoadNetwork,
    mut reserved: impl FnMut(f32, f32) -> bool,
    mut pulse: impl FnMut(),
) -> Option<GrainTerminal> {
    if line
        .stations()
        .iter()
        .any(|station| station.anchor.kind.is_port())
    {
        return None;
    }
    let mut fields = ParcelSurvey::new();
    for (index, station) in line.stations().iter().enumerate() {
        if station.anchor.kind != RailAnchorKind::RoadTerminus {
            continue;
        }
        if !catchment(
            world,
            seed,
            [station.anchor.x, station.anchor.z],
            &mut fields,
            &mut reserved,
            &mut pulse,
        ) {
            continue;
        }
        for inward in [34.0, 25.0, 42.0, 51.0] {
            pulse();
            let station_offset = |offset| {
                if index == 0 {
                    offset
                } else {
                    line.length() - offset
                }
            };
            let a = line.point_at(station_offset(inward - 4.0));
            let b = line.point_at(station_offset(inward + 4.0));
            let track = line.point_at(station_offset(inward));
            let [dx, dz] = [b.x - a.x, b.z - a.z];
            let length = vector_length(dx, dz);
            if length < 6.0 {
                continue;
            }
            let along = [dx / length, dz / length];
            for side in [-1.0, 1.0] {
                let outward = [-along[1] * side, along[0] * side];
                for offset in [11.0, 13.0, 15.0] {
                    let x = track.x + outward[0] * offset;
                    let z = track.z + outward[1] * offset;
                    let base = world.elevation(x, z);
                    if base < 4.3 || (base - track.elevation).abs() > 1.65 {
                        continue;
                    }
                    let mut plan = GrainTerminal {
                        origin: [x, base, z],
                        along,
                        outward,
                        track: [track.x, track.elevation, track.z],
                        contact: [0.0; 2],
                        apron: [0.0; 2],
                        bounds: [0.0; 4],
                    };
                    if pad_fits(
                        &plan,
                        line,
                        world,
                        seed,
                        road,
                        &mut fields,
                        &mut reserved,
                        &mut pulse,
                    ) && join_road(
                        &mut plan,
                        line,
                        world,
                        seed,
                        road,
                        &mut fields,
                        &mut reserved,
                        &mut pulse,
                    ) {
                        plan.update_bounds();
                        return Some(plan);
                    }
                }
            }
        }
    }
    None
}

fn catchment(
    world: &World,
    seed: u64,
    [x, z]: [f32; 2],
    fields: &mut ParcelSurvey,
    reserved: &mut impl FnMut(f32, f32) -> bool,
    pulse: &mut impl FnMut(),
) -> bool {
    let mut crops = 0;
    for dz in [-118.0, -58.0, 12.0, 72.0, 128.0] {
        pulse();
        for dx in [-113.0, -49.0, 23.0, 67.0, 121.0] {
            let [x, z] = [x + dx, z + dz];
            if world.sample(x, z).surface == Surface::Grass
                && !reserved(x, z)
                && fields.occupies_grass(world, seed, x, z)
            {
                crops += 1;
                if crops >= 2 {
                    return true;
                }
            }
        }
    }
    false
}

fn pad_fits(
    plan: &GrainTerminal,
    line: &RailLine,
    world: &World,
    seed: u64,
    road: &RoadNetwork,
    fields: &mut ParcelSurvey,
    reserved: &mut impl FnMut(f32, f32) -> bool,
    pulse: &mut impl FnMut(),
) -> bool {
    for k in -10..=10 {
        pulse();
        let along = k as f32 * 1.4;
        for j in -5..=5 {
            let [x, z] = plan.ground_point(along, j as f32 * 1.4);
            let raw = world.sample(x, z);
            if !matches!(raw.surface, Surface::Grass | Surface::Desert)
                || raw.elevation < 4.0
                || (raw.elevation - plan.origin[1]).abs() > 1.15
                || line.sample(x, z).is_some()
                || road.proximity(x, z).is_some()
                || reserved(x, z)
                || (raw.surface == Surface::Grass && fields.occupies_grass(world, seed, x, z))
            {
                return false;
            }
        }
    }
    // The elevated spout crosses only the intended loading track, at its end.
    let (_, end) = plan.coordinates(plan.track[0], plan.track[2]);
    let spans = ((-end) * 1.2) as usize;
    for step in 0..=spans {
        pulse();
        let side = end * step as f32 / spans as f32;
        for along in [-GALLERY_HALF, 0.0, GALLERY_HALF] {
            let [x, z] = plan.ground_point(along, side);
            let raw = world.sample(x, z);
            if !matches!(
                raw.surface,
                Surface::Grass | Surface::Desert | Surface::Rock
            ) || raw.elevation < 3.0
                || raw.elevation > plan.track[1] + 3.5
                || reserved(x, z)
                || road.proximity(x, z).is_some()
                || (step < spans - 2 && line.sample(x, z).is_some())
            {
                return false;
            }
        }
    }
    true
}

fn join_road(
    plan: &mut GrainTerminal,
    line: &RailLine,
    world: &World,
    seed: u64,
    road: &RoadNetwork,
    fields: &mut ParcelSurvey,
    reserved: &mut impl FnMut(f32, f32) -> bool,
    pulse: &mut impl FnMut(),
) -> bool {
    for span in road.polyline().windows(2) {
        let [a, b] = [span[0], span[1]];
        let start = [(a.x + b.x) * 0.5, (a.z + b.z) * 0.5];
        let (along, across) = plan.coordinates(start[0], start[1]);
        if along.abs() > 25.0 || !(GrainTerminal::HALF_WIDTH + 6.5..=52.0).contains(&across) {
            continue;
        }
        pulse();
        let finish = plan.ground_point(along.clamp(-9.5, 9.5), GrainTerminal::HALF_WIDTH);
        let d = [finish[0] - start[0], finish[1] - start[1]];
        let length = vector_length(d[0], d[1]);
        let [sx, sz] = [-d[1] / length, d[0] / length];
        let count = (length / 1.15) as usize + 1;
        let mut last: Option<f32> = None;
        let mut clear = true;
        for i in 0..=count {
            pulse();
            let t = i as f32 / count as f32;
            let center = [start[0] + d[0] * t, start[1] + d[1] * t];
            let h = world.elevation(center[0], center[1]);
            if last.is_some_and(|last| (h - last).abs() > length * 0.12 / count as f32 + 0.003)
                || (i == count && (h - plan.origin[1]).abs() > 0.55)
            {
                clear = false;
                break;
            }
            last = Some(h);
            if [-ACCESS_HALF, 0.0, ACCESS_HALF].into_iter().any(|lateral| {
                let [x, z] = [center[0] + sx * lateral, center[1] + sz * lateral];
                let raw = world.sample(x, z);
                raw.elevation < 3.5
                    || (raw.elevation - h).abs() > 0.55
                    || reserved(x, z)
                    || line.sample(x, z).is_some()
                    || (i > 5
                        && road.proximity(x, z).is_some_and(|hit| {
                            hit.distance_squared < ROAD_HALF_WIDTH * ROAD_HALF_WIDTH
                        }))
                    || (raw.surface == Surface::Grass && fields.occupies_grass(world, seed, x, z))
            }) {
                clear = false;
                break;
            }
        }
        if clear {
            plan.contact = start;
            plan.apron = finish;
            return true;
        }
    }
    false
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}
