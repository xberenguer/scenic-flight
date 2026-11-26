//! Parking positions and guidance markings inside an already surveyed apron.

use super::AirfieldMaterial;

#[derive(Clone, Copy)]
struct Stand {
    center: [f32; 2],
    half: [f32; 2],
    slant: f32,
    approach: f32,
}

const fn stand(x: f32, z: f32, width: f32, depth: f32, slant: f32, approach: f32) -> Stand {
    Stand {
        center: [x, z],
        half: [width, depth],
        slant,
        approach,
    }
}

#[derive(Clone, Copy)]
pub struct ApronProfile {
    pub name: &'static str,
    stands: &'static [Stand],
}

const fn apron(name: &'static str, stands: &'static [Stand]) -> ApronProfile {
    ApronProfile { name, stands }
}

impl ApronProfile {
    pub const ALL: [Self; 25] = [
        apron(
            "Twin pull-in stands",
            &[
                stand(-0.52, 0.49, 0.3, 0.31, 0.0, 1.0),
                stand(0.49, 0.49, 0.29, 0.31, 0.0, 1.0),
            ],
        ),
        apron(
            "Three light-aircraft bays",
            &[
                stand(-0.65, 0.49, 0.21, 0.3, 0.0, 1.0),
                stand(0.0, 0.49, 0.22, 0.3, 0.0, 1.0),
                stand(0.65, 0.49, 0.21, 0.3, 0.0, 1.0),
            ],
        ),
        apron(
            "Opposing pairs",
            &[
                stand(-0.5, 0.52, 0.29, 0.28, 0.0, 1.0),
                stand(0.47, 0.52, 0.29, 0.28, 0.0, 1.0),
                stand(-0.51, -0.55, 0.28, 0.24, 0.0, -1.0),
                stand(0.48, -0.55, 0.28, 0.24, 0.0, -1.0),
            ],
        ),
        apron(
            "Staggered parking comb",
            &[
                stand(-0.65, 0.49, 0.21, 0.31, 0.12, 1.0),
                stand(0.06, 0.58, 0.26, 0.26, 0.0, 1.0),
                stand(0.64, -0.53, 0.19, 0.25, -0.08, -1.0),
            ],
        ),
        apron(
            "Diagonal visitor row",
            &[
                stand(-0.61, 0.45, 0.22, 0.3, 0.24, 1.0),
                stand(0.02, 0.49, 0.22, 0.28, 0.24, 1.0),
                stand(0.63, 0.51, 0.21, 0.28, 0.24, 1.0),
            ],
        ),
        apron(
            "Reverse-angle visitor row",
            &[
                stand(-0.63, 0.5, 0.21, 0.29, -0.24, 1.0),
                stand(-0.02, 0.49, 0.22, 0.28, -0.24, 1.0),
                stand(0.61, 0.45, 0.22, 0.3, -0.24, 1.0),
            ],
        ),
        apron(
            "Cargo and visitor bays",
            &[
                stand(-0.26, 0.52, 0.56, 0.31, 0.0, 1.0),
                stand(0.67, -0.55, 0.2, 0.22, 0.06, -1.0),
            ],
        ),
        apron(
            "Wide service stand",
            &[
                stand(0.0, 0.49, 0.77, 0.29, 0.0, 1.0),
                stand(0.64, -0.59, 0.19, 0.21, 0.0, -1.0),
            ],
        ),
        apron(
            "Split operations ramp",
            &[
                stand(-0.44, 0.48, 0.39, 0.31, -0.1, 1.0),
                stand(0.5, -0.51, 0.33, 0.29, 0.12, -1.0),
            ],
        ),
        apron(
            "Bowed stand row",
            &[
                stand(-0.61, 0.52, 0.21, 0.29, -0.22, 1.0),
                stand(0.0, 0.44, 0.24, 0.26, 0.0, 1.0),
                stand(0.61, 0.52, 0.21, 0.29, 0.22, 1.0),
            ],
        ),
        apron(
            "Open training apron",
            &[
                stand(-0.66, 0.51, 0.19, 0.23, 0.12, 1.0),
                stand(0.65, 0.51, 0.19, 0.23, -0.12, 1.0),
                stand(0.0, -0.61, 0.28, 0.21, 0.0, -1.0),
            ],
        ),
        apron(
            "Southern training row",
            &[
                stand(-0.61, -0.49, 0.22, 0.29, -0.2, -1.0),
                stand(0.0, -0.49, 0.22, 0.29, 0.0, -1.0),
                stand(0.61, -0.49, 0.22, 0.29, 0.2, -1.0),
            ],
        ),
        apron(
            "Wide twin taxi stands",
            &[
                stand(-0.43, 0.49, 0.37, 0.31, 0.07, 1.0),
                stand(0.42, -0.48, 0.38, 0.31, -0.1, -1.0),
            ],
        ),
        apron(
            "Corner parking apron",
            &[
                stand(-0.61, 0.55, 0.23, 0.28, 0.0, 1.0),
                stand(0.54, 0.5, 0.29, 0.29, 0.09, 1.0),
                stand(-0.6, -0.56, 0.23, 0.24, -0.08, -1.0),
            ],
        ),
        apron(
            "Departure staging",
            &[
                stand(-0.41, 0.5, 0.43, 0.3, -0.1, 1.0),
                stand(0.63, 0.51, 0.21, 0.3, 0.15, 1.0),
                stand(0.03, -0.53, 0.27, 0.28, 0.0, -1.0),
            ],
        ),
        apron(
            "Offset crosswind stands",
            &[
                stand(-0.6, 0.54, 0.24, 0.27, -0.18, 1.0),
                stand(0.08, 0.49, 0.24, 0.28, -0.18, 1.0),
                stand(0.57, -0.51, 0.27, 0.26, 0.18, -1.0),
            ],
        ),
        apron(
            "Taxi-and-turn pad",
            &[
                stand(-0.5, 0.54, 0.3, 0.29, 0.0, 1.0),
                stand(0.48, 0.49, 0.3, 0.29, 0.19, 1.0),
                stand(0.59, -0.48, 0.21, 0.19, -0.22, -1.0),
                stand(-0.57, -0.57, 0.19, 0.22, 0.0, -1.0),
            ],
        ),
        apron(
            "Forked visitor apron",
            &[
                stand(-0.55, 0.52, 0.3, 0.31, -0.27, 1.0),
                stand(0.56, 0.52, 0.3, 0.31, 0.27, 1.0),
            ],
        ),
        apron(
            "Inward-facing ramp",
            &[
                stand(-0.55, 0.52, 0.28, 0.28, 0.27, 1.0),
                stand(0.55, 0.52, 0.28, 0.28, -0.27, 1.0),
                stand(0.0, -0.61, 0.33, 0.21, 0.0, -1.0),
            ],
        ),
        apron(
            "Single heavy stand",
            &[stand(0.0, 0.51, 0.8, 0.31, -0.06, 1.0)],
        ),
        apron(
            "Double maintenance pad",
            &[
                stand(-0.45, -0.47, 0.34, 0.3, 0.0, -1.0),
                stand(0.45, -0.47, 0.34, 0.3, 0.0, -1.0),
            ],
        ),
        apron(
            "Shallow staggered stands",
            &[
                stand(-0.7, 0.45, 0.16, 0.22, 0.08, 1.0),
                stand(-0.3, 0.59, 0.18, 0.23, 0.1, 1.0),
                stand(0.25, 0.49, 0.22, 0.25, 0.08, 1.0),
                stand(0.71, 0.59, 0.16, 0.22, 0.0, 1.0),
            ],
        ),
        apron(
            "Cargo apron with spare bay",
            &[
                stand(0.23, 0.53, 0.58, 0.28, 0.0, 1.0),
                stand(-0.63, 0.53, 0.2, 0.29, -0.16, 1.0),
                stand(-0.58, -0.58, 0.25, 0.24, 0.14, -1.0),
            ],
        ),
        apron(
            "East service pocket",
            &[
                stand(0.46, 0.52, 0.38, 0.28, 0.13, 1.0),
                stand(0.55, -0.53, 0.32, 0.26, -0.13, -1.0),
                stand(-0.6, 0.56, 0.22, 0.25, 0.0, 1.0),
            ],
        ),
        apron(
            "West service pocket",
            &[
                stand(-0.46, 0.52, 0.38, 0.28, -0.13, 1.0),
                stand(-0.55, -0.53, 0.32, 0.26, 0.13, -1.0),
                stand(0.6, 0.56, 0.22, 0.25, 0.0, 1.0),
            ],
        ),
    ];

