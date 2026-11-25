//! Static beach umbrellas and towels, placed on the natural shoreline.

use crate::math::{fraction, mix, sin_cos};
use crate::render::Camera;
use crate::world::{Sample, Surface, World, SEA_LEVEL};
use core::f32::consts::TAU;

mod reservations;

pub const BEACH_TILE_SIZE: i32 = 128;
const PIECES: usize = 7;
const UMBRELLA: f32 = 2.25;
const CANOPY_SHIFT: f32 = 0.47;
const TOWEL_LONG: f32 = 1.55;
const TOWEL_WIDE: f32 = 0.70;
const MIN_DRY_SAND: f32 = SEA_LEVEL + 0.10;
const NEAR_TERRAIN_STEP: f32 = 4.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fabric {
    Coral,
    Turquoise,
    Yellow,
    Indigo,
    Cream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum BeachMaterial {
    Umbrella(Fabric),
    Towel(Fabric),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeachSample {
    pub material: BeachMaterial,
    /// The canopy rises above the sand while a towel follows the ground.
    pub elevation: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeachSite {
    bounds: [f32; 4],
    items: [Piece; PIECES],
    count: u8,
}

impl BeachSite {
    pub fn generate(world: &World, seed: u64, tile_x: i32, tile_z: i32) -> Option<Self> {
        const LIMIT: i32 = 123_000;
        if !(-LIMIT..=LIMIT).contains(&tile_x) || !(-LIMIT..=LIMIT).contains(&tile_z) {
            return None;
        }
        let tile_key = (u64::from(tile_x as u32) << 32) | u64::from(tile_z as u32);
        let key = mix(seed ^ tile_key ^ 0x62b2_b84f_bd31_73a7);
        if key & 15 >= 4 {
            return None;
        }
        let origin_x = (tile_x * BEACH_TILE_SIZE) as f32;
        let origin_z = (tile_z * BEACH_TILE_SIZE) as f32;
        let jitter_x = fraction(mix(key ^ 632)) * 7.0;
        let jitter_z = fraction(mix(key ^ 371)) * 7.0;
        const GRID: u32 = 7;
        let mut cell = ((key >> 31) as u32) % (GRID * GRID);
        for index in 0..GRID * GRID {
            let x = origin_x + 15.0 + jitter_x + 15.0 * (cell % GRID) as f32;
            let z = origin_z + 15.0 + jitter_z + 15.0 * (cell / GRID) as f32;
            // A coprime stride visits every candidate in the patch exactly once.
            cell = (cell + 19) % (GRID * GRID);
            let sand = world.sample(x, z);
            if !dry_sand(sand) {
                continue;
            }
            let design = mix(key ^ (u64::from(index) << 25));
            let Some(shore) = Shore::find(world, x, z) else {
                continue;
            };
            let mut site = Self {
                bounds: [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
                items: [Piece::EMPTY; PIECES],
                count: 0,
            };
            if site.arrange(world, shore, design)
                && site.bounds[0] >= origin_x && site.bounds[1] >= origin_z
                && site.bounds[2] < origin_x + BEACH_TILE_SIZE as f32
                && site.bounds[3] < origin_z + BEACH_TILE_SIZE as f32
            {
                return Some(site);
            }
        }
        None
    }

    pub fn sample(&self, x: f32, z: f32) -> Option<BeachSample> {
        let [x0, z0, x1, z1] = self.bounds;
        if !(x >= x0 && x <= x1 && z >= z0 && z <= z1) {
            return None;
        }
        self.items().iter().find_map(|piece| piece.sample(x, z))
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    /// Draw the thin fabric geometry after the terrain pass. The height
    /// callback provides terrain and other solid scenery for occlusion.
    pub fn draw(
        &self,
        camera: Camera,
        daylight: f32,
        pixels: &mut [u16],
        width: usize,
        height: usize,
        mut elevation_at: impl FnMut(f32, f32) -> f32,
    ) {
        let Some(length) = width.checked_mul(height) else {
            return;
        };
        if width == 0 || height == 0 || length > pixels.len() {
            return;
        }
        let [x0, z0, x1, z1] = self.bounds;
        const VISIBILITY: f32 = 76.0;
        if camera.x < x0 - VISIBILITY
            || camera.x > x1 + VISIBILITY
            || camera.z < z0 - VISIBILITY
            || camera.z > z1 + VISIBILITY
        {
            return;
        }
        let view = BeachView::new(camera, width, height);
        let mut canvas = Canvas {
            pixels,
            width,
            height,
        };
        let mut umbrellas = [(0.0, Piece::EMPTY); 3];
        let mut count = 0;
        for &piece in self.items() {
            let depth = view.depth(piece.x, piece.z);
            if !(5.0..VISIBILITY).contains(&depth) {
                continue;
            }
            let mut piece = piece;
            let height = elevation_at(piece.x, piece.z);
            if !height.is_finite() || (height - piece.ground).abs() > 0.65 {
                continue;
            }
            piece.ground = height;
            if piece.umbrella {
                let mut i = count;
                while i > 0 && umbrellas[i - 1].0 < depth {
                    umbrellas[i] = umbrellas[i - 1];
                    i -= 1;
                }
                umbrellas[i] = (depth, piece);
                count += 1;
            } else if depth < 51.0 {
                paint_towel(
                    piece,
                    &view,
                    &mut canvas,
                    daylight,
                    depth,
                    &mut elevation_at,
                );
            }
        }
        for &(depth, piece) in &umbrellas[..count] {
            paint_umbrella(
                piece,
                &view,
                &mut canvas,
                daylight,
                depth,
                &mut elevation_at,
            );
        }
    }

    /// Check fixed footprints at load time against existing scenery overlays.
    /// The callback should report any occupied road, parcel or anchorage point.
    pub fn conflicts_with(&self, mut occupied: impl FnMut(f32, f32) -> bool) -> bool {
        for piece in self.items() {
            let radius = piece.extent();
            for dz in [-1.0, -0.5, 0.0, 0.5, 1.0] {
                for dx in [-1.0, -0.5, 0.0, 0.5, 1.0] {
                    if occupied(piece.x + radius * dx, piece.z + radius * dz) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check the actual terrain used by the renderer, including its LOD and overlays.
    pub fn fits_displayed_sand(&self, mut terrain: impl FnMut(f32, f32) -> Sample) -> bool {
        !self.conflicts_with(|x, z| !dry_sand(terrain(x, z)))
    }

    pub fn umbrella_count(&self) -> u8 {
        self.items().iter().filter(|piece| piece.umbrella).count() as u8
    }

    pub fn towel_count(&self) -> u8 {
        self.count - self.umbrella_count()
    }

    fn items(&self) -> &[Piece] {
        &self.items[..self.count as usize]
    }

    fn arrange(&mut self, world: &World, shore: Shore, key: u64) -> bool {
        let inset = 9.0 + 1.35 * fraction(mix(key ^ 0x4edf));
        if !self.add(world, &shore, inset, 0.0, true, key) {
            return false;
        }
        let side = if key & 16 == 0 { 1.0 } else { -1.0 };
        let fabric = mix(key ^ 467);
        if !self.add(world, &shore, inset - 1.05, 4.1 * side, false, fabric)
            && !self.add(world, &shore, inset - 0.55, -4.1 * side, false, fabric)
            && !self.add(world, &shore, inset + 4.0, 0.0, false, fabric)
        {
            return false;
        }
        if !self.add(world, &shore, inset - 0.3, -7.7 * side, true, mix(key ^ 71)) {
            self.add(world, &shore, inset + 0.9, -7.0 * side, true, mix(key ^ 939));
        }
        if key & 3 != 3 {
            self.add(world, &shore, inset + 0.4, 8.7 * side, true, mix(key ^ 212));
        }
        if key & 32 != 0 {
            self.add(world, &shore, inset + 2.1, -4.8 * side, false, mix(key ^ 843));
        }
        if key & 64 != 0 {
            self.add(world, &shore, inset + 0.7, -11.2 * side, false, mix(key ^ 375));
        }
        if key & 128 != 0 {
            self.add(world, &shore, inset - 1.0, 12.3 * side, false, mix(key ^ 867));
        }
        true
    }

    fn add(
        &mut self,
        world: &World,
        shore: &Shore,
        setback: f32,
        across: f32,
        umbrella: bool,
        key: u64,
    ) -> bool {
        if self.count as usize == PIECES {
            return false;
        }
        let Some(local) = shore.along(world, across) else { return false; };
        let rotation = fraction(mix(key));
        let (sin, cos) = if umbrella {
            sin_cos(rotation * TAU)
        } else {
            let (s, c) = sin_cos((rotation - 0.5) * 0.8);
            (local.fx * c - local.fz * s, -local.fz * c - local.fx * s)
        };
        for offset in [0.0, 1.55, -0.85] {
            let inset = (setback + offset).clamp(6.1, 12.5);
            if self.place(world, &local, inset, umbrella, (sin, cos), key) {
                return true;
            }
        }
        false
    }

    fn place(
        &mut self, world: &World, shore: &Shore, setback: f32, umbrella: bool,
        (sin, cos): (f32, f32), key: u64,
    ) -> bool {
        let x = shore.x - shore.fx * setback;
        let z = shore.z - shore.fz * setback;
        let mut piece = Piece {
            x,
            z,
            sin,
            cos,
            umbrella,
            ground: SEA_LEVEL,
            fabric: Fabric::from_key(key),
            trim: key & 128 == 0,
        };
        let radius = piece.extent();
        for other in self.items() {
            let dx = x - other.x;
            let dz = z - other.z;
            let gap = radius + other.extent() - 0.4;
            if dx * dx + dz * dz < gap * gap {
                return false;
            }
        }
        let Some(ground) = sand_pad(world, x, z, radius) else {
            return false;
        };
        if self.count == 0 && !shore.walkable(world, x, z, ground) {
            return false;
        }
        piece.ground = ground;
        self.bounds[0] = self.bounds[0].min(x - radius);
        self.bounds[1] = self.bounds[1].min(z - radius);
        self.bounds[2] = self.bounds[2].max(x + radius);
        self.bounds[3] = self.bounds[3].max(z + radius);
        self.items[self.count as usize] = piece;
        self.count += 1;
        true
    }
}

impl Fabric {
    pub const fn rgb(self) -> [u8; 3] {
        match self {
            Self::Coral => [216, 81, 63],
            Self::Turquoise => [44, 154, 158],
            Self::Yellow => [230, 171, 47],
            Self::Indigo => [68, 105, 170],
            Self::Cream => [239, 222, 182],
        }
    }

    fn from_key(key: u64) -> Self {
        [Self::Coral, Self::Turquoise, Self::Yellow, Self::Indigo][(key >> 15 & 3) as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Piece {
    x: f32,
    z: f32,
    sin: f32,
    cos: f32,
    ground: f32,
    fabric: Fabric,
    umbrella: bool,
    trim: bool,
}

impl Piece {
    const EMPTY: Self = Self {
        x: 0.0,
        z: 0.0,
        sin: 0.0,
        cos: 1.0,
        ground: 0.0,
        fabric: Fabric::Cream,
        umbrella: false,
        trim: false,
    };

    fn extent(self) -> f32 {
        if self.umbrella {
            UMBRELLA + CANOPY_SHIFT
        } else {
            1.75
        }
    }

    fn sample(self, x: f32, z: f32) -> Option<BeachSample> {
        let dx = x - self.x;
        let dz = z - self.z;
        let extent = self.extent();
        if dx.abs() > extent || dz.abs() > extent {
            return None;
        }
        let along = dx * self.cos + dz * self.sin;
        let across = dz * self.cos - dx * self.sin;
        if self.umbrella {
            let u = along - CANOPY_SHIFT;
            let v = across;
            let rim = v
                .abs()
                .max((0.8660254 * u + 0.5 * v).abs())
                .max((0.8660254 * u - 0.5 * v).abs());
            const APOTHEM: f32 = UMBRELLA * 0.8660254;
            if rim > APOTHEM {
                return None;
            }
            let depth = 1.0 - rim / APOTHEM;
            let wedge = u * (0.8660254 * v + 0.5 * u) * (0.8660254 * v - 0.5 * u);
            let fabric = if self.trim == (wedge >= 0.0) {
                Fabric::Cream
            } else {
                self.fabric
            };
            return Some(BeachSample {
                material: BeachMaterial::Umbrella(fabric),
                elevation: Some(self.ground + 2.45 + 0.60 * depth - u * 0.19),
            });
        }
        if along.abs() > TOWEL_LONG || across.abs() > TOWEL_WIDE {
            return None;
        }
        let fabric = if along.abs() >= TOWEL_LONG * 0.72 {
            Fabric::Cream
        } else {
            self.fabric
        };
        Some(BeachSample {
            material: BeachMaterial::Towel(fabric),
            elevation: None,
        })
    }
}

struct Shore {
    x: f32,
    z: f32,
    fx: f32,
    fz: f32,
}

impl Shore {
    fn find(world: &World, x: f32, z: f32) -> Option<Self> {
        let forward = seaward(world, x, z)?;
        let mut distance = 0.0;
        let mut land = world.coast_signal(x, z);
        for next in [4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 28.0, 32.0, 36.0] {
            let coast = world.coast_signal(x + forward[0] * next, z + forward[1] * next);
            if coast > land + 0.004 {
                return None;
            }
            if coast <= 0.0 {
                let shore = Self::edge(world, [x, z], forward, distance, next)?;
                return shore.open_water(world).then_some(shore);
            }
            distance = next;
            land = coast;
        }
        None
    }

    fn edge(
        world: &World, [x, z]: [f32; 2], [fx, fz]: [f32; 2], mut dry: f32, mut wet: f32,
    ) -> Option<Self> {
        let coast = |step| world.coast_signal(x + fx * step, z + fz * step);
        if coast(dry) <= 0.0 || coast(wet) > 0.0 {
            return None;
        }
        for _ in 0..8 {
            let center = (dry + wet) * 0.5;
            if coast(center) > 0.0 {
                dry = center;
            } else {
                wet = center;
            }
        }
        Some(Self { x: x + fx * wet, z: z + fz * wet, fx, fz })
    }

    // Follow the local shore at each prop instead of opening a straight
    // landward row across a cove or a curved sandbar.
    fn along(&self, world: &World, across: f32) -> Option<Self> {
        let x = self.x - self.fz * across;
        let z = self.z + self.fx * across;
        let forward = seaward(world, x, z)?;
        if forward[0] * self.fx + forward[1] * self.fz < 0.74 {
            return None;
        }
        Self::edge(world, [x, z], forward, -7.0, 7.0)
    }

    fn walkable(&self, world: &World, x: f32, z: f32, height: f32) -> bool {
        let mut previous = height;
        for distance in [4.0, 9.0, 15.0, 22.0] {
            let ground = world.elevation(x - distance * self.fx, z - distance * self.fz);
            if ground < height - 0.24 || (ground - previous).abs() > 1.05 {
                return false;
            }
            previous = ground;
        }
        true
    }

    fn open_water(&self, world: &World) -> bool {
        let coast = |along: f32, cross: f32| {
            world.coast_signal(self.x + self.fx * along - self.fz * cross,
                self.z + self.fz * along + self.fx * cross)
        };
        coast(4.0, 0.0) <= 0.0 && coast(12.0, 0.0) < -0.010
            && coast(19.0, -9.0) < -0.015 && coast(19.0, 9.0) < -0.015
    }
}

fn seaward(world: &World, x: f32, z: f32) -> Option<[f32; 2]> {
    let dx = world.coast_signal(x - 4.0, z) - world.coast_signal(x + 4.0, z);
    let dz = world.coast_signal(x, z - 4.0) - world.coast_signal(x, z + 4.0);
    let squared = dx * dx + dz * dz;
    if squared < 0.000_001 {
        return None;
    }
    let root = f32::from_bits((squared.to_bits() >> 1) + 0x1fc0_0000);
    let root = (root + squared / root) * 0.5;
    let size = (root + squared / root) * 0.5;
    Some([dx / size, dz / size])
}

fn dry_sand(sample: Sample) -> bool {
    sample.surface == Surface::Sand && sample.elevation >= MIN_DRY_SAND
}

fn sand_pad(world: &World, x: f32, z: f32, radius: f32) -> Option<f32> {
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for row in -3..=3 {
        for col in -3..=3 {
            let ground = world.sample(x + col as f32 * (radius / 3.0), z + row as f32 * (radius / 3.0));
            if !dry_sand(ground) {
                return None;
            }
            low = low.min(ground.elevation);
            high = high.max(ground.elevation);
        }
    }
    if high - low > 0.70 {
        return None;
    }
    // Every near-cache cell under this conservative footprint uses only dry
    // sand vertices. A nearby grass vertex cannot repaint the prop's pad.
    let grid = |axis: f32| {
        let scaled = axis / NEAR_TERRAIN_STEP;
        let cell = scaled as i32;
        cell - i32::from(scaled < cell as f32)
    };
    for row in grid(z - radius)..=grid(z + radius) + 1 {
        for col in grid(x - radius)..=grid(x + radius) + 1 {
            let ground = world.sample(col as f32 * NEAR_TERRAIN_STEP, row as f32 * NEAR_TERRAIN_STEP);
            if !dry_sand(ground) || ground.elevation < MIN_DRY_SAND + 0.02 {
                return None;
            }
        }
    }
    Some(world.elevation(x, z))
}

#[derive(Clone, Copy)]
struct ClothPoint {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Copy, Default)]
struct ScreenPoint {
    x: f32,
    y: f32,
}

struct BeachView {
    camera: Camera,
    heading: (f32, f32),
    pitch: (f32, f32),
    roll: (f32, f32),
    focal: f32,
    cx: f32,
    cy: f32,
}

impl BeachView {
    fn new(camera: Camera, width: usize, height: usize) -> Self {
        Self {
            camera,
            heading: sin_cos(camera.heading),
            pitch: sin_cos(camera.pitch.clamp(-1.3, 1.3)),
            roll: sin_cos(-camera.bank),
            focal: width.min(height) as f32 * 0.73,
            cx: (width - 1) as f32 * 0.5,
            cy: (height - 1) as f32 * 0.5,
        }
    }

    fn depth(&self, x: f32, z: f32) -> f32 {
        (x - self.camera.x) * self.heading.0 + (z - self.camera.z) * self.heading.1
    }

    fn project(&self, point: ClothPoint) -> Option<ScreenPoint> {
        let forward = self.depth(point.x, point.z);
        let relief = point.y - self.camera.altitude;
        let depth = forward * self.pitch.1 - relief * self.pitch.0;
        if depth < 0.5 || !depth.is_finite() {
            return None;
        }
        let across =
            (point.x - self.camera.x) * self.heading.1 - (point.z - self.camera.z) * self.heading.0;
        let u = across * self.focal / depth;
        let v = -(forward * self.pitch.0 + relief * self.pitch.1) * self.focal / depth;
        Some(ScreenPoint {
            x: self.cx + u * self.roll.1 - v * self.roll.0,
            y: self.cy + u * self.roll.0 + v * self.roll.1,
        })
    }

    fn visible(&self, point: ClothPoint, elevation_at: &mut impl FnMut(f32, f32) -> f32) -> bool {
        let dx = point.x - self.camera.x;
        let dz = point.z - self.camera.z;
        let count = ((dx.abs().max(dz.abs()) / 3.0) as usize).clamp(3, 28);
        for i in 1..count {
            let t = i as f32 / count as f32;
            let sightline = self.camera.altitude + (point.y - self.camera.altitude) * t;
            if elevation_at(self.camera.x + dx * t, self.camera.z + dz * t) > sightline + 0.045 {
                return false;
            }
        }
        true
    }
}

impl Piece {
    fn cloth(self, along: f32, across: f32, height: f32) -> ClothPoint {
        ClothPoint {
            x: self.x + self.cos * along - self.sin * across,
            y: height,
            z: self.z + self.sin * along + self.cos * across,
        }
    }
}

fn paint_umbrella(
    piece: Piece,
    view: &BeachView,
    canvas: &mut Canvas<'_>,
    daylight: f32,
    depth: f32,
    elevation_at: &mut impl FnMut(f32, f32) -> f32,
) {
    const HEX: [(f32, f32); 6] = [
        (1.0, 0.0),
        (0.5, 0.8660254),
        (-0.5, 0.8660254),
        (-1.0, 0.0),
        (-0.5, -0.8660254),
        (0.5, -0.8660254),
    ];
    let apex = piece.cloth(CANOPY_SHIFT, 0.0, piece.ground + 3.05);
    let Some(tip) = view.project(apex) else {
        return;
    };
    if !canvas.near_image(tip, 240.0 / depth + 2.0) || !view.visible(apex, elevation_at) {
        return;
    }
    let fade = ((76.0 - depth) / 17.0).clamp(0.0, 1.0);
    let ground = piece.cloth(0.0, 0.0, piece.ground + 0.045);
    if view.visible(ground, elevation_at) {
        if let Some(base) = view.project(ground) {
            let tint = toned([127, 102, 76], daylight, 0.87);
            canvas.line(base, tip, tint, fade);
        }
    }
    let mut rim = [ScreenPoint::default(); 6];
    let mut vertices = [apex; 6];
    let (mut left_edge, mut right_edge) = (0, 0);
    for (i, &(x, z)) in HEX.iter().enumerate() {
        let edge = piece.cloth(
            CANOPY_SHIFT + x * UMBRELLA,
            z * UMBRELLA,
            piece.ground + 2.45 - x * UMBRELLA * 0.19,
        );
        let Some(point) = view.project(edge) else {
            return;
        };
        rim[i] = point;
        vertices[i] = edge;
        if point.x < rim[left_edge].x {
            left_edge = i;
        }
        if point.x > rim[right_edge].x {
            right_edge = i;
        }
    }
    let visible_left = view.visible(vertices[left_edge], elevation_at);
    let visible_right = view.visible(vertices[right_edge], elevation_at);
    for i in 0..6 {
        let a = rim[i];
        let b = rim[(i + 1) % 6];
        if (visible_left || a.x.min(b.x) >= tip.x) && (visible_right || a.x.max(b.x) <= tip.x) {
            let fabric = if piece.trim == (i & 1 == 0) {
                Fabric::Cream
            } else {
                piece.fabric
            };
            let tint = toned(fabric.rgb(), daylight, if i < 3 { 1.0 } else { 0.84 });
            canvas.triangle(tip, a, b, tint, fade);
        }
    }
}

fn paint_towel(
    piece: Piece,
    view: &BeachView,
    canvas: &mut Canvas<'_>,
    daylight: f32,
    depth: f32,
    elevation_at: &mut impl FnMut(f32, f32) -> f32,
) {
    let center = piece.cloth(0.0, 0.0, piece.ground + 0.055);
    let Some(mid) = view.project(center) else {
        return;
    };
    if !canvas.near_image(mid, 190.0 / depth + 2.0) || !view.visible(center, elevation_at) {
        return;
    }
    let fade = ((51.0 - depth) / 9.0).clamp(0.0, 1.0);
    let hems = TOWEL_LONG * 0.72;
    let seams = [-TOWEL_LONG, -hems, hems, TOWEL_LONG];
    let mut edge = [[ScreenPoint::default(); 2]; 4];
    for (i, along) in seams.into_iter().enumerate() {
        for (j, across) in [-TOWEL_WIDE, TOWEL_WIDE].into_iter().enumerate() {
            let mut point = piece.cloth(along, across, piece.ground);
            point.y = elevation_at(point.x, point.z) + 0.055;
            let Some(projected) = view.project(point) else {
                return;
            };
            edge[i][j] = projected;
        }
    }
    for i in 0..3 {
        let fabric = if i == 1 { piece.fabric } else { Fabric::Cream };
        let tint = toned(fabric.rgb(), daylight, 0.92);
        let [a, b] = edge[i];
        let [d, c] = edge[i + 1];
        canvas.triangle(a, b, c, tint, fade);
        canvas.triangle(a, c, d, tint, fade);
    }
}

fn toned([r, g, b]: [u8; 3], daylight: f32, shade: f32) -> u16 {
    let light = (0.23 + 0.77 * daylight.clamp(0.0, 1.0)) * shade;
    let channel = |c: u8, range: f32| (c as f32 * light * range / 255.0 + 0.5) as u16;
    (channel(r, 31.0) << 11) | (channel(g, 63.0) << 5) | channel(b, 31.0)
}

struct Canvas<'a> {
    pixels: &'a mut [u16],
    width: usize,
    height: usize,
}

impl Canvas<'_> {
    fn near_image(&self, p: ScreenPoint, margin: f32) -> bool {
        p.x >= -margin
            && p.x <= self.width as f32 + margin
            && p.y >= -margin
            && p.y <= self.height as f32 + margin
    }

    fn pixel(&mut self, x: i32, y: i32, color: u16, opacity: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let dx = 2 * x + 1 - self.width as i32;
        let dy = 2 * y + 1 - self.height as i32;
        let radius = self.width.min(self.height) as i32;
        if dx * dx + dy * dy > radius * radius {
            return;
        }
        let pixel = &mut self.pixels[y as usize * self.width + x as usize];
        if opacity >= 1.0 {
            *pixel = color;
            return;
        }
        let channels = |shift: u32, mask: u16| {
            let a = (*pixel >> shift & mask) as f32;
            let b = (color >> shift & mask) as f32;
            ((a + (b - a) * opacity + 0.5) as u16) << shift
        };
        *pixel = channels(11, 31) | channels(5, 63) | channels(0, 31);
    }

    fn line(&mut self, a: ScreenPoint, b: ScreenPoint, color: u16, opacity: f32) {
        let n = ((b.x - a.x).abs().max((b.y - a.y).abs()) as usize + 1).min(100);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let x = a.x + (b.x - a.x) * t;
            let y = a.y + (b.y - a.y) * t;
            self.pixel((x + 0.5) as i32, (y + 0.5) as i32, color, opacity);
        }
    }

    fn triangle(
        &mut self,
        a: ScreenPoint,
        b: ScreenPoint,
        c: ScreenPoint,
        color: u16,
        opacity: f32,
    ) {
        let edge = |p: ScreenPoint, q: ScreenPoint, x: f32, y: f32| {
            (q.x - p.x) * (y - p.y) - (q.y - p.y) * (x - p.x)
        };
        let area = edge(a, b, c.x, c.y);
        if area.abs() < 0.03 {
            return;
        }
        let low_x = (a.x.min(b.x.min(c.x)) as i32).max(0);
        let low_y = (a.y.min(b.y.min(c.y)) as i32).max(0);
        let high_x = (a.x.max(b.x.max(c.x)) as i32 + 1).min(self.width as i32 - 1);
        let high_y = (a.y.max(b.y.max(c.y)) as i32 + 1).min(self.height as i32 - 1);
        let sign = if area < 0.0 { -1.0 } else { 1.0 };
        for y in low_y..=high_y {
            for x in low_x..=high_x {
                let (px, py) = (x as f32, y as f32);
                if edge(a, b, px, py) * sign >= 0.0
                    && edge(b, c, px, py) * sign >= 0.0
                    && edge(c, a, px, py) * sign >= 0.0
                {
                    self.pixel(x, y, color, opacity);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain_cache::TerrainCache;

    const SITES: [(i32, i32); 3] = [(-3, -15), (-2, -15), (-4, -14)];

    #[test]
    fn groups_repeat_and_stay_sparse_across_seeded_tiles() {
        let mut totals = [0; 3];
        for (i, seed) in [1, 7, 31].into_iter().enumerate() {
            let world = World::new(seed);
            for z in -12..=12 {
                for x in -12..=12 {
                    let site = BeachSite::generate(&world, seed, x, z);
                    if let Some(site) = site {
                        totals[i] += 1;
                        assert_eq!(Some(site), BeachSite::generate(&world, seed, x, z));
                        assert!((1..=3).contains(&site.umbrella_count()));
                        assert!((1..=4).contains(&site.towel_count()));
                        let [x0, z0, x1, z1] = site.bounds;
                        assert!(x0 >= (x * BEACH_TILE_SIZE) as f32);
                        assert!(z0 >= (z * BEACH_TILE_SIZE) as f32);
                        assert!(x1 < ((x + 1) * BEACH_TILE_SIZE) as f32);
                        assert!(z1 < ((z + 1) * BEACH_TILE_SIZE) as f32);
                    }
                }
            }
        }
        assert!(totals.into_iter().all(|count| (4..63).contains(&count)));
        let world = World::new(1);
        let (x, z) = SITES[0];
        assert_ne!(
            BeachSite::generate(&world, 1, x, z),
            BeachSite::generate(&world, 2, x, z)
        );
    }

    #[test]
    fn canopies_and_towels_fit_dry_sand_above_a_swimming_entry() {
        let world = World::new(1);
        for (tx, tz) in SITES {
            let site = BeachSite::generate(&world, 1, tx, tz).unwrap();
            let first = site.items()[0];
            let shore = Shore::find(&world, first.x, first.z).unwrap();
            assert!(shore.walkable(&world, first.x, first.z, first.ground));
            assert!(shore.open_water(&world));
            let mut terrain = TerrainCache::new();
            terrain.reset(world, first.x, first.z);
            assert!(site.fits_displayed_sand(|x, z| terrain.sample(x, z)));
            let mut colored = 0;
            let mut canvas = 0;
            let mut towels = 0;
            for piece in site.items() {
                assert!((0..36).any(|index| {
                    let (sin, cos) = sin_cos(TAU * index as f32 / 36.0);
                    world.elevation(piece.x + cos * 13.1, piece.z + sin * 13.1) == SEA_LEVEL
                }));
                let mut minimum = f32::INFINITY;
                let mut maximum = f32::NEG_INFINITY;
                for j in -15..=15 {
                    for i in -15..=15 {
                        let x = piece.x + i as f32 * 0.19;
                        let z = piece.z + j as f32 * 0.19;
                        let Some(sample) = piece.sample(x, z) else {
                            continue;
                        };
                        let natural = world.sample(x, z);
                        assert_eq!(natural.surface, Surface::Sand);
                        assert!(dry_sand(natural));
                        assert!(dry_sand(terrain.sample(x, z)));
                        minimum = minimum.min(natural.elevation);
                        maximum = maximum.max(natural.elevation);
                        match sample.material {
                            BeachMaterial::Umbrella(Fabric::Cream) => canvas += 1,
                            BeachMaterial::Umbrella(_) => colored += 1,
                            BeachMaterial::Towel(_) => {
                                towels += 1;
                                assert!(sample.elevation.is_none());
                            }
                        }
                        if let Some(elevation) = sample.elevation {
                            assert!(elevation - natural.elevation > 1.8);
                        }
                    }
                }
                assert!(maximum - minimum < 0.70);
            }
            assert!(colored > 50 && canvas > 50 && towels > 50);
        }
    }

    #[test]
    fn overlay_occupancy_can_exclude_existing_scenery() {
        let world = World::new(1);
        let (x, z) = SITES[0];
        let site = BeachSite::generate(&world, 1, x, z).unwrap();
        let first = site.items()[0];
        assert!(site.conflicts_with(|x, z| (x - first.x).abs() < 2.7 && (z - first.z).abs() < 2.7));
        assert!(!site.conflicts_with(|x, _| x > first.x + 30.0));
        assert!(!site.conflicts_with(|x, z| {
            matches!(
                world.sample(x, z).surface,
                Surface::DeepWater | Surface::Shallows
            )
        }));
        assert!(!site.fits_displayed_sand(|x, z| {
            let sample = world.sample(x, z);
            if (x - first.x).abs() < 1.0 && (z - first.z).abs() < 1.0 {
                Sample { surface: Surface::Grass, ..sample }
            } else {
                sample
            }
        }));
    }

    #[test]
    fn native_canvas_draws_fabric_and_respects_occlusion() {
        let world = World::new(1);
        let (x, z) = SITES[0];
        let site = BeachSite::generate(&world, 1, x, z).unwrap();
        let [left, near, right, far] = site.bounds();
        let camera = Camera {
            x: (left + right) * 0.5,
            altitude: 15.0,
            z: (near + far) * 0.5 - 29.0,
            heading: 0.0,
            pitch: 0.465,
            bank: 0.0,
        };
        let mut frame = [0x9be4; 128 * 128];
        for side in [112, 128] {
            frame.fill(0x9be4);
            site.draw(camera, 1.0, &mut frame, side, side, |x, z| {
                world.elevation(x, z)
            });
            assert!(frame.iter().filter(|&&pixel| pixel != 0x9be4).count() >= 150);
            assert_eq!(frame[0], 0x9be4);
            assert_eq!(frame[side * side - 1], 0x9be4);
            assert!(frame[side * side..].iter().all(|&pixel| pixel == 0x9be4));
            frame.fill(0x9be4);
            site.draw(camera, 1.0, &mut frame, side, side, |_, _| 100.0);
            assert!(frame.iter().all(|&pixel| pixel == 0x9be4));
        }
    }
}
