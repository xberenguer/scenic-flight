use crate::world::Surface;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IslandProfile(u8);

#[derive(Clone, Copy)]
pub(super) struct Profile {
    name: &'static str,
    /// Width and transverse center offset at upstream, middle and downstream knots.
    pub width: [f32; 5],
    pub shift: [f32; 5],
    pub crest: f32,
    pub rim: Surface,
    pub cover: Surface,
    pub core: Surface,
}

impl IslandProfile {
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

    pub(super) fn spec(self) -> Profile {
        PROFILES[self.index()]
    }
}

use Surface::{Gravel, ReedMarsh, RiverSilt, Rock, Sand, WetMeadow};

macro_rules! profile {
    ($name:literal, $width:expr, $shift:expr, $crest:literal, $rim:ident, $cover:ident, $core:ident) => {
        Profile {
            name: $name,
            width: $width,
            shift: $shift,
            crest: $crest,
            rim: $rim,
            cover: $cover,
            core: $core,
        }
    };
}

const CENTERED: [f32; 5] = [0.0; 5];

const PROFILES: [Profile; 25] = [
    profile!(
        "Rounded reed island",
        [0.0, 0.83, 1.00, 0.83, 0.0],
        CENTERED,
        0.82,
        RiverSilt,
        WetMeadow,
        ReedMarsh
    ),
    profile!(
        "Sickle-shaped reed island",
        [0.0, 0.64, 0.92, 0.68, 0.0],
        [0.18, 0.28, 0.10, -0.23, -0.32],
        0.91,
        RiverSilt,
        WetMeadow,
        ReedMarsh
    ),
    profile!(
        "Reed island with twin lobes",
        [0.0, 0.96, 0.42, 0.94, 0.0],
        [0.0, 0.05, 0.04, -0.05, 0.0],
        0.78,
        RiverSilt,
        WetMeadow,
        ReedMarsh
    ),
    profile!(
        "Upstream reed fan",
        [0.0, 1.00, 0.77, 0.53, 0.0],
        [-0.10, 0.06, 0.09, 0.13, 0.19],
        0.81,
        RiverSilt,
        WetMeadow,
        ReedMarsh
    ),
    profile!(
        "Bowed reed ribbon",
        [0.0, 0.54, 0.59, 0.58, 0.0],
        [-0.23, 0.05, 0.30, 0.06, -0.22],
        0.96,
        RiverSilt,
        WetMeadow,
        ReedMarsh
    ),
    profile!(
        "Gravel teardrop",
        [0.0, 0.96, 0.93, 0.52, 0.0],
        CENTERED,
        0.66,
        RiverSilt,
        Gravel,
        Gravel
    ),
    profile!(
        "Gravel island with a narrow neck",
        [0.0, 0.95, 0.29, 0.72, 0.0],
        [0.0, -0.07, -0.04, 0.16, 0.21],
        0.74,
        RiverSilt,
        Gravel,
        Gravel
    ),
    profile!(
        "Hooked gravel bar",
        [0.0, 0.63, 0.84, 0.72, 0.0],
        [0.27, 0.22, -0.01, -0.20, -0.27],
        0.64,
        RiverSilt,
        Gravel,
        Gravel
    ),
    profile!(
        "Flat-backed gravel lens",
        [0.0, 0.67, 0.96, 0.92, 0.0],
        [0.0, 0.11, 0.14, -0.03, -0.12],
        0.63,
        RiverSilt,
        Gravel,
        Gravel
    ),
    profile!(
        "Slender shingle spindle",
        [0.0, 0.47, 0.78, 0.50, 0.0],
        CENTERED,
        0.72,
        RiverSilt,
        Gravel,
        Rock
    ),
    profile!(
        "Sandy fan island",
        [0.0, 0.52, 0.76, 1.00, 0.0],
        CENTERED,
        0.67,
        RiverSilt,
        Sand,
        Sand
    ),
    profile!(
        "Sandy island with a crescent tail",
        [0.0, 0.86, 0.68, 0.49, 0.0],
        [-0.10, -0.12, 0.02, 0.22, 0.33],
        0.68,
        RiverSilt,
        Sand,
        Sand
    ),
    profile!(
        "Serrated sand and shingle island",
        [0.0, 0.89, 0.52, 0.90, 0.0],
        [0.0, 0.17, -0.10, 0.12, 0.0],
        0.69,
        RiverSilt,
        Sand,
        Gravel
    ),
    profile!(
        "Long pale sand ribbon",
        [0.0, 0.56, 0.58, 0.53, 0.0],
        [-0.14, -0.10, 0.01, 0.13, 0.15],
        0.60,
        RiverSilt,
        Sand,
        Sand
    ),
    profile!(
        "Sand head with a reed center",
        [0.0, 0.97, 0.80, 0.74, 0.0],
        [0.03, 0.08, 0.04, -0.15, -0.20],
        0.85,
        RiverSilt,
        Sand,
        ReedMarsh
    ),
    profile!(
        "Meadow kidney island",
        [0.0, 0.87, 0.80, 0.73, 0.0],
        [-0.26, -0.10, 0.24, 0.06, -0.18],
        0.91,
        RiverSilt,
        WetMeadow,
        WetMeadow
    ),
    profile!(
        "Meadow island with a tapered tail",
        [0.0, 0.90, 0.94, 0.41, 0.0],
        [-0.19, -0.09, 0.02, 0.18, 0.24],
        0.96,
        RiverSilt,
        WetMeadow,
        WetMeadow
    ),
    profile!(
        "Twin meadow knolls",
        [0.0, 0.86, 0.39, 0.96, 0.0],
        [0.15, 0.14, 0.03, -0.16, -0.21],
        0.95,
        RiverSilt,
        WetMeadow,
        WetMeadow
    ),
    profile!(
        "Island with meadow margins and gravel crest",
        [0.0, 0.78, 1.00, 0.62, 0.0],
        CENTERED,
        0.90,
        RiverSilt,
        WetMeadow,
        Gravel
    ),
    profile!(
        "Bent meadow ribbon",
        [0.0, 0.70, 0.60, 0.66, 0.0],
        [0.18, 0.17, -0.15, 0.05, 0.21],
        0.90,
        RiverSilt,
        WetMeadow,
        WetMeadow
    ),
    profile!(
        "Exposed stone lens",
        [0.0, 0.72, 0.96, 0.71, 0.0],
        CENTERED,
        1.00,
        Gravel,
        Gravel,
        Rock
    ),
    profile!(
        "Stone ridge with a shallow downstream tail",
        [0.0, 0.86, 0.95, 0.49, 0.0],
        [-0.10, 0.04, 0.15, 0.20, 0.27],
        0.93,
        Gravel,
        Gravel,
        Rock
    ),
    profile!(
        "Stone island with a sand apron",
        [0.0, 0.56, 0.84, 0.88, 0.0],
        [0.0, 0.07, 0.13, 0.12, 0.02],
        1.00,
        RiverSilt,
        Sand,
        Rock
    ),
    profile!(
        "Two stone crowns on shingle",
        [0.0, 0.88, 0.36, 0.87, 0.0],
        [0.12, -0.06, 0.02, 0.12, 0.20],
        0.98,
        Gravel,
        Gravel,
        Rock
    ),
    profile!(
        "Stone bend with a grassy crest",
        [0.0, 0.65, 0.89, 0.70, 0.0],
        [-0.19, 0.07, 0.23, 0.03, -0.18],
        0.97,
        Gravel,
        Rock,
        WetMeadow
    ),
];