    /// Sample normalized along-runway and across-runway apron coordinates.
    pub fn sample(self, u: f32, v: f32) -> AirfieldMaterial {
        if u.abs() >= 1.0 || !(0.13..1.0).contains(&v.abs()) {
            return AirfieldMaterial::Apron;
        }
        let mut surface = AirfieldMaterial::Apron;
        for stand in self.stands {
            let dz = v - stand.center[1];
            let x = (u - stand.center[0] - dz * stand.slant).abs();
            if x >= stand.half[0] || dz.abs() >= stand.half[1] {
                continue;
            }
            let y = dz * stand.approach;
            let side = stand.half[0] - x;
            let back = stand.half[1] - y;
            if back < 0.044 || side < 0.036 || (x < 0.026 && y > -0.08 && y < 0.088) {
                return AirfieldMaterial::ApronMarking;
            }
            surface = AirfieldMaterial::ApronStand;
        }
        surface
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parking_arrangements_have_different_footprints_and_clear_the_transfer_lane() {
        let mut signatures = std::collections::HashSet::new();
        for profile in ApronProfile::ALL {
            let mut footprint = std::vec::Vec::new();
            for z in -20_i32..=20 {
                for x in -20_i32..=20 {
                    let material = profile.sample(x as f32 / 20.0, z as f32 / 20.0);
                    if z.abs() <= 2 || x.abs() == 20 || z.abs() == 20 {
                        assert_eq!(material, AirfieldMaterial::Apron);
                    }
                    footprint.push(material != AirfieldMaterial::Apron);
                }
            }
            assert!(
                footprint.iter().filter(|&&marked| marked).count() > 70,
                "{}",
                profile.name
            );
            assert!(
                signatures.insert(footprint),
                "{} repeats a stand arrangement",
                profile.name
            );
        }
    }
}
