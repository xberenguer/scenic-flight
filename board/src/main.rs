#![no_std]
#![no_main]

#[cfg(feature = "native-flight-overlay")]
mod native_flight;
#[cfg(feature = "native-flight-overlay")]
mod medium_aircraft;
#[cfg(feature = "native-flight-overlay")]
mod native_ui;
#[cfg(feature = "native-airframe-probe")]
mod packed_airframe;
mod bands;
#[cfg(feature = "scenery")]
mod details;
mod display;
#[cfg(feature = "parity-fixtures")]
mod fixtures;
#[cfg(feature = "parity-fixtures")]
mod fixtures_device;
mod panel;
mod scene_raster;
#[cfg(feature = "panel-worker")]
mod button_capture;
#[cfg(feature = "transport-stream")]
mod scene_memory;
#[cfg(feature = "transport-stream")]
mod surveys;
#[cfg(feature = "urban-background")]
mod urban_survey;
#[cfg(feature = "scene-probe")]
mod probe;
#[cfg(feature = "sample-probe")]
mod sample_probe;
#[cfg(feature = "shore-details")]
mod shore;

#[cfg(feature = "cooperative-sky")]
mod sky_worker;

use display::Display;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    spi::{
        self,
        master::{Config as SpiConfig, Spi},
    },
    time::{Instant, Rate},
};
use scenic_flight::{
    aircraft_art::{draw_flight_scene, draw_preview},
    controls::{Buttons, Screen},
    flight::{Game, TICKS_PER_SECOND, View},
    render::Camera,
    sky::Sky,
    terrain_cache::TerrainCache,
    ui::{draw_flight_info, draw_new_world, draw_picker},
};
use static_cell::{ConstStaticCell, StaticCell};
#[cfg(feature = "river-stream")]
use scenic_flight::{river_stream::RiverStream, watershed::WatershedWork};
#[cfg(feature = "local-routes")]
use scenic_flight::local_routes::{LocalRouteStream, route_reservation};
#[cfg(feature = "natural-scenery")]
use scenic_flight::natural_stream::{NaturalBounds, NaturalBudget, NaturalLayer, NaturalReservation, NaturalStream};
#[cfg(feature = "natural-scenery")]
use scenic_flight::summit_stream::SummitStream;
#[cfg(all(feature = "natural-scenery", not(feature = "highway-stream")))]
use scenic_flight::natural_stream::scenery_reservation;
#[cfg(feature = "highway-stream")]
use scenic_flight::{highway_stream::HighwayStream, natural_stream::{highway_reservation_with_nature, scenery_reservation_with_highways}};
#[cfg(feature = "coastal-highways")]
use scenic_flight::coastal_highways::CoastalHighwayWork;
#[cfg(feature = "regional-rail")]
use scenic_flight::{rail_stream::{RailStream, scene_blocks}, rail_survey::RailSurveyWork};
#[cfg(feature = "transport-stream")]
use surveys::{Survey, SurveyResult};

const LOGICAL_SIDE: usize = render_side();
const ADAPTIVE_SCENERY: bool = LOGICAL_SIDE == scene_raster::FULL_SIDE
    && cfg!(feature = "native-flight-overlay")
    && (!cfg!(feature = "scene-probe") || option_env!("SCENIC_PROFILE_ADAPTIVE_RASTER").is_some());
const FRAME_PIXELS: usize = LOGICAL_SIDE * LOGICAL_SIDE;
const INTERNAL_COLOR_PIXELS: usize =
    if cfg!(feature = "transport-stream") && LOGICAL_SIDE > 144 { 0 } else { FRAME_PIXELS };
#[cfg(feature = "buffered-terrain")]
const INTERNAL_TERRAIN_PIXELS: usize =
    if cfg!(feature = "transport-stream") && LOGICAL_SIDE > 112 {
        0
    } else if cfg!(feature = "packed-60-terrain") && LOGICAL_SIDE == 60 {
        2 * FRAME_PIXELS
    } else {
        FRAME_PIXELS
    };
#[cfg(feature = "transport-stream")]
const LOCAL_RAIL_REACH: f32 = 112.0 * 0.69;
const PERIOD_US: u64 = 1_000_000 / TICKS_PER_SECOND as u64;
// A native frame can span several ticks. Bound catch-up after a longer stall.
const MAX_SIMULATION_STEPS: u64 = if LOGICAL_SIDE == bands::SIDE { 24 } else { 4 };
static IMAGE: ConstStaticCell<[u16; INTERNAL_COLOR_PIXELS]> = ConstStaticCell::new([0; INTERNAL_COLOR_PIXELS]);
#[cfg(feature = "panel-worker")]
static SPARE_IMAGE: ConstStaticCell<[u16; INTERNAL_COLOR_PIXELS]> = ConstStaticCell::new([0; INTERNAL_COLOR_PIXELS]);
static TERRAIN: ConstStaticCell<TerrainCache> = ConstStaticCell::new(TerrainCache::new());
#[cfg(feature = "far-horizon")]
static HORIZON: ConstStaticCell<scenic_flight::horizon_cache::HorizonCache> =
    ConstStaticCell::new(scenic_flight::horizon_cache::HorizonCache::new());
#[cfg(feature = "scenery")]
static DETAILS: ConstStaticCell<details::DetailLayers> = ConstStaticCell::new(details::DetailLayers::new());
#[cfg(feature = "streamed-land")]
static LAND: ConstStaticCell<scenic_flight::land_stream::LandStream> =
    ConstStaticCell::new(scenic_flight::land_stream::LandStream::new());
#[cfg(feature = "coastal-mesh")]
static COASTAL_MESH: StaticCell<scenic_flight::coastal_landforms::CoastalLandmarks> = StaticCell::new();
#[cfg(all(feature = "coastal-mesh", not(feature = "river-stream")))]
static SCENERY_DEPTH: StaticCell<[u16; LOGICAL_SIDE * LOGICAL_SIDE]> = StaticCell::new();
#[cfg(feature = "shore-details")]
static SHORE: static_cell::ConstStaticCell<shore::Shore> = static_cell::ConstStaticCell::new(shore::Shore::new());
#[cfg(feature = "buffered-terrain")]
static TERRAIN_WORK: ConstStaticCell<[u16; INTERNAL_TERRAIN_PIXELS]> = ConstStaticCell::new([0; INTERNAL_TERRAIN_PIXELS]);

const fn render_side() -> usize {
    let Some(value) = option_env!("SCENIC_RENDER_SIDE") else {
        return if cfg!(feature = "logical-112") { 112 } else { 128 };
    };
    let digits = value.as_bytes();
    assert!(!digits.is_empty() && digits.len() <= 3, "invalid SCENIC_RENDER_SIDE");
    let mut side = 0;
    let mut index = 0;
    while index < digits.len() {
        let digit = digits[index];
        assert!(digit >= b'0' && digit <= b'9', "invalid SCENIC_RENDER_SIDE");
        side = side * 10 + (digit - b'0') as usize;
        index += 1;
    }
    assert!(side >= 48 && side <= bands::SIDE, "SCENIC_RENDER_SIDE is outside the panel range");
    side
}

esp_bootloader_esp_idf::esp_app_desc!();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    esp_println::println!("Scenic Flight panic: {}", info);
    loop {
        core::hint::spin_loop();
    }
}

fn micros() -> u64 {
    Instant::now().duration_since_epoch().as_micros()
}

// Keep the temporary initialization storage out of the persistent frame loop's stack frame.
#[cfg(all(feature = "coastal-mesh", not(feature = "river-stream")))]
#[inline(never)]
fn image_buffer(cell: &'static StaticCell<[u16; LOGICAL_SIDE * LOGICAL_SIDE]>, fill: u16)
    -> &'static mut [u16; LOGICAL_SIDE * LOGICAL_SIDE]
{
    cell.init_with(|| [fill; LOGICAL_SIDE * LOGICAL_SIDE])
}

fn color_frame(
    cell: &'static ConstStaticCell<[u16; INTERNAL_COLOR_PIXELS]>,
    #[cfg(feature = "transport-stream")] memory: &mut scene_memory::SceneMemory,
) -> &'static mut [u16; FRAME_PIXELS] {
    #[cfg(feature = "transport-stream")]
    let pixels: &mut [u16] = if INTERNAL_COLOR_PIXELS == 0 {
        memory.place_pixels(FRAME_PIXELS)
    } else {
        cell.take()
    };
    #[cfg(not(feature = "transport-stream"))]
    let pixels: &mut [u16] = cell.take();
    pixels.try_into().ok().expect("color frame size")
}

#[cfg(feature = "native-flight-overlay")]
#[inline(never)]
fn native_flight_buffers(
    memory: &mut scene_memory::SceneMemory,
) -> [&'static mut native_flight::NativeFlight<'static>; 2] {
    core::array::from_fn(|_| {
        let pixels = memory.place_pixels(native_flight::STORAGE_PIXELS);
        let ui = memory.place_pixels(native_ui::STORAGE_WORDS);
        memory.place(|| native_flight::NativeFlight::new(pixels).with_ui(ui))
    })
}

#[cfg(feature = "native-airframe-probe")]
#[inline(never)]
fn native_capture_buffers(
    memory: &mut scene_memory::SceneMemory,
) -> [Option<&'static mut packed_airframe::PackedCapture<'static>>; 2] {
    let enabled = probe::native_capture_frame().is_some();
    core::array::from_fn(|_| enabled.then(|| {
        let pixels = memory.place_pixels(packed_airframe::CAPTURE_PIXELS);
        memory.place(|| packed_airframe::PackedCapture::new(pixels))
    }))
}

#[cfg(feature = "transport-stream")]
#[inline(never)]
fn facility_buffer(memory: &mut scene_memory::SceneMemory)
    -> &'static mut scenic_flight::facility_stream::FacilityStream<8>
{
    memory.place(|| scenic_flight::facility_stream::FacilityStream::new(
        scenic_flight::world::World::new(0), 0,
    ))
}

#[cfg(feature = "transport-stream")]
#[inline(never)]
fn survey_buffer(memory: &mut scene_memory::SceneMemory) -> &'static mut surveys::SurveySlots {
    memory.place(surveys::SurveySlots::new)
}

#[cfg(feature = "river-stream")]
#[inline(never)]
fn depth_buffer(memory: &mut scene_memory::SceneMemory) -> &'static mut [u16; LOGICAL_SIDE * LOGICAL_SIDE] {
    let pixels = memory.place_pixels(FRAME_PIXELS);
    pixels.fill(u16::MAX);
    pixels.try_into().ok().expect("depth frame size")
}

#[cfg(feature = "river-stream")]
#[inline(never)]
fn river_buffers(memory: &mut scene_memory::SceneMemory)
    -> (&'static mut RiverStream<3>, &'static mut WatershedWork, &'static mut WatershedWork)
{
    let rivers = memory.place(|| RiverStream::new(scenic_flight::world::World::new(0), 0));
    (rivers, memory.place(WatershedWork::new), memory.place(WatershedWork::new))
}

#[cfg(feature = "local-routes")]
#[inline(never)]
fn route_buffer(memory: &mut scene_memory::SceneMemory) -> &'static mut LocalRouteStream<8> {
    memory.place(|| LocalRouteStream::new(scenic_flight::world::World::new(0), 0))
}

#[cfg(feature = "natural-scenery")]
#[inline(never)]
fn natural_buffer(memory: &mut scene_memory::SceneMemory) -> &'static mut NaturalStream {
    memory.place(|| NaturalStream::new(scenic_flight::world::World::new(0), 0))
}

#[cfg(feature = "natural-scenery")]
#[inline(never)]
fn summit_buffer(memory: &mut scene_memory::SceneMemory) -> &'static mut SummitStream {
    memory.place(SummitStream::new)
}

#[cfg(feature = "highway-stream")]
#[inline(never)]
fn highway_buffer(memory: &mut scene_memory::SceneMemory) -> &'static mut HighwayStream<4> {
    memory.place(|| HighwayStream::new(scenic_flight::world::World::new(0), 0))
}

#[cfg(feature = "distant-roads")]
#[inline(never)]
fn road_buffer(memory: &mut scene_memory::SceneMemory) -> &'static mut scenic_flight::distant_roads::DistantRoads {
    memory.place(scenic_flight::distant_roads::DistantRoads::new)
}

#[cfg(feature = "coastal-highways")]
#[inline(never)]
fn coastal_work(memory: &mut scene_memory::SceneMemory) -> &'static mut CoastalHighwayWork {
    memory.place(CoastalHighwayWork::new)
}

#[cfg(feature = "regional-rail")]
#[inline(never)]
fn rail_stream_buffer(memory: &mut scene_memory::SceneMemory) -> &'static mut RailStream {
    memory.place(|| RailStream::new(scenic_flight::world::World::new(0), 0))
}

