//! Aircraft texels at about three panel pixels, composited over untouched scenery.

use crate::bands::{Band, SIDE};

pub const GRID_SIDE: usize = 156;
pub const GRID_PIXELS: usize = GRID_SIDE * GRID_SIDE;
const MASK_WORDS: usize = GRID_SIDE.div_ceil(u32::BITS as usize);

const fn panel_edges() -> [usize; GRID_SIDE + 1] {
    let mut edges = [0; GRID_SIDE + 1];
    let mut index = 1;
    while index <= GRID_SIDE {
        // Inverse of the panel's nearest-neighbor sample at pixel centers.
        edges[index] = (2 * SIDE * index - GRID_SIDE).div_ceil(2 * GRID_SIDE);
        index += 1;
    }
    edges
}

const EDGES: [usize; GRID_SIDE + 1] = panel_edges();

pub struct MediumGrid {
    columns: [[u32; MASK_WORDS]; GRID_SIDE],
    first: usize,
    end: usize,
}

impl MediumGrid {
    pub const fn new() -> Self {
        Self { columns: [[0; MASK_WORDS]; GRID_SIDE], first: GRID_SIDE, end: 0 }
    }

    pub fn clear(&mut self) {
        if self.first < self.end {
            self.columns[self.first..self.end].fill([0; MASK_WORDS]);
            self.first = GRID_SIDE;
            self.end = 0;
        }
    }

    pub fn touch(&mut self, x: usize, y: usize) -> bool {
        let word = &mut self.columns[x][y / u32::BITS as usize];
        let bit = 1 << (y % u32::BITS as usize);
        let was_clear = *word & bit == 0;
        *word |= bit;
        self.first = self.first.min(x);
        self.end = self.end.max(x + 1);
        was_clear
    }

    pub fn pack(&self, pixels: &[u16], target: &mut [u8], band: Band) {
        if self.first >= self.end {
            return;
        }
        let first_row = band.y.max(EDGES[self.first]);
        let last_row = (band.y + band.height).min(EDGES[self.end]);
        let last_column = band.x + band.width;
        for row in first_row..last_row {
            let x = ((2 * row + 1) * GRID_SIDE) / (2 * SIDE);
            let row_offset = (row - band.y) * band.width;
            for (index, &word) in self.columns[x].iter().enumerate() {
                let mut remaining = word;
                while remaining != 0 {
                    let y = index * u32::BITS as usize + remaining.trailing_zeros() as usize;
                    remaining &= remaining - 1;
                    // Panel columns walk up the original viewport.
                    let first = band.x.max(SIDE - EDGES[y + 1]);
                    let end = last_column.min(SIDE - EDGES[y]);
                    if first < end {
                        let ink = pixels[x * GRID_SIDE + y].to_be_bytes();
                        let from = 2 * (row_offset + first - band.x);
                        let to = 2 * (row_offset + end - band.x);
                        for output in target[from..to].chunks_exact_mut(2) {
                            output.copy_from_slice(&ink);
                        }
                    }
                }
            }
        }
    }
}
