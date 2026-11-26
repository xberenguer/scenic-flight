//! Cross sections for the already reserved shoulders of a surveyed railway.

use crate::math::smoothstep;

const SECTION_LENGTH: f32 = 68.0;
const TRANSITION_LENGTH: f32 = 18.0;

/// The two knees mark where the first face ends and the outer slope begins.
/// Distances use the corridor's normalized squared offset from the trackbed.
#[derive(Clone, Copy)]
struct Shoulder {
    inner: f32,
    outer: f32,
    bench: f32,
}

impl Shoulder {
    const fn new(inner: f32, outer: f32, bench: f32) -> Self {
        Self {
            inner,
            outer,
            bench,
        }
    }

    fn weight(self, across: f32) -> f32 {
        let face = 1.0 - smoothstep(across / self.inner);
        let toe = 1.0 - smoothstep((across - self.outer) / (1.0 - self.outer));
        (1.0 - self.bench) * face + self.bench * toe
    }
}

#[derive(Clone, Copy)]
pub struct EarthworkProfile {
    pub name: &'static str,
    cut: Shoulder,
    fill: Shoulder,
}

const ROCK: Shoulder = Shoulder::new(0.19, 0.24, 0.15);
const LEDGE: Shoulder = Shoulder::new(0.25, 0.69, 0.37);
const BENCH: Shoulder = Shoulder::new(0.34, 0.54, 0.55);
const DITCH: Shoulder = Shoulder::new(0.62, 0.73, 0.72);
const SCARP: Shoulder = Shoulder::new(0.45, 0.86, 0.28);

const RIPRAP: Shoulder = Shoulder::new(0.18, 0.31, 0.64);
const BERM: Shoulder = Shoulder::new(0.37, 0.78, 0.48);
const TERRACE: Shoulder = Shoulder::new(0.31, 0.56, 0.23);
const FAN: Shoulder = Shoulder::new(0.72, 0.58, 0.35);
const BATTER: Shoulder = Shoulder::new(0.89, 0.36, 0.43);

impl EarthworkProfile {
    pub const ALL: [Self; 25] = [
        Self::new("Rock notch and riprap", ROCK, RIPRAP),
        Self::new("Rock notch and toe berm", ROCK, BERM),
        Self::new("Rock notch and gravel terrace", ROCK, TERRACE),
        Self::new("Rock notch and spread apron", ROCK, FAN),
        Self::new("Rock notch and long batter", ROCK, BATTER),
        Self::new("High ledge and riprap", LEDGE, RIPRAP),
        Self::new("High ledge and toe berm", LEDGE, BERM),
        Self::new("High ledge and gravel terrace", LEDGE, TERRACE),
        Self::new("High ledge and spread apron", LEDGE, FAN),
        Self::new("High ledge and long batter", LEDGE, BATTER),
        Self::new("Benched cutting and riprap", BENCH, RIPRAP),
        Self::new("Benched cutting and toe berm", BENCH, BERM),
        Self::new("Benched cutting and gravel terrace", BENCH, TERRACE),
        Self::new("Benched cutting and spread apron", BENCH, FAN),
        Self::new("Benched cutting and long batter", BENCH, BATTER),
        Self::new("Drainage shelf and riprap", DITCH, RIPRAP),
        Self::new("Drainage shelf and toe berm", DITCH, BERM),
        Self::new("Drainage shelf and gravel terrace", DITCH, TERRACE),
        Self::new("Drainage shelf and spread apron", DITCH, FAN),
        Self::new("Drainage shelf and long batter", DITCH, BATTER),
        Self::new("Outer scarp and riprap", SCARP, RIPRAP),
        Self::new("Outer scarp and toe berm", SCARP, BERM),
        Self::new("Outer scarp and gravel terrace", SCARP, TERRACE),
        Self::new("Outer scarp and spread apron", SCARP, FAN),
        Self::new("Outer scarp and long batter", SCARP, BATTER),
    ];

    const fn new(name: &'static str, cut: Shoulder, fill: Shoulder) -> Self {
        Self { name, cut, fill }
    }

    pub fn weight(self, across: f32, cutting: bool) -> f32 {
        let shoulder = if cutting { self.cut } else { self.fill };
        shoulder.weight(across)
    }

    pub(crate) fn route_weight(offset: usize, distance: f32, across: f32, cutting: bool) -> f32 {
        let segment = (distance / SECTION_LENGTH) as usize;
        let index = (offset + segment * 7) % Self::ALL.len();
        let current = Self::ALL[index].weight(across, cutting);
        let into = distance - segment as f32 * SECTION_LENGTH;
        if into <= SECTION_LENGTH - TRANSITION_LENGTH {
            return current;
        }
        let next = Self::ALL[(index + 7) % Self::ALL.len()].weight(across, cutting);
        let factor = smoothstep((into - SECTION_LENGTH + TRANSITION_LENGTH) / TRANSITION_LENGTH);
        current + (next - current) * factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_and_fill_layouts_are_distinct_and_stay_inside_the_shoulder() {
        let mut signatures = [[0; 16]; 25];
        for (i, profile) in EarthworkProfile::ALL.into_iter().enumerate() {
            for (side, cutting) in [true, false].into_iter().enumerate() {
                let mut previous = 1.0;
                for step in 0..=64 {
                    let weight = profile.weight(step as f32 / 64.0, cutting);
                    assert!((0.0..=previous + 0.000_001).contains(&weight));
                    if step % 8 == 4 {
                        signatures[i][8 * side + step / 8] = (weight * 1000.0) as u16;
                    }
                    previous = weight;
                }
                assert_eq!(profile.weight(0.0, cutting), 1.0);
                assert_eq!(previous, 0.0);
            }
            assert!(!signatures[..i].contains(&signatures[i]));
        }
    }

    #[test]
    fn profiles_blend_continuously_between_route_sections() {
        for segment in 1..=25 {
            let distance = segment as f32 * SECTION_LENGTH;
            for cutting in [true, false] {
                for across in [0.08, 0.33, 0.61, 0.92] {
                    let before =
                        EarthworkProfile::route_weight(4, distance - 0.05, across, cutting);
                    let after = EarthworkProfile::route_weight(4, distance, across, cutting);
                    assert!((before - after).abs() < 0.000_1);
                }
            }
        }
    }
}
