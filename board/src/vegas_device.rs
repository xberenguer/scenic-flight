#![no_std]
#![no_main]

#[cfg(feature = "native-southwest-file")]
mod atlas_device;
#[cfg(feature = "native-southwest-file")]
mod atlas_elevation;
mod bands;
#[cfg(feature = "panel-worker")]
mod button_capture;
mod display;
#[cfg(not(feature = "native-vegas-embedded"))]
mod flash32;
mod panel;
mod scene_memory;
#[cfg(not(feature = "native-vegas-embedded"))]
mod vegas_storage;

use core::cell::RefCell;
use critical_section::Mutex;
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
use miniz_oxide::inflate::{
    TINFLStatus,
    core::{DecompressorOxide, decompress, inflate_flags::*},
};
use scene_memory::SceneMemory;
use scenic_flight::{
    DowntownLandmarks, LasVegasData, StripLandmarks,
    aircraft_art::{draw_flight_scene, draw_preview},
    controls::{Buttons, Screen},
    flight::{Game, TICKS_PER_SECOND, View},
    horizon_cache::HorizonCache,
    native_pack::{NativeDetail, NativeDetailLayout},
    render::{Camera, render_terrain_layer_region_with_horizon},
    sky::Sky,
    ui::{draw_flight_info, draw_new_world, draw_picker},
    world_mesh::WorldMesh,
};
use static_cell::ConstStaticCell;
#[cfg(all(not(feature = "native-vegas-embedded"), not(feature = "native-southwest-file")))]
use scenic_flight::native_pack::NativeFile;
#[cfg(feature = "native-vegas-embedded")]
use scenic_flight::native_pack::{NativeDetails, NativePack};

use LOGICAL_SIDE as SIDE;
const LOGICAL_SIDE: usize = 112;
const PIXELS: usize = SIDE * SIDE;
const PERIOD: u64 = 1_000_000 / TICKS_PER_SECOND as u64;
const WINDOW: usize = if cfg!(feature = "native-southwest-file") { 7 } else { 13 };
const SLOTS: usize = WINDOW * WINDOW;
const PREFETCH_ORDER: [[i8; 2]; SLOTS] = prefetch_offsets();
static IMAGE: ConstStaticCell<[u16; PIXELS]> = ConstStaticCell::new([0; PIXELS]);
#[cfg(feature = "panel-worker")]
static SPARE_IMAGE: ConstStaticCell<[u16; PIXELS]> = ConstStaticCell::new([0; PIXELS]);

esp_bootloader_esp_idf::esp_app_desc!();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    esp_println::println!("vegas panic: {}", info);
    loop {
        core::hint::spin_loop();
    }
}

fn micros() -> u64 {
    Instant::now().duration_since_epoch().as_micros()
}

fn inflate(state: &mut DecompressorOxide, encoded: &[u8], output: &mut [u8]) -> bool {
    state.init();
    let (status, read, written) = decompress(
        state,
        encoded,
        output,
        0,
        TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
    );
    status == TINFLStatus::Done && read == encoded.len() && written == output.len()
}

#[cfg(not(feature = "native-vegas-embedded"))]
#[inline(never)]
fn open_volume(
    flash: esp_hal::peripherals::FLASH<'static>,
    scratch: &mut [u8; 4096],
) -> vegas_storage::VegasVolume<impl FnMut(usize, &mut [u8]) -> bool + use<>> {
    use esp_storage::FlashStorage;
    use vegas_storage::VegasVolume;

    let mut flash = FlashStorage::new(flash);
    flash
        .read(0x8000, scratch)
        .expect("Vegas partition table read");
    let mut wide_read_probe = [0; 32];
    assert!(flash32::read(0x8000, &mut wide_read_probe) && wide_read_probe == scratch[..32],
        "The four-byte flash reader did not match the partition table");
    let partition = scratch
        .chunks_exact(32)
        .take_while(|entry| entry[0..2] == [0xaa, 0x50])
        .find(|entry| entry[2..4] == [1, 0x81] && &entry[12..20] == b"storage\0")
        .expect("Vegas storage partition missing");
    let base = u32::from_le_bytes(partition[4..8].try_into().unwrap());
    let size = u32::from_le_bytes(partition[8..12].try_into().unwrap());
    assert_eq!(base, 0x510000);
    assert_eq!(size, 0x1af0000);
    assert!(flash.capacity() >= (base + size) as usize);
    VegasVolume::open(
        move |offset, data| {
            let address = base + offset as u32;
            // The upper half of this device's flash needs an explicit four-byte read.
            if address as usize + data.len() > 0x100_0000 {
                flash32::read(address, data)
            } else {
                flash.read(address, data).is_ok()
            }
        },
        size as usize,
        scratch,
    )
    .expect("Open Waveshare map storage")
}

