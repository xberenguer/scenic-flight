use std::{io::{self, BufWriter, Write}, path::Path};
use scenic_flight::{
    aircraft_art::{draw_flight_scene, draw_preview},
    encounters::{flight_moment, moment_catalog},
    flight::{Aircraft, Flight, TICKS_PER_SECOND},
    render::{Camera, render_with_sky},
    sky::Sky,
    world::World,
};

const SIDE: usize = 128;
const FRAMES: usize = 60;
const OUTSIDE: [u8; 3] = [20, 26, 32];

fn write_ppm(path: &Path, images: &[[u16; SIDE * SIDE]]) -> io::Result<()> {
    let mut output = BufWriter::new(std::fs::File::create(path)?);
    writeln!(output, "P6\n{} {SIDE}\n255", SIDE * images.len())?;
    let mut row = Vec::with_capacity(SIDE * images.len() * 3);
    for y in 0..SIDE {
        row.clear();
        for image in images {
            for x in 0..SIDE {
                let dx = (2 * x + 1) as i32 - SIDE as i32;
                let dy = (2 * y + 1) as i32 - SIDE as i32;
                let pixel = image[y * SIDE + x];
                let color = if dx * dx + dy * dy > (SIDE * SIDE) as i32 {
                    OUTSIDE
                } else {
                    [
                        ((pixel >> 11) * 255 / 31) as u8,
                        (((pixel >> 5) & 63) * 255 / 63) as u8,
                        ((pixel & 31) * 255 / 31) as u8,
                    ]
                };
                row.extend(color);
            }
        }
        output.write_all(&row)?;
    }
    output.flush()
}

fn moment_window(aircraft: Aircraft, slot: usize) -> (u64, u64, u64) {
    let second = u64::from(TICKS_PER_SECOND);
    for seed in 0..30 {
        for tick in (90 * second..2_600 * second).step_by(second as usize) {
            if flight_moment(aircraft, seed, tick).is_some_and(|moment| moment.slot() == slot) {
                let mut start = tick;
                let mut end = tick;
                while flight_moment(aircraft, seed, start - 1).is_some() { start -= 1; }
                while flight_moment(aircraft, seed, end + 1).is_some() { end += 1; }
                return (seed, start, end);
            }
        }
    }
    panic!("Missing scheduled moment for {aircraft:?}, slot {slot}");
}

fn main() -> io::Result<()> {
    let directory = std::env::args_os().nth(1).expect("Provide scratch output directory");
    let directory = Path::new(&directory);
    let mut catalog = String::new();
    for aircraft in Aircraft::ALL {
        let id = aircraft as u8;
        catalog.push_str(&format!("{id}\t{}\t{}\n", aircraft.name(), moment_catalog(aircraft).join("\t")));
        let windows = core::array::from_fn::<_, 4, _>(|slot| moment_window(aircraft, slot));
        let mut flights = windows.map(|(seed, _, _)| Flight::start(&World::new(seed), seed, aircraft));
        let skies = core::array::from_fn::<_, 4, _>(|slot| {
            let view = flights[slot].view;
            Sky::from_time_at_position(windows[slot].0, 0, 0.53, [view.x, view.altitude, view.z])
        });
        let mut ground = [[0; SIDE * SIDE]; 4];
        for slot in 0..4 {
            let view = flights[slot].view;
            let camera = Camera { x: view.x, z: view.z, altitude: view.altitude,
                heading: view.heading, pitch: view.pitch, bank: view.bank };
            let world = World::new(windows[slot].0);
            render_with_sky(&world, camera, &skies[slot], &mut ground[slot], SIDE, SIDE).unwrap();
        }
        for frame in 0..FRAMES {
            let mut strip = ground;
            for slot in 0..4 {
                let (seed, start, end) = windows[slot];
                flights[slot].ticks = start + (end - start) * frame as u64 / (FRAMES - 1) as u64;
                draw_flight_scene(&flights[slot], seed, &skies[slot], &mut strip[slot], SIDE, SIDE);
            }
            write_ppm(&directory.join(format!("moments-{id}-{frame:03}.ppm")), &strip)?;
        }
        if matches!(aircraft, Aircraft::BushPlane | Aircraft::Racer | Aircraft::Trike) {
            for frame in 0..FRAMES {
                let mut plane = [(2 << 11) | (6 << 5) | 4; SIDE * SIDE];
                draw_preview(aircraft, frame as u32 * 4, &mut plane, SIDE, SIDE);
                write_ppm(&directory.join(format!("turntable-{id}-{frame:03}.ppm")), &[plane])?;
            }
        }
    }
    std::fs::write(directory.join("flight-moments.tsv"), catalog)
}
