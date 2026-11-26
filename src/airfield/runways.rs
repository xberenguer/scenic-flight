//! Game-scaled landing markings inside the surveyed runway rectangles.

use super::{AirfieldKind, AirfieldMaterial, Direction};
use core::f32::consts::TAU;

#[derive(Clone, Copy)]
enum Scheme {
    GrassBoundary,
    GrassDisplaced,
    Basic,
    Aiming,
    Displaced,
}

#[derive(Clone, Copy)]
pub struct RunwayProfile {
    pub name: &'static str,
    pub kind: AirfieldKind,
    scheme: Scheme,
    offset: f32,
    bars: u8,
    dash: f32,
}

use AirfieldKind::{Country, Regional};
use Scheme::*;

impl RunwayProfile {
    pub const ALL: [Self; 25] = [
        Self::new("Grass corner markers", Country, GrassBoundary, 1.0, 1, 0.0),
        Self::new(
            "Grass paired threshold boards",
            Country,
            GrassBoundary,
            2.9,
            2,
            0.0,
        ),
        Self::new(
            "Grass continuous threshold band",
            Country,
            GrassBoundary,
            1.7,
            3,
            0.0,
        ),
        Self::new(
            "Grass T and edge markers",
            Country,
            GrassBoundary,
            1.0,
            4,
            15.0,
        ),
        Self::new(
            "Grass threshold with inset edges",
            Country,
            GrassBoundary,
            3.8,
            2,
            13.0,
        ),
        Self::new(
            "Grass displaced threshold arrow",
            Country,
            GrassDisplaced,
            4.5,
            1,
            0.0,
        ),
        Self::new(
            "Grass twin arrows and bars",
            Country,
            GrassDisplaced,
            7.5,
            2,
            0.0,
        ),
        Self::new(
            "Grass inset landing gate",
            Country,
            GrassDisplaced,
            6.2,
            3,
            10.0,
        ),
        Self::new(
            "Grass extended lead-in arrow",
            Country,
            GrassDisplaced,
            9.0,
            4,
            0.0,
        ),
        Self::new(
            "Grass wide displaced threshold",
            Country,
            GrassDisplaced,
            6.8,
            3,
            13.5,
        ),
        Self::new(
            "Paved threshold and centerline",
            Regional,
            Basic,
            2.4,
            2,
            12.0,
        ),
        Self::new(
            "Paved threshold with edge stripes",
            Regional,
            Basic,
            3.2,
            3,
            15.0,
        ),
        Self::new(
            "Paved long centerline dashes",
            Regional,
            Basic,
            2.1,
            2,
            18.0,
        ),
        Self::new("Paved broad threshold bars", Regional, Basic, 3.6, 4, 13.0),
        Self::new(
            "Paved compact threshold bars",
            Regional,
            Basic,
            4.1,
            2,
            10.0,
        ),
        Self::new(
            "Aiming blocks and edge stripes",
            Regional,
            Aiming,
            2.5,
            3,
            12.0,
        ),
        Self::new(
            "Aiming blocks and short touchdown bars",
            Regional,
            Aiming,
            3.7,
            2,
            12.5,
        ),
        Self::new(
            "Aiming blocks and paired touchdown zones",
            Regional,
            Aiming,
            4.2,
            4,
            15.4,
        ),
        Self::new(
            "Aiming blocks behind a wide threshold",
            Regional,
            Aiming,
            2.1,
            4,
            16.5,
        ),
        Self::new(
            "Aiming blocks and long lead-in dashes",
            Regional,
            Aiming,
            3.1,
            3,
            19.0,
        ),
        Self::new(
            "Paved displaced threshold",
            Regional,
            Displaced,
            8.5,
            2,
            12.0,
        ),
        Self::new(
            "Paved displaced threshold and edge stripes",
            Regional,
            Displaced,
            10.3,
            3,
            15.0,
        ),
        Self::new(
            "Paved arrow lead-in and aiming blocks",
            Regional,
            Displaced,
            12.8,
            2,
            13.3,
        ),
        Self::new(
            "Paved double-arrow threshold",
            Regional,
            Displaced,
            14.2,
            4,
            15.0,
        ),
        Self::new(
            "Paved long displaced threshold",
            Regional,
            Displaced,
            16.4,
            4,
            18.4,
        ),
    ];

