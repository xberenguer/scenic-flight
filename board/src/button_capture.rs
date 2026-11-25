use core::cell::RefCell;
use critical_section::Mutex;
use esp_hal::{gpio::Input, handler, time::Duration, timer::PeriodicTimer, Blocking};
use scenic_flight::controls::Buttons;

pub const MAX_EDGES: usize = 128;

#[derive(Clone, Copy)]
pub struct Edge {
    pub at: u32,
    pub buttons: Buttons,
}

impl Edge {
    pub const UP: Self = Self { at: 0, buttons: Buttons { small: false, large: false } };
}

struct Capture {
    timer: PeriodicTimer<'static, Blocking>,
    pins: (Input<'static>, Input<'static>),
    edges: [Edge; MAX_EDGES],
    current: Buttons,
    count: usize,
    overflow: bool,
    #[cfg(feature = "scene-probe")]
    started: u32,
}

static CAPTURE: Mutex<RefCell<Option<Capture>>> = Mutex::new(RefCell::new(None));

impl Capture {
    fn read(&mut self, now: u32) {
        let buttons = Buttons { small: self.pins.0.is_low(), large: self.pins.1.is_high() };
        #[cfg(feature = "scene-probe")]
        let buttons = if option_env!("SCENIC_PROFILE_TIMED_INPUT") == Some("1") {
            let elapsed = now.wrapping_sub(self.started);
            Buttons {
                small: buttons.small || (8_400..8_480).contains(&elapsed) || (12_400..13_700).contains(&elapsed),
                large: buttons.large || (14_800..14_900).contains(&elapsed),
            }
        } else {
            buttons
        };
        if self.current == buttons {
            return;
        }
        self.current = buttons;
        if let Some(edge) = self.edges.get_mut(self.count) {
            *edge = Edge { at: now, buttons };
            self.count += 1;
        } else {
            self.overflow = true;
        }
    }
}

#[inline(never)]
pub fn start(mut timer: PeriodicTimer<'static, Blocking>, pins: (Input<'static>, Input<'static>)) {
    critical_section::with(|cs| {
        timer.set_interrupt_handler(sample_buttons);
        timer.start(Duration::from_millis(4)).expect("button capture timer");
        timer.listen();
        CAPTURE.borrow_ref_mut(cs).replace(Capture {
            timer,
            pins,
            edges: [Edge::UP; MAX_EDGES],
            current: Edge::UP.buttons,
            count: 0,
            overflow: false,
            #[cfg(feature = "scene-probe")]
            started: (crate::micros() / 1_000) as u32,
        });
    });
}

#[handler]
fn sample_buttons() {
    critical_section::with(|cs| {
        let mut slot = CAPTURE.borrow_ref_mut(cs);
        let capture = slot.as_mut().unwrap();
        capture.timer.clear_interrupt();
        capture.read((crate::micros() / 1_000) as u32);
    });
}

pub fn take(edges: &mut [Edge; MAX_EDGES]) -> (u32, usize, Buttons, bool) {
    critical_section::with(|cs| {
        let mut slot = CAPTURE.borrow_ref_mut(cs);
        let capture = slot.as_mut().unwrap();
        let now = (crate::micros() / 1_000) as u32;
        capture.read(now);
        let count = core::mem::take(&mut capture.count);
        edges[..count].copy_from_slice(&capture.edges[..count]);
        (now, count, capture.current, core::mem::take(&mut capture.overflow))
    })
}
