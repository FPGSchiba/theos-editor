//! The Thomas algorithm for tridiagonal systems.

use super::KernelError;

/// Solves `lower[i]·x[i-1] + diag[i]·x[i] + upper[i]·x[i+1] = rhs[i]` for `x`, which replaces
/// `rhs`. `lower[0]` and the last `upper` are never read. `scratch` holds the swept upper
/// coefficients and must be as long as `rhs`.
///
/// The Thomas algorithm needs no pivoting for the diagonally dominant systems of implicit
/// diffusion; the cost is linear in the length.
///
/// # Errors
/// [`KernelError::Shape`] when the slices differ in length; [`KernelError::Singular`] at a zero
/// pivot.
pub fn solve(
    lower: &[f64],
    diag: &[f64],
    upper: &[f64],
    rhs: &mut [f64],
    scratch: &mut [f64],
) -> Result<(), KernelError> {
    let n = rhs.len();
    if [lower.len(), diag.len(), upper.len(), scratch.len()]
        .iter()
        .any(|&len| len != n)
    {
        return Err(KernelError::Shape {
            rows: 1,
            cols: n,
            len: lower.len(),
        });
    }
    let mut previous = (0.0, 0.0);
    for i in 0..n {
        let below = if i == 0 { 0.0 } else { lower[i] };
        let pivot = diag[i] - below * previous.0;
        if pivot == 0.0 {
            return Err(KernelError::Singular { index: i });
        }
        let swept = upper[i] / pivot;
        let value = (rhs[i] - below * previous.1) / pivot;
        scratch[i] = swept;
        rhs[i] = value;
        previous = (swept, value);
    }
    for i in (0..n.saturating_sub(1)).rev() {
        rhs[i] -= scratch[i] * rhs[i + 1];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::solve;
    use crate::kernels::KernelError;

    #[test]
    fn solves_a_known_system() {
        // [2 1 0; 1 3 1; 0 1 2] x = [4, 10, 8]  ->  x = [1, 2, 3]
        let mut rhs = [4.0, 10.0, 8.0];
        solve(
            &[0.0, 1.0, 1.0],
            &[2.0, 3.0, 2.0],
            &[1.0, 1.0, 0.0],
            &mut rhs,
            &mut [0.0; 3],
        )
        .unwrap();
        for (got, want) in rhs.iter().zip([1.0, 2.0, 3.0]) {
            assert!((got - want).abs() < 1e-12, "{rhs:?}");
        }
    }

    #[test]
    fn solves_a_long_diffusion_matrix_to_the_residual() {
        let n = 500;
        let lower = vec![-0.4; n];
        let upper = vec![-0.4; n];
        let diag = vec![1.8; n];
        let b: Vec<f64> = (0..n)
            .map(|i| f64::from(u32::try_from(i % 7).unwrap()))
            .collect();
        let mut x = b.clone();
        solve(&lower, &diag, &upper, &mut x, &mut vec![0.0; n]).unwrap();
        for i in 0..n {
            let left = if i > 0 { lower[i] * x[i - 1] } else { 0.0 };
            let right = if i + 1 < n { upper[i] * x[i + 1] } else { 0.0 };
            assert!(
                (left + diag[i] * x[i] + right - b[i]).abs() < 1e-12,
                "row {i}"
            );
        }
    }

    #[test]
    fn empty_and_single_systems_work() {
        solve(&[], &[], &[], &mut [], &mut []).unwrap();
        let mut rhs = [6.0];
        solve(&[0.0], &[3.0], &[0.0], &mut rhs, &mut [0.0]).unwrap();
        assert!((rhs[0] - 2.0).abs() < 1e-15);
    }

    #[test]
    fn refuses_bad_lengths_and_a_zero_pivot() {
        let err = solve(
            &[0.0],
            &[1.0, 1.0],
            &[0.0, 0.0],
            &mut [1.0, 1.0],
            &mut [0.0; 2],
        );
        assert!(matches!(err, Err(KernelError::Shape { .. })));
        let err = solve(
            &[0.0, 1.0],
            &[1.0, 1.0],
            &[1.0, 0.0],
            &mut [1.0, 1.0],
            &mut [0.0; 2],
        );
        assert_eq!(err, Err(KernelError::Singular { index: 1 }));
    }
}
