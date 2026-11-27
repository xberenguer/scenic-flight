//! Regional city plan indexed directly in world space, with no streaming seams.

use crate::math::{fraction, mix, sin_cos, smoothstep};
use crate::world::{Sample, Surface};
mod airport;
mod outskirts;
pub(crate) mod transit;
use airport::{Airport, NORTH_LIMIT};
use transit::MetroTransit;

const BLOCK: f32 = 96.0;
const CITY_RADIUS: f32 = 11_500.0;
const CENTERS: [(f32, f32, f32); 4] = [
    (-0.10, 0.02, 0.21),
    (0.42, 0.31, 0.13),
    (-0.52, -0.20, 0.11),
    (0.24, -0.53, 0.11),
];

#[derive(Clone, Copy)]
pub(crate) struct Metropolis {
    seed: u64,
    rotation: (f32, f32),
    aspect: f32,
    edge_bias: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum District {
    Residential,
    Mixed,
    Center,
    Industry,
    Campus,
}

#[derive(Clone, Copy)]
enum Form {
    Bar,
    Pair,
    Court,
    Corner,
    Terrace,
    Offset,
    Cross,
    Cluster,
    Stepped,
    Shed,
}

impl Form {
    // Local coordinates centered on a block. Road space is kept clear by the caller.
    fn height(self, x: f32, z: f32) -> f32 {
        let a = x.abs();
        let b = z.abs();
        match self {
            Self::Bar => inside(a < 0.33 && b < 0.17),
            Self::Pair => inside((a - 0.21).abs() < 0.104 && b < 0.29),
            Self::Court => inside(a < 0.34 && b < 0.33 && (a > 0.21 || b > 0.20)),
            Self::Corner => inside(a < 0.33 && b < 0.33 && (x < -0.13 || z > 0.16)),
            Self::Terrace => {
                inside(a < 0.35 && b < 0.26) * 0.32 + inside(a < 0.21 && b < 0.16) * 0.68
            }
            Self::Offset => {
                inside((x + 0.15).abs() < 0.17 && (z - 0.12).abs() < 0.17)
                    + inside((x - 0.19).abs() < 0.125 && (z + 0.12).abs() < 0.19) * 0.72
            }
            Self::Cross => inside((a < 0.31 && b < 0.11) || (a < 0.125 && b < 0.30)),
            Self::Cluster => {
                let lot_x = (a - 0.19).abs();
                let lot_z = (b - 0.19).abs();
                inside(lot_x < 0.10 && lot_z < 0.12) * if x * z > 0.0 { 1.0 } else { 0.72 }
            }
            Self::Stepped => {
                inside(a < 0.31 && b < 0.31) * 0.23
                    + inside((x + 0.07).abs() < 0.22 && b < 0.21) * 0.33
                    + inside((x + 0.13).abs() < 0.13 && b < 0.12) * 0.44
            }
            Self::Shed => {
                inside(a < 0.35 && b < 0.26) * (1.0 - 3.0 * ((x + 0.35) * 6.0 % 1.0).min(0.15))
            }
        }
    }
}

fn inside(condition: bool) -> f32 {
    u8::from(condition) as f32
}

#[derive(Clone, Copy)]
struct BlockDesign {
    form: Form,
    height: f32,
    roof: Surface,
}

const fn block(form: Form, height: f32, roof: Surface) -> BlockDesign {
    BlockDesign { form, height, roof }
}

use Form::*;
use Surface::*;

// Five locally related forms per district. Rotation, mirrored plans, lot height
// and regional materials vary within each family.
const DESIGNS: [[BlockDesign; 5]; 5] = [
    [
        block(Cluster, 5.5, PitchedRoof),
        block(Pair, 6.0, PitchedRoof),
        block(Court, 8.5, LandmarkRoof03),
        block(Offset, 7.0, LandmarkRoof09),
        block(Bar, 7.5, LandmarkRoof22),
    ],
    [
        block(Court, 14.0, LandmarkRoof05),
        block(Terrace, 24.0, FlatRoof),
        block(Corner, 15.0, LandmarkRoof19),
        block(Pair, 18.0, LandmarkRoof27),
        block(Offset, 20.0, LandmarkRoof35),
    ],
    [
        block(Stepped, 69.0, TowerRoof),
        block(Terrace, 85.0, LandmarkRoof39),
        block(Cross, 54.0, LandmarkRoof41),
        block(Pair, 56.0, LandmarkRoof32),
        block(Offset, 65.0, LandmarkRoof45),
    ],
    [
        block(Shed, 12.0, IndustryMetal),
        block(Bar, 15.0, IndustryConcrete),
        block(Pair, 13.0, IndustryBrick),
        block(Corner, 19.0, IndustryMetal),
        block(Court, 11.0, IndustryBrick),
    ],
    [
        block(Cross, 12.0, LandmarkRoof11),
        block(Court, 16.0, LandmarkRoof28),
        block(Cluster, 13.0, LandmarkRoof26),
        block(Corner, 14.0, LandmarkRoof44),
        block(Terrace, 18.0, LandmarkRoof49),
    ],
];

impl Metropolis {
    pub(crate) fn new(seed: u64) -> Self {
        let angle = (fraction(seed) - 0.5) * 1.4;
        let rotation = sin_cos(angle);
        let aspect = 0.88 + ((seed >> 20) & 1023) as f32 * (0.24 / 1023.0);
        let edge_bias = (((seed >> 8) & 1023) as f32 / 1023.0 - 0.5) * 0.42;
        Self {
            seed,
            rotation,
            aspect,
            edge_bias,
        }
    }