#[cfg(all(not(feature = "native-vegas-embedded"), not(feature = "native-southwest-file")))]
#[inline(never)]
fn open_map(
    memory: &mut SceneMemory,
    flash: esp_hal::peripherals::FLASH<'static>,
) -> (usize, impl FnMut(usize, &mut [u8]) -> bool + use<>) {
    let scratch = memory.place(|| [0u8; 4096]);
    let mut volume = open_volume(flash, scratch);
    #[cfg(feature = "native-los-angeles-file")]
    let file = volume.los_angeles_file(scratch).expect("/scenic/losangel.pak missing");
    #[cfg(not(feature = "native-los-angeles-file"))]
    let file = volume.vegas_file(scratch).expect("/scenic/vegas.pak missing");
    assert!((80..=24 * 1024 * 1024).contains(&file.len));
    let index = volume.index_file(file, memory.place_pixels(volume.index_len(file))).expect("Map file cluster chain");
    (file.len, move |offset, data: &mut [u8]| volume.read_at(&index, offset, data).is_ok())
}

#[derive(Clone, Copy)]
struct Slot {
    chunk: u16,
    bytes: u16,
}

struct Residents {
    data: &'static mut [u8],
    city: u8,
    slots: [Slot; SLOTS],
}

const EMPTY_SLOT: Slot = Slot { chunk: u16::MAX, bytes: 0 };

struct DetailPool {
    stride: usize,
    residents: Mutex<RefCell<Residents>>,
}

impl DetailPool {
    fn new(memory: &mut SceneMemory, stride: usize) -> &'static Self {
        assert!(stride <= usize::from(u16::MAX));
        let data = memory.place_bytes(stride * SLOTS);
        memory.place(|| Self {
            stride,
            residents: Mutex::new(RefCell::new(Residents { data, city: u8::MAX, slots: [EMPTY_SLOT; SLOTS] })),
        })
    }
}

struct DetailCache {
    layout: NativeDetailLayout,
    city: u8,
    pool: &'static DetailPool,
}

impl DetailCache {
    fn new(layout: NativeDetailLayout, city: u8, pool: &'static DetailPool) -> Self {
        assert!(layout.chunk_count() < usize::from(u16::MAX));
        Self { layout, city, pool }
    }

    fn slot(&self, chunk: usize) -> usize {
        let [cols, _] = self.layout.grid_size();
        chunk / cols % WINDOW * WINDOW + chunk % cols % WINDOW
    }

    fn contains(&self, chunk: usize) -> bool {
        let slot = self.slot(chunk);
        critical_section::with(|cs| {
            let resident = self.pool.residents.borrow_ref(cs);
            resident.city == self.city && resident.slots[slot].chunk as usize == chunk
        })
    }

    fn insert(&self, chunk: usize, bytes: &[u8]) {
        let slot = self.slot(chunk);
        critical_section::with(|cs| {
            let mut resident = self.pool.residents.borrow_ref_mut(cs);
            if resident.city != self.city {
                resident.city = self.city;
                resident.slots.fill(EMPTY_SLOT);
            }
            let start = slot * self.pool.stride;
            resident.data[start..start + bytes.len()].copy_from_slice(bytes);
            resident.slots[slot] = Slot {
                chunk: chunk as u16,
                bytes: bytes.len() as u16,
            };
        });
    }

