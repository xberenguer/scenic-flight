const DEBOUNCE_MS: u32 = 24;
const PROMPT_MS: u32 = 250;
const NEW_WORLD_MS: u32 = 1_200;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Buttons {
    pub small: bool,
    pub large: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    AircraftPicker,
    Flight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    NextAircraft,
    Fly,
    ToggleFlightInfo,
    ReturnToPicker,
}

#[derive(Default)]
struct Button {
    raw: bool,
    pressed: bool,
    changed_at: u32,
}

impl Button {
    fn update(&mut self, now: u32, raw: bool) -> bool {
        if raw != self.raw {
            self.raw = raw;
            self.changed_at = now;
        }
        if self.pressed != raw && now.wrapping_sub(self.changed_at) >= DEBOUNCE_MS {
            self.pressed = raw;
            return true;
        }
        false
    }
}

#[derive(Default)]
pub struct Controls {
    small: Button,
    large: Button,
    hold_started: Option<u32>,
}

impl Controls {
    pub fn update(&mut self, now: u32, raw: Buttons, screen: Screen) -> Option<Command> {
        let small_edge = self.small.update(now, raw.small);
        let large_edge = self.large.update(now, raw.large);
        match screen {
            Screen::AircraftPicker => {
                self.hold_started = None;
                if large_edge && self.large.pressed {
                    Some(Command::Fly)
                } else if small_edge && self.small.pressed {
                    Some(Command::NextAircraft)
                } else {
                    None
                }
            }
            Screen::Flight => {
                if small_edge {
                    if self.small.pressed {
                        self.hold_started = Some(self.small.changed_at);
                    } else if let Some(start) = self.hold_started.take() {
                        let held = self.small.changed_at.wrapping_sub(start);
                        return Some(if held >= NEW_WORLD_MS {
                            Command::ReturnToPicker
                        } else {
                            Command::ToggleFlightInfo
                        });
                    }
                }
                if raw.small && self.hold_started.is_some_and(|start| {
                    now.wrapping_sub(start) >= NEW_WORLD_MS
                }) {
                    self.hold_started = None;
                    return Some(Command::ReturnToPicker);
                }
                None
            }
        }
    }

    pub fn new_world_progress(&self, now: u32) -> Option<f32> {
        let observed_end = if self.small.raw { now } else { self.small.changed_at };
        let elapsed = observed_end.wrapping_sub(self.hold_started?);
        (elapsed >= PROMPT_MS).then(|| {
            ((elapsed - PROMPT_MS) as f32 / (NEW_WORLD_MS - PROMPT_MS) as f32).min(1.0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UP: Buttons = Buttons { small: false, large: false };
    const SMALL: Buttons = Buttons { small: true, large: false };
    const LARGE: Buttons = Buttons { small: false, large: true };

    #[test]
    fn full_hold_enters_picker_before_release_and_does_not_browse() {
        let mut control = Controls::default();
        assert_eq!(control.update(100, SMALL, Screen::Flight), None);
        assert_eq!(control.update(124, SMALL, Screen::Flight), None);
        assert_eq!(control.new_world_progress(349), None);
        assert_eq!(control.new_world_progress(350), Some(0.0));
        assert_eq!(control.update(1_299, SMALL, Screen::Flight), None);
        assert!(control.new_world_progress(1_299).unwrap() > 0.99);
        assert_eq!(control.update(1_300, SMALL, Screen::Flight), Some(Command::ReturnToPicker));
        assert_eq!(control.update(2000, SMALL, Screen::AircraftPicker), None);
        assert_eq!(control.update(2050, UP, Screen::AircraftPicker), None);
        assert_eq!(control.update(2080, UP, Screen::AircraftPicker), None);
        assert_eq!(control.update(2100, SMALL, Screen::AircraftPicker), None);
        assert_eq!(control.update(2130, SMALL, Screen::AircraftPicker), Some(Command::NextAircraft));
        assert_eq!(control.update(2200, LARGE, Screen::AircraftPicker), None);
        assert_eq!(control.update(2230, LARGE, Screen::AircraftPicker), Some(Command::Fly));
    }

    #[test]
    fn tap_and_wrapping_milliseconds_are_supported() {
        let mut control = Controls::default();
        let start = u32::MAX - 399;
        assert_eq!(control.update(start, SMALL, Screen::Flight), None);
        assert_eq!(control.update(start + 30, SMALL, Screen::Flight), None);
        assert_eq!(control.update(1, UP, Screen::Flight), None);
        assert_eq!(control.update(26, UP, Screen::Flight), Some(Command::ToggleFlightInfo));
        assert_eq!(control.new_world_progress(26), None);
        assert_eq!(control.update(10_000, UP, Screen::AircraftPicker), None);
    }

    #[test]
    fn bounce_does_not_send_duplicate_commands() {
        let mut control = Controls::default();
        for (t, sample) in [(0, SMALL), (4, UP), (7, SMALL), (11, UP), (14, SMALL)] {
            assert_eq!(control.update(t, sample, Screen::AircraftPicker), None);
        }
        assert_eq!(control.update(38, SMALL, Screen::AircraftPicker), Some(Command::NextAircraft));
        assert_eq!(control.update(60, SMALL, Screen::AircraftPicker), None);
    }
}

#[cfg(test)]
mod transition_regressions {
    use super::*;

    const DOWN: Buttons = Buttons {
        small: true,
        large: false,
    };
    const UP: Buttons = Buttons {
        small: false,
        large: false,
    };

    #[test]
    fn release_after_a_render_gap_completes_the_hold() {
        let mut controls = Controls::default();
        for (now, buttons) in [(100, DOWN), (125, DOWN), (1_275, DOWN), (1_320, UP)] {
            assert_eq!(controls.update(now, buttons, Screen::Flight), None);
        }
        assert_eq!(
            controls.update(1_345, UP, Screen::Flight),
            Some(Command::ReturnToPicker)
        );
        assert_eq!(controls.new_world_progress(1_345), None);
        assert_eq!(controls.update(1_375, UP, Screen::AircraftPicker), None);
    }

    #[test]
    fn release_before_the_threshold_does_not_mature_during_debounce() {
        let mut controls = Controls::default();
        for (now, buttons) in [(100, DOWN), (125, DOWN), (1_299, UP)] {
            assert_eq!(controls.update(now, buttons, Screen::Flight), None);
        }
        assert!(controls.new_world_progress(1_320).unwrap() < 1.0);
        assert_eq!(
            controls.update(1_325, UP, Screen::Flight),
            Some(Command::ToggleFlightInfo)
        );
        assert_eq!(controls.new_world_progress(1_325), None);
    }
}
