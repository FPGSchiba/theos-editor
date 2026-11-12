//! The map package format: manifest, layers and their on-disk encoding.
//!
//! This crate holds the manifest types of `docs/map-package.md` §3 ([`manifest`]), the canonical
//! JSON writer ([`json`]) and the crate error ([`FormatError`]). Layer data ([`layer`]), package validation ([`package`]) and the
//! ZIP writer ([`mod@write`]) and reader ([`mod@read`]) follow.

pub mod error;
pub mod json;
pub mod layer;
pub mod manifest;
pub mod package;
pub mod read;
pub mod write;

pub use error::FormatError;
pub use layer::{LayerData, LayerKey, sha256_hex};
pub use manifest::{DType, FORMAT, FORMAT_VERSION, Frame, LayerEntry, Level, Manifest, Parent};
pub use package::Package;
pub use read::{IMPLEMENTED_CAPABILITIES, read};
pub use write::{write, write_cancellable, written_manifest};