    fn center(&self, x: f32, z: f32) -> [i32; 2] {
        let [west, south] = self.layout.origin();
        let scale = self.layout.block_metres() as f32;
        core::array::from_fn(|axis| {
            let value = ([x - west as f32, z - south as f32][axis]) / scale;
            let whole = value as i32;
            whole - i32::from(value < whole as f32)
        })
    }
}

impl NativeDetail for DetailCache {
    fn detail_word(&self, tile_col: usize, tile_row: usize, cell: usize) -> u16 {
        let Some((chunk, tile)) = self.layout.locate(tile_col, tile_row) else {
            return 0;
        };
        let slot = self.slot(chunk);
        critical_section::with(|cs| {
            let resident = self.pool.residents.borrow_ref(cs);
            let meta = resident.slots[slot];
            if resident.city != self.city || meta.chunk as usize != chunk {
                return 0;
            }
            let start = slot * self.pool.stride;
            self.layout.word_in_chunk(
                &resident.data[start..start + meta.bytes as usize],
                tile,
                cell,
            )
        })
    }
}

const fn prefetch_offsets() -> [[i8; 2]; SLOTS] {
    let mut offsets = [[0; 2]; SLOTS];
    let mut cursor = 0;
    let mut ring = 0i8;
    while ring <= (WINDOW / 2) as i8 {
        let mut z = -ring;
        while z <= ring {
            let mut x = -ring;
            while x <= ring {
                if x == -ring || x == ring || z == -ring || z == ring {
                    offsets[cursor] = [x, z];
                    cursor += 1;
                }
                x += 1;
            }
            z += 1;
        }
        ring += 1;
    }
    offsets
}

struct Prefetch {
    center: [i32; 2],
    cursor: usize,
}

impl Prefetch {
    fn new() -> Self {
        Self { center: [i32::MIN; 2], cursor: 0 }
    }

    fn fill(
        &mut self,
        cache: &DetailCache,
        camera: Camera,
        scratch: &mut [u8],
        budget: usize,
        decode: &mut impl FnMut(usize, &mut [u8]) -> usize,
    ) -> usize {
        let center = cache.center(camera.x, camera.z);
        if center != self.center {
            self.center = center;
            self.cursor = 0;
        }
        let [cols, rows] = cache.layout.grid_size();
        let mut loaded = 0;
        while self.cursor < SLOTS && loaded < budget {
            let [dx, dz] = PREFETCH_ORDER[self.cursor];
            self.cursor += 1;
            let x = (center[0] + i32::from(dx)) as usize;
            let z = (center[1] + i32::from(dz)) as usize;
            if x < cols && z < rows {
                let chunk = z * cols + x;
                if cache.contains(chunk) {
                    continue;
                }
                let len = decode(chunk, scratch);
                cache.insert(chunk, &scratch[..len]);
                loaded += 1;
            }
        }
        loaded
    }
}

#[inline(never)]
#[cfg(feature = "native-vegas-embedded")]
fn prepare_map(
    memory: &mut SceneMemory,
    bytes: &'static [u8],
    state: &mut DecompressorOxide,
) -> (
    &'static LasVegasData,
    NativeDetails<'static>,
    &'static DetailCache,
    &'static mut [u8],
) {
    let start = micros();
    let pack = NativePack::parse(bytes).expect("SCG1 map header");
    let decoded = memory.place_bytes(pack.ground_len());
    let scratch = memory.place_bytes(pack.ground_scratch_len().max(1));
    pack.decode_ground(decoded, scratch, &mut |a, b| inflate(state, a, b))
        .expect("Decode Vegas DEM and water");
    let source = pack.details();
    let stride = source.max_chunk_len();
    let pool = DetailPool::new(memory, stride);
    let cache = memory.place(|| DetailCache::new(source.layout(), 0, pool));
    let vegas = memory.place(|| LasVegasData::from_native(decoded, cache).expect("Vegas DEM"));
    let source = if let Some(details) = memory.try_place_bytes(source.storage().len()) {
        details.copy_from_slice(source.storage());
        esp_println::println!("mapped detail storage=psram");
        NativeDetails::from_storage(details, source.layout()).expect("Relocate Vegas details")
    } else {
        esp_println::println!("mapped detail storage=flash");
        source
    };
    esp_println::println!(
        "vegas map bytes={} ground={} chunks={} slot_bytes={} stage_us={}",
        bytes.len(),
        pack.ground_len(),
        source.layout().chunk_count(),
        stride,
        micros() - start,
    );
    (vegas, source, cache, scratch)
}

