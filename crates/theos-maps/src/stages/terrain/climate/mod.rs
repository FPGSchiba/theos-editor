//! The `climate_model` stage and the climate functions.
//!
//! The stage only fixes the parameters: no height exists when it runs. The functions
//! ([`orographic`], [`temperature`], [`pet`], [`fu_aet`], [`fields`]) are pure and are reused by
//! the stages that have a height (`climate_update`, hydrology, ecology).

mod orographic;
mod water;

use std::any::Any;
use std::collections::BTreeMap;

use serde::Deserialize;

pub use self::orographic::orographic;
pub use self::water::{fu_aet, pet, temperature};
use super::grid::FrameValue;
use crate::grid::Grid;
use crate::kernels::num::max;
use crate::kernels::stats::percentile;
use crate::stage::{
    LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::world::{Layer, MapFrame, ValueType, WorldValue};

/// The seconds in the year 0.3.0 converts with: mm/s to mm/yr in the climate, m³/yr to m³/s in
/// `hydrology`.
pub(crate) const SECONDS_PER_YEAR: f64 = 3.156e7;

/// The climate parameters; defaults are 0.3.0's, `budyko_w` is the builder's choice.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct ClimateParams {
    /// Direction the wind blows from, degrees clockwise from north (270 is west).
    pub wind_from_deg: f64,
    /// Wind speed, m/s.
    pub wind_speed: f64,
    /// Precipitation on flat ground, mm/yr.
    pub base_precip_mm: f64,
    /// The 95th percentile of the orographic gain on wet cells, mm/yr.
    pub orographic_mm: f64,
    /// Lowest precipitation anywhere, mm/yr.
    pub min_precip_mm: f64,
    /// Sea-level temperature, degrees Celsius (0.3.0's `T_base_c`).
    pub t_base_c: f64,
    /// Temperature drop per kilometre of height, degrees Celsius.
    pub lapse_c_per_km: f64,
    /// Moist stability frequency, 1/s.
    pub nm: f64,
    /// Moist layer depth, m.
    pub hw: f64,
    /// Uplift sensitivity, kg/m^3.
    pub cw: f64,
    /// Conversion time of cloud water to hydrometeors, s.
    pub tau_c: f64,
    /// Fallout time of hydrometeors, s.
    pub tau_f: f64,
    /// The shape `w` of Fu's Budyko curve; 2.6 lies within the range Fu (1981) and Zhang et al.
    /// (2004) report (about 2 to 3), and is the builder's choice.
    pub budyko_w: f64,
}

impl Default for ClimateParams {
    fn default() -> Self {
        Self {
            wind_from_deg: 270.0,
            wind_speed: 10.0,
            base_precip_mm: 600.0,
            orographic_mm: 1600.0,
            min_precip_mm: 120.0,
            t_base_c: 13.0,
            lapse_c_per_km: 6.5,
            nm: 0.005,
            hw: 3000.0,
            cw: 0.004,
            tau_c: 1000.0,
            tau_f: 1000.0,
            budyko_w: 2.6,
        }
    }
}

