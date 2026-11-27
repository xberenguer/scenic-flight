//! OpenStreetMap motorway pavement on the Southwest atlas ground.

use super::Surface;

const HEADER_BYTES: usize = 28;
const CELL_METRES: f32 = 512.0;
const PAGE_SIDE: usize = 8;
const PAGE_ENTRIES: usize = PAGE_SIDE * PAGE_SIDE + 1;
const SEGMENT_BYTES: usize = 16;
const MAX_CELL_REFERENCES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtlasRoads {
    bytes: &'static [u8],
    west: i32,
    south: i32,
    cols: usize,
    rows: usize,
    index: usize,
    references: usize,
    segments: usize,
}

impl AtlasRoads {
    /// Parse an uncompressed SRD1 layer, validating every range before sampling.
    pub fn parse(bytes: &'static [u8]) -> Option<Self> {
        let header = bytes.get(..HEADER_BYTES)?;
        if &header[..4] != b"SRD1" || read16(header, 18) != 0 {
            return None;
        }
        let west = read32(header, 4) as i32;
        let south = read32(header, 8) as i32;
        let cols = usize::from(read16(header, 12));
        let rows = usize::from(read16(header, 14));
        let pages = usize::from(read16(header, 16));
        let segment_count = read32(header, 20) as usize;
        let reference_count = read32(header, 24) as usize;
        let cells = cols.checked_mul(rows)?;
        if pages == 0 || pages > cells || !(1..=65_536).contains(&segment_count) {
            return None;
        }
        let index = HEADER_BYTES.checked_add(cells.checked_mul(2)?)?;
        let references = index.checked_add(pages.checked_mul(PAGE_ENTRIES * 4)?)?;
        let segments = references.checked_add(reference_count.checked_mul(2)?)?;
        if segments.checked_add(segment_count.checked_mul(SEGMENT_BYTES)?)? != bytes.len() {
            return None;
        }

        // The page IDs are written in directory order. Zero marks empty terrain.
        let mut expected_page = 1;
        for entry in bytes[HEADER_BYTES..index].chunks_exact(2) {
            let page = usize::from(read16(entry, 0));
            if page != 0 {
                if page != expected_page {
                    return None;
                }
                expected_page += 1;
            }
        }
        if expected_page != pages + 1 {
            return None;
        }

        let mut last = 0;
        for page in bytes[index..references].chunks_exact(PAGE_ENTRIES * 4) {
            if read32(page, 0) as usize != last {
                return None;
            }
            for entry in page[4..].chunks_exact(4) {
                let end = read32(entry, 0) as usize;
                if end > reference_count || end.checked_sub(last)? > MAX_CELL_REFERENCES {
                    return None;
                }
                last = end;
            }
        }
        if last != reference_count
            || bytes[references..segments]
                .chunks_exact(2)
                .any(|reference| usize::from(read16(reference, 0)) >= segment_count)
        {
            return None;
        }
        for segment in bytes[segments..].chunks_exact(SEGMENT_BYTES) {
            if (read16(segment, 8) == 0 && read16(segment, 10) == 0)
                || segment[12] == 0
                || segment[12] > segment[13]
                || segment[14] == 0
                || segment[15] > 3
            {
                return None;
            }
        }
        Some(Self {
            bytes,
            west,
            south,
            cols,
            rows,
            index,
            references,
            segments,
        })
    }

    /// Test the pavement and shoulder at an atlas position, without changing elevation.
    pub fn surface(self, x: f32, z: f32) -> Option<Surface> {
        let east = (x - self.west as f32) / CELL_METRES;
        let north = (z - self.south as f32) / CELL_METRES;
        if !(0.0..(self.cols * PAGE_SIDE) as f32).contains(&east)
            || !(0.0..(self.rows * PAGE_SIDE) as f32).contains(&north)
        {
            return None;
        }
        let [col, row] = [east as usize, north as usize];
        let page_index = row / PAGE_SIDE * self.cols + col / PAGE_SIDE;
        let page = usize::from(read16(self.bytes, HEADER_BYTES + page_index * 2)).checked_sub(1)?;
        let cell = row % PAGE_SIDE * PAGE_SIDE + col % PAGE_SIDE;
        let start = self.index + (page * PAGE_ENTRIES + cell) * 4;
        let references_at = |at| self.references + read32(self.bytes, at) as usize * 2;
        let mut shoulder = false;
        for reference in self.bytes[references_at(start)..references_at(start + 4)].chunks_exact(2)
        {
            let segment = self.segments + usize::from(read16(reference, 0)) * SEGMENT_BYTES;
            let x0 = read32(self.bytes, segment) as i32 as f32;
            let z0 = read32(self.bytes, segment + 4) as i32 as f32;
            let dx = f32::from(read16(self.bytes, segment + 8) as i16);
            let dz = f32::from(read16(self.bytes, segment + 10) as i16);
            let east = x - x0;
            let north = z - z0;
            let outer = f32::from(self.bytes[segment + 13]) * 0.25;
            if east < dx.min(0.0) - outer
                || east > dx.max(0.0) + outer
                || north < dz.min(0.0) - outer
                || north > dz.max(0.0) + outer
            {
                continue;
            }
            let progress = ((east * dx + north * dz) / (dx * dx + dz * dz)).clamp(0.0, 1.0);
            let de = east - progress * dx;
            let dn = north - progress * dz;
            let distance = de * de + dn * dn;
            let asphalt = f32::from(self.bytes[segment + 12]) * 0.25;
            if distance <= asphalt * asphalt {
                return Some(Surface::Pavement);
            }
            shoulder |= distance <= outer * outer;
        }
        shoulder.then_some(Surface::Gravel)
    }
}

fn read16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}
