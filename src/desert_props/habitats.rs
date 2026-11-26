//! Regional foreground objects. Site rules use developed ground to keep them
//! clear of pavement, water, roofs, cultivated fields, and airstrips.

use crate::world::{Sample, Surface};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(super) enum Feature {
    Ocotillo, YuccaRosette, JoshuaTree, HoneycombSandstone, VarnishedSlabs,
    LupineSpire, PoppyPatch, MulleinStalks, Rabbitbrush, ThistleCrown,
    FallenTrunk, UprootedSnag, FungusStump, SwordFern, BrambleArch,
    GraniteTor, LichenBoulder, ScreeFan, CushionPlant, MoraineErratic,
    Driftwood, WrackWindrow, SeaThrift, MarramGrass, ShingleBerm,
    BogCotton, SkunkCabbage, TussockSedge, Horsetail, MudHummock,
}

const DRYLAND: [Feature; 5] = [
    Feature::Ocotillo, Feature::YuccaRosette, Feature::JoshuaTree,
    Feature::HoneycombSandstone, Feature::VarnishedSlabs,
];
const MEADOW: [Feature; 5] = [
    Feature::LupineSpire, Feature::PoppyPatch, Feature::MulleinStalks,
    Feature::Rabbitbrush, Feature::ThistleCrown,
];
const WOODLAND: [Feature; 5] = [
    Feature::FallenTrunk, Feature::UprootedSnag, Feature::FungusStump,
    Feature::SwordFern, Feature::BrambleArch,
];
const ALPINE: [Feature; 5] = [
    Feature::GraniteTor, Feature::LichenBoulder, Feature::ScreeFan,
    Feature::CushionPlant, Feature::MoraineErratic,
];
const COAST: [Feature; 5] = [
    Feature::Driftwood, Feature::WrackWindrow, Feature::SeaThrift,
    Feature::MarramGrass, Feature::ShingleBerm,
];
const WETLAND: [Feature; 5] = [
    Feature::BogCotton, Feature::SkunkCabbage, Feature::TussockSedge,
    Feature::Horsetail, Feature::MudHummock,
];

impl Feature {
    pub(super) fn on(ground: Sample, key: u64, region: u64) -> Option<Self> {
        let plants = match ground.surface {
            Surface::Desert | Surface::DryWash => &DRYLAND,
            Surface::DuneSand if ground.elevation > 10.0 => &DRYLAND,
            Surface::Sand | Surface::DuneSand if ground.elevation >= 0.42 => &COAST,
            Surface::Forest => &WOODLAND,
            Surface::Grass if ground.elevation < 55.0 => &MEADOW,
            Surface::Grass | Surface::Rock | Surface::AlpineMeadow | Surface::ScreeEdge
            | Surface::ScreeFine | Surface::ScreeCoarse | Surface::GlacierMoraine => &ALPINE,
            Surface::WetMeadow | Surface::ReedMarsh => &WETLAND,
            Surface::RiverSilt if ground.elevation >= 0.45 => return Some(Self::MudHummock),
            _ => return None,
        };
        // Each patch favors a small group of related objects. The cell still
        // provides an independent choice so an entire slope is not one model.
        let rare = ((key >> 32) & 3) == 0;
        let index = (if rare { key >> 35 } else { (region >> 12) + ((key >> 36) & 1) }) % 5;
        Some(plants[index as usize])
    }

