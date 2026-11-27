//! Allocation-free reader for the chunked SCG1 Las Vegas map.

use super::{HEADER, TILE_SIDE};

#[path = "native_stream.rs"]
mod stream;
pub use stream::{NativeFile, NativeFileDetails};

const STREAM_HEADER: usize = 16;
const ENTRY_BYTES: usize = 8;
const SLAB_ROWS: usize = 8;
const GROUND_COMPRESSED: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackError {
    Header,
    Bounds,
    Directory,
    Buffer,
    Deflate,
    Tile,
    Read,
}

/// Look up one original LVS2 palette word in decoded resident detail.
/// Return zero for a missing or empty tile.
pub trait NativeDetail: Sync {
    fn detail_word(&self, tile_col: usize, tile_row: usize, cell: usize) -> u16;
}

/// The unchanged LVS2 coordinate header and the decoded terrain dimensions.
#[derive(Clone, Copy)]
pub(super) struct GroundHeader {
    pub(super) ground_origin: [i32; 2],
    pub(super) ground_cell: i32,
    pub(super) ground_size: [usize; 2],
    pub(super) feature_origin: [i32; 2],
    pub(super) feature_cell: i32,
    pub(super) feature_size: [usize; 2],
    pub(super) water_offset: usize,
    pub(super) ground_len: usize,
    pub(super) base_metres: i32,
}

impl GroundHeader {
    pub(super) fn parse(bytes: &[u8]) -> Result<Self, PackError> {
        let bytes = bytes.get(..HEADER).ok_or(PackError::Header)?;
        if &bytes[..4] != b"LVS2" {
            return Err(PackError::Header);
        }
        let field = |index: usize| read32(bytes, 4 + index * 4).unwrap() as i32;
        let positive = |index| {
            usize::try_from(field(index))
                .ok()
                .filter(|&value| value > 0)
                .ok_or(PackError::Header)
        };
        let ground_size = [positive(3)?, positive(4)?];
        let feature_size = [positive(8)?, positive(9)?];
        if ground_size.iter().any(|&side| side < 2) || field(2) != 60 || field(7) != 6 {
            return Err(PackError::Header);
        }
        if feature_size
            .iter()
            .any(|&side| side.checked_mul(TILE_SIDE).is_none())
        {
            return Err(PackError::Bounds);
        }
        let cells = ground_size[0]
            .checked_mul(ground_size[1])
            .ok_or(PackError::Bounds)?;
        let water_offset = cells
            .checked_mul(2)
            .and_then(|length| HEADER.checked_add(length))
            .ok_or(PackError::Bounds)?;
        let ground_len = water_offset
            .checked_add(cells.div_ceil(8))
            .ok_or(PackError::Bounds)?;
        Ok(Self {
            ground_origin: [field(0), field(1)],
            ground_cell: field(2),
            ground_size,
            feature_origin: [field(5), field(6)],
            feature_cell: field(7),
            feature_size,
            water_offset,
            ground_len,
            base_metres: field(13),
        })
    }
}

/// World bounds and indexing shared by the compressed detail payload and its cache.
#[derive(Clone, Copy, Debug)]
pub struct NativeDetailLayout {
    chunk_side: usize,
    grid: [usize; 2],
    feature_origin: [i32; 2],
    feature_size: [usize; 2],
    block_metres: usize,
}

impl NativeDetailLayout {
    pub fn chunk_side(self) -> usize {
        self.chunk_side
    }

    pub fn grid_size(self) -> [usize; 2] {
        self.grid
    }

    pub fn origin(self) -> [i32; 2] {
        self.feature_origin
    }

    pub fn block_metres(self) -> usize {
        self.block_metres
    }

    pub fn chunk_count(self) -> usize {
        self.grid[0] * self.grid[1]
    }

    fn tiles_per_chunk(self) -> usize {
        self.chunk_side * self.chunk_side
    }

    fn tile_index_bytes(self) -> usize {
        (self.tiles_per_chunk() + 1) * 2
    }

    /// Convert a global 96-metre tile into its row-major chunk and local tile IDs.
    pub fn locate(self, tile_col: usize, tile_row: usize) -> Option<(usize, usize)> {
        if tile_col >= self.feature_size[0] || tile_row >= self.feature_size[1] {
            return None;
        }
        let side = self.chunk_side;
        let chunk = tile_row / side * self.grid[0] + tile_col / side;
        let tile = tile_row % side * side + tile_col % side;
        Some((chunk, tile))
    }