#[cfg(feature = "regional-rail")]
#[inline(never)]
fn rail_work_buffer(memory: &mut scene_memory::SceneMemory) -> &'static mut RailSurveyWork {
    static INITIAL: RailSurveyWork = RailSurveyWork::new();
    memory.copy_from(&INITIAL)
}

#[cfg(feature = "natural-scenery")]
fn reserve_natural(
    land: &scenic_flight::land_stream::LandStream,
    facilities: &scenic_flight::facility_stream::FacilityStream<8>,
    rivers: &RiverStream<3>,
    routes: &LocalRouteStream<8>,
    #[cfg(feature = "highway-stream")] highways: &HighwayStream<4>,
    #[cfg(feature = "regional-rail")] rail: &RailStream,
    bounds: NaturalBounds,
) -> NaturalReservation {
    #[cfg(feature = "regional-rail")]
    if rail.reserves(bounds) {
        return NaturalReservation::Reserved;
    }
    #[cfg(feature = "highway-stream")]
    return scenery_reservation_with_highways(land, facilities, rivers, routes, highways, bounds);
    #[cfg(not(feature = "highway-stream"))]
    return scenery_reservation(land, facilities, rivers, routes, bounds);
}

#[cfg(feature = "streamed-land")]
#[inline(never)]
fn land_stream() -> &'static mut scenic_flight::land_stream::LandStream {
    LAND.take()
}

#[cfg(feature = "transport-stream")]
#[inline(never)]
fn update_surveys(
    display: &mut Display,
    facilities: &mut scenic_flight::facility_stream::FacilityStream<8>,
    land: &mut scenic_flight::land_stream::LandStream,
    camera: Camera,
    #[cfg(feature = "urban-background")] mut urban: urban_survey::Context<'_>,
    #[cfg(feature = "river-stream")] rivers: &mut RiverStream<3>,
    #[cfg(feature = "local-routes")] routes: &mut LocalRouteStream<8>,
    #[cfg(feature = "local-routes")] world: &scenic_flight::world::World,
    #[cfg(feature = "natural-scenery")] natural: &mut NaturalStream,
    #[cfg(feature = "natural-scenery")] summits: &mut SummitStream,
    #[cfg(feature = "natural-scenery")] summit_scan_due: bool,
    #[cfg(feature = "natural-scenery")] natural_layer: NaturalLayer,
    #[cfg(feature = "highway-stream")] highways: &mut HighwayStream<4>,
    #[cfg(feature = "regional-rail")] rail: &mut RailStream,
) {
    if let Some(result) = display.collect_survey() {
        match result {
            #[cfg(feature = "urban-background")]
            SurveyResult::Urban(result) => urban.receive(result),
            #[cfg(feature = "urban-background")]
            SurveyResult::Settlement(result) => {
                let accepted = land.commit_settlement(result);
                #[cfg(feature = "scene-probe")]
                esp_println::println!("town worker frame={} accepted={accepted}", urban.frame);
                #[cfg(not(feature = "scene-probe"))]
                let _ = accepted;
            }
            SurveyResult::Facility(result) => { facilities.commit(result, land, camera); }
            #[cfg(feature = "river-stream")]
            SurveyResult::River(result) => {
                rivers.commit_avoiding(result, land, camera, |river| {
                    if facilities.reserves_roadside_river(river) {
                        return true;
                    }
                    #[cfg(feature = "regional-rail")]
                    if rail.conflicts_with(|bounds| river.intersects_scenery(bounds)) {
                        return true;
                    }
                    #[cfg(feature = "natural-scenery")]
                    if natural.reserves_river(river)
                        || summits.conflicts_with(|bounds| river.intersects_scenery(bounds))
                    {
                        return true;
                    }
                    #[cfg(feature = "highway-stream")]
                    if highways.reserves_river(river) {
                        return true;
                    }
                    #[cfg(feature = "local-routes")]
                    return routes.routes().any(|(_, route)| {
                        route.ground_conflicts_with(|bounds| river.intersects_scenery(bounds))
                    });
                    #[cfg(not(feature = "local-routes"))]
                    { let _ = river; false }
                });
            }
            #[cfg(feature = "local-routes")]
            SurveyResult::Route(result) => { routes.commit(result, land, camera); }
            #[cfg(feature = "natural-scenery")]
            SurveyResult::Natural(result) => {
                natural.commit(result, camera, |bounds| {
                    if summits.reserves(bounds) { return NaturalReservation::Reserved; }
                    reserve_natural(land, facilities, rivers, routes,
                        #[cfg(feature = "highway-stream")] highways,
                        #[cfg(feature = "regional-rail")] rail,
                        bounds)
                });
            }
            #[cfg(feature = "natural-scenery")]
            SurveyResult::Summit(result) => { summits.commit(result, camera); }
            #[cfg(feature = "highway-stream")]
            SurveyResult::Highway(result) => { highways.commit(result, land, camera); }
            #[cfg(feature = "regional-rail")]
            SurveyResult::RegionalRail { request, corridor } => {
                rail.commit(request, corridor, land, camera);
            }
        }
    }
    #[cfg(feature = "urban-background")]
    if display.survey_ready() {
        if let Some(request) = land.request_settlement() {
            display.survey(Survey::Settlement(request));
        }
    }
    #[cfg(feature = "urban-background")]
    urban.service(display, land, world, camera,
        &scenic_flight::land_stream::UrbanAuthorities {
            facilities, rivers, routes, highways, rail, natural, summits: Some(summits),
        });
    #[cfg(feature = "natural-scenery")]
    if summit_scan_due && display.survey_ready() {
        if let Some(request) = summits.request(*world, camera) {
            display.survey(Survey::Summit(request));
        }
    }
    if let Some(request) = facilities.request(land, camera, display.survey_ready()) {
        display.survey(Survey::Facility(request));
    }
    #[cfg(feature = "river-stream")]
    {
        rivers.follow(land, camera);
        if display.survey_ready() {
            survey_river(display, facilities, land, camera, rivers);
        }
    }
    #[cfg(feature = "local-routes")]
    {
        routes.service_with_reservations(land, camera, true, |key, route| {
            #[cfg(feature = "natural-scenery")]
            if summits.conflicts_with(|[left, near, right, far]| {
                route.ground_conflicts_with(|[a, b, c, d]| left <= c && right >= a && near <= d && far >= b)
            }) {
                return scenic_flight::local_routes::Reservation::Blocked;
            }
            #[cfg(feature = "regional-rail")]
            if route.ground_conflicts_with(|bounds| rail.reserves(bounds)) {
                return scenic_flight::local_routes::Reservation::Blocked;
            }
            route_reservation(world, facilities, rivers, key, route)
        });
        if let Some(request) = routes.request(land, camera, display.survey_ready()) {
            display.survey(Survey::Route(request));
        }
    }
    #[cfg(feature = "natural-scenery")]
    {
        #[cfg(feature = "highway-stream")]
        {
            highways.service_with_reservations(land, camera, true, |pair, link| {
                if summits.conflicts_with(|bounds| link.intersects_bounds(bounds)) {
                    return scenic_flight::highway_stream::Reservation::Blocked;
                }
                #[cfg(feature = "regional-rail")]
                if rail.conflicts_with(|bounds| link.intersects_bounds(bounds)) {
                    return scenic_flight::highway_stream::Reservation::Blocked;
                }
                highway_reservation_with_nature(land, facilities, rivers, routes, natural, pair, link)
            });
            if let Some(request) = highways.request(land, camera, display.survey_ready()) {
                display.survey(Survey::Highway {
                    request,
                    #[cfg(feature = "coastal-highways")]
                    towns: request.copy_towns(land),
                });
            }
        }
        #[cfg(feature = "regional-rail")]
        {
            rail.service(land, camera, |bounds| {
                summits.reserves(bounds) || facilities.reserves_roadside_site(bounds)
                    || scene_blocks(land, rivers, routes, natural, highways, bounds)
            });
            if display.survey_ready() {
                survey_rail(display, rail, land, facilities, rivers, highways, camera);
            }
        }
        natural.follow(camera, NaturalBudget::default(), |bounds| {
            if summits.reserves(bounds) { return NaturalReservation::Reserved; }
            reserve_natural(land, facilities, rivers, routes,
                #[cfg(feature = "highway-stream")] highways,
                #[cfg(feature = "regional-rail")] rail,
                bounds)
        });
        summits.publish(camera, (LOGICAL_SIDE, LOGICAL_SIDE), |bounds| {
            if natural.reserves(bounds) { return NaturalReservation::Reserved; }
            reserve_natural(land, facilities, rivers, routes,
                #[cfg(feature = "highway-stream")] highways,
                #[cfg(feature = "regional-rail")] rail,
                bounds)
        });
        if display.survey_ready() {
            if let Some(request) = natural.request(camera, natural_layer) {
                display.survey(Survey::Natural(request));
            }
        }
        #[cfg(feature = "highway-stream")]
        facilities.publish_roadside_sites(camera, false, |bounds| {
            if summits.reserves(bounds) {
                return scenic_flight::facility_stream::RoadsideReservation::Reserved;
            }
            #[cfg(feature = "regional-rail")]
            if rail.reserves(bounds) {
                return scenic_flight::facility_stream::RoadsideReservation::Reserved;
            }
            scenic_flight::facility_stream::roadside_reservation(land, rivers, routes, highways, natural, bounds)
        });
        #[cfg(feature = "regional-rail")]
        rail.connect_quarries(land, facilities, camera);
    }
}

#[cfg(feature = "regional-rail")]
#[inline(never)]
fn survey_rail(
    display: &mut Display,
    rail: &mut RailStream,
    land: &scenic_flight::land_stream::LandStream,
    facilities: &scenic_flight::facility_stream::FacilityStream<8>,
    rivers: &RiverStream<3>,
    highways: &HighwayStream<4>,
    camera: Camera,
) {
    let Some(request) = rail.request(land, facilities, rivers, camera) else { return; };
    let [a, b] = request.pair.regions();
    let (Some(Some(first)), Some(Some(second))) = (land.planned(a), land.planned(b)) else {
        rail.cancel(request);
        return;
    };
    let (Some(first_facilities), Some(second_facilities)) = (facilities.planned(a), facilities.planned(b)) else {
        rail.cancel(request);
        return;
    };
    if !display.survey_rail(request, |data| data.load(
        [first, second], [first_facilities, second_facilities], highways, request.pair,
    )) {
        rail.cancel(request);
    }
}

#[cfg(feature = "river-stream")]
#[inline(never)]
fn survey_river(
    display: &mut Display,
    facilities: &scenic_flight::facility_stream::FacilityStream<8>,
    land: &scenic_flight::land_stream::LandStream,
    camera: Camera,
    rivers: &mut RiverStream<3>,
) {
    let Some(request) = rivers.request(land, facilities, camera) else { return; };
    let key = request.key();
    if let Some(district) = land.planned(key) {
        display.survey(Survey::River {
            request,
            district: district.cloned(),
            facilities: facilities.planned(key).cloned(),
        });
    } else {
        rivers.cancel(request);
    }
}

fn frame_fingerprint(image: &[u16]) -> u32 {
    image.iter().fold(0x811c_9dc5_u32, |hash, pixel| {
        (hash ^ u32::from(*pixel)).wrapping_mul(0x0100_0193)
    })
}

fn flight_camera(view: View) -> Camera {
    Camera {
        x: view.x,
        altitude: view.altitude,
        z: view.z,
        heading: view.heading,
        pitch: view.pitch,
        bank: view.bank,
    }
}

#[cfg(feature = "transport-stream")]
fn observe_nearby_train(
    game: &mut Game,
    facilities: &scenic_flight::facility_stream::FacilityStream<8>,
    #[cfg(feature = "regional-rail")] regional_rail: &RailStream,
) {
    use scenic_flight::train_art::Train;
    let Some((_, flight)) = game.active() else { return };
    if !flight.should_observe_trains() {
        return;
    }
    let seed = game.world_seed().unwrap();
    let camera = flight_camera(flight.view);
    let motions = facilities.rails(camera, 112.0)
        .filter_map(|line| Train::line_motion(line, seed, flight.ticks));
    #[cfg(feature = "regional-rail")]
    let motions = motions.chain(regional_rail.rails(camera, 112.0)
        .filter_map(|line| Train::corridor_motion(line, seed, flight.ticks)));
    let best = motions.filter_map(|train| flight.train_score(train).map(|score| (score, train)))
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, train)| train);
    game.observe_train(best);
}

