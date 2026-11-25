//! A one-frame crop of the bytes sent to the physical panel.

use crate::bands::{Band, BANDS, SIDE};

const CAPTURE_RIM: bool = option_env!("SCENIC_PROFILE_NATIVE_RIM").is_some();
const CAPTURE_REFUEL: bool = option_env!("SCENIC_PROFILE_NATIVE_REFUEL").is_some();
pub const CAPTURE_X: usize = if CAPTURE_RIM {
    0
} else if CAPTURE_REFUEL {
    114
} else {
    145
};
pub const CAPTURE_Y: usize = if CAPTURE_RIM {
    0
} else if CAPTURE_REFUEL {
    90
} else {
    172
};
pub const CAPTURE_WIDTH: usize = if CAPTURE_REFUEL { 238 } else { 176 };
pub const CAPTURE_HEIGHT: usize = if CAPTURE_REFUEL { 196 } else { 126 };
pub const CAPTURE_PIXELS: usize = CAPTURE_WIDTH * CAPTURE_HEIGHT;

pub struct PackedCapture<'a> {
    pixels: &'a mut [u16],
    frame: Option<u32>,
    written: usize,
    bands: usize,
}

impl<'a> PackedCapture<'a> {
    pub fn new(pixels: &'a mut [u16]) -> Self {
        assert_eq!(pixels.len(), CAPTURE_PIXELS);
        Self {
            pixels,
            frame: None,
            written: 0,
            bands: 0,
        }
    }

    pub fn request(&mut self, frame: u32) {
        self.frame = Some(frame);
        self.written = 0;
        self.bands = 0;
        self.pixels.fill(0);
    }

    pub fn record_band(&mut self, source: &[u8], band: Band) {
        if self.frame.is_none() {
            return;
        }
        self.bands += 1;
        let rows = band.y.max(CAPTURE_X)..(band.y + band.height).min(CAPTURE_X + CAPTURE_WIDTH);
        let left = band.x.max(SIDE - CAPTURE_Y - CAPTURE_HEIGHT);
        let right = (band.x + band.width).min(SIDE - CAPTURE_Y);
        if left >= right {
            return;
        }
        // The transfer is clockwise and big endian. Store an upright crop of
        // the final byte stream after both the terrain and aircraft were packed.
        for row in rows {
            let x = row - CAPTURE_X;
            let start = 2 * ((row - band.y) * band.width + left - band.x);
            let packed = &source[start..start + (right - left) * 2];
            for (offset, pixel) in packed.chunks_exact(2).enumerate() {
                let y = SIDE - 1 - left - offset - CAPTURE_Y;
                self.pixels[y * CAPTURE_WIDTH + x] = u16::from_be_bytes([pixel[0], pixel[1]]);
            }
            self.written += right - left;
        }
    }

    pub fn take(&mut self) -> Option<(u32, &[u16])> {
        let frame = self.frame.take()?;
        let covered = if CAPTURE_RIM {
            self.written > 0
        } else {
            self.written == CAPTURE_PIXELS
        };
        (self.bands == BANDS && covered).then_some((frame, &*self.pixels))
    }
}
