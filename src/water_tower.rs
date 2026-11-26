//! A municipal water tank connected to a surveyed settlement street.

use crate::math::{mix, sin_cos, smoothstep};
use crate::render::{Camera, ScreenRect, screen_region};
use crate::roads::{ROAD_HALF_WIDTH, ROAD_REGION_SIZE, RoadNetwork, RoadPoint};
use crate::settlement::{Settlement, SettlementRoadExit, SettlementSurface};
use crate::world::{Sample, Surface, World};
use crate::world_mesh::{WorldMesh, bounds_in_range, landmark_reach};

mod designs;
use designs::Support;
pub use designs::WaterTowerDesign;

type Point = [f32; 3];
type Color = [u8; 3];
const PAD: f32 = 8.6;
const SPUR_HALF: f32 = 1.55;
const YARD_STEP: f32 = 2.1;
const MIN_GRADE: f32 = 4.0;
const MAX_SPUR_GRADE: f32 = 0.15;
const FOUNDATION_DEPTH: f32 = 2.4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterTower {
    road: [RoadPoint; 3],
    axis: [f32; 2],
    bounds: [f32; 4],
    design: WaterTowerDesign,
}

impl WaterTower {
    /// Prepare no more than one tower for a road district. The callback keeps
    /// published waterways, rights-of-way and other facilities authoritative.
    pub fn plan(
        world: &World,
        seed: u64,
        region: [i32; 2],
        highway: &RoadNetwork,
        towns: [Option<&Settlement>; 2],
        mut reserved: impl FnMut(f32, f32) -> bool,
        mut service: impl FnMut(),
    ) -> Option<Self> {
        let region_bits = (u64::from(region[0] as u32) << 32) | u64::from(region[1] as u32);
        let key = mix(seed ^ region_bits ^ 0x1fa2_587e_1b32_9754);
        if key & 3 != 0 {
            return None;
        }
        let edges = highway.polyline();
        let ends = [*edges.first()?, *edges.last()?];
        let origin = region.map(|axis| axis as f32 * ROAD_REGION_SIZE as f32);
        for attempts in 0..2 {
            let end = (((key >> 2) & 1) as usize + attempts) % 2;
            let road_end = ends[end];
            let Some(town) = towns[end] else {
                continue;
            };
            if town.tower_count() > 3 || town.building_count() < 6 {
                continue;
            }
            let mut seen = [None; 8];
            let mut count = 0;
            for pass in 0..4 {
                service();
                let direction = (((key >> 8) & 3) as usize + pass) % 4;
                let [dx, dz] =
                    [[0.0, 160.0], [160.0, 0.0], [0.0, -160.0], [-160.0, 0.0]][direction];
                let toward = RoadPoint {
                    x: road_end.x + dx,
                    z: road_end.z + dz,
                    elevation: road_end.elevation,
                };
                for exit in town.coastal_road_exits(world, toward).into_iter().flatten() {
                    if seen[..count].contains(&Some(exit)) {
                        continue;
                    }
                    seen[count] = Some(exit);
                    count += 1;
                    if town
                        .sample(exit.point.x, exit.point.z)
                        .map(|sample| sample.surface)
                        != Some(SettlementSurface::Street)
                    {
                        continue;
                    }
                    let turn = if key & 0x80 == 0 { 0.31 } else { -0.31 };
                    let design = WaterTowerDesign::select(mix(key ^ 0x2c7b));
                    let mut plans =
                        [turn, -turn].map(|angle| Self::at_exit(world, exit, angle, design));
                    if plans[1].road[2].elevation > plans[0].road[2].elevation {
                        plans.swap(0, 1);
                    }
                    for plan in plans {
                        service();
                        let [min_x, min_z, max_x, max_z] = plan.bounds;
                        if min_x < origin[0] + 4.0
                            || max_x > origin[0] + (ROAD_REGION_SIZE - 4) as f32
                            || min_z < origin[1] + 4.0
                            || max_z > origin[1] + (ROAD_REGION_SIZE - 4) as f32
                        {
                            continue;
                        }
                        if plan.fits(world, highway, towns, &mut reserved) {
                            return Some(plan);
                        }
                    }
                }
            }
        }
        None
    }

