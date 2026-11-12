//! The deterministic terrain generator for maps.
//!
//! The crate holds the raster type [`grid::Grid`] with its neighbour tables, the float and cast
//! helpers every kernel shares ([`kernels::num`]) and the per-stage random streams ([`rng`]).
//! The same inputs give the same bytes on every platform.

pub mod grid;
pub mod kernels;
pub mod rng;
