//! Writes to git by record: what the editor's Changes screen does with a record or a file. Every
//! write runs under the repo lock ([`Workspace::hold`](crate::Workspace)), and every path goes to
//! git in git's own spelling, as status and diff gave it.

mod commit;
mod stage;

pub use commit::{CommitError, Committed, commit};
pub use stage::{stage_records, unstage_records};
