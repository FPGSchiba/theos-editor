//! The `ecology` stage (spec §5.9 "Ecology"): temperature, slope, vegetation density, snow cover
//! and the wetness of hollows.
//!
//! * **Temperature** is the lapse-rate temperature of `climate::temperature`, **slope** the
//!   gradient magnitude of `kernels::diff::slope` (a tangent).
//! * **Vegetation** follows the net primary production of the Thornthwaite Memorial model
//!   (Lieth 1975), `NPP = 3000 (1 − e^(−0.0009695 (AET − 20)))` in g/m²/yr, scaled by 2000 and
//!   capped at 1, thinned on steep ground by `1 − smoothstep(0.45, 0.9, slope)`, and raised in
//!   hollows by the wetness bonus.
//! * **Wetness** is the topographic wetness index of Beven and Kirkby (1979),
//!   `TWI = ln(a / tan β)` with `a = drainage_area / cell_m` (`tan β` not below 1e-3), mapped to
//!   `[0, 1]` by `clamp((TWI − twi_ref) / twi_span, 0, 1)`.
//! * **Snow** is 0.3.0's `smoothstep(−1.5, −5, T) (1 − smoothstep(0.6, 1.1, slope))`.
//!
//! The AET comes from `climate.fields` when present, else from the fields computed here from
//! `height` (and not written), as in `hydrology`.

use serde::{Deserialize, Serialize};

use crate::grid::Grid;
use crate::kernels::diff::slope;
use crate::kernels::num::{max, min, smoothstep};
use crate::stage::{
    Check, LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::terrain::climate::{self, Climate};
use crate::world::{Layer, ValueType};

/// The shallowest slope tangent the wetness index divides by.
const MIN_TAN_BETA: f64 = 1e-3;

/// NPP at which the density saturates, g/m²/yr.
const NPP_FULL: f64 = 2000.0;

/// The parameters of `ecology`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct EcologyParams {
    /// Largest relative gain of vegetation density in the wettest hollows (0.25 is a quarter).
    pub wet_bonus: f64,
    /// The wetness index below which a cell gets no bonus.
    pub twi_ref: f64,
    /// The span of the wetness index from no bonus to the full bonus.
    pub twi_span: f64,
}

impl Default for EcologyParams {
    fn default() -> Self {
        Self {
            wet_bonus: 0.25,
            twi_ref: 6.0,
            twi_span: 6.0,
        }
    }
}

/// Net primary production in g/m²/yr for an actual evapotranspiration of `aet_mm` mm/yr
/// (Thornthwaite Memorial model), never below 0.
#[must_use]
pub fn npp(aet_mm: f64) -> f64 {
    max(
        3000.0 * (1.0 - libm::exp(-0.000_969_5 * (aet_mm - 20.0))),
        0.0,
    )
}

/// The wetness of a cell in `[0, 1]`: the topographic wetness index of a cell draining
/// `area_m2` with slope tangent `tan_beta`, on cells of `cell_m` metres.
#[must_use]
pub fn wetness(area_m2: f64, tan_beta: f64, cell_m: f64, p: &EcologyParams) -> f64 {
    // A cell drains at least itself: `a` is not below one cell width, so the logarithm exists.
    let a = max(area_m2 / cell_m, cell_m);
    let twi = libm::log(a / max(tan_beta, MIN_TAN_BETA));
    min(max((twi - p.twi_ref) / p.twi_span, 0.0), 1.0)
}

/// Vegetation density in `[0, 1]` from the AET (mm/yr), the slope tangent and the wetness.
#[must_use]
pub fn vegetation(aet_mm: f64, tan_beta: f64, wetness: f64, p: &EcologyParams) -> f64 {
    let base = min(npp(aet_mm) / NPP_FULL, 1.0) * (1.0 - smoothstep(0.45, 0.9, tan_beta));
    min(base * (1.0 + p.wet_bonus * wetness), 1.0)
}

/// Snow cover in `[0, 1]` from the temperature (°C) and the slope tangent.
#[must_use]
pub fn snow(t_c: f64, tan_beta: f64) -> f64 {
    smoothstep(-1.5, -5.0, t_c) * (1.0 - smoothstep(0.6, 1.1, tan_beta))
}

/// The stage `ecology`.
pub struct Ecology;

