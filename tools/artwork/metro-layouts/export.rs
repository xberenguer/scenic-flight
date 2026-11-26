use scenic_flight::settlement::district_layouts::{COLUMNS, NeighborhoodLayout, ROWS};
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::PathBuf,
};

const TILE: usize = 106;
const TILE_COLUMNS: usize = 5;
const STEP: usize = 5;
const X: usize = (TILE - (COLUMNS - 1) * STEP) / 2;
const Z: usize = (TILE - (ROWS - 1) * STEP) / 2;

fn main() -> io::Result<()> {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("Provide an output directory"),
    );
    let layouts = NeighborhoodLayout::ALL;
    let names = layouts.map(|shape| shape.name).join("\n") + "\n";
    std::fs::write(output.join("metro-layouts.txt"), names)?;
    let width = TILE * TILE_COLUMNS;
    let height = TILE * layouts.len().div_ceil(TILE_COLUMNS);
    let mut image = vec![[19, 24, 30]; width * height];
    for (i, layout) in layouts.iter().enumerate() {
        let left = i % TILE_COLUMNS * TILE + X;
        let top = i / TILE_COLUMNS * TILE + Z;
        let mut paint = |x, z, rgb| image[(top + z) * width + left + x] = rgb;
        for z in 0..ROWS - 1 {
            for x in 0..COLUMNS - 1 {
                if !layout.allows_block(x, z) {
                    continue;
                }
                let core = (6..12).contains(&x) && z < 6;
                let color = if core { [151, 119, 93] } else { [80, 105, 92] };
                let road = [155, 165, 162];
                for row in 0..=STEP {
                    for col in 0..=STEP {
                        let edge = row == 0 || row == STEP || col == 0 || col == STEP;
                        paint(
                            x * STEP + col,
                            z * STEP + row,
                            if edge { road } else { color },
                        );
                    }
                }
            }
        }
    }
    let mut file = BufWriter::new(File::create(output.join("metro-layouts.ppm"))?);
    writeln!(file, "P6\n{width} {height}\n255")?;
    for pixel in image {
        file.write_all(&pixel)?;
    }
    Ok(())
}
