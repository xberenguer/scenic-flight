use scenic_flight::airfield::{AirfieldKind, AirfieldMaterial, RunwayProfile};
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
};

const TILE: usize = 96;
const GRID: usize = 5;
const SIDE: usize = TILE * GRID;

fn main() -> io::Result<()> {
    let arg = std::env::args_os().nth(1).expect("Output directory");
    let output = Path::new(&arg);
    let profiles = RunwayProfile::ALL;
    let names = profiles.map(|p| p.name).join("\n") + "\n";
    std::fs::write(output.join("runway-markings.txt"), names)?;
    for frame in 0..18 {
        let dusk = (1.0 - (frame as f32 * std::f32::consts::TAU / 18.0).cos()) * 0.30;
        let mut pixels = vec![[19, 24, 30]; SIDE * SIDE];
        for (index, profile) in profiles.into_iter().enumerate() {
            let width = if profile.kind == AirfieldKind::Country {
                3.7
            } else {
                5.5
            };
            let [left, top] = [index % GRID * TILE, index / GRID * TILE];
            for row in 6..TILE - 6 {
                let from_end = (TILE - 7 - row) as f32 * 0.58;
                for column in 18..TILE - 18 {
                    let across = (column as f32 - (TILE - 1) as f32 * 0.5) * 0.25;
                    let material = profile.sample(from_end, across, width, 9);
                    let inside = across.abs() <= width;
                    let (ink, shade): ([u8; 3], f32) = match material {
                        AirfieldMaterial::EdgeLight => ([241, 230, 181], 0.0),
                        AirfieldMaterial::RunwayMarking => ([227, 229, 216], dusk * 0.70),
                        AirfieldMaterial::GrassRunway if inside => ([109, 133, 83], dusk),
                        AirfieldMaterial::PavedRunway if inside => ([76, 84, 97], dusk),
                        _ => ([84, 105, 75], dusk),
                    };
                    pixels[(top + row) * SIDE + left + column] =
                        ink.map(|value| (f32::from(value) * (1.0 - shade)) as u8);
                }
            }
        }
        let mut writer =
            BufWriter::new(File::create(output.join(format!("runway-{frame:02}.ppm")))?);
        writeln!(writer, "P6\n{SIDE} {SIDE}\n255")?;
        for pixel in pixels {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}
