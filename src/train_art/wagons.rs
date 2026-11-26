pub(super) fn draw(
    point: impl Fn(f32, f32, f32) -> [f32; 3],
    mut face: impl FnMut([[f32; 3]; 4], [u8; 3]),
    color: [u8; 3],
    variant: u8,
    detail: bool,
) {
    let build = usize::from(variant % 5);
    let width = [1.01, 1.25, 1.12, 1.38, 1.31][build];
    let height = [2.59, 2.83, 3.13, 3.51, 3.79][build];
    let inset = [0.17, 0.12, 0.09, 0.055, 0.035][build];
    for side in [-1.36, 1.36] {
        face(
            [
                point(0.0, side, 0.62),
                point(1.0, side, 0.62),
                point(1.0, side, 1.17),
                point(0.0, side, 1.17),
            ],
            [68, 71, 70],
        );
        if detail {
            let axle = side * 0.70;
            for (from, to) in [(0.13, 0.27), (0.73, 0.87)] {
                face(
                    [
                        point(from, axle, 0.25),
                        point(to, axle, 0.25),
                        point(to, axle, 0.74),
                        point(from, axle, 0.74),
                    ],
                    [37, 47, 50],
                );
            }
        }
    }
    let base = 1.15;
    let squared = |top, shoulder: f32| {
        [
            [-width * shoulder, top],
            [width * shoulder, top],
            [width, top - 0.18],
            [width, base],
            [-width, base],
            [-width, top - 0.18],
        ]
    };
    match variant / 5 {
        // Intermodal container.
        0 => shell(
            &point,
            &mut face,
            squared(height, 0.96),
            [inset, 1.0 - inset],
            0.0,
            color,
        ),
        // Boxcar with rounded roof shoulders.
        1 => shell(
            &point,
            &mut face,
            squared(height, 0.58),
            [inset * 0.6, 1.0 - inset * 0.6],
            0.0,
            color,
        ),
        // Hopper with raised rim and tapered discharge section.
        2 => {
            let profile = [
                [-width, height],
                [width, height],
                [width, height - 0.23],
                [width * 0.48, base],
                [-width * 0.48, base],
                [-width, height - 0.23],
            ];
            shell(
                &point,
                &mut face,
                profile,
                [inset, 1.0 - inset],
                0.19,
                color,
            );
            let rim = width * 0.80;
            let floor = height + 0.012;
            face(
                [
                    point(inset + 0.035, -rim, floor),
                    point(1.0 - inset - 0.035, -rim, floor),
                    point(1.0 - inset - 0.035, rim, floor),
                    point(inset + 0.035, rim, floor),
                ],
                [55, 59, 61],
            );
        }
        // Tanker with a faceted cylinder and narrow mounting sill.
        3 => {
            let middle = (height + base) * 0.5;
            let profile = [
                [-width * 0.52, height],
                [width * 0.52, height],
                [width, middle],
                [width * 0.52, base + 0.13],
                [-width * 0.52, base + 0.13],
                [-width, middle],
            ];
            let steel = core::array::from_fn(|i| {
                ((u16::from(color[i]) + 3 * [154, 167, 176][i]) / 4) as u8
            });
            shell(&point, &mut face, profile, [inset, 1.0 - inset], 0.0, steel);
        }
        // Flatcar with two separate loads and an exposed center deck.
        _ => {
            face(
                [
                    point(0.0, -1.36, 1.17),
                    point(1.0, -1.36, 1.17),
                    point(1.0, 1.36, 1.17),
                    point(0.0, 1.36, 1.17),
                ],
                [105, 91, 71],
            );
            let load = tint(color, 0.88);
            shell(
                &point,
                &mut face,
                squared(height - 0.15, 0.75),
                [inset, 0.44],
                0.0,
                load,
            );
            shell(
                &point,
                &mut face,
                squared(height - 0.48, 0.75),
                [0.54, 1.0 - inset],
                0.0,
                load,
            );
        }
    }
}

fn shell(
    point: &impl Fn(f32, f32, f32) -> [f32; 3],
    face: &mut impl FnMut([[f32; 3]; 4], [u8; 3]),
    profile: [[f32; 2]; 6],
    [from, to]: [f32; 2],
    bottom_taper: f32,
    color: [u8; 3],
) {
    let [a, b] = [(from, bottom_taper), (to, -bottom_taper)]
        .map(|(t, taper)| profile.map(|[x, y]| point(t + taper * (2.0 - y).clamp(0.0, 1.0), x, y)));
    // The underframe hides the bottom face on the flight camera.
    for (i, j, shade) in [
        (0, 1, 1.13),
        (1, 2, 1.0),
        (2, 3, 0.89),
        (4, 5, 0.74),
        (5, 0, 0.91),
    ] {
        face([a[i], b[i], b[j], a[j]], tint(color, shade));
    }
    for end in [a, b] {
        face([end[0], end[1], end[2], end[5]], tint(color, 0.77));
        face([end[5], end[2], end[3], end[4]], tint(color, 0.72));
    }
}

pub(super) fn tint(color: [u8; 3], strength: f32) -> [u8; 3] {
    color.map(|channel| (f32::from(channel) * strength + 0.5) as u8)
}
