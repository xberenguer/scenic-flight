//! Shapes of rockfall deposits inside the surveyed alpine apron.

use crate::math::smoothstep;

#[derive(Clone, Copy)]
enum Bed {
    Chute,
    Fork,
    Terraces,
    Ribs,
    Apron,
}

#[derive(Clone, Copy)]
pub struct TalusProfile {
    pub name: &'static str,
    width: f32,
    toe: f32,
    drift: f32,
    bed: Bed,
}

use Bed::*;

impl TalusProfile {
    pub const ALL: [Self; 25] = [
        Self::new("Straight rockfall chute", 0.57, 0.96, 0.00, Chute),
        Self::new("Left-swept chute", 0.64, 0.94, -0.21, Chute),
        Self::new("Right-swept chute", 0.74, 0.91, 0.20, Chute),
        Self::new("Short pocket chute", 0.71, 0.73, -0.06, Chute),
        Self::new("Long confined chute", 0.44, 1.00, 0.08, Chute),
        Self::new("Twin rockfall tongues", 0.94, 0.95, 0.00, Fork),
        Self::new("Fork with a longer left tongue", 0.84, 1.00, -0.14, Fork),
        Self::new("Fork with a longer right tongue", 0.86, 0.98, 0.15, Fork),
        Self::new("High split below a rib", 0.72, 0.86, -0.06, Fork),
        Self::new("Broad low fork", 0.97, 0.89, 0.07, Fork),
        Self::new("Three terrace steps", 0.85, 0.97, 0.02, Terraces),
        Self::new("Shelf above a short toe", 0.93, 0.79, -0.05, Terraces),
        Self::new("Narrow stacked ledges", 0.63, 0.92, 0.09, Terraces),
        Self::new("Outer terrace fan", 0.95, 0.99, 0.03, Terraces),
        Self::new("Asymmetric stair fan", 0.77, 0.92, -0.19, Terraces),
        Self::new("Stone ribs and fine gullies", 0.91, 0.94, 0.00, Ribs),
        Self::new("Leaning ribbed fan", 0.85, 1.00, 0.11, Ribs),
        Self::new("Short ribbed bank", 0.78, 0.77, -0.07, Ribs),
        Self::new("Long stone ribbons", 0.64, 0.98, -0.18, Ribs),
        Self::new("Fan around a central rock rib", 0.96, 0.91, 0.02, Ribs),
        Self::new("Wide scalloped apron", 0.98, 0.99, -0.01, Apron),
        Self::new("Short boulder skirt", 0.96, 0.80, 0.03, Apron),
        Self::new("Left lobe apron", 0.89, 0.96, -0.11, Apron),
        Self::new("Right lobe apron", 0.82, 0.96, 0.17, Apron),
        Self::new("Deep convex apron", 0.94, 1.00, 0.05, Apron),
    ];

    const fn new(name: &'static str, width: f32, toe: f32, drift: f32, bed: Bed) -> Self {
        Self {
            name,
            width,
            toe,
            drift,
            bed,
        }
    }

    /// Coverage, added elevation, and coarse-block cover within the dry apron.
    /// The coordinates are normalized downhill along the grade and across it.
    pub fn study(self, along: f32, across: f32) -> (f32, f32, bool) {
        if !(0.0..1.0).contains(&along) || across.abs() >= 1.0 {
            return (0.0, 0.0, false);
        }
        let travel = smoothstep(along);
        let shift = self.drift * travel;
        let half_width = self.width * (1.0 - shift.abs());
        let side = (across - shift) / half_width;
        let wing = side.abs();
        let shape = 1.0 - wing * wing;
        let midpoint = 1.0 - smoothstep(wing * 2.1);
        let bank = 1.0 - smoothstep((wing - 0.42).abs() * 2.5);
        let wave = |phase: f32| 1.0 - (2.0 * (phase - phase as u32 as f32) - 1.0).abs();
        let (toe, channel, lumps) = match self.bed {
            Chute => (self.toe * (0.73 + 0.27 * shape), 0.0, 0.28 * bank),
            Fork => {
                let gap = midpoint * smoothstep((along - 0.36) * 2.15);
                let reach = 0.73 + 0.23 * bank + side * self.drift * 0.5;
                (self.toe * reach, gap * 0.92, bank * 0.37)
            }
            Terraces => {
                let tread = smoothstep(wave(along * 3.6 + 0.10));
                (
                    self.toe * (0.83 + 0.17 * shape),
                    0.18 * (1.0 - tread),
                    0.56 * tread,
                )
            }
            Ribs => {
                let rib = smoothstep(wave(0.13 + wing * 2.35 + 0.42 * along));
                (
                    self.toe * (0.70 + 0.12 * shape + 0.15 * rib),
                    0.56 * (1.0 - rib),
                    0.49 * rib,
                )
            }
            Apron => (
                self.toe * (0.77 + 0.16 * shape + 0.08 * bank),
                0.0,
                0.44 * bank,
            ),
        };
        let coverage = smoothstep(along * 8.5)
            * smoothstep((toe - along) * 9.0)
            * smoothstep((1.0 - wing) * 3.8)
            * (1.0 - channel);
        let deposit =
            coverage * smoothstep((coverage - 0.10) * 3.7) * (0.46 + along * 1.19 + lumps);
        let coarse = along + 0.14 * wing * wing + 0.39 * lumps - 0.11 * coverage > 0.55;
        (coverage, deposit, coarse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn talus_aprons_have_distinct_outlines_inside_the_surveyed_envelope() {
        let mut seen = [0_u64; TalusProfile::ALL.len()];
        for (i, profile) in TalusProfile::ALL.into_iter().enumerate() {
            let mut count = 0;
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            for row in 0..=60 {
                for col in -40..=40 {
                    let (cover, height, _) = profile.study(row as f32 / 60.0, col as f32 / 40.0);
                    assert!((0.0..=1.0).contains(&cover) && (0.0..=2.3).contains(&height));
                    if row == 0 || row == 60 || col == -40 || col == 40 {
                        assert_eq!(cover, 0.0);
                    }
                    count += usize::from(cover > 0.48);
                    hash = (hash ^ u64::from(cover.to_bits())).wrapping_mul(0x100_0000_01b3);
                }
            }
            assert!(count > 240, "{}", profile.name);
            assert!(!seen[..i].contains(&hash), "{}", profile.name);
            seen[i] = hash;
        }
    }
}
