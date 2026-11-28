//! The id of a commit.

use std::fmt;

/// The id of a commit, as git spells it (full hexadecimal).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommitId(pub String);

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
