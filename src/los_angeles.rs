//! Downtown Los Angeles flight routes in UTM metres relative to 34.05° N, 118.25° W.

#[path = "los_angeles/us_bank.rs"]
mod us_bank;
#[path = "los_angeles/wilshire_grand.rs"]
mod wilshire_grand;
#[path = "los_angeles/disney_hall.rs"]
mod disney_hall;
#[path = "los_angeles/city_hall.rs"]
mod city_hall;

use crate::flight::bearing;
use crate::render::{Camera, ScreenProjection, ScreenRect};
use crate::world::{Surface, World};
use crate::world_mesh::WorldMesh;

const TOUR: [[f32; 2]; 8] = [
    [-480.0, 620.0], [560.0, 910.0], [1_675.0, 470.0], [1_875.0, -1_280.0],
    [-200.0, -2_680.0], [-2_450.0, -1_740.0], [-2_710.0, 720.0], [-1_920.0, 1_640.0],
];
const APPROACHES: [([f32; 2], usize); 3] = [
    ([950.0, 1_660.0], 1),
    ([-2_580.0, -2_860.0], 5),
    ([620.0, 240.0], 3),
];
const WIDER_TOUR_START: usize = TOUR.len() + APPROACHES.len();
const WIDER_TOUR: [[f32; 2]; 12] = [
    [-2_200.0, 4_800.0], [-5_300.0, 8_250.0], [-7_550.0, 9_900.0],
    [-9_200.0, 6_450.0], [-10_050.0, 1_500.0], [-9_900.0, -5_900.0],
    [-7_420.0, -10_080.0], [-1_300.0, -6_800.0], [5_850.0, -1_180.0],
    [8_640.0, 9_590.0], [10_720.0, 11_170.0], [4_500.0, 7_050.0],
];
const COAST_TOUR_START: usize = WIDER_TOUR_START + WIDER_TOUR.len();
const COAST_TOUR: [[f32; 2]; 8] = [
    [-16_650.0, 5_100.0], [-20_970.0, 3_690.0], [-22_100.0, -2_500.0],
    [-22_950.0, -5_700.0], [-21_150.0, -7_050.0], [-19_100.0, -10_450.0],
    [-14_000.0, -11_650.0], [-10_280.0, -10_980.0],
];

pub(crate) const WIDER_COVERAGE_POINT: [f32; 2] = [-7_550.0, 9_900.0];
pub(crate) const COAST_COVERAGE_POINT: [f32; 2] = [-23_400.0, -5_900.0];

pub(crate) fn departure(area: u32) -> ((f32, f32, f32), usize) {
    let (start, target) = match area {
        1 => ([1_260.0, 3_250.0], TOUR.len()),
        2 => ([-4_230.0, -4_530.0], TOUR.len() + 1),
        3 => ([2_190.0, 1_040.0], TOUR.len() + 2),
        4 => ([-3_300.0, 6_940.0], WIDER_TOUR_START + 1),
        5 => ([-7_300.0, 5_000.0], WIDER_TOUR_START + 3),
        6 => ([8_640.0, 9_590.0], WIDER_TOUR_START + 10),
        7 => ([-9_480.0, -11_400.0], WIDER_TOUR_START + 6),
        8 => ([-18_150.0, -11_620.0], COAST_TOUR_START + 6),
        9 => (COAST_COVERAGE_POINT, COAST_TOUR_START + 2),
        10 => ([-23_090.0, -7_640.0], COAST_TOUR_START + 4),
        11 => ([-21_460.0, 1_540.0], COAST_TOUR_START + 1),
        _ => ([-2_270.0, -1_245.0], 0),
    };
    let (point, _) = tour_waypoint(target, true, true);
    ((start[0], start[1], bearing(point[0] - start[0], point[1] - start[1])), target)
}

