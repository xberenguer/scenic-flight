//! Rural solar arrays fitted to actual dry land and a continuous road approach.

use crate::ground_details::ParcelSurvey;
use crate::math::{fraction, mix, smoothstep};
use crate::roads::{ROAD_REGION_SIZE, RoadNetwork};
use crate::world::{Sample, Surface, World};

const PANEL_WIDTH: f32 = 16.5;
const PANEL_DEPTH: f32 = 6.1;
const COLUMN_STEP: f32 = 22.0;
const ROW_STEP: f32 = 12.0;
const ARRAY_START: f32 = 13.0;
const SPUR_WIDTH: f32 = 3.0;
const SPUR_SHOULDER: f32 = 4.3;

#[derive(Clone, Copy, Debug)]
pub struct SolarLayout {
    pub name: &'static str,
    rack_rows: [u8; 6],
}

impl SolarLayout {
    pub const ALL: [Self; 25] = [
        Self {
            name: "Full rack",
            rack_rows: [0b11111, 0b11111, 0b11111, 0b11111, 0b11111, 0b11111],
        },
        Self {
            name: "Service cross",
            rack_rows: [0b11011, 0b11011, 0b11100, 0b00000, 0b11011, 0b11011],
        },
        Self {
            name: "Center lane",
            rack_rows: [0b11011, 0b11011, 0b11011, 0b11011, 0b11011, 0b11011],
        },
        Self {
            name: "Cross lane",
            rack_rows: [0b11111, 0b11111, 0b00000, 0b00000, 0b11111, 0b11111],
        },
        Self {
            name: "Gate courtyard",
            rack_rows: [0b11011, 0b11011, 0b11011, 0b11111, 0b11111, 0b11111],
        },
        Self {
            name: "Rear courtyard",
            rack_rows: [0b11111, 0b11111, 0b11111, 0b11011, 0b11011, 0b11011],
        },
        Self {
            name: "Left setback",
            rack_rows: [0b00111, 0b00111, 0b00111, 0b11111, 0b11111, 0b11111],
        },
        Self {
            name: "Right setback",
            rack_rows: [0b11100, 0b11100, 0b11100, 0b11111, 0b11111, 0b11111],
        },
        Self {
            name: "Left taper",
            rack_rows: [0b11111, 0b11111, 0b11111, 0b11111, 0b11110, 0b11100],
        },
        Self {
            name: "Right taper",
            rack_rows: [0b11111, 0b11111, 0b11111, 0b11111, 0b01111, 0b00111],
        },
        Self {
            name: "Fan field",
            rack_rows: [0b00100, 0b01110, 0b01110, 0b11111, 0b11111, 0b11111],
        },
        Self {
            name: "Funnel field",
            rack_rows: [0b11111, 0b11111, 0b11111, 0b01110, 0b01110, 0b00100],
        },
        Self {
            name: "Diamond field",
            rack_rows: [0b00100, 0b01110, 0b11111, 0b11111, 0b01110, 0b00100],
        },
        Self {
            name: "Double pavilion",
            rack_rows: [0b01110, 0b11111, 0b11111, 0b11111, 0b11111, 0b11011],
        },
        Self {
            name: "Diagonal west",
            rack_rows: [0b11110, 0b11110, 0b11100, 0b01110, 0b01111, 0b01111],
        },
        Self {
            name: "Diagonal east",
            rack_rows: [0b01111, 0b01111, 0b00111, 0b01110, 0b11110, 0b11110],
        },
        Self {
            name: "Offset west banks",
            rack_rows: [0b11111, 0b11111, 0b11011, 0b01111, 0b00111, 0b00111],
        },
        Self {
            name: "Offset east banks",
            rack_rows: [0b11111, 0b11111, 0b11011, 0b11110, 0b11100, 0b11100],
        },
        Self {
            name: "Bilateral pockets",
            rack_rows: [0b11111, 0b11111, 0b00100, 0b00100, 0b11111, 0b11111],
        },
        Self {
            name: "Single west block",
            rack_rows: [0b11110, 0b11110, 0b11110, 0b11110, 0b11110, 0b11110],
        },
        Self {
            name: "Single east block",
            rack_rows: [0b01111, 0b01111, 0b01111, 0b01111, 0b01111, 0b01111],
        },
        Self {
            name: "Center block",
            rack_rows: [0b11111, 0b01110, 0b01110, 0b01110, 0b01110, 0b01110],
        },
        Self {
            name: "Three islands",
            rack_rows: [0b11011, 0b11011, 0b11011, 0b00100, 0b01110, 0b01110],
        },
        Self {
            name: "Fork field",
            rack_rows: [0b00100, 0b01110, 0b11111, 0b11011, 0b11011, 0b11011],
        },
        Self {
            name: "Striped field",
            rack_rows: [0b11111, 0b00000, 0b00000, 0b11111, 0b00000, 0b11111],
        },
    ];

