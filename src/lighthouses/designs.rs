use super::{Daymark, TowerForm};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LighthouseDesign(u8);

#[derive(Clone, Copy)]
pub(super) enum Roof {
    Flat,
    Cone,
    Dome,
    Pyramid,
    Cupola,
}

#[derive(Clone, Copy)]
pub(super) struct Design {
    name: &'static str,
    pub height: f32,
    pub radius: f32,
    pub taper: f32,
    pub break_at: f32,
    pub waist: f32,
    pub roof: Roof,
    pub mark: Daymark,
    pub trim: [u8; 3],
}

impl LighthouseDesign {
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

    pub fn family(self) -> &'static str {
        match self.form() {
            TowerForm::Masonry => "Masonry",
            TowerForm::Keeper => "Keeper towers",
            TowerForm::Lattice => "Open-frame towers",
        }
    }

    pub(super) fn form(self) -> TowerForm {
        match self.0 {
            0..=8 => TowerForm::Masonry,
            9..=16 => TowerForm::Keeper,
            _ => TowerForm::Lattice,
        }
    }

    pub(super) fn choose(form: TowerForm, key: u64) -> Self {
        let (first, count) = match form {
            TowerForm::Masonry => (0, 9),
            TowerForm::Keeper => (9, 8),
            TowerForm::Lattice => (17, 8),
        };
        Self(first + (key % count) as u8)
    }

    pub(super) fn spec(self) -> Design {
        DESIGNS[self.index()]
    }
}

use Daymark::{BlackBands, PaintedBands, RedBrick, Whitewash};
use Roof::{Cone, Cupola, Dome, Flat, Pyramid};

const RED: [u8; 3] = [146, 61, 44];
const GREEN: [u8; 3] = [62, 102, 89];
const GRAY: [u8; 3] = [72, 82, 88];
const BLUE: [u8; 3] = [59, 83, 114];
const COPPER: [u8; 3] = [115, 134, 104];

macro_rules! design {
    ($name:literal, $height:literal, $radius:literal, $taper:literal, $break_at:literal,
     $waist:literal, $roof:ident, $mark:ident, $trim:ident) => {
        Design {
            name: $name,
            height: $height,
            radius: $radius,
            taper: $taper,
            break_at: $break_at,
            waist: $waist,
            roof: $roof,
            mark: $mark,
            trim: $trim,
        }
    };
}

const DESIGNS: [Design; 25] = [
    design!(
        "Red-banded taper",
        15.8,
        2.72,
        0.56,
        0.18,
        0.91,
        Cone,
        PaintedBands,
        RED
    ),
    design!(
        "White column and broad cap",
        13.0,
        2.39,
        0.93,
        0.70,
        1.00,
        Flat,
        Whitewash,
        GRAY
    ),
    design!(
        "Stepped brick landmark",
        15.0,
        3.00,
        0.57,
        0.38,
        0.66,
        Pyramid,
        RedBrick,
        RED
    ),
    design!(
        "Black-banded needle",
        20.5,
        2.22,
        0.51,
        0.72,
        0.60,
        Cone,
        BlackBands,
        GRAY
    ),
    design!(
        "Copper-domed bastion",
        11.0,
        3.33,
        0.88,
        0.29,
        0.91,
        Dome,
        RedBrick,
        COPPER
    ),
    design!(
        "Inward-curved white shaft",
        16.5,
        2.86,
        0.61,
        0.63,
        0.69,
        Cupola,
        Whitewash,
        GREEN
    ),
    design!(
        "Broad octagonal bell tower",
        12.1,
        3.31,
        0.68,
        0.29,
        0.78,
        Pyramid,
        Whitewash,
        BLUE
    ),
    design!(
        "Broad-seat striped column",
        17.0,
        2.88,
        0.72,
        0.25,
        0.73,
        Flat,
        BlackBands,
        GRAY
    ),
    design!(
        "Brick chimney with lantern dome",
        18.1,
        2.43,
        0.81,
        0.60,
        0.92,
        Dome,
        RedBrick,
        GREEN
    ),
    design!(
        "Flat-roof keeper block",
        10.2,
        3.25,
        0.94,
        0.57,
        1.00,
        Flat,
        Whitewash,
        GRAY
    ),
    design!(
        "Tall square brick keeper tower",
        15.2,
        2.67,
        0.81,
        0.48,
        0.89,
        Pyramid,
        RedBrick,
        RED
    ),
    design!(
        "Two-stage green-cupola keeper",
        12.5,
        3.21,
        0.59,
        0.46,
        0.69,
        Cupola,
        Whitewash,
        GREEN
    ),
    design!(
        "Striped square signal house",
        11.7,
        3.04,
        0.94,
        0.66,
        0.96,
        Cone,
        PaintedBands,
        RED
    ),
    design!(
        "Black-banded narrow watch tower",
        16.6,
        2.52,
        0.71,
        0.30,
        0.81,
        Flat,
        BlackBands,
        BLUE
    ),
    design!(
        "Squat keeper keep",
        10.0,
        3.40,
        0.76,
        0.22,
        0.81,
        Dome,
        RedBrick,
        COPPER
    ),
    design!(
        "Blue-capped double plinth tower",
        13.9,
        3.09,
        0.62,
        0.24,
        0.76,
        Pyramid,
        Whitewash,
        BLUE
    ),
    design!(
        "White harbor signal tower",
        11.2,
        3.33,
        0.98,
        0.54,
        1.00,
        Cupola,
        Whitewash,
        RED
    ),
    design!(
        "Red open-frame pylon",
        15.1,
        3.36,
        0.45,
        0.56,
        0.88,
        Cone,
        PaintedBands,
        RED
    ),
    design!(
        "Broad gray braced gantry",
        12.3,
        3.36,
        0.87,
        0.42,
        0.97,
        Flat,
        Whitewash,
        GRAY
    ),
    design!(
        "Tall twin-deck navigation frame",
        20.6,
        3.22,
        0.61,
        0.67,
        0.87,
        Dome,
        Whitewash,
        GREEN
    ),
    design!(
        "Battered iron derrick light",
        17.5,
        3.45,
        0.36,
        0.33,
        0.67,
        Pyramid,
        RedBrick,
        GRAY
    ),
    design!(
        "Green narrow cage light",
        18.5,
        2.55,
        0.75,
        0.51,
        0.89,
        Cupola,
        Whitewash,
        GREEN
    ),
    design!(
        "Low copper-red day beacon",
        10.0,
        3.13,
        0.72,
        0.42,
        0.85,
        Dome,
        RedBrick,
        RED
    ),
    design!(
        "Wide coastal truss with red balcony",
        14.8,
        3.40,
        0.78,
        0.72,
        0.94,
        Flat,
        PaintedBands,
        RED
    ),
    design!(
        "Slim steel-frame point light",
        19.1,
        2.62,
        0.52,
        0.28,
        0.84,
        Cone,
        BlackBands,
        BLUE
    ),
];
