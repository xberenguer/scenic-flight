//! Allocation-free reader for the native Southwest elevation grid.
//!
//! SWT1 stores the SWE1 grid header, payload offsets measured from
//! the end of the directory, and independent zlib streams for 64 × 64 tiles.
//! Each stream contains low then high bytes of row-wise zigzag deltas.
//! The last tile on an axis may be smaller. Heights retain signed metres.

pub const TILE_SIDE: usize = 64;
pub const TILE_CELLS: usize = TILE_SIDE * TILE_SIDE;
pub const TILE_PLANE_BYTES: usize = 2 * TILE_CELLS;
const MAX_ENCODED_BYTES: usize = TILE_PLANE_BYTES + 32;
const HEADER: usize = 24;
const OFFSET_BYTES: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroundTileError {
    Header,
    Bounds,
    Directory,
    Buffer,
    Read,
    Deflate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroundTileLayout {
    origin: [i32; 2],
    spacing: i32,
    size: [usize; 2],
    grid: [usize; 2],
    count: usize,
}

impl GroundTileLayout {
    pub fn origin(self) -> [i32; 2] {
        self.origin
    }
    pub fn spacing(self) -> i32 {
        self.spacing
    }
    pub fn size(self) -> [usize; 2] {
        self.size
    }
    pub fn grid(self) -> [usize; 2] {
        self.grid
    }

    pub fn tile_dimensions(self, index: usize) -> Option<[usize; 2]> {
        if index >= self.count {
            return None;
        }
        let tile = [index % self.grid[0], index / self.grid[0]];
        Some(core::array::from_fn(|axis| {
            (self.size[axis] - tile[axis] * TILE_SIDE).min(TILE_SIDE)
        }))
    }

    /// Convert a ground-grid sample into a tile number and in-tile index.
    pub fn locate(self, col: usize, row: usize) -> Option<(usize, usize)> {
        if col >= self.size[0] || row >= self.size[1] {
            return None;
        }
        let index = row / TILE_SIDE * self.grid[0] + col / TILE_SIDE;
        let [width, _] = self.tile_dimensions(index)?;
        Some((index, row % TILE_SIDE * width + col % TILE_SIDE))
    }
}

pub struct GroundTileFile {
    layout: GroundTileLayout,
    directory_len: usize,
    payload_len: usize,
}

impl GroundTileFile {
    pub fn open(
        file_len: usize,
        read: &mut impl FnMut(usize, &mut [u8]) -> bool,
    ) -> Result<Self, GroundTileError> {
        if file_len < HEADER {
            return Err(GroundTileError::Header);
        }
        let mut header = [0; HEADER];
        if !read(0, &mut header) {
            return Err(GroundTileError::Read);
        }
        if header[..4] != *b"SWT1" {
            return Err(GroundTileError::Header);
        }
        let field =
            |offset: usize| i32::from_le_bytes(header[offset..offset + 4].try_into().unwrap());
        let origin = [field(4), field(8)];
        let spacing = field(12);
        let size = [field(16), field(20)];
        if !(20..=300).contains(&spacing) || size.iter().any(|&side| side < 2) {
            return Err(GroundTileError::Header);
        }
        let size = size.map(|side| side as usize);
        let grid = size.map(|side| side.div_ceil(TILE_SIDE));
        let count = grid[0]
            .checked_mul(grid[1])
            .ok_or(GroundTileError::Bounds)?;
        let directory_len = count
            .checked_add(1)
            .and_then(|n| n.checked_mul(OFFSET_BYTES))
            .ok_or(GroundTileError::Bounds)?;
        let payload_start = HEADER
            .checked_add(directory_len)
            .ok_or(GroundTileError::Bounds)?;
        let payload_len = file_len
            .checked_sub(payload_start)
            .ok_or(GroundTileError::Bounds)?;
        if u32::try_from(payload_len).is_err() {
            return Err(GroundTileError::Bounds);
        }
        Ok(Self {
            layout: GroundTileLayout {
                origin,
                spacing,
                size,
                grid,
                count,
            },
            directory_len,
            payload_len,
        })
    }

    pub fn directory_len(&self) -> usize {
        self.directory_len
    }

    pub fn read_directory<'a>(
        &self,
        buffer: &'a mut [u8],
        read: &mut impl FnMut(usize, &mut [u8]) -> bool,
    ) -> Result<GroundTiles<'a>, GroundTileError> {
        let directory = buffer
            .get_mut(..self.directory_len)
            .ok_or(GroundTileError::Buffer)?;
        if !read(HEADER, directory) {
            return Err(GroundTileError::Read);
        }
        let mut entries = directory.chunks_exact(OFFSET_BYTES).map(offset);
        if entries.next() != Some(0) {
            return Err(GroundTileError::Directory);
        }
        let mut start = 0;
        let mut max_input = 0;
        for end in entries {
            let len = end.checked_sub(start).ok_or(GroundTileError::Directory)?;
            if end > self.payload_len || !(6..=MAX_ENCODED_BYTES).contains(&len) {
                return Err(GroundTileError::Directory);
            }
            max_input = max_input.max(len);
            start = end;
        }
        if start != self.payload_len {
            return Err(GroundTileError::Directory);
        }
        Ok(GroundTiles {
            layout: self.layout,
            directory,
            payload_start: HEADER + self.directory_len,
            max_input,
        })
    }
}

pub struct GroundTiles<'a> {
    layout: GroundTileLayout,
    directory: &'a [u8],
    payload_start: usize,
    max_input: usize,
}

