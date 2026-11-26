use super::LandformKind;

/// Artist-authored silhouettes. The same dimensions drive the shore survey,
/// flight clearance, and triangle mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Silhouette {
    // Width, depth, relative height, crown, shoulder, seaward lean.
    Stack([f32; 6]),
    // Each rock gives tangent offset, sea-facing offset, width, depth, height.
    Skerries([[f32; 5]; 3]),
    // Span, opening ratio, depth, height, opening height ratio, skew, crown.
    Arch([f32; 7]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoastalProfile(u8);

impl CoastalProfile {
    pub const COUNT: usize = CATALOG.len();

    pub fn from_index(index: usize) -> Option<Self> {
        (index < Self::COUNT).then_some(Self(index as u8))
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn name(self) -> &'static str {
        CATALOG[self.index()].0
    }

    pub(super) fn silhouette(self) -> Silhouette {
        CATALOG[self.index()].1
    }

    pub(super) fn choose(kind: LandformKind, key: u64) -> Self {
        let (start, count) = match kind {
            LandformKind::Stack => (0, 9),
            LandformKind::Skerries => (9, 8),
            LandformKind::Arch => (17, 8),
        };
        Self(start + (key % count) as u8)
    }
}

use Silhouette::{Arch, Skerries, Stack};

const CATALOG: [(&str, Silhouette); 25] = [
    ("Needle stack", Stack([4.8, 3.6, 1.20, 0.12, 0.54, 0.45])),
    (
        "Flat-topped sea tower",
        Stack([7.1, 5.4, 0.83, 0.82, 0.90, 0.21]),
    ),
    (
        "Seaward leaning pillar",
        Stack([5.3, 4.2, 1.02, 0.38, 0.77, 3.90]),
    ),
    (
        "Ribbed coastal buttress",
        Stack([9.4, 4.4, 0.88, 0.46, 0.96, 0.72]),
    ),
    (
        "Rounded offshore dome",
        Stack([7.8, 6.1, 0.58, 0.29, 0.80, -0.22]),
    ),
    ("Pinched chimney", Stack([5.2, 4.9, 1.07, 0.59, 0.42, 0.92])),
    ("Low anvil stack", Stack([8.0, 5.0, 0.58, 0.87, 0.54, 0.68])),
    ("Knife-edge fin", Stack([10.2, 2.7, 1.02, 0.30, 0.72, 0.82])),
    (
        "Stepped coastal pyramid",
        Stack([8.4, 6.3, 0.75, 0.17, 0.60, 0.28]),
    ),
    (
        "Twin sentinel rocks",
        Skerries([
            [-6.2, -0.4, 4.2, 3.1, 8.5],
            [6.6, 0.9, 4.1, 3.0, 7.5],
            [0.0; 5],
        ]),
    ),
    (
        "Three descending skerries",
        Skerries([
            [-7.6, 0.0, 4.2, 3.3, 7.3],
            [2.5, 0.9, 3.7, 3.1, 4.5],
            [10.5, 1.6, 2.8, 2.4, 2.2],
        ]),
    ),
    (
        "Sawtooth reef",
        Skerries([
            [-7.1, 0.2, 5.0, 2.3, 4.6],
            [0.4, 0.7, 5.4, 2.7, 5.8],
            [7.9, 1.0, 4.7, 2.4, 4.3],
        ]),
    ),
    (
        "Split whale-back reef",
        Skerries([
            [-5.7, -1.0, 7.2, 3.2, 4.7],
            [8.0, 1.2, 3.5, 2.4, 2.2],
            [0.0; 5],
        ]),
    ),
    (
        "Triangular reef cluster",
        Skerries([
            [-3.8, -2.1, 4.6, 3.1, 5.7],
            [4.6, -1.8, 3.8, 3.0, 4.9],
            [0.7, 4.0, 3.5, 3.0, 3.0],
        ]),
    ),
    (
        "Tall stack and tidal ledge",
        Skerries([
            [-4.5, -0.6, 4.3, 3.7, 11.9],
            [5.6, 1.4, 6.8, 2.8, 2.7],
            [0.0; 5],
        ]),
    ),
    (
        "Offset boulder chain",
        Skerries([
            [-9.2, -1.6, 3.2, 2.7, 3.4],
            [-0.5, 1.5, 3.4, 3.2, 5.7],
            [8.1, -0.3, 4.7, 3.9, 4.1],
        ]),
    ),
    (
        "Paired narrow fins",
        Skerries([
            [-5.5, -0.9, 5.9, 2.3, 6.0],
            [4.7, 1.7, 5.9, 2.3, 5.8],
            [0.0; 5],
        ]),
    ),
    (
        "Low shelf arch",
        Arch([16.0, 0.65, 4.3, 9.0, 0.52, 0.35, 0.71]),
    ),
    (
        "High keyhole arch",
        Arch([12.5, 0.63, 3.6, 20.0, 0.75, 0.30, 1.45]),
    ),
    (
        "Thin stone bridge",
        Arch([16.7, 0.68, 3.0, 12.6, 0.80, 0.25, 0.79]),
    ),
    (
        "Leaning sea portal",
        Arch([14.4, 0.60, 3.5, 17.0, 0.69, 3.35, 1.05]),
    ),
    (
        "Heavy-brow arch",
        Arch([15.6, 0.52, 5.2, 15.0, 0.57, -0.60, 0.77]),
    ),
    (
        "Sloping ridge arch",
        Arch([15.0, 0.64, 4.0, 16.5, 0.68, -2.50, 0.69]),
    ),
    (
        "Gothic sea arch",
        Arch([13.9, 0.62, 3.7, 19.5, 0.72, 0.65, 1.96]),
    ),
    (
        "Squat rock window",
        Arch([12.8, 0.48, 4.5, 11.5, 0.56, -0.30, 1.23]),
    ),
];
