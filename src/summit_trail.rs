//! Short walking routes and rest stops below prepared summit lookouts.

use crate::math::{sin_cos, smoothstep};
use crate::summit_sites::SummitSite;
use crate::world::World;
use crate::world_mesh::WorldMesh;

type Point = [f32; 3];
type Area = [f32; 4];
const WIDTH: f32 = 1.18;
const REST_RADIUS: f32 = 5.2;
const PATH_LIFT: f32 = 0.34;
const PATH_MARGIN: f32 = 1.77;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SummitTrail {
    path: [Point; 5],
    rest: Point,
    marker: Point,
    heading: [f32; 2],
    wind_shelter: bool,
}

impl SummitTrail {
    /// Start at the existing lookout's downhill approach. Keep the mountain's
    /// profile and accept a cluster only if the turns and its pad fit the slope.
    pub fn survey(
        world: &World,
        site: &SummitSite,
        mut blocked: impl FnMut(Area) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        let [x, summit, z] = site.center();
        let (fx, fz) = sin_cos(site.heading());
        let initial_side = if site.variant() & 1 == 0 { -1.0 } else { 1.0 };
        let at = |side: f32, forward: f32| -> Point {
            let [a, b] = [x + fz * side + fx * forward, z - fx * side + fz * forward];
            [a, world.elevation(a, b), b]
        };
        for scale in [1.0, 1.3, 0.75] {
            for side in [initial_side, -initial_side] {
                pulse();
                let path = [
                    at(0.0, 5.6),
                    at(5.4 * side, 13.7),
                    at(19.8 * side * scale, 21.2),
                    at(6.7 * side, 31.5 * scale),
                    at(-7.5 * side, 46.0 * scale),
                ];
                let foot = path[4];
                if summit - foot[1] < 4.5
                    || summit - foot[1] > 16.0
                    || foot[1] <= 82.0
                    || !path
                        .windows(2)
                        .all(|pair| certify_edge(world, pair[0], pair[1], &mut pulse))
                {
                    continue;
                }
                // A short, unbuilt continuation ensures the path does not just
                // run out at the lip of a cliff or an isolated talus stone.
                let continuation = at(-13.0 * side, 54.0 * scale);
                if !certify_edge(world, foot, continuation, &mut pulse) {
                    continue;
                }
                let rest = at((19.8 * scale + 5.2) * side, 21.6);
                let marker_at = point_between(path[3], path[4], 0.62);
                let heading_last = unit([path[4][0] - path[3][0], path[4][2] - path[3][2]]);
                let marker_x = marker_at[0] + heading_last[1] * 2.6 * side;
                let marker_z = marker_at[2] - heading_last[0] * 2.6 * side;
                let marker = [marker_x, world.elevation(marker_x, marker_z), marker_z];
                let direction = [path[2][0] - rest[0], path[2][2] - rest[2]];
                let heading = unit(direction);
                if !certify_pad(world, rest, REST_RADIUS, 0.92)
                    || !certify_pad(world, marker, 1.3, 0.62)
                    || !certify_edge(world, path[2], rest, &mut pulse)
                {
                    continue;
                }
                let trail = Self {
                    path,
                    rest,
                    marker,
                    heading,
                    wind_shelter: (site.variant() >> 3) & 3 == 3,
                };
                if !trail.conflicts_with(&mut blocked) {
                    return Some(trail);
                }
            }
        }
        None
    }

    pub fn conflicts_with(&self, mut blocked: impl FnMut(Area) -> bool) -> bool {
        self.path.windows(2).any(|pair| {
            let [a, b] = [pair[0], pair[1]];
            [
                a,
                point_between(a, b, 0.25),
                point_between(a, b, 0.5),
                point_between(a, b, 0.75),
                b,
            ]
            .windows(2)
            .any(|edge| blocked(edge_box(edge[0], edge[1], PATH_MARGIN)))
        }) || blocked(edge_box(self.path[2], self.rest, PATH_MARGIN))
            || blocked(edge_box(self.rest, self.rest, REST_RADIUS))
            || blocked(edge_box(self.marker, self.marker, 1.4))
    }

