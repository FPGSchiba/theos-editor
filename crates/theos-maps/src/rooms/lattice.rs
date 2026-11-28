//! The interior lattice: lattice cells over the raster, with staggered walls (§5.7).

use std::any::Any;

use serde::{Deserialize, Serialize};

use super::graph::Cell;
use crate::grid::Grid;
use crate::rng::StageRng;
use crate::world::{ValueType, WorldValue};

/// The least pitch, in raster cells.
const MIN_PITCH: u32 = 6;

/// The least shared wall segment, in raster cells, that a connection needs: the widest door (2)
/// plus one cell at each end.
pub const MIN_SPAN: u32 = 4;

/// Which wall lines move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stagger {
    /// Horizontal lines are straight; vertical lines move within each lattice row.
    Rows,
    /// Vertical lines are straight; horizontal lines move within each lattice column.
    Columns,
}

/// A parameter of the lattice that is not allowed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LatticeError {
    /// The pitch is below 6 raster cells.
    #[error("pitch {0} is below 6 raster cells")]
    PitchTooSmall(u32),
    /// The jitter exceeds a quarter of the pitch.
    #[error("jitter {jitter} exceeds the maximum {max} (a quarter of the pitch)")]
    JitterTooLarge {
        /// The jitter asked for.
        jitter: u32,
        /// The largest jitter the pitch allows.
        max: u32,
    },
    /// The grid holds less than one lattice cell in a direction.
    #[error("a grid that gives {rows} x {cols} lattice cells is too small")]
    TooSmall {
        /// Lattice rows.
        rows: usize,
        /// Lattice columns.
        cols: usize,
    },
}

/// The raster extent of a block, walls included; rows and columns are half-open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockRect {
    /// The first raster row.
    pub r0: u32,
    /// One past the last raster row.
    pub r1: u32,
    /// The first raster column.
    pub c0: u32,
    /// One past the last raster column.
    pub c1: u32,
}

/// A stretch of wall shared by two blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// Whether the wall line is horizontal.
    pub horizontal: bool,
    /// The raster row (horizontal) or column (vertical) of the line.
    pub at: u32,
    /// The first raster cell along the line.
    pub from: u32,
    /// One past the last raster cell along the line.
    pub to: u32,
}

impl Segment {
    /// The number of raster cells along the segment.
    #[must_use]
    pub fn len(&self) -> u32 {
        self.to.saturating_sub(self.from)
    }

    /// Whether the segment holds no cell.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.to <= self.from
    }
}

/// The lattice over an interior's raster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lattice {
    /// The size of a lattice cell in raster cells.
    pub pitch: u32,
    /// Lattice rows.
    pub rows: u16,
    /// Lattice columns.
    pub cols: u16,
    /// The raster row and column of the lattice's north-west corner.
    pub origin: (u32, u32),
    /// Which wall lines move.
    pub stagger: Stagger,
    /// Whether each cell is enabled, row-major.
    pub enabled: Vec<bool>,
    /// Offsets of the moving lines: per lane (lattice row for `Rows`, column for `Columns`),
    /// one entry per line along it; the outer lines are zero.
    offsets: Vec<i32>,
}

