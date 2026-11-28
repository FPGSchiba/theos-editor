//! The space core (spec §5.7): rooms, walls and doors on the raster, from a room graph on its
//! lattice.

use std::any::Any;
use std::collections::BTreeSet;

use serde::Serialize;

use crate::grid::Grid;
use crate::kernels::labels::{Connectivity, labels};
use crate::rng::StageRng;
use crate::rooms::lattice::Lattice;
use crate::rooms::{Connection, Passage, RoomGraph, RoomId, Shape};
use crate::stages::interior::shapes;
use crate::world::{ValueType, WorldValue};

/// The parameters of the space core, in raster cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpaceParams {
    /// The wall thickness, at least 1.
    pub wall: u32,
    /// The door width along its wall, 1 or 2.
    pub door: u32,
    /// The width of a `passage` room's strip, at least 1.
    pub passage: u32,
    /// The noise threshold above which a `ragged` room loses an outer cell.
    pub ragged_t: f64,
}

/// The door of one connection: a gap through the wall the two rooms share.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Door {
    /// The index of the connection in [`RoomGraph::connections`].
    pub connection: usize,
    /// The raster cells of the gap, `(row, col)`, row-major.
    pub cells: Vec<(u32, u32)>,
    /// The connection's two rooms, `(a, b)`.
    pub rooms: (RoomId, RoomId),
    /// How the connection is walked.
    pub kind: Passage,
    /// The gap's middle cell, the lower one on even sizes.
    pub centre: (u32, u32),
}

/// The `doors` world value: one door per connection, in connection order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Doors(pub Vec<Door>);

impl WorldValue for Doors {
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

impl ValueType for Doors {
    const NAME: &'static str = "doors";
}

/// The rasters and doors the space core builds.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceOut {
    /// The room number + 1 per room cell, 0 elsewhere.
    pub rooms: Grid<u32>,
    /// Room and door cells.
    pub walkable: Grid<bool>,
    /// Built wall cells.
    pub walls: Grid<bool>,
    /// Door cells and the cells of `passage` rooms.
    pub corridors: Grid<bool>,
    /// The doors, one per connection.
    pub doors: Doors,
    /// The centre of every room, `(row, col)`, by room number.
    pub centres: Vec<(u32, u32)>,
}

/// Why the space core refused a graph.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpaceError {
    /// A parameter is out of its range: `wall` and `passage` at least 1, `door` 1 or 2.
    #[error("space parameter {name} = {value} is out of range")]
    BadParam {
        /// The parameter.
        name: &'static str,
        /// The value given.
        value: u32,
    },
    /// A connection's rooms share no wall segment long enough for a door.
    #[error("connection {connection} has no shared wall segment")]
    NoSharedSegment {
        /// The index of the connection.
        connection: usize,
    },
    /// A room has no cell, or a block of it is too small for its walls.
    #[error("room {room} has no cell")]
    EmptyRoom {
        /// The room.
        room: RoomId,
    },
    /// Cells of two rooms touch without a wall between them.
    #[error("rooms {a} and {b} touch without a wall")]
    RoomsTouch {
        /// The lower room.
        a: RoomId,
        /// The higher room.
        b: RoomId,
    },
    /// A door does not touch exactly its two rooms.
    #[error("door {door} does not touch exactly its two rooms")]
    DoorMismatch {
        /// The index of the door's connection.
        door: usize,
    },
    /// A walkable cell lies outside the envelope.
    #[error("cell ({}, {}) lies outside the envelope", .cell.0, .cell.1)]
    OutsideEnvelope {
        /// The cell, `(row, col)`.
        cell: (u32, u32),
    },
}

/// The widest door, in raster cells (ruling R1).
pub(crate) const MAX_DOOR: u32 = 2;

