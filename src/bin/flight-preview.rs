use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use scenic_flight::airfield::AirfieldSite;
use scenic_flight::aircraft_art::{draw_flight_scene, draw_preview};
use scenic_flight::alpine_lakes::AlpineLake;
use scenic_flight::beach::{BeachSite, BEACH_TILE_SIZE};
use scenic_flight::bridge_silhouette::BridgeSilhouettes;
use scenic_flight::coastal_highways::CoastalHighwayWork;
use scenic_flight::coastal_landforms::{CoastalLandform, CoastalLandmarks, LandformKind};
use scenic_flight::dry_landmarks::DryLandmark;
use scenic_flight::facility_stream::FacilityStream;
use scenic_flight::feature_tiles::FeatureTiles;
use scenic_flight::flight::{Aircraft, Flight, TICKS_PER_SECOND, View};
use scenic_flight::ground_details::GroundDetails;
use scenic_flight::highway_art::HighwayRibbon;
use scenic_flight::highway_stream::HighwayStream;
use scenic_flight::horizon_cache::HorizonCache;
use scenic_flight::distant_roads::DistantRoads;
use scenic_flight::land_stream::{LandBudget, LandStream};
use scenic_flight::lighthouses::{Lighthouse, LIGHTHOUSE_TILE};
use scenic_flight::local_routes::{LocalRouteStream, route_reservation};
use scenic_flight::maritime::{ANCHORAGE_TILE, Anchorage};
use scenic_flight::natural_stream::{NaturalBudget, NaturalStream, highway_reservation_with_nature, scenery_reservation_with_highways};
use scenic_flight::ports::Port;
use scenic_flight::rail_art::{corridor_screen_region, draw_rail, draw_rail_corridor, rail_screen_region};
use scenic_flight::rail_stream::{RailStream, scene_blocks};
use scenic_flight::rail_survey::{RailSurveyData, RailSurveyWork};
use scenic_flight::river_district::RiverDistrict;
use scenic_flight::river_stream::{RiverRequest, RiverStream, RiverSurvey};
use scenic_flight::render::{
    Camera, ScreenRect, render_from_with_sky, render_from_with_sky_buffered, render_terrain_layer_region_with_horizon, render_with_depth, render_with_sky,
};
use scenic_flight::scenery::{CoastalScenery, DistrictRegion, overlay_airfield, overlay_alpine, overlay_port, overlay_watershed};
use scenic_flight::sky::Sky;
use scenic_flight::skyline::Skyline;
use scenic_flight::terrain_cache::TerrainCache;
use scenic_flight::train_art::{Train, TRAIN_REACH};
use scenic_flight::ui;
use scenic_flight::watershed::{Watershed, WatershedWork};
use scenic_flight::weather_effects::{ValleyMist, WeatherEffects};
use scenic_flight::world::{SEA_LEVEL, Sample, Surface, World};
use scenic_flight::world_mesh::{bounds_in_view, WorldMesh};

fn camera(view: View) -> Camera {
    Camera {
        x: view.x,
        altitude: view.altitude,
        z: view.z,
        heading: view.heading,
        pitch: view.pitch,
        bank: view.bank,
    }
}

fn parse_seed(value: &str) -> Result<u64, Box<dyn Error>> {
    if let Some(hex) = value.strip_prefix("0x") {
        Ok(u64::from_str_radix(hex, 16)?)
    } else {
        Ok(value.parse()?)
    }
}

fn parse_aircraft(value: &str) -> Result<Aircraft, Box<dyn Error>> {
    Aircraft::ALL.get(value.parse::<usize>()?).copied().ok_or_else(|| "Aircraft index must be 0 through 7".into())
}

fn ppm(path: impl AsRef<Path>, pixels: &[u16], pixels_per_side: usize) -> std::io::Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(writer, "P6\n{pixels_per_side} {pixels_per_side}\n255")?;
    for &pixel in pixels {
        let r = ((pixel >> 11) & 31) * 255 / 31;
        let g = ((pixel >> 5) & 63) * 255 / 63;
        let b = (pixel & 31) * 255 / 31;
        writer.write_all(&[r as u8, g as u8, b as u8])?;
    }
    writer.flush()
}

fn render(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [seed, aircraft, seconds, time, size, path, mode] = arguments else {
        return Err("render needs SEED AIRCRAFT SECONDS TIME SIZE OUTPUT MODE".into());
    };
    let seed = parse_seed(seed)?;
    let aircraft = parse_aircraft(aircraft)?;
    let seconds = seconds.parse::<u64>()?;
    let size = size.parse::<usize>()?;
    if !(48..=512).contains(&size) {
        return Err("Size must be 48 through 512".into());
    }
    let world = World::new(seed);
    let mut flight = Flight::start(&world, seed, aircraft);
    let mut pixels = vec![0; size * size];
    if mode == "picker" {
        ui::draw_picker(aircraft, &mut pixels, size, size);
        draw_preview(aircraft, seconds as u32, &mut pixels, size, size);
        return Ok(ppm(path, &pixels, size)?);
    }
    for _ in 0..seconds * u64::from(TICKS_PER_SECOND) {
        flight.update(&world);
    }
    let view = camera(flight.view);
    let point = [view.x, view.altitude, view.z];
    let sky = if time == "auto" {
        Sky::at_position(seed, flight.ticks, point)
    } else {
        Sky::from_time_at_position(seed, flight.ticks, time.parse()?, point)
    };
    let before = Instant::now();
    match mode.as_str() {
        "cache" => {
            let mut cache = TerrainCache::new();
            cache.reset_for_altitude(world, view.x, view.z, view.altitude);
            render_from_with_sky(|x, z| cache.sample(x, z), view, &sky, &mut pixels, size, size)
        }
        "detailed" => {
            let mut cache = TerrainCache::new();
            cache.reset_for_altitude(world, view.x, view.z, view.altitude);
            let mut land = GroundDetails::new();
            land.reset(&world, seed, view.x, view.z);
            let mut coast = CoastalScenery::new();
            coast.reset(world, seed, view.x, view.z);
            let mut terrain = vec![0; size * size];
            render_from_with_sky_buffered(
                |x, z| coast.overlay(x, z, land.sample(x, z, cache.sample(x, z))),
                view, &sky, &mut pixels, &mut terrain, size, size,
            )
        }
        "direct" | "new-world" => render_with_sky(&world, view, &sky, &mut pixels, size, size),
        _ => return Err("Mode must be direct, cache, detailed, picker, or new-world".into()),
    }.map_err(|error| format!("Render failed ({error:?})"))?;
    draw_flight_scene(&flight, seed, &sky, &mut pixels, size, size);
    if mode == "new-world" {
        ui::draw_new_world(0.72, &mut pixels, size, size);
    }
    println!(
        "{}\t{seed:016x}\tx {:.3}\tz {:.3}\taltitude {:.3}\tbright {:.3}\t{:?}",
        aircraft.name(), flight.x, flight.z, flight.altitude, sky.daylight, before.elapsed(),
    );
    Ok(ppm(path, &pixels, size)?)
}

