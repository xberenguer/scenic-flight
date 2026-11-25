//! CO5300 scan windows and the enclosure's clockwise pixel orientation.

pub const SIDE: usize = 466;
pub(crate) const ROWS: usize = 8;
pub const BUFFER_BYTES: usize = 8192;
pub const BANDS: usize = SIDE.div_ceil(ROWS);

const _: () = assert!(SIDE % 2 == 0 && ROWS % 2 == 0 && SIDE * ROWS * 2 <= BUFFER_BYTES);

#[derive(Clone, Copy)]
pub struct Band {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Band {
    pub fn bytes(self) -> usize {
        self.width * self.height * 2
    }

    pub fn window(self) -> ([u8; 4], [u8; 4]) {
        fn range(start: usize, size: usize) -> [u8; 4] {
            let [lo_h, lo_l] = (start as u16).to_be_bytes();
            let [hi_h, hi_l] = ((start + size - 1) as u16).to_be_bytes();
            [lo_h, lo_l, hi_h, hi_l]
        }
        (range(self.x + 6, self.width), range(self.y, self.height))
    }
}

pub struct Scan<const LOGICAL: usize> {
    bands: [Band; BANDS],
    source: [u16; SIDE],
    native_inset: [u16; SIDE],
    source_extent: [[u16; 2]; LOGICAL],
}

fn inside(side: usize, x: usize, y: usize) -> bool {
    let dx = (2 * x + 1) as i32 - side as i32;
    let dy = (2 * y + 1) as i32 - side as i32;
    dx * dx + dy * dy <= (side * side) as i32
}

fn circle_inset(side: usize, row: usize) -> usize {
    (0..=side / 2).find(|&col| inside(side, col, row)).unwrap()
}

impl<const LOGICAL: usize> Scan<LOGICAL> {
    pub fn new() -> Self {
        assert!((1..=SIDE).contains(&LOGICAL));
        let source =
            core::array::from_fn(|pixel| (((2 * pixel + 1) * LOGICAL) / (2 * SIDE)) as u16);
        let native_inset = core::array::from_fn(|row| circle_inset(SIDE, row) as u16);
        Self {
            bands: core::array::from_fn(|index| {
                let y = index * ROWS;
                let height = ROWS.min(SIDE - y);
                let widest = (SIDE / 2).clamp(y, y + height - 1);
                let x = usize::from(native_inset[widest]) & !1;
                Band {
                    x,
                    y,
                    width: SIDE - 2 * x,
                    height,
                }
            }),
            source,
            native_inset,
            source_extent: core::array::from_fn(|row| {
                let inset = circle_inset(LOGICAL, row);
                [
                    source.partition_point(|&column| usize::from(column) < inset) as u16,
                    source.partition_point(|&column| usize::from(column) < LOGICAL - inset) as u16,
                ]
            }),
        }
    }

    pub fn band(&self, index: usize) -> Band {
        self.bands[index]
    }

    pub fn pack(&self, image: &[u16], target: &mut [u8], band: Band) {
        let row_bytes = band.width * 2;
        let mut last_source_x = usize::MAX;
        for row in 0..band.height {
            let source_x = usize::from(self.source[band.y + row]);
            let begin = row * row_bytes;
            if source_x == last_source_x {
                target.copy_within(begin - row_bytes..begin, begin);
                continue;
            }
            let columns = &self.source[band.x..band.x + band.width];
            let output = &mut target[begin..begin + row_bytes];
            for (&column, pixel) in columns.iter().zip(output.chunks_exact_mut(2)) {
                let source_y = LOGICAL - 1 - usize::from(column);
                pixel.copy_from_slice(&image[source_y * LOGICAL + source_x].to_be_bytes());
            }
            // Continue the nearest valid scenery color where upscaling would
            // sample the black area beyond the low-resolution circle.
            let [first, last] = self.source_extent[source_x].map(usize::from);
            let first = first.saturating_sub(band.x).min(band.width - 1) * 2;
            let last = last.saturating_sub(band.x).clamp(1, band.width) * 2;
            let left = [output[first], output[first + 1]];
            let right = [output[last - 2], output[last - 1]];
            for pixel in output[..first].chunks_exact_mut(2) {
                pixel.copy_from_slice(&left);
            }
            for pixel in output[last..].chunks_exact_mut(2) {
                pixel.copy_from_slice(&right);
            }
            last_source_x = source_x;
        }
        // Repeated logical rows share colors, but the physical circle has
        // a different edge on each native row. Clip after the row copies.
        for row in 0..band.height {
            let inset = usize::from(self.native_inset[band.y + row]) - band.x;
            let output = &mut target[row * row_bytes..(row + 1) * row_bytes];
            output[..inset * 2].fill(0);
            output[row_bytes - inset * 2..].fill(0);
        }
    }
}

#[cfg(feature = "native-flight-overlay")]
/// An upright native-panel image fragment, with the world behind it already filled.
pub struct NativeRegion<'a> {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub pixels: &'a [u16],
}

