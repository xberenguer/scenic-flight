//! Excavation profiles for road-served hillside quarries.

#[derive(Clone, Copy)]
pub struct QuarryProfile {
    pub name: &'static str,
    pub(super) flank: f32,
    pub(super) throat: f32,
    pub(super) drift: f32,
    pub(super) skew: f32,
    pub(super) terraces: [f32; 3],
}

const fn profile(
    name: &'static str,
    flank: f32,
    throat: f32,
    drift: f32,
    skew: f32,
    terraces: [f32; 3],
) -> QuarryProfile {
    QuarryProfile {
        name,
        flank,
        throat,
        drift,
        skew,
        terraces,
    }
}

impl QuarryProfile {
    pub const ALL: [Self; 25] = [
        profile("Classic half bowl", 0.24, 0.0, 0.0, 0.0, [0.22, 0.45, 0.71]),
        profile("Wide first bench", 0.2, 0.04, 0.0, 0.0, [0.32, 0.54, 0.72]),
        profile("Deep first cut", 0.26, 0.02, 0.04, 0.0, [0.17, 0.42, 0.68]),
        profile("Staged cutback", 0.18, 0.08, 0.02, 0.01, [0.19, 0.53, 0.74]),
        profile("Narrow inlet", 0.28, 0.32, -0.03, 0.0, [0.23, 0.46, 0.73]),
        profile("Westward sweep", 0.2, 0.09, -0.24, 0.13, [0.24, 0.48, 0.73]),
        profile("Eastward sweep", 0.2, 0.09, 0.24, -0.13, [0.24, 0.48, 0.73]),
        profile(
            "Southwest opening",
            0.33,
            0.2,
            -0.19,
            -0.15,
            [0.2, 0.45, 0.72],
        ),
        profile(
            "Southeast opening",
            0.33,
            0.2,
            0.19,
            0.15,
            [0.2, 0.45, 0.72],
        ),
        profile(
            "Long loading floor",
            0.12,
            0.06,
            -0.05,
            0.05,
            [0.35, 0.57, 0.77],
        ),
        profile(
            "Steep rear ledges",
            0.22,
            0.16,
            0.05,
            0.03,
            [0.18, 0.4, 0.77],
        ),
        profile(
            "Narrow upper saddle",
            0.36,
            0.12,
            0.09,
            -0.02,
            [0.24, 0.46, 0.66],
        ),
        profile("Open shelf", 0.07, 0.03, 0.01, -0.04, [0.31, 0.53, 0.75]),
        profile(
            "Shallow crescent",
            0.35,
            0.06,
            0.02,
            0.02,
            [0.29, 0.55, 0.76],
        ),
        profile("Splayed cut", 0.21, 0.25, 0.07, 0.02, [0.26, 0.49, 0.69]),
        profile(
            "Back-swept benches",
            0.27,
            0.14,
            -0.15,
            0.1,
            [0.19, 0.47, 0.78],
        ),
        profile(
            "Flared west ledges",
            0.34,
            0.16,
            0.08,
            0.19,
            [0.28, 0.53, 0.72],
        ),
        profile(
            "Flared east ledges",
            0.34,
            0.16,
            -0.08,
            -0.19,
            [0.28, 0.53, 0.72],
        ),
        profile(
            "Low triple terrace",
            0.31,
            0.18,
            0.02,
            0.0,
            [0.17, 0.39, 0.63],
        ),
        profile(
            "Staggered terraces",
            0.23,
            0.1,
            -0.06,
            0.14,
            [0.3, 0.46, 0.76],
        ),
        profile("Raked floor", 0.16, 0.23, 0.12, -0.14, [0.24, 0.4, 0.69]),
        profile("Ramped throat", 0.14, 0.3, -0.1, -0.07, [0.18, 0.44, 0.74]),
        profile(
            "Broad lower crescent",
            0.36,
            0.03,
            -0.1,
            0.06,
            [0.36, 0.55, 0.71],
        ),
        profile(
            "Tucked upper bench",
            0.25,
            0.15,
            0.13,
            -0.08,
            [0.26, 0.52, 0.8],
        ),
        profile(
            "Close lower benches",
            0.27,
            0.09,
            0.11,
            0.1,
            [0.22, 0.37, 0.72],
        ),
    ];

    /// Surface elevation and removed depth for the production excavation on a study slope.
    pub fn study(&self, across: f32, along: f32) -> Option<(f32, f32)> {
        let quarry = super::Quarry {
            junction: [0.0, 30.0, -54.0],
            yard: [0.0, 30.0, 0.0],
            axis: [0.0, 1.0],
            length: 54.0,
            inverse_length: 1.0 / 54.0,
            curve: 0.0,
            inverse_width: 1.0 / 34.0,
            inverse_depth: 1.0 / 72.0,
            bench: 6.1,
            bounds: [-34.0, -13.0, 34.0, 72.0],
            form: super::QuarryForm::Hillside,
            profile: 0,
        };
        let (floor, weight) = quarry.excavation_profile(along, across, Some(self))?;
        let raw = 30.0 + 0.38 * along.max(0.0);
        let depth = (raw - floor).max(0.0) * weight;
        Some((raw - depth, depth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_cut_has_a_distinct_terrace_profile_inside_the_surveyed_footprint() {
        let mut cuts = std::collections::HashSet::new();
        for profile in QuarryProfile::ALL {
            let mut signature = std::vec::Vec::new();
            for along in (-15..75).step_by(3) {
                for across in (-36_i32..=36).step_by(3) {
                    let sample = profile.study(across as f32, along as f32);
                    if across.abs() >= 34 || !(-11..=72).contains(&along) {
                        assert!(sample.is_none(), "{}", profile.name);
                    }
                    signature.push(sample.map(|(_, depth)| (depth * 2.0) as u8));
                }
            }
            assert!(
                signature
                    .iter()
                    .filter(|depth| depth.is_some_and(|depth| depth > 2))
                    .count()
                    > 100,
                "{} has no recognizable open cut",
                profile.name
            );
            assert!(
                cuts.insert(signature),
                "{} repeats another excavation",
                profile.name
            );
        }
    }
}
