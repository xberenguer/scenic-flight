//! Allegiant Stadium on the same metre grid as the mapped Strip and airport.

#[path = "stadium_footprints.rs"]
mod footprints;

use crate::{world::{Surface, World}, world_mesh::WorldMesh};
use footprints::{GOLD_MARKS, MARK_CUTOUTS, MEMBRANE, ORANGE_MARKS, OUTLINE, WHITE_MARKS};

pub(super) const CENTER: [f32; 2] = [-868.30, -2_376.15];
// Clark County's submitted elevation separates the 196 ft roof from the 225 ft maximum envelope.
pub(super) const HEIGHT: f32 = 59.74;
pub(super) const REACH: f32 = 3_200.0;
const COLLAR_HEIGHT: f32 = 51.82;
const COLLAR: [u8; 3] = [40, 53, 68];
const SILVER: [u8; 3] = [192, 207, 218];
const CONCRETE: [u8; 3] = [164, 161, 152];
const NORTH_INGRESS: [[f32; 2]; 4] = [
    [34.46, 175.57], [75.99, 168.62], [91.63, 126.00], [43.45, 138.70],
];
// OSM way 816371013, whose corners establish the outdoor field tray's local axis.
const FIELD_TRAY: [[f32; 2]; 4] = [
    [-107.56, -154.36], [-43.59, -182.97], [-93.51, -294.60], [-157.47, -265.99],
];

fn edges<const N: usize>(points: &[[f32; 2]; N]) -> impl Iterator<Item = ([f32; 2], [f32; 2])> + '_ {
    points.iter().copied().zip(points.iter().copied().cycle().skip(1))
}

pub(super) fn contains(x: f32, z: f32) -> bool {
    let x = x - CENTER[0];
    let z = z - CENTER[1];
    if x.abs() > 132.0 || z.abs() > 148.0 {
        return false;
    }
    // The surveyed outline is convex. The margin includes 4 m raster cells cut by an edge.
    edges(&OUTLINE).all(|([ax, az], [bx, bz])| {
        let [east, north] = [bx - ax, bz - az];
        east * (z - az) - north * (x - ax) <= 4.0 * (east.abs() + north.abs())
    })
}

pub(super) fn legacy_prism(x: f32, z: f32, surface: Surface, height: f32) -> bool {
    matches!(height, 67.0 | 69.0) && matches!(surface,
        Surface::PitchedRoof | Surface::LandmarkRoof06 | Surface::LandmarkRoof09 |
        Surface::LandmarkRoof11 | Surface::LandmarkRoof20 | Surface::LandmarkRoof25 |
        Surface::LandmarkRoof35 | Surface::TowerRoof
    ) && contains(x, z)
}

pub(super) struct Stadium {
    base: f32,
    ingress: [[f32; 3]; 4],
    tray: [[f32; 3]; 4],
}

impl Stadium {
    pub(super) fn new(world: &World, base: f32) -> Self {
        let ground = |[east, north]: [f32; 2]| {
            let [x, z] = [CENTER[0] + east, CENTER[1] + north];
            [x, world.elevation(x, z) + 0.65, z]
        };
        Self { base, ingress: NORTH_INGRESS.map(ground), tray: FIELD_TRAY.map(ground) }
    }

    pub(super) fn draw(&self, mesh: &mut WorldMesh<'_>) {
        self.draw_site(mesh);
        self.draw_shell(mesh);
        self.draw_lanai(mesh);
        self.draw_membrane(mesh);
        self.draw_roof_markings(mesh);
    }

    fn point(&self, [east, north]: [f32; 2], height: f32) -> [f32; 3] {
        [CENTER[0] + east, self.base + height, CENTER[1] + north]
    }

