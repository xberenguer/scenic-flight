use scenic_flight::{dry_landmarks::FanProfile, world::Surface};
use std::{
    io::{self, BufWriter, Write},
    path::PathBuf,
};

const TILE: usize = 112;
const COLS: usize = 5;
const BACKGROUND: [u8; 3] = [19, 24, 30];

fn main() -> io::Result<()> {
    let output = PathBuf::from(std::env::args_os().nth(1).expect("Output directory"));
    let names = FanProfile::ALL.map(|profile| profile.name);
    std::fs::write(output.join("alluvial-fans.txt"), names.join("\n") + "\n")?;
    let size = TILE * COLS;
    for frame in 0..16 {
        let mut pixels = vec![BACKGROUND; size * size];
        let (sin, cos) = (frame as f32 * std::f32::consts::TAU / 16.0).sin_cos();
        for (index, profile) in FanProfile::ALL.into_iter().enumerate() {
            let (left, top) = (index % COLS * TILE, index / COLS * TILE);
            let mut filled = 0;
            for y in 0..TILE {
                for x in 0..TILE {
                    let dx = (x as f32 + 0.5 - TILE as f32 * 0.5) * 1.90;
                    let dz = (y as f32 + 0.5 - TILE as f32 * 0.5) * 1.90;
                    let world_x = cos * dx + sin * dz;
                    let world_z = 56.0 + cos * dz - sin * dx;
                    let Some(sample) = profile.study(world_x, world_z) else {
                        continue;
                    };
                    let color: [u8; 3] = match sample.surface {
                        Surface::DryWash => [199, 177, 146],
                        Surface::Gravel => [152, 127, 100],
                        _ => [133, 96, 69],
                    };
                    let light = ((sample.elevation - 27.0).max(0.0) * 1.55) as u8;
                    assert!(
                        (1..TILE - 1).contains(&x) && (1..TILE - 1).contains(&y),
                        "clipped {}",
                        profile.name
                    );
                    let opacity = if sample.surface == Surface::Desert {
                        ((sample.elevation - (38.0 - world_z * 0.096)) * 5.0)
                            .abs()
                            .clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    pixels[(top + y) * size + left + x] = std::array::from_fn(|i| {
                        let background = f32::from(BACKGROUND[i]);
                        let tint = f32::from(color[i].saturating_add(light));
                        (background + (tint - background) * opacity) as u8
                    });
                    filled += 1;
                }
            }
            assert!(filled > 100, "empty apron for {}", profile.name);
        }
        let mut file = BufWriter::new(std::fs::File::create(
            output.join(format!("alluvial-fans-{frame:02}.ppm")),
        )?);
        writeln!(file, "P6\n{size} {size}\n255")?;
        for pixel in pixels {
            file.write_all(&pixel)?;
        }
    }
    Ok(())
}
