//! Native aircraft tiles and interface commands for one submitted frame.

use scenic_flight::aircraft_art::AircraftCanvas;

use crate::bands::{Band, NativeRegion, Scan, SIDE};
use crate::medium_aircraft::{GRID_PIXELS as MEDIUM_PIXELS, GRID_SIDE as MEDIUM_SIDE, MediumGrid};
use crate::native_ui::NativeUi;

const TILE: usize = 16;
const TILE_PIXELS: usize = TILE * TILE;
const GRID: usize = SIDE.div_ceil(TILE);
pub const TILE_CAPACITY: usize = 128;
pub const STORAGE_PIXELS: usize = TILE_CAPACITY * TILE_PIXELS;

pub const fn aircraft_side<const LOGICAL: usize>() -> usize {
    if LOGICAL < MEDIUM_SIDE { MEDIUM_SIDE } else { SIDE }
}

pub struct NativeFlight<'a> {
    colors: &'a mut [u16],
    slots: [u8; GRID * GRID],
    used: usize,
    exhausted: bool,
    medium: bool,
    medium_grid: MediumGrid,
    ui: Option<NativeUi<'a>>,
}

impl<'a> NativeFlight<'a> {
    pub fn new(colors: &'a mut [u16]) -> Self {
        assert!(colors.len() <= STORAGE_PIXELS && colors.len() % TILE_PIXELS == 0);
        Self {
            colors,
            slots: [0; GRID * GRID],
            used: 0,
            exhausted: false,
            medium: false,
            medium_grid: MediumGrid::new(),
            ui: None,
        }
    }

    pub fn clear(&mut self) {
        if self.medium {
            self.medium_grid.clear();
            self.medium = false;
        }
        if self.used != 0 {
            self.slots.fill(0);
            self.used = 0;
        }
        self.exhausted = false;
        if let Some(ui) = &mut self.ui {
            ui.clear();
        }
    }

    pub fn with_ui(mut self, storage: &'a mut [u16]) -> Self {
        self.ui = Some(NativeUi::new(storage));
        self
    }

    pub fn ui(&mut self) -> &mut NativeUi<'a> {
        self.ui.as_mut().expect("native interface storage")
    }

    pub fn canvas<'b, const LOGICAL: usize>(
        &'b mut self,
        world: &'b [u16],
    ) -> NativeCanvas<'a, 'b, LOGICAL> {
        self.clear();
        assert_eq!(world.len(), LOGICAL * LOGICAL);
        self.medium = aircraft_side::<LOGICAL>() == MEDIUM_SIDE;
        self.exhausted = self.medium && self.colors.len() < MEDIUM_PIXELS;
        NativeCanvas {
            overlay: self,
            world,
            excess: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        !self.exhausted
    }

    #[cfg(feature = "native-airframe-probe")]
    pub fn tile_count(&self) -> usize {
        self.used
    }

    pub fn pack<const LOGICAL: usize>(&self, scan: &Scan<LOGICAL>, target: &mut [u8], band: Band) {
        if self.medium {
            if self.is_complete() {
                self.medium_grid.pack(self.colors, target, band);
            }
        } else {
            self.pack_aircraft(scan, target, band);
        }
        if let Some(ui) = &self.ui {
            ui.pack(target, band);
        }
    }

    fn pack_aircraft<const LOGICAL: usize>(
        &self,
        scan: &Scan<LOGICAL>,
        target: &mut [u8],
        band: Band,
    ) {
        if self.used == 0 || !self.is_complete() {
            return;
        }
        // Panel columns run down the original viewport in reverse order.
        let x_runs = band.y / TILE..(band.y + band.height).div_ceil(TILE);
        let y_runs = (SIDE - band.x - band.width) / TILE..(SIDE - band.x).div_ceil(TILE);
        for cell_x in x_runs {
            for cell_y in y_runs.clone() {
                let index = self.slots[cell_y * GRID + cell_x];
                if index != 0 {
                    let x = cell_x * TILE;
                    let y = cell_y * TILE;
                    let width = TILE.min(SIDE - x);
                    let height = TILE.min(SIDE - y);
                    let start = (usize::from(index) - 1) * TILE_PIXELS;
                    let pixels = &self.colors[start..start + width * height];
                    let region = NativeRegion {
                        x,
                        y,
                        width,
                        height,
                        pixels,
                    };
                    scan.pack_native_region(target, band, &region);
                }
            }
        }
    }
}