    pub fn bounds(&self) -> Area {
        let [[x, _, z], [end_x, _, end_z]] = self.mesh_bounds();
        [x, z, end_x, end_z]
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        let mut clearance: f32 = 0.0;
        for ([px, grade, pz], height, radius) in [(self.rest, 4.2, 4.0), (self.marker, 1.65, 1.25)]
        {
            let distance_squared = (x - px) * (x - px) + (z - pz) * (z - pz);
            let weight = 1.0 - smoothstep((distance_squared - radius * radius) / 140.0);
            clearance = clearance.max((grade + height + 1.0) * weight);
        }
        clearance
    }

    pub fn mesh_bounds(&self) -> [Point; 2] {
        let mut low = self.rest;
        let mut high = self.rest;
        for point in self.path.into_iter().chain([self.marker]) {
            for i in 0..3 {
                low[i] = low[i].min(point[i]);
                high[i] = high[i].max(point[i]);
            }
        }
        [
            [low[0] - 5.5, low[1] - 1.7, low[2] - 5.5],
            [high[0] + 5.5, high[1] + 4.5, high[2] + 5.5],
        ]
    }

    /// The trail mesh uses the same prepared terrain surface as the visible
    /// frame. Raised structures retain their surveyed foundations.
    pub fn draw(&self, mesh: &mut WorldMesh<'_>, ground: impl Fn(f32, f32) -> f32) {
        self.draw_path(mesh, ground);
        self.draw_props(mesh);
    }

    fn draw_path(&self, mesh: &mut WorldMesh<'_>, ground: impl Fn(f32, f32) -> f32) {
        let axes: [[f32; 2]; 4] = core::array::from_fn(|i| {
            unit([
                self.path[i + 1][0] - self.path[i][0],
                self.path[i + 1][2] - self.path[i][2],
            ])
        });
        let sections: [[f32; 2]; 5] = core::array::from_fn(|i| {
            let a = axes[i.saturating_sub(1)];
            let b = axes[i.min(3)];
            let join = unit([a[0] + b[0], a[1] + b[1]]);
            let widen = (join[0] * b[0] + join[1] * b[1]).max(0.67);
            [-join[1] / widen, join[0] / widen]
        });
        for (i, points) in self.path.windows(2).enumerate() {
            draw_path(
                mesh,
                &ground,
                points[0],
                points[1],
                sections[i],
                sections[i + 1],
                i == 3,
            );
        }
        let side = [-self.heading[1], self.heading[0]];
        draw_path(mesh, &ground, self.path[2], self.rest, side, side, false);
    }

    fn draw_props(&self, mesh: &mut WorldMesh<'_>) {
        let mut rest = Art {
            mesh,
            point: self.rest,
            front: self.heading,
        };
        if self.wind_shelter {
            rest.draw_wind_shelter();
        } else {
            rest.draw_map_board();
        }
        let mut marker = Art {
            mesh,
            point: self.marker,
            front: self.heading,
        };
        marker.block([0.0, -1.10, 0.0], [1.54, 1.80, 1.37], [131, 139, 131]);
        marker.block([0.0, 0.70, 0.0], [1.11, 0.50, 1.02], [183, 176, 149]);
        marker.block([0.0, 1.20, 0.0], [0.63, 0.45, 0.60], [157, 149, 127]);
    }
}

impl Art<'_, '_> {
    fn draw_wind_shelter(&mut self) {
        let stone = [154, 151, 139];
        self.block([0.0, -1.35, -1.94], [5.40, 4.43, 0.62], stone);
        for side in [-2.39, 2.39] {
            self.block([side, -1.35, 0.01], [0.62, 4.43, 3.90], stone);
        }
        self.block([0.0, 0.44, -1.28], [4.08, 0.39, 0.71], [157, 110, 68]);
        self.roof([0.0, 3.06, 0.0], 3.02, 2.51, 0.86, [164, 84, 52]);
    }

