//! Device-side fixture replay. The interactive firmware never enters this path.

use esp_hal::delay::Delay;
use scenic_flight::terrain_cache::TerrainCache;

use crate::{
    PERIOD_US,
    details::DetailLayers,
    display::{Display, Frame},
    fixtures::FIXTURES,
    micros,
};

pub fn run(
    display: &mut Display,
    mut image: Frame,
    terrain: &mut TerrainCache,
    work: &mut [u16],
    details: &mut DetailLayers,
) -> ! {
    let delay = Delay::new();
    loop {
        for (id, fixture) in FIXTURES.into_iter().enumerate() {
            let world = fixture.world();
            let setup = micros();
            let district = fixture.prepare_district(&world);
            let terrain_at = micros();
            terrain.reset(world, fixture.camera.x, fixture.camera.z);
            let detail_at = micros();
            details.reset(&world, fixture.seed, fixture.camera.x, fixture.camera.z);
            let ready = micros();
            let buildings: usize = district.as_ref().map_or(0, |region| {
                region
                    .towns()
                    .map(|town| usize::from(town.building_count()))
                    .sum()
            });
            esp_println::println!(
                "fixture={} seed={:016x} build_us={} terrain_reset_us={} detail_reset_us={} buildings={}",
                id,
                fixture.seed,
                terrain_at - setup,
                detail_at - terrain_at,
                ready - detail_at,
                buildings,
            );

            let mut window_started = micros();
            let mut timings = [0; 3];
            for frame in 1..=120 {
                let started = micros();
                fixture
                    .render(terrain, Some(details), district.as_ref(), image, work)
                    .unwrap();
                let rendered = micros();
                let report = frame == 1 || frame % 60 == 0;
                let fingerprint = if report {
                    crate::frame_fingerprint(image)
                } else {
                    0
                };
                let submitted = micros();
                let result = display.present(image, || {});
                image = result.image;
                timings[0] += rendered - started;
                timings[1] += result.transfer_us;
                timings[2] += micros() - submitted;

                while micros() - started < PERIOD_US {
                    delay.delay_micros(80);
                }
                if report {
                    let count = if frame == 1 {
                        1
                    } else if frame == 60 {
                        59
                    } else {
                        60
                    };
                    let now = micros();
                    esp_println::println!(
                        "fixture_frame={} frame={} render_us={} panel_us={} present_us={} window_us={} panel_bytes={} rgb565={:08x}",
                        id,
                        frame,
                        timings[0] / count,
                        timings[1] / count,
                        timings[2] / count,
                        now - window_started,
                        result.bytes,
                        fingerprint,
                    );
                    window_started = now;
                    timings = [0; 3];
                }
            }
        }
    }
}
