//! Interior maps: the mission graph of rooms on a lattice and the stages that build an interior
//! from it (spec §5.5): `envelope`, `mission`, `space`, `cover` and `playability`.

pub mod cover;
pub mod cover_core;
pub mod envelope;
#[cfg(test)]
mod hand_graph;
pub mod mission;
pub mod mission_core;
mod outline;
pub mod playability;
pub mod playability_core;
pub mod program;
pub mod rules;
pub mod shapes;
pub mod space;
pub mod space_core;
mod support;
pub mod units;