    fn draw_site(&self, mesh: &mut WorldMesh<'_>) {
        mesh.faded_quad(self.ingress, CONCRETE, 1.0);
        for edge in [self.ingress[0], self.ingress[1]] {
            let top = [edge[0], edge[1] + 3.2, edge[2]];
            mesh.faded_segment([edge, top], 0.45, [73, 86, 97], 1.0);
        }

        // Field markings cover the directly mapped tray, with separate end zones and running rails.
        mesh.faded_quad(self.tray, [93, 98, 98], 1.0);
        let mix = |a: [f32; 3], b: [f32; 3], t: f32| core::array::from_fn(|axis| a[axis] + (b[axis] - a[axis]) * t);
        let field = |across, along| {
            let left = mix(self.tray[0], self.tray[3], along);
            let right = mix(self.tray[1], self.tray[2], along);
            let mut point = mix(left, right, across);
            point[1] += 0.22;
            point
        };
        let field_quad = |mesh: &mut WorldMesh<'_>, near, far, color| {
            mesh.faded_quad([field(0.15, near), field(0.85, near), field(0.85, far), field(0.15, far)], color, 1.0);
        };
        field_quad(mesh, 0.055, 0.13, [40, 49, 52]);
        field_quad(mesh, 0.87, 0.945, [40, 49, 52]);
        for section in 0..10 {
            let near = 0.13 + section as f32 * 0.074;
            let far = near + 0.074;
            let grass = if section % 2 == 0 { [88, 125, 65] } else { [76, 113, 62] };
            field_quad(mesh, near, far, grass);
            mesh.faded_segment([field(0.15, near), field(0.85, near)], 0.40, [222, 225, 209], 0.95);
        }
        mesh.faded_segment([field(0.15, 0.87), field(0.85, 0.87)], 0.40, [222, 225, 209], 0.95);
    }

    fn draw_shell(&self, mesh: &mut WorldMesh<'_>) {
        let wall = |[east, north]: [f32; 2], height, width| self.point([east * width, north * width], height);
        let ribbon = |[east, north]: [f32; 2]| {
            let north = (north / 144.0).clamp(-1.0, 1.0);
            16.0 + 7.0 * north + 3.0 * north * north + east / 80.0
        };
        for (a, b) in edges(&OUTLINE) {
            let light = ((a[0] - b[0]).abs() / ((a[0] - b[0]).abs() + (a[1] - b[1]).abs())).clamp(0.0, 1.0);
            let steel = [21_u8, 29, 36].map(|channel| (f32::from(channel) + light * 12.0) as u8);
            let fascia = |mesh: &mut WorldMesh<'_>, lo: [f32; 2], hi: [f32; 2], widths: [f32; 2], color| {
                mesh.faded_quad(
                    [wall(a, lo[0], widths[0]), wall(b, lo[1], widths[0]),
                     wall(b, hi[1], widths[1]), wall(a, hi[0], widths[1])],
                    color, 1.0,
                );
            };
            let bottom_trim = [ribbon(a), ribbon(b)];
            let upper_trim = bottom_trim.map(|height| height + 13.5);
            fascia(mesh, [0.0; 2], [4.6; 2], [0.952, 0.974], [62, 68, 73]);
            fascia(mesh, [4.6; 2], bottom_trim, [0.974, 0.995], steel);
            fascia(mesh, bottom_trim, bottom_trim.map(|height| height + 1.8), [0.995, 1.0], SILVER);
            fascia(mesh, bottom_trim.map(|height| height + 1.8), upper_trim, [1.0, 1.0], steel);
            fascia(mesh, upper_trim, upper_trim.map(|height| height + 1.1), [1.0, 1.0], [151, 164, 174]);
            fascia(mesh, upper_trim.map(|height| height + 1.1), [COLLAR_HEIGHT; 2], [1.0, 1.0], steel);
            mesh.triangle([self.point([0.0, 0.0], COLLAR_HEIGHT),
                self.point(a, COLLAR_HEIGHT), self.point(b, COLLAR_HEIGHT)], COLLAR);
        }
    }

    fn draw_lanai(&self, mesh: &mut WorldMesh<'_>) {
        // Four nominal 50-foot north-opening leaves, with dark mullions and upper reflections.
        let door = |east: f32, height: f32| {
            let [_, north] = horizontal_span(&OUTLINE, east, true);
            self.point([east, north + 0.65], height)
        };
        for section in 0..4 {
            let left = -2.4 + section as f32 * 15.24;
            let right = left + 15.24;
            let center = (left + right) * 0.5;
            for (a, b) in [(left, center), (center, right)] {
                mesh.faded_quad([door(a, 7.0), door(b, 7.0), door(b, 37.1), door(a, 37.1)], [76, 103, 124], 1.0);
                mesh.faded_quad([door(a, 30.0), door(b, 30.0), door(b, 36.7), door(a, 36.7)], [112, 138, 156], 1.0);
                mesh.faded_segment([door(a, 7.0), door(a, 37.1)], 0.65, [35, 49, 61], 1.0);
            }
        }
    }

    fn draw_membrane(&self, mesh: &mut WorldMesh<'_>) {
        const ROWS: usize = 24;
        const COLUMNS: usize = 6;
        let [south, north] = extent(&MEMBRANE);
        let row = |index: usize| {
            let northing = south + (north - south) * index as f32 / ROWS as f32;
            let span = horizontal_span(&MEMBRANE, northing, false);
            (northing, span)
        };
        let point = |(north, [left, right]): (f32, [f32; 2]), column: usize| {
            let across = column as f32 / COLUMNS as f32;
            let lateral = 2.0 * across - 1.0;
            let longitudinal = north / 103.7;
            let crown = (1.0 - lateral * lateral) * (1.0 - longitudinal * longitudinal);
            self.point([left + (right - left) * across, north],
                COLLAR_HEIGHT + (HEIGHT - COLLAR_HEIGHT) * crown.max(0.0))
        };
        let mut prior = row(0);
        for index in 0..ROWS {
            let next = row(index + 1);
            for column in 0..COLUMNS {
                let a = point(prior, column);
                let b = point(prior, column + 1);
                let c = point(next, column + 1);
                let d = point(next, column);
                let shade = (column as u8).abs_diff(3) * 3 + (index % 2) as u8 * 4;
                mesh.faded_quad([a, b, c, d], [171 - shade, 190 - shade, 207 - shade], 1.0);
                if index % 2 == 0 {
                    mesh.faded_segment([a, b], 0.38, [84, 105, 124], 0.72);
                }
            }
            prior = next;
        }
    }

    fn draw_roof_markings(&self, mesh: &mut WorldMesh<'_>) {
        // Surveyed positions of the factory-installed roof lettering and sunbursts.
        for (shapes, color) in [
            (&WHITE_MARKS[..], [235, 237, 231]), (&ORANGE_MARKS[..], [246, 133, 32]),
            (&GOLD_MARKS[..], [253, 205, 70]), (&MARK_CUTOUTS[..], COLLAR),
        ] {
            for triangle in shapes {
                mesh.triangle(triangle.map(|point| self.point(point, COLLAR_HEIGHT + 0.11)), color);
            }
        }
    }
}

