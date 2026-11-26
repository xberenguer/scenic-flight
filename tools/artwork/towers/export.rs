#[path = "../../../src/skyline/tower_forms.rs"]
mod tower_forms;

use scenic_flight::{render::Camera, sky::Sky, world_mesh::WorldMesh};
use std::io::{self, Write};

const TILE: usize = 68;
const COLS: usize = 10;

fn main() -> io::Result<()> {
    let rows = tower_forms::TOWER_DESIGN_COUNT / COLS;
    let (width, height) = (TILE * COLS, TILE * rows);
    let mut sheet = vec![[20u8, 25, 31]; width * height];
    let camera = Camera {
        x: -35.0,
        z: -41.0,
        altitude: 44.0,
        heading: 0.706,
        pitch: 0.365,
        bank: 0.0,
    };
    let sky = Sky::from_time(49, 0, 0.46);
    for index in 0..tower_forms::TOWER_DESIGN_COUNT {
        let design = tower_forms::design(index as u8);
        let mut pixels = [0; TILE * TILE];
        let mut depth = [u16::MAX; TILE * TILE];
        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), camera, &sky).unwrap();
        let half = design.size.map(|axis| axis * 15.0);
        let mut bottom = 0.0;
        for tier in design.tiers {
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
            let ground = ring(bottom);
            let top = tier.top * design.height;
            let roof = ring(top);
            for edge in 0..4 {
                let next = (edge + 1) % 4;
                mesh.faded_quad(
                    [ground[edge], ground[next], roof[next], roof[edge]],
                    design.wall,
                    1.0,
                );
            }
            mesh.faded_quad(roof, design.wall.map(|v| v.saturating_add(26)), 1.0);
            bottom = top;
        }
        let (x, y) = (index % COLS * TILE, index / COLS * TILE);
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
