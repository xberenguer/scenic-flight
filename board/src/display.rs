//! Transfer complete frames without sharing a mutable framebuffer between cores.

use crate::{LOGICAL_SIDE, micros, panel::Panel};

pub type Frame = &'static mut [u16; LOGICAL_SIDE * LOGICAL_SIDE];
type DevicePanel = Panel<LOGICAL_SIDE>;

pub struct Presented {
    pub image: Frame,
    pub bytes: u32,
    pub transfer_us: u64,
}

#[cfg(not(feature = "panel-worker"))]
pub struct Display(DevicePanel);

#[cfg(not(feature = "panel-worker"))]
impl Display {
    pub fn new(panel: DevicePanel) -> Self {
        Self(panel)
    }

    pub fn present(&mut self, image: Frame, poll: impl FnMut()) -> Presented {
        let started = micros();
        let bytes = self.0.present(image, poll).unwrap();
        Presented {
            image,
            bytes,
            transfer_us: micros() - started,
        }
    }
}

#[cfg(feature = "panel-worker")]
pub use background::Display;

#[cfg(feature = "panel-worker")]
mod background {
    use core::cell::RefCell;

    use critical_section::Mutex;
    use esp_hal::{
        delay::Delay,
        spi::Error,
        system::{AppCoreGuard, CpuControl, Stack},
    };
    #[cfg(not(feature = "transport-stream"))]
    use static_cell::StaticCell;
    #[cfg(feature = "transport-stream")]
    use crate::surveys::{Survey, SurveyResult, SurveySlots};
    #[cfg(feature = "native-flight-overlay")]
    use crate::native_flight::NativeFlight;
    #[cfg(feature = "native-airframe-probe")]
    use crate::packed_airframe::PackedCapture;
    #[cfg(feature = "river-stream")]
    use scenic_flight::watershed::WatershedWork;
    #[cfg(feature = "coastal-highways")]
    use scenic_flight::coastal_highways::CoastalHighwayWork;
    #[cfg(feature = "regional-rail")]
    use scenic_flight::{
        rail_stream::RailRequest,
        rail_survey::{RailSurveyData, RailSurveyWork},
    };

    use super::*;

    #[cfg(not(feature = "transport-stream"))]
    const STACK_BYTES: usize = 12_288;
    #[cfg(feature = "transport-stream")]
    const STACK_BYTES: usize = 65_536;
    #[cfg(not(feature = "transport-stream"))]
    static WORKER_STACK: StaticCell<Stack<STACK_BYTES>> = StaticCell::new();
    #[cfg(feature = "transport-stream")]
    #[esp_hal::ram(reclaimed)]
    static mut WORKER_STACK: core::mem::MaybeUninit<Stack<STACK_BYTES>> = core::mem::MaybeUninit::uninit();
    struct FrameTransfer {
        image: Frame,
        #[cfg(feature = "native-flight-overlay")]
        native: &'static mut NativeFlight<'static>,
        #[cfg(feature = "native-airframe-probe")]
        capture: Option<&'static mut PackedCapture<'static>>,
    }

    static WORK: Mutex<RefCell<Option<FrameTransfer>>> = Mutex::new(RefCell::new(None));
    static DONE: Mutex<RefCell<Option<(FrameTransfer, Result<u32, Error>, u64)>>> =
        Mutex::new(RefCell::new(None));
    #[cfg(feature = "transport-stream")]
    static SURVEY: Mutex<RefCell<Option<&'static mut SurveySlots>>> = Mutex::new(RefCell::new(None));
    #[cfg(feature = "transport-stream")]
    static SURVEY_DONE: Mutex<RefCell<Option<&'static mut SurveySlots>>> = Mutex::new(RefCell::new(None));

    #[cfg(feature = "parallel-terrain")]
    mod terrain_task {
        use core::{
            cell::UnsafeCell,
            sync::atomic::{AtomicBool, AtomicU32, Ordering},
        };

        use super::{micros, Delay, Mutex, RefCell};

        struct Packet<F> {
            operation: UnsafeCell<Option<F>>,
            done: AtomicBool,
            started: AtomicU32,
            finished: AtomicU32,
        }

        struct Job {
            packet: *const (),
            run: unsafe fn(*const ()),
        }

        // The only published values are packets holding an F: Send.
        // The sender waits for done before the packet or its borrows expire.
        unsafe impl Send for Job {}

        static PENDING: Mutex<RefCell<Option<Job>>> = Mutex::new(RefCell::new(None));

        unsafe fn run<F: FnOnce() + Send>(pointer: *const ()) {
            // The sender keeps this scoped packet alive until the release
            // store below. Only this worker touches the operation's cell.
            let packet = unsafe { &*pointer.cast::<Packet<F>>() };
            packet.started.store(micros() as u32, Ordering::Relaxed);
            let operation = unsafe { (&mut *packet.operation.get()).take().unwrap() };
            operation();
            packet.finished.store(micros() as u32, Ordering::Relaxed);
            packet.done.store(true, Ordering::Release);
        }