    fn at_exit(
        world: &World,
        exit: SettlementRoadExit,
        turn: f32,
        design: WaterTowerDesign,
    ) -> Self {
        let [x, z] = exit.outward;
        let (sin, cos) = sin_cos(turn);
        let axis = [x * cos + z * sin, z * cos - x * sin];
        let bend = road_point(world, exit.point.x + x * 12.0, exit.point.z + z * 12.0);
        let center = road_point(world, bend.x + axis[0] * 29.0, bend.z + axis[1] * 29.0);
        let bounds = [
            (exit.point.x.min(bend.x) - SPUR_HALF).min(center.x - PAD),
            (exit.point.z.min(bend.z) - SPUR_HALF).min(center.z - PAD),
            (exit.point.x.max(bend.x) + SPUR_HALF).max(center.x + PAD),
            (exit.point.z.max(bend.z) + SPUR_HALF).max(center.z + PAD),
        ];
        Self {
            road: [exit.point, bend, center],
            axis,
            design,
            bounds,
        }
    }

    fn fits(
        &self,
        world: &World,
        highway: &RoadNetwork,
        towns: [Option<&Settlement>; 2],
        reserved: &mut impl FnMut(f32, f32) -> bool,
    ) -> bool {
        let [exit, bend, pad] = self.road;
        if !(MIN_GRADE..=98.0).contains(&pad.elevation)
            || !(-2.4..=4.2).contains(&(pad.elevation - exit.elevation))
            || (bend.elevation - exit.elevation).abs() > 12.0 * MAX_SPUR_GRADE
            || (pad.elevation - bend.elevation).abs() > 29.0 * MAX_SPUR_GRADE
        {
            return false;
        }
        let mut clear = |x, z, at_join: bool| {
            !reserved(x, z)
                && !highway
                    .proximity(x, z)
                    .is_some_and(|hit| hit.distance_squared <= ROAD_HALF_WIDTH * ROAD_HALF_WIDTH)
                && towns.into_iter().flatten().all(|town| {
                    town.sample(x, z)
                        .is_none_or(|detail| at_join && detail.surface == SettlementSurface::Street)
                })
        };
        for dz in -4i32..=4 {
            for dx in -4i32..=4 {
                let [x, z] = [pad.x + dx as f32 * YARD_STEP, pad.z + dz as f32 * YARD_STEP];
                if !near_pad(pad, x, z) {
                    continue;
                }
                let height = world.elevation(x, z);
                if height < MIN_GRADE
                    || !(-1.45..=0.65).contains(&(height - pad.elevation))
                    || !clear(x, z, false)
                {
                    return false;
                }
            }
        }
        for (a, b, length) in [(exit, bend, 12), (bend, pad, 29)] {
            let side = [(b.z - a.z) / length as f32, (a.x - b.x) / length as f32];
            let mut last = a.elevation;
            for step in 0..=length {
                let t = step as f32 / length as f32;
                let x = a.x + (b.x - a.x) * t;
                let z = a.z + (b.z - a.z) * t;
                let current = world.elevation(x, z);
                if current < MIN_GRADE || (current - last).abs() > MAX_SPUR_GRADE {
                    return false;
                }
                for across in [-SPUR_HALF, 0.0, SPUR_HALF] {
                    let [x, z] = [x + side[0] * across, z + side[1] * across];
                    let height = world.elevation(x, z);
                    if height < MIN_GRADE
                        || (height - current).abs() > 0.42
                        || !clear(x, z, a == exit && step <= 2)
                    {
                        return false;
                    }
                }
                last = current;
            }
        }
        true
    }

    pub fn design(&self) -> WaterTowerDesign {
        self.design
    }