    fn draw_map_board(&mut self) {
        self.block([-1.82, -1.48, 0.16], [0.24, 5.06, 0.32], [87, 78, 67]);
        self.block([1.82, -1.48, 0.16], [0.24, 5.06, 0.32], [87, 78, 67]);
        self.block([0.0, 1.27, 0.16], [3.86, 1.68, 0.27], [132, 96, 59]);
        self.block([0.0, 1.52, 0.33], [3.40, 1.09, 0.11], [214, 186, 133]);
        self.roof([0.0, 3.54, 0.19], 2.09, 0.82, 0.62, [79, 95, 94]);
        // A separate stone-legged timber bench uses the small side clearing.
        self.block([-1.30, -1.28, -2.6], [0.38, 2.04, 0.63], [130, 128, 116]);
        self.block([1.30, -1.28, -2.6], [0.38, 2.04, 0.63], [130, 128, 116]);
        self.block([0.0, 0.77, -2.6], [3.27, 0.25, 0.92], [163, 120, 77]);
    }
}

fn certify_edge(world: &World, a: Point, b: Point, pulse: &mut impl FnMut()) -> bool {
    let delta = [b[0] - a[0], b[2] - a[2]];
    let length = length(delta);
    if length < 3.0 || (b[1] - a[1]).abs() > length * 0.34 || b[1] > a[1] + 1.7 {
        return false;
    }
    let side = [-delta[1] / length, delta[0] / length];
    let steps = (length / 2.0) as usize + 1;
    let mut old = [0.0; 3];
    for i in 0..=steps {
        if i & 7 == 0 {
            pulse();
        }
        let [x, _, z] = point_between(a, b, i as f32 / steps as f32);
        let samples: [f32; 3] = core::array::from_fn(|j| {
            let offset = (j as f32 - 1.0) * PATH_MARGIN;
            world.elevation(x + side[0] * offset, z + side[1] * offset)
        });
        if samples[1] <= 80.0
            || (samples[0] - samples[2]).abs() > 1.20
            || (i != 0
                && samples
                    .into_iter()
                    .zip(old)
                    .any(|(n, o)| (n - o).abs() * steps as f32 > length * 0.38))
        {
            return false;
        }
        old = samples;
    }
    true
}

fn certify_pad(world: &World, [x, y, z]: Point, radius: f32, grade: f32) -> bool {
    for row in -2..=2 {
        for column in -2..=2 {
            if row * row + column * column > 5 {
                continue;
            }
            let offset = |v| v as f32 * radius / 2.25;
            let sample = world.elevation(x + offset(column), z + offset(row));
            if sample <= 80.0 || (sample - y).abs() > grade {
                return false;
            }
        }
    }
    true
}

fn unit([x, z]: [f32; 2]) -> [f32; 2] {
    let magnitude = length([x, z]);
    [x / magnitude, z / magnitude]
}

fn length([x, z]: [f32; 2]) -> f32 {
    let n = x * x + z * z;
    let root = f32::from_bits((n.to_bits() >> 1) + 0x1fc0_0000);
    let root = (root + n / root) * 0.5;
    (root + n / root) * 0.5
}

fn edge_box(a: Point, b: Point, margin: f32) -> Area {
    [
        a[0].min(b[0]) - margin,
        a[2].min(b[2]) - margin,
        a[0].max(b[0]) + margin,
        a[2].max(b[2]) + margin,
    ]
}