    const fn new(
        name: &'static str,
        kind: AirfieldKind,
        scheme: Scheme,
        offset: f32,
        bars: u8,
        dash: f32,
    ) -> Self {
        Self {
            name,
            kind,
            scheme,
            offset,
            bars,
            dash,
        }
    }

    pub(super) fn choose(kind: AirfieldKind, key: u64) -> u8 {
        match kind {
            Country => (key % 10) as u8,
            Regional => (10 + key % 15) as u8,
        }
    }

    pub fn sample(self, end: f32, across: f32, width: f32, runway: u8) -> AirfieldMaterial {
        use AirfieldMaterial::{EdgeLight, GrassRunway, PavedRunway, RunwayMarking};
        let side = across.abs();
        let regional = self.kind == Regional;
        let base = if regional { PavedRunway } else { GrassRunway };
        if end < 0.0 || side > width {
            return base;
        }
        if regional && end > 1.0 && side >= width - 0.56 && end % 11.2 < 1.1 {
            return EdgeLight;
        }
        let from_threshold = end - self.offset;
        let displaced = matches!(self.scheme, GrassDisplaced | Displaced);
        if displaced && from_threshold < 0.0 {
            let along = (-from_threshold) % 5.95;
            return if (0.7..=4.0).contains(&along) && (side - (1.55 - 0.37 * along)).abs() < 0.36 {
                RunwayMarking
            } else {
                base
            };
        }
        if (0.0..1.4).contains(&from_threshold)
            && (displaced || (self.bars >= 3 && !regional))
            && side < width - 0.47
        {
            return RunwayMarking;
        }
        if matches!(self.scheme, GrassBoundary)
            && self.bars == 1
            && ((0.0..5.1).contains(&from_threshold)
                && (width - 0.98..width - 0.41).contains(&side)
                || (1.4..2.2).contains(&from_threshold)
                    && (width - 1.75..width - 0.41).contains(&side))
        {
            return RunwayMarking;
        }
        let comb = (0.9..width - 0.54).contains(&side)
            && (self.bars == 1 && side > width - 1.17
                || self.bars == 2 && (side < 1.83 || side > width - 1.25)
                || self.bars >= 3
                    && ((side - 0.87) / ((width - 1.41) / f32::from(self.bars))) % 1.0 < 0.56);
        if comb && (1.65..if regional { 5.3 } else { 3.7 }).contains(&from_threshold) {
            return RunwayMarking;
        }
        if !regional {
            if (self.dash > 0.0 && side > width - 0.48 && (end + 2.0) % self.dash < 1.15)
                || (self.bars == 4 && side < 0.47 && (2.0..5.1).contains(&from_threshold))
            {
                return RunwayMarking;
            }
            return base;
        }
        if let Some((row, col)) = digit_cell(from_threshold, across) {
            if digit_light(runway, row, col) {
                return RunwayMarking;
            }
        }
        let aim = matches!(self.scheme, Aiming) || displaced && self.offset >= 12.0;
        let striking = aim && (19.0..22.7).contains(&from_threshold) && (1.6..3.4).contains(&side);
        let touchdown = matches!(self.scheme, Aiming)
            && self.bars >= 2
            && (26.4..if self.bars >= 4 { 38.8 } else { 31.0 }).contains(&from_threshold)
            && from_threshold % 8.5 < 3.0
            && ((1.6..2.05).contains(&side) || (2.75..3.2).contains(&side));
        let stripe =
            self.bars >= 3 && from_threshold >= 2.0 && (width - 1.12..width - 0.82).contains(&side);
        let center = from_threshold > 14.6
            && side < 0.47
            && (from_threshold - 14.6) % self.dash < self.dash * 0.46;
        if striking || touchdown || stripe || center {
            RunwayMarking
        } else {
            base
        }
    }
}

