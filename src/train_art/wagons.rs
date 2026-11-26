use super::freight_catalog::{Family, design};

pub(super) fn draw(
    point: impl Fn(f32, f32, f32) -> [f32; 3],
    face: impl FnMut([[f32; 3]; 4], [u8; 3]),
    color: [u8; 3],
    variant: u8,
    detail: bool,
    loaded: bool,
) {
    let variant = design(variant);
    let job = usize::from(variant.job);
    let build = usize::from(variant.build);
    let mut mesh = WagonMesh {
        point,
        face,
        detail,
        loaded,
    };
    match variant.family {
        Family::Well => mesh.well(job, build, color),
        Family::Boxcar => mesh.boxcar(job, build, color),
        Family::CoveredHopper => mesh.covered_hopper(job, build),
        Family::Tank => mesh.tank(job, build),
        Family::Deck => mesh.flat(job, build),
        Family::Autorack => mesh.autorack(job, build),
        Family::OpenBulk => mesh.open_bulk(job, build),
        Family::Centerbeam => mesh.timber(job, build),
        Family::Coil => mesh.coil(job, build),
        Family::Reefer => mesh.reefer(job, build),
    }
}

const DECK: f32 = 1.10;
const FRAME: [u8; 3] = [62, 68, 67];
const WHEEL: [u8; 3] = [43, 47, 46];
const TIMBER: [u8; 3] = [130, 104, 76];

struct WagonMesh<P, F> {
    point: P,
    face: F,
    detail: bool,
    loaded: bool,
}

