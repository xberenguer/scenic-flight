//! Render the game's low-rise and airfield roofs as rotating artwork.
use scenic_flight::{render::Camera, sky::Sky, world_mesh::WorldMesh};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum SettlementSurface {
    PitchedRoof,
    FlatRoof,
    TowerRoof,
}

#[allow(dead_code)]
mod building_forms {
    include!(env!("SCENIC_BUILDING_FORMS"));
}

mod airfield {
    include!(env!("SCENIC_AIRFIELD_ROOFS"));
}

#[derive(Clone, Copy)]
enum Catalog {
    Street,
    Airfield,
}

impl Catalog {
    fn slug(self) -> &'static str {
        match self {
            Self::Street => "street-buildings",
            Self::Airfield => "airfield-buildings",
        }
    }

    fn profile(
        self,
        family: usize,
        index: usize,
        u: f32,
        v: f32,
    ) -> Option<(SettlementSurface, f32)> {
        use building_forms::*;
        match self {
            Self::Street => match family {
                0 => cottage(12 + index as u8, [u, v]),
                1 => duplex(10 + index as u8, [u, v]),
                2 => rowhouse(11 + index as u8, [v, u]),
                3 => industrial(12 + index as u8, [v, u]),
                _ => shop(9 + index as u8, [v, u]),
            },
            Self::Airfield => {
                airfield::sample((family * MODELS + index) as u8, u, v).map(|(roof, elevation)| {
                    let surface = match roof {
                        airfield::Roof::Flat => SettlementSurface::FlatRoof,
                        airfield::Roof::Pitched => SettlementSurface::PitchedRoof,
                        airfield::Roof::Glazed => SettlementSurface::TowerRoof,
                    };
                    (surface, elevation)
                })
            }
        }
    }

    fn dimensions(self, family: usize) -> [f32; 3] {
        match self {
            Self::Street => [
                [3.1, 3.6, 4.2],
                [4.0, 3.4, 4.8],
                [4.4, 2.9, 5.0],
                [4.7, 3.7, 4.8],
                [3.9, 3.4, 4.6],
            ][family],
            Self::Airfield => [
                [3.0, 3.1, 4.7],
                [4.6, 3.5, 6.1],
                [4.0, 3.2, 5.5],
                [2.5, 2.5, 7.9],
                [3.8, 2.6, 4.5],
            ][family],
        }
    }
}

const TILE: usize = 76;
const CROP: usize = 54;
const CROP_ORIGIN: [usize; 2] = [11, 20];
const CELLS: usize = 28;
const MODELS: usize = 5;
const BACKGROUND: [u8; 3] = [19, 24, 30];

#[derive(Clone, Copy)]
struct Cell {
    y: f32,
    roof: SettlementSurface,
}

fn face(mesh: &mut WorldMesh<'_>, [a, b, c, d]: [[f32; 3]; 4], color: [u8; 3]) {
    mesh.triangle([a, b, c], color);
    mesh.triangle([a, c, d], color);
}