impl GroundTiles<'_> {
    pub fn layout(&self) -> GroundTileLayout {
        self.layout
    }
    pub fn max_input_len(&self) -> usize {
        self.max_input
    }

    /// The inflater must verify the zlib checksum and consume/produce exactly
    /// the supplied input/output ranges. All sizes are bounded by one tile.
    pub fn decode_tile<'out>(
        &self,
        index: usize,
        encoded: &mut [u8],
        planes: &mut [u8],
        out: &'out mut [i16],
        read: &mut impl FnMut(usize, &mut [u8]) -> bool,
        inflate: &mut impl FnMut(&[u8], &mut [u8]) -> bool,
    ) -> Result<&'out [i16], GroundTileError> {
        let [width, height] = self
            .layout
            .tile_dimensions(index)
            .ok_or(GroundTileError::Bounds)?;
        let at = index * OFFSET_BYTES;
        let start = offset(&self.directory[at..at + OFFSET_BYTES]);
        let end = offset(&self.directory[at + OFFSET_BYTES..at + 2 * OFFSET_BYTES]);
        let encoded = encoded
            .get_mut(..end - start)
            .ok_or(GroundTileError::Buffer)?;
        let count = width * height;
        let planes = planes.get_mut(..count * 2).ok_or(GroundTileError::Buffer)?;
        let out = out.get_mut(..count).ok_or(GroundTileError::Buffer)?;
        if !read(self.payload_start + start, encoded) {
            return Err(GroundTileError::Read);
        }
        if !inflate(encoded, planes) {
            return Err(GroundTileError::Deflate);
        }

        let (low, high) = planes.split_at(count);
        for ((low, high), row) in low
            .chunks_exact(width)
            .zip(high.chunks_exact(width))
            .zip(out.chunks_exact_mut(width))
        {
            let mut previous = 0i16;
            for ((&lo, &hi), height) in low.iter().zip(high).zip(row) {
                let zigzag = u16::from_le_bytes([lo, hi]);
                let delta = (zigzag >> 1) as i16 ^ -((zigzag & 1) as i16);
                previous = previous.wrapping_add(delta);
                *height = previous;
            }
        }
        Ok(out)
    }
}

fn offset(bytes: &[u8]) -> usize {
    u32::from_le_bytes(bytes.try_into().unwrap()) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn source() -> Vec<u8> {
        // One short-width tile and one short-height tile at the grid edges.
        let mut bytes = b"SWT1".to_vec();
        for value in [-43_200i32, 263_100, 150, 65, 66] {
            bytes.extend(value.to_le_bytes());
        }
        for end in [0u32, 8192, 8320, 8576, 8584] {
            bytes.extend(end.to_le_bytes());
        }
        bytes.resize(HEADER + 20 + 8584, 0);
        // The last tile's two samples are −85 and 3,719. Each row starts
        // from zero. Inject a copy instead of the runtime zlib inflater.
        let at = bytes.len() - 8;
        bytes[at..at + 8].copy_from_slice(&[169, 14, 0, 29, 0, 0, 0, 0]);
        bytes
    }

    #[test]
    fn reconstructs_signed_heights_in_partial_tiles() {
        let bytes = source();
        let mut read = |at: usize, out: &mut [u8]| {
            out.copy_from_slice(&bytes[at..at + out.len()]);
            true
        };
        // A tile with two cells would be too short for zlib if its fake
        // input contained only the raw elevation bytes.
        let mut directory = [0; 20];
        let file = GroundTileFile::open(bytes.len(), &mut read).unwrap();
        let tiles = file.read_directory(&mut directory, &mut read).unwrap();
        let layout = tiles.layout();
        assert_eq!(layout.origin(), [-43_200, 263_100]);
        assert_eq!(layout.spacing(), 150);
        assert_eq!(layout.size(), [65, 66]);
        assert_eq!(layout.grid(), [2, 2]);
        assert_eq!(layout.locate(64, 65), Some((3, 1)));
        assert_eq!(layout.locate(65, 65), None);
        assert_eq!(layout.tile_dimensions(3), Some([1, 2]));
        assert_eq!(tiles.max_input_len(), TILE_PLANE_BYTES);
        let mut heights = [0; 2];
        let mut inflate = |src: &[u8], dst: &mut [u8]| {
            dst.copy_from_slice(&src[..dst.len()]);
            true
        };
        assert_eq!(
            tiles.decode_tile(
                3,
                &mut [0; 8],
                &mut [0; 4],
                &mut heights,
                &mut read,
                &mut inflate
            ),
            Ok(&[-85, 3_719][..]),
        );
        assert_eq!(
            tiles.decode_tile(
                3,
                &mut [0; 8],
                &mut [0; 3],
                &mut heights,
                &mut read,
                &mut inflate
            ),
            Err(GroundTileError::Buffer),
        );
    }

    #[test]
    fn rejects_non_monotonic_or_truncated_directory() {
        let mut bytes = source();
        bytes[HEADER + 8..HEADER + 12].copy_from_slice(&6u32.to_le_bytes());
        let mut read = |at: usize, out: &mut [u8]| {
            out.copy_from_slice(&bytes[at..at + out.len()]);
            true
        };
        assert!(matches!(
            GroundTileFile::open(HEADER + 19, &mut read),
            Err(GroundTileError::Bounds)
        ));
        let file = GroundTileFile::open(bytes.len(), &mut read).unwrap();
        assert!(matches!(
            file.read_directory(&mut [0; 20], &mut read),
            Err(GroundTileError::Directory)
        ));
    }
}
