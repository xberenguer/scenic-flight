//! Resident overview and an explicitly prefetched window of original Southwest survey tiles.

use core::cell::RefCell;
use critical_section::Mutex;
use scenic_flight::world::{
    AtlasElevation,
    atlas_ground_tiles::{GroundTileError, GroundTileLayout, GroundTiles, TILE_CELLS, TILE_SIDE},
};

const WINDOW: usize = 5;
pub const RESIDENT_TILES: usize = WINDOW * WINDOW;
pub const RESIDENT_CELLS: usize = RESIDENT_TILES * TILE_CELLS;
const OVERVIEW_HEADER: usize = 28;
const OVERVIEW_SAMPLE: usize = 4;
const SURVEY_EDGE_BLEND: f32 = 9_000.0;
const PREFETCH_ORDER: [[i32; 2]; RESIDENT_TILES] = prefetch_order();

#[derive(Clone, Copy)]
struct GroundPoint {
    cell: [usize; 2],
    fraction: [f32; 2],
    edge: f32,
}

impl GroundPoint {
    fn new(layout: GroundTileLayout, x: f32, z: f32) -> Option<Self> {
        let [west, north] = layout.origin();
        let spacing = layout.spacing() as f32;
        let [width, height] = layout.size();
        let east = (x - west as f32) / spacing - 0.5;
        let south = (north as f32 - z) / spacing - 0.5;
        if !east.is_finite() || !south.is_finite() {
            return None;
        }
        let edge = east
            .min(south)
            .min((width - 1) as f32 - east)
            .min((height - 1) as f32 - south);
        if edge <= 0.0 {
            return None;
        }
        let cell = [east as usize, south as usize];
        Some(Self {
            cell,
            fraction: [east - cell[0] as f32, south - cell[1] as f32],
            edge: edge * spacing,
        })
    }
}

struct Overview {
    bytes: &'static [u8],
    stride: usize,
    samples: [usize; 2],
    fine_size: [usize; 2],
}

impl Overview {
    fn parse(bytes: &'static [u8], layout: GroundTileLayout) -> Option<Self> {
        if bytes.get(..4)? != b"SWC1" {
            return None;
        }
        let field = |offset| {
            Some(i32::from_le_bytes(
                bytes.get(offset..offset + 4)?.try_into().ok()?,
            ))
        };
        let [width, height] = layout.size();
        if [field(4)?, field(8)?] != layout.origin()
            || field(12)? != layout.spacing()
            || usize::try_from(field(16)?).ok()? != width
            || usize::try_from(field(20)?).ok()? != height
        {
            return None;
        }
        let stride = usize::try_from(field(24)?).ok()?;
        if !(2..=64).contains(&stride) {
            return None;
        }
        let samples = layout.size().map(|side| (side - 1).div_ceil(stride) + 1);
        let payload = samples[0]
            .checked_mul(samples[1])?
            .checked_mul(OVERVIEW_SAMPLE)?;
        if bytes.len() != OVERVIEW_HEADER.checked_add(payload)? {
            return None;
        }
        Some(Self {
            bytes,
            stride,
            samples,
            fine_size: layout.size(),
        })
    }

    fn height(&self, point: GroundPoint, conservative: bool) -> f32 {
        let [x, z] = point.cell;
        let [col, row] = [x / self.stride, z / self.stride];
        let index = row * self.samples[0] + col;
        let at = |index: usize| {
            let offset = OVERVIEW_HEADER + index * OVERVIEW_SAMPLE + usize::from(conservative) * 2;
            f32::from(i16::from_le_bytes([
                self.bytes[offset],
                self.bytes[offset + 1],
            ]))
        };
        let fraction = core::array::from_fn(|axis| {
            let cell = point.cell[axis];
            let lower = cell / self.stride * self.stride;
            let upper = (lower + self.stride).min(self.fine_size[axis] - 1);
            ((cell - lower) as f32 + point.fraction[axis]) / (upper - lower) as f32
        });
        bilinear(
            [
                at(index),
                at(index + 1),
                at(index + self.samples[0]),
                at(index + self.samples[0] + 1),
            ],
            fraction,
        )
    }
}