/// Builds rooms, walls and doors on a raster the size of `envelope` (spec §5.7), then checks
/// them.
///
/// - **Walls.** Every block of a room has a wall `p.wall` cells thick on its north rows and west
///   columns, cell by cell wherever the raster cell beyond the block's edge does not belong to
///   the same room; and on its south rows and east columns wherever the cell beyond belongs to
///   no room (a disabled or unused block, or the frame). So a hall has no walls between its own
///   cells, and every room is closed on all four sides. Room cells are the blocks minus walls.
/// - **Doors**, one per connection in order: the first pair of a cell of `a` and a cell of `b`
///   (both in sorted order) that shares a segment gives the wall line. One draw from
///   `doors_rng` places the gap `p.door` cells long at offset
///   `below(len - door - 2 wall + 1) + wall` from the segment's start. For the default wall of
///   one cell that is the spec's `below(len - door - 1) + 1`; a thicker wall keeps its corners
///   closed. The gap runs through the wall, `door` x `wall` cells.
/// - **Shapes** (module [`super::shapes`]) cut each room inside its bounding rectangle; the
///   shaped room's centre is taken, and a line from every door's inner cells to it is carved
///   back. The centres returned are those of the rooms as built ([`room_centres`]), so the
///   stages that read the rasters from the world find the same ones.
///
/// # Errors
/// [`SpaceError::BadParam`] for a parameter out of range; [`SpaceError::EmptyRoom`] for a block
/// that leaves less than 2x2 room cells inside its walls; [`SpaceError::OutsideEnvelope`] for a
/// block beyond the raster; [`SpaceError::NoSharedSegment`] for a connection without a segment
/// long enough for its door; then the first failed check of [`check`].
pub fn build_space(
    g: &RoomGraph,
    lat: &Lattice,
    envelope: &Grid<bool>,
    p: &SpaceParams,
    ragged_noise: &Grid<f64>,
    doors_rng: &mut StageRng,
) -> Result<SpaceOut, SpaceError> {
    check_params(p)?;
    let (rows, cols) = (envelope.rows(), envelope.cols());
    let owner = owners(g, lat, rows, cols, p.wall)?;
    let mut walls = wall_cells(g, lat, &owner, p.wall);
    let mut base = owner;
    for (o, &w) in base.as_mut_slice().iter_mut().zip(walls.as_slice()) {
        if w {
            *o = 0;
        }
    }
    let placed = g
        .connections
        .iter()
        .enumerate()
        .map(|(i, c)| place_door(g, lat, p, doors_rng, i, c))
        .collect::<Result<Vec<Placed>, SpaceError>>()?;

    let mut walkable = Grid::filled(rows, cols, false);
    let mut corridors = Grid::filled(rows, cols, false);
    for d in &placed {
        for &(r, c) in &d.door.cells {
            let (r, c) = (us(r), us(c));
            walls.set(r, c, false);
            walkable.set(r, c, true);
            corridors.set(r, c, true);
        }
    }
    let shaping = Shaping {
        base: &base,
        p,
        noise: ragged_noise,
    };
    let mut rooms = Grid::filled(rows, cols, 0_u32);
    let boxes = bounds(&base, g.rooms.len());
    for (i, room) in g.rooms.iter().enumerate() {
        let stubs: Vec<((u32, u32), bool)> = placed.iter().flat_map(|d| d.stubs_of(i)).collect();
        if let Some(b) = boxes.get(i).copied().flatten() {
            shaping.room(&mut rooms, b, number(i), room.shape, &stubs);
        }
    }
    let centres = room_centres(&rooms, g.rooms.len());
    for i in 0..rooms.len() {
        if let Some(k) = rooms.as_slice()[i].checked_sub(1) {
            walkable.as_mut_slice()[i] = true;
            if g.rooms
                .get(us(k))
                .is_some_and(|r| r.shape == Shape::Passage)
            {
                corridors.as_mut_slice()[i] = true;
            }
        }
    }
    let out = SpaceOut {
        rooms,
        walkable,
        walls,
        corridors,
        doors: Doors(placed.into_iter().map(|d| d.door).collect()),
        centres,
    };
    check(&out, g, envelope)?;
    Ok(out)
}

/// The first parameter out of range, by name (`wall`, `door`, `passage`) with its value:
/// `wall` and `passage` at least 1, `door` from 1 to [`MAX_DOOR`]. The `space` stage refuses
/// the same values when it plans.
pub(crate) fn out_of_range(p: &SpaceParams) -> Option<(&'static str, u32)> {
    if p.wall == 0 {
        return Some(("wall", p.wall));
    }
    if !(1..=MAX_DOOR).contains(&p.door) {
        return Some(("door", p.door));
    }
    if p.passage == 0 {
        return Some(("passage", p.passage));
    }
    None
}

/// Refuses parameters out of range.
fn check_params(p: &SpaceParams) -> Result<(), SpaceError> {
    match out_of_range(p) {
        Some((name, value)) => Err(SpaceError::BadParam { name, value }),
        None => Ok(()),
    }
}

/// The room number + 1 of every raster cell of a room's blocks, walls included.
fn owners(
    g: &RoomGraph,
    lat: &Lattice,
    rows: usize,
    cols: usize,
    wall: u32,
) -> Result<Grid<u32>, SpaceError> {
    let least = wall.saturating_mul(2).saturating_add(2);
    let mut owner = Grid::filled(rows, cols, 0_u32);
    for (i, room) in g.rooms.iter().enumerate() {
        for &cell in &room.cells {
            let b = lat.block(cell);
            if b.r1.saturating_sub(b.r0) < least || b.c1.saturating_sub(b.c0) < least {
                return Err(SpaceError::EmptyRoom { room: room_id(i) });
            }
            if us(b.r1) > rows || us(b.c1) > cols {
                let cell = if us(b.c1) > cols {
                    (b.r0, to_u32(cols))
                } else {
                    (to_u32(rows), b.c0)
                };
                return Err(SpaceError::OutsideEnvelope { cell });
            }
            for r in b.r0..b.r1 {
                for c in b.c0..b.c1 {
                    owner.set(us(r), us(c), number(i));
                }
            }
        }
    }
    Ok(owner)
}

/// The wall cells of every room's blocks (see [`build_space`]).
fn wall_cells(g: &RoomGraph, lat: &Lattice, owner: &Grid<u32>, w: u32) -> Grid<bool> {
    let at = |r: i64, c: i64| -> u32 {
        match (usize::try_from(r), usize::try_from(c)) {
            (Ok(r), Ok(c)) if r < owner.rows() && c < owner.cols() => owner.get(r, c),
            _ => 0,
        }
    };
    let mut walls = Grid::filled(owner.rows(), owner.cols(), false);
    for (i, room) in g.rooms.iter().enumerate() {
        let id = number(i);
        for &cell in &room.cells {
            let b = lat.block(cell);
            let (above, left) = (i64::from(b.r0) - 1, i64::from(b.c0) - 1);
            let (below, right) = (i64::from(b.r1), i64::from(b.c1));
            for r in b.r0..b.r1 {
                for c in b.c0..b.c1 {
                    let (ri, ci) = (i64::from(r), i64::from(c));
                    let north = r < b.r0 + w && at(above, ci) != id;
                    let west = c < b.c0 + w && at(ri, left) != id;
                    let south = r >= b.r1.saturating_sub(w) && at(below, ci) == 0;
                    let east = c >= b.c1.saturating_sub(w) && at(ri, right) == 0;
                    if north || west || south || east {
                        walls.set(us(r), us(c), true);
                    }
                }
            }
        }
    }
    walls
}