    pub(super) fn draw(self, variant: u8, emit: impl FnMut([[f32; 3]; 3], [u8; 3])) {
        let yaw = f32::from(variant & 15) * (core::f32::consts::TAU / 16.0);
        let (sin, cos) = crate::math::sin_cos(yaw);
        let spread = 0.87 + f32::from(variant >> 4 & 3) * 0.075;
        let rise = 0.89 + f32::from(variant >> 6) * 0.075;
        let mut emit = emit;
        let mut model = Mesh(|faces: [[f32; 3]; 3], color| {
            emit(faces.map(|[x, y, z]| [(x * cos + z * sin) * spread,
                y * rise, (z * cos - x * sin) * spread]), color);
        });
        use Feature::*;
        match self {
            Ocotillo => {
                for [x, z, y] in [[-0.8, -0.4, 2.9], [-0.4, 0.8, 2.3], [0.3, -0.7, 2.5], [0.9, 0.5, 2.75], [0.1, 0.15, 3.35]] {
                    model.stalk([x * 0.12, 0.0, z * 0.12], [x, y, z], 0.10, [96, 119, 65]);
                    model.flower([x, y + 0.09, z], 0.17, [214, 79, 52]);
                }
            }
            YuccaRosette => {
                model.rosette([0.0, 0.0, 0.0], 1.05, 1.13, [133, 148, 82]);
                model.rosette([0.66, 0.0, 0.22], 0.66, 0.48, [106, 128, 72]);
                model.stalk([0.0, 0.0, 0.0], [0.07, 1.7, 0.05], 0.1, [158, 151, 93]);
                model.flower([0.07, 1.7, 0.05], 0.28, [222, 215, 159]);
            }
            JoshuaTree => {
                model.wood([0.0, 0.0, 0.0], [0.04, 2.25, -0.05], 0.25, [104, 91, 69]);
                for [x, y, z] in [[-0.89, 2.68, 0.29], [0.7, 2.86, 0.39], [0.08, 3.17, -0.18]] {
                    model.wood([0.03, 1.57, -0.03], [x, y, z], 0.17, [125, 103, 68]);
                    model.rosette([x, y, z], 0.58, 0.42, [130, 145, 78]);
                }
            }
            HoneycombSandstone => {
                model.stone([-0.42, 0.0, 0.0], [0.83, 1.54, 0.60], [173, 116, 76]);
                model.stone([0.42, 0.0, 0.17], [0.65, 0.91, 0.50], [195, 133, 85]);
                for [x, y, z] in [[-0.7, 0.83, -0.44], [-0.33, 0.53, -0.48], [0.33, 0.45, -0.23]] {
                    model.flower([x, y, z], 0.17, [91, 64, 54]);
                }
            }
            VarnishedSlabs => {
                model.stone([-0.31, 0.0, 0.05], [1.47, 0.28, 0.85], [90, 69, 61]);
                model.stone([0.56, 0.20, -0.15], [0.82, 0.36, 0.62], [116, 80, 59]);
                model.stone([-0.68, 0.18, -0.38], [0.55, 0.25, 0.44], [134, 89, 66]);
            }
            LupineSpire => {
                for [x, z, h] in [[-0.46, 0.15, 1.48], [0.48, 0.26, 1.18], [0.04, -0.35, 1.80]] {
                    model.rosette([x, 0.0, z], 0.38, 0.30, [73, 131, 87]);
                    model.stalk([x, 0.0, z], [x, h, z], 0.06, [76, 133, 82]);
                    model.stone([x, h - 0.75, z], [0.16, 0.76, 0.16], [144, 123, 205]);
                }
            }
            PoppyPatch => {
                for [x, y, z] in [[-0.66, 0.62, -0.12], [-0.3, 0.86, 0.56], [0.14, 0.74, -0.52],
                                  [0.47, 0.91, 0.26], [0.71, 0.58, -0.49]] {
                    model.stalk([x - 0.1, 0.0, z], [x, y, z], 0.07, [105, 145, 66]);
                    model.flower([x, y, z], 0.28, [239, 153, 55]);
                }
            }
            MulleinStalks => {
                for [x, z, h] in [[-0.49, 0.05, 2.3], [0.40, 0.26, 1.72]] {
                    model.rosette([x, 0.0, z], 0.64, 0.31, [159, 166, 120]);
                    model.stalk([x, 0.0, z], [x + 0.1, h, z], 0.09, [139, 144, 85]);
                    model.stone([x + 0.076, h - 0.64, z], [0.12, 0.67, 0.12], [211, 181, 85]);
                }
            }
            Rabbitbrush => {
                model.rosette([0.0, 0.0, 0.0], 0.88, 0.94, [109, 127, 89]);
                for [x, y, z] in [[-0.58, 0.75, 0.38], [0.56, 0.93, 0.15], [0.04, 1.11, -0.37], [-0.47, 0.75, -0.38]] {
                    model.stone([x, y - 0.26, z], [0.29, 0.31, 0.27], [203, 166, 68]);
                }
            }
            ThistleCrown => {
                model.rosette([0.0, 0.0, 0.0], 0.65, 0.6, [101, 130, 91]);
                for [x, y, z] in [[-0.43, 1.07, -0.2], [0.19, 1.54, 0.15], [0.62, 1.11, 0.31]] {
                    model.stalk([0.0, 0.0, 0.0], [x, y, z], 0.085, [110, 142, 85]);
                    model.flower([x, y + 0.08, z], 0.22, [174, 98, 163]);
                }
            }
            FallenTrunk => {
                model.wood([-1.9, 0.16, -0.28], [1.65, 0.39, 0.43], 0.32, [119, 87, 60]);
                model.wood([-0.16, 0.31, 0.14], [0.41, 1.01, -0.61], 0.16, [103, 82, 57]);
                model.flower([1.68, 0.4, 0.46], 0.29, [181, 134, 87]);
            }
            UprootedSnag => {
                model.wood([-0.39, 0.0, 0.0], [0.34, 2.6, -0.34], 0.22, [114, 92, 70]);
                model.wood([0.02, 1.09, -0.18], [-0.95, 1.90, 0.6], 0.13, [135, 111, 83]);
                for [x, z] in [[-0.94, 0.1], [-0.63, -0.8], [0.57, 0.45]] {
                    model.wood([x, 0.0, z], [-0.35, 0.62, 0.04], 0.17, [100, 79, 59]);
                }
            }
            FungusStump => {
                model.wood([0.0, 0.0, 0.0], [-0.11, 1.28, 0.04], 0.48, [115, 82, 57]);
                model.flower([-0.11, 1.28, 0.04], 0.40, [171, 123, 78]);
                for [x, y, z] in [[0.48, 0.55, 0.13], [0.49, 0.82, -0.24], [-0.51, 0.34, 0.27]] {
                    model.stone([x, y, z], [0.32, 0.15, 0.24], [194, 151, 105]);
                }
            }
            SwordFern => {
                for [dx, dz] in [[0.95, 0.0], [-0.72, 0.5], [0.34, -0.8], [-0.48, -0.59], [0.27, 0.82]] {
                    model.stalk([0.0, 0.0, 0.0], [dx, 0.88, dz], 0.07, [86, 132, 66]);
                    for scale in [0.43, 0.63, 0.82] {
                        let root = [dx * scale, scale * 0.92, dz * scale];
                        model.stalk(root, [root[0] - dz * 0.24, root[1] + 0.13, root[2] + dx * 0.24], 0.09, [82, 139, 73]);
                        model.stalk(root, [root[0] + dz * 0.24, root[1] + 0.13, root[2] - dx * 0.24], 0.09, [104, 149, 74]);
                    }
                }
            }
            BrambleArch => {
                model.wood([-0.91, 0.0, 0.25], [-0.30, 1.2, 0.03], 0.14, [112, 96, 63]);
                model.wood([0.07, 1.22, 0.03], [1.02, 0.0, -0.32], 0.15, [100, 87, 62]);
                model.stone([-0.25, 0.65, 0.05], [0.73, 0.58, 0.49], [91, 133, 79]);
                model.stone([0.51, 0.37, -0.19], [0.68, 0.62, 0.51], [67, 108, 66]);
            }
            GraniteTor => {
                model.stone([-0.57, 0.0, 0.10], [0.79, 2.29, 0.72], [163, 155, 145]);
                model.stone([0.54, 0.0, -0.23], [0.58, 2.51, 0.55], [142, 137, 130]);
                model.stone([0.09, 0.0, 0.64], [0.65, 0.61, 0.47], [185, 174, 164]);
            }
            LichenBoulder => {
                model.stone([0.0, 0.0, 0.0], [1.23, 1.03, 0.93], [132, 123, 111]);
                model.stone([-0.36, 0.61, 0.21], [0.39, 0.25, 0.34], [181, 169, 91]);
                model.stone([0.56, 0.34, -0.18], [0.28, 0.20, 0.29], [154, 157, 90]);
            }
            ScreeFan => {
                for [x, z, w, h] in [[-1.08, 0.58, 0.48, 0.20], [-0.38, 0.06, 0.70, 0.44],
                                      [0.18, -0.55, 0.49, 0.66], [0.88, 0.31, 0.73, 0.30], [-0.62, 0.82, 0.39, 0.25]] {
                    model.stone([x, 0.0, z], [w, h, w * 0.68], [155, 151, 147]);
                }
            }
            CushionPlant => {
                for [x, z, radius] in [[-0.64, 0.06, 0.59], [0.35, -0.38, 0.53], [0.28, 0.48, 0.58], [0.93, 0.12, 0.39]] {
                    model.stone([x, 0.0, z], [radius, radius * 0.66, radius], [109, 137, 80]);
                    model.flower([x, radius * 0.52, z], 0.16, [195, 148, 163]);
                }
            }
            MoraineErratic => {
                model.stone([0.15, 0.0, -0.12], [1.15, 1.93, 1.06], [122, 129, 145]);
                model.stone([-1.07, 0.0, 0.43], [0.50, 0.53, 0.43], [187, 177, 157]);
                model.stone([0.92, 0.0, 0.77], [0.36, 0.29, 0.31], [164, 153, 141]);
            }
            Driftwood => {
                model.wood([-1.52, 0.10, -0.35], [1.41, 0.32, 0.38], 0.20, [166, 143, 111]);
                model.wood([-0.76, 0.22, -0.15], [-1.41, 0.92, 0.65], 0.13, [184, 165, 133]);
                model.wood([0.53, 0.27, 0.2], [1.15, 0.68, -0.79], 0.12, [151, 131, 107]);
            }
            WrackWindrow => {
                for [x, z, h] in [[-1.39, 0.12, 0.14], [-0.65, -0.22, 0.19], [0.14, 0.11, 0.27],
                                  [0.74, 0.39, 0.21], [1.39, 0.16, 0.16]] {
                    model.stone([x, 0.0, z], [0.59, h, 0.24], [123, 100, 67]);
                    model.stalk([x - 0.31, 0.0, z], [x + 0.25, h + 0.18, z - 0.22], 0.08, [143, 119, 71]);
                }
            }
            SeaThrift => {
                model.rosette([0.0, 0.0, 0.0], 0.70, 0.34, [104, 139, 83]);
                for [x, y, z] in [[-0.56, 0.64, 0.08], [0.52, 0.56, 0.18], [0.07, 0.93, -0.25]] {
                    model.stalk([x * 0.7, 0.0, z], [x, y, z], 0.075, [119, 149, 91]);
                    model.flower([x, y, z], 0.23, [211, 140, 173]);
                }
            }
            MarramGrass => {
                model.rosette([-0.47, 0.0, 0.30], 0.86, 1.26, [149, 150, 86]);
                model.rosette([0.52, 0.0, -0.28], 0.75, 0.89, [163, 166, 102]);
                model.stalk([-0.47, 0.0, 0.22], [-0.88, 1.48, 0.06], 0.11, [190, 165, 104]);
            }
            ShingleBerm => {
                for [x, z, w, h] in [[-1.08, 0.18, 0.62, 0.33], [-0.36, -0.16, 0.63, 0.54],
                                      [0.36, 0.11, 0.56, 0.43], [1.07, 0.27, 0.62, 0.30]] {
                    model.stone([x, 0.0, z], [w, h, 0.49], [168, 164, 150]);
                }
                model.stone([0.46, 0.0, -0.41], [0.29, 0.23, 0.32], [107, 119, 119]);
            }
            BogCotton => {
                for [x, y, z] in [[-0.45, 1.09, -0.25], [-0.15, 1.53, 0.35], [0.42, 1.30, 0.05], [0.64, 0.78, -0.27]] {
                    model.stalk([0.5 * x, 0.0, z], [x, y, z], 0.07, [102, 136, 96]);
                    model.stone([x, y - 0.14, z], [0.22, 0.34, 0.18], [226, 223, 194]);
                }
            }
            SkunkCabbage => {
                for [x, y, z] in [[-0.69, 0.66, 0.07], [0.71, 0.57, 0.11], [0.02, 0.82, -0.54]] {
                    model.stalk([0.0, 0.0, 0.0], [x, y, z], 0.33, [103, 143, 67]);
                }
                model.stone([0.15, 0.30, 0.13], [0.17, 0.63, 0.17], [212, 180, 72]);
            }
            TussockSedge => {
                model.stone([-0.14, 0.0, 0.05], [0.80, 0.38, 0.71], [103, 113, 74]);
                model.rosette([-0.14, 0.29, 0.05], 1.02, 0.87, [135, 153, 99]);
                model.rosette([0.77, 0.0, -0.27], 0.46, 0.43, [139, 149, 93]);
            }
            Horsetail => {
                for [x, z, h] in [[-0.51, 0.17, 1.40], [0.02, -0.36, 1.8], [0.53, 0.24, 1.5]] {
                    model.stalk([x, 0.0, z], [x, h, z], 0.09, [96, 143, 83]);
                    for tier in [0.39, 0.69] {
                        model.rosette([x, h * tier, z], 0.35, 0.19, [108, 151, 86]);
                    }
                }
            }
            MudHummock => {
                model.stone([-0.20, 0.0, 0.04], [0.97, 0.31, 0.76], [121, 100, 74]);
                model.stone([0.57, 0.0, -0.29], [0.67, 0.23, 0.49], [145, 120, 88]);
                model.rosette([-0.37, 0.24, 0.1], 0.42, 0.36, [122, 137, 85]);
            }
        }
    }
}

