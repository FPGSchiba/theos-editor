//! Float and cast helpers shared by every kernel.
//!
//! Maxima, minima and sort keys never carry a negative zero, so equal inputs give equal bits on
//! every platform. The `as` casts between integers and floats live here and nowhere else.

/// The larger of two floats; `+0.0` when both are zeros of either sign.
#[must_use]
pub fn max(a: f64, b: f64) -> f64 {
    if a > b { a + 0.0 } else { b + 0.0 }
}

/// The smaller of two floats; `+0.0` when both are zeros of either sign.
#[must_use]
pub fn min(a: f64, b: f64) -> f64 {
    if a < b { a + 0.0 } else { b + 0.0 }
}

/// `x + 0.0`, which turns `-0.0` into `+0.0`; use it for sort and queue keys.
#[must_use]
pub fn key(x: f64) -> f64 {
    x + 0.0
}

/// A float ordered by [`f64::total_cmp`], so it can sit in a `BinaryHeap` or be sorted; build it
/// from [`key`] so that `-0.0` and `+0.0` are one value.
#[derive(Debug, Clone, Copy)]
pub struct OrdF64(pub f64);

impl PartialEq for OrdF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0).is_eq()
    }
}

impl Eq for OrdF64 {}

impl PartialOrd for OrdF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrdF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Turns `-0.0` into `+0.0` and leaves every other value unchanged.
#[must_use]
pub fn canonical(x: f64) -> f64 {
    x + 0.0
}

/// `x` rounded to the nearest integer, halves to even, as Python's and numpy's `round`. The
/// rounding is exact IEEE 754 (no libm), so it is safe for determinism (G20).
#[must_use]
pub fn round_half_even(x: f64) -> f64 {
    x.round_ties_even()
}

/// A cell count or index as a float.
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "indices stay far below 2^53")]
pub fn to_f64(i: usize) -> f64 {
    i as f64
}

/// A feature property integer (a radius, a count) as a float.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "property integers stay far below 2^53"
)]
pub fn to_f64_i64(i: i64) -> f64 {
    i as f64
}

/// `floor(x)` as an index, clamped to `0..len` (`0` for NaN or `len == 0`).
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is floored and clamped into 0..len first"
)]
pub fn to_index(x: f64, len: usize) -> usize {
    let floor = x.floor();
    if floor.is_nan() || floor <= 0.0 {
        0
    } else if floor >= to_f64(len) {
        len.saturating_sub(1)
    } else {
        floor as usize
    }
}

/// A float narrowed to `f32` (round to nearest).
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "narrowing to f32 is the purpose"
)]
pub fn to_f32(x: f64) -> f32 {
    x as f32
}

/// `round(clamp(x, 0, 1) * 255)` as a byte.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 and rounded first"
)]
pub fn to_u8_unit(x: f64) -> u8 {
    (min(max(x, 0.0), 1.0) * 255.0).round() as u8
}

/// The low and high 64 bits of a 128-bit value.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the halves are taken on purpose"
)]
pub fn split_u128(m: u128) -> (u64, u64) {
    (m as u64, (m >> 64) as u64)
}

/// The top 53 bits of `bits` scaled into `[0, 1)`; exact, since 53 bits fit a mantissa.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "a 53-bit value is exactly representable"
)]
pub fn unit_from_bits(bits: u64) -> f64 {
    (bits >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
}

/// `-1`, `0` or `1` by the sign of `x`; `0` for zero and NaN, like numpy's `sign` for zero.
#[must_use]
pub fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Smoothstep from `e0` to `e1`; for `e0 > e1` it falls from 1 to 0 as in 0.3.0.
#[must_use]
pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let span = e1 - e0;
    if span == 0.0 {
        return if x >= e1 { 1.0 } else { 0.0 };
    }
    let t = min(max((x - e0) / span, 0.0), 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_is_numpys() {
        let signs = [2.0, -0.5, 0.0, -0.0, f64::NAN].map(|x| sign(x).to_bits());
        let want = [1.0_f64, -1.0, 0.0, 0.0, 0.0].map(f64::to_bits);
        assert_eq!(signs, want);
    }

    #[test]
    fn halves_round_to_even() {
        for (x, want) in [
            (0.5, 0.0),
            (1.5, 2.0),
            (2.5, 2.0),
            (-0.5, 0.0),
            (-1.5, -2.0),
            (2.4, 2.0),
        ] {
            assert_eq!(round_half_even(x), want, "{x}");
        }
    }

    #[test]
    fn max_and_min_return_positive_zero_on_ties() {
        for (a, b) in [(-0.0, 0.0), (0.0, -0.0), (-0.0, -0.0)] {
            assert_eq!(max(a, b).to_bits(), 0);
            assert_eq!(min(a, b).to_bits(), 0);
        }
        assert_eq!(key(-0.0).to_bits(), 0);
        assert_eq!(canonical(-0.0).to_bits(), 0);
    }

    #[test]
    fn smoothstep_matches_0_3_0() {
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-15);
        assert!((smoothstep(-1.5, -5.0, -6.0) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn to_index_clamps() {
        assert_eq!(to_index(-0.5, 10), 0);
        assert_eq!(to_index(9.7, 10), 9);
        assert_eq!(to_index(3.2, 10), 3);
        assert_eq!(to_index(1e9, 10), 9);
    }

    #[test]
    fn casts_round_trip() {
        assert!((to_f64(7) - 7.0).abs() < 1e-15);
        assert!((f64::from(to_f32(0.5)) - 0.5).abs() < 1e-15);
        assert_eq!(to_u8_unit(-1.0), 0);
        assert_eq!(to_u8_unit(2.0), 255);
        assert_eq!(to_u8_unit(0.5), 128);
    }
}
