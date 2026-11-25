//! Small rock cascades fed by seepage on the downhill side of a tarn sill.

use crate::alpine_lakes::{AlpineLake, AlpineSurface};
use crate::math::{fraction, mix, smoothstep};
use crate::render::{screen_region, Camera, ScreenRect};
use crate::world::{Sample, Surface, World};
use crate::world_mesh::WorldMesh;

type Point = [f32; 2];
type Vertex = [f32; 3];
type Section = [Vertex; 2];

const SECTIONS: usize = 15;
const STEP: f32 = 1.65;
const SURFACE_LIFT: f32 = 0.145;
const MAX_REACH: f32 = 218.0;
const MOVES: [(f32, f32); 7] = [
    (1.0, 0.0),
    (0.965_925_8, 0.258_819),
    (0.965_925_8, -0.258_819),
    (0.866_025_4, 0.5),
    (0.866_025_4, -0.5),
    (0.707_106_8, 0.707_106_8),
    (0.707_106_8, -0.707_106_8),
];

/// A terrain-checked surface. The animation only shades the saved triangles.
#[derive(Clone, Debug, PartialEq)]
pub struct Waterfall {
    sections: [Section; SECTIONS],
    bounds: [Vertex; 2],
    count: u8,
    phase: u8,
}

impl Waterfall {
    pub(crate) fn survey(
        world: &World,
        lake: &AlpineLake,
        seed: u64,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let outlet = lake.outlet()?;
        let crest = xz(outlet.crest);
        let mut here = xz(outlet.downstream);
        let forward = normal([here[0] - crest[0], here[1] - crest[1]])?;
        let [fx, _, fz] = lake.floor();
        let uphill = normal([fx - crest[0], fz - crest[1]])?;
        if ![2.0, 4.0, 6.0, 9.0, 12.0, 16.0]
            .into_iter()
            .any(|distance| {
                let [x, z] = offset(crest, uphill, distance);
                covers_tarn(lake, x, z, world.elevation(x, z))
            })
        {
            return None;
        }

        let roll =
            mix(seed ^ (u64::from(crest[0].to_bits()) << 32) ^ u64::from(crest[1].to_bits()));
        let width = 0.52 + 0.36 * fraction(roll.rotate_left(21));
        let area = lake.bounds();
        let mut heading = forward;
        let mut nodes = [[0.0; 3]; SECTIONS];
        let mut count = 0;
        let mut rocky = 0;
        let mut steep = 0;
        // Start below the lake's hydraulic head after leaving the dry lip.
        // The interior flow is subsurface, so no mesh is placed on the sill.
        for _ in 0..SECTIONS + 8 {
            pulse();
            let [x, z] = here;
            let ground = world.sample(x, z);
            if !inside(area, here, 1.15)
                || !matches!(
                    ground.surface,
                    Surface::Rock | Surface::Snow | Surface::Grass
                )
                || covers_tarn(lake, x, z, ground.elevation)
            {
                break;
            }
            if count > 0 || (0.32..=2.9).contains(&(lake.level() - ground.elevation)) {
                if count == 0
                    && (ground.surface != Surface::Rock || squared_distance(crest, here) > 225.0)
                {
                    return None;
                }
                nodes[count] = [x, ground.elevation, z];
                if count != 0 {
                    steep += usize::from(nodes[count - 1][1] - ground.elevation > STEP * 0.42);
                }
                rocky += usize::from(ground.surface == Surface::Rock);
                count += 1;
                if count == SECTIONS {
                    break;
                }
            }
            let Some((next, direction)) =
                downhill(world, here, ground.elevation, heading, forward, pulse)
            else {
                break;
            };
            here = next;
            heading = direction;
        }
        if count < 6 || steep < 2 || rocky * 2 < count || nodes[0][1] - nodes[count - 1][1] < 4.2 {
            return None;
        }
        let nodes = &nodes[..count];
        let mut sections = [[[0.0; 3]; 2]; SECTIONS];
        for (i, &node) in nodes.iter().enumerate() {
            let before = xz(nodes[i.saturating_sub(1)]);
            let after = xz(nodes[(i + 1).min(count - 1)]);
            let [dx, dz] = normal([after[0] - before[0], after[1] - before[1]])?;
            let onset = smoothstep((i as f32 + 0.6) * 0.48);
            let toe = smoothstep((count - i) as f32 * 0.40);
            let spread = 0.88 + 0.17 * fraction(mix(roll ^ i as u64));
            let half = width * (0.35 + 0.65 * onset * toe) * spread;
            for (side, lateral) in [-half, half].into_iter().enumerate() {
                let [x, z] = offset(xz(node), [-dz, dx], lateral);
                let ground = world.elevation(x, z);
                let roof = if i == 0 {
                    lake.level() - 0.03
                } else {
                    sections[i - 1][side][1] - 0.025
                };
                let y = (ground + SURFACE_LIFT).min(roof);
                if y < ground + 0.045 || !inside(area, [x, z], 0.05) {
                    return None;
                }
                sections[i][side] = [x, y, z];
            }
        }
        let mut bounds = [sections[0][0]; 2];
        for section in &sections[..count] {
            for &p in section {
                for (axis, coordinate) in p.into_iter().enumerate() {
                    bounds[0][axis] = bounds[0][axis].min(coordinate);
                    bounds[1][axis] = bounds[1][axis].max(coordinate);
                }
            }
        }
        let fall = Self {
            sections,
            bounds,
            count: count as u8,
            phase: roll as u8,
        };
        for edge in fall.sections().windows(2) {
            for triangle in strip(edge[0], edge[1]) {
                pulse();
                if triangle_samples(triangle, |[x, y, z]| {
                    let ground = world.elevation(x, z);
                    ground - y > 0.06 || y - ground > 0.32 || covers_tarn(lake, x, z, ground)
                }) {
                    return None;
                }
            }
        }
        Some(fall)
    }

