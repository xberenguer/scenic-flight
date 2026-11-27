//! Generate RGB frames of the Vegas artwork with the production renderer.

use std::{
    env, fs, io::{self, BufWriter, Write}, path::{Path, PathBuf},
};

use scenic_flight::{
    landmark_gallery::{BACKGROUND, BACKGROUND_565, EXHIBITS, FRAME_STEP},
    world::World,
    LasVegasData,
};

fn write_frame(path: &Path, pixels: &[u16], side: usize) -> io::Result<()> {
    let mut file = BufWriter::new(fs::File::create(path)?);
    write!(file, "P6\n{side} {side}\n255\n")?;
    for &pixel in pixels {
        let rgb = if pixel == BACKGROUND_565 {
            BACKGROUND
        } else {
            [
                ((pixel >> 11) * 255 / 31) as u8,
                (((pixel >> 5) & 63) * 255 / 63) as u8,
                ((pixel & 31) * 255 / 31) as u8,
            ]
        };
        file.write_all(&rgb)?;
    }
    file.flush()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [pack, directory, side] = args.as_slice() else {
        return Err("usage: vegas-landmarks MAP.bin OUTPUT_DIR SIDE".into());
    };
    let side = side.to_string_lossy().parse::<usize>()?;
    if !(96..=320).contains(&side) {
        return Err("frame size must be between 96 and 320 pixels".into());
    }
    let directory = PathBuf::from(directory);
    let data = Box::leak(Box::new(
        LasVegasData::parse(Box::leak(fs::read(pack)?.into_boxed_slice()))
            .ok_or("invalid Las Vegas map pack")?,
    ));
    let world = World::las_vegas(data);
    let mut pixels = vec![0; side * side];
    let mut depth = vec![0; side * side];
    let mut work = vec![0; side * side];
    let mut background = vec![0; side * side];
    fs::create_dir_all(&directory)?;
    let mut catalogue = BufWriter::new(fs::File::create(directory.join("metadata.tsv"))?);
    for exhibit in EXHIBITS {
        let site = directory.join(exhibit.id);
        fs::create_dir_all(&site)?;
        let mut render = |ticks, name: &str| -> io::Result<()> {
            exhibit.render(&world, ticks, side, &mut pixels, &mut depth, &mut work, &mut background);
            if !pixels.iter().any(|&pixel| pixel != BACKGROUND_565) {
                return Err(io::Error::other(format!("{} rendered an empty frame", exhibit.id)));
            }
            write_frame(&site.join(name), &pixels, side)
        };
        render(exhibit.first_tick + exhibit.poster_offset, "poster.ppm")?;
        for frame in 0..exhibit.frames {
            render(exhibit.first_tick + frame * FRAME_STEP, &format!("frame-{frame:03}.ppm"))?;
        }
        let [x, z] = exhibit.center();
        writeln!(
            catalogue, "{}\t{}\t{}\t{}\t{}\t{}\t{:.2}\t{:.2}\t{}",
            exhibit.id, exhibit.name, exhibit.group, exhibit.venue, exhibit.source,
            exhibit.frames, x, z, exhibit.is_packed_terrain(),
        )?;
        eprintln!("Rendered {}", exhibit.name);
    }
    catalogue.flush()?;
    Ok(())
}
