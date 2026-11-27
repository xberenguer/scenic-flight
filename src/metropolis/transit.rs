//! The city rail network, including the timetable shared by rendering and
//! flight encounters. Positions use the same warped street grid as the city.

use super::{floor, Airport, Metropolis};
use crate::flight::TICKS_PER_SECOND;
use crate::math::{mix, smoothstep};
use crate::rail::RailPoint;
use crate::render::{Camera, ScreenProjection, ScreenRect};
use crate::train_art::{Train, TRAIN_RENDER_REACH};
use crate::train_follow::TrainMotion;
use crate::world::{Sample, Surface, World};
use crate::world_mesh::{bounds_in_range, WorldMesh};

const STOP_GAP: f32 = 864.0;
const END: f32 = 8.0 * STOP_GAP;
const TRACK_HALF: f32 = 7.2;
const TRACK_OFFSET: f32 = 2.65;
const REACH: f32 = 630.0;
const SECTION: f32 = 66.0;
const PLATFORM: f32 = 58.0;
const END_CLEARANCE: f32 = 54.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetroLine {
    Crosstown,
    Central,
    Airport,
}

impl MetroLine {
    pub const ALL: [Self; 3] = [Self::Crosstown, Self::Central, Self::Airport];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Crosstown => "Crosstown",
            Self::Central => "Central",
            Self::Airport => "Airport",
        }
    }

    pub const fn color(self) -> [u8; 3] {
        match self {
            Self::Crosstown => [74, 126, 177],
            Self::Central => [187, 100, 82],
            Self::Airport => [208, 172, 75],
        }
    }

    fn deck(self) -> f32 {
        match self {
            Self::Crosstown => 9.0,
            Self::Central | Self::Airport => 23.0,
        }
    }

    fn fleet_size(self) -> u64 {
        match self {
            Self::Crosstown => 12,
            Self::Central => 10,
            Self::Airport => 7,
        }
    }

    fn dwell(self) -> u64 {
        match self {
            Self::Airport => 8,
            _ => 6,
        }
    }
}

#[derive(Clone, Copy)]
pub struct MetroTransit {
    city: Metropolis,
    airport_axis: f32,
    airport_end: f32,
    airport_u: f32,
    first_regular: i32,
}

/// Scheduled position centered on one of the two service tracks.
#[derive(Clone, Copy)]
struct Departure {
    at: f32,
    direction: f32,
    speed: f32,
}

impl MetroTransit {
    pub(super) fn new(city: Metropolis) -> Self {
        let [east, north] = Airport::new(city.seed).point([0.0, 515.0]);
        let (airport_u, airport_end) = city.street_axes_from(east, north);
        let airport_axis = nearest(airport_u, STOP_GAP);
        let first_regular = floor((airport_end + 490.0) / STOP_GAP) + 1;
        Self {
            city,
            airport_axis,
            airport_end,
            airport_u,
            first_regular,
        }
    }

    pub fn stations(self, line: MetroLine) -> usize {
        match line {
            MetroLine::Crosstown | MetroLine::Central => 17,
            MetroLine::Airport => (6 - self.first_regular) as usize,
        }
    }

    fn station(self, line: MetroLine, index: usize) -> f32 {
        match line {
            MetroLine::Crosstown | MetroLine::Central => index as f32 * STOP_GAP - END,
            MetroLine::Airport if index == 0 => self.airport_end,
            MetroLine::Airport => (self.first_regular + index as i32 - 1) as f32 * STOP_GAP,
        }
    }

    pub fn station_position(self, line: MetroLine, index: usize) -> Option<[f32; 2]> {
        (index < self.stations(line)).then(|| self.position(line, self.station(line, index), 0.0))
    }

    /// Convert an unbanked rail center to world coordinates for the route map.
    pub fn route_position(self, line: MetroLine, fraction: f32) -> [f32; 2] {
        let [start, end] = self.station_limits(line);
        self.position(line, start + fraction.clamp(0.0, 1.0) * (end - start), 0.0)
    }

