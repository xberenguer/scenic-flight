use super::BridgeStyle;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BridgeDesign(u8);

#[derive(Clone, Copy)]
pub(super) enum Web {
    Posts,
    Pickets,
    Panel,
    Diagonals,
    Crossed,
    Vertical,
    Warren,
    Pratt,
    Howe,
}

#[derive(Clone, Copy)]
pub(super) struct Design {
    name: &'static str,
    pub height: f32,
    pub nodes: [f32; 7],
    pub web: Web,
    pub member: f32,
    pub color: [u8; 3],
}

impl BridgeDesign {
    pub(super) const DEFAULT: Self = Self(0);
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

    pub(super) fn select(style: BridgeStyle, key: u64) -> Self {
        let (offset, count) = match style {
            BridgeStyle::LowGirder => (0, 10),
            BridgeStyle::SmallTruss => (10, 15),
        };
        Self(offset + (key % count) as u8)
    }

    pub(super) fn style(self) -> BridgeStyle {
        if self.0 < 10 {
            BridgeStyle::LowGirder
        } else {
            BridgeStyle::SmallTruss
        }
    }

    pub(super) fn spec(self) -> Design {
        DESIGNS[self.index()]
    }
}

use Web::{Crossed, Diagonals, Howe, Panel, Pickets, Posts, Pratt, Vertical, Warren};
const CREAM: [u8; 3] = [177, 169, 145];
const CONCRETE: [u8; 3] = [155, 155, 148];
const IRON: [u8; 3] = [91, 116, 118];
const RUST: [u8; 3] = [157, 84, 65];
const GREEN: [u8; 3] = [91, 123, 102];
const BLUE: [u8; 3] = [80, 119, 150];
const TIMBER: [u8; 3] = [133, 104, 76];

macro_rules! design {
    ($name:literal, $height:literal, $nodes:expr, $web:ident, $member:literal, $color:ident) => {
        Design {
            name: $name,
            height: $height,
            nodes: $nodes,
            web: $web,
            member: $member,
            color: $color,
        }
    };
}

const FLAT: [f32; 7] = [1.0; 7];
const DESIGNS: [Design; 25] = [
    design!("Low iron pipe railing", 0.52, FLAT, Posts, 0.10, IRON),
    design!("Concrete flood parapet", 0.95, FLAT, Panel, 0.15, CONCRETE),
    design!(
        "Timber X-braced crossing",
        0.88,
        FLAT,
        Crossed,
        0.13,
        TIMBER
    ),
    design!(
        "Cream stone shoulder walls",
        0.83,
        [1.0, 0.75, 0.75, 0.75, 0.75, 0.75, 1.0],
        Panel,
        0.14,
        CREAM
    ),
    design!(
        "Green double-picket deck",
        0.88,
        FLAT,
        Pickets,
        0.084,
        GREEN
    ),
    design!("Blue diagonal railing", 0.86, FLAT, Diagonals, 0.13, BLUE),
    design!(
        "Raised-center masonry parapet",
        1.08,
        [0.56, 0.73, 0.91, 1.0, 0.91, 0.73, 0.56],
        Panel,
        0.14,
        CREAM
    ),
    design!(
        "Ramped steel shoulder rail",
        1.06,
        [0.62, 1.0, 0.72, 0.72, 0.72, 1.0, 0.62],
        Posts,
        0.14,
        IRON
    ),
    design!(
        "Red open roadside lattice",
        0.78,
        [0.62, 1.0, 1.0, 1.0, 1.0, 1.0, 0.62],
        Crossed,
        0.10,
        RUST
    ),
    design!(
        "Timber baluster crossing",
        0.79,
        [1.0, 0.83, 0.83, 1.0, 0.83, 0.83, 1.0],
        Pickets,
        0.115,
        TIMBER
    ),
    design!(
        "Flat-chord Warren truss",
        0.111,
        [0.30, 1.0, 1.0, 1.0, 1.0, 1.0, 0.30],
        Warren,
        0.22,
        RUST
    ),
    design!(
        "Green Pratt through truss",
        0.131,
        [0.28, 1.0, 1.0, 1.0, 1.0, 1.0, 0.28],
        Pratt,
        0.19,
        GREEN
    ),
    design!(
        "Timber Howe truss",
        0.112,
        [0.44, 0.86, 1.0, 1.0, 1.0, 0.86, 0.44],
        Howe,
        0.25,
        TIMBER
    ),
    design!(
        "Blue camelback truss",
        0.153,
        [0.25, 0.63, 0.88, 1.0, 0.88, 0.63, 0.25],
        Pratt,
        0.20,
        BLUE
    ),
    design!(
        "Low pony truss",
        0.075,
        [0.43, 1.0, 1.0, 1.0, 1.0, 1.0, 0.43],
        Warren,
        0.19,
        IRON
    ),
    design!(
        "Cream tied shallow arch",
        0.129,
        [0.10, 0.63, 0.90, 1.0, 0.90, 0.63, 0.10],
        Vertical,
        0.24,
        CREAM
    ),
    design!(
        "Tall crown bowstring",
        0.172,
        [0.12, 0.68, 0.92, 1.0, 0.92, 0.68, 0.12],
        Vertical,
        0.21,
        GREEN
    ),
    design!(
        "Double-X iron lattice",
        0.124,
        [0.33, 0.89, 1.0, 1.0, 1.0, 0.89, 0.33],
        Crossed,
        0.14,
        IRON
    ),
    design!(
        "Blue bowed rail frame",
        0.117,
        [0.56, 0.90, 1.0, 1.0, 1.0, 0.90, 0.56],
        Vertical,
        0.27,
        BLUE
    ),
    design!(
        "Red polygonal Warren",
        0.151,
        [0.30, 0.71, 1.0, 1.0, 1.0, 0.71, 0.30],
        Warren,
        0.24,
        RUST
    ),
    design!("Low parallel-chord Pratt", 0.082, FLAT, Pratt, 0.18, IRON),
    design!(
        "Raked blue tie arch",
        0.143,
        [0.10, 0.78, 1.0, 0.91, 0.72, 0.46, 0.10],
        Vertical,
        0.22,
        BLUE
    ),
    design!(
        "High-portal lattice bridge",
        0.149,
        [0.77, 1.0, 0.93, 0.92, 0.93, 1.0, 0.77],
        Crossed,
        0.17,
        GREEN
    ),
    design!(
        "Timber triple crest truss",
        0.134,
        [0.36, 0.92, 0.86, 1.0, 0.86, 0.92, 0.36],
        Howe,
        0.23,
        TIMBER
    ),
    design!(
        "Flat-topped cream arch frame",
        0.153,
        [0.10, 0.81, 1.0, 1.0, 1.0, 0.81, 0.10],
        Vertical,
        0.21,
        CREAM
    ),
];
