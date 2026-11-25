//! Rare road shortcuts through a continuous dry ridge.
//!
//! The connector and its old road bypass share two junctions. Its two portal
//! rings enclose the actual cross-section of the bore.

use crate::math::mix;
use crate::roads::{RoadNetwork, RoadPoint};
use crate::world::{SEA_LEVEL, World};

pub const TUNNEL_HALF_WIDTH: f32 = 2.9;
const FRAME_HALF_WIDTH: f32 = 3.9;
const JAMB_HEIGHT: f32 = 3.0;
const ARCH_RISE: f32 = 1.45;
const FRAME_RISE: f32 = 2.25;
const ROCK_COVER: f32 = 1.2;
const MAX_GRADE: f32 = 0.075;
const MIN_COVERED_LENGTH: f32 = 22.0;
const MIN_APPROACH: f32 = 5.0;
const MAX_APPROACH: f32 = 54.0;
const MIN_LENGTH: f32 = 43.0;
const MAX_LENGTH: f32 = 155.0;
const MAX_MARKS: usize = 96;
const MAX_CANDIDATES: usize = 16;
// Fraction of half-width and fraction of rise. Shared by the clear opening,
// ring mesh, bore query and geology check.
const ARCH: [(f32, f32); 5] = [
    (0.0, 1.0),
    (0.383, 0.924),
    (0.707, 0.707),
    (0.924, 0.383),
    (1.0, 0.0),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TunnelPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TunnelMouth {
    pub center: TunnelPoint,
    /// Unit vector into the ridge and toward the other entrance.
    pub inward_x: f32,
    pub inward_z: f32,
}

impl TunnelMouth {
    /// Wound from the left pavement edge over the crown to the right edge,
    /// as seen when looking in. The transport surface is the omitted edge.
    pub fn opening(self) -> [TunnelPoint; 11] {
        self.profile(TUNNEL_HALF_WIDTH, ARCH_RISE)
    }

    /// Stone-faced retaining ring. Each quad lies outside the clear opening.
    pub fn frame(self) -> [[TunnelPoint; 4]; 10] {
        let inside = self.opening();
        let outside = self.profile(FRAME_HALF_WIDTH, FRAME_RISE);
        core::array::from_fn(|i| [outside[i], inside[i], inside[i + 1], outside[i + 1]])
    }

    fn profile(self, half_width: f32, rise: f32) -> [TunnelPoint; 11] {
        core::array::from_fn(|i| {
            let (side, height) = match i {
                0 => (-half_width, 0.0),
                10 => (half_width, 0.0),
                _ => {
                    let index = i.abs_diff(5);
                    let (side, height) = ARCH[index];
                    (
                        if i < 5 { -side } else { side } * half_width,
                        JAMB_HEIGHT + height * rise,
                    )
                }
            };
            TunnelPoint {
                x: self.center.x + self.inward_z * side,
                y: self.center.y + height,
                z: self.center.z - self.inward_x * side,
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TunnelVoid {
    pub floor: f32,
    pub ceiling: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TunnelApproach {
    pub elevation: f32,
    /// Absolute distance from the center line, in world units.
    pub offset: f32,
    /// Weight of the new grade when joining the existing pavement or ground.
    /// Zero at the junction, one by the cut and portal.
    pub blend: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadTunnel {
    junctions: [RoadPoint; 2],
    ax: f32,
    az: f32,
    length: f32,
    grade: f32,
    bore_start: f32,
    bore_end: f32,
}

impl RoadTunnel {
    pub fn generate(world: &World, seed: u64, road: &RoadNetwork) -> Option<Self> {
        Self::generate_cooperatively(world, seed, road, &mut || {})
    }

    /// Survey the same seeded bore while periodically servicing other work.
    pub fn generate_cooperatively(
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let mut marks = [Mark::EMPTY; MAX_MARKS];
        let used = road_marks(world, road.polyline(), &mut marks, pulse);
        let marks = &marks[..used];
        let mut candidates = [Candidate::EMPTY; MAX_CANDIDATES];

        for (index, start) in marks.iter().copied().enumerate() {
            for (end_index, end) in marks.iter().copied().enumerate().skip(index + 1) {
                if end_index & 15 == 0 {
                    pulse();
                }
                let detour = end.distance - start.distance;
                if detour < MIN_LENGTH + 11.0 {
                    continue;
                }
                if detour > MAX_LENGTH * 2.2 {
                    break;
                }
                let dx = end.point.x - start.point.x;
                let dz = end.point.z - start.point.z;
                let squared = dx * dx + dz * dz;
                if !(MIN_LENGTH * MIN_LENGTH..=MAX_LENGTH * MAX_LENGTH).contains(&squared) {
                    continue;
                }
                let rise = end.point.elevation - start.point.elevation;
                if rise * rise > MAX_GRADE * MAX_GRADE * squared {
                    continue;
                }
                let length = sqrt(squared);
                if detour < length * 1.105 + 6.0 {
                    continue;
                }
                let ax = dx / length;
                let az = dz / length;
                if start.tx * ax + start.tz * az < 0.38 || end.tx * ax + end.tz * az < 0.38 {
                    continue;
                }
                let grade = rise / length;
                let mut reaches_cover = false;
                let mut max_cover: f32 = 0.0;
                for fraction in [0.29, 0.43, 0.57, 0.71] {
                    let s = length * fraction;
                    let height = world.elevation(start.point.x + ax * s, start.point.z + az * s);
                    let cover = height - (start.point.elevation + grade * s);
                    reaches_cover |= cover >= JAMB_HEIGHT + ARCH_RISE + ROCK_COVER;
                    max_cover = max_cover.max(cover);
                }
                if !reaches_cover {
                    continue;
                }
                let choice = mix(seed ^ ((index as u64) << 32) ^ end_index as u64);
                let score = detour - length + max_cover.min(22.0) + (choice & 255) as f32 / 256.0;
                let candidate = Candidate {
                    score,
                    tunnel: Self {
                        junctions: [start.point, end.point],
                        ax,
                        az,
                        length,
                        grade,
                        bore_start: 0.0,
                        bore_end: 0.0,
                    },
                };
                if let Some(slot) = candidates.iter().position(|other| score > other.score) {
                    candidates.copy_within(slot..MAX_CANDIDATES - 1, slot + 1);
                    candidates[slot] = candidate;
                }
            }
        }

        candidates
            .iter()
            .filter(|c| c.score > 0.0)
            .find_map(|c| c.tunnel.prove(world, road, pulse))
    }

    pub fn junctions(&self) -> [RoadPoint; 2] {
        self.junctions
    }

    pub fn mouths(&self) -> [TunnelMouth; 2] {
        [
            TunnelMouth {
                center: self.point(self.bore_start, 0.0),
                inward_x: self.ax,
                inward_z: self.az,
            },
            TunnelMouth {
                center: self.point(self.bore_end, 0.0),
                inward_x: -self.ax,
                inward_z: -self.az,
            },
        ]
    }

    pub fn covered_length(&self) -> f32 {
        self.bore_end - self.bore_start
    }

    pub fn grade(&self) -> f32 {
        self.grade
    }

    /// Includes the dry cut approaches and the tunnel frame.
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let [a, b] = self.junctions;
        (
            a.x.min(b.x) - FRAME_HALF_WIDTH,
            a.z.min(b.z) - FRAME_HALF_WIDTH,
            a.x.max(b.x) + FRAME_HALF_WIDTH,
            a.z.max(b.z) + FRAME_HALF_WIDTH,
        )
    }

    /// Clear bore volume in a column, continuous with both portal polygons.
    /// The caller can retain the land above `ceiling` and render the inside.
    pub fn void_at(&self, x: f32, z: f32) -> Option<TunnelVoid> {
        let (along, offset) = self.project(x, z);
        // Quantized world coordinates can place a returned portal vertex
        // slightly off its exact local plane.
        let epsilon = 0.003;
        if !(self.bore_start - epsilon..=self.bore_end + epsilon).contains(&along)
            || !(0.0..=TUNNEL_HALF_WIDTH + epsilon).contains(&offset.abs())
        {
            return None;
        }
        let floor = self.floor(along.clamp(self.bore_start, self.bore_end));
        Some(TunnelVoid {
            floor,
            ceiling: floor + roof_height(offset),
        })
    }

    /// Floor of the open road cuts on either side. Leave terrain outside the
    /// frame width for the retaining walls and preserve the existing road.
    pub fn approach_at(&self, x: f32, z: f32) -> Option<TunnelApproach> {
        let (along, offset) = self.project(x, z);
        let offset = offset.abs();
        if !(-0.003..=self.length + 0.003).contains(&along)
            || (along > self.bore_start && along < self.bore_end)
            || offset > TUNNEL_HALF_WIDTH
        {
            return None;
        }
        let along = along.clamp(0.0, self.length);
        let junction_distance = along.min(self.length - along);
        let blend = (junction_distance / MIN_APPROACH).clamp(0.0, 1.0);
        Some(TunnelApproach {
            elevation: self.floor(along),
            offset,
            blend: blend * blend * (3.0 - 2.0 * blend),
        })
    }

    fn project(&self, x: f32, z: f32) -> (f32, f32) {
        let dx = x - self.junctions[0].x;
        let dz = z - self.junctions[0].z;
        (dx * self.ax + dz * self.az, dx * self.az - dz * self.ax)
    }

    fn floor(&self, along: f32) -> f32 {
        self.junctions[0].elevation + along * self.grade
    }

    fn point(&self, along: f32, offset: f32) -> TunnelPoint {
        TunnelPoint {
            x: self.junctions[0].x + along * self.ax + offset * self.az,
            y: self.floor(along),
            z: self.junctions[0].z + along * self.az - offset * self.ax,
        }
    }

    fn terrain(&self, world: &World, along: f32, offset: f32) -> f32 {
        let point = self.point(along, offset);
        world.elevation(point.x, point.z)
    }

    fn rock_cover(&self, world: &World, along: f32) -> bool {
        let floor = self.floor(along);
        for i in [0, -4, 4, -2, 2, -3, 3, -1, 1] {
            let side = i as f32 * (TUNNEL_HALF_WIDTH / 4.0);
            let required = floor + roof_height(side) + ROCK_COVER;
            if self.terrain(world, along, side) < required {
                return false;
            }
        }
        self.terrain(world, along, -FRAME_HALF_WIDTH) >= floor + JAMB_HEIGHT
            && self.terrain(world, along, FRAME_HALF_WIDTH) >= floor + JAMB_HEIGHT
    }

    fn clear_approach(
        &self,
        world: &World,
        mouth: f32,
        far: bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        let length = if far { self.length - mouth } else { mouth };
        if !(MIN_APPROACH..=MAX_APPROACH).contains(&length) {
            return false;
        }
        let count = (length * 2.0 + 1.0) as usize;
        for step in 0..=count {
            if step & 3 == 0 {
                pulse();
            }
            let road_distance = length * step as f32 / count as f32;
            let along = if far {
                self.length - road_distance
            } else {
                road_distance
            };
            let floor = self.floor(along);
            let mouth_distance = length - road_distance;
            // The cut can deepen toward the portal. The existing road
            // junction stays nearly level with the original surface.
            let max_cut = 1.45
                + (road_distance / 22.0).clamp(0.0, 1.0) * 5.5
                + (1.0 - mouth_distance / 9.0).clamp(0.0, 1.0) * 1.2;
            let max_fill = 0.95 + (road_distance / 11.0).clamp(0.0, 1.0) * 1.20;
            for index in -4..=4 {
                let side = index as f32 * (FRAME_HALF_WIDTH / 4.0);
                let height = self.terrain(world, along, side);
                if height < SEA_LEVEL + 2.1 || height < floor - max_fill || height > floor + max_cut
                {
                    return false;
                }
            }
        }
        true
    }

    // After the junction taper, do not cut the old pavement down to a
    // separate level. Where the roof is intact, the bypass can pass above it.
    fn keeps_bypass(
        &self,
        world: &World,
        road: &RoadNetwork,
        first: f32,
        last: f32,
        pulse: &mut impl FnMut(),
    ) -> bool {
        for (start, end) in [(7.0, first), (last, self.length - 7.0)] {
            if end <= start {
                continue;
            }
            let checks = (end - start) as usize + 1;
            for index in 0..=checks {
                if index & 7 == 0 {
                    pulse();
                }
                let along = start + (end - start) * index as f32 / checks as f32;
                for offset in [-TUNNEL_HALF_WIDTH, -1.45, 0.0, 1.45, TUNNEL_HALF_WIDTH] {
                    let p = self.point(along, offset);
                    if road
                        .proximity(p.x, p.z)
                        .is_some_and(|p| p.distance_squared <= 2.2 * 2.2)
                        && (world.elevation(p.x, p.z) - p.y).abs() > 0.80
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn prove(
        mut self,
        world: &World,
        road: &RoadNetwork,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        if self.junctions.iter().any(|p| p.elevation < SEA_LEVEL + 3.0) {
            return None;
        }
        let samples = (self.length * 2.0 + 1.0) as usize;
        let step = self.length / samples as f32;
        let mut run_start = 0;
        for sample in 0..=samples {
            if sample & 3 == 0 {
                pulse();
            }
            let distance = sample as f32 * step;
            if self.rock_cover(world, distance) {
                if run_start == 0 {
                    run_start = sample + 1;
                }
                continue;
            }
            if run_start == 0 {
                continue;
            }
            let first = (run_start - 1) as f32 * step;
            let last = (sample - 1) as f32 * step;
            run_start = 0;
            if last - first >= MIN_COVERED_LENGTH
                && self.clear_approach(world, first, false, pulse)
                && self.clear_approach(world, last, true, pulse)
                && self.keeps_bypass(world, road, first, last, pulse)
            {
                self.bore_start = first;
                self.bore_end = last;
                return Some(self);
            }
        }
        None
    }
}

#[derive(Clone, Copy)]
struct Mark {
    point: RoadPoint,
    tx: f32,
    tz: f32,
    distance: f32,
}

impl Mark {
    const EMPTY: Self = Self {
        point: RoadPoint {
            x: 0.0,
            z: 0.0,
            elevation: 0.0,
        },
        tx: 0.0,
        tz: 0.0,
        distance: 0.0,
    };
}

fn road_marks(
    world: &World,
    points: &[RoadPoint],
    marks: &mut [Mark; MAX_MARKS],
    pulse: &mut impl FnMut(),
) -> usize {
    let mut length = 0.0;
    for pair in points.windows(2) {
        let dx = pair[1].x - pair[0].x;
        let dz = pair[1].z - pair[0].z;
        length += sqrt(dx * dx + dz * dz);
    }
    let spacing = 6.0f32.max(length / (MAX_MARKS - 1) as f32);
    let mut traversed = 0.0;
    let mut used = 0;
    for pair in points.windows(2) {
        let [from, to] = [pair[0], pair[1]];
        let dx = to.x - from.x;
        let dz = to.z - from.z;
        let length = sqrt(dx * dx + dz * dz);
        if length < 0.001 {
            continue;
        }
        while used < MAX_MARKS && used as f32 * spacing <= traversed + length {
            if used & 7 == 0 {
                pulse();
            }
            let distance = used as f32 * spacing;
            let local = (distance - traversed) / length;
            let x = from.x + dx * local;
            let z = from.z + dz * local;
            marks[used] = Mark {
                point: RoadPoint {
                    x,
                    z,
                    elevation: world.elevation(x, z),
                },
                tx: dx / length,
                tz: dz / length,
                distance,
            };
            used += 1;
        }
        traversed += length;
    }
    used
}

#[derive(Clone, Copy)]
struct Candidate {
    score: f32,
    tunnel: RoadTunnel,
}

impl Candidate {
    const EMPTY: Self = Self {
        score: 0.0,
        tunnel: RoadTunnel {
            junctions: [Mark::EMPTY.point; 2],
            ax: 0.0,
            az: 0.0,
            length: 0.0,
            grade: 0.0,
            bore_start: 0.0,
            bore_end: 0.0,
        },
    };
}

fn roof_height(offset: f32) -> f32 {
    let t = (offset.abs() / TUNNEL_HALF_WIDTH).min(1.0);
    for index in 1..ARCH.len() {
        let (b, hb) = ARCH[index];
        if t <= b {
            let (a, ha) = ARCH[index - 1];
            return JAMB_HEIGHT + (ha + (hb - ha) * (t - a) / (b - a)) * ARCH_RISE;
        }
    }
    JAMB_HEIGHT
}

fn sqrt(value: f32) -> f32 {
    if value == 0.0 {
        return 0.0;
    }
    let estimate = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    let estimate = (estimate + value / estimate) * 0.5;
    (estimate + value / estimate) * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(seed: u64, rx: i32, rz: i32) -> (World, RoadNetwork, RoadTunnel) {
        let world = World::new(seed);
        let road = RoadNetwork::generate(&world, rx, rz).unwrap();
        let tunnel = RoadTunnel::generate(&world, seed, &road).unwrap();
        (world, road, tunnel)
    }

    #[test]
    fn tunnels_join_real_roads_and_bore_a_continuous_dry_ridge() {
        for (seed, rx, rz) in [(8, 0, 0), (42, -4, -4), (44, -11, 6)] {
            let (world, road, tunnel) = fixture(seed, rx, rz);
            assert_eq!(RoadTunnel::generate(&world, seed, &road), Some(tunnel));
            assert!(tunnel.grade().abs() <= MAX_GRADE);
            assert!(tunnel.covered_length() >= MIN_COVERED_LENGTH);
            for point in tunnel.junctions() {
                let highway = road.proximity(point.x, point.z).unwrap();
                assert!(highway.distance_squared < 0.000_1);
                assert_eq!(world.elevation(point.x, point.z), point.elevation);
                let approach = tunnel.approach_at(point.x, point.z).unwrap();
                assert!((approach.elevation - point.elevation).abs() < 0.001);
                assert!(approach.blend < 0.000_001);
            }
            let checks = (tunnel.covered_length() * 5.0) as usize;
            for index in 0..=checks {
                let along =
                    tunnel.bore_start + tunnel.covered_length() * index as f32 / checks as f32;
                for lane in -9..=9 {
                    let side = TUNNEL_HALF_WIDTH * lane as f32 / 9.0;
                    let point = tunnel.point(along, side);
                    let bore = tunnel.void_at(point.x, point.z).unwrap();
                    assert!(bore.floor > SEA_LEVEL + 3.0);
                    assert!(world.elevation(point.x, point.z) - bore.ceiling > ROCK_COVER - 0.18);
                }
            }
            for (start, end) in [
                (7.0, tunnel.bore_start),
                (tunnel.bore_end, tunnel.length - 7.0),
            ] {
                let checks = ((end - start) * 3.0) as usize;
                for index in 0..checks {
                    let along = start + (end - start) * index as f32 / checks as f32;
                    for lane in -5..=5 {
                        let point = tunnel.point(along, TUNNEL_HALF_WIDTH * lane as f32 / 5.0);
                        if road
                            .proximity(point.x, point.z)
                            .is_some_and(|p| p.distance_squared < 2.2 * 2.2)
                        {
                            assert!((world.elevation(point.x, point.z) - point.y).abs() < 0.95);
                        }
                    }
                }
            }
            for along in [1.0, tunnel.length - 1.0] {
                let point = tunnel.point(along, 0.0);
                assert!(tunnel.void_at(point.x, point.z).is_none());
                assert!(tunnel.approach_at(point.x, point.z).is_some());
            }
        }
    }

    #[test]
    fn the_fascia_surrounds_the_same_open_arch_as_the_bore() {
        let (_, _, tunnel) = fixture(8, 0, 0);
        let [near, far] = tunnel.mouths();
        assert_eq!(near.inward_x, -far.inward_x);
        assert_eq!(near.inward_z, -far.inward_z);
        for mouth in [near, far] {
            let opening = mouth.opening();
            let frame = mouth.frame();
            for index in 1..10 {
                let point = opening[index];
                let bore = tunnel.void_at(point.x, point.z).unwrap();
                assert!((point.y - bore.ceiling).abs() < 0.01);
                assert_eq!(frame[index][1], point);
            }
            for [_, left, right, _] in frame {
                let radius_x = (left.x + right.x) / 2.0;
                let radius_z = (left.z + right.z) / 2.0;
                let bore = tunnel.void_at(radius_x, radius_z).unwrap();
                let inside_edge = (left.y + right.y) / 2.0;
                let side = tunnel.project(radius_x, radius_z).1;
                assert!(
                    (inside_edge - bore.ceiling).abs() < 0.01
                        || side.abs() >= TUNNEL_HALF_WIDTH - 0.01
                );
            }
            let opening_center = tunnel.void_at(mouth.center.x, mouth.center.z).unwrap();
            assert!((opening_center.floor - mouth.center.y).abs() < 0.001);
            assert!(opening_center.ceiling - opening_center.floor > 4.4);
        }
    }
}