    /// Downhill mesh cross-sections from the rock emergence toward scree.
    pub fn sections(&self) -> &[Section] {
        &self.sections[..usize::from(self.count)]
    }

    pub fn bounds(&self) -> [Vertex; 2] {
        self.bounds
    }

    pub fn conflicts_with(&self, mut occupied: impl FnMut(f32, f32) -> bool) -> bool {
        self.sections().windows(2).any(|edge| {
            strip(edge[0], edge[1])
                .into_iter()
                .any(|triangle| triangle_samples(triangle, |[x, _, z]| occupied(x, z)))
        })
    }

    pub fn screen_region(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        (self.opacity(camera) > 0.03).then_some(())?;
        screen_region(camera, dimensions, self.bounds)
    }

    pub fn draw(&self, camera: Camera, ticks: u64, mesh: &mut WorldMesh<'_>) {
        let visibility = self.opacity(camera);
        if visibility <= 0.03 {
            return;
        }
        let clock = (ticks as u8).wrapping_mul(5).wrapping_add(self.phase);
        for (i, edge) in self.sections().windows(2).enumerate() {
            let [a, b] = [edge[0], edge[1]];
            let descent = ((a[0][1] + a[1][1]) - (b[0][1] + b[1][1])) * 0.5;
            let foam = smoothstep((descent - 0.29) / 0.75);
            let wave = clock.wrapping_sub((i as u8).wrapping_mul(32));
            let pulse = f32::from(wave.min(255 - wave)) * (1.0 / 127.0);
            let color = [
                (75.0 + foam * 48.0 + pulse * 12.0) as u8,
                (128.0 + foam * 43.0 + pulse * 10.0) as u8,
                (150.0 + foam * 40.0 + pulse * 10.0) as u8,
            ];
            let toe = (self.sections().len() - i - 1) as f32;
            let alpha = visibility * smoothstep((toe + 0.45) * 0.48);
            for triangle in strip(a, b) {
                mesh.faded_triangle(triangle, color, alpha);
            }
            if foam > 0.20 && visibility > 0.54 && toe > 1.5 {
                let stripe =
                    0.26 + 0.20 * (f32::from(self.phase.wrapping_add(i as u8 * 17) & 15) / 15.0);
                let inset =
                    |section: Section| [across(section, stripe), across(section, stripe + 0.34)];
                let gray = (176.0 + foam * 29.0 + pulse * 19.0) as u8;
                for triangle in strip(inset(a), inset(b)) {
                    mesh.faded_triangle(triangle, [gray - 19, gray, gray + 12], alpha);
                }
            }
        }
    }

    fn opacity(&self, camera: Camera) -> f32 {
        let [x, y, z] = [camera.x, camera.altitude, camera.z];
        let [lo, hi] = self.bounds;
        let [dx, dy, dz] = [
            x - x.clamp(lo[0], hi[0]),
            y - y.clamp(lo[1], hi[1]),
            z - z.clamp(lo[2], hi[2]),
        ];
        let squared = dx * dx + dy * dy + dz * dz;
        1.0 - smoothstep((squared - 118.0 * 118.0) / (MAX_REACH * MAX_REACH - 118.0 * 118.0))
    }
}

fn downhill(
    world: &World,
    here: Point,
    height: f32,
    [u, v]: Point,
    forward: Point,
    pulse: &mut impl FnMut(),
) -> Option<(Point, Point)> {
    let mut best = None;
    let mut score = f32::INFINITY;
    for (cos, sin) in MOVES {
        pulse();
        let heading = [u * cos - v * sin, u * sin + v * cos];
        if dot(heading, forward) < 0.35 {
            continue;
        }
        let next = offset(here, heading, STEP);
        let end = world.elevation(next[0], next[1]);
        let cost = end + 0.095 * sin.abs();
        if height - end < 0.12 || cost >= score {
            continue;
        }
        let mut last = height;
        let mut slopes = true;
        for i in 1..=5 {
            let [x, z] = offset(here, heading, STEP * i as f32 * 0.20);
            let y = world.elevation(x, z);
            if y > last + 0.011 {
                slopes = false;
                break;
            }
            last = y;
        }
        if slopes {
            best = Some((next, heading));
            score = cost;
        }
    }
    best
}

