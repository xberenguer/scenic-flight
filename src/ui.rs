use crate::flight::{Aircraft, Flight};

const TEXT: u16 = color(242, 244, 238);
const MUTED: u16 = color(148, 175, 185);
const ACCENT: u16 = color(250, 194, 107);
const PANEL: u16 = color(19, 31, 37);
const LAYOUT_SIDE: usize = 112;
const EMPTY_RATING: u16 = color(33, 48, 63);
const INACTIVE_POSITION: u16 = color(46, 65, 83);

const fn color(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xf8) << 8) | ((g as u16 & 0xfc) << 3) | (b as u16 >> 3)
}

type PaintRect = [usize; 4];

struct Canvas<F> {
    paint: F,
    width: usize,
    height: usize,
    diameter: usize,
}

impl<F: FnMut(PaintRect, u16)> Canvas<F> {
    fn new(width: usize, height: usize, paint: F) -> Option<Self> {
        let diameter = width.min(height);
        (48..=512).contains(&diameter).then_some(Self {
            paint,
            width,
            height,
            diameter,
        })
    }

    fn rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: u16) {
        let right = x.saturating_add(width).min(self.width);
        let bottom = y.saturating_add(height).min(self.height);
        if x < right && y < bottom {
            (self.paint)([x, y, right - x, bottom - y], color);
        }
    }

    fn grid_bounds(&self, x: usize, y: usize, width: usize, height: usize) -> [usize; 4] {
        let scale = |n| (n * self.diameter + LAYOUT_SIDE / 2) / LAYOUT_SIDE;
        let left = scale(x);
        let top = scale(y);
        [
            (self.width - self.diameter) / 2 + left,
            (self.height - self.diameter) / 2 + top,
            scale(x + width) - left,
            scale(y + height) - top,
        ]
    }

    fn grid_rect(&mut self, x: usize, y: usize, width: usize, height: usize, ink: u16) {
        let [left, top, width, height] = self.grid_bounds(x, y, width, height);
        self.rect(left, top, width, height, ink);
    }

    fn grid_label(&mut self, center: usize, y: usize, text: &str, size: usize, ink: u16) {
        let width = (text.len() * 6).saturating_sub(1) * size;
        self.grid_label_at(center - width / 2, y, text, size, ink);
    }

    fn centered_grid_label(&mut self, y: usize, text: &str, ink: u16) {
        let width = (text.len() * 6).saturating_sub(1);
        self.grid_label_at(LAYOUT_SIDE.saturating_sub(width) / 2, y, text, 1, ink);
    }

    fn grid_label_at(&mut self, left: usize, y: usize, text: &str, size: usize, ink: u16) {
        for (index, letter) in text.bytes().enumerate() {
            for (row, bits) in glyph(letter).into_iter().enumerate() {
                let mut left_column = 0;
                while left_column < 5 {
                    if bits & (16 >> left_column) == 0 {
                        left_column += 1;
                        continue;
                    }
                    let mut right_column = left_column + 1;
                    while right_column < 5 && bits & (16 >> right_column) != 0 {
                        right_column += 1;
                    }
                    self.grid_rect(
                        left + (index * 6 + left_column) * size,
                        y + row * size,
                        (right_column - left_column) * size,
                        size,
                        ink,
                    );
                    left_column = right_column;
                }
            }
        }
    }

    fn picker_stage(&mut self) {
        // The stage leaves the middle of the screen open for the model preview.
        for (radius_x, radius_y, ink) in [
            (44, 9, color(6, 12, 20)),
            (41, 7, color(12, 22, 34)),
            (35, 5, color(17, 29, 42)),
        ] {
            let mut half = radius_x;
            let a = radius_x * radius_x;
            let b = radius_y * radius_y;
            for y in 0..=radius_y {
                while half * half * b + y * y * a > a * b {
                    half -= 1;
                }
                self.grid_rect(56 - half, 60 + y, 2 * half + 1, 1, ink);
                if y > 0 {
                    self.grid_rect(56 - half, 60 - y, 2 * half + 1, 1, ink);
                }
            }
        }
    }
}

