//! Snow bands viewed edge-on. Lobes can bend, split, or evaporate at
//! different heights while sharing one terrain-occluded precipitation volume.

use crate::math::smoothstep;

#[derive(Clone, Copy, Debug)]
pub struct SnowfallProfile {
    pub name: &'static str,
    lobes: [Lobe; 3],
}

#[derive(Clone, Copy, Debug)]
struct Lobe {
    start: f32,
    end: f32,
    spread: f32,
    reach: f32,
    density: f32,
}

const fn lobe(start: f32, end: f32, spread: f32, reach: f32, density: f32) -> Lobe {
    Lobe {
        start,
        end,
        spread,
        reach,
        density,
    }
}

const NONE: Lobe = lobe(0.0, 0.0, 1.0, 0.0, 0.0);

impl SnowfallProfile {
    const fn new(name: &'static str, lobes: [Lobe; 3]) -> Self {
        Self { name, lobes }
    }

    /// Cross-section opacity. Coordinates are measured from the wind-adjusted
    /// fall line and cloud base, respectively. Phase is in texture cycles.
    pub fn opacity(&self, across: f32, falling: f32, phase: f32) -> f32 {
        let mut density = 0.0;
        for lobe in self.lobes {
            let center = lobe.start + (lobe.end - lobe.start) * falling;
            let edge = (1.0 - ((across - center) * lobe.spread).abs()).max(0.0);
            let tail = ((lobe.reach - falling) * 6.0).clamp(0.0, 1.0);
            density += edge * edge * tail * lobe.density;
        }
        let streak = falling * 6.3 + across * 3.7 + 7.0 - phase;
        let grain = 1.0 - (2.0 * (streak - streak as i32 as f32) - 1.0).abs();
        let edges = smoothstep((1.0 - across.abs()) * 3.5)
            * smoothstep(falling * 7.0)
            * smoothstep((1.0 - falling) * 4.0);
        (0.08 + density.min(1.0) * (0.74 + grain * 0.26)) * edges
    }
}

