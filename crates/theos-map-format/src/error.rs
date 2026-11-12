//! The error type of the crate.

/// Everything that can go wrong while reading or writing a map package.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// A JSON value could not be written or read.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// A float layer holds NaN or an infinity (`docs/map-package.md` §4.2).
    #[error("layer value at index {index} is not finite (NaN and infinities are not allowed)")]
    NonFinite {
        /// Index of the first offending value.
        index: usize,
    },
    /// A layer file does not have `cells · size of dtype` bytes.
    #[error("layer has {found} bytes, expected {expected}")]
    LayerSize {
        /// The byte count the shape asks for.
        expected: usize,
        /// The byte count found.
        found: usize,
    },
}
