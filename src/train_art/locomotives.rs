// Original meshes based on wide-cab line haul, narrow branch hoods, and terminal switchers.
use super::{FreightRoute, wagons::tint};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(usize)]
pub(super) enum Locomotive {
    WideCab,
    HeavyHaul,
    RoadSwitcher,
    HighHood,
    DieselTransfer,
    BatteryTransfer,
}

impl Locomotive {
    pub(super) fn for_route(route: FreightRoute, trip_distance: f32, key: u64) -> Self {
        use FreightRoute::*;
        let roll = (key >> 21) & 7;
        match route {
            Intermodal | Automotive | Perishable => Self::WideCab,
            Quarry | Aggregate | Agriculture => Self::HeavyHaul,
            Mill if trip_distance < 230.0 => Self::DieselTransfer,
            Liquid if trip_distance < 230.0 => {
                if roll == 0 {
                    Self::BatteryTransfer
                } else {
                    Self::DieselTransfer
                }
            }
            Forest if roll != 0 => Self::HighHood,
            _ => Self::RoadSwitcher,
        }
    }
}

#[derive(Clone, Copy)]
struct Shell {
    ends: [f32; 2],
    half: f32,
    top: f32,
    roof_half: f32,
    shoulder: f32,
    roof_shade: f32,
    color: [u8; 3],
}

const fn shell(from: f32, to: f32, half: f32, top: f32, color: [u8; 3]) -> Shell {
    Shell {
        ends: [from, to],
        half,
        top,
        roof_half: half * 0.90,
        shoulder: top - 0.12,
        roof_shade: 0.75,
        color,
    }
}

const fn cab(from: f32, to: f32, half: f32, top: f32, color: [u8; 3]) -> Shell {
    Shell {
        roof_half: half * 0.80,
        shoulder: top - 0.21,
        roof_shade: 0.87,
        ..shell(from, to, half, top, color)
    }
}

struct Design {
    chassis: [f32; 2],
    shell: &'static [Shell],
    cab: usize,
    wheel_ends: [f32; 2],
    three_axle: bool,
    front: [u8; 3],
}

const DECK: f32 = 1.08;
const FRAME: [u8; 3] = [55, 62, 62];
const SILLS: [u8; 3] = [100, 108, 102];
const GLASS: [u8; 3] = [41, 68, 79];
const ACCENT: [u8; 3] = [204, 182, 119];
const GOLD: [u8; 3] = [206, 155, 66];
const DARK_RED: [u8; 3] = [156, 76, 60];
const BLUE: [u8; 3] = [75, 131, 149];
const GREEN: [u8; 3] = [86, 128, 82];
const ORANGE: [u8; 3] = [192, 103, 53];
const MINT: [u8; 3] = [92, 148, 127];