pub const SNOWFALL_PROFILES: [SnowfallProfile; 25] = [
    SnowfallProfile::new(
        "Broad curtain",
        [lobe(0.0, 0.0, 1.05, 1.0, 1.0), NONE, NONE],
    ),
    SnowfallProfile::new(
        "Narrow column",
        [lobe(0.02, 0.05, 3.8, 1.0, 1.0), NONE, NONE],
    ),
    SnowfallProfile::new(
        "Westward streamer",
        [lobe(0.37, -0.69, 2.7, 1.05, 1.0), NONE, NONE],
    ),
    SnowfallProfile::new(
        "Eastward streamer",
        [lobe(-0.40, 0.68, 2.8, 1.04, 1.0), NONE, NONE],
    ),
    SnowfallProfile::new(
        "Parallel columns",
        [
            lobe(-0.42, -0.40, 3.7, 1.04, 0.94),
            lobe(0.44, 0.46, 3.7, 1.03, 0.92),
            NONE,
        ],
    ),
    SnowfallProfile::new(
        "Splitting bank",
        [
            lobe(-0.04, -0.72, 2.8, 1.05, 0.97),
            lobe(0.06, 0.67, 2.8, 1.04, 0.93),
            NONE,
        ],
    ),
    SnowfallProfile::new(
        "Converging bank",
        [
            lobe(-0.72, -0.04, 3.0, 1.03, 0.92),
            lobe(0.70, 0.04, 3.0, 1.0, 0.89),
            NONE,
        ],
    ),
    SnowfallProfile::new(
        "Triple curtain",
        [
            lobe(-0.61, -0.62, 4.4, 1.02, 0.82),
            lobe(0.0, 0.0, 4.1, 1.03, 0.97),
            lobe(0.60, 0.59, 4.3, 1.03, 0.86),
        ],
    ),
    SnowfallProfile::new(
        "Crossed streamers",
        [
            lobe(-0.62, 0.62, 4.0, 1.03, 0.99),
            lobe(0.66, -0.55, 4.0, 1.03, 0.88),
            NONE,
        ],
    ),
    SnowfallProfile::new(
        "High virga",
        [lobe(-0.10, 0.18, 1.55, 0.53, 1.0), NONE, NONE],
    ),
    SnowfallProfile::new(
        "Stepped tails",
        [
            lobe(-0.54, -0.62, 4.0, 1.04, 0.96),
            lobe(-0.04, 0.02, 3.6, 0.76, 0.95),
            lobe(0.48, 0.57, 3.4, 0.49, 0.94),
        ],
    ),
    SnowfallProfile::new(
        "Forked virga",
        [
            lobe(0.13, -0.87, 3.6, 0.67, 0.96),
            lobe(0.15, 0.73, 4.2, 0.62, 0.95),
            NONE,
        ],
    ),
    SnowfallProfile::new(
        "Shelf and tail",
        [
            lobe(0.02, -0.20, 1.2, 0.54, 0.91),
            lobe(0.26, 0.55, 4.8, 1.02, 0.87),
            NONE,
        ],
    ),
    SnowfallProfile::new(
        "Wind comb",
        [
            lobe(-0.62, -0.15, 5.0, 1.02, 0.91),
            lobe(-0.17, 0.36, 5.0, 0.91, 0.91),
            lobe(0.28, 0.82, 5.0, 0.77, 0.91),
        ],
    ),
    SnowfallProfile::new(
        "Lee fan",
        [
            lobe(-0.45, -0.60, 2.4, 0.94, 0.82),
            lobe(-0.36, 0.21, 3.1, 1.0, 0.82),
            lobe(-0.32, 0.80, 3.9, 1.03, 0.92),
        ],
    ),
    SnowfallProfile::new(
        "Ridge cap",
        [
            lobe(0.0, 0.0, 1.13, 0.39, 0.95),
            lobe(-0.25, -0.36, 3.7, 0.65, 0.47),
            lobe(0.34, 0.49, 3.5, 0.64, 0.53),
        ],
    ),
    SnowfallProfile::new(
        "Uneven squall",
        [
            lobe(-0.24, -0.49, 1.55, 1.12, 0.95),
            lobe(0.53, 0.39, 4.7, 0.68, 0.91),
            NONE,
        ],
    ),
    SnowfallProfile::new(
        "Heavy core and fringe",
        [
            lobe(0.10, 0.17, 2.95, 1.12, 0.98),
            lobe(-0.48, -0.80, 3.8, 0.72, 0.53),
            lobe(0.54, 0.76, 3.4, 0.79, 0.48),
        ],
    ),
    SnowfallProfile::new(
        "Drifting sheet",
        [lobe(-0.39, 0.48, 1.28, 1.08, 0.96), NONE, NONE],
    ),
    SnowfallProfile::new(
        "Uneven twins",
        [
            lobe(-0.51, -0.20, 2.9, 0.59, 0.91),
            lobe(0.42, 0.33, 2.25, 1.11, 1.0),
            NONE,
        ],
    ),
    SnowfallProfile::new(
        "Braided fall",
        [
            lobe(-0.55, 0.25, 4.1, 1.03, 0.88),
            lobe(-0.01, -0.61, 4.4, 0.92, 0.88),
            lobe(0.49, 0.57, 4.0, 1.03, 0.92),
        ],
    ),
    SnowfallProfile::new(
        "Open center",
        [
            lobe(-0.49, -0.74, 2.3, 1.09, 1.0),
            lobe(0.51, 0.74, 2.3, 1.09, 1.0),
            NONE,
        ],
    ),
    SnowfallProfile::new(
        "Low drift and virga",
        [
            lobe(0.61, -0.25, 2.9, 1.12, 1.0),
            lobe(-0.44, -0.53, 2.45, 0.52, 0.94),
            NONE,
        ],
    ),
    SnowfallProfile::new(
        "Interrupted curtain",
        [
            lobe(-0.63, -0.50, 3.25, 0.74, 0.94),
            lobe(0.02, 0.05, 4.15, 0.51, 0.92),
            lobe(0.53, 0.47, 2.85, 0.88, 1.0),
        ],
    ),
    SnowfallProfile::new(
        "Dense snow squall",
        [
            lobe(-0.20, -0.63, 1.72, 1.13, 1.0),
            lobe(0.09, 0.43, 1.7, 1.13, 1.0),
            lobe(0.48, 0.18, 2.1, 1.12, 0.87),
        ],
    ),
];

#[cfg(test)]
mod tests {
    use super::SNOWFALL_PROFILES;
    use std::collections::HashSet;
    use std::vec::Vec;

    #[test]
    fn every_profile_shapes_a_distinct_continuous_band() {
        let mut silhouettes = HashSet::new();
        for profile in &SNOWFALL_PROFILES {
            let silhouette: Vec<_> = (1..20)
                .map(|row| {
                    let falling = row as f32 / 20.0;
                    assert_eq!(profile.opacity(-1.0, falling, 0.3), 0.0);
                    assert_eq!(profile.opacity(1.0, falling, 0.3), 0.0);
                    (0..32).fold(0_u32, |bits, col| {
                        let across = col as f32 / 16.0 - 1.0;
                        let lit = profile.opacity(across, falling, 0.3) > 0.29;
                        bits | (u32::from(lit) << col)
                    })
                })
                .collect();
            assert!(
                silhouette.iter().filter(|row| **row != 0).count() > 3,
                "{}",
                profile.name
            );
            assert!(silhouettes.insert(silhouette), "{}", profile.name);
        }
    }
}