pub(crate) fn tour_waypoint(leg: usize, expanded: bool, coast: bool) -> ([f32; 2], usize) {
    if let Some(&point) = TOUR.get(leg) {
        let next = if leg + 1 < TOUR.len() { leg + 1 }
            else if expanded { WIDER_TOUR_START } else { 0 };
        (point, next)
    } else if let Some(&approach) = APPROACHES.get(leg - TOUR.len()) {
        approach
    } else if leg < COAST_TOUR_START {
        let next = if coast && leg == WIDER_TOUR_START + 3 { COAST_TOUR_START }
            else if leg + 1 < COAST_TOUR_START { leg + 1 } else { 0 };
        (WIDER_TOUR[leg - WIDER_TOUR_START], next)
    } else {
        let next = if leg + 1 < COAST_TOUR_START + COAST_TOUR.len() { leg + 1 } else { WIDER_TOUR_START + 6 };
        (COAST_TOUR[leg - COAST_TOUR_START], next)
    }
}

pub(crate) fn safety_height(x: f32, z: f32) -> f32 {
    let east = x + 400.0;
    let north = z - 190.0;
    let distance_squared = east * east + north * north;
    (1.0 - crate::math::smoothstep((distance_squared - 396_900.0) / 1_793_500.0)) * 135.0
}

pub(crate) fn landmark_clearance(x: f32, z: f32) -> f32 {
    if us_bank::contains(x, z) { us_bank::HEIGHT }
    else { wilshire_grand::clearance(x, z).max(disney_hall::clearance(x, z)).max(city_hall::clearance(x, z)) }
}

pub(crate) fn replaces_raster(x: f32, z: f32, height: f32, surface: Surface) -> bool {
    us_bank::contains(x, z) || wilshire_grand::contains(x, z)
        || city_hall::contains(x, z)
        || (surface == Surface::VegasSlateGlass && disney_hall::replaces_raster(x, z, height))
}

struct Landmark {
    center: [f32; 2],
    half_extent: [f32; 2],
    height: f32,
    draw: fn(&mut WorldMesh<'_>, f32),
}

const LANDMARKS: [Landmark; 4] = [
    Landmark { center: us_bank::CENTER, half_extent: [35.0, 35.0], height: us_bank::HEIGHT, draw: us_bank::draw },
    Landmark { center: wilshire_grand::CENTER, half_extent: [75.0, 62.0], height: wilshire_grand::HEIGHT, draw: wilshire_grand::draw },
    Landmark { center: disney_hall::CENTER, half_extent: [76.0, 80.0], height: disney_hall::HEIGHT, draw: disney_hall::draw },
    Landmark { center: city_hall::CENTER, half_extent: city_hall::HALF_EXTENT, height: city_hall::HEIGHT, draw: city_hall::draw },
];

pub struct DowntownLandmarks {
    bases: [Option<f32>; LANDMARKS.len()],
    region: ScreenRect,
}

impl DowntownLandmarks {
    pub fn from_world(world: &World, camera: Camera, dimensions: (usize, usize)) -> Option<Self> {
        if world.mapped_city() != Some(crate::MappedCity::LosAngeles) {
            return None;
        }
        let projection = ScreenProjection::new(camera, dimensions)?;
        let mut region: Option<ScreenRect> = None;
        let bases = LANDMARKS.map(|landmark| {
            let [x, z] = landmark.center;
            let [east, north] = landmark.half_extent;
            let base = world.sample(x, z).elevation;
            let bounds = projection.bounds([
                [x - east, base - 7.0, z - north], [x + east, base + landmark.height, z + north],
            ])?;
            region = Some(region.map_or(bounds, |current| current.union(bounds)));
            Some(base)
        });
        Some(Self { bases, region: region? })
    }

    pub fn bounds(&self) -> ScreenRect { self.region }

    pub fn draw(&self, mesh: &mut WorldMesh<'_>) {
        for (landmark, base) in LANDMARKS.iter().zip(self.bases) {
            if let Some(base) = base { (landmark.draw)(mesh, base); }
        }
    }
}
