//! Low-poly foreground scenery anchored to surveyed desert and wetland surfaces.

mod habitats;

use habitats::Feature;
use crate::math::sin_cos;
use crate::render::{Camera, ScreenProjection, ScreenRect, floor_i32};
use crate::world::{Sample, Surface};
use crate::world_mesh::WorldMesh;

const CELL: f32 = 12.0;
const REACH: f32 = 64.0;
const HALF_SPAN: f32 = 3.1;
const HEIGHT: f32 = 4.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PropKind {
    Cactus,
    Rock,
    Bush,
    WetlandPlant,
    Habitat(Feature),
}

#[derive(Clone, Copy)]
struct Prop {
    origin: [f32; 3],
    grade: [f32; 2],
    size: f32,
    kind: PropKind,
    variant: u8,
    opacity: f32,
}

impl Prop {
    fn triangles(self, emit: impl FnMut([[f32; 3]; 3], [u8; 3])) {
        // Three four-step proportions in each of four families make 256 meshes per kind.
        let [a, b, c, d] = [0, 2, 4, 6].map(|shift| usize::from((self.variant >> shift) & 3));
        match self.kind {
            PropKind::Cactus => cactus_triangles(a, b, c, d, emit),
            PropKind::Rock => rock_triangles(a, b, c, d, emit),
            PropKind::Bush => bush_triangles(a, b, c, d, emit),
            PropKind::WetlandPlant => wetland_triangles(a, b, c, d, emit),
            PropKind::Habitat(feature) => feature.draw(self.variant, emit),
        }
    }
}

fn cactus_triangles(
    girth: usize, stature: usize, spread: usize, species: usize,
    mut triangle: impl FnMut([[f32; 3]; 3], [u8; 3]),
) {
    let girth = [0.78, 1.0, 1.23, 1.48][girth];
    let height = [0.72, 0.87, 1.0, 1.15][stature];
    let spread = [0.70, 1.0, 1.28, 1.52][spread];
    for swap in [false, true] {
        let at = |x, y| if swap { [0.0, y, x] } else { [x, y, 0.0] };
        let mut pad = |x: f32, low: f32, high: f32, w: f32, lean: f32, color| {
            let waist = low + (high - low) * 0.57;
            let [a, b, c, d] = [at(x, low), at(x - w, waist), at(x + lean, high), at(x + w, waist)];
            triangle([a, b, c], color);
            triangle([a, c, d], color);
        };
        match species {
            0 => {
                // Saguaro: tall central stem and unequal arms.
                pad(0.0, 0.0, height * 3.48, girth * 0.37, 0.08, [55, 103, 65]);
                pad(-spread * 0.53, height * 0.82, height * 2.40, girth * 0.35,
                    -spread * 0.40, [77, 128, 73]);
                pad(spread * 0.46, height * 1.25, height * 3.06, girth * 0.29,
                    spread * 0.49, [67, 117, 66]);
            }
            1 => {
                // Organ pipe: a group of tall stems rising from the same root.
                for [side, top, lean] in [[-0.62, 2.12, -0.28], [0.02, 3.38, 0.16], [0.58, 2.72, 0.40]] {
                    pad(side * spread, 0.0, height * top, girth * 0.26,
                        lean * spread, [60, 121, 104]);
                }
            }
            2 => {
                // Prickly pear: branching, broad pads close to the ground.
                pad(0.0, 0.0, height * 1.42, girth * 0.56, -spread * 0.14, [123, 151, 79]);
                pad(-spread * 0.66, height * 0.45, height * 1.65,
                    girth * 0.48, -spread * 0.53, [101, 143, 85]);
                pad(spread * 0.59, height * 0.49, height * 1.90,
                    girth * 0.59, spread * 0.47, [130, 158, 87]);
            }
            _ => {
                // Golden barrel: squat ribbed body with a second offset head.
                pad(0.0, 0.0, height * 1.44, girth * 0.85, spread * 0.10, [159, 155, 77]);
                pad(spread * 0.74, 0.0, height * 0.87,
                    girth * 0.55, spread * 0.19, [145, 129, 63]);
                pad(-spread * 0.45, 0.0, height * 1.06,
                    girth * 0.53, -spread * 0.23, [176, 157, 79]);
            }
        }
    }
}

