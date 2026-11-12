//! The per-step process `hillslope`: linear diffusion `∂h/∂t = ∇·(kd ∇h)` by the alternating
//! direction implicit scheme of Peaceman and Rachford (1955).
//!
//! One step is two half steps, each implicit along one axis and explicit along the other, and
//! each a set of tridiagonal systems solved by the Thomas algorithm
//! ([`tridiagonal`](crate::kernels::tridiagonal)). The scheme is unconditionally stable and
//! second-order accurate in time. The diffusivity at the interface of two cells is the
//! arithmetic mean of the two cells; fixed cells (the drained edges) are Dirichlet and keep their
//! height; the map's edges are closed (no flux). Per cell, a net loss over the step is booked
//! as erosion and a net gain as deposition.

use serde::Deserialize;

use super::booking::book;
use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::tridiagonal;
use crate::stage::{LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage};
use crate::world::Layer;

/// The parameters of `hillslope`: none; the diffusivity is the layer `diffusivity`.
#[derive(Debug, Default, Deserialize)]
pub struct HillslopeParams {}

/// The process `hillslope`.
pub struct Hillslope;

impl TypedStage for Hillslope {
    type Params = HillslopeParams;
    const ID: &'static str = "hillslope";
    const SUMMARY: &'static str = "Linear hillslope diffusion by a Peaceman–Rachford ADI scheme with Thomas tridiagonal solves; drained edges are Dirichlet, closed edges no-flux; cumulative erosion and deposition.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &HillslopeParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("diffusivity"),
            LayerSpec::mask("fixed"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
        ]
    }

    fn writes(&self, _: &HillslopeParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
        ]
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        _: &HillslopeParams,
    ) -> Result<StageReport, StageError> {
        let Some(step) = ctx.step() else {
            return Err(ctx.fail("runs only as a process of landscape_evolution"));
        };
        let cell_m = ctx.frame().cell_m;
        let start = ctx.take_f64("height")?;
        let mut erosion = ctx.take_f64("erosion")?;
        let mut deposition = ctx.take_f64("deposition")?;
        let medium = Medium {
            kd: ctx.f64("diffusivity")?.as_slice(),
            fixed: ctx.mask("fixed")?.as_slice(),
            // The half step: dt / 2 over the squared cell size.
            ratio: 0.5 * step.dt_yr / (cell_m * cell_m),
        };
        let height = diffuse(&medium, &start)
            .map_err(|e| ctx.fail(format!("the tridiagonal solve failed: {e}")))?;
        book(&start, &height, &mut erosion, &mut deposition);
        ctx.write("height", Layer::F64(height))?;
        ctx.write("erosion", Layer::F64(erosion))?;
        ctx.write("deposition", Layer::F64(deposition))?;
        Ok(StageReport::default())
    }
}

/// What diffuses: the diffusivity and the Dirichlet cells, with the half-step ratio
/// `dt / (2·Δx²)`.
struct Medium<'a> {
    kd: &'a [f64],
    fixed: &'a [bool],
    ratio: f64,
}

impl Medium<'_> {
    /// The interface diffusivity between cells `a` and `b`.
    fn interface(&self, a: usize, b: usize) -> f64 {
        f64::midpoint(self.kd[a], self.kd[b])
    }
}

/// One axis of the grid as a set of lines: cell `k` of line `l` is `l·line_step + k·step`, so
/// the next line's neighbour of a cell lies `line_step` cells on.
#[derive(Clone, Copy)]
struct Axis {
    lines: usize,
    len: usize,
    line_step: usize,
    step: usize,
}

impl Axis {
    /// The lines are the rows; the cells of a line run along the columns.
    fn along_rows<T>(g: &Grid<T>) -> Self
    where
        T: Copy,
    {
        Self {
            lines: g.rows(),
            len: g.cols(),
            line_step: g.cols(),
            step: 1,
        }
    }

    /// The lines are the columns; the cells of a line run along the rows.
    fn along_columns<T>(g: &Grid<T>) -> Self
    where
        T: Copy,
    {
        Self {
            lines: g.cols(),
            len: g.rows(),
            line_step: 1,
            step: g.cols(),
        }
    }

    fn cell(self, line: usize, k: usize) -> usize {
        line * self.line_step + k * self.step
    }
}

