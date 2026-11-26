//! Prepare sparse summit landmarks ahead of the chase camera.

use crate::{
    feature_tiles::FeatureTiles,
    lookout_art::{LookoutArt, LookoutKind},
    natural_stream::NaturalReservation,
    render::{screen_region, Camera, ScreenRect},
    summit_sites::{SummitKind, SummitSite, SUMMIT_TILE_SIZE},
    summit_trail::SummitTrail,
    world::World,
    world_mesh::{bounds_in_range, landmark_reach, WorldMesh},
};

#[derive(Clone, Copy)]
pub struct SummitRequest {
    world: World,
    seed: u64,
    tile: [i32; 2],
}

pub struct SummitResult {
    seed: u64,
    tile: [i32; 2],
    site: Option<SurveyedSummit>,
}

impl SummitRequest {
    pub fn prepare(self, pulse: impl FnMut()) -> SummitResult {
        SummitResult {
            seed: self.seed,
            tile: self.tile,
            site: SurveyedSummit::survey(&self.world, self.seed, self.tile, pulse),
        }
    }
}

pub struct SummitStream {
    tiles: FeatureTiles<Resident, 5, 25>,
    seed: u64,
}

struct SurveyedSummit {
    site: SummitSite,
    trail: Option<SummitTrail>,
}

impl SurveyedSummit {
    fn survey(world: &World, seed: u64, tile: [i32; 2], mut pulse: impl FnMut()) -> Option<Self> {
        let site = SummitSite::survey(world, seed, tile, |_| false, &mut pulse)?;
        let trail = SummitTrail::survey(world, &site, |_| false, &mut pulse);
        Some(Self { site, trail })
    }
}

struct Resident {
    plan: SurveyedSummit,
    state: NaturalReservation,
    trail_state: NaturalReservation,
}

impl Resident {
    fn active_trail(&self) -> Option<&SummitTrail> {
        if self.state == NaturalReservation::Clear && self.trail_state == NaturalReservation::Clear
        {
            self.plan.trail.as_ref()
        } else {
            None
        }
    }
}

impl SummitStream {
    pub const fn new() -> Self {
        Self {
            tiles: FeatureTiles::new(SUMMIT_TILE_SIZE),
            seed: 0,
        }
    }

    #[inline(never)]
    pub fn preflight(
        &mut self,
        world: World,
        seed: u64,
        camera: Camera,
        mut reserve: impl FnMut([f32; 4]) -> NaturalReservation,
    ) {
        self.seed = seed;
        self.tiles = FeatureTiles::new(SUMMIT_TILE_SIZE);
        self.tiles.follow_centered(camera.x, camera.z, 25, |x, z| {
            let plan = SurveyedSummit::survey(&world, seed, [x, z], || {})?;
            let state = reserve(plan.site.bounds());
            if state == NaturalReservation::Reserved {
                return None;
            }
            let trail_state = if state == NaturalReservation::Clear {
                plan.trail
                    .as_ref()
                    .map_or(NaturalReservation::Reserved, |trail| {
                        reserve(trail.bounds())
                    })
            } else {
                NaturalReservation::Pending
            };
            Some(Resident {
                plan,
                state,
                trail_state,
            })
        });
    }

    pub fn request(&mut self, world: World, camera: Camera) -> Option<SummitRequest> {
        self.tiles.recenter(camera.x, camera.z);
        let tile = self.tiles.next_unknown_centered()?;
        Some(SummitRequest {
            world,
            seed: self.seed,
            tile,
        })
    }

    pub fn commit(&mut self, result: SummitResult, camera: Camera) {
        if result.seed != self.seed {
            return;
        }
        self.tiles.recenter(camera.x, camera.z);
        let resident = result.site.map(|plan| Resident {
            plan,
            state: NaturalReservation::Pending,
            trail_state: NaturalReservation::Pending,
        });
        self.tiles.complete(result.tile, resident);
    }

