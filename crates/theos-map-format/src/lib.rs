//! The map package format: manifest, layers and their on-disk encoding.
//!
//! The crate holds the manifest types of `docs/map-package.md` §3 ([`manifest`]), the raster
//! layers and their encoding ([`layer`]), the [`Package`] with its validation ([`package`]), the
//! ZIP writer ([`mod@write`]) and reader ([`mod@read`]), the canonical JSON writer ([`json`]) and
//! the crate error ([`FormatError`]). Equal packages give equal bytes on every platform.

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
