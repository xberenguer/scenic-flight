//! Survey incoming districts on the display worker and validate reservations on the main core.

use scenic_flight::{
    coastal_landforms::CoastalLandmarks,
    ground_details::GroundDetails,
    land_stream::{LandStream, UrbanAuthorities, UrbanExpansionResult, UrbanPublication, UrbanReservations, UrbanStream},
    render::Camera,
    world::World,
};

use crate::{Display, shore::Shore, surveys::Survey};

pub struct Context<'a> {
    pub stream: &'a mut UrbanStream,
    pub background: &'a mut Background,
    pub coast: &'a CoastalLandmarks,
    pub shore: &'a Shore,
    pub seed: u64,
    pub frame: u32,
}

enum Reply {
    Ready(UrbanExpansionResult),
    Deferred { region: [i32; 2], retry_until: u32 },
}

pub struct Background {
    reply: Option<Reply>,
    next_attempt: u32,
    recently_deferred: Option<([i32; 2], u32)>,
}

impl Background {
    pub const fn new() -> Self {
        Self { reply: None, next_attempt: 0, recently_deferred: None }
    }

    pub fn reset(&mut self, stream: &mut UrbanStream) {
        self.reply = None;
        self.next_attempt = 0;
        self.recently_deferred = None;
        stream.reset();
    }
}

impl Context<'_> {
    pub fn receive(&mut self, result: Option<UrbanExpansionResult>) {
        self.background.reply = result.map(Reply::Ready);
        if self.background.reply.is_none() {
            self.stream.abort();
        }
    }

    pub fn service(
        &mut self,
        display: &mut Display,
        land: &LandStream,
        world: &World,
        camera: Camera,
        authorities: &UrbanAuthorities<'_>,
    ) {
        if self.frame < self.background.next_attempt {
            return;
        }
        let started = crate::micros();
        if let Some(reply) = self.background.reply.take() {
            self.background.next_attempt = self.frame + 16;
            let (region, retry_until) = match &reply {
                Reply::Ready(result) => (result.region(), self.frame.saturating_add(240)),
                Reply::Deferred { region, retry_until } => (*region, *retry_until),
            };
            if self.stream.pending_region() != Some(region) {
                return;
            }
            if self.frame >= retry_until {
                self.stream.abort();
                self.background.recently_deferred = Some((region, self.frame.saturating_add(600)));
                return;
            }
            let Some(mask) = self.mask(region, world, authorities) else {
                self.stream.abort();
                return;
            };
            let state = match reply {
                Reply::Ready(result) => self.stream.commit_reserved(land, result, &mask, Some(camera)),
                Reply::Deferred { .. } => self.stream.publish_reserved(land, 0, Some(camera), &mask),
            };
            match state {
                UrbanPublication::Deferred => {
                    self.background.reply = Some(Reply::Deferred { region, retry_until });
                }
                UrbanPublication::Published => {
                    // Allow the next road and its foundations to finish before
                    // another large expansion competes for the shared worker.
                    self.background.next_attempt = self.background.next_attempt.max(240);
                    #[cfg(feature = "scene-probe")]
                    if let Some(plan) = self.stream.approved(land, region) {
                        esp_println::println!(
                            "urban background published frame={} region={:?} skyline={} validation_us={}",
                            self.frame, region, plan.skyline().count(), crate::micros() - started,
                        );
                    }
                }
                _ => {}
            }
            return;
        }

        if !display.survey_ready() {
            return;
        }
        self.background.next_attempt = self.frame + 16;
        let Some(region) = self.stream.upcoming_region(land, camera, |key| {
            authorities.ready(key) && self.background.recently_deferred
                .is_none_or(|(recent, until)| recent != key || self.frame >= until)
        }) else {
            return;
        };
        let Some(mask) = self.mask(region, world, authorities) else { return; };
        let Some(job) = self.stream.claim(land, region, 0) else { return; };
        #[cfg(feature = "scene-probe")]
        esp_println::println!(
            "urban background queued frame={} region={:?} mask_us={}",
            self.frame, region, crate::micros() - started,
        );
        #[cfg(not(feature = "scene-probe"))]
        let _ = started;
        display.survey(Survey::Urban(job, mask));
    }

    #[inline(never)]
    fn mask(
        &self,
        [x, z]: [i32; 2],
        world: &World,
        authorities: &UrbanAuthorities<'_>,
    ) -> Option<UrbanReservations> {
        let mut mask = authorities.start([x, z], 0)?;
        let facilities = authorities.facilities;
        if let Some(grain) = facilities.planned((x, z))
            .and_then(|facility| facility.rail())
            .and_then(|rail| rail.grain_terminal())
        {
            for bounds in GroundDetails::survey_grain_fields(world, self.seed, grain.origin) {
                mask.reserve_area(bounds);
            }
        }
        for form in self.coast.forms().filter(|form| facilities.retains_landmark(form)) {
            mask.reserve_area(form.bounds());
        }
        for light in self.shore.lights().filter(|light| facilities.retains_lighthouse(light)) {
            mask.reserve_area(light.bounds());
        }
        authorities.complete(&mut mask);
        Some(mask)
    }
}