    /// Sample a resident decoded chunk. Only the requested palette record is read.
    pub fn word_in_chunk(self, chunk: &[u8], tile: usize, cell: usize) -> u16 {
        DecodedChunk {
            bytes: chunk,
            layout: self,
        }
        .word(tile, cell)
    }
}

#[derive(Clone, Copy)]
pub struct NativePack<'a> {
    ground: &'a [u8],
    header: GroundHeader,
    flags: u16,
    details: NativeDetails<'a>,
}

impl<'a> NativePack<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, PackError> {
        let (header, layout, flags) = pack_layout(bytes)?;
        let body = &bytes[STREAM_HEADER..];
        let ground_end = if flags & GROUND_COMPRESSED != 0 {
            let mut blocks = body.get(HEADER..).ok_or(PackError::Bounds)?;
            for _ in 0..=header.ground_size[1].div_ceil(SLAB_ROWS) {
                compressed_block(&mut blocks)?;
            }
            body.len() - blocks.len()
        } else {
            header.ground_len
        };
        let ground = body.get(..ground_end).ok_or(PackError::Bounds)?;
        let details = NativeDetails::from_storage(&body[ground_end..], layout)?;
        Ok(Self { ground, header, flags, details })
    }

    pub fn ground_len(self) -> usize {
        self.header.ground_len
    }

    pub fn ground_scratch_len(self) -> usize {
        if self.flags & GROUND_COMPRESSED == 0 {
            return 0;
        }
        self.header.ground_size[0] * self.header.ground_size[1].min(SLAB_ROWS) * 2
    }

    pub fn details(self) -> NativeDetails<'a> {
        self.details
    }

    /// Restore the original LVS2 header, absolute 16-bit heights and water bitmap.
    /// The inflater must consume the complete block and verify its checksum.
    pub fn decode_ground(
        self,
        out: &mut [u8],
        scratch: &mut [u8],
        inflate: &mut impl FnMut(&[u8], &mut [u8]) -> bool,
    ) -> Result<(), PackError> {
        let out = out.get_mut(..self.ground_len()).ok_or(PackError::Buffer)?;
        if self.flags & GROUND_COMPRESSED == 0 {
            out.copy_from_slice(self.ground);
            return Ok(());
        }
        out[..HEADER].copy_from_slice(&self.ground[..HEADER]);
        let mut blocks = &self.ground[HEADER..];
        restore_ground(self.header, out, scratch, |output| {
            inflate_block(compressed_block(&mut blocks)?, output, inflate)
        })
    }
}

fn pack_layout(bytes: &[u8]) -> Result<(GroundHeader, NativeDetailLayout, u16), PackError> {
    let prefix = bytes.get(..STREAM_HEADER).ok_or(PackError::Header)?;
    if &prefix[..4] != b"SCG1" {
        return Err(PackError::Header);
    }
    let chunk_side = usize::from(read16(prefix, 4).unwrap());
    let flags = read16(prefix, 6).unwrap();
    if !(1..=32).contains(&chunk_side) || flags & !GROUND_COMPRESSED != 0 {
        return Err(PackError::Header);
    }
    let header = GroundHeader::parse(bytes.get(STREAM_HEADER..).ok_or(PackError::Header)?)?;
    let grid = header.feature_size.map(|side| side.div_ceil(chunk_side));
    let supplied = [
        read32(prefix, 8).unwrap() as usize,
        read32(prefix, 12).unwrap() as usize,
    ];
    if grid != supplied || grid[0].checked_mul(grid[1]).is_none() {
        return Err(PackError::Header);
    }
    let layout = NativeDetailLayout {
        chunk_side,
        grid,
        feature_origin: header.feature_origin,
        feature_size: header.feature_size,
        block_metres: header.feature_cell as usize * TILE_SIDE * chunk_side,
    };
    Ok((header, layout, flags))
}