#[cfg(all(not(feature = "native-vegas-embedded"), not(feature = "native-southwest-file")))]
#[inline(never)]
fn prepare_file_map(
    memory: &mut SceneMemory,
    flash: esp_hal::peripherals::FLASH<'static>,
    state: &'static mut DecompressorOxide,
) -> (
    &'static LasVegasData,
    &'static DetailCache,
    &'static mut [u8],
    impl FnMut(usize, &mut [u8]) -> usize + use<>,
) {
    let start = micros();
    let (len, mut read) = open_map(memory, flash);
    let file = NativeFile::open(len, &mut read).expect("Map file header");
    let ground = memory.place_bytes(file.ground_len());
    let slab = memory.place_bytes(file.ground_scratch_len());
    let input = memory.place_bytes(file.ground_input_len().max(1));
    file.decode_ground(ground, slab, input, &mut read, &mut |a, b| inflate(state, a, b))
        .expect("Decode map file ground");
    let directory = memory.place_bytes(file.directory_len());
    let details = file.details(directory, &mut read).expect("Map chunk directory");
    let stride = details.max_chunk_len();
    let pool = DetailPool::new(memory, stride);
    let cache = memory.place(|| DetailCache::new(details.layout(), 0, pool));
    let map = memory.place(|| LasVegasData::from_native(ground, cache).expect("Map frame"));
    let decoded = memory.place_bytes(stride);
    let encoded = memory.place_bytes(details.max_input_len());
    esp_println::println!("mapped detail storage=fat bytes={} chunks={} stage_us={}",
        len, details.layout().chunk_count(), micros() - start);
    let decode = move |index, out: &mut [u8]| {
        details.decode_chunk(index, out, encoded, &mut read, &mut |a, b| inflate(state, a, b))
            .expect("Verified map chunk").bytes().len()
    };
    (map, cache, decoded, decode)
}

struct Quality {
    level: usize,
    slow: u8,
    fast: u8,
    average_cpu: u64,
    average_scene: u64,
    average_panel: u64,
}

impl Quality {
    const LEVELS: [usize; 11] = [36, 42, 48, 54, 60, 66, 72, 80, 88, 100, SIDE];

    fn new() -> Self {
        Self {
            level: 4,
            slow: 0,
            fast: 0,
            average_cpu: PERIOD,
            average_scene: 0,
            average_panel: 24_000,
        }
    }

    fn side(&self) -> usize {
        Self::LEVELS[self.level]
    }

    fn observe(&mut self, cpu: u64, scene: u64, panel: u64) {
        self.average_cpu = (self.average_cpu * 7 + cpu.min(90_000)) / 8;
        self.average_scene = (self.average_scene * 7 + scene.min(90_000)) / 8;
        if panel != 0 {
            self.average_panel = (self.average_panel * 7 + panel.min(90_000)) / 8;
        }
        self.slow = if self.critical_path(self.average_cpu) > 36_500 {
            self.slow.saturating_add(1)
        } else {
            0
        };
        if self.slow >= 12 && self.level > 0 {
            self.level -= 1;
            self.slow = 0;
            self.fast = 0;
            return;
        }
        if let Some(&higher) = Self::LEVELS.get(self.level + 1) {
            let higher = (higher * higher) as u64;
            let current = (self.side() * self.side()) as u64;
            let estimate = self.average_cpu + self.average_scene * (higher - current) / current;
            self.fast = if self.critical_path(estimate) < 33_000 {
                self.fast.saturating_add(1)
            } else {
                0
            };
            if self.fast >= 65 {
                self.level += 1;
                self.fast = 0;
            }
        }
    }

