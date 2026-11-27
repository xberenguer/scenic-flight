//! Walt Disney Concert Hall’s stainless sails, corner lobby and garden terrace.
//! The surveyed plan uses OpenStreetMap relation 6333150. The 36.7 m maximum
//! comes from that relation. The curved shell profiles are visual approximations
//! of Gehry's project drawings and photographs.
//! https://gehry.getty.edu/accessibility
//! https://www.laphil.com/posts/walt-disney-concert-hall-in-numbers
//! https://www.laphil.com/posts/a-rose-for-lilly
//! https://www.nagata-i.com/wp-content/uploads/2019/06/WaltDisney_NAGATA.pdf

#[path = "disney_hall_geometry.rs"]
mod geometry;

use crate::world_mesh::WorldMesh;

pub(crate) const CENTER: [f32; 2] = [12.39, 597.86];
pub(crate) const HEIGHT: f32 = 36.7;

const MAIN_AXIS: [f32; 2] = [0.6279, 0.7783];
const TERRACE: f32 = 4.8;
const AMPHITHEATRE_PEAK: f32 = 9.5;
const FOUNTAIN_RADIUS: f32 = 11.0 * 0.3048;
const FOUNTAIN_HEIGHT: f32 = 7.0 * 0.3048;
const STEEL: [u8; 3] = [191, 199, 202];
const SHADED_STEEL: [u8; 3] = [148, 160, 168];
const GLASS: [u8; 3] = [77, 111, 126];
const LIMESTONE: [u8; 3] = [164, 155, 140];

fn inside([east, north]: [f32; 2], ring: &[[f32; 2]]) -> bool {
    let mut hit = false;
    let mut a = ring[ring.len() - 1];
    for &b in ring {
        if (a[1] > north) != (b[1] > north)
            && east < a[0] + (b[0] - a[0]) * (north - a[1]) / (b[1] - a[1])
        {
            hit = !hit;
        }
        a = b;
    }
    hit
}

pub(crate) fn contains(x: f32, z: f32) -> bool {
    let offset = [x - CENTER[0], z - CENTER[1]];
    offset[0].abs() < 77.0 && offset[1].abs() < 80.0
        && inside(offset, &geometry::OUTER) && !inside(offset, &geometry::COURTYARD)
}

pub(crate) fn replaces_raster(x: f32, z: f32, source_height: f32) -> bool {
    // The separate space in the inner ring and the surrounding blocks retain
    // their raster payload. The building relation is rasterized at 37 metres.
    source_height == 37.0 && contains(x, z)
}

fn to_site(x: f32, z: f32) -> [f32; 2] {
    let [along_east, along_north] = MAIN_AXIS;
    let [east, north] = [x - CENTER[0], z - CENTER[1]];
    [east * along_north - north * along_east, east * along_east + north * along_north]
}

fn site_point([cross, height, along]: [f32; 3], base: f32) -> [f32; 3] {
    let [along_east, along_north] = MAIN_AXIS;
    [
        CENTER[0] + cross * along_north + along * along_east,
        base + height,
        CENTER[1] - cross * along_east + along * along_north,
    ]
}

// A loft consists of measured-scale sections across one building volume.
// In each section: along position, west base, west shoulder, crest offset,
// crest height, east shoulder, east base. The shoulders describe curved
// steel skin and give each of the asymmetric shells its own silhouette.
#[derive(Clone, Copy)]
struct Section {
    along: f32,
    west: f32,
    west_shoulder: f32,
    ridge: f32,
    roof: f32,
    east_shoulder: f32,
    east: f32,
}

const fn section(along: f32, root: [f32; 2], shoulder: [f32; 2], ridge: [f32; 2]) -> Section {
    Section {
        along, west: root[0], east: root[1], west_shoulder: shoulder[0],
        east_shoulder: shoulder[1], ridge: ridge[0], roof: ridge[1],
    }
}

