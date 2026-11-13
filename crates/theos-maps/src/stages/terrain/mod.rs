//! The terrain stages of the outdoor floors, one module per stage.

pub mod climate;
pub mod ecology;
pub mod elements;
pub mod geology;
pub mod grid;
pub mod hydrology;
pub mod rocks;
pub mod settlements;

pub mod tectonics;

#[cfg(test)]
pub(crate) mod support;
