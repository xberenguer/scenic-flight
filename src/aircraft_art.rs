//! Original low-polygon aircraft geometry and a small painter for the chase view.

use crate::air::Lift;
use crate::encounters::{Refuel, blackbird_refuel, blackbird_reheat, fighter_reheat, formation};
use crate::flight::{Aircraft, Flight};
use crate::math::{sin_cos, smoothstep};
use crate::sky::Sky;
use core::f32::consts::TAU;

type Point = [f32; 3];
type Paint = [u8; 3];

const FACE_CAPACITY: usize = 144;
const GLASS: Paint = [57, 99, 120];
const RUBBER: Paint = [40, 46, 49];
const PALE: Paint = [220, 226, 220];
const BLACKBIRD_ENGINE_X: f32 = 2.10;
const BLACKBIRD_NOZZLE: [f32; 4] = [-3.52, 0.34, 0.10, -0.23];
const BUSH_WINGTIP: [Point; 2] = [[3.75, 0.86, 0.54], [3.75, 0.86, -0.72]];
const RACER_WINGTIP: [Point; 2] = [[3.02, 0.06, 0.27], [3.13, 0.03, -0.60]];
const SPORT_JET_WINGTIP: [Point; 2] = [[3.71, 0.19, -0.65], [3.72, 0.18, -1.45]];
const FIGHTER_WINGTIP: [Point; 2] = [[3.58, 0.07, -0.45], [3.49, 0.05, -1.57]];
const BLACKBIRD_WINGTIP: [Point; 2] = [[4.14, 0.06, -1.47], [4.39, 0.01, -2.43]];
const WINGMATE_WINGTIP: [Point; 2] = [[3.52, 0.10, -0.51], [3.39, 0.08, -1.61]];
const TANKER_WINGTIP: [Point; 2] = [[8.6, 0.38, -2.35], [8.7, 0.35, -4.58]];

/// A writable surface with the composed scenery already behind the aircraft.
pub trait AircraftCanvas {
    fn supports_opaque_spans(&self) -> bool {
        false
    }

    fn pixel(&mut self, x: usize, y: usize) -> &mut u16;

    fn opaque_span(&mut self, first: usize, end: usize, y: usize, color: u16) {
        for x in first..end {
            *self.pixel(x, y) = color;
        }
    }
}

struct SliceCanvas<'a> {
    pixels: &'a mut [u16],
    width: usize,
}

impl AircraftCanvas for SliceCanvas<'_> {
    fn pixel(&mut self, x: usize, y: usize) -> &mut u16 {
        &mut self.pixels[y * self.width + x]
    }
}

/// Paint the aircraft into the circular terrain viewport after rendering the world.
pub fn draw_aircraft(flight: &Flight, pixels: &mut [u16], w: usize, h: usize) {
    if !usable(pixels, w, h) {
        return;
    }
    let mut canvas = SliceCanvas { pixels, width: w };
    paint_chase_aircraft(flight, chase_projection(flight, w, h), 1.0, &mut canvas, w, h, None);
}

/// Draw the player and any scheduled wingmates into the chase view.
pub fn draw_flight_scene(
    flight: &Flight,
    seed: u64,
    sky: &Sky,
    pixels: &mut [u16],
    w: usize,
    h: usize,
) {
    if !usable(pixels, w, h) {
        return;
    }
    draw_flight_scene_on(flight, seed, sky, &mut SliceCanvas { pixels, width: w }, w, h);
}

/// Draw the same chase scene on a canvas representing the complete viewport.
pub fn draw_flight_scene_on(
    flight: &Flight,
    seed: u64,
    sky: &Sky,
    canvas: &mut impl AircraftCanvas,
    w: usize,
    h: usize,
) {
    if w == 0 || h == 0 || w > 512 || h > 512 {
        return;
    }
    let player = chase_projection(flight, w, h);
    if flight.aircraft == Aircraft::Blackbird {
        if let Some(event) = blackbird_refuel(seed, flight.ticks) {
            draw_tanker(player, event, sky.daylight, canvas, w, h);
        }
    }
    if let Some(event) = formation(flight.aircraft, seed, flight.ticks) {
        let mut wingmates = [(player, 0, 0.0); 4];
        for (index, mate) in event.aircraft[..event.count].iter().enumerate() {
            let lateral = mate.right;
            let forward = mate.forward;
            let projection = Projection {
                origin: [
                    player.origin[0] + player.yaw.1 * lateral + player.yaw.0 * forward,
                    player.origin[1] + mate.above,
                    player.origin[2] + player.yaw.1 * forward - player.yaw.0 * lateral,
                ],
                yaw: sin_cos(flight.heading - flight.view.heading + mate.heading_offset),
                bank: sin_cos(flight.bank * 0.67 + mate.bank),
                ..player
            };
            let depth = projection.pitch.1 * projection.origin[2]
                - projection.pitch.0 * projection.origin[1];
            wingmates[index] = (projection, index, depth);
        }
        wingmates[..event.count].sort_unstable_by(|a, b| b.2.total_cmp(&a.2));
        let (sine, cosine) = sin_cos(flight.heading);
        let sun_right = sky.sun_direction[0] * cosine - sky.sun_direction[2] * sine;
        let warm = (1.0 - sky.sun_direction[1].abs() * 4.0).max(0.0) * sky.daylight;
        for (projection, index, _) in wingmates[..event.count].iter().copied() {
            if !visible_wingmate(&projection, w, h) {
                continue;
            }
            let side = (projection.origin[0] - player.origin[0]).signum();
            let gain = (0.91 + 0.08 * side * sun_right) * (0.52 + sky.daylight * 0.48);
            paint_model(projection, canvas, w, h, |p| wingmate_mesh(p, index, gain, warm));
            paint_position_lights(projection, WINGMATE_WINGTIP, None, sky.daylight, canvas, (w, h));
        }
    }
    paint_chase_aircraft(flight, player, sky.daylight, canvas, w, h, Some(sky));
}

fn paint_chase_aircraft(
    flight: &Flight,
    projection: Projection,
    daylight: f32,
    canvas: &mut impl AircraftCanvas,
    w: usize,
    h: usize,
    sky: Option<&Sky>,
) {
    if let Some(reflection) = sky.and_then(|sky| glider_sun_reflection(flight, sky)) {
        paint_model(projection, canvas, w, h, |p| glider(p, Some(reflection)));
        return;
    }
    let plume = match flight.aircraft {
        Aircraft::Blackbird => Some((BLACKBIRD_EXHAUST, blackbird_reheat(flight.seed(), flight.ticks))),
        Aircraft::Fighter => Some((FIGHTER_EXHAUST, fighter_reheat(flight.seed(), flight.ticks))),
        _ => None,
    };
    if let Some((exhaust, reheat)) = plume {
        if reheat > 0.0 {
            draw_exhaust(exhaust, projection, daylight, reheat, canvas, w, h);
        }
    }
    paint_aircraft(flight.aircraft, flight.ticks as u32, projection, canvas, w, h);
    let (tip, tail) = match flight.aircraft {
        Aircraft::BushPlane => (BUSH_WINGTIP, Some([0.0, 0.14, -2.71])),
        Aircraft::Racer => (RACER_WINGTIP, Some([0.0, 0.18, -2.47])),
        Aircraft::SportJet => (SPORT_JET_WINGTIP, Some([0.0, 0.12, -3.19])),
        Aircraft::Fighter => (FIGHTER_WINGTIP, None),
        Aircraft::Blackbird => (BLACKBIRD_WINGTIP, Some([0.0, 0.06, -3.69])),
        Aircraft::Trike | Aircraft::Glider | Aircraft::BaldEagle => return,
    };
    paint_position_lights(projection, tip, tail, daylight, canvas, (w, h));
}

#[derive(Clone, Copy)]
struct GliderReflection {
    side: f32,
    strength: f32,
}

fn glider_sun_reflection(flight: &Flight, sky: &Sky) -> Option<GliderReflection> {
    if flight.aircraft != Aircraft::Glider || flight.air.lift != Lift::Thermal || flight.air.upward < 0.85 {
        return None;
    }
    let [sx, above, sz] = sky.sun_direction;
    let turn = f32::from(flight.thermal_turn_elapsed()?);
    let timing = smoothstep((turn - 45.0) / 24.0) * (1.0 - smoothstep((turn - 267.0) / 26.0));
    if timing == 0.0 || above < 0.28 || sky.daylight < 0.70 || flight.bank.abs() < 0.14 {
        return None;
    }
    let (sine, cosine) = sin_cos(flight.heading);
    let lateral = sx * cosine - sz * sine;
    let forward = sx * sine + sz * cosine;
    // A brief view of the sun across the banked, rear-facing quarter of the wing.
    // Narrowing this to one side prevents a second flash on the far half of the turn.
    if flight.bank * lateral <= 0.0 {
        return None;
    }
    let angle = smoothstep((0.24 - (forward + 0.39).abs()) / 0.19);
    let sky_clear = 1.0 - smoothstep((sky.cloud_cover - 0.44) * 3.1);
    let strength = timing * angle * sky_clear * smoothstep((flight.air.upward - 0.85) / 0.95);
    (strength > 0.02).then_some(GliderReflection { side: lateral.signum(), strength })
}

