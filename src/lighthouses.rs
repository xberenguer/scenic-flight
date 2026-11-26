//! Grounded headland lights with slowly rotating, dusk-sensitive lanterns.

use core::f32::consts::TAU;

use crate::beach::{BEACH_TILE_SIZE, BeachSite};
use crate::coastal_landforms::{COASTAL_LANDFORM_TILE, CoastalLandform};
use crate::flight::TICKS_PER_SECOND;
use crate::math::{fraction, mix, sin_cos, smoothstep};
use crate::ports::Port;
use crate::world::{SEA_LEVEL, Sample, Surface, World};
use crate::world_mesh::WorldMesh;

mod designs;
pub use designs::LighthouseDesign;
use designs::{Design, Roof};

pub const LIGHTHOUSE_TILE: i32 = 384;
const OCTAGON: [[f32; 2]; 8] = [
    [1.0, 0.0],
    [0.707_106_77, 0.707_106_77],
    [0.0, 1.0],
    [-0.707_106_77, 0.707_106_77],
    [-1.0, 0.0],
    [-0.707_106_77, -0.707_106_77],
    [0.0, -1.0],
    [0.707_106_77, -0.707_106_77],
];
const SQUARE: [[f32; 2]; 8] = [
    [0.707_106_77, 0.0],
    OCTAGON[1],
    [0.0, 0.707_106_77],
    OCTAGON[3],
    [-0.707_106_77, 0.0],
    OCTAGON[5],
    [0.0, -0.707_106_77],
    OCTAGON[7],
];
const PLINTH: f32 = 3.55;
const PATH_HALF_WIDTH: f32 = 0.85;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Headland {
    edge: [f32; 2],
    out: [f32; 2],
}

impl Headland {
    fn point(self, along: f32, out: f32) -> [f32; 2] {
        offset(
            offset(self.edge, perpendicular(self.out), along),
            self.out,
            out,
        )
    }

    fn find(world: &World, point: [f32; 2]) -> Option<Self> {
        if shore_signal(world, point).abs() > 0.22 {
            return None;
        }
        let [x, z] = point;
        let dx = world.coast_signal(x - 10.0, z) - world.coast_signal(x + 10.0, z);
        let dz = world.coast_signal(x, z - 10.0) - world.coast_signal(x, z + 10.0);
        let gradient = root(dx * dx + dz * dz);
        if gradient < 0.021 {
            return None;
        }
        let out = [dx / gradient, dz / gradient];
        let p = |distance| offset(point, out, distance);
        let (mut dry, mut wet) = (-78.0, 78.0);
        if shore_signal(world, p(dry)) <= 0.0 || shore_signal(world, p(wet)) > 0.0 {
            return None;
        }
        for _ in 0..10 {
            let middle = (dry + wet) * 0.5;
            if shore_signal(world, p(middle)) > 0.0 {
                dry = middle;
            } else {
                wet = middle;
            }
        }
        let shore = Self {
            edge: p((dry + wet) * 0.5),
            out,
        };
        let left = shore_signal(world, shore.point(-39.0, 0.0));
        let right = shore_signal(world, shore.point(39.0, 0.0));
        if left + right > -0.010
            || left.max(right) > 0.026
            || elevation(world, shore.point(0.0, -96.0)) < SEA_LEVEL + 8.0
        {
            return None;
        }
        Some(shore)
    }

    fn hazardous_water(self, world: &World) -> bool {
        let mut shallow = 0;
        for distance in [14.0, 27.0, 39.0] {
            for along in [-18.0, 0.0, 18.0] {
                let [x, z] = self.point(along, distance);
                shallow += usize::from(world.sample(x, z).surface == Surface::Shallows);
            }
        }
        let mut offshore = 0;
        for distance in [70.0, 103.0] {
            for along in [-24.0, 0.0, 24.0] {
                let [x, z] = self.point(along, distance);
                offshore += usize::from(world.sample(x, z).surface == Surface::DeepWater);
            }
        }
        shallow >= 6 && offshore >= 2
    }
}

