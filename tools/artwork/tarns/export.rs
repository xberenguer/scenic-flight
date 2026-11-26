use scenic_flight::{
    alpine_lakes::{AlpineLake, TarnShoreline},
    render::Camera,
    sky::Sky,
    world::{Surface, World},
    world_mesh::WorldMesh,
};
use std::{
    io::{self, Write},
    path::Path,
};

#[path = "../relief.rs"]
mod relief;

const TILE: usize = 88;
const COLS: usize = 5;

fn main() -> io::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .expect("Provide an output directory");
    let output = Path::new(&path);
    let profiles: Vec<_> = (0..TarnShoreline::COUNT)
        .map(|i| TarnShoreline::from_index(i).unwrap())
        .collect();
    let sites: Vec<_> = include_str!("sites.txt")
        .lines()
        .zip(profiles.iter())
        .map(|(line, &profile)| {
            let mut values = line.split_whitespace();
            let seed: u64 = values.next().unwrap().parse().unwrap();
            let tx = values.next().unwrap().parse().unwrap();
            let tz = values.next().unwrap().parse().unwrap();
            let world = World::new(seed);
            let lake =
                AlpineLake::generate(&world, seed, tx, tz).expect("surveyed tarn for artwork");
            assert_eq!(lake.shoreline(), profile);
            let bounds = footprint(&world, &lake);
            (world, lake, bounds)
        })
        .collect();
    assert_eq!(sites.len(), profiles.len());
    let names = profiles
        .iter()
        .map(|p| p.name())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(output.join("tarns.txt"), names)?;
    let sky = Sky::from_time(41, 0, 0.48);
    let width = TILE * COLS;
    let height = TILE * profiles.len().div_ceil(COLS);
    for frame in 0..16 {
        let mut image = vec![[19, 24, 30]; width * height];
        let heading = 0.54 + frame as f32 * std::f32::consts::TAU / 16.0;
        let (sin, cos) = heading.sin_cos();
        let view = Camera {
            x: -28.0 * sin,
            z: -28.0 * cos,
            altitude: 25.0,
            heading,
            pitch: ((25.0_f32 - 0.8) / 28.0).atan(),
            bank: 0.0,
        };
        for (index, &profile) in profiles.iter().enumerate() {
            let mut pixels = [0; TILE * TILE];
            let mut depth = [u16::MAX; TILE * TILE];
            let mut mesh =
                WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), view, &sky).unwrap();
            let (world, lake, [left, near, right, far]) = &sites[index];
            let [dx, dz] = [right - left, far - near];
            let scale = 29.0 / dx.max(dz);
            relief::draw(&mut mesh, [dx * scale, dz * scale], |[u, v]| {
                let x = left + (u + 1.0) * 0.5 * dx;
                let z = near + (v + 1.0) * 0.5 * dz;
                let ground = world.sample(x, z);
                let sample = scenic_flight::scenery::overlay_alpine(lake, x, z, ground);
                (sample != ground).then(|| {
                    let elevation = (sample.elevation - lake.level()).clamp(0.0, 6.8);
                    (0.25 + elevation * scale * 0.75, material(sample.surface))
                })
            });
            let (left, top) = (index % COLS * TILE, index / COLS * TILE);
            assert!(
                pixels.iter().any(|&pixel| pixel != 0),
                "empty profile for {}",
                profile.name()
            );
            for (index, pixel) in pixels
                .into_iter()
                .enumerate()
                .filter(|(_, pixel)| *pixel != 0)
            {
                let (x, y) = (index % TILE, index / TILE);
                assert!(
                    (2..TILE - 2).contains(&x) && (2..TILE - 2).contains(&y),
                    "clipped profile for {}",
                    profile.name()
                );
                image[(top + y) * width + left + x] = [
                    ((pixel >> 11) * 255 / 31) as u8,
                    (((pixel >> 5) & 63) * 255 / 63) as u8,
                    ((pixel & 31) * 255 / 31) as u8,
                ];
            }
        }
        let file = std::fs::File::create(output.join(format!("tarns-{frame:02}.ppm")))?;
        let mut writer = io::BufWriter::new(file);
        writeln!(writer, "P6\n{width} {height}\n255")?;
        for pixel in image {
            writer.write_all(&pixel)?;
        }
    }
    Ok(())
}

fn footprint(world: &World, lake: &AlpineLake) -> [f32; 4] {
    let [left, near, right, far] = lake.bounds();
    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    for j in 0..63 {
        for i in 0..63 {
            let x = left + (right - left) * i as f32 / 63.0;
            let z = near + (far - near) * j as f32 / 63.0;
            if lake.sample(x, z, world.sample(x, z)).is_some() {
                bounds = [
                    bounds[0].min(x),
                    bounds[1].min(z),
                    bounds[2].max(x),
                    bounds[3].max(z),
                ];
            }
        }
    }
    [
        bounds[0] - 0.5,
        bounds[1] - 0.5,
        bounds[2] + 0.5,
        bounds[3] + 0.5,
    ]
}

fn material(surface: Surface) -> [u8; 3] {
    match surface {
        Surface::TarnWater => [37, 89, 112],
        Surface::TarnShallows => [62, 126, 137],
        Surface::WetMeadow => [99, 125, 77],
        Surface::AlpineMeadow => [127, 145, 91],
        Surface::ReedMarsh => [91, 105, 55],
        Surface::Gravel => [143, 136, 122],
        Surface::Rock => [120, 120, 117],
        Surface::ScreeFine => [150, 142, 128],
        Surface::Sand => [192, 173, 132],
        _ => unreachable!("tarn surface"),
    }
}
