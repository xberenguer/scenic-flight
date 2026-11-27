//! Wilshire Grand Center’s mapped podium and tower, with its glazed sail and east spire.
//! Plan geometry comes from OpenStreetMap. The published owner heights are 934 ft at the
//! crown and 1,100 ft at the spire. The California Geological Survey section documents the
//! 97 ft glazed sail above the main roof and its east-mounted spire.
//! https://wilshiregrand.buildingengines.com/geofire/BDPW?conid=1202760488&id_client_site_rel=0&id_site=758725065
//! https://conservation.ca.gov/cgs/documents/program-smi/seminar/SMIP18_P8_Paper%20by%20Huang.pdf

#[path = "wilshire_grand_geometry.rs"]
mod geometry;

use crate::world_mesh::WorldMesh;
use geometry::*;

pub(crate) const CENTER: [f32; 2] = [-961.56, 28.96];
pub(crate) const HEIGHT: f32 = 335.28;
const SAIL_PEAK: f32 = 284.6832;
const MAIN_ROOF: f32 = SAIL_PEAK - 97.0 * 0.3048;
const PODIUM_ROOF: f32 = 22.4;
// OSM way 901739648 maps the spire independently of the podium and facade.
const SPIRE: [f32; 2] = [66.49, -20.36];

pub(crate) fn contains(x: f32, z: f32) -> bool {
    let point = [x - CENTER[0], z - CENTER[1]];
    point[0].abs() <= 75.0 && point[1].abs() <= 62.0 && inside(point, &OUTLINE_EDGES)
}

pub(crate) fn clearance(x: f32, z: f32) -> f32 {
    if !contains(x, z) {
        return 0.0;
    }
    let point = [x - CENTER[0], z - CENTER[1]];
    if (point[0] - SPIRE[0]).abs() < 4.0 && (point[1] - SPIRE[1]).abs() < 4.0 {
        HEIGHT
    } else if inside(point, &TOWER_EDGES) {
        SAIL_PEAK
    } else {
        PODIUM_ROOF
    }
}

fn inside([east, north]: [f32; 2], edges: &[[[f32; 2]; 2]]) -> bool {
    let mut inside = false;
    for &[[a_x, a_z], [b_x, b_z]] in edges {
        if (a_z > north) != (b_z > north)
            && east < a_x + (b_x - a_x) * (north - a_z) / (b_z - a_z)
        {
            inside = !inside;
        }
    }
    inside
}

pub(crate) fn draw(mesh: &mut WorldMesh<'_>, base: f32) {
    let point = |[east, north]: [f32; 2], height| [
        CENTER[0] + east, base + height, CENTER[1] + north,
    ];
    for (walls, roof, top, glass, cap) in [
        (&PODIUM_EDGES[..], &geometry::PODIUM_ROOF[..], PODIUM_ROOF, [118, 128, 129], [150, 155, 153]),
        (&TOWER_EDGES[..], &TOWER_ROOF[..], MAIN_ROOF, [91, 127, 153], [134, 156, 172]),
    ] {
        for &triangle in roof {
            mesh.triangle(triangle.map(|position| point(position, top)), cap);
        }
        for &[a, b] in walls {
            let width = (b[0] - a[0]).abs();
            let across = (b[1] - a[1]).abs();
            let shade = (width * 34.0 / (width + across).max(0.01)) as u8;
            let tint = glass.map(|channel| channel + shade);
            mesh.faded_quad(
                [point(a, -5.0), point(b, -5.0), point(b, top), point(a, top)], tint, 1.0,
            );
            if top == MAIN_ROOF {
                let outward = [-(b[1] - a[1]) * 0.12 / (width + across).max(0.01),
                    (b[0] - a[0]) * 0.12 / (width + across).max(0.01)];
                let a = [a[0] + outward[0], a[1] + outward[1]];
                let b = [b[0] + outward[0], b[1] + outward[1]];
                let mullion = |fraction: f32| [
                    a[0] + (b[0] - a[0]) * fraction, a[1] + (b[1] - a[1]) * fraction,
                ];
                let divisions = ((width + across) as usize).div_ceil(5).max(1);
                for division in 0..divisions {
                    let fraction = division as f32 / divisions as f32;
                    let start = mullion(fraction);
                    let end = mullion((fraction + 0.085 / divisions as f32).min(1.0));
                    mesh.faded_quad(
                        [point(start, -2.0), point(end, -2.0), point(end, top), point(start, top)],
                        [128, 157, 174], 1.0,
                    );
                }
            }
        }
    }

    // The roof sail is roughly 97 ft high. Its profile follows the designer’s section.
    let profile: [([f32; 2], f32, f32); 6] = [
        ([25.0, 3.0], 256.0, 5.8),
        ([33.5, -2.2], 261.5, 7.4),
        ([43.8, -8.8], 273.0, 7.2),
        ([53.3, -15.6], 282.0, 6.7),
        ([59.0, -19.1], SAIL_PEAK, 5.5),
        ([64.1, -22.1], 279.0, 3.5),
    ];
    let cross = |(center, _, breadth): ([f32; 2], f32, f32), side: f32| {
        [center[0] + 0.564 * breadth * side, center[1] + 0.826 * breadth * side]
    };
    for pair in profile.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        for side in [-1.0, 1.0] {
            let [from, to] = [cross(a, side), cross(b, side)];
            mesh.faded_quad(
                [point(from, MAIN_ROOF), point(to, MAIN_ROOF), point(to, b.1), point(from, a.1)],
                [112, 159, 187], 1.0,
            );
            mesh.faded_segment([point(from, a.1), point(to, b.1)], 0.62, [171, 205, 220], 1.0);
        }
        mesh.faded_quad(
            [point(cross(a, -1.0), a.1), point(cross(b, -1.0), b.1),
             point(cross(b, 1.0), b.1), point(cross(a, 1.0), a.1)],
            [166, 194, 206], 1.0,
        );
    }
    for cap in [profile[0], profile[profile.len() - 1]] {
        let [left, right] = [cross(cap, -1.0), cross(cap, 1.0)];
        mesh.faded_quad(
            [point(left, MAIN_ROOF), point(right, MAIN_ROOF), point(right, cap.1), point(left, cap.1)],
            [135, 170, 192], 1.0,
        );
    }

    // The tubular spire continues down into the building so only its exposed length shows.
    let shaft = [[HEIGHT - 272.0 * 0.3048, 1.98], [SAIL_PEAK, 1.55], [321.0, 1.10], [HEIGHT, 0.82]];
    for pair in shaft.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        mesh.faded_segment([point(SPIRE, a[0]), point(SPIRE, b[0])],
            (a[1] + b[1]) * 0.5, [193, 208, 215], 1.0);
    }
    if mesh.daylight() < 0.42 {
        mesh.luminous_segment([point(SPIRE, HEIGHT - 3.4), point(SPIRE, HEIGHT)],
            0.85, [222, 233, 236]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapped_podium_tower_and_spire_keep_distinct_heights() {
        assert_eq!(clearance(-978.0, 40.0), PODIUM_ROOF);
        assert_eq!(clearance(-919.0, 21.0), SAIL_PEAK);
        assert_eq!(clearance(-895.07, 8.6), HEIGHT);
        assert_eq!(clearance(-1_046.0, 40.0), 0.0);
    }
}
