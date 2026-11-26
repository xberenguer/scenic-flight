//! Prepared, terrain-following links between neighboring regional highways.

use crate::roads::{
    RoadClass, RoadNetwork, RoadPoint, RoadProximity, ROAD_HALF_WIDTH, ROAD_REGION_SIZE,
};
use crate::world::{World, SEA_LEVEL};
use crate::settlement::{Settlement, SettlementRoadExit, SettlementSurface};

mod town_paths;
use town_paths::TOWN_PATHS;

const MAX_POINTS: usize = 97;
const EMPTY_POINT: RoadPoint = RoadPoint {
    x: 0.0,
    z: 0.0,
    elevation: 0.0,
};
const MARGIN: f32 = ROAD_HALF_WIDTH + 1.5;
const GRADE: f32 = 0.12;
const HEIGHT_RANGE: core::ops::RangeInclusive<f32> = (SEA_LEVEL + 1.6)..=(SEA_LEVEL + 68.0);
const SIDES: [f32; 5] = [-MARGIN, -ROAD_HALF_WIDTH, 0.0, ROAD_HALF_WIDTH, MARGIN];
const DETOURS: [(f32, f32); 19] = [
    (0.0, 0.0),
    (36.0, 0.0),
    (-36.0, 0.0),
    (70.0, 0.0),
    (-70.0, 0.0),
    (0.0, 120.0),
    (0.0, -120.0),
    (110.0, 0.0),
    (-110.0, 0.0),
    (160.0, 0.0),
    (-160.0, 0.0),
    (55.0, 125.0),
    (55.0, -125.0),
    (-55.0, 125.0),
    (-55.0, -125.0),
    (110.0, 145.0),
    (110.0, -145.0),
    (-110.0, 145.0),
    (-110.0, -145.0),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HighwayContact {
    pub region: [i32; 2],
    /// Exact endpoint of the existing regional plan, including elevation.
    pub point: RoadPoint,
    /// False denotes its first point, true its final point.
    pub final_endpoint: bool,
}

pub struct HighwayLink {
    contacts: [HighwayContact; 2],
    street_caps: [Option<RoadPoint>; 2],
    town_alignment: Option<u8>,
    points: [RoadPoint; MAX_POINTS],
    stations: [f32; MAX_POINTS],
    axis: [f32; 2],
    count: usize,
    bounds: [f32; 4],
}

/// The only road data a detached link survey needs. Existing layouts stay
/// owned by the streams; the worker only receives these exact termini.
#[derive(Clone, Copy)]
pub(crate) struct HighwaySurvey {
    pub(crate) contacts: [HighwayContact; 2],
    pub(crate) index: usize,
    principal: [usize; 2],
    pub(crate) outgoing: [f32; 2],
    pub(crate) inward: [f32; 2],
}

const TOWN_EXITS: usize = 6;
pub(crate) const TOWN_CAP_PAIRS: u8 = (TOWN_EXITS * TOWN_EXITS) as u8;

pub(crate) struct TownCapPlan {
    exits: [[Option<SettlementRoadExit>; TOWN_EXITS]; 2],
    occupied: [bool; 2],
}

impl TownCapPlan {
    pub(crate) fn pair(&self, index: u8) -> Option<[Option<SettlementRoadExit>; 2]> {
        if !self.occupied.into_iter().any(|town| town) {
            return None;
        }
        let entries = [usize::from(index) / TOWN_EXITS, usize::from(index) % TOWN_EXITS];
        let [a, b] = entries;
        let option = |i: usize, exit: usize| {
            if self.occupied[i] { self.exits[i].get(exit).copied().flatten().map(Some) }
            else { (exit == 0).then_some(None) }
        };
        Some([option(0, a)?, option(1, b)?])
    }
}

pub(crate) fn within_street_cap(cap: RoadPoint, x: f32, z: f32) -> bool {
    let [dx, dz] = [x - cap.x, z - cap.z];
    dx * dx + dz * dz <= (MARGIN + 0.8) * (MARGIN + 0.8)
}

impl HighwayLink {
    /// Fit against the world's natural elevation field. A composed scene with
    /// separately placed water or buildings should call [Self::fit_avoiding].
    pub fn fit(world: &World, a: &RoadNetwork, b: &RoadNetwork) -> Option<Self> {
        Self::fit_avoiding(world, a, b, |_, _| false)
    }

    /// Cold planning step. Supply shared watershed, lakes, or protected land
    /// footprints in `blocked` when the scene uses those layers.
    pub fn fit_avoiding(
        world: &World,
        a: &RoadNetwork,
        b: &RoadNetwork,
        blocked: impl FnMut(f32, f32) -> bool,
    ) -> Option<Self> {
        HighwaySurvey::between(a, b)?
            .fit(world, 0, false, blocked, &mut || {})
            .map(|(link, _)| link)
    }

    pub fn contacts(&self) -> [HighwayContact; 2] {
        self.contacts
    }

    /// The outer end of a connected town street, when reached via its grid.
    pub fn street_caps(&self) -> [Option<RoadPoint>; 2] {
        self.street_caps
    }

    pub fn town_alignment(&self) -> Option<&'static str> {
        self.town_alignment.map(|index| TOWN_PATHS[usize::from(index)].name)
    }

    pub fn polyline(&self) -> &[RoadPoint] {
        &self.points[..self.count]
    }

    /// World bounds [min x, min z, max x, max z] including the road margin.
    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    /// Intersect the actual round-ended shoulder with an axis-aligned area.
    /// Check each chord so a canyon or facility inside a bend's overall box
    /// does not reserve unrelated ground.
    pub fn intersects_bounds(&self, bounds: [f32; 4]) -> bool {
        let [left, near, right, far] = bounds;
        if !bounds.into_iter().all(f32::is_finite)
            || left > right
            || near > far
            || left > self.bounds[2]
            || right < self.bounds[0]
            || near > self.bounds[3]
            || far < self.bounds[1]
        {
            return false;
        }
        let radius = MARGIN + 0.002;
        let squared = radius * radius;
        self.polyline().windows(2).any(|segment| {
            let [a, b] = [segment[0], segment[1]];
            if a.x.min(b.x) > right + radius
                || a.x.max(b.x) < left - radius
                || a.z.min(b.z) > far + radius
                || a.z.max(b.z) < near - radius
            {
                return false;
            }
            [a, b].into_iter().any(|point| {
                let dx = point.x - point.x.clamp(left, right);
                let dz = point.z - point.z.clamp(near, far);
                dx * dx + dz * dz <= squared
            }) || chord_crosses_bounds(a, b, bounds)
                || [(left, near), (left, far), (right, near), (right, far)]
                    .into_iter()
                    .any(|(x, z)| segment_distance_squared(x, z, a, b) <= squared)
        })
    }
}

