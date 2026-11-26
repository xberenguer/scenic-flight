//! Rockfall aprons seated below a rocky face and widened across its lower slope.

use crate::alpine_lakes::ALPINE_TILE_SIZE;
use crate::math::{fraction, mix};
use crate::world::{Sample, Surface, World};

mod columnar;
mod columnar_profiles;
mod talus_profiles;
pub use columnar::ColumnarHeadwall;
pub use columnar_profiles::HeadwallProfile;
pub use talus_profiles::TalusProfile;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreeFan {
    center: [f32; 2],
    downhill: [f32; 2],
    bounds: [f32; 4],
    length: f32,
    width: f32,
    bend: f32,
    profile: u8,
    headwall: Option<ColumnarHeadwall>,
}

impl ScreeFan {
    pub fn generate(world: &World, seed: u64, x: i32, z: i32) -> Option<Self> {
        Self::generate_cooperatively(world, seed, x, z, &mut || {})
    }

    pub(crate) fn generate_cooperatively(
        world: &World,
        seed: u64,
        x: i32,
        z: i32,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        if !(-4096..=4096).contains(&x) || !(-4096..=4096).contains(&z) {
            return None;
        }
        let tile = (u64::from(x as u32) << 32) | u64::from(z as u32);
        let key = mix(seed ^ tile ^ 0x8a1c_0482_9abe_514d);
        if key & 3 != 0 {
            return None;
        }
        let origin = [x, z].map(|i| (i * ALPINE_TILE_SIZE) as f32);
        let at = |[x, z]: [f32; 2]| world.elevation(x, z);
        let jitter = [fraction(key) * 12.0, fraction(mix(key)) * 12.0];
        // Reject ordinary lowland tiles before checking individual slopes.
        if ![
            [102.0, 102.0],
            [102.0, 282.0],
            [282.0, 102.0],
            [282.0, 282.0],
            [192.0, 192.0],
        ]
        .iter()
        .any(|&[x, z]| at([origin[0] + x, origin[1] + z]) >= 84.0)
        {
            return None;
        }
        let mut proofs = 0;
        let first = ((key >> 20) & 63) as usize;
        for index in 0..64 {
            pulse();
            let cell = (first + index * 29) & 63;
            let center = [
                origin[0] + 70.0 + (cell % 8) as f32 * 33.0 + jitter[0],
                origin[1] + 70.0 + (cell / 8) as f32 * 33.0 + jitter[1],
            ];
            let height = at(center);
            if !(74.0..=131.0).contains(&height) {
                continue;
            }
            let [cx, cz] = center;
            let gx = at([cx - 19.0, cz]) - at([cx + 19.0, cz]);
            let gz = at([cx, cz - 19.0]) - at([cx, cz + 19.0]);
            let steep = gx.abs().max(gz.abs());
            if steep < 6.0 {
                continue;
            }
            let downhill = if gx.abs().min(gz.abs()) >= steep * 0.42 {
                [
                    gx.signum() * core::f32::consts::FRAC_1_SQRT_2,
                    gz.signum() * core::f32::consts::FRAC_1_SQRT_2,
                ]
            } else if gx.abs() > gz.abs() {
                [gx.signum(), 0.0]
            } else {
                [0.0, gz.signum()]
            };
            let site_key = mix(key ^ cell as u64);
            let length = 102.0 + 32.0 * fraction(site_key);
            let width = 33.0 + 11.0 * fraction(mix(site_key));
            let h = |p, s| at(local(center, downhill, p, s));
            let apex = h(-length * 0.46, 0.0);
            let headwall = h(-length * 0.68, 0.0);
            let toe = h(length * 0.49, 0.0);
            if headwall < 91.0
                || headwall - apex < 8.0
                || apex - height < 9.0
                || height - toe < 4.5
                || toe < 52.0
                || (height - toe) * 1.38 > apex - height + 3.0
            {
                continue;
            }
            let cross = [
                h(length * 0.30, -width * 0.56),
                h(length * 0.30, width * 0.56),
            ];
            let bend = ((cross[0] - cross[1]) * 0.81).clamp(-9.0, 9.0);
            let lip = width + bend.abs() + 2.0;
            let [ax, az] = downhill.map(f32::abs);
            let extent = [ax * length * 0.54 + az * lip, az * length * 0.54 + ax * lip];
            let mut bounds = [
                cx - extent[0],
                cz - extent[1],
                cx + extent[0],
                cz + extent[1],
            ];
            if (0..2).any(|j| {
                bounds[j] < origin[j] + 2.0
                    || bounds[j + 2] > origin[j] + ALPINE_TILE_SIZE as f32 - 2.0
            }) {
                continue;
            }
            if proofs == 4 {
                break;
            }
            proofs += 1;
            let p = local(center, downhill, -length * 0.68, 0.0);
            if !matches!(
                world.sample(p[0], p[1]).surface,
                Surface::Rock | Surface::Snow
            ) {
                continue;
            }
            let mut last = apex;
            let dry_lane = [-0.27, 0.0, 0.24, 0.46].into_iter().all(|distance| {
                let d = length * distance;
                let axis = h(d, 0.0);
                let lane = [h(d, -width * 0.40), h(d, width * 0.40)];
                let clear = axis <= last + 1.0
                    && lane[0].min(lane[1]) > axis - 12.0
                    && lane[0].max(lane[1]) < axis + 12.0;
                last = axis;
                clear
            });
            if dry_lane {
                let headwall = (site_key & 2 == 0)
                    .then(|| ColumnarHeadwall::survey(world, site_key, center, downhill, length, pulse))
                    .flatten()
                    .filter(|face| {
                        let area = face.bounds();
                        (0..2).all(|j| area[j] >= origin[j] + 2.0
                            && area[j + 2] <= origin[j] + ALPINE_TILE_SIZE as f32 - 2.0)
                    });
                if let Some(headwall) = &headwall {
                    let area = headwall.bounds();
                    for axis in 0..2 {
                        bounds[axis] = bounds[axis].min(area[axis]);
                        bounds[axis + 2] = bounds[axis + 2].max(area[axis + 2]);
                    }
                }
                return Some(Self {
                    center,
                    downhill,
                    bounds,
                    length,
                    width,
                    bend,
                    profile: ((mix(site_key ^ 0xff49_542d_665b_0ab6) >> 24) % TalusProfile::ALL.len() as u64) as u8,
                    headwall,
                });
            }
        }
        None
    }

