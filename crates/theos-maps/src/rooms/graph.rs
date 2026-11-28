//! The `room_graph` world value: rooms on lattice cells and the connections between them.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::world::{ValueType, WorldValue};

/// A lattice cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Cell {
    /// The lattice row.
    pub row: u16,
    /// The lattice column.
    pub col: u16,
}

impl Cell {
    /// Whether the two cells share an edge.
    #[must_use]
    pub fn touches(self, other: Cell) -> bool {
        self.row.abs_diff(other.row) + self.col.abs_diff(other.col) == 1
    }
}

/// The number of a room: its index in [`RoomGraph::rooms`].
pub type RoomId = u16;
/// The number of a key, from 1.
pub type KeyId = u8;

/// What a room is for in the mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Where the party enters.
    Start,
    /// The room the mission leads to.
    Goal,
    /// A room on a route.
    Path,
    /// A dead end.
    Leaf,
    /// A room where a party can leave.
    Exit,
}

/// The outline of a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    /// Rectangular.
    Rect,
    /// Rounded.
    Round,
    /// Irregular.
    Ragged,
    /// A narrow passage.
    Passage,
}

/// How a connection can be walked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Passage {
    /// Always walkable both ways.
    Open,
    /// Needs the key.
    Locked(KeyId),
    /// One way, from the named room to the other.
    Drop {
        /// The room the drop starts in.
        from: RoomId,
    },
}

/// One room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    /// The room type, e.g. `choir-hall`.
    pub ty: String,
    /// The role in the mission.
    pub role: Role,
    /// The lattice cells, sorted.
    pub cells: Vec<Cell>,
    /// The outline.
    pub shape: Shape,
    /// The keys found here.
    pub keys: Vec<KeyId>,
    /// Whether the room lies behind a lock.
    pub locked: bool,
}

/// A connection between two rooms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    /// The lower room number.
    pub a: RoomId,
    /// The higher room number.
    pub b: RoomId,
    /// How it can be walked.
    pub kind: Passage,
    /// Whether it lies on the main cycle.
    pub main: bool,
}

/// The size of the lattice the cells refer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatticeRef {
    /// Lattice rows.
    pub rows: u16,
    /// Lattice columns.
    pub cols: u16,
}

/// Rooms and connections of an interior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomGraph {
    /// The rooms; the index is the room number.
    pub rooms: Vec<Room>,
    /// The connections, sorted by `(a, b)`.
    pub connections: Vec<Connection>,
    /// The lattice the cells refer to.
    pub lattice: LatticeRef,
}

/// A rule of the room graph that does not hold.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GraphError {
    /// A room lies on a cell that is not enabled.
    #[error("room {room} lies on the disabled cell ({}, {})", .cell.row, .cell.col)]
    CellNotEnabled {
        /// The room.
        room: RoomId,
        /// The cell.
        cell: Cell,
    },
    /// A cell belongs to two rooms, or twice to one, or a cell list is not sorted.
    #[error("cell ({}, {}) is shared or its list is not sorted", .cell.row, .cell.col)]
    CellShared {
        /// The cell.
        cell: Cell,
    },
    /// A connection joins rooms that do not touch (or names a missing room).
    #[error("rooms {a} and {b} are not adjacent")]
    NotAdjacent {
        /// One room.
        a: RoomId,
        /// The other room.
        b: RoomId,
    },
    /// The number of `start` rooms is not one.
    #[error("expected one start room, found {0}")]
    StartCount(usize),
    /// The number of `goal` rooms is not one.
    #[error("expected one goal room, found {0}")]
    GoalCount(usize),
    /// A lock names a key that lies in no room.
    #[error("{} lies in no room", RoomGraph::key_name(*.0))]
    KeyMissing(KeyId),
    /// Connections are not sorted by `(a, b)` or have `a >= b`.
    #[error("connection {a}-{b} is out of order")]
    ConnectionOrder {
        /// The first room of the connection.
        a: RoomId,
        /// The second room of the connection.
        b: RoomId,
    },
    /// A key id lies outside 1..=16 (a room's key or a lock's).
    #[error("key id {0} lies outside 1..=16")]
    KeyOutOfRange(KeyId),
    /// A room type is not lower case with hyphens.
    #[error("room type {0:?} must match ^[a-z][a-z0-9-]*$")]
    BadType(String),
    /// A drop starts in a room that is neither end of its connection.
    #[error("drop {a}-{b} starts in room {from}, which is neither end")]
    DropEnd {
        /// The first room of the connection.
        a: RoomId,
        /// The second room of the connection.
        b: RoomId,
        /// The room the drop claims to start in.
        from: RoomId,
    },
}

