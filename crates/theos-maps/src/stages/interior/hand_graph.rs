//! CE-80.3: the interior stages on a hand-made room graph (spec §5.5, §5.16).
//!
//! The test-only stage `hand_graph` replaces `mission`: it reads the `lattice` of `envelope` (a
//! 6 x 6 rect lattice without offsets) and writes a fixed `room_graph`, and `space`, `cover` and
//! `playability` run on it unchanged. The graph has seven rooms, a main cycle around a 2 x 2
//! goal hall, one lock with its key on the other arc, and a drop out of the goal:
//!
//! ```text
//!   col:   1        2        3
//! row 1           [goal    goal]
//! row 2  A3 -L1- [goal    goal]
//! row 3  A2               B2 (drop from the goal)
//! row 4  A1 ---- start ---- B1 (key-1)
//! ```

use serde::{Deserialize, Serialize};

use crate::RunError;
use crate::rooms::lattice::Lattice;
use crate::rooms::{Cell, Connection, LatticeRef, Passage, Role, Room, RoomGraph, Shape};
use crate::runner::Execution;
use crate::stage::{
    LayerSpec, MapKind, Registry, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::interior::playability_core::Analysis;
use crate::stages::interior::support::{ROOM_GRAPH, execute_recipe};
use crate::world::{Layer, ValueType};

/// The parameters of `hand_graph`.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct HandParams {
    /// Puts key 1 in the goal, behind its own lock, instead of on the open arc.
    key_behind_lock: bool,
}

/// The test stage `hand_graph`.
struct HandGraph;

fn room(ty: &str, role: Role, cells: &[(u16, u16)]) -> Room {
    Room {
        ty: ty.to_owned(),
        role,
        cells: cells.iter().map(|&(row, col)| Cell { row, col }).collect(),
        shape: Shape::Rect,
        keys: Vec::new(),
        locked: false,
    }
}

fn link(a: u16, b: u16, kind: Passage) -> Connection {
    Connection {
        a,
        b,
        kind,
        main: true,
    }
}

/// The seven-room graph of the module docs on a lattice of `size`.
fn graph(size: LatticeRef, key_behind_lock: bool) -> RoomGraph {
    let mut rooms = vec![
        room("entrance", Role::Start, &[(4, 2)]),
        room("hall", Role::Goal, &[(1, 2), (1, 3), (2, 2), (2, 3)]),
        room("gallery", Role::Path, &[(4, 1)]),
        room("gallery", Role::Path, &[(3, 1)]),
        room("gallery", Role::Path, &[(2, 1)]),
        room("store", Role::Path, &[(4, 3)]),
        room("store", Role::Path, &[(3, 3)]),
    ];
    let holder = if key_behind_lock { 1 } else { 5 };
    rooms[holder].keys.push(1);
    RoomGraph {
        rooms,
        connections: vec![
            link(0, 2, Passage::Open),
            link(0, 5, Passage::Open),
            link(1, 4, Passage::Locked(1)),
            link(1, 6, Passage::Drop { from: 1 }),
            link(2, 3, Passage::Open),
            link(3, 4, Passage::Open),
            link(5, 6, Passage::Open),
        ],
        lattice: size,
    }
}

impl TypedStage for HandGraph {
    type Params = HandParams;
    const ID: &'static str = "hand_graph";
    const SUMMARY: &'static str = "A hand-made room graph (tests only).";
    const KINDS: &'static [MapKind] = &[MapKind::Interior];

    fn reads(&self, _: &HandParams) -> Vec<LayerSpec> {
        vec![LayerSpec::value("lattice", Lattice::NAME)]
    }

    fn writes(&self, _: &HandParams) -> Vec<LayerSpec> {
        vec![ROOM_GRAPH]
    }

    fn run(&self, ctx: &mut StageContext<'_>, p: &HandParams) -> Result<StageReport, StageError> {
        let lattice = ctx.value::<Lattice>("lattice")?;
        let size = LatticeRef {
            rows: lattice.rows,
            cols: lattice.cols,
        };
        let mut g = graph(size, p.key_behind_lock);
        g.mark_locked();
        g.validate(&|c| lattice.is_enabled(c))
            .map_err(|e| ctx.fail(e.to_string()))?;
        ctx.write("room_graph", Layer::Value(Box::new(g)))?;
        Ok(StageReport::default())
    }
}

/// A 48 x 48 interior at 1 m: a 6 x 6 rect lattice of 8 m without offsets, `hand_graph` in
/// place of `mission`.
fn recipe(key_behind_lock: bool) -> String {
    format!(
        "id = \"hand\"\ntitle = \"Hand-made graph\"\nkind = \"interior\"\n\
         grid = [48, 48]\nsize_km = [0.048, 0.048]\n\
         [[stage]]\nid = \"envelope\"\nshape = \"rect\"\npitch_m = 8.0\n\
         [[stage]]\nid = \"hand_graph\"\nkey_behind_lock = {key_behind_lock}\n\
         [[stage]]\nid = \"space\"\n\
         [[stage]]\nid = \"cover\"\n\
         [[stage]]\nid = \"playability\"\n"
    )
}

/// Plans and runs the recipe with the built-in stages plus `hand_graph`.
fn run(key_behind_lock: bool) -> Result<Execution, RunError> {
    let mut registry = Registry::builtin();
    registry.add(Box::new(HandGraph));
    execute_recipe(&recipe(key_behind_lock), &registry, 1)
}

// AC: CE-80.3
#[test]
fn hand_graph_interior_passes_playability() {
    let done = run(false).unwrap();
    let lattice = done.world.value::<Lattice>("lattice").unwrap();
    assert_eq!((lattice.rows, lattice.cols), (6, 6));
    assert!(lattice.enabled.iter().all(|&e| e), "a rect without margin");
    let g = done.world.value::<RoomGraph>("room_graph").unwrap();
    assert!(g.rooms[1].locked, "the goal lies behind lock 1");
    assert!(!g.rooms[0].locked, "the start never does");
    let a = done.world.value::<Analysis>("analysis").unwrap();
    assert_eq!(a.findings, []);
    assert_eq!(
        (a.rooms, a.connections, a.cycles, a.locks, a.one_way),
        (7.0, 7.0, 1.0, 1.0, 1.0)
    );
    // The route fetches the key on arc B, comes back and opens the lock on arc A.
    assert_eq!(a.route, [0, 5, 0, 2, 3, 4, 1]);
    assert!(a.critical_path_m > 0.0);
    let metrics = &done
        .stage_metrics
        .iter()
        .find(|(id, _)| id == "playability")
        .unwrap()
        .1;
    assert_eq!(metrics.0.get("rooms"), Some(&7.0));
    assert!(
        !metrics.0.contains_key("start_to_exit_min_m"),
        "no exit room"
    );
}

// AC: CE-80.3
#[test]
fn hand_graph_with_key_behind_its_lock_fails() {
    match run(true) {
        Err(RunError::FindingFailed { stage, message }) => {
            assert_eq!(stage, "playability");
            assert!(message.starts_with("unreachable_room"), "{message}");
            assert!(message.contains("room-2"), "the goal: {message}");
            assert!(message.contains("key-1"), "{message}");
        }
        other => panic!("expected a failed finding, got {:?}", other.map(|_| ())),
    }
}
