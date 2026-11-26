use scenic_flight::{render::Camera, sky::Sky, world_mesh::WorldMesh};

#[allow(dead_code)]
mod freight_catalog {
    include!(env!("SCENIC_FREIGHT_CATALOG"));
}
mod wagons {
    include!(env!("SCENIC_WAGON_SOURCE"));
}
use freight_catalog::FreightRoute;
#[allow(dead_code)]
#[path = "../../../src/train_art/locomotives.rs"]
mod locomotives;

mod art {
    use super::*;
    use std::{
        collections::BTreeSet,
        io::{self, Write},
    };

    const SIDE: usize = 96;
    const GRID: usize = 5;
    const FAMILIES: usize = 10;
    const BACKGROUND: [u8; 3] = [19, 24, 30];

    fn rgb565(pixel: u16) -> [u8; 3] {
        [
            ((pixel >> 11) * 255 / 31) as u8,
            (((pixel >> 5) & 63) * 255 / 63) as u8,
            ((pixel & 31) * 255 / 31) as u8,
        ]
    }

    pub fn export(metadata_path: &std::path::Path) -> io::Result<()> {
        let camera = Camera {
            x: 11.9,
            z: -8.25,
            altitude: 9.45,
            heading: -0.966,
            pitch: 0.445,
            bank: 0.0,
        };
        let opposite = Camera {
            x: -12.3,
            z: -8.4,
            altitude: 9.30,
            heading: 0.972,
            pitch: 0.440,
            bank: 0.0,
        };
        let sky = Sky::from_time(41, 0, 0.5);
        let width = SIDE * GRID * 2;
        let height = SIDE * GRID * (FAMILIES / 2);
        let mut rgb = vec![0; width * height * 3];
        let mut geometry = BTreeSet::new();
        let mut silhouettes = BTreeSet::new();
        let mut reduced_silhouettes = BTreeSet::new();
        let mut metadata = Vec::with_capacity(usize::from(freight_catalog::DESIGN_COUNT));
        for variant in 0..freight_catalog::DESIGN_COUNT {
            let design = freight_catalog::design(variant);
            metadata.push(format!(
                r#"{{"family":{:?},"kind":{:?},"cargo":{:?},"equipment":{:?},"assignment":{:?},"purpose":{:?},"reference":{:?}}}"#,
                design.family.name(), design.kind(), design.cargo, design.equipment, design.assignment,
                design.purpose, design.reference_url(),
            ));
            let family = usize::from(variant) / 25;
            let position = usize::from(variant) % 25;
            let tile_x = (family % 2) * GRID + position % GRID;
            let tile_y = (family / 2) * GRID + position / GRID;
            let paint = [[174, 78, 62], [95, 135, 151], [174, 146, 91], [109, 133, 112], [167, 167, 154]][position / 5];
            let (pixels, depth) = render_wagon(variant, design.body_length(), paint, camera, &sky);
            let (_, other_depth) = render_wagon(variant, design.body_length(), paint, opposite, &sky);
            let mut visible = 0;
            for (index, pixel) in pixels.into_iter().enumerate() {
                let [x, y] = [index % SIDE, index / SIDE];
                if pixel != 0 {
                    assert!((4..SIDE - 4).contains(&x) && (4..SIDE - 4).contains(&y),
                        "clipped wagon {variant}");
                    visible += 1;
                }
                let tint = if pixel == 0 { BACKGROUND } else { rgb565(pixel) };
                let offset = ((tile_y * SIDE + y) * width + tile_x * SIDE + x) * 3;
                rgb[offset..offset + 3].copy_from_slice(&tint);
            }
            assert!(visible > 80, "empty wagon {variant}");
            assert!(silhouettes.insert(depth.map(|pixel| pixel != u16::MAX)),
                "repeated wagon silhouette at {SIDE} px for car {variant}");
            reduced_silhouettes.insert([coverage_at_third_scale(&depth), coverage_at_third_scale(&other_depth)]);
            assert!(geometry.insert(depth), "repeated wagon geometry {variant}");
        }
        std::fs::write(metadata_path, format!("[\n{}\n]\n", metadata.join(",\n")))?;
        eprintln!(
            "Freight audit: {} depth renders, {} material-independent silhouettes at {} px, {} two-angle pairs at {} px.",
            geometry.len(), silhouettes.len(), SIDE, reduced_silhouettes.len(), SIDE / 3,
        );
        let mut out = io::BufWriter::new(io::stdout().lock());
        write!(out, "P6\n{width} {height}\n255\n")?;
        out.write_all(&rgb)?;
        out.flush()
    }