impl ClimateParams {
    /// The parameters as `(name, value)` pairs, in declaration order.
    fn named(&self) -> [(&'static str, f64); 13] {
        [
            ("wind_from_deg", self.wind_from_deg),
            ("wind_speed", self.wind_speed),
            ("base_precip_mm", self.base_precip_mm),
            ("orographic_mm", self.orographic_mm),
            ("min_precip_mm", self.min_precip_mm),
            ("t_base_c", self.t_base_c),
            ("lapse_c_per_km", self.lapse_c_per_km),
            ("nm", self.nm),
            ("hw", self.hw),
            ("cw", self.cw),
            ("tau_c", self.tau_c),
            ("tau_f", self.tau_f),
            ("budyko_w", self.budyko_w),
        ]
    }

    /// Checks that every value is finite and that the ones that must be positive are.
    ///
    /// # Errors
    /// [`ParamError`] of `climate_model` naming the first offending parameter.
    pub fn validate(&self) -> Result<(), ParamError> {
        let at = |name: &str, message: &str| ParamError::at(ClimateModel::ID, name, message);
        for (name, value) in self.named() {
            if !value.is_finite() {
                return Err(at(name, "must be finite"));
            }
        }
        let non_negative = [
            ("wind_speed", self.wind_speed),
            ("base_precip_mm", self.base_precip_mm),
            ("orographic_mm", self.orographic_mm),
            ("min_precip_mm", self.min_precip_mm),
            ("nm", self.nm),
            ("hw", self.hw),
            ("cw", self.cw),
            ("tau_c", self.tau_c),
            ("tau_f", self.tau_f),
        ];
        if let Some((name, _)) = non_negative.iter().find(|(_, v)| *v < 0.0) {
            return Err(at(name, "must not be negative"));
        }
        if self.budyko_w <= 0.0 {
            return Err(at("budyko_w", "must be positive"));
        }
        Ok(())
    }
}

/// The yearly water fields of a height, all in mm/yr.
#[derive(Debug, Clone, PartialEq)]
pub struct ClimateFields {
    /// Precipitation.
    pub precipitation: Grid<f64>,
    /// Potential evapotranspiration.
    pub pet: Grid<f64>,
    /// Actual evapotranspiration.
    pub aet: Grid<f64>,
    /// Runoff, precipitation minus actual evapotranspiration.
    pub runoff: Grid<f64>,
}

/// The world value `climate`: the parameters, and the fields once a height has been through
/// them.
#[derive(Debug, Clone, PartialEq)]
pub struct Climate {
    /// The parameters.
    pub params: ClimateParams,
    /// The fields of the current height; `None` until `climate_update` fills them.
    pub fields: Option<ClimateFields>,
}

impl WorldValue for Climate {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        let params: BTreeMap<&str, f64> = self.params.named().into_iter().collect();
        let fields = self.fields.as_ref().map(|f| {
            BTreeMap::from([
                ("precipitation", f.precipitation.as_slice()),
                ("pet", f.pet.as_slice()),
                ("aet", f.aet.as_slice()),
                ("runoff", f.runoff.as_slice()),
            ])
        });
        Ok(serde_json::json!({
            "params": theos_map_format::json::to_checked_value(&params)?,
            "fields": theos_map_format::json::to_checked_value(&fields)?,
        }))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for Climate {
    const NAME: &'static str = "climate";
}

/// The precipitation of a height in mm/yr: the base plus the orographic part scaled so that its
/// 95th percentile on wet cells is `orographic_mm` (scale 1 when no cell is wet), not below the
/// minimum.
fn precipitation(height: &Grid<f64>, frame: &MapFrame, p: &ClimateParams) -> Grid<f64> {
    let oro = orographic(height, frame, p);
    let wet: Vec<f64> = oro
        .as_slice()
        .iter()
        .copied()
        .filter(|v| *v > 0.0)
        .collect();
    let scale = percentile(&wet, 95.0).map_or(1.0, |p95| p.orographic_mm / p95);
    oro.map(|o| max(p.base_precip_mm + scale * o, p.min_precip_mm))
}

/// Precipitation, PET, AET and runoff of `height` (metres), all mm/yr. Pure: it reads nothing
/// but its arguments.
#[must_use]
pub fn fields(height: &Grid<f64>, frame: &MapFrame, p: &ClimateParams) -> ClimateFields {
    let precipitation = precipitation(height, frame, p);
    let pet_grid = height.map(|h| pet(temperature(h, p)));
    let aet: Vec<f64> = precipitation
        .as_slice()
        .iter()
        .zip(pet_grid.as_slice())
        .map(|(&rain, &demand)| fu_aet(rain, demand, p.budyko_w))
        .collect();
    let aet = Grid::from_vec(height.rows(), height.cols(), aet)
        .unwrap_or_else(|_| Grid::filled(height.rows(), height.cols(), 0.0));
    let runoff = Grid::from_vec(
        height.rows(),
        height.cols(),
        precipitation
            .as_slice()
            .iter()
            .zip(aet.as_slice())
            .map(|(&rain, &used)| max(rain - used, 0.0))
            .collect(),
    )
    .unwrap_or_else(|_| Grid::filled(height.rows(), height.cols(), 0.0));
    ClimateFields {
        precipitation,
        pet: pet_grid,
        aet,
        runoff,
    }
}

/// The stage `climate_model`.
pub struct ClimateModel;

impl TypedStage for ClimateModel {
    type Params = ClimateParams;
    const ID: &'static str = "climate_model";
    const SUMMARY: &'static str = "Climate parameters: wind, orographic precipitation, lapse-rate temperature and the Budyko water balance.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &ClimateParams) -> Vec<LayerSpec> {
        vec![LayerSpec::value("frame", FrameValue::NAME)]
    }

    fn writes(&self, _: &ClimateParams) -> Vec<LayerSpec> {
        vec![LayerSpec::value("climate", Climate::NAME)]
    }