impl Lattice {
    /// Builds the lattice over a `grid_rows` x `grid_cols` raster with every cell enabled.
    /// Offsets are drawn lane by lane, along the lane's lines, interior lines only.
    ///
    /// # Errors
    /// [`LatticeError`] for a pitch below 6, a jitter above `pitch / 4`, or a grid without a cell.
    pub fn new(
        grid_rows: usize,
        grid_cols: usize,
        pitch: u32,
        jitter: u32,
        stagger: Stagger,
        rng: &mut StageRng,
    ) -> Result<Self, LatticeError> {
        let (rows, cols) = Self::size(grid_rows, grid_cols, pitch, jitter)?;
        let p = pitch as usize;
        // Beyond u16 the lattice is clamped; the leftover then centres the clamped lattice.
        let rows = rows.min(usize::from(u16::MAX));
        let cols = cols.min(usize::from(u16::MAX));
        let to_u32 = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
        let origin = (
            to_u32((grid_rows - rows * p) / 2),
            to_u32((grid_cols - cols * p) / 2),
        );
        let (n_lanes, n_stops) = match stagger {
            Stagger::Rows => (rows, cols + 1),
            Stagger::Columns => (cols, rows + 1),
        };
        let mut offsets = vec![0_i32; n_lanes * n_stops];
        let width = i64::from(jitter);
        for lane in 0..n_lanes {
            for line in 1..n_stops - 1 {
                let draw = i64::try_from(rng.below(2 * u64::from(jitter) + 1)).unwrap_or(0);
                offsets[lane * n_stops + line] = i32::try_from(draw - width).unwrap_or(0);
            }
        }
        Ok(Self {
            pitch,
            rows: u16::try_from(rows).unwrap_or(u16::MAX),
            cols: u16::try_from(cols).unwrap_or(u16::MAX),
            origin,
            stagger,
            enabled: vec![true; rows * cols],
            offsets,
        })
    }

    /// The lattice rows and columns over a `grid_rows` x `grid_cols` raster, before the clamp
    /// to `u16`: the checks of [`Lattice::new`] without building anything, so the recipe loader
    /// refuses what the stage would.
    ///
    /// # Errors
    /// [`LatticeError`] for a pitch below 6, a jitter above `pitch / 4`, or a grid without a cell.
    pub fn size(
        grid_rows: usize,
        grid_cols: usize,
        pitch: u32,
        jitter: u32,
    ) -> Result<(usize, usize), LatticeError> {
        if pitch < MIN_PITCH {
            return Err(LatticeError::PitchTooSmall(pitch));
        }
        let max = pitch / 4;
        if jitter > max {
            return Err(LatticeError::JitterTooLarge { jitter, max });
        }
        let p = pitch as usize;
        let (rows, cols) = (grid_rows / p, grid_cols / p);
        if rows < 1 || cols < 1 {
            return Err(LatticeError::TooSmall { rows, cols });
        }
        Ok((rows, cols))
    }

    fn offset(&self, lane: u16, at: u16) -> i64 {
        let lines = match self.stagger {
            Stagger::Rows => usize::from(self.cols) + 1,
            Stagger::Columns => usize::from(self.rows) + 1,
        };
        let i = usize::from(lane) * lines + usize::from(at);
        i64::from(self.offsets.get(i).copied().unwrap_or(0))
    }

    /// The raster position of nominal line `line` (`0..=n`) shifted by `shift`.
    fn line(&self, origin: u32, line: u16, shift: i64) -> u32 {
        let base = i64::from(origin) + i64::from(line) * i64::from(self.pitch) + shift;
        u32::try_from(base).unwrap_or(0)
    }

    /// The raster extent of the block of `c`, walls included.
    #[must_use]
    pub fn block(&self, c: Cell) -> BlockRect {
        match self.stagger {
            Stagger::Rows => BlockRect {
                r0: self.line(self.origin.0, c.row, 0),
                r1: self.line(self.origin.0, c.row + 1, 0),
                c0: self.line(self.origin.1, c.col, self.offset(c.row, c.col)),
                c1: self.line(self.origin.1, c.col + 1, self.offset(c.row, c.col + 1)),
            },
            Stagger::Columns => BlockRect {
                r0: self.line(self.origin.0, c.row, self.offset(c.col, c.row)),
                r1: self.line(self.origin.0, c.row + 1, self.offset(c.col, c.row + 1)),
                c0: self.line(self.origin.1, c.col, 0),
                c1: self.line(self.origin.1, c.col + 1, 0),
            },
        }
    }