impl TypedStage for Ecology {
    type Params = EcologyParams;
    const ID: &'static str = "ecology";
    const SUMMARY: &'static str = "Temperature, slope, vegetation from the water balance and the wetness of hollows, and snow cover.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &EcologyParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::value("climate", Climate::NAME),
            LayerSpec::f64("drainage_area"),
        ]
    }

    fn writes(&self, _: &EcologyParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("temperature"),
            LayerSpec::f64("slope"),
            LayerSpec::f64("vegetation"),
            LayerSpec::f64("snow"),
            LayerSpec::f64("wetness"),
        ]
    }

    fn validate(&self, params: &mut EcologyParams) -> Result<(), ParamError> {
        let check = Check::of(Self::ID);
        check.at_least("wet_bonus", params.wet_bonus, 0.0)?;
        check.finite("twi_ref", params.twi_ref)?;
        check.positive("twi_span", params.twi_span)
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &EcologyParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let climate = ctx.value::<Climate>("climate")?;
        let height = ctx.f64("height")?;
        let area = ctx.f64("drainage_area")?;
        let local;
        let fields = if let Some(fields) = &climate.fields {
            fields
        } else {
            local = climate::fields(height, &frame, &climate.params);
            &local
        };
        let temperature = height.map(|h| climate::temperature(h, &climate.params));
        let tan_beta = slope(height, frame.cell_m);
        let (rows, cols) = (height.rows(), height.cols());
        let mut wet = Grid::filled(rows, cols, 0.0);
        let mut veg = Grid::filled(rows, cols, 0.0);
        let mut snowy = Grid::filled(rows, cols, 0.0);
        for i in 0..height.len() {
            let tb = tan_beta.as_slice()[i];
            let w = wetness(area.as_slice()[i], tb, frame.cell_m, params);
            wet.as_mut_slice()[i] = w;
            veg.as_mut_slice()[i] = vegetation(fields.aet.as_slice()[i], tb, w, params);
            snowy.as_mut_slice()[i] = snow(temperature.as_slice()[i], tb);
        }
        ctx.write("temperature", Layer::F64(temperature))?;
        ctx.write("slope", Layer::F64(tan_beta))?;
        ctx.write("vegetation", Layer::F64(veg))?;
        ctx.write("snow", Layer::F64(snowy))?;
        ctx.write("wetness", Layer::F64(wet))?;
        Ok(StageReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::num::to_f64;
    use crate::stage::Stage;
    use crate::stages::terrain::climate::{ClimateFields, ClimateParams};
    use crate::stages::terrain::support::{run_stage, table, world};
    use crate::world::World;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * (1.0 + b.abs())
    }

    fn p() -> EcologyParams {
        EcologyParams::default()
    }

    #[test]
    fn npp_follows_the_thornthwaite_memorial_formula() {
        let want = 3000.0 * (1.0 - libm::exp(-0.46536));
        assert!(near(npp(500.0), want));
        assert_eq!(npp(10.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(npp(20.0).to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn steep_slopes_thin_vegetation() {
        let flat = vegetation(800.0, 0.0, 0.0, &p());
        let mid = vegetation(800.0, 0.675, 0.0, &p());
        let steep = vegetation(800.0, 1.0, 0.0, &p());
        assert!(flat > mid && mid > steep);
        assert!(near(mid, flat * 0.5));
        assert_eq!(steep.to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn wet_hollows_get_the_bonus_capped_at_one() {
        // A flat cell of a big catchment: TWI = ln(1e8 / 100 / 1e-3) = 13.8, beyond the span.
        assert!(near(wetness(1e8, 0.0, 100.0, &p()), 1.0));
        // A hillslope cell draining itself: TWI = ln(100 / 0.5) = 5.3, below the reference.
        assert_eq!(wetness(1e4, 0.5, 100.0, &p()).to_bits(), 0.0_f64.to_bits());
        // In between the bonus scales linearly with the index: a = 1000 m, tan = 0.1.
        let twi = libm::log(1000.0 / 0.1);
        assert!(near(wetness(1e5, 0.1, 100.0, &p()), (twi - 6.0) / 6.0));
        // A moderately productive cell gains the bonus; a saturated one stays at 1.
        let dry = vegetation(500.0, 0.0, 0.0, &p());
        let wet = vegetation(500.0, 0.0, 1.0, &p());
        assert!(near(wet, dry * 1.25) && wet <= 1.0);
        assert!(near(vegetation(3000.0, 0.0, 1.0, &p()), 1.0));
        // A drainage area of zero does not make the index undefined.
        assert!(wetness(0.0, 0.0, 100.0, &p()).is_finite());
    }

    #[test]
    fn snow_needs_cold_and_gentle_ground() {
        assert_eq!(snow(5.0, 0.0).to_bits(), 0.0_f64.to_bits());
        assert!(near(snow(-10.0, 0.0), 1.0));
        assert_eq!(snow(-10.0, 1.2).to_bits(), 0.0_f64.to_bits());
        let part = snow(-3.25, 0.0);
        assert!(part > 0.4 && part < 0.6);
        assert!(snow(-10.0, 0.85) < snow(-10.0, 0.3));
    }

    #[test]
    fn parameters_are_checked_and_unknown_keys_refused() {
        for (toml, parameter) in [
            ("wet_bonus = -0.1", "wet_bonus"),
            ("wet_bonus = nan", "wet_bonus"),
            ("twi_ref = inf", "twi_ref"),
            ("twi_span = 0.0", "twi_span"),
        ] {
            let err = Ecology.params(&table(toml)).unwrap_err();
            assert_eq!(err.stage, "ecology");
            assert_eq!(err.parameter.as_deref(), Some(parameter), "{toml}");
        }
        let err = Ecology.params(&table("twi_spam = 1.0")).unwrap_err();
        assert!(err.to_string().contains("twi_spam"), "{err}");
        assert!(Ecology.params(&table("")).is_ok());
    }

    const N: usize = 16;

    fn ramp_world(fields: Option<ClimateFields>) -> World {
        let mut w = world(N, N);
        let data = (0..N * N).map(|i| 3000.0 - 15.0 * to_f64(i % N)).collect();
        w.insert("height", Layer::F64(Grid::from_vec(N, N, data).unwrap()))
            .unwrap();
        w.insert("drainage_area", Layer::F64(Grid::filled(N, N, 1e5)))
            .unwrap();
        let climate = Climate {
            params: ClimateParams::default(),
            fields,
        };
        w.insert("climate", Layer::Value(Box::new(climate)))
            .unwrap();
        w
    }

    #[test]
    fn the_stage_writes_its_rasters_in_range_and_uses_the_given_fields() {
        let mut w = ramp_world(None);
        run_stage(&mut w, &Ecology, "").unwrap();
        assert!(w.value::<Climate>("climate").unwrap().fields.is_none());
        for name in ["vegetation", "snow", "wetness"] {
            let g = w.f64(name).unwrap();
            assert!(
                g.as_slice().iter().all(|v| (0.0..=1.0).contains(v)),
                "{name}"
            );
        }
        // The ramp is cold at the high side: snow lies there.
        assert!(w.f64("snow").unwrap().as_slice().iter().any(|v| *v > 0.0));
        // Slope is the tangent of the 15 m per 100 m ramp.
        assert!(near(w.f64("slope").unwrap().get(8, 8), 0.15));
        let t = w.f64("temperature").unwrap().get(0, 0);
        assert!(near(t, 13.0 - 6.5 * 3.0));

        // Fields given in the climate give the same rasters as the local fallback computes.
        let height = w.f64("height").unwrap().clone();
        let fields = climate::fields(&height, w.frame(), &ClimateParams::default());
        let mut given = ramp_world(Some(fields));
        run_stage(&mut given, &Ecology, "").unwrap();
        assert_eq!(w.f64("vegetation"), given.f64("vegetation"));
        // Given fields with no water give no vegetation, so the stage does read them.
        let none = || Grid::filled(N, N, 0.0);
        let dry = ClimateFields {
            precipitation: none(),
            pet: none(),
            aet: none(),
            runoff: none(),
        };
        let mut barren = ramp_world(Some(dry));
        run_stage(&mut barren, &Ecology, "").unwrap();
        assert!(
            barren
                .f64("vegetation")
                .unwrap()
                .as_slice()
                .iter()
                .all(|v| v.to_bits() == 0.0_f64.to_bits())
        );
    }
}