    fn from_axes(self, east: f32, north: f32) -> [f32; 2] {
        let (sine, cosine) = self.rotation;
        let north = north * self.aspect;
        [east * cosine - north * sine, north * cosine + east * sine]
    }

    pub(crate) fn airport_departure(self) -> Option<(f32, f32, f32)> {
        if self.seed & 3 != 0 {
            return None;
        }
        let airport = Airport::new(self.seed);
        let [x, z] = airport.point([-790.0, -587.0]);
        let [x, z] = self.from_axes(x, z);
        let [a, b] = self.from_axes(990.0, 680.0);
        Some((x, z, crate::flight::bearing(a, b)))
    }

    fn axes(self, x: f32, z: f32) -> (f32, f32) {
        let (sine, cosine) = self.rotation;
        (x * cosine + z * sine, (z * cosine - x * sine) / self.aspect)
    }

    fn basin_axes(self, east: f32, north: f32) -> (f32, f32) {
        let u = east / 15_400.0;
        let v = north / 15_400.0 - (0.13 + self.aspect * 0.045) * u * u;
        (u, v)
    }

    fn fringe_grade(self, east: f32, north: f32) -> f32 {
        let (e, n) = self.basin_axes(east, north);
        e * e + n * n
    }

    pub(crate) fn fringe_departure(self) -> Option<(f32, f32, f32)> {
        if self.seed & 7 != 5 {
            return None;
        }
        let side = if self.seed & 256 == 0 { 1.0 } else { -1.0 };
        let point = |distance: f32| {
            let distance = side * distance;
            if self.seed & 128 == 0 {
                self.from_street_axes(0.0, distance)
            } else {
                self.from_street_axes(distance, 0.0)
            }
        };
        let [x, z] = point(12_570.0);
        let [a, b] = point(11_400.0);
        Some((x, z, crate::flight::bearing(a - x, b - z)))
    }

    pub(crate) fn basin(self, x: f32, z: f32) -> Option<(f32, f32)> {
        if x.abs().max(z.abs()) > 23_000.0 {
            return None;
        }
        let (east, north) = self.axes(x, z);
        let (u, v) = self.basin_axes(east, north);
        let radius = u * u + v * v;
        let original_weight = smoothstep((1.63 - radius) / 0.79);
        if original_weight == 0.0 {
            return None;
        }
        let weight = if original_weight < 1.0 && radius < 1.08 {
            let (road_u, road_v) = self.street_axes_from(east, north);
            let roadbed = outskirts::roadbed_influence(road_u, road_v, radius);
            original_weight + (1.0 - original_weight) * roadbed
        } else {
            original_weight
        };
        let rolling = sin_cos(u * 9.0 + v * 4.7).0 * 4.4 + sin_cos(v * 26.0 - u * 18.0).0 * 1.6;
        let foothills = (radius - 0.62).clamp(0.0, 1.0);
        let height = 19.0 + v * 8.0 + u * 3.0 + rolling + 46.0 * foothills * foothills;
        let airport = Airport::new(self.seed);
        Some((airport.grade(airport.local(east, north), height), weight))
    }