// The main auditorium follows the long northeast–southwest axis. The two
// rims of its stainless-steel roof are higher than the seat bowl between them.
const AUDITORIUM: [Section; 7] = [
    section(-42.0, [-10.0, 16.0], [-8.0, 16.0], [6.0, 22.0]),
    section(-31.0, [-12.0, 25.0], [-11.0, 26.0], [5.0, 31.0]),
    section(-18.0, [-11.0, 28.0], [-8.0, 28.0], [8.0, 30.0]),
    section(-3.0, [-10.0, 29.0], [-7.0, 28.0], [9.0, 28.5]),
    section(14.0, [-8.0, 26.0], [-5.0, 27.0], [11.0, 31.0]),
    section(29.0, [-4.0, 25.0], [0.0, 25.0], [15.0, 35.0]),
    section(42.0, [0.0, 20.0], [3.0, 20.0], [13.0, 29.0]),
];

const GRAND_AVENUE_SAIL: [Section; 8] = [
    section(-23.0, [22.0, 40.0], [25.0, 39.0], [29.0, 15.0]),
    section(-14.0, [25.0, 46.0], [27.0, 46.5], [33.0, 21.0]),
    section(-3.0, [27.0, 49.0], [29.0, 49.3], [37.0, 24.0]),
    section(9.0, [25.0, 49.0], [26.0, 49.0], [38.0, 29.0]),
    section(22.0, [22.0, 46.0], [21.0, 47.0], [38.0, 32.5]),
    section(34.0, [19.0, 45.0], [17.0, 45.0], [33.0, HEIGHT]),
    section(42.0, [27.0, 41.0], [27.0, 42.7], [33.0, 35.4]),
    section(51.0, [39.0, 41.3], [38.7, 41.7], [40.0, 25.0]),
];

const FIRST_STREET_SAIL: [Section; 7] = [
    section(27.0, [-8.0, 10.0], [-9.0, 11.0], [-2.0, 16.0]),
    section(38.0, [-14.0, 9.0], [-17.0, 12.0], [-5.0, 23.0]),
    section(48.0, [-24.0, -9.3], [-26.0, -8.6], [-13.0, 27.5]),
    section(56.0, [-25.8, -8.0], [-26.0, -7.5], [-15.0, 31.0]),
    section(61.0, [-25.0, -12.5], [-25.1, -11.0], [-19.0, 29.0]),
    section(63.0, [-24.8, -17.0], [-25.0, -16.8], [-19.7, 25.0]),
    section(64.5, [-20.8, -17.9], [-21.1, -17.8], [-19.4, 20.0]),
];

const GARDEN_SAIL: [Section; 6] = [
    // Keep the lower Keck roof visible at the southwest end of the terrace.
    section(-26.0, [-5.0, 2.0], [-4.0, 2.0], [-1.0, 18.0]),
    section(-21.0, [-19.0, 0.0], [-19.8, -3.0], [-12.0, 23.5]),
    section(-9.0, [-18.0, 1.0], [-22.0, -2.0], [-15.4, 27.5]),
    section(2.0, [-16.0, 2.0], [-20.0, 1.0], [-13.0, 31.0]),
    section(11.0, [-13.0, 4.0], [-16.0, 3.0], [-10.0, 29.0]),
    section(19.0, [-11.0, 10.0], [-13.0, 9.0], [-3.0, 18.5]),
];

const SECOND_STREET_SAIL: [Section; 6] = [
    section(-45.0, [21.0, 34.0], [20.0, 34.3], [27.0, 14.0]),
    section(-39.0, [22.0, 45.0], [20.0, 45.0], [32.0, 18.5]),
    section(-30.0, [23.0, 44.0], [21.0, 46.0], [34.0, 23.0]),
    section(-21.0, [23.0, 43.0], [23.0, 44.0], [34.0, 25.5]),
    section(-13.0, [24.0, 43.0], [24.0, 43.5], [34.0, 22.0]),
    section(-5.0, [26.0, 41.0], [27.0, 42.0], [34.0, 17.0]),
];

