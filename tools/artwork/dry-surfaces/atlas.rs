pub(super) fn export_dry_surfaces() -> std::io::Result<()> {
    use std::io::{self, Write};
    use dryland::Material;

    const SIDE: usize = 112;
    const GUTTER: usize = 3;
    const WIDTH: usize = 5 * (SIDE + GUTTER) + GUTTER;
    const BACKGROUND: [u8; 3] = [20, 26, 32];
    let families = [
        (Material::Scrub, [171.0, 137.0, 95.0]),
        (Material::Wash, [174.0, 163.0, 146.0]),
        (Material::Dune, [186.0, 147.0, 98.0]),
        (Material::Sandstone, [182.0, 123.0, 90.0]),
        (Material::Salt, [214.0, 198.0, 167.0]),
    ];
    let mut pixels = vec![BACKGROUND; WIDTH * WIDTH];
    for (row, (material, base)) in families.into_iter().enumerate() {
        const SEED: u32 = 42;
        let cell = material.cell_size();
        let mut tiles = [None; 5];
        for tz in 0..30 {
            for tx in 0..30 {
                let key = hash(tx, tz, SEED);
                let district = hash(tx >> 2, tz >> 2, SEED ^ 0x310a_689f);
                let index = Material::profile(key, district);
                tiles[index as usize].get_or_insert([tx, tz]);
            }
        }
        for (column, tile) in tiles.into_iter().enumerate() {
            let [tx, tz] = tile.expect("each material profile is reachable");
            for py in 0..SIDE {
                for px in 0..SIDE {
                    let u = (tx as f32 + (px as f32 + 0.5) / SIDE as f32) * cell;
                    let v = (tz as f32 + (py as f32 + 0.5) / SIDE as f32) * cell;
                    let inverse = 1.0 / (0.94_f32 * 0.94 + 0.34 * 0.34);
                    let x = (0.94 * u - 0.34 * v) * inverse;
                    let z = (0.34 * u + 0.94 * v) * inverse;
                    let color = material.shade([x, 28.0, z], SEED, base, 1.0, [0.0; 2]);
                    let out_x = GUTTER + column * (SIDE + GUTTER) + px;
                    let out_y = GUTTER + row * (SIDE + GUTTER) + py;
                    pixels[out_y * WIDTH + out_x] = color.map(|value| value.clamp(0.0, 255.0) as u8);
                }
            }
        }
    }
    let mut output = io::BufWriter::new(io::stdout().lock());
    writeln!(output, "P6\n{WIDTH} {WIDTH}\n255")?;
    for pixels in pixels.chunks(WIDTH) {
        output.write_all(pixels.as_flattened())?;
    }
    output.flush()
}
