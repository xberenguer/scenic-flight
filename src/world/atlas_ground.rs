//! Surveyed terrain between the mapped city grids.

use super::mix;
use crate::math::smoothstep;

const HEADER: usize = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtlasGround {
    bytes: &'static [u8],
    west: i32,
    north: i32,
    step: i32,
    width: usize,
    height: usize,
}

impl AtlasGround {
    pub fn parse(bytes: &'static [u8]) -> Option<Self> {
        if bytes.get(..4)? != b"SWE1" { return None; }
        let read = |at: usize| -> Option<i32> {
            Some(i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
        };
        let west = read(4)?;
        let north = read(8)?;
        let step = read(12)?;
        let width = usize::try_from(read(16)?).ok()?;
        let height = usize::try_from(read(20)?).ok()?;
        if !(20..=300).contains(&step) || width < 2 || height < 2
            || HEADER.checked_add(width.checked_mul(height)?.checked_mul(2)?)? != bytes.len()
        {
            return None;
        }
        Some(Self { bytes, west, north, step, width, height })
    }

    pub(super) fn sample(self, x: f32, z: f32) -> Option<(f32, f32)> {
        let step = self.step as f32;
        let east = (x - self.west as f32) / step - 0.5;
        let south = (self.north as f32 - z) / step - 0.5;
        let edge = east.min(south).min((self.width - 1) as f32 - east).min((self.height - 1) as f32 - south);
        if edge <= 0.0 { return None; }
        let index = south as usize * self.width + east as usize;
        let elevation = |offset| {
            let at = HEADER + 2 * (index + offset);
            f32::from(i16::from_le_bytes([self.bytes[at], self.bytes[at + 1]]))
        };
        let fraction = east - east as usize as f32;
        let top = mix(elevation(0), elevation(1), fraction);
        let bottom = mix(elevation(self.width), elevation(self.width + 1), fraction);
        let height = mix(top, bottom, south - south as usize as f32);
        Some((height, smoothstep(edge * step / 9_000.0)))
    }
}
