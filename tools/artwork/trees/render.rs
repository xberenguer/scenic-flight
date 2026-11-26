// A private export view for the current in-game canopy heightfield.
extern crate scenic_flight;

mod math {
    include!(concat!(env!("SCENIC_GAME_SOURCE"), "/math.rs"));
}

mod world {
    pub use scenic_flight::world::*;
}

#[path = "ground_details_export.rs"]
mod ground_details;

use ground_details::ArtworkCrown;
use std::io::{self, Write};

const SIDE: usize = 64;
const CELLS: usize = 16;
const ATLAS_SIDE: usize = SIDE * CELLS;
const PIXELS_PER_UNIT: f32 = 14.0;
const PITCH_SIN: f32 = 0.62;
const PITCH_COS: f32 = 0.7846;
const YAW_SIN: f32 = 0.259;
const YAW_COS: f32 = 0.966;

fn world_point(u: f32, v: f32, depth: f32) -> (f32, f32, f32) {
    let along = v * PITCH_SIN + depth * PITCH_COS;
    let x = u * YAW_COS + along * YAW_SIN;
    let z = along * YAW_COS - u * YAW_SIN;
    (x, v * -PITCH_COS + depth * PITCH_SIN, z)
}

fn hit(crown: &ArtworkCrown, u: f32, v: f32, depth: f32) -> bool {
    let (x, height, z) = world_point(u, v, depth);
    let (elevation, canopy) = crown.sample(x, z);
    canopy && height <= elevation
}

fn paint(crown: &ArtworkCrown, pixel_x: usize, pixel_y: usize) -> Option<[u8; 4]> {
    let u = (pixel_x as f32 + 0.5 - SIDE as f32 * 0.5) / PIXELS_PER_UNIT;
    let v = (pixel_y as f32 + 0.5 - SIDE as f32 * 0.56) / PIXELS_PER_UNIT;
    let floor = v * PITCH_COS / PITCH_SIN;
    const STEP: f32 = 0.047;
    let mut depth = floor + (crown.height() + 0.015) / PITCH_SIN;
    while depth > floor {
        if hit(crown, u, v, depth) {
            let mut outside = depth + STEP;
            let mut inside = depth;
            for _ in 0..4 {
                let midpoint = (outside + inside) * 0.5;
                if hit(crown, u, v, midpoint) {
                    inside = midpoint;
                } else {
                    outside = midpoint;
                }
            }
            let (x, _, z) = world_point(u, v, inside);
            return Some(shade(crown, x, z));
        }
        depth = (depth - STEP).max(floor);
    }
    hit(crown, u, v, floor).then(|| {
        let (x, _, z) = world_point(u, v, floor);
        shade(crown, x, z)
    })
}

fn shade(crown: &ArtworkCrown, x: f32, z: f32) -> [u8; 4] {
    const STEP: f32 = 0.055;
    let dx = (crown.sample(x + STEP, z).0 - crown.sample(x - STEP, z).0) / (2.0 * STEP);
    let dz = (crown.sample(x, z + STEP).0 - crown.sample(x, z - STEP).0) / (2.0 * STEP);
    let normal_length = (1.0 + dx * dx + dz * dz).sqrt();
    let incidence = ((0.76 + 0.44 * dx - 0.45 * dz) / normal_length).clamp(0.0, 1.0);
    let light = 0.67 + incidence * 0.79;
    let [red, green, blue] = crown.palette().map(|value| (f32::from(value) * light).round() as u8);
    [red, green, blue, 255]
}

fn main() -> io::Result<()> {
    let mut atlas = vec![0; ATLAS_SIDE * ATLAS_SIDE * 4];
    let mut measurements = io::BufWriter::new(std::fs::File::create("crown-metrics.tsv")?);
    for variant in u8::MIN..=u8::MAX {
        let crown = ArtworkCrown::new(variant);
        let (form, width, depth, height) = crown.measurements();
        writeln!(measurements, "{variant}\t{form}\t{width:.2}\t{depth:.2}\t{height:.2}")?;
        let column = usize::from(variant) % CELLS;
        let row = usize::from(variant) / CELLS;
        for y in 0..SIDE {
            for x in 0..SIDE {
                if let Some(pixel) = paint(&crown, x, y) {
                    let at = ((row * SIDE + y) * ATLAS_SIDE + column * SIDE + x) * 4;
                    atlas[at..at + 4].copy_from_slice(&pixel);
                }
            }
        }
    }
    measurements.flush()?;
    io::stdout().lock().write_all(&atlas)
}
