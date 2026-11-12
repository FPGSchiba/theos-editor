//! The deterministic terrain generator for maps.
//!
//! Every step of generation is a [`stage::Stage`]: it declares the layers of the [`world`] it
//! reads and writes, runs in a [`stage::StageContext`] that hands out only those layers, and is
//! listed once in the [`stage::Registry`] ([`stages`]). The crate also holds the raster type
//! [`grid::Grid`] with its neighbour tables, the float and cast helpers every kernel shares
//! ([`kernels::num`]), the per-stage random streams ([`rng`]) and the run's [`progress`] events
//! and cancel token. The same inputs give the same bytes on every platform.
//!
//! Engine callers must not set flush-to-zero (or denormals-are-zero) or change the
//! floating-point environment of the thread that calls the generator; the generator never does
//! (spec 5.7).

pub mod api;

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

pub use api::{
    Description, GENERATOR, ManifestFrame, Note, RecipeInfo, RunManifest, RunRequest, StageInfo,
    VERSION,
};
pub use error::RunError;
pub use progress::{CancelToken, Progress};
pub use recipe::RecipeProblem;
pub use runner::RunOptions;
