//! Opaque native UI ink composited directly into the rotated transfer bands.

use crate::bands::{BANDS, Band, ROWS, SIDE};

const INK_WORDS: usize = 5;
const CAPACITY: usize = 512;
pub const STORAGE_WORDS: usize = CAPACITY * INK_WORDS;
const _: () = assert!(BANDS <= u64::BITS as usize);

pub struct NativeUi<'a> {
    ink: &'a mut [[u16; INK_WORDS]],
    used: usize,
    bands: u64,
    exhausted: bool,
}

impl<'a> NativeUi<'a> {
    pub fn new(storage: &'a mut [u16]) -> Self {
        assert!(storage.len() <= STORAGE_WORDS && storage.len() % INK_WORDS == 0);
        Self {
            ink: storage.as_chunks_mut().0,
            used: 0,
            bands: 0,
            exhausted: false,
        }
    }

    pub fn clear(&mut self) {
        self.used = 0;
        self.bands = 0;
        self.exhausted = false;
    }

    pub fn is_complete(&self) -> bool {
        !self.exhausted
    }

    pub fn rect(&mut self, [x, y, width, height]: [usize; 4], color: u16) {
        debug_assert!(x + width <= SIDE && y + height <= SIDE);
        if width == 0 || height == 0 {
            return;
        }
        let Some(ink) = self.ink.get_mut(self.used) else {
            self.exhausted = true;
            return;
        };
        // Convert from the upright viewport to the clockwise packet layout.
        *ink = [
            (SIDE - y - height) as u16,
            x as u16,
            (SIDE - y) as u16,
            (x + width) as u16,
            color,
        ];
        self.used += 1;
        let first = x / ROWS;
        let last = (x + width - 1) / ROWS;
        self.bands |= ((1 << (last - first + 1)) - 1) << first;
    }

    pub fn pack(&self, target: &mut [u8], band: Band) {
        if self.bands & (1 << (band.y / ROWS)) == 0 || !self.is_complete() {
            return;
        }
        for &[left, top, right, bottom, color] in &self.ink[..self.used] {
            let top = usize::from(top).max(band.y);
            let bottom = usize::from(bottom).min(band.y + band.height);
            if top >= bottom {
                continue;
            }
            let left = usize::from(left).max(band.x);
            let right = usize::from(right).min(band.x + band.width);
            if left >= right {
                continue;
            }
            let packed = color.to_be_bytes();
            for row in top..bottom {
                let begin = 2 * ((row - band.y) * band.width + left - band.x);
                let end = begin + (right - left) * 2;
                for pixel in target[begin..end].chunks_exact_mut(2) {
                    pixel.copy_from_slice(&packed);
                }
            }
        }
    }
}
