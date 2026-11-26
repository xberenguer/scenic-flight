//! Bluffs attach landward of the measured shoreline and fade into native ground.

use super::BACK;
use crate::math::smoothstep;

#[derive(Clone, Copy)]
enum Form {
    Ramp,
    Cleft,
    Shelves,
    Promontory,
    Slump,
}

#[derive(Clone, Copy)]
pub struct BluffProfile {
    pub name: &'static str,
    form: Form,
    setback: f32,
    length: f32,
    shift: f32,
}

use Form::*;

impl BluffProfile {
    pub const ALL: [Self; 25] = [
        Self::new("Broad grass-capped ramp", Ramp, 0.2, 1.03, 0.00),
        Self::new("Short steep seaward brow", Ramp, 1.5, 0.94, -0.18),
        Self::new("Long lee ramp", Ramp, 2.8, 0.77, 0.25),
        Self::new("Skewed rock brow", Ramp, 0.9, 1.04, -0.32),
        Self::new("Wide sloping headland", Ramp, 3.6, 0.83, 0.11),
        Self::new("Central erosion cleft", Cleft, 0.3, 0.86, 0.00),
        Self::new("Offset cleft and wide cap", Cleft, 1.1, 0.91, 0.31),
        Self::new("Narrow left cleft", Cleft, 1.8, 1.00, -0.32),
        Self::new("Long slot between low bluffs", Cleft, 2.9, 0.79, 0.17),
        Self::new("Wide notch and short cap", Cleft, 0.7, 1.06, -0.20),
        Self::new("Two raised shore shelves", Shelves, 0.7, 0.91, 0.02),
        Self::new("Shallow shelf and upper step", Shelves, 2.7, 0.84, 0.20),
        Self::new("Broad lower marine terrace", Shelves, 0.4, 0.78, -0.19),
        Self::new("Narrow paired terraces", Shelves, 1.8, 0.96, -0.09),
        Self::new("Skewed shelf and high shoulder", Shelves, 3.2, 1.01, 0.29),
        Self::new("Twin rock noses", Promontory, 1.2, 0.96, 0.00),
        Self::new("Long left rock spur", Promontory, 0.8, 1.03, -0.31),
        Self::new(
            "Right nose with shallow saddle",
            Promontory,
            2.7,
            0.89,
            0.30,
        ),
        Self::new("Shallow twin promontories", Promontory, 3.5, 0.76, -0.11),
        Self::new(
            "Blunted spur and narrow shoulder",
            Promontory,
            1.9,
            0.87,
            0.13,
        ),
        Self::new("Rotated coastal slump", Slump, 0.5, 0.88, 0.04),
        Self::new("Broad slump toe", Slump, 2.4, 0.83, -0.20),
        Self::new("Steep slump above a narrow toe", Slump, 1.3, 1.00, 0.23),
        Self::new("Hummock and rounded shelf", Slump, 1.9, 0.95, -0.31),
        Self::new("Low double slump", Slump, 3.0, 0.79, 0.30),
    ];

    const fn new(name: &'static str, form: Form, setback: f32, length: f32, shift: f32) -> Self {
        Self {
            name,
            form,
            setback,
            length,
            shift,
        }
    }

    /// Normalized rise, end of the exposed rock, and end of the grass cap.
    /// The rise is supported only within the surveyed span and inland depth.
    pub fn study(self, along: f32, inland: f32, oblique: f32) -> (f32, f32, f32) {
        if along.abs() >= 1.0 || !(2.5..BACK).contains(&inland) {
            return (0.0, 0.0, 0.0);
        }
        let center = along - self.shift;
        let rounded = 1.0 - smoothstep(center.abs() * 0.82);
        let flank = smoothstep((1.0 - along.abs()) * 1.95);
        let start = (2.5 + 3.8 * along * along + oblique * along).max(2.5) + self.setback;
        let depth = inland - start;
        let fade = smoothstep((BACK - inland) * 0.024);
        let crest = smoothstep(depth * (0.16 * self.length));
        let (rise, stone, cap) = match self.form {
            Ramp => (crest * (0.88 + 0.12 * rounded), 0.0, 0.0),
            Cleft => {
                let notch = 1.0 - smoothstep(center.abs() * 2.9);
                let bite = notch * (1.0 - smoothstep((depth - 17.0) * 0.052));
                (crest * (1.0 - 0.32 * bite), -3.0 * bite, 3.0 * notch)
            }
            Shelves => {
                let lower = smoothstep(depth * (0.15 * self.length));
                let upper = smoothstep((depth - 11.0 - 4.5 * center) * 0.10);
                (0.53 * lower + 0.47 * upper, 2.7 + 2.0 * center, 6.0)
            }
            Promontory => {
                let nose = 1.0 - smoothstep((center.abs() - 0.38).abs() * 2.1);
                (crest * (0.78 + 0.22 * nose), 4.3 * nose, -3.6 * rounded)
            }
            Slump => {
                let hummock = rounded
                    * smoothstep((depth - 1.0) * 0.10)
                    * (1.0 - smoothstep((depth - 13.0) * 0.084));
                (0.84 * crest + 0.16 * hummock, 3.7 * hummock, 3.9 * rounded)
            }
        };
        let rock_edge = 13.5
            + 4.0 * (1.0 - along * along)
            + along * oblique * 3.0
            + self.setback * 0.43
            + stone;
        (rise * flank * fade, rock_edge, 36.0 + cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bluff_shapes_are_distinct_and_blend_to_surrounding_ground() {
        let mut signatures = [0; BluffProfile::ALL.len()];
        for (i, profile) in BluffProfile::ALL.into_iter().enumerate() {
            let mut signature = 0xcbf2_9ce4_8422_2325_u64;
            let mut rocky_shoulder = 0;
            for row in -28_i32..=28 {
                let along = row as f32 / 28.0;
                let mut previous = 0.0;
                for column in 0..=65 {
                    let (rise, rock_edge, _) = profile.study(along, column as f32, 0.0);
                    assert!((0.0..=1.0).contains(&rise));
                    assert!((rise - previous).abs() < 0.29, "{}", profile.name);
                    if row.abs() == 28 || column <= 2 || column == 65 {
                        assert_eq!(rise, 0.0);
                    }
                    rocky_shoulder += usize::from(rise > 0.32 && (column as f32) < rock_edge);
                    signature =
                        (signature ^ u64::from(rise.to_bits())).wrapping_mul(0x100_0000_01b3);
                    previous = rise;
                }
            }
            assert!(rocky_shoulder > 140, "{}", profile.name);
            assert!(!signatures[..i].contains(&signature), "{}", profile.name);
            signatures[i] = signature;
        }
    }
}