fn landmark(arguments: &[String], district: bool) -> Result<(), Box<dyn Error>> {
    let [seed, tile_x, tile_z, style, time, size, path] = arguments else {
        return Err("Use SEED TILE_X TILE_Z VIEW TIME SIZE OUTPUT".into());
    };
    let seed = parse_seed(seed)?;
    let world = World::new(seed);
    let tile_x = tile_x.parse::<i32>()?;
    let tile_z = tile_z.parse::<i32>()?;
    let size = size.parse::<usize>()?;
    if !(48..=512).contains(&size) {
        return Err("Size must be 48 through 512".into());
    }
    let altitude_angle = time.parse::<f32>()?;
    let sky = Sky::from_time(seed, 0, altitude_angle);
    let mut coastal = CoastalScenery::new();
    let mut region = None;
    let (x, z, elevation, heading) = if district {
        let plan = DistrictRegion::build(&world, tile_x, tile_z);
        let road = plan.road().ok_or("This region has no highway")?.polyline();
        let (point, other) = if style == "start" {
            (road[0], road[1])
        } else if style == "end" {
            (road[road.len() - 1], road[road.len() - 2])
        } else {
            return Err("District view must be start or end".into());
        };
        let heading = (point.x - other.x).atan2(point.z - other.z);
        println!("{} highway points, {} districts", road.len(), plan.towns().count());
        region = Some(plan);
        (point.x, point.z, point.elevation, heading)
    } else {
        let ship = Anchorage::generate(&world, seed, tile_x, tile_z).ok_or("This tile has no anchorage")?;
        let heading = style.parse()?;
        let distance = (ship.x - (tile_x * ANCHORAGE_TILE) as f32).abs();
        println!("{:?} at ({:.1}, {:.1}), {:.0} units from tile edge", ship.vessel, ship.x, ship.z, distance);
        (ship.x, ship.z, SEA_LEVEL, heading)
    };
    let (sin, cos) = heading.sin_cos();
    let distance = if district { 56.0 } else { 47.0 };
    let view = Camera {
        x: x - distance * sin,
        altitude: elevation + if district { 32.0 } else { 22.0 },
        z: z - distance * cos,
        heading,
        pitch: 0.26,
        bank: -0.035,
    };
    coastal.reset(world, seed, view.x, view.z);
    let mut pixels = vec![0; size * size];
    render_from_with_sky(
        |x, z| {
            let ground = coastal.overlay(x, z, world.sample(x, z));
            region.as_ref().map_or(ground, |area| area.overlay(x, z, ground))
        },
        view, &sky, &mut pixels, size, size,
    ).map_err(|error| format!("Render failed ({error:?})"))?;
    Ok(ppm(path, &pixels, size)?)
}

fn scene(
    world: &World, seed: u64, camera: Camera, sky: &Sky, size: usize, path: &str,
    sample: impl FnMut(f32, f32) -> Sample,
) -> Result<(), Box<dyn Error>> {
    let mut pixels = vec![0; size * size];
    let mut terrain = vec![0; size * size];
    render_from_with_sky_buffered(sample, camera, sky, &mut pixels, &mut terrain, size, size)
        .map_err(|error| format!("Render failed ({error:?})"))?;
    draw_chase_frame(world, seed, camera, sky, &mut pixels, size);
    Ok(ppm(path, &pixels, size)?)
}

fn draw_chase_frame(world: &World, seed: u64, camera: Camera, sky: &Sky, pixels: &mut [u16], size: usize) {
    let mut flight = Flight::start(world, seed, Aircraft::BushPlane);
    let (sin, cos) = camera.heading.sin_cos();
    flight.x = camera.x + sin * 12.0;
    flight.z = camera.z + cos * 12.0;
    flight.altitude = camera.altitude - 6.4;
    flight.heading = camera.heading;
    flight.bank = camera.bank * 2.1;
    flight.view = View {
        x: camera.x, z: camera.z, altitude: camera.altitude,
        heading: camera.heading, pitch: camera.pitch, bank: camera.bank,
    };
    draw_flight_scene(&flight, seed, sky, pixels, size, size);
}

fn port(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [seed, rx, rz, terminal, phase, size, output] = arguments else {
        return Err("Use port SEED REGION_X REGION_Z START_OR_END TIME SIZE OUTPUT".into());
    };
    let seed = parse_seed(seed)?;
    let world = World::new(seed);
    let district = DistrictRegion::build(&world, rx.parse()?, rz.parse()?);
    let road = district.road().ok_or("No highway in that region")?.polyline();
    let point = match terminal.as_str() {
        "start" => road[0],
        "end" => road[road.len() - 1],
        _ => return Err("Terminal must be start or end".into()),
    };
    let port = Port::generate(&world, seed, point.x, point.z).ok_or("No graded harbor at this endpoint")?;
    let [x, z] = port.shore_origin();
    let [out_x, out_z] = port.seaward();
    let camera = Camera {
        x: x + out_x * 39.0 + out_z * 34.0,
        z: z + out_z * 39.0 - out_x * 34.0,
        altitude: 24.0,
        heading: (-out_x * 39.0 - out_z * 22.0).atan2(-out_z * 39.0 + out_x * 22.0),
        pitch: 0.23,
        bank: -0.030,
    };
    let sky = Sky::from_time(seed, 0, phase.parse()?);
    println!("{:?} at ({x:.2}, {z:.2}) with {} moored vessels", port.kind(), port.vessels().len());
    scene(&world, seed, camera, &sky, size.parse()?, output, |x, z| {
        let natural = world.sample(x, z);
        let marked = overlay_port(&port, x, z, natural);
        if marked != natural { marked } else { district.overlay(x, z, natural) }
    })
}

