//! Exact predicates on the lattice: orientation, segment crossings and point containment.
//!
//! All arithmetic is in `i128`. For any `i32` coordinates a difference is below `2^32`, a cross
//! product below `2^65`, an intersection numerator below `2^100`: nothing overflows.

use std::cmp::Ordering;

use super::lattice::{Point, Polygon};

/// A rational point `(x / den, y / den)` with `den > 0`, in lowest terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PointQ {
    /// Numerator of x.
    pub x: i128,
    /// Numerator of y.
    pub y: i128,
    /// The common denominator, positive.
    pub den: i128,
}

impl PointQ {
    fn new(x: i128, y: i128, den: i128) -> Self {
        let sign = if den < 0 { -1 } else { 1 };
        let g = gcd(gcd(x.abs(), y.abs()), den.abs()).max(1);
        Self {
            x: sign * x / g,
            y: sign * y / g,
            den: sign * den / g,
        }
    }

    fn from_point(p: Point) -> Self {
        Self {
            x: i128::from(p.x),
            y: i128::from(p.y),
            den: 1,
        }
    }
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// How two segments meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crossing {
    /// They share no point.
    None,
    /// They cross at one interior point of both.
    Proper(PointQ),
    /// They share one point that is an endpoint of at least one.
    Touch(PointQ),
    /// They are collinear and share a stretch of positive length.
    Overlap,
}

/// Where a point lies against a polygon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Containment {
    /// In the interior (holes excluded).
    Inside,
    /// Not in the polygon.
    Outside,
    /// On a ring.
    Boundary,
}

fn cross(ax: i128, ay: i128, bx: i128, by: i128) -> i128 {
    ax * by - ay * bx
}

fn diff(p: i32, q: i32) -> i128 {
    i128::from(p) - i128::from(q)
}

/// The orientation of `a`, `b`, `c` in (x east, y south): `Greater` for a positive cross
/// product `(b - a) x (c - a)`, `Less` for a negative one, `Equal` when collinear.
#[must_use]
pub fn orient(a: Point, b: Point, c: Point) -> Ordering {
    cross(
        diff(b.x, a.x),
        diff(b.y, a.y),
        diff(c.x, a.x),
        diff(c.y, a.y),
    )
    .cmp(&0)
}

fn on_segment(p: Point, a: Point, b: Point) -> bool {
    orient(a, b, p) == Ordering::Equal
        && p.x >= a.x.min(b.x)
        && p.x <= a.x.max(b.x)
        && p.y >= a.y.min(b.y)
        && p.y <= a.y.max(b.y)
}

/// How the segments `ab` and `cd` meet. Degenerate segments (a single point) are allowed.
#[must_use]
pub fn segments(a: Point, b: Point, c: Point, d: Point) -> Crossing {
    let turns = [
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    ];
    if turns.iter().all(|s| *s != Ordering::Equal) && turns[0] != turns[1] && turns[2] != turns[3] {
        return Crossing::Proper(intersection(a, b, c, d));
    }
    if turns.iter().all(|s| *s == Ordering::Equal) {
        // Collinear: the lattice order (y, then x) is monotone along the line.
        let (lo1, hi1) = if a <= b { (a, b) } else { (b, a) };
        let (lo2, hi2) = if c <= d { (c, d) } else { (d, c) };
        let (start, end) = (lo1.max(lo2), hi1.min(hi2));
        return match start.cmp(&end) {
            Ordering::Greater => Crossing::None,
            Ordering::Equal => Crossing::Touch(PointQ::from_point(start)),
            Ordering::Less => Crossing::Overlap,
        };
    }
    [(c, a, b), (d, a, b), (a, c, d), (b, c, d)]
        .into_iter()
        .find(|&(p, s, e)| on_segment(p, s, e))
        .map_or(Crossing::None, |(p, _, _)| {
            Crossing::Touch(PointQ::from_point(p))
        })
}

/// The crossing point of two segments known to cross properly: `a + (b - a) * t / u`.
fn intersection(a: Point, b: Point, c: Point, d: Point) -> PointQ {
    let (dir_x, dir_y) = (diff(b.x, a.x), diff(b.y, a.y));
    let (other_x, other_y) = (diff(d.x, c.x), diff(d.y, c.y));
    let denom = cross(dir_x, dir_y, other_x, other_y);
    let along = cross(diff(c.x, a.x), diff(c.y, a.y), other_x, other_y);
    PointQ::new(
        i128::from(a.x) * denom + dir_x * along,
        i128::from(a.y) * denom + dir_y * along,
        denom,
    )
}

/// Where `p` lies against `polygon`: on a ring is `Boundary`, otherwise the parity of
/// crossings of a ray towards +x over the outer ring and the holes decides.
#[must_use]
pub fn contains(polygon: &Polygon, p: Point) -> Containment {
    let mut inside = false;
    for ring in std::iter::once(&polygon.outer).chain(&polygon.holes) {
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            if on_segment(p, a, b) {
                return Containment::Boundary;
            }
            if (a.y > p.y) != (b.y > p.y) {
                // The ray crosses when p lies left of the edge's x at p.y.
                let side = orient(a, b, p);
                let crosses = if b.y > a.y {
                    side == Ordering::Greater
                } else {
                    side == Ordering::Less
                };
                inside ^= crosses;
            }
        }
    }
    if inside {
        Containment::Inside
    } else {
        Containment::Outside
    }
}
