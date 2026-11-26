use scenic_flight::{
    alpine_glaciers::GlacierProfile, render::Camera, sky::Sky, world::Surface,
    world_mesh::WorldMesh,
};
use std::{
    io::{self, Write},
    path::Path,
};

#[path = "../relief.rs"]
mod relief;

const TILE: usize = 88;
const COLS: usize = 5;

fn main() -> io::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .expect("Provide an output directory");
    let output = Path::new(&path);
    let profiles: Vec<_> = (0..GlacierProfile::COUNT)
        .map(|i| GlacierProfile::from_index(i).unwrap())
        .collect();
    let names = profiles
        .iter()
        .map(|p| p.name())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(output.join("glaciers.txt"), names)?;
    let sky = Sky::from_time(41, 0, 0.48);
    let width = TILE * COLS;
    let height = TILE * profiles.len().div_ceil(COLS);
    for frame in 0..16 {
        let mut image = vec![[19, 24, 30]; width * height];
        let heading = 0.54 + frame as f32 * std::f32::consts::TAU / 16.0;
        let (sin, cos) = heading.sin_cos();
        let view = Camera {
            x: -28.0 * sin,
            z: -28.0 * cos,
            altitude: 25.0,
            heading,
            pitch: ((25.0_f32 - 0.8) / 28.0).atan(),
            bank: 0.0,
        };
        for (index, &profile) in profiles.iter().enumerate() {
            let mut pixels = [0; TILE * TILE];
            let mut depth = [u16::MAX; TILE * TILE];
            let mut mesh =
                WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), view, &sky).unwrap();
            relief::draw(&mut mesh, [21.6, 32.0], |[u, v]| {
                profile
                    .artwork(v, u * 1.2)
                    .map(|sample| (sample.elevation * 0.85, material(sample.surface)))
            });
            let (left, top) = (index % COLS * TILE, index / COLS * TILE);
            assert!(
                pixels.iter().any(|&pixel| pixel != 0),
                "empty profile for {}",
                profile.name()
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
                    profile.name()
                );
                image[(top + y) * width + left + x] = [
                    ((pixel >> 11) * 255 / 31) as u8,
                    (((pixel >> 5) & 63) * 255 / 63) as u8,
                    ((pixel & 31) * 255 / 31) as u8,
                ];
            }
        }
        let file = std::fs::File::create(output.join(format!("glaciers-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in image {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}

fn material(surface: Surface) -> [u8; 3] {
    match surface {
        Surface::Snow => [212, 224, 226],
        Surface::GlacierIce => [120, 187, 199],
        Surface::GlacierCrevasse => [53, 112, 143],
        Surface::GlacierMoraine => [139, 133, 120],
        _ => unreachable!("glacier surface"),
    }
}
