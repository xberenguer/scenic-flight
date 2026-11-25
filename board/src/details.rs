//! Ground and coastal layers used by the visual parity firmware and native fixture runner.

use scenic_flight::{
    ground_details::{GroundDetails, TreeSampler},
    scenery::CoastalScenery,
    world::{Sample, Surface, World},
};

pub struct DetailLayers {
    ground: GroundDetails,
    coastal: CoastalScenery,
}

impl DetailLayers {
    pub const fn new() -> Self {
        Self {
            ground: GroundDetails::new(),
            coastal: CoastalScenery::new(),
        }
    }

    pub fn reset(&mut self, world: &World, seed: u64, x: f32, z: f32) {
        self.ground.reset(world, seed, x, z);
        self.coastal.reset(*world, seed, x, z);
    }

    pub fn follow(&mut self, world: &World, x: f32, z: f32) {
        self.ground.follow(world, x, z);
        self.coastal.follow(x, z);
    }

    #[cfg(feature = "parity-fixtures")]
    pub fn sample(&self, x: f32, z: f32, base: Sample) -> Sample {
        self.coastal.overlay(x, z, self.ground.sample(x, z, base))
    }

    pub fn tree_sampler(&self) -> TreeSampler {
        self.ground.tree_sampler()
    }

    pub fn geometry(&self, x: f32, z: f32, base: Sample, trees: &TreeSampler) -> Sample {
        self.coastal.overlay(x, z, trees.geometry(x, z, base))
    }

    pub fn material(&self, x: f32, z: f32, geometry: Sample) -> Surface {
        self.ground.visible_material(x, z, geometry)
    }
}
