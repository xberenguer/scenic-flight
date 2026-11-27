//! Lossless tile palettes with bounded, allocation-free cell lookups.

use super::TILE_SIDE;

const CELLS: usize = TILE_SIDE * TILE_SIDE;

#[derive(Clone, Copy, Debug)]
pub(super) struct PaletteTiles {
    offsets: usize,
    payload: usize,
}

impl PaletteTiles {
    pub(super) fn parse(bytes: &[u8], offsets: usize, count: usize) -> Option<Self> {
        let payload = offsets.checked_add(count.checked_add(1)?.checked_mul(4)?)?;
        let mut index = bytes.get(offsets..payload)?.chunks_exact(4).map(offset);
        if index.next()? != 0 { return None; }
        let records = bytes.get(payload..)?;
        let mut start = 0;
        for end in index {
            let record = records.get(start..end)?;
            let &last = record.first()?;
            let colors = usize::from(last) + 1;
            let width = bits(last);
            if record.len() != 1 + 2 * colors + CELLS * width / 8 { return None; }
            start = end;
        }
        (start == records.len()).then_some(Self { offsets, payload })
    }

    pub(super) fn sample(self, bytes: &[u8], tile: usize, cell: usize) -> u16 {
        let index = self.offsets + tile * 4;
        let start = self.payload + offset(&bytes[index..index + 4]);
        let last = bytes[start];
        let width = bits(last);
        let color = if width == 0 {
            0
        } else {
            let bit = cell * width;
            let at = start + 1 + 2 * (usize::from(last) + 1) + bit / 8;
            let shift = bit & 7;
            let mut word = usize::from(bytes[at]);
            if shift + width > 8 { word |= usize::from(bytes[at + 1]) << 8; }
            ((word >> shift) & ((1 << width) - 1)).min(usize::from(last))
        };
        let at = start + 1 + color * 2;
        u16::from_le_bytes([bytes[at], bytes[at + 1]])
    }
}

fn bits(last_palette_index: u8) -> usize {
    (u8::BITS - last_palette_index.leading_zeros()) as usize
}

fn offset(bytes: &[u8]) -> usize {
    u32::from_le_bytes(bytes.try_into().unwrap()) as usize
}
