//! Steps 1 to 5 of `hydrology`: the outlets, the depressions of the filled surface, the shallow
//! pits filled in `height`, the inflow of every depression, and its lake by water balance
//! (Bracht-Flyr et al. 2013, rain on the lake counted, rearranged for the area).

use super::discharge::Drainage;
use crate::grid::Grid;
use crate::kernels::labels::{Connectivity, labels};
use crate::kernels::num::{OrdF64, key, max, to_f64, to_index};
use crate::stages::evolution::cell_index;
use crate::stages::terrain::climate::ClimateFields;

/// A cell the fill raised by more than this belongs to a depression, m.
const RAISED_M: f64 = 1e-3;

/// Millimetres in a metre.
pub(crate) const MM_PER_M: f64 = 1000.0;

/// The smallest depression that keeps its water: shallower or smaller ones are filled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Limits {
    /// Spill level − lowest height, m.
    pub min_depth_m: f64,
    /// Area of the depression's cells, m².
    pub min_area_m2: f64,
}

/// A maximal depression of the surface filled from the outlets.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Depression {
    /// Its cells, in index order.
    pub cells: Vec<usize>,
    /// Its highest filled value, m.
    pub spill_level: f64,
    /// Its lowest cell by height, ties by index.
    pub bottom: usize,
    /// Its cell the fill reached first, the lowest on the filled surface; its water leaves over
    /// this cell's receiver.
    pub exit: usize,
}

/// A depression's water balance.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Balance {
    /// The inflow from outside the depression's cells, m³/yr.
    pub q_land: f64,
    /// The mean PET over the depression, the lake evaporation, m/yr.
    pub evaporation: f64,
    /// The mean precipitation over the depression, m/yr.
    pub precipitation: f64,
}

/// The water standing in a depression.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Lake {
    /// Its number in `lake_id`, from 1 in the order of the basins.
    pub id: u32,
    /// Its cells, in index order.
    pub cells: Vec<usize>,
    /// Its level, m.
    pub level: f64,
}

/// A depression that keeps its water, settled by its balance.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Basin {
    /// The depression.
    pub depression: Depression,
    /// Its balance.
    pub balance: Balance,
    /// Whether it passes nothing on: its water evaporates at its equilibrium area.
    pub terminal: bool,
    /// Its lake; `None` for a terminal depression whose equilibrium area is below one cell.
    pub lake: Option<Lake>,
}

/// How the water of a depression settles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fill {
    /// It fills to its spill level and passes its water on.
    Spill,
    /// It keeps its water in this many of its lowest cells.
    Terminal(usize),
}

/// The lowest of `cells` on `height`, ties by index.
fn lowest(height: &Grid<f64>, cells: impl Iterator<Item = usize>) -> Option<usize> {
    let h = height.as_slice();
    cells.min_by_key(|&i| (OrdF64(key(h[i])), i))
}

/// The outlets: the `fixed` cells in index order or, on a closed map, the lowest cell (ties by
/// index), the map's sink.
pub(crate) fn outlets(height: &Grid<f64>, fixed: &Grid<bool>) -> Vec<usize> {
    let drained: Vec<usize> = (0..fixed.len()).filter(|&i| fixed.as_slice()[i]).collect();
    if drained.is_empty() {
        lowest(height, 0..height.len()).into_iter().collect()
    } else {
        drained
    }
}

/// The depressions of `filled` over `height`: the 8-connected cells raised by more than
/// [`RAISED_M`], in the scan order of their first cell.
pub(crate) fn depressions(height: &Grid<f64>, filled: &Drainage) -> Vec<Depression> {
    let surface = filled.surface.as_slice();
    let mut raised = Grid::filled(height.rows(), height.cols(), false);
    for ((cell, &z), &h) in raised
        .as_mut_slice()
        .iter_mut()
        .zip(surface)
        .zip(height.as_slice())
    {
        *cell = z - h > RAISED_M;
    }
    let (label, count) = labels(&raised, Connectivity::Eight);
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); cell_index(count)];
    for (i, &l) in label.as_slice().iter().enumerate() {
        if let Some(cells) = cell_index(l)
            .checked_sub(1)
            .and_then(|k| members.get_mut(k))
        {
            cells.push(i);
        }
    }
    let mut rank = vec![usize::MAX; height.len()];
    for (position, &cell) in filled.order.iter().enumerate() {
        if let Some(r) = rank.get_mut(cell_index(cell)) {
            *r = position;
        }
    }
    members
        .into_iter()
        .filter_map(|cells| {
            let exit = *cells.iter().min_by_key(|&&c| rank[c])?;
            let bottom = lowest(height, cells.iter().copied())?;
            let spill_level = cells
                .iter()
                .fold(f64::NEG_INFINITY, |m, &c| max(m, surface[c]));
            Some(Depression {
                cells,
                spill_level,
                bottom,
                exit,
            })
        })
        .collect()
}