const DESIGNS: [Design; 6] = [
    Design {
        chassis: [0.037, 0.963],
        shell: &[
            shell(0.088, 0.251, 1.19, 2.55, GOLD),
            cab(0.228, 0.412, 1.32, 3.60, GOLD),
            shell(0.416, 0.937, 0.96, 2.94, [181, 127, 66]),
        ],
        cab: 1,
        wheel_ends: [0.18, 0.82],
        three_axle: true,
        front: GOLD,
    },
    Design {
        chassis: [0.025, 0.975],
        shell: &[
            shell(0.068, 0.226, 1.25, 2.81, DARK_RED),
            cab(0.211, 0.377, 1.35, 3.68, DARK_RED),
            shell(0.379, 0.963, 1.12, 3.17, [113, 105, 96]),
        ],
        cab: 1,
        wheel_ends: [0.197, 0.803],
        three_axle: true,
        front: DARK_RED,
    },
    Design {
        chassis: [0.068, 0.932],
        shell: &[
            shell(0.114, 0.316, 0.69, 1.96, BLUE),
            cab(0.296, 0.472, 1.20, 3.39, BLUE),
            shell(0.474, 0.850, 0.78, 2.72, BLUE),
        ],
        cab: 1,
        wheel_ends: [0.221, 0.779],
        three_axle: false,
        front: BLUE,
    },
    Design {
        chassis: [0.062, 0.938],
        shell: &[
            shell(0.105, 0.340, 0.68, 3.12, GREEN),
            cab(0.327, 0.482, 1.17, 3.56, GREEN),
            shell(0.484, 0.884, 0.77, 3.00, GREEN),
        ],
        cab: 1,
        wheel_ends: [0.213, 0.787],
        three_axle: false,
        front: GREEN,
    },
    Design {
        chassis: [0.11, 0.89],
        shell: &[
            cab(0.155, 0.341, 1.15, 3.25, ORANGE),
            shell(0.343, 0.840, 0.70, 2.46, ORANGE),
        ],
        cab: 0,
        wheel_ends: [0.252, 0.748],
        three_axle: false,
        front: ORANGE,
    },
    Design {
        chassis: [0.10, 0.90],
        shell: &[
            shell(0.155, 0.427, 0.92, 2.03, MINT),
            cab(0.391, 0.611, 1.27, 3.34, MINT),
            shell(0.574, 0.846, 0.92, 2.03, MINT),
        ],
        cab: 1,
        wheel_ends: [0.240, 0.760],
        three_axle: true,
        front: MINT,
    },
];

