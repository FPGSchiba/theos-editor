//! Lakes as `GeoJSON` polygons: one polygon per exterior ring, with the holes inside it. A lake is
//! one feature; its polygons make a `MultiPolygon` when there are several.
//!
//! The outline of a lake ([`LakeShape::rings`]) is a list of closed rings: exteriors with a
//! positive area and holes with a negative one. A `GeoJSON` polygon has one exterior, so a lake of
//! several parts (a lake and an island's own pond, or two basins that share a level) gives one
//! polygon per exterior, the parts of its `MultiPolygon`. The orientation is kept: exteriors
//! counter-clockwise and holes clockwise in the frame's (x, y), which is RFC 7946's right-hand
//! rule taken over the coordinates as written.

use super::FeaturesError;

/// A ring of `[x, y]` metres.
type Ring = Vec<[f64; 2]>;

/// The signed area of `ring` by the shoelace formula; positive for the exteriors the outlines
/// hold.
fn area(ring: &[[f64; 2]]) -> f64 {
    ring.windows(2)
        .fold(0.0, |sum, w| sum + (w[0][0] * w[1][1] - w[1][0] * w[0][1]))
        / 2.0
}

/// Whether `p` lies inside `ring`, by the even-odd rule.
fn contains(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    ring.windows(2).fold(false, |inside, w| {
        let (a, b) = (w[0], w[1]);
        let crosses = (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0];
        inside != crosses
    })
}

/// `ring` with its first vertex repeated at the end when it is not closed.
fn closed(ring: &[[f64; 2]]) -> Ring {
    let mut out = ring.to_vec();
    if let (Some(&first), Some(&last)) = (out.first(), out.last())
        && first.map(f64::to_bits) != last.map(f64::to_bits)
    {
        out.push(first);
    }
    out
}

/// The polygons of the rings of lake `lake`: each is an exterior followed by its holes.
///
/// # Errors
/// [`FeaturesError::OrphanHole`] for a hole inside no exterior.
pub(super) fn polygons(lake: u32, rings: &[Ring]) -> Result<Vec<Vec<Ring>>, FeaturesError> {
    let rings: Vec<Ring> = rings.iter().map(|r| closed(r)).collect();
    let mut polygons: Vec<(f64, Vec<Ring>)> = Vec::new();
    for ring in rings.iter().filter(|r| area(r) >= 0.0) {
        polygons.push((area(ring), vec![ring.clone()]));
    }
    for hole in rings.iter().filter(|r| area(r) < 0.0) {
        let first = hole.first().copied().unwrap_or([0.0, 0.0]);
        // The smallest exterior that contains the hole: nested islands do not own the hole of
        // the lake around them.
        let owner = polygons
            .iter_mut()
            .filter(|(_, p)| p.first().is_some_and(|outline| contains(outline, first)))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .ok_or(FeaturesError::OrphanHole { lake })?;
        owner.1.push(hole.clone());
    }
    Ok(polygons.into_iter().map(|(_, p)| p).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, s: f64, ccw: bool) -> Ring {
        let mut ring = vec![[x, y], [x + s, y], [x + s, y + s], [x, y + s], [x, y]];
        if !ccw {
            ring.reverse();
        }
        ring
    }

    #[test]
    fn a_lake_with_a_hole_and_an_island_gives_two_polygons() -> Result<(), FeaturesError> {
        let rings = vec![
            square(0.0, 0.0, 100.0, true),
            square(10.0, 10.0, 80.0, false),
            square(30.0, 30.0, 20.0, true),
            square(200.0, 0.0, 10.0, true),
        ];
        let polygons = polygons(1, &rings)?;
        assert_eq!(polygons.len(), 3);
        // The hole belongs to the outer exterior; the island inside it stands alone.
        assert_eq!(polygons[0].len(), 2);
        assert_eq!(polygons[1].len(), 1);
        assert_eq!(polygons[2].len(), 1);
        Ok(())
    }

    #[test]
    fn a_hole_inside_an_island_belongs_to_the_island() -> Result<(), FeaturesError> {
        let rings = vec![
            square(0.0, 0.0, 100.0, true),
            square(10.0, 10.0, 80.0, false),
            square(20.0, 20.0, 40.0, true),
            square(30.0, 30.0, 10.0, false),
        ];
        let polygons = polygons(1, &rings)?;
        assert_eq!(polygons.len(), 2);
        assert_eq!(polygons[0].len(), 2);
        assert_eq!(polygons[1].len(), 2);
        Ok(())
    }

    #[test]
    fn an_open_ring_is_closed_and_a_loose_hole_is_an_error() {
        let open = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let closed = closed(&open);
        assert_eq!(closed.len(), 5);
        assert_eq!(closed.first(), closed.last());
        let result = polygons(7, &[square(0.0, 0.0, 10.0, false)]);
        assert!(matches!(result, Err(FeaturesError::OrphanHole { lake: 7 })));
    }
}
