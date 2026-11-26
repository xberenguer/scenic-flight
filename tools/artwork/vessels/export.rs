use scenic_flight::{
    maritime::{Anchorage, DeckMaterial, ShipSample, Vessel},
    render::Camera,
    sky::Sky,
    world_mesh::WorldMesh,
};
use std::collections::BTreeSet;
use std::io::{self, Write};

const SIDE: usize = 112;
const ACROSS: usize = 40;
const ALONG: usize = 90;
const BACKGROUND: [u8; 3] = [19, 24, 30];

fn shade(material: DeckMaterial) -> [u8; 3] {
    match material {
        DeckMaterial::Hull => [74, 103, 124],
        DeckMaterial::Deck => [184, 171, 140],
        DeckMaterial::Bridge => [218, 221, 207],
        DeckMaterial::Cargo => [166, 102, 68],
    }
}

fn render(ship: Anchorage, length: f32, width: f32, turn: f32) -> [u16; SIDE * SIDE] {
    let (sin, cos) = turn.sin_cos();
    let x = length * (1.85 * cos - 1.38 * sin);
    let z = length * (-1.38 * cos - 1.85 * sin);
    let altitude = length * 0.91;
    let camera = Camera {
        x, z, altitude,
        heading: (-x).atan2(-z),
        pitch: (altitude / x.hypot(z)).atan() + 0.025,
        bank: 0.0,
    };
    let sky = Sky::from_time(41, 0, 0.50);
    let mut pixels = [0; SIDE * SIDE];
    let mut depth = [u16::MAX; SIDE * SIDE];
    let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
    let dx = width * 2.10 / ACROSS as f32;
    let dz = length * 2.10 / ALONG as f32;
    let left = -(ACROSS as f32) * dx * 0.5;
    let near = -(ALONG as f32) * dz * 0.5;
    let mut cells = [None; ACROSS * ALONG];
    for row in 0..ALONG {
        for col in 0..ACROSS {
            cells[row * ACROSS + col] = ship.sample(
                left + (col as f32 + 0.5) * dx,
                near + (row as f32 + 0.5) * dz,
            );
        }
    }
    let sample = |col: i32, row: i32| -> Option<ShipSample> {
        if (0..ACROSS as i32).contains(&col) && (0..ALONG as i32).contains(&row) {
            cells[row as usize * ACROSS + col as usize]
        } else {
            None
        }
    };

    for row in 0..ALONG as i32 {
        for col in 0..ACROSS as i32 {
            let Some(deck) = sample(col, row) else { continue };
            let x = left + col as f32 * dx;
            let z = near + row as f32 * dz;
            let h = deck.height;
            let corners = [[x, h, z], [x + dx, h, z], [x + dx, h, z + dz], [x, h, z + dz]];
            let color = shade(deck.material);
            mesh.triangle([corners[0], corners[1], corners[2]], color);
            mesh.triangle([corners[0], corners[2], corners[3]], color);

            for (edge, (u, v)) in [(0, -1), (1, 0), (0, 1), (-1, 0)].into_iter().enumerate() {
                let side = sample(col + u, row + v);
                let bottom = side.map_or(0.0, |lower| lower.height);
                if bottom >= h {
                    continue;
                }
                let [a, b] = [corners[edge], corners[(edge + 1) % 4]];
                let wall = if side.is_none() { shade(DeckMaterial::Hull) } else { color };
                let lower_a = [a[0], bottom, a[2]];
                let lower_b = [b[0], bottom, b[2]];
                mesh.triangle([a, b, lower_b], wall);
                mesh.triangle([a, lower_b, lower_a], wall);
            }
        }
    }
    pixels
}

fn rgb(pixel: u16) -> [u8; 3] {
    if pixel == 0 {
        return BACKGROUND;
    }
    [
        (((pixel >> 11) & 31) * 255 / 31) as u8,
        (((pixel >> 5) & 63) * 255 / 63) as u8,
        ((pixel & 31) * 255 / 31) as u8,
    ]
}

fn main() -> io::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let metadata_path = args.next().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "expected metadata output path")
    })?;
    let service = args.next().is_some_and(|flag| flag == "--service");
    let turn = args.next().and_then(|angle| angle.to_str()?.parse().ok()).unwrap_or(0.0);
    let vessels: Vec<_> = Vessel::ALL.into_iter()
        .filter(|vessel| (*vessel == Vessel::Service) == service).collect();
    let columns = if service { 5 } else { 10 };
    let count: usize = vessels.iter().map(|vessel| usize::from(vessel.variants())).sum();
    let width = SIDE * columns;
    let height = SIDE * count.div_ceil(columns);
    let mut atlas = vec![0; width * height * 3];
    let mut images = BTreeSet::new();
    let mut metadata = Vec::new();
    for vessel in vessels {
        let fleet: Vec<_> = (0..vessel.variants())
            .map(|variant| Anchorage::catalog(vessel, variant).unwrap()).collect();
        let [length, beam] = fleet.iter().map(|ship| ship.half_extents())
            .fold([0.0_f32; 2], |[a, b], [l, w]| [a.max(l), b.max(w)]);
        for (variant, ship) in fleet.into_iter().enumerate() {
            let [model_length, model_beam] = if service { ship.half_extents() } else { [length, beam] };
            let pixels = render(ship, model_length, model_beam, turn);
            let index = metadata.len();
            metadata.push(format!(r#"{{"family":{:?},"configuration":{:?}}}"#,
                vessel.name(), ship.configuration()));
            let (col, row) = (index % columns, index / columns);
            let mut visible = 0;
            for (i, pixel) in pixels.into_iter().enumerate() {
                let (x, y) = (i % SIDE, i / SIDE);
                if pixel != 0 {
                    assert!((2..SIDE - 2).contains(&x) && (2..SIDE - 2).contains(&y),
                        "clipped vessel {vessel:?} / {variant}");
                    visible += 1;
                }
                let offset = ((row * SIDE + y) * width + col * SIDE + x) * 3;
                atlas[offset..offset + 3].copy_from_slice(&rgb(pixel));
            }
            assert!(visible >= 70, "empty vessel {vessel:?} / {variant}");
            assert!(images.insert(pixels), "repeated vessel artwork {vessel:?} / {variant}");
        }
    }

    std::fs::write(metadata_path, format!("[\n{}\n]\n", metadata.join(",\n")))?;
    eprintln!("Verified {} distinct vessel renders from production hull and deck sampling.", images.len());
    let mut out = io::BufWriter::new(io::stdout().lock());
    write!(out, "P6\n{width} {height}\n255\n")?;
    out.write_all(&atlas)?;
    out.flush()
}
