//! Bankward width and sediment beds inside an already reserved dry bench.

use crate::math::smoothstep;
use crate::world::Surface;

#[derive(Clone, Copy)]
enum Deposit {
    GravelApron,
    ScrollRidges,
    DrySwales,
    SandSheet,
    Cobbles,
}

/// The curve applies to the measured bankward width, from upstream to downstream.
#[derive(Clone, Copy)]
pub struct PointBarProfile {
    pub name: &'static str,
    width: [f32; 5],
    deposit: Deposit,
}

use Deposit::*;

impl PointBarProfile {
    pub const ALL: [Self; 25] = [
        Self::new(
            "Upstream gravel wedge",
            [0.70, 1.00, 0.83, 0.64, 0.48],
            GravelApron,
        ),
        Self::new(
            "Downstream gravel fan",
            [0.45, 0.55, 0.73, 1.00, 0.90],
            GravelApron,
        ),
        Self::new(
            "Broad shingle shoulder",
            [0.74, 0.95, 1.00, 0.98, 0.76],
            GravelApron,
        ),
        Self::new(
            "Offset gravel spur",
            [0.60, 0.82, 0.96, 0.58, 0.50],
            GravelApron,
        ),
        Self::new(
            "Twin shingle lobes",
            [0.63, 1.00, 0.61, 0.96, 0.65],
            GravelApron,
        ),
        Self::new(
            "Three scroll ridges",
            [0.64, 0.84, 1.00, 0.91, 0.52],
            ScrollRidges,
        ),
        Self::new(
            "Upstream scroll fan",
            [0.91, 1.00, 0.90, 0.73, 0.59],
            ScrollRidges,
        ),
        Self::new(
            "Trailing scroll bar",
            [0.49, 0.64, 0.88, 1.00, 0.82],
            ScrollRidges,
        ),
        Self::new(
            "Narrow scroll tongue",
            [0.45, 0.71, 0.91, 0.69, 0.43],
            ScrollRidges,
        ),
        Self::new(
            "Interrupted scroll terrace",
            [0.78, 0.93, 0.59, 1.00, 0.82],
            ScrollRidges,
        ),
        Self::new(
            "Cutoff swale bar",
            [0.76, 0.82, 1.00, 0.85, 0.55],
            DrySwales,
        ),
        Self::new("Broad dry chute", [0.82, 0.91, 0.98, 1.00, 0.94], DrySwales),
        Self::new(
            "Inlet swale and spit",
            [0.86, 1.00, 0.83, 0.52, 0.60],
            DrySwales,
        ),
        Self::new(
            "Forked downstream swales",
            [0.46, 0.66, 0.94, 0.75, 1.00],
            DrySwales,
        ),
        Self::new(
            "Swale between two terraces",
            [0.68, 0.99, 0.65, 0.83, 0.92],
            DrySwales,
        ),
        Self::new(
            "Fine sand crescent",
            [0.50, 0.83, 1.00, 0.86, 0.49],
            SandSheet,
        ),
        Self::new("Pale sand apron", [0.72, 0.98, 0.98, 0.77, 0.60], SandSheet),
        Self::new(
            "Tapered sand spit",
            [0.90, 0.94, 0.75, 0.63, 0.40],
            SandSheet,
        ),
        Self::new(
            "Sand fan below the bend",
            [0.47, 0.61, 0.78, 0.96, 1.00],
            SandSheet,
        ),
        Self::new(
            "Two sand scallops",
            [0.54, 0.91, 0.68, 0.98, 0.77],
            SandSheet,
        ),
        Self::new(
            "Armored cobble bench",
            [0.89, 1.00, 0.94, 0.99, 0.85],
            Cobbles,
        ),
        Self::new(
            "Cobble ridges and silt pockets",
            [0.69, 0.97, 0.70, 0.98, 0.62],
            Cobbles,
        ),
        Self::new(
            "Head of bend cobble bar",
            [0.95, 0.99, 0.88, 0.58, 0.41],
            Cobbles,
        ),
        Self::new(
            "Narrow cobble riffle margin",
            [0.48, 0.60, 0.79, 0.74, 0.53],
            Cobbles,
        ),
        Self::new(
            "Tail of bend cobble fan",
            [0.44, 0.58, 0.66, 0.97, 0.95],
            Cobbles,
        ),
    ];

    const fn new(name: &'static str, width: [f32; 5], deposit: Deposit) -> Self {
        Self {
            name,
            width,
            deposit,
        }
    }

    pub fn width(self, along: f32) -> f32 {
        let position = along.clamp(0.0, 1.0) * 4.0;
        let index = (position as usize).min(3);
        let weight = smoothstep(position - index as f32);
        self.width[index] + (self.width[index + 1] - self.width[index]) * weight
    }

    /// Return the local lift and surface on the dry deposit. The placement
    /// supplies normalized coordinates within its surveyed landward edge.
    pub fn sediment(self, along: f32, across: f32) -> (f32, Surface) {
        let bend = 4.0 * along * (1.0 - along);
        let wave = |phase: f32| 1.0 - (2.0 * (phase - (phase as u32 as f32)) - 1.0).abs();
        match self.deposit {
            GravelApron => {
                let band = across - (0.10 + 0.11 * bend);
                (
                    0.19 * smoothstep((0.42 - band) * 2.3),
                    if band < 0.24 || band > 0.63 {
                        Surface::Gravel
                    } else {
                        Surface::RiverSilt
                    },
                )
            }
            ScrollRidges => {
                let crest = smoothstep(wave(0.12 + across * 3.1 + 0.53 * bend));
                (
                    0.32 * crest,
                    if crest > 0.62 {
                        Surface::Sand
                    } else {
                        Surface::RiverSilt
                    },
                )
            }
            DrySwales => {
                let chute = (across - (0.66 - 0.28 * bend)).abs();
                let trench = smoothstep(1.0 - chute * 5.5);
                (
                    -0.43 * trench,
                    if trench > 0.25 {
                        Surface::RiverSilt
                    } else {
                        Surface::Gravel
                    },
                )
            }
            SandSheet => {
                let crest = smoothstep(1.0 - (across - 0.50 + 0.16 * along).abs() * 2.0);
                (
                    0.26 * crest,
                    if crest > 0.48 {
                        Surface::Sand
                    } else {
                        Surface::RiverSilt
                    },
                )
            }
            Cobbles => {
                let thread = smoothstep(wave(across * 2.16 + along * 0.86));
                (
                    0.27 * thread,
                    if thread > 0.37 {
                        Surface::Gravel
                    } else {
                        Surface::RiverSilt
                    },
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_bar_shapes_are_distinct_and_preserve_the_survey_envelope() {
        let mut seen = [[0; 17]; PointBarProfile::ALL.len()];
        for (i, profile) in PointBarProfile::ALL.into_iter().enumerate() {
            for step in 0..=64 {
                let at = step as f32 / 64.0;
                let width = profile.width(at);
                assert!((0.39..=1.0).contains(&width));
                if step % 4 == 0 {
                    seen[i][step / 4] = (width * 1000.0) as u16;
                }
                let (relief, _) = profile.sediment(at, 1.0 - at);
                assert!((-0.44..=0.33).contains(&relief));
            }
            assert!(!seen[..i].contains(&seen[i]), "{}", profile.name);
        }
    }
}
