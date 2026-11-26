//! Airfield roof plans in normalized coordinates inside a surveyed building pad.
//! Across faces the apron at -1. Along follows the runway axis.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Roof {
    Flat,
    Pitched,
    Glazed,
}

#[derive(Clone, Copy)]
enum Shape {
    Flat,
    Gable,
    Barrel,
    Shed,
    Hip,
    Round,
    Glazed,
}

#[derive(Clone, Copy)]
struct Mass {
    bounds: [f32; 4],
    rise: [f32; 2],
    shape: Shape,
}

const EMPTY: Mass = mass([0.0; 4], [0.0; 2], Shape::Flat);

const fn mass(bounds: [f32; 4], rise: [f32; 2], shape: Shape) -> Mass {
    Mass {
        bounds,
        rise,
        shape,
    }
}

use Shape::*;

// Each set has one building role. A smaller adjoining mass makes the
// different uses legible without relying on a roof color or a window texture.
const PLANS: [[Mass; 3]; 25] = [
    // Country hangars.
    [
        mass([-0.76, -0.92, 0.56, 1.0], [0.61, 0.93], Gable),
        mass([0.52, -0.55, 0.97, 0.72], [0.37, 0.51], Shed),
        EMPTY,
    ],
    [
        mass([-1.0, -0.88, -0.12, 0.93], [0.48, 1.0], Barrel),
        mass([0.12, -0.75, 1.0, 0.91], [0.41, 0.78], Barrel),
        mass([-0.19, 0.61, 0.19, 0.91], [0.43, 0.43], Flat),
    ],
    [
        mass([-0.83, -0.80, 0.86, 0.97], [0.54, 0.92], Shed),
        mass([-0.96, -0.98, 0.29, -0.71], [0.38, 0.38], Flat),
        EMPTY,
    ],
    [
        mass([-0.63, -0.94, 1.0, 0.82], [0.49, 0.96], Hip),
        mass([-1.0, 0.04, -0.54, 0.83], [0.44, 0.60], Gable),
        EMPTY,
    ],
    [
        mass([-0.91, -0.97, 0.96, 0.93], [0.47, 0.97], Barrel),
        mass([-0.56, 0.54, 0.39, 0.98], [0.70, 1.0], Glazed),
        mass([0.59, -0.92, 0.96, -0.34], [0.62, 0.72], Flat),
    ],
    // Regional hangars.
    [
        mass([-1.0, -0.88, 0.94, 0.97], [0.49, 0.88], Gable),
        mass([-0.64, -0.34, 0.70, 0.38], [0.82, 1.0], Glazed),
        EMPTY,
    ],
    [
        mass([-1.0, -0.94, -0.24, 0.88], [0.52, 0.92], Shed),
        mass([-0.30, -0.94, 0.44, 0.88], [0.51, 0.99], Shed),
        mass([0.38, -0.94, 0.98, 0.75], [0.50, 0.86], Shed),
    ],
    [
        mass([-0.95, -1.0, 0.14, 0.96], [0.50, 1.0], Barrel),
        mass([0.10, -0.54, 0.91, 0.96], [0.45, 0.71], Gable),
        mass([0.30, -0.94, 0.97, -0.47], [0.31, 0.31], Flat),
    ],
    [
        mass([-0.98, -0.87, -0.12, 0.89], [0.57, 0.92], Gable),
        mass([0.18, -0.87, 0.94, 0.89], [0.66, 1.0], Gable),
        mass([-0.34, 0.45, 0.38, 0.98], [0.55, 0.55], Flat),
    ],
    [
        mass([-1.0, -0.96, 0.83, 0.93], [0.63, 0.63], Flat),
        mass([-0.91, -0.79, -0.53, 0.65], [0.62, 0.94], Shed),
        mass([0.45, -0.39, 0.96, 0.83], [0.61, 0.97], Glazed),
    ],
    // Passenger terminals.
    [
        mass([-0.94, -0.51, 0.92, 0.86], [0.53, 0.84], Barrel),
        mass([-0.66, -0.97, 0.75, -0.32], [0.31, 0.44], Glazed),
        EMPTY,
    ],
    [
        mass([-0.95, -0.89, -0.40, 0.73], [0.46, 0.69], Shed),
        mass([0.39, -0.77, 0.98, 0.74], [0.55, 0.82], Shed),
        mass([-0.96, 0.29, 0.97, 0.95], [0.60, 0.88], Gable),
    ],
    [
        mass([-0.89, -0.73, 0.78, 0.95], [0.62, 0.78], Hip),
        mass([-0.57, -1.0, 0.15, -0.59], [0.44, 0.44], Glazed),
        mass([0.40, -0.27, 0.93, 0.40], [0.70, 0.95], Round),
    ],
    [
        mass([-0.90, -0.62, 0.91, 0.76], [0.48, 0.48], Flat),
        mass([-0.79, -0.17, -0.13, 0.97], [0.57, 0.91], Barrel),
        mass([0.24, -0.41, 0.82, 0.95], [0.52, 0.98], Barrel),
    ],
    [
        mass([-0.94, -0.67, -0.22, 0.91], [0.34, 0.63], Shed),
        mass([-0.24, -0.69, 0.39, 0.98], [0.56, 0.82], Gable),
        mass([0.37, -0.90, 0.95, 0.96], [0.63, 0.94], Glazed),
    ],
    // Operations buildings.
    [
        mass([-0.98, -0.86, 0.93, 0.94], [0.36, 0.36], Flat),
        mass([-0.65, -0.07, -0.06, 0.52], [0.83, 0.86], Glazed),
        mass([-0.73, -0.14, 0.02, 0.61], [0.87, 0.99], Hip),
    ],
    [
        mass([-0.95, -0.68, 0.49, 0.93], [0.35, 0.48], Shed),
        mass([0.23, -0.83, 0.98, 0.45], [0.43, 0.55], Gable),
        mass([0.32, -0.71, 0.90, 0.18], [0.91, 1.0], Round),
    ],
    [
        mass([-0.95, -0.95, 0.91, 0.81], [0.37, 0.51], Gable),
        mass([-0.24, -0.36, 0.90, 0.69], [0.68, 0.68], Flat),
        mass([0.42, -0.27, 0.96, 0.32], [0.98, 0.98], Glazed),
    ],
    [
        mass([-0.97, -0.85, -0.18, 0.82], [0.39, 0.53], Hip),
        mass([0.03, -0.86, 0.98, 0.43], [0.35, 0.47], Shed),
        mass([-0.23, 0.28, 0.37, 0.96], [0.82, 0.96], Round),
    ],
    [
        mass([-0.92, -0.98, 0.95, 0.70], [0.35, 0.39], Flat),
        mass([-0.72, 0.09, 0.77, 0.96], [0.57, 0.68], Gable),
        mass([-0.62, 0.22, -0.07, 0.90], [0.87, 0.98], Glazed),
    ],
    // Service buildings.
    [
        mass([-0.94, -0.94, 0.34, 0.92], [0.53, 0.93], Gable),
        mass([0.33, -0.59, 0.97, 0.41], [0.49, 0.66], Shed),
        EMPTY,
    ],
    [
        mass([-0.98, -0.84, -0.15, 0.78], [0.68, 0.96], Round),
        mass([0.05, -0.61, 0.98, 0.92], [0.57, 0.86], Round),
        mass([-0.75, 0.42, 0.77, 0.94], [0.40, 0.40], Flat),
    ],
    [
        mass([-0.99, -0.81, 0.94, 0.78], [0.48, 0.73], Shed),
        mass([-0.74, -0.84, -0.12, 0.74], [0.69, 0.88], Barrel),
        mass([0.20, -0.82, 0.86, 0.75], [0.71, 0.98], Barrel),
    ],
    [
        mass([-0.62, -0.97, 0.99, 0.90], [0.55, 0.86], Hip),
        mass([-0.99, -0.36, -0.49, 0.42], [0.41, 0.54], Flat),
        mass([0.53, 0.14, 0.96, 0.78], [0.86, 0.98], Shed),
    ],
    [
        mass([-0.96, 0.02, 0.97, 0.95], [0.52, 0.89], Gable),
        mass([-0.95, -0.89, -0.53, 0.17], [0.61, 0.84], Shed),
        mass([0.51, -0.88, 0.98, 0.21], [0.67, 0.99], Shed),
    ],
];

