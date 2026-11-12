//! The per-step process `uplift`: rock uplift less subsidence over one time step.

use serde::Deserialize;

use crate::stage::{LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage};
use crate::world::Layer;

/// The parameters of `uplift`: none.
#[derive(Debug, Default, Deserialize)]
pub struct UpliftParams {}

/// The process `uplift`: `height += (uplift − sag_rate)·dt` on every cell that is not fixed;
/// fixed cells keep their height, the base level.
pub struct Uplift;

impl TypedStage for Uplift {
    type Params = UpliftParams;
    const ID: &'static str = "uplift";
    const SUMMARY: &'static str =
        "Raises the height by (uplift − sag_rate)·dt; drained edges stay at the base level.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &UpliftParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("uplift"),
            LayerSpec::f64("sag_rate"),
            LayerSpec::mask("fixed"),
        ]
    }

    fn writes(&self, _: &UpliftParams) -> Vec<LayerSpec> {
        vec![LayerSpec::f64("height")]
    }

    fn run(&self, ctx: &mut StageContext<'_>, _: &UpliftParams) -> Result<StageReport, StageError> {
        let Some(step) = ctx.step() else {
            return Err(ctx.fail("runs only as a process of landscape_evolution"));
        };
        let mut height = ctx.take_f64("height")?;
        let (uplift, sag, fixed) = (ctx.f64("uplift")?, ctx.f64("sag_rate")?, ctx.mask("fixed")?);
        let cells = height
            .as_mut_slice()
            .iter_mut()
            .zip(uplift.as_slice())
            .zip(sag.as_slice())
            .zip(fixed.as_slice());
        for (((h, u), s), &on) in cells {
            if !on {
                *h += (u - s) * step.dt_yr;
            }
        }
        ctx.write("height", Layer::F64(height))?;
        Ok(StageReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::Uplift;
    use crate::grid::Grid;
    use crate::kernels::num::to_f64;
    use crate::stage::{LayerSpec, Stage, StageError};
    use crate::stages::evolution::test_support::{evolve, processes};
    use crate::stages::terrain::support::{run_stage, world};
    use crate::world::{Layer, World};

    /// A 64 x 64 world with a sloping height, varying uplift and sag, and the first column
    /// fixed at 0 although the uplift there is not 0.
    fn uplifting_world() -> World {
        let mut w = world(64, 64);
        let cells = |f: &dyn Fn(usize, usize) -> f64| {
            let mut g = Grid::filled(64, 64, 0.0);
            for r in 0..64 {
                for c in 0..64 {
                    g.set(r, c, f(r, c));
                }
            }
            g
        };
        let height = cells(&|r, c| if c == 0 { 0.0 } else { to_f64(r + c) });
        let uplift = cells(&|r, _| 1.0e-3 + 1.0e-5 * to_f64(r));
        let sag = cells(&|_, c| if c > 40 { 4.0e-4 } else { 0.0 });
        let fixed = Grid::from_vec(64, 64, (0..64 * 64).map(|i| i % 64 == 0).collect()).unwrap();
        w.insert("height", Layer::F64(height)).unwrap();
        w.insert("uplift", Layer::F64(uplift)).unwrap();
        w.insert("sag_rate", Layer::F64(sag)).unwrap();
        w.insert("fixed", Layer::Mask(fixed)).unwrap();
        w
    }

    #[test]
    fn uplift_adds_rate_times_dt_and_keeps_fixed_cells_at_base_level() {
        let mut w = uplifting_world();
        let before = w.f64("height").unwrap().clone();
        let steps = 3;
        let dt = 2.5e4;
        evolve(
            &mut w,
            &processes("[[process]]\nid = \"uplift\""),
            steps,
            dt,
            1,
        )
        .unwrap();
        let after = w.f64("height").unwrap();
        let (uplift, sag) = (w.f64("uplift").unwrap(), w.f64("sag_rate").unwrap());
        for i in 0..64 * 64 {
            let (h0, h) = (before.as_slice()[i], after.as_slice()[i]);
            if i % 64 == 0 {
                assert_eq!(h.to_bits(), 0.0_f64.to_bits(), "fixed cell {i} moved");
                assert!(uplift.as_slice()[i] > 0.0);
            } else {
                let rate = uplift.as_slice()[i] - sag.as_slice()[i];
                let expected = h0 + f64::from(steps) * rate * dt;
                assert!((h - expected).abs() < 1e-9, "cell {i}: {h} vs {expected}");
            }
        }
        let rise = |i: usize| after.as_slice()[i] - before.as_slice()[i];
        assert!(
            (rise(1) - 75.0).abs() < 1e-9 && (rise(63) - 45.0).abs() < 1e-9,
            "row 0 rises 3 x 2.5e4 yr x 1e-3 m/yr = 75 m, and 30 m less where it sags"
        );
    }

    #[test]
    fn uplift_declares_its_layers() {
        let stage: &dyn Stage = &Uplift;
        let params = stage.params(&toml::Table::new()).unwrap();
        assert_eq!(
            stage.reads(params.as_ref()),
            [
                LayerSpec::f64("height"),
                LayerSpec::f64("uplift"),
                LayerSpec::f64("sag_rate"),
                LayerSpec::mask("fixed"),
            ]
        );
        assert_eq!(stage.writes(params.as_ref()), [LayerSpec::f64("height")]);
        assert!(stage.params(&"rate = 1.0".parse().unwrap()).is_err());
    }

    #[test]
    fn uplift_outside_the_evolution_fails() {
        let mut w = uplifting_world();
        let err = run_stage(&mut w, &Uplift, "").unwrap_err();
        assert!(
            matches!(&err, StageError::Failed { stage, message }
                if stage == "uplift" && message.contains("landscape_evolution")),
            "{err:?}"
        );
    }
}