    pub fn publish(
        &mut self,
        camera: Camera,
        dimensions: (usize, usize),
        mut reserve: impl FnMut([f32; 4]) -> NaturalReservation,
    ) {
        let reach = landmark_reach(camera.altitude);
        let outside_view = |bounds, mesh_bounds| {
            !bounds_in_range(camera, bounds, reach)
                || screen_region(camera, dimensions, mesh_bounds).is_none()
        };
        for resident in self.tiles.sites_mut() {
            let site = &resident.plan.site;
            if resident.state == NaturalReservation::Pending
                && outside_view(site.bounds(), art(site).mesh_bounds())
            {
                resident.state = reserve(site.bounds());
            }
            // A path can be blocked independently of its lookout. Publish
            // both path and props together, before they enter the frame.
            if resident.state == NaturalReservation::Clear
                && resident.trail_state == NaturalReservation::Pending
            {
                if let Some(trail) = &resident.plan.trail {
                    if outside_view(trail.bounds(), trail.mesh_bounds()) {
                        resident.trail_state = reserve(trail.bounds());
                    }
                }
            }
        }
    }

    pub fn conflicts_with(&self, mut conflicts: impl FnMut([f32; 4]) -> bool) -> bool {
        self.active().any(|site| conflicts(site.bounds()))
            || self
                .tiles
                .sites()
                .filter_map(Resident::active_trail)
                .any(|trail| trail.conflicts_with(&mut conflicts))
    }

    pub fn reserves(&self, [left, near, right, far]: [f32; 4]) -> bool {
        self.conflicts_with(|[a, b, c, d]| left <= c && right >= a && near <= d && far >= b)
    }

    pub fn region(&self, camera: Camera, dimensions: (usize, usize)) -> Option<ScreenRect> {
        self.visible(camera)
            .map(|site| art(site).mesh_bounds())
            .chain(self.visible_trails(camera).map(SummitTrail::mesh_bounds))
            .filter_map(|bounds| screen_region(camera, dimensions, bounds))
            .reduce(ScreenRect::union)
    }

    pub fn draw(&self, camera: Camera, daylight: f32, mesh: &mut WorldMesh<'_>) {
        mesh.with_landmark_reach(landmark_reach(camera.altitude), |mesh| {
            for site in self.visible(camera) {
                art(site).draw(mesh, daylight);
            }
        });
    }

    pub fn draw_trails(
        &self,
        camera: Camera,
        mesh: &mut WorldMesh<'_>,
        ground: impl Fn(f32, f32) -> f32,
    ) {
        for trail in self.visible_trails(camera) {
            trail.draw(mesh, &ground);
        }
    }

    pub fn obstruction(&self, x: f32, z: f32) -> f32 {
        self.tiles
            .at(x, z)
            .filter(|site| site.state == NaturalReservation::Clear)
            .map_or(0.0, |resident| {
                let site = &resident.plan.site;
                let [site_x, ground, site_z] = site.center();
                let distance_squared = (x - site_x) * (x - site_x) + (z - site_z) * (z - site_z);
                let clearance = 1.0 - crate::math::smoothstep((distance_squared - 49.0) / 480.0);
                let height = art(site).height();
                let landmark = (ground + height + 1.0) * clearance;
                let trail = resident
                    .active_trail()
                    .map_or(0.0, |trail| trail.obstruction(x, z));
                landmark.max(trail)
            })
    }

    fn visible(&self, camera: Camera) -> impl Iterator<Item = &SummitSite> {
        self.active().filter(move |site| {
            bounds_in_range(camera, site.bounds(), landmark_reach(camera.altitude))
        })
    }

    fn visible_trails(&self, camera: Camera) -> impl Iterator<Item = &SummitTrail> {
        self.tiles
            .sites()
            .filter_map(Resident::active_trail)
            .filter(move |trail| bounds_in_range(camera, trail.bounds(), 400.0))
    }

    fn active(&self) -> impl Iterator<Item = &SummitSite> {
        self.tiles
            .sites()
            .filter(|site| site.state == NaturalReservation::Clear)
            .map(|site| &site.plan.site)
    }
}

