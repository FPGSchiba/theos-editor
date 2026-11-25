//! Steps 1 to 5 of `hydrology`: the outlets, the depressions of the filled surface, the shallow
//! pits filled in `height`, the inflow of every depression, and its lake by water balance
//! (Bracht-Flyr et al. 2013, rain on the lake counted, rearranged for the area); and, after the
//! routing, the dry nested pits of terminal depressions filled in `height`.

use super::discharge::Drainage;
use crate::grid::Grid;
use crate::kernels::flood::EPSILON;
use crate::kernels::flood::priority_flood_where;
use crate::kernels::labels::{Connectivity, labels};
use crate::kernels::num::{max, to_f64, to_usize};
use crate::kernels::routing::lowest;
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
    /// The land inflow `Q_land`: the runoff that reaches the depression from outside its cells
    /// and, for a terminal depression, the runoff of its cells the lake leaves dry, m³/yr.
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
    /// Its level, m: the spill level of a lake that fills to spill; for a terminal lake the
    /// value its fill from the bottom reached at its last cell (the height of that cell, or the
    /// rim it passed to reach it, plus ε steps).
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
#[derive(Debug, Clone, PartialEq)]
enum Fill {
    /// It fills to its spill level and passes its water on.
    Spill,
    /// It keeps its water in `cells`, the first its fill from the bottom reaches (none below one
    /// cell), at `level`, the fill's value at the last of them.
    Terminal {
        /// The lake cells, in fill order.
        cells: Vec<usize>,
        /// The lake level, m.
        level: f64,
    },
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
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); to_usize(count)];
    for (i, &l) in label.as_slice().iter().enumerate() {
        if let Some(cells) = to_usize(l).checked_sub(1).and_then(|k| members.get_mut(k)) {
            cells.push(i);
        }
    }
    let mut rank = vec![usize::MAX; height.len()];
    for (position, &cell) in filled.order.iter().enumerate() {
        if let Some(r) = rank.get_mut(to_usize(cell)) {
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

/// The means of PET (the lake evaporation `E`) and precipitation (`P`) over the cells of
/// `depression`, in m/yr.
fn means(depression: &Depression, fields: &ClimateFields) -> (f64, f64) {
    let cells = to_f64(depression.cells.len());
    let mean_m = |grid: &Grid<f64>| {
        let sum = depression
            .cells
            .iter()
            .fold(0.0, |sum, &c| sum + grid.as_slice()[c]);
        sum / cells / MM_PER_M
    };
    (mean_m(&fields.pet), mean_m(&fields.precipitation))
}

/// Whether a depression of `cells` cells of `cell_area` m² fills to spill: when `net = E − P`
/// is not positive, or when its whole area evaporates no more than the inflow `q_out` from
/// outside its cells (the equilibrium area `A = q_out / net` reaches its area). A full lake
/// leaves no cell dry, so `q_out` is its whole land inflow.
fn spills(q_out: f64, net: f64, cells: usize, cell_area: f64) -> bool {
    net <= 0.0 || q_out / net >= to_f64(cells) * cell_area
}

/// The lake of a terminal depression whose cells, in the order the lake reaches them, have the
/// runoff `runoff` (m³/yr each): the largest `n` with
/// `n·a·(E − P) <= q_out + the runoff of the cells after the first n`, where `a` is `cell_area`
/// and `net` is `E − P` (positive). The dry cells are land in
/// the lake's catchment. The left side rises and the right side falls with `n`, so one pass
/// from `n = 0` finds it. Returns `n` and that land inflow, m³/yr.
fn terminal_lake(runoff: &[f64], q_out: f64, net: f64, cell_area: f64) -> (usize, f64) {
    let mut dry = vec![0.0; runoff.len() + 1];
    for (j, r) in runoff.iter().enumerate().rev() {
        dry[j] = dry[j + 1] + r;
    }
    let mut n = 0;
    while n < runoff.len() && to_f64(n + 1) * cell_area * net <= q_out + dry[n + 1] {
        n += 1;
    }
    (n, q_out + dry[n])
}

/// Decides every depression at its exit. The fills from the terminal bottoms share one surface
/// and one visited table: each runs over its own depression only, so they never overlap.
struct Decider<'a> {
    /// Runoff per cell, m³/yr.
    runoff: &'a [f64],
    /// The yearly water fields, mm/yr.
    fields: &'a ClimateFields,
    /// Cell area, m².
    cell_area: f64,
    /// The depression of every cell (its index in `settle`'s list), `usize::MAX` outside.
    of: &'a [usize],
    /// `height`, raised where a fill from a terminal bottom passes over a rim.
    surface: Grid<f64>,
    /// The cells those fills have entered.
    visited: Vec<bool>,
}

impl Decider<'_> {
    /// The balance and the fill of depression `k` with the inflow `q_out` from outside its cells.
    ///
    /// A terminal lake grows connected from the bottom: the depression's cells are taken in the
    /// order a priority-flood from the bottom over the depression reaches them (ε, ties by index),
    /// so a nested pit joins only once the level passes its rim. The cells not yet reached are
    /// land in the lake's catchment ([`terminal_lake`]).
    fn decide(&mut self, k: usize, depression: &Depression, q_out: f64) -> (Balance, Fill) {
        let (evaporation, precipitation) = means(depression, self.fields);
        let net = evaporation - precipitation;
        let balance = |q_land| Balance {
            q_land,
            evaporation,
            precipitation,
        };
        if spills(q_out, net, depression.cells.len(), self.cell_area) {
            return (balance(q_out), Fill::Spill);
        }
        let of = self.of;
        let order = priority_flood_where(
            &mut self.surface,
            &mut self.visited,
            &[depression.bottom],
            EPSILON,
            |j, _| of[j] == k,
        );
        let mut cells: Vec<usize> = order.iter().map(|&c| to_usize(c)).collect();
        let in_order: Vec<f64> = cells.iter().map(|&c| self.runoff[c]).collect();
        let (n, q_land) = terminal_lake(&in_order, q_out, net, self.cell_area);
        cells.truncate(n);
        let level = cells.last().map_or(0.0, |&c| self.surface.as_slice()[c]);
        (balance(q_land), Fill::Terminal { cells, level })
    }
}