fn restore_ground(
    header: GroundHeader,
    out: &mut [u8],
    scratch: &mut [u8],
    mut next_block: impl FnMut(&mut [u8]) -> Result<(), PackError>,
) -> Result<(), PackError> {
    let [width, height] = header.ground_size;
    let scratch = scratch.get_mut(..width * height.min(SLAB_ROWS) * 2).ok_or(PackError::Buffer)?;
    for row in (0..height).step_by(SLAB_ROWS) {
        let rows = (height - row).min(SLAB_ROWS);
        let count = width * rows;
        next_block(&mut scratch[..2 * count])?;
        let (low, high) = scratch[..2 * count].split_at(count);
        let start = HEADER + (row * width) * 2;
        let elevations = &mut out[start..start + count * 2];
        for ((low, high), output) in low
            .chunks_exact(width)
            .zip(high.chunks_exact(width))
            .zip(elevations.chunks_exact_mut(width * 2))
        {
            let mut previous = 0u16;
            for ((&low, &high), output) in low.iter().zip(high).zip(output.chunks_exact_mut(2))
            {
                let zigzag = u16::from_le_bytes([low, high]);
                let delta = (zigzag >> 1) ^ 0u16.wrapping_sub(zigzag & 1);
                previous = previous.wrapping_add(delta);
                output.copy_from_slice(&previous.to_le_bytes());
            }
        }
    }
    next_block(&mut out[header.water_offset..])
}

/// A validated, copyable view over only the compressed chunk directory and payload.
#[derive(Clone, Copy)]
pub struct NativeDetails<'a> {
    storage: &'a [u8],
    directory_bytes: usize,
    layout: NativeDetailLayout,
    max_chunk_len: usize,
}

impl<'a> NativeDetails<'a> {
    pub fn from_storage(storage: &'a [u8], layout: NativeDetailLayout) -> Result<Self, PackError> {
        let directory_bytes = directory_len(layout)?;
        let directory = storage.get(..directory_bytes).ok_or(PackError::Directory)?;
        let (max_chunk_len, _) = validate_directory(directory, storage.len() - directory_bytes, layout)?;
        Ok(Self {
            storage,
            directory_bytes,
            layout,
            max_chunk_len,
        })
    }

    pub fn storage(self) -> &'a [u8] {
        self.storage
    }

    pub fn layout(self) -> NativeDetailLayout {
        self.layout
    }

    pub fn max_chunk_len(self) -> usize {
        self.max_chunk_len
    }

    pub fn chunk_len(self, index: usize) -> Option<usize> {
        if index >= self.layout.chunk_count() {
            return None;
        }
        read32(self.storage, index * ENTRY_BYTES + 4).map(|length| length as usize)
    }

    pub fn decode_chunk<'out>(
        self,
        index: usize,
        out: &'out mut [u8],
        inflate: &mut impl FnMut(&[u8], &mut [u8]) -> bool,
    ) -> Result<DecodedChunk<'out>, PackError> {
        let raw_len = self.chunk_len(index).ok_or(PackError::Bounds)?;
        let out = out.get_mut(..raw_len).ok_or(PackError::Buffer)?;
        let at = index * ENTRY_BYTES;
        let start = self.directory_bytes + read32(self.storage, at).unwrap() as usize;
        let end = self.directory_bytes + read32(self.storage, at + ENTRY_BYTES).unwrap() as usize;
        inflate_block(&self.storage[start..end], out, inflate)?;
        let chunk = DecodedChunk {
            bytes: out,
            layout: self.layout,
        };
        chunk.validate()?;
        Ok(chunk)
    }
}

fn directory_len(layout: NativeDetailLayout) -> Result<usize, PackError> {
    layout.chunk_count().checked_add(1).and_then(|n| n.checked_mul(ENTRY_BYTES)).ok_or(PackError::Bounds)
}

fn validate_directory(
    directory: &[u8], payload_len: usize, layout: NativeDetailLayout,
) -> Result<(usize, usize), PackError> {
    let length = directory_len(layout)?;
    if directory.len() != length || read32(directory, 0) != Some(0) {
        return Err(PackError::Directory);
    }
    let index_bytes = layout.tile_index_bytes();
    let mut offset = 0;
    let mut max_raw = 0;
    let mut max_encoded = 0;
    for entry in 0..layout.chunk_count() {
        let at = entry * ENTRY_BYTES;
        let raw = read32(directory, at + 4).unwrap() as usize;
        let next = read32(directory, at + ENTRY_BYTES).unwrap() as usize;
        let encoded = next.checked_sub(offset).ok_or(PackError::Directory)?;
        if !(index_bytes..=index_bytes + usize::from(u16::MAX)).contains(&raw)
            || encoded < 6 || next > payload_len
        {
            return Err(PackError::Directory);
        }
        max_raw = max_raw.max(raw);
        max_encoded = max_encoded.max(encoded);
        offset = next;
    }
    if offset != payload_len || read32(directory, length - 4) != Some(0) {
        return Err(PackError::Directory);
    }
    Ok((max_raw, max_encoded))
}