fn river(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [seed, rx, rz, phase, size, output] = arguments else {
        return Err("Use river SEED REGION_X REGION_Z TIME SIZE OUTPUT".into());
    };
    let seed = parse_seed(seed)?;
    let world = World::new(seed);
    let mut watershed = Watershed::new();
    let mut work = WatershedWork::new();
    watershed.rebuild(&world, seed, rx.parse()?, rz.parse()?, &mut work);
    let reach = watershed.reaches().filter(|reach| !reach.reaches_sea)
        .min_by(|a, b| (a.start_level - 9.0).abs().total_cmp(&(b.start_level - 9.0).abs()))
        .ok_or("No approved stream reach")?;
    let angle = (reach.start[0] - reach.end[0]).atan2(reach.start[1] - reach.end[1]);
    let (sin, cos) = angle.sin_cos();
    let camera = Camera {
        x: reach.end[0] - sin * 72.0 + cos * 25.0,
        z: reach.end[1] - cos * 72.0 - sin * 25.0,
        altitude: reach.start_level + 24.0,
        heading: angle - 0.24,
        pitch: 0.27,
        bank: 0.035,
    };
    let sky = Sky::from_time(seed, 0, phase.parse()?);
    let mut ground = GroundDetails::new();
    ground.reset(&world, seed, camera.x, camera.z);
    println!("{} streams and {} verified reaches", watershed.streams().len(), watershed.reaches().count());
    scene(&world, seed, camera, &sky, size.parse()?, output, |x, z| {
        let bare = world.sample(x, z);
        let changed = overlay_watershed(&watershed, x, z, bare);
        if changed != bare { changed } else { ground.sample(x, z, bare) }
    })
}

fn connected_river(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [seed, rx, rz, phase, output] = arguments else {
        return Err("Use connected-river SEED REGION_X REGION_Z TIME OUTPUT".into());
    };
    const SIDE: usize = 112;
    let seed = parse_seed(seed)?;
    let world = World::new(seed);
    let district = DistrictRegion::build(&world, rx.parse()?, rz.parse()?);
    let river = RiverDistrict::prepare(&world, seed, &district, &mut WatershedWork::new(), |_, _| false)
        .ok_or("No connected river crossing in this region")?;
    let [_, a, b, _] = *river.crossings().bridges()[0].sections();
    let x = (a[0] + b[0]) * 0.5;
    let z = (a[2] + b[2]) * 0.5;
    let height = (a[1] + b[1]) * 0.5;
    let angle = (b[0] - a[0]).atan2(b[2] - a[2]);
    let (sin, cos) = angle.sin_cos();
    let camera_x = x - 8.5 * sin + 33.0 * cos;
    let camera_z = z - 8.5 * cos - 33.0 * sin;
    let camera = Camera {
        x: camera_x, z: camera_z, altitude: height + 16.0,
        heading: (x - camera_x).atan2(z - camera_z) + 0.21,
        pitch: 0.35, bank: -0.024,
    };
    let sky = Sky::from_time(seed, 0, phase.parse()?);
    let mut pixels = [0; SIDE * SIDE];
    let mut work = [0; SIDE * SIDE];
    let mut depth = [0; SIDE * SIDE];
    render_with_depth(|x, z| river.overlay(x, z, district.overlay(x, z, world.sample(x, z))),
        camera, &sky, &mut pixels, &mut work, &mut depth, (SIDE, SIDE))
        .map_err(|error| format!("Render failed ({error:?})"))?;
    let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap();
    river.draw(camera, &mut mesh);
    draw_chase_frame(&world, seed, camera, &sky, &mut pixels, SIDE);
    println!("{} prepared bridge decks", river.crossings().bridges().len());
    Ok(ppm(output, &pixels, SIDE)?)
}

fn airfield(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [seed, rx, rz, kind, phase, size, output] = arguments else {
        return Err("Use airfield SEED REGION_X REGION_Z TYPE TIME SIZE OUTPUT".into());
    };
    let seed = parse_seed(seed)?;
    let world = World::new(seed);
    let rx = rx.parse()?;
    let rz = rz.parse()?;
    let district = DistrictRegion::build(&world, rx, rz);
    let site = match kind.as_str() {
        "country" => AirfieldSite::generate(&world, seed, rx, rz),
        "regional" => district.road().and_then(|road| AirfieldSite::for_road(&world, seed, road)),
        _ => return Err("Airfield type must be country or regional".into()),
    }.ok_or("No airfield in this fixture")?;
    let (x, z) = site.center();
    let [a, b] = site.runway_endpoints();
    let heading = (b.0 - a.0).atan2(b.1 - a.1);
    let (sin, cos) = heading.sin_cos();
    let camera = Camera {
        x: x - sin * 88.0 + cos * 28.0,
        z: z - cos * 88.0 - sin * 28.0,
        altitude: world.elevation(x, z) + 28.0,
        heading: heading - 0.26,
        pitch: 0.25,
        bank: 0.016,
    };
    let sky = Sky::from_time(seed, 0, phase.parse()?);
    println!("{:?} airfield ({x:.2}, {z:.2}), joined road {:?}", site.kind, site.road_contact());
    scene(&world, seed, camera, &sky, size.parse()?, output, |x, z| {
        let natural = world.sample(x, z);
        let built = overlay_airfield(&site, x, z, natural);
        if built != natural { built } else { district.overlay(x, z, natural) }
    })
}

