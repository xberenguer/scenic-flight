use scenic_flight::{
    render::Camera,
    settlement::{SettlementSurface, neighborhood_blocks::NeighborhoodBlock},
    sky::Sky,
    world_mesh::WorldMesh,
};
use std::{
    io::{self, Write},
    path::Path,
};

#[path = "../relief.rs"]
mod relief;

const TILE: usize = 104;
const COLS: usize = 5;

fn main() -> io::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .expect("Provide an output directory");
    let output = Path::new(&path);
    let profiles = NeighborhoodBlock::ALL;
    let names = profiles.map(|p| p.name).join("\n") + "\n";
    std::fs::write(output.join("outer-blocks.txt"), names)?;
    let sky = Sky::from_time(41, 0, 0.48);
    let width = TILE * COLS;
    let height = TILE * profiles.len().div_ceil(COLS);
    for frame in 0..16 {
        let mut image = vec![[19, 24, 30]; width * height];
        let heading = 0.64 + frame as f32 * std::f32::consts::TAU / 16.0;
        let (sin, cos) = heading.sin_cos();
        let view = Camera {
            x: -30.0 * sin,
            z: -30.0 * cos,
            altitude: 18.0,
            heading,
            pitch: ((18.0_f32 - 3.8) / 30.0).atan(),
            bank: 0.0,
        };
        for (index, &profile) in profiles.iter().enumerate() {
            let mut pixels = [0; TILE * TILE];
            let mut depth = [u16::MAX; TILE * TILE];
            let mut mesh =
                WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), view, &sky).unwrap();
            relief::draw(&mut mesh, [19.0, 18.0], |[u, v]| {
                let [x, z] = [u * 9.0, v * 9.0];
                let ground = if x.abs() > 7.12 || z.abs() > 7.12 {
                    [61, 65, 68]
                } else {
                    [85, 106, 75]
                };
                let Some(sample) = profile.study(x, z) else {
                    return Some((0.12, ground));
                };
                let color = match sample.surface {
                    SettlementSurface::PitchedRoof => [166, 108, 79],
                    SettlementSurface::FlatRoof => [146, 143, 131],
                    SettlementSurface::TowerRoof => [107, 150, 166],
                    SettlementSurface::IndustryBrick => [142, 108, 93],
                    SettlementSurface::IndustryConcrete => [159, 153, 136],
                    SettlementSurface::IndustryMetal => [123, 135, 141],
                    SettlementSurface::Street => [107, 111, 109],
                    _ => [88, 118, 78],
                };
                Some((0.12 + sample.elevation.unwrap_or(0.0) * 0.65, color))
            });
            let (left, top) = (index % COLS * TILE, index / COLS * TILE);
            assert!(
                pixels.iter().any(|&pixel| pixel != 0),
                "empty profile for {}",
                profile.name
            );
            for (index, pixel) in pixels
                .into_iter()
                .enumerate()
                .filter(|(_, pixel)| *pixel != 0)
            {
                let (x, y) = (index % TILE, index / TILE);
                assert!(
                    (2..TILE - 2).contains(&x) && (2..TILE - 2).contains(&y),
                    "clipped profile for {}",
                    profile.name
                );
                image[(top + y) * width + left + x] = [
                    ((pixel >> 11) * 255 / 31) as u8,
                    (((pixel >> 5) & 63) * 255 / 63) as u8,
                    ((pixel & 31) * 255 / 31) as u8,
                ];
            }
        }
        let file = std::fs::File::create(output.join(format!("outer-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in image {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}
