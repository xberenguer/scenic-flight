//! Low-poly foreground silhouettes anchored to the surveyed desert surface.

use crate::math::sin_cos;
use crate::render::{Camera, ScreenProjection, ScreenRect, floor_i32};
use crate::world::{Sample, Surface};
use crate::world_mesh::WorldMesh;

const CELL: f32 = 12.0;
const REACH: f32 = 64.0;

#[derive(Clone, Copy)]
struct Prop {
    origin: [f32; 3],
    size: f32,
    kind: u8,
    opacity: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreground_props_follow_the_same_seed_and_respect_developed_ground() {
        let camera = Camera { x: -7.0, z: -11.0, altitude: 21.0, heading: 0.2, pitch: 0.45, bank: 0.0 };
        let placements = |seed, surface| {
            let sample = |_, _| Sample { surface, elevation: 4.0 };
            DesertProps::gather(camera, (72, 72), seed, sample, sample)
        };
        let original = placements(19, Surface::Desert);
        let repeated = placements(19, Surface::Desert);
        let different = placements(22, Surface::Desert);
        let points = |set: &DesertProps| set.props.map(|prop| prop.map(|p| p.origin));
        assert!(original.region().is_some());
        assert!(original.props.iter().flatten().count() >= 5);
        assert_eq!(points(&original), points(&repeated));
        assert_ne!(points(&original), points(&different));
        for surface in [Surface::Pavement, Surface::GrassRunway, Surface::RiverWater, Surface::CargoOchre] {
            assert!(placements(19, surface).region().is_none());
        }
    }
}

pub struct DesertProps {
    props: [Option<Prop>; 72],
    region: Option<ScreenRect>,
}

impl DesertProps {
    #[inline(never)]
    pub fn gather(
        camera: Camera, dimensions: (usize, usize), seed: u64,
        terrain: impl Fn(f32, f32) -> Sample, sample: impl Fn(f32, f32) -> Sample,
    ) -> Self {
        let mut result = Self { props: [None; 72], region: None };
        let (sin, cos) = sin_cos(camera.heading);
        let Some(projection) = ScreenProjection::new(camera, dimensions) else {
            return result;
        };
        let cx = floor_i32(camera.x / CELL);
        let cz = floor_i32(camera.z / CELL);
        let mut count = 0;
        for tz in cz - 6..=cz + 6 {
            for tx in cx - 6..=cx + 6 {
                let id = (u64::from(tx as u32) << 32) | u64::from(tz as u32);
                let key = crate::math::mix(seed ^ id ^ 0x98c4_c6a8_51ea_759f);
                let x = (tx as f32 + 0.17 + (key & 63) as f32 * 0.010) * CELL;
                let z = (tz as f32 + 0.17 + ((key >> 6) & 63) as f32 * 0.010) * CELL;
                let [dx, dz] = [x - camera.x, z - camera.z];
                let distance = dx * dx + dz * dz;
                let forward = sin * dx + cos * dz;
                if distance > REACH * REACH || forward < -12.0 {
                    continue;
                }
                if !matches!(terrain(x, z).surface, Surface::Desert | Surface::DuneSand | Surface::Sand) { continue; }
                let ground = sample(x, z);
                if !matches!(ground.surface, Surface::Desert | Surface::DuneSand) { continue; }
                let opacity = ((REACH * REACH - distance) / 1500.0).clamp(0.0, 1.0)
                    * ((66.0 - camera.altitude + ground.elevation) / 18.0).clamp(0.0, 1.0)
                    * ((forward + 12.0) / 6.0).clamp(0.0, 1.0);
                if opacity < 0.02 { continue; }
                let size = 1.12 + ((key >> 16) & 7) as f32 * 0.12;
                let y = ground.elevation - 0.14;
                let bounds = [[x - 3.1*size, y, z - 3.1*size],
                              [x + 3.1*size, y + 4.1*size, z + 3.1*size]];
                let Some(rect) = projection.bounds(bounds) else { continue; };
                result.region = Some(result.region.map_or(rect, |old| old.union(rect)));
                result.props[count] = Some(Prop { origin: [x, y, z], size, kind: (key >> 12 & 3) as u8, opacity });
                count += 1;
                if count == result.props.len() { return result; }
            }
        }
        result
    }

    pub fn region(&self) -> Option<ScreenRect> { self.region }

    pub fn draw(&self, mesh: &mut WorldMesh<'_>) {
        for prop in self.props.iter().flatten() {
            let mut triangle = |points: [[f32; 3]; 3], color| {
                mesh.faded_triangle(points.map(|point| {
                    core::array::from_fn(|i| prop.origin[i] + point[i] * prop.size)
                }), color, prop.opacity);
            };
            match prop.kind {
                0 => {
                    // Two intersecting fans form the main cactus with separate arms.
                    for swap in [false, true] {
                        let at = |x, y| if swap { [0.0, y, x] } else { [x, y, 0.0] };
                        triangle([at(-0.42,0.0), at(0.43,0.0), at(0.05,4.0)], [51,90,60]);
                        triangle([at(0.0,1.2), at(-1.34,2.1), at(-1.44,3.03)], [77,115,67]);
                        triangle([at(0.1,1.4), at(1.06,2.0), at(1.21,2.8)], [89,122,68]);
                    }
                }
                1 | 3 => {
                    let corners = [[-1.5,0.0,-0.8],[1.35,0.0,-1.2],[1.4,0.0,1.25],[-1.1,0.0,1.5]];
                    let top = [-0.29, if prop.kind == 3 { 1.48 } else { 0.83 }, 0.18];
                    for (i, color) in [[151,124,100],[114,98,87],[129,101,80],[173,145,118]].into_iter().enumerate() {
                        triangle([corners[i], corners[(i+1)%4], top], color);
                    }
                    triangle([[0.7,0.0,-0.4],[2.95,0.0,-0.7],[2.07,1.01,0.22]], [161,131,108]);
                    triangle([[2.95,0.0,-0.7],[2.52,0.0,0.84],[2.07,1.01,0.22]], [112,91,76]);
                }
                _ => {
                    for swap in [false, true] {
                        let at = |x, y| if swap { [0.0, y, x] } else { [x, y, 0.0] };
                        triangle([at(-1.27,0.2), at(0.41,0.0), at(-0.72,1.7)], [99,104,59]);
                        triangle([at(-0.41,0.0), at(1.54,0.3), at(0.66,1.5)], [130,126,68]);
                        triangle([at(1.27,0.0), at(2.7,0.18), at(2.07,1.17)], [113,118,72]);
                    }
                }
            }
        }
    }
}
