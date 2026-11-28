//! Writes to git by record: what the editor's Changes screen does with a record or a file. Every
//! write runs under the repo lock ([`Workspace::hold`](crate::Workspace)), and every path goes to
//! git in git's own spelling, as status and diff gave it.

mod commit;
mod pull;
mod push;
mod stage;

pub use commit::{CommitError, Committed, commit};
pub use pull::{PullError, Pulled, abort_merge, pull};
pub use push::{PushFailed, push};
pub use stage::{stage_records, unstage_records};
