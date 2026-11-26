//! Erosion profiles within the surveyed badlands footprint.

use crate::math::smoothstep;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RavineProfile {
    pub name: &'static str,
    width: [f32; 3],
    bank: [f32; 2],
    throat: f32,
    shelf: f32,
    pub(super) layers: f32,
}

impl RavineProfile {
    pub const ALL: [Self; 25] = [
        Self::new(
            "Box-headed ravine",
            [0.99, 0.77, 0.68],
            [0.92, 0.94],
            0.42,
            0.26,
            0.34,
        ),
        Self::new(
            "Narrow outlet gorge",
            [0.99, 0.83, 0.55],
            [0.94, 0.97],
            0.61,
            0.17,
            0.35,
        ),
        Self::new(
            "Open wash mouth",
            [0.60, 0.82, 1.00],
            [1.00, 0.96],
            0.42,
            0.34,
            0.31,
        ),
        Self::new(
            "Twin shelf walls",
            [0.88, 0.98, 0.87],
            [1.00, 1.00],
            0.59,
            0.42,
            0.24,
        ),
        Self::new(
            "Western talus apron",
            [0.88, 0.94, 0.91],
            [1.00, 0.66],
            0.37,
            0.43,
            0.40,
        ),
        Self::new(
            "Eastern talus apron",
            [0.81, 0.97, 0.88],
            [0.68, 1.00],
            0.40,
            0.44,
            0.36,
        ),
        Self::new(
            "Hourglass narrows",
            [0.99, 0.57, 0.99],
            [0.89, 0.92],
            0.54,
            0.19,
            0.27,
        ),
        Self::new(
            "Broad middle amphitheater",
            [0.62, 1.00, 0.66],
            [0.97, 0.99],
            0.62,
            0.36,
            0.23,
        ),
        Self::new(
            "Splayed left mouth",
            [0.70, 0.88, 1.00],
            [1.00, 0.69],
            0.55,
            0.31,
            0.31,
        ),
        Self::new(
            "Splayed right mouth",
            [0.70, 0.88, 1.00],
            [0.69, 1.00],
            0.55,
            0.31,
            0.35,
        ),
        Self::new(
            "High sandstone slot",
            [0.60, 0.63, 0.55],
            [0.87, 0.92],
            0.55,
            0.15,
            0.21,
        ),
        Self::new(
            "Low sandstone slot",
            [0.62, 0.55, 0.69],
            [0.91, 0.86],
            0.41,
            0.18,
            0.48,
        ),
        Self::new(
            "Pebble bench gorge",
            [0.83, 0.86, 0.88],
            [1.00, 0.91],
            0.37,
            0.45,
            0.19,
        ),
        Self::new(
            "Staggered wall benches",
            [0.96, 0.75, 0.93],
            [0.90, 0.74],
            0.59,
            0.44,
            0.38,
        ),
        Self::new(
            "Lower clay amphitheater",
            [0.64, 0.79, 0.99],
            [0.90, 0.99],
            0.62,
            0.39,
            0.44,
        ),
        Self::new(
            "Upper clay amphitheater",
            [1.00, 0.80, 0.59],
            [0.96, 0.81],
            0.64,
            0.32,
            0.41,
        ),
        Self::new(
            "Sawtooth notch",
            [0.84, 0.56, 0.92],
            [0.99, 0.72],
            0.41,
            0.18,
            0.43,
        ),
        Self::new(
            "Strata bowl",
            [0.73, 0.97, 0.74],
            [0.97, 1.00],
            0.49,
            0.40,
            0.49,
        ),
        Self::new(
            "Mudstone ledge route",
            [0.98, 0.88, 0.71],
            [0.69, 0.98],
            0.37,
            0.34,
            0.46,
        ),
        Self::new(
            "Wide red shelf",
            [0.98, 0.91, 1.00],
            [1.00, 0.93],
            0.55,
            0.45,
            0.28,
        ),
        Self::new(
            "Limestone cap over shale",
            [0.62, 0.83, 0.89],
            [0.89, 0.96],
            0.62,
            0.22,
            0.18,
        ),
        Self::new(
            "Hardpan pinch",
            [0.93, 0.61, 0.81],
            [0.96, 0.92],
            0.46,
            0.26,
            0.24,
        ),
        Self::new(
            "East wall runout",
            [0.71, 0.89, 0.99],
            [0.77, 1.00],
            0.38,
            0.37,
            0.40,
        ),
        Self::new(
            "Deep terraced cleft",
            [0.81, 0.87, 0.66],
            [0.89, 1.00],
            0.63,
            0.43,
            0.32,
        ),
        Self::new(
            "Long bench and slot",
            [0.77, 0.91, 0.56],
            [1.00, 0.84],
            0.61,
            0.31,
            0.44,
        ),
    ];

    const fn new(
        name: &'static str,
        width: [f32; 3],
        bank: [f32; 2],
        throat: f32,
        shelf: f32,
        layers: f32,
    ) -> Self {
        Self {
            name,
            width,
            bank,
            throat,
            shelf,
            layers,
        }
    }

    pub(super) fn sides(self, progress: f32, across: f32) -> f32 {
        let [head, middle, mouth] = self.width;
        let width = if progress < 0.5 {
            head + (middle - head) * smoothstep(progress * 2.0)
        } else {
            middle + (mouth - middle) * smoothstep(progress * 2.0 - 1.0)
        };
        across.abs() / (width * self.bank[usize::from(across >= 0.0)])
    }

    pub(super) fn cut(self, sides: f32) -> f32 {
        let inner = smoothstep((self.throat - sides) * 3.1);
        let shoulder = smoothstep((1.0 - sides) * 2.25);
        (1.0 - self.shelf) * inner + self.shelf * shoulder
    }

    pub fn study(self, progress: f32, across: f32) -> (f32, bool, u8) {
        let sides = self.sides(progress, across);
        let fade = super::end_fade(progress);
        let cut = self.cut(sides) * fade * (1.0 - 0.55 * sides.min(1.0) * (1.0 - fade));
        let floor = 9.0 * (1.0 - progress);
        let land = floor + 12.0;
        let elevation = land - 12.0 * cut;
        let stratum = ((elevation + 1.2) * self.layers) as u8 % 5;
        (
            elevation,
            sides < 0.26 && fade > 0.48 && cut > 0.09,
            stratum,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_profile_has_unique_wall_geometry_with_a_continuous_surveyed_center() {
        let profiles = RavineProfile::ALL.map(|design| {
            [0.21, 0.39, 0.57, 0.77, 0.88].map(|t| {
                assert_eq!(design.cut(design.sides(t, 0.0)), 1.0);
                assert_eq!(design.cut(design.sides(t, -1.0)), 0.0);
                assert_eq!(design.cut(design.sides(t, 1.0)), 0.0);
                [-0.81, -0.56, -0.38, -0.19, 0.19, 0.38, 0.56, 0.81]
                    .map(|s| (design.cut(design.sides(t, s)) * 100.0).round() as u8)
            })
        });
        for (index, profile) in profiles.iter().enumerate() {
            assert!(
                !profiles[..index].contains(profile),
                "{}",
                RavineProfile::ALL[index].name
            );
        }
    }
}