impl Mass {
    fn sample(self, along: f32, across: f32) -> Option<(Roof, f32)> {
        let [a, b, c, d] = self.bounds;
        if self.rise[1] == 0.0 || along < a || along > c || across < b || across > d {
            return None;
        }
        let u = (along - a) / (c - a) * 2.0 - 1.0;
        let v = (across - b) / (d - b) * 2.0 - 1.0;
        let (roof, profile) = match self.shape {
            Flat => (Roof::Flat, 0.0),
            Gable => (Roof::Pitched, 1.0 - v.abs()),
            Barrel => (Roof::Pitched, 1.0 - v * v),
            Shed => (Roof::Pitched, (u + 1.0) * 0.5),
            Hip => (Roof::Pitched, 1.0 - u.abs().max(v.abs())),
            Round => {
                let radius = u * u + v * v;
                if radius > 1.0 {
                    return None;
                }
                (Roof::Flat, 1.0 - radius)
            }
            Glazed => (Roof::Glazed, 1.0 - v.abs()),
        };
        Some((roof, self.rise[0] + (self.rise[1] - self.rise[0]) * profile))
    }
}

#[inline(never)]
pub(super) fn sample(design: u8, along: f32, across: f32) -> Option<(Roof, f32)> {
    let plan = &PLANS[usize::from(design)];
    let mut result = None;
    for &mass in plan {
        if let Some(roof) = mass.sample(along, across) {
            if result.is_none_or(|(_, elevation)| roof.1 > elevation) {
                result = Some(roof);
            }
        }
    }
    result
}