    pub(super) fn approach(self) -> (f32, f32, f32) {
        let line = MetroLine::Crosstown;
        let target = (0..line.fleet_size())
            .map(|vehicle| self.departure(line, vehicle, 360))
            .min_by(|left, right| {
                (left.at + 4_400.0)
                    .abs()
                    .total_cmp(&(right.at + 4_400.0).abs())
            })
            .unwrap();
        let [x, z] = self.position(line, target.at - target.direction * 135.0, -82.0);
        let [a, b] = self.position(line, target.at + target.direction * 48.0, 0.0);
        (x, z, crate::flight::bearing(a - x, b - z))
    }

    fn station_limits(self, line: MetroLine) -> [f32; 2] {
        [
            self.station(line, 0),
            self.station(line, self.stations(line) - 1),
        ]
    }

    fn airport_cross(self, along: f32) -> f32 {
        let blend = smoothstep((along - self.airport_end) / 1_180.0);
        self.airport_u + (self.airport_axis - self.airport_u) * blend
    }

    fn axes(self, line: MetroLine, at: f32, offset: f32) -> (f32, f32) {
        match line {
            MetroLine::Crosstown => (at, offset),
            MetroLine::Central => (offset, at),
            MetroLine::Airport => (self.airport_cross(at) + offset, at),
        }
    }

    fn position(self, line: MetroLine, at: f32, offset: f32) -> [f32; 2] {
        let (u, v) = self.axes(line, at, offset);
        self.city.from_street_axes(u, v)
    }

    fn local(self, line: MetroLine, u: f32, v: f32) -> (f32, f32) {
        match line {
            MetroLine::Crosstown => (u, v),
            MetroLine::Central => (v, u),
            MetroLine::Airport => (v, u - self.airport_cross(v)),
        }
    }

    fn near_station(self, line: MetroLine, along: f32) -> f32 {
        let regular = (along - nearest(along, STOP_GAP)).abs();
        if line != MetroLine::Airport {
            return regular;
        }
        (along - self.airport_end).abs().min(
            if along >= self.first_regular as f32 * STOP_GAP - PLATFORM {
                regular
            } else {
                f32::MAX
            },
        )
    }

    fn includes(self, line: MetroLine, along: f32) -> bool {
        let [start, end] = self.station_limits(line);
        (start - END_CLEARANCE..=end + END_CLEARANCE).contains(&along)
    }

    pub(super) fn paint(self, u: f32, v: f32, ground: Sample) -> Option<Sample> {
        if u.abs() > END + END_CLEARANCE || v.abs() > END + END_CLEARANCE {
            return None;
        }
        let mut paved = false;
        let mut street_center = false;
        let mut roof = None;
        for line in MetroLine::ALL {
            let (along, across) = self.local(line, u, v);
            let side = across.abs();
            if side > 12.8 || !self.includes(line, along) {
                continue;
            }
            let station = self.near_station(line, along);
            if (8.8..=12.8).contains(&side) && station <= PLATFORM - 7.0 {
                let platform = Sample {
                    elevation: ground.elevation + line.deck() + 4.9,
                    surface: match line {
                        MetroLine::Crosstown => Surface::RailGlass,
                        MetroLine::Central => Surface::RailRust,
                        MetroLine::Airport => Surface::RailMetal,
                    },
                };
                roof = Some(roof.map_or(platform, |old: Sample| {
                    if old.elevation >= platform.elevation {
                        old
                    } else {
                        platform
                    }
                }));
            }
            street_center |= side <= 2.9;
            paved |= side <= TRACK_HALF || station <= PLATFORM;
        }
        if !street_center && roof.is_some() {
            return roof;
        }
        paved.then_some(Sample {
            surface: Surface::Pavement,
            ..ground
        })
    }

    pub(super) fn obstruction(self, u: f32, v: f32, ground: f32) -> f32 {
        if u.abs() > END + END_CLEARANCE || v.abs() > END + END_CLEARANCE {
            return ground;
        }
        MetroLine::ALL
            .into_iter()
            .filter(|&line| {
                let (along, across) = self.local(line, u, v);
                across.abs() <= TRACK_HALF && self.includes(line, along)
            })
            .map(|line| ground + line.deck() + 4.3)
            .fold(ground, f32::max)
    }

