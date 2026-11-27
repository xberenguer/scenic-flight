//! Isolated views of the same geometry and animation called by the game.

use super::{
    draw_eiffel_tower, draw_high_roller, draw_paris_arc, draw_paris_balloon,
    fremont, sphere, stadium, StratTower, EIFFEL_TOWER, EIFFEL_TOWER_HEIGHT,
    HIGH_ROLLER, HIGH_ROLLER_HEIGHT, PARIS_ARC, PARIS_ARC_HEIGHT,
    PARIS_BALLOON, PARIS_BALLOON_HEIGHT, STRAT, STRAT_HEIGHT,
};
use crate::{
    flight::TICKS_PER_SECOND,
    math::sin_cos,
    render::{self, Camera, ScreenProjection},
    sky::Sky,
    world::{Sample, Surface, World},
    world_mesh::WorldMesh,
};

pub const BACKGROUND: [u8; 3] = [20, 26, 32];
pub const BACKGROUND_565: u16 = 0x10c4;
pub const FRAME_RATE: u64 = 10;
pub const FRAME_STEP: u64 = TICKS_PER_SECOND as u64 / FRAME_RATE;
const LUXOR: [f32; 2] = [-200.42, -1_837.95];

#[derive(Clone, Copy)]
enum Model {
    Sphere(u64),
    HighRoller,
    Balloon,
    Eiffel,
    Arc,
    Strat,
    Allegiant,
    AllegiantTray,
    VivaVision,
    Luxor,
}

pub struct Exhibit {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub venue: &'static str,
    pub source: &'static str,
    pub first_tick: u64,
    pub poster_offset: u64,
    pub frames: u64,
    model: Model,
}

const SPHERE_SOURCE: &str = "https://www.thesphere.com/";
const PARIS_SOURCE: &str =
    "https://newsroom.caesars.com/files/doc_downloads/2025/04/Paris-Las-Vegas-Fact-Sheet_April-2025-FINAL.pdf";
const STADIUM_SOURCE: &str = "https://www.hntb.com/projects/allegiant-stadium/";

pub const EXHIBITS: &[Exhibit] = &[
    sphere_show("sphere-face", "Face", 0, 28, "Sphere"),
    sphere_show("sphere-waves", "Waves", 1, 45, ""),
    sphere_show("sphere-moon", "Moon", 2, 55, ""),
    sphere_show("sphere-ribbons", "Ribbons", 3, 48, ""),
    sphere_show("sphere-eclipse", "Eclipse", 4, 55, ""),
    sphere_show("sphere-earth", "Earth", 5, 66, ""),
    sphere_show("sphere-meteors", "Meteor Trails", 6, 50, ""),
    sphere_show("sphere-aurora", "Aurora", 7, 50, ""),
    sphere_show("sphere-rings", "Neon Rings", 8, 55, ""),
    sphere_show("sphere-eye", "Blinking Eye", 9, 40, ""),
    Exhibit {
        id: "high-roller", name: "High Roller", venue: "High Roller",
        group: "animated-landmarks", model: Model::HighRoller,
        source: "https://www.poma.net/en/work/the-high-roller-observation-wheel/",
        first_tick: 0, poster_offset: 40, frames: 90,
    },
    Exhibit {
        id: "paris-balloon", name: "Paris Balloon", venue: "Paris Balloon",
        group: "animated-landmarks", model: Model::Balloon,
        source: "https://federalheath.com/portfolio/paris-balloon",
        first_tick: 0, poster_offset: 45, frames: 80,
    },
    Exhibit {
        id: "viva-vision", name: "Viva Vision", venue: "Fremont Street Experience",
        group: "animated-landmarks", model: Model::VivaVision,
        source: "https://fremontstreetexperience.com/the-canopy/",
        first_tick: 0, poster_offset: 80, frames: 90,
    },
    model("paris-eiffel-tower", "Eiffel Tower", "Eiffel Tower", PARIS_SOURCE, Model::Eiffel),
    model("paris-arc-de-triomphe", "Arc de Triomphe", "Arc de Triomphe", PARIS_SOURCE, Model::Arc),
    model("strat", "The STRAT", "The STRAT", "https://thestrat.com/attractions/tower", Model::Strat),
    model("allegiant-stadium", "Allegiant Stadium", "Allegiant Stadium", STADIUM_SOURCE, Model::Allegiant),
    model("allegiant-field-tray", "Allegiant Stadium field tray", "", STADIUM_SOURCE, Model::AllegiantTray),
    model("luxor-pyramid", "Luxor Pyramid", "Luxor Las Vegas", "https://luxor.mgmresorts.com/", Model::Luxor),
];

const fn sphere_show(id: &'static str, name: &'static str, program: u64, frames: u64, venue: &'static str) -> Exhibit {
    Exhibit {
        id, name, venue, group: "sphere-programs", source: SPHERE_SOURCE,
        first_tick: program * sphere::SHOW_TICKS, poster_offset: 24, frames,
        model: Model::Sphere(program),
    }
}

