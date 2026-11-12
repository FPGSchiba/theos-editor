//! The helpers of the unit tests, each in one place: a project in a temporary folder with a
//! workspace over it.

mod fixture;

pub(crate) use fixture::Fixture;

/// The result of a unit test.
pub(crate) type TestResult = Result<(), Box<dyn std::error::Error>>;
