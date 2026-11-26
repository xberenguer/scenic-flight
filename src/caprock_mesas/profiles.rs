//! Erosion profiles for pairs of remnants cut from one sandstone shoulder.

use crate::math::smoothstep;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MesaProfile(u8);

impl MesaProfile {
    pub const COUNT: usize = NAMES.len();

    pub const fn from_index(index: usize) -> Option<Self> {
        if index < Self::COUNT { Some(Self(index as u8)) } else { None }
    }

    pub const fn name(self) -> &'static str {
        NAMES[self.0 as usize]
    }

    pub(super) fn pair(key: u64) -> [Self; 2] {
        let group = (key % 5) as u8 * 5;
        let main = ((key >> 12) % 5) as u8;
        let companion = (main + 1 + ((key >> 19) & 3) as u8) % 5;
        [Self(group + main), Self(group + companion)]
    }

    /// Normalized crown and bench height. The caller tapers the enclosing footprint.
    pub(super) fn crown(self, u: f32, v: f32, rim: f32) -> f32 {
        let [x, z] = [u.abs(), v.abs()];
        let core = shoulder(rim, 0.76, 0.25);
        match self.0 {
            // Broad remnants with distinct cap outlines.
            0 => core.max(0.77 * lobe(u - 0.40, v + 0.18, [0.46, 0.31])),
            1 => shoulder(x * 0.79 + z * (0.82 + 0.63 * u), 0.76, 0.24),
            2 => shoulder(x * 0.74 + (v - 0.37 * u).abs() * 1.21, 0.73, 0.24),
            3 => shoulder(x.max((v + 0.42 * x).abs() * 1.17), 0.75, 0.24),
            4 => shoulder(x.max(z * (1.22 + 0.68 * u)), 0.71, 0.27),

            // Cuts that end on the plateau or leave a central horn.
            5 => core * (1.0 - 0.76 * notch(u, v - 0.14, 0.20)),
            6 => core * (1.0 - 0.63 * notch(v, x - 0.36, 0.15)),
            7 => core * (1.0 - 0.83 * notch(-u, v - 0.24 - u * 0.31, 0.16)),
            8 => core * (1.0 - 0.58 * notch(z, u + 0.21 + v * 0.22, 0.18)),
            9 => core * (1.0 - 0.73 * notch(u.max(v), (u - v).abs() - 0.30, 0.17)),

            // Two-tier ledges and benches. A flat rim lies below the upper cap.
            10 => terrace(rim, 0.53, 0.80, 0.47),
            11 => terrace((u + 0.09).abs().max(z * 1.30), 0.47, 0.83, 0.55),
            12 => terrace(rim + 0.23 * v, 0.53, 0.82, 0.38),
            13 => terrace((x * 1.24).max((v - 0.25 * u).abs()), 0.42, 0.74, 0.62),
            14 => terrace(x.max(z) + 0.43 * x.min(z), 0.49, 0.87, 0.51),

            // Long divides. Cross-strike erosion controls their shoulders.
            15 => shoulder((x * 0.85).max(z * 2.10), 0.76, 0.42),
            16 => shoulder((x * 0.84).max((v - 0.45 * u).abs() * 2.25), 0.74, 0.34),
            17 => shoulder((x * 0.82).max((v + 0.58 * x).abs() * 2.02), 0.73, 0.36),
            18 => shoulder((z * 0.85).max((u + 0.46 * v * v).abs() * 2.55), 0.78, 0.40),
            19 => shoulder(x * 0.83 + z * 1.48 * (0.75 + x), 0.84, 0.47),

            // Smaller towers rise separately from the common footslope.
            20 => lobe(u + 0.11, v, [0.50, 0.54]).max(0.78 * lobe(u - 0.65, v - 0.23, [0.27, 0.26])),
            21 => lobe(u, v + 0.15, [0.39, 0.54]).max(0.82 * lobe(u + 0.41, v - 0.54, [0.24, 0.29])),
            22 => lobe(u, v, [0.56, 0.26]).max(0.73 * lobe(u - 0.04, v + 0.60, [0.43, 0.22])),
            23 => lobe(u + 0.10, v + 0.09, [0.32, 0.43])
                .max(0.82 * lobe(u - 0.51, v - 0.05, [0.23, 0.26]))
                .max(0.66 * lobe(u + 0.32, v - 0.57, [0.20, 0.25])),
            _ => lobe(u + 0.09, v + 0.19, [0.33, 0.53])
                .max(0.73 * lobe(u - 0.47, v - 0.34, [0.31, 0.43])),
        }
    }
}

fn shoulder(rim: f32, edge: f32, fade: f32) -> f32 {
    smoothstep((edge - rim) / fade)
}

fn lobe(u: f32, v: f32, [width, depth]: [f32; 2]) -> f32 {
    let [x, z] = [u.abs() / width, v.abs() / depth];
    smoothstep((1.0 - x.max(z) - 0.27 * x.min(z)) * 1.86)
}

fn notch(inward: f32, cross: f32, width: f32) -> f32 {
    smoothstep((inward - 0.12) * 4.8) * smoothstep((width - cross.abs()) * 10.0)
}

fn terrace(radius: f32, inner: f32, outer: f32, shelf: f32) -> f32 {
    shelf * shoulder(radius, outer, 0.15) + (1.0 - shelf) * shoulder(radius, inner, 0.17)
}

const NAMES: [&str; 25] = [
    "Broad cap with a spur", "Flaring tableland", "Diagonal sandstone slab",
    "Curved escarpment", "Blunt sandstone prow",
    "Rim cleft", "Paired gullies", "Diagonal ravine",
    "Double-ended notch", "Branching canyon head",
    "Broad lower bench", "Offset double ledge", "Dipping terrace",
    "Long intermediate shelf", "Angular tiered remnant",
    "Narrow sandstone divide", "Oblique sandstone spine", "Horseshoe divide",
    "Bowed fin", "Knife-edge ridge",
    "Mesa and standing stone", "Offset pillar and cap", "Split sandstone fins",
    "Three sandstone towers", "Staggered buttresses",
];