fn coastal_landmark(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [seed, tx, tz, phase, size, path] = arguments else {
        return Err("Use coastal-landmark SEED TILE_X TILE_Z TIME SIZE OUTPUT".into());
    };
    let seed = parse_seed(seed)?;
    let size = size.parse::<usize>()?;
    if !(48..=512).contains(&size) {
        return Err("Size must be 48 through 512".into());
    }
    let world = World::new(seed);
    let form = CoastalLandform::generate(&world, seed, tx.parse()?, tz.parse()?)
        .ok_or("No verified coastal landmark in that tile")?;
    let [x, z] = form.center();
    let [coast_x, coast_z] = form.shore();
    let shore_length = (x - coast_x).hypot(z - coast_z);
    let out_x = (x - coast_x) / shore_length;
    let out_z = (z - coast_z) / shore_length;
    let (distance, lateral, altitude, target): (f32, f32, f32, f32) = match form.kind() {
        LandformKind::Stack => (43.0, 18.0, 20.0, 8.0),
        LandformKind::Skerries => (31.0, -8.0, 12.0, 2.0),
        LandformKind::Arch => (42.0, 15.0, 14.0, 6.0),
    };
    let camera_x = x + out_x * distance - out_z * lateral;
    let camera_z = z + out_z * distance + out_x * lateral;
    let camera = Camera {
        x: camera_x, z: camera_z, altitude,
        heading: (x - camera_x).atan2(z - camera_z) + 0.30,
        pitch: ((altitude - target) / distance.hypot(lateral)).atan(),
        bank: 0.018,
    };
    let sky = Sky::from_time(seed, 0, phase.parse()?);
    let mut pixels = vec![0; size * size];
    let mut terrain = vec![0; size * size];
    let mut depth = vec![0; size * size];
    render_with_depth(
        |x, z| world.sample(x, z), camera, &sky, &mut pixels, &mut terrain, &mut depth, (size, size),
    ).map_err(|error| format!("Render failed ({error:?})"))?;
    let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (size, size), camera, &sky)
        .ok_or("Unsupported mesh viewport")?;
    form.draw(&mut mesh);
    draw_chase_frame(&world, seed, camera, &sky, &mut pixels, size);
    println!("{:?} at ({x:.2}, {z:.2})", form.kind());
    Ok(ppm(path, &pixels, size)?)
}

fn basin(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [seed, tx, tz, kind, angle, phase, output] = arguments else {
        return Err("Use basin SEED TILE_X TILE_Z TARN_OR_DRY HEADING TIME OUTPUT".into());
    };
    let seed = parse_seed(seed)?;
    let tx = tx.parse()?;
    let tz = tz.parse()?;
    let world = World::new(seed);
    let sky = Sky::from_time(seed, 0, phase.parse()?);
    let heading = angle.parse::<f32>()?;
    let (sin, cos) = heading.sin_cos();
    let view = |[x, z]: [f32; 2], surface: f32, climb: f32, distance: f32| {
        let camera_x = x - sin * distance + cos * 11.0;
        let camera_z = z - cos * distance - sin * 11.0;
        let altitude = (surface + climb).max(world.elevation(camera_x, camera_z) + 18.0);
        Camera { x: camera_x, z: camera_z, altitude, heading: heading + 0.12,
            pitch: ((altitude - surface) / distance).atan(), bank: 0.025 }
    };
    match kind.as_str() {
        "tarn" => {
            let lake = AlpineLake::generate(&world, seed, tx, tz).ok_or("No contained tarn in this tile")?;
            let [x, _, z] = lake.floor();
            let camera = view([x, z], lake.level(), 39.0, 73.0);
            scene(&world, seed, camera, &sky, 112, output, |x, z| {
                overlay_alpine(&lake, x, z, world.sample(x, z))
            })
        }
        "dry" => {
            let landform = DryLandmark::generate(&world, seed, tx, tz).ok_or("No dry basin in this tile")?;
            let camera = view(landform.center(), landform.floor_elevation(), 26.0, 59.0);
            scene(&world, seed, camera, &sky, 112, output, |x, z| {
                landform.overlay(x, z, world.sample(x, z))
            })
        }
        _ => Err("Basin type must be tarn or dry".into()),
    }
}