    fn critical_path(&self, cpu: u64) -> u64 {
        if cfg!(feature = "panel-worker") {
            cpu.max(self.average_panel)
        } else {
            cpu + self.average_panel
        }
    }
}

fn expand(frame: &[u16], logical: usize, output: &mut [u16]) {
    for y in 0..SIDE {
        let source_y = (2 * y + 1) * logical / (2 * SIDE);
        let d = (2 * source_y + 1) as i32 - logical as i32;
        let step = logical as i32;
        let inset = (0..logical / 2)
            .find(|&x| {
                let u = (2 * x + 1) as i32 - step;
                u * u + d * d <= step * step
            })
            .unwrap_or(logical / 2 - 1);
        for x in 0..SIDE {
            let source_x = (2 * x + 1) * logical / (2 * SIDE);
            output[y * SIDE + x] =
                frame[source_y * logical + source_x.clamp(inset, logical - 1 - inset)];
        }
    }
}

fn camera(v: View) -> Camera {
    Camera {
        x: v.x,
        z: v.z,
        altitude: v.altitude,
        heading: v.heading,
        pitch: v.pitch,
        bank: v.bank,
    }
}

#[derive(Default)]
struct CapturedControls {
    previous: Buttons,
    await_release: bool,
    poll_at: u64,
}

impl CapturedControls {
    fn apply(game: &mut Game, at: u32, buttons: Buttons) {
        if let Some(action) = game.read_buttons(at, buttons) {
            esp_println::println!("vegas button={} ms={}", action as u8, at);
        }
    }

    #[cfg(feature = "panel-worker")]
    fn poll(&mut self, game: &mut Game) {
        let now = micros();
        if now < self.poll_at {
            return;
        }
        self.poll_at = now + 2_000;
        let mut edges = [button_capture::Edge::UP; button_capture::MAX_EDGES];
        let (at, count, current, overflow) = button_capture::take(&mut edges);
        if overflow {
            game.reset_buttons();
            self.await_release = true;
        }
        if self.await_release {
            if current == Buttons::default() {
                self.previous = current;
                self.await_release = false;
            }
            return;
        }
        for edge in &edges[..count] {
            Self::apply(game, edge.at, self.previous);
            self.previous = edge.buttons;
            Self::apply(game, edge.at, self.previous);
        }
        Self::apply(game, at, current);
    }
}