struct PickerDetails {
    title: &'static str,
    subtype: &'static str,
    title_color: u16,
}

// Five-point comparisons of speed, maneuverability and typical flight height.
const PICKER_DETAILS: [PickerDetails; Aircraft::ALL.len()] = [
    PickerDetails {
        title: "CUB",
        subtype: "BUSH PLANE",
        title_color: color(242, 192, 122),
    },
    PickerDetails {
        title: "RACER",
        subtype: "AEROBATICS",
        title_color: color(238, 152, 143),
    },
    PickerDetails {
        title: "JET",
        subtype: "SPORT JET",
        title_color: color(213, 219, 241),
    },
    PickerDetails {
        title: "TRIKE",
        subtype: "ULTRALIGHT",
        title_color: color(235, 171, 132),
    },
    PickerDetails {
        title: "GLIDER",
        subtype: "SAILPLANE",
        title_color: color(228, 230, 205),
    },
    PickerDetails {
        title: "F-18",
        subtype: "FAST FIGHTER",
        title_color: color(178, 201, 217),
    },
    PickerDetails {
        title: "SR-71",
        subtype: "HIGH CRUISE",
        title_color: color(189, 191, 200),
    },
    PickerDetails {
        title: "EAGLE",
        subtype: "RIDGE SOARER",
        title_color: color(227, 198, 164),
    },
];

fn slice_paint(
    pixels: &mut [u16],
    width: usize,
    height: usize,
) -> Option<impl FnMut(PaintRect, u16) + '_> {
    (width.checked_mul(height)? <= pixels.len()).then_some(
        move |[x, y, w, h]: PaintRect, ink: u16| {
            for row in y..y + h {
                pixels[row * width + x..row * width + x + w].fill(ink);
            }
        },
    )
}

pub fn draw_picker(aircraft: Aircraft, pixels: &mut [u16], width: usize, height: usize) {
    draw_picker_stage(pixels, width, height);
    let Some(paint) = slice_paint(pixels, width, height) else {
        return;
    };
    draw_picker_labels_on(aircraft, width, height, paint);
}

/// Paint the logical preview's background before applying the spinner and native labels.
pub fn draw_picker_stage(pixels: &mut [u16], width: usize, height: usize) {
    let Some(total) = width.checked_mul(height) else {
        return;
    };
    if total > pixels.len() || !(48..=512).contains(&width.min(height)) {
        return;
    }
    pixels[..total].fill(0);
    let Some(paint) = slice_paint(pixels, width, height) else {
        return;
    };
    let Some(mut canvas) = Canvas::new(width, height, paint) else {
        return;
    };
    canvas.picker_stage();
}

/// Emit the picker labels and indicators as opaque rectangles in viewport coordinates.
pub fn draw_picker_labels_on(
    aircraft: Aircraft,
    width: usize,
    height: usize,
    paint: impl FnMut(PaintRect, u16),
) {
    let Some(mut canvas) = Canvas::new(width, height, paint) else {
        return;
    };
    let selected = aircraft as usize;
    let details = &PICKER_DETAILS[selected];
    canvas.grid_label(56, 11, details.title, 2, details.title_color);
    canvas.grid_label(56, 26, details.subtype, 1, MUTED);

    for ((center, label), rating) in [(28, "SPD"), (56, "AGI"), (84, "ALT")]
        .into_iter()
        .zip(aircraft.ratings())
    {
        canvas.grid_label(center, 71, label, 1, MUTED);
        for index in 0..5 {
            let ink = if index < usize::from(rating) {
                ACCENT
            } else {
                EMPTY_RATING
            };
            canvas.grid_rect(center - 12 + index * 5, 80, 4, 3, ink);
        }
    }

    // Match the action order on the Waveshare's upright display.
    for (row, width) in [1, 2, 3, 4, 3, 2, 1].into_iter().enumerate() {
        canvas.grid_rect(25, 91 + row, width, 1, ACCENT);
    }
    for (row, offset) in [0, 1, 2, 3, 2, 1, 0].into_iter().enumerate() {
        canvas.grid_rect(61 + offset, 91 + row, 2, 1, MUTED);
    }
    canvas.grid_label_at(33, 91, "FLY", 1, TEXT);
    canvas.grid_label_at(69, 91, "NEXT", 1, TEXT);
    let picker_index = aircraft.picker_index();
    for (index, _) in Aircraft::PICKER_ORDER.iter().enumerate() {
        let ink = if index == picker_index {
            TEXT
        } else {
            INACTIVE_POSITION
        };
        let left = (LAYOUT_SIDE - (Aircraft::PICKER_ORDER.len() * 6 - 2)) / 2;
        canvas.grid_rect(left + index * 6, 105, 4, 2, ink);
    }
}