    fn leg_time(self, line: MetroLine, leg: usize) -> u64 {
        let edge = self.station(line, leg + 1) - self.station(line, leg);
        (edge / 29.0) as u64 + 4 + line.dwell()
    }

    fn departure(self, line: MetroLine, vehicle: u64, ticks: u64) -> Departure {
        let legs = self.stations(line) - 1;
        let hz = u64::from(TICKS_PER_SECOND);
        let regular = self.leg_time(line, legs - 1) * hz;
        let first = self.leg_time(line, 0) * hz;
        let one_way = first + (legs as u64 - 1) * regular;
        let period = 2 * one_way;
        let offset = mix(self.city.seed ^ (line as u64 + 0x9e37));
        let phase =
            (ticks % period + (offset % (9 * hz)) + vehicle * period / line.fleet_size()) % period;
        let reverse = phase >= one_way;
        let elapsed = phase % one_way;
        let (leg, within, duration) = if !reverse {
            if elapsed < first {
                (0, elapsed, first)
            } else {
                let rest = elapsed - first;
                (1 + (rest / regular) as usize, rest % regular, regular)
            }
        } else {
            let back = (legs as u64 - 1) * regular;
            if elapsed >= back {
                (0, elapsed - back, first)
            } else {
                (
                    legs - 1 - (elapsed / regular) as usize,
                    elapsed % regular,
                    regular,
                )
            }
        };
        let dwell = line.dwell() * hz;
        let movement = (duration - dwell) as f32;
        let progress = within.saturating_sub(dwell) as f32 / movement;
        let ramp = (3.0 * hz as f32 / movement).min(0.24);
        let speed_factor = (progress / ramp).min((1.0 - progress) / ramp).min(1.0);
        let scale = 1.0 / (1.0 - ramp);
        let along = if progress < ramp {
            progress * progress * (0.5 / ramp) * scale
        } else if progress > 1.0 - ramp {
            let rest = 1.0 - progress;
            1.0 - rest * rest * (0.5 / ramp) * scale
        } else {
            (progress - 0.5 * ramp) * scale
        };
        let low = self.station(line, leg);
        let high = self.station(line, leg + 1);
        let direction = if reverse { -1.0 } else { 1.0 };
        Departure {
            at: if reverse {
                high - (high - low) * along
            } else {
                low + (high - low) * along
            },
            direction,
            speed: speed_factor * scale * (high - low) * hz as f32 / movement,
        }
    }

    fn rail_point(self, world: &World, line: MetroLine, at: f32, offset: f32) -> RailPoint {
        let [x, z] = self.position(line, at, offset);
        RailPoint {
            x,
            z,
            elevation: world.elevation(x, z) + line.deck(),
        }
    }

    fn lane_offset(self, line: MetroLine, at: f32) -> f32 {
        let [start, end] = self.station_limits(line);
        let open = (at - start).min(end - at).max(0.0);
        TRACK_OFFSET * smoothstep(open / 112.0)
    }

    fn vehicle_point(self, world: &World, line: MetroLine, at: f32, direction: f32) -> RailPoint {
        self.rail_point(world, line, at, self.lane_offset(line, at) * direction)
    }

    fn ranges(self, camera: Camera, line: MetroLine, reach: f32) -> Option<(f32, f32)> {
        let (u, v) = self.city.street_axes(camera.x, camera.z);
        let (along, across) = self.local(line, u, v);
        if across.abs() > reach * 1.45 + 35.0 {
            return None;
        }
        let [start, end] = self.station_limits(line);
        let low = (along - reach * 1.35).max(start - END_CLEARANCE);
        let high = (along + reach * 1.35).min(end + END_CLEARANCE);
        (low < high).then_some((low, high))
    }

