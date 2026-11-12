//! The terrain stages of the outdoor floors, one module per stage.

pub mod climate;
pub mod geology;
pub mod grid;
pub mod rocks;
pub mod tectonics;

#[cfg(test)]
pub(crate) mod support;
