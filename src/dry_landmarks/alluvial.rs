//! Dry alluvial aprons where a confined mountain drainage meets a gentler valley.

use super::{DryLandmark, RAVINE_AXES, Shape, in_dry_tile};
use crate::math::{fraction, mix, smoothstep};
use crate::world::{SEA_LEVEL, Sample, Surface, World};

mod profiles;
pub use profiles::FanProfile;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Fan {
    pub(super) axis: [f32; 2],
    length: f32,
    inverse_length: f32,
    spread: f32,
    bend: f32,
    flare: f32,
    pub(super) profile: u8,
    branches: [Wash; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Wash {
    split: f32,
    toe: f32,
    direction: f32,
    bend: f32,
    inverse_width: f32,
}

impl Wash {
    fn lateral(self, t: f32, spread: f32) -> f32 {
        let travel = (t - self.split).max(0.0);
        let arc = travel * (1.0 - t) * (0.67 - t);
        travel * self.direction * spread * (1.18 - 0.24 * t) + self.bend * arc
    }

    fn score(self, t: f32, across: f32, spread: f32) -> f32 {
        let distance = (across - self.lateral(t, spread)).abs();
        let toe = smoothstep((self.toe - t) * 7.5);
        let snake = (t - self.split).max(0.0) * (1.0 - t);
        let taper = 1.22 - snake * 0.91;
        toe * smoothstep(1.0 - distance * self.inverse_width * taper)
    }
}

impl Fan {
    pub(super) fn survey(
        world: &World,
        origin: [f32; 2],
        key: u64,
        excluded: &mut impl FnMut([f32; 4]) -> bool,
        pulse: &mut impl FnMut(),
    ) -> Option<DryLandmark> {
        if key & 3 == 0 {
            return None;
        }
        let mut proofs = 0;
        for attempt in 0..12 {
            pulse();
            let state = mix(key ^ (0xa17f + attempt));
            let apex = [
                origin[0] + 118.0 + 276.0 * fraction(state),
                origin[1] + 118.0 + 276.0 * fraction(state.rotate_left(27)),
            ];
            let Some(head) = land(world, apex) else {
                continue;
            };
            let length = 89.0 + 38.0 * fraction(mix(state ^ 0x9c12));
            let spread = 34.0 + 16.0 * fraction(mix(state ^ 0x781a));
            let bend = (fraction(mix(state ^ 0x47c3)) - 0.5) * 14.0;
            let profile_id =
                (mix(key ^ 0xd3bc_1d7a_59ca_3310) % FanProfile::ALL.len() as u64) as u8;
            let profile = FanProfile::ALL[usize::from(profile_id)];
            for n in 0..8 {
                pulse();
                let axis = RAVINE_AXES[(n + (state & 7) as usize) & 7];
                let fan = Self::new(axis, length, spread, bend, profile_id, profile);
                let length = fan.length;
                let bend = fan.bend;
                let bounds = fan.bounds(apex);
                if !in_dry_tile(origin, bounds) {
                    continue;
                }
                let at = |distance, offset| land(world, fan.point(apex, distance, offset));
                let (Some(source), Some(middle), Some(toe)) = (
                    at(-26.0, 0.0),
                    at(length * 0.50, 0.0),
                    at(length * 0.94, bend * 0.8),
                ) else {
                    continue;
                };
                let upper_drop = head - middle;
                let lower_drop = middle - toe;
                if !(3.0..=16.0).contains(&(source - head))
                    || !(2.1..=12.5).contains(&upper_drop)
                    || !(0.65..=9.0).contains(&lower_drop)
                    || upper_drop > (source - head) * 1.55
                    || lower_drop > upper_drop * 0.96
                {
                    continue;
                }
                let [Some(left), Some(right)] = [at(-11.0, -14.0), at(-11.0, 14.0)] else {
                    continue;
                };
                if left < head + 0.5
                    || right < head + 0.5
                    || left.max(right) < (source + head) * 0.5 + 0.5
                    || excluded(bounds)
                {
                    continue;
                }
                proofs += 1;
                if fan.prove(world, apex, head, pulse) {
                    return Some(DryLandmark {
                        center: apex,
                        bounds,
                        floor: toe,
                        rim: head,
                        shape: Shape::AlluvialFan(fan),
                    });
                }
                if proofs == 5 {
                    return None;
                }
            }
        }
        None
    }

    fn new(
        axis: [f32; 2],
        length: f32,
        spread: f32,
        bend: f32,
        profile_id: u8,
        profile: FanProfile,
    ) -> Self {
        let length = length * profile.aspect[0];
        let spread = spread * profile.aspect[1];
        Self {
            axis,
            length,
            inverse_length: 1.0 / length,
            spread,
            bend: bend + spread * profile.bend,
            flare: profile.flare,
            profile: profile_id,
            branches: profile.streams.map(|channel| Wash {
                split: channel.split,
                toe: channel.toe,
                direction: channel.direction,
                bend: channel.bend,
                inverse_width: 1.0 / channel.width,
            }),
        }
    }

    fn prove(self, world: &World, apex: [f32; 2], head: f32, pulse: &mut impl FnMut()) -> bool {
        let mut previous = [head; 3];
        for step in 1..=16 {
            pulse();
            let t = step as f32 / 16.0;
            let t_center = self.bend * t * t;
            let width = self.half_width(t);
            for offset in [-1.0, -0.78, -0.49, 0.0, 0.49, 0.78, 1.0] {
                let at = self.point(apex, self.length * t, t_center + width * offset);
                let Some(height) = land(world, at) else {
                    return false;
                };
                if height < SEA_LEVEL + 3.0 || (height - previous[1]).abs() > 5.8 {
                    return false;
                }
            }
            for (index, last) in previous.iter_mut().enumerate() {
                let cross = t_center + self.branches[index].lateral(t, self.spread);
                let Some(height) = land(world, self.point(apex, t * self.length, cross)) else {
                    return false;
                };
                if height > *last + 0.34 {
                    return false;
                }
                *last = height;
            }
        }
        true
    }

    #[inline(never)]
    pub(super) fn overlay(&self, [dx, dz]: [f32; 2], ground: Sample) -> Sample {
        let t = (dx * self.axis[0] + dz * self.axis[1]) * self.inverse_length;
        if !(0.0..1.0).contains(&t) {
            return ground;
        }
        let across = dz * self.axis[0] - dx * self.axis[1] - self.bend * t * t;
        let width = self.half_width(t);
        if across.abs() >= width {
            return ground;
        }
        let margin = smoothstep((width - across.abs()) * 0.17);
        let edge = across / width;
        let toe = 1.0 - (0.09 + 0.16 * self.flare) * edge * edge;
        let apron = margin * smoothstep(t * 16.0) * smoothstep((toe - t) * 9.0);
        if apron == 0.0 {
            return ground;
        }
        let [left, middle, right] = self.branches.map(|wash| wash.score(t, across, self.spread));
        let amount = apron * left.max(middle).max(right);
        let surface = if amount > 0.24 {
            Surface::DryWash
        } else if amount > 0.078 {
            Surface::Gravel
        } else {
            ground.surface
        };
        Sample {
            elevation: ground.elevation + 0.32 * apron - amount * (0.61 + 0.33 * t),
            surface,
        }
    }

    fn half_width(self, t: f32) -> f32 {
        5.8 + self.spread * t * (1.0 + self.flare * (1.0 - t))
    }

    fn point(self, apex: [f32; 2], along: f32, across: f32) -> [f32; 2] {
        let [dx, dz] = self.axis;
        [
            apex[0] + dx * along - dz * across,
            apex[1] + dz * along + dx * across,
        ]
    }

    fn bounds(self, apex: [f32; 2]) -> [f32; 4] {
        let radius = self.half_width(1.0) + self.bend.abs();
        let a = self.point(apex, 0.0, 0.0);
        let b = self.point(apex, self.length, -radius);
        let c = self.point(apex, self.length, radius);
        [
            a[0].min(b[0]).min(c[0]) - 7.0,
            a[1].min(b[1]).min(c[1]) - 7.0,
            a[0].max(b[0]).max(c[0]) + 7.0,
            a[1].max(b[1]).max(c[1]) + 7.0,
        ]
    }
}

fn land(world: &World, point: [f32; 2]) -> Option<f32> {
    let sample = world.sample(point[0], point[1]);
    (sample.surface == Surface::Desert).then_some(sample.elevation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dry_landmarks::DryLandmarkKind;
    use crate::facility_stream::FacilityStream;
    use crate::highway_stream::HighwayStream;
    use crate::land_stream::LandStream;
    use crate::local_routes::{LocalRouteStream, route_reservation};
    use crate::natural_stream::{
        NaturalBudget, NaturalReservation, NaturalStream, highway_reservation_with_nature,
        scenery_reservation_with_highways,
    };
    use crate::render::Camera;
    use crate::river_stream::RiverStream;
    use crate::watershed::WatershedWork;

    #[test]
    fn fan_profiles_have_distinct_footprints_and_channels() {
        let mut footprints = std::collections::HashSet::new();
        for profile in FanProfile::ALL {
            let mut footprint = std::vec::Vec::new();
            for z in (0..132).step_by(3) {
                for x in (-75..75).step_by(3) {
                    let sample = profile.study(x as f32, z as f32);
                    footprint.push(sample.map(|sample| sample.surface as u8));
                }
            }
            assert!(
                footprint
                    .iter()
                    .filter(|&&surface| surface == Some(Surface::DryWash as u8))
                    .count()
                    >= 25,
                "{} has no readable network",
                profile.name,
            );
            assert!(
                footprints.insert(footprint),
                "{} duplicates a profile",
                profile.name
            );
        }
    }

    #[test]
    fn a_seeded_alluvial_fan_keeps_the_washes_dry_and_off_reserved_land() {
        let seed = 1;
        let world = World::new(seed);
        let expected = DryLandmark::generate(&world, seed, 11, 12).unwrap();
        let Shape::AlluvialFan(fan) = expected.shape else {
            panic!("expected a dry mountain-front fan");
        };
        let [left, near, right, far] = expected.bounds();
        let camera = Camera {
            x: (left + right) * 0.5,
            z: (near + far) * 0.5,
            altitude: 90.0,
            heading: 0.0,
            pitch: 0.2,
            bank: 0.0,
        };
        let mut land = LandStream::<8>::new();
        land.reset(world);
        land.preflight(camera, false);
        let mut facilities = FacilityStream::<8>::new(world, seed);
        facilities.preflight(&land);
        let mut rivers = RiverStream::<4>::new(world, seed);
        rivers.preflight(&land, &facilities, camera, &mut WatershedWork::new());
        let mut routes = LocalRouteStream::<6>::new(world, seed);
        routes.preflight(&land, camera, |key, route| {
            route_reservation(&world, &facilities, &rivers, key, route)
        });
        let mut highways = HighwayStream::<6>::new(world, seed);
        let mut nature = NaturalStream::new(world, seed);
        highways.preflight_defer_coast(
            &land,
            camera,
            |pair, link| {
                highway_reservation_with_nature(
                    &land,
                    &facilities,
                    &rivers,
                    &routes,
                    &nature,
                    pair,
                    link,
                )
            },
            || {},
        );
        let reserve = |bounds| {
            scenery_reservation_with_highways(
                &land,
                &facilities,
                &rivers,
                &routes,
                &highways,
                bounds,
            )
        };
        let budget = NaturalBudget {
            alpine_tiles: 0,
            dry_tiles: 12,
        };
        nature.preflight(camera, budget, reserve);
        let placed = nature
            .dry_landmarks()
            .find(|site| **site == expected)
            .unwrap();
        assert_eq!(placed.kind(), DryLandmarkKind::AlluvialFan);

        let apex = placed.center();
        let near_source = fan.point(apex, -26.0, 0.0);
        assert!(world.elevation(near_source[0], near_source[1]) > placed.rim + 3.0);
        assert!(placed.rim > placed.floor_elevation() + 2.75);
        let sampler = nature.sampler();
        for wash in fan.branches {
            let mut previous = placed.rim;
            let end = 0.63_f32.min(wash.toe - 0.16);
            for fraction in [0.0, 0.35, 0.70, 1.0] {
                let t = 0.27 + (end - 0.27) * fraction;
                let cross = fan.bend * t * t + wash.lateral(t, fan.spread);
                let [x, z] = fan.point(apex, t * fan.length, cross);
                let raw = world.sample(x, z);
                let sample = sampler.overlay(x, z, raw);
                assert_eq!(sample.surface, Surface::DryWash);
                assert!((SEA_LEVEL + 2.0..raw.elevation).contains(&sample.elevation));
                assert!(sample.elevation <= previous + 0.35);
                previous = sample.elevation;
            }
        }
        let [x, z] = fan.point(apex, fan.length + 1.0, 0.0);
        let outside = world.sample(x, z);
        assert_eq!(sampler.overlay(x, z, outside), outside);
        for surface in [Surface::RiverWater, Surface::Pavement, Surface::FieldGreen] {
            let protected = Sample {
                surface,
                elevation: placed.rim,
            };
            let [x, z] = fan.point(apex, fan.length * 0.55, 0.0);
            assert_eq!(sampler.overlay(x, z, protected), protected);
        }

        let mut blocked = NaturalStream::new(world, seed);
        blocked.preflight(camera, budget, |bounds| {
            if bounds == expected.bounds() {
                NaturalReservation::Reserved
            } else {
                reserve(bounds)
            }
        });
        assert!(
            blocked
                .dry_landmarks()
                .all(|site| site.bounds() != expected.bounds())
        );
    }
}
