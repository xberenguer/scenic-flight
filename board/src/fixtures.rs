//! Stable views of real generated details for the board-only parity build.

use scenic_flight::{
    render::{Camera, RenderError, render_from_with_sky_buffered},
    scenery::DistrictRegion,
    sky::Sky,
    terrain_cache::TerrainCache,
    world::World,
};

use crate::details::DetailLayers;

pub const SIDE: usize = 112;

#[derive(Clone, Copy)]
pub struct Fixture {
    pub seed: u64,
    pub camera: Camera,
    pub district: Option<(i32, i32)>,
}

pub const FIXTURES: [Fixture; 4] = [
    // Site in a broad, flat grass district with cultivable parcels.
    Fixture {
        seed: 31,
        camera: Camera {
            x: -1184.0,
            z: -651.0,
            altitude: 27.0,
            heading: 0.0,
            pitch: 0.35,
            bank: -0.025,
        },
        district: None,
    },
    // Low forest with tall, world-positioned crowns visible along the slope.
    Fixture {
        seed: 819,
        camera: Camera {
            x: 416.0,
            z: 302.0,
            altitude: 30.0,
            heading: 0.0,
            pitch: 0.30,
            bank: 0.02,
        },
        district: None,
    },
    // A seeded ferry at (719.930, -476.700), with raised decks and a coastal backdrop.
    Fixture {
        seed: 1,
        camera: Camera {
            x: 686.696,
            z: -509.934,
            altitude: 22.0,
            heading: core::f32::consts::FRAC_PI_4,
            pitch: 0.30,
            bank: -0.035,
        },
        district: None,
    },
    // Southern terminus of the seeded regional highway, including a 47-building settlement.
    Fixture {
        seed: 1,
        camera: Camera {
            x: 3451.7654,
            z: -2411.091,
            altitude: 43.40532,
            heading: -0.6927659,
            pitch: 0.26,
            bank: -0.035,
        },
        district: Some((3, -3)),
    },
];

impl Fixture {
    pub fn world(self) -> World {
        World::new(self.seed)
    }

    pub fn prepare_district(self, world: &World) -> Option<DistrictRegion> {
        self.district
            .map(|(x, z)| DistrictRegion::build(world, x, z))
    }

    pub fn render(
        self,
        terrain: &TerrainCache,
        details: Option<&DetailLayers>,
        district: Option<&DistrictRegion>,
        pixels: &mut [u16],
        work: &mut [u16],
    ) -> Result<(), RenderError> {
        let sky = Sky::from_time(self.seed, 0, 0.50);
        render_from_with_sky_buffered(
            |x, z| {
                let base = terrain.sample(x, z);
                if let Some(region) = district {
                    let developed = region.overlay(x, z, base);
                    if developed != base {
                        return developed;
                    }
                }
                details.map_or(base, |scene| scene.sample(x, z, base))
            },
            self.camera,
            &sky,
            pixels,
            work,
            SIDE,
            SIDE,
        )
    }
}