fn replay(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [seed, aircraft, advance, duration, directory] = arguments else {
        return Err("Use replay SEED AIRCRAFT START_SECONDS DURATION_SECONDS DIRECTORY".into());
    };
    let seed = parse_seed(seed)?;
    let aircraft = parse_aircraft(aircraft)?;
    let world = World::new(seed);
    let duration = duration.parse::<u64>()?;
    let advance = advance.parse::<u64>()?;
    if !(1..=60).contains(&duration) || advance > 86_400 {
        return Err("Replay accepts 1–60 seconds of footage from the first day of flight".into());
    }

    const SIDE: usize = 112;
    let dimensions = (SIDE, SIDE);
    let mut flight = Flight::start(&world, seed, aircraft);
    let mut pixels = [0; SIDE * SIDE];
    let mut work = [0; SIDE * SIDE];
    let mut depth = [0; SIDE * SIDE];
    let mut ground = GroundDetails::new();
    let mut vessels = CoastalScenery::new();
    let mut cliffs = CoastalLandmarks::new();
    let mut terrain = TerrainCache::new();
    let mut horizon = HorizonCache::new();
    let mut skyline = Skyline::new();
    let mut bridge_silhouettes = BridgeSilhouettes::new();
    let mut district = LandStream::<8>::new();
    let mut facilities = FacilityStream::<8>::new(world, seed);
    let mut rivers = RiverStream::<3>::new(world, seed);
    let mut river_work = WatershedWork::new();
    let mut river_survey: Option<(RiverRequest, RiverSurvey)> = None;
    let mut routes = LocalRouteStream::<8>::new(world, seed);
    let mut highways = HighwayStream::<4>::new(world, seed);
    let mut distant_roads = DistantRoads::new();
    let mut coast_work = CoastalHighwayWork::new();
    let mut regional_rail = RailStream::new(world, seed);
    let mut rail_data = RailSurveyData::new();
    let mut rail_work = RailSurveyWork::new();
    let mut weather = WeatherEffects::new();
    let mut valley_mist = ValleyMist::new();
    let mut natural = NaturalStream::new(world, seed);
    let mut lights = FeatureTiles::<Lighthouse, 9, 12>::new(LIGHTHOUSE_TILE);
    let mut beaches = FeatureTiles::<BeachSite, 5, 16>::new(BEACH_TILE_SIZE);
    let start = advance * u64::from(TICKS_PER_SECOND);
    let mut first_view = camera(flight.view);
    district.reset(world);
    district.preflight(first_view, false);
    if flight.stage_town_approach(&world, &district) {
        first_view = camera(flight.view);
        district.preflight(first_view, false);
    }
    facilities.preflight(&district);
    rivers.preflight(&district, &facilities, first_view, &mut river_work);
    routes.preflight(&district, first_view, |key, route| {
        route_reservation(&world, &facilities, &rivers, key, route)
    });
    highways.preflight_with_coast(&district, first_view, &mut coast_work, |pair, link| {
        highway_reservation_with_nature(&district, &facilities, &rivers, &routes, &natural, pair, link)
    }, || {});
    cliffs.reset(&world, seed, first_view.x, first_view.z);
    natural.preflight(first_view, NaturalBudget { alpine_tiles: 49, dry_tiles: 49 }, |bounds| {
        scenery_reservation_with_highways(&district, &facilities, &rivers, &routes, &highways, bounds)
    });
    facilities.publish_roadside_sites(first_view, true, |bounds| {
        scenic_flight::facility_stream::roadside_reservation(&district, &rivers, &routes, &highways, &natural, bounds)
    });
    lights.reset(first_view.x, first_view.z, |x, z| Lighthouse::generate(&world, seed, x, z));
    beaches.reset(first_view.x, first_view.z, |x, z| BeachSite::generate(&world, seed, x, z));
    std::fs::create_dir_all(directory)?;
    let mut landmark_frames = 0;
    let mut district_frames = 0;
    let mut facility_frames = 0;
    let mut river_frames = 0;
    let mut wetland_frames = 0;
    let mut waterfall_frames = 0;
    let mut bridge_frames = 0;
    let mut route_frames = 0;
    let mut portal_frames = 0;
    let mut natural_frames = 0;
    let mut highway_frames = 0;
    let mut coastal_bridge_frames = 0;
    let mut rain_frames = 0;
    let mut skyline_frames = 0;
    let mut distant_road_frames = 0;
    let mut distant_bridge_frames = 0;
    for frame in 0..(advance + duration) * u64::from(TICKS_PER_SECOND) {
        if frame > 0 {
            flight.update_with_obstacles(&world, |x, z| {
                cliffs.obstruction(x, z).max(district.obstruction(x, z)).max(facilities.obstruction(x, z))
                    .max(rivers.obstruction(x, z)).max(routes.obstruction(x, z)).max(natural.obstruction(x, z))
                    .max(highways.obstruction(x, z))
                    .max(lights.at(x, z).map_or(0.0, |site| site.clearance(x, z)))
            });
        }
        let view = camera(flight.view);
        cliffs.follow(&world, seed, view.x, view.z, 1);
        if frame % 2 == 0 {
            district.service(view, LandBudget {
                expansions: 8, terrain_samples: 32, settlement_jobs: u8::from(frame % 16 == 0),
            });
        }
        if frame % 16 == 0 {
            facilities.service(&district, view);
        }
        rivers.follow(&district, view);
        if frame % 16 == 4 {
            if let Some((request, survey)) = river_survey.take() {
                let key = request.key();
                if let Some(land) = district.planned(key) {
                    let result = survey.certify_land(land, facilities.planned(key));
                    rivers.commit_avoiding(result, &district, view, |river| {
                        facilities.reserves_roadside_river(river)
                            || regional_rail.conflicts_with(|bounds| river.intersects_scenery(bounds))
                            || natural.reserves_river(river) || highways.reserves_river(river)
                            || routes.routes().any(|(_, route)| {
                                route.ground_conflicts_with(|bounds| river.intersects_scenery(bounds))
                            })
                    });
                } else {
                    rivers.cancel(request);
                }
            } else if let Some(request) = rivers.request(&district, &facilities, view) {
                river_survey = Some((request, request.survey(&mut river_work)));
            }
        }
        routes.service(&district, view, frame % 16 == 12, |key, route| {
            if route.ground_conflicts_with(|bounds| regional_rail.reserves(bounds)) {
                return scenic_flight::local_routes::Reservation::Blocked;
            }
            route_reservation(&world, &facilities, &rivers, key, route)
        });
        if frame % 16 == 8 {
            highways.service_with_reservations(&district, view, true, |pair, link| {
                if regional_rail.conflicts_with(|bounds| link.intersects_bounds(bounds)) {
                    return scenic_flight::highway_stream::Reservation::Blocked;
                }
                highway_reservation_with_nature(&district, &facilities, &rivers, &routes, &natural, pair, link)
            });
            if let Some(request) = highways.request(&district, view, true) {
                highways.commit(request.prepare_with_land(&mut coast_work, &district, || {}), &district, view);
            }
        }
        if frame % 4 == 0 {
            regional_rail.service(&district, view, |bounds| {
                facilities.reserves_roadside_site(bounds)
                    || scene_blocks(&district, &rivers, &routes, &natural, &highways, bounds)
            });
        }
        if frame % 16 == 14 {
            if let Some(request) = regional_rail.request(&district, &facilities, &rivers, view) {
                let [a, b] = request.pair.regions();
                let plans = (district.planned(a), district.planned(b),
                    facilities.planned(a), facilities.planned(b));
                if let (Some(Some(first)), Some(Some(second)), Some(source), Some(destination)) = plans {
                    let loaded = rail_data.load([first, second], [source, destination], &highways, request.pair);
                    if loaded {
                        let line = rail_data.prepare(&world, seed, &mut rail_work, &mut river_work, || {});
                        regional_rail.commit(request, line, &district, view);
                    } else {
                        regional_rail.cancel(request);
                    }
                } else {
                    regional_rail.cancel(request);
                }
            }
        }
        match frame % 4 {
            0 | 1 => natural.follow(view, NaturalBudget {
                alpine_tiles: u8::from(frame % 4 == 0),
                dry_tiles: u8::from(frame % 4 == 1),
            }, |bounds| {
                if regional_rail.reserves(bounds) {
                    scenic_flight::natural_stream::NaturalReservation::Reserved
                } else {
                    scenery_reservation_with_highways(&district, &facilities, &rivers, &routes, &highways, bounds)
                }
            }),
            2 => lights.follow(view.x, view.z, 1, |x, z| Lighthouse::generate(&world, seed, x, z)),
            _ => beaches.follow(view.x, view.z, 1, |x, z| BeachSite::generate(&world, seed, x, z)),
        }
        facilities.publish_roadside_sites(view, false, |bounds| {
            if regional_rail.reserves(bounds) {
                scenic_flight::facility_stream::RoadsideReservation::Reserved
            } else {
                scenic_flight::facility_stream::roadside_reservation(&district, &rivers, &routes, &highways, &natural, bounds)
            }
        });
        if frame < start {
            continue;
        }
        if frame == start {
            ground.reset(&world, seed, view.x, view.z);
            vessels.reset(world, seed, view.x, view.z);
            terrain.reset_for_altitude(world, view.x, view.z, view.altitude);
            horizon.reset(world, view.x, view.z);
        } else {
            ground.follow(&world, view.x, view.z);
            vessels.follow(view.x, view.z);
            terrain.follow(view.x, view.z);
            horizon.follow(view.x, view.z, 16);
        }
        let sky = Sky::at_position(seed, flight.ticks, [view.x, view.altitude, view.z]);
        let conditions = weather.observe(seed, flight.ticks, view, &sky);
        let sky = sky.with_shower(conditions.shower);
        let rain = conditions.distant_rain;
        let mist = valley_mist.observe(&world, seed, flight.ticks, view, &sky, dimensions);
        let has_land = district.has_visible_scenery(view);
        let land_sampler = has_land.then(|| district.paint_sampler());
        let has_facilities = facilities.has_visible_facilities(view);
        let has_river = rivers.has_visible_rivers(view);
        let has_routes = routes.routes().next().is_some();
        let visible_cliff = |form: &CoastalLandform| {
            bounds_in_view(view, form.bounds()) && facilities.retains_landmark(form)
        };
        let has_cliff = cliffs.forms().any(visible_cliff);
        let visible_light = |site: &Lighthouse| {
            bounds_in_view(view, site.bounds()) && facilities.retains_lighthouse(site)
        };
        let light_region = lights.sites().filter(|site| visible_light(site))
            .filter_map(|site| site.screen_bounds(view, dimensions)).reduce(ScreenRect::union);
        let tower_region = district.distant_tower_region(view, dimensions);
        let distant_skyline = skyline.prepare(&district, view, flight.ticks, &sky, dimensions,
            |x, z| horizon.sample(x, z));
        let skyline_region = distant_skyline.bounds();
        let bridge_region = rivers.bridge_region(view, dimensions);
        let waterfall_region = natural.waterfall_region(view, dimensions);
        let route_region = routes.depth_region(view, dimensions);
        let rail_region = facilities.rails(view, 78.0)
            .filter_map(|line| rail_screen_region(line, view, dimensions))
            .reduce(ScreenRect::union);
        let corridor_region = regional_rail.rails(view, scenic_flight::render::view_distance(view.altitude))
            .filter_map(|line| corridor_screen_region(line, view, dimensions)).reduce(ScreenRect::union);
        let trains: [_; 2] = {
            let mut visible = facilities.rails(view, TRAIN_REACH)
                .filter_map(|line| Train::at(line, seed, flight.ticks, view, dimensions))
                .chain(regional_rail.rails(view, TRAIN_REACH)
                    .filter_map(|line| Train::on_corridor(line, seed, flight.ticks, view, dimensions)));
            std::array::from_fn(|_| visible.next())
        };
        let train_region = trains.iter().flatten().map(Train::depth_region).reduce(ScreenRect::union);
        let visible_highways = highways.has_visible_links(view);
        let highway_ribbons: [_; 4] = {
            let mut visible = highways.links().filter(|_| visible_highways)
                .filter_map(|(_, link)| HighwayRibbon::new(link, view, dimensions));
            std::array::from_fn(|_| visible.next())
        };
        let coastal_ribbons: [_; 8] = {
            let mut visible = highways.coastal_links().filter(|_| visible_highways)
                .flat_map(|(_, link)| link.approaches())
                .filter_map(|points| HighwayRibbon::for_approach(points, view, dimensions));
            std::array::from_fn(|_| visible.next())
        };
        let ribbons = || highway_ribbons.iter().chain(coastal_ribbons.iter()).flatten();
        distant_roads.prepare(&district, &highways, view, flight.ticks, dimensions);
        let highway_region = ribbons().map(|ribbon| ribbon.depth_region())
            .chain(distant_roads.bounds())
            .reduce(ScreenRect::union);
        let coastal_bridge_region = highways.crossing_region(view, dimensions);
        let distant_bridges = bridge_silhouettes.prepare(view, flight.ticks, &sky, dimensions,
            rivers.bridges().chain(highways.coastal_links().map(|(_, link)| link.bridge())),
            |x, z| horizon.sample(x, z));
        let clipped_depth = light_region.into_iter().chain(tower_region).chain(skyline_region)
            .chain(bridge_region).chain(route_region).chain(rail_region).chain(corridor_region)
            .chain(train_region).chain(highway_region)
            .chain(coastal_bridge_region)
            .chain(distant_bridges.bounds())
            .chain(waterfall_region)
            .chain(mist.depth_region())
            .reduce(ScreenRect::union);
        let has_mesh = has_cliff || clipped_depth.is_some();
        let depth_region = if has_cliff || rain.is_some() {
            Some(ScreenRect::full(dimensions))
        } else {
            clipped_depth
        };
        let visible_natural = natural.has_visible_features(view);
        let natural_sampler = visible_natural.then(|| natural.sampler());
        let river_sampler = has_river.then(|| rivers.sampler());
        let highway_sampler = visible_highways.then(|| highways.sampler());
        let trees = ground.tree_sampler();
        let regional_sampler = regional_rail.rails(view, scenic_flight::render::view_distance(view.altitude))
            .next().map(|_| regional_rail.sampler());
        let sample = |x, z| {
            let raw = terrain.sample(x, z);
            let base = highway_sampler.as_ref().map_or(raw, |sampler| sampler.overlay(x, z, raw));
            let natural = natural_sampler.as_ref().map_or(base, |sampler| sampler.overlay(x, z, base));
            let natural = lights.at(x, z).filter(|site| facilities.retains_lighthouse(site))
                .map_or(natural, |light| light.overlay(x, z, natural));
            let cover = trees.geometry(x, z, natural);
            let developed = land_sampler.as_ref().map_or(cover, |sampler| sampler.overlay(x, z, cover));
            let water = river_sampler.as_ref().map_or(developed, |sampler| sampler.overlay(x, z, developed));
            let coast = vessels.overlay(x, z, water);
            let facility = if has_facilities { facilities.overlay(x, z, raw, coast) } else { coast };
            let developed = if has_routes { routes.overlay(x, z, facility) } else { facility };
            regional_sampler.as_ref().map_or(developed, |sampler| sampler.overlay(x, z, developed))
        };
        render_terrain_layer_region_with_horizon(
            (&sample, |x, z, geometry| ground.visible_material(x, z, geometry)),
            |x, z| horizon.sample(x, z),
            view, &sky, &mut pixels, &mut work, depth_region.map(|rect| (&mut depth[..], rect)),
            dimensions, |_| {}, |_| None,
        ).map_err(|error| format!("Render failed ({error:?})"))?;
        distant_roads.draw(&mut pixels, &depth, &mut work, dimensions);
        distant_road_frames += usize::from(distant_roads.bounds().is_some());
        distant_bridges.draw(&mut pixels, &depth);
        distant_bridge_frames += usize::from(distant_bridges.bounds().is_some());
        distant_skyline.draw(&mut pixels, &depth);
        skyline_frames += usize::from(skyline_region.is_some());
        if route_region.is_some() && routes.open_portals(view, &mut pixels, &mut depth, dimensions) > 0 {
            portal_frames += 1;
        }
        if has_mesh {
            let mut mesh = WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), view, &sky).unwrap();
            for ribbon in ribbons() {
                ribbon.draw(&mut mesh, &sample);
            }
            if coastal_bridge_region.is_some() {
                highways.draw_crossings_fading(view, dimensions, &mut mesh,
                    |bridge| distant_bridges.detail_opacity(bridge));
            }
            for form in cliffs.forms().filter(|form| visible_cliff(form)) {
                form.draw(&mut mesh);
            }
            for site in lights.sites().filter(|site| visible_light(site)) {
                site.draw(&mut mesh, sky.daylight, flight.ticks);
            }
            if tower_region.is_some() { distant_skyline.draw_legacy(&district, view, &mut mesh); }
            for line in facilities.rails(view, 78.0) {
                draw_rail(&mut mesh, line, view, dimensions);
            }
            for line in regional_rail.rails(view, scenic_flight::render::view_distance(view.altitude)) {
                draw_rail_corridor(&mut mesh, line, view, dimensions);
            }
            for train in trains.iter().flatten() {
                train.draw(&mut mesh, sky.daylight);
            }
            if bridge_region.is_some() {
                rivers.draw_fading(view, &mut mesh, |bridge| distant_bridges.detail_opacity(bridge));
            }
            if waterfall_region.is_some() {
                natural.draw_waterfalls(view, flight.ticks, &mut mesh);
            }
            if route_region.is_some() {
                routes.draw(view, flight.ticks, &mut mesh);
            }
        }
        if let Some(rain) = rain {
            rain.paint(view, (SIDE, SIDE), &mut pixels, &depth);
            rain_frames += 1;
        }
        for beach in beaches.sites() {
            let [left, near, right, far] = beach.bounds();
            let usable = (left - 76.0..=right + 76.0).contains(&view.x)
                && (near - 76.0..=far + 76.0).contains(&view.z)
                && beach.fits_displayed_sand(&sample)
                && !beach.conflicts_with(|x, z| {
                    let natural = world.sample(x, z);
                    let road = highway_sampler.as_ref().map_or(natural, |sampler| sampler.overlay(x, z, natural));
                    let developed = district.overlay(x, z, road);
                    let water = rivers.overlay(x, z, developed);
                    let facility = facilities.overlay(x, z, natural, water);
                    routes.overlay(x, z, facility) != natural
                });
            if usable {
                beach.draw(view, sky.daylight, &mut pixels, SIDE, SIDE, |x, z| sample(x, z).elevation);
            }
        }
        mist.paint(view, dimensions, &mut pixels, &depth);
        district_frames += usize::from(has_land);
        landmark_frames += usize::from(has_cliff);
        facility_frames += usize::from(has_facilities);
        river_frames += usize::from(has_river);
        wetland_frames += usize::from(rivers.wetland_sites(view).next().is_some());
        waterfall_frames += usize::from(waterfall_region.is_some());
        bridge_frames += usize::from(bridge_region.is_some());
        route_frames += usize::from(route_region.is_some());
        natural_frames += usize::from(visible_natural);
        highway_frames += usize::from(visible_highways);
        coastal_bridge_frames += usize::from(coastal_bridge_region.is_some());
        draw_flight_scene(&flight, seed, &sky, &mut pixels, SIDE, SIDE);
        ppm(Path::new(directory).join(format!("{:05}.ppm", frame - start)), &pixels, SIDE)?;
    }
    println!(
        "{} recoveries, {landmark_frames} landmark frames, {district_frames} district frames, \
         {facility_frames} facility frames, {river_frames} river frames, {bridge_frames} bridge frames, \
         {wetland_frames} floodplain frames, {waterfall_frames} cascade frames, \
         {route_frames} local-route frames, {portal_frames} open-portal frames, \
         {natural_frames} natural-landmark frames, {highway_frames} connected-highway frames, \
         {coastal_bridge_frames} coastal-bridge frames, {rain_frames} rain frames, {skyline_frames} distant skyline frames, \
         {distant_road_frames} distant road frames, {distant_bridge_frames} distant bridge frames",
        flight.terrain_recoveries,
    );
    Ok(())
}

