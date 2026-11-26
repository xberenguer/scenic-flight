//! Export the same height profiles that the game samples at roadside stops.

#[derive(Clone, Copy)]
enum TurnoutKind {
    Overlook,
    Trailhead,
    RestStop,
}

#[path = "../../../src/scenic_turnout/fixtures.rs"]
mod fixtures;

use fixtures::RoadsideFixture;
use scenic_flight::world;
use scenic_flight::{render::Camera, sky::Sky, world_mesh::WorldMesh};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

const TILE: usize = 72;
const COLUMNS: usize = 5;
const ROWS: usize = 6;
const CELL: usize = 48;
const INSET: usize = (TILE - CELL) / 2;
const WIDTH: usize = CELL * COLUMNS;
const HEIGHT: usize = CELL * ROWS;

#[path = "../relief.rs"]
mod relief;

fn main() -> io::Result<()> {
    let output = std::env::args_os()
        .nth(1)
        .expect("Provide an atlas output directory");
    let output = Path::new(&output);
    let sky = Sky::from_time(41, 0, 0.46);
    for frame in 0..24 {
        let heading = 0.47 + frame as f32 * core::f32::consts::TAU / 24.0;
        let (sin, cos) = heading.sin_cos();
        let mut sheet = vec![[19_u8, 24, 30]; WIDTH * HEIGHT];
        for row in 0..ROWS {
            let kind = [
                TurnoutKind::Overlook,
                TurnoutKind::Trailhead,
                TurnoutKind::RestStop,
            ][row / 2];
            let size = if row < 2 { [3.84, 1.64] } else { [3.84, 2.28] };
            for col in 0..COLUMNS {
                let variant = ((row % 2 * COLUMNS + col) * 25) as u8;
                let form = RoadsideFixture::select(kind, variant);
                let distance = 9.2;
                let camera = Camera {
                    x: -distance * sin,
                    z: -distance * cos,
                    altitude: 4.85,
                    heading,
                    pitch: 0.41,
                    bank: 0.0,
                };
                let mut pixels = [0; TILE * TILE];
                let mut depths = [u16::MAX; TILE * TILE];
                relief::draw(
                    &mut WorldMesh::new(&mut pixels, &mut depths, (TILE, TILE), camera, &sky)
                        .unwrap(),
                    size,
                    |coordinates| {
                        form.profile(coordinates).map(|(height, surface)| {
                            use world::Surface::{
                                Boardwalk, CargoBlue, CargoOchre, FlatRoof, PitchedRoof, Rock,
                            };
                            let color = match surface {
                                Boardwalk => [151, 114, 75],
                                CargoBlue => [104, 148, 166],
                                CargoOchre => [186, 142, 65],
                                FlatRoof => [138, 150, 154],
                                PitchedRoof => [156, 102, 85],
                                Rock => [158, 151, 129],
                                _ => [137, 128, 111],
                            };
                            (height, color)
                        })
                    },
                );
                assert!(pixels.iter().any(|&color| color != 0), "{form:?} empty");
                for (i, pixel) in pixels
                    .into_iter()
                    .enumerate()
                    .filter(|(_, value)| *value != 0)
                {
                    let [x, y] = [i % TILE, i / TILE];
                    assert!(
                        (INSET + 2..INSET + CELL - 2).contains(&x)
                            && (INSET + 2..INSET + CELL - 2).contains(&y),
                        "{form:?} clipped at frame {frame}"
                    );
                    sheet[(row * CELL + y - INSET) * WIDTH + col * CELL + x - INSET] = [
                        ((pixel >> 11 & 31) * 255 / 31) as u8,
                        ((pixel >> 5 & 63) * 255 / 63) as u8,
                        ((pixel & 31) * 255 / 31) as u8,
                    ];
                }
            }
        }
        let mut out = BufWriter::new(File::create(
            output.join(format!("roadside-{frame:02}.ppm")),
        )?);
        write!(out, "P6\n{WIDTH} {HEIGHT}\n255\n")?;
        for pixel in sheet {
            out.write_all(&pixel)?;
        }
        out.flush()?;
    }
    Ok(())
}