fn rock_triangles(
    width: usize, depth: usize, relief: usize, family: usize,
    mut triangle: impl FnMut([[f32; 3]; 3], [u8; 3]),
) {
    let width = [0.88, 1.15, 1.44, 1.73][width];
    let depth = [0.78, 1.04, 1.32, 1.62][depth];
    let relief = [0.72, 1.0, 1.32, 1.67][relief];
    match family {
        0 => {
            // Tumbled granite: broad irregular dome, pale facets and a grounded chip.
            let ring = [
                [-1.05 * width, 0.0, -0.37 * depth],
                [-0.34 * width, 0.0, -0.96 * depth],
                [0.76 * width, 0.0, -0.64 * depth],
                [1.14 * width, 0.0, 0.32 * depth],
                [-0.36 * width, 0.0, 0.94 * depth],
            ];
            let peak = [-0.23 * width, 0.72 * relief, 0.12 * depth];
            let tones = [[130, 125, 114], [177, 169, 152], [150, 143, 131],
                         [121, 116, 106], [161, 153, 139]];
            for (index, color) in tones.into_iter().enumerate() {
                triangle([ring[index], ring[(index + 1) % ring.len()], peak], color);
            }
            triangle([[0.74 * width, 0.0, 0.45 * depth], [1.02 * width, 0.0, 0.90 * depth],
                      [0.85 * width, 0.24 * relief, 0.65 * depth]], [149, 140, 127]);
        }
        1 => {
            // Sandstone ledge: long tabletop, low skirt and a rust-colored cut face.
            let bottom = [[-1.34 * width, 0.0, -0.53 * depth], [1.40 * width, 0.0, -0.72 * depth],
                          [1.03 * width, 0.0, 0.42 * depth], [-1.16 * width, 0.0, 0.64 * depth]];
            let roof = [[-0.99 * width, 0.43 * relief, -0.39 * depth],
                        [1.16 * width, 0.58 * relief, -0.42 * depth],
                        [0.97 * width, 0.51 * relief, 0.34 * depth],
                        [-1.03 * width, 0.35 * relief, 0.44 * depth]];
            triangle([roof[0], roof[1], roof[2]], [192, 126, 80]);
            triangle([roof[0], roof[2], roof[3]], [180, 108, 66]);
            triangle([bottom[0], bottom[1], roof[1]], [138, 75, 54]);
            triangle([bottom[0], roof[1], roof[0]], [158, 82, 54]);
            triangle([bottom[1], bottom[2], roof[2]], [126, 68, 46]);
            triangle([bottom[2], bottom[3], roof[3]], [160, 91, 56]);
        }
        2 => {
            // Basalt shards: tall, narrow paired blades with charcoal facets.
            let shards = [
                ([-0.36 * width, 0.0, -0.30 * depth], [-0.18 * width, 1.19 * relief, -0.08 * depth]),
                ([0.60 * width, 0.0, 0.57 * depth], [0.74 * width, 0.61 * relief, 0.38 * depth]),
            ];
            let tones = [[60, 63, 73], [91, 88, 93], [73, 72, 80]];
            for ([x, _, z], peak) in shards {
                let base = [[x - 0.42 * width, 0.0, z - 0.12 * depth],
                            [x + 0.39 * width, 0.0, z - 0.30 * depth],
                            [x + 0.14 * width, 0.0, z + 0.48 * depth]];
                for (index, color) in tones.into_iter().enumerate() {
                    triangle([base[index], base[(index + 1) % base.len()], peak], color);
                }
            }
        }
        _ => {
            // Pale scree: a low fan of loose fragments, with ground between them.
            for [x, z, span, rise] in [[-0.88, -0.35, 0.57, 0.34],
                                      [0.19, -0.51, 0.40, 0.22],
                                      [0.76, 0.59, 0.48, 0.42]] {
                let a = [(x - span) * width, 0.0, (z - 0.12) * depth];
                let b = [(x + span) * width, 0.0, (z - 0.23) * depth];
                let c = [(x + 0.25 * span) * width, 0.0, (z + 0.45) * depth];
                let peak = [(x - 0.13 * span) * width, rise * relief, (z + 0.15) * depth];
                triangle([a, b, peak], [172, 155, 125]);
                triangle([a, peak, c], [198, 180, 141]);
            }
        }
    }
}

fn bush_triangles(
    width: usize, height: usize, spread: usize, family: usize,
    mut triangle: impl FnMut([[f32; 3]; 3], [u8; 3]),
) {
    let width = [0.78, 1.0, 1.27, 1.53][width];
    let height = [0.72, 0.93, 1.18, 1.45][height];
    let spread = [0.67, 0.96, 1.27, 1.59][spread];
    // Cross two planes to keep the families recognizable on changing headings.
    for swap in [false, true] {
        let at = |x, y| if swap { [0.0, y, x] } else { [x, y, 0.0] };
        let mut shape = |points: [[f32; 2]; 3], color| {
            triangle(points.map(|[x, y]| at(x, y)), color);
        };
        match family {
            0 => {
                // Sagebrush: cool rounded cushion and a low asymmetric skirt.
                let left = [-1.05 * width, 0.0];
                let shoulder = [-0.61 * width, 0.56 * height];
                let top = [0.20 * width, 0.83 * height];
                let right = [0.92 * width, 0.0];
                shape([left, shoulder, right], [87, 115, 104]);
                shape([shoulder, top, right], [107, 135, 117]);
                shape([[0.40 * width, 0.0], [0.84 * width + 0.26 * spread, (0.35 + 0.25 * spread) * height],
                       [0.87 * width + 0.82 * spread, 0.0]], [91, 122, 103]);
            }
            1 => {
                // Bunchgrass: golden blades that flare from a narrow tuft.
                shape([[-0.27 * width, 0.0], [0.02 * width, 0.0], [-0.98 * spread, 1.02 * height]], [158, 138, 74]);
                shape([[-0.12 * width, 0.0], [0.31 * width, 0.0], [0.14 * spread + 1.03 * (width - 1.0), 1.48 * height]], [186, 165, 94]);
                shape([[0.02 * width, 0.0], [0.48 * width, 0.0], [1.17 * spread, 0.80 * height]], [151, 126, 68]);
            }
            2 => {
                // Saltbush: wind-pruned silver-green scrub with open lobes.
                shape([[-0.28 * width, 0.0], [0.68 * width, 0.0], [0.72 * spread, 0.79 * height]], [107, 123, 104]);
                shape([[-1.04 * spread, 0.0], [-0.26 * width, 0.0], [-0.69 * spread, 0.52 * height]], [132, 143, 119]);
                shape([[0.40 * width, 0.0], [1.35 * spread, 0.0], [1.11 * spread, 0.49 * height]], [123, 133, 116]);
            }
            _ => {
                // Flowering yucca: broad pointed blades under one pale stalk.
                shape([[-0.39 * width, 0.0], [0.16 * width, 0.0], [-1.16 * spread, 0.76 * height]], [73, 106, 65]);
                shape([[-0.14 * width, 0.0], [0.55 * width, 0.0], [1.29 * spread, 1.01 * height]], [106, 131, 67]);
                shape([[-0.16 * width, 0.0], [0.20 * width, 0.0], [0.20 * spread, 1.89 * height]], [184, 160, 105]);
            }
        }
    }
}

