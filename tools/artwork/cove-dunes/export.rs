use scenic_flight::{dry_landmarks::CoveDuneProfile, world::Surface};
use std::{
    io::{self, BufWriter, Write},
    path::PathBuf,
};

const TILE: usize = 112;
const COLS: usize = 5;
const BACKGROUND: [u8; 3] = [19, 24, 30];

fn main() -> io::Result<()> {
    let output = PathBuf::from(std::env::args_os().nth(1).expect("Output directory"));
    let names = CoveDuneProfile::ALL.map(|profile| profile.name);
    std::fs::write(output.join("cove-dunes.txt"), names.join("\n") + "\n")?;
    let size = TILE * COLS;
    for frame in 0..16 {
        let mut pixels = vec![BACKGROUND; size * size];
        let (sin, cos) = (frame as f32 * std::f32::consts::TAU / 16.0).sin_cos();
        for (index, profile) in CoveDuneProfile::ALL.into_iter().enumerate() {
            let (left, top) = (index % COLS * TILE, index / COLS * TILE);
            let mut filled = 0;
            for y in 0..TILE {
                for x in 0..TILE {
                    let dx = (x as f32 + 0.5 - TILE as f32 * 0.5) * 1.06;
                    let dz = (y as f32 + 0.5 - TILE as f32 * 0.5) * 1.06;
                    let world_x = cos * dx + sin * dz;
                    let world_z = 4.0 + cos * dz - sin * dx;
                    let Some(sample) = profile.study([world_x, world_z]) else {
                        continue;
                    };
                    let opposite = profile
                        .study([world_x - 0.83, world_z - 0.55])
                        .map_or(3.4, |sample| sample.elevation);
                    let light = ((sample.elevation - opposite) * 30.0).clamp(-24.0, 24.0) as i16;
                    let color: [u8; 3] = match sample.surface {
                        Surface::DuneSand => [218, 194, 142],
                        _ => [92, 124, 64],
                    };
                    let opacity = ((sample.elevation - 3.4) * 3.0).clamp(0.0, 1.0);
                    assert!(
                        (1..TILE - 1).contains(&x) && (1..TILE - 1).contains(&y),
                        "clipped {}",
                        profile.name
                    );
                    pixels[(top + y) * size + left + x] = std::array::from_fn(|i| {
                        let background = f32::from(BACKGROUND[i]);
                        let tint = (i16::from(color[i]) + light).clamp(0, 255) as f32;
                        (background + (tint - background) * opacity) as u8
                    });
                    filled += 1;
                }
            }
            assert!(filled > 100, "empty apron for {}", profile.name);
        }
        let mut file = BufWriter::new(std::fs::File::create(
            output.join(format!("cove-dunes-{frame:02}.ppm")),
        )?);
        writeln!(file, "P6\n{size} {size}\n255")?;
        for pixel in pixels {
            file.write_all(&pixel)?;
        }
    }
    Ok(())
}