fn tint(color: [u8; 3], light: i16) -> [u8; 3] {
    color.map(|sample| (i16::from(sample) + light).clamp(0, 255) as u8)
}

struct Mesh<F>(F);

impl<F: FnMut([[f32; 3]; 3], [u8; 3])> Mesh<F> {
    fn stalk(&mut self, [x, y, z]: [f32; 3], top: [f32; 3], h: f32, color: [u8; 3]) {
        let waist = [(x + top[0]) * 0.5, (y + top[1]) * 0.5, (z + top[2]) * 0.5];
        (self.0)([[x, y, z], [waist[0] - h, waist[1], waist[2]], top], color);
        (self.0)([[x, y, z], [waist[0], waist[1], waist[2] + h], top], tint(color, -16));
    }

    fn rosette(&mut self, [x, y, z]: [f32; 3], span: f32, lift: f32, color: [u8; 3]) {
        for [dx, dz, up] in [[1.0, 0.06, 0.52], [0.43, 0.83, 0.72], [-0.63, 0.71, 0.87],
                             [-0.97, -0.16, 0.51], [-0.25, -1.0, 0.64], [0.34, -0.51, 1.0]] {
            self.stalk([x, y, z], [x + span * dx, y + lift * up, z + span * dz], span * 0.22, color);
        }
    }

