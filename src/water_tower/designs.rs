type Color = [u8; 3];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaterTowerDesign(u8);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Support {
    Pedestal,
    Frame,
    Masonry,
    Twin,
    Portal,
}

pub(super) struct Design {
    name: &'static str,
    pub support: Support,
    pub height: f32,
    pub depth: f32,
    pub radius: f32,
    /// Tank radius at six evenly placed levels from vessel bottom to roof.
    pub profile: [f32; 6],
    pub angular: bool,
    pub paint: [Color; 3],
}

impl WaterTowerDesign {
    pub const COUNT: usize = DESIGNS.len();

    pub fn from_index(index: usize) -> Option<Self> {
        (index < Self::COUNT).then_some(Self(index as u8))
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    pub(super) fn select(key: u64) -> Self {
        Self((key % Self::COUNT as u64) as u8)
    }

    pub(super) fn spec(self) -> &'static Design {
        &DESIGNS[self.index()]
    }
}

use Support::{Frame, Masonry, Pedestal, Portal, Twin};

const WHITE: [Color; 3] = [[218, 222, 213], [119, 148, 163], [150, 159, 158]];
const CREAM: [Color; 3] = [[230, 214, 176], [157, 116, 78], [156, 151, 130]];
const BLUE: [Color; 3] = [[146, 186, 203], [76, 116, 142], [147, 158, 165]];
const GREEN: [Color; 3] = [[154, 183, 160], [86, 122, 106], [138, 154, 147]];
const BRICK: [Color; 3] = [[176, 146, 124], [102, 96, 93], [155, 113, 94]];

const BULB: [f32; 6] = [0.28, 0.67, 1.0, 1.0, 0.80, 0.22];
const DRUM: [f32; 6] = [0.73, 0.99, 1.0, 1.0, 1.0, 0.86];
const SAUCER: [f32; 6] = [0.25, 0.59, 0.98, 1.0, 0.68, 0.23];
const CAPSULE: [f32; 6] = [0.50, 0.90, 1.0, 1.0, 0.94, 0.50];
const GABLED: [f32; 6] = [0.88, 1.0, 1.0, 1.0, 0.85, 0.15];
const INVERTED: [f32; 6] = [0.31, 0.52, 0.63, 0.82, 1.0, 0.94];
const DOME: [f32; 6] = [0.76, 0.95, 1.0, 1.0, 0.83, 0.32];
const OCTAGON: [f32; 6] = [0.51, 0.77, 1.0, 1.0, 0.90, 0.72];

macro_rules! design {
    ($name:literal, $support:ident, $height:literal, $depth:literal, $radius:literal,
      $outline:ident, $angular:literal, $paint:ident) => {
        Design {
            name: $name,
            support: $support,
            height: $height,
            depth: $depth,
            radius: $radius,
            profile: $outline,
            angular: $angular,
            paint: $paint,
        }
    };
}

const DESIGNS: [Design; 25] = [
    design!(
        "Blue-band pedestal bulb",
        Pedestal,
        31.5,
        9.4,
        7.6,
        BULB,
        false,
        WHITE
    ),
    design!(
        "Low cream saucer tower",
        Pedestal,
        24.1,
        5.8,
        7.6,
        SAUCER,
        false,
        CREAM
    ),
    design!(
        "Tall tapered tulip tank",
        Pedestal,
        36.7,
        11.2,
        6.8,
        INVERTED,
        false,
        BLUE
    ),
    design!(
        "Green oval capsule",
        Pedestal,
        29.7,
        11.4,
        6.2,
        CAPSULE,
        false,
        GREEN
    ),
    design!(
        "Wide pedestal drum",
        Pedestal,
        26.9,
        7.1,
        7.7,
        DRUM,
        false,
        WHITE
    ),
    design!(
        "Cream four-legged drum",
        Frame,
        28.7,
        8.8,
        6.9,
        DRUM,
        false,
        CREAM
    ),
    design!(
        "Steel frame with a tall capsule",
        Frame,
        35.9,
        12.3,
        6.1,
        CAPSULE,
        false,
        BLUE
    ),
    design!(
        "Open frame with a faceted roof",
        Frame,
        31.1,
        9.7,
        6.4,
        GABLED,
        true,
        GREEN
    ),
    design!(
        "Tall frame with a shallow saucer",
        Frame,
        37.8,
        6.8,
        7.45,
        SAUCER,
        false,
        WHITE
    ),
    design!(
        "Short braced frame with a domed tank",
        Frame,
        24.3,
        8.7,
        7.4,
        DOME,
        false,
        CREAM
    ),
    design!(
        "Brick standpipe with a peaked roof",
        Masonry,
        29.9,
        9.0,
        5.6,
        GABLED,
        true,
        BRICK
    ),
    design!(
        "Square pump house and flat tank",
        Masonry,
        23.6,
        7.0,
        6.2,
        DRUM,
        true,
        CREAM
    ),
    design!(
        "Tall masonry shaft with a copper dome",
        Masonry,
        35.2,
        8.1,
        5.75,
        DOME,
        false,
        GREEN
    ),
    design!(
        "Stone column with a stepped tank",
        Masonry,
        31.6,
        10.3,
        5.75,
        OCTAGON,
        true,
        BRICK
    ),
    design!(
        "Wide brick base and spherical crest",
        Masonry,
        26.2,
        10.7,
        6.5,
        BULB,
        false,
        WHITE
    ),
    design!(
        "Twin legs with a broad reservoir",
        Twin,
        26.7,
        7.4,
        7.65,
        DRUM,
        true,
        BLUE
    ),
    design!(
        "Twin-column capsule tank",
        Twin,
        32.6,
        10.6,
        6.4,
        CAPSULE,
        false,
        WHITE
    ),
    design!(
        "Tall twin-column conical tank",
        Twin,
        36.1,
        11.7,
        6.2,
        INVERTED,
        true,
        CREAM
    ),
    design!(
        "Low twin-column saucer",
        Twin,
        24.8,
        6.7,
        7.25,
        SAUCER,
        false,
        GREEN
    ),
    design!(
        "Twin pier tank with a domed roof",
        Twin,
        28.7,
        8.3,
        6.75,
        DOME,
        true,
        CREAM
    ),
    design!(
        "Concrete portal with a flat cistern",
        Portal,
        24.8,
        8.8,
        7.15,
        DRUM,
        true,
        CREAM
    ),
    design!(
        "Tall splayed portal tank",
        Portal,
        36.9,
        9.6,
        6.35,
        OCTAGON,
        true,
        BLUE
    ),
    design!(
        "Low portal under a wide cap",
        Portal,
        23.5,
        6.7,
        7.5,
        SAUCER,
        false,
        GREEN
    ),
    design!(
        "Concrete portal with a raised gable",
        Portal,
        32.4,
        10.5,
        6.9,
        GABLED,
        true,
        WHITE
    ),
    design!(
        "Asymmetric portal with a round dome",
        Portal,
        29.3,
        9.1,
        6.75,
        DOME,
        false,
        BRICK
    ),
];