const fn model(id: &'static str, name: &'static str, venue: &'static str, source: &'static str, model: Model) -> Exhibit {
    Exhibit {
        id, name, venue, group: "landmark-models", source, model,
        first_tick: 0, poster_offset: 0, frames: 0,
    }
}

impl Exhibit {
    pub fn center(&self) -> [f32; 2] {
        match self.model {
            Model::Sphere(_) => sphere::CENTER,
            Model::HighRoller => HIGH_ROLLER,
            Model::Balloon => PARIS_BALLOON,
            Model::Eiffel => EIFFEL_TOWER,
            Model::Arc => PARIS_ARC,
            Model::Strat => STRAT,
            Model::Allegiant | Model::AllegiantTray => stadium::CENTER,
            Model::VivaVision => fremont::CENTER,
            Model::Luxor => LUXOR,
        }
    }

    pub fn is_packed_terrain(&self) -> bool { matches!(self.model, Model::Luxor) }

    pub fn render(
        &self, world: &World, ticks: u64, side: usize,
        pixels: &mut [u16], depth: &mut [u16], work: &mut [u16], background: &mut [u16],
    ) {
        let [x, z] = self.center();
        let base = world.elevation(x, z);
        let [distance, heading, pitch, centre_height, time] = match self.model {
            Model::Sphere(program) => {
                let heading = if program == 2 { -0.47 } else { 0.8916 };
                [188.0, heading, 0.07, 58.0, 0.46]
            }
            Model::HighRoller => [181.0, -1.44, 0.05, HIGH_ROLLER_HEIGHT * 0.50, 0.72],
            Model::Balloon => [63.0, 0.25, 0.05, PARIS_BALLOON_HEIGHT * 0.50, 0.0],
            Model::Eiffel => [178.0, 1.08, 0.07, EIFFEL_TOWER_HEIGHT * 0.50, 0.44],
            Model::Arc => [40.0, 1.11, 0.22, PARIS_ARC_HEIGHT * 0.48, 0.45],
            Model::Strat => [379.0, 0.68, 0.02, STRAT_HEIGHT * 0.51, 0.45],
            Model::Allegiant => [365.0, -2.46, 0.48, 22.0, 0.45],
            Model::AllegiantTray => [398.0, -0.49, 0.54, 16.0, 0.45],
            Model::VivaVision => [330.0, 1.35, 0.24, fremont::HEIGHT * 0.55, 0.0],
            Model::Luxor => [245.0, 0.87, 0.33, 28.0, 0.45],
        };
        let target = if matches!(self.model, Model::AllegiantTray) { [x - 48.0, z - 67.0] } else { [x, z] };
        let (sin, cos) = sin_cos(heading);
        let (up, forward) = sin_cos(pitch);
        let camera = Camera {
            x: target[0] - sin * distance, z: target[1] - cos * distance,
            altitude: base + centre_height + distance * up / forward,
            heading, pitch, bank: 0.0,
        };
        let sky = Sky::from_time(1, 0, time);
        let dimensions = (side, side);
        pixels.fill(BACKGROUND_565);
        depth.fill(u16::MAX);
        if self.is_packed_terrain() {
            render_luxor(world, base, camera, &sky, dimensions, pixels, work, background);
            return;
        }
        let mut mesh = WorldMesh::new(pixels, depth, dimensions, camera, &sky)
            .expect("valid landmark gallery buffers");
        match self.model {
            Model::Sphere(_) => {
                let projection = ScreenProjection::new(camera, dimensions).unwrap();
                sphere::Sphere::at(base, &projection).unwrap().draw(&mut mesh, ticks);
            }
            Model::HighRoller => draw_high_roller(&mut mesh, base, ticks),
            Model::Balloon => draw_paris_balloon(&mut mesh, base, ticks),
            Model::Eiffel => draw_eiffel_tower(&mut mesh, base),
            Model::Arc => draw_paris_arc(&mut mesh, base),
            Model::Strat => StratTower { base }.draw(&mut mesh),
            Model::Allegiant | Model::AllegiantTray => stadium::Stadium::new(world, base).draw(&mut mesh),
            Model::VivaVision => fremont::Canopy::new(world, camera.altitude, base).draw(&mut mesh, ticks),
            Model::Luxor => unreachable!(),
        }
    }
}

fn render_luxor(
    world: &World, base: f32, camera: Camera, sky: &Sky, dimensions: (usize, usize),
    pixels: &mut [u16], work: &mut [u16], reference: &mut [u16],
) {
    let terrain = Sample { elevation: base - 7.0, surface: Surface::Mojave };
    let features = world.city_surface();
    let sample = |x, z| features(x, z, world.sample(x, z))
        .filter(|sample| sample.surface == Surface::PyramidGlass)
        .unwrap_or(terrain);
    render::render_buffered_observed(sample, camera, sky, pixels, work, dimensions, |_| {}).unwrap();
    render::render_buffered_observed(|_, _| terrain, camera, sky, reference, work, dimensions, |_| {}).unwrap();
    for (pixel, &background) in pixels.iter_mut().zip(reference.iter()) {
        if *pixel == background {
            *pixel = BACKGROUND_565;
        }
    }
}