        pub(super) fn poll() -> bool {
            let job = critical_section::with(|cs| PENDING.borrow_ref_mut(cs).take());
            if let Some(job) = job {
                // Only fork can publish jobs, and every fork joins its worker.
                unsafe { (job.run)(job.packet) };
                true
            } else {
                false
            }
        }

        struct Join<'a>(&'a AtomicBool);

        impl Drop for Join<'_> {
            fn drop(&mut self) {
                let delay = Delay::new();
                while !self.0.load(Ordering::Acquire) {
                    delay.delay_micros(30);
                }
            }
        }

        #[derive(Clone, Copy)]
        pub struct Timing {
            pub queue_us: u32,
            pub left_us: u32,
            pub right_us: u32,
            pub join_us: u32,
            pub background: bool,
        }

        pub(super) fn fork<F: FnOnce() + Send>(left: impl FnOnce(), right: F) -> Timing {
            let packet = Packet {
                operation: UnsafeCell::new(Some(right)),
                done: AtomicBool::new(false),
                started: AtomicU32::new(0),
                finished: AtomicU32::new(0),
            };
            let submitted = micros() as u32;
            critical_section::with(|cs| {
                let mut pending = PENDING.borrow_ref_mut(cs);
                assert!(pending.is_none());
                *pending = Some(Job {
                    packet: core::ptr::addr_of!(packet).cast(),
                    run: run::<F>,
                });
            });
            let join = Join(&packet.done);
            left();
            let left_finished = micros() as u32;
            // If the right lane is still queued, the worker cannot be using
            // any of the slab.
            #[cfg(feature = "terrain-opportunistic")]
            let background = !poll();
            #[cfg(not(feature = "terrain-opportunistic"))]
            let background = true;
            drop(join);
            let completed = micros() as u32;
            let worker_started = packet.started.load(Ordering::Relaxed);
            let worker_finished = packet.finished.load(Ordering::Relaxed);
            Timing {
                queue_us: worker_started.wrapping_sub(submitted),
                left_us: left_finished.wrapping_sub(submitted),
                right_us: worker_finished.wrapping_sub(worker_started),
                join_us: completed.wrapping_sub(left_finished),
                background,
            }
        }
    }

    pub struct Display {
        _worker: AppCoreGuard<'static>,
        spare: Option<FrameTransfer>,
        #[cfg(feature = "native-flight-overlay")]
        current_native: Option<&'static mut NativeFlight<'static>>,
        #[cfg(feature = "native-airframe-probe")]
        current_capture: Option<&'static mut PackedCapture<'static>>,
        first: bool,
        bytes: u32,
        #[cfg(feature = "transport-stream")]
        survey_slots: Option<&'static mut SurveySlots>,
        #[cfg(feature = "parallel-terrain")]
        terrain_timing: Option<terrain_task::Timing>,
    }

    impl Display {
        #[inline(never)]
        pub fn start(
            panel: DevicePanel, cpu: &mut CpuControl<'_>, spare: Frame,
            #[cfg(feature = "native-flight-overlay")] native: [&'static mut NativeFlight<'static>; 2],
            #[cfg(feature = "native-airframe-probe")] capture: [Option<&'static mut PackedCapture<'static>>; 2],
            #[cfg(feature = "transport-stream")] survey_slots: &'static mut SurveySlots,
            #[cfg(feature = "river-stream")] water: &'static mut WatershedWork,
            #[cfg(feature = "coastal-highways")] coast: &'static mut CoastalHighwayWork,
            #[cfg(feature = "regional-rail")] rail_work: &'static mut RailSurveyWork,
        ) -> Self {
            #[cfg(not(feature = "transport-stream"))]
            let stack = WORKER_STACK.init_with(Stack::new);
            #[cfg(feature = "transport-stream")]
            let stack = {
                use core::sync::atomic::{AtomicBool, Ordering};
                static CLAIMED: AtomicBool = AtomicBool::new(false);
                assert!(!CLAIMED.swap(true, Ordering::AcqRel));
                // Internal RAM reclaimed from the bootloader. The caller can
                // claim this stack once, before starting the display core.
                unsafe { (&mut *core::ptr::addr_of_mut!(WORKER_STACK)).write(Stack::new()) }
            };
            let worker = cpu.start_app_core(stack, move || work(panel,
                #[cfg(feature = "river-stream")] water,
                #[cfg(feature = "coastal-highways")] coast,
                #[cfg(feature = "regional-rail")] rail_work,
            )).unwrap();
            #[cfg(feature = "native-flight-overlay")]
            let [current_native, spare_native] = native;
            #[cfg(feature = "native-airframe-probe")]
            let [current_capture, spare_capture] = capture;
            Self {
                _worker: worker,
                spare: Some(FrameTransfer {
                    image: spare,
                    #[cfg(feature = "native-flight-overlay")]
                    native: spare_native,
                    #[cfg(feature = "native-airframe-probe")]
                    capture: spare_capture,
                }),
                #[cfg(feature = "native-flight-overlay")]
                current_native: Some(current_native),
                #[cfg(feature = "native-airframe-probe")]
                current_capture,
                first: true,
                bytes: 0,
                #[cfg(feature = "transport-stream")]
                survey_slots: Some(survey_slots),
                #[cfg(feature = "parallel-terrain")]
                terrain_timing: None,
            }
        }

        #[cfg(feature = "transport-stream")]
        pub fn survey_ready(&self) -> bool { self.survey_slots.is_some() }

        #[cfg(feature = "parallel-terrain")]
        pub fn terrain_ready(&self) -> bool {
            self.survey_ready() || cfg!(feature = "terrain-opportunistic")
        }

        #[cfg(feature = "parallel-terrain")]
        pub fn take_terrain_timing(&mut self) -> Option<terrain_task::Timing> {
            self.terrain_timing.take()
        }

        #[cfg(feature = "transport-stream")]
        #[inline(never)]
        pub fn survey(&mut self, request: Survey) {
            let slots = self.survey_slots.take().expect("one active scenery survey");
            slots.request.write(request);
            critical_section::with(|cs| {
                *SURVEY.borrow_ref_mut(cs) = Some(slots);
            });
        }

        #[cfg(feature = "regional-rail")]
        #[inline(never)]
        pub fn survey_rail(
            &mut self,
            request: RailRequest,
            load: impl FnOnce(&mut RailSurveyData) -> bool,
        ) -> bool {
            let slots = self.survey_slots.take().expect("one active scenery survey");
            if !load(&mut slots.rail) {
                self.survey_slots = Some(slots);
                return false;
            }
            slots.request.write(Survey::RegionalRail(request));
            critical_section::with(|cs| {
                *SURVEY.borrow_ref_mut(cs) = Some(slots);
            });
            true
        }

        #[cfg(feature = "transport-stream")]
        #[inline(never)]
        pub fn collect_survey(&mut self) -> Option<SurveyResult> {
            if self.survey_ready() { return None; }
            let slots = critical_section::with(|cs| SURVEY_DONE.borrow_ref_mut(cs).take())?;
            // The worker only publishes a slot after writing its result.
            let result = unsafe { slots.result.assume_init_read() };
            self.survey_slots = Some(slots);
            Some(result)
        }

        #[cfg(feature = "cooperative-sky")]
        pub fn prepare_sky(
            &mut self, camera: scenic_flight::render::Camera, sky: scenic_flight::sky::Sky,
            dimensions: (usize, usize),
        ) -> crate::sky_worker::PreparedSky {
            crate::sky_worker::request(camera, sky, dimensions)
        }

        #[cfg(feature = "native-flight-overlay")]
        pub fn native_overlay(&mut self) -> &mut NativeFlight<'static> {
            self.current_native.as_deref_mut().unwrap()
        }

        #[cfg(feature = "native-airframe-probe")]
        pub fn packed_capture(&mut self) -> Option<&mut PackedCapture<'static>> {
            self.current_capture.as_deref_mut()
        }

        pub fn present(&mut self, image: Frame, mut poll: impl FnMut()) -> Presented {
            let transfer = FrameTransfer {
                image,
                #[cfg(feature = "native-flight-overlay")]
                native: self.current_native.take().unwrap(),
                #[cfg(feature = "native-airframe-probe")]
                capture: self.current_capture.take(),
            };
            let (free, bytes, transfer_us) = if self.first {
                self.first = false;
                submit(transfer);
                let result = collect(&mut poll);
                self.bytes = result.1;
                result
            } else {
                let free = self.spare.take().map_or_else(
                    || collect(&mut poll),
                    |frame| (frame, self.bytes, 0),
                );
                submit(transfer);
                free
            };
            #[cfg(feature = "native-flight-overlay")]
            { self.current_native = Some(free.native); }
            #[cfg(feature = "native-airframe-probe")]
            { self.current_capture = free.capture; }
            Presented { image: free.image, bytes, transfer_us }
        }
    }

    #[cfg(feature = "parallel-terrain")]
    impl scenic_flight::render::TerrainFork for Display {
        fn fork(&mut self, left: impl FnOnce(), right: impl FnOnce() + Send) {
            assert!(self.terrain_ready());
            self.terrain_timing = Some(terrain_task::fork(left, right));
        }
    }

    fn submit(frame: FrameTransfer) {
        critical_section::with(|cs| {
            let mut work = WORK.borrow_ref_mut(cs);
            assert!(work.is_none());
            *work = Some(frame);
        });
    }

    fn collect(poll: &mut impl FnMut()) -> (FrameTransfer, u32, u64) {
        let delay = Delay::new();
        loop {
            if let Some((frame, bytes, transfer_us)) =
                critical_section::with(|cs| DONE.borrow_ref_mut(cs).take())
            {
                return (frame, bytes.unwrap(), transfer_us);
            }
            poll();
            delay.delay_micros(40);
        }
    }

    struct Worker {
        panel: DevicePanel,
        #[cfg(feature = "cooperative-sky")]
        sky: crate::sky_worker::SkyWorker,
    }

    impl Worker {
        #[inline(never)]
        fn send_frame(&mut self) -> bool {
            let Some(frame) = critical_section::with(|cs| WORK.borrow_ref_mut(cs).take()) else {
                return false;
            };
            #[cfg(feature = "native-airframe-probe")]
            let mut frame = frame;
            let started = micros();
            let bytes = self.panel.present(
                frame.image,
                #[cfg(feature = "native-flight-overlay")]
                frame.native,
                #[cfg(feature = "native-airframe-probe")]
                frame.capture.as_deref_mut(),
                || {
                    #[cfg(feature = "packed-60-terrain")]
                    if terrain_task::poll() { return; }
                    #[cfg(feature = "cooperative-sky")]
                    let _ = self.sky.poll();
                },
            );
            let transfer_us = micros() - started;
            critical_section::with(|cs| {
                let mut done = DONE.borrow_ref_mut(cs);
                assert!(done.is_none());
                *done = Some((frame, bytes, transfer_us));
            });
            true
        }

        #[cfg(feature = "transport-stream")]
        #[inline(never)]
        fn survey_next(
            &mut self,
            #[cfg(feature = "river-stream")] water: &mut WatershedWork,
            #[cfg(feature = "coastal-highways")] coast: &mut CoastalHighwayWork,
            #[cfg(feature = "regional-rail")] rail_work: &mut RailSurveyWork,
        ) -> bool {
            let Some(slots) = critical_section::with(|cs| SURVEY.borrow_ref_mut(cs).take()) else {
                return false;
            };
            // The app writes a request before transferring the sole mutable slot.
            let request = unsafe { slots.request.assume_init_ref() };
            #[cfg(feature = "scene-probe")]
            let started = micros();
            #[cfg(feature = "scene-probe")]
            let mut last = started;
            #[cfg(feature = "scene-probe")]
            let mut longest = 0;
            request.prepare_into(
                &mut slots.result,
                #[cfg(feature = "river-stream")] water,
                #[cfg(feature = "coastal-highways")] coast,
                #[cfg(feature = "regional-rail")] (&slots.rail, rail_work),
                || {
                    #[cfg(feature = "scene-probe")]
                    { longest = longest.max(micros() - last); }
                    if !self.send_frame() {
                        #[cfg(feature = "packed-60-terrain")]
                        if terrain_task::poll() {
                            #[cfg(feature = "scene-probe")]
                            { last = micros(); }
                            return;
                        }
                        #[cfg(feature = "cooperative-sky")]
                        let _ = self.sky.poll();
                    }
                    #[cfg(feature = "scene-probe")]
                    { last = micros(); }
                },
            );
            #[cfg(feature = "scene-probe")]
            if option_env!("SCENIC_PROFILE_CAPTURE").is_none() {
                esp_println::println!("transport survey_us={} longest_slice_us={}",
                    micros() - started, longest.max(micros() - last));
            }
            unsafe { slots.request.assume_init_drop(); }
            critical_section::with(|cs| {
                *SURVEY_DONE.borrow_ref_mut(cs) = Some(slots);
            });
            true
        }
    }

    fn work(
        panel: DevicePanel,
        #[cfg(feature = "river-stream")] water: &mut WatershedWork,
        #[cfg(feature = "coastal-highways")] coast: &mut CoastalHighwayWork,
        #[cfg(feature = "regional-rail")] rail_work: &mut RailSurveyWork,
    ) -> ! {
        let delay = Delay::new();
        let mut worker = Worker {
            panel,
            #[cfg(feature = "cooperative-sky")]
            sky: crate::sky_worker::SkyWorker::new(),
        };
        loop {
            if worker.send_frame() {
                continue;
            }
            #[cfg(feature = "cooperative-sky")]
            if worker.sky.poll() { continue; }
            #[cfg(feature = "parallel-terrain")]
            if terrain_task::poll() { continue; }
            #[cfg(feature = "transport-stream")]
            if worker.survey_next(
                #[cfg(feature = "river-stream")] water,
                #[cfg(feature = "coastal-highways")] coast,
                #[cfg(feature = "regional-rail")] rail_work,
            ) {
                continue;
            }
            delay.delay_micros(40);
        }
    }
}