pub(super) fn draw(
    point: impl Fn(f32, f32, f32) -> [f32; 3],
    face: impl FnMut([[f32; 3]; 4], [u8; 3], f32),
    locomotive: Locomotive,
    detail: bool,
    leading: bool,
    dusk: f32,
) {
    let design = &DESIGNS[locomotive as usize];
    let mut model = Model { point, face };
    model.hood(design.chassis, [1.25, 1.34], [0.54, DECK], FRAME);
    // Keep the couplers at the established car length on every frame.
    model.sides([0.012, 0.10], 0.40, [0.67, 0.91], FRAME);
    model.sides([0.90, 0.988], 0.40, [0.67, 0.91], FRAME);
    model.sides(design.chassis, 1.34, [0.94, DECK], SILLS);

    for (index, segment) in design.shell.iter().enumerate() {
        let is_cab = index == design.cab;
        let top_half = segment.roof_half;
        let wall_half = segment.half * 0.96;
        let shoulder = segment.shoulder;
        model.hood(
            segment.ends,
            [segment.half, wall_half],
            [DECK, shoulder],
            segment.color,
        );
        let [a, b] = segment.ends;
        let roof = tint(segment.color, segment.roof_shade);
        model.hood(
            segment.ends,
            [wall_half, top_half],
            [shoulder, segment.top],
            roof,
        );
        if is_cab {
            // Side glazing stays visible in the coarse mesh. It follows the wall slope.
            let levels = [segment.top - 0.83, segment.top - 0.26];
            let window_half = levels.map(|h| {
                segment.half + (wall_half - segment.half) * ((h - DECK) / (shoulder - DECK)) + 0.014
            });
            model.tapered_sides([a + 0.018, b - 0.021], window_half, levels, GLASS);
            let front_at = a - 0.002;
            let g = segment.half * 0.69;
            if locomotive == Locomotive::HighHood {
                for [left, right] in [[-g, -0.46], [0.46, g]] {
                    model.end(
                        front_at,
                        [left, right],
                        [segment.top - 0.73, segment.top - 0.25],
                        GLASS,
                        0.0,
                    );
                }
            } else {
                model.end(
                    front_at,
                    [-g, g],
                    [segment.top - 0.79, segment.top - 0.25],
                    GLASS,
                    0.0,
                );
            }
        }
    }
    match locomotive {
        Locomotive::WideCab => {
            // Low canted nose stands out in a shallow head-on view.
            let base = design.shell[0];
            model.quad(
                [
                    [0.054, -0.88, 1.64],
                    [0.054, 0.88, 1.64],
                    [base.ends[0], 1.01, 2.43],
                    [base.ends[0], -1.01, 2.43],
                ],
                tint(GOLD, 0.85),
                0.0,
            );
            model.top([0.70, 0.895], 0.70, 2.947, [77, 79, 72]);
        }
        Locomotive::HeavyHaul => {
            // Raised wide radiator shoulders overhang the narrower engine compartment.
            model.hood([0.643, 0.923], [1.39, 1.26], [2.95, 3.64], [87, 92, 92]);
            model.top([0.671, 0.897], 1.10, 3.648, [53, 64, 64]);
        }
        Locomotive::RoadSwitcher => {
            model.top([0.639, 0.831], 0.59, 2.728, [62, 82, 86]);
        }
        Locomotive::HighHood => {
            model.top([0.613, 0.867], 0.58, 3.008, [79, 95, 67]);
        }
        Locomotive::DieselTransfer => {
            model.top([0.603, 0.810], 0.51, 2.468, [103, 83, 68]);
        }
        Locomotive::BatteryTransfer => {
            model.top([0.181, 0.381], 0.74, 2.039, [136, 168, 145]);
            model.top([0.628, 0.818], 0.74, 2.039, [136, 168, 145]);
        }
    }

    // Bright pilot for terminal visibility and as a daytime front marker.
    let [front, rear] = design.chassis;
    model.end(front - 0.003, [-1.04, 1.04], [0.70, 1.19], ACCENT, 0.0);
    model.end(
        rear + 0.003,
        [-1.04, 1.04],
        [0.70, 1.09],
        tint(design.front, 0.75),
        0.0,
    );
    if detail {
        let axles = if design.three_axle { 3 } else { 2 };
        for center in design.wheel_ends {
            for n in 0..axles {
                let t = center + (n as f32 - (axles - 1) as f32 * 0.5) * 0.071;
                model.sides([t - 0.026, t + 0.026], 0.98, [0.19, 0.56], [39, 47, 49]);
            }
        }
    }
    if dusk > 0.02 {
        let (color, emission) = if leading {
            ([255, 223, 150], 0.93)
        } else {
            ([207, 63, 48], 0.65)
        };
        for center in [-0.78, 0.78] {
            model.end(
                front - 0.006,
                [center - 0.215, center + 0.215],
                [1.08, 1.42],
                color,
                dusk * emission,
            );
        }
    }
}

struct Model<P, F> {
    point: P,
    face: F,
}