    fn render_wagon(variant: u8, length: f32, paint: [u8; 3], camera: Camera, sky: &Sky)
        -> ([u16; SIDE * SIDE], [u16; SIDE * SIDE])
    {
        let mut pixels = [0; SIDE * SIDE];
        let mut depth = [u16::MAX; SIDE * SIDE];
        let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, sky).unwrap();
        wagons::draw(
            |t, side, height| [-side, height, (t - 0.5) * length],
            |[a, b, c, d], color| {
                mesh.triangle([a, b, c], color);
                mesh.triangle([a, c, d], color);
            },
            paint,
            variant,
            true,
            true,
        );
        (pixels, depth)
    }

    fn coverage_at_third_scale(depth: &[u16; SIDE * SIDE]) -> [bool; SIDE * SIDE / 9] {
        let side = SIDE / 3;
        std::array::from_fn(|i| {
            let x = 3 * (i % side);
            let y = 3 * (i / side);
            (0..3)
                .flat_map(|dy| (0..3).map(move |dx| depth[(y + dy) * SIDE + x + dx]))
                .filter(|&pixel| pixel != u16::MAX)
                .count() >= 5
        })
    }

    pub fn export_locomotives(path: &std::path::Path) -> io::Result<()> {
        use locomotives::Locomotive;

        const CELL: usize = 112;
        const WIDTH: usize = CELL * 3;
        const HEIGHT: usize = CELL * 2;
        let camera = Camera {
            x: 9.6, z: -6.4, altitude: 7.0, heading: -0.983, pitch: 0.425, bank: 0.0,
        };
        let sky = Sky::from_time(41, 0, 0.5);
        let mut atlas = vec![BACKGROUND; WIDTH * HEIGHT];
        let mut geometry = BTreeSet::new();
        for (index, locomotive) in [
            Locomotive::WideCab, Locomotive::HeavyHaul, Locomotive::RoadSwitcher,
            Locomotive::HighHood, Locomotive::DieselTransfer, Locomotive::BatteryTransfer,
        ].into_iter().enumerate() {
            let mut pixels = [0; CELL * CELL];
            let mut depth = [u16::MAX; CELL * CELL];
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (CELL, CELL), camera, &sky).unwrap();
            locomotives::draw(
                |t, side, height| [-side, height, (t - 0.5) * 13.6],
                |[a, b, c, d], color, _| {
                    mesh.triangle([a, b, c], color);
                    mesh.triangle([a, c, d], color);
                },
                locomotive, true, true, 0.0,
            );
            let mut visible = 0;
            for (i, pixel) in pixels.into_iter().enumerate().filter(|&(_, pixel)| pixel != 0) {
                let [x, y] = [i % CELL, i / CELL];
                assert!((5..CELL - 5).contains(&x) && (5..CELL - 5).contains(&y), "clipped {locomotive:?}");
                atlas[(index / 3 * CELL + y) * WIDTH + index % 3 * CELL + x] = rgb565(pixel);
                visible += 1;
            }
            assert!(visible > 80 && geometry.insert(depth), "empty or repeated {locomotive:?}");
        }
        let mut out = io::BufWriter::new(std::fs::File::create(path)?);
        write!(out, "P6\n{WIDTH} {HEIGHT}\n255\n")?;
        out.write_all(atlas.as_flattened())?;
        out.flush()
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [metadata, locomotive_atlas] = args.as_slice() else {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput,
            "expected wagon metadata and locomotive image output paths"));
    };
    art::export_locomotives(std::path::Path::new(locomotive_atlas))?;
    art::export(std::path::Path::new(metadata))
}