/// A door with the cells just inside its two rooms, where the carve to each centre starts.
struct Placed {
    /// The door.
    door: Door,
    /// Per side, the room and its cells next to the gap.
    sides: [(RoomId, Vec<(u32, u32)>); 2],
    /// Whether the gap crosses a horizontal wall, so its axis runs along the rows.
    rows_first: bool,
}

impl Placed {
    /// The carve starts of room `i`, with whether to carve along the rows first.
    fn stubs_of(&self, i: usize) -> Vec<((u32, u32), bool)> {
        self.sides
            .iter()
            .filter(|(r, _)| usize::from(*r) == i)
            .flat_map(|(_, cells)| cells.iter().map(|&c| (c, self.rows_first)))
            .collect()
    }
}

/// Places the door of connection `i` with one draw from `rng` (see [`build_space`]).
fn place_door(
    graph: &RoomGraph,
    lat: &Lattice,
    p: &SpaceParams,
    rng: &mut StageRng,
    i: usize,
    conn: &Connection,
) -> Result<Placed, SpaceError> {
    let c = conn;
    let missing = || SpaceError::NoSharedSegment { connection: i };
    let (Some(ra), Some(rb)) = (
        graph.rooms.get(usize::from(c.a)),
        graph.rooms.get(usize::from(c.b)),
    ) else {
        return Err(missing());
    };
    let (cell_a, cell_b, seg) = ra
        .cells
        .iter()
        .flat_map(|&ca| rb.cells.iter().map(move |&cb| (ca, cb)))
        .find_map(|(ca, cb)| lat.shared_segment(ca, cb).map(|s| (ca, cb, s)))
        .ok_or_else(missing)?;
    let need = p.door.saturating_add(p.wall.saturating_mul(2));
    if seg.len() < need {
        return Err(missing());
    }
    let draw = rng.below(u64::from(seg.len() - need + 1));
    let start = seg.from + u32::try_from(draw).unwrap_or(0) + p.wall;
    let pos = |across: u32, j: u32| {
        if seg.horizontal {
            (across, start + j)
        } else {
            (start + j, across)
        }
    };
    let mut cells: Vec<(u32, u32)> = (0..p.wall)
        .flat_map(|k| (0..p.door).map(move |j| pos(seg.at + k, j)))
        .collect();
    cells.sort_unstable();
    let (first, second) = if cell_a < cell_b {
        (c.a, c.b)
    } else {
        (c.b, c.a)
    };
    let side = |across: u32| (0..p.door).map(|j| pos(across, j)).collect::<Vec<_>>();
    Ok(Placed {
        door: Door {
            connection: i,
            cells,
            rooms: (c.a, c.b),
            kind: c.kind,
            centre: pos(seg.at + (p.wall - 1) / 2, (p.door - 1) / 2),
        },
        sides: [
            (first, side(seg.at.saturating_sub(1))),
            (second, side(seg.at + p.wall)),
        ],
        rows_first: seg.horizontal,
    })
}

/// A half-open raster rectangle: rows `.0..1`, columns `.2..3`.
type Rect = (usize, usize, usize, usize);

/// The bounding rectangle of every room's cells in `base`, by room index.
fn bounds(base: &Grid<u32>, n: usize) -> Vec<Option<Rect>> {
    let mut boxes: Vec<Option<Rect>> = vec![None; n];
    for i in 0..base.len() {
        let (r, c) = base.cell(i);
        if let Some(b) = base.as_slice()[i]
            .checked_sub(1)
            .and_then(|k| boxes.get_mut(us(k)))
        {
            *b = Some(b.map_or((r, r + 1, c, c + 1), |(r0, r1, c0, c1)| {
                (r0.min(r), r1.max(r + 1), c0.min(c), c1.max(c + 1))
            }));
        }
    }
    boxes
}

/// The part of `g` under rectangle `b`; `fill` off the grid.
fn window<T: Copy>(grid: &Grid<T>, rect: Rect, fill: T) -> Grid<T> {
    let (height, width) = (rect.1 - rect.0, rect.3 - rect.2);
    let cells = (0..height * width)
        .map(|i| {
            let (r, c) = (rect.0 + i / width, rect.2 + i % width);
            if r < grid.rows() && c < grid.cols() {
                grid.get(r, c)
            } else {
                fill
            }
        })
        .collect();
    Grid::from_vec(height, width, cells).unwrap_or_else(|_| Grid::filled(height, width, fill))
}

/// What shaping a room reads: the room cells before shaping, the parameters and the noise.
struct Shaping<'a> {
    /// Room number + 1 per room cell, walls removed, before shaping.
    base: &'a Grid<u32>,
    /// The parameters.
    p: &'a SpaceParams,
    /// The run's ragged noise over the whole raster.
    noise: &'a Grid<f64>,
}

