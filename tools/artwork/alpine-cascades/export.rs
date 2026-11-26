use scenic_flight::{render::Camera, sky::Sky, waterfall::CascadeProfile, world_mesh::WorldMesh};
use std::io::{self, Write};
use std::path::Path;

const TILE: usize = 100;
const COLS: usize = 5;

fn rock_height(z: f32) -> f32 {
    let step = (z + 9.0) * 0.65;
    1.0 + 0.44 * step + 0.57 * (step.floor() + step.fract().powi(3))
}

fn main() -> io::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .expect("Provide an output directory");
    let output = Path::new(&path);
    let profiles = CascadeProfile::ALL;
    std::fs::write(
        output.join("alpine-cascades.txt"),
        profiles.map(|profile| profile.name).join("\n") + "\n",
    )?;
    let sky = Sky::from_time(41, 0, 0.42);
    let width = TILE * COLS;
    let height = TILE * profiles.len().div_ceil(COLS);
    let camera = Camera {
        x: 5.6,
        z: -22.0,
        altitude: 14.5,
        heading: (-5.6_f32).atan2(22.0),
        pitch: 0.40,
        bank: 0.0,
    };
    let sections: [[[f32; 3]; 2]; 15] = core::array::from_fn(|index| {
        let progress = index as f32 / 14.0;
        let z = 8.4 - 16.8 * progress;
        let crest = rock_height(z) + 0.18;
        let wave = (progress * 5.6).sin() * 0.45;
        [[-2.66 + wave, crest, z], [2.66 + wave, crest, z]]
    });
    let mut fingerprints = [0_u64; CascadeProfile::ALL.len()];
    let mut first_frame = [0; CascadeProfile::ALL.len()];
    let mut animated = [false; CascadeProfile::ALL.len()];
    for frame in 0..32 {
        let mut image = vec![[19, 24, 30]; width * height];
        for (index, profile) in profiles.into_iter().enumerate() {
            let mut pixels = [0; TILE * TILE];
            let mut depth = [u16::MAX; TILE * TILE];
            let mut mesh =
                WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), camera, &sky).unwrap();
            let x_coords = [-5.0_f32, -4.2, -3.2, -2.0, 0.0, 2.0, 3.2, 4.2, 5.0];
            let ridge = |x: f32, y: f32, z: f32| [x, y + (x.abs() - 3.1).max(0.0) * 0.65, z];
            for reach in sections.windows(2) {
                let [_, high, far] = reach[0][0];
                let [_, low, near] = reach[1][0];
                for pair in x_coords.windows(2) {
                    let [left, right] = [pair[0], pair[1]];
                    let hue = 77 + ((left * 2.5 + far * 2.1).abs() as u8 % 6) * 6;
                    mesh.faded_quad(
                        [
                            ridge(left, high - 0.27, far),
                            ridge(right, high - 0.27, far),
                            ridge(right, low - 0.27, near),
                            ridge(left, low - 0.27, near),
                        ],
                        [hue + 13, hue + 10, hue],
                        1.0,
                    );
                }
            }
            drop(mesh);
            let terrain = pixels;
            let mut mesh =
                WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), camera, &sky).unwrap();
            profile.draw_reach(&sections, frame * 8, 0, 1.0, &mut mesh);
            assert!(
                pixels
                    .iter()
                    .zip(terrain)
                    .filter(|(a, b)| **a != *b)
                    .count()
                    > 20,
                "water should be visible for {}",
                profile.name
            );
            let hash = pixels
                .iter()
                .fold(0xcbf2_9ce4_8422_2325_u64, |digest, &pixel| {
                    (digest ^ u64::from(pixel)).wrapping_mul(0x0000_0100_0000_01b3)
                });
            if frame == 0 {
                first_frame[index] = hash;
            }
            animated[index] |= hash != first_frame[index];
            fingerprints[index] = fingerprints[index].rotate_left(1) ^ hash;
            let (left, top) = (index % COLS * TILE, index / COLS * TILE);
            for (pixel_index, pixel) in pixels.into_iter().enumerate() {
                let (x, y) = (pixel_index % TILE, pixel_index / TILE);
                if pixel == 0 {
                    continue;
                }
                image[(top + y) * width + left + x] = [
                    ((pixel >> 11) * 255 / 31) as u8,
                    (((pixel >> 5) & 63) * 255 / 63) as u8,
                    ((pixel & 31) * 255 / 31) as u8,
                ];
            }
        }
        let mut file = io::BufWriter::new(std::fs::File::create(
            output.join(format!("cascade-{frame:02}.ppm")),
        )?);
        writeln!(file, "P6\n{width} {height}\n255")?;
        for pixel in image {
            file.write_all(&pixel)?;
        }
    }
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        assert!(animated[index], "{} is static", profiles[index].name);
        assert!(
            !fingerprints[..index].contains(fingerprint),
            "{} is repeated",
            profiles[index].name
        );
    }
    Ok(())
}
