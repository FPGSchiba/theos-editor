use std::cmp::Ordering;

use super::lattice::*;
use super::predicates::*;

fn p(x: i32, y: i32) -> Point {
    Point::new(x, y)
}

fn q(x: i128, y: i128, den: i128) -> PointQ {
    PointQ { x, y, den }
}

#[test]
fn snap_rounds_half_away_from_zero() {
    assert_eq!(snap(0.03125), Ok(1));
    assert_eq!(snap(-0.03125), Ok(-1));
    assert_eq!(snap(0.09375), Ok(2));
    assert_eq!(snap(2400.0), Ok(38_400));
}

#[test]
fn a_134_km_window_fits_and_a_larger_one_is_refused() {
    assert_eq!(snap(134_000.0), Ok(2_144_000));
    assert!(matches!(
        snap(134_218_000.0),
        Err(LatticeError::OutOfRange { .. })
    ));
    assert!(matches!(
        snap(f64::NAN),
        Err(LatticeError::OutOfRange { .. })
    ));
    assert!(matches!(
        snap(f64::INFINITY),
        Err(LatticeError::OutOfRange { .. })
    ));
    // The cap itself is allowed, one unit beyond it is not.
    assert_eq!(snap(67_108_864.0), Ok(1 << 30));
    assert!(snap(67_108_864.062_5).is_err());
    assert_eq!(snap(-67_108_864.0), Ok(-(1 << 30)));
}

#[test]
fn to_m_is_exact() {
    assert_eq!(to_m(1).to_bits(), 0.0625_f64.to_bits());
    assert_eq!(to_m(38_400).to_bits(), 2400.0_f64.to_bits());
    assert_eq!(point_m(p(16, -8)), [1.0, -0.5]);
}

#[test]
fn orientation_and_crossings_use_integers() {
    let m = i32::MAX;
    assert_eq!(orient(p(0, 0), p(4, 0), p(0, 4)), Ordering::Greater);
    assert_eq!(orient(p(0, 0), p(4, 0), p(0, -4)), Ordering::Less);
    assert_eq!(orient(p(0, 0), p(4, 4), p(8, 8)), Ordering::Equal);
    // Differences reach 2^32: no overflow.
    assert_eq!(orient(p(-m, -m), p(m, -m), p(m, m)), Ordering::Greater);
    assert_eq!(orient(p(-m, -m), p(m, m), p(0, 0)), Ordering::Equal);

    // Proper.
    assert_eq!(
        segments(p(0, 0), p(4, 4), p(0, 4), p(4, 0)),
        Crossing::Proper(q(2, 2, 1))
    );
    assert_eq!(
        segments(p(0, 0), p(3, 0), p(1, -1), p(2, 1)),
        Crossing::Proper(q(3, 0, 2))
    );
    // Touching: endpoint on the interior of the other, and shared endpoint.
    assert_eq!(
        segments(p(0, 0), p(4, 0), p(2, 0), p(2, 5)),
        Crossing::Touch(q(2, 0, 1))
    );
    assert_eq!(
        segments(p(0, 0), p(4, 0), p(4, 0), p(8, 3)),
        Crossing::Touch(q(4, 0, 1))
    );
    // Collinear: overlap, end-to-end touch, gap.
    assert_eq!(
        segments(p(0, 0), p(4, 0), p(2, 0), p(8, 0)),
        Crossing::Overlap
    );
    assert_eq!(
        segments(p(0, 0), p(4, 0), p(4, 0), p(8, 0)),
        Crossing::Touch(q(4, 0, 1))
    );
    assert_eq!(segments(p(0, 0), p(4, 0), p(5, 0), p(8, 0)), Crossing::None);
    // Parallel and plain misses.
    assert_eq!(segments(p(0, 0), p(4, 0), p(0, 1), p(4, 1)), Crossing::None);
    assert_eq!(segments(p(0, 0), p(1, 1), p(3, 0), p(0, 3)), Crossing::None);
    // Degenerate segment: a point on, and off, a segment.
    assert_eq!(
        segments(p(2, 0), p(2, 0), p(0, 0), p(4, 0)),
        Crossing::Touch(q(2, 0, 1))
    );
    assert_eq!(segments(p(2, 1), p(2, 1), p(0, 0), p(4, 0)), Crossing::None);
    // Near i32::MAX: proper crossing with huge numerators, and a touch.
    assert_eq!(
        segments(p(-m, -m), p(m, m), p(-m, m), p(m, -m)),
        Crossing::Proper(q(0, 0, 1))
    );
    assert_eq!(
        segments(p(m, m - 2), p(m, m), p(m - 1, m - 1), p(m, m - 1)),
        Crossing::Touch(q(i128::from(m), i128::from(m) - 1, 1))
    );
}

#[test]
fn area_sign_follows_the_winding() {
    let sq = [p(0, 0), p(16, 0), p(16, 16), p(0, 16)];
    assert_eq!(twice_area(&sq), 512);
    let rev: Vec<Point> = sq.iter().rev().copied().collect();
    assert_eq!(twice_area(&rev), -512);
}

#[test]
fn contains_tells_inside_outside_boundary_and_holes() {
    let poly = Polygon {
        outer: vec![p(0, 0), p(16, 0), p(16, 16), p(0, 16)],
        holes: vec![vec![p(4, 4), p(4, 8), p(8, 8), p(8, 4)]],
    };
    assert!(twice_area(&poly.holes[0]) < 0);
    assert_eq!(contains(&poly, p(2, 2)), Containment::Inside);
    assert_eq!(contains(&poly, p(6, 6)), Containment::Outside);
    assert_eq!(contains(&poly, p(4, 6)), Containment::Boundary);
    assert_eq!(contains(&poly, p(0, 0)), Containment::Boundary);
    assert_eq!(contains(&poly, p(16, 8)), Containment::Boundary);
    assert_eq!(contains(&poly, p(20, 8)), Containment::Outside);
    assert_eq!(contains(&poly, p(-1, 8)), Containment::Outside);
    assert_eq!(contains(&poly, p(10, 8)), Containment::Inside);
}