    fn validate(&self, params: &mut ClimateParams) -> Result<(), ParamError> {
        params.validate()
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &ClimateParams,
    ) -> Result<StageReport, StageError> {
        ctx.value::<FrameValue>("frame")?;
        let climate = Climate {
            params: params.clone(),
            fields: None,
        };
        ctx.write("climate", Layer::Value(Box::new(climate)))?;
        Ok(StageReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::world::MapFrame;

    /// 1 km cells: at 100 m the 1000 s fallout times carry the rain beyond a hill (0.3.0 too).
    fn frame(n: usize) -> MapFrame {
        MapFrame {
            rows: n,
            cols: n,
            cell_m: 1000.0,
        }
    }

    #[test]
    fn a_flat_surface_gets_only_base_precipitation() {
        let h = Grid::filled(32, 32, 100.0);
        let f = fields(&h, &frame(32), &ClimateParams::default());
        assert!(
            f.precipitation
                .as_slice()
                .iter()
                .all(|v| (v - 600.0).abs() < 1e-3)
        );
    }

    #[test]
    fn precipitation_respects_the_minimum() {
        let h = orographic::tests::hill(64, 6.0, 1500.0);
        let p = ClimateParams {
            min_precip_mm: 450.0,
            ..ClimateParams::default()
        };
        let f = fields(&h, &frame(64), &p);
        assert!(f.precipitation.as_slice().iter().all(|v| *v >= 450.0));
        assert!(
            f.precipitation
                .as_slice()
                .iter()
                .any(|v| (*v - 450.0).abs() < 1e-9)
        );
    }

    #[test]
    fn runoff_is_precipitation_minus_aet_and_never_negative() {
        let hill = orographic::tests::hill(64, 6.0, 1500.0);
        let out = fields(&hill, &frame(64), &ClimateParams::default());
        for i in 0..64 * 64 {
            let rain = out.precipitation.as_slice()[i];
            let aet = out.aet.as_slice()[i];
            let runoff = out.runoff.as_slice()[i];
            assert!(runoff >= 0.0 && aet >= 0.0 && aet <= rain + 1e-9);
            assert!((runoff - (rain - aet)).abs() < 1e-9);
            assert!((200.0..=1800.0).contains(&out.pet.as_slice()[i]));
        }
    }

    #[test]
    fn the_windward_slope_is_wetter_than_the_lee() {
        let h = orographic::tests::hill(64, 6.0, 1500.0);
        let f = fields(&h, &frame(64), &ClimateParams::default());
        assert!(f.precipitation.get(32, 24) > f.precipitation.get(32, 42));
    }

    use crate::stage::Stage;
    use crate::stages::terrain::grid::GridStage;
    use crate::stages::terrain::support::{run_stage, table, world};

    #[test]
    fn the_stage_writes_parameters_without_fields() {
        let mut w = world(16, 16);
        run_stage(&mut w, &GridStage, "").unwrap();
        run_stage(
            &mut w,
            &ClimateModel,
            "wind_from_deg = 90.0
t_base_c = 15.0",
        )
        .unwrap();
        let c = w.value::<Climate>("climate").unwrap();
        assert!(c.fields.is_none());
        assert!((c.params.wind_from_deg - 90.0).abs() < 1e-12);
        assert!((c.params.t_base_c - 15.0).abs() < 1e-12);
        assert!((c.params.budyko_w - 2.6).abs() < 1e-12);
        assert!((c.params.min_precip_mm - 120.0).abs() < 1e-12);
    }

    #[test]
    fn the_stage_needs_the_frame_first() {
        let mut w = world(8, 8);
        assert!(run_stage(&mut w, &ClimateModel, "").is_err());
    }

    #[test]
    fn bad_values_and_old_keys_are_refused() {
        for (toml, parameter, text) in [
            ("nm = nan", "nm", "finite"),
            ("wind_speed = -1.0", "wind_speed", "not be negative"),
            ("budyko_w = 0.0", "budyko_w", "positive"),
            ("hw = inf", "hw", "finite"),
            ("T_base_c = 10.0", "T_base_c", "unknown"),
        ] {
            let err = ClimateModel.params(&table(toml)).unwrap_err();
            assert_eq!(err.parameter.as_deref(), Some(parameter), "{toml}: {err}");
            assert!(err.to_string().contains(text), "{toml}: {err}");
        }
    }

    #[test]
    fn declarations_name_their_layers() {
        let params = ClimateModel.params(&table("")).unwrap();
        let reads: Vec<_> = Stage::reads(&ClimateModel, params.as_ref())
            .iter()
            .map(|l| l.name)
            .collect();
        let writes: Vec<_> = Stage::writes(&ClimateModel, params.as_ref())
            .iter()
            .map(|l| l.name)
            .collect();
        assert_eq!((reads, writes), (vec!["frame"], vec!["climate"]));
    }

    #[test]
    fn the_canonical_json_covers_the_fields() {
        let h = orographic::tests::hill(16, 3.0, 500.0);
        let params = ClimateParams::default();
        let bare = Climate {
            params: params.clone(),
            fields: None,
        };
        let full = Climate {
            fields: Some(fields(&h, &frame(16), &params)),
            params,
        };
        let (a, b) = (
            bare.canonical_json().unwrap(),
            full.canonical_json().unwrap(),
        );
        assert!(a["fields"].is_null());
        assert_eq!(b["fields"]["runoff"].as_array().map(Vec::len), Some(256));
        assert!((b["params"]["budyko_w"].as_f64().unwrap() - 2.6).abs() < 1e-12);
        assert_eq!(bare.type_name(), "climate");
        let mut moved = full.clone();
        assert!(moved.as_any().is::<Climate>() && moved.as_any_mut().is::<Climate>());
    }

    #[test]
    fn fields_are_repeatable() {
        let h = orographic::tests::hill(32, 5.0, 900.0);
        let a = fields(&h, &frame(32), &ClimateParams::default());
        let b = fields(&h, &frame(32), &ClimateParams::default());
        assert_eq!(a, b);
    }
}