#[esp_hal::main]
fn main() -> ! {
    esp_println::println!("Scenic Flight {} boot", env!("CARGO_PKG_VERSION"));
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    let mut memory = SceneMemory::new(peripherals.PSRAM);
    let small = Input::new(
        peripherals.GPIO0,
        InputConfig::default().with_pull(Pull::Up),
    );
    let large = Input::new(peripherals.GPIO3, InputConfig::default());
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
    let panel = panel::Panel::<SIDE>::new(spi, panel::pixel_buffers(), &mut reset).unwrap();
    #[cfg(feature = "native-vegas-embedded")]
    let bytes: &'static [u8] = include_bytes!(env!("SCENIC_VEGAS_PACK"));
    let decoder = memory.place(DecompressorOxide::new);
    let mut quality = Quality::new();
    let scenery = memory.place_pixels(PIXELS);
    let work = memory.place_pixels(2 * PIXELS);
    let depth = memory.place_pixels(PIXELS);
    let horizon = memory.place(HorizonCache::new);
    #[cfg(feature = "native-vegas-embedded")]
    let (vegas, source, cache, scratch) = prepare_map(&mut memory, bytes, decoder);
    #[cfg(feature = "native-vegas-embedded")]
    let mut decode = |index, out: &mut [u8]| source
        .decode_chunk(index, out, &mut |a, b| inflate(decoder, a, b))
        .expect("Verified embedded map chunk")
        .bytes()
        .len();
    #[cfg(all(not(feature = "native-vegas-embedded"), not(feature = "native-southwest-file")))]
    let (vegas, cache, scratch, mut decode) = prepare_file_map(&mut memory, peripherals.FLASH, decoder);
    #[cfg(not(feature = "native-southwest-file"))]
    let mut preload = {
        let mut prefetch = Prefetch::new();
        move |view, budget| prefetch.fill(cache, view, scratch, budget, &mut decode)
    };
    #[cfg(feature = "native-southwest-file")]
    let (atlas, mut preload) = atlas_device::prepare(&mut memory, peripherals.FLASH, decoder);
    let mut image = IMAGE.take();
    #[cfg(feature = "panel-worker")]
    let mut display = Display::start(
        panel,
        &mut esp_hal::system::CpuControl::new(peripherals.CPU_CTRL),
        SPARE_IMAGE.take(),
    );
    #[cfg(not(feature = "panel-worker"))]
    let mut display = Display::new(panel);
    #[cfg(feature = "panel-worker")]
    let mut controls = {
        let timers = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
        button_capture::start(
            esp_hal::timer::PeriodicTimer::new(timers.timer0),
            (small, large),
        );
        CapturedControls::default()
    };
    let mut poll = |game: &mut Game| {
        #[cfg(feature = "panel-worker")]
        controls.poll(game);
        #[cfg(not(feature = "panel-worker"))]
        CapturedControls::apply(
            game,
            (micros() / 1_000) as u32,
            Buttons {
                small: small.is_low(),
                large: large.is_high(),
            },
        );
    };
    esp_println::println!("vegas display worker={}", cfg!(feature = "panel-worker"));
    let seed = (micros() << 16) ^ 0xf24f_0a31_0bec_3f17;
    #[cfg(not(feature = "native-southwest-file"))]
    let departure = if cfg!(feature = "native-los-angeles-file") { 8 } else { 0 };
    #[cfg(not(feature = "native-southwest-file"))]
    let mut game = Game::new_mapped_city(seed, vegas, departure);
    #[cfg(not(feature = "native-southwest-file"))]
    esp_println::println!("mapped destination={:?}", vegas.city());
    #[cfg(feature = "native-southwest-file")]
    let mut game = {
        let start = match option_env!("SCENIC_ATLAS_START") {
            Some("vegas") => scenic_flight::MappedCity::LasVegas,
            _ => scenic_flight::MappedCity::LosAngeles,
        };
        esp_println::println!("mapped destination=Southwest start={:?}", start);
        Game::new_atlas(seed, atlas, start, 0, true).with_alternating_atlas_departures()
    };
    let mut shown_since = None;
    let mut selected = None;
    let mut menu_since = micros();
    let mut menu_drawn = 0;
    let mut active_seed = None;
    let mut tick_at = 0;
    let mut frame_at = 0;
    let mut frames = 0u32;
    let mut report_start = micros();
    let mut peak_work = 0;
    let delay = Delay::new();

    loop {
        poll(&mut game);
        let now = micros();
        if let Some(since) = shown_since {
            game.launch_if_boot_idle((now - since) / 1_000);
        }
        if game.screen() == Screen::AircraftPicker {
            active_seed = None;
            if selected != Some(game.selected()) {
                menu_since = now;
                selected = Some(game.selected());
            } else if now < menu_drawn + 50_000 {
                delay.delay_millis(1);
                continue;
            }
            draw_picker(game.selected(), image, SIDE, SIDE);
            draw_preview(
                game.selected(),
                ((now - menu_since) / PERIOD) as u32,
                image,
                SIDE,
                SIDE,
            );
            image = display.present(image, || poll(&mut game)).image;
            menu_drawn = micros();
            shown_since.get_or_insert(menu_drawn);
            continue;
        }
        selected = None;
        let seed = game.world_seed().unwrap();
        if active_seed != Some(seed) {
            let (world, flight) = game.active().unwrap();
            let view = camera(flight.view);
            // Warm the seven-tile city window before the first frame.
            preload(view, 49);
            horizon.reset(*world, view.x, view.z);
            let ready = micros();
            tick_at = ready + PERIOD;
            frame_at = ready;
            report_start = ready;
            frames = 0;
            peak_work = 0;
            active_seed = Some(seed);
            esp_println::println!(
                "vegas launch aircraft={} seed={:016x}",
                flight.aircraft as u8,
                seed
            );
        }
        if micros() < frame_at {
            delay.delay_millis(1);
            continue;
        }
        let start = micros();
        frame_at = start + PERIOD;
        let mut steps = 0;
        while start >= tick_at && steps < 4 {
            game.tick();
            tick_at += PERIOD;
            steps += 1;
        }
        if start >= tick_at {
            tick_at = start + PERIOD;
        }
        let (world, flight) = game.active().unwrap();
        let world = *world;
        let blend = 1.0 - (tick_at.saturating_sub(start) as f32 / PERIOD as f32).clamp(0.0, 1.0);
        let pose = flight.display_pose(blend);
        let view = camera(pose.view);
        let loaded = preload(view, 1);
        horizon.follow(view.x, view.z, 24);
        let side = quality.side();
        let count = side * side;
        let dims = (side, side);
        let sky = Sky::over_world(&world, seed, pose.ticks, [view.x, view.altitude, view.z]);
        let strip = StripLandmarks::from_world(&world, view, dims);
        let downtown = DowntownLandmarks::from_world(&world, view, dims);
        let region = strip
            .as_ref()
            .map(StripLandmarks::bounds)
            .or_else(|| downtown.as_ref().map(DowntownLandmarks::bounds));
        let scene_start = micros();
        render_terrain_layer_region_with_horizon(
            (
                |x, z| world.mapped_sample(x, z).unwrap_or_else(|| {
                    let ground = world.sample(x, z);
                    world.corridor_surface(x, z, ground).unwrap_or(ground)
                }),
                |_, _, sample| sample.surface,
            ),
            |x, z| horizon.backdrop(x, z),
            view,
            &sky,
            &mut scenery[..count],
            &mut work[..count * 2],
            region.map(|bounds| (&mut depth[..count], bounds)),
            dims,
            |_| {},
            |_| None,
        )
        .expect("Vegas render");
        if region.is_some() {
            let mut mesh =
                WorldMesh::new(&mut scenery[..count], &mut depth[..count], dims, view, &sky)
                    .unwrap();
            if let Some(strip) = strip {
                strip.draw_for_flight(&mut mesh, pose.ticks, seed);
            }
            if let Some(downtown) = downtown {
                downtown.draw(&mut mesh);
            }
        }
        let scene_us = micros() - scene_start;
        if side == SIDE {
            image.copy_from_slice(&scenery[..PIXELS]);
        } else {
            expand(scenery, side, image);
        }
        draw_flight_scene(&pose, seed, &sky, image, SIDE, SIDE);
        if game.flight_info_visible() {
            draw_flight_info(&pose, image, SIDE, SIDE);
        }
        if let Some(progress) = game.new_world_progress((micros() / 1_000) as u32) {
            draw_new_world(progress, image, SIDE, SIDE);
        }
        let submitted = micros();
        let presented = display.present(image, || poll(&mut game));
        let finished = micros();
        image = presented.image;
        let work_us = finished - start;
        let cpu_us = submitted - start;
        quality.observe(cpu_us, scene_us, presented.transfer_us);
        peak_work = peak_work.max(work_us);
        frames += 1;
        if frames.is_multiple_of(60) {
            let window = finished - report_start;
            let fps_milli = 60_000_000_000u64 / window.max(1);
            esp_println::println!(
                "vegas frame={} tick={} fps={}.{:03} scene={} panel=466 bytes={} work_us={} peak_us={} cpu_us={} scene_us={} transfer_us={} wait_us={} decode={} speed_mps={}",
                frames,
                pose.ticks,
                fps_milli / 1_000,
                fps_milli % 1_000,
                side,
                presented.bytes,
                work_us,
                peak_work,
                cpu_us,
                scene_us,
                presented.transfer_us,
                finished - submitted,
                loaded,
                pose.speed() as u32,
            );
            report_start = finished;
            peak_work = 0;
        }
    }
}
