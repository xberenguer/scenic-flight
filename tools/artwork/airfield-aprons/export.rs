use scenic_flight::airfield::{AirfieldMaterial, ApronProfile};
use std::{
    io::{self, BufWriter, Write},
    path::PathBuf,
};

const TILE: usize = 112;
const COLS: usize = 5;
const BACKGROUND: [u8; 3] = [19, 24, 30];

fn main() -> io::Result<()> {
    let output = PathBuf::from(std::env::args_os().nth(1).expect("Output directory"));
    let names = ApronProfile::ALL.map(|profile| profile.name);
    std::fs::write(output.join("airfield-aprons.txt"), names.join("\n") + "\n")?;
    let size = TILE * COLS;
    for frame in 0..16 {
        let mut pixels = vec![BACKGROUND; size * size];
        let (sin, cos) = (frame as f32 * std::f32::consts::TAU / 16.0).sin_cos();
        for (index, profile) in ApronProfile::ALL.into_iter().enumerate() {
            let (left, top) = (index % COLS * TILE, index / COLS * TILE);
            let mut filled = 0;
            for y in 0..TILE {
                for x in 0..TILE {
                    let dx = (x as f32 + 0.5 - TILE as f32 * 0.5) * 0.030;
                    let dz = (y as f32 + 0.5 - TILE as f32 * 0.5) * 0.030;
                    let u = cos * dx + sin * dz;
                    let v = (cos * dz - sin * dx) * 1.47;
                    if u.abs() >= 1.0 || v.abs() >= 1.0 {
                        continue;
                    }
                    let color = match profile.sample(u, v) {
                        AirfieldMaterial::ApronStand => [83, 92, 106],
                        AirfieldMaterial::ApronMarking => [216, 180, 72],
                        _ => [141, 131, 108],
                    };
                    assert!(
                        (1..TILE - 1).contains(&x) && (1..TILE - 1).contains(&y),
                        "clipped {}",
                        profile.name
                    );
                    pixels[(top + y) * size + left + x] = color;
                    filled += 1;
                }
            }
            assert!(filled > 100, "empty apron for {}", profile.name);
        }
        let mut file = BufWriter::new(std::fs::File::create(
            output.join(format!("airfield-aprons-{frame:02}.ppm")),
        )?);
        writeln!(file, "P6\n{size} {size}\n255")?;
        for pixel in pixels {
            file.write_all(&pixel)?;
        }
    }
    Ok(())
}
