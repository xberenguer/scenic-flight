use scenic_flight::{
    render::Camera,
    sky::Sky,
    water_tower::{WaterTower, WaterTowerDesign},
    world_mesh::WorldMesh,
};
use std::{
    io::{self, Write},
    path::Path,
};

const TILE: usize = 96;
const COLS: usize = 5;

fn main() -> io::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .expect("Provide an output directory");
    let output = Path::new(&path);
    let profiles: Vec<_> = (0..WaterTowerDesign::COUNT)
        .map(|i| WaterTowerDesign::from_index(i).unwrap())
        .collect();
    let names = profiles
        .iter()
        .map(|p| p.name())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(output.join("water-towers.txt"), names)?;
    let sky = Sky::from_time(41, 0, 0.48);
    let width = TILE * COLS;
    let height = TILE * profiles.len().div_ceil(COLS);
    for frame in 0..16 {
        let mut image = vec![[19, 24, 30]; width * height];
        let heading = 0.17 + frame as f32 * std::f32::consts::TAU / 16.0;
        let (sin, cos) = heading.sin_cos();
        for (index, &profile) in profiles.iter().enumerate() {
            let form = WaterTower::artwork(profile);
            let (range, altitude, focus): (f32, f32, f32) = (51.0, 22.0, 18.0);
            let view = Camera {
                x: -range * sin,
                z: -range * cos,
                altitude,
                heading,
                pitch: ((altitude - focus) / range).atan(),
                bank: 0.0,
            };
            let mut pixels = [0; TILE * TILE];
            let mut depth = [u16::MAX; TILE * TILE];
            let mut mesh =
                WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), view, &sky).unwrap();
            form.draw(&mut mesh);
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
        let file = std::fs::File::create(output.join(format!("water-towers-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in image {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}
