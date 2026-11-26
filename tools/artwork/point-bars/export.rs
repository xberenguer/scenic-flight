use scenic_flight::{
    point_bars::PointBarProfile, render::Camera, sky::Sky, world::Surface, world_mesh::WorldMesh,
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
    let profiles = PointBarProfile::ALL;
    let names = profiles.map(|p| p.name).join("\n") + "\n";
    std::fs::write(output.join("point-bars.txt"), names)?;
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
                let along = (u + 1.0) * 0.5;
                let rim = -0.29 + 0.33 * (1.0 - u * u);
                if v < rim {
                    return Some((
                        0.09,
                        if v < rim - 0.18 {
                            [52, 111, 137]
                        } else {
                            [100, 162, 169]
                        },
                    ));
                }
                let across = v - rim - 0.05;
                let width = (0.18 + 0.61 * (1.0 - u * u)) * profile.width(along);
                if !(0.0..width).contains(&across) {
                    return Some((0.80 + 0.9 * v.max(0.0), [130, 107, 76]));
                }
                let (relief, material) = profile.sediment(along, across / width);
                let color = match material {
                    Surface::Gravel => [146, 144, 132],
                    Surface::Sand => [200, 170, 113],
                    _ => [161, 120, 78],
                };
                Some((0.57 + across * 0.79 + 1.25 * relief, color))
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
        let file = std::fs::File::create(output.join(format!("point-bar-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in image {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}