fn chord_crosses_bounds(a: RoadPoint, b: RoadPoint, [left, near, right, far]: [f32; 4]) -> bool {
    let mut start = 0.0_f32;
    let mut stop = 1.0_f32;
    for (position, direction, low, high) in
        [(a.x, b.x - a.x, left, right), (a.z, b.z - a.z, near, far)]
    {
        if direction == 0.0 {
            if position < low || position > high {
                return false;
            }
        } else {
            let a = (low - position) / direction;
            let b = (high - position) / direction;
            start = start.max(a.min(b));
            stop = stop.min(a.max(b));
            if start > stop {
                return false;
            }
        }
    }
    true
}

impl HighwaySurvey {
    pub(crate) fn between(a: &RoadNetwork, b: &RoadNetwork) -> Option<Self> {
        let mut regions = [road_region(a)?, road_region(b)?];
        let (a, b) = if regions[0] > regions[1] {
            regions.swap(0, 1);
            (b, a)
        } else {
            (a, b)
        };
        let index = match (regions[1][0] - regions[0][0], regions[1][1] - regions[0][1]) {
            (1, 0) => 0,
            (0, 1) => 1,
            _ => return None,
        };
        let (start, outgoing) = terminus(a, regions[0], index, true);
        let (finish, inward) = terminus(b, regions[1], index, false);
        let principal = [a, b].map(|road| {
            let points = road.polyline();
            let a = points[0];
            let b = points[points.len() - 1];
            usize::from((b.z - a.z).abs() > (b.x - a.x).abs())
        });
        Some(Self {
            contacts: [start, finish],
            index,
            principal,
            outgoing,
            inward,
        })
    }

