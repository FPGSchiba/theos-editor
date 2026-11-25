//! A small complex-number type.
//!
//! Our own, because `num-complex` brings `num-traits` with std float methods (spec 4.3); the
//! polar form goes through `libm`.

use std::num::NonZeroUsize;
use std::ops::{Add, Div, Mul, Sub};

use super::num::to_f64;

/// A complex number with `f64` parts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Complex {
    /// Real part.
    pub re: f64,
    /// Imaginary part.
    pub im: f64,
}

impl Complex {
    /// The zero value.
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };

    /// `re + im i`.
    #[must_use]
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// `r (cos theta + i sin theta)`, with `libm` trigonometry.
    #[must_use]
    pub fn from_polar(r: f64, theta: f64) -> Self {
        Self::new(r * libm::cos(theta), r * libm::sin(theta))
    }

    /// `e^(2 pi i num / den)`, the unit number `num / den` turns round. `num` is reduced modulo
    /// `den` in integers before the angle is formed, so a large `num` costs no precision.
    #[must_use]
    pub fn from_turns(num: usize, den: NonZeroUsize) -> Self {
        let den = den.get();
        Self::from_polar(
            1.0,
            2.0 * core::f64::consts::PI * to_f64(num % den) / to_f64(den),
        )
    }

    /// Both parts multiplied by `s`.
    #[must_use]
    pub fn scale(self, s: f64) -> Self {
        Self::new(self.re * s, self.im * s)
    }

    /// The complex conjugate.
    #[must_use]
    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }
}

impl Add for Complex {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.re + o.re, self.im + o.im)
    }
}

impl Sub for Complex {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.re - o.re, self.im - o.im)
    }
}

impl Mul for Complex {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        Self::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
}

impl Div for Complex {
    type Output = Self;
    fn div(self, o: Self) -> Self {
        let d = o.re * o.re + o.im * o.im;
        let n = self * o.conj();
        Self::new(n.re / d, n.im / d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Complex, re: f64, im: f64) -> bool {
        (a.re - re).abs() < 1e-14 && (a.im - im).abs() < 1e-14
    }

    #[test]
    fn arithmetic_matches_hand_values() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, -1.0);
        assert!(close(a + b, 4.0, 1.0));
        assert!(close(a - b, -2.0, 3.0));
        assert!(close(a * b, 5.0, 5.0));
        assert!(close(a / b, 0.1, 0.7));
        assert!(close(a.scale(2.0), 2.0, 4.0));
        assert!(close(a.conj(), 1.0, -2.0));
        assert!(close(Complex::ZERO, 0.0, 0.0));
    }

    #[test]
    fn from_polar_uses_the_angle() {
        let q = Complex::from_polar(2.0, core::f64::consts::FRAC_PI_2);
        assert!(close(q, 0.0, 2.0));
    }

    #[test]
    fn from_turns_reduces_whole_turns_first() {
        let den = |d: usize| NonZeroUsize::new(d).unwrap();
        assert!(close(Complex::from_turns(0, den(7)), 1.0, 0.0));
        assert!(close(Complex::from_turns(1, den(4)), 0.0, 1.0));
        assert!(close(Complex::from_turns(2, den(4)), -1.0, 0.0));
        let h = core::f64::consts::FRAC_1_SQRT_2;
        assert!(close(Complex::from_turns(3, den(8)), -h, h));
        // 10^9 + 1 quarter turns is one quarter turn, to the last bit.
        let far = Complex::from_turns(4_000_000_001, den(4));
        assert_eq!(far, Complex::from_turns(1, den(4)));
    }
}