fn survey(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    if arguments.is_empty() {
        return Err("Use survey SEED [SEED ...]".into());
    }
    for value in arguments {
        let seed = parse_seed(value)?;
        let world = World::new(seed);
        let mut tiles = [0_u32; 8];
        let mut high = 0.0_f32;
        let mut grade = 0.0_f32;
        let mut coastline = 0;
        for z in -96..=96 {
            for x in -96..=96 {
                let x = x as f32 * 24.0;
                let z = z as f32 * 24.0;
                let sample = world.sample(x, z);
                let kind = match sample.surface {
                    Surface::DeepWater => 0,
                    Surface::Shallows => 1,
                    Surface::Sand => 2,
                    Surface::Grass => 3,
                    Surface::Forest => 4,
                    Surface::Rock => 5,
                    Surface::Snow => 6,
                    Surface::Desert => 7,
                    _ => return Err("The base world returned a scenery material".into()),
                };
                tiles[kind] += 1;
                high = high.max(sample.elevation);
                let east = world.elevation(x + 1.0, z);
                let north = world.elevation(x, z + 1.0);
                grade = grade.max((east - sample.elevation).abs()).max((north - sample.elevation).abs());
                if (sample.elevation == SEA_LEVEL) != (world.elevation(x + 24.0, z) == SEA_LEVEL) {
                    coastline += 1;
                }
            }
        }
        println!("{seed:016x}\tsea {}\tshelf {}\tbeach {}\tgrass {}\tforest {}\trock {}\tsnow {}\tdesert {}\tpeak {high:.2}\tgrade {grade:.2}\tshore {coastline}",
            tiles[0], tiles[1], tiles[2], tiles[3], tiles[4], tiles[5], tiles[6], tiles[7]);
    }
    Ok(())
}