const PORT_LIGHT: u16 = rgb565(252, 56, 60);
const STARBOARD_LIGHT: u16 = rgb565(70, 248, 139);
const TAIL_LIGHT: u16 = rgb565(242, 238, 222);

fn wingtip_lamp(tip: [Point; 2]) -> Point {
    core::array::from_fn(|axis| (tip[0][axis] + tip[1][axis]) * 0.5)
}

fn paint_position_lights(
    projection: Projection,
    wingtip: [Point; 2],
    tail: Option<Point>,
    daylight: f32,
    canvas: &mut impl AircraftCanvas,
    size: (usize, usize),
) {
    if daylight >= 0.72 {
        return;
    }
    let brightness = smoothstep((0.72 - daylight) * 2.0);
    let [x, y, z] = wingtip_lamp(wingtip);
    for (anchor, color, strength) in [
        (Some([-x, y, z]), PORT_LIGHT, 0.95),
        (Some([x, y, z]), STARBOARD_LIGHT, 0.95),
        (tail, TAIL_LIGHT, 0.73),
    ] {
        let Some((point, depth)) = anchor.and_then(|point| projection.screen_point(point)) else {
            continue;
        };
        // A soft footprint preserves the lamp as the wing crosses pixel boundaries.
        // Use a smaller footprint for the farther formation and tanker lights.
        let radius = (projection.focal * 0.11 / depth).clamp(0.80, 1.65);
        paint_lamp(canvas, size, point, radius, color, brightness * strength);
    }
}

fn paint_lamp(
    canvas: &mut impl AircraftCanvas,
    (w, h): (usize, usize),
    [cx, cy]: [f32; 2],
    radius: f32,
    color: u16,
    brightness: f32,
) {
    let x0 = ((cx - radius) as i32).max(0);
    let y0 = ((cy - radius) as i32).max(0);
    let x1 = ((cx + radius) as i32).min(w as i32 - 1);
    let y1 = ((cy + radius) as i32).min(h as i32 - 1);
    let aperture = (w.min(h) * w.min(h)) as i32;
    let inverse_radius_squared = 1.0 / (radius * radius);
    for y in y0..=y1 {
        let dy = 2 * y + 1 - h as i32;
        for x in x0..=x1 {
            let dx = 2 * x + 1 - w as i32;
            if dx * dx + dy * dy > aperture {
                continue;
            }
            let (u, v) = (x as f32 - cx, y as f32 - cy);
            let coverage = 1.0 - (u * u + v * v) * inverse_radius_squared;
            let alpha = (coverage * brightness * 32.0 + 0.5) as u16;
            if alpha != 0 {
                glow_over(canvas.pixel(x as usize, y as usize), color, alpha);
            }
        }
    }
}

fn chase_projection(flight: &Flight, w: usize, h: usize) -> Projection {
    let view = flight.view;
    let (sine, cosine) = sin_cos(view.heading);
    let dx = flight.x - view.x;
    let dz = flight.z - view.z;
    let (pitch_sine, pitch_cosine) = sin_cos(view.pitch);
    let (roll_sine, roll_cosine) = sin_cos(-view.bank);
    let model_pitch = (flight.aircraft == Aircraft::BushPlane && flight.climb_rate() != 0.0)
        .then(|| {
            // Follow the Cub's vertical flight path, holding a modest visible
            // attitude when terrain avoidance calls for a steep maneuver.
            let inclination = flight.climb_rate() / flight.speed().max(8.0);
            sin_cos((inclination * 0.82).clamp(-0.18, 0.22))
        });
    let mut projection = Projection {
        origin: [
            cosine * dx - sine * dz,
            flight.altitude - view.altitude,
            sine * dx + cosine * dz,
        ],
        pitch: (pitch_sine, pitch_cosine),
        scene_roll: (roll_sine, roll_cosine),
        yaw: sin_cos(flight.heading - view.heading),
        bank: sin_cos(flight.bank),
        model_pitch,
        center: [(w - 1) as f32 * 0.5, (h - 1) as f32 * 0.5],
        focal: w.min(h) as f32 * 0.73,
    };
    // The chase camera frames terrain independently. Keep the player’s
    // fuselage at the viewport center without changing its 3D perspective.
    if let Some((offset, _)) = projection.screen_offset(projection.origin) {
        projection.center[0] -= offset[0];
        projection.center[1] -= offset[1];
    }
    projection
}

fn visible_wingmate(projection: &Projection, w: usize, h: usize) -> bool {
    let Some(center) = projection.vertex([0.0; 3]) else {
        return false;
    };
    if center.depth <= 4.0 {
        return true;
    }
    let dx = f32::from(center.x) * 0.25 - (w - 1) as f32 * 0.5;
    let dy = f32::from(center.y) * 0.25 - (h - 1) as f32 * 0.5;
    // Leave room for the nearest wingtip, including perspective at the rim.
    let margin = w.min(h) as f32 * 0.5 + projection.focal * 7.5 / (center.depth - 4.0);
    dx * dx + dy * dy <= margin * margin
}

/// Paint a rotating model centered in the supplied circular viewport.
pub fn draw_preview(aircraft: Aircraft, ticks: u32, pixels: &mut [u16], w: usize, h: usize) {
    if !usable(pixels, w, h) {
        return;
    }
    draw_preview_on(aircraft, ticks, &mut SliceCanvas { pixels, width: w }, w, h);
}

/// Paint the same rotating model on a canvas representing the complete viewport.
pub fn draw_preview_on(
    aircraft: Aircraft,
    ticks: u32,
    canvas: &mut impl AircraftCanvas,
    w: usize,
    h: usize,
) {
    if w == 0 || h == 0 || w > 512 || h > 512 {
        return;
    }
    let away = match aircraft {
        Aircraft::Glider | Aircraft::Blackbird => 14.8,
        Aircraft::Fighter | Aircraft::SportJet => 12.8,
        _ => 10.7,
    };
    let orbit = (ticks % 252) as f32 * (TAU / 252.0) + 0.48;
    let pitch = sin_cos(0.23);
    let diameter = w.min(h) as f32;
    let projection = Projection {
        origin: [0.0, -away * (pitch.0 / pitch.1), away],
        pitch,
        scene_roll: (0.0, 1.0),
        yaw: sin_cos(orbit),
        bank: sin_cos(0.10 * sin_cos(orbit * 2.0 + 0.62).0),
        model_pitch: None,
        center: [(w - 1) as f32 * 0.5, (h - 1) as f32 * 0.5 - diameter / 28.0],
        focal: diameter * if aircraft == Aircraft::BaldEagle { 0.66 } else { 0.89 },
    };
    paint_aircraft(aircraft, ticks, projection, canvas, w, h);
}

fn usable(pixels: &[u16], w: usize, h: usize) -> bool {
    w != 0 && h != 0 && w <= 512 && h <= 512 && pixels.len() >= w * h
}

#[derive(Clone, Copy)]
struct Projection {
    origin: Point,
    pitch: (f32, f32),
    scene_roll: (f32, f32),
    yaw: (f32, f32),
    bank: (f32, f32),
    model_pitch: Option<(f32, f32)>,
    center: [f32; 2],
    focal: f32,
}

#[derive(Clone, Copy)]
struct Vertex {
    x: i16,
    y: i16,
    depth: f32,
}

impl Projection {
    fn vertex(&self, point: Point) -> Option<Vertex> {
        let ([x, y], depth) = self.screen_point(point)?;
        Some(Vertex { x: rounded_pixel(x), y: rounded_pixel(y), depth })
    }

    fn camera_point(&self, [right, up, forward]: Point) -> Point {
        let (up, forward) = self.model_pitch.map_or((up, forward), |(sin, cos)| {
            (up * cos + forward * sin, forward * cos - up * sin)
        });
        let side = self.bank.1 * right + self.bank.0 * up;
        let height = self.origin[1] + self.bank.1 * up - self.bank.0 * right;
        let z = self.origin[2] + forward * self.yaw.1 - side * self.yaw.0;
        let x = self.origin[0] + side * self.yaw.1 + forward * self.yaw.0;
        [x, height, z]
    }

    fn screen_point(&self, point: Point) -> Option<([f32; 2], f32)> {
        let ([dx, dy], depth) = self.screen_offset(self.camera_point(point))?;
        Some(([self.center[0] + dx, self.center[1] + dy], depth))
    }

