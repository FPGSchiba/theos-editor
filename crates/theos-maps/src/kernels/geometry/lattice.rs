//! The integer lattice of map geometry: points in 1/16 m units, rings and polygons on it.
//!
//! Coordinates are `i32` units of 1/16 m, x east and y south. Values entering through [`snap`]
//! are capped at `|c| <= 2^30` units (about 67,108 km), so that sums and differences of two
//! lattice values stay far inside `i32` and the predicates have wide headroom in `i128`.

use serde::{Deserialize, Serialize};

use crate::kernels::num;

/// Lattice units per metre.
pub const UNITS_PER_M: i32 = 16;

/// The largest lattice coordinate [`snap`] accepts, in units.
pub const COORD_LIMIT: i32 = 1 << 30;

/// A lattice point. The field order is the lattice order: by `y`, then by `x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Point {
    /// Southward coordinate in units.
    pub y: i32,
    /// Eastward coordinate in units.
    pub x: i32,
}

impl Point {
    /// A point from its `x` (east) and `y` (south) coordinates in units.
    #[must_use]
    pub fn new(x: i32, y: i32) -> Self {
        Self { y, x }
    }
}

/// A ring, closed implicitly: the first point is not repeated at the end.
pub type Ring = Vec<Point>;

/// A polygon with holes. The outer ring has positive [`twice_area`] and every hole negative
/// (L17: wound in (x east, y south)).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Polygon {
    /// The outer ring.
    pub outer: Ring,
    /// The holes.
    pub holes: Vec<Ring>,
}

/// Errors of the lattice.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LatticeError {
    /// A length that is not finite or lies beyond the lattice cap.
    #[error("{metres} m is not finite or lies beyond the lattice")]
    OutOfRange {
        /// The metres given.
        metres: f64,
    },
}

/// Metres snapped to the lattice, halves away from zero.
///
/// # Errors
/// [`LatticeError::OutOfRange`] for NaN, an infinity, or `|units| > 2^30`.
pub fn snap(m: f64) -> Result<i32, LatticeError> {
    // Multiplying by 16 is exact in binary floating point.
    let units = m * f64::from(UNITS_PER_M);
    if !units.is_finite() || units.abs() > f64::from(COORD_LIMIT) {
        return Err(LatticeError::OutOfRange { metres: m });
    }
    Ok(to_units(num::round_half_away(units)))
}

/// A rounded, range-checked float as `i32`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is an integer within +-2^30, checked by snap"
)]
fn to_units(rounded: f64) -> i32 {
    rounded as i32
}

/// Lattice units as metres; exact, as `n / 16` is representable.
#[must_use]
pub fn to_m(n: i32) -> f64 {
    f64::from(n) / f64::from(UNITS_PER_M)
}

/// A lattice point as `[x, y]` in metres.
#[must_use]
pub fn point_m(p: Point) -> [f64; 2] {
    [to_m(p.x), to_m(p.y)]
}

/// Twice the signed area of a ring (shoelace) in square units, positive when wound with
/// positive area in (x east, y south). Exact in `i128` for any `i32` coordinates.
#[must_use]
pub fn twice_area(ring: &[Point]) -> i128 {
    let mut sum = 0_i128;
    for (i, a) in ring.iter().enumerate() {
        let b = ring[(i + 1) % ring.len()];
        sum += i128::from(a.x) * i128::from(b.y) - i128::from(b.x) * i128::from(a.y);
    }
    sum
}
