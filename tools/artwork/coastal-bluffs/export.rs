use scenic_flight::{dry_landmarks::BluffProfile, render::Camera, sky::Sky, world_mesh::WorldMesh};
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
    let profiles = BluffProfile::ALL;
    let names = profiles.map(|p| p.name).join("\n") + "\n";
    std::fs::write(output.join("coastal-bluffs.txt"), names)?;
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
                let shore = -0.15 - 0.27 * (1.0 - u * u);
                let local = v - shore;
                if local < 0.0 {
                    let color = if local < -0.2 {
                        [56, 110, 140]
                    } else {
                        [86, 153, 159]
                    };
                    return Some((0.11, color));
                }
                let inland = local * 54.0;
                let (rise, rock, cap) = profile.study(u, inland, 0.0);
                let hill = (0.15 + (inland * 0.019)).min(1.55);
                let color = if inland < 3.0 {
                    [172, 151, 109]
                } else if rise > 0.19 && inland < rock {
                    let layer = (inland * 0.63) as u8 % 3;
                    [140 + layer * 7, 123 + layer * 7, 107 + layer * 7]
                } else if inland < cap && rise > 0.35 {
                    [107, 131, 79]
                } else {
                    [84, 111, 72]
                };
                Some((hill + 6.5 * rise, color))
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
        let file = std::fs::File::create(output.join(format!("bluff-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in image {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}