/// An immutable tower and its short inland maintenance approach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lighthouse {
    center: [f32; 2],
    approach: [f32; 2],
    headland: Headland,
    foundation: [f32; 2],
    gallery_height: f32,
    radius: f32,
    period: u16,
    phase: u16,
    design: LighthouseDesign,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TowerForm {
    Masonry,
    Keeper,
    Lattice,
}

impl TowerForm {
    fn on_headland(elevation: f32, seed: u64) -> Self {
        let pick = mix(seed ^ 0x1b82_b4c6_b867_22c3) % 8;
        // Lower sites favor taller, solid towers. A higher footing can
        // carry a compact keeper tower or a lighter open-frame lantern.
        if pick < 4 {
            if elevation < 7.0 {
                return Self::Masonry;
            }
            if elevation >= 10.0 {
                return Self::Keeper;
            }
        }
        match pick {
            0..=2 | 6 => Self::Lattice,
            3 | 4 => Self::Keeper,
            _ => Self::Masonry,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Daymark {
    Whitewash,
    PaintedBands,
    RedBrick,
    BlackBands,
}

impl Daymark {
    fn draw(
        self,
        mesh: &mut WorldMesh<'_>,
        start: [[f32; 3]; 8],
        end: [[f32; 3]; 8],
        from: f32,
        to: f32,
    ) {
        let white = [215, 212, 193];
        let stripe = match self {
            Self::Whitewash => return shell(mesh, start, end, white),
            Self::RedBrick => return shell(mesh, start, end, [161, 102, 77]),
            Self::PaintedBands => [146, 54, 46],
            Self::BlackBands => [55, 63, 67],
        };
        let mut lower = start;
        for (level, color) in [
            (0.23, white),
            (0.39, stripe),
            (0.65, white),
            (0.82, stripe),
            (1.0, white),
        ] {
            if level <= from {
                continue;
            }
            let upper = core::array::from_fn(|i| {
                lerp(start[i], end[i], ((level - from) / (to - from)).min(1.0))
            });
            shell(mesh, lower, upper, color);
            lower = upper;
            if level >= to {
                break;
            }
        }
    }
}

impl Lighthouse {
    pub fn generate(world: &World, seed: u64, tile_x: i32, tile_z: i32) -> Option<Self> {
        Self::generate_with(world, seed, tile_x, tile_z, |_| false)
    }

    /// Supplies the same prepared-region reservations whenever this tile is planned.
    pub fn generate_with(
        world: &World,
        seed: u64,
        tile_x: i32,
        tile_z: i32,
        mut excluded: impl FnMut(&Self) -> bool,
    ) -> Option<Self> {
        if !(-5400..5400).contains(&tile_x) || !(-5400..5400).contains(&tile_z) {
            return None;
        }
        let tile = (u64::from(tile_x as u32) << 32) | u64::from(tile_z as u32);
        let key = mix(seed ^ tile ^ 0x9c66_8012_36d2_e91f);
        if key & 7 >= 6 {
            return None;
        }
        let origin = [
            (tile_x * LIGHTHOUSE_TILE) as f32,
            (tile_z * LIGHTHOUSE_TILE) as f32,
        ];
        let first = (key >> 17) as usize & 15;
        let jitter = [fraction(mix(key)) * 11.0, fraction(mix(key ^ 69)) * 11.0];
        for attempt in 0..16 {
            let cell = (first + attempt * 7) & 15;
            let point = [
                origin[0] + 51.0 + (cell % 4) as f32 * 89.0 + jitter[0],
                origin[1] + 51.0 + (cell / 4) as f32 * 89.0 + jitter[1],
            ];
            let Some(shore) = Headland::find(world, point) else {
                continue;
            };
            if !shore.hazardous_water(world) {
                continue;
            }
            let state = mix(key ^ (attempt as u64 + 0xae9d));
            for inset in [56.0, 66.0, 76.0] {
                let Some(plan) = Self::place(world, shore, inset, state) else {
                    continue;
                };
                let [left, near, right, far] = plan.bounds();
                let edge = LIGHTHOUSE_TILE as f32 - 12.0;
                if left < origin[0] + 12.0
                    || near < origin[1] + 12.0
                    || right > origin[0] + edge
                    || far > origin[1] + edge
                    || excluded(&plan)
                    || !plan.clears_beaches(world, seed)
                    || !plan.clears_rock_sites(world, seed)
                {
                    continue;
                }
                return Some(plan);
            }
        }
        None
    }

    fn place(world: &World, headland: Headland, inset: f32, state: u64) -> Option<Self> {
        let side = (fraction(mix(state ^ 792)) - 0.5) * 18.0;
        let center = headland.point(side, -inset);
        let approach = headland.point(
            side + (fraction(mix(state ^ 18)) - 0.5) * 10.0,
            -inset - 23.0,
        );
        let [low, high] = foundation(world, center)?;
        if !accessible(world, center, approach) {
            return None;
        }
        let variety = fraction(state);
        let tower = TowerForm::on_headland(high, state);
        let design = LighthouseDesign::choose(tower, mix(state ^ 0x36ea));
        let spec = design.spec();
        let gallery_height = spec.height * (0.96 + variety * 0.08);
        let radius = spec.radius;
        let period =
            (TICKS_PER_SECOND * 7 + ((state >> 17) as u32 % (TICKS_PER_SECOND * 4))) as u16;
        Some(Self {
            center,
            approach,
            headland,
            foundation: [low - 0.35, high + 0.19],
            gallery_height,
            radius,
            period,
            phase: ((state >> 30) % u64::from(period)) as u16,
            design,
        })
    }

    pub fn design(&self) -> LighthouseDesign {
        self.design
    }

    /// Builds the real tower mesh above a level footing for the artwork catalog.
    pub fn artwork(design: LighthouseDesign) -> Self {
        let spec = design.spec();
        Self {
            center: [0.0; 2],
            approach: [0.0, -23.0],
            headland: Headland {
                edge: [0.0, 56.0],
                out: [0.0, 1.0],
            },
            foundation: [-0.35, 0.19],
            gallery_height: spec.height,
            radius: spec.radius,
            period: (TICKS_PER_SECOND * 8) as u16,
            phase: 0,
            design,
        }
    }

    pub fn center(&self) -> [f32; 2] {
        self.center
    }

    pub fn shore(&self) -> [f32; 2] {
        self.headland.edge
    }

    pub fn approach(&self) -> [f32; 2] {
        self.approach
    }

    /// Bounds in minimum-x, minimum-z, maximum-x, maximum-z order.
    pub fn bounds(&self) -> [f32; 4] {
        let [x, z] = self.center;
        let [a, b] = self.approach;
        [
            (x - PLINTH).min(a - PATH_HALF_WIDTH),
            (z - PLINTH).min(b - PATH_HALF_WIDTH),
            (x + PLINTH).max(a + PATH_HALF_WIDTH),
            (z + PLINTH).max(b + PATH_HALF_WIDTH),
        ]
    }

    pub fn screen_bounds(
        &self,
        camera: crate::render::Camera,
        dimensions: (usize, usize),
    ) -> Option<crate::render::ScreenRect> {
        let [left, near, right, far] = self.bounds();
        crate::render::screen_region(
            camera,
            dimensions,
            [
                [left, self.foundation[0] - 0.05, near],
                [right, self.foundation[1] + self.gallery_height + 4.2, far],
            ],
        )
    }

    pub fn clears_port(&self, port: &Port) -> bool {
        separated(self.bounds(), port.bounds(), 12.0)
    }

    pub fn clears_landform(&self, rock: &CoastalLandform) -> bool {
        separated(self.bounds(), rock.bounds(), 15.0) && !within(self.shore(), rock.shore(), 32.0)
    }

    fn clears_rock_sites(&self, world: &World, seed: u64) -> bool {
        let [tx, tz] = self
            .shore()
            .map(|value| tile_of(value, COASTAL_LANDFORM_TILE));
        for z in tz - 1..=tz + 1 {
            for x in tx - 1..=tx + 1 {
                if CoastalLandform::generate(world, seed, x, z)
                    .is_some_and(|rock| !self.clears_landform(&rock))
                {
                    return false;
                }
            }
        }
        true
    }

    fn clears_beaches(&self, world: &World, seed: u64) -> bool {
        let [tx, tz] = self.shore().map(|value| tile_of(value, BEACH_TILE_SIZE));
        for z in tz - 1..=tz + 1 {
            for x in tx - 1..=tx + 1 {
                if BeachSite::reserved_footprint(world, seed, x, z).is_some_and(|beach| {
                    beach.conflicts_with(|x, z| {
                        let point = [x, z];
                        within(self.shore(), point, 42.0) || self.apron(point, 5.5)
                    })
                }) {
                    return false;
                }
            }
        }
        true
    }

    fn apron(&self, point: [f32; 2], margin: f32) -> bool {
        if within(self.center, point, PLINTH + margin) {
            return true;
        }
        let direction = subtract(self.approach, self.center);
        let relative = subtract(point, self.center);
        let along = (dot(direction, relative) / dot(direction, direction)).clamp(0.0, 1.0);
        within(
            point,
            offset(self.center, direction, along),
            PATH_HALF_WIDTH + margin,
        )
    }

    /// Only the small maintenance apron and path are part of the heightfield.
    /// Existing water and built overlays keep their material and elevation.
    pub fn overlay(&self, x: f32, z: f32, ground: Sample) -> Sample {
        if ground.elevation > SEA_LEVEL
            && matches!(
                ground.surface,
                Surface::Grass | Surface::Sand | Surface::Rock | Surface::Desert | Surface::Forest
            )
            && self.apron([x, z], 0.0)
        {
            Sample {
                surface: Surface::Gravel,
                ..ground
            }
        } else {
            ground
        }
    }

    /// The surveyed tower footprint and full roof height for clearance queries.
    pub fn obstruction(&self, x: f32, z: f32) -> Option<f32> {
        within(self.center, [x, z], PLINTH)
            .then_some(self.foundation[1] + self.gallery_height + 4.15)
    }

    pub fn clearance(&self, x: f32, z: f32) -> f32 {
        let dx = x - self.center[0];
        let dz = z - self.center[1];
        let blend = 1.0 - smoothstep((dx * dx + dz * dz - PLINTH * PLINTH) * (1.0 / 412.0));
        (self.foundation[1] + self.gallery_height + 5.0) * blend
    }

    /// Draw against the terrain depth with the frame's daylight and flight ticks.
    pub fn draw(&self, mesh: &mut WorldMesh<'_>, daylight: f32, ticks: u64) {
        let base = self.foundation[1];
        let deck = base + self.gallery_height;
        let stone = [118, 117, 108];
        let spec = self.design.spec();
        let tower = self.design.form();
        let painted_trim = spec.trim;
        let frame = [58, 70, 73];
        let center = self.center;
        let shaft_section = match tower {
            TowerForm::Masonry => OCTAGON,
            TowerForm::Keeper | TowerForm::Lattice => SQUARE,
        };
        let lantern_radius = (self.radius * 0.88).clamp(1.75, 2.80);
        let lower = ring(center, self.foundation[0], PLINTH);
        let plinth = ring(center, base, PLINTH * 0.94);
        let foot = polygon(center, base, self.radius, shaft_section);
        let shoulder = polygon(center, deck - 0.55, self.radius * spec.taper, shaft_section);
        let middle = polygon(
            center,
            base + (self.gallery_height - 0.55) * spec.break_at,
            self.radius * spec.waist,
            shaft_section,
        );
        let ledge = ring(center, deck, lantern_radius * 1.06);
        let lower_glass = ring(center, deck + 0.16, lantern_radius * 0.67);
        let upper_glass = ring(center, deck + 2.08, lantern_radius * 0.67);
        shell(mesh, lower, plinth, stone);
        shell(mesh, plinth, foot, [151, 145, 128]);
        match tower {
            TowerForm::Masonry | TowerForm::Keeper => {
                spec.mark.draw(mesh, foot, middle, 0.0, spec.break_at);
                spec.mark.draw(mesh, middle, shoulder, spec.break_at, 1.0);
                if tower == TowerForm::Keeper {
                    draw_balcony(
                        mesh,
                        center,
                        middle[0][1],
                        (self.radius * spec.waist * 1.08).min(3.52),
                        painted_trim,
                    );
                }
            }
            TowerForm::Lattice => {
                draw_lattice(mesh, foot, shoulder);
                let radius = (self.radius * spec.waist).min(3.30);
                draw_balcony(mesh, center, middle[0][1], radius, painted_trim);
            }
        }
        shell(mesh, shoulder, ledge, painted_trim);
        shell(mesh, ledge, lower_glass, [115, 111, 97]);
        shell(mesh, lower_glass, upper_glass, frame);
        draw_roof(mesh, center, deck, lantern_radius, upper_glass, spec);

        let inland = [-self.headland.out[0], -self.headland.out[1]];
        let mut entrance = 0;
        let mut alignment = f32::NEG_INFINITY;
        let (dir, dusk) = self.lantern(daylight, ticks);
        for side in 0..8 {
            let next = (side + 1) & 7;
            let normal = offset(OCTAGON[side], OCTAGON[next], 1.0).map(|v| v * 0.541_196_1);
            let forward = dot(normal, inland);
            if forward > alignment {
                alignment = forward;
                entrance = side;
            }
            let shade = light_strength(normal, dir, dusk);
            let glass = [54.0, 88.0, 110.0];
            let lamp = [255.0, 207.0, 108.0];
            let color = core::array::from_fn(|i| (glass[i] + shade * (lamp[i] - glass[i])) as u8);
            let pane = window(lower_glass, upper_glass, side, 0.11, 0.12, 0.04);
            let [a, b, c, d] = pane;
            let emission = 0.32 * dusk + 0.68 * shade;
            mesh.luminous_triangle([a, b, c], color, emission);
            mesh.luminous_triangle([a, c, d], color, emission);
        }
        if tower != TowerForm::Lattice {
            let door = window(foot, middle, entrance, 0.24, 0.0, 0.06);
            let upper = (2.03 / ((self.gallery_height - 0.55) * spec.break_at)).min(1.0);
            let entry = [
                door[0],
                door[1],
                lerp(door[1], door[2], upper),
                lerp(door[0], door[3], upper),
            ];
            quad(mesh, entry, [57, 73, 75], false);
        }
    }

    fn lantern(&self, daylight: f32, ticks: u64) -> ([f32; 2], f32) {
        let dusk = smoothstep((0.54 - daylight) * (1.0 / 0.42));
        if dusk == 0.0 {
            return ([1.0, 0.0], 0.0);
        }
        let period = u64::from(self.period);
        let angle =
            ((ticks % period + u64::from(self.phase)) % period) as f32 * (TAU / period as f32);
        let (sine, cosine) = sin_cos(angle);
        ([cosine, sine], dusk)
    }
}

fn foundation(world: &World, center: [f32; 2]) -> Option<[f32; 2]> {
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for dx in [-3.65, 0.0, 3.65] {
        for dz in [-3.65, 0.0, 3.65] {
            let ground = world.sample(center[0] + dx, center[1] + dz);
            let height = ground.elevation;
            if ground.surface == Surface::Forest
                || !(SEA_LEVEL + 3.15..=SEA_LEVEL + 22.0).contains(&height)
            {
                return None;
            }
            low = low.min(height);
            high = high.max(height);
        }
    }
    (high - low <= 0.98).then_some([low, high])
}

fn accessible(world: &World, center: [f32; 2], endpoint: [f32; 2]) -> bool {
    let direction = subtract(endpoint, center);
    let length = root(dot(direction, direction));
    let normal = perpendicular(direction).map(|axis| axis / length);
    let spacing = length * (1.0 / 14.0);
    let mut previous = [0.0; 3];
    for step in 0..=14 {
        let point = offset(center, direction, step as f32 * (1.0 / 14.0));
        for (index, side) in [-PATH_HALF_WIDTH, 0.0, PATH_HALF_WIDTH]
            .into_iter()
            .enumerate()
        {
            let height = elevation(world, offset(point, normal, side));
            if height < SEA_LEVEL + 2.95
                || step > 0 && (height - previous[index]).abs() > spacing * 0.176 + 0.012
            {
                return false;
            }
            previous[index] = height;
        }
    }
    true
}

fn light_strength(normal: [f32; 2], direction: [f32; 2], dusk: f32) -> f32 {
    let sweep = smoothstep((dot(normal, direction) - 0.40) * (1.0 / 0.60));
    dusk * (0.12 + 0.88 * sweep)
}

fn ring(center: [f32; 2], height: f32, radius: f32) -> [[f32; 3]; 8] {
    polygon(center, height, radius, OCTAGON)
}

fn polygon(center: [f32; 2], height: f32, radius: f32, section: [[f32; 2]; 8]) -> [[f32; 3]; 8] {
    section.map(|axis| {
        let [x, z] = offset(center, axis, radius);
        [x, height, z]
    })
}

fn draw_balcony(
    mesh: &mut WorldMesh<'_>,
    center: [f32; 2],
    height: f32,
    radius: f32,
    tint: [u8; 3],
) {
    let bottom = ring(center, height - 0.24, radius * 0.90);
    let top = ring(center, height, radius);
    shell(mesh, bottom, top, tint);
    for i in 1..7 {
        mesh.lit_triangle([top[0], top[i + 1], top[i]], tint);
    }
}

fn draw_roof(
    mesh: &mut WorldMesh<'_>,
    center: [f32; 2],
    deck: f32,
    radius: f32,
    glass: [[f32; 3]; 8],
    spec: Design,
) {
    let square = if matches!(spec.roof, Roof::Pyramid) {
        SQUARE
    } else {
        OCTAGON
    };
    let profile: &[[f32; 2]] = match spec.roof {
        Roof::Flat => &[[2.25, 0.93], [2.66, 0.93]],
        Roof::Cone => &[[2.24, 0.88], [3.96, 0.09]],
        Roof::Dome => &[[2.24, 0.86], [2.89, 0.79], [3.48, 0.46], [3.61, 0.16]],
        Roof::Pyramid => &[[2.25, 1.00], [3.85, 0.08]],
        Roof::Cupola => &[
            [2.24, 0.86],
            [2.55, 0.91],
            [3.13, 0.62],
            [3.22, 0.29],
            [3.94, 0.15],
        ],
    };
    let mut lower = glass;
    for &[height, width] in profile {
        let upper = polygon(center, deck + height, radius * width, square);
        shell(mesh, lower, upper, spec.trim);
        lower = upper;
    }
    for i in 1..7 {
        mesh.lit_triangle([lower[0], lower[i + 1], lower[i]], spec.trim);
    }
}

fn draw_lattice(mesh: &mut WorldMesh<'_>, foot: [[f32; 3]; 8], shoulder: [[f32; 3]; 8]) {
    let steel = [111, 129, 129];
    for a in [1, 3, 5, 7] {
        let b = (a + 2) & 7;
        for (from, to) in [(a, b), (b, a)] {
            quad(
                mesh,
                [
                    foot[from],
                    lerp(foot[from], foot[to], 0.12),
                    lerp(shoulder[from], shoulder[to], 0.16),
                    shoulder[from],
                ],
                steel,
                true,
            );
        }
        let mid_b = lerp(foot[b], shoulder[b], 0.52);
        for (mut lower, mut upper) in [(foot[a], mid_b), (mid_b, shoulder[a])] {
            let first = [lower, upper];
            lower[1] += 0.25;
            upper[1] += 0.25;
            quad(mesh, [first[0], first[1], upper, lower], steel, true);
        }
    }
}

fn shell(mesh: &mut WorldMesh<'_>, lower: [[f32; 3]; 8], upper: [[f32; 3]; 8], color: [u8; 3]) {
    for side in 0..8 {
        let next = (side + 1) & 7;
        quad(
            mesh,
            [lower[side], upper[side], upper[next], lower[next]],
            color,
            true,
        );
    }
}

fn quad(mesh: &mut WorldMesh<'_>, [a, b, c, d]: [[f32; 3]; 4], color: [u8; 3], lit: bool) {
    if lit {
        mesh.lit_triangle([a, b, c], color);
        mesh.lit_triangle([a, c, d], color);
    } else {
        mesh.triangle([a, b, c], color);
        mesh.triangle([a, c, d], color);
    }
}

fn window(
    lower: [[f32; 3]; 8],
    upper: [[f32; 3]; 8],
    side: usize,
    inset: f32,
    trim: f32,
    stand_off: f32,
) -> [[f32; 3]; 4] {
    let next = (side + 1) & 7;
    let a = lerp(lower[side], lower[next], inset);
    let b = lerp(lower[next], lower[side], inset);
    let c = lerp(upper[next], upper[side], inset);
    let d = lerp(upper[side], upper[next], inset);
    let [nx, nz] = offset(OCTAGON[side], OCTAGON[next], 1.0);
    [
        lerp(a, d, trim),
        lerp(b, c, trim),
        lerp(c, b, trim),
        lerp(d, a, trim),
    ]
    .map(|[x, y, z]| [x + nx * stand_off, y, z + nz * stand_off])
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn elevation(world: &World, [x, z]: [f32; 2]) -> f32 {
    world.elevation(x, z)
}

fn shore_signal(world: &World, [x, z]: [f32; 2]) -> f32 {
    world.coast_signal(x, z)
}

fn perpendicular([x, z]: [f32; 2]) -> [f32; 2] {
    [-z, x]
}

fn subtract(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn offset(p: [f32; 2], axis: [f32; 2], distance: f32) -> [f32; 2] {
    [p[0] + axis[0] * distance, p[1] + axis[1] * distance]
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

fn within(a: [f32; 2], b: [f32; 2], radius: f32) -> bool {
    let d = subtract(a, b);
    dot(d, d) <= radius * radius
}

fn separated(a: [f32; 4], b: [f32; 4], margin: f32) -> bool {
    a[2] + margin < b[0] || b[2] + margin < a[0] || a[3] + margin < b[1] || b[3] + margin < a[1]
}

fn tile_of(coordinate: f32, side: i32) -> i32 {
    let scaled = coordinate / side as f32;
    let cell = scaled as i32;
    cell - i32::from(scaled < cell as f32)
}

fn root(value: f32) -> f32 {
    if value == 0.0 {
        return 0.0;
    }
    let estimate = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    let settled = 0.5 * (estimate + value / estimate);
    0.5 * (settled + value / settled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Camera, render_with_depth};
    use crate::sky::Sky;

    fn fixture() -> (World, Lighthouse) {
        let world = World::new(0);
        let lighthouse = Lighthouse::generate(&world, 0, -3, -3).unwrap();
        (world, lighthouse)
    }

    #[test]
    fn seeded_tower_preserves_dry_footings_and_reserved_ground() {
        let (world, plan) = fixture();
        assert_eq!(Some(plan), Lighthouse::generate(&world, 0, -3, -3));
        assert!(Lighthouse::generate_with(&world, 0, -3, -3, |_| true).is_none());
        assert!(plan.headland.hazardous_water(&world));
        assert!(plan.clears_beaches(&world, 0) && plan.clears_rock_sites(&world, 0));
        assert!(accessible(&world, plan.center(), plan.approach()));
        let [x, z] = plan.center();
        let [low, high] = foundation(&world, [x, z]).unwrap();
        assert!(high - low <= 0.98);
        let ground = world.sample(x, z);
        let apron = plan.overlay(x, z, ground);
        assert_eq!(apron.elevation, ground.elevation);
        assert_eq!(apron.surface, Surface::Gravel);
        assert!(plan.obstruction(x, z).unwrap() > high + 14.0);

        let port_world = World::new(1);
        let port = Port::generate(&port_world, 1, -368.0, -2584.0).unwrap();
        let mut reserved = plan;
        reserved.center = port.shore_origin();
        reserved.approach = offset(reserved.center, port.seaward(), -18.0);
        assert!(!reserved.clears_port(&port));
        assert!(plan.clears_port(&port));
    }

    #[test]
    fn all_tower_forms_are_grounded_and_draw_distinct_silhouettes() {
        const SIDE: usize = 112;
        const COUNT: usize = SIDE * SIDE;
        let silhouettes = [
            (0, -9, 6, TowerForm::Masonry),
            (0, -3, -3, TowerForm::Keeper),
            (1, -7, -12, TowerForm::Lattice),
        ]
        .map(|(seed, tile_x, tile_z, form)| {
            let world = World::new(seed);
            let plan = Lighthouse::generate(&world, seed, tile_x, tile_z).unwrap();
            assert_eq!(plan.design.form(), form);
            assert!(foundation(&world, plan.center()).is_some());
            assert!(accessible(&world, plan.center(), plan.approach()));
            let [x, z] = plan.center();
            assert!(
                plan.obstruction(x, z).unwrap() > plan.foundation[1] + plan.gallery_height + 3.50
            );
            let camera = Camera {
                x: x + 24.0,
                z: z - 30.0,
                altitude: plan.foundation[1] + 10.0,
                heading: (-24.0_f32).atan2(30.0),
                pitch: 0.0,
                bank: 0.0,
            };
            let sky = Sky::from_time(seed, 0, 0.35);
            let mut pixels = [0; COUNT];
            let mut depth = [u16::MAX; COUNT];
            plan.draw(
                &mut WorldMesh::new(&mut pixels, &mut depth, (SIDE, SIDE), camera, &sky).unwrap(),
                sky.daylight,
                0,
            );
            let mask = pixels.map(|pixel| pixel != 0);
            assert!(mask.iter().filter(|&&pixel| pixel).count() > 90);
            mask
        });
        assert!(silhouettes[0] != silhouettes[1]);
        assert!(silhouettes[0] != silhouettes[2]);
        assert!(silhouettes[1] != silhouettes[2]);
    }

    #[test]
    fn sweep_is_periodic_and_fades_in_after_dusk() {
        let (_, plan) = fixture();
        let mut previous = light_strength([1.0, 0.0], plan.lantern(0.0, 0).0, 1.0);
        let mut weakest = 1.0_f32;
        let mut brightest = 0.0_f32;
        for tick in 1..=u64::from(plan.period) {
            let (direction, dusk) = plan.lantern(0.0, tick);
            let strength = light_strength([1.0, 0.0], direction, dusk);
            assert!((strength - previous).abs() < 0.07);
            weakest = weakest.min(strength);
            brightest = brightest.max(strength);
            previous = strength;
        }
        assert!(weakest < 0.2 && brightest > 0.97);
        assert_eq!(
            plan.lantern(0.0, 0),
            plan.lantern(0.0, u64::from(plan.period))
        );
        assert_eq!(plan.lantern(1.0, 17).1, 0.0);
        assert!((0.0..1.0).contains(&plan.lantern(0.35, 17).1));
    }

    #[test]
    fn native_lantern_changes_only_a_small_aperture_region() {
        const SIDE: usize = 112;
        const COUNT: usize = SIDE * SIDE;
        let (world, plan) = fixture();
        let camera = Camera {
            x: -852.20,
            z: -1000.73,
            altitude: 17.75,
            heading: 2.625,
            pitch: 0.077,
            bank: -0.034,
        };
        let sky = Sky::from_time(0, 0, 0.0);
        let mut ground = [0; COUNT];
        let mut work = ground;
        let mut depth = ground;
        render_with_depth(
            |x, z| plan.overlay(x, z, world.sample(x, z)),
            camera,
            &sky,
            &mut ground,
            &mut work,
            &mut depth,
            (SIDE, SIDE),
        )
        .unwrap();
        let mut dim = ground;
        let mut lit = ground;
        let mut first_depth = depth;
        plan.draw(
            &mut WorldMesh::new(&mut dim, &mut first_depth, (SIDE, SIDE), camera, &sky).unwrap(),
            sky.daylight,
            81,
        );
        plan.draw(
            &mut WorldMesh::new(&mut lit, &mut depth, (SIDE, SIDE), camera, &sky).unwrap(),
            sky.daylight,
            288,
        );
        assert_eq!(first_depth, depth);
        let pixels = lit.iter().zip(dim).filter(|(a, b)| **a != *b).count();
        assert!((3..35).contains(&pixels));
    }
}