struct Controls {
    pins: Option<(Input<'static>, Input<'static>)>,
    #[cfg(feature = "panel-worker")]
    last_capture: Buttons,
    #[cfg(feature = "panel-worker")]
    await_capture_release: bool,
    next_read: u64,
    #[cfg(feature = "scene-probe")]
    probe_since: u64,
    #[cfg(feature = "lineup-probe")]
    lineup: probe::Lineup,
}

impl Controls {
    fn apply(&self, game: &mut Game, at: u32, buttons: Buttons) {
        if let Some(command) = game.read_buttons(at, buttons) {
            esp_println::println!("button id={} ms={}", command as u8, at);
        }
    }

    #[cfg(feature = "panel-worker")]
    #[inline(never)]
    fn sample_captured(&mut self, game: &mut Game) {
        let mut edges = [button_capture::Edge::UP; button_capture::MAX_EDGES];
        let (at, count, current, overflow) = button_capture::take(&mut edges);
        if overflow {
            game.reset_buttons();
            self.await_capture_release = true;
        }
        if self.await_capture_release {
            if current == Buttons::default() {
                self.last_capture = current;
                self.await_capture_release = false;
            }
            return;
        }
        for edge in &edges[..count] {
            // Replay the stable interval before the edge as well as the new level.
            self.apply(game, edge.at, self.last_capture);
            self.last_capture = edge.buttons;
            self.apply(game, edge.at, edge.buttons);
        }
        self.last_capture = current;
        self.apply(game, at, current);
    }

    fn sample(&mut self, game: &mut Game) {
        let now = micros();
        if now >= self.next_read {
            self.next_read = now + 2_000;
            #[cfg(feature = "panel-worker")]
            if self.pins.is_none() {
                self.sample_captured(game);
                return;
            }
            let (small, large) = self.pins.as_ref().unwrap();
            let buttons = Buttons {
                small: small.is_low(),
                large: large.is_high(),
            };
            #[cfg(all(feature = "scene-probe", not(feature = "lineup-probe")))]
            let buttons = probe::input(buttons, (now - self.probe_since) / 1_000);
            #[cfg(feature = "lineup-probe")]
            let buttons = self.lineup.input(buttons, game, now / 1_000);
            self.apply(game, (now / 1_000) as u32, buttons);
        }
    }
}