pub struct DecodedChunk<'a> {
    bytes: &'a [u8],
    layout: NativeDetailLayout,
}

impl<'a> DecodedChunk<'a> {
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    fn validate(&self) -> Result<(), PackError> {
        let index_bytes = self.layout.tile_index_bytes();
        if read16(self.bytes, 0) != Some(0) || self.bytes.len() < index_bytes {
            return Err(PackError::Tile);
        }
        let mut start = 0;
        let records = &self.bytes[index_bytes..];
        for entry in self.bytes[2..index_bytes].chunks_exact(2) {
            let end = usize::from(u16::from_le_bytes([entry[0], entry[1]]));
            let record = records.get(start..end).ok_or(PackError::Tile)?;
            if !record.is_empty() && palette_width(record).is_none() {
                return Err(PackError::Tile);
            }
            start = end;
        }
        if start != records.len() {
            return Err(PackError::Tile);
        }
        Ok(())
    }

    /// The unchanged LVS2 palette record for a local tile. Empty tiles are slices of length zero.
    pub fn tile(&self, index: usize) -> Option<&'a [u8]> {
        if index >= self.layout.tiles_per_chunk() {
            return None;
        }
        let table_end = self.layout.tile_index_bytes();
        let start = table_end + usize::from(read16(self.bytes, index * 2)?);
        let end = table_end + usize::from(read16(self.bytes, index * 2 + 2)?);
        self.bytes.get(start..end)
    }

    pub fn word(&self, tile: usize, cell: usize) -> u16 {
        const CELLS: usize = TILE_SIDE * TILE_SIDE;
        if cell >= CELLS {
            return 0;
        }
        let Some(record) = self.tile(tile) else {
            return 0;
        };
        let Some((colors, width)) = palette_width(record) else {
            return 0;
        };
        let color = if width == 0 {
            0
        } else {
            let bit = cell * width;
            let at = 1 + 2 * colors + bit / 8;
            let shift = bit & 7;
            let mut word = usize::from(record[at]);
            if shift + width > 8 {
                word |= usize::from(record[at + 1]) << 8;
            }
            ((word >> shift) & ((1 << width) - 1)).min(colors - 1)
        };
        read16(record, 1 + color * 2).unwrap()
    }
}

fn palette_width(record: &[u8]) -> Option<(usize, usize)> {
    let last = *record.first()?;
    let colors = usize::from(last) + 1;
    let width = (u8::BITS - last.leading_zeros()) as usize;
    (record.len() == 1 + 2 * colors + TILE_SIDE * TILE_SIDE * width / 8).then_some((colors, width))
}

fn compressed_block<'a>(remaining: &mut &'a [u8]) -> Result<&'a [u8], PackError> {
    let size = read32(remaining, 0).ok_or(PackError::Bounds)? as usize;
    let body = remaining.get(4..).ok_or(PackError::Bounds)?;
    if size < 6 {
        return Err(PackError::Bounds);
    }
    let (block, tail) = body.split_at_checked(size).ok_or(PackError::Bounds)?;
    *remaining = tail;
    Ok(block)
}

fn inflate_block(
    input: &[u8],
    output: &mut [u8],
    inflate: &mut impl FnMut(&[u8], &mut [u8]) -> bool,
) -> Result<(), PackError> {
    if inflate(input, output) {
        Ok(())
    } else {
        Err(PackError::Deflate)
    }
}

fn read16(source: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        source
            .get(offset..offset.checked_add(2)?)?
            .try_into()
            .ok()?,
    ))
}

fn read32(source: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        source
            .get(offset..offset.checked_add(4)?)?
            .try_into()
            .ok()?,
    ))
}
