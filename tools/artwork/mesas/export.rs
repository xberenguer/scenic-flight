use std::{io::{self, Write}, path::Path};
use scenic_flight::{caprock_mesas::MesaProfile, render::Camera, sky::Sky, world_mesh::WorldMesh};
use scenic_flight::dry_landmarks::RidgeProfile;

#[path = "../relief.rs"]
mod relief;

const TILE: usize = 88;
const COLS: usize = 5;

fn main() -> io::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next().expect("Provide an output directory");
    let output = Path::new(&path);
    let ridges = arguments.next().is_some_and(|mode| mode == "--ridges");
    if ridges {
        let profiles: Vec<_> = (0..RidgeProfile::COUNT).map(|i| RidgeProfile::from_index(i).unwrap()).collect();
        let names = profiles.iter().map(|profile| profile.name()).collect::<Vec<_>>();
        export(output, "ridge", "sandstone-ridges", &names, [31.5, 17.3], 7.9, |index, u, v| {
            profiles[index].study(u, v)
        })
    } else {
        let profiles: Vec<_> = (0..MesaProfile::COUNT).map(|i| MesaProfile::from_index(i).unwrap()).collect();
        let names = profiles.iter().map(|profile| profile.name()).collect::<Vec<_>>();
        export(output, "mesa", "mesa-profiles", &names, [24.0, 23.0], 7.1, |index, u, v| {
            profiles[index].study(u, v)
        })
    }
}

fn export(
    output: &Path, stem: &str, label_file: &str, names: &[&str], size: [f32; 2], relief_height: f32,
    study: impl Fn(usize, f32, f32) -> Option<(f32, scenic_flight::world::Surface)>,
) -> io::Result<()> {
    std::fs::write(output.join(format!("{label_file}.txt")), names.join("\n") + "\n")?;
    let sky = Sky::from_time(41, 0, 0.48);
    let width = TILE * COLS;
    let height = TILE * names.len().div_ceil(COLS);
    for frame in 0..16 {
        let mut image = vec![[19, 24, 30]; width * height];
        let heading = 0.54 + frame as f32 * std::f32::consts::TAU / 16.0;
        let (sin, cos) = heading.sin_cos();
        let view = Camera {
            x: -30.0 * sin, z: -30.0 * cos, altitude: 18.0, heading,
            pitch: ((18.0_f32 - 3.8) / 30.0).atan(),
            bank: 0.0,
        };
        for (index, name) in names.iter().enumerate() {
            let mut pixels = [0; TILE*TILE];
            let mut depth = [u16::MAX; TILE*TILE];
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), view, &sky).unwrap();
            relief::draw(&mut mesh, size, |[u, v]| {
                study(index, u, v).map(|(height, surface)| {
                    (height * relief_height, surface.sandstone_color().unwrap())
                })
            });
            let (left, top) = (index % COLS * TILE, index / COLS * TILE);
            assert!(pixels.iter().any(|&pixel| pixel != 0), "empty profile for {name}");
            for (index, pixel) in pixels.into_iter().enumerate().filter(|(_, pixel)| *pixel != 0) {
                let (x, y) = (index % TILE, index / TILE);
                assert!((2..TILE-2).contains(&x) && (2..TILE-2).contains(&y), "clipped profile for {name}");
                image[(top + y) * width + left + x] = [
                    ((pixel >> 11) * 255 / 31) as u8,
                    (((pixel >> 5) & 63) * 255 / 63) as u8,
                    ((pixel & 31) * 255 / 31) as u8,
                ];
            }
        }
        let file = std::fs::File::create(output.join(format!("{stem}-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in image { writer.write_all(&pixel)?; }
    }
    Ok(())
}
