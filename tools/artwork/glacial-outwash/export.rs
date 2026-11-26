use scenic_flight::{
    outwash_fans::OutwashProfile, render::Camera, sky::Sky, world_mesh::WorldMesh,
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
    let profiles = OutwashProfile::ALL;
    let names = profiles.map(|p| p.name).join("\n") + "\n";
    std::fs::write(output.join("glacial-outwash.txt"), names)?;
    let sky = Sky::from_time(41, 0, 0.48);
    let width = TILE * COLS;
    let height = TILE * profiles.len().div_ceil(COLS);
    for frame in 0..16 {
        let mut image = vec![[19, 24, 30]; width * height];
        let heading = 0.18 + 0.34 * (frame as f32 * std::f32::consts::TAU / 16.0).sin();
        let (sin, cos) = heading.sin_cos();
        let view = Camera {
            x: -22.0 * sin,
            z: -22.0 * cos,
            altitude: 24.0,
            heading,
            pitch: (24.0_f32 / 22.0).atan(),
            bank: 0.0,
        };
        for (index, &profile) in profiles.iter().enumerate() {
            let mut pixels = [0; TILE * TILE];
            let mut depth = [u16::MAX; TILE * TILE];
            let mut mesh =
                WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), view, &sky).unwrap();
            relief::draw(&mut mesh, [24.0, 22.0], |[u, v]| {
                let t = (1.0 - v) * 0.5;
                let (gravel, deposit) = profile.study(t, u * 39.0);
                let shade = (8.0 + 6.0 * (v * 19.0 + u * 17.0).sin()) as u8;
                let hill = 6.2 * (1.0 - t) + 0.2 * (v * 11.0 + u * 6.0).sin();
                let color = if gravel > 0.49 {
                    [192 + shade, 186 + shade, 170 + shade]
                } else if deposit > 0.20 {
                    [113 + shade, 126 + shade, 90 + shade]
                } else {
                    [82 + shade, 98 + shade, 66 + shade]
                };
                Some((hill.max(0.1) + deposit, color))
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
        let file = std::fs::File::create(output.join(format!("outwash-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in image {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}