/// Sets every depression shallower or smaller than `limits` to `surface`, the filled surface,
/// in `height`. Returns the depressions kept and the number filled.
pub(crate) fn fill_shallow(
    height: &mut Grid<f64>,
    surface: &Grid<f64>,
    depressions: Vec<Depression>,
    limits: &Limits,
    cell_area: f64,
) -> (Vec<Depression>, usize) {
    let mut kept = Vec::with_capacity(depressions.len());
    let mut filled = 0;
    for depression in depressions {
        let depth = depression.spill_level - height.as_slice()[depression.bottom];
        let area = to_f64(depression.cells.len()) * cell_area;
        if depth < limits.min_depth_m || area < limits.min_area_m2 {
            for &c in &depression.cells {
                height.as_mut_slice()[c] = surface.as_slice()[c];
            }
            filled += 1;
        } else {
            kept.push(depression);
        }
    }
    (kept, filled)
}

/// The balance of `depression` with the inflow `q_land` (m³/yr): the means of PET and
/// precipitation over its cells, in m/yr.
fn balance(depression: &Depression, q_land: f64, fields: &ClimateFields) -> Balance {
    let cells = to_f64(depression.cells.len());
    let mean_m = |grid: &Grid<f64>| {
        let sum = depression
            .cells
            .iter()
            .fold(0.0, |sum, &c| sum + grid.as_slice()[c]);
        sum / cells / MM_PER_M
    };
    Balance {
        q_land,
        evaporation: mean_m(&fields.pet),
        precipitation: mean_m(&fields.precipitation),
    }
}

/// How a depression of `cells` cells of `cell_area` m² settles with `balance`: when evaporation
/// exceeds precipitation, the equilibrium area `A = Q_land / (E − P)` below the depression's
/// area makes it terminal, holding the whole cells that fit in `A`; otherwise it fills to spill.
fn fill(balance: &Balance, cells: usize, cell_area: f64) -> Fill {
    let net = balance.evaporation - balance.precipitation;
    if net <= 0.0 {
        return Fill::Spill;
    }
    let area = balance.q_land / net;
    if area >= to_f64(cells) * cell_area {
        Fill::Spill
    } else {
        Fill::Terminal(to_index(area / cell_area, cells))
    }
}

