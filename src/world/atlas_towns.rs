//! Sparse surveyed town footprints and streets over the common atlas elevation.

use super::{Sample, Surface};

const HEADER_BYTES: usize = 16;
const PATCH_BYTES: usize = 16;
const TILE: usize = 16;
const TILE_PIXELS: usize = TILE * TILE;
const CELL_METRES: f32 = 4.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtlasTowns {
    bytes: &'static [u8],
    index: usize,
    offsets: usize,
    records: usize,
}

#[derive(Clone, Copy)]
struct Patch {
    west: i32,
    south: i32,
    cols: usize,
    rows: usize,
    index: usize,
}

impl Patch {
    fn read(bytes: &[u8]) -> Self {
        Self {
            west: read32(bytes, 0) as i32,
            south: read32(bytes, 4) as i32,
            cols: usize::from(read16(bytes, 8)),
            rows: usize::from(read16(bytes, 10)),
            index: read32(bytes, 12) as usize,
        }
    }

    fn cell(self, x: f32, z: f32) -> Option<[usize; 2]> {
        let east = (x - self.west as f32) / CELL_METRES;
        let north = (z - self.south as f32) / CELL_METRES;
        if !(0.0..(self.cols * TILE) as f32).contains(&east)
            || !(0.0..(self.rows * TILE) as f32).contains(&north)
        {
            return None;
        }
        Some([east as usize, north as usize])
    }
}

impl AtlasTowns {
    /// Parse the uncompressed STN1 asset. Town data uses separate immutable storage.
    pub fn parse(bytes: &'static [u8]) -> Option<Self> {
        let header = bytes.get(..HEADER_BYTES)?;
        if &header[..4] != b"STN1" || read16(header, 6) != CELL_METRES as u16 {
            return None;
        }
        let patch_count = usize::from(read16(header, 4));
        let tile_slots = read32(header, 8) as usize;
        let tile_count = read32(header, 12) as usize;
        if patch_count == 0 || !(1..=65_535).contains(&tile_count) {
            return None;
        }
        let index = HEADER_BYTES.checked_add(patch_count.checked_mul(PATCH_BYTES)?)?;
        let offsets = index.checked_add(tile_slots.checked_mul(2)?)?;
        let records = offsets.checked_add((tile_count + 1).checked_mul(4)?)?;
        let data = bytes.get(records..)?;
        let mut expected_tile = 0usize;
        for raw in bytes[HEADER_BYTES..index].chunks_exact(PATCH_BYTES) {
            let patch = Patch::read(raw);
            if patch.index != expected_tile || patch.cols == 0 || patch.rows == 0 {
                return None;
            }
            expected_tile = expected_tile.checked_add(patch.cols.checked_mul(patch.rows)?)?;
        }
        if expected_tile != tile_slots
            || bytes[index..offsets]
                .chunks_exact(2)
                .any(|tile| usize::from(read16(tile, 0)) > tile_count)
        {
            return None;
        }
        if read32(bytes, offsets) != 0 {
            return None;
        }
        let mut start = 0;
        for offset in bytes[offsets + 4..records].chunks_exact(4) {
            let end = read32(offset, 0) as usize;
            let record = data.get(start..end)?;
            let (&last, _) = record.split_first()?;
            let colors = usize::from(last) + 1;
            let after_palette = 1 + colors * 2;
            if after_palette + TILE_PIXELS * bit_width(last) / 8 != record.len() {
                return None;
            }
            for word in record[1..after_palette].chunks_exact(2) {
                let word = read16(word, 0);
                if word & 15 > 8 || (word & 15 < 6 && (word >> 4) != 0) {
                    return None;
                }
            }
            start = end;
        }
        if start != data.len() {
            return None;
        }
        Some(Self {
            bytes,
            index,
            offsets,
            records,
        })
    }

    fn patch_at(self, x: f32, z: f32) -> Option<(Patch, [usize; 2])> {
        self.bytes[HEADER_BYTES..self.index]
            .chunks_exact(PATCH_BYTES)
            .map(Patch::read)
            .find_map(|patch| patch.cell(x, z).map(|cell| (patch, cell)))
    }

    /// True throughout the surveyed town window, including unbuilt ground.
    pub fn covers(self, x: f32, z: f32) -> bool {
        self.patch_at(x, z).is_some()
    }

    pub fn overlay(self, x: f32, z: f32, base: Sample) -> Option<Sample> {
        let (patch, [east, north]) = self.patch_at(x, z)?;
        let index = patch.index + north / TILE * patch.cols + east / TILE;
        let tile = usize::from(read16(self.bytes, self.index + index * 2)).checked_sub(1)?;
        let start = self.records + read32(self.bytes, self.offsets + tile * 4) as usize;
        let end = self.records + read32(self.bytes, self.offsets + (tile + 1) * 4) as usize;
        let record = &self.bytes[start..end];
        let last = record[0];
        let width = bit_width(last);
        let color = if width == 0 {
            0
        } else {
            let bit = (north % TILE * TILE + east % TILE) * width;
            let offset = 1 + (usize::from(last) + 1) * 2 + bit / 8;
            let shift = bit & 7;
            let word = if shift + width > 8 {
                read16(record, offset)
            } else {
                u16::from(record[offset])
            };
            usize::from((word >> shift) & ((1 << width) - 1)).min(usize::from(last))
        };
        let word = read16(record, 1 + color * 2);
        let surface = match word & 15 {
            1 => Surface::Pavement,
            2 => Surface::Gravel,
            3 => Surface::RailMetal,
            4 => Surface::UrbanLot,
            5 => Surface::Park,
            6 => Surface::FlatRoof,
            7 => Surface::PitchedRoof,
            8 => Surface::IndustryMetal,
            _ => return None,
        };
        Some(Sample {
            surface,
            elevation: base.elevation + f32::from(word >> 4) * 0.1,
        })
    }
}

fn bit_width(last: u8) -> usize {
    (u8::BITS - last.leading_zeros()) as usize
}

fn read16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}
