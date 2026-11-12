//! The water balance: temperature by lapse rate, potential and actual evapotranspiration.

use super::ClimateParams;
use crate::kernels::num::{max, min};

/// Air temperature in degrees Celsius at `height` metres: the base temperature minus the lapse
/// rate times the height above sea level (the sea floor counts as sea level).
#[must_use]
pub fn temperature(height: f64, p: &ClimateParams) -> f64 {
    p.t_base_c - p.lapse_c_per_km * max(height, 0.0) / 1000.0
}

/// Potential evapotranspiration in mm/yr, 0.3.0's `clip(300 + 45 T, 200, 1800)`.
#[must_use]
pub fn pet(t_c: f64) -> f64 {
    min(max(300.0 + 45.0 * t_c, 200.0), 1800.0)
}

/// Actual evapotranspiration by Fu's form of the Budyko curve,
/// `P (1 + phi - (1 + phi^w)^(1/w))` with `phi = PET / P`; `w` shapes the curve.
///
/// It is 0 for no precipitation, tends to 0 for a dry demand (`phi -> 0`) and to `P` for a
/// thirsty one (`phi -> infinity`), and never exceeds `P`. For `phi > 1` the same value is
/// computed as `P (1 - phi expm1(ln_1p(phi^-w) / w))`, which cannot overflow or cancel.
#[must_use]
pub fn fu_aet(p: f64, pet: f64, w: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    let phi = pet / p;
    if !phi.is_finite() {
        return p;
    }
    let share = if phi <= 1.0 {
        1.0 + phi - libm::pow(1.0 + libm::pow(phi, w), 1.0 / w)
    } else {
        1.0 - phi * libm::expm1(libm::log1p(libm::pow(phi, -w)) / w)
    };
    p * min(max(share, 0.0), 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stages::terrain::climate::ClimateParams;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * (1.0 + b.abs())
    }

    #[test]
    fn pet_is_clamped() {
        assert!(near(pet(-20.0), 200.0));
        assert!(near(pet(40.0), 1800.0));
        assert!(near(pet(10.0), 750.0));
    }

    #[test]
    fn temperature_falls_with_height_and_ignores_the_sea_floor() {
        let p = ClimateParams::default();
        assert!(near(temperature(0.0, &p), 13.0));
        assert!(near(temperature(-500.0, &p), 13.0));
        assert!(near(temperature(1000.0, &p), 6.5));
    }

    #[test]
    fn fu_curve_limits() {
        let w = 2.6;
        assert!(fu_aet(1000.0, 1e-9, w) < 1e-6);
        assert!(near(fu_aet(1000.0, 1e9, w), 1000.0));
        assert!(near(fu_aet(1000.0, f64::MAX, w), 1000.0));
        let want = 1000.0 * (2.0 - libm::pow(2.0, 1.0 / 2.6));
        assert!(near(fu_aet(1000.0, 1000.0, w), want));
        assert_eq!(fu_aet(0.0, 500.0, w).to_bits(), 0.0_f64.to_bits());
        assert_eq!(fu_aet(-5.0, 500.0, w).to_bits(), 0.0_f64.to_bits());
        assert!(near(fu_aet(1e-300, 1e300, w), 1e-300));
    }

    #[test]
    fn fu_curve_is_bounded_and_monotonic_in_pet() {
        let mut last = 0.0;
        for pet in [10.0, 100.0, 400.0, 800.0, 1600.0, 5000.0] {
            let a = fu_aet(800.0, pet, 2.6);
            assert!((0.0..=800.0).contains(&a) && a >= last);
            assert!(a <= pet + 1e-9);
            last = a;
        }
    }
}