    fn street_axes_from(self, east: f32, north: f32) -> (f32, f32) {
        let bending = north / CITY_RADIUS;
        let u = east + bending * bending * 2_500.0;
        let bend = u / CITY_RADIUS;
        (u, north + bend * bend * 1_400.0)
    }

    fn from_street_axes(self, u: f32, v: f32) -> [f32; 2] {
        let bend = u / CITY_RADIUS;
        let north = v - bend * bend * 1_400.0;
        let bending = north / CITY_RADIUS;
        self.from_axes(u - bending * bending * 2_500.0, north)
    }

    pub(crate) fn transit(self) -> MetroTransit {
        MetroTransit::new(self)
    }

    pub(crate) fn rail_departure(self) -> Option<(f32, f32, f32)> {
        (self.seed & 7 == 3).then(|| self.transit().approach())
    }

    pub(crate) fn rail_clearance(self, rail: MetroTransit, x: f32, z: f32, ground: f32) -> f32 {
        let (u, v) = self.street_axes(x, z);
        rail.obstruction(u, v, ground)
    }

    fn street_axes(self, x: f32, z: f32) -> (f32, f32) {
        let (east, north) = self.axes(x, z);
        self.street_axes_from(east, north)
    }

    fn airport_road(self, airport: Airport, [x, z]: [f32; 2], street_north: f32) -> bool {
        if x.abs() > 16.0 || !(NORTH_LIMIT..=NORTH_LIMIT + 230.0).contains(&z) {
            return false;
        }
        let [exit_east, exit_north] = airport.point([0.0, NORTH_LIMIT]);
        let (_, terminal_north) = self.street_axes_from(exit_east, exit_north);
        let junction = (floor((terminal_north + 110.0) / BLOCK) + 1) as f32 * BLOCK;
        street_north <= junction
    }

    fn district(self, i: i32, j: i32) -> Option<District> {
        let x = (i as f32 + 0.5) * BLOCK / CITY_RADIUS;
        let z = (j as f32 + 0.5) * BLOCK / CITY_RADIUS;
        let skew = x + z * self.edge_bias;
        let outline = skew * skew + z * z;
        if outline > 0.81
            && outline
                > 0.97
                    + 0.095 * sin_cos(x * 13.0 + z * 5.0).0
                    + 0.065 * sin_cos(z * 18.0 - x * 11.0).0
        {
            return None;
        }

        let near = |cx: f32, cz: f32, radius: f32| {
            (x - cx) * (x - cx) + (z - cz) * (z - cz) < radius * radius
        };
        let center = CENTERS.iter().any(|&(cx, cz, radius)| near(cx, cz, radius));
        let mixed = outline < 0.30
            || (z - x * 0.35 - 0.16).abs() < 0.08
            || (x + z * 0.29 + 0.29).abs() < 0.06;
        let industry = near(0.49, -0.54, 0.25) || near(-0.48, 0.39, 0.19) || near(0.54, 0.16, 0.13);
        let campus = near(-0.39, -0.46, 0.16) || near(0.21, 0.51, 0.16) || near(0.36, -0.08, 0.12);
        Some(if center {
            District::Center
        } else if industry {
            District::Industry
        } else if campus {
            District::Campus
        } else if mixed {
            District::Mixed
        } else {
            District::Residential
        })
    }

    pub(crate) fn contains(self, x: f32, z: f32) -> bool {
        if x.abs().max(z.abs()) > 15_000.0 {
            return false;
        }
        let (east, north) = self.axes(x, z);
        if Airport::contains(Airport::new(self.seed).local(east, north)) {
            return true;
        }
        let (east, north) = self.street_axes_from(east, north);
        self.district(floor(east / BLOCK), floor(north / BLOCK))
            .is_some()
    }

    pub(crate) fn owns(self, x: f32, z: f32) -> bool {
        if x.abs().max(z.abs()) > 16_800.0 {
            return false;
        }
        if self.contains(x, z) {
            return true;
        }
        let (east, north) = self.axes(x, z);
        let (u, v) = self.street_axes_from(east, north);
        let grade = self.fringe_grade(east, north);
        outskirts::eligible(self.seed, u, v, grade) || outskirts::road(u, v, grade)
    }

