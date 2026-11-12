//! Projects, layered settings, editor-part registry, command layer, agent sessions, review queue,
//! generators, LLM client and binaries folder.
//!
//! P06 adds the command layer; so far it has the repo lock ([`lock`]), which lets one process at
//! a time change a project. The rest arrives in P06 and P07.

pub mod lock;
