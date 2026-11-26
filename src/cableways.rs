//! Sparse gondolas between road terraces and high overlooks.

use core::f32::consts::TAU;

use crate::flight::TICKS_PER_SECOND;
use crate::math::{fraction, mix, sin_cos};
use crate::roads::{ROAD_HALF_WIDTH, RoadNetwork};
use crate::watershed::Watershed;
use crate::world::{SEA_LEVEL, World};

mod designs;
pub use designs::CablewayDesign;
use designs::{Car, Paint, Terminal};

const LANE: f32 = 2.15;
const STATION_WIDTH: f32 = 4.25;
const STATION_LENGTH: f32 = 4.5;
const CABIN_DROP: f32 = 3.9;
const RAW_CLEARANCE: f32 = 3.65;
const CANDIDATES: usize = 8;

type Point = [f32; 3];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Station {
    /// World position of the grounded platform, ordered x, height, z.
    pub platform: Point,
    pub anchor: Point,
    base: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pylon {
    pub base: Point,
    pub anchor: Point,
}

impl Pylon {
    const EMPTY: Self = Self {
        base: [0.0; 3],
        anchor: [0.0; 3],
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SagSpan {
    pub from: Point,
    pub to: Point,
    pub sag: f32,
    pub length: f32,
}

impl SagSpan {
    const EMPTY: Self = Self {
        from: [0.0; 3],
        to: [0.0; 3],
        sag: 0.0,
        length: 0.0,
    };

    fn between(from: Point, to: Point) -> Self {
        let length = horizontal(from, to);
        Self {
            from,
            to,
            sag: (length * 0.025).clamp(0.8, 3.7),
            length,
        }
    }

    /// Sample the same sag curve followed by the travelling cabins.
    pub fn point(self, t: f32) -> Point {
        let t = t.clamp(0.0, 1.0);
        let mut point = lerp(self.from, self.to, t);
        point[1] -= 4.0 * self.sag * t * (1.0 - t);
        point
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cabin {
    pub suspension: Point,
    pub floor: Point,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CablePart {
    AccessPath,
    Station,
    StationRoof,
    Overlook,
    Foundation,
    Pylon,
    Cabin,
    Window,
    Hanger,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CableTriangle {
    pub corners: [Point; 3],
    pub part: CablePart,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CableSegment {
    pub from: Point,
    pub to: Point,
    pub diameter: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cableway {
    stations: [Station; 2],
    pylons: [Pylon; 2],
    spans: [SagSpan; 3],
    pylon_count: usize,
    path: [Point; 3],
    axis: [f32; 2],
    length: f32,
    phase: f32,
    design: CablewayDesign,
}

impl Cableway {
    pub fn generate(
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        watershed: Option<&Watershed>,
    ) -> Option<Self> {
        Self::generate_cooperatively(world, seed, road, watershed, &mut || {})
    }

    /// Survey the same seeded cableway while periodically servicing other work.
    pub fn generate_cooperatively(
        world: &World,
        seed: u64,
        road: &RoadNetwork,
        watershed: Option<&Watershed>,
        pulse: &mut impl FnMut(),
    ) -> Option<Self> {
        let points = road.polyline();
        let first = points.first()?;
        let cell = |n: f32| {
            let n = n / 1024.0;
            let i = n as i32;
            i - i32::from(n < i as f32)
        };
        let region = (u64::from(cell(first.x) as u32) << 32) | u64::from(cell(first.z) as u32);
        let key = mix(seed ^ region ^ 0x4383_af18_6e8d_5b27);
        if key & 7 >= 3 {
            return None;
        }
        let fit = Survey { world, watershed };
        let mut proposals = [None; CANDIDATES];
        let mut scores = [f32::NEG_INFINITY; CANDIDATES];
        let offset = fraction(mix(key ^ 9)) * TAU;
        for index in [
            0,
            points.len() - 1,
            points.len() / 4,
            points.len() / 2,
            points.len() * 3 / 4,
        ] {
            let road_point = points[index];
            for i in 0..16 {
                pulse();
                let (sx, sz) = sin_cos(offset + i as f32 * (TAU / 16.0));
                let axis = [sx, sz];
                let Some((lower, access)) =
                    fit.lower(road, [road_point.x, road_point.z], axis, pulse)
                else {
                    continue;
                };
                for length in [104.0, 132.0, 172.0, 217.0, 260.0] {
                    pulse();
                    let center = lower.platform;
                    let candidate = [center[0] + sx * length, center[2] + sz * length];
                    let y = world.elevation(candidate[0], candidate[1]);
                    let rise = y - center[1];
                    if !(20.0..=78.0).contains(&rise)
                        || rise < length * 0.115
                        || rise > length * 0.51
                    {
                        continue;
                    }
                    let Some((upper, vista)) = fit.upper(candidate, axis) else {
                        continue;
                    };
                    let score = rise + vista * 0.55 - length * 0.085;
                    if let Some(position) = scores.iter().position(|&other| score > other) {
                        for slot in (position + 1..CANDIDATES).rev() {
                            proposals[slot] = proposals[slot - 1];
                            scores[slot] = scores[slot - 1];
                        }
                        proposals[position] = Some((lower, upper, access, axis, length));
                        scores[position] = score;
                    }
                }
            }
        }
        for (lower, upper, path, axis, length) in proposals.into_iter().flatten() {
            let phase = 0.19 + fraction(mix(key ^ 72)) * 0.49;
            let mut line = Self {
                stations: [lower, upper],
                pylons: [Pylon::EMPTY; 2],
                spans: [SagSpan::EMPTY; 3],
                pylon_count: 0,
                path,
                axis,
                length,
                phase,
                design: CablewayDesign::select(mix(key ^ 0xd997)),
            };
            for lift in [0.0, 0.85, 1.50] {
                line.stations = [lower, upper];
                for station in &mut line.stations {
                    station.anchor[1] += lift;
                }
                for [a, b] in [
                    [0.48, 0.0],
                    [0.57, 0.0],
                    [0.38, 0.0],
                    [0.65, 0.0],
                    [0.30, 0.67],
                    [0.36, 0.73],
                    [0.25, 0.60],
                ] {
                    pulse();
                    let Some(first) = fit.tower(line.stations, axis, a, lift) else {
                        continue;
                    };
                    if b == 0.0 {
                        line.set_pylons(&[first]);
                    } else {
                        let Some(second) = fit.tower(line.stations, axis, b, lift) else {
                            continue;
                        };
                        line.set_pylons(&[first, second]);
                    }
                    if line.clears_cooperatively(|x, z| world.elevation(x, z), pulse) {
                        return Some(line);
                    }
                }
            }
        }
        None
    }

    pub fn design(&self) -> CablewayDesign {
        self.design
    }

    pub fn stations(&self) -> &[Station; 2] {
        &self.stations
    }

    pub fn pylons(&self) -> &[Pylon] {
        &self.pylons[..self.pylon_count]
    }

    pub fn spans(&self) -> &[SagSpan] {
        &self.spans[..=self.pylon_count]
    }

    pub fn access_path(&self) -> &[Point; 3] {
        &self.path
    }

    /// Minimum x, minimum z, maximum x, maximum z, including the terminals.
    pub fn bounds(&self) -> [f32; 4] {
        let a = self.path[0];
        let b = self.stations[1].platform;
        [
            a[0].min(b[0]) - 8.0,
            a[2].min(b[2]) - 8.0,
            a[0].max(b[0]) + 8.0,
            a[2].max(b[2]) + 8.0,
        ]
    }

    /// Verify the entire swept cable corridor against terrain or prepared scenery heights.
    pub fn clears(&self, height: impl FnMut(f32, f32) -> f32) -> bool {
        self.clears_cooperatively(height, &mut || {})
    }

    pub fn clears_cooperatively(
        &self,
        mut height: impl FnMut(f32, f32) -> f32,
        pulse: &mut impl FnMut(),
    ) -> bool {
        for span in self.spans() {
            let steps = (span.length / 1.1) as usize + 1;
            let cabin_length = 1.66 / span.length;
            let sag_minimum = 0.5 - (span.to[1] - span.from[1]) / (8.0 * span.sag);
            for step in 0..=steps {
                if step & 7 == 0 {
                    pulse();
                }
                let t = step as f32 / steps as f32;
                let mut rope = span.point(t);
                let lowest_t = sag_minimum.clamp(t - cabin_length, t + cabin_length);
                rope[1] = span.point(lowest_t)[1];
                for lane in [-4.05, -LANE, 0.0, LANE, 4.05] {
                    let [x, y, z] = self.shift(rope, 0.0, lane, -CABIN_DROP);
                    if !(y - height(x, z) >= RAW_CLEARANCE) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// A slow-running car on each of two opposite lanes. Cars pass inside the terminal bays.
    pub fn cabin(&self, ticks: u64, lane: usize) -> Option<Cabin> {
        if lane > 1 {
            return None;
        }
        let speed = 2.7 / TICKS_PER_SECOND as f32;
        let period = (self.length / speed) as u64 + 1;
        let cycle = (ticks % period) as f32 / period as f32 + self.phase;
        let mut along = (cycle - cycle as u32 as f32) * self.length;
        if lane != 0 {
            along = self.length - ((along + self.length * 0.32) % self.length);
        }
        if along < 6.0 || along > self.length - 6.0 {
            return None;
        }
        let side = if lane == 0 { -LANE } else { LANE };
        for span in self.spans() {
            if along <= span.length {
                let suspension = self.shift(span.point(along / span.length), 0.0, side, 0.0);
                return Some(Cabin {
                    suspension,
                    floor: [suspension[0], suspension[1] - CABIN_DROP, suspension[2]],
                });
            }
            along -= span.length;
        }
        None
    }

    /// Both track cables, split into short line segments for the renderer.
    pub fn cable_segments(&self, mut emit: impl FnMut(CableSegment)) {
        for span in self.spans() {
            let steps = (span.length / 8.0) as usize + 1;
            for side in [-LANE, LANE] {
                let mut from = self.shift(span.from, 0.0, side, 0.0);
                for step in 1..=steps {
                    let to = self.shift(span.point(step as f32 / steps as f32), 0.0, side, 0.0);
                    emit(CableSegment {
                        from,
                        to,
                        diameter: 0.11,
                    });
                    from = to;
                }
            }
        }
    }

    pub fn draw(&self, mesh: &mut crate::world_mesh::WorldMesh<'_>, ticks: u64) {
        let paint = self.design.spec().paint;
        self.triangles(ticks, |triangle| draw_part(mesh, triangle, paint));
        self.cable_segments(|wire| mesh.wire(wire.from, wire.to, wire.diameter, [86, 92, 93]));
    }

    pub fn triangles(&self, ticks: u64, mut emit: impl FnMut(CableTriangle)) {
        for edge in self.path.windows(2) {
            quad(
                &mut emit,
                CablePart::AccessPath,
                [
                    self.shift(edge[0], 0.0, 0.82, 0.02),
                    self.shift(edge[1], 0.0, 0.82, 0.02),
                    self.shift(edge[1], 0.0, -0.82, 0.02),
                    self.shift(edge[0], 0.0, -0.82, 0.02),
                ],
            );
        }
        let design = self.design.spec();
        for (i, station) in self.stations.iter().enumerate() {
            self.terminal_geometry(&mut emit, *station, design.terminal, i == 1);
            self.tower_geometry(
                &mut emit,
                station.platform,
                station.anchor,
                design.terminal.upright,
            );
            if i == 1 {
                let terrace = self.shift(station.platform, 3.95, 0.0, 0.31);
                self.block(
                    &mut emit,
                    terrace,
                    [0.49, 3.65],
                    [-0.25, 0.12],
                    CablePart::Overlook,
                );
            }
        }
        for tower in self.pylons() {
            self.block(
                &mut emit,
                tower.base,
                [1.20, 1.33],
                [-0.28, 0.43],
                CablePart::Foundation,
            );
            self.tower_geometry(&mut emit, tower.base, tower.anchor, 0.43);
        }
        for lane in 0..2 {
            if let Some(cabin) = self.cabin(ticks, lane) {
                self.cabin_geometry(&mut emit, cabin.floor, design.car);
            }
        }
    }

    fn terminal_geometry(
        &self,
        emit: &mut impl FnMut(CableTriangle),
        station: Station,
        terminal: Terminal,
        upper: bool,
    ) {
        let origin = station.platform;
        self.block(
            emit,
            origin,
            [4.42, 4.16],
            [station.base - origin[1], 0.35],
            CablePart::Foundation,
        );
        let width = terminal.half_width - 0.28;
        let levels = if upper {
            core::array::from_fn(|i| terminal.roof[3 - i])
        } else {
            terminal.roof
        };
        self.block(
            emit,
            origin,
            [3.98, width],
            [0.35, terminal.upright],
            CablePart::Station,
        );

        let roof_point = |index: usize, lateral: f32| {
            self.shift(
                origin,
                [-4.38, -1.35, 1.35, 4.38][index],
                lateral,
                levels[index],
            )
        };
        let roof = [-terminal.half_width, terminal.half_width];
        for i in 0..levels.len() - 1 {
            let a = roof_point(i, roof[0]);
            let b = roof_point(i + 1, roof[0]);
            let c = roof_point(i + 1, roof[1]);
            let d = roof_point(i, roof[1]);
            quad(emit, CablePart::StationRoof, [a, b, c, d]);
            for sign in [-1.0, 1.0] {
                let edge = |idx: usize| {
                    self.shift(
                        origin,
                        [-3.98, -1.35, 1.35, 3.98][idx],
                        width * sign,
                        levels[idx],
                    )
                };
                let left = edge(i);
                let right = edge(i + 1);
                let floor = origin[1] + terminal.upright;
                quad(
                    emit,
                    CablePart::Station,
                    [
                        left,
                        right,
                        [right[0], floor, right[2]],
                        [left[0], floor, left[2]],
                    ],
                );
            }
        }
        for (edge, end) in [(0, -3.98), (3, 3.98)] {
            let top = [-width, width].map(|side| self.shift(origin, end, side, levels[edge]));
            let base = [-width, width].map(|side| self.shift(origin, end, side, terminal.upright));
            quad(emit, CablePart::Station, [base[0], base[1], top[1], top[0]]);
            let windows = [-width * 0.70, width * 0.70];
            let front = end + end.signum() * 0.03;
            let [a, b] = windows.map(|side| self.shift(origin, front, side, 1.38));
            let [c, d] =
                windows.map(|side| self.shift(origin, front, side, terminal.upright - 0.48));
            quad(emit, CablePart::Window, [a, b, d, c]);
        }
    }

    fn cabin_geometry(&self, emit: &mut impl FnMut(CableTriangle), center: Point, car: Car) {
        let [length, width, bevel] = [car.length, car.width, car.chisel];
        let shape = [
            [-length, -width + bevel],
            [-length, width - bevel],
            [-length + bevel, width],
            [length - bevel, width],
            [length, width - bevel],
            [length, -width + bevel],
            [length - bevel, -width],
            [-length + bevel, -width],
        ];
        let ring =
            |height, scale| shape.map(|[x, z]| self.shift(center, x * scale, z * scale, height));
        let lower = ring(0.0, 0.85);
        let belt = ring(car.lower, 1.0);
        let lintel = ring(car.cap - 0.22, 0.92);
        let top = ring(car.cap, 0.77);
        for i in 0..8 {
            let next = (i + 1) % 8;
            quad(
                emit,
                CablePart::Cabin,
                [lower[i], lower[next], belt[next], belt[i]],
            );
            quad(
                emit,
                CablePart::Window,
                [belt[i], belt[next], lintel[next], lintel[i]],
            );
            quad(
                emit,
                CablePart::Cabin,
                [lintel[i], lintel[next], top[next], top[i]],
            );
        }
        for i in 1..7 {
            emit(CableTriangle {
                corners: [top[0], top[i], top[i + 1]],
                part: CablePart::Cabin,
            });
        }
        self.block(
            emit,
            center,
            [0.15, 0.15],
            [car.cap, CABIN_DROP],
            CablePart::Hanger,
        );
    }

    /// Draw a terminal and its cabin together for the game's artwork catalog.
    pub fn draw_artwork(design: CablewayDesign, mesh: &mut crate::world_mesh::WorldMesh<'_>) {
        let station = Station {
            platform: [-3.3, 0.0, 0.0],
            anchor: [-3.3, 8.65, 0.0],
            base: -0.30,
        };
        let line = Self {
            stations: [station; 2],
            pylons: [Pylon::EMPTY; 2],
            spans: [SagSpan::EMPTY; 3],
            pylon_count: 0,
            path: [[0.0; 3]; 3],
            axis: [0.0, 1.0],
            length: 0.0,
            phase: 0.0,
            design,
        };
        let spec = design.spec();
        let mut emit = |triangle| draw_part(mesh, triangle, spec.paint);
        line.terminal_geometry(&mut emit, station, spec.terminal, false);
        line.tower_geometry(
            &mut emit,
            station.platform,
            station.anchor,
            spec.terminal.upright,
        );
        line.cabin_geometry(&mut emit, [4.4, 0.25, 0.0], spec.car);
    }

    fn set_pylons(&mut self, pylons: &[Pylon]) {
        self.pylon_count = pylons.len();
        self.pylons[..pylons.len()].copy_from_slice(pylons);
        let mut from = self.stations[0].anchor;
        for (i, pylon) in pylons.iter().enumerate() {
            self.spans[i] = SagSpan::between(from, pylon.anchor);
            from = pylon.anchor;
        }
        self.spans[pylons.len()] = SagSpan::between(from, self.stations[1].anchor);
        self.length = self.spans().iter().map(|span| span.length).sum();
    }

    fn shift(&self, p: Point, along: f32, cross: f32, height: f32) -> Point {
        shift(p, self.axis, along, cross, height)
    }

    fn block(
        &self,
        emit: &mut impl FnMut(CableTriangle),
        center: Point,
        [l, w]: [f32; 2],
        [low, high]: [f32; 2],
        part: CablePart,
    ) {
        let corners = [[-l, -w], [-l, w], [l, w], [l, -w]];
        let upper = corners.map(|[x, z]| self.shift(center, x, z, high));
        quad(emit, part, upper);
        for i in 0..4 {
            let j = (i + 1) % 4;
            let [a, b] = corners[i];
            let [c, d] = corners[j];
            quad(
                emit,
                part,
                [
                    upper[j],
                    upper[i],
                    self.shift(center, a, b, low),
                    self.shift(center, c, d, low),
                ],
            );
        }
    }

    fn tower_geometry(
        &self,
        emit: &mut impl FnMut(CableTriangle),
        base: Point,
        anchor: Point,
        bottom: f32,
    ) {
        let height = anchor[1] - base[1];
        let waist = self.design.spec().mast_waist;
        self.block(
            emit,
            base,
            [0.46, 0.59],
            [bottom, bottom + (height - bottom) * 0.42],
            CablePart::Pylon,
        );
        self.block(
            emit,
            base,
            [waist, waist + 0.09],
            [bottom + (height - bottom) * 0.42, height],
            CablePart::Pylon,
        );
        self.block(emit, anchor, [0.35, 2.78], [-0.38, 0.02], CablePart::Pylon);
    }
}

struct Survey<'a> {
    world: &'a World,
    watershed: Option<&'a Watershed>,
}

impl Survey<'_> {
    fn is_dry(&self, x: f32, z: f32, y: f32) -> bool {
        y > SEA_LEVEL + 1.5
            && self.watershed.is_none_or(|watershed| {
                watershed
                    .sample(x, z)
                    .is_none_or(|water| !water.is_channel())
            })
    }

    fn pad(
        &self,
        at: [f32; 2],
        axis: [f32; 2],
        along: f32,
        across: f32,
        max_grade: f32,
    ) -> Option<[f32; 2]> {
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        for dx in [-along, 0.0, along] {
            for dz in [-across, 0.0, across] {
                let [x, _, z] = shift([at[0], 0.0, at[1]], axis, dx, dz, 0.0);
                let y = self.world.elevation(x, z);
                if !self.is_dry(x, z, y) {
                    return None;
                }
                low = low.min(y);
                high = high.max(y);
            }
        }
        (high - low <= max_grade).then_some([low, high])
    }

    fn lower(
        &self,
        road: &RoadNetwork,
        point: [f32; 2],
        axis: [f32; 2],
        pulse: &mut impl FnMut(),
    ) -> Option<(Station, [Point; 3])> {
        let at = [point[0] + axis[0] * 10.5, point[1] + axis[1] * 10.5];
        for along in [-STATION_LENGTH, 0.0, STATION_LENGTH] {
            for cross in [-STATION_WIDTH, 0.0, STATION_WIDTH] {
                let [x, _, z] = shift([at[0], 0.0, at[1]], axis, along, cross, 0.0);
                if road
                    .proximity(x, z)
                    .is_some_and(|near| near.distance_squared < ROAD_HALF_WIDTH * ROAD_HALF_WIDTH)
                {
                    return None;
                }
            }
        }
        let [base, ground] = self.pad(at, axis, STATION_LENGTH, STATION_WIDTH, 1.35)?;
        let near = [point[0], self.world.elevation(point[0], point[1]), point[1]];
        let mid = shift(near, axis, 3.0, 0.0, 0.0);
        let mut approach = [near, mid, shift(near, axis, 6.1, 0.0, 0.0)];
        approach[1][1] = self.world.elevation(mid[0], mid[2]);
        approach[2][1] = ground + 0.02;
        for segment in approach.windows(2) {
            let [a, b] = [segment[0], segment[1]];
            if (b[1] - a[1]).abs() > horizontal(a, b) * 0.25 {
                return None;
            }
            for step in 0..=6 {
                if step & 3 == 0 {
                    pulse();
                }
                let at = lerp(a, b, step as f32 / 6.0);
                for side in [-0.85, 0.0, 0.85] {
                    let [x, y, z] = shift(at, axis, 0.0, side, 0.0);
                    let natural = self.world.elevation(x, z);
                    if !self.is_dry(x, z, natural) || !(-0.19..=0.92).contains(&(y - natural)) {
                        return None;
                    }
                }
            }
        }
        let station = Station {
            platform: [at[0], ground, at[1]],
            anchor: [at[0], ground + 8.85, at[1]],
            base,
        };
        Some((station, approach))
    }

    fn upper(&self, at: [f32; 2], axis: [f32; 2]) -> Option<(Station, f32)> {
        let [base, ground] = self.pad(at, axis, STATION_LENGTH, STATION_WIDTH, 2.15)?;
        let mut vistas = 0;
        let mut drop = 0.0_f32;
        for (along, across) in [(-35.0, 0.0), (32.0, 0.0), (0.0, -32.0), (0.0, 32.0)] {
            let [x, _, z] = shift([at[0], 0.0, at[1]], axis, along, across, 0.0);
            let floor = self.world.elevation(x, z);
            if ground - floor > 6.5 {
                vistas += 1;
                drop = drop.max(ground - floor);
            }
        }
        if vistas < 2 && drop < 15.0 {
            return None;
        }
        Some((
            Station {
                platform: [at[0], ground, at[1]],
                anchor: [at[0], ground + 8.65, at[1]],
                base,
            },
            drop.min(23.0),
        ))
    }

    fn tower(
        &self,
        [lower, upper]: [Station; 2],
        axis: [f32; 2],
        t: f32,
        lift: f32,
    ) -> Option<Pylon> {
        let at = lerp(lower.anchor, upper.anchor, t);
        let [base, ground] = self.pad([at[0], at[2]], axis, 1.40, 1.55, 1.55)?;
        let support = (at[1] + 1.0).max(ground + 9.15 + lift);
        if support - base > 25.0 {
            return None;
        }
        Some(Pylon {
            base: [at[0], base, at[2]],
            anchor: [at[0], support, at[2]],
        })
    }
}

fn draw_part(mesh: &mut crate::world_mesh::WorldMesh<'_>, triangle: CableTriangle, paint: Paint) {
    let color = match triangle.part {
        CablePart::AccessPath => [138, 122, 88],
        CablePart::Station => paint.walls,
        CablePart::StationRoof => paint.roof,
        CablePart::Overlook => [156, 130, 92],
        CablePart::Foundation => [127, 125, 115],
        CablePart::Pylon => paint.steel,
        CablePart::Cabin => paint.car,
        CablePart::Window => [97, 142, 159],
        CablePart::Hanger => [60, 66, 68],
    };
    mesh.lit_triangle(triangle.corners, color);
}

fn horizontal(a: Point, b: Point) -> f32 {
    let x = a[0] - b[0];
    let z = a[2] - b[2];
    let sum = x * x + z * z;
    let mut guess = x.abs().max(z.abs());
    if guess == 0.0 {
        return 0.0;
    }
    for _ in 0..4 {
        guess = (guess + sum / guess) * 0.5;
    }
    guess
}

fn shift(p: Point, axis: [f32; 2], along: f32, cross: f32, height: f32) -> Point {
    [
        p[0] + axis[0] * along - axis[1] * cross,
        p[1] + height,
        p[2] + axis[1] * along + axis[0] * cross,
    ]
}

fn lerp(a: Point, b: Point, t: f32) -> Point {
    core::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn quad(emit: &mut impl FnMut(CableTriangle), part: CablePart, [a, b, c, d]: [Point; 4]) {
    emit(CableTriangle {
        corners: [a, b, c],
        part,
    });
    emit(CableTriangle {
        corners: [a, c, d],
        part,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watershed::WatershedWork;

    #[test]
    fn prepared_gondolas_connect_real_dry_terraces_and_clear_the_cabin_envelope() {
        let mut work = WatershedWork::new();
        let mut watershed = Watershed::new();
        for (seed, x, z) in [(1, 1, -2), (31, 5, 6)] {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, x, z).unwrap();
            watershed.rebuild(&world, seed, x, z, &mut work);
            let cable = Cableway::generate(&world, seed, &road, Some(&watershed)).unwrap();
            assert_eq!(
                Cableway::generate(&world, seed, &road, Some(&watershed)),
                Some(cable)
            );
            let [a, _, _] = *cable.access_path();
            assert!(road.proximity(a[0], a[2]).unwrap().distance_squared < 0.001);
            let [lower, upper] = *cable.stations();
            assert!(upper.platform[1] - lower.platform[1] > 20.0);
            assert!(!cable.pylons().is_empty());
            assert_eq!(cable.spans().len(), cable.pylons().len() + 1);
            let survey = Survey {
                world: &world,
                watershed: Some(&watershed),
            };
            for station in cable.stations() {
                let [x, _, z] = station.platform;
                assert!(
                    survey
                        .pad([x, z], cable.axis, STATION_LENGTH, STATION_WIDTH, 2.15)
                        .is_some()
                );
            }
            for tower in cable.pylons() {
                let [x, _, z] = tower.base;
                assert!(survey.pad([x, z], cable.axis, 1.4, 1.55, 1.55).is_some());
                assert!((8.0..=25.0).contains(&(tower.anchor[1] - tower.base[1])));
            }

            for span in cable.spans() {
                assert!(span.point(0.5)[1] < (span.from[1] + span.to[1]) * 0.5);
                let steps = (span.length * 4.0) as usize;
                for step in 0..=steps {
                    let mount = span.point(step as f32 / steps as f32);
                    for along in [-1.66, 0.0, 1.66] {
                        for side in [-3.51, -LANE, -0.79, 0.79, LANE, 3.51] {
                            let [x, floor, z] = cable.shift(mount, along, side, -CABIN_DROP);
                            assert!(floor - world.elevation(x, z) >= 3.55, "{seed} at ({x},{z})");
                        }
                    }
                }
            }
            assert!(!cable.clears(|x, z| world.elevation(x, z) + 40.0));
        }
    }

    #[test]
    fn cabins_progress_slowly_on_both_lanes_and_mesh_is_finite() {
        let seed = 1;
        let world = World::new(seed);
        let road = RoadNetwork::generate(&world, 1, -2).unwrap();
        let cable = Cableway::generate(&world, seed, &road, None).unwrap();
        for lane in 0..2 {
            let mut visible = 0;
            for tick in 0..2_500 {
                if let (Some(car), Some(next)) =
                    (cable.cabin(tick, lane), cable.cabin(tick + 1, lane))
                {
                    visible += 1;
                    assert!((0.08..0.10).contains(&horizontal(car.floor, next.floor)));
                    assert!((car.suspension[1] - car.floor[1] - CABIN_DROP).abs() < 0.001);
                }
            }
            assert!(visible > 1_000);
        }
        let mut cable_segments = 0;
        cable.cable_segments(|segment| {
            cable_segments += 1;
            assert!(horizontal(segment.from, segment.to) <= 8.01);
        });
        assert!(cable_segments >= 30);
        let mut cabins = 0;
        cable.triangles(0, |triangle| {
            assert!(triangle.corners.into_iter().flatten().all(f32::is_finite));
            cabins += usize::from(triangle.part == CablePart::Cabin);
        });
        assert!(cabins >= 20);
    }
}