const NORTH_FOLD: [Section; 6] = [
    section(50.0, [6.0, 11.5], [5.8, 12.1], [8.0, 14.0]),
    section(57.0, [7.5, 19.0], [7.1, 19.4], [13.5, 21.7]),
    section(61.0, [10.0, 21.0], [9.6, 21.4], [16.0, 26.0]),
    section(66.0, [13.5, 23.0], [13.0, 23.3], [21.0, 28.5]),
    section(71.0, [22.0, 31.0], [21.0, 31.5], [29.0, 24.0]),
    section(76.0, [24.0, 28.0], [24.0, 28.2], [27.1, 16.0]),
];

const SAILS: [(&[Section], [u8; 3], f32); 6] = [
    (&AUDITORIUM, [177, 186, 190], 0.72),
    (&GARDEN_SAIL, [176, 187, 190], 0.52),
    (&FIRST_STREET_SAIL, [193, 200, 202], 0.48),
    (&SECOND_STREET_SAIL, [158, 173, 181], 0.57),
    (&GRAND_AVENUE_SAIL, STEEL, 0.59),
    (&NORTH_FOLD, [176, 190, 197], 0.54),
];

fn section_row(part: Section, fraction: f32) -> [[f32; 3]; 5] {
    let Section { along, west, west_shoulder, ridge, roof, east_shoulder, east } = part;
    [
        [west, -3.0, along], [west_shoulder, roof * fraction, along], [ridge, roof, along],
        [east_shoulder, roof * (fraction + 0.05), along], [east, -3.0, along],
    ]
}

fn roof_at(cross: f32, along: f32, sections: &[Section]) -> f32 {
    let Some(pair) = sections.windows(2).find(|pair| (pair[0].along..=pair[1].along).contains(&along)) else {
        return 0.0;
    };
    let [a, b] = [pair[0], pair[1]];
    let t = (along - a.along) / (b.along - a.along);
    let lerp = |start, end| start + (end - start) * t;
    if cross >= lerp(a.west.min(a.west_shoulder), b.west.min(b.west_shoulder))
        && cross <= lerp(a.east.max(a.east_shoulder), b.east.max(b.east_shoulder))
    {
        lerp(a.roof, b.roof)
    } else {
        0.0
    }
}

pub(crate) fn clearance(x: f32, z: f32) -> f32 {
    if !contains(x, z) { return 0.0; }
    let [cross, along] = to_site(x, z);
    let lobby = if (-5.0..=31.0).contains(&cross) && (43.0..=72.0).contains(&along) {
        17.0
    } else {
        TERRACE
    };
    let offset = [x - CENTER[0], z - CENTER[1]];
    let dx = offset[0] - geometry::FOUNTAIN[0];
    let dz = offset[1] - geometry::FOUNTAIN[1];
    let garden = if dx * dx + dz * dz <= FOUNTAIN_RADIUS * FOUNTAIN_RADIUS {
        TERRACE + FOUNTAIN_HEIGHT
    } else if inside(offset, &geometry::AMPHITHEATRE_RING) {
        AMPHITHEATRE_PEAK
    } else {
        lobby
    };
    SAILS.iter().fold(garden.max(lobby), |height, (sections, _, _)| height.max(roof_at(cross, along, sections)))
}

fn quad(mesh: &mut WorldMesh<'_>, base: f32, points: [[f32; 3]; 4], color: [u8; 3]) {
    mesh.faded_quad(points.map(|point| site_point(point, base)), color, 1.0);
}

fn loft(mesh: &mut WorldMesh<'_>, base: f32, sections: &[Section], color: [u8; 3], fraction: f32) {
    let tone = |side: usize, end: f32| {
        let delta = [-19.0, 5.0, 23.0, -5.0][side] - end;
        color.map(|c| (c as f32 + delta).clamp(0.0, 255.0) as u8)
    };
    for (index, pair) in sections.windows(2).enumerate() {
        let [a, b] = [section_row(pair[0], fraction), section_row(pair[1], fraction)];
        for side in 0..4 {
            quad(mesh, base, [a[side], a[side + 1], b[side + 1], b[side]], tone(side, index as f32 * 1.65));
        }
    }
    for cap in [sections[0], sections[sections.len() - 1]] {
        let row = section_row(cap, fraction);
        for side in 0..4 {
            let ground = [row[side][0], -3.0, cap.along];
            let other_ground = [row[side + 1][0], -3.0, cap.along];
            quad(mesh, base, [ground, row[side], row[side + 1], other_ground], SHADED_STEEL);
        }
    }
}