fn draw(catalog: Catalog, family: usize, index: usize, angle: f32) -> [u16; TILE * TILE] {
    let (sin, cos) = angle.sin_cos();
    let camera = Camera {
        x: -14.4 * sin,
        z: -14.4 * cos,
        altitude: 9.9,
        heading: angle,
        pitch: 0.39,
        bank: 0.0,
    };
    let sky = Sky::from_time(41, 0, 0.47);
    let mut pixels = [0; TILE * TILE];
    let mut depth = [u16::MAX; TILE * TILE];
    let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (TILE, TILE), camera, &sky).unwrap();

    let [width, length, height] = catalog.dimensions(family);
    let normalized = |i| i as f32 * (2.0 / CELLS as f32) - 1.0;
    let mut grid = [[Cell {
        y: 0.0,
        roof: SettlementSurface::FlatRoof,
    }; CELLS]; CELLS];
    for (row, cells) in grid.iter_mut().enumerate() {
        for (col, cell) in cells.iter_mut().enumerate() {
            if let Some((roof, elevation)) = catalog.profile(
                family,
                index,
                normalized(col) + 1.0 / CELLS as f32,
                normalized(row) + 1.0 / CELLS as f32,
            ) {
                *cell = Cell {
                    roof,
                    y: elevation * height,
                };
            }
        }
    }
    let get = |row: usize, col: usize| {
        grid.get(row)
            .and_then(|cells| cells.get(col))
            .map_or(0.0, |cell| cell.y)
    };
    let tint = |color: [u8; 3], value: f32| color.map(|channel| (f32::from(channel) * value) as u8);
    let wall = [
        [173, 155, 121],
        [182, 167, 139],
        [149, 133, 122],
        [146, 142, 131],
        [170, 145, 114],
    ][family];
    for (row, cells) in grid.iter().enumerate() {
        for (col, cell) in cells.iter().enumerate() {
            let high = cell.y;
            if high == 0.0 {
                continue;
            }
            let [x0, x1] = [col, col + 1].map(|index| normalized(index) * width);
            let [z0, z1] = [row, row + 1].map(|index| normalized(index) * length);
            let roof = match cell.roof {
                SettlementSurface::PitchedRoof => [133, 100, 82],
                SettlementSurface::FlatRoof => [168, 152, 126],
                SettlementSurface::TowerRoof => [111, 144, 150],
            };
            let ridge_light = (0.95 + (get(row, col.wrapping_sub(1)) - get(row, col + 1)) * 0.14)
                .clamp(0.77, 1.12);
            face(
                &mut mesh,
                [
                    [x0, high, z0],
                    [x1, high, z0],
                    [x1, high, z1],
                    [x0, high, z1],
                ],
                tint(roof, ridge_light),
            );
            for ([x0, z0, x1, z1], low, shade) in [
                ([x0, z0, x1, z0], get(row.wrapping_sub(1), col), 0.87),
                ([x1, z0, x1, z1], get(row, col + 1), 0.74),
                ([x1, z1, x0, z1], get(row + 1, col), 0.95),
                ([x0, z1, x0, z0], get(row, col.wrapping_sub(1)), 0.81),
            ] {
                if high > low {
                    let color = if high - low > 0.60 { wall } else { roof };
                    face(
                        &mut mesh,
                        [[x0, low, z0], [x1, low, z1], [x1, high, z1], [x0, high, z0]],
                        tint(color, shade),
                    );
                }
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
        ((pixel >> 11) & 31) * 255 / 31,
        ((pixel >> 5) & 63) * 255 / 63,
        (pixel & 31) * 255 / 31,
    ]
    .map(|channel| channel as u8)
}

fn main() -> std::io::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .expect("Provide output directory");
    let path = Path::new(&path);
    let mode = std::env::args_os().nth(2);
    let catalog = if mode.as_deref() == Some(std::ffi::OsStr::new("airfield")) {
        Catalog::Airfield
    } else {
        Catalog::Street
    };
    let slug = catalog.slug();
    let side = CROP * MODELS;
    for frame in 0..16 {
        let angle = 0.6 + frame as f32 * std::f32::consts::TAU / 16.0;
        let mut atlas = vec![0; side * side * 3];
        for family in 0..MODELS {
            for design in 0..MODELS {
                let scene = draw(catalog, family, design, angle);
                assert!(
                    scene.iter().any(|&pixel| pixel != 0),
                    "Empty roof {family}/{design}"
                );
                for (index, pixel) in scene.into_iter().enumerate() {
                    let [x, y] = [index % TILE, index / TILE];
                    let [left, top] = match catalog {
                        Catalog::Street => CROP_ORIGIN,
                        Catalog::Airfield => [11, 13],
                    };
                    let inside = (left..left + CROP).contains(&x) && (top..top + CROP).contains(&y);
                    if pixel != 0 {
                        assert!(
                            inside,
                            "Cropped roof {family}/{design} at frame {frame}: {x},{y}"
                        );
                    }
                    if inside {
                        let offset =
                            ((family * CROP + y - top) * side + design * CROP + x - left) * 3;
                        atlas[offset..offset + 3].copy_from_slice(&rgb(pixel));
                    }
                }
            }
        }
        let mut file = BufWriter::new(File::create(path.join(format!("{slug}-{frame:02}.ppm")))?);
        write!(file, "P6\n{side} {side}\n255\n")?;
        file.write_all(&atlas)?;
        file.flush()?;
    }
    Ok(())
}
