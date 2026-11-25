use core::f32::consts::{FRAC_PI_2, PI, TAU};

pub(crate) const fn wrap_angle(angle: f32) -> f32 {
    let wrapped = if angle >= -TAU && angle < TAU { angle } else { angle % TAU };
    if wrapped < -PI {
        wrapped + TAU
    } else if wrapped >= PI {
        wrapped - TAU
    } else {
        wrapped
    }
}

pub(crate) const fn sin_cos(angle: f32) -> (f32, f32) {
    let mut x = wrap_angle(angle);
    let cos_sign = if x > FRAC_PI_2 {
        x = PI - x;
        -1.0
    } else if x < -FRAC_PI_2 {
        x = -PI - x;
        -1.0
    } else {
        1.0
    };
    let xx = x * x;
    let sin = x * (1.0 + xx * (-1.0 / 6.0 + xx * (1.0 / 120.0 + xx * (-1.0 / 5040.0 + xx / 362880.0))));
    let cos = 1.0 + xx * (-0.5 + xx * (1.0 / 24.0 + xx * (-1.0 / 720.0 + xx * (1.0 / 40320.0 - xx / 3628800.0))));
    (sin.clamp(-1.0, 1.0), cos * cos_sign)
}

pub(crate) fn mix(value: u64) -> u64 {
    let value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    let value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

pub(crate) fn fraction(bits: u64) -> f32 {
    (bits >> 40) as f32 * (1.0 / 16_777_216.0)
}

pub(crate) fn smoothstep(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_trig_is_continuous_and_accurate() {
        for n in -300..=300 {
            let angle = n as f32 * 0.019;
            let (sin, cos) = sin_cos(angle);
            assert!((sin - angle.sin()).abs() < 0.000005);
            assert!((cos - angle.cos()).abs() < 0.000005);
        }
    }
}