fn wall_box(
    mesh: &mut WorldMesh<'_>, base: f32, floor: f32, roof: f32,
    footprint: [[f32; 2]; 4], wall: [u8; 3], top: [u8; 3],
) {
    let floor = footprint.map(|[x, z]| [x, floor, z]);
    let roof = footprint.map(|[x, z]| [x, roof, z]);
    quad(mesh, base, roof, top);
    for side in 0..4 {
        let next = (side + 1) % 4;
        quad(mesh, base, [floor[side], floor[next], roof[next], roof[side]], wall);
    }
}

fn draw_lobby(mesh: &mut WorldMesh<'_>, base: f32) {
    // The glazed entry sits back from the Grand Avenue/First Street corner.
    let lobby = [[-2.6, 44.0], [15.0, 43.5], [19.9, 63.0], [5.8, 72.0]];
    wall_box(mesh, base, -1.0, 16.0, lobby, GLASS, [116, 144, 151]);
    let [start, end] = [lobby[2], lobby[3]];
    for index in 0..7 {
        let t = index as f32 / 7.0;
        let cross = start[0] + (end[0] - start[0]) * t;
        let along = start[1] + (end[1] - start[1]) * t + 0.12;
        mesh.faded_segment(
            [site_point([cross, 0.0, along], base), site_point([cross - 1.4, 16.2, along], base)],
            0.36, [157, 169, 169], 1.0,
        );
    }
    for step in 0..4 {
        let top = 0.65 + step as f32 * 0.85;
        let advance = step as f32 * 0.86;
        wall_box(mesh, base, -0.6, top, [
            [6.5, 72.6 - advance], [18.4, 69.8 - advance],
            [18.8, 67.0 - advance], [7.0, 70.6 - advance],
        ], [134, 129, 119], [176, 167, 149]);
    }
    if mesh.daylight() < 0.44 {
        mesh.luminous_segment(
            [site_point([16.6, 5.6, 69.5], base), site_point([25.5, 5.6, 65.1], base)],
            1.1, [226, 160, 95],
        );
    }
}

fn draw_garden(mesh: &mut WorldMesh<'_>, base: f32) {
    let point = |[east, north]: [f32; 2], height| [
        CENTER[0] + east, base + height, CENTER[1] + north,
    ];
    for &triangle in &geometry::GARDEN_TOP {
        mesh.triangle(triangle.map(|p| point(p, TERRACE + 0.13)), [120, 134, 94]);
    }
    // The mapped amphitheatre cover is open at the front. Its ridges rise
    // above a low concrete terrace and share the hall's cool metal shades.
    let amphitheatre_height = |[east, north]: [f32; 2]| {
        let slope = (east + 31.0 + (north + 16.0) * 0.17).abs() * 0.12;
        (AMPHITHEATRE_PEAK - slope).clamp(6.2, AMPHITHEATRE_PEAK)
    };
    for &triangle in &geometry::AMPHITHEATRE_TOP {
        mesh.triangle(triangle.map(|p| point(p, amphitheatre_height(p))), [168, 182, 183]);
        mesh.triangle(triangle.map(|p| point(p, TERRACE + 0.16)), [161, 152, 138]);
    }
    let seating_height = |[east, north]: [f32; 2]| {
        let floor = (east + 31.0) * 0.066 + (north + 20.0) * 0.051;
        TERRACE + 0.53 + floor
    };
    for &triangle in &geometry::AMPHITHEATRE_SEATS {
        mesh.triangle(triangle.map(|p| point(p, seating_height(p))), [179, 174, 158]);
    }
    for &index in &[0, 3, 7, 11] {
        let corner = geometry::AMPHITHEATRE_RING[index];
        mesh.faded_segment([point(corner, TERRACE), point(corner, amphitheatre_height(corner))],
            0.47, [122, 133, 135], 1.0);
    }

    // A Rose for Lilly is a low blue-and-white porcelain sculpture over a
    // shallow pool. Its size is small even in the close garden pass.
    let center = geometry::FOUNTAIN;
    let flower = |angle, radius, height| {
        let (sine, cosine) = crate::math::sin_cos(angle);
        point([center[0] + radius * sine, center[1] + radius * cosine], TERRACE + height)
    };
    for petal in 0..8 {
        let angle = petal as f32 * core::f32::consts::FRAC_PI_4;
        let next = angle + core::f32::consts::FRAC_PI_4;
        mesh.triangle([point(center, TERRACE + 0.21), flower(angle, FOUNTAIN_RADIUS, 0.21),
            flower(next, FOUNTAIN_RADIUS, 0.21)], [58, 91, 114]);
        mesh.faded_quad([
            flower(angle, FOUNTAIN_RADIUS * 0.3, 0.37),
            flower(angle + 0.24, FOUNTAIN_RADIUS * 0.84, FOUNTAIN_HEIGHT * 0.76),
            flower(angle + 0.47, FOUNTAIN_RADIUS * 0.86, FOUNTAIN_HEIGHT * 0.92),
            flower(angle + 0.57, FOUNTAIN_RADIUS * 0.33, 0.54),
        ], if petal % 2 == 0 { [225, 230, 220] } else { [90, 143, 178] }, 1.0);
    }
    mesh.triangle([flower(0.0, 0.83, FOUNTAIN_HEIGHT), flower(2.1, 0.71, FOUNTAIN_HEIGHT * 0.9),
        flower(4.2, 0.71, FOUNTAIN_HEIGHT * 0.84)], [208, 222, 224]);
}