/// Routes `runoff` (m³/yr per cell) over the receivers of `filled` in reverse fill order and
/// settles every depression at its exit, after every depression upstream of it.
///
/// A depression pools what reaches its cells: its own runoff and the inflow from outside its
/// cells. At its exit the balance decides ([`Decider::decide`]): a depression that fills to
/// spill passes the pool on to the exit's receiver, a terminal one keeps it in a lake grown from
/// its bottom over `height`. Returns the basins in the order of `depressions`, their lakes
/// numbered from 1 in that order.
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
    let mut decider = Decider {
        runoff: runoff.as_slice(),
        fields,
        cell_area,
        of: &of,
        surface: height.clone(),
        visited: vec![false; height.len()],
    };
    let mut flow = runoff.as_slice().to_vec();
    let mut pooled = vec![0.0; depressions.len()];
    let mut inflow = vec![0.0; depressions.len()];
    let mut balances = vec![Balance::default(); depressions.len()];
    let mut fills = vec![Fill::Spill; depressions.len()];
    for &cell in filled.order.iter().rev() {
        let i = to_usize(cell);
        let k = of[i];
        let carried = if k == NONE {
            flow[i]
        } else {
            pooled[k] += flow[i];
            if i != depressions[k].exit {
                continue;
            }
            let (balance, fill) = decider.decide(k, &depressions[k], inflow[k]);
            balances[k] = balance;
            fills[k] = fill;
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
                Fill::Terminal { mut cells, level } => {
                    cells.sort_unstable();
                    (true, cells, level)
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

/// Sets every cell outside the lakes of `basins` that `surface`, the routing surface, raises by
/// more than [`RAISED_M`] to that surface in `height`, so river beds never cross dry pits (spec
/// ruling 32). Only the dry nested pits of terminal depressions are raised there: the second
/// fill starts at their bottom, so it fills the pits nested in them that the lake does not
/// cover. Lake cells keep their bottom. Returns the number of cells set.
pub(crate) fn fill_dry_pits(
    height: &mut Grid<f64>,
    surface: &Grid<f64>,
    basins: &[Basin],
) -> usize {
    let mut wet = vec![false; height.len()];
    for lake in basins.iter().filter_map(|b| b.lake.as_ref()) {
        for &c in &lake.cells {
            wet[c] = true;
        }
    }
    let mut set = 0;
    let cells = height.as_mut_slice().iter_mut().zip(surface.as_slice());
    for ((h, &z), &is_lake) in cells.zip(&wet) {
        if !is_lake && z - *h > RAISED_M {
            *h = z;
            set += 1;
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::flood::EPSILON;
    use crate::kernels::routing::outlets;
    use crate::stages::terrain::hydrology::discharge::drain;

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
        // U is arid: 5 cells × 0.1 m³/yr come from outside and its 2 dry cells add 0.2 m³/yr;
        // one lake cell would evaporate 1 m³/yr over E − P = 1 m/yr, more than the 0.6 m³/yr
        // left without its runoff, so there is no lake. D then receives only cell 4's runoff.
        let arid = basins(&weather(250.0));
        let (lower, upper) = (&arid[0], &arid[1]);
        assert!(upper.terminal && upper.lake.is_none());
        assert!(close(upper.balance.q_land, 0.7));
        assert!(close(upper.balance.evaporation, 1.25));
        assert!(close(upper.balance.precipitation, 0.25));
        assert!(close(lower.balance.q_land, 0.1 + 0.2));
        assert!(lower.terminal && lower.lake.is_none());
        // U is wet: it fills to spill and passes its pool of 7 cells on; D receives 8 cells.
        let wet = basins(&weather(2000.0));
        let (lower, upper) = (&wet[0], &wet[1]);
        assert!(!upper.terminal);
        assert!(
            close(upper.balance.q_land, 0.5),
            "a full lake has no dry cells"
        );
        assert!(close(lower.balance.q_land, 0.8 + 0.2));
        let lake = upper.lake.as_ref().unwrap();
        assert_eq!((lake.id, lake.cells.as_slice()), (1, [5, 6].as_slice()));
        assert_eq!(lake.level.to_bits(), upper.depression.spill_level.to_bits());
    }

    #[test]
    fn a_depression_spills_when_its_whole_area_cannot_evaporate_the_inflow() {
        assert!(spills(0.0, 0.0, 5, 2.0), "E = P");
        assert!(spills(0.0, -0.5, 5, 2.0), "E < P");
        assert!(spills(10.0, 1.0, 5, 2.0), "A is the area");
        assert!(!spills(9.0, 1.0, 5, 2.0));
        assert!(!spills(0.0, 1.0, 5, 2.0));
    }

    #[test]
    fn a_terminal_lake_counts_its_dry_cells_as_land() {
        // (n, inflow): the largest n with n·a·(E − P) <= Q_out + the runoff of the cells after
        // the first n, and that inflow.
        let lake = |runoff: &[f64], q_out: f64| {
            let (n, inflow) = terminal_lake(runoff, q_out, 1.0, 1.0);
            assert!(close(inflow, q_out + runoff[n..].iter().sum::<f64>()));
            n
        };
        assert_eq!(lake(&[1.0, 1.0, 1.0, 1.0], 2.0), 3, "3 <= 2 + 1, 4 > 2");
        assert_eq!(
            lake(&[0.0, 5.0, 0.0], 0.0),
            1,
            "the second cell's runoff feeds one"
        );
        assert_eq!(lake(&[0.1, 0.1], 0.5), 0, "below one cell");
        assert_eq!(
            lake(&[0.0, 0.0, 0.0], 2.5),
            2,
            "no dry runoff: whole cells of A"
        );
        let (n, inflow) = terminal_lake(&[], 1.0, 1.0, 1.0);
        assert_eq!((n, inflow.to_bits()), (0, 1.0_f64.to_bits()));
    }

    #[test]
    fn a_terminal_lake_grows_from_its_bottom_over_its_rims() {
        // One depression of cells 1 to 4 at 5, 2, 3, 2 m behind a 9 m rim, drained at cell 6.
        // Cell 4 is as low as the bottom (cell 2) but lies behind cell 3, a 3 m rim.
        let height = Grid::from_vec(1, 7, vec![9.0, 5.0, 2.0, 3.0, 2.0, 9.0, 0.0]).unwrap();
        let filled = drain(&height, &[6], 1.0);
        let fields = ClimateFields {
            precipitation: Grid::filled(1, 7, 0.0),
            pet: Grid::filled(1, 7, 1000.0),
            aet: Grid::filled(1, 7, 0.0),
            runoff: Grid::filled(1, 7, 0.0),
        };
        // Cell 0 drains into the depression with `q_out` m³/yr; E − P = 1 m/yr.
        let lake = |q_out: f64| {
            let found = depressions(&height, &filled);
            assert_eq!(found.len(), 1);
            let mut runoff = Grid::filled(1, 7, 0.0);
            runoff.as_mut_slice()[0] = q_out;
            let basins = settle(&filled, found, &runoff, &fields, &height, 1.0);
            assert!(basins[0].terminal);
            assert_eq!(basins[0].depression.bottom, 2);
            basins[0].lake.clone().unwrap()
        };
        // Two cells: the rim joins before the cell behind it, at the rim's height.
        let two = lake(2.0);
        assert_eq!(two.cells, [2, 3]);
        assert_eq!(two.level.to_bits(), 3.0_f64.to_bits());
        // Three cells: the cell behind the rim joins once the level passes the rim.
        let three = lake(3.0);
        assert_eq!(three.cells, [2, 3, 4]);
        assert_eq!(three.level.to_bits(), (3.0 + EPSILON).to_bits());
    }
}
