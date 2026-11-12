//! The deterministic terrain generator for maps.
//!
//! Every step of generation is a [`stage::Stage`]: it declares the layers of the [`world`] it
//! reads and writes, runs in a [`stage::StageContext`] that hands out only those layers, and is
//! listed once in the [`stage::Registry`] ([`stages`]). The crate also holds the raster type
//! [`grid::Grid`] with its neighbour tables, the float and cast helpers every kernel shares
//! ([`kernels::num`]), the per-stage random streams ([`rng`]) and the run's [`progress`] events
//! and cancel token. The same inputs give the same bytes on every platform.

pub mod constraints;
pub mod error;
pub mod features;
pub mod golden;
pub mod grid;
pub mod kernels;
pub mod metrics;
pub mod progress;
pub mod recipe;
pub mod rng;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "wired into the public `run` by Task 41")
)]
pub mod runner;
pub mod stage;
pub mod stages;
pub mod world;