    fn screen_offset(&self, [x, height, z]: Point) -> Option<([f32; 2], f32)> {
        let depth = self.pitch.1 * z - self.pitch.0 * height;
        if depth < 0.8 || !depth.is_finite() {
            return None;
        }

        let scale = self.focal / depth;
        let u = x * scale;
        let v = -(self.pitch.1 * height + self.pitch.0 * z) * scale;
        Some((
            [u * self.scene_roll.1 - v * self.scene_roll.0,
             u * self.scene_roll.0 + v * self.scene_roll.1],
            depth,
        ))
    }
}

#[derive(Clone, Copy)]
struct Exhaust {
    engine_x: f32,
    engine_y: f32,
    nozzle: [f32; 4],
    trail: [f32; 2],
    palette: [u16; 2],
}

const BLACKBIRD_EXHAUST: Exhaust = Exhaust {
    engine_x: BLACKBIRD_ENGINE_X,
    engine_y: 0.0,
    nozzle: BLACKBIRD_NOZZLE,
    trail: [2.2, 4.5],
    palette: [rgb565(153, 88, 212), rgb565(255, 201, 127)],
};

const FIGHTER_ENGINE_X: f32 = 0.41;
const FIGHTER_ENGINE_Y: f32 = -0.12;
const FIGHTER_NOZZLE: [f32; 4] = [-2.98, 0.27, 0.12, -0.16];
const FIGHTER_EXHAUST: Exhaust = Exhaust {
    engine_x: FIGHTER_ENGINE_X,
    engine_y: FIGHTER_ENGINE_Y,
    nozzle: FIGHTER_NOZZLE,
    trail: [1.25, 2.75],
    palette: [rgb565(249, 98, 18), rgb565(255, 223, 126)],
};

fn draw_exhaust(
    exhaust: Exhaust,
    projection: Projection,
    daylight: f32,
    reheat: f32,
    canvas: &mut impl AircraftCanvas,
    w: usize,
    h: usize,
) {
    let [z, radius, top, bottom] = exhaust.nozzle;
    let y = exhaust.engine_y + (top + bottom) * 0.43;
    let strength = 0.73 + 0.19 * (1.0 - daylight.clamp(0.0, 1.0));
    for x in [-exhaust.engine_x, exhaust.engine_x] {
        // Start inside the nozzle. The solid aircraft pass masks the lip.
        let Some((start, depth)) = projection.screen_point([x, y, z + 0.06]) else {
            continue;
        };
        let Some((end, _)) = projection.screen_point([x, y, z - exhaust.trail[0] - exhaust.trail[1] * reheat]) else {
            continue;
        };
        let width = projection.focal * radius * 1.42 / depth;
        paint_exhaust_plume(canvas, w, h, start, end, width, strength, reheat, exhaust.palette);
    }
}

fn paint_exhaust_plume(
    canvas: &mut impl AircraftCanvas,
    w: usize,
    h: usize,
    start: [f32; 2],
    end: [f32; 2],
    width: f32,
    strength: f32,
    reheat: f32,
    [sheath_color, core_color]: [u16; 2],
) {
    let axis = [end[0] - start[0], end[1] - start[1]];
    let length_squared = axis[0] * axis[0] + axis[1] * axis[1];
    if length_squared < 0.01 {
        return;
    }
    let inverse_length = 1.0 / length_squared;
    let pad = 1.15 * width + 0.65;
    let x0 = ((start[0].min(end[0]) - pad) as i32).max(0);
    let x1 = ((start[0].max(end[0]) + pad) as i32 + 1).min(w as i32 - 1);
    let y0 = ((start[1].min(end[1]) - pad) as i32).max(0);
    let y1 = ((start[1].max(end[1]) + pad) as i32 + 1).min(h as i32 - 1);
    let aperture = (w.min(h) * w.min(h)) as i32;
    for y in y0..=y1 {
        let cy = 2 * y + 1 - h as i32;
        for x in x0..=x1 {
            let cx = 2 * x + 1 - w as i32;
            if cx * cx + cy * cy > aperture {
                continue;
            }
            let dx = x as f32 - start[0];
            let dy = y as f32 - start[1];
            let t = (dx * axis[0] + dy * axis[1]) * inverse_length;
            if !(0.0..=1.0).contains(&t) {
                continue;
            }
            let cross = dx * axis[1] - dy * axis[0];
            let distance_squared = cross * cross * inverse_length;
            let radius = width * (0.94 + t * (0.95 - 1.86 * t));
            let fade = ((1.0 - t) * 4.5).min(1.0);
            let soft_edge = radius + 0.50;
            let edge = (1.0 - distance_squared / (soft_edge * soft_edge)).clamp(0.0, 1.0);

            // Fixed pressure cells follow the plume, so brightness never pulses in time.
            let cycle = t * 3.0 + 0.04;
            let diamond = 1.0 - 2.0 * (cycle - cycle as u32 as f32 - 0.5).abs();
            let hot_width = radius * 0.72 * diamond + 0.24;
            let hot = (1.0 - distance_squared / (hot_width * hot_width)).clamp(0.0, 1.0);
            let sheath = (edge * fade * strength * reheat * 32.0 + 0.5) as u16;
            let hot_core = (hot * diamond * fade * (0.28 + 0.65 * strength) * reheat * 32.0 + 0.5) as u16;
            if sheath != 0 || hot_core != 0 {
                let pixel = canvas.pixel(x as usize, y as usize);
                glow_over(pixel, sheath_color, sheath);
                glow_over(pixel, core_color, hot_core);
            }
        }
    }
}

fn glow_over(pixel: &mut u16, color: u16, alpha: u16) {
    let mix = |shift: u32, mask: u16| {
        let background = (*pixel >> shift) & mask;
        let source = (color >> shift) & mask;
        (background * (32 - alpha) + source * alpha + 16) >> 5
    };
    *pixel = (mix(11, 31) << 11) | (mix(5, 63) << 5) | mix(0, 31);
}

