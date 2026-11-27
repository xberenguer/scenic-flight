//! Los Angeles City Hall, from the mapped footprint and the architects’ west elevation.
//! Plate 6, The Architectural Forum, July 1928: https://www.usmodernist.org/AF/AF-1928-07-1.PDF
//! HABS CA-2159: https://tile.loc.gov/storage-services/master/pnp/habshaer/ca/ca1200/ca1261/data/ca1261data.pdf
//! The City Clerk records the full height as 454 ft: https://cityclerk.lacity.org/onlinedocs/2023/23-0838_misc_08-11-23.pdf

#[path = "city_hall_geometry.rs"]
mod geometry;

use crate::world_mesh::WorldMesh;
use geometry::*;

type Point = [f32; 2];
type Edge = [Point; 2];
type Roof = [Point; 3];

pub(crate) const CENTER: Point = geometry::CENTER;
pub(crate) const HALF_EXTENT: Point = [72.0, 76.0];
pub(crate) const HEIGHT: f32 = 454.0 * 0.3048;
const BASE_ROOF_HEIGHT: f32 = 20.0;
const WING_EAVE: f32 = 42.5;
const WING_RIDGE: f32 = 46.5;
const SHAFT_TOP: f32 = 98.5;
const SHOULDER_TOP: f32 = 110.0;
const GALLERY_TOP: f32 = 121.6;
const PYRAMID_BASE: f32 = HEIGHT - 11.0;
const PYRAMID_APEX: f32 = 452.0 * 0.3048;
const STEPS: [f32; 8] = [1.0, 0.89, 0.78, 0.67, 0.56, 0.46, 0.36, 0.28];
const STEP_HEIGHT: f32 = (PYRAMID_APEX - PYRAMID_BASE) / STEPS.len() as f32;
const BEACON_HALF: f32 = 0.58;

const GRANITE: [u8; 3] = [157, 149, 142];
const GRANITE_ROOF: [u8; 3] = [176, 168, 157];
const TERRACOTTA: [u8; 3] = [201, 191, 175];
const TERRACE: [u8; 3] = [216, 205, 187];
const TILES: [u8; 3] = [140, 80, 63];
const CROWN_COLOR: [u8; 3] = [185, 173, 153];
const WINDOWS: [u8; 3] = [101, 109, 112];

fn within([east, north]: Point, edges: &[Edge]) -> bool {
    let mut inside = false;
    for &[[a_x, a_z], [b_x, b_z]] in edges {
        if (a_z > north) != (b_z > north) && east < a_x + (b_x - a_x) * (north - a_z) / (b_z - a_z)
        {
            inside = !inside;
        }
    }
    inside
}

fn local(x: f32, z: f32) -> Point {
    [x - CENTER[0], z - CENTER[1]]
}

fn in_bounds([east, north]: Point) -> bool {
    east.abs() <= HALF_EXTENT[0] && north.abs() <= HALF_EXTENT[1]
}

pub(crate) fn contains(x: f32, z: f32) -> bool {
    let point = local(x, z);
    // City Hall's five light courts are open where a higher wing does not cross them.
    in_bounds(point)
        && (within(point, &BASE_EDGES)
            || within(point, &WINGS_EDGES)
            || within(point, &SHAFT_EDGES))
}

const CROWN_EDGES: [Edge; 4] = [
    [CROWN[0], CROWN[1]],
    [CROWN[1], CROWN[2]],
    [CROWN[2], CROWN[3]],
    [CROWN[3], CROWN[0]],
];

pub(crate) fn clearance(x: f32, z: f32) -> f32 {
    let point = local(x, z);
    if !in_bounds(point) {
        return 0.0;
    }
    if point.iter().all(|offset| offset.abs() <= BEACON_HALF) {
        return HEIGHT;
    }
    if within(point, &CROWN_EDGES) {
        for (level, &scale) in STEPS.iter().enumerate().rev() {
            if within(point.map(|offset| offset / scale), &CROWN_EDGES) {
                return PYRAMID_BASE + (level + 1) as f32 * STEP_HEIGHT;
            }
        }
    }
    for (height, edges) in [
        (GALLERY_TOP, &GALLERY_EDGES[..]),
        (SHOULDER_TOP, &SHOULDER_EDGES[..]),
        (SHAFT_TOP, &SHAFT_EDGES[..]),
        (WING_RIDGE, &WINGS_EDGES[..]),
        (BASE_ROOF_HEIGHT, &BASE_EDGES[..]),
    ] {
        if within(point, edges) {
            return height;
        }
    }
    0.0
}

