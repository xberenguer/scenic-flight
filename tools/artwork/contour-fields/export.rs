use scenic_flight::{ground_details::ContourProfile, world::Surface};
use std::{
    io::{self, BufWriter, Write},
    path::PathBuf,
};

const TILE: usize = 112;
const COLS: usize = 5;
const BACKGROUND: [u8; 3] = [19, 24, 30];

fn main() -> io::Result<()> {
    let output = PathBuf::from(std::env::args_os().nth(1).expect("Output directory"));
    let names = ContourProfile::ALL.map(|profile| profile.name);
    std::fs::write(output.join("contour-fields.txt"), names.join("\n") + "\n")?;
    let size = TILE * COLS;
    for frame in 0..16 {
        let mut pixels = vec![BACKGROUND; size * size];
        let (sin, cos) = (frame as f32 * std::f32::consts::TAU / 16.0).sin_cos();
        for (index, profile) in ContourProfile::ALL.into_iter().enumerate() {
            let (left, top) = (index % COLS * TILE, index / COLS * TILE);
            let mut filled = 0;
            for y in 0..TILE {
                for x in 0..TILE {
                    let dx = (x as f32 + 0.5 - TILE as f32 * 0.5) * 0.96;
                    let dz = (y as f32 + 0.5 - TILE as f32 * 0.5) * 0.96;
                    let world_x = 32.0 + cos * dx + sin * dz;
                    let world_z = 32.0 + cos * dz - sin * dx;
                    let along = world_x - 32.0;
                    let bend = along * along * (4.5 / (28.0 * 28.0));
                    let across = world_z + along * 0.15 + bend;
                    if !(1.0..63.0).contains(&world_x) || !(1.0..63.0).contains(&world_z) {
                        continue;
                    }
                    let color = match profile.sample(world_x, across) {
                        Surface::FieldGreen => [111, 137, 69],
                        Surface::FieldGold => [183, 164, 86],
                        Surface::FieldSoil => [152, 109, 77],
                        _ => [63, 91, 55],
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
            output.join(format!("contour-fields-{frame:02}.ppm")),
        )?);
        writeln!(file, "P6\n{size} {size}\n255")?;
        for pixel in pixels {
            file.write_all(&pixel)?;
        }
    }
    Ok(())
}
