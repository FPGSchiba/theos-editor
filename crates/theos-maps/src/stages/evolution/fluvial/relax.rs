//! Aitken's dynamic relaxation of Yuan et al.'s Gauss–Seidel iterations once they stop
//! contracting (Irons & Tuck 1969; in the form Küttler & Wall 2008 give for fixed-point
//! iterations).
//!
//! **Why the iterations can need it.** One iteration maps the heights `h^k` to the sweep's
//! result `T(h^k)`. Its error moves as `e ↦ M·e` with `M = −L⁻¹·D`: `L` is the implicit
//! stream-power part that a sweep solves exactly (an M-matrix, so `L⁻¹ ≥ 0`) and `D ≥ 0` the
//! deposition term of the sediment that the sweep lags by one iteration. `M` is non-positive
//! cell by cell, so by Perron–Frobenius its largest eigenvalue in modulus is `−ρ` with a
//! non-negative eigenvector: the slowest error raises a whole patch of cells together, lowers it
//! in the next iteration, and so on. Per cell, the lagged term weighs `G·(A − area)/(Q +
//! G·area)` against 1, below 1 while the catchment's mean runoff weight `Q/A` is at least G:
//! Yuan et al.'s condition `G/p̂ ≤ 1` (spec §5.9). A real climate breaks it in dry catchments
//! (the 384² budget recipe has `p̂` from 0.005, below 0.5 on three tenths of the map). There
//! `ρ` can exceed 1, and the plain iterations then oscillate without end or diverge (Task 27b
//! measured `−1.009` for a dry patch at Δt 2.5e4).
//!
//! **What it does.** While the largest height change shrinks fast enough to fall below the
//! tolerance within the iteration limit at the rate of the last iteration, the iterations are
//! Yuan's plain sweeps, untouched. From the first iteration at which it would not (it grows,
//! stalls, or shrinks too slowly, as when `−ρ` lies just inside −1), they move the heights only
//! part of the way, `h^{k+1} = h^k + ω_k·r_k` with `r_k = T(h^k) − h^k`, and Aitken's secant
//! estimate `ω_k = −ω_{k−1}·r_{k−1}·(r_k − r_{k−1}) / |r_k − r_{k−1}|²` tends to `1/(1 + ρ)`
//! along the oscillating mode, which takes it out. The factor never exceeds 1 and never falls
//! below [`MIN_FACTOR`]. A fixed point of the relaxed iterations is a fixed point of the plain
//! ones, so the scheme and its solution stay those of spec §5.9; only the path to it changes.

use libm::pow;

use crate::kernels::num::{max, min};

/// The smallest relaxation factor: it still damps an oscillating mode with `ρ` up to 39.
const MIN_FACTOR: f64 = 0.05;

/// The state of the relaxation between iterations.
pub(super) struct Relaxation {
    /// The iterations stop once no height changes by this much, metres.
    tolerance: f64,
    /// The most iterations.
    max_iterations: u32,
    /// The iterations so far.
    iterations: u32,
    /// Whether an iteration was not on track to reach the tolerance within the limit.
    on: bool,
    /// The factor `ω` of the last iteration.
    factor: f64,
    /// The update `r` of the last iteration, empty before the first.
    last: Vec<f64>,
    /// The largest height change of the last iteration.
    last_residual: f64,
}

impl Relaxation {
    /// The relaxation of iterations that stop at `tolerance` (metres) or after
    /// `max_iterations`, before the first: off, factor 1.
    pub(super) fn new(tolerance: f64, max_iterations: u32) -> Self {
        Self {
            tolerance,
            max_iterations,
            iterations: 0,
            on: false,
            factor: 1.0,
            last: Vec::new(),
            last_residual: f64::INFINITY,
        }
    }

    /// Whether the largest height change `residual`, shrinking by the ratio of the last
    /// iteration from now on, stays at or above the tolerance through the last iteration.
    fn off_track(&self, residual: f64) -> bool {
        let ratio = residual / self.last_residual;
        let left = self.max_iterations.saturating_sub(self.iterations);
        ratio >= 1.0 || residual * pow(ratio, f64::from(left)) >= self.tolerance
    }

    /// The factor the last call used.
    #[cfg(test)]
    pub(super) fn factor(&self) -> f64 {
        self.factor
    }

