//! Interior maps: the rooms of a building as a graph of lattice cells.

pub mod graph;
pub mod lattice;
pub mod measure;
pub mod solve;

pub use graph::{
    Cell, Connection, GraphError, KeyId, LatticeRef, Passage, Role, Room, RoomGraph, RoomId, Shape,
};