    /// A terminus serves at most one adjacent seam, determined by the main
    /// direction of its real district road. The endpoint tangent is checked
    /// separately when fitting the transition.
    pub(crate) fn owns_termini(self) -> bool {
        self.principal == [self.index; 2]
            && self
                .contacts
                .into_iter()
                .all(|contact| self.inside_seam(contact.point.x, contact.point.z))
    }

    pub(crate) fn inside_seam(self, x: f32, z: f32) -> bool {
        let position = [x, z];
        let side = ROAD_REGION_SIZE as f32;
        let cross = 1 - self.index;
        let seam = self.contacts[1].region[self.index] as f32 * side;
        let low = self.contacts[0].region[cross] as f32 * side;
        let depth = (position[self.index] - seam).abs();
        let lateral = position[cross] - low;
        // Both possible links in a district run along its principal axis.
        // Reserve opposing half-regions so their shoulders cannot cross.
        depth <= side * 0.5 - MARGIN - 0.01 && (MARGIN..=side - MARGIN).contains(&lateral)
    }

    pub(crate) fn town_cap_plan(
        self, world: &World, towns: [Option<&Settlement>; 2],
    ) -> TownCapPlan {
        TownCapPlan {
            exits: core::array::from_fn(|i| towns[i].map_or([None; TOWN_EXITS], |town| {
                town.road_exits(world, self.contacts[i ^ 1].point)
            })),
            occupied: towns.map(|town| town.is_some()),
        }
    }

