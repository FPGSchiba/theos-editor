//! The per-step process `route_steepest`: the D8 steepest-descent receiver of every cell.

use crate::kernels::routing::steepest_receivers;
use serde::Deserialize;

use crate::stage::{LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage};
use crate::world::Layer;

/// The parameters of `route_steepest`: none.
#[derive(Debug, Default, Deserialize)]
pub struct RouteSteepestParams {}

/// The process `route_steepest` (Braun & Willett 2013): every cell's receiver is the D8
/// neighbour with the steepest drop ÷ distance, equal slopes going to the lower neighbour index.
/// A cell with no lower neighbour (a pit or a flat) and a fixed cell receive themselves.
pub struct RouteSteepest;

impl TypedStage for RouteSteepest {
    type Params = RouteSteepestParams;
    const ID: &'static str = "route_steepest";
    const SUMMARY: &'static str = "Steepest-descent receiver per cell (D8 distances), ties by index; pits and drained edges receive themselves.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &RouteSteepestParams) -> Vec<LayerSpec> {
        vec![LayerSpec::f64("height"), LayerSpec::mask("fixed")]
    }

    fn writes(&self, _: &RouteSteepestParams) -> Vec<LayerSpec> {
        vec![LayerSpec::u32("receivers")]
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        _: &RouteSteepestParams,
    ) -> Result<StageReport, StageError> {
        let cell_m = ctx.frame().cell_m;
        let receivers = steepest_receivers(ctx.f64("height")?, ctx.mask("fixed")?, cell_m);
        ctx.write("receivers", Layer::U32(receivers))?;
        Ok(StageReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::RouteSteepest;
    use crate::grid::Grid;
    use crate::stage::{LayerSpec, Stage};
    use crate::stages::terrain::support::{run_stage, world};
    use crate::world::Layer;

    /// The receivers `route_steepest` gives a 3 x 3 world of 100 m cells.
    fn receivers(height: [f64; 9], fixed: [bool; 9]) -> Vec<u32> {
        let mut w = world(3, 3);
        let height = Grid::from_vec(3, 3, height.to_vec()).unwrap();
        let fixed = Grid::from_vec(3, 3, fixed.to_vec()).unwrap();
        w.insert("height", Layer::F64(height)).unwrap();
        w.insert("fixed", Layer::Mask(fixed)).unwrap();
        run_stage(&mut w, &RouteSteepest, "").unwrap();
        w.u32("receivers").unwrap().as_slice().to_vec()
    }

    /// A centre at 10 m in a ring at 20 m, with `low` cells set to their heights.
    fn ring(low: &[(usize, f64)]) -> [f64; 9] {
        let mut h = [20.0; 9];
        h[4] = 10.0;
        for &(i, v) in low {
            h[i] = v;
        }
        h
    }

    const OPEN: [bool; 9] = [false; 9];

    #[test]
    fn the_receiver_is_the_steepest_drop_per_distance() {
        // N drops 1 m over 100 m (0.0100); NE 1.4 m over 141.4 m (0.0099): N wins.
        assert_eq!(receivers(ring(&[(1, 9.0), (2, 8.6)]), OPEN)[4], 1);
        // NE 1.5 m over 141.4 m (0.0106) beats N.
        assert_eq!(receivers(ring(&[(1, 9.0), (2, 8.5)]), OPEN)[4], 2);
    }

    #[test]
    fn equal_slopes_go_to_the_lower_neighbour_index() {
        assert_eq!(receivers(ring(&[(5, 9.0), (3, 9.0)]), OPEN)[4], 3);
        assert_eq!(receivers(ring(&[(7, 9.0), (1, 9.0)]), OPEN)[4], 1);
        assert_eq!(receivers(ring(&[(6, 9.0), (2, 9.0)]), OPEN)[4], 2);
    }

    #[test]
    fn pits_flats_and_fixed_cells_receive_themselves() {
        let pit = receivers(ring(&[]), OPEN);
        assert_eq!(pit[4], 4, "the centre is a pit");
        assert_eq!(pit[0], 4, "a corner drains diagonally into the pit");
        let flat = receivers([5.0; 9], OPEN);
        assert_eq!(
            flat,
            (0..9).collect::<Vec<u32>>(),
            "an equal neighbour is not lower"
        );
        let mut fixed = OPEN;
        fixed[4] = true;
        let held = receivers(ring(&[(1, 0.0)]), fixed);
        assert_eq!(
            held[4], 4,
            "a fixed cell is an outlet even with a lower neighbour"
        );
    }

    #[test]
    fn route_steepest_declares_its_layers() {
        let stage: &dyn Stage = &RouteSteepest;
        let params = stage.params(&toml::Table::new()).unwrap();
        assert_eq!(
            stage.reads(params.as_ref()),
            [LayerSpec::f64("height"), LayerSpec::mask("fixed")]
        );
        assert_eq!(stage.writes(params.as_ref()), [LayerSpec::u32("receivers")]);
        assert!(stage.params(&"ties = 1".parse().unwrap()).is_err());
    }
}
