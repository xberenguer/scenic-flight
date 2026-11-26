use crate::math::smoothstep;
use crate::world::Surface;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TarnShoreline(u8);

pub(super) struct Profile {
    name: &'static str,
    pub drawdown: i16,
    // Shelf depth toward the two ends and the two shores, in world units.
    shelf: [f32; 4],
    form: Form,
    inner: Surface,
    outer: Surface,
    relief: f32,
}

#[derive(Clone, Copy)]
enum Form {
    Strand,
    Cove,
    Paired,
    Fan,
    Apron,
    Swale,
    Ridge,
    Lobe,
}

impl TarnShoreline {
    pub const COUNT: usize = PROFILES.len();

    pub fn from_index(index: usize) -> Option<Self> {
        (index < Self::COUNT).then_some(Self(index as u8))
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    pub(super) fn choose(key: u64) -> Self {
        Self((key % Self::COUNT as u64) as u8)
    }

    pub(super) fn spec(self) -> &'static Profile {
        &PROFILES[self.index()]
    }
}

impl Profile {
    /// Blend the surveyed shelves so the transitions stay attached to the shore.
    pub(super) fn shallow_depth(&self, [along, across]: [f32; 2]) -> f32 {
        let [head, toe, left, right] = self.shelf;
        let a = head + (toe - head) * (along * 0.5 + 0.5).clamp(0.0, 1.0);
        let b = left + (right - left) * (across * 0.5 + 0.5).clamp(0.0, 1.0);
        let share = across.abs() / (along.abs() + across.abs()).max(0.06);
        a + (b - a) * share
    }

    pub(super) fn bench(&self, [u, v]: [f32; 2], rise: f32) -> (Surface, f32) {
        let track = match self.form {
            Form::Strand => 1.05 - rise * 0.51 + u * 0.18,
            Form::Cove => 1.04 - (u + 0.62).abs() * 1.15 - (v - 0.42).abs() * 0.43,
            Form::Paired => 1.02 - (u.abs() - 0.76).abs() * 1.40 - rise * 0.18,
            Form::Fan => 1.13 - (u - 0.60).abs() * 0.50 - (v * (0.72 - u * 0.18)).abs(),
            Form::Apron => 1.10 - rise * 0.22 + u * 0.28,
            Form::Swale => 1.18 - (v - 0.17 * u * u + 0.24).abs() * 1.03 - rise * 0.19,
            Form::Ridge => 1.05 - (rise - 2.0 - 0.65 * u).abs() * 0.58,
            Form::Lobe => 1.15 - (u - 0.48).abs() * 0.85 - (v + 0.42).abs() * 0.52,
        };
        let weight = smoothstep(track * 1.60);
        let material = if weight > 0.66 {
            self.inner
        } else if weight > 0.34 {
            self.outer
        } else {
            Surface::AlpineMeadow
        };
        let relief = if material == Surface::AlpineMeadow {
            0.0
        } else {
            self.relief * weight * smoothstep((rise - 0.30) * 0.85) * smoothstep((6.7 - rise) * 0.8)
        };
        (material, relief)
    }
}

use Form::{Apron, Cove, Fan, Lobe, Paired, Ridge, Strand, Swale};
use Surface::{AlpineMeadow, Gravel, ReedMarsh, Rock, Sand, ScreeFine, WetMeadow};

macro_rules! profile {
    ($name:literal, $draw:literal, $shelf:expr, $form:ident, $inner:ident, $outer:ident, $relief:literal) => {
        Profile {
            name: $name,
            drawdown: $draw,
            shelf: $shelf,
            form: $form,
            inner: $inner,
            outer: $outer,
            relief: $relief,
        }
    };
}