#[esp_hal::main]
fn main() -> ! {
    #[cfg(all(feature = "transport-stream", feature = "scene-probe"))]
    esp_println::println!("transport boot enter");
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    #[cfg(all(feature = "transport-stream", feature = "scene-probe"))]
    esp_println::println!("transport boot hal");
    #[cfg(feature = "transport-stream")]
    let mut scene_memory = scene_memory::SceneMemory::new(peripherals.PSRAM);
    #[cfg(all(feature = "transport-stream", feature = "cooperative-sky"))]
    sky_worker::initialize(&mut scene_memory);
    #[cfg(all(feature = "transport-stream", feature = "scene-probe"))]
    esp_println::println!("transport boot psram");
    #[cfg(feature = "transport-stream")]
    let facilities = facility_buffer(&mut scene_memory);
    #[cfg(feature = "transport-stream")]
    let survey_slots = survey_buffer(&mut scene_memory);
    #[cfg(feature = "river-stream")]
    let scenery_depth = depth_buffer(&mut scene_memory);
    #[cfg(feature = "river-stream")]
    let (rivers, launch_water, worker_water) = river_buffers(&mut scene_memory);
    #[cfg(feature = "local-routes")]
    let routes = route_buffer(&mut scene_memory);
    #[cfg(feature = "natural-scenery")]
    let natural = natural_buffer(&mut scene_memory);
    #[cfg(feature = "natural-scenery")]
    let summits = summit_buffer(&mut scene_memory);
    #[cfg(feature = "highway-stream")]
    let highways = highway_buffer(&mut scene_memory);
    #[cfg(feature = "distant-roads")]
    let distant_roads = road_buffer(&mut scene_memory);
    #[cfg(feature = "coastal-highways")]
    let worker_coast = coastal_work(&mut scene_memory);
    #[cfg(feature = "regional-rail")]
    let regional_rail = rail_stream_buffer(&mut scene_memory);
    #[cfg(feature = "regional-rail")]
    let rail_work = rail_work_buffer(&mut scene_memory);
    #[cfg(feature = "coastal-fixture")]
    let launch_coast = coastal_work(&mut scene_memory);
    #[cfg(feature = "burst-capture")]
    let mut burst = probe::Burst::new(&mut scene_memory);
    #[cfg(all(feature = "transport-stream", feature = "scene-probe"))]
    esp_println::println!("transport boot facility bytes={}", core::mem::size_of_val(facilities));
    let pins = (
        Input::new(
            peripherals.GPIO0,
            InputConfig::default().with_pull(Pull::Up),
        ),
        Input::new(peripherals.GPIO3, InputConfig::default()),
    );
    let mut controls = Controls {
        pins: Some(pins),
        #[cfg(feature = "panel-worker")]
        last_capture: Buttons::default(),
        #[cfg(feature = "panel-worker")]
        await_capture_release: false,
        next_read: 0,
        #[cfg(feature = "scene-probe")]
        probe_since: 0,
        #[cfg(feature = "lineup-probe")]
        lineup: probe::Lineup::default(),
    };
    let mut reset = Output::new(peripherals.GPIO1, Level::High, OutputConfig::default());
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(spi::Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO38)
    .with_sio0(peripherals.GPIO4)
    .with_sio1(peripherals.GPIO5)
    .with_sio2(peripherals.GPIO6)
    .with_sio3(peripherals.GPIO7)
    .with_cs(peripherals.GPIO12)
    .with_dma(peripherals.DMA_CH0)
    .with_buffers(
        esp_hal::dma_rx_buffer!(4).unwrap(),
        esp_hal::dma_tx_buffer!(64).unwrap(),
    );
    let panel = panel::Panel::<LOGICAL_SIDE>::new(spi, panel::pixel_buffers(), &mut reset).unwrap();
    #[cfg(all(feature = "transport-stream", feature = "scene-probe"))]
    esp_println::println!("transport boot panel");
    #[cfg(not(feature = "panel-worker"))]
    let mut display = Display::new(panel);
    #[cfg(feature = "panel-worker")]
    let mut display = Display::start(
        panel,
        &mut esp_hal::system::CpuControl::new(peripherals.CPU_CTRL),
        color_frame(&SPARE_IMAGE, #[cfg(feature = "transport-stream")] &mut scene_memory),
        #[cfg(feature = "native-flight-overlay")]
        native_flight_buffers(&mut scene_memory),
        #[cfg(feature = "native-airframe-probe")]
        native_capture_buffers(&mut scene_memory),
        #[cfg(feature = "transport-stream")]
        survey_slots,
        #[cfg(feature = "river-stream")]
        worker_water,
        #[cfg(feature = "coastal-highways")]
        worker_coast,
        #[cfg(feature = "regional-rail")]
        rail_work,
    );
    let mut image = color_frame(&IMAGE, #[cfg(feature = "transport-stream")] &mut scene_memory);
    #[cfg(all(feature = "transport-stream", feature = "scene-probe"))]
    esp_println::println!("transport boot worker");
    let terrain = TERRAIN.take();
    #[cfg(feature = "far-horizon")]
    let horizon = HORIZON.take();
    #[cfg(feature = "scenery")]
    let details = DETAILS.take();
    #[cfg(all(feature = "transport-stream", feature = "scene-probe"))]
    esp_println::println!("transport boot terrain");
    #[cfg(feature = "streamed-land")]
    let land = land_stream();
    #[cfg(feature = "urban-expansion")]
    let urban = scene_memory.place(scenic_flight::land_stream::UrbanStream::<8>::new);
    #[cfg(feature = "urban-background")]
    let mut urban_background = urban_survey::Background::new();
    #[cfg(feature = "coastal-mesh")]
    let coastal_mesh = COASTAL_MESH.init_with(scenic_flight::coastal_landforms::CoastalLandmarks::new);
    #[cfg(all(feature = "coastal-mesh", not(feature = "river-stream")))]
    let scenery_depth = image_buffer(&SCENERY_DEPTH, u16::MAX);
    #[cfg(feature = "coastal-mesh")]
    let mut weather = scenic_flight::weather_effects::WeatherEffects::new();
    #[cfg(feature = "coastal-mesh")]
    let mut valley_mist = scenic_flight::weather_effects::ValleyMist::new();
    #[cfg(feature = "distant-skyline")]
    let mut skyline = scenic_flight::skyline::Skyline::new();
    #[cfg(feature = "distant-bridges")]
    let mut bridge_silhouettes = scenic_flight::bridge_silhouette::BridgeSilhouettes::new();
    #[cfg(feature = "shore-details")]
    let shore = SHORE.take();
    #[cfg(feature = "buffered-terrain")]
    let terrain_work: &mut [u16] = TERRAIN_WORK.take();
    #[cfg(all(feature = "buffered-terrain", feature = "transport-stream"))]
    let terrain_work = if INTERNAL_TERRAIN_PIXELS == 0 {
        let frames = if LOGICAL_SIDE == bands::SIDE { 2 } else { 1 };
        scene_memory.place_pixels(FRAME_PIXELS * frames)
    } else {
        terrain_work
    };
    let mut game = create_game();
    #[cfg(feature = "panel-worker")]
    if !cfg!(feature = "scene-probe") || (
        !cfg!(feature = "lineup-probe") && option_env!("SCENIC_PROFILE_TIMED_INPUT") == Some("1")
    ) {
        let timers = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
        button_capture::start(esp_hal::timer::PeriodicTimer::new(timers.timer0), controls.pins.take().unwrap());
    }
    #[cfg(feature = "scene-probe")]
    let fixed_probe_camera = probe::fixed_camera();
    #[cfg(feature = "scene-probe")]
    let fixed_probe_day = probe::fixed_day();
    #[cfg(feature = "native-airframe-probe")]
    let native_capture_frame = probe::native_capture_frame();
    #[cfg(not(feature = "scene-probe"))]
    let fixed_probe_camera: Option<Camera> = None;
    #[cfg(feature = "scene-probe")]
    {
        controls.probe_since = micros();
    }
    let delay = Delay::new();
    let mut next_tick = micros();
    let mut painted_picker = None;
    let mut picker_since = 0;
    let mut next_picker_frame = 0;
    #[cfg(not(feature = "scene-probe"))]
    let mut boot_picker_visible_since = None;
    let mut buffered_seed = None;
    let mut frame = 0_u32;
    let mut phase_total = [0_u64; 12];
    #[cfg(feature = "sample-probe")]
    let samples = sample_probe::SampleProbe::new();
    let mut steps_total = 0;
    #[cfg(feature = "scene-probe")]
    let mut peak_work_us = 0_u64;
    #[cfg(feature = "scene-probe")]
    let mut peak_scenery_us = 0_u64;
    #[cfg(all(feature = "scene-probe", feature = "packed-60-terrain"))]
    let mut terrain_comparison =
        (option_env!("SCENIC_PROFILE_PACKED_AB") == Some("1")).then(probe::TerrainComparison::default);
    #[cfg(feature = "scene-probe")]
    let mut detail_phases = [0_u64; 4];
    let mut window_started = 0;
    let mut scenery_raster = scene_raster::SceneRaster::new();
    #[cfg(feature = "urban-background")]
    let mut terrain_worker = scene_raster::TerrainWorker::new();
    #[cfg(feature = "scene-probe")]
    let report_interval = option_env!("SCENIC_PROFILE_REPORT_INTERVAL")
        .and_then(|value| value.parse::<u32>().ok()).filter(|&value| value >= 2).unwrap_or(60);
    #[cfg(not(feature = "scene-probe"))]
    let report_interval = 60;

    esp_println::println!(
        "Scenic Flight Waveshare 1.75C {}px probe={} buffered={} worker={} replay={} scenery={}",
        LOGICAL_SIDE,
        u8::from(cfg!(feature = "scene-probe")),
        u8::from(cfg!(feature = "buffered-terrain")),
        u8::from(cfg!(feature = "panel-worker")),
        u8::from(cfg!(feature = "replay-ticks")),
        u8::from(cfg!(feature = "scenery")),
    );
    loop {
        #[cfg(feature = "parity-fixtures")]
        if micros() - controls.probe_since >= 17_000_000 {
            fixtures_device::run(&mut display, image, terrain, terrain_work, details);
        }
        controls.sample(&mut game);
        let now = micros();
        #[cfg(not(feature = "scene-probe"))]
        if let Some(since) = boot_picker_visible_since {
            if game.launch_if_boot_idle((now - since) / 1_000) {
                esp_println::println!("boot idle flight aircraft={}", game.selected() as u8);
                next_tick = now;
            }
        }
        if game.screen() == Screen::AircraftPicker {
            buffered_seed = None;
            next_tick = now;
            let selected = game.selected();
            let changed = painted_picker != Some(selected);
            if changed {
                picker_since = now;
            } else if now < next_picker_frame {
                delay.delay_millis(2);
                continue;
            }
            next_picker_frame = now + 50_000;
            let orbit = ((now - picker_since) / PERIOD_US) as u32;
            #[cfg(feature = "native-flight-overlay")]
            let native_picker = {
                scenic_flight::ui::draw_picker_stage(image, LOGICAL_SIDE, LOGICAL_SIDE);
                let native = display.native_overlay();
                let mut canvas = native.canvas::<LOGICAL_SIDE>(image);
                let side = native_flight::aircraft_side::<LOGICAL_SIDE>();
                scenic_flight::aircraft_art::draw_preview_on(
                    selected, orbit, &mut canvas, side, side,
                );
                let complete = if native.is_complete() {
                    let ui = native.ui();
                    scenic_flight::ui::draw_picker_labels_on(selected, bands::SIDE, bands::SIDE, |rect, ink| {
                        ui.rect(rect, ink);
                    });
                    ui.is_complete()
                } else {
                    false
                };
                if !complete {
                    native.clear();
                }
                complete
            };
            #[cfg(not(feature = "native-flight-overlay"))]
            let native_picker = false;
            if !native_picker {
                draw_picker(selected, image, LOGICAL_SIDE, LOGICAL_SIDE);
                draw_preview(selected, orbit, image, LOGICAL_SIDE, LOGICAL_SIDE);
            }
            let presented = display.present(image, || controls.sample(&mut game));
            image = presented.image;
            #[cfg(not(feature = "scene-probe"))]
            boot_picker_visible_since.get_or_insert_with(micros);
            painted_picker = Some(selected);
            if changed {
                esp_println::println!(
                    "picker aircraft={} panel_bytes={}",
                    selected as u8,
                    presented.bytes
                );
            }
            continue;
        }
        if now < next_tick {
            delay.delay_millis(1);
            continue;
        }

        painted_picker = None;
        let started = micros();
        let seed = game.world_seed().unwrap();
        let mut reset_us = 0;
        #[cfg(feature = "scenery")]
        let mut detail_reset_us = 0;
        #[cfg(not(feature = "scenery"))]
        let detail_reset_us = 0;
        let mut simulation_steps = 0;
        if buffered_seed != Some(seed) {
            let (world, launch) = {
                let (world, flight) = game.active().unwrap();
                (*world, flight_camera(flight.view))
            };
            #[cfg(feature = "coastal-fixture")]
            let launch = probe::coastal_camera(seed, 0).unwrap_or(launch);
            #[cfg(feature = "cascade-fixture")]
            let launch = probe::cascade_camera(seed, 0).unwrap_or(launch);
            #[cfg(feature = "caprock-fixture")]
            let launch = probe::caprock_camera(seed).unwrap_or(launch);
            #[cfg(feature = "outwash-fixture")]
            let launch = probe::outwash_camera(seed).unwrap_or(launch);
            let launch = fixed_probe_camera.unwrap_or(launch);
            #[cfg(all(feature = "streamed-land", not(feature = "coastal-fixture")))]
            let survey_launch = launch;
            #[cfg(feature = "coastal-fixture")]
            let survey_launch = probe::coastal_preflight_camera(seed).unwrap_or(launch);
            #[cfg(feature = "streamed-land")]
            let land_reset_at = micros();
            #[cfg(feature = "streamed-land")]
            {
                land.reset(world);
                land.preflight(survey_launch, cfg!(any(feature = "coastal-fixture", feature = "cascade-fixture")));
            }
            #[cfg(feature = "distant-skyline")]
            skyline.reset();
            #[cfg(feature = "distant-roads")]
            distant_roads.reset();
            #[cfg(feature = "distant-bridges")]
            bridge_silhouettes.reset();
            #[cfg(all(feature = "streamed-land", not(any(feature = "replay-ticks", feature = "coastal-fixture", feature = "cascade-fixture", feature = "caprock-fixture", feature = "outwash-fixture"))))]
            let (launch, survey_launch) = if fixed_probe_camera.is_none() && game.stage_town_approach(land) {
                let approach = flight_camera(game.active().unwrap().1.view);
                land.preflight(approach, false);
                (approach, approach)
            } else {
                (launch, survey_launch)
            };
            #[cfg(feature = "distant-skyline")]
            land.preflight_skyline(survey_launch);
            #[cfg(feature = "scene-probe")]
            probe::report_launch(seed, launch);
            #[cfg(feature = "streamed-land")]
            let land_reset_us = micros() - land_reset_at;
            #[cfg(all(feature = "scenery", not(feature = "streamed-land")))]
            let land_reset_us = 0;
            let reset_at = micros();
            terrain.reset_for_altitude(world, launch.x, launch.z, launch.altitude);
            #[cfg(feature = "far-horizon")]
            horizon.reset(world, launch.x, launch.z);
            reset_us = micros() - reset_at;
            #[cfg(feature = "scenery")]
            {
                let detail_reset_at = micros();
                details.reset(&world, seed, launch.x, launch.z);
                #[cfg(feature = "coastal-mesh")]
                coastal_mesh.reset(&world, seed, launch.x, launch.z);
                #[cfg(feature = "shore-details")]
                shore.reset(&world, seed, launch.x, launch.z);
                #[cfg(feature = "transport-stream")]
                {
                    facilities.reset(world, seed);
                    facilities.preflight(land);
                }
                #[cfg(feature = "river-stream")]
                {
                    rivers.reset(world, seed);
                    rivers.preflight(land, facilities, survey_launch, launch_water);
                }
                #[cfg(feature = "local-routes")]
                {
                    routes.reset(world, seed);
                    routes.preflight(land, survey_launch, |key, route| {
                        route_reservation(&world, facilities, rivers, key, route)
                    });
                }
                #[cfg(feature = "natural-scenery")]
                natural.reset(world, seed);
                #[cfg(feature = "regional-rail")]
                regional_rail.reset(world, seed);
                #[cfg(feature = "highway-stream")]
                {
                    highways.reset(world, seed);
                    let reserve = |pair, link: &scenic_flight::highway_stream::HighwayConnection| {
                        highway_reservation_with_nature(land, facilities, rivers, routes, natural, pair, link)
                    };
                    #[cfg(feature = "coastal-fixture")]
                    highways.preflight_with_coast(land, survey_launch, launch_coast, reserve, || {});
                    #[cfg(all(feature = "coastal-highways", not(feature = "coastal-fixture")))]
                    highways.preflight_defer_coast(land, survey_launch, reserve, || {});
                    #[cfg(not(feature = "coastal-highways"))]
                    highways.preflight(land, survey_launch, reserve, || {});
                }
                #[cfg(feature = "natural-scenery")]
                {
                    natural.preflight(launch, NaturalBudget { alpine_tiles: 9, dry_tiles: 9 }, |bounds| {
                        reserve_natural(land, facilities, rivers, routes,
                            #[cfg(feature = "highway-stream")] highways,
                            #[cfg(feature = "regional-rail")] regional_rail,
                            bounds)
                    });
                    summits.preflight(world, seed, launch, |bounds| {
                        if natural.reserves(bounds) { return NaturalReservation::Reserved; }
                        reserve_natural(land, facilities, rivers, routes,
                            #[cfg(feature = "highway-stream")] highways,
                            #[cfg(feature = "regional-rail")] regional_rail,
                            bounds)
                    });
                }
                #[cfg(feature = "highway-stream")]
                facilities.publish_roadside_sites(launch, true, |bounds| {
                    scenic_flight::facility_stream::roadside_reservation(land, rivers, routes, highways, natural, bounds)
                });
                #[cfg(feature = "urban-background")]
                urban_background.reset(urban);
                #[cfg(all(feature = "urban-expansion", not(feature = "urban-background")))]
                prepare_urban_launch(urban, land, facilities, rivers, routes, highways, regional_rail, natural, summits,
                    coastal_mesh, shore, &world, seed);
                detail_reset_us = micros() - detail_reset_at + land_reset_us;
            }
            buffered_seed = Some(seed);
            next_tick = micros() + PERIOD_US;
            frame = 0;
            scenery_raster = scene_raster::SceneRaster::new();
            #[cfg(feature = "urban-background")]
            { terrain_worker = scene_raster::TerrainWorker::new(); }
            phase_total = [0; 12];
            steps_total = 0;
            #[cfg(feature = "scene-probe")]
            {
                peak_work_us = 0;
                peak_scenery_us = 0;
                detail_phases = [0; 4];
            }
            #[cfg(all(feature = "scene-probe", feature = "packed-60-terrain"))]
            if let Some(comparison) = terrain_comparison.as_mut() {
                comparison.reset();
            }
            window_started = started;
            esp_println::println!(
                "flight aircraft={} seed={:016x}",
                game.selected() as u8,
                seed
            );
        } else {
            let elapsed = now - next_tick;
            let periods = elapsed / PERIOD_US + 1;
            simulation_steps = if cfg!(feature = "replay-ticks") {
                1
            } else {
                periods.min(MAX_SIMULATION_STEPS)
            };
            for _ in 0..simulation_steps {
                #[cfg(feature = "transport-stream")]
                observe_nearby_train(&mut game, facilities, #[cfg(feature = "regional-rail")] regional_rail);
                #[cfg(not(any(feature = "coastal-mesh", feature = "streamed-land")))]
                game.tick();
                #[cfg(any(feature = "coastal-mesh", feature = "streamed-land"))]
                game.tick_with_obstacles(|x, z| {
                    let height = 0.0_f32;
                    #[cfg(feature = "coastal-mesh")]
                    let height = height.max(coastal_mesh.obstruction(x, z));
                    #[cfg(feature = "streamed-land")]
                    let height = height.max(land.obstruction(x, z));
                    #[cfg(feature = "urban-expansion")]
                    let height = height.max(urban.obstruction(land, x, z));
                    #[cfg(feature = "shore-details")]
                    let height = height.max(shore.obstruction(x, z, |_light| {
                        #[cfg(feature = "transport-stream")]
                        { facilities.retains_lighthouse(_light) }
                        #[cfg(not(feature = "transport-stream"))]
                        { true }
                    }));
                    #[cfg(feature = "transport-stream")]
                    let height = height.max(facilities.obstruction(x, z));
                    #[cfg(feature = "river-stream")]
                    let height = height.max(rivers.obstruction(x, z));
                    #[cfg(feature = "local-routes")]
                    let height = height.max(routes.obstruction(x, z));
                    #[cfg(feature = "natural-scenery")]
                    let height = height.max(natural.obstruction(x, z));
                    #[cfg(feature = "natural-scenery")]
                    let height = height.max(summits.obstruction(x, z));
                    #[cfg(feature = "coastal-highways")]
                    let height = height.max(highways.obstruction(x, z));
                    height
                });
            }
            next_tick += periods * PERIOD_US;
        }

        let (_world, flight) = game.active().unwrap();
        #[cfg(not(feature = "replay-ticks"))]
        let display_pose = flight.display_pose(1.0 - next_tick.saturating_sub(now) as f32 / PERIOD_US as f32);
        #[cfg(not(feature = "replay-ticks"))]
        let flight = &display_pose;
        let ticks = flight.ticks;
        #[cfg(all(feature = "scene-probe", feature = "natural-scenery"))]
        let recoveries = flight.terrain_recoveries;
        let view = flight.view;
        let camera = flight_camera(view);
        #[cfg(feature = "coastal-fixture")]
        let camera = probe::coastal_camera(seed, ticks).unwrap_or(camera);
        #[cfg(feature = "cascade-fixture")]
        let camera = probe::cascade_camera(seed, ticks).unwrap_or(camera);
        #[cfg(feature = "caprock-fixture")]
        let camera = probe::caprock_camera(seed).unwrap_or(camera);
        #[cfg(feature = "outwash-fixture")]
        let camera = probe::outwash_camera(seed).unwrap_or(camera);
        let camera = fixed_probe_camera.unwrap_or(camera);
        let scene_side = if ADAPTIVE_SCENERY { scenery_raster.side() } else { LOGICAL_SIDE };
        let scene_dimensions = (scene_side, scene_side);
        let position = [camera.x, camera.altitude, camera.z];
        #[cfg(not(feature = "scene-probe"))]
        let sky = Sky::at_position(seed, ticks, position);
        #[cfg(feature = "scene-probe")]
        let sky = fixed_probe_day.map_or_else(
            || Sky::at_position(seed, ticks, position),
            |day| Sky::from_time_at_position(seed, ticks, day, position),
        );
        #[cfg(feature = "coastal-mesh")]
        let (sky, rain) = {
            let conditions = weather.observe(seed, ticks, camera, &sky);
            (sky.with_shower(conditions.shower), conditions.distant_rain)
        };
        #[cfg(feature = "coastal-mesh")]
        let mist = (!cfg!(feature = "scene-probe") || option_env!("SCENIC_PROFILE_NO_MIST").is_none())
            .then(|| valley_mist.observe(_world, seed, ticks, camera, &sky, scene_dimensions));
        #[cfg(feature = "cooperative-sky")]
        let mut prepared_sky = (option_env!("SCENIC_PROFILE_NO_SKY_WORKER").is_none()
            || !cfg!(feature = "scene-probe"))
            .then(|| display.prepare_sky(camera, sky, scene_dimensions));
        let follow_at = micros();
        terrain.follow(camera.x, camera.z);
        #[cfg(feature = "far-horizon")]
        horizon.follow(camera.x, camera.z, 16);
        let details_at = micros();
        #[cfg(feature = "scenery")]
        details.follow(_world, camera.x, camera.z);
        #[cfg(feature = "coastal-mesh")]
        coastal_mesh.follow(_world, seed, camera.x, camera.z, 1);
        #[cfg(feature = "shore-details")]
        shore.follow(_world, seed, camera.x, camera.z, frame);
        #[cfg(feature = "scene-probe")]
        let detail_caches_at = micros();
        #[cfg(feature = "streamed-land")]
        if frame % 2 == 0 {
            use scenic_flight::land_stream::LandBudget;
            land.service(camera, LandBudget {
                expansions: 8,
                terrain_samples: 32,
                settlement_jobs: u8::from(!cfg!(feature = "urban-background") && frame % 16 == 2),
            });
        }
        #[cfg(feature = "streamed-land")]
        let visible_land = land.has_visible_scenery(camera);
        #[cfg(feature = "scene-probe")]
        let detail_land_at = micros();
        #[cfg(feature = "transport-stream")]
        if frame % 4 == 0 && (frame >= 240 || !cfg!(feature = "scene-probe")
            || option_env!("SCENIC_PROFILE_DELAY_SURVEYS").is_none()) {
            #[cfg(feature = "natural-scenery")]
            let summit_scan_due = frame % 32 == 0;
            update_surveys(&mut display, facilities, land, camera,
                #[cfg(feature = "urban-background")]
                urban_survey::Context {
                    stream: urban, background: &mut urban_background, coast: coastal_mesh,
                    shore, seed, frame,
                },
                #[cfg(feature = "river-stream")] rivers,
                #[cfg(feature = "local-routes")] routes,
                #[cfg(feature = "local-routes")] _world,
                #[cfg(feature = "natural-scenery")] natural,
                #[cfg(feature = "natural-scenery")] summits,
                #[cfg(feature = "natural-scenery")] summit_scan_due,
                #[cfg(feature = "natural-scenery")] if frame % 8 == 0 { NaturalLayer::Alpine } else { NaturalLayer::Dry },
                #[cfg(feature = "highway-stream")] highways,
                #[cfg(feature = "regional-rail")] regional_rail,
            );
        }
        #[cfg(feature = "scene-probe")]
        let detail_surveys_at = micros();
        #[cfg(feature = "transport-stream")]
        let visible_facilities = facilities.has_visible_facilities(camera);
        #[cfg(feature = "transport-stream")]
        let facility_sampler = visible_facilities.then(|| facilities.sampler());
        #[cfg(all(feature = "streamed-land", not(feature = "urban-expansion")))]
        let land_sampler = visible_land.then(|| land.paint_sampler());
        #[cfg(feature = "urban-expansion")]
        let land_sampler = visible_land.then(|| urban.paint_sampler(land, camera));
        #[cfg(feature = "coastal-mesh")]
        let visible_mesh = |form: &scenic_flight::coastal_landforms::CoastalLandform| {
            scenic_flight::world_mesh::bounds_in_view(camera, form.bounds()) && {
                #[cfg(feature = "transport-stream")]
                { facilities.retains_landmark(form) }
                #[cfg(not(feature = "transport-stream"))]
                { true }
            }
        };
        #[cfg(feature = "coastal-mesh")]
        let has_landmark = coastal_mesh.forms().any(visible_mesh);
        #[cfg(all(feature = "coastal-mesh", feature = "streamed-land"))]
        let tower_region = if visible_land { land.distant_tower_region(camera, scene_dimensions) } else { None };
        #[cfg(all(feature = "coastal-mesh", not(feature = "streamed-land")))]
        let tower_region: Option<scenic_flight::render::ScreenRect> = None;
        #[cfg(feature = "coastal-mesh")]
        let has_towers = tower_region.is_some();
        #[cfg(all(feature = "distant-skyline", not(feature = "urban-expansion")))]
        let distant_skyline = skyline.prepare(land, camera, ticks, &sky, scene_dimensions,
            |x, z| horizon.backdrop(x, z));
        #[cfg(feature = "urban-expansion")]
        let distant_skyline = skyline.prepare_with_additions(land, camera, ticks, &sky, scene_dimensions,
            urban.skyline(land).chain(facilities.waterfront_silhouettes()), |x, z| horizon.backdrop(x, z));
        #[cfg(feature = "distant-skyline")]
        let tower_region = tower_region.into_iter().chain(distant_skyline.bounds())
            .reduce(scenic_flight::render::ScreenRect::union);
        #[cfg(feature = "shore-details")]
        let retains_light = |_light: &scenic_flight::lighthouses::Lighthouse| {
            #[cfg(feature = "transport-stream")]
            { facilities.retains_lighthouse(_light) }
            #[cfg(not(feature = "transport-stream"))]
            { true }
        };
        #[cfg(feature = "coastal-mesh")]
        let light_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(feature = "shore-details")]
            { shore.light_region(camera, scene_dimensions, retains_light) }
            #[cfg(not(feature = "shore-details"))]
            { None }
        };
        #[cfg(feature = "coastal-mesh")]
        let has_light = light_region.is_some();
        #[cfg(feature = "shore-details")]
        let near_light = has_light && shore.has_near_light(camera, retains_light);
        #[cfg(feature = "coastal-mesh")]
        let bridge_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(feature = "river-stream")]
            { rivers.bridge_region(camera, scene_dimensions) }
            #[cfg(not(feature = "river-stream"))]
            { None }
        };
        #[cfg(feature = "river-stream")]
        let visible_rivers = rivers.has_visible_rivers(camera);
        #[cfg(feature = "river-stream")]
        let river_sampler = visible_rivers.then(|| rivers.sampler());
        #[cfg(feature = "coastal-mesh")]
        let route_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(feature = "local-routes")]
            { routes.depth_region(camera, scene_dimensions) }
            #[cfg(not(feature = "local-routes"))]
            { None }
        };
        #[cfg(feature = "local-routes")]
        let visible_routes = routes.has_visible_routes(camera);
        #[cfg(feature = "natural-scenery")]
        let visible_nature = natural.has_visible_features(camera);
        #[cfg(feature = "natural-scenery")]
        let natural_sampler = visible_nature.then(|| natural.sampler());
        #[cfg(feature = "coastal-mesh")]
        let waterfall_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(feature = "natural-scenery")]
            { natural.waterfall_region(camera, scene_dimensions) }
            #[cfg(not(feature = "natural-scenery"))]
            { None }
        };
        #[cfg(feature = "coastal-mesh")]
        let summit_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(feature = "natural-scenery")]
            { summits.region(camera, scene_dimensions) }
            #[cfg(not(feature = "natural-scenery"))]
            { None }
        };
        #[cfg(feature = "coastal-mesh")]
        let water_tower_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(feature = "transport-stream")]
            {
                let reach = scenic_flight::world_mesh::landmark_reach(camera.altitude);
                facilities.water_towers().filter(|tower| {
                    scenic_flight::world_mesh::bounds_in_range(camera, tower.bounds(), reach)
                }).filter_map(|tower| {
                    scenic_flight::render::screen_region(camera, scene_dimensions, tower.mesh_bounds())
                }).reduce(scenic_flight::render::ScreenRect::union)
            }
            #[cfg(not(feature = "transport-stream"))]
            { None }
        };
        #[cfg(feature = "highway-stream")]
        let visible_highways = highways.has_visible_links(camera);
        #[cfg(feature = "highway-stream")]
        let highway_sampler = visible_highways.then(|| highways.sampler());
        #[cfg(feature = "highway-stream")]
        let highway_ribbons: [_; 4] = {
            use scenic_flight::highway_art::HighwayRibbon;
            let mut visible = highways.links().filter(|_| visible_highways).filter_map(|(_, link)| {
                HighwayRibbon::new(link, camera, scene_dimensions)
            });
            core::array::from_fn(|_| visible.next())
        };
        #[cfg(feature = "coastal-highways")]
        let coastal_ribbons: [_; 8] = {
            use scenic_flight::highway_art::HighwayRibbon;
            let mut visible = highways.coastal_links()
                .filter(|_| visible_highways)
                .flat_map(|(_, link)| link.approaches())
                .filter_map(|points| HighwayRibbon::for_approach(points, camera, scene_dimensions));
            core::array::from_fn(|_| visible.next())
        };
        #[cfg(feature = "coastal-mesh")]
        let highway_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(feature = "highway-stream")]
            {
                let visible = highway_ribbons.iter().flatten();
                #[cfg(feature = "coastal-highways")]
                let visible = visible.chain(coastal_ribbons.iter().flatten());
                visible.map(|ribbon| ribbon.depth_region()).reduce(scenic_flight::render::ScreenRect::union)
            }
            #[cfg(not(feature = "highway-stream"))]
            { None }
        };
        #[cfg(feature = "distant-roads")]
        let highway_region = {
            distant_roads.prepare(land, highways, camera, ticks, scene_dimensions,
                |x, z| horizon.backdrop(x, z));
            highway_region.into_iter().chain(distant_roads.bounds())
                .reduce(scenic_flight::render::ScreenRect::union)
        };
        #[cfg(feature = "coastal-mesh")]
        let crossing_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(feature = "coastal-highways")]
            { highways.crossing_region(camera, scene_dimensions) }
            #[cfg(not(feature = "coastal-highways"))]
            { None }
        };
        #[cfg(feature = "distant-bridges")]
        let distant_bridges = bridge_silhouettes.prepare(camera, ticks, &sky, scene_dimensions,
            rivers.bridges().chain(highways.coastal_links().map(|(_, link)| link.bridge())),
            |x, z| horizon.backdrop(x, z));
        #[cfg(feature = "distant-bridges")]
        let distant_bridge_region = distant_bridges.bounds();
        #[cfg(all(feature = "coastal-mesh", not(feature = "distant-bridges")))]
        let distant_bridge_region = None;
        #[cfg(all(feature = "transport-stream", feature = "far-horizon"))]
        let distant_airfields = scenic_flight::distant_airfields::DistantAirfields::prepare(
            facilities.airfields(), camera, &sky, scene_dimensions, |x, z| horizon.backdrop(x, z),
        );
        #[cfg(feature = "coastal-mesh")]
        let distant_airfield_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(all(feature = "transport-stream", feature = "far-horizon"))]
            { distant_airfields.depth_region() }
            #[cfg(not(all(feature = "transport-stream", feature = "far-horizon")))]
            { None }
        };
        #[cfg(feature = "coastal-mesh")]
        let rail_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(feature = "transport-stream")]
            {
                facilities.rails(camera, LOCAL_RAIL_REACH)
                    .filter_map(|line| scenic_flight::rail_art::rail_screen_region(line, camera, scene_dimensions))
                    .reduce(scenic_flight::render::ScreenRect::union)
            }
            #[cfg(not(feature = "transport-stream"))]
            { None }
        };
        #[cfg(feature = "coastal-mesh")]
        let has_rails = rail_region.is_some();
        #[cfg(feature = "regional-rail")]
        let corridor_region = regional_rail.rails(camera, scenic_flight::render::view_distance(camera.altitude))
            .filter_map(|line| scenic_flight::rail_art::corridor_screen_region(
                line, camera, scene_dimensions,
            ))
            .reduce(scenic_flight::render::ScreenRect::union);
        #[cfg(all(feature = "coastal-mesh", not(feature = "regional-rail")))]
        let corridor_region: Option<scenic_flight::render::ScreenRect> = None;
        #[cfg(feature = "regional-rail")]
        let corridor_sampler = regional_rail.rails(camera, scenic_flight::render::view_distance(camera.altitude))
            .next().map(|_| regional_rail.sampler());
        #[cfg(feature = "transport-stream")]
        let trains = {
            use scenic_flight::train_art::{Train, TRAIN_RENDER_REACH};
            let visible = facilities.rails(camera, TRAIN_RENDER_REACH).filter_map(|line| {
                Train::at(line, seed, ticks, camera, scene_dimensions)
            });
            #[cfg(feature = "regional-rail")]
            let visible = visible.chain(regional_rail.rails(camera, TRAIN_RENDER_REACH).filter_map(|line| {
                Train::on_corridor(line, seed, ticks, camera, scene_dimensions)
            }));
            Train::visible_pair(visible)
        };
        #[cfg(feature = "coastal-mesh")]
        let train_region: Option<scenic_flight::render::ScreenRect> = {
            #[cfg(feature = "transport-stream")]
            {
                trains.iter().flatten().map(scenic_flight::train_art::Train::depth_region)
                    .reduce(scenic_flight::render::ScreenRect::union)
            }
            #[cfg(not(feature = "transport-stream"))]
            { None }
        };
        #[cfg(feature = "coastal-mesh")]
        let has_mesh = has_landmark || has_towers || has_light || has_rails
            || bridge_region.is_some() || route_region.is_some()
            || highway_region.is_some() || crossing_region.is_some() || waterfall_region.is_some()
            || train_region.is_some() || corridor_region.is_some() || summit_region.is_some()
            || water_tower_region.is_some();
        let render_at = micros();
        #[cfg(feature = "scenery")]
        let trees = details.tree_sampler().for_view(camera.x, camera.altitude, camera.z);
        let sample = |x, z| {
            #[cfg(feature = "sample-probe")]
            let mut timer = samples.begin();
            #[cfg(feature = "sample-probe")]
            if let Some(timer) = &mut timer { timer.mark(sample_probe::CLOCK); }
            let base = terrain.sample(x, z);
            #[cfg(feature = "sample-probe")]
            if let Some(timer) = &mut timer { timer.mark(sample_probe::TERRAIN); }
            #[cfg(feature = "transport-stream")]
            let raw = base;
            #[cfg(feature = "highway-stream")]
            let base = highway_sampler.as_ref().map_or(base, |sampler| sampler.overlay(x, z, base));
            #[cfg(feature = "natural-scenery")]
            let base = natural_sampler.as_ref().map_or(base, |sampler| sampler.overlay(x, z, base));
            #[cfg(feature = "sample-probe")]
            if let Some(timer) = &mut timer { timer.mark(sample_probe::NATURAL); }
            #[cfg(feature = "scenery")]
            let base = details.geometry(x, z, base, &trees);
            #[cfg(feature = "shore-details")]
            let base = if near_light { shore.overlay(x, z, base, retains_light) } else { base };
            #[cfg(feature = "streamed-land")]
            let base = land_sampler.as_ref().map_or(base, |sampler| sampler.overlay(x, z, base));
            #[cfg(feature = "sample-probe")]
            if let Some(timer) = &mut timer { timer.mark(sample_probe::DETAILS); }
            #[cfg(feature = "river-stream")]
            let base = river_sampler.as_ref().map_or(base, |sampler| sampler.overlay(x, z, base));
            #[cfg(feature = "sample-probe")]
            if let Some(timer) = &mut timer { timer.mark(sample_probe::RIVER); }
            #[cfg(feature = "transport-stream")]
            let base = facility_sampler.as_ref().map_or(base, |sampler| sampler.overlay(x, z, raw, base));
            #[cfg(feature = "local-routes")]
            let base = if visible_routes { routes.overlay(x, z, base) } else { base };
            #[cfg(feature = "regional-rail")]
            let base = corridor_sampler.as_ref().map_or(base, |sampler| sampler.overlay(x, z, base));
            #[cfg(feature = "sample-probe")]
            if let Some(mut timer) = timer {
                timer.mark(sample_probe::FACILITIES);
                timer.finish();
            }
            base
        };
        #[cfg(all(feature = "coastal-mesh", feature = "ground-cover"))]
        let desert_props = scenic_flight::desert_props::DesertProps::gather_with_habitats(
            camera, scene_dimensions, seed,
            {
                #[cfg(feature = "river-stream")]
                { river_sampler.is_some() }
                #[cfg(not(feature = "river-stream"))]
                { false }
            },
            |x, z| terrain.sample(x, z),
            |x, z| {
                let ground = sample(x, z);
                #[cfg(feature = "scenery")]
                let ground = scenic_flight::world::Sample {
                    surface: details.material(x, z, ground),
                    ..ground
                };
                ground
            },
        );
        #[cfg(all(feature = "coastal-mesh", feature = "ground-cover"))]
        let desert_region = desert_props.region();
        #[cfg(all(feature = "coastal-mesh", not(feature = "ground-cover")))]
        let desert_region: Option<scenic_flight::render::ScreenRect> = None;
        #[cfg(feature = "sample-probe")]
        samples.enable(true);
        let scene = &mut image[..scene_side * scene_side];
        #[cfg(feature = "coastal-mesh")]
        let scene_depth = &mut scenery_depth[..scene.len()];
        #[cfg(not(feature = "buffered-terrain"))]
        scenic_flight::render::render_from_with_sky(
            sample,
            camera,
            &sky,
            scene,
            scene_side,
            scene_side,
        )
        .unwrap();
        #[cfg(feature = "buffered-terrain")]
        {
            #[cfg(feature = "far-horizon")]
            use scenic_flight::render::render_terrain_layer_region_with_horizon as render_layers;
            #[cfg(not(feature = "far-horizon"))]
            use scenic_flight::render::render_terrain_layer_region_with_sky as render_layers;
            #[cfg(feature = "scene-probe")]
            let mut last = render_at;
            #[cfg(feature = "coastal-mesh")]
            let depth = if has_landmark || rain.is_some() {
                Some((&mut scene_depth[..], scenic_flight::render::ScreenRect::full(scene_dimensions)))
            } else {
                tower_region.into_iter().chain(light_region).chain(bridge_region).chain(route_region)
                    .chain(rail_region).chain(corridor_region).chain(train_region)
                    .chain(highway_region).chain(crossing_region)
                    .chain(distant_bridge_region)
                    .chain(distant_airfield_region)
                    .chain(waterfall_region)
                    .chain(summit_region)
                    .chain(water_tower_region)
                    .chain(desert_region)
                    .chain(mist.as_ref().and_then(|mist| mist.depth_region()))
                    .reduce(scenic_flight::render::ScreenRect::union)
                    .map(|rect| (&mut scene_depth[..], rect))
            };
            #[cfg(not(feature = "coastal-mesh"))]
            let depth = None;
            let material = |x, z, geometry: scenic_flight::world::Sample| {
                #[cfg(feature = "scenery")]
                { details.material(x, z, geometry) }
                #[cfg(not(feature = "scenery"))]
                { let _ = (x, z); geometry.surface }
            };
            let checkpoint = |point| {
                #[cfg(feature = "scene-probe")]
                {
                    use scenic_flight::render::RenderCheckpoint;
                    let now = micros();
                    let index = match point {
                        RenderCheckpoint::Terrain => 9,
                        RenderCheckpoint::Rotation => 10,
                        RenderCheckpoint::Atmosphere => 11,
                    };
                    #[cfg(feature = "sample-probe")]
                    if index == 9 { samples.enable(false); }
                    phase_total[index] += now - last;
                    last = now;
                }
                #[cfg(not(feature = "scene-probe"))]
                let _ = point;
            };
            let ready_sky_tile = |index| {
                #[cfg(feature = "cooperative-sky")]
                { prepared_sky.as_mut().and_then(|prepared| prepared.color(index)) }
                #[cfg(not(feature = "cooperative-sky"))]
                { let _ = index; None }
            };
            macro_rules! serial_terrain {
                () => {
                    render_layers(
                        (&sample, material),
                        #[cfg(feature = "far-horizon")]
                        |x, z| horizon.backdrop(x, z),
                        camera, &sky, scene, terrain_work, depth, scene_dimensions,
                        checkpoint, ready_sky_tile,
                    )
                };
            }

            #[cfg(not(feature = "parallel-terrain"))]
            let result = serial_terrain!();
            #[cfg(feature = "parallel-terrain")]
            let packed_60 = cfg!(feature = "packed-60-terrain")
                && matches!(scene_side, scene_raster::MIN_SIDE | scene_raster::HEAVY_SIDE | scene_raster::REDUCED_SIDE
                    | scene_raster::BALANCED_SIDE | scene_raster::FULL_SIDE);
            #[cfg(feature = "urban-background")]
            let packed_60 = {
                // DMA and terrain both preempt surveys on the worker. Give a
                // queued survey regular uninterrupted slices during city passes.
                let survey_turn = !display.survey_ready() && frame % 16 == 5;
                let complex_terrain = land_sampler.as_ref().is_some_and(|land| land.has_expansion())
                    || (terrain_worker.wants_help() && (frame >= 24 || urban.pending_region().is_none()));
                packed_60 && complex_terrain && !survey_turn
            };
            #[cfg(all(feature = "scene-probe", feature = "packed-60-terrain"))]
            let packed_60 = packed_60 && terrain_comparison.as_ref()
                .is_none_or(|_| probe::TerrainComparison::use_worker(frame));
            #[cfg(feature = "urban-background")]
            let terrain_started = micros();
            #[cfg(feature = "parallel-terrain")]
            let result = if (scene_side == 466 || packed_60)
                && display.terrain_ready() && !cfg!(feature = "sample-probe") {
                let worker_terrain = &*terrain;
                let worker_details = &*details;
                let worker_trees = details.tree_sampler().for_view(camera.x, camera.altitude, camera.z);
                let worker_highways = visible_highways.then(|| highways.sampler());
                let worker_natural = visible_nature.then(|| natural.sampler());
                #[cfg(not(feature = "urban-expansion"))]
                let worker_land = visible_land.then(|| land.paint_sampler());
                #[cfg(feature = "urban-expansion")]
                let worker_land = visible_land.then(|| urban.paint_sampler(land, camera));
                let worker_water = visible_rivers.then(|| rivers.sampler());
                let worker_sites = visible_facilities.then(|| facilities.sampler());
                let worker_routes = &*routes;
                let worker_rail = corridor_sampler.as_ref().map(|_| regional_rail.sampler());
                #[cfg(feature = "shore-details")]
                let (worker_shore, worker_facilities) = (&*shore, &*facilities);
                let worker_sample = move |x, z| {
                    let raw = worker_terrain.sample(x, z);
                    let base = worker_highways.as_ref().map_or(raw, |h| h.overlay(x, z, raw));
                    let base = worker_natural.as_ref().map_or(base, |n| n.overlay(x, z, base));
                    let base = worker_details.geometry(x, z, base, &worker_trees);
                    #[cfg(feature = "shore-details")]
                    let base = if near_light {
                        worker_shore.overlay(x, z, base, |light| worker_facilities.retains_lighthouse(light))
                    } else { base };
                    let base = worker_land.as_ref().map_or(base, |l| l.overlay(x, z, base));
                    let base = worker_water.as_ref().map_or(base, |r| r.overlay(x, z, base));
                    let base = worker_sites.as_ref().map_or(base, |f| f.overlay(x, z, raw, base));
                    let base = if visible_routes { worker_routes.overlay(x, z, base) } else { base };
                    worker_rail.as_ref().map_or(base, |r| r.overlay(x, z, base))
                };
                let worker_material = move |x, z, sample| worker_details.material(x, z, sample);
                macro_rules! packed_terrain {
                    ($side:expr) => {
                        scenic_flight::render::render_with_packed_terrain_fork_sized::<{ $side }>(
                            &mut display, &sample, material, worker_sample, worker_material,
                            camera, &sky, scene, terrain_work, depth, checkpoint,
                            ready_sky_tile, |x, z| horizon.backdrop(x, z),
                        )
                    };
                }
                match scene_side {
                    scene_raster::MIN_SIDE if ADAPTIVE_SCENERY => packed_terrain!(scene_raster::MIN_SIDE),
                    scene_raster::HEAVY_SIDE if ADAPTIVE_SCENERY => packed_terrain!(scene_raster::HEAVY_SIDE),
                    scene_raster::REDUCED_SIDE if ADAPTIVE_SCENERY => packed_terrain!(scene_raster::REDUCED_SIDE),
                    scene_raster::BALANCED_SIDE if ADAPTIVE_SCENERY => packed_terrain!(scene_raster::BALANCED_SIDE),
                    _ => packed_terrain!(LOGICAL_SIDE),
                }
            } else {
                serial_terrain!()
            };

            #[cfg(feature = "parallel-terrain")]
            let fork_timing = display.take_terrain_timing();
            #[cfg(feature = "urban-background")]
            if !cfg!(feature = "sample-probe") {
                match fork_timing {
                    Some(timing) => terrain_worker.observe_split(
                        timing.left_us, timing.right_us, timing.join_us, timing.background,
                    ),
                    None => terrain_worker.observe_serial(micros() - terrain_started),
                }
            }
            #[cfg(all(feature = "scene-probe", feature = "packed-60-terrain"))]
            if let Some(comparison) = terrain_comparison.as_mut() {
                if let Some(timing) = fork_timing {
                    comparison.record(timing.background, timing.queue_us, timing.join_us);
                }
            }
            #[cfg(all(feature = "parallel-terrain", feature = "scene-probe"))]
            if option_env!("SCENIC_PROFILE_PACKED_AB") != Some("1") && (frame == 1 || frame % 6 == 0) {
                if let Some(timing) = fork_timing {
                    esp_println::println!("terrain fork frame={} mode={} queue_us={} left_us={} right_us={} join_us={} survey_busy={}",
                        frame, if timing.background { "worker" } else { "main-reclaim" },
                        timing.queue_us, timing.left_us, timing.right_us, timing.join_us, !display.survey_ready());
                } else {
                    esp_println::println!("terrain fork frame={} mode=serial survey_busy={}",
                        frame, !display.survey_ready());
                }
            }
            #[cfg(all(feature = "parallel-terrain", not(feature = "scene-probe")))]
            let _ = fork_timing;
            #[cfg(feature = "scene-probe")]
            if let Err(error) = result {
                esp_println::println!("render failure {:?} at frame={}", error, frame);
                esp_println::println!("render camera(x,y,z,heading,pitch,bank)={:08x?}",
                    [camera.x, camera.altitude, camera.z, camera.heading, camera.pitch, camera.bank].map(f32::to_bits));
                esp_println::println!("render sky(sun,moon,light,haze,cloud,base,drift)={:08x?}",
                    [sky.sun_direction[0], sky.sun_direction[1], sky.sun_direction[2],
                     sky.moon_direction[0], sky.moon_direction[1], sky.moon_direction[2],
                     sky.daylight, sky.haze, sky.cloud_cover, sky.cloud_base,
                     sky.cloud_offset[0], sky.cloud_offset[1]].map(f32::to_bits));
            }
            result.unwrap();
        }
        #[cfg(feature = "sample-probe")]
        samples.enable(false);
        #[cfg(all(feature = "cooperative-sky", feature = "scene-probe"))]
        if let Some(prepared_sky) = prepared_sky.filter(|_| frame % 60 == 0) {
            esp_println::println!("sky tiles reused={} fallback={}", prepared_sky.reused, prepared_sky.fallback);
        }
        let overlays_at = micros();
        #[cfg(all(feature = "transport-stream", feature = "far-horizon"))]
        distant_airfields.draw(scene, scene_depth);
        #[cfg(feature = "distant-roads")]
        distant_roads.draw(scene, scene_depth, terrain_work, scene_dimensions);
        #[cfg(feature = "distant-bridges")]
        distant_bridges.draw(scene, scene_depth);
        #[cfg(feature = "distant-skyline")]
        distant_skyline.draw(scene, scene_depth);
        #[cfg(feature = "local-routes")]
        if route_region.is_some() {
            routes.open_portals(camera, scene, scene_depth, scene_dimensions);
        }
        #[cfg(feature = "coastal-mesh")]
        if has_mesh || desert_region.is_some() {
            let mut mesh = scenic_flight::world_mesh::WorldMesh::new(
                scene, scene_depth, scene_dimensions, camera, &sky,
            ).unwrap();
            #[cfg(feature = "ground-cover")]
            desert_props.draw(&mut mesh);
            #[cfg(feature = "highway-stream")]
            for ribbon in highway_ribbons.iter().flatten() {
                ribbon.draw(&mut mesh, &sample);
            }
            #[cfg(feature = "coastal-highways")]
            {
                for ribbon in coastal_ribbons.iter().flatten() {
                    ribbon.draw(&mut mesh, &sample);
                }
                if crossing_region.is_some() {
                    #[cfg(not(feature = "distant-bridges"))]
                    highways.draw_crossings(camera, scene_dimensions, &mut mesh);
                    #[cfg(feature = "distant-bridges")]
                    highways.draw_crossings_fading(camera, scene_dimensions, &mut mesh,
                        |bridge| distant_bridges.detail_opacity(bridge));
                }
            }
            for form in coastal_mesh.forms().filter(|form| visible_mesh(form)) {
                form.draw(&mut mesh);
            }
            #[cfg(all(feature = "streamed-land", not(feature = "distant-skyline")))]
            if has_towers { land.draw_distant_towers(camera, &mut mesh); }
            #[cfg(feature = "distant-skyline")]
            if has_towers { distant_skyline.draw_legacy(land, camera, &mut mesh); }
            #[cfg(feature = "shore-details")]
            if has_light { shore.draw_lights(camera, sky.daylight, ticks, &mut mesh, retains_light); }
            #[cfg(all(feature = "river-stream", not(feature = "distant-bridges")))]
            if bridge_region.is_some() { rivers.draw(camera, &mut mesh); }
            #[cfg(feature = "distant-bridges")]
            if bridge_region.is_some() {
                rivers.draw_fading(camera, &mut mesh, |bridge| distant_bridges.detail_opacity(bridge));
            }
            #[cfg(feature = "natural-scenery")]
            if waterfall_region.is_some() { natural.draw_waterfalls(camera, ticks, &mut mesh); }
            #[cfg(feature = "natural-scenery")]
            if summit_region.is_some() {
                summits.draw(camera, sky.daylight, &mut mesh);
                summits.draw_trails(camera, &mut mesh, |x, z| sample(x, z).elevation);
            }
            #[cfg(feature = "transport-stream")]
            if water_tower_region.is_some() {
                let reach = scenic_flight::world_mesh::landmark_reach(camera.altitude);
                mesh.with_landmark_reach(reach, |mesh| {
                    for tower in facilities.water_towers().filter(|tower| {
                        scenic_flight::world_mesh::bounds_in_range(camera, tower.bounds(), reach)
                    }) {
                        tower.draw(mesh);
                    }
                });
            }
            #[cfg(feature = "local-routes")]
            if route_region.is_some() { routes.draw(camera, ticks, &mut mesh); }
            #[cfg(feature = "transport-stream")]
            if has_rails {
                for line in facilities.rails(camera, LOCAL_RAIL_REACH) {
                    scenic_flight::rail_art::draw_rail(&mut mesh, line, camera, scene_dimensions);
                }
            }
            #[cfg(feature = "regional-rail")]
            if corridor_region.is_some() {
                for line in regional_rail.rails(camera, scenic_flight::render::view_distance(camera.altitude)) {
                    scenic_flight::rail_art::draw_rail_corridor(&mut mesh, line, camera, scene_dimensions);
                }
            }
            #[cfg(feature = "transport-stream")]
            for train in trains.iter().flatten() {
                train.draw(&mut mesh, sky.daylight);
            }
        }
        #[cfg(feature = "coastal-mesh")]
        if let Some(rain) = rain {
            rain.paint(camera, scene_dimensions, scene, scene_depth);
        }
        #[cfg(feature = "shore-details")]
        shore.draw_beaches(camera, sky.daylight, scene, scene_side, &sample, |x, z| {
            #[cfg(feature = "streamed-land")]
            {
                let ground = terrain.sample(x, z);
                #[cfg(feature = "transport-stream")]
                {
                    #[cfg(feature = "highway-stream")]
                    let road = highway_sampler.as_ref().map_or(ground, |sampler| sampler.overlay(x, z, ground));
                    #[cfg(not(feature = "highway-stream"))]
                    let road = ground;
                    let developed = land.overlay(x, z, road);
                    #[cfg(feature = "river-stream")]
                    let developed = rivers.overlay(x, z, developed);
                    let developed = facilities.overlay(x, z, ground, developed);
                    #[cfg(feature = "local-routes")]
                    let developed = routes.overlay(x, z, developed);
                    developed != ground
                }
                #[cfg(not(feature = "transport-stream"))]
                { land.overlay(x, z, ground) != ground }
            }
            #[cfg(not(feature = "streamed-land"))]
            { let _ = (x, z); false }
        });
        #[cfg(feature = "coastal-mesh")]
        if let Some(mist) = &mist {
            mist.paint(camera, scene_dimensions, scene, scene_depth);
        }
        let scenery_rendered = if ADAPTIVE_SCENERY { micros() } else { 0 };
        if scene_side != LOGICAL_SIDE {
            scene_raster::expand_reduced(image, scene_side);
        }
        let flight_info = game.flight_info_visible();
        #[cfg(feature = "native-flight-overlay")]
        let new_world = game.new_world_progress((micros() / 1_000) as u32);
        #[cfg(feature = "native-airframe-probe")]
        let trace_native = option_env!("SCENIC_PROFILE_NATIVE_TRACE") == Some("1");
        #[cfg(feature = "native-airframe-probe")]
        let mut native_stats = (0usize, false, 0u64);
        #[cfg(feature = "native-flight-overlay")]
        let used_native_aircraft = {
            let native = display.native_overlay();
            #[cfg(feature = "native-airframe-probe")]
            let before = trace_native.then(micros);
            let mut canvas = native.canvas::<LOGICAL_SIDE>(image);
            let side = native_flight::aircraft_side::<LOGICAL_SIDE>();
            scenic_flight::aircraft_art::draw_flight_scene_on(
                flight, seed, &sky, &mut canvas, side, side,
            );
            let exhausted = !native.is_complete();
            #[cfg(feature = "native-airframe-probe")]
            if let Some(before) = before {
                native_stats = (native.tile_count(), exhausted, micros() - before);
            }
            if exhausted {
                native.clear();
                draw_flight_scene(flight, seed, &sky, image, LOGICAL_SIDE, LOGICAL_SIDE);
            }
            !exhausted
        };
        #[cfg(not(feature = "native-flight-overlay"))]
        {
            #[cfg(feature = "native-airframe-probe")]
            let before = trace_native.then(micros);
            draw_flight_scene(flight, seed, &sky, image, LOGICAL_SIDE, LOGICAL_SIDE);
            #[cfg(feature = "native-airframe-probe")]
            if let Some(before) = before {
                native_stats.2 = micros() - before;
            }
        }
        #[cfg(not(feature = "native-flight-overlay"))]
        let new_world = game.new_world_progress((micros() / 1_000) as u32);
        #[cfg(feature = "native-flight-overlay")]
        let native_ui_complete = {
            let ui = display.native_overlay().ui();
            if flight_info {
                scenic_flight::ui::draw_flight_info_on(flight, bands::SIDE, bands::SIDE, |rect, ink| {
                    ui.rect(rect, ink);
                });
            }
            if let Some(progress) = new_world {
                scenic_flight::ui::draw_new_world_on(progress, bands::SIDE, bands::SIDE, |rect, ink| {
                    ui.rect(rect, ink);
                });
            }
            ui.is_complete()
        };
        #[cfg(not(feature = "native-flight-overlay"))]
        let native_ui_complete = false;
        if !native_ui_complete {
            #[cfg(feature = "native-flight-overlay")]
            if used_native_aircraft {
                // A logical panel must cover the aircraft, including on overflow.
                display.native_overlay().clear();
                draw_flight_scene(flight, seed, &sky, image, LOGICAL_SIDE, LOGICAL_SIDE);
                #[cfg(feature = "native-airframe-probe")]
                { native_stats.1 = true; }
            }
            if flight_info {
                draw_flight_info(flight, image, LOGICAL_SIDE, LOGICAL_SIDE);
            }
            if let Some(progress) = new_world {
                draw_new_world(progress, image, LOGICAL_SIDE, LOGICAL_SIDE);
            }
        }

        let rendered = micros();
        frame += 1;
        #[cfg(feature = "scene-probe")]
        if option_env!("SCENIC_CUB_PROBE") == Some("1") && matches!(ticks, 0 | 60 | 120 | 240) {
            esp_println::println!("cub tick={} x={:.5} z={:.5} altitude={:.5} heading={:.6} clearance={:.4}",
                ticks, flight.x, flight.z, flight.altitude, flight.heading,
                flight.altitude - _world.elevation(flight.x, flight.z).max(scenic_flight::world::SEA_LEVEL));
        }
        #[cfg(feature = "coastal-fixture")]
        probe::report_coastal(
            seed, ticks, frame, camera, highways.coastal_links().count(),
            usize::from(crossing_region.is_some()),
        );
        #[cfg(feature = "scene-probe")]
        probe::capture(frame, image, LOGICAL_SIDE);
        #[cfg(feature = "burst-capture")]
        let burst_complete = burst.record(frame, ticks, rendered, image);
        let report = frame == 1 || frame % report_interval == 0;
        #[cfg(feature = "native-airframe-probe")]
        {
            let capture_frame = native_capture_frame == Some(frame);
            if capture_frame {
                if let Some(capture) = display.packed_capture() {
                    capture.request(frame);
                }
            }
            if trace_native && (report || capture_frame) {
                let (tiles, fallback, cpu_us) = native_stats;
                esp_println::println!(
                    "native airframe frame={} tick={} native={} tiles={} fallback={} cpu_us={} logical_ui={}",
                    frame, ticks, cfg!(feature = "native-flight-overlay"), tiles, fallback, cpu_us,
                    !native_ui_complete && (flight_info || new_world.is_some()),
                );
            }
        }
        let fingerprint = if report { frame_fingerprint(image) } else { 0 };
        let submit_at = micros();
        let presented = display.present(image, || controls.sample(&mut game));
        let finished = micros();
        image = presented.image;
        if ADAPTIVE_SCENERY && frame > 1 {
            scenery_raster.observe(rendered - started + finished - submit_at, scenery_rendered - render_at);
            #[cfg(feature = "scene-probe")]
            if scenery_raster.side() != scene_side {
                esp_println::println!("scene raster frame={} side={}", frame, scenery_raster.side());
            }
        }
        phase_total[0] += follow_at - started - reset_us - detail_reset_us;
        phase_total[1] += reset_us;
        phase_total[2] += details_at - follow_at;
        phase_total[3] += overlays_at - render_at;
        phase_total[4] += rendered - overlays_at;
        phase_total[5] += presented.transfer_us;
        phase_total[6] += finished - submit_at;
        phase_total[7] += detail_reset_us;
        phase_total[8] += render_at - details_at;
        steps_total += simulation_steps;
        #[cfg(feature = "scene-probe")]
        {
            peak_work_us = peak_work_us.max(rendered - started + finished - submit_at);
            peak_scenery_us = peak_scenery_us.max(render_at - details_at);
            detail_phases[0] += detail_caches_at - details_at;
            detail_phases[1] += detail_land_at - detail_caches_at;
            detail_phases[2] += detail_surveys_at - detail_land_at;
            detail_phases[3] += render_at - detail_surveys_at;
        }
        if report {
            #[cfg(feature = "scene-probe")]
            if let Some((_world, aircraft)) = game.active() {
                probe::navigation("render", seed, aircraft);
                #[cfg(feature = "lineup-probe")]
                probe::report_aircraft(_world, aircraft);
            }
            let window = u64::from(if frame == report_interval {
                report_interval - 1
            } else if frame == 1 {
                1
            } else {
                report_interval
            });
            esp_println::println!(
                "frame={} tick={} sim_steps={} sim_us={} reset_us={} follow_us={} detail_reset_us={} detail_follow_us={} render_us={} overlay_us={} panel_us={} present_us={} window_us={} panel_bytes={} rgb565={:08x} scene_side={}",
                frame,
                ticks,
                steps_total,
                phase_total[0] / window,
                phase_total[1] / window,
                phase_total[2] / window,
                phase_total[7] / window,
                phase_total[8] / window,
                phase_total[3] / window,
                phase_total[4] / window,
                phase_total[5] / window,
                phase_total[6] / window,
                finished - window_started,
                presented.bytes,
                fingerprint,
                scene_side,
            );
            #[cfg(all(feature = "buffered-terrain", feature = "scene-probe"))]
            esp_println::println!(
                "renderer frame={} ground_us={} rotate_us={} atmosphere_us={}",
                frame, phase_total[9] / window, phase_total[10] / window, phase_total[11] / window,
            );
            #[cfg(feature = "scene-probe")]
            {
                let [cache, land, survey, visible] = detail_phases.map(|phase| phase / window);
                esp_println::println!(
                    "detail frame={} cache_us={} land_us={} survey_us={} visible_us={}",
                    frame, cache, land, survey, visible,
                );
                detail_phases = [0; 4];
            }
            #[cfg(all(feature = "scene-probe", feature = "packed-60-terrain"))]
            if let Some(comparison) = terrain_comparison.as_mut() {
                comparison.report(frame);
            }
            #[cfg(feature = "sample-probe")]
            samples.report(frame);
            #[cfg(all(feature = "scene-probe", feature = "transport-stream"))]
            if fixed_probe_camera.is_some() {
                for (key, _) in land.districts() {
                    if let Some(site) = facilities.planned(key).and_then(|owner| owner.scenic_turnout()) {
                        esp_println::println!("roadside frame={} kind={:?} contact={:?} bounds={:?}",
                            frame, site.kind(), site.road_contact(), site.bounds());
                    }
                }
            }
            #[cfg(all(feature = "scene-probe", feature = "coastal-mesh"))]
            esp_println::println!("scenery frame={} coastal={} towers={} light={} rain={}", frame, has_landmark, has_towers, has_light, rain.is_some());
            #[cfg(all(feature = "scene-probe", feature = "coastal-mesh"))]
            if fixed_probe_day.is_some() {
                esp_println::println!("mist frame={} visible={}", frame, mist.as_ref().and_then(|mist| mist.depth_region()).is_some());
            }
            #[cfg(all(feature = "scene-probe", feature = "distant-skyline"))]
            esp_println::println!("skyline frame={} towers={} roofs={} neighborhoods={}", frame, distant_skyline.len(), distant_skyline.roof_count(), distant_skyline.neighborhood_count());
            #[cfg(all(feature = "scene-probe", feature = "distant-roads"))]
            esp_println::println!("distant roads frame={} strokes={}", frame, distant_roads.len());
            #[cfg(all(feature = "scene-probe", feature = "distant-bridges"))]
            esp_println::println!("distant bridges frame={} silhouettes={}", frame, distant_bridges.len());
            #[cfg(feature = "caprock-fixture")]
            if let Some(mesa) = natural.mesa() {
                esp_println::println!("caprock frame={} centers={:?}", frame, mesa.centers());
            }
            #[cfg(feature = "outwash-fixture")]
            if let Some(fan) = natural.outwash() {
                esp_println::println!("outwash frame={} bounds={:?}", frame, fan.bounds());
            }
            #[cfg(all(feature = "scene-probe", feature = "transport-stream"))]
            esp_println::println!("transport frame={} visible={} rails={} worker_ready={}", frame, visible_facilities, has_rails, display.survey_ready());
            #[cfg(all(feature = "scene-probe", feature = "transport-stream"))]
            esp_println::println!("water-towers frame={} visible={} retained={}", frame, water_tower_region.is_some(), facilities.water_towers().count());
            #[cfg(all(feature = "scene-probe", feature = "transport-stream"))]
            if fixed_probe_camera.is_some() {
                for quarry in facilities.quarries() {
                    esp_println::println!("quarry frame={} yard={:?} junction={:?}", frame, quarry.yard(), quarry.junction());
                }
                for site in facilities.solar_sites() {
                    esp_println::println!("solar frame={} gate={:?} junction={:?} layout={:?}",
                        frame, site.gate(), site.trunk(), site.rows());
                }
                for quay in facilities.quaysides() {
                    esp_println::println!("quayside frame={} contact={:?} end={:?} shops={}",
                        frame, quay.contact(), quay.end(), quay.shops());
                }
                for tower in facilities.water_towers() {
                    esp_println::println!("water-tower frame={} pad={:?} street={:?}",
                        frame, tower.access_road()[2], tower.access_road()[0]);
                }
            }
            #[cfg(all(feature = "scene-probe", feature = "river-stream"))]
            esp_println::println!("waterway frame={} water={} bridge={} route={}", frame, visible_rivers, bridge_region.is_some(), route_region.is_some());
            #[cfg(all(feature = "scene-probe", feature = "river-stream"))]
            esp_println::println!("wetlands frame={} visible={}", frame, rivers.wetland_sites(camera).count());
            #[cfg(all(feature = "scene-probe", feature = "river-stream"))]
            esp_println::println!("river islands frame={} visible={}", frame, rivers.wetland_sites(camera)
                .filter(|&kind| kind == scenic_flight::floodplain::FloodplainKind::RiverIsland).count());
            #[cfg(all(feature = "scene-probe", feature = "natural-scenery"))]
            esp_println::println!("nature frame={} features={} recoveries={}", frame, visible_nature, recoveries);
            #[cfg(all(feature = "scene-probe", feature = "natural-scenery"))]
            if fixed_probe_camera.is_some() {
                if let Some(glacier) = natural.glacier() {
                    esp_println::println!("glacier frame={} bounds={:?}", frame, glacier.bounds());
                }
            }
            #[cfg(all(feature = "scene-probe", feature = "natural-scenery"))]
            esp_println::println!("cascade frame={} visible={}", frame, waterfall_region.is_some());
            #[cfg(all(feature = "scene-probe", feature = "natural-scenery"))]
            esp_println::println!("scree frame={} visible={}", frame, natural.scree_fan().is_some_and(|fan| {
                scenic_flight::world_mesh::bounds_in_view(camera, fan.bounds())
            }));
            #[cfg(all(feature = "scene-probe", feature = "natural-scenery"))]
            esp_println::println!("summits frame={} visible={}", frame, summit_region.is_some());
            #[cfg(all(feature = "scene-probe", feature = "highway-stream"))]
            esp_println::println!("highway frame={} visible={} links={} ribbons={}",
                frame, visible_highways, highways.links().count(), highway_ribbons.iter().flatten().count());
            #[cfg(all(feature = "scene-probe", feature = "coastal-highways"))]
            esp_println::println!("coastal roads frame={} links={} bridge_visible={}",
                frame, highways.coastal_links().count(), crossing_region.is_some());
            #[cfg(feature = "scene-probe")]
            {
                esp_println::println!("peak frame={} work_us={} scenery_us={}", frame, peak_work_us, peak_scenery_us);
                peak_work_us = 0;
                peak_scenery_us = 0;
            }
            phase_total = [0; 12];
            steps_total = 0;
            window_started = finished;
        }
        #[cfg(feature = "native-airframe-probe")]
        if let Some((frame, pixels)) = display.packed_capture().and_then(|capture| capture.take()) {
            probe::emit_native_aircraft(frame, pixels);
        }
        #[cfg(feature = "burst-capture")]
        if burst_complete {
            burst.emit();
        }
    }
}