pub fn draw_new_world(progress: f32, pixels: &mut [u16], width: usize, height: usize) {
    let Some(paint) = slice_paint(pixels, width, height) else {
        return;
    };
    draw_new_world_on(progress, width, height, paint);
}

/// Emit the reset prompt in viewport coordinates.
pub fn draw_new_world_on(
    progress: f32,
    width: usize,
    height: usize,
    paint: impl FnMut(PaintRect, u16),
) {
    let Some(mut canvas) = Canvas::new(width, height, paint) else {
        return;
    };
    canvas.grid_rect(21, 45, 70, 21, PANEL);
    canvas.centered_grid_label(49, "NEW WORLD", TEXT);
    let [x, y, track, height] = canvas.grid_bounds(32, 59, 48, 3);
    let filled = (track as f32 * progress.clamp(0.0, 1.0) + 0.5) as usize;
    canvas.rect(x, y, track, height, MUTED);
    canvas.rect(x, y, filled, height, ACCENT);
}

pub fn draw_flight_info(flight: &Flight, pixels: &mut [u16], width: usize, height: usize) {
    let Some(paint) = slice_paint(pixels, width, height) else {
        return;
    };
    draw_flight_info_on(flight, width, height, paint);
}

/// Emit the flight readout in viewport coordinates.
pub fn draw_flight_info_on(
    flight: &Flight,
    width: usize,
    height: usize,
    paint: impl FnMut(PaintRect, u16),
) {
    let Some(mut canvas) = Canvas::new(width, height, paint) else {
        return;
    };
    let mut altitude = *b"ALT 0000";
    let mut speed = *b"SPD 000";
    write_digits(&mut altitude[4..], flight.altitude.max(0.0) as u32);
    write_digits(&mut speed[4..], flight.speed().max(0.0) as u32);
    if let Ok(label) = core::str::from_utf8(&altitude) {
        canvas.centered_grid_label(13, label, TEXT);
    }
    if let Ok(label) = core::str::from_utf8(&speed) {
        canvas.centered_grid_label(22, label, TEXT);
    }
}

fn write_digits(buffer: &mut [u8], mut value: u32) {
    for digit in buffer.iter_mut().rev() {
        *digit = b'0' + (value % 10) as u8;
        value /= 10;
    }
    for digit in buffer.iter_mut().rev().skip(1).rev() {
        if *digit != b'0' {
            break;
        }
        *digit = b' ';
    }
}

