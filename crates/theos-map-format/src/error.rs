//! The error type of the crate.

/// Everything that can go wrong while reading or writing a map package.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// A JSON value could not be written or read.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