impl<P, F> Model<P, F>
where
    P: Fn(f32, f32, f32) -> [f32; 3],
    F: FnMut([[f32; 3]; 4], [u8; 3], f32),
{
    fn quad(&mut self, corners: [[f32; 3]; 4], color: [u8; 3], emission: f32) {
        (self.face)(
            corners.map(|[t, s, h]| (self.point)(t, s, h)),
            color,
            emission,
        );
    }

    fn end(
        &mut self,
        t: f32,
        [a, b]: [f32; 2],
        [bottom, top]: [f32; 2],
        color: [u8; 3],
        emission: f32,
    ) {
        self.quad(
            [[t, a, bottom], [t, b, bottom], [t, b, top], [t, a, top]],
            color,
            emission,
        );
    }

    fn sides(&mut self, span: [f32; 2], half: f32, levels: [f32; 2], color: [u8; 3]) {
        self.tapered_sides(span, [half; 2], levels, color);
    }

    fn tapered_sides(
        &mut self,
        [a, b]: [f32; 2],
        [low, high]: [f32; 2],
        [bottom, top]: [f32; 2],
        color: [u8; 3],
    ) {
        for (s, shade) in [(-1.0, 0.81), (1.0, 1.0)] {
            self.quad(
                [
                    [a, s * low, bottom],
                    [b, s * low, bottom],
                    [b, s * high, top],
                    [a, s * high, top],
                ],
                tint(color, shade),
                0.0,
            );
        }
    }

    fn top(&mut self, [a, b]: [f32; 2], half: f32, y: f32, color: [u8; 3]) {
        self.quad(
            [[a, -half, y], [a, half, y], [b, half, y], [b, -half, y]],
            color,
            0.0,
        );
    }

    fn hood(
        &mut self,
        [a, b]: [f32; 2],
        [bottom_half, top_half]: [f32; 2],
        [bottom, top]: [f32; 2],
        color: [u8; 3],
    ) {
        self.top([a, b], top_half, top, tint(color, 1.09));
        for s in [-1.0, 1.0] {
            self.quad(
                [
                    [a, s * bottom_half, bottom],
                    [b, s * bottom_half, bottom],
                    [b, s * top_half, top],
                    [a, s * top_half, top],
                ],
                tint(color, if s > 0.0 { 1.0 } else { 0.81 }),
                0.0,
            );
        }
        for t in [a, b] {
            self.quad(
                [
                    [t, -bottom_half, bottom],
                    [t, bottom_half, bottom],
                    [t, top_half, top],
                    [t, -top_half, top],
                ],
                tint(color, 0.75),
                0.0,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{render::Camera, sky::Sky, world_mesh::WorldMesh};

    #[test]
    fn engine_assignments_fit_freight_and_terminal_distance() {
        use FreightRoute::*;
        use Locomotive::*;
        for (route, distance, key, expected) in [
            (Intermodal, 410.0, 0, WideCab),
            (Aggregate, 162.0, 41, HeavyHaul),
            (Forest, 312.0, 1 << 21, HighHood),
            (Mixed, 270.0, 23, RoadSwitcher),
            (Mill, 165.0, 53, DieselTransfer),
            (Liquid, 160.0, 1 << 21, DieselTransfer),
            (Liquid, 160.0, 0, BatteryTransfer),
            (Liquid, 410.0, 0, RoadSwitcher),
        ] {
            assert_eq!(Locomotive::for_route(route, distance, key), expected);
        }
    }

    #[test]
    fn locomotives_fit_train_bounds_and_keep_distinct_uncolored_profiles() {
        const SIDE: usize = 96;
        let sky = Sky::from_time(41, 0, 0.5);
        for (detail, zoom) in [(true, 1.0), (false, 2.0)] {
            let camera = Camera {
                x: 10.4 * zoom,
                z: -7.2 * zoom,
                altitude: 8.4 * zoom,
                heading: -0.966,
                pitch: 0.445,
                bank: 0.0,
            };
            let mut shapes = std::collections::BTreeSet::new();
            for locomotive in [
                Locomotive::WideCab,
                Locomotive::HeavyHaul,
                Locomotive::RoadSwitcher,
                Locomotive::HighHood,
                Locomotive::DieselTransfer,
                Locomotive::BatteryTransfer,
            ] {
                let mut pixels = [0; SIDE * SIDE];
                let mut depth = [u16::MAX; SIDE * SIDE];
                let mut mesh =
                    WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
                draw(
                    |t, side, height| {
                        assert!((0.0..=1.0).contains(&t));
                        assert!(side.abs() <= 1.43);
                        assert!((0.0..=3.87).contains(&height));
                        [-side, height, (t - 0.5) * 13.6]
                    },
                    |[a, b, c, d], _, _| {
                        mesh.triangle([a, b, c], [167, 164, 157]);
                        mesh.triangle([a, c, d], [167, 164, 157]);
                    },
                    locomotive,
                    detail,
                    true,
                    0.0,
                );
                let outline = depth.map(|d| d != u16::MAX);
                assert!(outline.iter().filter(|&&pixel| pixel).count() > 55);
                assert!(shapes.insert(outline), "{locomotive:?} shares an outline");
            }
        }
    }
}