/// One full step: implicit along the rows and explicit along the columns, then the other way.
fn diffuse(medium: &Medium<'_>, start: &Grid<f64>) -> Result<Grid<f64>, KernelError> {
    let (rows, cols) = (Axis::along_rows(start), Axis::along_columns(start));
    let half = half_step(medium, start.as_slice(), rows, cols)?;
    let full = half_step(medium, &half, cols, rows)?;
    let len = full.len();
    Grid::from_vec(start.rows(), start.cols(), full).map_err(|_| KernelError::Shape {
        rows: start.rows(),
        cols: start.cols(),
        len,
    })
}

/// One half step of `h`: implicit along the lines of `implicit`, explicit across them (the
/// neighbours on the lines either side, `explicit` being the other axis of the grid).
fn half_step(
    medium: &Medium<'_>,
    h: &[f64],
    implicit: Axis,
    explicit: Axis,
) -> Result<Vec<f64>, KernelError> {
    let mut next = h.to_vec();
    let n = implicit.len;
    let (mut lower, mut diag, mut upper) = (vec![0.0; n], vec![1.0; n], vec![0.0; n]);
    let (mut rhs, mut scratch) = (vec![0.0; n], vec![0.0; n]);
    for line in 0..implicit.lines {
        for k in 0..n {
            let i = implicit.cell(line, k);
            (lower[k], diag[k], upper[k], rhs[k]) = (0.0, 1.0, 0.0, h[i]);
            if medium.fixed[i] {
                continue;
            }
            let mut across = 0.0;
            if line > 0 {
                across += medium.interface(i, i - explicit.step) * (h[i - explicit.step] - h[i]);
            }
            if line + 1 < implicit.lines {
                across += medium.interface(i, i + explicit.step) * (h[i + explicit.step] - h[i]);
            }
            rhs[k] = h[i] + medium.ratio * across;
            if k > 0 {
                let a = medium.ratio * medium.interface(i, implicit.cell(line, k - 1));
                lower[k] = -a;
                diag[k] += a;
            }
            if k + 1 < n {
                let a = medium.ratio * medium.interface(i, implicit.cell(line, k + 1));
                upper[k] = -a;
                diag[k] += a;
            }
        }
        tridiagonal::solve(&lower, &diag, &upper, &mut rhs, &mut scratch)?;
        for (k, &value) in rhs.iter().enumerate() {
            next[implicit.cell(line, k)] = value;
        }
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use libm::exp;

    use super::Hillslope;
    use crate::grid::Grid;
    use crate::kernels::num::{max, to_f64};
    use crate::stage::{LayerSpec, Stage, StageError};
    use crate::stages::evolution::test_support::{evolve, processes};
    use crate::stages::terrain::support::run_stage;
    use crate::world::{Layer, MapFrame, World};

    const KD: f64 = 0.01;

    fn medium_world(n: usize, cell_m: f64, height: Grid<f64>, fixed: Grid<bool>) -> World {
        let mut w = World::new(MapFrame {
            rows: n,
            cols: n,
            cell_m,
        });
        w.insert("height", Layer::F64(height)).unwrap();
        w.insert("diffusivity", Layer::F64(Grid::filled(n, n, KD)))
            .unwrap();
        w.insert("fixed", Layer::Mask(fixed)).unwrap();
        w.insert("erosion", Layer::F64(Grid::filled(n, n, 0.0)))
            .unwrap();
        w.insert("deposition", Layer::F64(Grid::filled(n, n, 0.0)))
            .unwrap();
        w
    }

    fn hillslope() -> Vec<crate::stages::evolution::PlannedProcess> {
        processes("[[process]]\nid = \"hillslope\"")
    }

    /// The squared distance in m² of cell `(r, c)` from the grid centre cell `centre`.
    fn dist2(r: usize, c: usize, centre: f64, cell_m: f64) -> f64 {
        let (dr, dc) = (to_f64(r) - centre, to_f64(c) - centre);
        (dr * dr + dc * dc) * cell_m * cell_m
    }

    // Physics test 3.
    #[test]
    fn adi_matches_the_heat_kernel() {
        let (n, cell_m, sigma0, amp, centre) = (128, 10.0, 80.0, 10.0, 64.0);
        let mut height = Grid::filled(n, n, 0.0);
        for r in 0..n {
            for c in 0..n {
                let d2 = dist2(r, c, centre, cell_m);
                height.set(r, c, amp * exp(-d2 / (2.0 * sigma0 * sigma0)));
            }
        }
        let mut w = medium_world(n, cell_m, height, Grid::filled(n, n, false));
        evolve(&mut w, &hillslope(), 20, 1e4, 1).unwrap();
        let spread = sigma0 * sigma0 + 2.0 * KD * 20.0 * 1e4;
        let peak = amp * sigma0 * sigma0 / spread;
        let after = w.f64("height").unwrap();
        let mut worst = 0.0;
        for r in 0..n {
            for c in 0..n {
                let want = peak * exp(-dist2(r, c, centre, cell_m) / (2.0 * spread));
                worst = max(worst, (after.get(r, c) - want).abs());
            }
        }
        assert!(worst < 0.01 * peak, "worst error {worst} vs 1 % of {peak}");
    }

    #[test]
    fn hillslope_books_loss_and_gain() {
        let side = 16;
        let mut height = Grid::filled(side, side, 0.0);
        height.set(8, 8, 50.0);
        let mut fixed = Grid::filled(side, side, false);
        fixed.set(8, 9, true);
        let mut world = medium_world(side, 100.0, height.clone(), fixed);
        evolve(&mut world, &hillslope(), 1, 1e5, 1).unwrap();
        let after = world.f64("height").unwrap();
        let (eroded, deposited) = (
            world.f64("erosion").unwrap(),
            world.f64("deposition").unwrap(),
        );
        let zero = |x: f64| x.to_bits() == 0.0_f64.to_bits();
        assert!(
            eroded.get(8, 8) > 0.0 && zero(deposited.get(8, 8)),
            "the spike erodes"
        );
        assert!(
            deposited.get(8, 7) > 0.0 && zero(eroded.get(8, 7)),
            "a neighbour gains"
        );
        assert!(zero(after.get(8, 9)), "a fixed cell keeps its height");
        assert!(zero(eroded.get(8, 9)) && zero(deposited.get(8, 9)));
        for i in 0..side * side {
            let (lost, gained) = (eroded.as_slice()[i], deposited.as_slice()[i]);
            assert!(lost >= 0.0 && gained >= 0.0);
            let change = after.as_slice()[i] - height.as_slice()[i];
            assert!((gained - lost - change).abs() < 1e-12, "cell {i}");
        }
    }

    #[test]
    fn closed_edges_keep_the_volume() {
        let n = 12;
        let mut height = Grid::filled(n, n, 0.0);
        for r in 0..n {
            for c in 0..n {
                height.set(r, c, to_f64((r * 7 + c * 3) % 11));
            }
        }
        let before: f64 = height.as_slice().iter().sum();
        let mut w = medium_world(n, 100.0, height, Grid::filled(n, n, false));
        evolve(&mut w, &hillslope(), 5, 5e4, 1).unwrap();
        let after: f64 = w.f64("height").unwrap().as_slice().iter().sum();
        assert!(
            (after - before).abs() < 1e-9 * before,
            "{after} vs {before}"
        );
    }

    #[test]
    fn hillslope_declares_its_layers_and_runs_only_in_the_evolution() {
        let stage: &dyn Stage = &Hillslope;
        let params = stage.params(&toml::Table::new()).unwrap();
        let names = |l: Vec<LayerSpec>| l.iter().map(|s| s.name).collect::<Vec<_>>();
        assert_eq!(
            names(stage.reads(params.as_ref())),
            ["height", "diffusivity", "fixed", "erosion", "deposition"]
        );
        assert_eq!(
            names(stage.writes(params.as_ref())),
            ["height", "erosion", "deposition"]
        );
        assert!(stage.params(&"kd = 1.0".parse().unwrap()).is_err());
        let mut w = medium_world(4, 100.0, Grid::filled(4, 4, 1.0), Grid::filled(4, 4, false));
        let err = run_stage(&mut w, &Hillslope, "").unwrap_err();
        assert!(matches!(&err, StageError::Failed { stage, .. } if stage == "hillslope"));
    }
}
