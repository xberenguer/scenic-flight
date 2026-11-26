#[path = "../../../src/skyline/tower_forms.rs"]
mod tower_forms;

use scenic_flight::{render::Camera, sky::Sky, world_mesh::WorldMesh};
use std::io::{self, Write};

const TILE: usize = 68;
const COLS: usize = 10;

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let landmarks = args.first().is_some_and(|arg| arg == "--landmarks");
    if args.first().is_some_and(|arg| arg == "--landmark-names") {
        for design in &tower_forms::landmarks::LANDMARKS {
            println!("{}", design.name);
        }
        return Ok(());
    }
    let count = if landmarks {
        tower_forms::landmarks::LANDMARKS.len()
    } else {
        tower_forms::TOWER_DESIGN_COUNT
    };
    let columns = COLS;
    let rows = count.div_ceil(columns);
    let (width, height) = (TILE * columns, TILE * rows);
    let mut sheet = vec![[19u8, 24, 30]; width * height];
    let heading = args
        .get(1)
        .and_then(|angle| angle.parse::<f32>().ok())
        .unwrap_or(0.706);
    let (sin, cos) = heading.sin_cos();
    let distance = if landmarks { 79.0 } else { 54.0 };
    let camera = Camera {
        x: -distance * sin,
        z: -distance * cos,
        altitude: if landmarks { 45.0 } else { 44.0 },
        heading,
        pitch: if landmarks { 0.29 } else { 0.365 },
        bank: 0.0,
    };
    let sky = Sky::from_time(49, 0, 0.46);
    for index in 0..count {
        let (size, building_height, wall, tiers) = if landmarks {
            let design = &tower_forms::landmarks::LANDMARKS[index];
            (design.size, design.height, design.wall, design.masses)
        } else {
            let design = tower_forms::design(index as u16);
            (
                design.size,
                design.height,
                design.wall,
                design.tiers.as_slice(),
            )
        };
        let mut pixels = [0; TILE * TILE];
        let mut depth = [u16::MAX; TILE * TILE];
        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), camera, &sky).unwrap();
        let half = size.map(|axis| axis * 15.0);
        for &tier in tiers {
            let [x, z] = tier.center;
            let [hx, hz] = tier.half;
            let ring = |y| {
                [
                    [(x - hx) * half[0], y, (z - hz) * half[1]],
                    [(x + hx) * half[0], y, (z - hz) * half[1]],
                    [(x + hx) * half[0], y, (z + hz) * half[1]],
                    [(x - hx) * half[0], y, (z + hz) * half[1]],
                ]
            };
            let ground = ring(tier.bottom * building_height);
            let top = tier.top * building_height;
            let roof = ring(top);
            for edge in 0..4 {
                let next = (edge + 1) % 4;
                mesh.faded_quad(
                    [ground[edge], ground[next], roof[next], roof[edge]],
                    wall,
                    1.0,
                );
            }
            mesh.faded_quad(roof, wall.map(|v| v.saturating_add(26)), 1.0);
            if landmarks && tier.bottom > 0.0 {
                mesh.faded_quad(
                    [ground[0], ground[3], ground[2], ground[1]],
                    wall.map(|v| v.saturating_sub(26)),
                    1.0,
                );
            }
        }
        let (x, y) = (index % columns * TILE, index / columns * TILE);
        for (pixel_index, pixel) in pixels.into_iter().enumerate() {
            if pixel != 0 {
                sheet[(y + pixel_index / TILE) * width + x + pixel_index % TILE] = [
                    ((pixel >> 11) * 255 / 31) as u8,
                    (((pixel >> 5) & 63) * 255 / 63) as u8,
                    ((pixel & 31) * 255 / 31) as u8,
                ];
            }
        }
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    write!(out, "P6\n{width} {height}\n255\n")?;
    for pixel in sheet {
        out.write_all(&pixel)?;
    }
    Ok(())
}
