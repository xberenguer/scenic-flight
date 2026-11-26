use std::{io::{self, Write}, path::Path};
use scenic_flight::{
    lookout_art::{LookoutArt, LookoutKind, SummitDesign},
    render::Camera,
    sky::Sky,
    world_mesh::WorldMesh,
};

const TILE: usize = 84;
const COLS: usize = 5;

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let output = Path::new(&args[0]);
    let sky = Sky::from_time(41, 0, 0.45);
    let width = TILE * COLS;
    let height = TILE * SummitDesign::COUNT.div_ceil(COLS);
    let mut names = String::new();
    let designs: Vec<_> = (0..SummitDesign::COUNT).map(|i| SummitDesign::from_index(i).unwrap()).collect();
    for design in &designs {
        names.push_str(design.name());
        names.push('\n');
    }
    std::fs::write(output.join("summit-landmarks.txt"), names)?;

    for frame in 0..16 {
        let mut image = vec![[19, 24, 30]; width * height];
        let heading = 0.58 + frame as f32 * std::f32::consts::TAU / 16.0;
        let (sin, cos) = heading.sin_cos();
        for (index, &design) in designs.iter().enumerate() {
            let art = LookoutArt::new([0.0; 3], 0.0, LookoutKind::Landmark(design), 0);
            let [_, [half_x, highest, half_z]] = art.mesh_bounds();
            let target = (highest - 2.0) * 0.48;
            let radius = (highest * 1.58).max(half_x.max(half_z) * 5.4);
            let altitude = target + radius * 0.32;
            let view = Camera {
                x: -radius * sin, z: -radius * cos, altitude, heading,
                pitch: ((altitude - target) / radius).atan(), bank: 0.0,
            };
            let mut pixels = [0; TILE * TILE];
            let mut depth = [u16::MAX; TILE * TILE];
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), view, &sky).unwrap();
            art.draw(&mut mesh, sky.daylight);
            let left = index % COLS * TILE;
            let top = index / COLS * TILE;
            assert!(pixels.iter().any(|&pixel| pixel != 0), "empty sprite for {}", design.name());
            for (pixel_index, pixel) in pixels.into_iter().enumerate().filter(|(_, pixel)| *pixel != 0) {
                let x = pixel_index % TILE;
                let y = pixel_index / TILE;
                assert!((1..TILE-1).contains(&x) && (1..TILE-1).contains(&y), "clipped sprite for {}", design.name());
                image[(top + y) * width + left + x] = [
                    ((pixel >> 11) * 255 / 31) as u8,
                    (((pixel >> 5) & 63) * 255 / 63) as u8,
                    ((pixel & 31) * 255 / 31) as u8,
                ];
            }
        }
        let file = std::fs::File::create(output.join(format!("summits-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for rgb in image {
            writer.write_all(&rgb)?;
        }
    }
    Ok(())
}
