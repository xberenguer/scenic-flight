//! Building massing shared by the terrain, close mesh and distant silhouette.

pub(crate) const TOWER_DESIGN_COUNT: usize = 250;
const VARIANTS: usize = 10;

#[derive(Clone, Copy)]
pub(crate) struct Tier {
    pub top: f32,
    pub half: [f32; 2],
    pub center: [f32; 2],
}

#[derive(Clone, Copy)]
pub(crate) struct TowerDesign {
    pub tiers: [Tier; 3],
    pub size: [f32; 2],
    pub height: f32,
    pub wall: [u8; 3],
}

struct Family {
    levels: [f32; 2],
    widths: [[f32; 2]; 2],
    offsets: [[f32; 2]; 2],
    size: [f32; 2],
    height: f32,
    material: usize,
}

const WALLS: [[u8; 3]; 8] = [
    [107, 139, 157], // Blue curtain wall
    [161, 145, 116], // Sandstone
    [99, 115, 108],  // Green glass and weathered bronze
    [151, 153, 146], // Concrete
    [140, 93, 76],   // Brick and terracotta
    [91, 101, 116],  // Dark steel and slate glass
    [167, 161, 135], // Limestone
    [120, 145, 138], // Pale copper glass
];

const fn family(
    levels: [f32; 2], widths: [[f32; 2]; 2], offsets: [[f32; 2]; 2],
    size: [f32; 2], height: f32, material: usize,
) -> Family {
    Family { levels, widths, offsets, size, height, material }
}

const FAMILIES: [Family; 25] = [
    // Flat-topped office slab, Art Deco shoulder, pitched-back terrace.
    family([0.22, 0.87], [[0.95, 0.72], [0.86, 0.63]], [[0.0,0.0],[0.0,0.0]], [1.00,0.73], 26.0, 0),
    family([0.58, 0.88], [[0.70, 0.78], [0.28, 0.33]], [[0.0,0.0],[0.0,0.0]], [0.91,0.86], 37.0, 1),
    family([0.34, 0.69], [[0.78, 0.66], [0.58, 0.43]], [[0.86,0.65],[0.75,0.68]], [1.00,0.90], 24.0, 6),
    // Pencil tower, monolithic grid, offset penthouse.
    family([0.24, 0.84], [[0.37, 0.49], [0.16, 0.24]], [[0.10,0.0],[-0.10,0.1]], [0.77,0.65], 45.0, 5),
    family([0.75, 0.92], [[0.92, 0.89], [0.78, 0.68]], [[0.0,0.0],[0.0,0.0]], [0.94,0.93], 29.0, 3),
    family([0.65, 0.83], [[0.79, 0.72], [0.37, 0.32]], [[-0.49,0.25],[-0.86,0.68]], [0.98,0.81], 27.0, 2),
    // Thin end wall, broad hotel, stacked terraces.
    family([0.22, 0.92], [[0.72, 0.95], [0.56, 0.76]], [[0.60,0.0],[0.25,0.0]], [0.62,1.00], 35.0, 7),
    family([0.41, 0.79], [[0.92, 0.58], [0.69, 0.25]], [[0.0,-0.70],[0.78,-0.61]], [1.00,0.80], 22.0, 4),
    family([0.29, 0.63], [[0.83, 0.87], [0.59, 0.62]], [[0.0,0.0],[0.0,0.0]], [0.96,0.96], 32.0, 3),
    // Edge spire, broad base with a compact stack, end-set crown.
    family([0.30, 0.77], [[0.54, 0.62], [0.23, 0.31]], [[0.86,-0.74],[0.76,-0.65]], [0.86,0.88], 42.0, 0),
    family([0.57, 0.86], [[0.49, 0.55], [0.29, 0.28]], [[0.20,-0.20],[0.05,0.0]], [1.00,0.95], 30.0, 6),
    family([0.49, 0.83], [[0.93, 0.57], [0.35, 0.45]], [[0.03,0.70],[-0.88,0.10]], [0.97,0.78], 33.0, 1),
    // Narrow portal block, library terraces, centered observatory.
    family([0.28, 0.75], [[0.64, 0.89], [0.51, 0.59]], [[-0.72,0.0],[-0.10,-0.70]], [0.73,0.94], 34.0, 5),
    family([0.38, 0.72], [[0.79, 0.61], [0.52, 0.25]], [[0.15,0.79],[0.32,0.74]], [1.00,0.91], 21.0, 6),
    family([0.54, 0.89], [[0.68, 0.67], [0.55, 0.53]], [[0.0,0.0],[0.0,0.0]], [0.83,0.83], 25.0, 2),
    // Three slabs rising diagonally, setback warehouse, narrow street-corner block.
    family([0.33, 0.66], [[0.69, 0.73], [0.36, 0.46]], [[-0.81,0.72],[0.88,-0.85]], [0.97,1.00], 31.0, 7),
    family([0.69, 0.88], [[0.95, 0.62], [0.62, 0.20]], [[0.0,-0.65],[-0.40,-0.65]], [0.99,0.70], 23.0, 4),
    family([0.26, 0.73], [[0.48, 0.86], [0.30, 0.58]], [[-0.85,0.3],[0.60,0.85]], [0.66,0.96], 35.0, 0),
    // Twin setbacks, tall flat glass, wide stone shaft.
    family([0.43, 0.71], [[0.86, 0.71], [0.50, 0.38]], [[0.75,-0.66],[-0.77,0.61]], [0.98,0.92], 28.0, 3),
    family([0.21, 0.92], [[0.81, 0.86], [0.75, 0.78]], [[0.15,0.1],[0.10,0.1]], [0.90,0.90], 40.0, 2),
    family([0.47, 0.80], [[0.93, 0.51], [0.76, 0.35]], [[0.0,0.35],[0.47,0.0]], [1.00,0.84], 32.0, 6),
    // Brick stair tower, side-mounted crown, elliptical-equivalent narrow plan.
    family([0.28, 0.60], [[0.64, 0.81], [0.31, 0.55]], [[0.78,0.68],[0.81,0.67]], [0.85,0.97], 30.0, 4),
    family([0.61, 0.85], [[0.89, 0.76], [0.19, 0.48]], [[0.12,0.06],[0.92,-0.25]], [0.94,0.85], 38.0, 7),
    family([0.35, 0.89], [[0.55, 0.87], [0.35, 0.72]], [[0.0,0.15],[0.0,0.0]], [0.73,0.89], 41.0, 5),
    // A wide civic podium with a low offset rooftop block.
    family([0.62, 0.78], [[0.81, 0.63], [0.45, 0.31]], [[0.25,0.67],[-0.79,-0.25]], [1.00,1.00], 20.0, 1),
];