struct Residents {
    data: &'static mut [u16],
    slots: [usize; RESIDENT_TILES],
    eye: [f32; 2],
}

impl Residents {
    fn tile(&self, index: usize, columns: usize) -> Option<&[u16]> {
        let position = slot(index, columns);
        let start = position * TILE_CELLS;
        (self.slots[position] == index).then(|| &self.data[start..start + TILE_CELLS])
    }

    fn height(&self, layout: GroundTileLayout, col: usize, row: usize) -> Option<f32> {
        let (tile, cell) = layout.locate(col, row)?;
        Some(f32::from(self.tile(tile, layout.grid()[0])?[cell] as i16))
    }

    fn interpolate(&self, layout: GroundTileLayout, point: GroundPoint) -> Option<f32> {
        let [col, row] = point.cell;
        // Almost every bilinear quad is entirely inside a single tile.
        if col % TILE_SIDE < TILE_SIDE - 1 && row % TILE_SIDE < TILE_SIDE - 1 {
            let (tile, cell) = layout.locate(col, row)?;
            let [width, _] = layout.tile_dimensions(tile)?;
            let heights = self.tile(tile, layout.grid()[0])?;
            let sample = |offset| f32::from(heights[cell + offset] as i16);
            return Some(bilinear(
                [sample(0), sample(1), sample(width), sample(width + 1)],
                point.fraction,
            ));
        }
        Some(bilinear(
            [
                self.height(layout, col, row)?,
                self.height(layout, col + 1, row)?,
                self.height(layout, col, row + 1)?,
                self.height(layout, col + 1, row + 1)?,
            ],
            point.fraction,
        ))
    }
}

/// A source-matched map whose sampling path relies on prefetched tiles and the overview.
pub struct CorridorGround {
    layout: GroundTileLayout,
    overview: Overview,
    residents: Mutex<RefCell<Residents>>,
}

impl CorridorGround {
    pub fn new(
        layout: GroundTileLayout,
        overview: &'static [u8],
        resident: &'static mut [u16],
    ) -> Option<Self> {
        if resident.len() != RESIDENT_CELLS {
            return None;
        }
        Some(Self {
            layout,
            overview: Overview::parse(overview, layout)?,
            residents: Mutex::new(RefCell::new(Residents {
                data: resident,
                slots: [usize::MAX; RESIDENT_TILES],
                eye: [f32::INFINITY; 2],
            })),
        })
    }

    fn center(&self, [x, z]: [f32; 2]) -> [i32; 2] {
        let [west, north] = self.layout.origin();
        let span = self.layout.spacing() as f32 * TILE_SIDE as f32;
        let half_sample = self.layout.spacing() as f32 * 0.5;
        let [cols, rows] = self.layout.grid();
        let offset = [
            x - west as f32 - half_sample,
            north as f32 - z - half_sample,
        ];
        core::array::from_fn(|axis| {
            // Leave a two-tile prefetch margin beyond the edges of the survey.
            let at = (offset[axis] / span).clamp(-3.0, [cols, rows][axis] as f32 + 2.0);
            let whole = at as i32;
            whole - i32::from(at < whole as f32)
        })
    }

    fn set_eye(&self, position: [f32; 2]) {
        critical_section::with(|cs| self.residents.borrow_ref_mut(cs).eye = position);
    }

    fn contains(&self, tile: usize) -> bool {
        let position = slot(tile, self.layout.grid()[0]);
        critical_section::with(|cs| self.residents.borrow_ref(cs).slots[position] == tile)
    }

    fn insert(&self, tile: usize, heights: &[i16]) {
        let position = slot(tile, self.layout.grid()[0]);
        let start = position * TILE_CELLS;
        critical_section::with(|cs| {
            let mut resident = self.residents.borrow_ref_mut(cs);
            for (target, source) in resident.data[start..start + heights.len()]
                .iter_mut()
                .zip(heights)
            {
                *target = *source as u16;
            }
            resident.slots[position] = tile;
        });
    }
}

