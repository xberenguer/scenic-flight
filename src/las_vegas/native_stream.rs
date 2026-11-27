//! Random-access SCG1 reader for assets larger than the firmware partition.

use super::{
    DecodedChunk, ENTRY_BYTES, GROUND_COMPRESSED, GroundHeader, HEADER, NativeDetailLayout,
    PackError, SLAB_ROWS, STREAM_HEADER, directory_len, inflate_block, pack_layout,
    read32, restore_ground, validate_directory,
};

pub struct NativeFile {
    length: usize,
    coordinate_header: [u8; HEADER],
    ground: GroundHeader,
    layout: NativeDetailLayout,
    flags: u16,
    detail_offset: usize,
    ground_encoded: usize,
}

impl NativeFile {
    pub fn open(length: usize, mut read: impl FnMut(usize, &mut [u8]) -> bool) -> Result<Self, PackError> {
        let mut prefix = [0; STREAM_HEADER + HEADER];
        read_range(length, 0, &mut prefix, &mut read)?;
        let (ground, layout, flags) = pack_layout(&prefix)?;
        let mut ground_encoded = 0;
        let detail_offset = if flags & GROUND_COMPRESSED != 0 {
            let mut block = prefix.len();
            for _ in 0..=ground.ground_size[1].div_ceil(SLAB_ROWS) {
                let size = block_size(length, block, &mut read)?;
                ground_encoded = ground_encoded.max(size);
                block += 4 + size;
            }
            block
        } else {
            STREAM_HEADER.checked_add(ground.ground_len).ok_or(PackError::Bounds)?
        };
        if detail_offset.checked_add(directory_len(layout)?).is_none_or(|end| end > length) {
            return Err(PackError::Directory);
        }
        Ok(Self {
            length,
            coordinate_header: prefix[STREAM_HEADER..].try_into().unwrap(),
            ground, layout, flags, detail_offset, ground_encoded,
        })
    }

    pub fn ground_len(&self) -> usize {
        self.ground.ground_len
    }

    pub fn ground_scratch_len(&self) -> usize {
        self.ground.ground_size[0] * self.ground.ground_size[1].min(SLAB_ROWS) * 2
    }

    pub fn ground_input_len(&self) -> usize {
        self.ground_encoded
    }

    pub fn directory_len(&self) -> usize {
        directory_len(self.layout).expect("validated SCG1 directory")
    }

    pub fn decode_ground(
        &self, out: &mut [u8], slab: &mut [u8], encoded: &mut [u8],
        mut read: impl FnMut(usize, &mut [u8]) -> bool,
        inflate: &mut impl FnMut(&[u8], &mut [u8]) -> bool,
    ) -> Result<(), PackError> {
        let out = out.get_mut(..self.ground_len()).ok_or(PackError::Buffer)?;
        if self.flags & GROUND_COMPRESSED == 0 {
            return read_range(self.length, STREAM_HEADER, out, &mut read);
        }
        out[..HEADER].copy_from_slice(&self.coordinate_header);
        let mut block = STREAM_HEADER + HEADER;
        restore_ground(self.ground, out, slab, |output| {
            let size = block_size(self.detail_offset, block, &mut read)?;
            let input = encoded.get_mut(..size).ok_or(PackError::Buffer)?;
            read_range(self.detail_offset, block + 4, input, &mut read)?;
            block += 4 + size;
            inflate_block(input, output, inflate)
        })
    }

    pub fn details<'a>(
        &self, directory: &'a mut [u8], mut read: impl FnMut(usize, &mut [u8]) -> bool,
    ) -> Result<NativeFileDetails<'a>, PackError> {
        let directory = directory.get_mut(..self.directory_len()).ok_or(PackError::Buffer)?;
        read_range(self.length, self.detail_offset, directory, &mut read)?;
        let payload = self.detail_offset + directory.len();
        let (max_chunk_len, max_input_len) = validate_directory(directory, self.length - payload, self.layout)?;
        Ok(NativeFileDetails { directory, payload, layout: self.layout, max_chunk_len, max_input_len })
    }
}

pub struct NativeFileDetails<'a> {
    directory: &'a [u8],
    payload: usize,
    layout: NativeDetailLayout,
    max_chunk_len: usize,
    max_input_len: usize,
}

impl NativeFileDetails<'_> {
    pub fn layout(&self) -> NativeDetailLayout { self.layout }
    pub fn max_chunk_len(&self) -> usize { self.max_chunk_len }
    pub fn max_input_len(&self) -> usize { self.max_input_len }

    pub fn decode_chunk<'out>(
        &self, index: usize, out: &'out mut [u8], encoded: &mut [u8],
        mut read: impl FnMut(usize, &mut [u8]) -> bool,
        inflate: &mut impl FnMut(&[u8], &mut [u8]) -> bool,
    ) -> Result<DecodedChunk<'out>, PackError> {
        if index >= self.layout.chunk_count() {
            return Err(PackError::Bounds);
        }
        let at = index * ENTRY_BYTES;
        let start = read32(self.directory, at).unwrap() as usize;
        let end = read32(self.directory, at + ENTRY_BYTES).unwrap() as usize;
        let raw = read32(self.directory, at + 4).unwrap() as usize;
        let input = encoded.get_mut(..end - start).ok_or(PackError::Buffer)?;
        if !read(self.payload + start, input) {
            return Err(PackError::Read);
        }
        let out = out.get_mut(..raw).ok_or(PackError::Buffer)?;
        inflate_block(input, out, inflate)?;
        let chunk = DecodedChunk { bytes: out, layout: self.layout };
        chunk.validate()?;
        Ok(chunk)
    }
}

fn read_range(
    len: usize, offset: usize, out: &mut [u8], read: &mut impl FnMut(usize, &mut [u8]) -> bool,
) -> Result<(), PackError> {
    if offset.checked_add(out.len()).is_none_or(|end| end > len) {
        return Err(PackError::Bounds);
    }
    if !read(offset, out) {
        return Err(PackError::Read);
    }
    Ok(())
}

fn block_size(
    len: usize, offset: usize, read: &mut impl FnMut(usize, &mut [u8]) -> bool,
) -> Result<usize, PackError> {
    let mut bytes = [0; 4];
    read_range(len, offset, &mut bytes, read)?;
    let size = u32::from_le_bytes(bytes) as usize;
    if size < 6 || size > len - offset - 4 {
        return Err(PackError::Bounds);
    }
    Ok(size)
}