    pub(crate) fn fit_town_cap_pair(
        self,
        world: &World,
        towns: [Option<&Settlement>; 2],
        caps: [Option<SettlementRoadExit>; 2],
        next: u8,
        mut reserved: impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<(HighwayLink, u8)> {
        let mut trial = self;
        if let Some(cap) = caps[0] {
            trial.contacts[0].point = cap.point;
            trial.outgoing = cap.outward;
        }
        if let Some(cap) = caps[1] {
            trial.contacts[1].point = cap.point;
            trial.inward = cap.outward.map(|value| -value);
        }
        let blocked = |x, z| {
            reserved(x, z) || towns.into_iter().zip(caps).any(|(town, cap)| {
                town.is_some_and(|town| town.sample_transport_reservation(x,z).is_some_and(|sample| {
                    sample.surface != SettlementSurface::Street
                        || !cap.is_some_and(|exit| within_street_cap(exit.point, x, z))
                }))
            })
        };
        let (mut link, continuation) = trial.fit_route(world, next, true, true, blocked, pulse)?;
        link.contacts = self.contacts;
        link.street_caps = caps.map(|cap| cap.map(|exit| exit.point));
        Some((link, continuation))
    }

    pub(crate) fn fit(
        self,
        world: &World,
        next: u8,
        stay_on_seam: bool,
        blocked: impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<(HighwayLink, u8)> {
        self.fit_route(world, next, stay_on_seam, false, blocked, pulse)
    }

    fn fit_route(
        self,
        world: &World,
        next: u8,
        stay_on_seam: bool,
        town: bool,
        mut blocked: impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<(HighwayLink, u8)> {
        let Self {
            contacts: [start, finish],
            index,
            outgoing,
            inward,
            ..
        } = self;
        let delta = [
            finish.point.x - start.point.x,
            finish.point.z - start.point.z,
        ];
        let distance = length(delta);
        if !(1.0..=850.0).contains(&distance) || delta[index] < distance * 0.60 {
            return None;
        }
        let axis = [delta[0] / distance, delta[1] / distance];
        if dot(axis, outgoing) < 0.50 || dot(axis, inward) < 0.50 {
            return None;
        }
        let p = start.point;
        let q = finish.point;
        // Same endpoint derivatives for each detour. The lateral offset and
        // its derivative both vanish at the two real road junctions.
        let handle = (distance * 0.19).min(72.0);
        let segments = (distance / 10.0) as usize + 1;
        let junction_step = 3.0 * handle / segments as f32;
        let lateral_region = start.region[1 - index];
        let lateral_min = (lateral_region * ROAD_REGION_SIZE) as f32 + MARGIN;
        let lateral_max = lateral_min + ROAD_REGION_SIZE as f32 - 2.0 * MARGIN;
        let mut link = HighwayLink {
            contacts: [start, finish],
            street_caps: [None; 2],
            town_alignment: None,
            points: [EMPTY_POINT; MAX_POINTS],
            stations: [0.0; MAX_POINTS],
            axis,
            count: segments + 1,
            bounds: [0.0; 4],
        };
        let profile_count = if town { TOWN_PATHS.len() } else { DETOURS.len() };
        for variant in usize::from(next)..profile_count {
            let [shift, skew, counter_bend] = if town { TOWN_PATHS[variant].curve } else {
                let (shift, skew) = DETOURS[variant];
                [shift, skew, 0.0]
            };
            link.town_alignment = town.then_some(variant as u8);
            pulse();
            link.points[0] = p;
            link.points[segments] = q;
            link.stations[segments] = distance;
            let mut valid = true;
            // Cheap centerline screen before committing to subcell shoulder probes.
            for i in 1..segments {
                if i & 7 == 0 {
                    pulse();
                }
                let t = i as f32 / segments as f32;
                let u = 1.0 - t;
                let middle = t * t * (3.0 - 2.0 * t);
                let head = 3.0 * u * u * t * handle;
                let tail = 3.0 * u * t * t * handle;
                let balance = t - u;
                let lateral_offset = shift + skew * balance
                    + counter_bend * balance * (1.0 - 2.1 * balance * balance);
                let detour = 16.0 * lateral_offset * t * t * u * u;
                let [x, z] = if i == 1 {
                    [
                        p.x + outgoing[0] * junction_step,
                        p.z + outgoing[1] * junction_step,
                    ]
                } else if i == segments - 1 {
                    [
                        q.x - inward[0] * junction_step,
                        q.z - inward[1] * junction_step,
                    ]
                } else {
                    [
                        p.x + delta[0] * middle + outgoing[0] * head
                            - inward[0] * tail
                            - axis[1] * detour,
                        p.z + delta[1] * middle + outgoing[1] * head - inward[1] * tail
                            + axis[0] * detour,
                    ]
                };
                let along = dot([x - p.x, z - p.z], axis);
                let lateral = if index == 0 { z } else { x };
                let height = world.elevation(x, z);
                let previous = link.points[i - 1];
                if !(lateral_min..=lateral_max).contains(&lateral)
                    || (stay_on_seam && !self.inside_seam(x, z))
                    || along <= link.stations[i - 1]
                    || along >= distance
                    || !HEIGHT_RANGE.contains(&height)
                    || (height - previous.elevation).abs()
                        > length([x - previous.x, z - previous.z]) * GRADE + 0.002
                    || blocked(x, z)
                {
                    valid = false;
                    break;
                }
                link.points[i] = RoadPoint {
                    x,
                    z,
                    elevation: height,
                };
                link.stations[i] = along;
            }
            if valid && link.prove(world, &mut blocked, pulse) {
                link.bounds = link
                    .polyline()
                    .iter()
                    .fold([p.x, p.z, p.x, p.z], |b, point| {
                        [
                            b[0].min(point.x),
                            b[1].min(point.z),
                            b[2].max(point.x),
                            b[3].max(point.z),
                        ]
                    });
                link.bounds[0] -= MARGIN;
                link.bounds[1] -= MARGIN;
                link.bounds[2] += MARGIN;
                link.bounds[3] += MARGIN;
                return Some((link, variant as u8 + 1));
            }
        }
        None
    }
}

impl HighwayLink {
    /// Finds the pavement or margin using the saved link, without terrain probes.
    pub fn proximity(&self, x: f32, z: f32) -> Option<RoadProximity> {
        let [min_x, min_z, max_x, max_z] = self.bounds;
        if !(x >= min_x && x <= max_x && z >= min_z && z <= max_z) {
            return None;
        }
        let p = self.points[0];
        let along = dot([x - p.x, z - p.z], self.axis);
        let stations = &self.stations[..self.count];
        let reach = MARGIN + 0.001;
        let low = stations
            .partition_point(|&v| v < along - reach)
            .saturating_sub(1);
        let high = stations
            .partition_point(|&v| v <= along + reach)
            .min(self.count - 1);
        let mut squared = f32::INFINITY;
        for i in low..high {
            squared = squared.min(segment_distance_squared(
                x,
                z,
                self.points[i],
                self.points[i + 1],
            ));
        }
        (squared <= MARGIN * MARGIN).then_some(RoadProximity {
            class: RoadClass::Highway,
            distance_squared: squared,
        })
    }

    fn prove(
        &self,
        world: &World,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        for (i, pair) in self.polyline().windows(2).enumerate() {
            let (a, b) = (pair[0], pair[1]);
            let delta = [b.x - a.x, b.z - a.z];
            let distance = length(delta);
            let unit = [delta[0] / distance, delta[1] / distance];
            let count = (distance / 0.80) as usize + 1;
            let max_step = GRADE * distance / count as f32 + 0.002;
            let mut previous = [0.0; SIDES.len()];
            for j in 0..=count {
                if j & 3 == 0 {
                    pulse();
                }
                let t = j as f32 / count as f32;
                let x = a.x + delta[0] * t;
                let z = a.z + delta[1] * t;
                let mut section = [0.0; SIDES.len()];
                for (band, side) in SIDES.into_iter().enumerate() {
                    let px = x - unit[1] * side;
                    let pz = z + unit[0] * side;
                    let height = world.elevation(px, pz);
                    if !HEIGHT_RANGE.contains(&height)
                        || (j != 0 && (height - previous[band]).abs() > max_step)
                        || blocked(px, pz)
                    {
                        return false;
                    }
                    section[band] = height;
                }
                if (section[1] - section[3]).abs() > 0.70
                    || (section[2] - section[1]).abs() > 0.525
                    || (section[2] - section[3]).abs() > 0.525
                    || (section[0] - section[1]).abs() > 0.35
                    || (section[4] - section[3]).abs() > 0.35
                {
                    return false;
                }
                previous = section;
            }
            // Along a joint the queried shoulder includes the little round cap.
            // Cover it in addition to the normal sections of the adjacent chords.
            if (i == 0 && !dry_cap(world, a, blocked)) || !dry_cap(world, b, blocked) {
                return false;
            }
        }
        true
    }
}

fn road_region(road: &RoadNetwork) -> Option<[i32; 2]> {
    let region_of = |point: RoadPoint| {
        [point.x, point.z].map(|coordinate| {
            let fraction = coordinate / ROAD_REGION_SIZE as f32;
            let integer = fraction as i32;
            integer - i32::from(fraction < integer as f32)
        })
    };
    let points = road.polyline();
    let region = region_of(*points.first()?);
    (points.len() >= 2 && region_of(*points.last()?) == region).then_some(region)
}

fn terminus(
    road: &RoadNetwork,
    region: [i32; 2],
    axis: usize,
    positive: bool,
) -> (HighwayContact, [f32; 2]) {
    let points = road.polyline();
    let last = points.len() - 1;
    let side = |p: RoadPoint| if axis == 0 { p.x } else { p.z };
    let final_endpoint = (side(points[last]) > side(points[0])) == positive;
    let (point, neighbor) = if final_endpoint {
        (points[last], points[last - 1])
    } else {
        (points[0], points[1])
    };
    let delta = if positive {
        [point.x - neighbor.x, point.z - neighbor.z]
    } else {
        [neighbor.x - point.x, neighbor.z - point.z]
    };
    let distance = length(delta);
    (
        HighwayContact {
            region,
            point,
            final_endpoint,
        },
        [delta[0] / distance, delta[1] / distance],
    )
}

fn dry_cap(world: &World, at: RoadPoint, blocked: &mut impl FnMut(f32, f32) -> bool) -> bool {
    const DIAGONAL: f32 = 0.70710677;
    for radius in [MARGIN * 0.5, MARGIN] {
        for (x, z) in [
            (1.0, 0.0),
            (DIAGONAL, DIAGONAL),
            (0.0, 1.0),
            (-DIAGONAL, DIAGONAL),
            (-1.0, 0.0),
            (-DIAGONAL, -DIAGONAL),
            (0.0, -1.0),
            (DIAGONAL, -DIAGONAL),
        ] {
            let x = at.x + x * radius;
            let z = at.z + z * radius;
            if !HEIGHT_RANGE.contains(&world.elevation(x, z)) || blocked(x, z) {
                return false;
            }
        }
    }
    true
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn length(delta: [f32; 2]) -> f32 {
    let squared = dot(delta, delta);
    let mut estimate = delta[0].abs().max(delta[1].abs());
    if estimate != 0.0 {
        for _ in 0..3 {
            estimate = 0.5 * (estimate + squared / estimate);
        }
    }
    estimate
}

fn segment_distance_squared(x: f32, z: f32, a: RoadPoint, b: RoadPoint) -> f32 {
    let from_start = [x - a.x, z - a.z];
    let delta = [b.x - a.x, b.z - a.z];
    let t = (dot(from_start, delta) / dot(delta, delta)).clamp(0.0, 1.0);
    let distance = [from_start[0] - delta[0] * t, from_start[1] - delta[1] * t];
    dot(distance, distance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settlement::{Settlement, Terminus};
    use crate::watershed::{Watershed, WatershedWork};

    const LINKS: [(u64, [i32; 2], [i32; 2]); 3] = [
        (31, [-5, -17], [-4, -17]),
        (7911, [-2, 3], [-2, 4]),
        (503_821, [6, 3], [6, 4]),
    ];

    const VACANT_LINKS: [(u64, [i32; 2], [i32; 2]); 3] = [
        (82, [13, -5], [14, -5]),
        (2114, [35, -19], [35, -18]),
        (503_821, [-19, 35], [-19, 36]),
    ];

    fn watershed_pair(world: &World, seed: u64, regions: [[i32; 2]; 2]) -> [Watershed; 2] {
        let mut work = WatershedWork::new();
        let mut water = [Watershed::new(), Watershed::new()];
        for (plan, [x, z]) in water.iter_mut().zip(regions) {
            plan.rebuild(world, seed, x, z, &mut work);
        }
        water
    }

    fn channel(water: &[Watershed; 2], x: f32, z: f32) -> bool {
        water
            .iter()
            .any(|plan| plan.sample(x, z).is_some_and(|river| river.is_channel()))
    }

    #[test]
    fn real_connections_are_dry_graded_and_match_both_regional_termini() {
        for (seed, a, b) in LINKS {
            let world = World::new(seed);
            let roads = [
                RoadNetwork::generate(&world, a[0], a[1]).unwrap(),
                RoadNetwork::generate(&world, b[0], b[1]).unwrap(),
            ];
            let water = watershed_pair(&world, seed, [a, b]);
            let link = HighwayLink::fit_avoiding(&world, &roads[0], &roads[1], |x, z| {
                channel(&water, x, z)
            })
            .unwrap();
            let reverse = HighwayLink::fit_avoiding(&world, &roads[1], &roads[0], |x, z| {
                channel(&water, x, z)
            })
            .unwrap();
            assert_eq!(reverse.polyline(), link.polyline());
            assert_eq!(reverse.contacts(), link.contacts());
            for (i, (road, contact)) in roads.iter().zip(link.contacts()).enumerate() {
                assert_eq!(contact.region, [a, b][i]);
                let end = if contact.final_endpoint {
                    road.polyline().last()
                } else {
                    road.polyline().first()
                };
                assert_eq!(end, Some(&contact.point));
                assert!(
                    road.proximity(contact.point.x, contact.point.z)
                        .unwrap()
                        .distance_squared
                        < 0.001
                );
                assert!(
                    link.proximity(contact.point.x, contact.point.z)
                        .unwrap()
                        .distance_squared
                        < 0.001
                );
                let other = if contact.final_endpoint {
                    road.polyline()[road.polyline().len() - 2]
                } else {
                    road.polyline()[1]
                };
                let next = if i == 0 {
                    link.polyline()[1]
                } else {
                    link.polyline()[link.polyline().len() - 2]
                };
                let dx = [other.x - contact.point.x, other.z - contact.point.z];
                let dz = [next.x - contact.point.x, next.z - contact.point.z];
                assert!(
                    dot(dx, dz) / (length(dx) * length(dz)) < -0.99,
                    "seed {seed}, contact {contact:?}"
                );
            }
            for pair in link.polyline().windows(2) {
                let [a, b] = [pair[0], pair[1]];
                let [dx, dz] = [b.x - a.x, b.z - a.z];
                let len = dx.hypot(dz);
                let steps = (len / 0.40) as usize + 1;
                let mut previous = [0.0; 19];
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    let x = a.x + dx * t;
                    let z = a.z + dz * t;
                    for (band, last) in previous.iter_mut().enumerate() {
                        let side = band as f32 * 0.5 - MARGIN;
                        let px = x - side * dz / len;
                        let pz = z + side * dx / len;
                        let y = world.elevation(px, pz);
                        assert!(HEIGHT_RANGE.contains(&y));
                        assert!(!channel(&water, px, pz));
                        assert!(i == 0 || (y - *last).abs() < len / steps as f32 * 0.126);
                        assert!(link.proximity(px, pz).is_some() || side.abs() == MARGIN);
                        *last = y;
                    }
                }
            }
        }
    }

    #[test]
    fn reserved_terminal_pairs_remain_available_with_land_use_and_watersheds() {
        for (seed, a, b) in VACANT_LINKS {
            let world = World::new(seed);
            let roads = [
                RoadNetwork::generate(&world, a[0], a[1]).unwrap(),
                RoadNetwork::generate(&world, b[0], b[1]).unwrap(),
            ];
            let water = watershed_pair(&world, seed, [a, b]);
            let towns = [
                Settlement::generate(&world, &roads[0], Terminus::Start),
                Settlement::generate(&world, &roads[0], Terminus::End),
                Settlement::generate(&world, &roads[1], Terminus::Start),
                Settlement::generate(&world, &roads[1], Terminus::End),
            ];
            let link = HighwayLink::fit_avoiding(&world, &roads[0], &roads[1], |x, z| {
                channel(&water, x, z)
                    || towns.iter().flatten().any(|town| {
                        town.sample(x, z)
                            .is_some_and(|sample| sample.elevation.is_some())
                    })
            })
            .unwrap();
            for (i, contact) in link.contacts().into_iter().enumerate() {
                let index = i * 2 + usize::from(contact.final_endpoint);
                assert!(towns[index].is_none());
            }
        }
    }

    #[test]
    fn channel_barriers_and_non_neighboring_plans_are_rejected() {
        let (seed, a, b) = LINKS[0];
        let world = World::new(seed);
        let left = RoadNetwork::generate(&world, a[0], a[1]).unwrap();
        let right = RoadNetwork::generate(&world, b[0], b[1]).unwrap();
        let distant = RoadNetwork::generate(&world, 4, -5).unwrap();
        assert!(HighwayLink::fit(&world, &left, &left).is_none());
        assert!(HighwayLink::fit(&world, &left, &distant).is_none());
        let seam = (b[0] * ROAD_REGION_SIZE) as f32;
        assert!(
            HighwayLink::fit_avoiding(&world, &left, &right, |x, _| (x - seam).abs() < 1.0)
                .is_none()
        );

        let world = World::new(31);
        let a = RoadNetwork::generate(&world, 1, 5).unwrap();
        let b = RoadNetwork::generate(&world, 2, 5).unwrap();
        let water = watershed_pair(&world, 31, [[1, 5], [2, 5]]);
        assert!(HighwayLink::fit(&world, &a, &b).is_some());
        assert!(HighwayLink::fit_avoiding(&world, &a, &b, |x, z| channel(&water, x, z)).is_none());
    }

    #[test]
    fn indexed_margin_matches_the_full_saved_path() {
        for (seed, a, b) in LINKS {
            let world = World::new(seed);
            let left = RoadNetwork::generate(&world, a[0], a[1]).unwrap();
            let right = RoadNetwork::generate(&world, b[0], b[1]).unwrap();
            let water = watershed_pair(&world, seed, [a, b]);
            let link =
                HighwayLink::fit_avoiding(&world, &left, &right, |x, z| channel(&water, x, z))
                    .unwrap();
            let points = link.polyline();
            for pair in points.windows(2) {
                for i in 0..=6 {
                    let t = i as f32 / 6.0;
                    let x = pair[0].x + (pair[1].x - pair[0].x) * t;
                    let z = pair[0].z + (pair[1].z - pair[0].z) * t;
                    for dx in [-5.0, -4.5, -3.0, 0.0, 3.0, 4.5, 5.0] {
                        for dz in [-5.0, -4.5, -3.0, 0.0, 3.0, 4.5, 5.0] {
                            let x = x + dx;
                            let z = z + dz;
                            let expected = points
                                .windows(2)
                                .map(|point| segment_distance_squared(x, z, point[0], point[1]))
                                .fold(f32::INFINITY, f32::min);
                            let actual = link.proximity(x, z).map(|r| r.distance_squared);
                            assert_eq!(
                                actual,
                                (expected <= MARGIN * MARGIN).then_some(expected),
                                "{seed} {x} {z}"
                            );
                        }
                    }
                }
            }
        }
    }
}
