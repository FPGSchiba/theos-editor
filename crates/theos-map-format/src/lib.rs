//! The map package format: manifest, layers and their on-disk encoding.
//!
//! This crate holds the manifest types of `docs/map-package.md` §3 ([`manifest`]), the canonical
//! JSON writer ([`json`]) and the crate error ([`FormatError`]). Layer data, the package writer
//! and the reader follow in later tasks of phase P08.

pub mod error;
pub mod json;
pub mod manifest;

pub use error::FormatError;
pub use manifest::{DType, FORMAT, FORMAT_VERSION, Frame, LayerEntry, Level, Manifest, Parent};