fn glyph(letter: u8) -> [u8; 7] {
    match letter.to_ascii_uppercase() {
        b'A' => [14, 17, 17, 31, 17, 17, 17],
        b'B' => [30, 17, 17, 30, 17, 17, 30],
        b'C' => [15, 16, 16, 16, 16, 16, 15],
        b'D' => [30, 17, 17, 17, 17, 17, 30],
        b'E' => [31, 16, 16, 30, 16, 16, 31],
        b'F' => [31, 16, 16, 30, 16, 16, 16],
        b'G' => [15, 16, 16, 19, 17, 17, 15],
        b'H' => [17, 17, 17, 31, 17, 17, 17],
        b'I' => [14, 4, 4, 4, 4, 4, 14],
        b'J' => [7, 2, 2, 2, 18, 18, 12],
        b'K' => [17, 18, 20, 24, 20, 18, 17],
        b'L' => [16, 16, 16, 16, 16, 16, 31],
        b'M' => [17, 27, 21, 21, 17, 17, 17],
        b'N' => [17, 25, 25, 21, 19, 19, 17],
        b'O' => [14, 17, 17, 17, 17, 17, 14],
        b'P' => [30, 17, 17, 30, 16, 16, 16],
        b'Q' => [14, 17, 17, 17, 21, 18, 13],
        b'R' => [30, 17, 17, 30, 20, 18, 17],
        b'S' => [15, 16, 16, 14, 1, 1, 30],
        b'T' => [31, 4, 4, 4, 4, 4, 4],
        b'U' => [17, 17, 17, 17, 17, 17, 14],
        b'V' => [17, 17, 17, 17, 17, 10, 4],
        b'W' => [17, 17, 17, 21, 21, 27, 17],
        b'X' => [17, 17, 10, 4, 10, 17, 17],
        b'Y' => [17, 17, 10, 4, 4, 4, 4],
        b'Z' => [31, 1, 2, 4, 8, 16, 31],
        b'0' => [14, 17, 19, 21, 25, 17, 14],
        b'1' => [4, 12, 4, 4, 4, 4, 14],
        b'2' => [14, 17, 1, 2, 4, 8, 31],
        b'3' => [30, 1, 1, 14, 1, 1, 30],
        b'4' => [2, 6, 10, 18, 31, 2, 2],
        b'5' => [31, 16, 16, 30, 1, 1, 30],
        b'6' => [14, 16, 16, 30, 17, 17, 14],
        b'7' => [31, 1, 2, 4, 8, 8, 8],
        b'8' => [14, 17, 17, 14, 17, 17, 14],
        b'9' => [14, 17, 17, 15, 1, 1, 14],
        b'-' => [0, 0, 0, 31, 0, 0, 0],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_respects_the_round_viewport() {
        for (width, height) in [(48, 48), (64, 64), (112, 112), (162, 112), (466, 466)] {
            let diameter = width.min(height) as i64;
            let mut frame = std::vec![0; width * height];
            for aircraft in Aircraft::ALL {
                draw_picker(aircraft, &mut frame, width, height);
                for (index, &pixel) in frame.iter().enumerate().filter(|&(_, value)| *value != 0) {
                    let x = 2 * (index % width) as i64 + 1 - width as i64;
                    let y = 2 * (index / width) as i64 + 1 - height as i64;
                    assert!(
                        x * x + y * y < diameter * diameter,
                        "{aircraft:?} {width}x{height}: {x},{y} has {pixel:04x}"
                    );
                }
                assert!(frame.iter().filter(|&&pixel| pixel == ACCENT).count() > 5);
            }
        }
    }

    #[test]
    fn filled_bar_stays_inside_centered_panel() {
        let mut frame = [0; 112 * 112];
        draw_new_world(1.0, &mut frame, 112, 112);
        let filled = frame.iter().filter(|pixel| **pixel == ACCENT).count();
        assert_eq!(filled, 48 * 3);
        let first = frame.iter().position(|&pixel| pixel != 0).unwrap();
        let last = frame.iter().rposition(|&pixel| pixel != 0).unwrap();
        assert_eq!((first / 112 + last / 112) / 2, 55);
    }

    #[test]
    fn overlay_titles_fit_the_same_layout_at_different_resolutions() {
        let world = crate::world::World::new(1);
        let flight = Flight::start(&world, 1, Aircraft::Blackbird);
        for side in [112, 144, 160, 192, 466] {
            let mut frame = std::vec![0; side * side];
            let [left, top, width, height] = Canvas::new(side, side, |_, _| {})
                .unwrap()
                .grid_bounds(21, 45, 70, 21);
            draw_new_world(0.625, &mut frame, side, side);
            for (index, _) in frame.iter().enumerate().filter(|&(_, &pixel)| pixel != 0) {
                assert!((left..left + width).contains(&(index % side)));
                assert!((top..top + height).contains(&(index / side)));
            }
            frame.fill(0);
            draw_flight_info(&flight, &mut frame, side, side);
            let ink = frame.iter().filter(|&&pixel| pixel == TEXT).count();
            assert!(ink * 100 > side * side, "small flight readout at {side}px");
            assert!(
                ink * 60 < side * side,
                "oversized flight readout at {side}px"
            );
        }
    }
}