    pub fn rows(self, cols: u8, rows: u8) -> [u8; 6] {
        let source_cols: &[u8] = match cols {
            3 => &[0, 2, 4],
            4 => &[0, 1, 2, 4],
            _ => &[0, 1, 2, 3, 4],
        };
        let source_rows: &[usize] = match rows {
            4 => &[0, 2, 3, 5],
            5 => &[0, 1, 2, 4, 5],
            _ => &[0, 1, 2, 3, 4, 5],
        };
        let mut masks = [0; 6];
        for (row, &source) in source_rows.iter().enumerate() {
            for (col, &bit) in source_cols.iter().enumerate() {
                masks[row] |= ((self.rack_rows[source] >> (4 - bit)) & 1) << col;
            }
        }
        masks
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SolarSite {
    trunk: [f32; 3],
    gate: [f32; 3],
    axis: [f32; 2],
    length: f32,
    cols: u8,
    rows: u8,
    layout: u8,
    racks: [u8; 6],
    bounds: [f32; 4],
}

impl SolarSite {
    pub fn generate(
        world: &World,
        seed: u64,
        key: (i32, i32),
        road: &RoadNetwork,
        mut blocked: impl FnMut(f32, f32) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        let region = (u64::from(key.0 as u32) << 32) | u64::from(key.1 as u32);
        let key_bits = mix(seed ^ region ^ 0xf4a0_2219_728b_a3a7);
        if key_bits & 15 > 2 {
            return None;
        }
        let points = road.polyline();
        let available = points.len().checked_sub(8)?;
        if available == 0 {
            return None;
        }
        let mut parcels = ParcelSurvey::new();
        let mut rigorous = 0;
        let start = ((key_bits >> 17) % available as u64) as usize;
        for index in 0..available.min(16) {
            let segment = 4 + (start + index) % available;
            let [a, b] = [points[segment], points[segment + 1]];
            let [dx, dz] = [b.x - a.x, b.z - a.z];
            let length = root(dx * dx + dz * dz);
            if length < 10.0 {
                continue;
            }
            let [tx, tz] = [dx / length, dz / length];
            let [x, z] = [(a.x + b.x) * 0.5, (a.z + b.z) * 0.5];
            let trunk = [x, world.elevation(x, z), z];
            if !(7.0..=49.0).contains(&trunk[1]) {
                continue;
            }
            let layout = mix(key_bits ^ segment as u64);
            let first = if layout & 1 == 0 { 1.0 } else { -1.0 };
            for side in [first, -first] {
                for bias in [0.0, -0.36, 0.36] {
                    pulse();
                    let normal = 1.0 / root(1.0 + bias * bias);
                    let axis = [
                        (-tz * side + tx * bias) * normal,
                        (tx * side + tz * bias) * normal,
                    ];
                    let length = 44.0 + 44.0 * fraction(mix(layout ^ 0x735));
                    let [gx, gz] = point(trunk, axis, length, 0.0);
                    let gate = [gx, world.elevation(gx, gz), gz];
                    if (gate[1] - trunk[1]).abs() > length * 0.095 || gate[1] < 7.0 {
                        continue;
                    }
                    let cols = 3 + (layout >> 38) as u8 % 3;
                    let rows = 4 + (layout >> 41) as u8 % 3;
                    let width = f32::from(cols) * COLUMN_STEP * 0.5 + 3.0;
                    let depth = ARRAY_START + f32::from(rows) * ROW_STEP;
                    let last_height = world.elevation(gx + depth * axis[0], gz + depth * axis[1]);
                    if (last_height - gate[1]).abs() > 8.0 || last_height < 7.0 {
                        continue;
                    }
                    let [mut left, mut near, mut right, mut far] =
                        [x - 5.0, z - 5.0, x + 5.0, z + 5.0];
                    for (u, v) in [
                        (-width, -6.0),
                        (width, -6.0),
                        (-width, depth + 3.0),
                        (width, depth + 3.0),
                    ] {
                        let [x, z] = point(gate, axis, v, u);
                        [left, near, right, far] =
                            [left.min(x), near.min(z), right.max(x), far.max(z)];
                    }
                    let region = [key.0 as f32, key.1 as f32].map(|i| i * ROAD_REGION_SIZE as f32);
                    if left < region[0] + 4.0
                        || near < region[1] + 4.0
                        || right > region[0] + ROAD_REGION_SIZE as f32 - 4.0
                        || far > region[1] + ROAD_REGION_SIZE as f32 - 4.0
                    {
                        continue;
                    }
                    let sample_at = |u, v| {
                        let [x, z] = point(gate, axis, v, u);
                        world.sample(x, z)
                    };
                    if [-24.0, 0.0, 24.0].into_iter().any(|u| {
                        [14.0, depth - 8.0].into_iter().any(|v| {
                            let sample = sample_at(u, v);
                            !matches!(sample.surface, Surface::Grass | Surface::Desert)
                                || (sample.elevation - gate[1]).abs() > 8.0
                        })
                    }) {
                        continue;
                    }
                    if rigorous == 12 {
                        return None;
                    }
                    rigorous += 1;
                    let design =
                        (mix(layout ^ 0x5cd6_f4a5_b7c0_01ee) % SolarLayout::ALL.len() as u64) as u8;
                    let site = Self {
                        trunk,
                        gate,
                        axis,
                        length,
                        cols,
                        rows,
                        layout: design,
                        racks: SolarLayout::ALL[usize::from(design)].rows(cols, rows),
                        bounds: [left, near, right, far],
                    };
                    if site.prove(world, seed, road, &mut blocked, &mut parcels, &mut pulse) {
                        return Some(site);
                    }
                }
            }
        }
        None
    }

    /// An isolated full-size parcel for the artwork gallery.
    pub fn artwork(layout: usize) -> Self {
        Self {
            trunk: [0.0, 10.0, -56.0],
            gate: [0.0, 10.0, 0.0],
            axis: [0.0, 1.0],
            length: 56.0,
            cols: 5,
            rows: 6,
            layout: layout as u8,
            racks: SolarLayout::ALL[layout].rows(5, 6),
            bounds: [-58.0, -57.0, 58.0, 86.0],
        }
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }
    pub fn gate(&self) -> [f32; 3] {
        self.gate
    }
    pub fn trunk(&self) -> [f32; 3] {
        self.trunk
    }
    pub fn rows(&self) -> (u8, u8) {
        (self.cols, self.rows)
    }
    pub fn design(&self) -> usize {
        usize::from(self.layout)
    }
    pub fn direction(&self) -> [f32; 2] {
        self.axis
    }

    fn spans(&self) -> (f32, f32) {
        (
            f32::from(self.cols) * COLUMN_STEP * 0.5,
            ARRAY_START + f32::from(self.rows) * ROW_STEP,
        )
    }

    fn prove(
        &self,
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        parcels: &mut ParcelSurvey,
        pulse: &mut impl FnMut(),
    ) -> bool {
        let count = (self.length * 0.80) as usize + 1;
        let check =
            |s: Sample| matches!(s.surface, Surface::Grass | Surface::Desert) && s.elevation >= 6.0;
        let mut previous = self.trunk[1];
        for i in 0..=count {
            pulse();
            let t = i as f32 / count as f32;
            let v = self.length * t;
            let grade = self.trunk[1] + (self.gate[1] - self.trunk[1]) * t;
            for side in [-SPUR_SHOULDER, 0.0, SPUR_SHOULDER] {
                let [x, z] = point(self.trunk, self.axis, v, side);
                let raw = world.sample(x, z);
                if !check(raw)
                    || (raw.elevation - grade).abs() > if v < 12.0 { 1.0 } else { 3.6 }
                    || (v > 12.0 && road.proximity(x, z).is_some())
                    || blocked(x, z)
                    || (raw.surface == Surface::Grass && parcels.occupies_grass(world, seed, x, z))
                {
                    return false;
                }
                if side == 0.0 {
                    let model = self.cover(x, z, raw, raw).elevation;
                    if i != 0 && (model - previous).abs() > 0.109 * self.length / count as f32 {
                        return false;
                    }
                    previous = model;
                }
            }
        }
        let (width, depth) = self.spans();
        let cols = (2.0 * width / 3.5) as usize + 1;
        let rows = (depth / 3.5) as usize + 1;
        for row in 0..=rows {
            pulse();
            let v = -3.0 + (depth + 3.0) * row as f32 / rows as f32;
            for col in 0..=cols {
                let u = -width + 2.0 * width * col as f32 / cols as f32;
                let [x, z] = point(self.gate, self.axis, v, u);
                let raw = world.sample(x, z);
                if !check(raw)
                    || (raw.elevation - self.gate[1]).abs() > 9.0
                    || blocked(x, z)
                    || road.proximity(x, z).is_some()
                    || (raw.surface == Surface::Grass && parcels.occupies_grass(world, seed, x, z))
                {
                    return false;
                }
                let [x1, z1] = point(self.gate, self.axis, v + 4.0, u);
                let [x2, z2] = point(self.gate, self.axis, v, u + 4.0);
                if (world.elevation(x1, z1) - raw.elevation).abs() > 0.65
                    || (world.elevation(x2, z2) - raw.elevation).abs() > 0.65
                {
                    return false;
                }
            }
        }
        true
    }

    pub fn cover(&self, x: f32, z: f32, raw: Sample, developed: Sample) -> Sample {
        self.sample(x, z, raw)
            .map_or(developed, |piece| cover_sample(piece, developed))
    }

    /// Return the actual planted footprint without querying the world.
    pub fn sample(&self, x: f32, z: f32, raw: Sample) -> Option<Sample> {
        let [left, near, right, far] = self.bounds;
        if !(left..right).contains(&x) || !(near..far).contains(&z) {
            return None;
        }
        let [dx, dz] = [x - self.gate[0], z - self.gate[2]];
        let [ax, az] = self.axis;
        let u = dx * az - dz * ax;
        let v = dx * ax + dz * az;
        if (-self.length..=4.0).contains(&v) && u.abs() <= SPUR_SHOULDER {
            let travel = (v + self.length).min(self.length);
            let t = travel / self.length;
            let grade = self.trunk[1] + (self.gate[1] - self.trunk[1]) * t;
            let weight = smoothstep(travel / 12.0) * smoothstep((SPUR_SHOULDER - u.abs()) / 1.45);
            return Some(Sample {
                elevation: raw.elevation + (grade - raw.elevation) * weight,
                surface: if u.abs() <= SPUR_WIDTH {
                    Surface::Pavement
                } else {
                    Surface::Gravel
                },
            });
        }
        let (width, depth) = self.spans();
        if !(-width..width).contains(&u) || !(0.0..depth).contains(&v) {
            return None;
        }
        if v < ARRAY_START {
            if (-width + 6.0..width - 6.0).contains(&u) {
                if (width - 16.0..width - 6.0).contains(&u) && (4.0..10.5).contains(&v) {
                    return Some(Sample {
                        elevation: raw.elevation + 3.1,
                        surface: Surface::FlatRoof,
                    });
                }
                return Some(Sample {
                    surface: Surface::Gravel,
                    ..raw
                });
            }
            return None;
        }
        let array_x = (u + width) / COLUMN_STEP;
        let array_z = (v - ARRAY_START) / ROW_STEP;
        let col = array_x as u8;
        let row = array_z as u8;
        let dx = ((array_x - f32::from(col)) - 0.5) * COLUMN_STEP;
        let dz = ((array_z - f32::from(row)) - 0.5) * ROW_STEP;
        let row_mask = *self.racks.get(usize::from(row))?;
        let occupied = row_mask & (1 << col) != 0;
        if occupied && dx.abs() <= PANEL_WIDTH * 0.5 && dz.abs() <= PANEL_DEPTH * 0.5 {
            let edge = dx.abs() > PANEL_WIDTH * 0.5 - 0.65 || dz.abs() > PANEL_DEPTH * 0.5 - 0.65;
            return Some(Sample {
                elevation: raw.elevation + 1.45 + 0.24 * dz,
                surface: if edge {
                    Surface::TowerRoof
                } else {
                    Surface::SolarGlass
                },
            });
        }
        let cross_lane = row_mask == 0 && dz.abs() <= 2.5;
        let center_lane = row_mask & (1 << (self.cols / 2)) == 0 && u.abs() <= 2.5;
        if cross_lane || center_lane || (COLUMN_STEP * 0.5 - dx.abs() < 1.4 && col > 0) {
            return Some(Sample {
                surface: Surface::Gravel,
                ..raw
            });
        }
        None
    }
}

fn point(origin: [f32; 3], axis: [f32; 2], v: f32, u: f32) -> [f32; 2] {
    [
        origin[0] + axis[0] * v + axis[1] * u,
        origin[2] + axis[1] * v - axis[0] * u,
    ]
}

fn root(value: f32) -> f32 {
    if value <= 0.0 {
        return 0.0;
    }
    let mut estimate = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    for _ in 0..3 {
        estimate = 0.5 * (estimate + value / estimate);
    }
    estimate
}

pub(crate) fn cover_sample(piece: Sample, developed: Sample) -> Sample {
    if piece.surface == Surface::Gravel && developed.surface == Surface::Pavement {
        Sample {
            surface: Surface::Pavement,
            ..piece
        }
    } else {
        piece
    }
}

#[cfg(test)]
mod tests {
    use super::SolarLayout;

    #[test]
    fn every_layout_stays_distinct_and_populated_on_every_parcel_size() {
        for cols in 3..=5 {
            for rows in 4..=6 {
                let masks = SolarLayout::ALL.map(|layout| layout.rows(cols, rows));
                for (index, mask) in masks.iter().enumerate() {
                    let panels: u32 = mask.iter().map(|row| row.count_ones()).sum();
                    assert!(
                        panels >= 6,
                        "{} leaves too few panels",
                        SolarLayout::ALL[index].name
                    );
                    assert!(
                        !masks[..index].contains(mask),
                        "{} loses its shape on {cols}×{rows}",
                        SolarLayout::ALL[index].name
                    );
                }
            }
        }
    }
}
