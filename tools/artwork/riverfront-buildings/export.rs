use scenic_flight::{
    render::Camera,
    settlement::riverfront_buildings::RiverfrontBuilding,
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
    let profiles = RiverfrontBuilding::ALL;
    let names = profiles.map(|p| p.name).join("\n") + "\n";
    std::fs::write(output.join("riverfront-buildings.txt"), names)?;
    let sky = Sky::from_time(41, 0, 0.48);
    let width = TILE * COLS;
    let height = TILE * profiles.len().div_ceil(COLS);
    for frame in 0..16 {
        let mut image = vec![[19, 24, 30]; width * height];
        let heading = 0.64 + frame as f32 * std::f32::consts::TAU / 16.0;
        let (sin, cos) = heading.sin_cos();
        let view = Camera {
            x: -23.5 * sin,
            z: -23.5 * cos,
            altitude: 13.9,
            heading,
            pitch: ((13.9_f32 - 2.0) / 23.5).atan(),
            bank: 0.0,
        };
        for (index, &profile) in profiles.iter().enumerate() {
            let mut pixels = [0; TILE * TILE];
            let mut depth = [u16::MAX; TILE * TILE];
            let mut mesh =
                WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), view, &sky).unwrap();
            relief::draw_materials(&mut mesh, [12.8, 12.8], |[u, v]| {
                let [x, z] = [u * 6.4, v * 6.4];
                let [half_width, half_depth] = profile.size;
                let on_landing =
                    x.abs() <= 2.05 && (-half_depth - 2.4..=-half_depth + 0.1).contains(&z);
                let ground = if on_landing || z < -half_depth - 2.25 {
                    [148, 124, 91]
                } else if z > 4.2 {
                    [63, 103, 130]
                } else {
                    [97, 122, 85]
                };
                let Some((_, height)) = profile.roof([x / half_width, z / half_depth]) else {
                    return Some((0.13, ground, ground));
                };
                let finish = scenic_flight::settlement::building_palettes::RIVER[index];
                Some((0.12 + height, finish.roof, finish.wall))
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
        let file = std::fs::File::create(output.join(format!("riverfront-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in image {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}