// Real changes in massing within each architectural family. The rows vary
// height, plan proportions, setbacks, and the placement of the upper volumes.
const OPTIONS: [[f32; 6]; VARIANTS] = [
    [0.82, 0.76, 0.92, -0.06, 0.82, -0.86],
    [0.92, 0.86, 0.73,  0.05, 0.96,  0.64],
    [1.05, 0.98, 0.82, -0.02, 0.76, -0.45],
    [1.13, 0.81, 1.00,  0.07, 0.98,  0.86],
    [0.89, 1.00, 0.96, -0.09, 0.89,  0.25],
    [1.19, 0.90, 0.76,  0.00, 0.93, -0.68],
    [0.96, 0.72, 0.81,  0.11, 0.78,  0.48],
    [1.09, 0.95, 0.88, -0.04, 1.00, -0.42],
    [1.03, 0.85, 0.94,  0.09, 0.91,  0.70],
    [1.24, 0.97, 0.70,  0.02, 0.84, -0.95],
];

const BASE: Tier = Tier { top: 0.0, half: [1.0; 2], center: [0.0; 2] };
const EMPTY: TowerDesign = TowerDesign { tiers: [BASE; 3], size: [1.0; 2], height: 0.0, wall: WALLS[0] };
const DESIGNS: [TowerDesign; TOWER_DESIGN_COUNT] = {
    let mut output = [EMPTY; TOWER_DESIGN_COUNT];
    let mut index = 0;
    while index < output.len() {
        let family = &FAMILIES[index / VARIANTS];
        let variant = index % VARIANTS;
        let [height, width, depth, setback, crown_scale, bias] = OPTIONS[variant];
        let shaft_half = family.widths[0];
        let crown_half = [family.widths[1][0] * crown_scale, family.widths[1][1] * crown_scale];
        let sx = family.offsets[0][0] * (1.0 - shaft_half[0]);
        let sz = family.offsets[0][1] * (1.0 - shaft_half[1]);
        let cx = sx + (family.offsets[1][0] * 0.58 + bias * 0.42) * (shaft_half[0] - crown_half[0]);
        let cz = sz + (family.offsets[1][1] * 0.65 - bias * 0.35) * (shaft_half[1] - crown_half[1]);
        output[index] = TowerDesign {
            tiers: [
                Tier { top: family.levels[0] + setback * 0.65, ..BASE },
                Tier { top: family.levels[1] + setback * 0.34, half: shaft_half, center: [sx, sz] },
                Tier { top: 1.0, half: crown_half, center: [cx, cz] },
            ],
            size: [family.size[0] * width, family.size[1] * depth],
            height: family.height * height,
            wall: WALLS[family.material],
        };
        index += 1;
    }
    output
};

pub(crate) fn design(profile: u16) -> &'static TowerDesign {
    &DESIGNS[usize::from(profile)]
}

/// Give one downtown a coherent mix while keeping neighboring tower families distinct.
pub(crate) fn select(layout: u32, parcel_key: u32, rank: u8) -> u16 {
    // Prominent sites draw from the landmark catalog. The center cluster still
    // includes conventional offices and apartments between landmarks.
    if rank == 0 || ((parcel_key >> 4) & 3 == 0 && rank < 3) {
        let family = ((layout >> 14) as usize + usize::from(rank) * 7) % landmarks::LANDMARKS.len();
        return (TOWER_DESIGN_COUNT + family) as u16;
    }
    select_regular(layout, parcel_key, rank)
}

pub(crate) fn select_regular(layout: u32, parcel_key: u32, rank: u8) -> u16 {
    let family = ((layout >> 9) as usize + usize::from(rank) * 7) % FAMILIES.len();
    (family * VARIANTS + (parcel_key >> 19) as usize % VARIANTS) as u16
}

#[path = "tower_forms/landmarks.rs"]
pub(crate) mod landmarks;

pub(crate) fn landmark(profile: u16) -> Option<&'static landmarks::Landmark> {
    landmarks::LANDMARKS.get(usize::from(profile).checked_sub(TOWER_DESIGN_COUNT)?)
}