fn soak(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    let [seed, aircraft, seconds] = arguments else {
        return Err("soak needs SEED AIRCRAFT SECONDS".into());
    };
    let seed = parse_seed(seed)?;
    let aircraft = parse_aircraft(aircraft)?;
    let world = World::new(seed);
    let ticks_per_second = u64::from(TICKS_PER_SECOND);
    let ticks = seconds.parse::<u64>()? * ticks_per_second;
    let mut flight = Flight::start(&world, seed, aircraft);
    let mut min_body = f32::INFINITY;
    let mut min_view = f32::INFINITY;
    let mut over_water = 0;
    let mut longest_water = 0;
    let mut max_edge: f32 = 0.0;
    let mut max_turn = 0.0_f32;
    let mut speed = 0.0_f32;
    let mut pass_seconds = 0u32;
    let start = Instant::now();
    for _ in 0..ticks {
        flight.update(&world);
        if flight.ticks.is_multiple_of(ticks_per_second) {
            let ground = world.elevation(flight.x, flight.z);
            let view_ground = world.elevation(flight.view.x, flight.view.z);
            min_body = min_body.min(flight.altitude - ground);
            min_view = min_view.min(flight.view.altitude - view_ground);
            if ground <= SEA_LEVEL {
                over_water += 1;
                longest_water = longest_water.max(over_water);
            } else {
                over_water = 0;
            }
            max_edge = max_edge.max(flight.x.abs()).max(flight.z.abs());
            max_turn = max_turn.max(flight.turn_rate().abs());
            speed += flight.speed();
            pass_seconds += u32::from(flight.flying_coastal_pass());
        }
    }
    println!(
        "seed {seed:016x}\t{}\t{} ticks\tbody {:.2}\tcamera {:.2}\trecoveries {}\tlongest ocean {}s\tmax extent {:.0}\tmean speed {:.2}\tpeak turn {:.3}\tcoast pass {}s\t{:?}",
        aircraft.name(), ticks, min_body, min_view, flight.terrain_recoveries, longest_water, max_edge,
        speed / (ticks / ticks_per_second).max(1) as f32, max_turn, pass_seconds, start.elapsed(),
    );
    if min_body <= 0.0 || min_view <= 0.0 || !flight.altitude.is_finite() {
        return Err("Flight ground clearance failed".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((verb, args)) if verb == "render" => render(args),
        Some((verb, args)) if verb == "replay" => replay(args),
        Some((verb, args)) if verb == "soak" => soak(args),
        Some((verb, args)) if verb == "district" => landmark(args, true),
        Some((verb, args)) if verb == "harbor" => landmark(args, false),
        Some((verb, args)) if verb == "port" => port(args),
        Some((verb, args)) if verb == "river" => river(args),
        Some((verb, args)) if verb == "connected-river" => connected_river(args),
        Some((verb, args)) if verb == "survey" => survey(args),
        Some((verb, args)) if verb == "airfield" => airfield(args),
        Some((verb, args)) if verb == "coastal-landmark" => coastal_landmark(args),
        Some((verb, args)) if verb == "basin" => basin(args),
        _ => Err("Commands available are render, replay, soak, survey, district, harbor, port, river, connected-river, airfield, basin, and coastal-landmark".into()),
    }
}