fn point_between(a: Point, b: Point, t: f32) -> Point {
    core::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn draw_path(
    mesh: &mut WorldMesh<'_>,
    ground: &impl Fn(f32, f32) -> f32,
    a: Point,
    b: Point,
    start: [f32; 2],
    end: [f32; 2],
    taper: bool,
) {
    let steps = (length([b[0] - a[0], b[2] - a[2]]) / 3.0) as usize + 1;
    let mut last = [[0.0; 3]; 2];
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let border = if i == 0 {
            start
        } else if i == steps {
            end
        } else {
            unit([
                start[0] + (end[0] - start[0]) * t,
                start[1] + (end[1] - start[1]) * t,
            ])
        };
        let [x, _, z] = point_between(a, b, t);
        let width = WIDTH * if taper { 1.0 - t * t * 0.89 } else { 1.0 };
        let pair = [-width, width].map(|w| {
            let [x, z] = [x + border[0] * w, z + border[1] * w];
            [x, ground(x, z) + PATH_LIFT, z]
        });
        if i > 0 {
            let color = [164, 139, 105];
            mesh.lit_triangle([last[0], last[1], pair[1]], color);
            mesh.lit_triangle([last[0], pair[1], pair[0]], color);
        }
        last = pair;
    }
}

struct Art<'a, 'b> {
    mesh: &'a mut WorldMesh<'b>,
    point: Point,
    front: [f32; 2],
}

impl Art<'_, '_> {
    fn polygon(&mut self, points: [Point; 4], color: [u8; 3]) {
        let [a, b, c, d] = points.map(|[x, y, z]| {
            [
                self.point[0] + self.front[1] * x + self.front[0] * z,
                self.point[1] + y,
                self.point[2] - self.front[0] * x + self.front[1] * z,
            ]
        });
        self.mesh.lit_triangle([a, b, c], color);
        self.mesh.lit_triangle([a, c, d], color);
    }

    fn block(&mut self, [x, y, z]: Point, [width, height, depth]: Point, color: [u8; 3]) {
        let ring = |y| {
            [
                [x - width * 0.5, y, z - depth * 0.5],
                [x + width * 0.5, y, z - depth * 0.5],
                [x + width * 0.5, y, z + depth * 0.5],
                [x - width * 0.5, y, z + depth * 0.5],
            ]
        };
        let low = ring(y);
        let high = ring(y + height);
        for i in 0..4 {
            let next = (i + 1) % 4;
            self.polygon([low[i], low[next], high[next], high[i]], color);
        }
        self.polygon(high, color);
    }

    fn roof(&mut self, [x, y, z]: Point, span: f32, depth: f32, rise: f32, color: [u8; 3]) {
        for side in [-1.0, 1.0] {
            let edge = z + depth * side;
            self.polygon(
                [
                    [x - span, y + rise, z],
                    [x + span, y + rise, z],
                    [x + span, y, edge],
                    [x - span, y, edge],
                ],
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_summit_has_a_connected_walkable_switchback() {
        let world = World::new(204);
        let site = SummitSite::survey(&world, 204, [2, 0], |_| false, || {}).unwrap();
        let trail = SummitTrail::survey(&world, &site, |_| false, || {}).unwrap();
        assert!(trail.wind_shelter);
        assert_eq!(
            trail,
            SummitTrail::survey(&world, &site, |_| false, || {}).unwrap()
        );
        assert!(SummitTrail::survey(&world, &site, |_| true, || {}).is_none());
        assert_eq!(trail.path[0], [2054.0, 165.612_05, 224.6]);
        assert!(trail.path[4][1] < trail.path[0][1] - 5.0);
        for pair in trail.path.windows(2) {
            let distance = length([pair[1][0] - pair[0][0], pair[1][2] - pair[0][2]]);
            let steps = (distance / 0.5) as usize + 1;
            let mut prior = pair[0][1];
            for index in 1..=steps {
                let [x, _, z] = point_between(pair[0], pair[1], index as f32 / steps as f32);
                let elevation = world.elevation(x, z);
                assert!(elevation > 80.0);
                assert!((elevation - prior).abs() < distance * 0.40 / steps as f32);
                prior = elevation;
            }
        }
        let [x, ground, z] = trail.rest;
        assert!(trail.conflicts_with(|[a, b, c, d]| a <= x && x <= c && b <= z && z <= d));
        assert!(trail.obstruction(x, z) > ground + 4.0);
        assert_eq!(trail.obstruction(x + 22.0, z), 0.0);
    }
}
