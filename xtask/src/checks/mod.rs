//! The checks of `docs/gate.md` §1, one module each (or one per family).
//!
//! A check module has a pure core, inputs in and a [`Verdict`](crate::verdict::Verdict) out, and a
//! `run` function that gathers the real inputs. `main` and the gate call `run`.

pub mod cargo_cmds;
