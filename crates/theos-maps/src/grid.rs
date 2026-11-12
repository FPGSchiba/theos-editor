//! A dense row-major raster and the neighbour tables used to walk it.

/// Offsets `(d_row, d_col)` in the order N, NE, E, SE, S, SW, W, NW; `flow_dir` codes 1-8 follow
/// this order.
pub const D8: [(isize, isize); 8] = [
    (-1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
];

/// [`D8`], then the knight moves NNE, ENE, ESE, SSE, SSW, WSW, WNW, NNW.
pub const D16: [(isize, isize); 16] = [
    (-1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-2, 1),
    (-1, 2),
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
];

/// Errors of building a [`Grid`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GridError {
    /// The data length is not `rows * cols`.
    #[error("grid of {rows} x {cols} needs {expected} cells, got {actual}")]
    Length {
        /// Rows asked for.
        rows: usize,
        /// Columns asked for.
        cols: usize,
        /// `rows * cols`.
        expected: usize,
        /// Length of the data given.
        actual: usize,
    },
}

/// A row-major raster of `rows * cols` cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid<T> {
    rows: usize,
    cols: usize,
    data: Vec<T>,
}

impl<T: Copy> Grid<T> {
    /// A grid with every cell set to `value`.
    #[must_use]
    pub fn filled(rows: usize, cols: usize, value: T) -> Self {
        Self {
            rows,
            cols,
            data: vec![value; rows * cols],
        }
    }

    /// A grid over `data`, row by row.
    ///
    /// # Errors
    /// [`GridError::Length`] when `data.len() != rows * cols`.
    pub fn from_vec(rows: usize, cols: usize, data: Vec<T>) -> Result<Self, GridError> {
        let expected = rows * cols;
        if data.len() == expected {
            Ok(Self { rows, cols, data })
        } else {
            Err(GridError::Length {
                rows,
                cols,
                expected,
                actual: data.len(),
            })
        }
    }

    /// Number of rows.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns.
    #[must_use]
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Number of cells.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the grid has no cells.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The flat index of row `r`, column `c`.
    #[must_use]
    pub fn index(&self, r: usize, c: usize) -> usize {
        r * self.cols + c
    }

    /// The `(row, column)` of flat index `i`.
    #[must_use]
    pub fn cell(&self, i: usize) -> (usize, usize) {
        (i / self.cols, i % self.cols)
    }

    /// The value at row `r`, column `c`.
    #[must_use]
    pub fn get(&self, r: usize, c: usize) -> T {
        self.data[self.index(r, c)]
    }

    /// Sets the value at row `r`, column `c`.
    pub fn set(&mut self, r: usize, c: usize, v: T) {
        let i = self.index(r, c);
        self.data[i] = v;
    }

    /// The cells, row by row.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// The cells, row by row, mutable.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// A grid of the same shape with `f` applied to every cell.
    #[must_use]
    pub fn map<U: Copy>(&self, f: impl Fn(T) -> U) -> Grid<U> {
        Grid {
            rows: self.rows,
            cols: self.cols,
            data: self.data.iter().map(|&v| f(v)).collect(),
        }
    }

    /// The flat index of the cell at offset `d` from cell `i`, or `None` off the grid.
    #[must_use]
    pub fn neighbour(&self, i: usize, d: (isize, isize)) -> Option<usize> {
        let (r, c) = self.cell(i);
        let r = r.checked_add_signed(d.0).filter(|&r| r < self.rows)?;
        let c = c.checked_add_signed(d.1).filter(|&c| c < self.cols)?;
        Some(self.index(r, c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_index_and_cell_round_trip() {
        let grid = Grid::filled(3, 5, 0_u8);
        for r in 0..3 {
            for c in 0..5 {
                assert_eq!(grid.cell(grid.index(r, c)), (r, c));
            }
        }
        assert_eq!(grid.len(), 15);
        assert!(!grid.is_empty());
    }

    #[test]
    fn from_vec_checks_the_length() {
        assert!(Grid::from_vec(2, 2, vec![1, 2, 3]).is_err());
        let grid = Grid::from_vec(2, 2, vec![1, 2, 3, 4]).unwrap();
        assert_eq!(grid.get(1, 0), 3);
    }

    #[test]
    fn get_set_map_and_slices() {
        let mut grid = Grid::filled(2, 3, 1_i32);
        grid.set(1, 2, 9);
        assert_eq!(grid.get(1, 2), 9);
        grid.as_mut_slice()[0] = 4;
        assert_eq!(grid.as_slice()[0], 4);
        let doubled = grid.map(|v| v * 2);
        assert_eq!((doubled.rows(), doubled.cols()), (2, 3));
        assert_eq!(doubled.get(1, 2), 18);
    }

    #[test]
    fn neighbour_stops_at_the_edge() {
        let grid = Grid::filled(3, 3, 0_u8);
        assert_eq!(grid.neighbour(4, (-1, 0)), Some(1));
        assert_eq!(grid.neighbour(0, (-1, 0)), None);
        assert_eq!(grid.neighbour(8, (0, 1)), None);
        assert_eq!(grid.neighbour(0, (1, 1)), Some(4));
    }

    #[test]
    fn knight_moves_follow_the_d16_table() {
        assert_eq!(D8[0], (-1, 0));
        assert_eq!(D8[2], (0, 1));
        assert_eq!(D16[..8], D8);
        assert_eq!(
            D16[8..],
            [
                (-2, 1),
                (-1, 2),
                (1, 2),
                (2, 1),
                (2, -1),
                (1, -2),
                (-1, -2),
                (-2, -1)
            ]
        );
    }
}
