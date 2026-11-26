//! Crest and secondary-ridge shapes for sheltered beach dunes.

#[derive(Clone, Copy)]
pub struct CoveDuneProfile {
    pub name: &'static str,
    pub(super) breadth: f32,
    pub(super) apron: f32,
    pub(super) crest_gap: f32,
    lean: [f32; 7],
    height: [f32; 7],
}

const fn profile(
    name: &'static str,
    breadth: f32,
    apron: f32,
    crest_gap: f32,
    lean: [f32; 7],
    height: [f32; 7],
) -> CoveDuneProfile {
    CoveDuneProfile {
        name,
        breadth,
        apron,
        crest_gap,
        lean,
        height,
    }
}

impl CoveDuneProfile {
    pub const ALL: [Self; 25] = [
        profile(
            "Even crescent",
            1.02,
            0.65,
            0.07,
            [0.0, 0.0, 0.03, 0.04, 0.03, 0.0, 0.0],
            [0.88, 0.96, 1.0, 1.0, 1.0, 0.96, 0.88],
        ),
        profile(
            "Twin crests",
            1.0,
            0.77,
            0.08,
            [-0.04, -0.06, 0.04, 0.18, 0.04, -0.06, -0.04],
            [0.91, 1.05, 0.96, 0.7, 0.96, 1.05, 0.91],
        ),
        profile(
            "Sheltered notch",
            1.02,
            0.62,
            0.14,
            [0.05, 0.08, 0.04, -0.15, 0.04, 0.08, 0.05],
            [0.88, 0.96, 1.08, 0.59, 1.08, 0.96, 0.88],
        ),
        profile(
            "West ramp",
            1.08,
            0.58,
            0.02,
            [0.22, 0.17, 0.08, -0.02, -0.11, -0.16, -0.17],
            [1.05, 1.05, 1.04, 0.99, 0.92, 0.82, 0.72],
        ),
        profile(
            "East ramp",
            1.08,
            0.58,
            0.02,
            [-0.17, -0.16, -0.11, -0.02, 0.08, 0.17, 0.22],
            [0.72, 0.82, 0.92, 0.99, 1.04, 1.05, 1.05],
        ),
        profile(
            "Wind-cut shoulder",
            0.96,
            0.56,
            0.13,
            [0.13, -0.07, -0.18, -0.01, 0.12, 0.18, -0.03],
            [0.95, 0.76, 0.67, 0.94, 1.08, 0.99, 0.82],
        ),
        profile(
            "Wind-built shoulder",
            0.96,
            0.56,
            0.13,
            [-0.03, 0.18, 0.12, -0.01, -0.18, -0.07, 0.13],
            [0.82, 0.99, 1.08, 0.94, 0.67, 0.76, 0.95],
        ),
        profile(
            "Hourglass ridge",
            1.08,
            0.68,
            0.11,
            [-0.04, 0.22, 0.14, -0.03, 0.14, 0.22, -0.04],
            [0.87, 1.02, 0.9, 0.68, 0.9, 1.02, 0.87],
        ),
        profile(
            "Elbow dune",
            1.03,
            0.77,
            0.02,
            [-0.22, -0.19, -0.16, -0.11, 0.12, 0.21, 0.19],
            [0.84, 0.92, 0.98, 1.07, 1.03, 0.9, 0.81],
        ),
        profile(
            "Reversed elbow",
            1.03,
            0.77,
            0.02,
            [0.19, 0.21, 0.12, -0.11, -0.16, -0.19, -0.22],
            [0.81, 0.9, 1.03, 1.07, 0.98, 0.92, 0.84],
        ),
        profile(
            "Three blowouts",
            1.08,
            0.74,
            0.16,
            [-0.06, 0.19, -0.17, 0.17, -0.17, 0.19, -0.06],
            [0.78, 0.6, 1.08, 0.63, 1.08, 0.6, 0.78],
        ),
        profile(
            "Fine double crescent",
            0.92,
            0.91,
            0.23,
            [0.08, 0.05, 0.01, -0.03, 0.01, 0.05, 0.08],
            [0.8, 0.9, 1.02, 1.02, 1.02, 0.9, 0.8],
        ),
        profile(
            "Blunt central crest",
            1.15,
            0.56,
            0.02,
            [-0.19, -0.04, 0.16, 0.22, 0.16, -0.04, -0.19],
            [0.65, 0.82, 1.05, 1.08, 1.05, 0.82, 0.65],
        ),
        profile(
            "Broken sill",
            1.04,
            0.63,
            0.13,
            [0.1, -0.08, 0.16, -0.1, 0.1, -0.15, 0.03],
            [0.85, 1.06, 0.74, 1.05, 0.75, 1.06, 0.85],
        ),
        profile(
            "Offset twin arcs",
            1.13,
            0.78,
            0.18,
            [0.04, 0.21, 0.1, -0.16, -0.1, 0.13, 0.05],
            [0.91, 1.06, 0.91, 0.65, 1.04, 1.06, 0.87],
        ),
        profile(
            "Splayed wings",
            1.12,
            0.66,
            0.07,
            [0.24, 0.16, -0.02, -0.21, -0.02, 0.16, 0.24],
            [0.99, 1.01, 0.94, 0.81, 0.94, 1.01, 0.99],
        ),
        profile(
            "Long lee ridge",
            1.14,
            0.5,
            0.03,
            [-0.18, -0.2, -0.17, -0.08, 0.02, 0.05, 0.14],
            [0.9, 1.01, 1.04, 1.04, 1.04, 0.96, 0.77],
        ),
        profile(
            "Shallow bow",
            0.98,
            0.83,
            0.14,
            [0.22, 0.12, 0.01, -0.04, 0.0, 0.1, 0.21],
            [0.91, 0.86, 0.93, 0.98, 0.96, 0.87, 0.91],
        ),
        profile(
            "Shelter pocket",
            1.08,
            0.82,
            0.06,
            [-0.06, 0.01, 0.18, 0.21, -0.16, -0.14, 0.02],
            [0.88, 0.96, 1.07, 0.65, 0.67, 0.98, 0.93],
        ),
        profile(
            "Upwind bulge",
            1.02,
            0.74,
            0.19,
            [-0.19, -0.09, 0.18, 0.24, 0.09, -0.1, -0.18],
            [0.88, 0.94, 1.04, 1.07, 0.9, 0.85, 0.79],
        ),
        profile(
            "Downwind bulge",
            1.02,
            0.74,
            0.19,
            [-0.18, -0.1, 0.09, 0.24, 0.18, -0.09, -0.19],
            [0.79, 0.85, 0.9, 1.07, 1.04, 0.94, 0.88],
        ),
        profile(
            "Rolling scallops",
            1.09,
            0.76,
            0.13,
            [-0.12, -0.01, 0.15, -0.11, 0.18, -0.05, -0.14],
            [0.76, 0.91, 1.07, 0.83, 1.04, 0.77, 0.84],
        ),
        profile(
            "Paired blowouts",
            1.07,
            0.59,
            0.16,
            [-0.05, 0.22, -0.06, -0.2, -0.04, 0.22, -0.04],
            [0.91, 0.6, 1.04, 1.06, 1.04, 0.6, 0.91],
        ),
        profile(
            "Terraced lee",
            1.15,
            0.83,
            0.1,
            [-0.16, -0.16, -0.09, -0.02, 0.08, 0.16, 0.2],
            [0.9, 1.04, 0.98, 1.06, 0.91, 1.05, 0.81],
        ),
        profile(
            "Tapered hook",
            0.94,
            0.69,
            0.09,
            [0.23, 0.16, 0.04, -0.07, -0.16, -0.06, 0.1],
            [1.03, 1.04, 1.0, 0.95, 0.9, 0.83, 0.64],
        ),
    ];

    pub(super) fn crest(&self, u: f32) -> [f32; 2] {
        let sample = (u + 1.0) * 3.0;
        let knot = (sample as usize).min(5);
        let t = crate::math::smoothstep(sample - knot as f32);
        [
            self.lean[knot] + (self.lean[knot + 1] - self.lean[knot]) * t,
            self.height[knot] + (self.height[knot + 1] - self.height[knot]) * t,
        ]
    }

    /// Illustrate the production dune relief on an isolated flat shore.
    pub fn study(&self, [x, z]: [f32; 2]) -> Option<crate::world::Sample> {
        use crate::world::{Sample, Surface};
        let raw = Sample {
            elevation: 3.4,
            surface: Surface::Grass,
        };
        let dune = super::CoveDune {
            axis: [1.0, 0.0],
            seaward: [0.0, 1.0],
            inverse_length: 1.0 / 48.0,
            inverse_width: 1.0 / 15.0,
            bow: 0.72,
            tilt: 0.0,
            relief: 2.87,
            profile: 0,
        };
        let sample = dune.overlay_profile([x, z], raw, self);
        (sample != raw).then_some(sample)
    }
}