    /// Render the same geometry above an even foundation for the artwork atlas.
    pub fn artwork(design: WaterTowerDesign) -> Self {
        let center = RoadPoint {
            x: 0.0,
            elevation: 0.0,
            z: 0.0,
        };
        Self {
            road: [center; 3],
            axis: [0.0, 1.0],
            bounds: [-8.0, -8.0, 8.0, 8.0],
            design,
        }
    }

    pub fn access_road(&self) -> &[RoadPoint; 3] {
        &self.road
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub fn reserves(&self, x: f32, z: f32) -> bool {
        let [min_x, min_z, max_x, max_z] = self.bounds;
        x >= min_x
            && x <= max_x
            && z >= min_z
            && z <= max_z
            && (near_pad(self.road[2], x, z)
                || self
                    .road
                    .windows(2)
                    .any(|edge| near_spur(edge[0], edge[1], x, z)))
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        if ground.elevation >= MIN_GRADE
            && matches!(
                ground.surface,
                Surface::Grass
                    | Surface::Forest
                    | Surface::Rock
                    | Surface::Desert
                    | Surface::Gravel
                    | Surface::FieldGreen
                    | Surface::FieldGold
                    | Surface::FieldSoil
            )
            && self.reserves(x, z)
        {
            Sample {
                surface: Surface::Gravel,
                ..ground
            }
        } else {
            ground
        }
    }

    pub fn mesh_bounds(&self) -> [Point; 2] {
        let RoadPoint { x, z, elevation } = self.road[2];
        [
            [x - 8.0, elevation - FOUNDATION_DEPTH, z - 8.0],
            [x + 8.0, elevation + self.height(), z + 8.0],
        ]
    }

    pub fn screen_bounds(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        bounds_in_range(camera, self.bounds, landmark_reach(camera.altitude))
            .then(|| screen_region(camera, dimensions, self.mesh_bounds()))
            .flatten()
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        let center = self.road[2];
        let (dx, dz) = (x - center.x, z - center.z);
        let distance_squared = dx * dx + dz * dz;
        if distance_squared >= 16.0 * 16.0 {
            return 0.0;
        }
        let edge = (16.0 * 16.0 - distance_squared) / (16.0 * 16.0 - 8.0 * 8.0);
        (center.elevation + self.height() + 2.0) * smoothstep(edge)
    }

    pub fn draw(&self, mesh: &mut WorldMesh<'_>) {
        let RoadPoint { x, z, elevation } = self.road[2];
        let [east, north] = self.axis;
        self.faces(|points, color| {
            mesh.lit_triangle(
                points.map(|[a, h, b]| {
                    [
                        x + a * north + b * east,
                        elevation + h,
                        z - a * east + b * north,
                    ]
                }),
                color,
            );
        });
    }

    fn height(&self) -> f32 {
        self.design.spec().height
    }

    fn faces(&self, mut emit: impl FnMut([Point; 3], Color)) {
        let spec = self.design.spec();
        let [tank, band, steel] = spec.paint;
        let concrete = [144, 139, 125];
        let bottom = spec.height - spec.depth;
        let first_radius = spec.radius * spec.profile[0];
        match spec.support {
            Support::Pedestal => {
                let neck = first_radius.min(2.45);
                let foot = ring_tank(-FOUNDATION_DEPTH, 3.60, false);
                let apron = ring_tank(0.68, 3.60, false);
                let shoulder = ring_tank(4.3, neck, false);
                let stem = ring_tank(bottom, neck, spec.angular);
                shell(&mut emit, foot, apron, concrete);
                shell(&mut emit, apron, shoulder, steel);
                shell(&mut emit, shoulder, stem, steel);
            }
            Support::Frame => self.steel_frame(&mut emit, bottom, steel, concrete),
            Support::Masonry => {
                let shaft = first_radius.clamp(2.2, 4.9);
                for (a, b, lo, hi, paint) in [
                    (-FOUNDATION_DEPTH, 0.7, shaft + 0.54, shaft + 0.54, concrete),
                    (0.7, 4.1, shaft + 0.54, shaft, steel),
                    (4.1, bottom - 1.1, shaft, shaft * 0.94, steel),
                    (bottom - 1.1, bottom, shaft * 0.94, first_radius, band),
                ] {
                    shell(
                        &mut emit,
                        ring_tank(a, lo, spec.angular),
                        ring_tank(b, hi, spec.angular),
                        paint,
                    );
                }
            }
            Support::Twin | Support::Portal => {
                let portal = spec.support == Support::Portal;
                let head = if portal { 2.20 } else { 1.70 };
                let width = if portal { 3.38 } else { 1.99 };
                let centers: [f32; 2] = if portal && spec.angular {
                    [-width, width]
                } else {
                    [-width * 0.88, width]
                };
                for ground_x in centers {
                    let up = ground_x.signum() * head;
                    shell(
                        &mut emit,
                        ring_square([ground_x, -FOUNDATION_DEPTH, 0.0], 1.02),
                        ring_square([ground_x, 0.72, 0.0], 1.02),
                        concrete,
                    );
                    shell(
                        &mut emit,
                        ring_square([ground_x, 0.72, 0.0], 0.85),
                        ring_square([up, bottom, 0.0], 0.88),
                        steel,
                    );
                }
                let lintel = if portal { 3.0 } else { 2.42 };
                let rectangle = |y| {
                    [
                        [-lintel, y, -1.17],
                        [lintel, y, -1.17],
                        [lintel, y, 1.17],
                        [-lintel, y, 1.17],
                    ]
                };
                shell(&mut emit, rectangle(bottom - 1.1), rectangle(bottom), band);
            }
        }
        let mut lower = ring_tank(bottom, first_radius, spec.angular);
        // Keep the bottom closed where a flared head sits on narrow legs.
        for i in 1..lower.len() - 1 {
            emit([lower[0], lower[i], lower[i + 1]], tank);
        }
        for (i, &fraction) in spec.profile.iter().enumerate().skip(1) {
            let y = if i == spec.profile.len() - 1 {
                spec.height
            } else {
                bottom + spec.depth * (i as f32 * 0.20)
            };
            let upper = ring_tank(y, spec.radius * fraction, spec.angular);
            shell(&mut emit, lower, upper, if i == 3 { band } else { tank });
            lower = upper;
        }
        for i in 1..lower.len() - 1 {
            emit([lower[0], lower[i + 1], lower[i]], tank);
        }
    }

    fn steel_frame(
        &self,
        emit: &mut impl FnMut([Point; 3], Color),
        bottom: f32,
        steel: Color,
        concrete: Color,
    ) {
        let base = ring_square([0.0, 0.72, 0.0], 4.8);
        let collar = ring_square([0.0, bottom + 0.38, 0.0], 3.65);
        shell(
            emit,
            ring_square([0.0, -FOUNDATION_DEPTH, 0.0], 0.54),
            ring_square([0.0, bottom - 0.02, 0.0], 0.54),
            steel,
        );
        for ([x, y, z], top) in base.into_iter().zip(collar) {
            let low = ring_square([x, -FOUNDATION_DEPTH, z], 0.82);
            let high = ring_square([x, 0.82, z], 0.82);
            shell(emit, low, high, concrete);
            let [a, b, c, d] = high;
            quad(emit, [d, c, b, a], concrete);
            shell(
                emit,
                ring_square([x, y, z], 0.31),
                ring_square(top, 0.25),
                steel,
            );
        }
        shell(
            emit,
            ring_square([0.0, bottom - 0.18, 0.0], 4.0),
            ring_square([0.0, bottom + 0.38, 0.0], 4.0),
            steel,
        );
        for side in 0..4 {
            let next = (side + 1) % 4;
            let at = |along, up| {
                lerp(
                    lerp(base[side], base[next], along),
                    lerp(collar[side], collar[next], along),
                    up,
                )
            };
            for floor in [0.04, 0.48] {
                quad(
                    emit,
                    [
                        at(0.03, floor),
                        at(0.09, floor),
                        at(0.97, floor + 0.41),
                        at(0.91, floor + 0.41),
                    ],
                    steel,
                );
            }
        }
    }
}

fn road_point(world: &World, x: f32, z: f32) -> RoadPoint {
    RoadPoint {
        x,
        z,
        elevation: world.elevation(x, z),
    }
}

fn near_pad(center: RoadPoint, x: f32, z: f32) -> bool {
    let (x, z) = (x - center.x, z - center.z);
    x * x + z * z <= PAD * PAD
}

fn near_spur(a: RoadPoint, b: RoadPoint, x: f32, z: f32) -> bool {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    let fraction = (((x - a.x) * dx + (z - a.z) * dz) / (dx * dx + dz * dz)).clamp(0.0, 1.0);
    let (u, v) = (x - (a.x + fraction * dx), z - (a.z + fraction * dz));
    u * u + v * v <= SPUR_HALF * SPUR_HALF
}

fn ring_square([x, y, z]: Point, half: f32) -> [Point; 4] {
    [
        [x - half, y, z - half],
        [x + half, y, z - half],
        [x + half, y, z + half],
        [x - half, y, z + half],
    ]
}

fn ring_tank(y: f32, r: f32, angular: bool) -> [Point; 12] {
    const RING: [[f32; 2]; 12] = [
        [1.0, 0.0],
        [0.866_025_4, 0.5],
        [0.5, 0.866_025_4],
        [0.0, 1.0],
        [-0.5, 0.866_025_4],
        [-0.866_025_4, 0.5],
        [-1.0, 0.0],
        [-0.866_025_4, -0.5],
        [-0.5, -0.866_025_4],
        [0.0, -1.0],
        [0.5, -0.866_025_4],
        [0.866_025_4, -0.5],
    ];
    const SQUARE: [[f32; 2]; 12] = [
        [0.707_106_77, 0.0],
        [0.707_106_77, 0.43],
        [0.707_106_77, 0.707_106_77],
        [0.0, 0.707_106_77],
        [-0.43, 0.707_106_77],
        [-0.707_106_77, 0.707_106_77],
        [-0.707_106_77, 0.0],
        [-0.707_106_77, -0.43],
        [-0.707_106_77, -0.707_106_77],
        [0.0, -0.707_106_77],
        [0.43, -0.707_106_77],
        [0.707_106_77, -0.707_106_77],
    ];
    let outline = if angular { SQUARE } else { RING };
    outline.map(|[x, z]| [x * r, y, z * r])
}

fn shell<const N: usize>(
    emit: &mut impl FnMut([Point; 3], Color),
    low: [Point; N],
    high: [Point; N],
    color: Color,
) {
    for i in 0..N {
        let j = (i + 1) % N;
        quad(emit, [low[i], high[i], high[j], low[j]], color);
    }
}

fn quad(emit: &mut impl FnMut([Point; 3], Color), [a, b, c, d]: [Point; 4], color: Color) {
    emit([a, b, c], color);
    emit([a, c, d], color);
}

fn lerp(from: Point, to: Point, t: f32) -> Point {
    core::array::from_fn(|i| from[i] + (to[i] - from[i]) * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tower_fits_its_draw_bounds_and_clearance() {
        let world = World::new(31);
        let exit = SettlementRoadExit {
            point: road_point(&world, 1374.0, 1758.0),
            outward: [0.0, 1.0],
        };
        for index in 0..WaterTowerDesign::COUNT {
            let design = WaterTowerDesign::from_index(index).unwrap();
            let tower = WaterTower::at_exit(&world, exit, -0.31, design);
            let mut triangles = 0;
            tower.faces(|points, _| {
                triangles += 1;
                for [x, y, z] in points {
                    assert!(x * x + z * z <= 8.0 * 8.0);
                    assert!((-FOUNDATION_DEPTH..=tower.height()).contains(&y));
                }
            });
            assert!((120..320).contains(&triangles));
            let center = tower.access_road()[2];
            assert!(tower.obstruction(center.x, center.z) > tower.mesh_bounds()[1][1]);
            assert_eq!(tower.obstruction(center.x + 20.0, center.z), 0.0);
        }
    }
}
