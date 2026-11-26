//! Dry-season sediment benches on the inside of surveyed river bends.

use crate::math::{fraction, mix, smoothstep};
use crate::watershed::{Reach, RiverSample, Watershed, REGION_SIZE};
use crate::world::{Sample, Surface, World};

mod profiles;
pub use profiles::PointBarProfile;

const KNOTS: usize = 9;
const SECTIONS: f32 = (KNOTS - 1) as f32;
const BANK_MARGIN: f32 = 0.40;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PointBar {
    contact: [f32; 2],
    axis: [f32; 2],
    inward: [f32; 2],
    inverse_length: f32,
    bank: [f32; KNOTS],
    level: [f32; KNOTS],
    spread: f32,
    design: u8,
    bounds: [f32; 4],
}

impl PointBar {
    pub(crate) fn survey(
        world: &World,
        water: &Watershed,
        mut reserved: impl FnMut(f32, f32) -> bool,
        mut excluded: impl FnMut([f32; 4]) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let outlet = water.streams().first()?.outlet;
        let reach_key = (u64::from(outlet[0].to_bits()) << 32) | u64::from(outlet[1].to_bits());
        if mix(reach_key ^ 0xbd56_01bc_b03f_1689) & 7 != 0 {
            return None;
        }
        let mut attempted = 0;
        for a in water.reaches().filter(|reach| !reach.reaches_sea) {
            pulse();
            let turn = a.end;
            if world.sample(turn[0], turn[1]).surface != Surface::Desert {
                continue;
            }
            let key = mix((u64::from(turn[0].to_bits()) << 32) | u64::from(turn[1].to_bits()));
            let Some(b) = water
                .reaches()
                .find(|next| next.start == turn && !next.reaches_sea)
            else {
                continue;
            };
            let Some(bar) = Self::fit(world, water, a, b, key) else {
                continue;
            };
            if excluded(bar.bounds) {
                continue;
            }
            attempted += 1;
            if bar.prove(world, water, &mut reserved, pulse) {
                return Some(bar);
            }
            if attempted == 5 {
                return None;
            }
        }
        None
    }

    fn fit(world: &World, water: &Watershed, a: Reach, b: Reach, key: u64) -> Option<Self> {
        let start = a.position(0.04);
        let end = b.position(0.96);
        let first = unit(sub(a.end, start))?;
        let second = unit(sub(end, a.end))?;
        let curve = first[0] * second[1] - first[1] * second[0];
        if curve.abs() < 0.28 || dot(first, second) < 0.46 {
            return None;
        }
        let (axis, length) = basis(sub(end, start))?;
        if length < 49.0 {
            return None;
        }
        let inward = if curve > 0.0 {
            [-axis[1], axis[0]]
        } else {
            [axis[1], -axis[0]]
        };
        let apex = add(a.end, scale(inward, 7.0));
        let river = water.sample(a.end[0], a.end[1])?;
        let base = world.sample(apex[0], apex[1]);
        if base.surface != Surface::Desert || !(0.4..=5.8).contains(&(base.elevation - river.level))
        {
            return None;
        }
        let spread = 8.5 + fraction(mix(key ^ 0x2bf8)) * 5.0;
        let mut bar = Self {
            contact: start,
            axis,
            inward,
            inverse_length: 1.0 / length,
            bank: [0.0; KNOTS],
            level: [0.0; KNOTS],
            spread,
            design: ((mix(key ^ 0xe654_6e22_e52d_b415) >> 24) % PointBarProfile::ALL.len() as u64) as u8,
            bounds: [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ],
        };
        let chain = |t: f32| {
            if t <= 1.0 {
                a.position(t)
            } else {
                b.position(t - 1.0)
            }
        };
        for index in 0..KNOTS {
            let forward = index as f32 * (length / SECTIONS);
            let mut lower = 0.04;
            let mut upper = 1.96;
            for _ in 0..12 {
                let middle = (lower + upper) * 0.5;
                if dot(sub(chain(middle), start), axis) < forward {
                    lower = middle;
                } else {
                    upper = middle;
                }
            }
            let flow = chain((lower + upper) * 0.5);
            let side = dot(sub(flow, start), inward);
            let probe = |v| {
                let [x, z] = bar.point(forward, v);
                water.sample(x, z)
            };
            let channel = probe(side).filter(|sample| sample.is_channel())?;
            let mut wet = side;
            let mut dry = wet + channel.half_width * 1.6 + 4.0;
            if probe(dry).is_some_and(RiverSample::is_channel) {
                return None;
            }
            for _ in 0..9 {
                let middle = (dry + wet) * 0.5;
                if probe(middle).is_some_and(RiverSample::is_channel) {
                    wet = middle;
                } else {
                    dry = middle;
                }
            }
            bar.bank[index] = (wet + dry) * 0.5;
            bar.level[index] = channel.level;
            // The row envelope can be wider than its bounding knots between
            // samples. Reserve the full cross-section at every knot.
            for offset in [0.0, BANK_MARGIN + spread] {
                let [x, z] = bar.point(forward, bar.bank[index] + offset);
                let [l, n, r, f] = bar.bounds;
                bar.bounds = [l.min(x), n.min(z), r.max(x), f.max(z)];
            }
        }
        let region = start.map(|p| {
            let v = p / REGION_SIZE as f32;
            let tile = v as i32;
            (tile - i32::from(v < tile as f32)) as f32 * REGION_SIZE as f32
        });
        if !(0..2).all(|i| {
            bar.bounds[i] >= region[i] + 1.0
                && bar.bounds[i + 2] < region[i] + REGION_SIZE as f32 - 1.0
        }) {
            return None;
        }
        Some(bar)
    }