    fn services(
        self,
        camera: Camera,
        ticks: u64,
        reach: f32,
    ) -> impl Iterator<Item = (MetroLine, u64, Departure)> {
        MetroLine::ALL
            .into_iter()
            .filter_map(move |line| {
                let (first, last) = self.ranges(camera, line, reach)?;
                Some((line, first, last))
            })
            .flat_map(move |(line, first, last)| {
                (0..line.fleet_size()).filter_map(move |vehicle| {
                    let departure = self.departure(line, vehicle, ticks);
                    (first..=last)
                        .contains(&departure.at)
                        .then_some((line, vehicle, departure))
                })
            })
    }

    pub fn trains(
        self,
        world: &World,
        camera: Camera,
        dimensions: (usize, usize),
        ticks: u64,
    ) -> impl Iterator<Item = Train> + '_ {
        self.services(camera, ticks, TRAIN_RENDER_REACH)
            .filter_map(move |(line, _, departure)| {
                let cars = if line == MetroLine::Airport { 4 } else { 3 };
                Train::on_metro(
                    departure.at,
                    departure.direction,
                    cars,
                    line.color(),
                    camera,
                    dimensions,
                    |at| self.vehicle_point(world, line, at, departure.direction),
                )
            })
    }

    pub fn motions(
        self,
        world: &World,
        camera: Camera,
        ticks: u64,
    ) -> impl Iterator<Item = TrainMotion> + '_ {
        self.services(camera, ticks, 140.0)
            .filter_map(move |(line, vehicle, departure)| {
                if departure.speed < 2.0 {
                    return None;
                }
                let a = self.vehicle_point(world, line, departure.at, departure.direction);
                let b = self.vehicle_point(
                    world,
                    line,
                    departure.at + departure.direction * 5.0,
                    departure.direction,
                );
                let (sin, cos) = crate::math::sin_cos(crate::flight::bearing(b.x - a.x, b.z - a.z));
                Some(TrainMotion {
                    key: mix(self.city.seed ^ (line as u64 * 32 + vehicle + 0x937ec4)),
                    position: [a.x, a.elevation, a.z],
                    direction: [sin, cos],
                    speed: departure.speed,
                    passenger: true,
                })
            })
    }

    fn sections(
        self,
        world: &World,
        camera: Camera,
        mut visit: impl FnMut(MetroLine, f32, f32, [f32; 3], [f32; 3]),
    ) {
        for line in MetroLine::ALL {
            let Some((low, high)) = self.ranges(camera, line, REACH) else {
                continue;
            };
            let [start, end] = self.station_limits(line);
            let low = (floor(low / SECTION) as f32 * SECTION).max(start - END_CLEARANCE);
            let high = (floor(high / SECTION) as f32 * SECTION + SECTION).min(end + END_CLEARANCE);
            let mut at = low;
            while at < high {
                let next = (at + SECTION).min(high);
                let [ax, az] = self.position(line, at, 0.0);
                let [bx, bz] = self.position(line, next, 0.0);
                let bounds = [
                    ax.min(bx) - 12.0,
                    az.min(bz) - 12.0,
                    ax.max(bx) + 12.0,
                    az.max(bz) + 12.0,
                ];
                if bounds_in_range(camera, bounds, REACH) {
                    let y = world.elevation(ax, az) + line.deck();
                    let top = world.elevation(bx, bz) + line.deck();
                    visit(line, at, next, [ax, y, az], [bx, top, bz]);
                }
                at = next;
            }
        }
    }

    pub fn depth_region(
        self,
        world: &World,
        camera: Camera,
        dimensions: (usize, usize),
    ) -> Option<ScreenRect> {
        let projection = ScreenProjection::new(camera, dimensions)?;
        let mut region = None;
        self.sections(world, camera, |line, _, _, a, b| {
            let bounds = [
                [
                    a[0].min(b[0]) - 12.0,
                    a[1].min(b[1]) - line.deck(),
                    a[2].min(b[2]) - 12.0,
                ],
                [
                    a[0].max(b[0]) + 12.0,
                    a[1].max(b[1]) + 1.0,
                    a[2].max(b[2]) + 12.0,
                ],
            ];
            if let Some(rect) = projection.bounds(bounds) {
                region = Some(region.map_or(rect, |prior: ScreenRect| prior.union(rect)));
            }
        });
        region
    }

    pub fn draw(self, world: &World, camera: Camera, mesh: &mut WorldMesh<'_>) {
        self.sections(world, camera, |line, at, next, a, b| {
            let dx = a[0] - camera.x;
            let dz = a[2] - camera.z;
            let range = dx * dx + dz * dz;
            let alpha = 1.0 - smoothstep((range - 460.0 * 460.0) / (REACH * REACH - 460.0 * 460.0));
            let platform_distance = self.near_station(line, at + SECTION * 0.5);
            let station = platform_distance <= PLATFORM;
            let width = TRACK_HALF + 2.2 * (1.0 - smoothstep((platform_distance - 49.0) / 10.0));
            let heading = crate::flight::bearing(b[0] - a[0], b[2] - a[2]);
            let (sin, cos) = crate::math::sin_cos(heading);
            let edge =
                |p: [f32; 3], side, down| [p[0] + side * cos, p[1] - down, p[2] - side * sin];
            let [left_a, right_a, right_b, left_b] = [
                edge(a, -width, 0.0),
                edge(a, width, 0.0),
                edge(b, width, 0.0),
                edge(b, -width, 0.0),
            ];
            mesh.faded_quad([left_a, right_a, right_b, left_b], [106, 108, 109], alpha);
            for side in [-width, width] {
                mesh.faded_quad(
                    [
                        edge(a, side, 0.0),
                        edge(b, side, 0.0),
                        edge(b, side, 2.2),
                        edge(a, side, 2.2),
                    ],
                    if station { line.color() } else { [81, 88, 96] },
                    alpha,
                );
            }
            if alpha > 0.15 {
                let rail = |at, y, side| {
                    let [x, z] = self.position(line, at, side);
                    [x, y + 0.18, z]
                };
                let a_lane = self.lane_offset(line, at);
                let b_lane = self.lane_offset(line, next);
                for lane in [-1.0, 1.0] {
                    if lane > 0.0 && a_lane == 0.0 && b_lane == 0.0 {
                        continue;
                    }
                    for head in [-1.32, 1.32] {
                        mesh.faded_segment(
                            [
                                rail(at, a[1], lane * a_lane + head),
                                rail(next, b[1], lane * b_lane + head),
                            ],
                            0.48,
                            [152, 165, 170],
                            alpha,
                        );
                    }
                }
            }
            let (u, v) = self.city.street_axes(a[0], a[2]);
            // The high deck spans an interchange without a pillar through
            // the lower track or its train clearance.
            if line == MetroLine::Crosstown || v.abs() >= 21.0 || u.abs() > END {
                let foot = [a[0], a[1] - line.deck() + 0.2, a[2]];
                mesh.faded_segment([foot, a], 4.6, [108, 109, 105], alpha);
            }
        });
    }
}