    /// Relaxes one iteration: `next` holds the sweep's result `T(h^k)` for the heights `height`
    /// (`h^k`), and `residual` is the largest `|T(h^k) − h^k|`. Once on, `next` becomes `h^k +
    /// ω·(T(h^k) − h^k)` with the factor updated by Aitken's estimate; with a factor of 1 it is
    /// left as it is.
    pub(super) fn apply(&mut self, height: &[f64], next: &mut [f64], residual: f64) {
        let update: Vec<f64> = next.iter().zip(height).map(|(t, h)| t - h).collect();
        self.iterations += 1;
        self.on = self.on || self.off_track(residual);
        if self.on && self.last.len() == update.len() {
            let (mut along, mut squared) = (0.0, 0.0);
            for (now, before) in update.iter().zip(&self.last) {
                let change = now - before;
                along += before * change;
                squared += change * change;
            }
            let estimate = -self.factor * along / squared;
            if squared > 0.0 && estimate.is_finite() {
                self.factor = min(max(estimate, MIN_FACTOR), 1.0);
            }
        }
        if self.factor < 1.0 {
            for ((n, h), r) in next.iter_mut().zip(height).zip(&update) {
                *n = h + self.factor * r;
            }
        }
        self.last = update;
        self.last_residual = residual;
    }
}

#[cfg(test)]
mod tests {
    use super::{MIN_FACTOR, Relaxation};
    use crate::kernels::num::max;

    /// The iterations' stopping rule in the tests: Yuan's tolerance for a 1 m/kyr uplift over
    /// 25 kyr, and the default limit.
    const TOLERANCE: f64 = 0.025;
    const LIMIT: u32 = 50;

    /// The largest absolute difference between `a` and `b`.
    fn largest_change(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b).fold(0.0, |m, (x, y)| max(m, (x - y).abs()))
    }

    /// Iterates the map `x ↦ λ·x` (fixed point 0) with the relaxation, as `deposit` does, until
    /// the largest change falls below [`TOLERANCE`] or [`LIMIT`] iterations ran. Returns the
    /// iterations, the last factor, and whether any iteration was relaxed.
    fn iterate(lambda: f64) -> (u32, f64, bool) {
        let mut relaxation = Relaxation::new(TOLERANCE, LIMIT);
        let mut x = vec![10.0, 5.0, 20.0];
        let mut relaxed = false;
        for k in 1..=LIMIT {
            let sweep: Vec<f64> = x.iter().map(|v| lambda * v).collect();
            let residual = largest_change(&sweep, &x);
            if residual < TOLERANCE {
                return (k, relaxation.factor(), relaxed);
            }
            let mut next = sweep.clone();
            relaxation.apply(&x, &mut next, residual);
            relaxed |= next != sweep;
            x = next;
        }
        (LIMIT, relaxation.factor(), relaxed)
    }

    #[test]
    fn an_oscillating_mode_that_diverges_is_damped() {
        for lambda in [-1.01, -3.0, -10.0] {
            let (iterations, factor, relaxed) = iterate(lambda);
            // The plain second iteration does not shrink the change; from then on Aitken's
            // estimate, exact for a single mode, is 1/(1 − λ) and the error is gone.
            assert!(relaxed, "{lambda}");
            assert!(
                (factor - 1.0 / (1.0 - lambda)).abs() < 1e-12,
                "{lambda}: {factor}"
            );
            assert!(iterations <= 4, "{lambda}: {iterations}");
        }
    }

    #[test]
    fn a_slowly_shrinking_oscillation_is_damped() {
        // At −0.95 the plain iterations shrink the change, but too slowly: 50 of them leave it
        // at 0.95^50 ≈ 8 % of the start, far above the tolerance.
        let (iterations, factor, relaxed) = iterate(-0.95);
        assert!(relaxed);
        assert!((factor - 1.0 / 1.95).abs() < 1e-12, "{factor}");
        assert!(iterations <= 4, "{iterations}");
    }

    #[test]
    fn iterations_on_track_stay_the_plain_sweep() {
        for lambda in [-0.6, -0.3, 0.5] {
            let (iterations, factor, relaxed) = iterate(lambda);
            assert!(!relaxed, "{lambda}: the sweep's results untouched");
            assert_eq!(factor.to_bits(), 1.0_f64.to_bits(), "{lambda}");
            assert!(iterations < LIMIT, "{lambda}");
        }
    }

    #[test]
    fn the_factor_stays_above_its_floor() {
        // A monotone divergence (each update twice the last) asks for a negative factor.
        let mut relaxation = Relaxation::new(TOLERANCE, LIMIT);
        let mut next = vec![2.0];
        relaxation.apply(&[1.0], &mut next, 1.0);
        let mut next = vec![4.0];
        relaxation.apply(&[2.0], &mut next, 2.0);
        assert_eq!(relaxation.factor().to_bits(), MIN_FACTOR.to_bits());
        assert!((next[0] - (2.0 + MIN_FACTOR * 2.0)).abs() < 1e-15);
        // The same update again gives no estimate; the factor is kept.
        let mut next = vec![4.0];
        relaxation.apply(&[2.0], &mut next, 2.0);
        assert_eq!(relaxation.factor().to_bits(), MIN_FACTOR.to_bits());
    }
}
