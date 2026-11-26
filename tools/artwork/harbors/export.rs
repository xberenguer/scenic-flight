//! Export the same height profiles that the game samples on surveyed quays.

#[derive(Clone, Copy)]
enum PortKind {
    Marina,
    Ferry,
    Freight,
}

#[path = "../../../src/ports/forms.rs"]
mod forms;

use forms::HarborForm;
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
        for (row, (kind, equipment, size, color)) in [
            (PortKind::Marina, false, [5.5_f32, 3.5_f32], [174, 142, 111]),
            (PortKind::Ferry, false, [11.0, 5.5], [146, 159, 162]),
            (PortKind::Freight, false, [23.0, 6.5], [167, 129, 99]),
            (PortKind::Marina, true, [5.8, 3.0], [178, 137, 81]),
            (PortKind::Ferry, true, [7.1, 4.3], [103, 147, 153]),
            (PortKind::Freight, true, [7.6, 4.5], [145, 105, 87]),
        ]
        .into_iter()
        .enumerate()
        {
            for col in 0..COLUMNS {
                let form = if equipment {
                    HarborForm::equipment(kind, (col as u64) << 23)
                } else {
                    HarborForm::building(kind, (col as u64) << 42)
                };
                let distance = size[0].max(size[1]) * 1.37 + form.height() * 0.78 + 2.2;
                let camera = Camera {
                    x: -distance * sin,
                    z: -distance * cos,
                    altitude: distance * 0.44 + form.height() * 0.36,
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
                        let fraction = form.roof(coordinates);
                        (fraction > 0.0).then_some((fraction * form.height(), color))
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
        let mut out = BufWriter::new(File::create(output.join(format!("harbor-{frame:02}.ppm")))?);
        write!(out, "P6\n{WIDTH} {HEIGHT}\n255\n")?;
        for pixel in sheet {
            out.write_all(&pixel)?;
        }
        out.flush()?;
    }
    Ok(())
}
