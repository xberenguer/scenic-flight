//! Mount both city packs and the regional elevation tiles from the same volume.

use scenic_flight::{
    LasVegasData, MappedCity,
    native_pack::{NativeFile, NativeFileDetails},
    render::Camera,
    world::{
        AtlasMaps, AtlasRoads, AtlasTowns, LAS_VEGAS_ATLAS_ORIGIN,
        atlas_ground_tiles::{GroundTileFile, TILE_CELLS, TILE_PLANE_BYTES},
    },
};

use super::{
    DetailCache, DetailPool, Prefetch, SLOTS, SceneMemory, inflate, micros, open_volume,
    atlas_elevation::{CorridorGround, CorridorPrefetch, CorridorScratch, RESIDENT_CELLS},
    vegas_storage::FileIndex,
};
use miniz_oxide::inflate::core::DecompressorOxide;

const OVERVIEW: &[u8] = include_bytes!(env!("SCENIC_ATLAS_OVERVIEW"));
const ROADS: &[u8] = include_bytes!(env!("SCENIC_ATLAS_ROADS"));
const TOWNS: &[u8] = include_bytes!(env!("SCENIC_ATLAS_TOWNS"));

struct CityFile {
    index: FileIndex<'static>,
    details: NativeFileDetails<'static>,
    data: &'static LasVegasData,
    cache: &'static DetailCache,
    prefetch: Prefetch,
}

pub fn prepare(
    memory: &mut SceneMemory,
    flash: esp_hal::peripherals::FLASH<'static>,
    decoder: &'static mut DecompressorOxide,
) -> (&'static AtlasMaps, impl FnMut(Camera, usize) -> usize + use<>) {
    let start = micros();
    let sector = memory.place(|| [0; 4096]);
    let mut volume = open_volume(flash, sector);
    let entries = [
        volume.los_angeles_file(sector).expect("Missing /scenic/losangel.pak"),
        volume.vegas_file(sector).expect("Missing /scenic/vegas.pak"),
    ];
    let indices = entries.map(|file| {
        let count = volume.index_len(file);
        volume.index_file(file, memory.place_pixels(count)).expect("City cluster chain")
    });
    let files: [NativeFile; 2] = core::array::from_fn(|city| {
        NativeFile::open(entries[city].len, |offset, data| volume.read_at(&indices[city], offset, data).is_ok())
            .expect("City map header")
    });
    let ground_slab = memory.place_bytes(files.iter().map(NativeFile::ground_scratch_len).max().unwrap());
    let ground_input = memory.place_bytes(files.iter().map(NativeFile::ground_input_len).max().unwrap().max(1));
    let mut ground = files.each_ref().map(|file| memory.place_bytes(file.ground_len()));
    let details: [NativeFileDetails<'static>; 2] = core::array::from_fn(|city| {
        let directory = memory.place_bytes(files[city].directory_len());
        files[city].details(directory, |offset, data| volume.read_at(&indices[city], offset, data).is_ok())
            .expect("City detail directory")
    });
    for (city, out) in ground.iter_mut().enumerate() {
        files[city].decode_ground(
            out, ground_slab, ground_input,
            |offset, data| volume.read_at(&indices[city], offset, data).is_ok(),
            &mut |input, output| inflate(decoder, input, output),
        ).expect("City elevation and water");
    }
    // The immutable city elevations stay resident while detail tiles share one pool.
    let stride = details.iter().map(NativeFileDetails::max_chunk_len).max().unwrap();
    let city_input = details.iter().map(NativeFileDetails::max_input_len).max().unwrap();
    let pool = DetailPool::new(memory, stride);
    let mut mounts = indices.into_iter().zip(details).zip(ground);
    let mut cities: [CityFile; 2] = core::array::from_fn(|city| {
        let ((index, details), ground) = mounts.next().unwrap();
        let cache = memory.place(|| DetailCache::new(details.layout(), city as u8, pool));
        let data = memory.place(|| LasVegasData::from_native(ground, cache).expect("City elevation raster"));
        assert_eq!(data.city(), [MappedCity::LosAngeles, MappedCity::LasVegas][city]);
        CityFile { index, details, data, cache, prefetch: Prefetch::new() }
    });

    let corridor = volume.southwest_file(sector).expect("Missing /scenic/southwst.pak");
    let corridor_index = volume.index_file(corridor, memory.place_pixels(volume.index_len(corridor)))
        .expect("Southwest cluster chain");
    let mut read_corridor = |offset, bytes: &mut [u8]| volume.read_at(&corridor_index, offset, bytes).is_ok();
    let tile_file = GroundTileFile::open(corridor.len, &mut read_corridor).expect("Southwest elevation header");
    let directory = memory.place_bytes(tile_file.directory_len());
    let tiles = tile_file.read_directory(directory, &mut read_corridor).expect("Southwest elevation directory");
    let resident = memory.place_pixels(RESIDENT_CELLS);
    let regional_ground: &'static CorridorGround = memory.place(|| CorridorGround::new(tiles.layout(), OVERVIEW, resident)
        .expect("Matching Southwest overview"));
    let [la, vegas] = cities.each_ref().map(|city| city.data);
    let maps = memory.place(|| AtlasMaps::new(la, vegas, None)
        .with_streamed_ground(regional_ground)
        .with_roads(Some(AtlasRoads::parse(ROADS).expect("Southwest highway layer")))
        .with_towns(AtlasTowns::parse(TOWNS).expect("Southwest town layer")));

    // Reuse the ground decoder's boot buffers in the frame loop.
    let input_len = city_input.max(tiles.max_input_len());
    let encoded = if ground_input.len() >= input_len { ground_input } else { memory.place_bytes(input_len) };
    let (planes, rest) = ground_slab.split_at_mut(TILE_PLANE_BYTES);
    let city_out = &mut rest[..stride];
    let heights = memory.place_heights(TILE_CELLS);
    let mut scratch = CorridorScratch { encoded, planes, heights };
    let mut ground_prefetch = CorridorPrefetch::new();
    esp_println::println!(
        "Southwest atlas mounted la_bytes={} vegas_bytes={} elevation_bytes={} psram_free={} stage_us={}",
        entries[0].len, entries[1].len, corridor.len, memory.remaining(), micros() - start,
    );

    let prefetch = move |mut camera: Camera, budget: usize| {
        let regional_position = [camera.x, camera.z];
        let [east, north] = LAS_VEGAS_ATLAS_ORIGIN;
        let city = usize::from(camera.x * east + camera.z * north > (east * east + north * north) * 0.5);
        if city == 1 {
            camera.x -= east;
            camera.z -= north;
        }
        let mount = &mut cities[city];
        let count = mount.prefetch.fill(mount.cache, camera, city_out, budget, &mut |index, out| {
            mount.details.decode_chunk(
                index, out, scratch.encoded,
                |offset, bytes| volume.read_at(&mount.index, offset, bytes).is_ok(),
                &mut |input, output| inflate(decoder, input, output),
            ).expect("Verified city detail tile").bytes().len()
        });
        let regional_budget = if budget >= SLOTS { 25 } else { budget.saturating_sub(count) };
        let regional = ground_prefetch.fill(
            regional_ground, &tiles, regional_position, regional_budget, &mut scratch,
            &mut |offset, bytes| volume.read_at(&corridor_index, offset, bytes).is_ok(),
            &mut |input, output| inflate(decoder, input, output),
        ).expect("Verified Southwest elevation tile");
        count + regional
    };
    (maps, prefetch)
}