    // Look beyond individual roofs so a ridge of tall blocks does not cause
    // alternating dives and abrupt clearance corrections down each street.
    pub(crate) fn overflight_height(self, x: f32, z: f32) -> f32 {
        let (east, north) = self.street_axes(x, z);
        let (u, v) = (east / CITY_RADIUS, north / CITY_RADIUS);
        let height = CENTERS
            .into_iter()
            .fold(0.0_f32, |height, (cx, cz, radius)| {
                let distance = (u - cx) * (u - cx) + (v - cz) * (v - cz);
                let inner = (radius + 0.013) * (radius + 0.013);
                let outer = (radius + 0.056) * (radius + 0.056);
                height.max(smoothstep((outer - distance) / (outer - inner)) * 85.0)
            });
        let (x_axis, z_axis) = self.axes(x, z);
        let [airport_x, airport_z] = Airport::new(self.seed).local(x_axis, z_axis);
        let tower_separation = (airport_x - 691.0).abs().max((airport_z - 315.0).abs());
        let tower = 20.0 * (1.0 - smoothstep((tower_separation - 70.0) / 360.0));
        if self.contains(x, z) {
            25.0 + height.max(tower)
        } else if outskirts::eligible(self.seed, east, north, self.fringe_grade(x_axis, z_axis)) {
            height.max(14.0)
        } else {
            height
        }
    }

    #[cfg(test)]
    pub(crate) fn overlay(self, x: f32, z: f32, ground: Sample) -> Option<Sample> {
        self.overlay_with_transit(self.transit(), x, z, ground)
    }