impl Shaping<'_> {
    /// Shapes the room numbered `id` inside its rectangle `b`, carves its door stubs to the
    /// shaped room's centre and writes its cells into `rooms`; a room without a cell writes
    /// nothing.
    fn room(
        &self,
        rooms: &mut Grid<u32>,
        b: Rect,
        id: u32,
        shape: Shape,
        stubs: &[((u32, u32), bool)],
    ) {
        let room = window(self.base, b, 0).map(|v| v == id);
        let Some(anchor) = shapes::centre(&room) else {
            return;
        };
        let mut mask = match shape {
            Shape::Rect => shapes::rect(&room),
            Shape::Round => shapes::round(&room),
            Shape::Ragged => {
                let local = window(self.noise, b, f64::NEG_INFINITY);
                shapes::ragged(&room, &local, self.p.ragged_t, anchor)
            }
            Shape::Passage => shapes::passage(&room, us(self.p.passage)),
        };
        let Some(centre) = shapes::centre(&mask) else {
            return;
        };
        for &((r, c), rows_first) in stubs {
            if let (Some(lr), Some(lc)) = (us(r).checked_sub(b.0), us(c).checked_sub(b.2)) {
                shapes::carve(&mut mask, &room, (lr, lc), centre, rows_first);
            }
        }
        // One walkable piece per room: whatever the shape and the carving left apart from the
        // centre has no way to a door.
        let (pieces, _) = labels(&mask, Connectivity::Four);
        let home = pieces.get(centre.0, centre.1);
        for (m, &piece) in mask.as_mut_slice().iter_mut().zip(pieces.as_slice()) {
            *m = *m && piece == home;
        }
        for i in 0..mask.len() {
            if mask.as_slice()[i] {
                let (r, c) = mask.cell(i);
                rooms.set(b.0 + r, b.2 + c, id);
            }
        }
    }
}

/// The centre of each of the `n` rooms of a `rooms` raster (room number + 1 per cell), by room
/// number: the room's cell nearest to the mean of its cells ([`shapes::centre`], spec §5.7), on
/// the room as built, door stubs included. A room without a cell gets `(0, 0)`.
///
/// `space` returns it with the rasters, and `cover` and `playability`, which read the rasters
/// from the world, compute the same centres from the `rooms` layer.
#[must_use]
pub fn room_centres(rooms: &Grid<u32>, n: usize) -> Vec<(u32, u32)> {
    bounds(rooms, n)
        .into_iter()
        .enumerate()
        .map(|(i, b)| {
            b.and_then(|b| {
                let mask = window(rooms, b, 0).map(|v| v == number(i));
                shapes::centre(&mask).map(|(r, c)| (to_u32(b.0 + r), to_u32(b.2 + c)))
            })
            .unwrap_or((0, 0))
        })
        .collect()
}

/// The checks of spec §5.7, in order; the first failure is returned.
///
/// # Errors
/// [`SpaceError::EmptyRoom`] for a room without cells; [`SpaceError::RoomsTouch`] for two rooms'
/// cells side by side; [`SpaceError::DoorMismatch`] for a door beside other rooms than its two;
/// [`SpaceError::OutsideEnvelope`] for a walkable cell outside `envelope`.
fn check(out: &SpaceOut, g: &RoomGraph, envelope: &Grid<bool>) -> Result<(), SpaceError> {
    let rooms = &out.rooms;
    let mut count = vec![0_usize; g.rooms.len()];
    for &v in rooms.as_slice() {
        if let Some(n) = v.checked_sub(1).and_then(|k| count.get_mut(us(k))) {
            *n += 1;
        }
    }
    if let Some(i) = count.iter().position(|&n| n == 0) {
        return Err(SpaceError::EmptyRoom { room: room_id(i) });
    }
    for i in 0..rooms.len() {
        let v = rooms.as_slice()[i];
        for d in [(0, 1), (1, 0)] {
            let w = rooms.neighbour(i, d).map_or(0, |j| rooms.as_slice()[j]);
            if v > 0 && w > 0 && w != v {
                let id = |n: u32| RoomId::try_from(n - 1).unwrap_or(RoomId::MAX);
                return Err(SpaceError::RoomsTouch {
                    a: id(v.min(w)),
                    b: id(v.max(w)),
                });
            }
        }
    }
    for d in &out.doors.0 {
        let want: BTreeSet<u32> = [u32::from(d.rooms.0), u32::from(d.rooms.1)].into();
        if beside(rooms, &d.cells) != want {
            return Err(SpaceError::DoorMismatch { door: d.connection });
        }
    }
    for i in 0..out.walkable.len() {
        if out.walkable.as_slice()[i] && !envelope.as_slice().get(i).copied().unwrap_or(false) {
            let (r, c) = out.walkable.cell(i);
            return Err(SpaceError::OutsideEnvelope {
                cell: (to_u32(r), to_u32(c)),
            });
        }
    }
    Ok(())
}

/// The room indexes 4-adjacent to `cells`.
fn beside(rooms: &Grid<u32>, cells: &[(u32, u32)]) -> BTreeSet<u32> {
    let mut seen = BTreeSet::new();
    for &(r, c) in cells {
        for d in [(-1, 0), (0, 1), (1, 0), (0, -1)] {
            if let Some(j) = rooms.neighbour_at(us(r), us(c), d)
                && let Some(k) = rooms.as_slice()[j].checked_sub(1)
            {
                seen.insert(k);
            }
        }
    }
    seen
}

