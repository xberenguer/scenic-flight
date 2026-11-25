//! Rare distant rain, retained in world coordinates and composited through depth.

use crate::flight::TICKS_PER_SECOND;
use crate::math::{fraction, mix, sin_cos, smoothstep};
use crate::render::{encoded_depth, view_distance, Camera};
use crate::sky::Sky;

mod valley_mist;
pub use valley_mist::{MistView, ValleyMist};

const CELL: f32 = 648.0;
const NEAR: f32 = 220.0;
const FAR: f32 = 556.0;
const TICKS: u64 = TICKS_PER_SECOND as u64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RainShaft {
    /// World positions in east, height, north order.
    pub top: [f32; 3],
    pub foot: [f32; 3],
    pub half_width: f32,
    pub opacity: f32,
    tint: u16,
    cloud_tint: u16,
    asymmetry: f32,
    rainbow: Option<Rainbow>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rainbow {
    sun: [f32; 3],
    clarity: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WeatherView {
    pub distant_rain: Option<RainShaft>,
    pub shower: f32,
}

#[derive(Clone, Copy)]
struct Site {
    key: u64,
    center: [f32; 2],
    width: f32,
    period: u16,
    phase: u16,
    life: u16,
}

/// Nine neighboring cells, refreshed only after the weather gate passes and
/// the camera enters a different cell or world. No terrain probes are needed.
pub struct WeatherEffects {
    region: Option<(u64, i32, i32)>,
    sites: [Option<Site>; 9],
}

impl Default for WeatherEffects {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherEffects {
    pub const fn new() -> Self {
        Self {
            region: None,
            sites: [None; 9],
        }
    }

    /// The returned shaft can be painted after terrain, meshes and sky.
    /// Use its presence to request a depth buffer only for the relevant frames.
    pub fn visible(
        &mut self,
        seed: u64,
        ticks: u64,
        camera: Camera,
        sky: &Sky,
    ) -> Option<RainShaft> {
        self.observe(seed, ticks, camera, sky).distant_rain
    }

    /// Observe both the column on the horizon and the same shower at the
    /// aircraft. Site weather is allowed to differ from the observer's air.
    pub fn observe(&mut self, seed: u64, ticks: u64, camera: Camera, sky: &Sky) -> WeatherView {
        if (sky.weather.is_none() && (sky.cloud_cover <= 0.70 || sky.haze <= 0.204))
            || sky.daylight <= 0.18
            || camera.altitude >= 180.0
            || ![camera.x, camera.altitude, camera.z, camera.heading]
                .into_iter()
                .all(f32::is_finite)
        {
            return WeatherView::default();
        }
        let gx = cell_at(camera.x);
        let gz = cell_at(camera.z);
        let key = (seed, gx, gz);
        if self.region != Some(key) {
            self.region = Some(key);
            self.sites = core::array::from_fn(|i| {
                Site::at(
                    seed,
                    gx.saturating_add(i as i32 % 3 - 1),
                    gz.saturating_add(i as i32 / 3 - 1),
                )
            });
        }
        let (sin, cos) = sin_cos(camera.heading);
        let mut view = WeatherView::default();
        for site in self.sites.iter().flatten() {
            let dx = site.center[0] - camera.x;
            let dz = site.center[1] - camera.z;
            let distance = dx * dx + dz * dz;
            if distance >= FAR * FAR {
                continue;
            }
            let forward = dx * sin + dz * cos;
            let side = (dx * cos - dz * sin).abs();
            let distant = distance >= NEAR * NEAR && forward >= 175.0
                && side <= forward * 0.71 + site.width + 24.0;
            if !distant && distance > 185.0 * 185.0 {
                continue;
            }
            let Some(event) = site.shower(ticks, camera.altitude, sky) else { continue; };
            view.shower = view.shower.max(event.local_strength(camera));
            if distant {
                let range = smoothstep((distance - NEAR * NEAR) / (330.0 * 330.0 - NEAR * NEAR))
                    * smoothstep((FAR * FAR - distance) / (FAR * FAR - 467.0 * 467.0));
                let opacity = event.opacity * range;
                if opacity > 0.014 && view.distant_rain.is_none_or(|previous| opacity > previous.opacity) {
                    view.distant_rain = Some(RainShaft { opacity, ..event });
                }
            }
        }
        view
    }
}

impl Site {
    fn at(seed: u64, x: i32, z: i32) -> Option<Self> {
        let key = cell_key(seed, x, z);
        // Spatial thinning leaves at least one empty cell between patches.
        // At the draw range, even drifting patches cannot replace each other
        // when the camera rounds a bend or crosses a cell boundary.
        for dz in -1_i32..=1 {
            for dx in -1_i32..=1 {
                if (dx != 0 || dz != 0)
                    && cell_key(seed, x.wrapping_add(dx), z.wrapping_add(dz)) >= key
                {
                    return None;
                }
            }
        }
        let field = mix(key ^ 0x97d2_7808_4ab4_9b67);
        let position = |tile, bits| (tile as f32 + (0.44 + fraction(bits) * 0.12)) * CELL;
        let period = (122 + (mix(field ^ 0x6421) % 66)) * TICKS;
        Some(Self {
            key,
            center: [position(x, field), position(z, mix(field))],
            width: 28.0 + fraction(mix(field ^ 0x9724)) * 20.0,
            period: period as u16,
            phase: (mix(field ^ 0x11c3) % period) as u16,
            life: ((67 + (mix(field ^ 0x6840) % 31)) * TICKS) as u16,
        })
    }

    fn shower(self, ticks: u64, altitude: f32, sky: &Sky) -> Option<RainShaft> {
        let period = u64::from(self.period);
        let phase = ((ticks % period + u64::from(self.phase)) % period) as u16;
        if phase >= self.life {
            return None;
        }
        let local = sky.weather_over(self.center[0], self.center[1]);
        let (cloud_start, haze_start) = if sky.weather.is_some() { (0.624, 0.186) } else { (0.70, 0.204) };
        let climate = smoothstep((local.cloud_cover - cloud_start) / 0.119)
            * smoothstep((local.haze - haze_start) / 0.104)
            * smoothstep((sky.daylight - 0.18) / 0.35)
            * (1.0 - smoothstep((altitude - 111.0) / 69.0));
        let growth = smoothstep(phase as f32 / (18 * TICKS) as f32)
            * smoothstep((self.life - phase) as f32 / (17 * TICKS) as f32);
        let opacity = climate * growth * 0.54;
        if opacity <= 0.004 {
            return None;
        }
        let progress = phase as f32 / self.life as f32;
        let pulse = sin_cos(progress * 8.8 + fraction(self.key) * 6.28).0;
        let wind = sky.cloud_offset;
        let scale = 15.0 / (wind[0].abs() + wind[1].abs()).max(16.0);
        let x = self.center[0] - wind[0] * scale * (0.22 + progress * 0.50);
        let z = self.center[1] - wind[1] * scale * (0.22 + progress * 0.50);
        let y = local.cloud_base + fraction(mix(self.key ^ 48)) * 6.0;
        let color = |base: f32, day: f32| base + (day - base) * sky.daylight;
        let tint = rgb565([color(71.0, 131.0), color(90.0, 150.0), color(115.0, 174.0)]);
        let cloud_tint = rgb565([color(80.0, 182.0), color(95.0, 190.0), color(121.0, 201.0)]);
        let sun_height = sky.sun_direction[1];
        let light = smoothstep(sun_height / 0.13)
            * (1.0 - smoothstep((sun_height - 0.40) / 0.23))
            * (1.0 - smoothstep((sky.cloud_cover - 0.44) / 0.31));
        let rainbow = (sky.weather.is_some() && light > 0.04).then_some(Rainbow {
            sun: sky.sun_direction,
            clarity: light,
        });
        Some(RainShaft {
            top: [x - wind[0] * scale * 0.38, y, z - wind[1] * scale * 0.38],
            foot: [x + wind[0] * scale * 0.64, -2.0, z + wind[1] * scale * 0.64],
            half_width: self.width * (1.0 + progress * 0.18 + pulse * 0.055),
            opacity,
            tint,
            cloud_tint,
            asymmetry: 0.22 * (fraction(mix(self.key ^ 775)) - 0.5) + 0.07 * pulse,
            rainbow,
        })
    }
}

impl RainShaft {
    fn local_strength(self, camera: Camera) -> f32 {
        let height = ((camera.altitude - self.foot[1]) / (self.top[1] - self.foot[1])).clamp(0.0, 1.0);
        let x = self.foot[0] + (self.top[0] - self.foot[0]) * height;
        let z = self.foot[2] + (self.top[2] - self.foot[2]) * height;
        let dx = camera.x - x;
        let dz = camera.z - z;
        let outer = self.half_width * 2.45;
        let horizontal = smoothstep(1.0 - (dx * dx + dz * dz) / (outer * outer));
        let under_cloud = 1.0 - smoothstep((camera.altitude - self.top[1] + 4.0) / 30.0);
        self.opacity * (1.0 / 0.54) * horizontal * under_cloud
    }

    /// Draw a soft depth-tested curtain in the circular view. The translucent
    /// layer never updates depth and casts no screen-space drops.
    pub fn paint(
        self,
        camera: Camera,
        dimensions: (usize, usize),
        pixels: &mut [u16],
        depth: &[u16],
    ) {
        let (width, height) = dimensions;
        if width == 0
            || height == 0
            || width > 512
            || height > 512
            || pixels.len() < width * height
            || depth.len() < width * height
            || self.opacity <= 0.0
            || self.half_width <= 0.0
        {
            return;
        }
        let view = Projection::new(camera, width, height);
        let (Some([ax, ay, za]), Some([bx, by, zb])) =
            (view.project(self.top), view.project(self.foot))
        else {
            return;
        };
        let dx = bx - ax;
        let dy = by - ay;
        let length_squared = dx * dx + dy * dy;
        if length_squared < 9.0 {
            return;
        }
        let length = sqrt(length_squared);
        let tx = dx / length;
        let ty = dy / length;
        let along_x = tx / length;
        let along_y = ty / length;
        let opacity = self.opacity.min(1.0);
        let inverse_focal = 1.0 / view.focal;
        let rainbow = self.rainbow.and_then(|bow| {
            let anti = view.direction(bow.sun.map(|value| -value));
            (anti[2] > 0.12).then_some((anti, bow.clarity * 1.22))
        });
        let ra = self.half_width * view.focal * za * 0.72;
        let rb = self.half_width * view.focal * zb * 1.22;
        let radius = (ra * 1.87).max(rb) + 1.0;
        let cloud_x = ax - dx * 0.26;
        let cloud_y = ay - dy * 0.26;
        let start_x = ((ax.min(bx).min(cloud_x) - radius) as i32)
            .max(0)
            .min(width as i32);
        let end_x = ((ax.max(bx).max(cloud_x) + radius) as i32 + 1).min(width as i32);
        let start_y = ((ay.min(by).min(cloud_y) - radius) as i32)
            .max(0)
            .min(height as i32);
        let end_y = ((ay.max(by).max(cloud_y) + radius) as i32 + 1).min(height as i32);
        let aperture = width.min(height) as i32;
        for y in start_y..end_y {
            let py = y as f32 - ay;
            let sy = 2 * y + 1 - height as i32;
            let limit = aperture * aperture - sy * sy;
            for x in start_x..end_x {
                let sx = 2 * x + 1 - width as i32;
                if sx * sx > limit {
                    continue;
                }
                let px = x as f32 - ax;
                let falling = px * along_x + py * along_y;
                if !(-0.26..1.0).contains(&falling) {
                    continue;
                }
                let cross = px * ty - py * tx;
                let cloud = if (-0.26..0.14).contains(&falling) {
                    let across = cross / ra - self.asymmetry;
                    opacity * cloud_patches(across, falling)
                } else {
                    0.0
                };
                let rain = if falling >= 0.0 {
                    let width = ra + (rb - ra) * falling;
                    let across = cross / width;
                    let softened = bell(across);
                    let vertical = smoothstep(falling / 0.18) * smoothstep((1.0 - falling) / 0.33);
                    let filaments = 0.76
                        + 0.28 * bell((across + 0.34 + self.asymmetry) * 2.5)
                        + 0.23 * bell((across - 0.37 + self.asymmetry * 0.6) * 2.8);
                    opacity * softened * softened * vertical * filaments
                } else {
                    0.0
                };
                if cloud <= 0.012 && rain <= 0.012 {
                    continue;
                }
                let reciprocal = za + (zb - za) * falling.max(0.0);
                let index = y as usize * width + x as usize;
                if encoded_depth(1.0 / reciprocal) < depth[index] {
                    let mut pixel = pixels[index];
                    if cloud > 0.012 {
                        pixel = blend(pixel, self.cloud_tint, cloud);
                    }
                    if rain > 0.012 {
                        pixel = blend(pixel, self.tint, rain);
                    }
                    if rain > 0.045 {
                        if let Some((anti, clarity)) = rainbow {
                            let u = (x as f32 - view.center[0]) * inverse_focal;
                            let v = (y as f32 - view.center[1]) * inverse_focal;
                            if let Some((color, edge)) = spectrum(anti, u, v) {
                                pixel = blend(pixel, color, (edge * clarity * rain).min(0.52));
                            }
                        }
                    }
                    pixels[index] = pixel;
                }
            }
        }
    }
}

fn spectrum(anti: [f32; 3], u: f32, v: f32) -> Option<(u16, f32)> {
    let dot = anti[0] * u + anti[1] * v + anti[2];
    if dot <= 0.0 {
        return None;
    }
    // The red edge is about 42 degrees from the antisolar point. The inner
    // colors converge more tightly, with a little width to read at 112 px.
    let cosine = dot * dot / (1.0 + u * u + v * v);
    let t = (cosine - 0.547) / 0.057;
    if !(0.0..1.0).contains(&t) {
        return None;
    }
    const BANDS: [[f32; 3]; 5] = [
        [245.0, 112.0, 94.0], [249.0, 188.0, 94.0], [165.0, 206.0, 113.0],
        [113.0, 185.0, 228.0], [171.0, 149.0, 225.0],
    ];
    let index = (t * 4.0) as usize;
    let mix = t * 4.0 - index as f32;
    let color = core::array::from_fn(|channel| {
        BANDS[index][channel] * (1.0 - mix) + BANDS[index + 1][channel] * mix
    });
    let edge = smoothstep(t / 0.17) * smoothstep((1.0 - t) / 0.20);
    Some((rgb565(color), edge))
}

fn bell(x: f32) -> f32 {
    (1.0 - x * x).max(0.0)
}

fn cloud_patches(across: f32, falling: f32) -> f32 {
    let west = bell((across + 0.72) * 1.12) * bell((falling + 0.015) * 9.5);
    let middle = bell((across + 0.04) * 0.97) * bell((falling + 0.055) * 5.9);
    let east = bell((across - 0.78) * 1.23) * bell((falling + 0.033) * 8.8);
    // Overlapping wisps create one soft cloud lip over the falling rain.
    (west * 0.33 + middle * 0.61 + east * 0.30).min(0.82)
}

struct Projection {
    camera: Camera,
    heading: (f32, f32),
    pitch: (f32, f32),
    bank: (f32, f32),
    focal: f32,
    center: [f32; 2],
    curvature: f32,
}

impl Projection {
    fn new(camera: Camera, width: usize, height: usize) -> Self {
        Self {
            camera,
            heading: sin_cos(camera.heading),
            pitch: sin_cos(camera.pitch.clamp(-1.3, 1.3)),
            bank: sin_cos(-camera.bank),
            focal: width.min(height) as f32 * 0.73,
            center: [(width - 1) as f32 * 0.5, (height - 1) as f32 * 0.5],
            curvature: (view_distance(camera.altitude) - 512.0) * (0.000040 / 384.0),
        }
    }

    fn project(&self, [x, y, z]: [f32; 3]) -> Option<[f32; 3]> {
        let dx = x - self.camera.x;
        let dz = z - self.camera.z;
        let y = y - self.camera.altitude - self.curvature * (dx * dx + dz * dz);
        let (hs, hc) = self.heading;
        let (ps, pc) = self.pitch;
        let (bs, bc) = self.bank;
        let forward = dx * hs + dz * hc;
        let depth = forward * pc - y * ps;
        if !depth.is_finite() || depth < 16.0 {
            return None;
        }
        let inverse = 1.0 / depth;
        let scale = self.focal * inverse;
        let right = dx * hc - dz * hs;
        let below = -ps * forward - pc * y;
        Some([
            self.center[0] + (right * bc - below * bs) * scale,
            self.center[1] + (right * bs + below * bc) * scale,
            inverse,
        ])
    }

    fn direction(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let (hs, hc) = self.heading;
        let (ps, pc) = self.pitch;
        let (bs, bc) = self.bank;
        let forward = x * hs + z * hc;
        let right = x * hc - z * hs;
        let below = -ps * forward - pc * y;
        [right * bc - below * bs, right * bs + below * bc, forward * pc - y * ps]
    }
}

fn cell_at(position: f32) -> i32 {
    let position = position / CELL;
    let index = position as i32;
    index.saturating_sub(i32::from(position < index as f32))
}

fn cell_key(seed: u64, x: i32, z: i32) -> u64 {
    let tile = (u64::from(x as u32) << 32) | u64::from(z as u32);
    mix(seed ^ mix(tile ^ 0x3a97_1468_a696_2199))
}

fn sqrt(value: f32) -> f32 {
    let estimate = f32::from_bits((value.to_bits() >> 1) + 0x1fc0_0000);
    let estimate = (estimate + value / estimate) * 0.5;
    (estimate + value / estimate) * 0.5
}

fn rgb565([r, g, b]: [f32; 3]) -> u16 {
    ((r * (31.0 / 255.0) + 0.5) as u16) << 11
        | ((g * (63.0 / 255.0) + 0.5) as u16) << 5
        | (b * (31.0 / 255.0) + 0.5) as u16
}

fn blend(background: u16, tint: u16, opacity: f32) -> u16 {
    blend_alpha(background, tint, (opacity * 256.0 + 0.5) as u32)
}

fn blend_alpha(background: u16, tint: u16, alpha: u32) -> u16 {
    let old = 256 - alpha;
    let chan = |shift: u32, mask: u16| {
        let bg = u32::from((background >> shift) & mask);
        let aim = u32::from((tint >> shift) & mask);
        (((bg * old + aim * alpha + 128) >> 8) as u16) << shift
    };
    chan(11, 31) | chan(5, 63) | chan(0, 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_with_depth;
    use crate::world::World;

    fn coast_camera() -> Camera {
        Camera {
            x: 217.04001,
            z: -78.32,
            altitude: 58.0,
            heading: 0.64,
            pitch: 0.105,
            bank: 0.04,
        }
    }

    #[test]
    fn unsuitable_weather_has_no_site_work() {
        let seed = 19;
        let ticks = 1200;
        let mut weather = WeatherEffects::new();
        let storm = Sky::at(seed, ticks);
        let camera = coast_camera();
        for sky in [
            Sky {
                cloud_cover: 0.35,
                ..storm
            },
            Sky {
                haze: 0.18,
                ..storm
            },
            Sky {
                daylight: 0.06,
                ..storm
            },
        ] {
            assert!(weather.visible(seed, ticks, camera, &sky).is_none());
            assert_eq!(weather.region, None);
        }
        let high = Camera {
            altitude: 255.0,
            ..camera
        };
        assert!(weather.visible(seed, ticks, high, &storm).is_none());
        assert_eq!(weather.region, None);
        assert!(weather.visible(seed, ticks, camera, &storm).is_some());
    }

    #[test]
    fn cell_changes_preserve_the_shaft_and_wind_advances_continuously() {
        let seed = 76;
        let (x, site) = (-7..7)
            .find_map(|x| Site::at(seed, x, -2).map(|site| (x, site)))
            .unwrap();
        let boundary = x as f32 * CELL;
        let camera = Camera {
            x: boundary - 0.01,
            z: site.center[1],
            altitude: 63.0,
            heading: core::f32::consts::FRAC_PI_2,
            pitch: 0.08,
            bank: 0.0,
        };
        let tick = u64::from(site.period - site.phase + site.life / 2);
        let climate = |tick| Sky {
            cloud_cover: 0.82,
            haze: 0.35,
            ..Sky::from_time(seed, tick, 0.5)
        };
        let mut weather = WeatherEffects::new();
        let before = weather.visible(seed, tick, camera, &climate(tick)).unwrap();
        let after = Camera {
            x: boundary + 0.01,
            ..camera
        };
        let crossed = weather.visible(seed, tick, after, &climate(tick)).unwrap();
        assert_eq!(before.top, crossed.top);
        assert_eq!(before.foot, crossed.foot);
        assert_eq!(before.half_width, crossed.half_width);
        assert!((before.opacity - crossed.opacity).abs() < 0.001);
        let next = weather
            .visible(seed, tick + 1, after, &climate(tick + 1))
            .unwrap();
        assert!(crossed
            .top
            .iter()
            .zip(next.top)
            .all(|(a, b)| (a - b).abs() < 0.025));
        assert!((crossed.opacity - next.opacity).abs() < 0.001);
        assert_eq!(
            WeatherEffects::new().visible(seed, tick, after, &climate(tick)),
            Some(crossed)
        );
    }

    #[test]
    fn a_shower_advects_from_a_distant_curtain_into_local_visibility() {
        let seed = 19;
        let ticks = 1200;
        let sky = Sky::at(seed, ticks);
        let distant = coast_camera();
        let mut effects = WeatherEffects::new();
        let far = effects.observe(seed, ticks, distant, &sky);
        assert_eq!(far.shower, 0.0);
        let plume = far.distant_rain.unwrap();
        let mut previous = 0.0;
        let [foot_x, _, foot_z] = plume.foot;
        let [top_x, _, top_z] = plume.top;
        let mut nearest = distant;
        for length in (0..=180).rev() {
            let x = foot_x * 0.56 + top_x * 0.44 + length as f32;
            nearest = Camera { x, z: foot_z * 0.56 + top_z * 0.44, ..distant };
            let inside = effects.observe(seed, ticks, nearest, &sky).shower;
            assert!((inside - previous).abs() < 0.06);
            previous = inside;
        }
        assert!(previous > 0.38);
        assert!(sky.with_shower(previous).haze > sky.haze + 0.12);
        let above = Camera { altitude: 240.0, ..nearest };
        assert_eq!(effects.observe(seed, ticks, above, &sky).shower, 0.0);
    }

    #[test]
    fn native_rain_respects_depth_and_the_round_aperture() {
        const SIDE: usize = 112;
        const PIXELS: usize = SIDE * SIDE;
        let seed = 19;
        let ticks = 1200;
        let camera = coast_camera();
        let sky = Sky::at(seed, ticks);
        let world = World::new(seed);
        let rain = WeatherEffects::new()
            .visible(seed, ticks, camera, &sky)
            .unwrap();
        let mut pixels = [0; PIXELS];
        let mut work = [0; PIXELS];
        let mut depth = [u16::MAX; PIXELS];
        render_with_depth(
            |x, z| world.sample(x, z),
            camera,
            &sky,
            &mut pixels,
            &mut work,
            &mut depth,
            (SIDE, SIDE),
        )
        .unwrap();
        let base = pixels;
        rain.paint(camera, (SIDE, SIDE), &mut pixels, &depth);
        let mut changed = 0;
        for (index, (&before, after)) in base.iter().zip(pixels).enumerate() {
            if before == after {
                continue;
            }
            let sx = (2 * (index % SIDE) + 1) as i32 - SIDE as i32;
            let sy = (2 * (index / SIDE) + 1) as i32 - SIDE as i32;
            assert!(sx * sx + sy * sy <= (SIDE * SIDE) as i32);
            assert!(depth[index] > encoded_depth(NEAR));
            changed += 1;
        }
        assert!(
            (280..600).contains(&changed),
            "rain changed {changed} pixels"
        );
        pixels = base;
        depth.fill(0);
        rain.paint(camera, (SIDE, SIDE), &mut pixels, &depth);
        assert_eq!(pixels, base);
    }

    #[test]
    fn the_rainbow_spectrum_has_red_toward_the_outer_edge() {
        let anti = [0.0, 0.0, 1.0];
        assert!(spectrum(anti, 0.2, 0.0).is_none());
        assert!(spectrum(anti, 1.1, 0.0).is_none());
        let (outer, _) = spectrum(anti, 0.891, 0.0).unwrap();
        let (inner, _) = spectrum(anti, 0.827, 0.0).unwrap();
        assert!((outer >> 11) > (inner >> 11));
        assert!((inner & 31) > (outer & 31));
    }

    #[test]
    fn regional_rainbow_renders_only_where_the_shower_is_visible() {
        let seed = 13;
        let ticks = 15_390;
        let camera = Camera {
            x: 1004.0, z: -1332.0, altitude: 58.0,
            heading: 2.923976, pitch: 0.04, bank: 0.0,
        };
        let sky = Sky::at_position(seed, ticks, [camera.x, camera.altitude, camera.z]);
        let event = WeatherEffects::new().observe(seed, ticks, camera, &sky).distant_rain.unwrap();
        assert!(event.rainbow.is_some());
        let dimensions = (112, 112);
        let world = World::new(seed);
        let mut base = [0; 112 * 112];
        let mut scratch = [0; 112 * 112];
        let mut depth = [0; 112 * 112];
        render_with_depth(
            |x, z| world.sample(x, z), camera, &sky, &mut base, &mut scratch, &mut depth, dimensions,
        ).unwrap();
        let mut shower = base;
        let mut with_bow = base;
        RainShaft { rainbow: None, ..event }.paint(camera, dimensions, &mut shower, &depth);
        event.paint(camera, dimensions, &mut with_bow, &depth);
        let mut colored = 0;
        for (index, (&with, &without)) in with_bow.iter().zip(&shower).enumerate() {
            if with != without {
                let x = 2 * (index % 112) as i32 + 1 - 112;
                let y = 2 * (index / 112) as i32 + 1 - 112;
                assert!(x * x + y * y <= 112 * 112);
                assert!(depth[index] > encoded_depth(120.0));
                colored += 1;
            }
        }
        assert!(colored > 35, "{colored} rainbow pixels");
        depth.fill(0);
        let mut hidden = base;
        event.paint(camera, dimensions, &mut hidden, &depth);
        assert_eq!(hidden, base);
    }
}