fn rounded_pixel(value: f32) -> i16 {
    let scaled = value.clamp(-1024.0, 1024.0) * 4.0 + 0.5;
    scaled as i16 - i16::from(scaled < 0.0 && (scaled as i16) as f32 != scaled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;

    #[test]
    fn every_fuselage_stays_at_the_flight_view_center() {
        for seed in [439, 47, 9_805, 6_716, 25] {
            let world = World::new(seed);
            for aircraft in Aircraft::ALL {
                let mut flight = Flight::start(&world, seed, aircraft);
                for _ in 0..120 {
                    flight.update(&world);
                }
                for bank in [-0.55, -0.23, 0.0, 0.20, 0.44] {
                    flight.view.bank = bank;
                    let fuselage = chase_projection(&flight, 112, 112).vertex([0.0; 3]).unwrap();
                    assert_eq!([fuselage.x, fuselage.y], [222, 222], "{aircraft:?}");
                }
            }
        }
    }

    #[test]
    fn fighter_reheat_draws_warm_exhaust_behind_both_engines() {
        const SIDE: usize = 466;
        let seed = 11;
        let world = World::new(seed);
        let mut flight = Flight::start(&world, seed, Aircraft::Fighter);
        let peak = (0..720 * 30).find(|&tick| fighter_reheat(seed, tick) == 1.0)
            .expect("fixture includes a reheat event");
        for bank in [-0.4, 0.0, 0.4] {
            flight.bank = bank;
            flight.view.bank = bank * 0.24;
            flight.ticks = 0;
            let mut body = std::vec![0; SIDE * SIDE];
            draw_aircraft(&flight, &mut body, SIDE, SIDE);
            let mut lit = std::vec![0; SIDE * SIDE];
            flight.ticks = peak;
            draw_aircraft(&flight, &mut lit, SIDE, SIDE);
            let mut warm = [0; 2];
            for (index, (&pixel, &base)) in lit.iter().zip(&body).enumerate() {
                if pixel == base {
                    continue;
                }
                let red = pixel >> 11;
                let green = ((pixel >> 5) & 63) / 2;
                let blue = pixel & 31;
                assert!(red >= green && green >= blue);
                warm[usize::from(index % SIDE >= SIDE / 2)] += usize::from(red >= 16 && green >= 5);
            }
            assert!(warm.into_iter().all(|count| count >= 9), "warm cells {warm:?}; bank {bank}");
        }
    }

    #[test]
    fn blackbird_fins_stay_connected_at_the_waveshare_aircraft_resolution() {
        const SIDE: usize = 156;
        let world = World::new(0);
        let mut flight = Flight::start(&world, 0, Aircraft::Blackbird);
        for tick in 1..=975 {
            flight.update(&world);
            if !matches!(tick, 3 | 972) {
                continue;
            }
            let mut image = [0; SIDE * SIDE];
            draw_aircraft(&flight, &mut image, SIDE, SIDE);
            let middle = (SIDE / 2) * SIDE + SIDE / 2;
            assert_ne!(image[middle], 0);
            image[middle] = 0;
            let mut seen = std::vec![middle];
            while let Some(index) = seen.pop() {
                for offset in [-(SIDE as isize) - 1, -(SIDE as isize), -(SIDE as isize) + 1,
                    -1, 1, SIDE as isize - 1, SIDE as isize, SIDE as isize + 1]
                {
                    let neighbor = index.wrapping_add_signed(offset);
                    if let Some(pixel) = image.get_mut(neighbor) {
                        if *pixel != 0 {
                            *pixel = 0;
                            seen.push(neighbor);
                        }
                    }
                }
            }
            assert!(image.into_iter().all(|pixel| pixel == 0), "detached pixel at tick {tick}");
        }
    }

    #[test]
    fn steady_position_lights_follow_the_projected_wingtips_at_night() {
        const SIDE: usize = 156;
        let world = World::new(439);
        for (aircraft, tip) in [
            (Aircraft::BushPlane, BUSH_WINGTIP),
            (Aircraft::Racer, RACER_WINGTIP),
            (Aircraft::SportJet, SPORT_JET_WINGTIP),
            (Aircraft::Fighter, FIGHTER_WINGTIP),
            (Aircraft::Blackbird, BLACKBIRD_WINGTIP),
        ] {
            let mut flight = Flight::start(&world, 439, aircraft);
            let mut sky = Sky::from_time(439, 0, 0.0);
            for bank in [-0.42, 0.0, 0.39] {
                flight.ticks = 0;
                flight.bank = bank;
                flight.view.bank = bank * 0.28;
                let level = rgb565(46, 55, 68);
                let mut day = [level; SIDE * SIDE];
                let mut dusk = day;
                let mut night = day;
                let mut next = day;
                for (image, daylight) in [(&mut day, 1.0), (&mut dusk, 0.57), (&mut night, 0.0)] {
                    sky.daylight = daylight;
                    draw_flight_scene(&flight, 439, &sky, image, SIDE, SIDE);
                }
                flight.ticks = 13;
                draw_flight_scene(&flight, 439, &sky, &mut next, SIDE, SIDE);
                assert_eq!(night, next, "{aircraft:?}, bank {bank}: the lamps must not pulse");
                let pose = chase_projection(&flight, SIDE, SIDE);
                let [x, y, z] = wingtip_lamp(tip);
                for (hand, red) in [(-1.0, true), (1.0, false)] {
                    let ([sx, sy], _) = pose.screen_point([hand * x, y, z]).unwrap();
                    let mut dusk_delta = 0;
                    let mut night_delta = 0;
                    for iy in (sy as usize - 1)..=(sy as usize + 2) {
                        for ix in (sx as usize - 1)..=(sx as usize + 2) {
                            let i = iy * SIDE + ix;
                            if night[i] == day[i] {
                                continue;
                            }
                            let channels = |pixel: u16| [2 * (pixel >> 11), (pixel >> 5) & 63, 2 * (pixel & 31)];
                            let [r, g, b] = channels(night[i]);
                            if if red { r > g + 6 && r > b + 6 } else { g > r + 6 && g > b + 6 } {
                                let distance = |pixel| channels(pixel).into_iter().zip(channels(day[i]))
                                    .map(|(a, b)| a.abs_diff(b)).sum::<u16>();
                                dusk_delta += distance(dusk[i]);
                                night_delta += distance(night[i]);
                            }
                        }
                    }
                    assert!(night_delta > dusk_delta && dusk_delta > 0,
                        "{aircraft:?}, bank {bank}, hand {hand}: midnight {night_delta}, dusk {dusk_delta}");
                }
            }
        }
    }

    #[test]
    fn glider_eagle_and_open_trike_keep_their_unlit_airframes() {
        const SIDE: usize = 156;
        let world = World::new(439);
        let mut sky = Sky::from_time(439, 0, 0.0);
        for aircraft in [Aircraft::Glider, Aircraft::BaldEagle, Aircraft::Trike] {
            let flight = Flight::start(&world, 439, aircraft);
            let mut day = [0; SIDE * SIDE];
            let mut night = day;
            sky.daylight = 1.0;
            draw_flight_scene(&flight, 439, &sky, &mut day, SIDE, SIDE);
            sky.daylight = 0.0;
            draw_flight_scene(&flight, 439, &sky, &mut night, SIDE, SIDE);
            assert_eq!(day, night, "{aircraft:?}");
        }
    }

    #[test]
    fn glider_reflection_follows_real_lift_and_stays_within_the_drawn_airframe() {
        const SIDE: usize = 156;
        let mut best: Option<(f32, u64, Flight, Sky)> = None;
        for seed in [41, 33, 5, 6, 11, 15, 29, 45] {
            let world = World::new(seed);
            let mut flight = Flight::start(&world, seed, Aircraft::Glider);
            let mut visible_flash = false;
            let mut flashes = 0;
            for _ in 0..260 * crate::flight::TICKS_PER_SECOND {
                flight.update(&world);
                if flight.thermal_turn_elapsed().is_none() {
                    assert!(flashes <= 1, "repeat reflection in the same turn, seed {seed}");
                    visible_flash = false;
                    flashes = 0;
                    continue;
                }
                let sky = Sky::at_position(seed, flight.ticks, [flight.x, flight.altitude, flight.z]);
                let strength = glider_sun_reflection(&flight, &sky).map_or(0.0, |light| light.strength);
                if strength > 0.45 && !visible_flash {
                    flashes += 1;
                    visible_flash = true;
                }
                visible_flash &= strength >= 0.16;
                if strength > 0.38 && best.as_ref().is_none_or(|(score, ..)| strength > *score) {
                    best = Some((strength, seed, flight.clone(), sky));
                }
            }
        }
        let (strength, seed, flight, sky) = best.expect("natural sunny thermal reflection");
        let mut bare = [0; SIDE * SIDE];
        let mut bright = bare;
        draw_aircraft(&flight, &mut bare, SIDE, SIDE);
        draw_flight_scene(&flight, seed, &sky, &mut bright, SIDE, SIDE);
        let mut highlighted = 0;
        for (&a, &b) in bare.iter().zip(&bright) {
            if a != b {
                assert!(a != 0 && b != 0, "reflection escaped the aircraft silhouette");
                highlighted += 1;
            }
        }
        assert!(highlighted >= 6);
        let mut overcast = [0; SIDE * SIDE];
        let mut clouded = sky;
        clouded.cloud_cover = 1.0;
        draw_flight_scene(&flight, seed, &clouded, &mut overcast, SIDE, SIDE);
        assert_eq!(overcast, bare);
        std::eprintln!("glider reflection seed {seed}, tick {}, strength {strength:.3}, {highlighted} overlay pixels", flight.ticks);
    }

    #[test]
    fn blackbird_plumes_are_steady_during_reheat_and_absent_in_cruise() {
        const SIDE: usize = 112;
        let seed = 439;
        let world = World::new(seed);
        let mut flight = Flight::start(&world, seed, Aircraft::Blackbird);
        let steady = (0..6 * 7 * 60 * 30).find(|&tick| {
            blackbird_reheat(seed, tick) == 1.0 && blackbird_reheat(seed, tick + 30) == 1.0
        }).expect("a steady part of a scheduled burst");
        for bank in [-0.36, 0.0, 0.34] {
            flight.bank = bank;
            flight.view.bank = bank * 0.36;
            let pose = chase_projection(&flight, SIDE, SIDE);
            let mut body = [0; SIDE * SIDE];
            paint_aircraft(
                flight.aircraft, 0, pose,
                &mut SliceCanvas { pixels: &mut body, width: SIDE }, SIDE, SIDE,
            );

            flight.ticks = 0;
            let mut cruise = [0; SIDE * SIDE];
            draw_aircraft(&flight, &mut cruise, SIDE, SIDE);
            assert_eq!(cruise, body);

            flight.ticks = steady;
            let mut day = [0; SIDE * SIDE];
            draw_aircraft(&flight, &mut day, SIDE, SIDE);
            flight.ticks += 30;
            let mut next = [0; SIDE * SIDE];
            draw_aircraft(&flight, &mut next, SIDE, SIDE);
            assert_eq!(day, next);

            let mut warm = [0; 2];
            let mut sheath = [0; 2];
            for (index, (&pixel, &base)) in day.iter().zip(&body).enumerate() {
                if pixel == base {
                    continue;
                }
                let (x, y) = (index % SIDE, index / SIDE);
                let dx = 2 * x as i32 + 1 - SIDE as i32;
                let dy = 2 * y as i32 + 1 - SIDE as i32;
                assert!(dx * dx + dy * dy <= (SIDE * SIDE) as i32);
                let side = usize::from(x >= SIDE / 2);
                let red = pixel >> 11;
                let blue = pixel & 31;
                warm[side] += usize::from(red >= 16 && red > blue);
                sheath[side] += usize::from(blue >= 5 && blue > red);
            }
            assert!(warm.into_iter().all(|count| count >= 2), "warm cells {warm:?}; bank {bank}");
            assert!(sheath.into_iter().all(|count| count >= 2), "violet edge {sheath:?}; bank {bank}");

            let mut dusk = [0; SIDE * SIDE];
            paint_chase_aircraft(&flight, pose, 0.25, &mut SliceCanvas { pixels: &mut dusk, width: SIDE }, SIDE, SIDE, None);
            let intensity = |frame: &[u16]| frame.iter().zip(&body)
                .filter(|(pixel, base)| pixel != base)
                .map(|(&pixel, _)| u32::from((pixel >> 11) + ((pixel >> 5) & 63) + (pixel & 31)))
                .sum::<u32>();
            assert!(intensity(&dusk) > intensity(&day));
        }
    }

    #[test]
    fn other_aircraft_keep_the_plain_mesh_in_flight() {
        const SIDE: usize = 112;
        let world = World::new(439);
        for aircraft in Aircraft::ALL.into_iter().filter(|&craft| craft != Aircraft::Blackbird) {
            let flight = Flight::start(&world, 439, aircraft);
            let mut expected = [0; SIDE * SIDE];
            paint_aircraft(
                aircraft, 0, chase_projection(&flight, SIDE, SIDE),
                &mut SliceCanvas { pixels: &mut expected, width: SIDE }, SIDE, SIDE,
            );
            let mut actual = [0; SIDE * SIDE];
            draw_aircraft(&flight, &mut actual, SIDE, SIDE);
            assert_eq!(expected, actual, "{aircraft:?}");
        }
    }

    #[test]
    fn cub_nose_tracks_the_interpolated_climb_without_moving_its_center() {
        let world = World::new(439);
        let mut flight = Flight::start(&world, 439, Aircraft::BushPlane);
        assert!(chase_projection(&flight, 112, 112).model_pitch.is_none());
        let mut ascending = None;
        let mut descending = None;
        for _ in 0..4_000 {
            let before = flight.climb_rate();
            flight.update(&world);
            let after = flight.climb_rate();
            assert_eq!(flight.display_pose(0.0).climb_rate(), before);
            let middle = flight.display_pose(0.5);
            assert!((middle.climb_rate() - (before + after) * 0.5).abs() < 0.00001);
            if middle.climb_rate() > 4.0 && ascending.is_none() {
                ascending = Some(middle);
            } else if middle.climb_rate() < -4.0 && descending.is_none() {
                descending = Some(middle);
            }
            if ascending.is_some() && descending.is_some() { break; }
        }
        for (flight, direction) in [(ascending.unwrap(), -1.0), (descending.unwrap(), 1.0)] {
            for side in [112, 144, 192, 466] {
                let pose = chase_projection(&flight, side, side);
                let level = Projection { model_pitch: None, ..pose };
                let nose = [0.0, 0.16, 2.24];
                let (nose, _) = pose.screen_point(nose).unwrap();
                let (old_nose, _) = level.screen_point([0.0, 0.16, 2.24]).unwrap();
                assert!((nose[1] - old_nose[1]) * direction > side as f32 * 0.012);
                assert_eq!(pose.screen_point([0.0; 3]), level.screen_point([0.0; 3]));
                let mut pixels = std::vec![0; side * side];
                draw_aircraft(&flight, &mut pixels, side, side);
                for (index, _) in pixels.iter().enumerate().filter(|&(_, &pixel)| pixel != 0) {
                    let dx = 2 * (index % side) as i32 + 1 - side as i32;
                    let dy = 2 * (index / side) as i32 + 1 - side as i32;
                    assert!(dx * dx + dy * dy < (side * side) as i32);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Face {
    corners: [[i16; 2]; 3],
    depth: f32,
    color: u16,
}

impl Face {
    const EMPTY: Self = Self {
        corners: [[0; 2]; 3],
        depth: 0.0,
        color: 0,
    };
}

struct Painter {
    projection: Projection,
    triangles: [Face; FACE_CAPACITY],
    len: usize,
}

impl Painter {
    fn triangle(&mut self, a: Point, b: Point, c: Point, paint: Paint) {
        if self.len == FACE_CAPACITY {
            return;
        }
        let Some(pa) = self.projection.vertex(a) else {
            return;
        };
        let Some(pb) = self.projection.vertex(b) else {
            return;
        };
        let Some(pc) = self.projection.vertex(c) else {
            return;
        };
        let color = face_color(a, b, c, paint);
        let face = Face {
            corners: [[pa.x, pa.y], [pb.x, pb.y], [pc.x, pc.y]],
            depth: pa.depth + pb.depth + pc.depth,
            color,
        };
        let mut index = self.len;
        while index > 0 && self.triangles[index - 1].depth < face.depth {
            self.triangles[index] = self.triangles[index - 1];
            index -= 1;
        }
        self.triangles[index] = face;
        self.len += 1;
    }

    fn quad(&mut self, a: Point, b: Point, c: Point, d: Point, paint: Paint) {
        self.triangle(a, b, c, paint);
        self.triangle(a, c, d, paint);
    }
}

fn paint_aircraft(
    aircraft: Aircraft,
    ticks: u32,
    projection: Projection,
    canvas: &mut impl AircraftCanvas,
    w: usize,
    h: usize,
) {
    paint_model(projection, canvas, w, h, |painter| match aircraft {
        Aircraft::BushPlane => bush_plane(painter),
        Aircraft::Racer => racer(painter),
        Aircraft::SportJet => sport_jet(painter),
        Aircraft::Trike => ultralight(painter),
        Aircraft::Glider => glider(painter, None),
        Aircraft::Fighter => fighter(painter),
        Aircraft::Blackbird => blackbird(painter),
        Aircraft::BaldEagle => bald_eagle(painter, ticks),
    });
}

fn paint_model(
    projection: Projection,
    canvas: &mut impl AircraftCanvas,
    w: usize,
    h: usize,
    mesh: impl FnOnce(&mut Painter),
) {
    let mut painter = Painter {
        projection,
        triangles: [Face::EMPTY; FACE_CAPACITY],
        len: 0,
    };
    mesh(&mut painter);
    for &face in &painter.triangles[..painter.len] {
        rasterize(face, canvas, w, h);
    }
}

fn face_color(a: Point, b: Point, c: Point, paint: Paint) -> u16 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let dz = b[2] - a[2];
    let ex = c[0] - a[0];
    let ey = c[1] - a[1];
    let ez = c[2] - a[2];
    let nx = (dy * ez - dz * ey).abs();
    let ny = (dz * ex - dx * ez).abs();
    let nz = (dx * ey - dy * ex).abs();
    let gain = 0.72 + (ny * 0.25 + nz * 0.08) / (nx + ny + nz).max(0.0001);
    rgb565(
        (paint[0] as f32 * gain) as u16,
        (paint[1] as f32 * gain) as u16,
        (paint[2] as f32 * gain) as u16,
    )
}

const fn rgb565(red: u16, green: u16, blue: u16) -> u16 {
    ((red >> 3) << 11) | ((green >> 2) << 5) | (blue >> 3)
}

fn rasterize(face: Face, canvas: &mut impl AircraftCanvas, w: usize, h: usize) {
    let [a, b, c] = face.corners.map(|v| v.map(i32::from));
    let area = edge(a, b, c);
    if area == 0 {
        return;
    }
    let orientation = area.signum();
    let x0 = (a[0].min(b[0]).min(c[0]) >> 2).max(0);
    let y0 = (a[1].min(b[1]).min(c[1]) >> 2).max(0);
    let x1 = ((a[0].max(b[0]).max(c[0]) + 3) >> 2).min(w as i32 - 1);
    let y1 = ((a[1].max(b[1]).max(c[1]) + 3) >> 2).min(h as i32 - 1);
    let radius_squared = (w.min(h) * w.min(h)) as i32;
    let start = [x0 << 2, y0 << 2];
    let mut ab_row = orientation * edge(a, b, start);
    let mut bc_row = orientation * edge(b, c, start);
    let mut ca_row = orientation * edge(c, a, start);
    let dx_ab = (a[1] - b[1]) * 4 * orientation;
    let dx_bc = (b[1] - c[1]) * 4 * orientation;
    let dx_ca = (c[1] - a[1]) * 4 * orientation;
    let dy_ab = (b[0] - a[0]) * 4 * orientation;
    let dy_bc = (c[0] - b[0]) * 4 * orientation;
    let dy_ca = (a[0] - c[0]) * 4 * orientation;
    // Keep the affine interval calculation inside the original i32 edge
    // arithmetic for faces projecting far outside the viewport.
    let use_spans = canvas.supports_opaque_spans() && face.corners.iter().flatten()
        .all(|&coordinate| (-8192..=8192).contains(&coordinate));
    for y in y0..=y1 {
        let cy = 2 * y + 1 - h as i32;
        if use_spans {
            let edges = [(ab_row, dx_ab), (bc_row, dx_bc), (ca_row, dx_ca)];
            let available = radius_squared - cy * cy;
            if let Some((first, last)) = opaque_span(edges, x0, x1, w as i32, available) {
                canvas.opaque_span(first, last + 1, y as usize, face.color);
            }
        } else {
            let (mut ab, mut bc, mut ca) = (ab_row, bc_row, ca_row);
            for x in x0..=x1 {
                let cx = 2 * x + 1 - w as i32;
                if ab >= 0 && bc >= 0 && ca >= 0 && cx * cx + cy * cy <= radius_squared {
                    *canvas.pixel(x as usize, y as usize) = face.color;
                }
                ab += dx_ab;
                bc += dx_bc;
                ca += dx_ca;
            }
        }
        ab_row += dy_ab;
        bc_row += dy_bc;
        ca_row += dy_ca;
    }
}

// Each edge is affine along a scanline. Intersect its inclusive integer
// interval with the other edges and the circle used by the ordinary painter.
fn opaque_span(
    edges: [(i32, i32); 3],
    x0: i32,
    x1: i32,
    width: i32,
    available: i32,
) -> Option<(usize, usize)> {
    if x1 < x0 || available < 0 {
        return None;
    }
    let mut first = 0;
    let mut last = x1 - x0;
    for (edge, step) in edges {
        let at_first = edge + first * step;
        let at_last = edge + last * step;
        if at_first < 0 {
            if at_last < 0 {
                return None;
            }
            first += (-at_first + step - 1) / step;
        } else if at_last < 0 {
            last = first + at_first / -step;
        }
    }
    let in_circle = |x| {
        let dx = 2 * x + 1 - width;
        dx * dx <= available
    };
    first += x0;
    last += x0;
    while first <= last && !in_circle(first) {
        first += 1;
    }
    if first > last {
        return None;
    }
    while !in_circle(last) {
        last -= 1;
    }
    Some((first as usize, last as usize))
}

fn edge(a: [i32; 2], b: [i32; 2], p: [i32; 2]) -> i32 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

fn shell(p: &mut Painter, sections: &[[f32; 4]], x: f32, y: f32, paint: Paint) {
    let ring = |[z, width, top, bottom]: [f32; 4]| {
        let middle = (top + bottom) * 0.43;
        [
            [x, y + top, z],
            [x + width, y + middle, z],
            [x, y + bottom, z],
            [x - width, y + middle, z],
        ]
    };
    for pair in sections.windows(2) {
        let a = ring(pair[0]);
        let b = ring(pair[1]);
        for i in 0..4 {
            let next = (i + 1) % 4;
            p.quad(a[i], a[next], b[next], b[i], paint);
        }
    }
    for row in [sections[0], sections[sections.len() - 1]] {
        let a = ring(row);
        p.quad(a[0], a[1], a[2], a[3], paint);
    }
}

fn wing(p: &mut Painter, root: [Point; 2], tip: [Point; 2], paint: Paint, trim: Paint) {
    for hand in [-1.0, 1.0] {
        wing_half(p, hand, root, tip, paint, trim);
    }
}

fn wing_half(p: &mut Painter, hand: f32, root: [Point; 2], tip: [Point; 2], paint: Paint, trim: Paint) {
    let mirror = |[x, y, z]: Point| [hand * x, y, z];
    let [a, b] = root.map(mirror);
    let [c, d] = tip.map(mirror);
    let blend = |point: Point, edge: Point| {
        core::array::from_fn(|axis| point[axis] + (edge[axis] - point[axis]) * 0.82)
    };
    let e = blend(a, c);
    let f = blend(b, d);
    p.quad(a, e, f, b, paint);
    p.quad(e, c, d, f, trim);
}

fn fin(p: &mut Painter, x: f32, top_x: f32, y: f32, top: f32, lead: f32, trail: f32, paint: Paint) {
    p.quad(
        [x, y, lead],
        [top_x, top, lead - 0.52],
        [top_x, top, trail + 0.14],
        [x, y, trail],
        paint,
    );
}

fn strut(p: &mut Painter, a: Point, b: Point, width: f32, paint: Paint) {
    p.quad(
        [a[0] - width, a[1], a[2]],
        [a[0] + width, a[1], a[2]],
        [b[0] + width, b[1], b[2]],
        [b[0] - width, b[1], b[2]],
        paint,
    );
    p.quad(
        [a[0], a[1] - width, a[2]],
        [a[0], a[1] + width, a[2]],
        [b[0], b[1] + width, b[2]],
        [b[0], b[1] - width, b[2]],
        paint,
    );
}

fn window(p: &mut Painter, y: f32, front: f32, rear: f32, width: f32) {
    canopy(p, y, front, rear, width, GLASS);
}

fn canopy(p: &mut Painter, y: f32, front: f32, rear: f32, width: f32, glass: Paint) {
    p.quad(
        [-width, y - 0.17, front],
        [width, y - 0.17, front],
        [width * 0.80, y, rear],
        [-width * 0.80, y, rear],
        glass,
    );
    for side in [-1.0, 1.0] {
        p.triangle(
            [side * width, y - 0.17, front],
            [side * width * 0.8, y, rear],
            [side * width, y - 0.17, rear - 0.17],
            glass,
        );
    }
}

fn wheel(p: &mut Painter, x: f32, y: f32, z: f32, size: f32) {
    let a = size * 0.43;
    p.quad(
        [x - a, y + size, z - a],
        [x + a, y + size, z - a],
        [x + a, y - size, z + a],
        [x - a, y - size, z + a],
        RUBBER,
    );
    for side in [-1.0, 1.0] {
        let side = side * a + x;
        p.quad(
            [side, y + size, z],
            [side, y, z + size],
            [side, y - size, z],
            [side, y, z - size],
            RUBBER,
        );
    }
}

fn bush_plane(p: &mut Painter) {
    let ochre = [223, 161, 47];
    shell(
        p,
        &[
            [2.2, 0.12, 0.21, -0.16],
            [1.4, 0.43, 0.55, -0.35],
            [0.1, 0.57, 0.84, -0.47],
            [-2.7, 0.11, 0.30, -0.14],
        ],
        0.0,
        0.0,
        ochre,
    );
    wing(
        p,
        [[0.16, 0.73, 0.95], [0.16, 0.73, -0.83]],
        BUSH_WINGTIP,
        ochre,
        [191, 62, 48],
    );
    wing(
        p,
        [[0.05, 0.18, -1.95], [0.05, 0.18, -2.67]],
        [[1.42, 0.23, -2.12], [1.42, 0.23, -2.62]],
        ochre,
        PALE,
    );
    fin(p, 0.0, 0.0, 0.24, 1.26, -1.45, -2.69, ochre);
    window(p, 0.79, 0.89, 0.15, 0.55);
    for side in [-1.0, 1.0] {
        strut(
            p,
            [0.38 * side, -0.36, 0.40],
            [2.40 * side, 0.70, 0.17],
            0.045,
            ochre,
        );
        wheel(p, 0.69 * side, -0.72, 0.49, 0.23);
    }
    strut(p, [-0.76, 0.16, 2.24], [0.76, 0.16, 2.24], 0.044, RUBBER);
}

fn racer(p: &mut Painter) {
    let crimson = [191, 48, 52];
    shell(
        p,
        &[
            [2.32, 0.08, 0.18, -0.12],
            [1.13, 0.42, 0.43, -0.25],
            [-0.18, 0.45, 0.55, -0.30],
            [-2.45, 0.09, 0.20, -0.08],
        ],
        0.0,
        0.0,
        crimson,
    );
    wing(
        p,
        [[0.18, -0.17, 0.9], [0.18, -0.19, -0.95]],
        RACER_WINGTIP,
        PALE,
        crimson,
    );
    wing(
        p,
        [[0.0, 0.19, -1.73], [0.0, 0.19, -2.45]],
        [[1.37, 0.25, -1.99], [1.37, 0.21, -2.48]],
        crimson,
        PALE,
    );
    fin(p, 0.0, 0.0, 0.17, 1.09, -1.36, -2.47, crimson);
    window(p, 0.64, 0.60, -0.23, 0.37);
    strut(p, [-0.71, 0.12, 2.35], [0.71, 0.24, 2.35], 0.042, RUBBER);
}

fn sport_jet(p: &mut Painter) {
    let pearl = [220, 228, 222];
    let blue = [75, 145, 178];
    shell(
        p,
        &[
            [3.12, 0.04, 0.16, -0.08],
            [1.88, 0.43, 0.54, -0.43],
            [-1.75, 0.55, 0.57, -0.45],
            [-3.10, 0.11, 0.24, -0.08],
        ],
        0.0,
        0.0,
        pearl,
    );
    wing(
        p,
        [[0.14, -0.18, 0.9], [0.14, -0.18, -1.32]],
        SPORT_JET_WINGTIP,
        pearl,
        blue,
    );
    fin(p, 0.0, 0.0, 0.33, 1.58, -1.75, -3.19, blue);
    wing(
        p,
        [[0.0, 1.12, -2.10], [0.0, 1.10, -3.10]],
        [[1.43, 1.15, -2.60], [1.43, 1.13, -3.11]],
        pearl,
        blue,
    );
    for side in [-1.0, 1.0] {
        shell(
            p,
            &[
                [-0.64, 0.24, 0.26, -0.21],
                [-1.92, 0.27, 0.20, -0.22],
                [-2.23, 0.13, 0.14, -0.13],
            ],
            side * 0.72,
            0.08,
            blue,
        );
    }
    window(p, 0.58, 2.48, 1.65, 0.37);
}

fn ultralight(p: &mut Painter) {
    let teal = [70, 171, 169];
    let orange = [228, 146, 55];
    let tubing = [109, 115, 122];
    p.triangle(
        [0.0, 1.77, 1.72],
        [-4.12, 1.35, -1.62],
        [0.0, 1.41, -0.96],
        teal,
    );
    p.triangle(
        [0.0, 1.77, 1.72],
        [0.0, 1.41, -0.96],
        [4.12, 1.35, -1.62],
        teal,
    );
    p.triangle(
        [-3.05, 1.46, -0.75],
        [-4.12, 1.35, -1.62],
        [-2.66, 1.41, -1.39],
        orange,
    );
    p.triangle(
        [3.05, 1.46, -0.75],
        [2.66, 1.41, -1.39],
        [4.12, 1.35, -1.62],
        orange,
    );
    shell(
        p,
        &[
            [1.44, 0.17, -0.34, -0.64],
            [0.12, 0.39, -0.15, -0.71],
            [-0.85, 0.28, -0.32, -0.65],
        ],
        0.0,
        0.0,
        orange,
    );
    strut(p, [0.0, -0.36, -0.2], [0.0, 1.74, 0.59], 0.05, tubing);
    for side in [-1.0, 1.0] {
        strut(
            p,
            [0.26 * side, -0.41, 0.22],
            [side * 1.82, 1.45, -0.31],
            0.032,
            tubing,
        );
        wheel(p, side * 0.62, -0.83, -0.52, 0.17);
    }
    wheel(p, 0.0, -0.80, 0.90, 0.16);
    strut(p, [-0.42, 0.04, -0.73], [0.42, 0.10, -0.73], 0.052, RUBBER);
}

fn glider(p: &mut Painter, reflection: Option<GliderReflection>) {
    let white = [229, 231, 222];
    let stripe = [198, 78, 62];
    shell(
        p,
        &[
            [2.38, 0.05, 0.10, -0.07],
            [1.39, 0.35, 0.43, -0.30],
            [-0.22, 0.29, 0.31, -0.27],
            [-3.00, 0.07, 0.15, -0.07],
        ],
        0.0,
        0.0,
        white,
    );
    for side in [-1.0, 1.0] {
        let glare = reflection.filter(|light| light.side == side).map_or(0.0, |light| light.strength);
        wing_half(p, side, [[0.1, 0.14, 0.65], [0.1, 0.10, -0.74]],
                  [[5.03, 0.49, -0.22], [5.09, 0.51, -0.57]],
                  reflect(white, [253, 245, 219], glare * 0.76),
                  reflect(stripe, [252, 185, 118], glare * 0.73));
    }
    fin(p, 0.0, 0.0, 0.15, 1.25, -1.91, -3.10, white);
    wing(
        p,
        [[0.0, 1.10, -2.37], [0.0, 1.08, -3.10]],
        [[1.49, 1.15, -2.78], [1.49, 1.13, -3.12]],
        white,
        stripe,
    );
    let glass = reflect(GLASS, [200, 225, 227], reflection.map_or(0.0, |light| light.strength * 0.78));
    canopy(p, 0.46, 1.49, 0.45, 0.32, glass);
}

fn reflect(base: Paint, sheen: Paint, strength: f32) -> Paint {
    core::array::from_fn(|i| {
        let start = f32::from(base[i]);
        (start + (f32::from(sheen[i]) - start) * strength + 0.5) as u8
    })
}

fn fighter(p: &mut Painter) {
    let silver = [151, 165, 172];
    let dark = [90, 109, 126];
    shell(
        p,
        &[
            [3.30, 0.04, 0.09, -0.11],
            [1.80, 0.40, 0.55, -0.33],
            [0.25, 0.71, 0.51, -0.55],
            [-2.66, 0.60, 0.36, -0.44],
        ],
        0.0,
        0.0,
        silver,
    );
    wing(
        p,
        [[0.20, 0.08, 1.66], [0.42, 0.03, -1.80]],
        FIGHTER_WINGTIP,
        silver,
        dark,
    );
    wing(
        p,
        [[0.49, 0.12, -1.68], [0.49, 0.13, -2.64]],
        [[2.31, 0.23, -2.07], [2.22, 0.22, -2.88]],
        silver,
        dark,
    );
    for side in [-1.0, 1.0] {
        fin(p, side * 0.54, side * 0.98, 0.29, 1.49, -1.18, -2.67, dark);
        shell(
            p,
            &[[-1.25, 0.29, 0.13, -0.14], FIGHTER_NOZZLE],
            side * FIGHTER_ENGINE_X,
            FIGHTER_ENGINE_Y,
            RUBBER,
        );
    }
    window(p, 0.63, 1.65, 0.49, 0.37);
}

fn wingmate_mesh(p: &mut Painter, index: usize, gain: f32, warm: f32) {
    let paint = |[r, g, b]: Paint| [
        (f32::from(r) * gain + 18.0 * warm) as u8,
        (f32::from(g) * gain + 5.0 * warm) as u8,
        (f32::from(b) * gain - 9.0 * warm) as u8,
    ];
    let skin = paint(if index < 2 { [173, 184, 191] } else { [138, 157, 168] });
    let trim = paint(if index & 1 == 0 { [94, 111, 126] } else { [108, 110, 122] });
    let engines = paint([60, 69, 81]);
    let canopy = paint([76, 111, 136]);
    shell(
        p,
        &[
            [3.25, 0.04, 0.09, -0.08],
            [0.72, 0.68, 0.60, -0.52],
            [-2.74, 0.64, 0.31, -0.40],
        ],
        0.0,
        0.0,
        skin,
    );
    for side in [-1.0, 1.0] {
        p.quad(
            [side * 0.24, 0.10, 1.62],
            [side * WINGMATE_WINGTIP[0][0], WINGMATE_WINGTIP[0][1], WINGMATE_WINGTIP[0][2]],
            [side * WINGMATE_WINGTIP[1][0], WINGMATE_WINGTIP[1][1], WINGMATE_WINGTIP[1][2]],
            [side * 0.59, 0.02, -1.96],
            skin,
        );
        p.triangle(
            [side * 0.60, 0.15, -1.70],
            [side * 2.23, 0.24, -2.57],
            [side * 0.43, 0.14, -2.91],
            skin,
        );
        fin(p, side * 0.54, side * 0.97, 0.26, 1.45, -1.17, -2.74, trim);
        p.quad(
            [side * 0.18, 0.10, -2.79],
            [side * 0.60, 0.10, -2.79],
            [side * 0.60, -0.35, -2.79],
            [side * 0.18, -0.35, -2.79],
            engines,
        );
    }
    p.quad(
        [-0.25, 0.52, 1.74],
        [0.25, 0.52, 1.74],
        [0.29, 0.73, 0.57],
        [-0.29, 0.73, 0.57],
        canopy,
    );
}

fn solid_wing(
    p: &mut Painter, root: [Point; 2], tip: [Point; 2], thickness: f32, paint: Paint, trim: Paint,
) {
    for side in [-1.0, 1.0] {
        let wing = [root[0], tip[0], tip[1], root[1]].map(|[x, y, z]| [side * x, y, z]);
        let underside = wing.map(|[x, y, z]| [x, y - thickness, z]);
        p.quad(wing[0], wing[1], wing[2], wing[3], paint);
        p.quad(underside[0], underside[1], underside[2], underside[3], trim);
        for i in 0..4 {
            let j = (i + 1) % 4;
            p.quad(wing[i], wing[j], underside[j], underside[i], trim);
        }
    }
}

fn solid_fin(p: &mut Painter, root: [Point; 4], top: [Point; 4], paint: Paint, trim: Paint) {
    for i in 0..4 {
        let next = (i + 1) % 4;
        p.quad(root[i], root[next], top[next], top[i], paint);
    }
    p.quad(top[0], top[1], top[2], top[3], trim);
}

fn draw_tanker(
    player: Projection, event: Refuel, daylight: f32, canvas: &mut impl AircraftCanvas,
    width: usize, height: usize,
) {
    let arrival = 1.0 - smoothstep((event.progress - 0.19) / 0.25);
    let departure = smoothstep((event.progress - 0.70) / 0.30);
    let lateral = event.side * (48.0 * arrival + 49.0 * departure);
    let tanker = Projection {
        origin: player.camera_point([lateral, 3.0 + 3.6 * arrival + 6.1 * departure,
            14.0 + 8.0 * arrival + 10.0 * departure]),
        ..player
    };
    let boom = smoothstep((event.progress - 0.29) / 0.15)
        * (1.0 - smoothstep((event.progress - 0.65) / 0.12));
    if visible_wingmate(&tanker, width, height) {
        paint_model(tanker, canvas, width, height, |p| tanker_mesh(p, daylight, boom));
        paint_position_lights(tanker, TANKER_WINGTIP, Some([0.0, 0.15, -6.42]),
            daylight, canvas, (width, height));
    }
}

fn tanker_mesh(p: &mut Painter, light: f32, boom: f32) {
    let paint = |color: Paint| color.map(|channel| (f32::from(channel) * (0.48 + light * 0.52)) as u8);
    let skin = paint([176, 183, 190]);
    let edge = paint([116, 133, 151]);
    let metal = paint([202, 192, 174]);
    shell(p, &[
        [6.8, 0.12, 0.24, -0.22], [4.8, 0.92, 1.0, -0.85],
        [-3.6, 0.91, 0.92, -0.80], [-6.4, 0.27, 0.35, -0.27],
    ], 0.0, 0.0, skin);
    solid_wing(p, [[0.50, 0.22, 2.45], [0.64, 0.21, -2.80]],
        TANKER_WINGTIP, 0.56, skin, edge);
    wing(p, [[0.19, 0.19, -4.8], [0.20, 0.22, -6.6]],
        [[3.25, 0.32, -6.52], [3.25, 0.34, -7.0]], skin, edge);
    solid_fin(p,
        [[-0.31, 0.06, -4.2], [0.31, 0.06, -4.2], [0.24, 0.04, -6.36], [-0.24, 0.04, -6.36]],
        [[-0.26, 2.78, -5.07], [0.26, 2.78, -5.07], [0.24, 2.78, -6.18], [-0.24, 2.78, -6.18]],
        edge, skin);
    for side in [-1.0, 1.0] {
        for (x, z) in [(side * 2.75, -0.65), (side * 5.06, -1.99)] {
            p.quad([x - 0.35, -0.08, z + 0.1], [x + 0.35, -0.08, z + 0.1],
                [x + 0.35, -0.58, z - 0.5], [x - 0.35, -0.58, z - 0.5], edge);
            shell(p, &[
                [z + 0.40, 0.37, 0.62, -0.33], [z - 1.35, 0.29, 0.47, -0.26],
            ], x, -0.53, edge);
        }
    }
    if boom > 0.0 {
        let hinge = [0.0, -0.45, -5.45];
        let elbow = [0.0, -0.47 - 1.15 * boom, -6.30 - 3.55 * boom];
        let tip = [0.0, -0.57 - 1.91 * boom, -6.85 - 6.83 * boom];
        strut(p, hinge, elbow, 0.33, edge);
        strut(p, elbow, tip, 0.33, metal);
    }
}

fn blackbird(p: &mut Painter) {
    let obsidian = [51, 62, 74];
    let edge = [87, 104, 119];
    shell(
        p,
        &[
            [4.45, 0.025, 0.075, -0.06],
            [2.08, 0.39, 0.43, -0.25],
            [-0.51, 0.70, 0.51, -0.37],
            [-3.67, 0.15, 0.17, -0.14],
        ],
        0.0,
        0.0,
        obsidian,
    );
    solid_wing(p, [[0.1, 0.22, 2.63], [0.21, -0.05, -3.42]],
        BLACKBIRD_WINGTIP, 0.24, obsidian, edge);
    for side in [-1.0, 1.0] {
        let x = side * BLACKBIRD_ENGINE_X;
        shell(
            p,
            &[
                [1.26, 0.04, 0.11, -0.06],
                [0.38, 0.55, 0.46, -0.40],
                [-2.98, 0.47, 0.41, -0.32],
                BLACKBIRD_NOZZLE,
            ],
            x,
            0.0,
            edge,
        );
        let root = [[x - 0.15, 0.13, -1.55], [x + 0.15, 0.13, -1.55],
                    [x + 0.15, 0.10, -3.34], [x - 0.15, 0.10, -3.34]];
        let top_x = side * 1.89;
        let top = [[top_x - 0.16, 1.69, -2.19], [top_x + 0.16, 1.69, -2.19],
                   [top_x + 0.16, 1.69, -3.10], [top_x - 0.16, 1.69, -3.10]];
        solid_fin(p, root, top, obsidian, edge);
    }
    window(p, 0.49, 2.28, 1.60, 0.28);
}

fn bald_eagle(p: &mut Painter, ticks: u32) {
    let brown = [81, 58, 45];
    let covert = [105, 76, 56];
    let primary = [54, 41, 36];
    let cream = [238, 234, 219];
    let amber = [225, 151, 48];
    shell(
        p,
        &[
            [0.91, 0.35, 0.27, -0.37],
            [0.13, 0.48, 0.42, -0.49],
            [-1.15, 0.23, 0.26, -0.17],
        ],
        0.0,
        0.0,
        brown,
    );
    shell(
        p,
        &[
            [1.49, 0.28, 0.52, 0.19],
            [0.92, 0.37, 0.61, 0.12],
            [0.57, 0.24, 0.46, 0.12],
        ],
        0.0,
        0.0,
        cream,
    );
    p.triangle(
        [-0.23, 0.29, 1.44],
        [0.23, 0.29, 1.44],
        [0.0, 0.12, 1.92],
        amber,
    );
    p.triangle(
        [0.0, 0.54, 1.45],
        [0.24, 0.29, 1.45],
        [0.0, 0.12, 1.92],
        amber,
    );
    p.triangle(
        [0.0, 0.54, 1.45],
        [0.0, 0.12, 1.92],
        [-0.24, 0.29, 1.45],
        amber,
    );

    let turn = (ticks % 360) as f32;
    let wave = (ticks % 30) as f32 * (TAU / 30.0);
    let effort = smoothstep(turn / 11.0) * (1.0 - smoothstep((turn - 32.0) / 32.0));
    let shoulder = 0.36 * effort * sin_cos(wave).0;
    let wrist = 0.17 * effort * sin_cos(wave - 0.85).0;
    let breeze = 0.10 * (1.0 - effort) * sin_cos((ticks % 160) as f32 * (TAU / 160.0)).0;
    for side in [-1.0, 1.0] {
        let feather_flex = breeze + side * (1.0 - effort) * 0.055
            * sin_cos((ticks % 203) as f32 * (TAU / 203.0)).0;
        let r = |x: f32, y: f32, z: f32| [side * x, y, z];
        let root_front = r(0.33, 0.19, 0.71);
        let root_back = r(0.34, 0.12, -0.89);
        let elbow_front = r(1.44, 0.41 + shoulder, 0.69);
        let elbow_back = r(1.49, 0.23 + shoulder, -1.04);
        let wrist_front = r(2.73, 0.35 + shoulder * 1.8 + wrist, 0.26);
        let wrist_back = r(2.53, 0.21 + shoulder * 1.8 + wrist, -1.36);
        p.quad(root_front, elbow_front, elbow_back, root_back, brown);
        p.quad(elbow_front, wrist_front, wrist_back, elbow_back, covert);
        for feather in 0..4 {
            let t = feather as f32 * 0.25;
            let s = t + 0.25;
            let base = |t: f32| {
                r(
                    2.73 - t * 0.20,
                    0.35 - t * 0.14 + shoulder * 1.8 + wrist,
                    0.26 - t * 1.62,
                )
            };
            let lift = feather_flex + 0.14 * effort * sin_cos(wave - 1.25 - t * 0.45).0;
            let tip = r(
                3.90 - t * 0.52 + feather_flex * (1.0 - t),
                0.26 + shoulder * 2.05 + wrist + lift,
                0.04 - t * 1.87 + breeze * (0.8 - t),
            );
            p.triangle(base(t), tip, base(s), primary);
        }
        p.triangle(
            r(0.29, 0.45, 1.22),
            r(0.22, 0.46, 1.37),
            r(0.26, 0.38, 1.31),
            RUBBER,
        );
        p.triangle(
            r(0.16, -0.36, -0.37),
            r(0.28, -0.51, -0.57),
            r(0.23, -0.52, -0.34),
            amber,
        );
    }
    let tail_sway = 0.045 * sin_cos((ticks % 142) as f32 * (TAU / 142.0)).0;
    let tail_spread = 0.72 + p.projection.bank.0.abs() * 0.23;
    for feather in 0..5 {
        let side = (feather as f32 - 2.5) * 0.4;
        let point = |s: f32| {
            [s * tail_spread + tail_sway, 0.05 + 0.09 * s * s, -2.21 + 0.24 * s * s]
        };
        p.quad(
            [side * 0.23, 0.14, -0.91],
            point(side),
            point(side + 0.4),
            [(side + 0.4) * 0.23, 0.14, -0.91],
            if feather % 2 == 0 { cream } else { [226, 224, 212] },
        );
    }
}
