//! The helpers of the unit tests, each in one place: a project in a temporary folder with a
//! workspace over it, findings, plans, and the disk around a test.

mod codex;
mod disk;
mod findings;
mod fixture;
mod plans;

pub(crate) use codex::codex_workspace;
pub(crate) use disk::{ReadOnly, entries};
pub(crate) use findings::{error, warning};
pub(crate) use fixture::{Fixture, TRUTH, TRUTH_PATH};
pub(crate) use plans::{Planned, change, plan};

/// The result of a unit test.
pub(crate) type TestResult = Result<(), Box<dyn std::error::Error>>;