    /// The wall segment two neighbouring cells share; `None` when the cells are not orthogonal
    /// neighbours or the overlap is shorter than [`MIN_SPAN`].
    #[must_use]
    pub fn shared_segment(&self, a: Cell, b: Cell) -> Option<Segment> {
        if !a.touches(b) {
            return None;
        }
        let (first, second) = if a <= b { (a, b) } else { (b, a) };
        let (x, y) = (self.block(first), self.block(second));
        let seg = if first.col == second.col {
            Segment {
                horizontal: true,
                at: x.r1,
                from: x.c0.max(y.c0),
                to: x.c1.min(y.c1),
            }
        } else {
            Segment {
                horizontal: false,
                at: x.c1,
                from: x.r0.max(y.r0),
                to: x.r1.min(y.r1),
            }
        };
        (seg.len() >= MIN_SPAN).then_some(seg)
    }

    /// Enables the cells whose whole block lies inside `envelope`, and disables the others.
    pub fn enable_inside(&mut self, envelope: &Grid<bool>) {
        let cells: Vec<Cell> = self.cells().collect();
        for c in cells {
            let b = self.block(c);
            let inside = b.r1 as usize <= envelope.rows()
                && b.c1 as usize <= envelope.cols()
                && (b.r0..b.r1)
                    .all(|r| (b.c0..b.c1).all(|col| envelope.get(r as usize, col as usize)));
            let i = usize::from(c.row) * usize::from(self.cols) + usize::from(c.col);
            self.enabled[i] = inside;
        }
    }

    /// Whether the cell is enabled; cells outside the lattice are not.
    #[must_use]
    pub fn is_enabled(&self, c: Cell) -> bool {
        c.row < self.rows
            && c.col < self.cols
            && self
                .enabled
                .get(usize::from(c.row) * usize::from(self.cols) + usize::from(c.col))
                .copied()
                .unwrap_or(false)
    }

    /// Every cell, row-major.
    pub fn cells(&self) -> impl Iterator<Item = Cell> + use<> {
        let cols = self.cols;
        (0..self.rows).flat_map(move |row| (0..cols).map(move |col| Cell { row, col }))
    }
}

impl WorldValue for Lattice {
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

impl ValueType for Lattice {
    const NAME: &'static str = "lattice";
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(n: usize, pitch: u32, jitter: u32, stagger: Stagger, seed: u64) -> Lattice {
        let mut rng = StageRng::for_key(seed, "lattice");
        Lattice::new(n, n, pitch, jitter, stagger, &mut rng).unwrap()
    }

    // AC: CE-80.3
    #[test]
    fn size_and_centring() {
        let l = make(100, 16, 4, Stagger::Rows, 1);
        assert_eq!((l.rows, l.cols, l.origin), (6, 6, (2, 2)));
        assert_eq!(l.enabled.len(), 36);
        assert_eq!(Lattice::NAME, "lattice");
        assert!(l.canonical_json().is_ok());
    }

    // AC: CE-80.3
    #[test]
    fn blocks_partition_each_lattice_row() {
        for seed in 1..=20 {
            let l = make(100, 16, 4, Stagger::Rows, seed);
            for row in 0..l.rows {
                let first = l.block(Cell { row, col: 0 });
                let mut edge = l.origin.1;
                for col in 0..l.cols {
                    let b = l.block(Cell { row, col });
                    assert_eq!(b.c0, edge, "gap or overlap, seed {seed}");
                    assert!(b.c1 > b.c0);
                    assert_eq!((b.r0, b.r1), (first.r0, first.r1));
                    edge = b.c1;
                }
                assert_eq!(
                    edge,
                    l.origin.1 + u32::from(l.cols) * 16,
                    "outer edge moved"
                );
            }
        }
    }