fn nearest(value: f32, spacing: f32) -> f32 {
    floor(value / spacing + 0.5) as f32 * spacing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trains_dwell_at_each_stop_and_reverse_continuously() {
        let network = World::for_flight_seed(15).metro_transit().unwrap();
        let hz = u64::from(TICKS_PER_SECOND);
        for line in MetroLine::ALL {
            let [start, end] = network.station_limits(line);
            let transit = (0..network.stations(line) - 1)
                .map(|leg| network.leg_time(line, leg))
                .sum::<u64>()
                * hz;
            let phase_offset = mix(network.city.seed ^ (line as u64 + 0x9e37)) % (9 * hz);
            let base = transit * 2 - phase_offset;
            let mut time = 0;
            for stop in 0..network.stations(line) {
                let dwell = network.departure(line, 0, base + time + 2 * hz);
                assert_eq!(dwell.at, network.station(line, stop));
                assert_eq!(dwell.speed, 0.0);
                if stop + 1 < network.stations(line) {
                    let moving = network.departure(line, 0, base + time + (line.dwell() + 7) * hz);
                    assert!(moving.at > dwell.at && moving.speed > 14.0);
                    time += network.leg_time(line, stop) * hz;
                }
            }
            let before = network.departure(line, 0, base + transit - 1);
            let after = network.departure(line, 0, base + transit);
            assert_eq!((before.direction, after.direction), (1.0, -1.0));
            assert!((before.at - after.at).abs() < 0.03 && (after.at - end).abs() < 0.01);
            assert_eq!(network.lane_offset(line, start), 0.0);
            assert_eq!(network.lane_offset(line, end), 0.0);

            let city = network.city;
            for tick in (0..transit * 2).step_by(99) {
                for a in 0..line.fleet_size() {
                    let one = network.departure(line, a, tick);
                    for b in a + 1..line.fleet_size() {
                        let two = network.departure(line, b, tick);
                        let separation = (one.at - two.at).abs();
                        let join = |p: Departure| network.lane_offset(line, p.at) < 2.1;
                        if one.direction == two.direction || join(one) || join(two) {
                            assert!(separation > 65.0, "{:?} {line:?}: {separation}", city.seed);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn crossings_are_shared_stations_with_two_clear_decks_and_airport_access() {
        for seed in [0, 15, 19, 23, 40] {
            let world = World::for_flight_seed(seed);
            let city = world.metropolis().unwrap();
            let rail = city.transit();
            assert_eq!(
                rail.station_position(MetroLine::Crosstown, 8),
                rail.station_position(MetroLine::Central, 8)
            );
            let q = (rail.airport_axis / STOP_GAP) as i32 + 8;
            let airport_transfer = (1 - rail.first_regular) as usize;
            assert_eq!(
                rail.station_position(MetroLine::Crosstown, q as usize),
                rail.station_position(MetroLine::Airport, airport_transfer)
            );

            let [x, z] = rail.station_position(MetroLine::Airport, 0).unwrap();
            let (east, north) = city.axes(x, z);
            let [local_x, local_z] = Airport::new(city.seed).local(east, north);
            assert!(local_x.abs() < 0.02 && (local_z - 515.0).abs() < 0.02);
            assert!(MetroLine::Airport.deck() - MetroLine::Crosstown.deck() > 10.0);
            assert_eq!(rail.stations(MetroLine::Central), 17);
            assert!(rail.stations(MetroLine::Airport) >= 9);

            for line in MetroLine::ALL {
                for index in 0..=160 {
                    let [x, z] = rail.route_position(line, index as f32 / 160.0);
                    assert!(city.contains(x, z) && world.elevation(x, z) > 3.0);
                }
            }
        }
    }

    #[test]
    fn timetable_renders_a_train_and_an_elevated_deck_in_the_same_scene() {
        use crate::sky::Sky;
        const SIDE: usize = 88;
        let world = World::for_flight_seed(19);
        let rail = world.metro_transit().unwrap();
        let line = MetroLine::Crosstown;
        let phase = rail.departure(line, 0, 0);
        let center = rail.vehicle_point(&world, line, phase.at, phase.direction);
        let camera = Camera {
            x: center.x - 65.0,
            z: center.z - 100.0,
            altitude: center.elevation + 39.0,
            heading: crate::flight::bearing(65.0, 100.0),
            pitch: 0.31,
            bank: 0.0,
        };
        let dimensions = (SIDE, SIDE);
        assert!(rail.depth_region(&world, camera, dimensions).is_some());
        let train = rail.trains(&world, camera, dimensions, 0).next().unwrap();
        assert!(train.is_near());
        let mut pixels = std::vec![0; SIDE * SIDE];
        let mut depth = std::vec![u16::MAX; SIDE * SIDE];
        let sky = Sky::from_time(19, 0, 0.42);
        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, dimensions, camera, &sky).unwrap();
        rail.draw(&world, camera, &mut mesh);
        drop(mesh);
        let before = pixels.clone();
        assert!(before.iter().filter(|&&p| p != 0).count() > 10);
        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, dimensions, camera, &sky).unwrap();
        train.draw(&mut mesh, sky.daylight);
        assert_ne!(pixels, before);
    }
}