    fn flower(&mut self, [x, y, z]: [f32; 3], r: f32, color: [u8; 3]) {
        let peak = [x, y + r * 0.29, z];
        (self.0)([[x-r, y, z], [x, y, z-r], peak], color);
        (self.0)([[x, y, z-r], [x+r, y, z], peak], tint(color, 15));
        (self.0)([[x+r, y, z], [x, y, z+r], peak], tint(color, -13));
        (self.0)([[x, y, z+r], [x-r, y, z], peak], color);
    }

    fn stone(&mut self, [x, y, z]: [f32; 3], [w, h, d]: [f32; 3], color: [u8; 3]) {
        let bottom = [[x-w, y, z-d*0.44], [x-w*0.32, y, z-d],
                      [x+w*0.88, y, z-d*0.58], [x+w, y, z+d*0.39],
                      [x+w*0.12, y, z+d], [x-w*0.8, y, z+d*0.6]];
        let top = [[x-w*0.52, y+h*0.63, z-d*0.36], [x-w*0.14, y+h*0.87, z-d*0.61],
                   [x+w*0.44, y+h, z-d*0.32], [x+w*0.63, y+h*0.63, z+d*0.14],
                   [x+w*0.06, y+h*0.72, z+d*0.68], [x-w*0.51, y+h*0.63, z+d*0.32]];
        for i in 0..6 {
            let j = (i + 1) % 6;
            let side = tint(color, [-26, 0, 10, -15, -29, -37][i]);
            (self.0)([bottom[i], bottom[j], top[j]], side);
            (self.0)([bottom[i], top[j], top[i]], side);
        }
        for i in 1..5 {
            (self.0)([top[0], top[i], top[i + 1]], tint(color, 12 + i as i16 * 3));
        }
    }

    fn wood(&mut self, a: [f32; 3], b: [f32; 3], r: f32, color: [u8; 3]) {
        let offset = |[x, y, z]: [f32; 3], sign: f32| [x, (y + r * sign).max(0.0), z + r * sign];
        let end = |[x, y, z]: [f32; 3], sign: f32| [x + r * sign, y, z - r * sign * 0.45];
        let [lo, hi] = [offset(a, -0.55), offset(a, 0.57)];
        let [low, high] = [offset(b, -0.48), offset(b, 0.52)];
        (self.0)([lo, low, high], tint(color, -16));
        (self.0)([lo, high, hi], color);
        (self.0)([hi, high, end(b, 1.0)], tint(color, 15));
        (self.0)([hi, end(b, 1.0), end(a, 1.0)], tint(color, 22));
        (self.0)([low, end(b, 1.0), high], tint(color, 30));
    }
}
