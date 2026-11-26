use scenic_flight::{
    solar_sites::{SolarLayout, SolarSite},
    world::{Sample, Surface},
};
use std::{
    io::{self, BufWriter, Write},
    path::PathBuf,
};

const TILE: usize = 112;
const COLS: usize = 5;
const BACKGROUND: [u8; 3] = [19, 24, 30];

fn main() -> io::Result<()> {
    let output = PathBuf::from(std::env::args_os().nth(1).expect("Output directory"));
    let names = SolarLayout::ALL.map(|layout| layout.name);
    std::fs::write(output.join("solar-layouts.txt"), names.join("\n") + "\n")?;
    let width = TILE * COLS;
    let height = TILE * names.len().div_ceil(COLS);
    let raw = Sample {
        elevation: 10.0,
        surface: Surface::Grass,
    };
    for frame in 0..16 {
        let mut pixels = vec![BACKGROUND; width * height];
        let (sin, cos) = (frame as f32 * std::f32::consts::TAU / 16.0).sin_cos();
        for index in 0..names.len() {
            let site = SolarSite::artwork(index);
            for y in 0..TILE {
                for x in 0..TILE {
                    let dx = (x as f32 + 0.5 - TILE as f32 * 0.5) * 1.82;
                    let dz = (y as f32 + 0.5 - TILE as f32 * 0.5) * 1.82;
                    let world_x = cos * dx + sin * dz;
                    let world_z = 15.0 + cos * dz - sin * dx;
                    let Some(sample) = site.sample(world_x, world_z, raw) else {
                        continue;
                    };
                    let color = match sample.surface {
                        Surface::SolarGlass => {
                            let tint = ((sample.elevation - raw.elevation) * 12.0) as u8;
                            [26 + tint / 3, 64 + tint, 100 + tint]
                        }
                        Surface::TowerRoof => [102, 122, 138],
                        Surface::FlatRoof => [162, 153, 136],
                        Surface::Pavement => [57, 62, 67],
                        _ => [111, 101, 82],
                    };
                    let (left, top) = (index % COLS * TILE, index / COLS * TILE);
                    pixels[(top + y) * width + left + x] = color;
                }
            }
        }
        let mut writer = BufWriter::new(std::fs::File::create(
            output.join(format!("solar-layouts-{frame:02}.ppm")),
        )?);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in pixels {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}
