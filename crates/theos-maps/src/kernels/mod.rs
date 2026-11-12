//! Numeric kernels shared by the generator stages.

pub mod complex;
pub mod fft;
pub mod noise;
pub mod num;

/// Errors of the numeric kernels.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KernelError {
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