/// The key ids a graph may use: keys are held as a 16-bit mask.
pub const KEY_IDS: std::ops::RangeInclusive<KeyId> = 1..=16;

/// Whether a room type is lower case with hyphens, `^[a-z][a-z0-9-]*$`.
pub(crate) fn type_ok(ty: &str) -> bool {
    let mut chars = ty.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

impl RoomGraph {
    /// The name of a key in features and renders: `key-<k>`.
    #[must_use]
    pub fn key_name(k: KeyId) -> String {
        format!("key-{k}")
    }

    /// The rooms joined to `r` and the connections that join them (both directions).
    pub fn neighbours(&self, r: RoomId) -> impl Iterator<Item = (RoomId, &Connection)> {
        self.connections.iter().filter_map(move |c| {
            if c.a == r {
                Some((c.b, c))
            } else if c.b == r {
                Some((c.a, c))
            } else {
                None
            }
        })
    }

    /// Checks the rules of the value; `enabled` says which lattice cells may hold a room.
    ///
    /// # Errors
    /// The first [`GraphError`] found.
    pub fn validate(&self, enabled: &dyn Fn(Cell) -> bool) -> Result<(), GraphError> {
        let mut owner: BTreeMap<Cell, usize> = BTreeMap::new();
        for (i, room) in self.rooms.iter().enumerate() {
            if !type_ok(&room.ty) {
                return Err(GraphError::BadType(room.ty.clone()));
            }
            if let Some(&k) = room.keys.iter().find(|k| !KEY_IDS.contains(k)) {
                return Err(GraphError::KeyOutOfRange(k));
            }
            let room_id = RoomId::try_from(i).unwrap_or(RoomId::MAX);
            for (j, &cell) in room.cells.iter().enumerate() {
                if !enabled(cell) {
                    return Err(GraphError::CellNotEnabled {
                        room: room_id,
                        cell,
                    });
                }
                if j > 0 && room.cells[j - 1] >= cell {
                    return Err(GraphError::CellShared { cell });
                }
                if owner.insert(cell, i).is_some() {
                    return Err(GraphError::CellShared { cell });
                }
            }
        }
        let starts = self.rooms.iter().filter(|r| r.role == Role::Start).count();
        if starts != 1 {
            return Err(GraphError::StartCount(starts));
        }
        let goals = self.rooms.iter().filter(|r| r.role == Role::Goal).count();
        if goals != 1 {
            return Err(GraphError::GoalCount(goals));
        }
        let mut previous: Option<(RoomId, RoomId)> = None;
        for c in &self.connections {
            if c.a >= c.b || previous.is_some_and(|p| p >= (c.a, c.b)) {
                return Err(GraphError::ConnectionOrder { a: c.a, b: c.b });
            }
            previous = Some((c.a, c.b));
            if let Passage::Locked(k) = c.kind
                && !KEY_IDS.contains(&k)
            {
                return Err(GraphError::KeyOutOfRange(k));
            }
            let (Some(ra), Some(rb)) = (
                self.rooms.get(usize::from(c.a)),
                self.rooms.get(usize::from(c.b)),
            ) else {
                return Err(GraphError::NotAdjacent { a: c.a, b: c.b });
            };
            let touching = ra
                .cells
                .iter()
                .any(|&x| rb.cells.iter().any(|&y| x.touches(y)));
            if !touching {
                return Err(GraphError::NotAdjacent { a: c.a, b: c.b });
            }
            if let Passage::Drop { from } = c.kind
                && from != c.a
                && from != c.b
            {
                return Err(GraphError::DropEnd {
                    a: c.a,
                    b: c.b,
                    from,
                });
            }
        }
        let held: BTreeSet<KeyId> = self
            .rooms
            .iter()
            .flat_map(|r| r.keys.iter().copied())
            .collect();
        for c in &self.connections {
            if let Passage::Locked(k) = c.kind
                && !held.contains(&k)
            {
                return Err(GraphError::KeyMissing(k));
            }
        }
        Ok(())
    }
}

impl WorldValue for RoomGraph {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        theos_map_format::json::to_checked_value(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for RoomGraph {
    const NAME: &'static str = "room_graph";
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(role: Role, cells: &[(u16, u16)]) -> Room {
        Room {
            ty: "hall".to_owned(),
            role,
            cells: cells.iter().map(|&(row, col)| Cell { row, col }).collect(),
            shape: Shape::Rect,
            keys: Vec::new(),
            locked: false,
        }
    }

    fn open(a: RoomId, b: RoomId) -> Connection {
        Connection {
            a,
            b,
            kind: Passage::Open,
            main: true,
        }
    }

    fn square() -> RoomGraph {
        RoomGraph {
            rooms: vec![
                room(Role::Start, &[(0, 0)]),
                room(Role::Path, &[(0, 1)]),
                room(Role::Path, &[(1, 0)]),
                room(Role::Goal, &[(1, 1)]),
            ],
            connections: vec![open(0, 1), open(0, 2), open(1, 3), open(2, 3)],
            lattice: LatticeRef { rows: 2, cols: 2 },
        }
    }

    fn all(_: Cell) -> bool {
        true
    }

    // AC: CE-80.3
    #[test]
    fn validate_accepts_a_minimal_loop() {
        assert_eq!(square().validate(&all), Ok(()));
    }

    // AC: CE-80.3
    #[test]
    fn validate_refuses_each_rule() {
        let g = square();
        let off = |c: Cell| c != Cell { row: 1, col: 1 };
        assert!(matches!(
            g.validate(&off),
            Err(GraphError::CellNotEnabled { room: 3, .. })
        ));

        let mut shared = square();
        shared.rooms[1].cells = vec![Cell { row: 0, col: 0 }];
        assert!(matches!(
            shared.validate(&all),
            Err(GraphError::CellShared { .. })
        ));

        let mut unsorted = square();
        unsorted.rooms[1].cells = vec![Cell { row: 0, col: 1 }, Cell { row: 0, col: 1 }];
        assert!(matches!(
            unsorted.validate(&all),
            Err(GraphError::CellShared { .. })
        ));

        let mut far = square();
        far.connections = vec![open(0, 3)];
        assert_eq!(
            far.validate(&all),
            Err(GraphError::NotAdjacent { a: 0, b: 3 })
        );

        let mut no_start = square();
        no_start.rooms[0].role = Role::Path;
        assert_eq!(no_start.validate(&all), Err(GraphError::StartCount(0)));
        let mut two_starts = square();
        two_starts.rooms[1].role = Role::Start;
        assert_eq!(two_starts.validate(&all), Err(GraphError::StartCount(2)));

        let mut no_goal = square();
        no_goal.rooms[3].role = Role::Path;
        assert_eq!(no_goal.validate(&all), Err(GraphError::GoalCount(0)));

        let mut keyless = square();
        keyless.connections[0].kind = Passage::Locked(1);
        assert_eq!(keyless.validate(&all), Err(GraphError::KeyMissing(1)));
        keyless.rooms[2].keys = vec![1];
        assert_eq!(keyless.validate(&all), Ok(()));

        let mut unordered = square();
        unordered.connections.swap(0, 1);
        assert_eq!(
            unordered.validate(&all),
            Err(GraphError::ConnectionOrder { a: 0, b: 1 })
        );
        let mut reversed = square();
        reversed.connections[0] = open(1, 0);
        assert_eq!(
            reversed.validate(&all),
            Err(GraphError::ConnectionOrder { a: 1, b: 0 })
        );

        let mut bad = square();
        bad.rooms[0].ty = "choir_hall".to_owned();
        assert_eq!(
            bad.validate(&all),
            Err(GraphError::BadType("choir_hall".to_owned()))
        );
    }

    // AC: CE-80.3
    #[test]
    fn validate_refuses_key_ids_outside_one_to_sixteen() {
        for k in [0, 17] {
            let mut held = square();
            held.rooms[1].keys = vec![k];
            assert_eq!(held.validate(&all), Err(GraphError::KeyOutOfRange(k)));
            let mut lock = square();
            lock.rooms[1].keys = vec![1];
            lock.connections[0].kind = Passage::Locked(k);
            assert_eq!(lock.validate(&all), Err(GraphError::KeyOutOfRange(k)));
        }
        let mut edge = square();
        edge.rooms[1].keys = vec![1, 16];
        edge.connections[0].kind = Passage::Locked(16);
        assert_eq!(edge.validate(&all), Ok(()));
    }

    // AC: CE-80.3
    #[test]
    fn canonical_json_is_stable() {
        let mut a = square();
        a.rooms[0].cells = vec![Cell { row: 0, col: 0 }, Cell { row: 0, col: 1 }];
        let mut b = a.clone();
        b.rooms[0].cells.reverse();
        b.rooms[0].cells.sort();
        let ja = serde_json::to_string(&a.canonical_json().unwrap()).unwrap();
        let jb = serde_json::to_string(&b.canonical_json().unwrap()).unwrap();
        assert_eq!(ja, jb);
        // The canonical form P30 reads, frozen: a lock with its key, a drop and a two-cell hall.
        let mut g = square();
        g.lattice = LatticeRef { rows: 2, cols: 3 };
        g.rooms[1].keys = vec![1];
        g.rooms[3].ty = "choir-hall".to_owned();
        g.rooms[3].cells.push(Cell { row: 1, col: 2 });
        g.rooms[3].shape = Shape::Round;
        g.rooms[3].locked = true;
        g.connections[1].kind = Passage::Locked(1);
        g.connections[3].kind = Passage::Drop { from: 3 };
        g.connections[3].main = false;
        assert_eq!(g.validate(&all), Ok(()));
        let json = serde_json::to_string(&g.canonical_json().unwrap()).unwrap();
        let expected = concat!(
            r#"{"connections":["#,
            r#"{"a":0,"b":1,"kind":"open","main":true},"#,
            r#"{"a":0,"b":2,"kind":{"locked":1},"main":true},"#,
            r#"{"a":1,"b":3,"kind":"open","main":true},"#,
            r#"{"a":2,"b":3,"kind":{"drop":{"from":3}},"main":false}],"#,
            r#""lattice":{"cols":3,"rows":2},"#,
            r#""rooms":["#,
            r#"{"cells":[{"col":0,"row":0}],"keys":[],"locked":false,"role":"start","shape":"rect","ty":"hall"},"#,
            r#"{"cells":[{"col":1,"row":0}],"keys":[1],"locked":false,"role":"path","shape":"rect","ty":"hall"},"#,
            r#"{"cells":[{"col":0,"row":1}],"keys":[],"locked":false,"role":"path","shape":"rect","ty":"hall"},"#,
            r#"{"cells":[{"col":1,"row":1},{"col":2,"row":1}],"keys":[],"locked":true,"role":"goal","#,
            r#""shape":"round","ty":"choir-hall"}]}"#,
        );
        assert_eq!(json, expected);
        assert_eq!(RoomGraph::key_name(1), "key-1");
        assert_eq!(RoomGraph::NAME, "room_graph");
    }

    // AC: CE-80.3
    #[test]
    fn validate_refuses_a_drop_from_neither_end() {
        let mut g = square();
        g.connections[0].kind = Passage::Drop { from: 0 };
        assert_eq!(g.validate(&all), Ok(()));
        g.connections[0].kind = Passage::Drop { from: 2 };
        assert_eq!(
            g.validate(&all),
            Err(GraphError::DropEnd {
                a: 0,
                b: 1,
                from: 2
            })
        );
    }

    #[test]
    fn neighbours_list_both_directions() {
        let g = square();
        let ids: Vec<RoomId> = g.neighbours(3).map(|(r, _)| r).collect();
        assert_eq!(ids, [1, 2]);
    }

    #[test]
    fn passage_round_trips() {
        for p in [Passage::Open, Passage::Locked(2), Passage::Drop { from: 4 }] {
            let v = serde_json::to_value(p).unwrap();
            assert_eq!(serde_json::from_value::<Passage>(v).unwrap(), p);
        }
    }
}
