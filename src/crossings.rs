//! Raised bridge plans fitted to saved road alignments and water geometry.

use crate::roads::{ROAD_HALF_WIDTH, RoadNetwork, RoadPoint};
use crate::watershed::Watershed;
use crate::world::{SEA_LEVEL, World};

mod designs;
pub use designs::BridgeDesign;
use designs::Web;

const BRIDGE_LIMIT: usize = 3;
const HALF_WIDTH: f32 = ROAD_HALF_WIDTH + 0.18;
const DECK_THICKNESS: f32 = 0.34;
const FREEBOARD: f32 = 0.56;
const APPROACH_GRADE: f32 = 0.12;
const COASTAL_FOOTING: f32 = 0.14;
const COASTAL_MAX_WET: f32 = 35.0;
const COASTAL_MAX_SPAN: f32 = 56.0;
const COASTAL_MAX_CHORD: f32 = 128.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeWater {
    River,
    SeaNarrows,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeStyle {
    LowGirder,
    SmallTruss,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgePart {
    Approach,
    Deck,
    Girder,
    Abutment,
    Guardrail,
    Truss,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BridgeTriangle {
    /// World positions ordered as x, height, z.
    pub corners: [[f32; 3]; 3],
    pub part: BridgePart,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bridge {
    pub water: BridgeWater,
    pub style: BridgeStyle,
    /// Centerline stations from the first dry road join to the second.
    /// The middle pair bounds the main deck.
    sections: [[f32; 3]; 4],
    along: [f32; 4],
    axis: [f32; 2],
    bank_floor: [f32; 2],
    high_water: f32,
    design: BridgeDesign,
}

impl Bridge {
    const EMPTY: Self = Self {
        water: BridgeWater::River,
        style: BridgeStyle::LowGirder,
        sections: [[0.0; 3]; 4],
        along: [0.0; 4],
        axis: [0.0; 2],
        bank_floor: [0.0; 2],
        high_water: SEA_LEVEL,
        design: BridgeDesign::DEFAULT,
    };

    /// Fit one short coastal crossing to a straight chord between dry approaches.
    /// The caller owns the outer routes. The exclusion query protects other
    /// infrastructure throughout the deck, banks, and ramp footprints.
    pub fn fit_sea_narrows(
        world: &World,
        start: [f32; 2],
        end: [f32; 2],
        mut blocked: impl FnMut(f32, f32) -> bool,
        mut pulse: impl FnMut(),
    ) -> Option<Self> {
        if !start.into_iter().chain(end).all(f32::is_finite) {
            return None;
        }
        let line = Line::between(
            RoadPoint {
                x: start[0],
                z: start[1],
                elevation: SEA_LEVEL,
            },
            RoadPoint {
                x: end[0],
                z: end[1],
                elevation: SEA_LEVEL,
            },
        )?;
        if line.length > COASTAL_MAX_CHORD {
            return None;
        }
        let fit = Fit {
            world,
            watershed: None,
            allow_sea_narrows: true,
            coastal: true,
        };
        let [a, b] = fit.single_coastal_run(line, &mut pulse)?;
        let bridge = fit.bridge(line, a, b, BridgeWater::SeaNarrows, &mut pulse)?;
        if !bridge.prove_coastal_footprint(world, &mut blocked, &mut pulse)
            || !reaches_marine_water(world, line, (a + b) * 0.5, &mut pulse)
        {
            return None;
        }
        Some(bridge)
    }

    pub fn design(&self) -> BridgeDesign {
        self.design
    }

    /// The rendered sidework also supplies the horizon outline and clearance.
    pub(crate) fn sidework_height(&self) -> f32 {
        let spec = self.design.spec();
        match self.style {
            BridgeStyle::LowGirder => spec.height,
            BridgeStyle::SmallTruss => (self.span() * spec.height).clamp(1.10, 4.55),
        }
    }

    pub(crate) fn sidework_outline(&self, fraction: f32) -> [f32; 3] {
        let spec = self.design.spec();
        let p = fraction * 6.0;
        let segment = (p as usize).min(5);
        let a = spec.nodes[segment];
        let height = a + (spec.nodes[segment + 1] - a) * (p - segment as f32);
        self.vertex(
            self.along[1] + fraction * self.span(),
            0.0,
            self.sections[1][1] + height * self.sidework_height(),
        )
    }

    pub(crate) fn sidework_color(&self) -> [u8; 3] {
        self.design.spec().color
    }

    /// Render the live construction geometry in the project's artwork atlas.
    pub fn artwork(design: BridgeDesign) -> Self {
        let span = if design.style() == BridgeStyle::LowGirder {
            12.0
        } else {
            23.0
        };
        let half = span * 0.5;
        Self {
            water: BridgeWater::River,
            style: design.style(),
            sections: [
                [-half - 1.0, 0.85, 0.0],
                [-half, 1.15, 0.0],
                [half, 1.15, 0.0],
                [half + 1.0, 0.85, 0.0],
            ],
            along: [0.0, 1.0, span + 1.0, span + 2.0],
            axis: [1.0, 0.0],
            bank_floor: [0.25; 2],
            high_water: 0.0,
            design,
        }
    }

    pub fn sections(&self) -> &[[f32; 3]; 4] {
        &self.sections
    }

    pub fn span(&self) -> f32 {
        self.along[2] - self.along[1]
    }

    pub fn width(&self) -> f32 {
        HALF_WIDTH * 2.0
    }

    pub fn freeboard(&self) -> f32 {
        self.sections[1][1] - DECK_THICKNESS - self.high_water
    }

    pub fn bounds(&self) -> [f32; 4] {
        let [a, _, _, b] = self.sections;
        let width = HALF_WIDTH + 0.34;
        [
            a[0].min(b[0]) - width,
            a[2].min(b[2]) - width,
            a[0].max(b[0]) + width,
            a[2].max(b[2]) + width,
        ]
    }

    pub fn mesh_bounds(&self) -> [[f32; 3]; 2] {
        let [left, near, right, far] = self.bounds();
        let low = (self.sections[1][1] - DECK_THICKNESS)
            .min(self.sections[0][1].min(self.sections[3][1]))
            .min(self.bank_floor[0].min(self.bank_floor[1]) - 0.10);
        let rise = self.sidework_height() + 0.15;
        let top = (self.sections[1][1] + rise).max(self.sections[0][1].max(self.sections[3][1]));
        [[left, low, near], [right, top, far]]
    }

    pub fn draw(&self, mesh: &mut crate::world_mesh::WorldMesh<'_>) {
        self.draw_fading(mesh, 1.0);
    }

    /// Fade the detailed geometry only when a matching distant silhouette exists.
    pub fn draw_fading(&self, mesh: &mut crate::world_mesh::WorldMesh<'_>, opacity: f32) {
        if opacity <= 0.02 {
            return;
        }
        let sidework = self.sidework_color();
        self.triangles(|triangle| {
            let color = match triangle.part {
                BridgePart::Approach => [106, 103, 95],
                BridgePart::Deck => [119, 119, 114],
                BridgePart::Girder => [61, 80, 86],
                BridgePart::Abutment => [129, 122, 105],
                BridgePart::Guardrail if self.style == BridgeStyle::SmallTruss => [177, 178, 163],
                BridgePart::Guardrail | BridgePart::Truss => sidework,
            };
            if opacity >= 1.0 {
                mesh.lit_triangle(triangle.corners, color);
            } else {
                mesh.faded_triangle(triangle.corners, color, opacity);
            }
        });
    }

    /// Surface of the bridge or its dry approaches. Use for a 3D road surface
    /// or clearance query; the terrain and water beneath remain independent.
    pub fn road_height(&self, x: f32, z: f32) -> Option<f32> {
        let dx = x - self.sections[0][0];
        let dz = z - self.sections[0][2];
        let along = dx * self.axis[0] + dz * self.axis[1];
        let across = dz * self.axis[0] - dx * self.axis[1];
        if !(across.abs() <= HALF_WIDTH && (-0.001..=self.along[3] + 0.001).contains(&along)) {
            return None;
        }
        let segment = usize::from(along > self.along[1]) + usize::from(along > self.along[2]);
        let t = ((along - self.along[segment]) / (self.along[segment + 1] - self.along[segment]))
            .clamp(0.0, 1.0);
        Some(
            self.sections[segment][1]
                + (self.sections[segment + 1][1] - self.sections[segment][1]) * t,
        )
    }

    /// Emit real 3D surfaces and open sidework. All supports terminate on dry banks.
    pub fn triangles(&self, mut emit: impl FnMut(BridgeTriangle)) {
        let [_, deck_start, deck_end, _] = self.along;
        let height = self.sections[1][1];
        for i in 0..3 {
            let a = self.along[i];
            let b = self.along[i + 1];
            let ay = self.sections[i][1];
            let by = self.sections[i + 1][1];
            quad(
                &mut emit,
                if i == 1 {
                    BridgePart::Deck
                } else {
                    BridgePart::Approach
                },
                [
                    self.vertex(a, HALF_WIDTH, ay),
                    self.vertex(b, HALF_WIDTH, by),
                    self.vertex(b, -HALF_WIDTH, by),
                    self.vertex(a, -HALF_WIDTH, ay),
                ],
            );
        }
        for hand in [-1.0, 1.0] {
            let edge = hand * HALF_WIDTH;
            let mut corners = [
                self.vertex(deck_start, edge, height),
                self.vertex(deck_end, edge, height),
                self.vertex(deck_end, edge, height - DECK_THICKNESS),
                self.vertex(deck_start, edge, height - DECK_THICKNESS),
            ];
            if hand > 0.0 {
                corners.reverse();
            }
            quad(&mut emit, BridgePart::Girder, corners);
        }
        quad(
            &mut emit,
            BridgePart::Girder,
            [
                self.vertex(deck_start, -HALF_WIDTH, height - DECK_THICKNESS),
                self.vertex(deck_end, -HALF_WIDTH, height - DECK_THICKNESS),
                self.vertex(deck_end, HALF_WIDTH, height - DECK_THICKNESS),
                self.vertex(deck_start, HALF_WIDTH, height - DECK_THICKNESS),
            ],
        );
        for (end, bank) in [deck_start, deck_end].into_iter().zip(self.bank_floor) {
            self.abutment(&mut emit, end, bank);
        }
        for hand in [-1.0, 1.0] {
            self.sidework(&mut emit, hand * (HALF_WIDTH - 0.12));
        }
    }

    fn sidework(&self, emit: &mut impl FnMut(BridgeTriangle), side: f32) {
        let spec = self.design.spec();
        let truss = self.style == BridgeStyle::SmallTruss;
        let part = if truss {
            BridgePart::Truss
        } else {
            BridgePart::Guardrail
        };
        let rise = self.sidework_height();
        let height = self.sections[1][1];
        let start = self.along[1];
        let span = self.span();
        let top: [[f32; 2]; 7] = core::array::from_fn(|i| {
            [
                start + span * (i as f32 / 6.0),
                height + rise * spec.nodes[i],
            ]
        });
        let base = |column: usize| [top[column][0], height + 0.02];
        let member =
            |e: &mut _, part, from, to, width| self.side_member(e, part, side, from, to, width);
        if truss {
            member(
                emit,
                BridgePart::Guardrail,
                [start, height + 0.50],
                [self.along[2], height + 0.50],
                0.12,
            );
        }
        for i in 0..6 {
            let [a, b] = [top[i], top[i + 1]];
            if matches!(spec.web, Web::Panel) {
                quad(
                    emit,
                    part,
                    [
                        self.vertex(a[0], side, a[1]),
                        self.vertex(b[0], side, b[1]),
                        self.vertex(b[0], side, height + 0.02),
                        self.vertex(a[0], side, height + 0.02),
                    ],
                );
                continue;
            }
            member(emit, part, a, b, spec.member);
            let [left, right] = [base(i), base(i + 1)];
            match spec.web {
                Web::Posts | Web::Vertical => {}
                Web::Pickets => {
                    let station = (a[0] + b[0]) * 0.5;
                    member(
                        emit,
                        part,
                        [station, height + 0.02],
                        [station, (a[1] + b[1]) * 0.5],
                        spec.member,
                    );
                }
                Web::Crossed => {
                    member(emit, part, left, b, spec.member);
                    member(emit, part, a, right, spec.member);
                }
                Web::Pratt if i < 3 => member(emit, part, a, right, spec.member),
                Web::Howe if i >= 3 => member(emit, part, a, right, spec.member),
                Web::Warren | Web::Diagonals if i % 2 != 0 => {
                    member(emit, part, a, right, spec.member)
                }
                Web::Pratt | Web::Howe | Web::Warren | Web::Diagonals => {
                    member(emit, part, left, b, spec.member)
                }
                Web::Panel => unreachable!(),
            }
        }
        if !matches!(spec.web, Web::Panel) {
            for (i, tip) in top.into_iter().enumerate() {
                member(emit, part, base(i), tip, spec.member);
            }
        }
    }

    fn vertex(&self, along: f32, across: f32, height: f32) -> [f32; 3] {
        [
            self.sections[0][0] + self.axis[0] * along - self.axis[1] * across,
            height,
            self.sections[0][2] + self.axis[1] * along + self.axis[0] * across,
        ]
    }

    fn prove_coastal_footprint(
        &self,
        world: &World,
        blocked: &mut impl FnMut(f32, f32) -> bool,
        pulse: &mut impl FnMut(),
    ) -> bool {
        for (segment, pair) in self.along.windows(2).enumerate() {
            let start = pair[0];
            let distance = pair[1] - start;
            let steps = (distance / 0.34) as usize + 1;
            for step in 0..=steps {
                if step & 3 == 0 {
                    pulse();
                }
                let t = step as f32 / steps as f32;
                let station = start + distance * t;
                let a = self.sections[segment][1];
                let b = self.sections[segment + 1][1];
                let height = a + (b - a) * t;
                for side in -10..=10 {
                    let [x, _, z] = self.vertex(station, side as f32 * (HALF_WIDTH / 10.0), height);
                    let ground = world.elevation(x, z);
                    let valid = if segment == 1 {
                        if ground <= SEA_LEVEL {
                            world.coast_signal(x, z) >= -0.105
                                && height - DECK_THICKNESS >= SEA_LEVEL + FREEBOARD - 0.001
                        } else {
                            height - DECK_THICKNESS >= ground
                        }
                    } else {
                        ground >= SEA_LEVEL + COASTAL_FOOTING
                            && (-0.25..=1.65).contains(&(height - ground))
                    };
                    if !valid || blocked(x, z) {
                        return false;
                    }
                }
            }
        }
        for bank in [self.along[1], self.along[2]] {
            for offset in [-0.58, -0.29, 0.0, 0.29, 0.58] {
                pulse();
                for side in -10..=10 {
                    let [x, _, z] = self.vertex(
                        bank + offset,
                        side as f32 * ((HALF_WIDTH + 0.25) / 10.0),
                        0.0,
                    );
                    if world.elevation(x, z) < SEA_LEVEL + COASTAL_FOOTING || blocked(x, z) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn abutment(&self, emit: &mut impl FnMut(BridgeTriangle), station: f32, ground: f32) {
        let width = HALF_WIDTH + 0.25;
        let top = self.sections[1][1] - 0.12;
        let footprint = [
            [station - 0.58, -width],
            [station + 0.58, -width],
            [station + 0.58, width],
            [station - 0.58, width],
        ];
        for i in 0..4 {
            let [a, x] = footprint[i];
            let [b, z] = footprint[(i + 1) % 4];
            quad(
                emit,
                BridgePart::Abutment,
                [
                    self.vertex(a, x, top),
                    self.vertex(b, z, top),
                    self.vertex(b, z, ground - 0.10),
                    self.vertex(a, x, ground - 0.10),
                ],
            );
        }
    }

    fn side_member(
        &self,
        emit: &mut impl FnMut(BridgeTriangle),
        part: BridgePart,
        across: f32,
        a: [f32; 2],
        b: [f32; 2],
        thickness: f32,
    ) {
        let [along, rise] = [b[0] - a[0], b[1] - a[1]];
        let scale = thickness * 0.5 / length(along, rise);
        let [dx, dy] = [-rise * scale, along * scale];
        quad(
            emit,
            part,
            [
                self.vertex(a[0] - dx, across, a[1] - dy),
                self.vertex(b[0] - dx, across, b[1] - dy),
                self.vertex(b[0] + dx, across, b[1] + dy),
                self.vertex(a[0] + dx, across, a[1] + dy),
            ],
        );
    }
}

fn quad(emit: &mut impl FnMut(BridgeTriangle), part: BridgePart, [a, b, c, d]: [[f32; 3]; 4]) {
    emit(BridgeTriangle {
        corners: [a, b, c],
        part,
    });
    emit(BridgeTriangle {
        corners: [a, c, d],
        part,
    });
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crossings {
    bridges: [Bridge; BRIDGE_LIMIT],
    count: usize,
}

impl Crossings {
    pub const fn new() -> Self {
        Self {
            bridges: [Bridge::EMPTY; BRIDGE_LIMIT],
            count: 0,
        }
    }

    pub fn bridges(&self) -> &[Bridge] {
        &self.bridges[..self.count]
    }

    pub fn fit(
        world: &World,
        road: &RoadNetwork,
        watershed: Option<&Watershed>,
        allow_sea_narrows: bool,
    ) -> Self {
        Self::fit_cooperatively(world, road, watershed, allow_sea_narrows, &mut || {})
    }

    pub(crate) fn fit_cooperatively(
        world: &World,
        road: &RoadNetwork,
        watershed: Option<&Watershed>,
        allow_sea_narrows: bool,
        pulse: &mut impl FnMut(),
    ) -> Self {
        Self::fit_alignment(world, road.polyline(), watershed, allow_sea_narrows, pulse)
    }

    fn fit_alignment(
        world: &World,
        path: &[RoadPoint],
        watershed: Option<&Watershed>,
        allow_sea_narrows: bool,
        pulse: &mut impl FnMut(),
    ) -> Self {
        let mut result = Self::new();
        if path.len() < 2
            || (!allow_sea_narrows && watershed.is_none_or(|water| water.streams().is_empty()))
        {
            return result;
        }
        let context = Fit {
            world,
            watershed,
            allow_sea_narrows,
            coastal: false,
        };
        let mut from = 0;
        while from + 1 < path.len() && result.count < BRIDGE_LIMIT {
            let mut to = from + 1;
            let Some(mut line) = Line::between(path[from], path[to]) else {
                from = to;
                continue;
            };
            while to + 1 < path.len() {
                let [along, across] = line.local(path[to + 1].x, path[to + 1].z);
                if across.abs() > 0.015 || along <= line.length {
                    break;
                }
                line.length = along;
                to += 1;
            }
            context.scan(line, &mut result, pulse);
            from = to;
        }
        result
    }
}

impl Default for Crossings {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
struct Line {
    origin: [f32; 2],
    axis: [f32; 2],
    length: f32,
}

impl Line {
    fn between(a: RoadPoint, b: RoadPoint) -> Option<Self> {
        let [dx, dz] = [b.x - a.x, b.z - a.z];
        let length = length(dx, dz);
        (length >= 0.5 && length.is_finite()).then(|| Self {
            origin: [a.x, a.z],
            axis: [dx / length, dz / length],
            length,
        })
    }

    fn point(self, along: f32, across: f32) -> [f32; 2] {
        [
            self.origin[0] + self.axis[0] * along - self.axis[1] * across,
            self.origin[1] + self.axis[1] * along + self.axis[0] * across,
        ]
    }

    fn local(self, x: f32, z: f32) -> [f32; 2] {
        let [dx, dz] = [x - self.origin[0], z - self.origin[1]];
        [
            dx * self.axis[0] + dz * self.axis[1],
            dz * self.axis[0] - dx * self.axis[1],
        ]
    }
}

fn length(x: f32, z: f32) -> f32 {
    let mut estimate = x.abs().max(z.abs());
    if estimate == 0.0 {
        return 0.0;
    }
    let sum = x * x + z * z;
    for _ in 0..4 {
        estimate = (estimate + sum / estimate) * 0.5;
    }
    estimate
}

fn reaches_marine_water(world: &World, line: Line, middle: f32, pulse: &mut impl FnMut()) -> bool {
    const SIDE: usize = 41;
    const MID: usize = SIDE / 2;
    const STEP: f32 = 6.0;
    const DEEP_WATER: f32 = -0.105;
    let coast = |row: f32, column: f32| {
        let [x, z] = line.point(
            middle + (column - MID as f32) * STEP,
            (row - MID as f32) * STEP,
        );
        world.coast_signal(x, z)
    };
    let mut seen = [0_u64; SIDE];
    let mut queue = [0_u16; SIDE * SIDE];
    seen[MID] = 1_u64 << MID;
    queue[0] = (MID * SIDE + MID) as u16;
    let mut tail = 1;
    let mut head = 0;
    let mut deep = false;
    let mut open = false;
    while head < tail {
        if head & 7 == 0 {
            pulse();
        }
        let index = usize::from(queue[head]);
        head += 1;
        let row = index / SIDE;
        let column = index % SIDE;
        deep |= coast(row as f32, column as f32) < DEEP_WATER;
        open |= row == 0 || row == SIDE - 1 || column == 0 || column == SIDE - 1;
        if deep && open {
            return true;
        }
        for (r, c) in [
            (row.wrapping_sub(1), column),
            (row + 1, column),
            (row, column.wrapping_sub(1)),
            (row, column + 1),
        ] {
            if r >= SIDE || c >= SIDE || seen[r] & (1 << c) != 0 {
                continue;
            }
            if coast(r as f32, c as f32) > 0.0 {
                seen[r] |= 1 << c;
                continue;
            }
            if [0.25, 0.5, 0.75].into_iter().any(|t| {
                coast(
                    row as f32 + (r as f32 - row as f32) * t,
                    column as f32 + (c as f32 - column as f32) * t,
                ) > 0.0
            }) {
                continue;
            }
            seen[r] |= 1 << c;
            queue[tail] = (r * SIDE + c) as u16;
            tail += 1;
        }
    }
    false
}

struct Fit<'a> {
    world: &'a World,
    watershed: Option<&'a Watershed>,
    allow_sea_narrows: bool,
    coastal: bool,
}

impl Fit<'_> {
    fn wet(&self, x: f32, z: f32) -> Option<BridgeWater> {
        if self.allow_sea_narrows && self.world.coast_signal(x, z) <= 0.0 {
            return Some(BridgeWater::SeaNarrows);
        }
        self.watershed
            .and_then(|water| water.sample(x, z))
            .filter(|water| water.is_channel())
            .map(|_| BridgeWater::River)
    }

    fn ground(&self, line: Line, along: f32, across: f32) -> Option<f32> {
        let [x, z] = line.point(along, across);
        let height = self.world.elevation(x, z);
        let footing = if self.coastal { COASTAL_FOOTING } else { 0.36 };
        (height >= SEA_LEVEL + footing && self.wet(x, z).is_none()).then_some(height)
    }

    fn single_coastal_run(&self, line: Line, pulse: &mut impl FnMut()) -> Option<[f32; 2]> {
        let steps = (line.length / 0.55) as usize + 1;
        let step = line.length / steps as f32;
        let mut first = None;
        let mut span = None;
        for i in 0..=steps {
            if i & 7 == 0 {
                pulse();
            }
            let at = i as f32 * step;
            let [x, z] = line.point(at, 0.0);
            if self.world.coast_signal(x, z) <= 0.0 {
                if i == 0 || i == steps || span.is_some() {
                    return None;
                }
                first.get_or_insert(i);
            } else if let Some(begin) = first.take() {
                let kind = BridgeWater::SeaNarrows;
                let a = self.bank(line, (begin - 1) as f32 * step, begin as f32 * step, kind);
                let b = self.bank(line, at, (i - 1) as f32 * step, kind);
                span = Some([a, b]);
            }
        }
        span
    }

    fn scan(&self, line: Line, result: &mut Crossings, pulse: &mut impl FnMut()) {
        let steps = (line.length / 0.70) as usize + 1;
        let step = line.length / steps as f32;
        let mut run = None;
        for i in 0..=steps {
            if i & 7 == 0 {
                pulse();
            }
            let along = i as f32 * step;
            let [x, z] = line.point(along, 0.0);
            let wet = self.wet(x, z);
            if let Some(kind) = wet {
                let active = run.get_or_insert((i, kind, false));
                active.2 |= active.1 != kind;
            } else if let Some((first, kind, mixed)) = run.take() {
                if first == 0 || mixed {
                    continue;
                }
                let a = self.bank(line, (first - 1) as f32 * step, first as f32 * step, kind);
                let b = self.bank(line, along, (i - 1) as f32 * step, kind);
                if let Some(bridge) = self.bridge(line, a, b, kind, pulse) {
                    if result.bridges().iter().any(|other| {
                        bridge
                            .sections
                            .iter()
                            .any(|p| other.road_height(p[0], p[2]).is_some())
                            || other
                                .sections
                                .iter()
                                .any(|p| bridge.road_height(p[0], p[2]).is_some())
                    }) {
                        continue;
                    }
                    result.bridges[result.count] = bridge;
                    result.count += 1;
                    if result.count == BRIDGE_LIMIT {
                        return;
                    }
                }
            }
        }
    }

    fn bank(&self, line: Line, mut dry: f32, mut wet: f32, kind: BridgeWater) -> f32 {
        for _ in 0..5 {
            let middle = (dry + wet) * 0.5;
            let [x, z] = line.point(middle, 0.0);
            if self.wet(x, z) == Some(kind) {
                wet = middle;
            } else {
                dry = middle;
            }
        }
        dry
    }

    fn landing(
        &self,
        line: Line,
        bank: f32,
        side: f32,
        pulse: &mut impl FnMut(),
    ) -> Option<(f32, f32)> {
        let setbacks: &[f32] = if self.coastal {
            &[1.25, 2.25, 3.4, 4.7, 6.2, 8.0, 10.0]
        } else {
            &[1.25, 2.25, 3.4, 4.7]
        };
        for &setback in setbacks {
            if self.coastal {
                pulse();
            }
            let at = bank + side * setback;
            if at < 0.65 || at > line.length - 0.65 {
                continue;
            }
            let mut low = f32::INFINITY;
            let mut high = f32::NEG_INFINITY;
            for out in [at - 0.65, at, at + 0.65] {
                for cross in [-HALF_WIDTH - 0.26, 0.0, HALF_WIDTH + 0.26] {
                    let h = self.ground(line, out, cross).unwrap_or(f32::NEG_INFINITY);
                    low = low.min(h);
                    high = high.max(h);
                }
            }
            if low.is_finite() && high - low <= 0.78 {
                return Some((at, low));
            }
        }
        None
    }

    fn bridge(
        &self,
        line: Line,
        wet_start: f32,
        wet_end: f32,
        kind: BridgeWater,
        pulse: &mut impl FnMut(),
    ) -> Option<Bridge> {
        let wet_width = wet_end - wet_start;
        let span_limit = if self.coastal {
            COASTAL_MAX_SPAN
        } else if kind == BridgeWater::River {
            40.0
        } else {
            24.0
        };
        let wet_limit = if self.coastal {
            COASTAL_MAX_WET
        } else {
            span_limit - 2.5
        };
        if !(2.0..=wet_limit).contains(&wet_width) {
            return None;
        }
        let (a, low_a) = self.landing(line, wet_start, -1.0, pulse)?;
        let (b, low_b) = self.landing(line, wet_end, 1.0, pulse)?;
        let span = b - a;
        if span > span_limit {
            return None;
        }
        let (top, high_water) = self.deck_height(line, a, b, kind, pulse)?;
        let (start, start_height) = self.approach(line, a, top, -1.0, pulse)?;
        let (end, end_height) = self.approach(line, b, top, 1.0, pulse)?;
        let distances = [start, a, b, end];
        let heights = [start_height, top, top, end_height];
        let sections = core::array::from_fn(|i| {
            let [x, z] = line.point(distances[i], 0.0);
            [x, heights[i], z]
        });
        let style = if span >= 14.0 && wet_width >= 7.0 {
            BridgeStyle::SmallTruss
        } else {
            BridgeStyle::LowGirder
        };
        let [x, _, z] = sections[1];
        let key = crate::math::mix((u64::from(x.to_bits()) << 32) | u64::from(z.to_bits()));
        Some(Bridge {
            water: kind,
            style,
            design: BridgeDesign::select(style, key),
            sections,
            along: distances.map(|s| s - start),
            axis: line.axis,
            bank_floor: [low_a, low_b],
            high_water,
        })
    }

    fn deck_height(
        &self,
        line: Line,
        a: f32,
        b: f32,
        kind: BridgeWater,
        pulse: &mut impl FnMut(),
    ) -> Option<(f32, f32)> {
        let steps = ((b - a) / 0.90) as usize + 1;
        let mut peak_water = f32::NEG_INFINITY;
        let mut peak_dry = f32::NEG_INFINITY;
        let mut spans_channel = false;
        for step in 0..=steps {
            if self.coastal && step & 7 == 0 {
                pulse();
            }
            let at = a + (b - a) * step as f32 / steps as f32;
            let mut all_wet = true;
            for cross in [
                -HALF_WIDTH,
                -HALF_WIDTH * 0.5,
                0.0,
                HALF_WIDTH * 0.5,
                HALF_WIDTH,
            ] {
                let [x, z] = line.point(at, cross);
                let raw = self.world.elevation(x, z);
                if raw <= SEA_LEVEL {
                    if kind != BridgeWater::SeaNarrows || self.world.coast_signal(x, z) < -0.105 {
                        return None;
                    }
                    peak_water = peak_water.max(SEA_LEVEL);
                } else if let Some(river) = self
                    .watershed
                    .and_then(|water| water.sample(x, z))
                    .filter(|r| r.is_channel())
                {
                    if kind != BridgeWater::River {
                        return None;
                    }
                    peak_water = peak_water.max(river.level.min(raw));
                } else {
                    all_wet = false;
                    peak_dry = peak_dry.max(raw);
                }
            }
            spans_channel |= all_wet;
        }
        if !spans_channel || !peak_water.is_finite() {
            return None;
        }
        let top = (peak_water + FREEBOARD + DECK_THICKNESS).max(peak_dry + DECK_THICKNESS + 0.07);
        Some((top, peak_water))
    }

    fn approach(
        &self,
        line: Line,
        deck: f32,
        deck_height: f32,
        side: f32,
        pulse: &mut impl FnMut(),
    ) -> Option<(f32, f32)> {
        for distance in [2.5, 3.5, 5.0, 7.0, 9.5, 12.0, 15.0, 18.5] {
            if self.coastal {
                pulse();
            }
            let outer = deck + side * distance;
            if !(0.0..=line.length).contains(&outer) {
                continue;
            }
            let Some(start) = self.ground(line, outer, 0.0) else {
                continue;
            };
            if (deck_height - start).abs() > distance * APPROACH_GRADE {
                continue;
            }
            let steps = (distance / 1.0) as usize + 1;
            let supported = (0..=steps).all(|i| {
                let t = i as f32 / steps as f32;
                let at = outer + (deck - outer) * t;
                let y = start + (deck_height - start) * t;
                [-HALF_WIDTH, 0.0, HALF_WIDTH].into_iter().all(|cross| {
                    self.ground(line, at, cross)
                        .is_some_and(|land| (-0.25..=1.65).contains(&(y - land)))
                })
            });
            if supported {
                return Some((outer, start));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watershed::WatershedWork;

    #[test]
    fn coastal_chords_fit_shallow_inlets_with_grounded_ends() {
        for (seed, a, b) in [
            (67, [-991.8888, 7583.4556], [-895.02423, 7558.3716]),
            (7, [-2295.4922, 2081.726], [-2237.907, 2160.9849]),
        ] {
            let world = World::new(seed);
            let mut pulses = 0;
            let bridge =
                Bridge::fit_sea_narrows(&world, a, b, |_, _| false, || pulses += 1).unwrap();
            assert!(pulses > 0 && pulses < 500, "{pulses} pulses");
            assert_eq!(
                bridge,
                Bridge::fit_sea_narrows(&world, a, b, |_, _| false, || {}).unwrap()
            );
            assert_eq!(bridge.water, BridgeWater::SeaNarrows);
            assert_eq!(bridge.style, BridgeStyle::SmallTruss);
            assert!((45.0..=COASTAL_MAX_SPAN).contains(&bridge.span()));
            assert!(bridge.freeboard() >= FREEBOARD - 0.001);
            for point in [bridge.sections[0], bridge.sections[3]] {
                let [x, height, z] = point;
                assert_offset(bridge.road_height(x, z).unwrap(), height);
                assert_offset(world.elevation(x, z), height);
            }
            for (i, segment) in bridge.sections.windows(2).enumerate() {
                let [start, end] = [segment[0], segment[1]];
                let distance = length(end[0] - start[0], end[2] - start[2]);
                assert!((end[1] - start[1]).abs() <= distance * APPROACH_GRADE + 0.001);
                let steps = (distance / 0.22) as usize + 1;
                let mut wet = 0;
                for step in 0..=steps {
                    let station = bridge.along[i] + distance * (step as f32 / steps as f32);
                    for side in -10..=10 {
                        let [x, _, z] = bridge.vertex(station, side as f32 * 0.30, 0.0);
                        if world.coast_signal(x, z) <= 0.0 {
                            assert_eq!(i, 1);
                            assert!(world.coast_signal(x, z) >= -0.105);
                            assert!(
                                bridge.road_height(x, z).unwrap() - DECK_THICKNESS
                                    >= FREEBOARD - 0.001
                            );
                            wet += 1;
                        }
                    }
                }
                assert_eq!(wet > 0, i == 1);
            }

            let [low, high] = bridge.mesh_bounds();
            bridge.triangles(|triangle| {
                for point in triangle.corners {
                    for i in 0..3 {
                        assert!((low[i]..=high[i]).contains(&point[i]));
                    }
                    if triangle.part == BridgePart::Abutment {
                        assert!(world.elevation(point[0], point[2]) >= SEA_LEVEL + COASTAL_FOOTING);
                    }
                }
            });
            let [x, _, z] = bridge.vertex((bridge.along[1] + bridge.along[2]) * 0.5, 0.0, 0.0);
            let near_center = |px: f32, pz: f32| (px - x).abs() < 0.8 && (pz - z).abs() < 0.8;
            assert!(Bridge::fit_sea_narrows(&world, a, b, near_center, || {}).is_none());
        }
    }

    #[test]
    fn coastal_chords_reject_isolated_water_and_unbounded_alignment() {
        let world = World::new(85);
        let a = [778.5, -820.74365];
        let b = [778.5, -742.7678];
        let line = Line::between(
            RoadPoint {
                x: a[0],
                z: a[1],
                elevation: 0.0,
            },
            RoadPoint {
                x: b[0],
                z: b[1],
                elevation: 0.0,
            },
        )
        .unwrap();
        let fit = Fit {
            world: &world,
            watershed: None,
            allow_sea_narrows: true,
            coastal: true,
        };
        let [near, far] = fit.single_coastal_run(line, &mut || {}).unwrap();
        assert!(
            fit.bridge(line, near, far, BridgeWater::SeaNarrows, &mut || {})
                .is_some()
        );
        assert!(Bridge::fit_sea_narrows(&world, a, b, |_, _| false, || {}).is_none());
        assert!(Bridge::fit_sea_narrows(&world, a, [778.5, -600.0], |_, _| false, || {}).is_none());
        assert!(
            Bridge::fit_sea_narrows(&world, [f32::NAN, a[1]], b, |_, _| false, || {}).is_none()
        );

        let world = World::new(67);
        let a = [-1002.4393, 7594.4517];
        let b = [-880.4628, 7562.8647];
        let dry_road = |[x, z]: [f32; 2]| RoadPoint {
            x,
            z,
            elevation: world.elevation(x, z),
        };
        let line = Line::between(dry_road(a), dry_road(b)).unwrap();
        let fit = Fit {
            world: &world,
            watershed: None,
            allow_sea_narrows: true,
            coastal: true,
        };
        let [near, far] = fit.single_coastal_run(line, &mut || {}).unwrap();
        assert!((40.0..42.0).contains(&(far - near)));
        assert!(Bridge::fit_sea_narrows(&world, a, b, |_, _| false, || {}).is_none());
    }

    fn assert_offset(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 0.002, "{actual} vs {expected}");
    }

    #[test]
    fn bridges_follow_actual_roads_and_leave_water_below_the_deck() {
        let mut work = WatershedWork::new();
        let mut water = Watershed::new();
        for (seed, rx, rz, count) in [(1, 1, -1, 1), (98, 1, 2, 2)] {
            let world = World::new(seed);
            let road = RoadNetwork::generate(&world, rx, rz).unwrap();
            water.rebuild(&world, seed, rx, rz, &mut work);
            let plans = Crossings::fit(&world, &road, Some(&water), false);
            assert_eq!(plans.bridges().len(), count);
            assert_eq!(plans, Crossings::fit(&world, &road, Some(&water), false));
            assert!(
                Crossings::fit(&world, &road, None, false)
                    .bridges()
                    .is_empty()
            );
            for bridge in plans.bridges() {
                assert_eq!(bridge.water, BridgeWater::River);
                assert!(bridge.freeboard() >= FREEBOARD - 0.001);
                assert!(bridge.road_height(f32::NAN, 0.0).is_none());
                for (segment, [a, b]) in
                    bridge.sections.windows(2).map(|q| [q[0], q[1]]).enumerate()
                {
                    let len = length(b[0] - a[0], b[2] - a[2]);
                    assert!((b[1] - a[1]).abs() / len <= APPROACH_GRADE + 0.001);
                    for x in [a, b] {
                        assert!((bridge.road_height(x[0], x[2]).unwrap() - x[1]).abs() < 0.002);
                        assert!(road.proximity(x[0], x[2]).unwrap().distance_squared < 0.001);
                    }
                    if segment != 1 {
                        let join = if segment == 0 { a } else { b };
                        assert!((world.elevation(join[0], join[2]) - join[1]).abs() < 0.001);
                    }
                }

                let mut wet = 0;
                let limit = (bridge.along[3] * 4.0) as usize;
                for step in 0..=limit {
                    let along = bridge.along[3] * step as f32 / limit as f32;
                    for across in [-ROAD_HALF_WIDTH, 0.0, ROAD_HALF_WIDTH] {
                        let [x, _, z] = bridge.vertex(along, across, 0.0);
                        let height = bridge.road_height(x, z).unwrap();
                        let base = world.elevation(x, z);
                        if let Some(sample) =
                            water.sample(x, z).filter(|sample| sample.is_channel())
                        {
                            assert!((bridge.along[1]..=bridge.along[2]).contains(&along));
                            assert!(height - DECK_THICKNESS - sample.level.min(base) > 0.40);
                            wet += 1;
                        } else {
                            assert!(base > SEA_LEVEL);
                        }
                    }
                }
                assert!(wet >= 20);

                let (mut deck_faces, mut guard_faces, mut truss_faces) = (0, 0, 0);
                bridge.triangles(|face| {
                    let [a, b, c] = face.corners;
                    assert!(face.corners.into_iter().flatten().all(f32::is_finite));
                    if face.part == BridgePart::Deck {
                        let normal_y =
                            (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]);
                        assert!(normal_y > 0.0);
                        deck_faces += 1;
                    }
                    if face.part == BridgePart::Abutment {
                        for [x, _, z] in [a, b, c] {
                            assert!(world.elevation(x, z) > SEA_LEVEL + 0.35);
                            assert!(water.sample(x, z).is_none_or(|sample| !sample.is_channel()));
                        }
                    }
                    guard_faces += usize::from(face.part == BridgePart::Guardrail);
                    truss_faces += usize::from(face.part == BridgePart::Truss);
                });
                assert_eq!(deck_faces, 2);
                assert!(guard_faces > 0);
                assert_eq!(truss_faces > 0, bridge.style == BridgeStyle::SmallTruss);
            }
        }
    }
}
