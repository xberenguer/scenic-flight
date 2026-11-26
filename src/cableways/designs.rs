#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CablewayDesign(u8);

#[derive(Clone, Copy)]
pub(super) struct Paint {
    pub walls: [u8; 3],
    pub roof: [u8; 3],
    pub car: [u8; 3],
    pub steel: [u8; 3],
}

#[derive(Clone, Copy)]
pub(super) struct Terminal {
    /// Roof height at the back wall, rear ridge, front ridge and front wall.
    pub roof: [f32; 4],
    pub half_width: f32,
    pub upright: f32,
}

#[derive(Clone, Copy)]
pub(super) struct Car {
    pub length: f32,
    pub width: f32,
    pub chisel: f32,
    pub lower: f32,
    pub cap: f32,
}

#[derive(Clone, Copy)]
pub(super) struct Design {
    name: &'static str,
    pub terminal: Terminal,
    pub car: Car,
    pub paint: Paint,
    pub mast_waist: f32,
}

impl CablewayDesign {
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

    pub(super) fn spec(self) -> Design {
        DESIGNS[self.index()]
    }

    pub(super) fn select(key: u64) -> Self {
        Self((key % Self::COUNT as u64) as u8)
    }
}

const TIMBER: Paint = Paint {
    walls: [145, 108, 79],
    roof: [70, 80, 73],
    car: [203, 98, 58],
    steel: [145, 151, 142],
};
const BRICK: Paint = Paint {
    walls: [162, 116, 91],
    roof: [136, 76, 55],
    car: [233, 169, 67],
    steel: [130, 143, 145],
};
const STONE: Paint = Paint {
    walls: [175, 166, 147],
    roof: [101, 116, 116],
    car: [156, 57, 57],
    steel: [174, 180, 170],
};
const ALUMINUM: Paint = Paint {
    walls: [171, 178, 179],
    roof: [82, 115, 130],
    car: [80, 146, 164],
    steel: [146, 165, 167],
};
const ENAMEL: Paint = Paint {
    walls: [203, 198, 171],
    roof: [78, 101, 75],
    car: [76, 127, 89],
    steel: [114, 137, 127],
};

macro_rules! design {
    ($name:literal, $walls:literal, $width:literal, $roof:expr,
     [$car_length:literal, $car_width:literal, $chisel:literal, $lower:literal, $cap:literal],
     $waist:literal, $paint:ident) => {
        Design {
            name: $name,
            terminal: Terminal {
                roof: $roof,
                half_width: $width,
                upright: $walls,
            },
            car: Car {
                length: $car_length,
                width: $car_width,
                chisel: $chisel,
                lower: $lower,
                cap: $cap,
            },
            paint: $paint,
            mast_waist: $waist,
        }
    };
}