    pub(crate) fn overlay_with_transit(
        self,
        rail: MetroTransit,
        x: f32,
        z: f32,
        ground: Sample,
    ) -> Option<Sample> {
        if ground.elevation < 3.0 || x.abs().max(z.abs()) > 16_800.0 {
            return None;
        }
        let (east, north) = self.axes(x, z);
        let airport = Airport::new(self.seed);
        let local = airport.local(east, north);
        let (u, v) = self.street_axes_from(east, north);
        if self.airport_road(airport, local, v)
            || (local[0].abs() <= 18.0 && (397.0..=NORTH_LIMIT).contains(&local[1]))
        {
            return Some(Sample {
                surface: Pavement,
                ..ground
            });
        }
        if let Some(sample) = rail.paint(u, v, ground) {
            return Some(sample);
        }
        if let Some(sample) = airport.overlay(local, ground) {
            return Some(sample);
        }
        let ix = floor(u / BLOCK);
        let iz = floor(v / BLOCK);
        let grade = self.fringe_grade(east, north);
        let fringe = || {
            outskirts::eligible(self.seed, u, v, grade)
                .then(|| outskirts::paint(self.seed, u, v, grade, ground))
        };
        let Some(district) = self.district(ix, iz) else {
            if outskirts::road(u, v, grade) {
                return Some(Sample {
                    surface: Pavement,
                    ..ground
                });
            }
            return fringe();
        };
        let tx = u / BLOCK - ix as f32;
        let tz = v / BLOCK - iz as f32;
        let width = |index: i32| {
            if index.rem_euclid(9) == 0 {
                0.118
            } else {
                0.057
            }
        };
        if tx < width(ix) || 1.0 - tx < width(ix + 1) || tz < width(iz) || 1.0 - tz < width(iz + 1)
        {
            return Some(Sample {
                surface: Pavement,
                ..ground
            });
        }
        let seed = mix(self.seed
            ^ (ix as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9)
            ^ (iz as u64).wrapping_mul(0x94d0_49bb_1331_11eb));
        // Wider lots and gardens replace some of the last residential
        // blocks while their local streets still meet the city network.
        if district == District::Residential {
            let a = u / CITY_RADIUS;
            let b = v / CITY_RADIUS;
            let density = smoothstep((a * a + b * b - 0.77) / 0.31);
            if fraction(seed) < density * 0.60 {
                if let Some(outskirts) = fringe() {
                    return Some(outskirts);
                }
            }
        }
        let (a, b) = (tx - 0.5, tz - 0.5);
        let greenway = |cx: f32, cz: f32, rx: f32, rz: f32| {
            let east = (u - cx) / rx;
            let north = (v - cz) / rz;
            east * east + north * north < 1.0
        };
        let park = greenway(2_550.0, 1_800.0, 1_320.0, 720.0)
            || greenway(-5_800.0, 2_620.0, 1_220.0, 940.0)
            || greenway(4_880.0, -2_050.0, 880.0, 1_200.0)
            || greenway(-1_700.0, -6_900.0, 980.0, 1_400.0)
            || seed % 29 == 0;
        if park {
            let trees = fraction_of(a * 4.0 + 0.5) - 0.5;
            let rows = fraction_of(b * 4.0 + 0.5) - 0.5;
            let grove = trees * trees + rows * rows < 0.085 && (a - b * 0.45).abs() > 0.055;
            return Some(if grove {
                Sample {
                    elevation: ground.elevation + 5.5,
                    surface: TreeCanopy,
                }
            } else {
                Sample {
                    surface: Park,
                    ..ground
                }
            });
        }
        let design = DESIGNS[district as usize][(seed % 5) as usize];
        let (a, b) = if seed & 8 == 0 { (a, b) } else { (b, a) };
        let a = if seed & 16 == 0 { a } else { -a };
        let b = if seed & 32 == 0 { b } else { -b };
        let roof = design.form.height(a, b);
        if roof == 0.0 {
            let surface = if district == District::Industry || a.abs() < 0.055 || b.abs() < 0.055 {
                Gravel
            } else {
                UrbanLot
            };
            return Some(Sample { surface, ..ground });
        }
        Some(Sample {
            elevation: ground.elevation + roof * design.height * (0.76 + fraction(seed) * 0.49),
            surface: design.roof,
        })
    }
}

fn floor(value: f32) -> i32 {
    let integer = value as i32;
    integer - i32::from(value < integer as f32)
}

fn fraction_of(value: f32) -> f32 {
    value - floor(value) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roads_are_continuous_across_the_city_and_the_center_has_tall_roofs() {
        let city = Metropolis {
            seed: 7,
            rotation: (0.0, 1.0),
            aspect: 1.0,
            edge_bias: 0.0,
        };
        let mut longest = 0;
        let mut run = 0;
        let ground = Sample {
            elevation: 19.0,
            surface: Grass,
        };
        for x in (-12_000..12_000).step_by(4) {
            let u = x as f32;
            // Follow the curved zero-indexed street in world coordinates.
            let z = -(u / CITY_RADIUS) * (u / CITY_RADIUS) * 1_400.0;
            let x = u - (z / CITY_RADIUS) * (z / CITY_RADIUS) * 2_500.0;
            if city
                .overlay(x, z, ground)
                .is_some_and(|s| s.surface == Pavement)
            {
                run += 4;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
        assert!(longest >= 20_000, "continuous street was {longest}");
        let tallest = (-2_400..2_400)
            .step_by(19)
            .flat_map(|x| {
                (-1_500..1_500)
                    .step_by(17)
                    .filter_map(move |z| city.overlay(x as f32, z as f32, ground))
            })
            .map(|s| s.elevation)
            .fold(0.0, f32::max);
        assert!(tallest > 110.0);
    }

    #[test]
    fn seeded_metropolises_integrate_with_the_world_and_flight_controller() {
        use crate::flight::{Aircraft, Flight};
        use crate::world::World;

        for (seed, world) in (0..100)
            .map(|seed| (seed, World::for_flight_seed(seed)))
            .filter(|(_, world)| world.has_metropolis())
            .take(3)
        {
            assert!((-10_000..=10_000)
                .step_by(80)
                .all(|x| world.elevation(x as f32, 0.0) > 3.0));
            let original = World::new(seed);
            for (x, z) in [(24_000.0, 0.0), (-24_000.0, 24_000.0)] {
                assert_eq!(world.sample(x, z), original.sample(x, z));
            }
            assert!(crate::roads::RoadPlanner::new(&world, 0, 0).is_none());
            for aircraft in Aircraft::ALL {
                let mut flight = Flight::start(&world, seed, aircraft);
                assert!(world.city_occupies(flight.x, flight.z));
                for _ in 0..2_100 {
                    flight.update(&world);
                    let ground = world.elevation(flight.x, flight.z);
                    assert!(flight.altitude > world.built_height(flight.x, flight.z, ground));
                }
                assert!(
                    flight.terrain_recoveries <= 1,
                    "{seed} {aircraft:?}: {}",
                    flight.terrain_recoveries
                );
            }
        }
    }

    #[test]
    fn the_airport_is_graded_and_joins_a_city_street_across_seeds() {
        use crate::world::World;

        for seed in (0..200)
            .filter(|&seed| World::for_flight_seed(seed).has_metropolis())
            .take(10)
        {
            let world = World::for_flight_seed(seed);
            let city = world.metropolis().unwrap();
            let airport = Airport::new(city.seed);
            let road = |local_north: f32| {
                let [east, north] = airport.point([0.0, local_north]);
                let [x, z] = city.from_axes(east, north);
                let (_, v) = city.street_axes(x, z);
                (x, z, v)
            };
            let (_, _, boundary) = road(NORTH_LIMIT);
            let junction = (floor((boundary + 110.0) / BLOCK) + 1) as f32 * BLOCK;
            let mut finished = false;
            for local in (398..=850).step_by(2) {
                let (x, z, v) = road(local as f32);
                let ground = world.sample(x, z);
                assert_eq!(
                    city.overlay(x, z, ground).unwrap().surface,
                    Pavement,
                    "{seed} {local}"
                );
                if v >= junction {
                    finished = true;
                    break;
                }
            }
            assert!(finished, "{seed}: missing street junction");
            for z in [-438.0, -224.0] {
                let height = |x| {
                    let [east, north] = airport.point([x, z]);
                    let [x, z] = city.from_axes(east, north);
                    world.elevation(x, z)
                };
                assert!((height(-1000.0) - height(1000.0)).abs() < 4.0);
                for x in (-1100..1100).step_by(40) {
                    assert!((height(x as f32) - height((x + 40) as f32)).abs() < 0.10);
                }
            }
        }
    }

    #[test]
    fn arterial_roads_continue_through_outlying_farms_into_connected_villages() {
        use crate::world::World;

        for seed in [0, 15, 19, 23, 40] {
            let world = World::for_flight_seed(seed);
            let city = world.metropolis().unwrap();
            let overlay = world.city_surface();
            let mut fields = 0;
            let mut roofs = 0;
            for axis in [false, true] {
                for side in [-1.0, 1.0] {
                    for through in [-3_456.0, 0.0, 3_456.0] {
                        let mut previous: Option<f32> = None;
                        for ahead in (8_600..=12_510).step_by(13) {
                            let forward = ahead as f32 * side;
                            let (u, v) = if axis {
                                (forward, through)
                            } else {
                                (through, forward)
                            };
                            let [x, z] = city.from_street_axes(u, v);
                            let ground = world.sample(x, z);
                            let (east, north) = city.axes(x, z);
                            let grade = city.fringe_grade(east, north);
                            let site = overlay(x, z, ground).unwrap_or_else(|| {
                                panic!(
                                    "{seed} road {u} {v}, basin radius {grade}, ground {:?}",
                                    ground
                                )
                            });
                            assert_eq!(site.surface, Pavement, "{seed} road {u} {v}");
                            if let Some(last) = previous {
                                assert!((ground.elevation - last).abs() < 0.45);
                            }
                            previous = Some(ground.elevation);
                            if ahead >= 11_000 {
                                let (ux, uz) = if axis { (u, v + 65.0) } else { (u + 65.0, v) };
                                let [tx, tz] = city.from_street_axes(ux, uz);
                                let ground = world.sample(tx, tz);
                                if let Some(plot) = overlay(tx, tz, ground) {
                                    fields += usize::from(matches!(
                                        plot.surface,
                                        FieldGold | FieldGreen | FieldSoil
                                    ));
                                    roofs += usize::from(
                                        plot.surface.building_palette().is_some()
                                            || matches!(plot.surface, FlatRoof | PitchedRoof),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            assert!(
                fields > 30 && roofs > 24,
                "{seed} fields={fields} roofs={roofs}"
            );
            // At this radius, all points are beyond the basin and its fringe.
            for [u, v] in [[22_000.0, 0.0], [-21_000.0, 0.0], [0.0, 22_000.0]] {
                let [x, z] = city.from_street_axes(u, v);
                assert!(overlay(x, z, world.sample(x, z)).is_none());
            }
        }
    }

    #[test]
    fn all_twenty_five_blocks_have_distinct_footprints_sections_or_materials() {
        let mut hashes = [0_u64; 25];
        for (i, design) in DESIGNS.into_iter().flatten().enumerate() {
            let mut hash = design.roof as u64;
            for x in -20..=20 {
                for z in -20..=20 {
                    let height =
                        design.height * design.form.height(x as f32 * 0.021, z as f32 * 0.021);
                    hash = hash.rotate_left(5) ^ u64::from(height.to_bits());
                }
            }
            assert!(!hashes[..i].contains(&hash), "duplicate plot {i}");
            hashes[i] = hash;
        }
    }
}
