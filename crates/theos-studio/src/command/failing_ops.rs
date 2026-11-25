//! Disk operations that fail the calls a test chooses (ruling R4), so the rollback tests are
//! deterministic on every OS, root included.

use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use super::files::{DiskOps, FileOps};

/// An operation of [`FileOps`].
#[derive(PartialEq)]
pub(crate) enum Op {
    Write,
    Remove,
}

/// One injected failure: the `nth` call of `op` on a path that ends with `path`.
pub(crate) struct Fail {
    op: Op,
    path: &'static str,
    nth: usize,
    seen: usize,
}

impl Fail {
    pub(crate) fn new(op: Op, path: &'static str, nth: usize) -> Fail {
        Fail {
            op,
            path,
            nth,
            seen: 0,
        }
    }
}

/// The disk operations, except for the calls chosen to fail.
pub(crate) struct FailingOps {
    fails: Vec<Fail>,
}

impl FailingOps {
    /// Operations that fail the calls `fails` names and do every other one on the disk.
    pub(crate) fn new(fails: Vec<Fail>) -> FailingOps {
        FailingOps { fails }
    }

    fn hit(&mut self, op: &Op, target: &Path) -> io::Result<()> {
        for fail in &mut self.fails {
            if fail.op == *op && target.ends_with(fail.path) {
                fail.seen += 1;
                if fail.seen == fail.nth {
                    return Err(io::Error::new(ErrorKind::PermissionDenied, "injected"));
                }
            }
        }
        Ok(())
    }
}

impl FileOps for FailingOps {
    fn write(
        &mut self,
        target: &Path,
        bytes: &[u8],
        replace: bool,
        created: &mut Vec<PathBuf>,
    ) -> io::Result<()> {
        self.hit(&Op::Write, target)?;
        DiskOps.write(target, bytes, replace, created)
    }

    fn remove(&mut self, target: &Path) -> io::Result<bool> {
        self.hit(&Op::Remove, target)?;
        DiskOps.remove(target)
    }
}