fn extent<const N: usize>(polygon: &[[f32; 2]; N]) -> [f32; 2] {
    polygon.iter().fold([f32::INFINITY, f32::NEG_INFINITY], |[min, max], point| {
        [min.min(point[1]), max.max(point[1])]
    })
}

fn horizontal_span<const N: usize>(polygon: &[[f32; 2]; N], ordinate: f32, swap: bool) -> [f32; 2] {
    let axis = usize::from(swap);
    let scan = 1 - axis;
    let mut range = [f32::INFINITY, f32::NEG_INFINITY];
    for (a, b) in edges(polygon) {
        let delta = b[scan] - a[scan];
        if delta == 0.0 || ordinate < a[scan].min(b[scan]) || ordinate > a[scan].max(b[scan]) {
            continue;
        }
        let coordinate = a[axis] + (b[axis] - a[axis]) * ((ordinate - a[scan]) / delta);
        range[0] = range[0].min(coordinate);
        range[1] = range[1].max(coordinate);
    }
    range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_prisms_only_yield_to_the_stadium_on_its_mapped_site() {
        assert!(legacy_prism(CENTER[0], CENTER[1], Surface::LandmarkRoof11, 69.0));
        assert!(legacy_prism(CENTER[0] - 90.0, CENTER[1], Surface::LandmarkRoof25, 67.0));
        assert!(!legacy_prism(CENTER[0] + 145.0, CENTER[1], Surface::LandmarkRoof11, 69.0));
        assert!(!legacy_prism(CENTER[0], CENTER[1], Surface::PitchedRoof, 5.0));
        assert!(!contains(CENTER[0] + 125.0, CENTER[1] + 125.0));
    }
}