#[cfg(not(feature = "scene-probe"))]
fn create_game() -> Game {
    let rng = esp_hal::rng::Rng::new();
    Game::new((u64::from(rng.random()) << 32) | u64::from(rng.random()))
}

#[cfg(feature = "scene-probe")]
fn create_game() -> Game {
    probe::initial_game()
}

#[cfg(all(feature = "urban-expansion", not(feature = "urban-background")))]
#[inline(never)]
fn prepare_urban_launch(
    urban: &mut scenic_flight::land_stream::UrbanStream,
    land: &scenic_flight::land_stream::LandStream,
    facilities: &scenic_flight::facility_stream::FacilityStream<8>,
    rivers: &RiverStream<3>,
    routes: &LocalRouteStream<8>,
    highways: &HighwayStream<4>,
    rail: &RailStream,
    natural: &NaturalStream,
    summits: &SummitStream,
    coast: &scenic_flight::coastal_landforms::CoastalLandmarks,
    shore: &shore::Shore,
    world: &scenic_flight::world::World,
    seed: u64,
) {
    use scenic_flight::land_stream::{UrbanAuthorities, UrbanPublication};
    let started = micros();
    urban.reset();
    let authorities = UrbanAuthorities {
        facilities, rivers, routes, highways, rail, natural, summits: Some(summits),
    };
    let mut count = 0_u8;
    for ((x, z), _) in land.districts() {
        let Some(mut mask) = authorities.start([x, z], 0) else { continue };
        let Some(job) = urban.claim(land, [x, z], 0) else { continue };
        if let Some(grain) = facilities.planned((x, z)).and_then(|facility| facility.rail())
            .and_then(|rail| rail.grain_terminal())
        {
            let mut fields = scenic_flight::ground_details::GroundDetails::new();
            fields.reset(world, seed, grain.origin[0], grain.origin[2]);
            for bounds in fields.grain_field_bounds(grain.origin) {
                mask.reserve_area(bounds);
            }
        }
        for form in coast.forms().filter(|form| facilities.retains_landmark(form)) {
            mask.reserve_area(form.bounds());
        }
        for light in shore.lights().filter(|light| facilities.retains_lighthouse(light)) {
            mask.reserve_area(light.bounds());
        }
        authorities.advance(&mut mask, usize::MAX, |_| false);
        let result = job.prepare_reserved(&mask, || {}).unwrap();
        if urban.commit_reserved(land, result, &mask, None) == UrbanPublication::Published
            && urban.approved(land, [x, z]).is_some_and(|plan| plan.bounds().is_some())
        {
            count += 1;
        }
    }
    esp_println::println!("urban launched regions={} survey_us={}", count, micros() - started);
}