fn wetland_triangles(
    width: usize, height: usize, spread: usize, family: usize,
    mut triangle: impl FnMut([[f32; 3]; 3], [u8; 3]),
) {
    let w = [0.22, 0.42, 0.62, 0.83][width];
    let h = [0.96, 1.29, 1.68, 2.12][height];
    let s = [0.44, 0.78, 1.15, 1.52][spread];
    for swap in [false, true] {
        let at = |x, y| if swap { [0.0, y, x] } else { [x, y, 0.0] };
        let mut blade = |a: [f32; 2], b: [f32; 2], tip: [f32; 2], color| {
            triangle([at(a[0], a[1]), at(b[0], b[1]), at(tip[0], tip[1])], color);
        };
        match family {
            0 => {
                // Bulrushes: tall straw tips clustered along a silt margin.
                blade([-0.74 * w, 0.0], [0.08, 0.0], [-0.89 * s, h * 1.19], [143, 156, 86]);
                blade([0.11, 0.0], [0.79 * w, 0.0], [0.81 * s, h * 0.91], [102, 139, 64]);
                blade([-0.31 * w, 0.0], [0.38 * w, 0.0], [-0.17 * s, h * 1.43], [174, 164, 102]);
            }
            1 => {
                // Cattail: chocolate seed head above a narrow leaf.
                blade([-0.66 * w, 0.0], [0.05, 0.0], [-0.89 * s, h * 0.62], [104, 144, 67]);
                blade([0.02, 0.0], [0.71 * w, 0.0], [0.63 * s, h * 1.06], [125, 154, 74]);
                let x = 0.63 * s;
                blade([x - w * 0.40, h * 0.91], [x + w * 0.36, h * 0.89], [x + w * 0.22, h * 1.46], [117, 79, 53]);
            }
            2 => {
                // Iris: broad saffron bloom at the end of a low fan of leaves.
                blade([-w * 0.9, 0.0], [w * 0.19, 0.0], [-1.05 * s, h * 0.50], [78, 131, 87]);
                blade([-w * 0.28, 0.0], [w, 0.0], [0.84 * s, h * 0.69], [117, 151, 78]);
                let x = 0.63 * s;
                blade([x - w * 0.83, h * 0.63], [x + w * 0.90, h * 0.71], [x + w * 0.47, h * 1.12], [238, 189, 82]);
            }
            _ => {
                // Willow scrub: broad silver-green crowns and exposed low branches.
                blade([-w, 0.0], [w * 0.53, 0.0], [-s * 0.82, h * 0.86], [101, 139, 115]);
                blade([-0.55 * w, 0.0], [0.92 * w, 0.0], [1.14 * s, h * 0.57], [123, 153, 124]);
                blade([-s * 0.66, h * 0.54], [s * 0.69, h * 0.44], [-0.27 * s, h * 1.04], [163, 174, 129]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::Sky;
    use std::collections::BTreeMap;

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
        let points = |set: &DesertProps| set.props.map(|prop| prop.map(|p| (p.origin, p.kind, p.variant)));
        assert!(original.region().is_some());
        assert!(original.props.iter().flatten().count() >= 5);
        assert_eq!(points(&original), points(&repeated));
        assert_ne!(points(&original), points(&different));
        for surface in [Surface::Pavement, Surface::GrassRunway, Surface::RiverWater, Surface::CargoOchre] {
            assert!(placements(19, surface).region().is_none());
        }
    }

    #[test]
    fn river_vegetation_uses_approved_banks_and_only_the_matching_families() {
        let camera = Camera { x: 17.0, z: -13.0, altitude: 21.0, heading: 0.48, pitch: 0.50, bank: 0.0 };
        let raw = |_, _| Sample { surface: Surface::Grass, elevation: 3.0 };
        let near = |surface, enabled| {
            DesertProps::gather_with_wetlands(camera, (60, 60), 79, enabled, raw,
                |_, _| Sample { surface, elevation: 3.0 })
        };
        for (surface, families) in [(Surface::ReedMarsh, 0..2), (Surface::WetMeadow, 2..4), (Surface::RiverSilt, 3..4)] {
            let props = near(surface, true);
            let mut count = 0;
            for prop in props.props.iter().flatten() {
                assert_eq!(prop.kind, PropKind::WetlandPlant);
                assert!(families.contains(&(prop.variant >> 6)));
                count += 1;
            }
            assert!(count >= 2);
            assert!(near(surface, false).region().is_none());
        }
        for surface in [Surface::RiverWater, Surface::Pavement] {
            assert!(near(surface, true).region().is_none());
        }
    }

    #[test]
    fn habitat_placement_draws_thirty_distinct_grounded_forms() {
        use std::collections::HashSet;

        let habitats = [
            (Surface::Desert, 8.0),
            (Surface::Grass, 18.0),
            (Surface::Forest, 28.0),
            (Surface::AlpineMeadow, 82.0),
            (Surface::Sand, 1.4),
            (Surface::WetMeadow, 3.0),
        ];
        let mut seen = HashSet::new();
        for (surface, elevation) in habitats {
            let camera = Camera { x: 17.0, z: -13.0, altitude: elevation + 16.0,
                heading: 0.48, pitch: 0.50, bank: 0.0 };
            let ground = |_, _| Sample { surface, elevation };
            let mut forms = HashSet::new();
            for seed in 0..96 {
                let props = DesertProps::gather_with_habitats(camera, (60, 60), seed, true, ground, ground);
                for prop in props.props.into_iter().flatten() {
                    if let PropKind::Habitat(form) = prop.kind {
                        assert!(props.region.is_some());
                        if !matches!(form, Feature::Shore(_) | Feature::Mountain(_) | Feature::Freshwater(_) | Feature::Woodland(_)) {
                            forms.insert(form);
                        }
                    }
                }
            }
            assert_eq!(forms.len(), 5, "{surface:?} forms: {forms:?}");
            seen.extend(forms);
            let road = |_, _| Sample { surface: Surface::Pavement, elevation };
            assert!(DesertProps::gather_with_habitats(camera, (60, 60), 1, true, ground, road).region().is_none());
        }
        assert_eq!(seen.len(), 30);

        const SIDE: usize = 60;
        let view = Camera { x: -3.5, z: -7.0, altitude: 3.8, heading: 0.464, pitch: 0.30, bank: 0.0 };
        let sky = Sky::from_time(41, 0, 0.48);
        let mut silhouettes = HashSet::new();
        for form in seen {
            for variant in 0..=u8::MAX {
                let mut grounded = false;
                form.draw(variant, |points, _| {
                    for [x, y, z] in points {
                        assert!(x.abs() <= HALF_SPAN && z.abs() <= HALF_SPAN, "{form:?}: {x}, {z}");
                        assert!((0.0..=HEIGHT).contains(&y), "{form:?}: {y}");
                        grounded |= y == 0.0;
                    }
                });
                assert!(grounded, "{form:?}");
            }
            let mut pixels = [0; SIDE * SIDE];
            let mut depth = [u16::MAX; SIDE * SIDE];
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), view, &sky).unwrap();
            form.draw(0, |points, _| mesh.triangle(points, [255; 3]));
            let silhouette = pixels.map(|color| color != 0);
            assert!(silhouette.contains(&true), "{form:?}");
            assert!(silhouettes.insert(silhouette), "{form:?} duplicates another silhouette");
        }
    }

    #[test]
    fn woodland_models_spawn_in_real_clearings_and_keep_distinct_shapes() {
        use crate::world::World;
        use std::collections::HashSet;

        let mut forms = HashSet::new();
        for seed in [1, 11, 19, 23, 29, 41, 49, 67, 79, 98, 112, 119] {
            let world = World::new(seed);
            let ground = |x, z| world.sample(x, z);
            for row in -19..=19 {
                for col in -19..=19 {
                    let (x, z) = (col as f32 * 53.0, row as f32 * 53.0);
                    let floor = ground(x, z);
                    if floor.surface != Surface::Forest { continue; }
                    let view = Camera { x, z: z - 5.0, altitude: floor.elevation + 14.0,
                        heading: 0.32, pitch: 0.53, bank: 0.0 };
                    let scene = DesertProps::gather_with_habitats(view, (60, 60), seed, true, ground, ground);
                    for prop in scene.props.into_iter().flatten() {
                        if let PropKind::Habitat(form @ Feature::Woodland(_)) = prop.kind {
                            let [x, _, z] = prop.origin;
                            assert_eq!(ground(x, z).surface, Surface::Forest);
                            forms.insert(form);
                        }
                    }
                }
                if forms.len() == 25 { break; }
            }
            if forms.len() == 25 { break; }
        }
        assert_eq!(forms.len(), 25, "Seeded woodland models: {forms:?}");

        const SIDE: usize = 60;
        let view = Camera { x: -3.5, z: -7.0, altitude: 3.8, heading: 0.464, pitch: 0.30, bank: 0.0 };
        let sky = Sky::from_time(41, 0, 0.48);
        let mut silhouettes = HashSet::new();
        for form in forms {
            for variant in 0..=u8::MAX {
                let mut grounded = false;
                form.draw(variant, |points, _| {
                    for [x, y, z] in points {
                        assert!(x.abs() <= HALF_SPAN && z.abs() <= HALF_SPAN, "{form:?}: {x}, {z}");
                        assert!((0.0..=HEIGHT).contains(&y), "{form:?}: {y}");
                        grounded |= y == 0.0;
                    }
                });
                assert!(grounded, "{form:?}");
            }
            let mut pixels = [0; SIDE * SIDE];
            let mut depth = [u16::MAX; SIDE * SIDE];
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), view, &sky).unwrap();
            form.draw(0, |points, _| mesh.triangle(points, [255; 3]));
            let silhouette = pixels.map(|color| color != 0);
            assert!(silhouette.contains(&true), "{form:?}");
            assert!(silhouettes.insert(silhouette), "{form:?} duplicates another silhouette");
        }
    }

    #[test]
    fn freshwater_models_cover_seeded_floodplains_and_keep_the_channel_clear() {
        use crate::river_district::RiverDistrict;
        use crate::watershed::{Watershed, WatershedWork};
        use crate::world::World;
        use std::collections::HashSet;

        let mut forms = HashSet::new();
        let mut work = WatershedWork::new();
        let fixtures = [(22, 0, 0), (14_692, 2, 0), (14_692, -6, -5),
                        (1, 1, -1), (49, 0, 0), (98, 1, 2)];
        let surveyed = [1, 19, 22, 49, 79, 98, 119, 14_692].into_iter().flat_map(|seed| {
            (-2..=2).flat_map(move |rx| (-2..=2).map(move |rz| (seed, rx, rz)))
        });
        for (seed, rx, rz) in fixtures.into_iter().chain(surveyed) {
            if forms.len() == 25 { break; }
            let world = World::new(seed);
            let mut water = Watershed::new();
            water.rebuild(&world, seed, rx, rz, &mut work);
            let Some(plan) = RiverDistrict::from_prepared_land_cooperatively(
                &world, None, water, |_, _| false, &mut || {},
            ) else { continue; };
            let raw = |x, z| world.sample(x, z);
            let developed = |x, z| plan.overlay(x, z, raw(x, z));
            for (_, [left, near, right, far]) in plan.floodplain().sites() {
                for row in 0..=5 {
                    for col in 0..=5 {
                        let x = left + (right - left) * col as f32 / 5.0;
                        let z = near + (far - near) * row as f32 / 5.0;
                        let camera = Camera { x, z: z - 5.0, altitude: developed(x, z).elevation + 15.0,
                            heading: 0.23, pitch: 0.56, bank: 0.0 };
                        let scenery = DesertProps::gather_with_habitats(camera, (60, 60), seed, true, raw, developed);
                        for prop in scenery.props.into_iter().flatten() {
                            if let PropKind::Habitat(form @ Feature::Freshwater(_)) = prop.kind {
                                let [x, _, z] = prop.origin;
                                assert!(matches!(developed(x, z).surface,
                                    Surface::RiverSilt | Surface::ReedMarsh | Surface::WetMeadow));
                                let pad = prop.size * 1.92;
                                for [dx, dz] in [[pad, 0.0], [-pad, 0.0], [0.0, pad], [0.0, -pad]] {
                                    assert_ne!(developed(x + dx, z + dz).surface, Surface::RiverWater);
                                }
                                forms.insert(form);
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(forms.len(), 25, "Seeded freshwater models: {forms:?}");

        const SIDE: usize = 60;
        let view = Camera { x: -3.5, z: -7.0, altitude: 3.8, heading: 0.464, pitch: 0.30, bank: 0.0 };
        let sky = Sky::from_time(41, 0, 0.48);
        let mut silhouettes = HashSet::new();
        for form in forms {
            for variant in 0..=u8::MAX {
                let mut grounded = false;
                form.draw(variant, |points, _| {
                    for [x, y, z] in points {
                        assert!(x.abs() <= HALF_SPAN && z.abs() <= HALF_SPAN, "{form:?}: {x}, {z}");
                        assert!((0.0..=HEIGHT).contains(&y), "{form:?}: {y}");
                        grounded |= y == 0.0;
                    }
                });
                assert!(grounded, "{form:?}");
            }
            let mut pixels = [0; SIDE * SIDE];
            let mut depth = [u16::MAX; SIDE * SIDE];
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), view, &sky).unwrap();
            form.draw(0, |points, _| mesh.triangle(points, [255; 3]));
            let silhouette = pixels.map(|color| color != 0);
            assert!(silhouette.contains(&true), "{form:?}");
            assert!(silhouettes.insert(silhouette), "{form:?} duplicates another silhouette");
        }
    }

    #[test]
    fn coastal_features_spawn_on_natural_shores_and_have_distinct_meshes() {
        use crate::world::World;
        use std::collections::HashSet;

        let mut forms = HashSet::new();
        for seed in [1, 11, 19, 23, 41, 49, 79, 98, 112, 119, 29, 83, 14692] {
            let world = World::new(seed);
            let sample = |x, z| world.sample(x, z);
            for row in -35..=35 {
                for col in -35..=35 {
                    let (x, z) = (col as f32 * 29.0, row as f32 * 29.0);
                    let ground = sample(x, z);
                    if ground.surface != Surface::Sand || ground.elevation < 0.42 {
                        continue;
                    }
                    let camera = Camera {
                        x, z: z - 5.0, altitude: 13.0, heading: 0.13, pitch: 0.54, bank: 0.0,
                    };
                    let scenery = DesertProps::gather_with_habitats(camera, (60, 60), seed, true, sample, sample);
                    for prop in scenery.props.into_iter().flatten() {
                        if let PropKind::Habitat(form @ Feature::Shore(_)) = prop.kind {
                            let [x, _, z] = prop.origin;
                            assert_eq!(sample(x, z).surface, Surface::Sand);
                            forms.insert(form);
                        }
                    }
                }
            }
            if forms.len() == 25 { break; }
        }
        assert_eq!(forms.len(), 25, "Natural coast catalog: {forms:?}");

        const SIDE: usize = 60;
        let view = Camera { x: -3.5, z: -7.0, altitude: 3.8, heading: 0.464, pitch: 0.30, bank: 0.0 };
        let sky = Sky::from_time(41, 0, 0.48);
        let mut silhouettes = HashSet::new();
        for form in forms {
            for variant in 0..=u8::MAX {
                let mut grounded = false;
                form.draw(variant, |points, _| {
                    for [x, y, z] in points {
                        assert!(x.abs() <= HALF_SPAN && z.abs() <= HALF_SPAN, "{form:?}: {x}, {z}");
                        assert!((0.0..=HEIGHT).contains(&y), "{form:?}: {y}");
                        grounded |= y == 0.0;
                    }
                });
                assert!(grounded, "{form:?}");
            }
            let mut pixels = [0; SIDE * SIDE];
            let mut depth = [u16::MAX; SIDE * SIDE];
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), view, &sky).unwrap();
            form.draw(0, |points, _| mesh.triangle(points, [255; 3]));
            let silhouette = pixels.map(|color| color != 0);
            assert!(silhouette.contains(&true), "{form:?}");
            assert!(silhouettes.insert(silhouette), "{form:?} duplicates another silhouette");
        }
    }

    #[test]
    fn mountain_features_spawn_in_seeded_terrain_and_have_distinct_silhouettes() {
        use crate::world::World;
        use std::collections::HashSet;

        let mut forms = HashSet::new();
        let survey = |seed, center: [f32; 2], extent: i32, spacing: f32, forms: &mut HashSet<Feature>| {
            let world = World::new(seed);
            let fan = (seed == 10).then(|| crate::scree_fans::ScreeFan::generate(&world, seed, -6, -7)).flatten();
            let natural = |x, z| world.sample(x, z);
            let developed = |x, z| {
                let ground = natural(x, z);
                fan.as_ref().map_or(ground, |fan| fan.overlay(x, z, ground))
            };
            for row in -extent..=extent {
                for col in -extent..=extent {
                    let (x, z) = (center[0] + col as f32 * spacing, center[1] + row as f32 * spacing);
                    let ground = natural(x, z);
                    if ground.elevation < 49.0 { continue; }
                    let camera = Camera { x, z: z - 8.0, altitude: ground.elevation + 15.0,
                        heading: 0.17, pitch: 0.56, bank: 0.0 };
                    let scene = DesertProps::gather_with_habitats(camera, (60, 60), seed, true, natural, developed);
                    for prop in scene.props.into_iter().flatten() {
                        if let PropKind::Habitat(form @ Feature::Mountain(_)) = prop.kind {
                            let [x, _, z] = prop.origin;
                            assert!(developed(x, z).elevation >= 48.0);
                            forms.insert(form);
                        }
                    }
                    if forms.len() == 25 { return; }
                }
            }
        };
        survey(10, [-2100.0, -2474.0], 8, 13.0, &mut forms);
        for seed in [1, 11, 19, 23, 29, 41, 47, 49, 67, 79, 83, 98, 112, 119, 14692] {
            survey(seed, [0.0; 2], 28, 57.0, &mut forms);
            if forms.len() == 25 { break; }
        }
        assert_eq!(forms.len(), 25, "Seeded mountain catalog: {forms:?}");

        const SIDE: usize = 60;
        let view = Camera { x: -3.5, z: -7.0, altitude: 3.8, heading: 0.464, pitch: 0.30, bank: 0.0 };
        let sky = Sky::from_time(41, 0, 0.48);
        let mut silhouettes = HashSet::new();
        for form in forms {
            for variant in 0..=u8::MAX {
                let mut grounded = false;
                form.draw(variant, |points, _| {
                    for [x, y, z] in points {
                        assert!(x.abs() <= HALF_SPAN && z.abs() <= HALF_SPAN, "{form:?}: {x}, {z}");
                        assert!((0.0..=HEIGHT).contains(&y), "{form:?}: {y}");
                        grounded |= y == 0.0;
                    }
                });
                assert!(grounded, "{form:?}");
            }
            let mut pixels = [0; SIDE * SIDE];
            let mut depth = [u16::MAX; SIDE * SIDE];
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), view, &sky).unwrap();
            form.draw(0, |points, _| mesh.triangle(points, [255; 3]));
            let silhouette = pixels.map(|color| color != 0);
            assert!(silhouette.contains(&true), "{form:?}");
            assert!(silhouettes.insert(silhouette), "{form:?} duplicates another silhouette");
        }
    }

    #[test]
    fn farm_details_cover_all_models_on_real_crop_margins() {
        use crate::ground_details::GroundDetails;
        use crate::world::World;
        use std::collections::HashSet;

        let mut forms = HashSet::new();
        for seed in [1, 11, 19, 23, 29, 41, 47, 49, 67, 79, 83, 98, 112, 119, 14692] {
            let world = World::new(seed);
            let mut details = GroundDetails::new();
            for center in [[0.0, 0.0], [-680.0, 0.0], [680.0, -480.0]] {
                details.reset(&world, seed, center[0], center[1]);
                let raw = |x, z| world.sample(x, z);
                let ground = |x, z| {
                    let sample = raw(x, z);
                    Sample { surface: details.visible_material(x, z, sample), ..sample }
                };
                for row in -16..=16 {
                    for col in -16..=16 {
                        let [x, z] = [center[0] + col as f32 * 28.0, center[1] + row as f32 * 28.0];
                        let base = ground(x, z);
                        if !matches!(base.surface, Surface::Grass | Surface::FieldGold
                            | Surface::FieldGreen | Surface::FieldSoil) { continue; }
                        let camera = Camera {
                            x, z: z - 8.0, altitude: base.elevation + 13.0,
                            heading: 0.12, pitch: 0.54, bank: 0.0,
                        };
                        let props = DesertProps::gather_with_habitats(camera, (60, 60), seed, true, raw, ground);
                        for prop in props.props.into_iter().flatten() {
                            if let PropKind::Habitat(form @ Feature::Farm(_)) = prop.kind {
                                let [x, _, z] = prop.origin;
                                assert_eq!(ground(x, z).surface, Surface::Grass);
                                let (sin, cos) = sin_cos(f32::from(prop.variant & 15) * core::f32::consts::TAU / 16.0);
                                assert!(matches!(ground(x + sin * 8.2, z + cos * 8.2).surface,
                                    Surface::FieldGold | Surface::FieldGreen | Surface::FieldSoil));
                                forms.insert(form);
                            }
                        }
                    }
                }
                if forms.len() == 25 { break; }
            }
            if forms.len() == 25 { break; }
        }
        assert_eq!(forms.len(), 25, "Seeded farmland catalog: {forms:?}");

        const SIDE: usize = 60;
        let view = Camera { x: -3.5, z: -7.0, altitude: 3.8, heading: 0.464, pitch: 0.30, bank: 0.0 };
        let sky = Sky::from_time(41, 0, 0.48);
        let mut silhouettes = HashSet::new();
        for form in forms {
            for variant in 0..=u8::MAX {
                let mut grounded = false;
                form.draw(variant, |points, _| {
                    for [x, y, z] in points {
                        assert!(x.abs() <= HALF_SPAN && z.abs() <= HALF_SPAN, "{form:?}: {x}, {z}");
                        assert!((0.0..=HEIGHT).contains(&y), "{form:?}: {y}");
                        grounded |= y == 0.0;
                    }
                });
                assert!(grounded, "{form:?}");
            }
            let mut pixels = [0; SIDE * SIDE];
            let mut depth = [u16::MAX; SIDE * SIDE];
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), view, &sky).unwrap();
            form.draw(0, |points, _| mesh.triangle(points, [255; 3]));
            let silhouette = pixels.map(|color| color != 0);
            assert!(silhouette.contains(&true), "{form:?}");
            assert!(silhouettes.insert(silhouette), "{form:?} duplicates another silhouette");
        }
    }

    #[test]
    fn world_seeds_use_every_mesh_variant_of_each_kind() {
        let camera = Camera { x: 17.0, z: -13.0, altitude: 20.0, heading: 0.48, pitch: 0.50, bank: 0.0 };
        let ground = |_, _| Sample { surface: Surface::Desert, elevation: 4.0 };
        let mut seen = [[false; 256]; 3];
        for seed in 0..512 {
            let scene = DesertProps::gather(camera, (60, 60), seed, ground, ground);
            for prop in scene.props.into_iter().flatten() {
                let index = match prop.kind {
                    PropKind::Cactus => 0,
                    PropKind::Rock => 1,
                    PropKind::Bush => 2,
                    _ => continue,
                };
                seen[index][usize::from(prop.variant)] = true;
            }
        }
        assert!(seen.into_iter().flatten().all(|found| found));
    }

    #[test]
    fn desert_variants_have_distinct_rendered_silhouettes_and_fit_the_cull_bounds() {
        const SIDE: usize = 60;
        const ROWS: usize = (SIDE * SIDE).div_ceil(64);
        let cameras = [
            Camera { x: -3.5, z: -7.0, altitude: 3.8, heading: 0.464, pitch: 0.30, bank: 0.0 },
            Camera { x: 7.0, z: -3.5, altitude: 5.5, heading: -1.107, pitch: 0.48, bank: 0.0 },
        ];
        let sky = Sky::from_time(41, 0, 0.48);
        for kind in [PropKind::Cactus, PropKind::Rock, PropKind::Bush] {
            let mut forms = BTreeMap::new();
            let mut duplicates = std::vec::Vec::new();
            for variant in 0..=u8::MAX {
                let prop = Prop { origin: [0.0; 3], grade: [0.0; 2], size: 1.0, kind, variant, opacity: 1.0 };
                let mut silhouettes = [[0_u64; ROWS]; 2];
                for (view, camera) in cameras.into_iter().enumerate() {
                    let mut pixels = [0; SIDE * SIDE];
                    let mut depth = [u16::MAX; SIDE * SIDE];
                    let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
                    let mut count = 0;
                    let mut grounded = false;
                    prop.triangles(|points, _| {
                        for [x, y, z] in points {
                            assert!(x.abs() <= HALF_SPAN && z.abs() <= HALF_SPAN);
                            assert!((0.0..=HEIGHT).contains(&y));
                            grounded |= y == 0.0;
                        }
                        count += 1;
                        mesh.triangle(points, [255; 3]);
                    });
                    assert_eq!(count, if kind == PropKind::Cactus { 12 } else { 6 });
                    assert!(grounded);
                    for (i, color) in pixels.into_iter().enumerate() {
                        silhouettes[view][i / 64] |= u64::from(color != 0) << (i % 64);
                    }
                    assert!(silhouettes[view].iter().any(|row| *row != 0));
                }
                if let Some(previous) = forms.insert(silhouettes, variant) {
                    duplicates.push((previous, variant));
                }
            }
            assert!(duplicates.is_empty(), "{kind:?} has duplicate silhouettes: {duplicates:?}");
        }
    }
}