    pub fn center(&self) -> [f32; 2] {
        self.center
    }
    pub fn downhill(&self) -> [f32; 2] {
        self.downhill
    }
    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }
    pub fn headwall(&self) -> Option<&ColumnarHeadwall> {
        self.headwall.as_ref()
    }

    pub fn profile(&self) -> TalusProfile {
        TalusProfile::ALL[usize::from(self.profile)]
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        let [left, near, right, far] = self.bounds;
        if !(left..=right).contains(&x)
            || !(near..=far).contains(&z)
            || !(51.0..=150.0).contains(&ground.elevation)
            || !matches!(ground.surface, Surface::Grass | Surface::Rock)
        {
            return ground;
        }
        let [dx, dz] = [x - self.center[0], z - self.center[1]];
        let [ax, az] = self.downhill;
        let t = (dx * ax + dz * az) / self.length + 0.47;
        if !(0.0..1.0).contains(&t) {
            return ground;
        }
        let centerline = self.bend * t * t;
        let side = (dz * ax - dx * az - centerline) / (self.width * (0.20 + 0.80 * t));
        let (coverage, deposit, coarse) = self.profile().study(t, side);
        if coverage <= 0.10 {
            return ground;
        }
        let surface = if coverage < 0.48 {
            if ground.surface == Surface::Rock {
                Surface::Rock
            } else {
                Surface::ScreeEdge
            }
        } else if coarse {
            Surface::ScreeCoarse
        } else {
            Surface::ScreeFine
        };
        Sample {
            elevation: ground.elevation + deposit,
            surface,
        }
    }
}

fn local([x, z]: [f32; 2], [ax, az]: [f32; 2], along: f32, across: f32) -> [f32; 2] {
    [x + ax * along - az * across, z + az * along + ax * across]
}
