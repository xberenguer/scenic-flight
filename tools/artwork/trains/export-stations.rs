#[allow(dead_code)]
#[path = "../../../src/train_art/freight_catalog.rs"]
mod train_art;

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum StationLayout {
    RuralHalt,
    TownStop,
    RegionalStation,
    FreightApron { regional: bool },
    FreightShed,
}

#[allow(dead_code)]
#[path = "../../../src/rail_station_art/platform.rs"]
mod platform;

use scenic_flight::{render::Camera, sky::Sky, world_mesh::WorldMesh};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

const SIDE: usize = 64;
const COLS: usize = 5;
const TILE: usize = 40;
const LEFT: usize = 12;
const TOP: usize = 13;
const WIDTH: usize = TILE * COLS;

fn main() -> io::Result<()> {
    use platform::Fixture::*;
    let forms = [
        StationClock,
        TimetableCase,
        PlatformMarker,
        LampPair,
        DispatchHut,
        LuggageShelter,
        ParcelTrolley,
        TicketKiosk,
        WaitingScreen,
        NewsStand,
        Semaphore,
        SignalCabinet,
        ToolLocker,
        SandBox,
        TrackGaugeRack,
        JibCrane,
        LoadingHoist,
        WeighDesk,
        SackHopper,
        HoseManifold,
        TimberBunks,
        CoilCradles,
        MilkChurns,
        DrumPallet,
        StackedCrates,
    ];
    let output = std::env::args_os()
        .nth(1)
        .expect("Provide the atlas output directory");
    let output = Path::new(&output);
    let sky = Sky::from_time(41, 0, 0.48);
    for frame in 0..24 {
        let heading = 0.48 + frame as f32 * core::f32::consts::TAU / 24.0;
        let (sin, cos) = heading.sin_cos();
        let camera = Camera {
            x: -6.25 * sin,
            z: -6.25 * cos,
            altitude: 3.08,
            heading,
            pitch: 0.305,
            bank: 0.0,
        };
        let mut sheet = vec![[19_u8, 24, 30]; WIDTH * WIDTH];
        for (index, model) in forms.into_iter().enumerate() {
            let mut pixels = [0; SIDE * SIDE];
            let mut depth = [u16::MAX; SIDE * SIDE];
            let mut mesh =
                WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
            model.paint(|triangle, color| {
                let width = if index >= 15 { 1.51 } else { 1.42 };
                mesh.triangle(triangle.map(|[u, y, v]| [u * width, y, v * 0.39]), color);
            });
            assert!(pixels.iter().any(|&pixel| pixel != 0), "{model:?}");
            for (p, pixel) in pixels
                .into_iter()
                .enumerate()
                .filter(|(_, pixel)| *pixel != 0)
            {
                let [x, y] = [p % SIDE, p / SIDE];
                assert!(
                    (LEFT + 2..LEFT + TILE - 2).contains(&x)
                        && (TOP + 2..TOP + TILE - 2).contains(&y),
                    "{model:?} is clipped"
                );
                sheet[(index / COLS * TILE + y - TOP) * WIDTH + index % COLS * TILE + x - LEFT] = [
                    ((pixel >> 11 & 31) * 255 / 31) as u8,
                    ((pixel >> 5 & 63) * 255 / 63) as u8,
                    ((pixel & 31) * 255 / 31) as u8,
                ];
            }
        }
        let mut image = BufWriter::new(File::create(
            output.join(format!("rail-stations-{frame:02}.ppm")),
        )?);
        write!(image, "P6\n{WIDTH} {WIDTH}\n255\n")?;
        for pixel in sheet {
            image.write_all(&pixel)?;
        }
        image.flush()?;
    }
    Ok(())
}
