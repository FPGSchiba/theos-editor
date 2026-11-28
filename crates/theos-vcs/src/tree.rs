//! git's process tree: the per-OS code that starts git so that it can be stopped together with
//! every process it starts (P46 §5.3).
//!
//! [`Tree::spawn`] starts git; [`Tree::stop`] ends the whole tree on a cancel.
//! A tree that is not stopped is left as it is on Unix; on Windows, dropping the tree at the end
//! of the run closes its job, which ends any process of git's that is still running.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub(crate) use unix::Tree;
#[cfg(windows)]
pub(crate) use windows::Tree;