    // AC: CE-80.3
    #[test]
    fn vertical_neighbours_share_at_least_half_a_pitch() {
        for seed in 1..=20 {
            let l = make(100, 16, 4, Stagger::Rows, seed);
            for row in 0..l.rows - 1 {
                for col in 0..l.cols {
                    let (a, b) = (Cell { row, col }, Cell { row: row + 1, col });
                    let s = l.shared_segment(a, b).unwrap();
                    assert!(s.horizontal);
                    assert!(s.len() >= 16 - 2 * 4 && s.len() >= 8);
                    assert_eq!(l.shared_segment(b, a), Some(s));
                }
            }
        }
    }

    // AC: CE-80.3
    #[test]
    fn invariant_gives_min_span_for_all_pitches() {
        for pitch in 6..=20 {
            let jitter = pitch / 4;
            for stagger in [Stagger::Rows, Stagger::Columns] {
                for seed in 1..=10 {
                    let l = make(pitch as usize * 5 + 3, pitch, jitter, stagger, seed);
                    for a in l.cells() {
                        for b in l.cells().filter(|&b| a.touches(b)) {
                            let s = l.shared_segment(a, b);
                            assert!(s.is_some_and(|s| s.len() >= MIN_SPAN), "pitch {pitch}");
                        }
                    }
                }
            }
        }
    }

    // AC: CE-80.3
    #[test]
    fn columns_stagger_mirrors_rows() {
        let l = make(100, 16, 4, Stagger::Columns, 7);
        for col in 0..l.cols {
            let first = l.block(Cell { row: 0, col });
            let mut edge = l.origin.0;
            for row in 0..l.rows {
                let b = l.block(Cell { row, col });
                assert_eq!(b.r0, edge);
                edge = b.r1;
                assert_eq!((b.c0, b.c1), (first.c0, first.c1));
            }
        }
        let s = l
            .shared_segment(Cell { row: 0, col: 0 }, Cell { row: 0, col: 1 })
            .unwrap();
        assert!(!s.horizontal);
        assert!(s.len() >= 8);
        assert!(l.cells().any(|c| {
            let b = l.block(c);
            b.r1 - b.r0 != 16
        }));
    }

    #[test]
    fn non_neighbours_have_no_segment() {
        let l = make(100, 16, 4, Stagger::Rows, 1);
        let a = Cell { row: 0, col: 0 };
        assert_eq!(l.shared_segment(a, Cell { row: 1, col: 1 }), None);
        assert_eq!(l.shared_segment(a, a), None);
    }

    // AC: CE-80.3
    #[test]
    fn refuses_bad_parameters() {
        let mut rng = StageRng::for_key(1, "lattice");
        let mut new = |n, p, j| Lattice::new(n, n, p, j, Stagger::Rows, &mut rng);
        assert_eq!(new(100, 5, 1).unwrap_err(), LatticeError::PitchTooSmall(5));
        assert_eq!(
            new(100, 16, 5).unwrap_err(),
            LatticeError::JitterTooLarge { jitter: 5, max: 4 }
        );
        assert_eq!(
            new(10, 16, 0).unwrap_err(),
            LatticeError::TooSmall { rows: 0, cols: 0 }
        );
    }

    // AC: CE-80.3
    #[test]
    fn enable_inside_needs_the_whole_block() {
        let n = 96;
        let mut l = make(n, 16, 0, Stagger::Rows, 1);
        let mask = Grid::from_vec(
            n,
            n,
            (0..n * n)
                .map(|i| {
                    let (r, c) = (
                        i64::try_from(i / n).unwrap() - 48,
                        i64::try_from(i % n).unwrap() - 48,
                    );
                    r * r + c * c <= 46 * 46
                })
                .collect(),
        )
        .unwrap();
        l.enable_inside(&mask);
        for (row, col) in [(0, 0), (0, 5), (5, 0), (5, 5)] {
            assert!(!l.is_enabled(Cell { row, col }));
        }
        assert!(l.is_enabled(Cell { row: 2, col: 3 }));
        assert!(!l.is_enabled(Cell { row: 9, col: 9 }));
        assert_eq!(l.cells().count(), 36);
    }
}
