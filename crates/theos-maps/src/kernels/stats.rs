//! Statistics with numpy's semantics.

use super::KernelError;
use super::num::{key, max, min, round_half_even, to_f64, to_index};

/// A sorted copy of `values`, ordered by `total_cmp` on `-0.0`-normalised keys.
fn sorted(values: &[f64]) -> Vec<f64> {
    let mut v: Vec<f64> = values.iter().map(|&x| key(x)).collect();
    v.sort_by(f64::total_cmp);
    v
}

/// numpy `percentile(values, q)` with the default `linear` method; `q` is clamped to 0..=100.
///
/// # Errors
/// [`KernelError::Empty`] when `values` is empty.
pub fn percentile(values: &[f64], q: f64) -> Result<f64, KernelError> {
    if values.is_empty() {
        return Err(KernelError::Empty);
    }
    let ordered = sorted(values);
    let count = ordered.len();
    let virtual_index = to_f64(count - 1) * (min(max(q, 0.0), 100.0) / 100.0);
    let prev = to_index(virtual_index, count);
    let next = (prev + 1).min(count - 1);
    let frac = virtual_index - to_f64(prev);
    let (lo, hi) = (ordered[prev], ordered[next]);
    // numpy's `_lerp`: approach from the nearer end.
    let diff = hi - lo;
    Ok(if frac >= 0.5 {
        hi - diff * (1.0 - frac)
    } else {
        lo + diff * frac
    })
}

/// numpy `median(values)`: the middle value, or the mean of the two middle ones.
///
/// # Errors
/// [`KernelError::Empty`] when `values` is empty.
pub fn median(values: &[f64]) -> Result<f64, KernelError> {
    if values.is_empty() {
        return Err(KernelError::Empty);
    }
    let v = sorted(values);
    let n = v.len();
    Ok(if n % 2 == 1 {
        v[n / 2]
    } else {
        f64::midpoint(v[n / 2 - 1], v[n / 2])
    })
}

/// numpy `mean(values)`, summed left to right (numpy sums pairwise above eight values; the
/// results agree to about 1e-15 relative).
///
/// # Errors
/// [`KernelError::Empty`] when `values` is empty.
pub fn mean(values: &[f64]) -> Result<f64, KernelError> {
    if values.is_empty() {
        return Err(KernelError::Empty);
    }
    Ok(values.iter().sum::<f64>() / to_f64(values.len()))
}

/// `max(1, round(m / cell_m))`, rounding halves to even as Python's and numpy's `round` do.
#[must_use]
pub fn metres_to_cells(m: f64, cell_m: f64) -> usize {
    to_index(round_half_even(m / cell_m), usize::MAX).max(1)
}

#[cfg(test)]
mod tests {
    use super::super::test_data::*;
    use super::*;

    const V: [f64; 10] = [3.5, 1.0, 9.0, 4.25, 7.0, 2.0, 8.5, 6.0, 5.5, 0.5];

    #[test]
    fn percentiles_match_numpy() {
        for (q, e) in [(5.0, P5), (50.0, P50), (95.0, P95)] {
            assert_close(&[percentile(&V, q).unwrap()], &[e], 1e-12);
        }
        assert_close(&[median(&V).unwrap()], &[MEDIAN_EVEN], 1e-12);
        assert_close(&[median(&V[..9]).unwrap()], &[MEDIAN_ODD], 1e-12);
        assert_close(&[mean(&V).unwrap()], &[MEAN], 1e-12);
    }

    #[test]
    fn percentile_edges_and_single_value() {
        assert_close(&[percentile(&V, 0.0).unwrap()], &[0.5], 1e-15);
        assert_close(&[percentile(&V, 100.0).unwrap()], &[9.0], 1e-15);
        assert_close(&[percentile(&[4.0], 37.0).unwrap()], &[4.0], 1e-15);
    }

    #[test]
    fn empty_input_is_an_error() {
        assert_eq!(percentile(&[], 50.0), Err(KernelError::Empty));
        assert_eq!(median(&[]), Err(KernelError::Empty));
        assert_eq!(mean(&[]), Err(KernelError::Empty));
    }

    #[test]
    fn metres_to_cells_rounds_and_is_at_least_one() {
        assert_eq!(metres_to_cells(100.0, 30.0), 3);
        assert_eq!(metres_to_cells(1.0, 30.0), 1);
        assert_eq!(metres_to_cells(0.0, 30.0), 1);
        assert_eq!(metres_to_cells(75.0, 30.0), 2); // 2.5 rounds to even, as Python's round
        assert_eq!(metres_to_cells(105.0, 30.0), 4); // 3.5
    }
}