fn digit_cell(end: f32, across: f32) -> Option<(usize, usize)> {
    if !(7.0..13.5).contains(&end) || !(-2.45..2.45).contains(&across) {
        return None;
    }
    Some((
        ((13.5 - end) * (5.0 / 6.5)) as usize,
        ((across + 2.45) * (7.0 / 4.90)) as usize,
    ))
}

fn digit_light(runway: u8, row: usize, col: usize) -> bool {
    const DIGITS: [[u8; 5]; 10] = [
        [7, 5, 5, 5, 7],
        [2, 6, 2, 2, 7],
        [7, 1, 7, 4, 7],
        [7, 1, 7, 1, 7],
        [5, 5, 7, 1, 1],
        [7, 4, 7, 1, 7],
        [7, 4, 7, 5, 7],
        [7, 1, 1, 1, 1],
        [7, 5, 7, 5, 7],
        [7, 5, 7, 1, 7],
    ];
    if row >= 5 || col >= 7 || col == 3 {
        return false;
    }
    let digit = if col < 3 { runway / 10 } else { runway % 10 };
    (DIGITS.get(usize::from(digit)).map_or(0, |glyph| glyph[row]) & (4 >> (col % 4))) != 0
}

pub(super) fn reciprocal(number: u8) -> u8 {
    (number + 17) % 36 + 1
}

pub(super) fn number(direction: Direction) -> u8 {
    const COMPASS: [Direction; 36] = {
        let mut compass = [Direction { fx: 0.0, fz: 1.0 }; 36];
        let mut i = 0;
        while i < 36 {
            let (fx, fz) = crate::math::sin_cos(i as f32 * (TAU / 36.0));
            compass[i] = Direction { fx, fz };
            i += 1;
        }
        compass
    };
    let mut closest = 0;
    let mut score = f32::NEG_INFINITY;
    for (i, bearing) in COMPASS.into_iter().enumerate() {
        let dot = direction.fx * bearing.fx + direction.fz * bearing.fz;
        if dot > score {
            closest = i;
            score = dot;
        }
    }
    if closest == 0 { 36 } else { closest as u8 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runway_designators_follow_the_bearing_and_its_reciprocal() {
        for step in 0..36 {
            let bearing = (step as f32 + 0.44) * (TAU / 36.0);
            let expected = if step == 0 { 36 } else { step };
            assert_eq!(number(Direction::at(bearing)), expected);
            assert_eq!(
                number(Direction::at(bearing + core::f32::consts::PI)),
                reciprocal(expected)
            );
        }
        assert!(digit_light(9, 0, 0) && digit_light(9, 0, 4) && !digit_light(9, 0, 3));
    }

    #[test]
    fn marking_designs_are_distinct_and_keep_the_runway_footprint() {
        let mut signatures = [0; RunwayProfile::ALL.len()];
        for (i, profile) in RunwayProfile::ALL.into_iter().enumerate() {
            let width = if profile.kind == Country { 3.7 } else { 5.5 };
            let mut signature = 0xcbf2_9ce4_8422_2325_u64;
            let mut painted = 0;
            for y in 0..180 {
                for x in -22..=22 {
                    let position = y as f32 * 0.25;
                    let offset = x as f32 * 0.25;
                    let sample = profile.sample(position, offset, width, 9);
                    let value = match sample {
                        AirfieldMaterial::RunwayMarking => 1,
                        AirfieldMaterial::EdgeLight => 2,
                        _ => 0,
                    };
                    if offset.abs() > width {
                        assert_eq!(value, 0);
                    }
                    painted += usize::from(value == 1);
                    signature = (signature ^ value).wrapping_mul(0x100_0000_01b3);
                }
            }
            assert!(painted > 55, "{}", profile.name);
            assert!(!signatures[..i].contains(&signature), "{}", profile.name);
            signatures[i] = signature;
        }
    }
}
