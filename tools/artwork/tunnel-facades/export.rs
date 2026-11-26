use scenic_flight::{
    render::Camera,
    sky::Sky,
    tunnel_facade::TunnelFacade,
    tunnels::{TunnelMouth, TunnelPoint},
    world_mesh::WorldMesh,
};
use std::{
    io::{self, Write},
    path::Path,
};

const TILE: usize = 96;
const GRID: usize = 5;

fn main() -> io::Result<()> {
    let arg = std::env::args_os().nth(1).expect("Output directory");
    let output = Path::new(&arg);
    let profiles = TunnelFacade::ALL;
    let names = profiles.map(|p| p.name).join("\n") + "\n";
    std::fs::write(output.join("tunnel-facades.txt"), names)?;
    let sky = Sky::from_time(8, 0, 0.41);
    let mouth = TunnelMouth {
        center: TunnelPoint {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        inward_x: 0.0,
        inward_z: 1.0,
    };
    let size = TILE * GRID;
    for frame in 0..20 {
        let angle = 0.62 * (frame as f32 * std::f32::consts::TAU / 20.0).sin();
        let [x, z] = [13.4 * angle.sin(), -13.4 * angle.cos()];
        let camera = Camera {
            x,
            z,
            altitude: 6.8,
            heading: (-x).atan2(-z),
            pitch: ((6.8_f32 - 2.65) / 13.4).atan(),
            bank: 0.0,
        };
        let mut image = vec![[19, 24, 30]; size * size];
        for (index, profile) in profiles.into_iter().enumerate() {
            let mut pixels = [0; TILE * TILE];
            let mut depth = [u16::MAX; TILE * TILE];
            let mut mesh =
                WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), camera, &sky).unwrap();
            // Driveway and a short cutaway keep the actual arch silhouette legible.
            mesh.faded_quad(
                [
                    [-3.4, -0.07, -4.7],
                    [3.4, -0.07, -4.7],
                    [3.4, -0.07, 0.26],
                    [-3.4, -0.07, 0.26],
                ],
                [86, 88, 88],
                1.0,
            );
            for pair in mouth.opening().windows(2) {
                let [a, b] = [pair[0], pair[1]];
                mesh.triangle(
                    [[a.x, a.y, 0.22], [b.x, b.y, 0.22], [0.0, 0.0, 0.22]],
                    [46, 52, 60],
                );
            }
            profile.draw(mouth, &mut mesh);
            let [left, top] = [index % GRID * TILE, index / GRID * TILE];
            assert!(pixels.iter().filter(|&&pixel| pixel != 0).count() > 200);
            for (index, pixel) in pixels.into_iter().enumerate().filter(|(_, p)| *p != 0) {
                let [x, y] = [index % TILE, index / TILE];
                assert!(
                    (2..TILE - 2).contains(&x) && (2..TILE - 2).contains(&y),
                    "{}",
                    profile.name
                );
                image[(top + y) * size + left + x] = [
                    ((pixel >> 11) * 255 / 31) as u8,
                    (((pixel >> 5) & 63) * 255 / 63) as u8,
                    ((pixel & 31) * 255 / 31) as u8,
                ];
            }
        }
        let file = std::fs::File::create(output.join(format!("facade-{frame:02}.ppm")))?;
        let mut file = io::BufWriter::new(file);
        writeln!(file, "P6\n{size} {size}\n255")?;
        for pixel in image {
            file.write_all(&pixel)?;
        }
    }
    Ok(())
}
