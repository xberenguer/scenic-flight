//! Sparse shoreline props for the ordinary chase-camera sampler.

use scenic_flight::{
    beach::{BEACH_TILE_SIZE, BeachSite},
    feature_tiles::FeatureTiles,
    lighthouses::{LIGHTHOUSE_TILE, Lighthouse},
    render::{Camera, ScreenRect},
    world::{Sample, World},
    world_mesh::{WorldMesh, bounds_in_range, bounds_in_view, landmark_reach},
};

pub struct Shore {
    beaches: FeatureTiles<BeachSite, 5, 12>,
    lights: FeatureTiles<Lighthouse, 7, 12>,
}

impl Shore {
    pub const fn new() -> Self {
        Self { beaches: FeatureTiles::new(BEACH_TILE_SIZE), lights: FeatureTiles::new(LIGHTHOUSE_TILE) }
    }

    pub fn reset(&mut self, world: &World, seed: u64, x: f32, z: f32) {
        self.beaches.reset(x, z, |x, z| BeachSite::generate(world, seed, x, z));
        self.lights.reset(x, z, |x, z| Lighthouse::generate(world, seed, x, z));
    }

    pub fn follow(&mut self, world: &World, seed: u64, x: f32, z: f32, frame: u32) {
        match frame & 3 {
            0 => self.beaches.follow(x, z, 1, |x, z| BeachSite::generate(world, seed, x, z)),
            2 => self.lights.follow(x, z, 1, |x, z| Lighthouse::generate(world, seed, x, z)),
            _ => {}
        }
    }

    pub fn obstruction(&self, x: f32, z: f32, retained: impl Fn(&Lighthouse) -> bool) -> f32 {
        self.lights.at(x, z).filter(|light| retained(light)).map_or(0.0, |light| light.clearance(x, z))
    }

    pub fn overlay(&self, x: f32, z: f32, ground: Sample, retained: impl Fn(&Lighthouse) -> bool) -> Sample {
        self.lights.at(x, z).filter(|light| retained(light)).map_or(ground, |light| light.overlay(x, z, ground))
    }

    pub fn light_region(&self, camera: Camera, dimensions: (usize, usize), retained: impl Fn(&Lighthouse) -> bool) -> Option<ScreenRect> {
        let reach = light_reach(camera);
        self.lights.sites().filter(|light| bounds_in_range(camera, light.bounds(), reach) && retained(light))
            .filter_map(|light| light.screen_bounds(camera, dimensions)).reduce(ScreenRect::union)
    }

    pub fn has_near_light(&self, camera: Camera, retained: impl Fn(&Lighthouse) -> bool) -> bool {
        self.lights.sites().any(|light| bounds_in_view(camera, light.bounds()) && retained(light))
    }

    pub fn draw_lights(&self, camera: Camera, daylight: f32, ticks: u64, mesh: &mut WorldMesh<'_>, retained: impl Fn(&Lighthouse) -> bool) {
        let reach = light_reach(camera);
        mesh.with_landmark_reach(reach, |mesh| {
            for light in self.lights.sites().filter(|light| bounds_in_range(camera, light.bounds(), reach) && retained(light)) {
                light.draw(mesh, daylight, ticks);
            }
        });
    }

    pub fn draw_beaches(
        &self, camera: Camera, daylight: f32, image: &mut [u16], side: usize,
        terrain: impl Fn(f32, f32) -> Sample,
        occupied: impl Fn(f32, f32) -> bool,
    ) {
        if camera.altitude > 68.0 { return; }
        for beach in self.beaches.sites() {
            let [left, near, right, far] = beach.bounds();
            if (left - 76.0..=right + 76.0).contains(&camera.x)
                && (near - 76.0..=far + 76.0).contains(&camera.z)
                && bounds_in_view(camera, beach.bounds())
                && beach.fits_displayed_sand(&terrain)
                && !beach.conflicts_with(&occupied)
            {
                beach.draw(camera, daylight, image, side, side, |x, z| terrain(x, z).elevation);
            }
        }
    }
}

fn light_reach(camera: Camera) -> f32 {
    landmark_reach(camera.altitude).min(1050.0)
}