#[cfg(feature = "native-flight-overlay")]
impl<const LOGICAL: usize> Scan<LOGICAL> {
    /// Replace the intersecting part of a packed scan band with native pixels.
    pub fn pack_native_region(&self, target: &mut [u8], band: Band, region: &NativeRegion<'_>) {
        debug_assert!(region.x + region.width <= SIDE && region.y + region.height <= SIDE);
        debug_assert_eq!(region.pixels.len(), region.width * region.height);
        let rows = band.y.max(region.x)..(band.y + band.height).min(region.x + region.width);
        let first = band.x.max(SIDE - region.y - region.height);
        let last = (band.x + band.width).min(SIDE - region.y);
        if first >= last {
            return;
        }
        for row in rows {
            let screen_x = row - region.x;
            let begin = 2 * ((row - band.y) * band.width + first - band.x);
            let out = &mut target[begin..begin + 2 * (last - first)];
            let screen_y = SIDE - 1 - first - region.y;
            let pixel = screen_y * region.width + screen_x;
            let sources = region.pixels[..=pixel].iter().rev().step_by(region.width);
            for (color, packed) in sources.zip(out.chunks_exact_mut(2)) {
                packed.copy_from_slice(&color.to_be_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_cover_the_aperture_with_even_dma_geometry() {
        let scan = Scan::<128>::new();
        let mut y = 0;
        for index in 0..BANDS {
            let band = scan.band(index);
            let (columns, rows) = band.window();
            assert_eq!(band.y, y);
            assert_eq!(
                u16::from_be_bytes([columns[0], columns[1]]) as usize,
                band.x + 6
            );
            assert_eq!(
                u16::from_be_bytes([columns[2], columns[3]]) as usize,
                band.x + band.width + 5
            );
            assert_eq!(u16::from_be_bytes([rows[0], rows[1]]) as usize, band.y);
            assert_eq!(
                u16::from_be_bytes([rows[2], rows[3]]) as usize,
                band.y + band.height - 1
            );
            assert_eq!((band.x | band.y | band.width | band.height) & 1, 0);
            assert!(band.bytes() <= BUFFER_BYTES);
            assert!(band.width * band.height <= 4096);
            for row in y..y + band.height {
                for x in 0..SIDE {
                    if inside(SIDE, x, row) {
                        assert!((band.x..band.x + band.width).contains(&x));
                    }
                }
            }
            y += band.height;
        }
        assert_eq!(y, SIDE);
    }

    #[test]
    fn all_packed_pixels_match_clockwise_big_endian_reference() {
        check_packed_pixels::<60>();
        check_packed_pixels::<112>();
        check_packed_pixels::<128>();
        check_packed_pixels::<144>();
        check_packed_pixels::<160>();
        check_packed_pixels::<466>();
    }

    fn check_packed_pixels<const SIZE: usize>() {
        let scan = Scan::<SIZE>::new();
        let image: Vec<u16> = (0..SIZE * SIZE)
            .map(|index| 0x9a24_u16.wrapping_add((index as u16).wrapping_mul(3)))
            .collect();
        let mut packed = [0xac; BUFFER_BYTES];
        for index in 0..BANDS {
            let band = scan.band(index);
            packed.fill(0xac);
            scan.pack(&image, &mut packed[..band.bytes()], band);
            for row in 0..band.height {
                for column in 0..band.width {
                    let y = band.y + row;
                    let x = band.x + column;
                    let logical_x = ((y as f64 + 0.5) * SIZE as f64 / SIDE as f64) as usize;
                    let logical_y =
                        SIZE - 1 - ((x as f64 + 0.5) * SIZE as f64 / SIDE as f64) as usize;
                    let expected = if inside(SIDE, x, y) {
                        let edge = circle_inset(SIZE, logical_x);
                        let source_y = logical_y.clamp(edge, SIZE - 1 - edge);
                        assert!(inside(SIZE, logical_x, source_y));
                        image[source_y * SIZE + logical_x]
                    } else {
                        0
                    };
                    let offset = 2 * (row * band.width + column);
                    assert_eq!(packed[offset..offset + 2], expected.to_be_bytes());
                }
            }
            assert!(packed[band.bytes()..].iter().all(|&byte| byte == 0xac));
        }
    }
}
