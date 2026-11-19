//! The `tectonics` stage: the uplift field from parametric sources and a subsidence rate.

mod sources;

use std::any::Any;

use serde::Deserialize;

use self::sources::{Env, Source};
use super::grid::FrameValue;
use crate::grid::Grid;
use crate::kernels::noise::{spectral_noise, warped_coords};
use crate::kernels::num::max;
use crate::stage::{
    Check, LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::world::{Layer, ValueType, WorldValue};

/// The world value `belts`: the signed distance (map fractions) to the first orogen's axis.
#[derive(Debug, Clone, PartialEq)]
pub struct Belts {
    /// The signed distance grid, or `None` when the recipe has no orogen.
    pub first_orogen: Option<Grid<f64>>,
}

impl WorldValue for Belts {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        let first = self.first_orogen.as_ref().map(Grid::as_slice);
        theos_map_format::json::to_checked_value(&std::collections::BTreeMap::from([(
            "first_orogen",
            first,
        )]))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for Belts {
    const NAME: &'static str = "belts";
}

/// The world value `warp`: the domain-warped coordinates (fractions) of every cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Warp {
    /// Warped x, with the columns.
    pub x: Grid<f64>,
    /// Warped y, with the rows.
    pub y: Grid<f64>,
}

impl WorldValue for Warp {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        theos_map_format::json::to_checked_value(&std::collections::BTreeMap::from([
            ("x", self.x.as_slice()),
            ("y", self.y.as_slice()),
        ]))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for Warp {
    const NAME: &'static str = "warp";
}

/// The parameters of `tectonics`; defaults as 0.3.0 (the sag duration is plan ruling L13). The
/// background and warp numbers must be finite and `sag_duration_yr` positive and finite; the
/// sources have the domains of [`Source`].
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct TectonicsParams {
    /// Uplift everywhere, m/yr.
    pub background_uplift: f64,
    /// Relative strength of the noise on the background uplift.
    pub background_variation: f64,
    /// Spectral slope of the background noise.
    pub background_beta: f64,
    /// Strength of the domain warp, map fractions.
    pub warp: f64,
    /// Spectral slope of the warp noise.
    pub warp_beta: f64,
    /// Years a `depth_m` of sag is spread over: `sag_rate = depth_m / sag_duration_yr`.
    pub sag_duration_yr: f64,
    /// The sources, applied in order.
    pub source: Vec<Source>,
}

impl Default for TectonicsParams {
    fn default() -> Self {
        Self {
            background_uplift: 1.5e-4,
            background_variation: 0.4,
            background_beta: 3.2,
            warp: 0.035,
            warp_beta: 3.6,
            sag_duration_yr: 6.0e6,
            source: Vec::new(),
        }
    }
}

/// The stage `tectonics`.
pub struct Tectonics;

impl TypedStage for Tectonics {
    type Params = TectonicsParams;
    const ID: &'static str = "tectonics";
    const SUMMARY: &'static str = "Rock uplift (m/yr) and sag rate (m/yr) from orogens, faults, rifts, domes, volcanoes and basins on domain-warped coordinates.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &TectonicsParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::value("frame", FrameValue::NAME),
            LayerSpec::mask("fixed"),
        ]
    }

    fn writes(&self, _: &TectonicsParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("uplift"),
            LayerSpec::f64("sag_rate"),
            LayerSpec::value("belts", Belts::NAME),
            LayerSpec::value("warp", Warp::NAME),
        ]
    }

    fn validate(&self, params: &mut TectonicsParams) -> Result<(), ParamError> {
        let check = Check::of(Self::ID);
        for (name, value) in [
            ("background_uplift", params.background_uplift),
            ("background_variation", params.background_variation),
            ("background_beta", params.background_beta),
            ("warp", params.warp),
            ("warp_beta", params.warp_beta),
        ] {
            check.finite(name, value)?;
        }
        check.positive("sag_duration_yr", params.sag_duration_yr)?;
        for (k, source) in params.source.iter().enumerate() {
            source.check(&format!("source[{k}]"), check)?;
        }
        Ok(())
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &TectonicsParams,
    ) -> Result<StageReport, StageError> {
        ctx.value::<FrameValue>("frame")?;
        let (rows, cols) = (ctx.frame().rows, ctx.frame().cols);
        // 0.3.0 seeds the warp's y noise with `tec-warp` and x with that seed + 7919.
        let (mut warp_y, mut warp_x) = (ctx.sub_rng("tec-warp"), ctx.sub_rng("tec-warp-x"));
        let (xx, yy) = warped_coords(
            rows,
            cols,
            &mut warp_x,
            &mut warp_y,
            params.warp,
            params.warp_beta,
        );
        let mut bg_rng = ctx.sub_rng("tec-bg");
        let noise = spectral_noise(rows, cols, &mut bg_rng, params.background_beta, 0.0, None);
        let mut uplift = noise
            .map(|n| params.background_uplift * max(1.0 + params.background_variation * n, 0.05));
        let mut sag_rate = Grid::filled(rows, cols, 0.0);
        let mut first_orogen = None;
        for (k, source) in params.source.iter().enumerate() {
            ctx.check_cancel()?;
            let (along, wid) = (
                ctx.sub_rng(&format!("along{k}")),
                ctx.sub_rng(&format!("wid{k}")),
            );
            let mut env = Env {
                xx: &xx,
                yy: &yy,
                uplift: &mut uplift,
                sag_rate: &mut sag_rate,
                sag_duration_yr: params.sag_duration_yr,
                rng: ctx.rng(),
                along,
                wid,
            };
            let belt = source
                .apply(&mut env)
                .map_err(|e| ctx.fail(format!("source[{k}]: {e}")))?;
            if first_orogen.is_none() {
                first_orogen = belt;
            }
        }
        let fixed = ctx.mask("fixed")?.clone();
        for ((u, s), &on) in uplift
            .as_mut_slice()
            .iter_mut()
            .zip(sag_rate.as_mut_slice())
            .zip(fixed.as_slice())
        {
            *u = if on { 0.0 } else { max(*u, 0.0) };
            if on {
                *s = 0.0;
            }
        }
        ctx.write("uplift", Layer::F64(uplift))?;
        ctx.write("sag_rate", Layer::F64(sag_rate))?;
        ctx.write("belts", Layer::Value(Box::new(Belts { first_orogen })))?;
        ctx.write("warp", Layer::Value(Box::new(Warp { x: xx, y: yy })))?;
        Ok(StageReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::Stage;
    use crate::stages::terrain::grid::GridStage;
    use crate::stages::terrain::support::{run_stage, table, world};
    use crate::world::World;

    /// A 64 x 64 world after `grid` (default edges) and `tectonics` with `toml`.
    fn tectonic_world(toml: &str) -> World {
        let mut w = world(64, 64);
        run_stage(&mut w, &GridStage, "").unwrap();
        run_stage(&mut w, &Tectonics, toml).unwrap();
        w
    }

    const FLAT: &str = "background_variation = 0.0\nwarp = 0.0\n";

    fn argmax(g: &Grid<f64>) -> (usize, usize) {
        let mut best = 0;
        for (i, v) in g.as_slice().iter().enumerate() {
            if *v > g.as_slice()[best] {
                best = i;
            }
        }
        g.cell(best)
    }

    #[test]
    fn a_source_at_a_known_fraction_appears_at_the_expected_cell() {
        let toml = format!(
            "{FLAT}\n[[source]]\ntype = \"dome\"\nat = [0.25, 0.75]\nr = 0.05\nrate = 1.0e-3\njitter = 0.0\n"
        );
        let w = tectonic_world(&toml);
        let (row, col) = argmax(w.f64("uplift").unwrap());
        assert!(row.abs_diff(48) <= 1, "row {row}");
        assert!(col.abs_diff(16) <= 1, "col {col}");
    }

    #[test]
    fn basins_become_a_sag_rate() {
        let toml = format!(
            "{FLAT}\n[[source]]\ntype = \"basin\"\nat = [0.5, 0.5]\nr = 0.2\ndepth_m = 60.0\njitter = 0.0\n"
        );
        let w = tectonic_world(&toml);
        let (sag, uplift, fixed) = (
            w.f64("sag_rate").unwrap(),
            w.f64("uplift").unwrap(),
            w.mask("fixed").unwrap(),
        );
        // 60 m over the default 6.0e6 years.
        let full = 1.0e-5;
        let peak = sag.as_slice().iter().fold(0.0, |a, &v| max(a, v));
        assert!((peak - full).abs() < 1.0e-7, "{peak}");
        // The bowl `b = exp(-d²/2)`, d the distance in radii, sags at `b·full` and damps the
        // flat background uplift (1.5e-4 m/yr, no variation) to `1 − 0.7·b` of it, so on every
        // cell that is not fixed the uplift follows from the sag rate; fixed cells hold 0.
        let background = 1.5e-4;
        for (i, (&s, &u)) in sag.as_slice().iter().zip(uplift.as_slice()).enumerate() {
            if fixed.as_slice()[i] {
                assert_eq!((s.to_bits(), u.to_bits()), (0, 0), "cell {i}");
            } else {
                let expected = background * (1.0 - 0.7 * s / full);
                assert!(
                    (u - expected).abs() < 1.0e-15,
                    "cell {i}: {u} vs {expected}"
                );
            }
        }
        // The centre cell lies within a few hundredths of a radius of the bowl's centre.
        assert!(sag.get(32, 32) > 0.99 * full, "{}", sag.get(32, 32));
    }

    #[test]
    fn a_longer_sag_duration_lowers_the_rate() {
        let toml = format!(
            "{FLAT}sag_duration_yr = 1.2e7\n[[source]]\ntype = \"rift\"\npath = [[0.1, 0.5], [0.9, 0.5]]\nrate = 1.0e-4\ndepth_m = 120.0\njitter = 0.0\n"
        );
        let w = tectonic_world(&toml);
        let peak = w
            .f64("sag_rate")
            .unwrap()
            .as_slice()
            .iter()
            .fold(0.0, |a, &v| max(a, v));
        assert!((peak - 1.0e-5).abs() < 1.0e-6, "{peak}");
    }

    #[test]
    fn uplift_is_never_negative_and_zero_on_fixed_cells() {
        let toml = "\n[[source]]\ntype = \"orogen\"\npath = [[0.1, 0.3], [0.5, 0.2], [0.9, 0.4]]\nwidth = 0.08\nrate = 1.0e-3\n\
            [[source]]\ntype = \"fault\"\npath = [[0.2, 0.8], [0.8, 0.7]]\nrate = 5.0e-4\n\
            [[source]]\ntype = \"rift\"\npath = [[0.5, 0.1], [0.5, 0.9]]\nrate = 3.0e-4\n\
            [[source]]\ntype = \"volcano\"\nat = [0.7, 0.5]\nr = 0.1\nrate = 6.0e-4\n\
            [[source]]\ntype = \"basin\"\nat = [0.3, 0.6]\nr = 0.12\n";
        let w = tectonic_world(toml);
        let uplift = w.f64("uplift").unwrap();
        let sag = w.f64("sag_rate").unwrap();
        let fixed = w.mask("fixed").unwrap();
        assert!(uplift.as_slice().iter().all(|v| v.is_finite() && *v >= 0.0));
        assert!(sag.as_slice().iter().all(|v| v.is_finite() && *v >= 0.0));
        assert!(uplift.as_slice().iter().any(|v| *v > 0.0));
        for c in 0..64 {
            assert!(fixed.get(63, c));
            assert_eq!(uplift.get(63, c).to_bits(), 0);
            assert_eq!(sag.get(63, c).to_bits(), 0);
        }
        let belts = w.value::<Belts>("belts").unwrap();
        assert!(belts.first_orogen.is_some());
        let warp = w.value::<Warp>("warp").unwrap();
        assert_eq!((warp.x.rows(), warp.y.cols()), (64, 64));
        assert_eq!(belts.type_name(), "belts");
        assert!(belts.canonical_json().unwrap()["first_orogen"].is_array());
        assert!(warp.canonical_json().unwrap()["x"].is_array());
        assert_eq!(warp.type_name(), "warp");
    }

    #[test]
    fn a_run_without_an_orogen_has_no_belt_and_is_repeatable() {
        let a = tectonic_world("");
        let b = tectonic_world("");
        assert!(a.value::<Belts>("belts").unwrap().first_orogen.is_none());
        assert!(
            a.value::<Belts>("belts").unwrap().canonical_json().unwrap()["first_orogen"].is_null()
        );
        assert_eq!(a.f64("uplift"), b.f64("uplift"));
        let u = a.f64("uplift").unwrap();
        assert!(u.get(0, 0) > 0.0 && u.get(0, 0) < 1.0e-3);
    }

    #[test]
    fn asymmetry_makes_the_positive_side_steeper() {
        let toml = format!(
            "{FLAT}\n[[source]]\ntype = \"orogen\"\npath = [[0.1, 0.5], [0.9, 0.5]]\nwidth = 0.1\nrate = 1.0e-3\nasymmetry = 0.9\njitter = 0.0\n"
        );
        let leaning = tectonic_world(&toml);
        let level = tectonic_world(&toml.replace("asymmetry = 0.9", "asymmetry = 0.0"));
        let belt = leaning
            .value::<Belts>("belts")
            .unwrap()
            .first_orogen
            .as_ref()
            .unwrap();
        // Rows 20 and 44 sit at about the same distance on either side of the axis (row 32).
        let (a, b) = ((20, 32), (44, 32));
        assert!((belt.get(a.0, a.1) + belt.get(b.0, b.1)).abs() < 0.02);
        assert!(belt.get(a.0, a.1).abs() > 0.1);
        let (positive, negative) = if belt.get(a.0, a.1) > 0.0 {
            (a, b)
        } else {
            (b, a)
        };
        // The belt's share of the uplift: the flat background of 1.5e-4 m/yr taken off.
        let belt_uplift =
            |w: &World, (r, c): (usize, usize)| w.f64("uplift").unwrap().get(r, c) - 1.5e-4;
        // Both runs draw the same noise from the same streams, so only the lean differs: the
        // positive side narrows (less uplift at the same distance: the steeper flank) and the
        // negative side widens.
        let pos = (
            belt_uplift(&leaning, positive),
            belt_uplift(&level, positive),
        );
        let neg = (
            belt_uplift(&leaning, negative),
            belt_uplift(&level, negative),
        );
        assert!(pos.0 < 0.5 * pos.1, "positive side {pos:?}");
        assert!(neg.0 > 1.5 * neg.1, "negative side {neg:?}");
        assert!(pos.0 < 0.1 * neg.0, "leaning {pos:?} {neg:?}");
    }

    #[test]
    fn unknown_source_types_are_parameter_errors() {
        let err = Tectonics
            .params(&table("[[source]]\ntype = \"glacier\"\nrate = 1.0\n"))
            .unwrap_err();
        assert!(err.to_string().contains("glacier"), "{err}");
        let err = Tectonics
            .params(&table("[[source]]\nrate = 1.0\n"))
            .unwrap_err();
        assert!(err.to_string().contains("type"), "{err}");
    }

    #[test]
    fn unknown_keys_inside_a_tagged_source_are_refused_naming_them() {
        let err = Tectonics
            .params(&table(
                "[[source]]\ntype = \"dome\"\nat = [0.5, 0.5]\nr = 0.1\nrate = 1.0e-3\nzz = 1\n",
            ))
            .unwrap_err();
        assert!(err.to_string().contains("zz"), "{err}");
        let err = Tectonics.params(&table("bogus = 1")).unwrap_err();
        assert_eq!(err.parameter.as_deref(), Some("bogus"));
    }

    const OROGEN: &str = "[[source]]\ntype = \"orogen\"\npath = [[0.1, 0.5], [0.9, 0.5]]\nwidth = 0.1\nrate = 1.0e-3";
    const DOME: &str = "[[source]]\ntype = \"dome\"\nat = [0.5, 0.5]\nr = 0.1\nrate = 1.0e-3";
    const RIFT: &str =
        "[[source]]\ntype = \"rift\"\npath = [[0.1, 0.5], [0.9, 0.5]]\nrate = 1.0e-3";
    const FAULT: &str = "[[source]]\ntype = \"fault\"\npath = [[0.1, 0.1], [0.9, 0.1]]\nrate = 1.0";
    const BASIN: &str = "[[source]]\ntype = \"basin\"\nat = [0.5, 0.5]\nr = 0.1";

    const FINITE: &str = "must be finite";
    const POSITIVE: &str = "must be positive and finite";
    const NOT_NEGATIVE: &str = "must be at least 0 and finite";

    /// Each `(toml, parameter, message)` is refused naming exactly that parameter and message.
    fn refused(cases: &[(String, &str, &str)]) {
        for (toml, parameter, message) in cases {
            let err = Tectonics.params(&table(toml)).unwrap_err();
            assert_eq!(err.parameter.as_deref(), Some(*parameter), "{toml}: {err}");
            assert_eq!(err.message, *message, "{toml}: {err}");
        }
    }

    // Final review round 2, finding 2: every number of the stage has its domain (ruling of
    // the round), refused naming its path before any stage runs.
    #[test]
    fn bad_values_are_refused() {
        let case = |toml: &str, parameter, message| (toml.to_owned(), parameter, message);
        refused(&[
            case("sag_duration_yr = 0.0", "sag_duration_yr", POSITIVE),
            case("sag_duration_yr = inf", "sag_duration_yr", POSITIVE),
            case("background_uplift = nan", "background_uplift", FINITE),
            case("background_variation = inf", "background_variation", FINITE),
            case("background_beta = nan", "background_beta", FINITE),
            case("warp = -inf", "warp", FINITE),
            case("warp_beta = nan", "warp_beta", FINITE),
        ]);
    }

    #[test]
    fn bad_source_values_are_refused() {
        let lean = "must be in [0, 2)";
        let two = "must hold at least two points";
        let with = |source: &str, extra: &str| format!("{source}\n{extra}");
        let swap = |source: &str, from: &str, to: &str| source.replace(from, to);
        refused(&[
            (swap(FAULT, ", [0.9, 0.1]", ""), "source[0].path", two),
            (
                swap(OROGEN, "[0.9, 0.5]", "[inf, 0.5]"),
                "source[0].path[1][0]",
                FINITE,
            ),
            (with(OROGEN, "asymmetry = 2.0"), "source[0].asymmetry", lean),
            (
                with(OROGEN, "asymmetry = -0.1"),
                "source[0].asymmetry",
                lean,
            ),
            (with(OROGEN, "asymmetry = nan"), "source[0].asymmetry", lean),
            (
                swap(OROGEN, "width = 0.1", "width = -1.0"),
                "source[0].width",
                POSITIVE,
            ),
            (
                swap(OROGEN, "width = 0.1", "width = inf"),
                "source[0].width",
                POSITIVE,
            ),
            (
                swap(OROGEN, "rate = 1.0e-3", "rate = nan"),
                "source[0].rate",
                FINITE,
            ),
            (
                with(OROGEN, "jitter = inf"),
                "source[0].jitter",
                NOT_NEGATIVE,
            ),
            (
                with(OROGEN, "jitter = -0.1"),
                "source[0].jitter",
                NOT_NEGATIVE,
            ),
            (swap(DOME, "r = 0.1", "r = 0.0"), "source[0].r", POSITIVE),
            (
                swap(DOME, "[0.5, 0.5]", "[0.5, nan]"),
                "source[0].at[1]",
                FINITE,
            ),
            (
                swap(DOME, "rate = 1.0e-3", "rate = -inf"),
                "source[0].rate",
                FINITE,
            ),
            (
                with(RIFT, "depth_m = -1.0"),
                "source[0].depth_m",
                NOT_NEGATIVE,
            ),
            (
                with(RIFT, "depth_m = inf"),
                "source[0].depth_m",
                NOT_NEGATIVE,
            ),
            (with(RIFT, "width = 0.0"), "source[0].width", POSITIVE),
            (with(FAULT, "reach = nan"), "source[0].reach", POSITIVE),
            (
                with(BASIN, "depth_m = nan"),
                "source[0].depth_m",
                NOT_NEGATIVE,
            ),
            (with(BASIN, "rate = inf"), "source[0].rate", FINITE),
            (
                with(DOME, &swap(OROGEN, "rate = 1.0e-3", "rate = inf")),
                "source[1].rate",
                FINITE,
            ),
        ]);
    }

    #[test]
    fn the_edges_of_the_domains_are_accepted() {
        for toml in [
            format!("{OROGEN}\nasymmetry = 0.0\njitter = 0.0"),
            format!("{OROGEN}\nasymmetry = 1.99"),
            format!("{RIFT}\ndepth_m = 0.0").replace("rate = 1.0e-3", "rate = -1.0e-3"),
            format!("{BASIN}\ndepth_m = 0.0"),
            format!("{DOME}\nat = [-0.2, 1.3]").replace("at = [0.5, 0.5]\n", ""),
            "background_uplift = -1.0e-4\nwarp = 0.0\nwarp_beta = -1.0".to_owned(),
        ] {
            assert!(Tectonics.params(&table(&toml)).is_ok(), "{toml}");
        }
    }

    #[test]
    fn the_stage_needs_the_grid_stage_first() {
        let mut w = world(8, 8);
        assert!(run_stage(&mut w, &Tectonics, "").is_err());
    }

    #[test]
    fn declarations_name_their_layers() {
        let params = Tectonics.params(&table("")).unwrap();
        let reads: Vec<_> = Stage::reads(&Tectonics, params.as_ref())
            .iter()
            .map(|l| l.name)
            .collect();
        let writes: Vec<_> = Stage::writes(&Tectonics, params.as_ref())
            .iter()
            .map(|l| l.name)
            .collect();
        assert_eq!(reads, ["frame", "fixed"]);
        assert_eq!(writes, ["uplift", "sag_rate", "belts", "warp"]);
    }
}
