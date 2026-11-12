//! Numeric kernels shared by the generator stages.

pub mod complex;
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
