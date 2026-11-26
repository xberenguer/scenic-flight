use scenic_flight::{
    rail_earthwork::EarthworkProfile, render::Camera, sky::Sky, world_mesh::WorldMesh,
};
use std::{
    io::{self, Write},
    path::Path,
};

#[path = "../relief.rs"]
mod relief;

const TILE: usize = 96;
const COLS: usize = 5;

fn main() -> io::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .expect("Provide an output directory");
    let output = Path::new(&path);
    let profiles = EarthworkProfile::ALL;
    let names = profiles.map(|p| p.name).join("\n") + "\n";
    std::fs::write(output.join("rail-earthworks.txt"), names)?;
    let sky = Sky::from_time(41, 0, 0.48);
    let width = TILE * COLS;
    let height = TILE * profiles.len().div_ceil(COLS);
    for frame in 0..16 {
        let mut image = vec![[19, 24, 30]; width * height];
        let heading = 0.65 + 0.40 * (frame as f32 * std::f32::consts::TAU / 16.0).sin();
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
                let across = (u.abs() * 9.5 - 1.5).max(0.0) / 5.0;
                let raw = 4.0 * u + 0.24 * v;
                let toe = ((across * 5.0 + 1.5).powi(2) - 2.25) / 40.0;
                let weight = profile.weight(toe, u > 0.0);
                let cut = raw * weight;
                let color = if u.abs() < 0.12 {
                    if (v * 9.0).rem_euclid(1.0) < 0.35 {
                        [92, 83, 78]
                    } else {
                        [141, 131, 116]
                    }
                } else if cut > 0.45 {
                    [129, 122, 112]
                } else if cut < -0.40 {
                    [157, 135, 102]
                } else {
                    [100, 118, 82]
                };
                Some((5.0 + raw - cut, color))
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
            draw_section(&mut image, width, left, top, profile);
        }
        let file = std::fs::File::create(output.join(format!("rail-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in image {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}

fn draw_section(
    image: &mut [[u8; 3]],
    canvas_width: usize,
    left: usize,
    top: usize,
    profile: EarthworkProfile,
) {
    let mut last = None;
    for column in 10..TILE - 10 {
        let x = (column as f32 / (TILE - 1) as f32 - 0.5) * 19.0;
        let raw = x * 0.42;
        let toe = (x * x - 2.25).max(0.0) / 40.0;
        let adjustment = raw * profile.weight(toe, x > 0.0);
        let y = (80.0 - (raw - adjustment) * 3.6).round() as usize;
        let base = top * canvas_width + left + column;
        image[base + 80 * canvas_width] = [39, 52, 62];
        let color = if x.abs() < 1.5 {
            [200, 194, 179]
        } else if x > 0.0 {
            [191, 157, 119]
        } else {
            [162, 175, 155]
        };
        let previous = last.unwrap_or(y);
        for row in y.min(previous)..=y.max(previous) {
            image[base + row * canvas_width] = color;
        }
        last = Some(y);
    }
}
