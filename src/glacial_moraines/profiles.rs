#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoraineProfile(u8);

pub(super) struct Shape {
    name: &'static str,
    /// Cross-valley changes in crest location and height. Values are normalized
    /// to the surveyed footprint, whose center is reserved for the outlet.
    pub shift: [f32; 5],
    pub height: [f32; 5],
    pub steepness: f32,
    pub shoulder: f32,
    pub twin: f32,
}

impl MoraineProfile {
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

    pub(super) fn spec(self) -> &'static Shape {
        &PROFILES[self.index()]
    }
}

impl Shape {
    pub fn section(&self, cross: f32) -> [f32; 2] {
        let t = (cross + 1.0) * 2.0;
        let i = (t as usize).min(3);
        let f = t - i as f32;
        [
            self.shift[i] + (self.shift[i + 1] - self.shift[i]) * f,
            self.height[i] + (self.height[i + 1] - self.height[i]) * f,
        ]
    }
}

const LEVEL: [f32; 5] = [1.0; 5];

macro_rules! shape {
    ($name:literal, $shift:expr, $height:expr, $steep:literal, $wide:literal, $twin:literal) => {
        Shape {
            name: $name,
            shift: $shift,
            height: $height,
            steepness: $steep,
            shoulder: $wide,
            twin: $twin,
        }
    };
}

const PROFILES: [Shape; 25] = [
    shape!("Even terminal arc", [0.0; 5], LEVEL, 0.19, 1.03, 0.0),
    shape!(
        "Deep-bowed terminal ridge",
        [-0.20, -0.06, 0.06, -0.06, -0.20],
        LEVEL,
        0.22,
        0.91,
        0.0
    ),
    shape!(
        "Open horseshoe moraine",
        [0.17, 0.09, -0.01, 0.09, 0.17],
        [0.84, 0.95, 1.0, 0.97, 0.86],
        0.22,
        0.94,
        0.0
    ),
    shape!(
        "Flat-crowned terminal shelf",
        [-0.09, 0.05, 0.05, 0.05, -0.09],
        [0.91, 1.0, 1.0, 1.0, 0.91],
        0.10,
        1.18,
        0.0
    ),
    shape!(
        "Narrow receding arc",
        [0.11, -0.06, -0.19, -0.06, 0.11],
        [0.92, 0.98, 1.0, 0.98, 0.92],
        0.28,
        0.83,
        0.0
    ),
    shape!(
        "West-heavy glacial bank",
        [0.06, 0.18, 0.06, -0.08, -0.14],
        [0.93, 1.0, 0.98, 0.86, 0.87],
        0.18,
        1.08,
        0.0
    ),
    shape!(
        "East-heavy glacial bank",
        [-0.10, -0.06, 0.02, 0.12, 0.19],
        [0.85, 0.88, 0.98, 1.0, 0.98],
        0.18,
        1.09,
        0.0
    ),
    shape!(
        "Swept-back left lobe",
        [-0.22, -0.15, 0.04, 0.12, 0.15],
        [0.94, 0.97, 0.99, 0.92, 0.89],
        0.29,
        0.89,
        0.0
    ),
    shape!(
        "Swept-back right lobe",
        [0.11, 0.17, 0.06, -0.14, -0.23],
        [0.86, 0.92, 0.98, 1.0, 0.98],
        0.25,
        0.91,
        0.0
    ),
    shape!(
        "Offset stony half moons",
        [0.15, 0.18, -0.07, -0.20, -0.22],
        [0.92, 1.0, 0.96, 0.93, 0.91],
        0.23,
        0.86,
        0.0
    ),
    shape!(
        "Twin crests with a shallow saddle",
        [0.01, 0.07, 0.10, 0.08, 0.0],
        LEVEL,
        0.20,
        0.87,
        0.67
    ),
    shape!(
        "Low outer recessional ridge",
        [-0.15, -0.08, 0.08, -0.04, -0.15],
        [0.89, 0.97, 1.0, 0.97, 0.89],
        0.21,
        0.93,
        0.47
    ),
    shape!(
        "Tall pair of recessional ridges",
        [0.13, 0.04, -0.13, 0.04, 0.13],
        LEVEL,
        0.27,
        0.82,
        0.82
    ),
    shape!(
        "Wide inner ridge and gravel spur",
        [-0.12, 0.10, 0.06, 0.16, -0.08],
        [0.89, 0.96, 1.0, 1.0, 0.86],
        0.20,
        1.09,
        0.55
    ),
    shape!(
        "Broken arc with a second bank",
        [0.14, -0.18, 0.05, 0.18, -0.12],
        [0.87, 0.99, 0.98, 0.97, 0.88],
        0.22,
        0.88,
        0.69
    ),
    shape!(
        "Steep-faced gravel rampart",
        [0.09, 0.02, 0.0, -0.04, -0.12],
        LEVEL,
        0.44,
        0.88,
        0.0
    ),
    shape!(
        "Low broad debris apron",
        [-0.11, 0.06, 0.03, 0.05, -0.13],
        [0.92, 1.0, 0.99, 1.0, 0.92],
        0.04,
        1.20,
        0.0
    ),
    shape!(
        "Narrow high crest and footslope",
        [-0.14, 0.10, 0.11, 0.08, -0.14],
        LEVEL,
        0.37,
        0.77,
        0.28
    ),
    shape!(
        "Long asymmetric gravel apron",
        [-0.03, 0.17, 0.10, -0.08, 0.02],
        [0.87, 0.97, 0.99, 0.94, 0.90],
        0.38,
        1.17,
        0.0
    ),
    shape!(
        "Twin shoulders with a steep toe",
        [0.06, -0.14, -0.03, -0.15, 0.08],
        [0.90, 1.0, 0.91, 1.0, 0.90],
        0.40,
        0.94,
        0.41
    ),
    shape!(
        "Scalloped glacial arc",
        [0.13, -0.19, 0.13, -0.16, 0.14],
        [0.91, 1.0, 0.91, 1.0, 0.92],
        0.23,
        0.96,
        0.0
    ),
    shape!(
        "Sinuous ridge with a broad tail",
        [-0.18, 0.16, 0.12, -0.14, 0.10],
        [0.89, 0.97, 1.0, 0.95, 0.91],
        0.17,
        1.14,
        0.29
    ),
    shape!(
        "Hooked moraine lobes",
        [-0.21, 0.11, 0.09, 0.12, -0.22],
        [0.91, 1.0, 0.99, 0.99, 0.93],
        0.25,
        0.93,
        0.17
    ),
    shape!(
        "Rounded steps in the glacial debris",
        [0.14, 0.16, -0.10, 0.15, 0.13],
        [0.84, 1.0, 0.89, 1.0, 0.87],
        0.19,
        1.13,
        0.42
    ),
    shape!(
        "Three-bend recessional moraine",
        [-0.13, 0.17, -0.19, 0.16, -0.09],
        [0.93, 0.95, 0.98, 1.0, 0.88],
        0.26,
        0.92,
        0.64
    ),
];