fn world_point(base: f32, [east, north]: Point, height: f32) -> [f32; 3] {
    [CENTER[0] + east, base + height, CENTER[1] + north]
}

fn face_tint(color: [u8; 3], [a, b]: Edge) -> [u8; 3] {
    let east = b[0] - a[0];
    let north = b[1] - a[1];
    let light = (0.9 * north + 0.5 * east) / (east.abs() + north.abs()).max(0.01);
    let shade = (14.0 * light) as i16;
    color.map(|value| (i16::from(value) + shade).clamp(0, 255) as u8)
}

struct Tier {
    edges: &'static [Edge],
    roof: &'static [Roof],
    bottom: f32,
    top: f32,
    wall_color: [u8; 3],
    roof_color: [u8; 3],
}

impl Tier {
    fn draw(&self, mesh: &mut WorldMesh<'_>, base: f32) {
        for &roof in self.roof {
            mesh.triangle(
                roof.map(|p| world_point(base, p, self.top)),
                self.roof_color,
            );
        }
        for &edge in self.edges {
            wall(
                mesh,
                base,
                edge,
                self.bottom,
                self.top,
                face_tint(self.wall_color, edge),
            );
        }
    }
}

const TIERS: [Tier; 5] = [
    Tier {
        edges: &BASE_EDGES,
        roof: &BASE_ROOF,
        bottom: -5.0,
        top: BASE_ROOF_HEIGHT,
        wall_color: GRANITE,
        roof_color: GRANITE_ROOF,
    },
    Tier {
        edges: &WINGS_EDGES,
        roof: &[],
        bottom: BASE_ROOF_HEIGHT,
        top: WING_EAVE,
        wall_color: TERRACOTTA,
        roof_color: TILES,
    },
    Tier {
        edges: &SHAFT_EDGES,
        roof: &SHAFT_ROOF,
        bottom: BASE_ROOF_HEIGHT,
        top: SHAFT_TOP,
        wall_color: TERRACOTTA,
        roof_color: TERRACE,
    },
    Tier {
        edges: &SHOULDER_EDGES,
        roof: &SHOULDER_ROOF,
        bottom: SHAFT_TOP,
        top: SHOULDER_TOP,
        wall_color: TERRACOTTA,
        roof_color: TERRACE,
    },
    Tier {
        edges: &GALLERY_EDGES,
        roof: &GALLERY_ROOF,
        bottom: SHOULDER_TOP,
        top: GALLERY_TOP,
        wall_color: TERRACOTTA,
        roof_color: TERRACE,
    },
];

fn wall(mesh: &mut WorldMesh<'_>, base: f32, [a, b]: Edge, low: f32, high: f32, color: [u8; 3]) {
    mesh.faded_quad(
        [
            world_point(base, a, low),
            world_point(base, b, low),
            world_point(base, b, high),
            world_point(base, a, high),
        ],
        color,
        1.0,
    );
}

fn inset_edge([a, b]: Edge, start: f32, end: f32) -> Edge {
    let [east, north] = [b[0] - a[0], b[1] - a[1]];
    let offset = 0.38 / (east.abs() + north.abs()).max(0.01);
    [start, end].map(|t| {
        [
            a[0] + t * east - north * offset,
            a[1] + t * north + east * offset,
        ]
    })
}

fn draw_windows(
    mesh: &mut WorldMesh<'_>,
    base: f32,
    edges: &[Edge],
    first_floor: f32,
    floors: usize,
    gap: f32,
    height: f32,
    gallery: bool,
) {
    for &edge @ [a, b] in edges {
        let length = (b[0] - a[0]).abs() + (b[1] - a[1]).abs();
        if length < 9.0 {
            continue;
        }
        let bays = if gallery {
            5
        } else {
            (length / 5.5 + 0.5).clamp(2.0, 6.0) as usize
        };
        for bay in 0..bays {
            let width = if gallery { 0.63 } else { 0.43 };
            let left = (bay as f32 + (1.0 - width) * 0.5) / bays as f32;
            let window = inset_edge(edge, left, left + width / bays as f32);
            for floor in 0..floors {
                let bottom = first_floor + gap * floor as f32;
                wall(
                    mesh,
                    base,
                    window,
                    bottom,
                    bottom + height,
                    face_tint(WINDOWS, edge),
                );
            }
        }
    }
}

