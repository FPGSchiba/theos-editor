//! Numeric kernels shared by the generator stages.

pub mod complex;
pub mod contour;
pub mod curves;
pub mod diff;
pub mod distance;
pub mod fft;
pub mod filters;
pub mod labels;
pub mod morphology;
pub mod noise;
pub mod num;
pub mod reflect;
pub mod stats;
pub mod voronoi;

#[cfg(test)]
#[expect(
    clippy::unreadable_literal,
    reason = "reference values are pasted verbatim from numpy and scipy"
)]
mod test_data;

/// Errors of the numeric kernels.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KernelError {
    /// A statistic of no values.
    #[error("no values to summarise")]
    Empty,
    /// A distance transform of a mask with no `true` cell.
    #[error("the mask has no true cell")]
    EmptyMask,
    /// An FFT length that is not a power of two.
    #[error("FFT length {len} is not a power of two")]
    NotPowerOfTwo {
        /// The length given.
        len: usize,
    },
    /// A point that cannot enter a triangulation: a NaN, an infinity, or a value spade refuses.
    #[error("point {index} is not finite or is out of range for the triangulation")]
    InvalidPoint {
        /// Index of the point (or grid cell) at fault.
        index: usize,
    },
    /// More points than a `u32` label can number.
    #[error("{len} points do not fit a u32 label")]
    TooMany {
        /// Number of points given.
        len: usize,
    },
    /// A data slice that does not hold `rows * cols` values.
    #[error("data of length {len} does not fit {rows} x {cols}")]
    Shape {
        /// Rows asked for.
        rows: usize,
        /// Columns asked for.
        cols: usize,
        /// Length of the data given.
        len: usize,
    },
}
