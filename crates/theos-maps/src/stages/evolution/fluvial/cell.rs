//! The implicit step of one cell over its receivers (spec §5.9; Braun & Willett 2013,
//! generalised to the flow weights), with the receivers that take a share decided at the cell's
//! solved height.
//!
//! For a cell `i` the sweep knows `top = h*_i·(1 + c_i) + Δt·G·(Qs_in + V/Δt)/Q_i` and
//! `bottom = 1 + c_i` (without deposition `top = h*_i` and `bottom = 1`), so `top / bottom` is
//! the height the cell reaches without incision. Stream power erodes only downhill: a receiver
//! `r` (share `F_ir`, height `h_r`) takes a share exactly when it lies below the cell's new
//! height `h`. So `h` solves
//!
//! ```text
//! g(h) = h·bottom − top + Σ over r with h_r < h of F_ir·(h − h_r) = 0.
//! ```
//!
//! `g` is continuous, piecewise linear and strictly increasing (its slope is at least `bottom`,
//! above 0), so the solution is unique, and it never lies above `top / bottom`: incision never
//! raises a cell. [`solve`] finds it from the receivers below `top / bottom`, dropping those at
//! or above the solution until the two agree; nearly always the first guess holds.
//!
//! **Why the gate is decided at the solved height.** Deciding it at `top / bottom` instead, as
//! a predictor, counts a receiver that lies between the two: it then pulls the cell *up*. With
//! several receivers, such a receiver changes the result by `F_ir·(top/bottom − h')/(bottom +
//! Σ F + F_ir)` as it crosses `top / bottom`, with `h'` the result without it and `Σ F` the
//! shares of the others, so the sweep jumps. In the Gauss–Seidel iterations a receiver that
//! hovers there flips in and out and the iterations cycle on that one cell (Task 27d: every one
//! of the 28 steps of the 384² budget run that stopped at the iteration limit, with jumps of
//! 0.4 to 8.6 m). At the solved height the step is continuous. With one receiver both gates
//! agree up to rounding: they can differ only when the receiver lies within a few ulp of
//! `top / bottom`, and then by about as little.

use std::cmp::Ordering;

use crate::kernels::num::max;

/// One receiver of a cell in the implicit step.
#[derive(Debug, Clone, Copy)]
pub(super) struct Receiver {
    /// Its height in the sweep, m.
    pub height: f64,
    /// Its share `F_ir = K_i·Q_i^m·w_ir·Δt/d_ir`.
    pub share: f64,
}