/// The id of room index `i`.
fn room_id(i: usize) -> RoomId {
    RoomId::try_from(i).unwrap_or(RoomId::MAX)
}

/// The value room index `i` has in the `rooms` raster.
fn number(i: usize) -> u32 {
    u32::try_from(i + 1).unwrap_or(u32::MAX)
}

/// A raster coordinate as an index.
fn us(v: u32) -> usize {
    usize::try_from(v).unwrap_or(usize::MAX)
}

/// An index as a raster coordinate.
fn to_u32(v: usize) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::noise::spectral_noise;
    use crate::rooms::lattice::Stagger;
    use crate::rooms::{Cell, LatticeRef, Role, Room};

    fn lattice(n: usize, pitch: u32, jitter: u32, seed: u64) -> Lattice {
        staggered(n, pitch, jitter, seed, Stagger::Rows)
    }

    fn staggered(n: usize, pitch: u32, jitter: u32, seed: u64, stagger: Stagger) -> Lattice {
        let mut rng = StageRng::for_key(seed, "lattice");
        Lattice::new(n, n, pitch, jitter, stagger, &mut rng).unwrap()
    }

    fn room(cells: &[(u16, u16)], shape: Shape) -> Room {
        Room {
            ty: "hall".to_owned(),
            role: Role::Path,
            cells: cells.iter().map(|&(row, col)| Cell { row, col }).collect(),
            shape,
            keys: Vec::new(),
            locked: false,
        }
    }

    fn open(a: RoomId, b: RoomId) -> Connection {
        Connection {
            a,
            b,
            kind: Passage::Open,
            main: false,
        }
    }

    fn graph(rooms: Vec<Room>, connections: Vec<Connection>, lat: &Lattice) -> RoomGraph {
        RoomGraph {
            rooms,
            connections,
            lattice: LatticeRef {
                rows: lat.rows,
                cols: lat.cols,
            },
        }
    }

    fn params() -> SpaceParams {
        SpaceParams {
            wall: 1,
            door: 1,
            passage: 2,
            ragged_t: 0.0,
        }
    }

    fn noise(n: usize, seed: u64) -> Grid<f64> {
        spectral_noise(n, n, &mut StageRng::for_key(seed, "ragged"), 2.2, 1.0, None)
    }

    fn build(
        g: &RoomGraph,
        lat: &Lattice,
        env: &Grid<bool>,
        p: &SpaceParams,
        ragged: &Grid<f64>,
        seed: u64,
    ) -> Result<SpaceOut, SpaceError> {
        build_space(
            g,
            lat,
            env,
            p,
            ragged,
            &mut StageRng::for_key(seed, "doors"),
        )
    }

    /// A 3x3 lattice with one 1x1 room per cell, row-major, of the given shapes.
    fn nine(lat: &Lattice, shapes: &[(usize, Shape)], connections: Vec<Connection>) -> RoomGraph {
        let rooms = lat
            .cells()
            .enumerate()
            .map(|(i, c)| {
                let shape = shapes
                    .iter()
                    .find(|(j, _)| *j == i)
                    .map_or(Shape::Rect, |&(_, s)| s);
                room(&[(c.row, c.col)], shape)
            })
            .collect();
        graph(rooms, connections, lat)
    }

    /// The rooms (by number) 4-adjacent to the cells of a door.
    fn rooms_beside(out: &SpaceOut, door: &Door) -> BTreeSet<u32> {
        let mut seen = BTreeSet::new();
        for &(r, c) in &door.cells {
            for d in [(-1, 0), (0, 1), (1, 0), (0, -1)] {
                if let Some(j) = out.rooms.neighbour_at(r as usize, c as usize, d) {
                    let v = out.rooms.as_slice()[j];
                    if v > 0 {
                        seen.insert(v - 1);
                    }
                }
            }
        }
        seen
    }

    fn snake() -> Vec<Connection> {
        vec![
            open(0, 1),
            open(1, 2),
            open(2, 5),
            open(3, 4),
            open(3, 6),
            open(4, 5),
            open(6, 7),
            open(7, 8),
        ]
    }

    // AC: CE-80.1, CE-80.3
    #[test]
    fn rect_rooms_share_walls_with_doors() {
        for (seed, door) in [(1, 1), (2, 2), (3, 1), (4, 2)] {
            let lat = lattice(24, 8, 2, seed);
            let graph = nine(&lat, &[], snake());
            let env = Grid::filled(24, 24, true);
            let p = SpaceParams { door, ..params() };
            let out = build(&graph, &lat, &env, &p, &noise(24, seed), seed).unwrap();
            assert_eq!(out.doors.0.len(), 8);
            for (i, d) in out.doors.0.iter().enumerate() {
                let conn = graph.connections[i];
                assert_eq!(
                    (d.connection, d.rooms, d.kind),
                    (i, (conn.a, conn.b), conn.kind)
                );
                assert_eq!(d.cells.len(), door as usize);
                let want: BTreeSet<u32> = [u32::from(conn.a), u32::from(conn.b)].into();
                assert_eq!(rooms_beside(&out, d), want, "door {i}, seed {seed}");
                for &(row, col) in &d.cells {
                    let (row, col) = (row as usize, col as usize);
                    assert!(out.walkable.get(row, col) && out.corridors.get(row, col));
                    assert!(!out.walls.get(row, col) && out.rooms.get(row, col) == 0);
                }
                assert!(d.cells.contains(&d.centre));
            }
            // Unconnected neighbours keep a solid wall along their whole shared segment.
            for (one, two) in [(0, 3), (1, 4), (4, 7), (5, 8)] {
                let cells: Vec<Cell> = lat.cells().collect();
                let seg = lat.shared_segment(cells[one], cells[two]).unwrap();
                for along in seg.from..seg.to {
                    let (row, col) = if seg.horizontal {
                        (seg.at, along)
                    } else {
                        (along, seg.at)
                    };
                    assert!(
                        out.walls.get(row as usize, col as usize),
                        "{one}-{two} at {along}"
                    );
                }
            }
            // Every room is closed: the walkable cells form one component through the doors,
            // and the frame is wall.
            let (_, n) = labels(&out.walkable, Connectivity::Four);
            assert_eq!(n, 1);
            for i in 0..24 {
                assert!(!out.walkable.get(0, i) && !out.walkable.get(23, i));
                assert!(!out.walkable.get(i, 0) && !out.walkable.get(i, 23));
            }
            assert_eq!(out.centres.len(), 9);
            for (i, &(r, c)) in out.centres.iter().enumerate() {
                assert_eq!(out.rooms.get(r as usize, c as usize), number(i));
            }
        }
    }

    // AC: CE-80.1, CE-80.3
    #[test]
    fn halls_have_no_inner_walls() {
        let lat = lattice(24, 8, 0, 1);
        let mut rooms = vec![room(&[(0, 0), (0, 1)], Shape::Rect)];
        rooms.extend([(0, 2), (1, 0), (1, 1), (1, 2)].map(|c| room(&[c], Shape::Rect)));
        let g = graph(
            rooms,
            vec![open(0, 1), open(0, 2), open(2, 3), open(3, 4)],
            &lat,
        );
        let env = Grid::filled(24, 24, true);
        let out = build(&g, &lat, &env, &params(), &noise(24, 1), 1).unwrap();
        // Column 8 is the west line of cell (0, 1): inside the hall it is room, not wall.
        for r in 1..8 {
            assert_eq!(out.rooms.get(r, 8), 1, "row {r}");
            assert!(!out.walls.get(r, 8));
        }
        assert!(out.walls.get(0, 8) && out.walls.get(8, 8) && out.walls.get(4, 16));
        // Staggered halls spanning lattice rows never leak into a neighbour.
        for (seed, stagger) in (1..=10).flat_map(|s| [(s, Stagger::Rows), (s, Stagger::Columns)]) {
            let lat = staggered(24, 8, 2, seed, stagger);
            let rooms = vec![
                room(&[(0, 0), (1, 0)], Shape::Rect),
                room(&[(0, 1), (0, 2)], Shape::Rect),
                room(&[(1, 1)], Shape::Rect),
                room(&[(1, 2), (2, 2)], Shape::Rect),
                room(&[(2, 0), (2, 1)], Shape::Rect),
            ];
            let g = graph(
                rooms,
                vec![open(0, 1), open(1, 2), open(2, 3), open(2, 4)],
                &lat,
            );
            let out = build(&g, &lat, &env, &params(), &noise(24, seed), seed).unwrap();
            let (_, n) = labels(&out.walkable, Connectivity::Four);
            assert_eq!(n, 1, "seed {seed}");
        }
    }

    // AC: CE-80.1
    #[test]
    fn shaped_l_hall_keeps_one_piece() {
        let ells: [[(u16, u16); 3]; 4] = [
            [(0, 0), (0, 1), (1, 0)],
            [(0, 0), (0, 1), (1, 1)],
            [(0, 0), (1, 0), (1, 1)],
            [(0, 1), (1, 0), (1, 1)],
        ];
        for shape in [Shape::Round, Shape::Passage] {
            for cells in ells {
                for (seed, pitch) in [
                    (1, 8),
                    (2, 9),
                    (3, 12),
                    (4, 16),
                    (5, 8),
                    (6, 8),
                    (7, 9),
                    (8, 10),
                ] {
                    for door in [1, 2] {
                        let n = 3 * pitch as usize;
                        let lat = lattice(n, pitch, 2, seed);
                        let rooms = vec![
                            room(&cells, shape),
                            room(&[(0, 2)], Shape::Rect),
                            room(&[(2, 0)], Shape::Rect),
                            room(&[(2, 2)], Shape::Rect),
                        ];
                        let g = graph(rooms, vec![open(0, 1), open(0, 2), open(0, 3)], &lat);
                        let env = Grid::filled(n, n, true);
                        let p = SpaceParams { door, ..params() };
                        let Ok(out) = build(&g, &lat, &env, &p, &noise(n, seed), seed) else {
                            continue;
                        };
                        let own = out.rooms.map(|v| v == 1);
                        let (_, parts) = labels(&own, Connectivity::Four);
                        assert_eq!(parts, 1, "{shape:?} {cells:?} seed {seed} door {door}");
                    }
                }
            }
        }
    }

    // AC: CE-80.1
    #[test]
    fn shaping_keeps_only_the_piece_with_the_centre() {
        // A one-cell-wide hall along the top and left edges: the round shape cuts the bend off
        // and leaves two pieces, of which only the one with the centre may stay.
        let base = Grid::from_vec(
            15,
            15,
            (0..225)
                .map(|i| u32::from(i / 15 == 1 || i % 15 == 1))
                .collect(),
        )
        .unwrap();
        let p = params();
        let z = Grid::filled(15, 15, 0.0);
        let shaping = Shaping {
            base: &base,
            p: &p,
            noise: &z,
        };
        let mut rooms = Grid::filled(15, 15, 0_u32);
        shaping.room(&mut rooms, (0, 15, 0, 15), 1, Shape::Round, &[]);
        let centre = room_centres(&rooms, 1)[0];
        assert_eq!(rooms.get(centre.0 as usize, centre.1 as usize), 1);
        let (_, parts) = labels(&rooms.map(|v| v == 1), Connectivity::Four);
        assert_eq!(parts, 1);
    }

    // AC: CE-80.3
    #[test]
    fn space_refuses_corner_only_connection() {
        let lat = lattice(24, 8, 2, 1);
        let g = graph(
            vec![room(&[(0, 0)], Shape::Rect), room(&[(1, 1)], Shape::Rect)],
            vec![open(0, 1)],
            &lat,
        );
        let env = Grid::filled(24, 24, true);
        assert_eq!(
            build(&g, &lat, &env, &params(), &noise(24, 1), 1),
            Err(SpaceError::NoSharedSegment { connection: 0 })
        );
    }

    fn shaped_nine(lat: &Lattice) -> RoomGraph {
        let connections = vec![
            open(0, 1),
            open(1, 2),
            open(1, 4),
            open(3, 4),
            open(4, 5),
            open(4, 7),
            open(6, 7),
            open(7, 8),
        ];
        nine(
            lat,
            &[
                (4, Shape::Round),
                (1, Shape::Ragged),
                (7, Shape::Passage),
                (3, Shape::Ragged),
            ],
            connections,
        )
    }

    // AC: CE-80.1
    #[test]
    fn round_and_ragged_rooms_keep_door_paths() {
        for seed in 1..=10 {
            let lat = lattice(18, 6, 1, seed);
            let graph = shaped_nine(&lat);
            let env = Grid::filled(18, 18, true);
            let p = SpaceParams {
                door: 1 + u32::from(seed % 2 == 0),
                ragged_t: -0.5,
                passage: 1,
                ..params()
            };
            let out = build(&graph, &lat, &env, &p, &noise(18, seed), seed).unwrap();
            for (i, &(cr, cc)) in out.centres.iter().enumerate() {
                let mut mask = out.rooms.map(|v| v == number(i));
                let mine: Vec<&Door> = out
                    .doors
                    .0
                    .iter()
                    .filter(|door| door.rooms.0 == room_id(i) || door.rooms.1 == room_id(i))
                    .collect();
                for door in &mine {
                    for &(row, col) in &door.cells {
                        mask.set(row as usize, col as usize, true);
                    }
                }
                let (lab, _) = labels(&mask, Connectivity::Four);
                let home = lab.get(cr as usize, cc as usize);
                assert!(home > 0);
                for door in &mine {
                    for &(row, col) in &door.cells {
                        assert_eq!(
                            lab.get(row as usize, col as usize),
                            home,
                            "room {i}, seed {seed}"
                        );
                    }
                }
            }
            let (_, parts) = labels(&out.walkable, Connectivity::Four);
            assert_eq!(parts, 1, "seed {seed}");
        }
    }

    // AC: CE-80.1
    #[test]
    fn shapes_change_the_rooms() {
        let lat = lattice(24, 8, 0, 1);
        let g = shaped_nine(&lat);
        let env = Grid::filled(24, 24, true);
        let p = SpaceParams {
            ragged_t: -0.5,
            ..params()
        };
        let out = build(&g, &lat, &env, &p, &noise(24, 1), 1).unwrap();
        let area = |id: u32| out.rooms.as_slice().iter().filter(|&&v| v == id).count();
        // A rect room at pitch 8 is 7x7 (6x7 or 7x6 at the frame); round and passage are less.
        assert_eq!(area(1), 7 * 7);
        assert!(area(5) < 49, "round");
        assert!(area(2) < 49, "ragged");
        assert!(area(8) < 6 * 7, "passage");
        // A passage room is corridor.
        let (r, c) = out.centres[7];
        assert!(out.corridors.get(r as usize, c as usize));
        let (r, c) = out.centres[0];
        assert!(!out.corridors.get(r as usize, c as usize));
    }

    // AC: CE-80.3
    #[test]
    fn door_positions_depend_only_on_the_doors_stream() {
        for seed in 1..=5 {
            let lat = lattice(18, 6, 1, seed);
            let graph = shaped_nine(&lat);
            let env = Grid::filled(18, 18, true);
            let p = SpaceParams {
                ragged_t: -0.2,
                ..params()
            };
            let first = build(&graph, &lat, &env, &p, &noise(18, seed), 7).unwrap();
            let renoised = build(&graph, &lat, &env, &p, &noise(18, seed + 100), 7).unwrap();
            assert_eq!(first.doors, renoised.doors);
            let flat = nine(&lat, &[], graph.connections.clone());
            let unshaped = build(&flat, &lat, &env, &p, &noise(18, seed), 7).unwrap();
            assert_eq!(first.doors, unshaped.doors);
            let restreamed = build(&graph, &lat, &env, &p, &noise(18, seed), 8).unwrap();
            assert_ne!(
                first.doors, restreamed.doors,
                "the doors stream moves the doors"
            );
        }
    }

    fn circle(n: usize, radius: i64) -> Grid<bool> {
        let half = i64::try_from(n).unwrap() / 2;
        Grid::from_vec(
            n,
            n,
            (0..n * n)
                .map(|i| {
                    let r = i64::try_from(i / n).unwrap() - half;
                    let c = i64::try_from(i % n).unwrap() - half;
                    r * r + c * c <= radius * radius
                })
                .collect(),
        )
        .unwrap()
    }

    // AC: CE-80.3
    #[test]
    fn nothing_outside_the_envelope() {
        let n = 40;
        let env = circle(n, 19);
        let mut lat = lattice(n, 8, 2, 3);
        lat.enable_inside(&env);
        let cells: Vec<Cell> = lat.cells().filter(|&c| lat.is_enabled(c)).collect();
        assert!(cells.len() >= 5);
        let rooms: Vec<Room> = cells
            .iter()
            .map(|c| room(&[(c.row, c.col)], Shape::Rect))
            .collect();
        let mut connections = Vec::new();
        for (i, &a) in cells.iter().enumerate() {
            for (j, &b) in cells.iter().enumerate().skip(i + 1) {
                if a.touches(b) {
                    connections.push(open(room_id(i), room_id(j)));
                }
            }
        }
        let g = graph(rooms, connections, &lat);
        let out = build(&g, &lat, &env, &params(), &noise(n, 1), 1).unwrap();
        for i in 0..env.len() {
            assert!(!out.walkable.as_slice()[i] || env.as_slice()[i]);
            assert!(!out.walls.as_slice()[i] || env.as_slice()[i]);
        }
        // A room cell outside the envelope is refused.
        let (r, c) = out.centres[0];
        let mut cut = env.clone();
        cut.set(r as usize, c as usize, false);
        assert_eq!(
            build(&g, &lat, &cut, &params(), &noise(n, 1), 1),
            Err(SpaceError::OutsideEnvelope { cell: (r, c) })
        );
    }

    // AC: CE-80.3
    #[test]
    fn refuses_bad_parameters_and_thick_walls() {
        let lat = lattice(24, 8, 0, 1);
        let g = nine(&lat, &[], snake());
        let env = Grid::filled(24, 24, true);
        let z = noise(24, 1);
        for (p, name, value) in [
            (
                SpaceParams {
                    door: 3,
                    ..params()
                },
                "door",
                3,
            ),
            (
                SpaceParams {
                    door: 0,
                    ..params()
                },
                "door",
                0,
            ),
            (
                SpaceParams {
                    wall: 0,
                    ..params()
                },
                "wall",
                0,
            ),
            (
                SpaceParams {
                    passage: 0,
                    ..params()
                },
                "passage",
                0,
            ),
        ] {
            assert_eq!(
                build(&g, &lat, &env, &p, &z, 1),
                Err(SpaceError::BadParam { name, value })
            );
        }
        // Wall 3 leaves 2x2 room cells in an 8-cell block; wall 4 leaves none.
        let thick = SpaceParams {
            wall: 3,
            ..params()
        };
        assert!(build(&g, &lat, &env, &thick, &z, 1).is_ok());
        let thicker = SpaceParams {
            wall: 4,
            ..params()
        };
        assert_eq!(
            build(&g, &lat, &env, &thicker, &z, 1),
            Err(SpaceError::EmptyRoom { room: 0 })
        );
    }

    // AC: CE-80.3
    #[test]
    fn checks_catch_broken_rasters() {
        let lat = lattice(24, 8, 0, 1);
        let g = nine(&lat, &[], snake());
        let env = Grid::filled(24, 24, true);
        let out = build(&g, &lat, &env, &params(), &noise(24, 1), 1).unwrap();
        assert_eq!(check(&out, &g, &env), Ok(()));

        let mut empty = out.clone();
        empty.rooms = out.rooms.map(|v| if v == 3 { 0 } else { v });
        assert_eq!(
            check(&empty, &g, &env),
            Err(SpaceError::EmptyRoom { room: 2 })
        );

        // Room 0 spills over the wall line between cells (0, 0) and (0, 1).
        let mut touch = out.clone();
        touch.rooms.set(4, 8, 1);
        assert_eq!(
            check(&touch, &g, &env),
            Err(SpaceError::RoomsTouch { a: 0, b: 1 })
        );

        let mut moved = out.clone();
        moved.doors.0[0].cells = vec![(4, 4)];
        assert_eq!(
            check(&moved, &g, &env),
            Err(SpaceError::DoorMismatch { door: 0 })
        );

        let mut outside = env.clone();
        outside.set(4, 4, false);
        assert_eq!(
            check(&out, &g, &outside),
            Err(SpaceError::OutsideEnvelope { cell: (4, 4) })
        );
    }

    #[test]
    fn doors_are_a_world_value() {
        let d = Doors(vec![Door {
            connection: 0,
            cells: vec![(1, 2)],
            rooms: (0, 1),
            kind: Passage::Locked(1),
            centre: (1, 2),
        }]);
        assert_eq!(d.type_name(), "doors");
        assert!(d.canonical_json().is_ok());
    }
}