impl AtlasElevation for CorridorGround {
    fn sample(&self, x: f32, z: f32) -> Option<(f32, f32)> {
        let point = GroundPoint::new(self.layout, x, z)?;
        let (detail, weight) = critical_section::with(|cs| {
            let resident = self.residents.borrow_ref(cs);
            let distance = (x - resident.eye[0]).abs().max((z - resident.eye[1]).abs());
            // Fade outside 1.5 tiles and finish before a two-tile cache
            // boundary. Prefetch has time to fill the next incoming column.
            let span = self.layout.spacing() as f32 * TILE_SIDE as f32;
            let weight = smooth((1.95 * span - distance) / (0.45 * span));
            let detail = (weight > 0.0)
                .then(|| resident.interpolate(self.layout, point))
                .flatten();
            (detail, weight)
        });
        let height = match detail {
            Some(detail) if weight == 1.0 => detail,
            Some(detail) => lerp(self.overview.height(point, false), detail, weight),
            None => self.overview.height(point, false),
        };
        Some((height, smooth(point.edge / SURVEY_EDGE_BLEND)))
    }

    fn safety_height(&self, x: f32, z: f32) -> Option<f32> {
        let point = GroundPoint::new(self.layout, x, z)?;
        let exact = critical_section::with(|cs| {
            self.residents
                .borrow_ref(cs)
                .interpolate(self.layout, point)
        });
        Some(exact.unwrap_or_else(|| self.overview.height(point, true)))
    }
}

pub struct CorridorScratch<'a> {
    pub encoded: &'a mut [u8],
    pub planes: &'a mut [u8],
    pub heights: &'a mut [i16],
}

pub struct CorridorPrefetch {
    anchor: [i32; 2],
    next: usize,
}

impl CorridorPrefetch {
    pub fn new() -> Self {
        Self {
            anchor: [i32::MIN; 2],
            next: 0,
        }
    }

    /// Read at most budget missing tiles. A failing tile is skipped until
    /// a new window is requested, while its surveyed overview is still available.
    pub fn fill(
        &mut self,
        cache: &CorridorGround,
        tiles: &GroundTiles<'_>,
        position: [f32; 2],
        budget: usize,
        scratch: &mut CorridorScratch<'_>,
        read: &mut impl FnMut(usize, &mut [u8]) -> bool,
        inflate: &mut impl FnMut(&[u8], &mut [u8]) -> bool,
    ) -> Result<usize, GroundTileError> {
        if cache.layout != tiles.layout() {
            return Err(GroundTileError::Header);
        }
        if position.iter().any(|value| !value.is_finite()) {
            return Ok(0);
        }
        cache.set_eye(position);
        let anchor = cache.center(position);
        if anchor != self.anchor {
            self.anchor = anchor;
            self.next = 0;
        }
        let [columns, rows] = cache.layout.grid();
        let mut loaded = 0;
        while self.next < RESIDENT_TILES && loaded < budget {
            let [dx, dz] = PREFETCH_ORDER[self.next];
            self.next += 1;
            let x = (anchor[0] + dx) as usize;
            let z = (anchor[1] + dz) as usize;
            if x >= columns || z >= rows {
                continue;
            }
            let tile = z * columns + x;
            if !cache.contains(tile) {
                let heights = tiles.decode_tile(
                    tile,
                    scratch.encoded,
                    scratch.planes,
                    scratch.heights,
                    read,
                    inflate,
                )?;
                cache.insert(tile, heights);
                loaded += 1;
            }
        }
        Ok(loaded)
    }
}

fn slot(tile: usize, columns: usize) -> usize {
    tile / columns % WINDOW * WINDOW + tile % columns % WINDOW
}

const fn prefetch_order() -> [[i32; 2]; RESIDENT_TILES] {
    let mut offsets = [[0; 2]; RESIDENT_TILES];
    let mut cursor = 0;
    let mut ring = 0;
    while ring <= (WINDOW / 2) as i32 {
        let mut z = -ring;
        while z <= ring {
            let mut x = -ring;
            while x <= ring {
                if x == -ring || x == ring || z == -ring || z == ring {
                    offsets[cursor] = [x, z];
                    cursor += 1;
                }
                x += 1;
            }
            z += 1;
        }
        ring += 1;
    }
    offsets
}

fn bilinear([nw, ne, sw, se]: [f32; 4], [east, south]: [f32; 2]) -> f32 {
    lerp(lerp(nw, ne, east), lerp(sw, se, east), south)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smooth(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