fn covers_tarn(lake: &AlpineLake, x: f32, z: f32, elevation: f32) -> bool {
    lake.sample(
        x,
        z,
        Sample {
            elevation,
            surface: Surface::Rock,
        },
    )
    .is_some_and(|sample| {
        matches!(
            sample.surface,
            AlpineSurface::Tarn | AlpineSurface::TarnShallows
        )
    })
}

fn inside([lo_x, lo_z, hi_x, hi_z]: [f32; 4], [x, z]: Point, pad: f32) -> bool {
    (lo_x + pad..=hi_x - pad).contains(&x) && (lo_z + pad..=hi_z - pad).contains(&z)
}

fn normal([x, z]: Point) -> Option<Point> {
    let squared = x * x + z * z;
    if squared < 0.01 {
        return None;
    }
    let mut length = x.abs().max(z.abs());
    for _ in 0..3 {
        length = 0.5 * (length + squared / length);
    }
    Some([x / length, z / length])
}

fn xz([x, _, z]: Vertex) -> Point {
    [x, z]
}
fn dot(a: Point, b: Point) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}
fn squared_distance(a: Point, b: Point) -> f32 {
    let delta = [b[0] - a[0], b[1] - a[1]];
    dot(delta, delta)
}
fn offset(point: Point, direction: Point, distance: f32) -> Point {
    [
        point[0] + distance * direction[0],
        point[1] + distance * direction[1],
    ]
}

fn strip(a: Section, b: Section) -> [[Vertex; 3]; 2] {
    [[a[0], a[1], b[1]], [a[0], b[1], b[0]]]
}

fn across([a, b]: Section, width: f32) -> Vertex {
    [
        a[0] + (b[0] - a[0]) * width,
        a[1] + (b[1] - a[1]) * width,
        a[2] + (b[2] - a[2]) * width,
    ]
}

fn triangle_samples([a, b, c]: [Vertex; 3], mut test: impl FnMut(Vertex) -> bool) -> bool {
    for i in 0..=4 {
        for j in 0..=4 - i {
            let [s, t] = [i as f32 * 0.25, j as f32 * 0.25];
            let p = core::array::from_fn(|k| a[k] + (b[k] - a[k]) * s + (c[k] - a[k]) * t);
            if test(p) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposed_cascade_starts_below_the_lake_and_descends_the_rock_face() {
        for (seed, x, z) in [(3, 4, -3), (29, -9, 11), (41, 11, 9)] {
            let world = World::new(seed);
            let lake = AlpineLake::generate(&world, seed, x, z).unwrap();
            assert_eq!(Some(lake.clone()), AlpineLake::generate(&world, seed, x, z));
            let fall = lake.waterfall().unwrap();
            let [low, high] = fall.bounds();
            assert!(high[1] < lake.level() && high[1] - low[1] > 4.2);
            let crest = lake.outlet().unwrap().crest;
            assert!(crest[1] > lake.level() && high[1] < crest[1]);
            let endpoint = fall.sections()[0][0];
            assert!(squared_distance(xz(endpoint), xz(crest)) <= 225.0);
            assert!(lake.conflicts_with(|x, z| squared_distance([x, z], xz(endpoint)) < 1e-6));
            let area = lake.bounds();
            for edge in fall.sections().windows(2) {
                for (a, b) in edge[0].into_iter().zip(edge[1]) {
                    assert!(b[1] < a[1]);
                }
                for [a, b, c] in strip(edge[0], edge[1]) {
                    for i in 0..=12 {
                        for j in 0..=12 - i {
                            let t = i as f32 / 12.0;
                            let u = j as f32 / 12.0;
                            let [x, y, z] = core::array::from_fn(|k| {
                                a[k] * (1.0 - t - u) + b[k] * t + c[k] * u
                            });
                            let ground = world.elevation(x, z);
                            assert!((-0.08..=0.34).contains(&(y - ground)));
                            assert!(inside(area, [x, z], 0.0));
                            assert!(!covers_tarn(&lake, x, z, ground));
                        }
                    }
                }
            }
        }
        let closed = AlpineLake::generate(&World::new(1), 1, -2, 3).unwrap();
        assert!(closed.outlet().is_none() && closed.waterfall().is_none());
        let gentle = AlpineLake::generate(&World::new(7), 7, 0, -6).unwrap();
        assert!(gentle.outlet().is_some() && gentle.waterfall().is_none());
    }
}