fn draw_arcades(mesh: &mut WorldMesh<'_>, base: f32) {
    // The Spring and Main Street fronts share the long-axis bays. The shorter
    // north/south facades and the inner light courts use square windows.
    for &edge @ [a, b] in &BASE_EDGES {
        let east = b[0] - a[0];
        let north = b[1] - a[1];
        let along = (east * 0.620 + north * 0.784).abs();
        let width = ((a[0] + b[0]) * 0.784 - (a[1] + b[1]) * 0.620).abs() * 0.5;
        if along < 9.0 || width < 34.0 {
            continue;
        }
        let bays = (along / 5.2 + 0.25).clamp(2.0, 7.0) as usize;
        let glass = face_tint(WINDOWS, edge);
        for bay in 0..bays {
            let left = (bay as f32 + 0.16) / bays as f32;
            let opening = inset_edge(edge, left, left + 0.68 / bays as f32);
            wall(mesh, base, opening, 5.2, 10.1, glass);
            let [a, b] = opening;
            let at = |t: f32, y| {
                world_point(
                    base,
                    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t],
                    y,
                )
            };
            mesh.faded_quad(
                [at(0.0, 10.1), at(1.0, 10.1), at(0.79, 11.7), at(0.21, 11.7)],
                glass,
                1.0,
            );
            mesh.triangle([at(0.21, 11.7), at(0.79, 11.7), at(0.5, 12.2)], glass);
        }
    }
}

fn draw_crown(mesh: &mut WorldMesh<'_>, base: f32) {
    for edge in CROWN_EDGES {
        wall(
            mesh,
            base,
            edge,
            GALLERY_TOP,
            PYRAMID_BASE,
            face_tint(CROWN_COLOR, edge),
        );
    }
    for (level, &scale) in STEPS.iter().enumerate() {
        let low = PYRAMID_BASE + level as f32 * STEP_HEIGHT;
        let high = low + STEP_HEIGHT;
        let inner = STEPS.get(level + 1).copied().unwrap_or(0.0);
        for edge in CROWN_EDGES {
            let [a, b] = edge.map(|p| p.map(|coordinate| coordinate * scale));
            let [c, d] = edge.map(|p| p.map(|coordinate| coordinate * inner));
            let color = face_tint(CROWN_COLOR, edge);
            wall(mesh, base, [a, b], low, high, color);
            if inner > 0.0 {
                mesh.faded_quad(
                    [a, b, d, c].map(|point| world_point(base, point, high)),
                    TERRACE,
                    1.0,
                );
            } else {
                mesh.triangle(
                    [a, b, [0.0, 0.0]].map(|point| world_point(base, point, high)),
                    TERRACE,
                );
            }
        }
    }
    mesh.wire(
        world_point(base, [0.0, 0.0], PYRAMID_APEX),
        world_point(base, [0.0, 0.0], HEIGHT),
        BEACON_HALF * 2.0,
        [196, 163, 112],
    );
}

pub(crate) fn draw(mesh: &mut WorldMesh<'_>, base: f32) {
    for tier in &TIERS {
        tier.draw(mesh, base);
    }
    for triangle in TILE_ROOFS {
        mesh.lit_triangle(
            triangle.map(|[east, height, north]| world_point(base, [east, north], height)),
            TILES,
        );
    }
    draw_arcades(mesh, base);
    draw_windows(mesh, base, &WINGS_EDGES, 26.0, 3, 5.3, 2.5, false);
    draw_windows(mesh, base, &SHAFT_EDGES, 50.0, 7, 6.4, 3.0, false);
    draw_windows(mesh, base, &GALLERY_EDGES, 111.5, 1, 0.0, 8.2, true);
    draw_crown(mesh, base);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapped_wings_courts_and_tower_keep_their_separate_levels() {
        assert_eq!(clearance(CENTER[0], CENTER[1]), HEIGHT);
        assert_eq!(clearance(640.500, 401.346), BASE_ROOF_HEIGHT);
        assert_eq!(clearance(689.454, 400.585), WING_RIDGE);
        assert_eq!(clearance(679.474, 411.830), SHAFT_TOP);
        assert_eq!(clearance(681.769, 391.547), SHOULDER_TOP);
        assert_eq!(clearance(683.841, 400.827), GALLERY_TOP);
        assert!(!contains(683.172, 438.685));
        assert_eq!(clearance(683.172, 438.685), 0.0);
        assert!(!contains(735.0, 460.0));
    }
}