fn art(site: &SummitSite) -> LookoutArt {
    let kind = match site.kind() {
        SummitKind::Watchtower => LookoutKind::Watchtower,
        SummitKind::SummitHut => LookoutKind::SummitHut,
        SummitKind::Landmark(design) => LookoutKind::Landmark(design),
    };
    LookoutArt::new(site.center(), site.heading(), kind, site.variant())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_registers_surveyed_tower_and_respects_exclusion() {
        let mut stream = SummitStream::new();
        let camera = Camera {
            x: -2021.0,
            z: -309.0,
            altitude: 198.721_12,
            heading: 0.0,
            pitch: 0.18,
            bank: 0.0,
        };
        stream.preflight(World::new(6), 6, camera, |_| NaturalReservation::Clear);
        assert!(stream.region(camera, (144, 144)).is_some());
        let site = stream.active().find(|site| site.center()[0] == -2021.0).unwrap();
        assert_eq!(
            stream.obstruction(-2021.0, -229.0),
            site.center()[1] + art(site).height() + 1.0,
        );
        assert_eq!(stream.obstruction(-1900.0, -229.0), 0.0);

        let moved = Camera {
            x: camera.x + SUMMIT_TILE_SIZE as f32,
            ..camera
        };
        let old = stream.request(World::new(6), moved).unwrap().prepare(|| {});
        stream.preflight(World::new(52), 52, camera, |_| NaturalReservation::Reserved);
        stream.commit(old, camera);
        assert!(stream.region(camera, (144, 144)).is_none());
        assert_eq!(stream.obstruction(-2021.0, -229.0), 0.0);
    }

    #[test]
    fn trail_has_independent_reservations_and_waits_until_outside_view() {
        let world = World::new(204);
        let camera = Camera {
            x: 1996.0,
            z: 210.0,
            altitude: 193.0,
            heading: 1.3,
            pitch: 0.51,
            bank: 0.0,
        };
        let origin = [2029.0, 240.6];
        let area = [origin[0], origin[1], origin[0], origin[1]];
        let mut stream = SummitStream::new();
        stream.preflight(world, 204, camera, |[a, b, c, d]| {
            if a <= origin[0] && origin[0] <= c && b <= origin[1] && origin[1] <= d {
                NaturalReservation::Pending
            } else {
                NaturalReservation::Clear
            }
        });
        assert!(stream.region(camera, (144, 144)).is_some());
        assert!(!stream.reserves(area));
        stream.publish(camera, (144, 144), |_| NaturalReservation::Clear);
        assert!(!stream.reserves(area));
        let away = Camera {
            heading: camera.heading + core::f32::consts::PI,
            ..camera
        };
        stream.publish(away, (144, 144), |_| NaturalReservation::Clear);
        assert!(stream.reserves(area));
        assert!(stream.obstruction(origin[0], origin[1]) > 168.0);
    }

    #[test]
    fn pending_tower_waits_until_clear_and_out_of_view() {
        let mut stream = SummitStream::new();
        let camera = Camera {
            x: -2021.0,
            z: -309.0,
            altitude: 198.721_12,
            heading: 0.0,
            pitch: 0.18,
            bank: 0.0,
        };
        let toward_peak = camera;
        let facing_away = Camera {
            heading: core::f32::consts::PI,
            ..camera
        };
        let world = World::new(6);
        stream.preflight(world, 6, camera, |_| NaturalReservation::Pending);
        stream.publish(toward_peak, (144, 144), |_| NaturalReservation::Clear);
        assert!(stream.region(toward_peak, (144, 144)).is_none());
        stream.publish(facing_away, (144, 144), |_| NaturalReservation::Pending);
        assert!(stream.region(toward_peak, (144, 144)).is_none());
        stream.publish(facing_away, (144, 144), |_| NaturalReservation::Clear);
        assert!(stream.region(toward_peak, (144, 144)).is_some());
        assert!(stream.reserves([-2022.0, -230.0, -2020.0, -228.0]));
    }
}