pub struct DesertProps {
    props: [Option<Prop>; 72],
    region: Option<ScreenRect>,
}

impl DesertProps {
    pub fn gather(
        camera: Camera, dimensions: (usize, usize), seed: u64,
        terrain: impl Fn(f32, f32) -> Sample, sample: impl Fn(f32, f32) -> Sample,
    ) -> Self {
        Self::gather_with_wetlands(camera, dimensions, seed, false, terrain, sample)
    }

    #[inline(never)]
    pub fn gather_with_wetlands(
        camera: Camera, dimensions: (usize, usize), seed: u64, wetlands: bool,
        terrain: impl Fn(f32, f32) -> Sample, sample: impl Fn(f32, f32) -> Sample,
    ) -> Self {
        Self::gather_details(camera, dimensions, seed, wetlands, false, terrain, sample)
    }

    pub fn gather_with_habitats(
        camera: Camera, dimensions: (usize, usize), seed: u64, wetlands: bool,
        terrain: impl Fn(f32, f32) -> Sample, sample: impl Fn(f32, f32) -> Sample,
    ) -> Self {
        Self::gather_details(camera, dimensions, seed, wetlands, true, terrain, sample)
    }

    #[inline(never)]
    fn gather_details(
        camera: Camera, dimensions: (usize, usize), seed: u64, wetlands: bool, habitats: bool,
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
                let raw = terrain(x, z);
                if camera.altitude - raw.elevation >= 72.0 { continue; }
                let surface = raw.surface;
                let sand = matches!(surface, Surface::Desert | Surface::DuneSand | Surface::Sand);
                let next_to_water = wetlands && matches!(surface, Surface::Grass | Surface::Forest
                    | Surface::WetMeadow | Surface::ReedMarsh | Surface::RiverSilt);
                let new_candidate = habitats && (key >> 40) & 7 < 3
                    && (next_to_water || matches!(surface, Surface::Grass | Surface::Forest | Surface::Rock
                        | Surface::Desert | Surface::DryWash | Surface::DuneSand | Surface::Sand
                        | Surface::AlpineMeadow | Surface::ScreeEdge | Surface::ScreeFine
                        | Surface::ScreeCoarse | Surface::GlacierMoraine | Surface::Snow));
                let farm_candidate = habitats && surface == Surface::Grass && key >> 45 & 3 <= 1
                    && forward > 0.0;
                if !sand && !next_to_water && !new_candidate && !farm_candidate { continue; }
                let ground = sample(x, z);
                let wet_plant = wetlands
                    && (matches!(ground.surface, Surface::WetMeadow | Surface::ReedMarsh)
                        || ground.surface == Surface::RiverSilt && ground.elevation >= 0.45 && key & 3 == 0);
                let mut size = 1.12 + ((key >> 16) & 7) as f32 * 0.12;
                let district = (u64::from((tx >> 2) as u32) << 32) | u64::from((tz >> 2) as u32);
                let district = crate::math::mix(seed ^ district ^ 0x6863_583d_6157_6cbc);
                let mut feature = if new_candidate && (!matches!(ground.surface,
                    Surface::WetMeadow | Surface::ReedMarsh | Surface::RiverSilt) || wetlands)
                {
                    Feature::on(ground, key, district)
                } else {
                    None
                };
                let mut variant = (key >> 19) as u8;
                // Farm features attach to nearby cultivated parcels. A wider
                // grass margin leaves space for their entire footprint.
                if farm_candidate && ground.surface == Surface::Grass {
                    let directions = [[0.0, 8.2], [8.2, 0.0], [0.0, -8.2], [-8.2, 0.0]];
                    for step in 0..4 {
                        let heading = ((key >> 47) as usize + step) & 3;
                        let [dx, dz] = directions[heading];
                        let neighbor = sample(x + dx, z + dz);
                        if (neighbor.elevation - ground.elevation).abs() > 1.35 { continue; }
                        if let Some(farm) = Feature::beside_crop(neighbor.surface, key, district) {
                            feature = Some(farm);
                            variant = (variant & !15) | (heading as u8 * 4);
                            break;
                        }
                    }
                }
                if !wet_plant && !matches!(ground.surface, Surface::Desert | Surface::DuneSand) && feature.is_none() {
                    continue;
                }
                if let Some(form) = feature {
                    size *= match form {
                        Feature::Shore(_) => 1.23,
                        Feature::Farm(_) => 1.45,
                        Feature::Mountain(_) => 1.24,
                        Feature::Freshwater(_) if ground.surface == Surface::RiverSilt => 0.82,
                        Feature::Freshwater(_) => 1.16,
                        Feature::Woodland(_) => 1.14,
                        _ => 1.08,
                    };
                }
                let opacity = ((REACH * REACH - distance) / 1500.0).clamp(0.0, 1.0)
                    * ((66.0 - camera.altitude + ground.elevation) / 18.0).clamp(0.0, 1.0)
                    * ((forward + 12.0) / 6.0).clamp(0.0, 1.0);
                if opacity < 0.02 { continue; }
                let mountain = matches!(feature, Some(Feature::Mountain(_)));
                let y = ground.elevation - if mountain { 0.24 } else { 0.14 };
                let variation = if mountain { 2.1 * size } else { 0.0 };
                let bounds = [[x - HALF_SPAN * size, y - variation, z - HALF_SPAN * size],
                              [x + HALF_SPAN * size, y + HEIGHT * size + variation, z + HALF_SPAN * size]];
                let Some(rect) = projection.bounds(bounds) else { continue; };
                let mut grade = [0.0; 2];
                if mountain {
                    // Fit the footprint to the slope, then reject discontinuities
                    // instead of limiting exposed mountains to horizontal pads.
                    let pad = size * 2.12;
                    let edges = [sample(x - pad, z), sample(x + pad, z),
                                 sample(x, z - pad), sample(x, z + pad)];
                    if !edges.iter().all(|&edge| Feature::on(edge, key, district).is_some()) {
                        continue;
                    }
                    let [west, east, north, south] = edges.map(|edge| edge.elevation - ground.elevation);
                    grade = [(east - west) / (2.0 * pad), (south - north) / (2.0 * pad)];
                    if grade[0] * grade[0] + grade[1] * grade[1] > 0.61 * 0.61
                        || (west + east).abs() > 1.00 || (north + south).abs() > 1.00
                    {
                        continue;
                    }
                } else if feature.is_some() {
                    let farm = matches!(feature, Some(Feature::Farm(_)));
                    let freshwater = matches!(feature, Some(Feature::Freshwater(_)));
                    let pad = size * if freshwater { 1.92 } else if farm { 1.83 } else { 1.55 };
                    let clear_edge = |[dx, dz]: [f32; 2]| {
                        let edge = sample(x + dx, z + dz);
                        let grade = (edge.elevation - ground.elevation).abs();
                        if farm {
                            grade <= 0.74 && edge.surface == Surface::Grass
                        } else {
                            grade <= 0.95 && Feature::on(edge, key, district).is_some()
                        }
                    };
                    let clear = [[-pad, 0.0], [pad, 0.0], [0.0, -pad], [0.0, pad]]
                        .into_iter().all(clear_edge);
                    let corner = pad * 0.7;
                    let clear = clear && (!(farm || freshwater) || [[-corner, -corner], [-corner, corner],
                        [corner, -corner], [corner, corner]].into_iter().all(clear_edge));
                    if !clear {
                        continue;
                    }
                }
                result.region = Some(result.region.map_or(rect, |old| old.union(rect)));
                let roll = (key >> 12) & 15;
                let kind = if let Some(form) = feature {
                    PropKind::Habitat(form)
                } else if wet_plant {
                    PropKind::WetlandPlant
                } else if ground.surface == Surface::DuneSand {
                    if roll < 4 { PropKind::Rock } else { PropKind::Bush }
                } else {
                    let cactus = [1, 2, 5, 6][(district & 3) as usize];
                    let rock = [12, 9, 11, 8][(district & 3) as usize];
                    if roll < cactus {
                        PropKind::Cactus
                    } else if roll < rock {
                        PropKind::Rock
                    } else {
                        PropKind::Bush
                    }
                };
                result.props[count] = Some(Prop {
                    origin: [x, y, z], grade, size, kind,
                    variant: if wet_plant && feature.is_none() {
                        let family = match ground.surface {
                            Surface::ReedMarsh => (key >> 29) & 1,
                            Surface::RiverSilt => 3,
                            _ => 2 + ((key >> 30) & 1),
                        };
                        ((family << 6) | ((key >> 19) & 63)) as u8
                    } else {
                        variant
                    },
                    opacity,
                });
                count += 1;
                if count == result.props.len() { return result; }
            }
        }
        result
    }

    pub fn region(&self) -> Option<ScreenRect> { self.region }

    pub fn draw(&self, mesh: &mut WorldMesh<'_>) {
        for prop in self.props.iter().flatten() {
            prop.triangles(|points, color| {
                mesh.faded_triangle(points.map(|[x, y, z]| {
                    let y = y + x * prop.grade[0] + z * prop.grade[1];
                    [prop.origin[0] + x * prop.size, prop.origin[1] + y * prop.size,
                     prop.origin[2] + z * prop.size]
                }), color, prop.opacity);
            });
        }
    }
}