pub struct NativeCanvas<'a, 'b, const LOGICAL: usize> {
    overlay: &'b mut NativeFlight<'a>,
    world: &'b [u16],
    excess: u16,
}

impl<const LOGICAL: usize> NativeCanvas<'_, '_, LOGICAL> {
    fn tile_slot(&mut self, cell_x: usize, cell_y: usize, width: usize) -> Option<usize> {
        let index = cell_y * GRID + cell_x;
        let slot = usize::from(self.overlay.slots[index]);
        if slot != 0 {
            return Some((slot - 1) * TILE_PIXELS);
        }

        let used = self.overlay.used;
        let start = used * TILE_PIXELS;
        if start >= self.overlay.colors.len() {
            self.overlay.exhausted = true;
            return None;
        }
        let location_x = cell_x * TILE;
        let location_y = cell_y * TILE;
        let height = TILE.min(SIDE - location_y);
        let storage = &mut self.overlay.colors[start..start + width * height];
        for (row, line) in storage.chunks_exact_mut(width).enumerate() {
            let y = location_y + row;
            // Match the reversed half-pixel tie rule in Scan::pack.
            let src_y = LOGICAL - 1 - source::<LOGICAL, SIDE>(SIDE - 1 - y);
            let src = &self.world[src_y * LOGICAL..(src_y + 1) * LOGICAL];
            for (offset, color) in line.iter_mut().enumerate() {
                *color = src[source::<LOGICAL, SIDE>(location_x + offset)];
            }
        }
        self.overlay.slots[index] = (used + 1) as u8;
        self.overlay.used = used + 1;
        Some(start)
    }
}

impl<const LOGICAL: usize> AircraftCanvas for NativeCanvas<'_, '_, LOGICAL> {
    fn supports_opaque_spans(&self) -> bool {
        true
    }

    fn pixel(&mut self, x: usize, y: usize) -> &mut u16 {
        if aircraft_side::<LOGICAL>() == MEDIUM_SIDE {
            if self.overlay.exhausted {
                return &mut self.excess;
            }
            let position = x * MEDIUM_SIDE + y;
            if self.overlay.medium_grid.touch(x, y) {
                // Blended effects sample the underlying world at the center
                // of this texel. Untouched texels never cover the world.
                let center_x = source::<SIDE, MEDIUM_SIDE>(x);
                let center_y = source::<SIDE, MEDIUM_SIDE>(y);
                let base_x = source::<LOGICAL, SIDE>(center_x);
                let base_y = LOGICAL - 1 - source::<LOGICAL, SIDE>(SIDE - 1 - center_y);
                self.overlay.colors[position] = self.world[base_y * LOGICAL + base_x];
            }
            return &mut self.overlay.colors[position];
        }
        let cell_x = x / TILE;
        let width = TILE.min(SIDE - cell_x * TILE);
        let Some(start) = self.tile_slot(cell_x, y / TILE, width) else {
            return &mut self.excess;
        };
        &mut self.overlay.colors[start + (y % TILE) * width + (x % TILE)]
    }

    fn opaque_span(&mut self, mut first: usize, end: usize, y: usize, color: u16) {
        if aircraft_side::<LOGICAL>() == MEDIUM_SIDE {
            if !self.overlay.exhausted {
                for x in first..end {
                    self.overlay.medium_grid.touch(x, y);
                    self.overlay.colors[x * MEDIUM_SIDE + y] = color;
                }
            }
            return;
        }
        let cell_y = y / TILE;
        let row = y % TILE;
        while first < end {
            let cell_x = first / TILE;
            let location_x = cell_x * TILE;
            let width = TILE.min(SIDE - location_x);
            let next = end.min(location_x + width);
            if let Some(start) = self.tile_slot(cell_x, cell_y, width) {
                let from = start + row * width + first - location_x;
                let len = next - first;
                self.overlay.colors[from..from + len].fill(color);
            }
            first = next;
        }
    }
}

const fn source<const LOGICAL: usize, const DEST: usize>(pixel: usize) -> usize {
    ((2 * pixel + 1) * LOGICAL) / (2 * DEST)
}