/// The cell's new height: the solution `h` of `h·bottom − top + Σ F_ir·(h − h_r) = 0` over the
/// receivers below `h`. `bottom` is above 0 and the shares are not negative.
///
/// The first guess takes the receivers below `top / bottom` and solves with them. Where one of
/// them lies above that solution, the next round takes only the receivers below it, and so on.
/// Each solution is a weighted mean of `top / bottom` and the receivers taken, so dropping
/// receivers above it lowers it: the rounds take ever fewer receivers and stop, after at most one
/// round more than there are receivers below `top / bottom`, when no receiver taken lies above
/// the solution. A receiver exactly at the solution may stay: it adds nothing to `g`. Those left
/// out lie at or above the round's limit, which is not below its solution. Nearly always the
/// first guess holds, and the sums run in the receivers' order, so a cell whose receivers all lie
/// below it or above `top / bottom` gets the same bits as with the guess alone. (Sorting the
/// receivers by height and adding them from the lowest finds the same solution, but sorting
/// them in every cell of every sweep cost the 384² budget run about 9 s more in `fluvial`.)
///
/// A `top` that is NaN or infinite comes back as it is (NaN, or the infinity), so the runner
/// reports the layer as not finite: a round whose solution is NaN, or no lower than its highest
/// receiver, ends the loop.
pub(super) fn solve(top: f64, bottom: f64, receivers: &[Receiver]) -> f64 {
    let mut limit = top / bottom;
    loop {
        let (mut t, mut b, mut highest) = (top, bottom, f64::NEG_INFINITY);
        for r in receivers.iter().filter(|r| r.height < limit) {
            t += r.share * r.height;
            b += r.share;
            highest = max(highest, r.height);
        }
        let solved = t / b;
        if highest.partial_cmp(&solved) != Some(Ordering::Greater) {
            return solved;
        }
        limit = solved;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::{Receiver, solve};
    use crate::kernels::num::{max, min};
    use crate::rng::StageRng;

    /// `g(h) = h·(bottom + Σ f) − (top + Σ f·h_r)` over the receivers below `h`: zero exactly at
    /// the consistent step.
    fn residual(top: f64, bottom: f64, receivers: &[Receiver], h: f64) -> f64 {
        receivers
            .iter()
            .filter(|r| r.height < h)
            .fold(h * bottom - top, |g, r| g + r.share * (h - r.height))
    }

    /// Random cells (synthetic): 1 to 8 receivers from 60 m below to 20 m above the height
    /// without incision, the third 7 m below it, with shares up to 5.
    fn random_cell(rng: &mut StageRng) -> (f64, f64, Vec<Receiver>) {
        let bottom = rng.range(1.0, 3.0);
        let base = rng.range(0.0, 500.0);
        let count = 1 + (rng.next_u64() % 8);
        let receivers = (0..count)
            .map(|k| {
                let height = if k == 2 {
                    base - 7.0
                } else {
                    base + rng.range(-60.0, 20.0)
                };
                Receiver {
                    height,
                    share: rng.range(0.0, 5.0),
                }
            })
            .collect();
        (base * bottom, bottom, receivers)
    }

    #[test]
    fn the_step_counts_exactly_the_receivers_below_it() {
        let mut rng = StageRng::for_key(3, "cells");
        for case in 0..500 {
            let (top, bottom, receivers) = random_cell(&mut rng);
            let h = solve(top, bottom, &receivers);
            let scale = max(1.0, top.abs());
            let g = residual(top, bottom, &receivers, h);
            assert!(
                g.abs() <= 1e-12 * scale * 10.0,
                "case {case}: g(h) = {g} at h = {h}"
            );
            // Incision never raises a cell, and it never sinks below its lowest receiver.
            assert!(h <= top / bottom, "case {case}");
            let lowest = receivers
                .iter()
                .fold(f64::INFINITY, |m, r| min(m, r.height));
            assert!(h >= min(lowest, top / bottom), "case {case}");
        }
    }

    #[test]
    fn the_step_is_continuous_as_a_receiver_crosses_it() {
        // A receiver at 0 m pulls the cell (100 m without incision) well down; a second one
        // moves from 40 m to 120 m and crosses the cell's height on the way. The step may not
        // jump.
        let at = |second: f64| {
            let receivers = [
                Receiver {
                    height: 0.0,
                    share: 2.0,
                },
                Receiver {
                    height: second,
                    share: 0.5,
                },
            ];
            solve(100.0, 1.0, &receivers)
        };
        let mut last = at(40.0);
        for k in 1..=8000 {
            let second = 40.0 + 0.01 * f64::from(k);
            let now = at(second);
            assert!((now - last).abs() <= 0.01, "at {second}: {last} to {now}");
            last = now;
        }
        // Above the cell the second receiver takes no share: (100 + 2·0) / (1 + 2). At 90 m it
        // lies below 100 m, so the first guess takes it, but above the guess's 41.4 m: dropped.
        for second in [90.0, 120.0] {
            assert!((at(second) - 100.0 / 3.0).abs() < 1e-12, "{second}");
        }
    }

    #[test]
    fn one_receiver_is_braun_and_willetts_step() {
        let below = |height: f64| {
            let receivers = [Receiver { height, share: 1.5 }];
            solve(20.0, 2.0, &receivers)
        };
        assert!((below(4.0) - (20.0 + 1.5 * 4.0) / 3.5).abs() < 1e-12);
        assert_eq!(
            below(10.0).to_bits(),
            10.0_f64.to_bits(),
            "at the cell's height"
        );
        assert_eq!(below(12.0).to_bits(), 10.0_f64.to_bits(), "uphill");
        assert_eq!(solve(20.0, 2.0, &[]).to_bits(), 10.0_f64.to_bits());
    }

    #[test]
    fn a_height_that_is_not_finite_comes_back_without_hanging() {
        // A NaN or infinite height upstream reaches `top`. The step hands it on (the runner then
        // reports the layer as not finite) and does not loop. Each solve runs on its own thread
        // with a deadline, so a regression fails here instead of hanging the test run.
        for (top, nan) in [
            (f64::NAN, true),
            (f64::NEG_INFINITY, false),
            (f64::INFINITY, false),
        ] {
            let (send, receive) = mpsc::channel();
            thread::spawn(move || {
                let receivers = [
                    Receiver {
                        height: 0.0,
                        share: 1.0,
                    },
                    Receiver {
                        height: 5.0,
                        share: 2.0,
                    },
                ];
                send.send(solve(top, 1.0, &receivers)).ok();
            });
            let got = receive
                .recv_timeout(Duration::from_secs(10))
                .unwrap_or_else(|_| panic!("the step for top = {top} does not return"));
            if nan {
                assert!(got.is_nan(), "{got}");
            } else {
                assert_eq!(got.to_bits(), top.to_bits());
            }
        }
    }

    #[test]
    fn the_order_of_the_receivers_does_not_matter() {
        // The solution is unique; only the rounding of the sums follows the order.
        let mut rng = StageRng::for_key(5, "order");
        for _ in 0..200 {
            let (top, bottom, receivers) = random_cell(&mut rng);
            let backward: Vec<Receiver> = receivers.iter().rev().copied().collect();
            let (a, b) = (
                solve(top, bottom, &receivers),
                solve(top, bottom, &backward),
            );
            assert!((a - b).abs() <= 1e-12 * max(1.0, a.abs()), "{a} vs {b}");
        }
    }
}
