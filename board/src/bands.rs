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
    run_start: [u16; LOGICAL],
    logical_inset: [u16; LOGICAL],
}

/// Keep the unmasked rotated row across scan windows. The physical aperture
/// and the native overlay are applied after copying each row into DMA memory.
pub struct Scanline {
    source_x: usize,
    pixels: [[u8; 2]; SIDE],
}

impl Scanline {
    pub const fn new() -> Self {
        Self { source_x: usize::MAX, pixels: [[0; 2]; SIDE] }
    }

    pub fn begin_frame(&mut self) {
        self.source_x = usize::MAX;
    }
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
            run_start: core::array::from_fn(|column| {
                source.partition_point(|&mapped| usize::from(mapped) < column) as u16
            }),
            logical_inset: core::array::from_fn(|row| circle_inset(LOGICAL, row) as u16),
        }
    }

    pub fn band(&self, index: usize) -> Band {
        self.bands[index]
    }

    pub fn pack(&self, image: &[u16], target: &mut [u8], band: Band, line: &mut Scanline) {
        let (target, remaining) = target.as_chunks_mut::<2>();
        debug_assert!(remaining.is_empty());
        for (y, output) in (band.y..band.y + band.height).zip(target.chunks_exact_mut(band.width)) {
            let edge = usize::from(self.native_inset[y]);
            let inset = edge - band.x;
            output[..inset].fill([0; 2]);
            output[band.width - inset..].fill([0; 2]);
            let visible = &mut output[inset..band.width - inset];
            if LOGICAL == SIDE {
                let first = (SIDE - 1 - edge) * SIDE + y;
                let source = image[..=first].iter().rev().step_by(SIDE);
                for (pixel, &color) in visible.iter_mut().zip(source) {
                    *pixel = color.to_be_bytes();
                }
            } else {
                let source_x = usize::from(self.source[y]);
                if source_x != line.source_x {
                    self.expand_row(image, source_x, &mut line.pixels);
                    line.source_x = source_x;
                }
                visible.copy_from_slice(&line.pixels[edge..SIDE - edge]);
            }
        }
    }

    fn expand_row(&self, image: &[u16], source_x: usize, output: &mut [[u8; 2]; SIDE]) {
        let edge = usize::from(self.logical_inset[source_x]);
        let columns = &self.run_start;
        let color = |column: usize| image[(LOGICAL - 1 - column) * LOGICAL + source_x].to_be_bytes();
        // The midpoint mapping is monotone. Assign each source pixel once,
        // then extend the nearest valid color to the edge of the circle.
        let first = usize::from(columns[edge]);
        output[..first].fill(color(edge));
        let mut start = first;
        for column in edge..LOGICAL - edge {
            let end = columns.get(column + 1).copied().map_or(SIDE, usize::from);
            output[start..end].fill(color(column));
            start = end;
        }
        output[start..].fill(color(LOGICAL - 1 - edge));
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

    #[test]
    fn restarting_a_frame_invalidates_the_cached_source_row() {
        let scan = Scan::<60>::new();
        let mut line = Scanline::new();
        let mut packed = [0; BUFFER_BYTES];
        let band = scan.band(BANDS / 2);
        for color in [0x17b4_u16, 0xab03] {
            line.begin_frame();
            scan.pack(&[color; 60 * 60], &mut packed[..band.bytes()], band, &mut line);
            let at = band.width;
            assert_eq!(packed[at..at + 2], color.to_be_bytes());
        }
    }

    fn check_packed_pixels<const SIZE: usize>() {
        let scan = Scan::<SIZE>::new();
        let image: Vec<u16> = (0..SIZE * SIZE)
            .map(|index| 0x9a24_u16.wrapping_add((index as u16).wrapping_mul(3)))
            .collect();
        let mut packed = [0xac; BUFFER_BYTES];
        let mut line = Scanline::new();
        for index in 0..BANDS {
            let band = scan.band(index);
            packed.fill(0xac);
            scan.pack(&image, &mut packed[..band.bytes()], band, &mut line);
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