/// Routes `runoff` (m³/yr per cell) over the receivers of `filled` in reverse fill order and
/// settles every depression at its exit, after every depression upstream of it.
///
/// A depression pools what reaches its cells: its own runoff and the inflow from outside its
/// cells, `Q_land`. At its exit the balance decides: a depression that fills to spill passes the
/// pool on to the exit's receiver, a terminal one keeps it. Returns the basins in the order of
/// `depressions`, their lakes numbered from 1 in that order.
pub(crate) fn settle(
    filled: &Drainage,
    depressions: Vec<Depression>,
    runoff: &Grid<f64>,
    fields: &ClimateFields,
    height: &Grid<f64>,
    cell_area: f64,
) -> Vec<Basin> {
    const NONE: usize = usize::MAX;
    let mut of = vec![NONE; runoff.len()];
    for (k, depression) in depressions.iter().enumerate() {
        for &c in &depression.cells {
            of[c] = k;
        }
    }
    let mut flow = runoff.as_slice().to_vec();
    let mut pooled = vec![0.0; depressions.len()];
    let mut inflow = vec![0.0; depressions.len()];
    let mut balances = vec![Balance::default(); depressions.len()];
    let mut fills = vec![Fill::Spill; depressions.len()];
    for &cell in filled.order.iter().rev() {
        let i = cell_index(cell);
        let k = of[i];
        let carried = if k == NONE {
            flow[i]
        } else {
            pooled[k] += flow[i];
            if i != depressions[k].exit {
                continue;
            }
            balances[k] = balance(&depressions[k], inflow[k], fields);
            fills[k] = fill(&balances[k], depressions[k].cells.len(), cell_area);
            if fills[k] != Fill::Spill {
                continue;
            }
            pooled[k]
        };
        let to = filled.receivers[i];
        if to == i {
            continue;
        }
        flow[to] += carried;
        if of[to] != NONE && of[to] != k {
            inflow[of[to]] += carried;
        }
    }
    let mut next_id = 0_u32;
    depressions
        .into_iter()
        .zip(balances)
        .zip(fills)
        .map(|((depression, balance), fill)| {
            let (terminal, cells, level) = match fill {
                Fill::Spill => (false, depression.cells.clone(), depression.spill_level),
                Fill::Terminal(n) => {
                    let mut lowest = depression.cells.clone();
                    lowest.sort_unstable_by_key(|&c| (OrdF64(key(height.as_slice()[c])), c));
                    lowest.truncate(n);
                    let level = lowest.last().map_or(0.0, |&c| height.as_slice()[c]);
                    lowest.sort_unstable();
                    (true, lowest, level)
                }
            };
            let lake = (!cells.is_empty()).then(|| {
                next_id += 1;
                Lake {
                    id: next_id,
                    cells,
                    level,
                }
            });
            Basin {
                depression,
                balance,
                terminal,
                lake,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stages::terrain::hydrology::discharge::{EPSILON, drain};

    /// A drained cell 0, a lower depression D (cells 2, 3) and an upper depression U (cells 5,
    /// 6) that spills into D over cell 4; cells 7 to 11 slope into U.
    fn steps() -> Grid<f64> {
        let h = vec![
            0.0, 20.0, 10.0, 10.0, 30.0, 25.0, 25.0, 40.0, 50.0, 60.0, 70.0, 80.0,
        ];
        Grid::from_vec(1, 12, h).unwrap()
    }

    /// PET 1250 mm/yr, runoff 100 mm/yr, precipitation 250 mm/yr except `upper` on U.
    fn weather(upper: f64) -> ClimateFields {
        let precipitation: Vec<f64> = (0..12)
            .map(|i| if i == 5 || i == 6 { upper } else { 250.0 })
            .collect();
        let precipitation = Grid::from_vec(1, 12, precipitation).unwrap();
        ClimateFields {
            aet: precipitation.map(|p| p - 100.0),
            precipitation,
            pet: Grid::filled(1, 12, 1250.0),
            runoff: Grid::filled(1, 12, 100.0),
        }
    }

    /// The basins of [`steps`] on cells of 1 m² with `fields`.
    fn basins(fields: &ClimateFields) -> Vec<Basin> {
        let height = steps();
        let fixed = Grid::from_vec(1, 12, (0..12).map(|i| i == 0).collect()).unwrap();
        let filled = drain(&height, &outlets(&height, &fixed), 1.0);
        let found = depressions(&height, &filled);
        let runoff = fields.runoff.map(|mm| mm / MM_PER_M);
        settle(&filled, found, &runoff, fields, &height, 1.0)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn outlets_are_the_fixed_cells_or_the_lowest_cell() {
        let height = Grid::from_vec(1, 4, vec![3.0, 1.0, 2.0, 1.0]).unwrap();
        let fixed = Grid::from_vec(1, 4, vec![false, false, true, true]).unwrap();
        assert_eq!(outlets(&height, &fixed), [2, 3]);
        let closed = Grid::filled(1, 4, false);
        assert_eq!(outlets(&height, &closed), [1], "ties go to the lower index");
    }

    #[test]
    fn depressions_know_their_spill_level_bottom_and_exit() {
        let height = steps();
        let filled = drain(&height, &[0], 1.0);
        let found = depressions(&height, &filled);
        let lower = Depression {
            cells: vec![2, 3],
            spill_level: 20.0 + EPSILON + EPSILON,
            bottom: 2,
            exit: 2,
        };
        let upper = Depression {
            cells: vec![5, 6],
            spill_level: 30.0 + EPSILON + EPSILON,
            bottom: 5,
            exit: 5,
        };
        assert_eq!(found, [lower, upper]);
        assert_eq!((filled.receivers[2], filled.receivers[5]), (1, 4));
    }

    #[test]
    fn shallow_depressions_are_filled_to_the_surface() {
        let mut height = steps();
        let filled = drain(&height, &[0], 1.0);
        let found = depressions(&height, &filled);
        let limits = Limits {
            min_depth_m: 6.0,
            min_area_m2: 0.0,
        };
        let (kept, count) = fill_shallow(&mut height, &filled.surface, found, &limits, 1.0);
        assert_eq!((kept.len(), count), (1, 1));
        assert_eq!(kept[0].cells, [2, 3], "10 m deep stays");
        for c in 0..12 {
            let want = if c == 5 || c == 6 {
                filled.surface.as_slice()[c]
            } else {
                steps().as_slice()[c]
            };
            assert_eq!(height.as_slice()[c].to_bits(), want.to_bits(), "cell {c}");
        }
        let small = Limits {
            min_depth_m: 0.0,
            min_area_m2: 2.5,
        };
        let found = depressions(&steps(), &filled);
        let (kept, count) = fill_shallow(&mut steps(), &filled.surface, found, &small, 1.0);
        assert_eq!((kept.len(), count), (0, 2), "two cells are below 2.5 m²");
    }

    #[test]
    fn a_terminal_depression_passes_nothing_on() {
        // U is arid: Q_land = 5 cells × 0.1 m³/yr, A = 0.5 m² over E − P = 1 m/yr, below one
        // cell; D then receives only cell 4's runoff.
        let arid = basins(&weather(250.0));
        let (lower, upper) = (&arid[0], &arid[1]);
        assert!(upper.terminal && upper.lake.is_none());
        assert!(close(upper.balance.q_land, 0.5));
        assert!(close(upper.balance.evaporation, 1.25));
        assert!(close(upper.balance.precipitation, 0.25));
        assert!(close(lower.balance.q_land, 0.1));
        assert!(lower.terminal && lower.lake.is_none());
        // U is wet: it fills to spill and passes its pool of 7 cells on; D receives 8 cells.
        let wet = basins(&weather(2000.0));
        let (lower, upper) = (&wet[0], &wet[1]);
        assert!(!upper.terminal);
        assert!(close(lower.balance.q_land, 0.8));
        let lake = upper.lake.as_ref().unwrap();
        assert_eq!((lake.id, lake.cells.as_slice()), (1, [5, 6].as_slice()));
        assert_eq!(lake.level.to_bits(), upper.depression.spill_level.to_bits());
    }

    #[test]
    fn fill_follows_the_equilibrium_area() {
        let at = |q_land: f64, e: f64, p: f64| Balance {
            q_land,
            evaporation: e,
            precipitation: p,
        };
        assert_eq!(fill(&at(0.0, 1.0, 1.0), 5, 2.0), Fill::Spill, "E = P");
        assert_eq!(fill(&at(0.0, 0.5, 1.0), 5, 2.0), Fill::Spill, "E < P");
        assert_eq!(
            fill(&at(10.0, 2.0, 1.0), 5, 2.0),
            Fill::Spill,
            "A is the area"
        );
        assert_eq!(fill(&at(9.0, 2.0, 1.0), 5, 2.0), Fill::Terminal(4));
        assert_eq!(fill(&at(5.0, 2.0, 1.0), 5, 2.0), Fill::Terminal(2));
        assert_eq!(fill(&at(1.9, 2.0, 1.0), 5, 2.0), Fill::Terminal(0));
        assert_eq!(fill(&at(0.0, 2.0, 1.0), 5, 2.0), Fill::Terminal(0));
    }

    #[test]
    fn a_terminal_lake_takes_the_lowest_cells_at_the_last_ones_height() {
        // One depression of cells 1 to 4 at 5, 2, 3, 2 m behind a 9 m rim, drained at cell 6.
        let height = Grid::from_vec(1, 7, vec![9.0, 5.0, 2.0, 3.0, 2.0, 9.0, 0.0]).unwrap();
        let filled = drain(&height, &[6], 1.0);
        let found = depressions(&height, &filled);
        assert_eq!(found.len(), 1);
        let fields = ClimateFields {
            precipitation: Grid::filled(1, 7, 0.0),
            pet: Grid::filled(1, 7, 1000.0),
            aet: Grid::filled(1, 7, 0.0),
            runoff: Grid::filled(1, 7, 0.0),
        };
        // Cell 0 drains into the depression with 3 m³/yr; E − P = 1 m/yr: three whole cells.
        let mut runoff = Grid::filled(1, 7, 0.0);
        runoff.as_mut_slice()[0] = 3.0;
        let basins = settle(&filled, found, &runoff, &fields, &height, 1.0);
        let lake = basins[0].lake.as_ref().unwrap();
        assert!(basins[0].terminal);
        assert_eq!(lake.cells, [2, 3, 4]);
        assert_eq!(lake.level.to_bits(), 3.0_f64.to_bits());
        assert_eq!(basins[0].depression.bottom, 2);
    }
}