const DESIGNS: [Design; 25] = [
    design!(
        "Timber saddle terminal",
        3.1,
        3.99,
        [3.5, 5.5, 5.5, 3.5],
        [1.54, 1.21, 0.41, 0.86, 2.31],
        0.27,
        TIMBER
    ),
    design!(
        "Steep forest chalet",
        2.7,
        3.70,
        [3.0, 5.9, 5.3, 3.0],
        [1.25, 1.15, 0.12, 0.95, 2.33],
        0.34,
        TIMBER
    ),
    design!(
        "Saltbox loading hall",
        2.8,
        3.44,
        [5.1, 5.4, 4.8, 3.2],
        [1.64, 1.08, 0.35, 1.05, 2.27],
        0.41,
        TIMBER
    ),
    design!(
        "Butterfly timber canopy",
        3.0,
        4.08,
        [4.9, 3.4, 3.4, 4.9],
        [1.49, 1.27, 0.53, 0.81, 2.36],
        0.22,
        TIMBER
    ),
    design!(
        "Low timber transit lodge",
        2.65,
        3.93,
        [3.0, 3.8, 3.8, 3.4],
        [1.32, 1.19, 0.25, 0.94, 2.32],
        0.47,
        TIMBER
    ),
    design!(
        "Brick sawtooth depot",
        3.2,
        3.85,
        [3.6, 5.65, 3.55, 5.25],
        [1.51, 1.26, 0.16, 0.98, 2.36],
        0.35,
        BRICK
    ),
    design!(
        "Low red-brick engine house",
        2.8,
        4.00,
        [3.1, 4.0, 4.1, 3.1],
        [1.65, 1.30, 0.13, 1.15, 2.30],
        0.44,
        BRICK
    ),
    design!(
        "Square brick upper landing",
        4.0,
        3.45,
        [4.45, 4.45, 4.45, 4.45],
        [1.24, 1.10, 0.20, 0.95, 2.22],
        0.38,
        BRICK
    ),
    design!(
        "Brick ramp-and-monitor depot",
        3.1,
        3.79,
        [3.55, 5.45, 5.45, 4.40],
        [1.58, 1.24, 0.46, 1.07, 2.24],
        0.30,
        BRICK
    ),
    design!(
        "Brick shed terminal",
        2.9,
        3.59,
        [5.5, 4.8, 4.1, 3.4],
        [1.43, 1.20, 0.33, 0.74, 2.21],
        0.42,
        BRICK
    ),
    design!(
        "Stone barrel-vault terminal",
        3.4,
        4.10,
        [3.8, 5.2, 5.2, 3.8],
        [1.54, 1.29, 0.55, 0.69, 2.35],
        0.36,
        STONE
    ),
    design!(
        "Low stone ridge shelter",
        2.7,
        3.82,
        [3.2, 4.6, 4.6, 3.2],
        [1.30, 0.97, 0.30, 0.92, 2.16],
        0.25,
        STONE
    ),
    design!(
        "High-backed stone interchange",
        3.15,
        3.98,
        [5.8, 5.2, 4.3, 3.6],
        [1.64, 1.28, 0.10, 1.18, 2.37],
        0.46,
        STONE
    ),
    design!(
        "Crenelled stone landing",
        3.25,
        3.42,
        [4.9, 3.7, 3.7, 4.9],
        [1.18, 1.27, 0.39, 0.92, 2.26],
        0.29,
        STONE
    ),
    design!(
        "Broken-ridge stone hall",
        3.25,
        3.69,
        [3.65, 5.5, 4.1, 4.0],
        [1.49, 1.08, 0.23, 1.02, 2.34],
        0.45,
        STONE
    ),
    design!(
        "Aluminum folded-wing station",
        3.1,
        4.09,
        [5.35, 3.45, 4.00, 5.45],
        [1.58, 1.27, 0.60, 0.76, 2.33],
        0.23,
        ALUMINUM
    ),
    design!(
        "Wide aluminum arc terminal",
        3.3,
        4.12,
        [3.8, 4.9, 4.9, 3.8],
        [1.65, 1.31, 0.51, 0.82, 2.31],
        0.32,
        ALUMINUM
    ),
    design!(
        "Compact blue skylight hall",
        3.0,
        3.32,
        [3.5, 5.6, 5.6, 3.5],
        [1.19, 1.06, 0.48, 0.66, 2.23],
        0.43,
        ALUMINUM
    ),
    design!(
        "Aluminum single-slope pavilion",
        3.15,
        3.70,
        [3.6, 4.1, 4.7, 5.4],
        [1.51, 1.19, 0.37, 0.87, 2.31],
        0.34,
        ALUMINUM
    ),
    design!(
        "Flat blue transfer canopy",
        3.6,
        4.05,
        [4.3, 4.3, 4.3, 4.3],
        [1.64, 1.21, 0.27, 0.97, 2.37],
        0.40,
        ALUMINUM
    ),
    design!(
        "Green stepped ridge terminal",
        3.1,
        3.84,
        [3.7, 4.7, 5.6, 4.2],
        [1.37, 1.18, 0.47, 0.83, 2.28],
        0.31,
        ENAMEL
    ),
    design!(
        "Low green double-slope canopy",
        2.85,
        4.07,
        [3.2, 4.4, 4.3, 3.55],
        [1.62, 1.21, 0.19, 1.12, 2.22],
        0.44,
        ENAMEL
    ),
    design!(
        "Green twin-rake station",
        3.1,
        3.58,
        [5.2, 3.55, 5.45, 3.6],
        [1.27, 1.10, 0.42, 0.94, 2.36],
        0.38,
        ENAMEL
    ),
    design!(
        "Enamel observation terminal",
        3.7,
        3.44,
        [4.1, 5.6, 5.7, 4.2],
        [1.40, 1.30, 0.52, 0.67, 2.35],
        0.28,
        ENAMEL
    ),
    design!(
        "Green cantilevered plateau terminal",
        3.35,
        3.92,
        [5.25, 5.25, 4.0, 4.0],
        [1.61, 1.28, 0.34, 0.89, 2.29],
        0.46,
        ENAMEL
    ),
];