impl<P, F> WagonMesh<P, F>
where
    P: Fn(f32, f32, f32) -> [f32; 3],
    F: FnMut([[f32; 3]; 4], [u8; 3]),
{
    fn quad(&mut self, corners: [[f32; 3]; 4], color: [u8; 3]) {
        (self.face)(corners.map(|[t, s, h]| (self.point)(t, s, h)), color);
    }

    fn side(&mut self, [a, b]: [f32; 2], s: f32, [bottom, top]: [f32; 2], color: [u8; 3]) {
        self.quad(
            [[a, s, bottom], [b, s, bottom], [b, s, top], [a, s, top]],
            color,
        );
    }

    fn sides(&mut self, span: [f32; 2], width: f32, levels: [f32; 2], color: [u8; 3]) {
        self.side(span, width, levels, color);
        self.side(span, -width, levels, tint(color, 0.83));
    }

    fn end(&mut self, t: f32, width: f32, [bottom, top]: [f32; 2], color: [u8; 3]) {
        self.quad(
            [
                [t, -width, bottom],
                [t, width, bottom],
                [t, width, top],
                [t, -width, top],
            ],
            color,
        );
    }

    fn floor(&mut self, [a, b]: [f32; 2], [l, r]: [f32; 2], h: f32, color: [u8; 3]) {
        self.quad([[a, l, h], [b, l, h], [b, r, h], [a, r, h]], color);
    }

    fn block(
        &mut self,
        span: [f32; 2],
        lateral: [f32; 2],
        [bottom, top]: [f32; 2],
        color: [u8; 3],
    ) {
        let [a, b] = span;
        let [l, r] = lateral;
        self.floor(span, lateral, top, tint(color, 1.16));
        self.side(span, l, [bottom, top], tint(color, 0.80));
        self.side(span, r, [bottom, top], color);
        for t in [a, b] {
            self.quad(
                [[t, l, bottom], [t, r, bottom], [t, r, top], [t, l, top]],
                tint(color, 0.72),
            );
        }
    }

    fn cap(&mut self, t: f32, profile: [[f32; 2]; 6], color: [u8; 3]) {
        let [a, b, c, d, e, f] = profile.map(|[s, h]| [t, s, h]);
        self.quad([a, b, c, f], color);
        self.quad([f, c, d, e], tint(color, 0.91));
    }

    /// Six-sided transverse profile extruded between the car ends.
    fn body(
        &mut self,
        [a, b]: [f32; 2],
        width: f32,
        [bottom, top]: [f32; 2],
        [roof, flank, base]: [f32; 3],
        color: [u8; 3],
    ) {
        let shoulder = top - flank;
        let profile = [
            [-width * roof, top],
            [width * roof, top],
            [width, shoulder],
            [width * base, bottom],
            [-width * base, bottom],
            [-width, shoulder],
        ];
        self.join_profiles(a, b, profile, profile, color);
        self.cap(a, profile, tint(color, 0.73));
        self.cap(b, profile, tint(color, 0.80));
    }

    fn join_profiles(
        &mut self,
        a: f32,
        b: f32,
        from: [[f32; 2]; 6],
        to: [[f32; 2]; 6],
        color: [u8; 3],
    ) {
        for (i, shade) in [1.16, 1.01, 0.88, 0.0, 0.71, 0.96].into_iter().enumerate() {
            if shade == 0.0 {
                continue;
            }
            let j = (i + 1) % 6;
            let ([ls, lh], [rs, rh]) = (from[i], from[j]);
            let ([ls2, lh2], [rs2, rh2]) = (to[i], to[j]);
            self.quad(
                [[a, ls, lh], [b, ls2, lh2], [b, rs2, rh2], [a, rs, rh]],
                tint(color, shade),
            );
        }
    }

    fn frame(&mut self, span: [f32; 2], width: f32, deck: f32, trucks: f32) {
        self.sides(span, width, [deck - 0.35, deck], FRAME);
        self.sides([0.0, 1.0], 0.31, [0.66, 0.84], FRAME);
        if self.detail {
            for center in [trucks, 1.0 - trucks] {
                self.sides([center - 0.068, center + 0.068], 0.87, [0.18, 0.71], WHEEL);
            }
        }
    }

    fn bulkheads(&mut self, [a, b]: [f32; 2], width: f32, [base, top]: [f32; 2], color: [u8; 3]) {
        self.block([a, a + 0.038], [-width, width], [base, top], color);
        self.block([b - 0.038, b], [-width, width], [base, top], color);
    }

    fn well(&mut self, job: usize, build: usize, color: [u8; 3]) {
        let [a, b] = [
            [0.19, 0.81],
            [0.145, 0.855],
            [0.09, 0.91],
            [0.066, 0.934],
            [0.115, 0.885],
        ][build];
        let well = [0.62, 0.69, 0.74, 0.62, 0.83][build];
        let beam = [0.98, 1.13, 1.04, 1.09, 1.26][build];
        let steel = [124, 84, 59];
        self.frame([a, b], 1.37, beam, [0.13, 0.12, 0.12, 0.097, 0.17][build]);
        if build == 3 {
            // Open framing still has crossbearers under the corner castings.
            for rail in [[-1.31, -1.0], [1.0, 1.31]] {
                self.floor([a, b], rail, well, FRAME);
            }
            for t in [a + 0.023, 0.47, b - 0.060] {
                self.floor([t, t + 0.036], [-1.17, 1.17], well, steel);
            }
        } else {
            self.floor([a, b], [-1.31, 1.31], well, [74, 64, 57]);
        }
        let bolster = [1.14, 1.22, 1.01, 1.11, 1.39][build];
        self.block([0.031, a], [-1.28, 1.28], [0.61, bolster], steel);
        self.block([b, 0.969], [-1.28, 1.28], [0.61, bolster], steel);
        if build == 1 && job == 0 {
            // An unloaded single well exposes its trailer hitch.
            self.block([0.255, 0.344], [-0.37, 0.37], [well, 1.28], [122, 122, 110]);
        }
        if build == 4 {
            self.floor([a, b], [-1.37, -1.16], beam, [163, 100, 65]);
            self.floor([a, b], [1.16, 1.37], beam, [163, 100, 65]);
        }

        if !self.loaded { return; }
        let cargo = [a + 0.014, b - 0.014];
        let end = a + (b - a) * 0.48;
        let top = well + 1.43;
        match job {
            0 => {}
            1 => {
                let short = [a + 0.018, end];
                let placement = [0.0, 0.11, 0.18, 0.02, 0.14][build];
                self.container(short.map(|t| t + placement), [well, top], color);
            }
            2 => {
                self.container([cargo[0], 0.49], [well, top], color);
                self.container([0.51, cargo[1]], [well, top + 0.12], [87, 133, 133]);
            }
            3 => self.container(cargo, [well, top + 0.22], color),
            _ => {
                self.container(cargo, [well, top], color);
                let upper = [
                    [0.20, 0.80],
                    [0.19, 0.81],
                    [0.13, 0.87],
                    [0.10, 0.90],
                    [0.15, 0.85],
                ][build];
                self.container(upper, [top, top + 1.39], [91, 151, 151]);
            }
        }
    }

    fn container(&mut self, span: [f32; 2], levels: [f32; 2], color: [u8; 3]) {
        self.block(span, [-1.19, 1.19], levels, color);
        if self.detail {
            let [a, b] = span;
            let [bottom, top] = levels;
            for n in 1..5 {
                let t = a + (b - a) * n as f32 * 0.2;
                self.sides(
                    [t, t + 0.012],
                    1.20,
                    [bottom + 0.13, top - 0.11],
                    tint(color, 0.76),
                );
            }
        }
    }

    fn boxcar(&mut self, job: usize, build: usize, livery: [u8; 3]) {
        let body = [
            [0.10, 0.90],
            [0.055, 0.945],
            [0.15, 0.85],
            [0.075, 0.925],
            [0.035, 0.965],
        ][build];
        let width = [1.20, 1.23, 1.16, 1.25, 1.29][build];
        let rise = [3.37, 3.27, 3.08, 3.43, 3.70][build];
        let lumber_clearance = f32::from(u8::from(job == 4 && build == 2)) * 0.23;
        let height = rise + [0.04, -0.09, 0.10, -0.21, 0.14][job] + lumber_clearance;
        let tint_job = [
            [144, 79, 59],
            [92, 126, 134],
            [156, 126, 68],
            [120, 108, 84],
            [95, 106, 77],
        ][job];
        let color = mix(livery, tint_job);
        self.frame(body, width, DECK, body[0] + 0.079);
        self.body(
            body,
            width,
            [DECK, height],
            [[0.66, 0.57, 0.84, 0.66, 0.91][build], 0.24, 1.0],
            color,
        );

        let center = [0.50, 0.48, 0.53, 0.56, 0.47][job] + if build == 3 { 0.085 } else { 0.0 };
        let half = [0.096, 0.19, 0.105, 0.12, 0.14][build] + [0.01, 0.0, 0.03, -0.02, 0.022][job];
        let levels = [DECK + 0.13, height - 0.31];
        self.sides(
            [center - half, center + half],
            width + 0.014,
            levels,
            tint(color, 0.62),
        );
        match build {
            0 | 1 => {
                // The long top track guides the sliding door clear of its opening.
                self.sides(
                    [center - half - 0.09, center + half + 0.03],
                    width + 0.020,
                    [levels[1], levels[1] + 0.075],
                    tint(color, 0.72),
                );
                if build == 1 {
                    self.sides(
                        [center - 0.017, center + 0.017],
                        width + 0.027,
                        levels,
                        tint(color, 1.24),
                    );
                }
            }
            2 | 3 => {
                // Plug doors close against a gasket. Their latch posts are exposed.
                let latch = center + [0.0, 0.0, -0.059, 0.075, 0.0][build];
                self.sides(
                    [latch - 0.018, latch + 0.018],
                    width + 0.037,
                    levels,
                    tint(color, 1.28),
                );
                self.sides(
                    [center - half, center + half],
                    width + 0.030,
                    [levels[0], levels[0] + 0.11],
                    tint(color, 0.47),
                );
            }
            _ => {
                self.sides(
                    [center - 0.015, center + 0.015],
                    width + 0.026,
                    levels,
                    tint(color, 1.26),
                );
                self.floor(
                    [body[0] + 0.03, body[1] - 0.03],
                    [-0.73, 0.73],
                    height + 0.022,
                    tint(color, 0.68),
                );
            }
        }
        if self.detail {
            let [a, b] = body;
            for n in 1..6 {
                let t = a + (b - a) * n as f32 / 6.0;
                if (t - center).abs() > half + 0.012 {
                    self.sides([t, t + 0.014], width + 0.021, levels, tint(color, 1.18));
                }
            }
        }
    }

    fn covered_hopper(&mut self, job: usize, build: usize) {
        let span = [
            [0.17, 0.83],
            [0.115, 0.885],
            [0.071, 0.929],
            [0.105, 0.895],
            [0.061, 0.939],
        ][build];
        // Dense cement and sand fill their weight allowance in shorter bodies.
        let loading_space = [0.008, 0.048, -0.018, 0.031, 0.022][job];
        let span = [span[0] + loading_space, span[1] - loading_space];
        let bays = [
            [2, 3, 4, 3, 4], // Grain
            [2, 2, 3, 2, 3], // Cement
            [3, 3, 4, 3, 5], // Pellets
            [2, 3, 3, 2, 3], // Sand
            [2, 3, 4, 3, 4], // Sugar
        ][job][build];
        let body = [3.52, 2.97, 3.60, 3.12, 3.38][job] + [-0.14, -0.04, 0.0, 0.13, 0.19][build];
        let width = [1.26, 1.12, 1.23, 1.17, 1.28][job];
        let color = [
            [176, 165, 128],
            [158, 163, 159],
            [189, 196, 199],
            [142, 124, 100],
            [202, 185, 158],
        ][job];
        self.frame(span, 0.91, 0.91, span[0] + 0.037);
        let discharge_top = [1.71, 1.76, 1.44, 1.93, 1.58][build];
        let taper = [0.87, 0.89, 0.60, 0.88, 0.72][build];
        let shoulder = [0.22, 0.18, 0.48, 0.23, 0.31][build];
        self.body(
            span,
            width,
            [discharge_top, body],
            [[0.64, 0.83, 0.53, 0.70, 0.79][job], shoulder, taper],
            color,
        );
        let pitch = (span[1] - span[0]) / bays as f32;
        let chute = [0.96, 0.92, 1.08, 0.81, 1.03][job] - if build == 3 { 0.15 } else { 0.0 };
        for bay in 0..bays {
            let start = span[0] + pitch * bay as f32;
            self.discharge(
                [start + 0.01, start + pitch - 0.01],
                width * taper,
                discharge_top + 0.025,
                chute,
                color,
            );
        }
        if job == 0 {
            self.floor(
                [span[0] + 0.018, span[1] - 0.018],
                [-0.33, 0.33],
                body + 0.025,
                tint(color, 0.65),
            );
        } else {
            for n in 0..bays {
                let t = span[0] + pitch * (n as f32 + 0.29);
                let hatch = if job == 2 {
                    [-0.26, 0.26]
                } else {
                    [-0.37, 0.37]
                };
                self.block(
                    [t, t + pitch * 0.39],
                    hatch,
                    [body - 0.021, body + 0.08],
                    tint(color, 0.74),
                );
            }
        }
        if job == 1 {
            // Low air-assist manifold links the dense powder discharge gates.
            let [a, b] = span;
            self.sides([a + 0.016, b - 0.016], 0.67, [chute + 0.10, chute + 0.20], tint(color, 0.61));
            if self.detail {
                for n in 0..bays {
                    let joint = a + pitch * (n as f32 + 0.48);
                    self.sides([joint, joint + 0.022], 0.70, [chute - 0.020, chute + 0.27], tint(color, 0.72));
                }
            }
        }
        match build {
            1 => {
                // External ribs carry the deeper sides of this gravity hopper.
                for n in 1..bays {
                    let t = span[0] + n as f32 * pitch;
                    for s in [-1.0, 1.0] {
                        self.quad(
                            [
                                [t - 0.025, s * width * taper, discharge_top],
                                [t + 0.025, s * width * taper, discharge_top],
                                [t + 0.016, s * (width + 0.035), body - shoulder],
                                [t - 0.016, s * (width + 0.035), body - shoulder],
                            ],
                            tint(color, 0.62),
                        );
                    }
                }
            }
            3 => {
                self.sides(
                    [span[0] - 0.031, span[1] + 0.031],
                    0.76,
                    [chute + 0.11, chute + 0.23],
                    tint(color, 0.60),
                );
            }
            4 => {
                // Long outside walkways provide access to multiple roof hatches.
                for tread in [[-0.79, -0.52], [0.52, 0.79]] {
                    self.floor(
                        [span[0] + 0.033, span[1] - 0.033],
                        tread,
                        body + 0.036,
                        tint(color, 0.68),
                    );
                }
            }
            _ => {}
        }
    }

    fn discharge(&mut self, [a, b]: [f32; 2], w: f32, h: f32, foot: f32, color: [u8; 3]) {
        let step = (b - a) * 0.34;
        for s in [-w, w] {
            self.quad(
                [
                    [a, s, h],
                    [b, s, h],
                    [b - step, s * 0.36, foot],
                    [a + step, s * 0.36, foot],
                ],
                tint(color, if s > 0.0 { 0.91 } else { 0.66 }),
            );
        }
        if self.detail {
            self.quad(
                [
                    [a, -w, h],
                    [a, w, h],
                    [a + step, w * 0.36, foot],
                    [a + step, -w * 0.36, foot],
                ],
                tint(color, 0.72),
            );
        }
    }

    fn tank(&mut self, job: usize, build: usize) {
        let [a, b] = [
            [0.19, 0.81],
            [0.13, 0.87],
            [0.073, 0.927],
            [0.106, 0.894],
            [0.079, 0.921],
        ][build];
        // Thick-jacket asphalt and short water tanks carry dense commodities.
        let loading_space = [0.012, 0.045, -0.025, -0.012, 0.073][job];
        let short_asphalt = f32::from(u8::from(job == 1 && build == 1)) * 0.044;
        let [a, b] = [a + loading_space + short_asphalt, b - loading_space - short_asphalt];
        let heat_jacket = f32::from(u8::from(job == 1 && build == 2)) * 0.12;
        let radius = [1.08, 1.16, 1.14, 1.10, 1.19][job] + [-0.14, -0.04, 0.00, 0.095, 0.16][build] + heat_jacket;
        let center = [2.20, 2.18, 2.24, 2.31, 2.13][job];
        let height = center + radius;
        let short = [0.066, 0.037, 0.053, 0.089, 0.042][job];
        let color = [
            [183, 182, 165],
            [82, 76, 65],
            [115, 137, 140],
            [187, 193, 188],
            [130, 141, 157],
        ][job];
        let profile = |factor: f32| {
            let r = radius * factor;
            [
                [-r * 0.49, center + r],
                [r * 0.49, center + r],
                [r, center + r * 0.20],
                [r * 0.61, center - r * 0.80],
                [-r * 0.61, center - r * 0.80],
                [-r, center + r * 0.20],
            ]
        };
        let middle = profile(1.0);
        let cap = profile([0.61, 0.76, 0.67, 0.49, 0.72][job]);
        self.frame([a, b], 0.88, 0.93, a.clamp(0.083, 0.17));
        self.join_profiles(a, a + short, cap, middle, color);
        self.join_profiles(a + short, b - short, middle, middle, color);
        self.join_profiles(b - short, b, middle, cap, color);
        self.cap(a, cap, tint(color, 0.77));
        self.cap(b, cap, tint(color, 0.77));

        let hatch = [0.42, 0.54, 0.48, 0.44, 0.32][job];
        let stem = if job == 3 { 0.32 } else { 0.19 };
        self.block(
            [hatch, hatch + if job == 3 { 0.15 } else { 0.092 }],
            [-0.30, 0.30],
            [height, (height + stem).min(3.86)],
            tint(color, 0.72),
        );
        match build {
            0 => self.floor(
                [hatch - 0.035, hatch + 0.15],
                [-0.47, 0.47],
                height + 0.021,
                tint(color, 0.57),
            ),
            2 => self.floor(
                [hatch - 0.077, hatch + 0.20],
                [-0.44, 0.44],
                height + 0.050,
                tint(color, 0.58),
            ),
            4 => {
                let access = [hatch - 0.10, hatch + 0.23];
                self.floor(access, [-0.69, 0.69], height + 0.068, tint(color, 0.54));
                self.sides(
                    [hatch - 0.095, hatch + 0.229],
                    0.68,
                    [height + 0.15, (height + 0.22).min(3.86)],
                    tint(color, 0.60),
                );
            }
            _ => {}
        }
        if build == 3 || job == 1 {
            for mount in [a + short + 0.071, b - short - 0.071] {
                self.join_profiles(
                    mount,
                    mount + 0.026,
                    profile(1.013),
                    profile(1.013),
                    tint(color, 0.66),
                );
            }
        }
        if job == 1 {
            // Insulated asphalt tanks carry low heating connections.
            self.sides([a + short + 0.018, b - short - 0.018], 0.75, [1.04, 1.17], [136, 125, 102]);
            self.block([0.47, 0.55], [-0.37, 0.37], [0.92, 1.20], [95, 91, 78]);
        }
        if self.detail && job == 4 {
            self.block(
                [0.64, 0.718],
                [-0.27, 0.27],
                [height, height + 0.15],
                tint(color, 0.68),
            );
        }
    }

    fn flat(&mut self, job: usize, build: usize) {
        let body = [
            [0.072, 0.928],
            [0.052, 0.948],
            [0.105, 0.895],
            [0.071, 0.929],
            [0.039, 0.961],
        ][build];
        let deck = [1.00, 1.19, 1.12, 0.72, 1.34][build];
        let width = [1.27, 1.33, 1.22, 1.25, 1.38][build];
        self.frame(body, width, deck, [0.13, 0.12, 0.17, 0.124, 0.105][build]);
        self.floor(body, [-width, width], deck, TIMBER);
        if build == 1 || job == 0 || job == 3 {
            self.bulkheads(
                body,
                width,
                [deck, deck + [0.70, 1.25, 0.91, 0.84, 1.20][build]],
                [144, 103, 57],
            );
        }
        if build == 2 || build == 3 {
            self.block([body[0], 0.22], [-width, width], [deck - 0.03, 1.43], FRAME);
            self.block([0.78, body[1]], [-width, width], [deck - 0.03, 1.43], FRAME);
        }
        let rings = [0.24, 0.48, 0.73];
        if build == 0 || job == 4 {
            // Tall log stakes and short pipe stops retain round loads.
            let h = deck + [0.30, 0.69, 0.29, 0.42, 1.00][job];
            for t in rings {
                self.sides([t, t + 0.025], width - 0.09, [deck, h], [112, 93, 64]);
            }
        } else if build == 4 {
            self.sides(
                [0.14, 0.86],
                width + 0.018,
                [deck - 0.39, deck - 0.25],
                [90, 98, 92],
            );
        }

        if !self.loaded { return; }
        let start = body[0].max(0.14) + 0.045;
        let tail = body[1].min(0.89) - 0.035;
        match job {
            0 => {}
            1 => self.tubes(
                [start, tail],
                deck,
                [94, 128, 141],
                if build % 2 == 0 { 2 } else { 3 },
                true,
            ),
            2 => self.machinery(build, deck),
            3 => {
                // I-beam webs and wider flange caps remain readable from above the track.
                let flange = deck + [0.93, 1.14, 0.88, 1.06, 1.09][build];
                for side in [-0.72, 0.43] {
                    self.block(
                        [start, tail],
                        [side, side + 0.21],
                        [deck + 0.08, flange],
                        [112, 126, 144],
                    );
                    self.floor(
                        [start, tail],
                        [side - 0.145, side + 0.355],
                        flange + 0.025,
                        [141, 154, 164],
                    );
                }
                if build >= 2 {
                    self.block(
                        [start + 0.08, tail - 0.04],
                        [-0.22, 0.22],
                        [deck, deck + 0.72],
                        [94, 105, 125],
                    );
                }
            }
            _ => {
                self.tubes([start, tail], deck, [135, 91, 55], 3, false);
                self.tubes(
                    [start + 0.042, tail - 0.062],
                    deck + 0.46,
                    [160, 111, 72],
                    if build == 4 { 3 } else { 2 },
                    false,
                );
                for t in [start, tail - 0.026] {
                    self.sides([t, t + 0.025], 1.14, [deck, deck + 0.99], [111, 93, 66]);
                }
            }
        }
    }

    #[inline(never)]
    fn machinery(&mut self, build: usize, deck: f32) {
        let yellow = [204, 153, 54];
        let dark = [56, 58, 54];
        let glass = [88, 122, 130];
        match build {
            0 => {
                // Excavator tracks, rotating house and lowered two-piece boom.
                self.block([0.26, 0.54], [-0.86, 0.86], [deck, deck + 0.45], dark);
                self.block([0.28, 0.54], [-0.70, 0.70], [deck + 0.45, deck + 1.04], yellow);
                self.block([0.34, 0.48], [-0.62, -0.03], [deck + 1.03, deck + 1.70], glass);
                self.quad([[0.44, 0.28, deck + 0.97], [0.60, 0.28, deck + 1.61],
                           [0.77, 0.28, deck + 0.74], [0.60, 0.28, deck + 1.35]], yellow);
                self.block([0.72, 0.80], [0.01, 0.52], [deck + 0.07, deck + 0.75], tint(yellow, 0.71));
            }
            1 => {
                // Wheel loader with separate axles and a broad lowered bucket.
                for t in [0.31, 0.55] {
                    self.sides([t - 0.053, t + 0.053], 0.93, [deck, deck + 0.57], dark);
                }
                self.block([0.25, 0.63], [-0.70, 0.70], [deck + 0.41, deck + 0.94], yellow);
                self.block([0.42, 0.52], [-0.58, 0.58], [deck + 0.94, deck + 1.63], glass);
                self.block([0.63, 0.78], [-1.01, 1.01], [deck + 0.12, deck + 0.57], [147, 114, 66]);
            }
            2 => {
                // Enclosed generator with end intake, exhaust and transport skids.
                let casing = [147, 175, 157];
                self.block([0.20, 0.78], [-0.98, 0.98], [deck, deck + 0.26], dark);
                self.body([0.25, 0.73], 0.83, [deck + 0.23, deck + 1.57], [0.86, 0.23, 0.94], casing);
                self.end(0.733, 0.62, [deck + 0.46, deck + 1.24], [70, 92, 88]);
                self.block([0.39, 0.55], [-0.36, 0.36], [deck + 1.57, deck + 1.75], tint(casing, 0.65));
            }
            3 => {
                // Compact crawler with its blade dropped for the move.
                self.block([0.31, 0.66], [-0.98, 0.98], [deck, deck + 0.44], dark);
                self.block([0.29, 0.63], [-0.72, 0.72], [deck + 0.43, deck + 0.98], yellow);
                self.block([0.48, 0.62], [-0.53, 0.53], [deck + 0.98, deck + 1.54], glass);
                self.block([0.22, 0.28], [-1.16, 1.16], [deck + 0.04, deck + 0.79], tint(yellow, 0.82));
            }
            _ => {
                // Heavy transformer with side radiators and raised bushings.
                let casing = [133, 151, 149];
                self.block([0.23, 0.76], [-0.67, 0.67], [deck, deck + 1.36], casing);
                self.sides([0.30, 0.69], 0.98, [deck + 0.16, deck + 1.05], tint(casing, 0.64));
                for t in [0.35, 0.51, 0.65] {
                    self.block([t, t + 0.025], [-0.23, 0.23], [deck + 1.35, deck + 1.81], [204, 189, 144]);
                }
            }
        }
    }

    fn tubes(&mut self, span: [f32; 2], h: f32, color: [u8; 3], count: usize, hollow: bool) {
        for n in 0..count {
            let center = (n as f32 - (count - 1) as f32 * 0.5) * 0.67;
            let [a, b] = span;
            let section = [
                [center - 0.18, h + 0.54],
                [center + 0.18, h + 0.54],
                [center + 0.30, h + 0.34],
                [center + 0.21, h + 0.13],
                [center - 0.21, h + 0.13],
                [center - 0.30, h + 0.34],
            ];
            self.join_profiles(a, b, section, section, color);
            if hollow {
                let bore = section.map(|[s, y]| [center + (s - center) * 0.72, h + 0.34 + (y - h - 0.34) * 0.72]);
                for t in [a, b] {
                    for edge in 0..section.len() {
                        let next = (edge + 1) % section.len();
                        let [s0, h0] = section[edge];
                        let [s1, h1] = section[next];
                        let [u0, y0] = bore[edge];
                        let [u1, y1] = bore[next];
                        self.quad([[t, s0, h0], [t, s1, h1], [t, u1, y1], [t, u0, y0]], tint(color, 0.83));
                    }
                    let recessed = if t == a { t + 0.014 } else { t - 0.014 };
                    self.cap(recessed, bore, [44, 58, 65]);
                }
            } else {
                self.cap(b, section, tint(color, 0.64));
            }
        }
    }

    fn autorack(&mut self, job: usize, build: usize) {
        let body = [
            [0.147, 0.853],
            [0.066, 0.934],
            [0.025, 0.975],
            [0.053, 0.947],
            [0.048, 0.952],
        ][build];
        let overhang = [0.0, 0.013, -0.019, -0.014, 0.058][job];
        let body = [body[0] + overhang, body[1] - overhang];
        // Tall vans ride one deck, and the articulated carriers are bi-level.
        let levels = [2, 2, 3, 2, 1][job];
        let width = [1.17, 1.27, 1.23, 1.31, 1.36][build];
        let top = ([3.24_f32, 3.61, 3.65, 3.60, 3.83][job]
            + [-0.07, -0.06, 0.0, 0.078, 0.079][build])
            .min(3.87_f32);
        let steel = [
            [159, 128, 89],
            [182, 168, 109],
            [161, 160, 137],
            [156, 151, 118],
            [181, 158, 89],
        ][job];
        let shelf = (top - DECK) / levels as f32;
        self.frame(body, width, DECK, body[0] + 0.066);
        if job == 0 {
            self.floor(body, [-width, width], DECK, FRAME);
            self.floor(body, [-width, width], DECK + shelf, steel);
            for n in 0..levels * usize::from(self.loaded) {
                let h = DECK + n as f32 * shelf;
                let load = if n == 0 {
                    [114, 142, 155]
                } else {
                    [158, 92, 70]
                };
                for car in [0.18, 0.50, 0.82] {
                    let length = body[1] - body[0];
                    let t = body[0] + length * car;
                    let hood = [t - length * 0.117, t + length * 0.117];
                    self.block(hood, [-0.87, 0.87], [h, h + 0.31], load);
                    if self.detail {
                        self.block(
                            [t - 0.043, t + 0.048],
                            [-0.66, 0.66],
                            [h + 0.31, h + 0.62],
                            tint(load, 0.80),
                        );
                    } else {
                        self.floor(
                            [t - 0.042, t + 0.048],
                            [-0.66, 0.66],
                            h + 0.322,
                            tint(load, 0.60),
                        );
                    }
                }
            }
        } else if job == 3 {
            self.body(
                [body[0], 0.463],
                width,
                [DECK, top],
                [0.85, 0.26, 1.0],
                steel,
            );
            self.body(
                [0.537, body[1]],
                width,
                [DECK, top],
                [0.85, 0.26, 1.0],
                steel,
            );
            self.floor([0.457, 0.543], [-0.91, 0.91], DECK + 0.011, FRAME);
        } else {
            let chamfer = f32::from(u8::from(job == 1 && build == 1));
            self.body(
                body,
                width,
                [DECK, top],
                [
                    [0.70, 0.78, 0.87, 0.92, 0.95][build] - chamfer * 0.25,
                    [0.24, 0.24, 0.40, 0.24, 0.18][job] + chamfer * 0.20,
                    1.0,
                ],
                steel,
            );
        }
        let [a, b] = body;
        for n in 0..levels {
            let h = DECK + 0.17 + n as f32 * shelf;
            if job == 3 {
                self.sides(
                    [a + 0.02, 0.46],
                    width + 0.015,
                    [h, h + 0.12],
                    tint(steel, 0.57),
                );
                self.sides(
                    [0.54, b - 0.02],
                    width + 0.015,
                    [h, h + 0.12],
                    tint(steel, 0.57),
                );
            } else {
                self.sides(
                    [a + 0.013, b - 0.013],
                    width + 0.015,
                    [h, h + 0.12],
                    tint(steel, 0.57),
                );
            }
        }
        if job != 0 && job != 3 {
            let vent = [0.16, 0.23, 0.30, 0.18, 0.34][build];
            let upper = top - [0.27, 0.27, 0.43, 0.27, 0.24][job];
            self.sides(
                [a + 0.065, b - 0.065],
                width + 0.021,
                [upper - vent, upper],
                tint(steel, 0.73),
            );
            if build == 3 {
                let mid = (a + b) * 0.5;
                for [start, end] in [
                    [a + 0.026, a + 0.092],
                    [mid - 0.042, mid + 0.042],
                    [b - 0.092, b - 0.026],
                ] {
                    self.sides(
                        [start, end],
                        width + 0.032,
                        [DECK, top - [0.23, 0.24, 0.41, 0.24, 0.19][job]],
                        tint(steel, 1.23),
                    );
                }
            }
        }
        if self.detail || job == 0 {
            let posts = [4, 5, 7, 6, 5][build];
            for n in 0..posts {
                let t = a + (b - a - 0.04) * n as f32 / (posts - 1) as f32;
                if job != 3 || (t - 0.5).abs() > 0.055 {
                    self.sides(
                        [t, t + 0.038],
                        width + 0.024,
                        [DECK, top - [0.21, 0.24, 0.41, 0.26, 0.19][job]],
                        tint(steel, 1.13),
                    );
                }
            }
        }
        if build == 2 && job != 0 && job != 3 {
            // The long enclosed rack is braced at the panel joints.
            for t in [a + 0.19, b - 0.22] {
                self.sides(
                    [t, t + 0.032],
                    width + 0.035,
                    [DECK, top - 0.22],
                    tint(steel, 0.81),
                );
            }
        }
        if build == 4 && job != 0 && top < 3.866 {
            self.floor(
                [a + 0.021, b - 0.021],
                [-0.66, 0.66],
                top + 0.004,
                [170, 174, 167],
            );
        }
    }

    fn open_bulk(&mut self, job: usize, build: usize) {
        let span = [
            [0.165, 0.835],
            [0.113, 0.887],
            [0.056, 0.944],
            [0.094, 0.906],
            [0.053, 0.947],
        ][build];
        let loading_space = [0.0, 0.028, 0.095, 0.004, -0.021][job];
        let span = [span[0] + loading_space, span[1] - loading_space];
        let width = [1.18, 1.23, 1.20, 1.29, 1.35][build];
        let rim = [2.62, 2.22, 2.36, 2.13, 1.74][job] + [-0.15, 0.0, -0.02, 0.09, 0.23][build];
        let color = [
            [100, 89, 76],
            [152, 120, 83],
            [146, 93, 75],
            [116, 107, 90],
            [102, 119, 125],
        ][job];
        // Keep these two levels as scalar arithmetic. With a conditional and
        // min(), Xtensa LLVM emits an unsupported PC-relative float table.
        let hopper = f32::from(u8::from(job <= 2));
        let body_bottom = DECK + hopper * (1.46 - DECK);
        let frame_top = DECK + hopper * (1.13 - DECK);
        self.frame(span, width * 0.86, frame_top, span[0] + 0.038);
        self.sides(span, width, [body_bottom, rim], color);
        for t in span {
            self.end(t, width, [body_bottom, rim], tint(color, 0.73));
        }
        let [a, b] = span;
        let cargo = [a + 0.014, b - 0.014];
        if job <= 2 {
            let bays = [2, 3, 4, 3, 4][build];
            let pitch = (b - a) / bays as f32;
            let load = [[62, 63, 64], [167, 163, 144], [156, 99, 73]][job];
            for n in 0..bays {
                let from = a + pitch * n as f32;
                let to = from + pitch;
                self.discharge(
                    [from + 0.011, to - 0.011],
                    width,
                    body_bottom + 0.015,
                    0.76,
                    color,
                );
                if !self.loaded {
                    self.floor([from + 0.012, to - 0.012], [-width + 0.03, width - 0.03],
                               body_bottom + 0.023, tint(color, 0.47));
                    if self.detail {
                        self.floor([from + 0.04, to - 0.04], [-width * 0.39, width * 0.39],
                                   body_bottom + 0.025, tint(color, 0.31));
                    }
                    continue;
                }
                let m = 0.5 * (from + to);
                let crown = rim + [0.20, 0.10, 0.14][job] + (n % 2) as f32 * 0.13;
                self.quad(
                    [
                        [from, -width + 0.03, rim - 0.085],
                        [m, -width * 0.30, crown],
                        [m, width * 0.30, crown],
                        [from, width - 0.03, rim - 0.085],
                    ],
                    load,
                );
                self.quad(
                    [
                        [m, -width * 0.30, crown],
                        [to, -width + 0.03, rim - 0.085],
                        [to, width - 0.03, rim - 0.085],
                        [m, width * 0.30, crown],
                    ],
                    tint(load, 0.80),
                );
                self.quad(
                    [
                        [from, -width + 0.03, rim - 0.085],
                        [to, -width + 0.03, rim - 0.085],
                        [m, -width * 0.30, crown],
                        [m, -width * 0.30, crown],
                    ],
                    tint(load, 0.88),
                );
                self.quad(
                    [
                        [from, width - 0.03, rim - 0.085],
                        [to, width - 0.03, rim - 0.085],
                        [m, width * 0.30, crown],
                        [m, width * 0.30, crown],
                    ],
                    tint(load, 0.96),
                );
            }
        } else if job == 3 {
            self.floor(cargo, [-width, width], DECK + 0.02, [83, 77, 69]);
            let piles = [2, 4, 3, 5, 3][build] * usize::from(self.loaded);
            for i in 0..piles {
                let t = a + (b - a) * (i as f32 + 0.13) / piles as f32;
                let frag = (b - a) / piles as f32;
                let side = (i % 3) as f32 * 0.44 - 0.92;
                let top = rim + 0.11 + (i % 2) as f32 * 0.25;
                let start = t + frag * 0.04;
                let end = t + frag * 0.80;
                let high = [t + frag * (0.40 + (i % 3) as f32 * 0.10), side + 0.25, top];
                let low = [t + frag * 0.31, side + 0.85, top - 0.31];
                let corners = [[start, side, rim - 0.27], [end, side, rim - 0.14],
                               [end, side + 1.04, rim - 0.18], [start, side + 1.04, rim - 0.24]];
                self.quad([corners[0], corners[1], high, high], [112, 86, 68]);
                self.quad([corners[1], corners[2], low, high], [149, 126, 101]);
                self.quad([corners[2], corners[3], low, low], [116, 111, 103]);
                self.quad([corners[3], corners[0], high, low], [130, 115, 99]);
                self.quad([high, [high[0] + frag * 0.11, high[1], top - 0.095],
                           [low[0] + frag * 0.08, low[1], low[2]], low], [174, 179, 163]);
            }
        } else {
            self.floor(cargo, [-width, width], DECK + 0.02, [82, 85, 88]);
            if self.loaded {
                let length = [0.35, 0.53, 0.67, 0.50, 0.72][build];
                let cap = [1.56, 1.69, 1.57, 1.91, 1.77][build];
                self.block([0.48 - length * 0.5, 0.48 + length * 0.5],
                           [-0.91, 0.82], [DECK, cap - 0.11], [116, 127, 128]);
                self.block([0.51 - length * 0.44, 0.51 + length * 0.44],
                           [-0.78, 0.75], [cap - 0.11, cap + 0.022], [153, 159, 159]);
            }
        }
        if build == 4 || job >= 3 {
            // The lip breaks up the open bins and carries sidewall loads.
            for rail in [
                [-width - 0.018, -width + 0.13],
                [width - 0.13, width + 0.018],
            ] {
                self.floor(span, rail, rim + 0.019, tint(color, 1.19));
            }
        }
        if self.detail || build == 3 {
            let ribs = [4, 5, 6, 7, 5][build];
            for n in 1..ribs {
                let t = a + (b - a) * n as f32 / ribs as f32;
                self.sides(
                    [t, t + 0.021],
                    width + 0.026,
                    [body_bottom, rim],
                    tint(color, 0.76),
                );
            }
        }
    }

    fn timber(&mut self, job: usize, build: usize) {
        let span = [
            [0.155, 0.845],
            [0.109, 0.891],
            [0.048, 0.952],
            [0.091, 0.909],
            [0.079, 0.921],
        ][build];
        let width = [1.13, 1.21, 1.25, 1.26, 1.31][build];
        let mast = [2.87, 3.00, 3.12, 3.28, 3.53][build];
        let beam = [140, 101, 62];
        self.frame(span, width, DECK, span[0] + 0.046);
        self.floor(span, [-width, width], DECK, TIMBER);
        self.bulkheads(
            span,
            width,
            [
                DECK,
                mast - if job == 4 || build == 4 { 0.04 } else { 0.31 },
            ],
            beam,
        );
        let [a, b] = span;
        if build == 3 {
            self.side(span, 0.0, [DECK, mast], beam);
        } else {
            self.side(span, 0.0, [mast - 0.15, mast], beam);
            let posts = [3, 4, 5, 4, 6][build];
            for n in 0..posts {
                let start = a + (b - a - 0.056) * n as f32 / (posts - 1) as f32;
                self.side([start, start + 0.044], 0.0, [DECK, mast], beam);
                if (self.detail || job == 0 || build == 2) && n + 1 < posts {
                    let end = start + (b - a - 0.056) / (posts - 1) as f32;
                    self.quad(
                        [
                            [start, 0.0, DECK + 0.1],
                            [start + 0.035, 0.0, DECK + 0.1],
                            [end, 0.0, mast - 0.1],
                            [end - 0.035, 0.0, mast - 0.1],
                        ],
                        beam,
                    );
                }
            }
        }
        if job == 0 || !self.loaded { return; }
        let load_color = [
            [0, 0, 0],
            [213, 204, 174],
            [182, 140, 86],
            [177, 181, 172],
            [127, 95, 64],
        ][job];
        let count = [2, 3, 4, 3, 3][build];
        let pitch = (b - a - 0.088) / count as f32;
        for n in 0..count {
            let start = a + 0.048 + pitch * n as f32;
            let span = [start, start + pitch - 0.024];
            let rise = if job == 3 && build == 3 {
                // One taller pack fills the center between the low end stacks.
                [0.90, 0.27, 0.90][n]
            } else {
                [0.0, 0.36, 0.21, 0.94, 0.69][job] + (n % 2) as f32 * 0.16
            };
            let top = mast - rise;
            self.block(span, [0.07, width - 0.045], [DECK + 0.01, top], load_color);
            self.block(
                span,
                [-width + 0.045, -0.07],
                [DECK + 0.01, top - 0.085],
                load_color,
            );
        }
    }

    fn coil(&mut self, job: usize, build: usize) {
        let span = [
            [0.17, 0.83],
            [0.135, 0.865],
            [0.07, 0.93],
            [0.13, 0.87],
            [0.055, 0.945],
        ][build];
        let deck = [1.01, 1.03, 0.94, 1.24, 1.38][build];
        let width = [1.17, 1.22, 1.26, 1.27, 1.33][build];
        self.frame(span, width, deck, span[0] + 0.036);
        self.floor(span, [-width, width], deck, FRAME);
        self.sides(
            span,
            width + 0.015,
            [deck, deck + [0.12, 0.31, 0.19, 0.32, 0.24][build]],
            [116, 98, 79],
        );
        let count = [if build < 2 { 2 } else { 3 }, 1, 2, 3, 1][job];
        let pitch = (span[1] - span[0]) / count as f32;
        for n in 0..count {
            let a = span[0] + pitch * n as f32 + 0.012;
            let b = a + pitch - 0.024;
            if job == 0 || job == 4 {
                let radius =
                    [0.69, 0.79, 0.76, 0.68, 0.84][build] + if job == 4 { 0.18 } else { 0.0 };
                let center = (a + b) * 0.5;
                let saddle = [0.051, 0.067, 0.070, 0.061, 0.095][build];
                self.floor(
                    [center - saddle, center + saddle],
                    [-width, width],
                    deck + 0.110,
                    [112, 104, 92],
                );
                if !self.loaded { continue; }
                if job == 0 {
                    let half = 0.28 * (b - a);
                    self.longitudinal_coil([center - half, center + half], radius, deck);
                } else {
                    self.coil_roll(center, radius, width * 0.83, deck);
                }
            } else {
                let height = deck + [0.00, 1.56, 1.36, 1.10, 0.00][job] + build as f32 * 0.065;
                let color = [
                    [0, 0, 0],
                    [134, 139, 152],
                    [168, 157, 133],
                    [134, 143, 156],
                    [0, 0, 0],
                ][job];
                self.body(
                    [a, b],
                    width - 0.055,
                    [deck + 0.13, height],
                    [[0.39, 0.50, 0.58, 0.44, 0.61][build], 0.38, 0.89],
                    color,
                );
                if self.detail || build == 1 || build == 4 {
                    self.floor(
                        [a + (b - a) * 0.42, b - (b - a) * 0.42],
                        [-0.48, 0.48],
                        height + 0.032,
                        tint(color, 0.64),
                    );
                }
            }
        }
    }

    #[inline(never)]
    fn longitudinal_coil(&mut self, [a, b]: [f32; 2], radius: f32, floor: f32) {
        let y = floor + radius;
        let rim = [[-radius, 0.0], [-radius * 0.50, radius * 0.88],
                   [radius * 0.50, radius * 0.88], [radius, 0.0],
                   [radius * 0.50, -radius * 0.88], [-radius * 0.50, -radius * 0.88]];
        for (n, shade) in [0.93, 1.12, 0.99, 0.76, 0.56, 0.63].into_iter().enumerate() {
            let [s, h] = rim[n];
            let [t, k] = rim[(n + 1) % rim.len()];
            let steel = tint([133, 149, 153], shade);
            if n < 3 || self.detail {
                self.quad([[a, s, y + h], [a, t, y + k], [b, t, y + k], [b, s, y + h]], steel);
            }
            for end in [a, b] {
                self.quad([[end, s, y + h], [end, t, y + k],
                           [end, t * 0.42, y + k * 0.42], [end, s * 0.42, y + h * 0.42]],
                          tint(steel, 0.82));
            }
        }
    }

    fn coil_roll(&mut self, center: f32, radius: f32, width: f32, floor: f32) {
        let y = floor + radius;
        let x = radius * 0.088;
        let outer = [
            [-x, 0.0],
            [-x * 0.50, radius * 0.88],
            [x * 0.50, radius * 0.88],
            [x, 0.0],
            [x * 0.50, -radius * 0.88],
            [-x * 0.50, -radius * 0.88],
        ];
        for (n, shade) in [0.93, 1.12, 0.99, 0.76, 0.56, 0.63].into_iter().enumerate() {
            let [a, h] = outer[n];
            let [b, k] = outer[(n + 1) % 6];
            let steel = tint([133, 149, 153], shade);
            if n < 3 || self.detail {
                self.quad(
                    [
                        [center + a, -width, y + h],
                        [center + b, -width, y + k],
                        [center + b, width, y + k],
                        [center + a, width, y + h],
                    ],
                    steel,
                );
            }
            for side in [-width, width] {
                self.quad(
                    [
                        [center + a, side, y + h],
                        [center + b, side, y + k],
                        [center + b * 0.42, side, y + k * 0.42],
                        [center + a * 0.42, side, y + h * 0.42],
                    ],
                    tint(steel, 0.82),
                );
            }
        }
    }

    fn reefer(&mut self, job: usize, build: usize) {
        let [a, b] = [
            [0.165, 0.905],
            [0.122, 0.926],
            [0.091, 0.955],
            [0.072, 0.934],
            [0.084, 0.955],
        ][build];
        let b = b - [0.0, 0.011, 0.0, 0.025, 0.077][job];
        let width = [1.17, 1.24, 1.25, 1.27, 1.32][build];
        let rise = [3.10, 3.34, 3.26, 3.46, 3.69][build];
        let top = rise + [-0.018, 0.072, -0.053, 0.12, -0.10][job];
        let color = [
            [201, 208, 180],
            [190, 208, 219],
            [211, 199, 175],
            [209, 212, 202],
            [193, 203, 184],
        ][job];
        let stripe = [
            [122, 147, 91],
            [95, 139, 163],
            [158, 105, 83],
            [111, 135, 145],
            [153, 135, 83],
        ][job];
        self.frame([a, b], width, DECK, a + 0.019);
        self.body(
            [a, b],
            width,
            [DECK, top],
            [[0.73, 0.85, 0.89, 0.77, 0.93][build], 0.21, 1.0],
            color,
        );
        let door = [0.093, 0.11, 0.19, 0.102, 0.13][build];
        let center = [0.49, 0.54, 0.47, 0.58, 0.55][job];
        let rail = [1.50, 1.57, 1.75, 1.42, 1.61][job];
        self.sides(
            [a + 0.018, b - 0.018],
            width + 0.011,
            [rail, rail + 0.18],
            stripe,
        );
        self.sides(
            [center - door, center + door],
            width + 0.024,
            [1.22, top - 0.30],
            tint(color, 0.76),
        );
        if build == 3 {
            self.block([0.30, 0.49], [-0.99, 0.99], [0.50, DECK], WHEEL);
            self.sides([0.319, 0.471], 1.01, [0.55, 0.93], stripe);
        } else {
            let unit = [0.67, 0.81, 0.97, 0.71, 1.14][build];
            let mounting = [0.045, 0.042, 0.043, 0.039, 0.031][build];
            let base = top - [0.79, 1.03, 1.37, 0.88, 1.54][build];
            self.block(
                [mounting, a + 0.006],
                [-unit, unit],
                [base, top - 0.13],
                stripe,
            );
            self.end(
                mounting - 0.005,
                unit * 0.71,
                [base + 0.16, top - 0.34],
                [70, 85, 85],
            );
        }
        if build == 2 {
            for latch in [center - 0.086, center + 0.086] {
                self.sides(
                    [latch - 0.013, latch + 0.013],
                    width + 0.035,
                    [1.24, top - 0.31],
                    tint(color, 0.54),
                );
            }
        }
        if self.detail {
            self.sides(
                [center - 0.011, center + 0.011],
                width + 0.037,
                [1.23, top - 0.31],
                tint(color, 0.57),
            );
        }
    }
}

fn mix(a: [u8; 3], b: [u8; 3]) -> [u8; 3] {
    core::array::from_fn(|i| ((u16::from(a[i]) + 3 * u16::from(b[i])) / 4) as u8)
}

pub(super) fn tint(color: [u8; 3], strength: f32) -> [u8; 3] {
    color.map(|channel| (f32::from(channel) * strength + 0.5) as u8)
}