const PROFILES: [Profile; 25] = [
    profile!(
        "Tarn with a broad sedge strand",
        16,
        [1.8, 1.3, 0.75, 0.9],
        Strand,
        WetMeadow,
        AlpineMeadow,
        0.0
    ),
    profile!(
        "Sheltered reed cove",
        22,
        [2.1, 0.52, 0.64, 1.32],
        Cove,
        ReedMarsh,
        WetMeadow,
        0.0
    ),
    profile!(
        "Sedge beds at both ends",
        10,
        [1.92, 1.81, 0.56, 0.60],
        Paired,
        ReedMarsh,
        WetMeadow,
        0.0
    ),
    profile!(
        "Outflow sedge fan",
        18,
        [0.55, 2.23, 0.92, 0.72],
        Fan,
        WetMeadow,
        AlpineMeadow,
        0.0
    ),
    profile!(
        "Low green shore swale",
        26,
        [1.26, 1.23, 1.98, 0.49],
        Swale,
        ReedMarsh,
        WetMeadow,
        0.0
    ),
    profile!(
        "Pale sand strand",
        33,
        [1.35, 1.51, 0.56, 1.95],
        Strand,
        Sand,
        Gravel,
        0.08
    ),
    profile!(
        "Sand cove under a grassy bench",
        23,
        [1.96, 0.72, 0.53, 1.32],
        Cove,
        Sand,
        AlpineMeadow,
        0.08
    ),
    profile!(
        "Two shallow sand shelves",
        39,
        [1.87, 1.78, 0.58, 0.63],
        Paired,
        Sand,
        Gravel,
        0.10
    ),
    profile!(
        "Drained sand fan",
        44,
        [0.62, 2.35, 0.81, 0.91],
        Fan,
        Sand,
        Gravel,
        0.14
    ),
    profile!(
        "Sand apron above a blue pool",
        37,
        [0.82, 1.94, 1.20, 0.63],
        Apron,
        Sand,
        AlpineMeadow,
        0.10
    ),
    profile!(
        "Low shingle strand",
        12,
        [0.85, 1.26, 0.73, 1.52],
        Strand,
        Gravel,
        ScreeFine,
        0.22
    ),
    profile!(
        "Sheltered gravel pocket",
        20,
        [1.52, 0.60, 0.60, 1.09],
        Cove,
        Gravel,
        AlpineMeadow,
        0.24
    ),
    profile!(
        "Paired gravel shoulders",
        5,
        [1.55, 1.26, 0.59, 0.57],
        Paired,
        Gravel,
        ScreeFine,
        0.26
    ),
    profile!(
        "Outwash gravel fan",
        25,
        [0.56, 2.02, 0.91, 0.73],
        Fan,
        Gravel,
        WetMeadow,
        0.29
    ),
    profile!(
        "Winding shingle berm",
        31,
        [1.40, 0.77, 1.35, 0.52],
        Ridge,
        Gravel,
        ScreeFine,
        0.40
    ),
    profile!(
        "Exposed stone strand",
        -8,
        [0.79, 1.12, 0.62, 0.76],
        Strand,
        Rock,
        ScreeFine,
        0.34
    ),
    profile!(
        "Stone ridge above a deep tarn",
        -16,
        [0.55, 0.70, 0.49, 0.88],
        Ridge,
        Rock,
        Gravel,
        0.44
    ),
    profile!(
        "High rock outwash lobe",
        5,
        [0.73, 1.64, 1.21, 0.54],
        Lobe,
        Rock,
        ScreeFine,
        0.45
    ),
    profile!(
        "Stone lobes at both ends",
        -10,
        [0.80, 0.91, 0.48, 0.57],
        Paired,
        Rock,
        Gravel,
        0.42
    ),
    profile!(
        "Moraine apron at a tarn",
        -4,
        [0.57, 1.44, 1.01, 0.64],
        Apron,
        ScreeFine,
        Rock,
        0.37
    ),
    profile!(
        "Dry meadow above a wet strand",
        0,
        [1.06, 1.49, 0.89, 1.18],
        Strand,
        WetMeadow,
        AlpineMeadow,
        0.0
    ),
    profile!(
        "Wide alpine meadow apron",
        -12,
        [0.64, 1.48, 0.88, 0.65],
        Apron,
        AlpineMeadow,
        WetMeadow,
        0.10
    ),
    profile!(
        "Green cove with a stone edge",
        -6,
        [1.50, 0.58, 0.79, 1.32],
        Cove,
        WetMeadow,
        Gravel,
        0.14
    ),
    profile!(
        "Wet swale between shingle banks",
        19,
        [1.12, 0.83, 1.65, 0.45],
        Swale,
        WetMeadow,
        Gravel,
        0.12
    ),
    profile!(
        "Meadow lobe with a gravel rim",
        14,
        [0.69, 1.59, 1.73, 0.57],
        Lobe,
        AlpineMeadow,
        Gravel,
        0.20
    ),
];
