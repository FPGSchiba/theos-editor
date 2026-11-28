//! Numeric kernels shared by the generator stages.

pub mod complex;
pub mod contour;
pub mod curves;
pub mod czt;
pub mod diff;
pub mod distance;
pub mod fft;
pub mod filters;
pub mod flood;
pub mod geometry;
pub mod labels;
pub mod morphology;
pub mod mst;
pub mod noise;
pub mod num;
pub mod outline;
pub mod paths;
pub mod reflect;
pub mod resample;
pub mod routing;
pub mod stats;
pub mod tridiagonal;
pub mod voronoi;

#[cfg(test)]
#[expect(
    clippy::unreadable_literal,
    reason = "reference values are pasted verbatim from numpy and scipy"
)]
mod test_data;

/// How many units of work (queue pops, inserts, queries) pass between two calls of a kernel's
/// cancel callback: often enough to stop well within a second, rarely enough to cost nothing.
pub(crate) const CANCEL_EVERY: usize = 4096;

/// A kernel's cancel point after `done` units of work: calls `cancel` when `done` is a multiple
/// of [`CANCEL_EVERY`].
///
/// # Errors
/// [`KernelError::Cancelled`] when `cancel` is called and returns `true`.
pub(crate) fn cancel_point(done: usize, cancel: &dyn Fn() -> bool) -> Result<(), KernelError> {
    if done.is_multiple_of(CANCEL_EVERY) && cancel() {
        Err(KernelError::Cancelled)
    } else {
        Ok(())
    }
}

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
    /// A tridiagonal system with a zero pivot, which the Thomas algorithm cannot solve.
    #[error("the tridiagonal system has a zero pivot at row {index}")]
    Singular {
        /// Row of the zero pivot.
        index: usize,
    },
    /// The caller asked to stop a long kernel.
    #[error("the kernel was cancelled")]
    Cancelled,
    /// A resample from a parent grid with no cells to sample.
    #[error("the parent grid has no cells to sample")]
    NoCells,
    /// A resample with a cell size that is not positive and finite, or an origin that is not
    /// finite.
    #[error("the resample needs positive, finite cell sizes and a finite origin")]
    BadGeometry,
}
