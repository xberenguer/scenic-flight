#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlacierProfile(u8);

pub(super) struct Profile {
    name: &'static str,
    /// Relative width from the accumulation basin to the terminus.
    pub width: [f32; 5],
    pub nose: f32,
    pub thickness: f32,
    /// Snow line, crevasse separation, arc, diagonal shear, medial moraine.
    pub snow: f32,
    pub spacing: f32,
    pub arc: f32,
    pub shear: f32,
    pub medial: f32,
}

impl GlacierProfile {
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
    pub fn width_at(&self, fraction: f32) -> f32 {
        let t = fraction * 4.0;
        let i = (t as usize).min(3);
        let outline = self.width[i] + (self.width[i + 1] - self.width[i]) * (t - i as f32);
        let neck = (0.80 - 0.38 * self.width[4]).clamp(0.35, 0.59);
        outline * (1.0 - neck * crate::math::smoothstep((fraction - 9.0 / 13.0) * 3.51))
    }
}

macro_rules! profile {
    ($name:literal, $width:expr, $nose:literal, $thickness:literal, $snow:literal,
     $spacing:literal, $arc:literal, $shear:literal, $medial:literal) => {
        Profile {
            name: $name,
            width: $width,
            nose: $nose,
            thickness: $thickness,
            snow: $snow,
            spacing: $spacing,
            arc: $arc,
            shear: $shear,
            medial: $medial,
        }
    };
}

const PROFILES: [Profile; 25] = [
    profile!(
        "Broad-basin ice tongue",
        [1.08, 1.15, 0.91, 0.72, 0.67],
        7.5,
        4.7,
        28.0,
        12.8,
        5.1,
        -0.16,
        0.0
    ),
    profile!(
        "Snow bowl with a narrow outlet",
        [1.11, 1.10, 0.69, 0.63, 0.63],
        10.0,
        4.3,
        32.0,
        14.0,
        6.9,
        0.05,
        0.0
    ),
    profile!(
        "Snow saddle feeding a long ribbon",
        [0.76, 0.71, 0.63, 0.61, 0.69],
        8.2,
        4.2,
        25.0,
        13.7,
        5.9,
        0.16,
        0.0
    ),
    profile!(
        "Tapered snow apron",
        [1.13, 1.03, 0.85, 0.66, 0.58],
        15.5,
        4.4,
        37.0,
        13.1,
        5.2,
        -0.13,
        0.0
    ),
    profile!(
        "Double-basin icefield",
        [1.02, 1.15, 0.67, 1.03, 0.79],
        9.0,
        4.6,
        23.0,
        11.4,
        6.7,
        0.18,
        0.0
    ),
    profile!(
        "Evenly spaced blue ice ladder",
        [0.86, 0.92, 0.96, 0.88, 0.74],
        7.0,
        4.8,
        11.0,
        8.2,
        3.6,
        -0.08,
        0.0
    ),
    profile!(
        "Sharp-sheared crevasse fan",
        [0.74, 0.89, 1.08, 1.02, 0.82],
        9.2,
        4.9,
        15.0,
        8.7,
        7.6,
        0.36,
        0.0
    ),
    profile!(
        "Bowed blue icefall",
        [0.88, 1.04, 1.10, 0.84, 0.70],
        11.0,
        5.2,
        14.0,
        8.9,
        10.1,
        -0.10,
        0.0
    ),
    profile!(
        "Broken crevasse throat",
        [1.04, 0.78, 0.68, 0.94, 0.86],
        8.8,
        4.9,
        19.0,
        7.2,
        10.7,
        -0.28,
        0.0
    ),
    profile!(
        "Open diagonal ice steps",
        [0.79, 0.96, 0.87, 0.97, 0.89],
        6.1,
        4.7,
        10.0,
        10.4,
        4.8,
        -0.42,
        0.0
    ),
    profile!(
        "Bulging ice terminus",
        [0.80, 0.82, 0.88, 1.07, 1.12],
        4.0,
        5.0,
        17.0,
        11.2,
        6.1,
        0.13,
        0.0
    ),
    profile!(
        "Long pinched ice tongue",
        [1.05, 0.91, 0.78, 0.59, 0.57],
        18.0,
        4.5,
        18.0,
        9.3,
        7.4,
        -0.03,
        0.0
    ),
    profile!(
        "Waisted ice tongue with a broad toe",
        [1.06, 1.0, 0.65, 0.83, 1.07],
        6.2,
        4.7,
        20.0,
        12.5,
        5.3,
        0.15,
        0.0
    ),
    profile!(
        "Flared apron terminus",
        [0.81, 0.81, 0.82, 0.99, 1.15],
        12.0,
        4.7,
        17.0,
        12.2,
        6.2,
        0.22,
        0.0
    ),
    profile!(
        "Thin hooked ice tail",
        [0.91, 0.95, 0.86, 0.72, 0.61],
        20.5,
        4.1,
        15.0,
        13.2,
        5.4,
        -0.34,
        0.0
    ),
    profile!(
        "Single-streak moraine glacier",
        [0.91, 0.99, 0.96, 0.89, 0.86],
        9.5,
        4.7,
        18.0,
        12.7,
        6.7,
        -0.08,
        0.85
    ),
    profile!(
        "Thin ice between stony margins",
        [0.78, 0.79, 0.76, 0.72, 0.68],
        12.5,
        4.1,
        17.0,
        13.3,
        6.1,
        0.17,
        0.38
    ),
    profile!(
        "Curving central moraine ribbon",
        [0.92, 1.06, 0.92, 0.99, 0.91],
        6.6,
        4.6,
        20.0,
        13.2,
        5.7,
        0.39,
        1.0
    ),
    profile!(
        "Moraine-lined lower ice lens",
        [0.74, 0.83, 0.92, 1.10, 0.78],
        12.0,
        5.0,
        19.0,
        12.0,
        5.4,
        -0.22,
        0.66
    ),
    profile!(
        "Rock-streaked ice apron",
        [1.08, 1.05, 0.90, 0.94, 1.11],
        7.7,
        4.6,
        26.0,
        11.1,
        5.9,
        -0.27,
        0.92
    ),
    profile!(
        "Three-stage glacial ribbon",
        [0.76, 1.14, 0.61, 1.03, 0.67],
        13.0,
        4.7,
        22.0,
        10.4,
        7.2,
        0.12,
        0.0
    ),
    profile!(
        "Broad shoulder ice apron",
        [0.87, 1.10, 1.12, 0.68, 0.62],
        9.5,
        4.7,
        22.0,
        11.3,
        9.8,
        -0.18,
        0.29
    ),
    profile!(
        "Snow neck with a late icefall",
        [1.08, 0.73, 0.68, 1.07, 0.84],
        8.5,
        4.9,
        29.0,
        7.8,
        7.1,
        0.19,
        0.0
    ),
    profile!(
        "Short-faced blocky ice tongue",
        [0.83, 0.85, 1.01, 1.02, 0.90],
        3.5,
        5.0,
        16.0,
        8.6,
        4.7,
        -0.12,
        0.0
    ),
    profile!(
        "Twisting icefall with exposed moraine",
        [0.89, 1.04, 0.81, 1.10, 0.74],
        11.0,
        4.9,
        19.0,
        9.2,
        9.0,
        0.37,
        0.75
    ),
];