pub(crate) fn draw(mesh: &mut WorldMesh<'_>, base: f32) {
    // The urban garden is a shallow raised terrace along Hope Street.
    for &triangle in &geometry::TERRACE_TOP {
        mesh.triangle(triangle.map(|[e, n]| [CENTER[0] + e, base + TERRACE, CENTER[1] + n]),
            [154, 153, 136]);
    }
    for &[[a_x, a_z], [b_x, b_z]] in &geometry::TERRACE_EDGES {
        let [a, b] = [[CENTER[0] + a_x, CENTER[1] + a_z], [CENTER[0] + b_x, CENTER[1] + b_z]];
        mesh.faded_quad([
            [a[0], base - 7.0, a[1]], [b[0], base - 7.0, b[1]],
            [b[0], base + TERRACE, b[1]], [a[0], base + TERRACE, a[1]],
        ], LIMESTONE, 1.0);
    }
    draw_garden(mesh, base);
    wall_box(mesh, base, -4.0, 12.0, [
        [-16.0, -59.0], [12.2, -59.2], [12.0, -45.0], [-18.0, -42.0],
    ], [154, 156, 149], [179, 177, 167]);
    draw_lobby(mesh, base);
    for (sections, color, fraction) in SAILS {
        loft(mesh, base, sections, color, fraction);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_inner_court_and_other_city_blocks_unmasked() {
        assert!(replaces_raster(12.39, 597.86, 37.0));
        assert!(!replaces_raster(12.39, 597.86, 42.0));
        assert!(!replaces_raster(2.60, 551.12, 37.0));
        assert!(!replaces_raster(-48.31, 494.61, 37.0)); // The Broad
        assert!(!replaces_raster(110.71, 671.57, 37.0)); // Dorothy Chandler Pavilion
    }

    #[test]
    fn flight_uses_terrace_and_sail_heights() {
        assert!((clearance(-8.37, 623.0) - (TERRACE + FOUNTAIN_HEIGHT)).abs() < 0.01);
        assert_eq!(clearance(-19.24, 580.37), AMPHITHEATRE_PEAK);
        let [west, _, north] = site_point([-35.0, 0.0, 18.0], 0.0);
        assert_eq!(clearance(west, north), TERRACE);
        let [east, _, north] = site_point([33.0, 0.0, 34.0], 0.0);
        assert!((clearance(east, north) - HEIGHT).abs() < 0.01);
        assert_eq!(clearance(2.60, 551.12), 0.0);
    }
}
