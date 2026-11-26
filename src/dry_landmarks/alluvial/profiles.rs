//! Distinct footprints and drainage courses for surveyed dry fans.

#[derive(Clone, Copy)]
pub struct FanProfile {
    pub name: &'static str,
    pub(super) aspect: [f32; 2],
    pub(super) flare: f32,
    pub(super) bend: f32,
    pub(super) streams: [Channel; 3],
}

#[derive(Clone, Copy)]
pub(super) struct Channel {
    pub split: f32,
    pub toe: f32,
    pub direction: f32,
    pub bend: f32,
    pub width: f32,
}

const fn channel(split: f32, toe: f32, direction: f32, bend: f32, width: f32) -> Channel {
    Channel {
        split,
        toe,
        direction,
        bend,
        width,
    }
}

const fn profile(
    name: &'static str,
    aspect: [f32; 2],
    flare: f32,
    bend: f32,
    streams: [Channel; 3],
) -> FanProfile {
    FanProfile {
        name,
        aspect,
        flare,
        bend,
        streams,
    }
}

impl FanProfile {
    pub const ALL: [Self; 25] = [
        profile(
            "Wide gravel apron",
            [1.0, 1.12],
            0.38,
            0.0,
            [
                channel(0.12, 0.92, -0.86, -27.0, 5.9),
                channel(0.0, 1.0, 0.02, 18.0, 6.2),
                channel(0.15, 0.96, 0.83, 29.0, 5.4),
            ],
        ),
        profile(
            "Narrow canyon outlet",
            [1.05, 0.79],
            0.22,
            0.16,
            [
                channel(0.31, 0.94, -0.59, -13.0, 4.1),
                channel(0.0, 1.0, -0.12, -18.0, 5.0),
                channel(0.18, 0.83, 0.71, 20.0, 3.8),
            ],
        ),
        profile(
            "Late three-way split",
            [1.12, 1.02],
            0.25,
            -0.12,
            [
                channel(0.41, 0.99, -0.99, -16.0, 4.4),
                channel(0.0, 1.0, 0.08, 8.0, 5.7),
                channel(0.4, 0.95, 0.97, 14.0, 4.4),
            ],
        ),
        profile(
            "Open straight fork",
            [0.96, 1.11],
            0.44,
            0.04,
            [
                channel(0.07, 0.99, -0.84, -7.0, 5.0),
                channel(0.0, 0.84, -0.04, 0.0, 4.2),
                channel(0.1, 1.0, 0.88, 8.0, 5.4),
            ],
        ),
        profile(
            "Rocky scallop",
            [0.91, 1.06],
            0.67,
            0.07,
            [
                channel(0.13, 0.68, -0.86, -22.0, 5.2),
                channel(0.0, 1.0, 0.03, 15.0, 6.4),
                channel(0.24, 0.81, 0.77, 28.0, 5.2),
            ],
        ),
        profile(
            "Southwest sweep",
            [1.01, 0.96],
            0.37,
            -0.32,
            [
                channel(0.09, 0.98, -0.96, -51.0, 5.4),
                channel(0.0, 1.0, -0.34, -24.0, 5.3),
                channel(0.17, 0.84, 0.51, -48.0, 4.0),
            ],
        ),
        profile(
            "Southeast sweep",
            [1.01, 0.96],
            0.37,
            0.32,
            [
                channel(0.17, 0.84, -0.51, 48.0, 4.0),
                channel(0.0, 1.0, 0.34, 24.0, 5.3),
                channel(0.09, 0.98, 0.96, 51.0, 5.4),
            ],
        ),
        profile(
            "Double left lobe",
            [1.03, 1.11],
            0.49,
            -0.15,
            [
                channel(0.1, 0.94, -0.92, -56.0, 5.3),
                channel(0.0, 1.0, -0.49, -31.0, 5.0),
                channel(0.27, 0.89, 0.63, -24.0, 5.5),
            ],
        ),
        profile(
            "Double right lobe",
            [1.03, 1.11],
            0.49,
            0.15,
            [
                channel(0.27, 0.89, -0.63, 24.0, 5.5),
                channel(0.0, 1.0, 0.49, 31.0, 5.0),
                channel(0.1, 0.94, 0.92, 56.0, 5.3),
            ],
        ),
        profile(
            "Opposing hooks",
            [1.08, 0.97],
            0.3,
            0.0,
            [
                channel(0.14, 0.97, -0.74, 49.0, 4.7),
                channel(0.0, 1.0, 0.02, -32.0, 5.0),
                channel(0.19, 0.95, 0.82, -50.0, 4.6),
            ],
        ),
        profile(
            "Long gravel tongue",
            [1.14, 0.82],
            0.14,
            -0.06,
            [
                channel(0.12, 0.64, -0.8, -30.0, 5.0),
                channel(0.0, 1.0, -0.23, -23.0, 6.2),
                channel(0.17, 0.73, 0.71, 27.0, 4.2),
            ],
        ),
        profile(
            "Short twin outlets",
            [0.88, 1.1],
            0.62,
            0.04,
            [
                channel(0.08, 1.0, -0.93, -34.0, 5.9),
                channel(0.0, 0.67, 0.07, 1.0, 4.3),
                channel(0.11, 1.0, 0.93, 36.0, 5.8),
            ],
        ),
        profile(
            "Narrow braided fan",
            [1.09, 0.78],
            0.29,
            0.05,
            [
                channel(0.13, 0.91, -0.66, 36.0, 4.2),
                channel(0.0, 1.0, 0.02, -37.0, 5.0),
                channel(0.27, 0.98, 0.62, -35.0, 4.2),
            ],
        ),
        profile(
            "Plunging foothill apron",
            [0.85, 0.98],
            0.49,
            0.11,
            [
                channel(0.21, 0.81, -0.78, -17.0, 4.8),
                channel(0.0, 1.0, 0.15, 30.0, 6.7),
                channel(0.33, 0.93, 0.91, 37.0, 4.5),
            ],
        ),
        profile(
            "Stepped apron",
            [0.98, 0.99],
            0.54,
            -0.11,
            [
                channel(0.07, 0.76, -0.74, -50.0, 4.6),
                channel(0.0, 1.0, -0.15, 38.0, 5.1),
                channel(0.24, 0.92, 0.88, 49.0, 4.3),
            ],
        ),
        profile(
            "Broad distributary garden",
            [0.96, 1.18],
            0.45,
            -0.02,
            [
                channel(0.07, 0.92, -0.98, -29.0, 6.5),
                channel(0.0, 1.0, -0.03, 33.0, 6.2),
                channel(0.07, 0.93, 0.97, 27.0, 6.4),
            ],
        ),
        profile(
            "Bent trunk and split",
            [1.1, 0.95],
            0.25,
            0.22,
            [
                channel(0.28, 0.99, -0.92, 27.0, 4.8),
                channel(0.0, 0.98, 0.24, 56.0, 5.4),
                channel(0.36, 0.94, 0.72, -35.0, 4.7),
            ],
        ),
        profile(
            "Foothill wedge",
            [1.01, 0.94],
            0.11,
            -0.22,
            [
                channel(0.17, 0.84, -0.9, -19.0, 6.0),
                channel(0.0, 1.0, -0.32, 11.0, 5.5),
                channel(0.3, 0.95, 0.83, 41.0, 4.1),
            ],
        ),
        profile(
            "Bulb apron",
            [0.88, 1.19],
            0.72,
            0.02,
            [
                channel(0.09, 0.73, -0.83, -44.0, 6.1),
                channel(0.0, 1.0, 0.03, -4.0, 6.0),
                channel(0.12, 0.79, 0.89, 43.0, 6.1),
            ],
        ),
        profile(
            "Curved central throat",
            [1.08, 0.89],
            0.35,
            0.25,
            [
                channel(0.25, 0.79, -0.82, -36.0, 4.2),
                channel(0.0, 1.0, -0.28, 65.0, 5.1),
                channel(0.24, 0.91, 0.79, -22.0, 4.8),
            ],
        ),
        profile(
            "Feathered eastern toe",
            [0.98, 1.13],
            0.5,
            0.16,
            [
                channel(0.14, 0.74, -0.78, -17.0, 4.3),
                channel(0.0, 0.91, 0.09, 44.0, 6.5),
                channel(0.06, 1.0, 0.91, 62.0, 6.1),
            ],
        ),
        profile(
            "Feathered western toe",
            [0.98, 1.13],
            0.5,
            -0.16,
            [
                channel(0.06, 1.0, -0.91, -62.0, 6.1),
                channel(0.0, 0.91, -0.09, -44.0, 6.5),
                channel(0.14, 0.74, 0.78, 17.0, 4.3),
            ],
        ),
        profile(
            "Uneven splay",
            [1.05, 1.04],
            0.39,
            -0.03,
            [
                channel(0.07, 1.0, -0.82, -11.0, 5.5),
                channel(0.0, 1.0, -0.17, 52.0, 5.3),
                channel(0.39, 0.81, 0.84, 29.0, 5.2),
            ],
        ),
        profile(
            "Tight curved apron",
            [0.87, 0.84],
            0.61,
            0.22,
            [
                channel(0.18, 0.97, -0.86, -43.0, 4.5),
                channel(0.0, 1.0, 0.17, 38.0, 6.7),
                channel(0.18, 0.92, 0.71, 52.0, 4.7),
            ],
        ),
        profile(
            "Concave mountain apron",
            [1.13, 1.15],
            0.06,
            -0.06,
            [
                channel(0.13, 0.91, -0.88, 21.0, 4.9),
                channel(0.0, 1.0, -0.1, -27.0, 7.0),
                channel(0.23, 0.98, 0.85, -20.0, 5.1),
            ],
        ),
    ];

    /// Sample the exact production overlay for an isolated illustration.
    pub fn study(self, x: f32, z: f32) -> Option<crate::world::Sample> {
        let fan = super::Fan::new([0.0, 1.0], 108.0, 42.0, 0.0, 0, self);
        let raw = crate::world::Sample {
            elevation: 38.0 - z * 0.096,
            surface: crate::world::Surface::Desert,
        };
        let sample = fan.overlay([x, z], raw);
        (sample != raw).then_some(sample)
    }
}