    fn prove(
        self,
        world: &World,
        water: &Watershed,
        reserved: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        let mut near_water = 0;
        let mut low_bank = 0;
        for row in 0..=37 {
            pulse();
            let u = row as f32 / 37.0;
            let (edge, level, width) = self.profile(u);
            let fwd = u / self.inverse_length;
            let [x, z] = self.point(fwd, edge - 0.95);
            near_water += usize::from(water.sample(x, z).is_some_and(RiverSample::is_channel));
            let steps = (width * 1.1) as usize + 1;
            for column in 0..=steps {
                let d = width * column as f32 / steps as f32;
                let [x, z] = self.point(fwd, edge + BANK_MARGIN + d);
                let sample = world.sample(x, z);
                if sample.surface != Surface::Desert
                    || !(0.22..=5.8).contains(&(sample.elevation - level))
                    || water.sample(x, z).is_some_and(RiverSample::is_channel)
                    || reserved(x, z)
                {
                    return false;
                }
                if column == 0 && (sample.elevation - level) <= 3.2 {
                    low_bank += 1;
                }
            }
        }
        near_water >= 32 && low_bank >= 28
    }

    fn profile(&self, u: f32) -> (f32, f32, f32) {
        let position = u * SECTIONS;
        let index = (position as usize).min(KNOTS - 2);
        let t = position - index as f32;
        let read = |values: &[f32; KNOTS]| values[index] + (values[index + 1] - values[index]) * t;
        let width = 1.8 + (self.spread - 1.8) * smoothstep(4.0 * u * (1.0 - u));
        (read(&self.bank), read(&self.level), width)
    }

    pub(crate) fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub(crate) fn design(&self) -> PointBarProfile {
        PointBarProfile::ALL[usize::from(self.design)]
    }

    fn point(&self, forward: f32, lateral: f32) -> [f32; 2] {
        add(
            self.contact,
            add(scale(self.axis, forward), scale(self.inward, lateral)),
        )
    }

    pub(crate) fn overlay(
        &self,
        x: f32,
        z: f32,
        developed: Sample,
        bank: Sample,
        water: Option<RiverSample>,
    ) -> Sample {
        let [l, n, r, f] = self.bounds;
        if !(l..r).contains(&x)
            || !(n..f).contains(&z)
            || developed.surface != Surface::Desert
            || water.is_some_and(RiverSample::is_channel)
        {
            return bank;
        }
        let offset = sub([x, z], self.contact);
        let along = dot(offset, self.axis) * self.inverse_length;
        if !(0.0..1.0).contains(&along) {
            return bank;
        }
        let (outer, level, surveyed_width) = self.profile(along);
        let design = self.design();
        let width = surveyed_width * design.width(along);
        let across = dot(offset, self.inward) - outer - BANK_MARGIN;
        let rise = bank.elevation - level;
        if !(0.0..width).contains(&across) || !(0.22..5.8).contains(&rise) {
            return bank;
        }
        let strength = smoothstep(6.5 * along.min(1.0 - along))
            * smoothstep(across * 1.1)
            * smoothstep((width - across) * 0.47)
            * smoothstep((rise - 0.22) * 2.6)
            * smoothstep((5.8 - rise) * 0.55);
        let (relief, material) = design.sediment(along, across / width);
        let target = 0.96 + across * 0.097 + relief;
        let adjusted = (target - rise).clamp(-1.15, 0.44) * strength;
        Sample {
            elevation: bank.elevation + adjusted,
            surface: if strength >= 0.38 {
                if across < 1.35 {
                    Surface::Gravel
                } else {
                    material
                }
            } else {
                bank.surface
            },
        }
    }
}

fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}
fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn scale(a: [f32; 2], b: f32) -> [f32; 2] {
    [a[0] * b, a[1] * b]
}
fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn unit(delta: [f32; 2]) -> Option<[f32; 2]> {
    basis(delta).map(|(axis, _)| axis)
}

fn basis(delta: [f32; 2]) -> Option<([f32; 2], f32)> {
    let d2 = dot(delta, delta);
    if d2 < 0.001 {
        return None;
    }
    let mut length = delta[0].abs().max(delta[1].abs());
    for _ in 0..4 {
        length = 0.5 * (length + d2 / length);
    }
    Some((scale(delta, 1.0 / length), length))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facility_stream::FacilityStream;
    use crate::floodplain::FloodplainKind;
    use crate::land_stream::LandStream;
    use crate::render::Camera;
    use crate::river_stream::RiverStream;
    use crate::watershed::WatershedWork;

    #[test]
    fn ordinary_river_startup_reserves_a_continuous_dry_bar_and_keeps_the_channel() {
        let world = World::new(0);
        let key = (10, -1);
        let camera = Camera {
            x: 10532.189,
            z: -366.9119,
            altitude: 86.901115,
            heading: 2.3,
            pitch: 0.5846012,
            bank: 0.023,
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        assert!(land.preflight(camera, false));
        let mut facilities = FacilityStream::<8>::new(world, 0);
        facilities.preflight(&land);
        let mut rivers = RiverStream::<3>::new(world, 0);
        rivers.preflight(&land, &facilities, camera, &mut WatershedWork::new());
        let river = rivers.planned(key).flatten().unwrap();
        let water = river.watershed();
        let bar = PointBar::survey(&world, water, |_, _| false, |_| false, &mut || {}).unwrap();
        let [l, n, r, f] = bar.bounds();
        assert!(river
            .floodplain()
            .sites()
            .any(|(kind, bounds)| kind == FloodplainKind::PointBar && bounds == bar.bounds()));
        assert!(river.floodplain().intersects(bar.bounds()));
        assert!(rivers.reserves_scenery(bar.bounds()));
        let elsewhere = PointBar::survey(
            &world,
            water,
            |x, z| (l..=r).contains(&x) && (n..=f).contains(&z),
            |_| false,
            &mut || {},
        );
        assert!(!elsewhere.is_some_and(|site| site.bounds() == bar.bounds()));

        let mut painted = 0;
        let mut channel = 0;
        let sampler = rivers.sampler();
        for row in 0..=110 {
            let u = row as f32 / 110.0;
            let (edge, _, width) = bar.profile(u);
            let mut last = 0.0_f32;
            for col in -8..=((width + 1.25) * 4.0) as i32 {
                let d = col as f32 * 0.25;
                let [x, z] = bar.point(u / bar.inverse_length, edge + BANK_MARGIN + d);
                let raw = world.sample(x, z);
                let water = water.sample(x, z);
                let bank = water.map_or(raw, |sample| {
                    crate::river_district::RiverDistrict::cover(sample, raw)
                });
                let formed = river.overlay(x, z, raw);
                assert_eq!(sampler.overlay(x, z, raw), formed);
                if water.is_some_and(RiverSample::is_channel) {
                    assert_eq!(bank, formed);
                    channel += 1;
                }
                let adjustment = formed.elevation - bank.elevation;
                assert!((adjustment - last).abs() < 0.42, "abrupt dry terrace edge");
                last = adjustment;
                if formed.surface == Surface::RiverSilt {
                    painted += 1;
                    let pavement = Sample {
                        surface: Surface::Pavement,
                        ..raw
                    };
                    assert_eq!(sampler.overlay(x, z, pavement), pavement);
                }
            }
        }
        assert!(painted > 1000 && channel > 400);
    }
}
