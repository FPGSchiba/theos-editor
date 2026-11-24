//! The per-step process `climate_update`: recomputes the climate fields from the current surface.

use serde::{Deserialize, Serialize};

use crate::stage::{LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage};
use crate::stages::terrain::climate::{self, Climate};
use crate::world::{Layer, ValueType};

/// The parameters of `climate_update`: none. How often it runs is the evolution's generic
/// `every` option (plan ruling L27).
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClimateUpdateParams {}

/// The process `climate_update`: `climate.fields = fields(height)` with the climate's own
/// parameters, so precipitation, evapotranspiration and runoff follow the rising and eroding
/// surface.
pub struct ClimateUpdate;

impl TypedStage for ClimateUpdate {
    type Params = ClimateUpdateParams;
    const ID: &'static str = "climate_update";
    const SUMMARY: &'static str = "Recomputes the climate fields (precipitation, evapotranspiration, runoff) from the current height.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &ClimateUpdateParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::value("climate", Climate::NAME),
        ]
    }

    fn writes(&self, _: &ClimateUpdateParams) -> Vec<LayerSpec> {
        vec![LayerSpec::value("climate", Climate::NAME)]
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        _: &ClimateUpdateParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let params = ctx.value::<Climate>("climate")?.params.clone();
        let fields = climate::fields(ctx.f64("height")?, &frame, &params);
        let climate = Climate {
            params,
            fields: Some(fields),
        };
        ctx.write("climate", Layer::Value(Box::new(climate)))?;
        Ok(StageReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::ClimateUpdate;
    use crate::grid::Grid;
    use crate::kernels::num::to_f64;
    use crate::stage::{LayerSpec, Stage};
    use crate::stages::evolution::test_support::{evolve, processes};
    use crate::stages::terrain::climate::{self, Climate, ClimateParams};
    use crate::stages::terrain::support::world;
    use crate::world::{Layer, World};

    fn ramp(slope: f64) -> World {
        let mut w = world(32, 32);
        let mut height = Grid::filled(32, 32, 0.0);
        for r in 0..32 {
            for c in 0..32 {
                height.set(r, c, slope * to_f64(c) + 3.0 * to_f64(r % 5));
            }
        }
        w.insert("height", Layer::F64(height)).unwrap();
        let climate = Climate {
            params: ClimateParams {
                t_base_c: 11.0,
                ..ClimateParams::default()
            },
            fields: None,
        };
        w.insert("climate", Layer::Value(Box::new(climate)))
            .unwrap();
        w
    }

    fn update(w: &mut World) {
        let p = processes("[[process]]\nid = \"climate_update\"");
        evolve(w, &p, 1, 2.5e4, 1).unwrap();
    }

    #[test]
    fn climate_update_fills_the_fields_from_the_current_surface() {
        let mut w = ramp(20.0);
        update(&mut w);
        let first = w.value::<Climate>("climate").unwrap().clone();
        let height = w.f64("height").unwrap().clone();
        let want = climate::fields(&height, w.frame(), &first.params);
        assert_eq!(first.fields.as_ref(), Some(&want));
        assert!((first.params.t_base_c - 11.0).abs() < 1e-12, "params kept");

        w.insert("height", Layer::F64(height.map(|h| h * 3.0)))
            .unwrap();
        update(&mut w);
        let second = w.value::<Climate>("climate").unwrap();
        assert_ne!(
            second.fields, first.fields,
            "the new surface changes the fields"
        );
    }

    #[test]
    fn climate_update_declares_its_layers() {
        let stage: &dyn Stage = &ClimateUpdate;
        let params = stage.params(&toml::Table::new()).unwrap();
        let names = |l: Vec<LayerSpec>| l.iter().map(|s| s.name).collect::<Vec<_>>();
        assert_eq!(names(stage.reads(params.as_ref())), ["height", "climate"]);
        assert_eq!(names(stage.writes(params.as_ref())), ["climate"]);
        assert!(stage.params(&"every_n = 3".parse().unwrap()).is_err());
    }
}
