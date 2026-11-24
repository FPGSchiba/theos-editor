//! The `geology` stage: rock provinces, a fold belt, plutons, basin fill and the rock-dependent
//! erodibility, diffusivity and critical slope.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::grid::FrameValue;
use super::rocks::{Rock, props};
use super::tectonics::{Belts, Warp};
use crate::grid::Grid;
use crate::kernels::noise::spectral_noise;
use crate::kernels::num::{max, min};
use crate::kernels::voronoi::nearest_seed;
use crate::stage::{
    Check, LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::world::{Layer, ValueType};

/// The fold belt around the first orogen: alternating hard and weak strata.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct FoldBelt {
    /// Width of the belt, map fractions.
    pub width: f64,
    /// Peak strength of the strata.
    pub strength: f64,
    /// Spacing of the strata, map fractions.
    pub period: f64,
    /// Erodibility ratio between weak and hard strata.
    pub contrast: f64,
}

impl Default for FoldBelt {
    fn default() -> Self {
        Self {
            width: 0.12,
            strength: 0.6,
            period: 0.035,
            contrast: 2.5,
        }
    }
}

/// The parameters of `geology`; defaults as 0.3.0 where it had them.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct GeologyParams {
    /// Number of Voronoi rock provinces.
    pub provinces: usize,
    /// Weight of every rock a province may get; taken in `Rock` order, not key order.
    pub rock_mix: BTreeMap<Rock, f64>,
    /// The fold belt; absent means none.
    pub fold_belt: Option<FoldBelt>,
    /// Smallest and largest number of granite plutons.
    pub plutons: [u32; 2],
}

impl Default for GeologyParams {
    fn default() -> Self {
        // 0.3.0 required `rock_mix`; this default is a generic sedimentary mix.
        let rock_mix = BTreeMap::from([
            (Rock::Granite, 0.15),
            (Rock::Quartzite, 0.1),
            (Rock::Basalt, 0.1),
            (Rock::Limestone, 0.25),
            (Rock::Sandstone, 0.25),
            (Rock::Shale, 0.15),
        ]);
        Self {
            provinces: 6,
            rock_mix,
            fold_belt: None,
            plutons: [0, 2],
        }
    }
}

/// The stage `geology`.
pub struct Geology;

impl TypedStage for Geology {
    type Params = GeologyParams;
    const ID: &'static str = "geology";
    const SUMMARY: &'static str = "Rock provinces, fold belt, plutons and basin fill, with per-rock erodibility, diffusivity and critical slope.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &GeologyParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::value("frame", FrameValue::NAME),
            LayerSpec::f64("uplift"),
            LayerSpec::f64("sag_rate"),
            LayerSpec::value("belts", Belts::NAME),
            LayerSpec::value("warp", Warp::NAME),
        ]
    }

    fn writes(&self, _: &GeologyParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::u8("rock"),
            LayerSpec::f64("erodibility"),
            LayerSpec::f64("diffusivity"),
            LayerSpec::f64("critical_slope"),
        ]
    }

    fn validate(&self, params: &mut GeologyParams) -> Result<(), ParamError> {
        let check = Check::of(Self::ID);
        let at = |parameter: &str, message: &str| ParamError::at(Self::ID, parameter, message);
        check.count_at_least("provinces", params.provinces, 1)?;
        for (rock, &weight) in &params.rock_mix {
            check.at_least(&format!("rock_mix.{}", rock.name()), weight, 0.0)?;
        }
        if params.rock_mix.values().sum::<f64>() <= 0.0 {
            return Err(at("rock_mix", "needs a positive total weight"));
        }
        if params.plutons[0] > params.plutons[1] {
            return Err(at("plutons", "must be [min, max] with min <= max"));
        }
        if let Some(fb) = &params.fold_belt {
            check.positive("fold_belt.width", fb.width)?;
            check.positive("fold_belt.period", fb.period)?;
            check.positive("fold_belt.contrast", fb.contrast)?;
            check.finite("fold_belt.strength", fb.strength)?;
        }
        Ok(())
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &GeologyParams,
    ) -> Result<StageReport, StageError> {
        let (rows, cols) = (ctx.frame().rows, ctx.frame().cols);
        let warp = ctx.value::<Warp>("warp")?.clone();
        let belt = ctx.value::<Belts>("belts")?.first_orogen.clone();
        let sag = ctx.f64("sag_rate")?.clone();

        let mut rock = province_rocks(ctx, params, &warp)?;
        let mut modifier = Grid::filled(rows, cols, 1.0);
        if let (Some(fb), Some(sd)) = (&params.fold_belt, &belt) {
            modifier = fold_belt(ctx, fb, sd, &mut rock);
        }
        add_plutons(ctx, params, &warp, &mut rock)?;
        fill_basins(&sag, &mut rock);

        let (kmin, kmax) = k_range();
        let mut weather_rng = ctx.sub_rng("weather");
        let weather = spectral_noise(rows, cols, &mut weather_rng, 2.6, 0.0, None);
        let mut erodibility = Vec::with_capacity(rock.len());
        let mut diffusivity = Vec::with_capacity(rock.len());
        let mut critical = Vec::with_capacity(rock.len());
        for ((&code, &m), &n) in rock.iter().zip(modifier.as_slice()).zip(weather.as_slice()) {
            let p = props(Rock::from_code(code).unwrap_or(Rock::Alluvium));
            let k = p.k * m * libm::exp(0.25 * min(max(n, -2.0), 2.0));
            erodibility.push(min(max(k, kmin * 0.5), kmax * 1.5));
            diffusivity.push(p.kd);
            critical.push(p.critical_slope);
        }
        let grid = |v: Vec<f64>| {
            Grid::from_vec(rows, cols, v).map_err(|e| ctx.fail(format!("grid shape: {e}")))
        };
        let (erodibility, diffusivity, critical) =
            (grid(erodibility)?, grid(diffusivity)?, grid(critical)?);
        let rock =
            Grid::from_vec(rows, cols, rock).map_err(|e| ctx.fail(format!("grid shape: {e}")))?;
        ctx.write("rock", Layer::U8(rock))?;
        ctx.write("erodibility", Layer::F64(erodibility))?;
        ctx.write("diffusivity", Layer::F64(diffusivity))?;
        ctx.write("critical_slope", Layer::F64(critical))?;
        Ok(StageReport::default())
    }
}

/// The smallest and largest K of the table.
fn k_range() -> (f64, f64) {
    Rock::ALL.iter().fold((f64::INFINITY, 0.0), |(lo, hi), r| {
        (min(lo, props(*r).k), max(hi, props(*r).k))
    })
}

/// The rock of every cell from Voronoi provinces in warped space.
///
/// 0.3.0's `rng.random((n, 2))` is n points drawn x then y; `rng.choice(p=...)` is one uniform
/// per province against the cumulative weights in `Rock` order.
fn province_rocks(
    ctx: &mut StageContext<'_>,
    params: &GeologyParams,
    warp: &Warp,
) -> Result<Vec<u8>, StageError> {
    let rng = ctx.rng();
    let seeds: Vec<[f64; 2]> = (0..params.provinces)
        .map(|_| {
            let x = rng.uniform();
            [x, rng.uniform()]
        })
        .collect();
    let weights: Vec<f64> = Rock::ALL
        .iter()
        .map(|r| params.rock_mix.get(r).copied().unwrap_or(0.0))
        .collect();
    let total: f64 = weights.iter().sum();
    let mut cdf = Vec::with_capacity(weights.len());
    let mut run = 0.0;
    for w in &weights {
        run += w / total;
        cdf.push(run);
    }
    let last = cdf.last().copied().unwrap_or(1.0);
    let last_used = weights.iter().rposition(|w| *w > 0.0).unwrap_or(0);
    let rock_of: Vec<u8> = seeds
        .iter()
        .map(|_| {
            let u = rng.uniform();
            let i = cdf.iter().position(|c| c / last > u).unwrap_or(last_used);
            Rock::ALL[i].code()
        })
        .collect();
    let labels =
        nearest_seed(&seeds, &warp.x, &warp.y).map_err(|e| ctx.fail(format!("provinces: {e}")))?;
    Ok(labels
        .as_slice()
        .iter()
        .map(|&l| {
            usize::try_from(l)
                .ok()
                .and_then(|i| rock_of.get(i))
                .copied()
                .unwrap_or(0)
        })
        .collect())
}

/// Quartzite and shale strata along the belt; returns the erodibility modifier.
fn fold_belt(ctx: &StageContext<'_>, fb: &FoldBelt, sd: &Grid<f64>, rock: &mut [u8]) -> Grid<f64> {
    let (rows, cols) = (sd.rows(), sd.cols());
    let noise = |name: &str, beta: f64| {
        let mut rng = ctx.sub_rng(name);
        spectral_noise(rows, cols, &mut rng, beta, 0.0, None)
    };
    let period_noise = noise("fb-p", 3.5);
    let phase_noise = noise("fb-ph", 2.8);
    let amp_noise = noise("fb-amp", 3.2);
    let log_contrast = libm::log(fb.contrast);
    let mut modifier = Vec::with_capacity(rock.len());
    for (i, cell) in rock.iter_mut().enumerate() {
        let d = sd.as_slice()[i];
        let z = d / fb.width;
        let env = libm::exp(-0.5 * z * z) * fb.strength;
        let period = fb.period * (1.0 + 0.35 * period_noise.as_slice()[i]);
        let phase = d / period + 1.1 * phase_noise.as_slice()[i];
        let amp = 0.6 + 0.4 * min(max(amp_noise.as_slice()[i], -1.0), 1.0);
        let strata = libm::sin(2.0 * std::f64::consts::PI * phase) * amp;
        modifier.push(libm::exp(log_contrast * env * -strata));
        if env > 0.25 && strata > 0.3 {
            *cell = Rock::Quartzite.code();
        } else if env > 0.25 && strata < -0.3 {
            *cell = Rock::Shale.code();
        }
    }
    Grid::from_vec(rows, cols, modifier).unwrap_or_else(|_| Grid::filled(rows, cols, 1.0))
}

/// Granite plutons: noisy discs at random places on the warped coordinates.
///
/// # Errors
/// [`StageError::Cancelled`], checked before each pluton: each costs a noise field of the whole
/// grid, and a recipe may ask for up to a million.
fn add_plutons(
    ctx: &mut StageContext<'_>,
    params: &GeologyParams,
    warp: &Warp,
    rock: &mut [u8],
) -> Result<(), StageError> {
    let [lo, hi] = params.plutons;
    let count = u64::from(lo) + ctx.rng().below(u64::from(hi - lo) + 1);
    for i in 0..count {
        ctx.check_cancel()?;
        let rng = ctx.rng();
        let cx = rng.uniform() * 0.8 + 0.1;
        let cy = rng.uniform() * 0.8 + 0.1;
        let r = rng.range(0.04, 0.09);
        let mut noise_rng = ctx.sub_rng(&format!("pl{i}"));
        let noise = spectral_noise(warp.x.rows(), warp.x.cols(), &mut noise_rng, 3.0, 0.0, None);
        for (j, cell) in rock.iter_mut().enumerate() {
            let (dx, dy) = (warp.x.as_slice()[j] - cx, warp.y.as_slice()[j] - cy);
            let blob = (dx * dx + dy * dy).sqrt() / r + 0.25 * noise.as_slice()[j];
            if blob < 1.0 {
                *cell = Rock::Granite.code();
            }
        }
    }
    Ok(())
}

/// Alluvium on basin floors: where the sag rate exceeds 0.3 of its maximum (0.3.0's rule).
fn fill_basins(sag: &Grid<f64>, rock: &mut [u8]) {
    let top = sag.as_slice().iter().fold(0.0, |a, &v| max(a, v));
    let threshold = 0.3 * top;
    for (cell, &s) in rock.iter_mut().zip(sag.as_slice()) {
        if s > threshold && s > 0.0 {
            *cell = Rock::Alluvium.code();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::Stage;
    use crate::stages::terrain::grid::GridStage;
    use crate::stages::terrain::support::{run_stage, table, world};
    use crate::stages::terrain::tectonics::Tectonics;
    use crate::world::World;

    const BASIN: &str =
        "[[source]]\ntype = \"basin\"\nat = [0.5, 0.5]\nr = 0.2\ndepth_m = 60.0\njitter = 0.0\n";
    const OROGEN: &str = "[[source]]\ntype = \"orogen\"\npath = [[0.1, 0.5], [0.9, 0.5]]\nwidth = 0.1\nrate = 1.0e-3\njitter = 0.0\n";

    fn geo_world(tectonics: &str, geology: &str) -> World {
        let mut w = world(64, 64);
        run_stage(&mut w, &GridStage, "").unwrap();
        run_stage(&mut w, &Tectonics, tectonics).unwrap();
        run_stage(&mut w, &Geology, geology).unwrap();
        w
    }

    fn count(w: &World, rock: Rock) -> usize {
        let cells = w.u8("rock").unwrap().as_slice();
        cells
            .iter()
            .fold(0, |n, c| n + usize::from(*c == rock.code()))
    }

    #[test]
    fn a_single_rock_mix_fills_the_map() {
        let w = geo_world("", "plutons = [0, 0]\n[rock_mix]\ngranite = 1.0\n");
        assert_eq!(count(&w, Rock::Granite), 64 * 64);
        let kd = w.f64("diffusivity").unwrap();
        assert!(kd.as_slice().iter().all(|v| (v - 0.004).abs() < 1e-12));
        let sc = w.f64("critical_slope").unwrap();
        let want = props(Rock::Granite).critical_slope;
        assert!(sc.as_slice().iter().all(|v| (v - want).abs() < 1e-12));
    }

    #[test]
    fn alluvium_fills_sinking_basins() {
        let w = geo_world(BASIN, "plutons = [0, 0]\n[rock_mix]\ngranite = 1.0\n");
        let sag = w.f64("sag_rate").unwrap();
        let top = sag.as_slice().iter().fold(0.0, |a, &v| max(a, v));
        let rock = w.u8("rock").unwrap();
        assert!(count(&w, Rock::Alluvium) > 10);
        assert!(count(&w, Rock::Granite) > 10);
        for (s, r) in sag.as_slice().iter().zip(rock.as_slice()) {
            assert_eq!(*r == Rock::Alluvium.code(), *s > 0.3 * top);
        }
        assert_eq!(rock.get(32, 32), Rock::Alluvium.code());
    }

    #[test]
    fn erodibility_stays_in_the_table_range() {
        let w = geo_world(
            &format!("{OROGEN}{BASIN}"),
            "[fold_belt]\ncontrast = 4.0\n[rock_mix]\ngranite = 1.0\nalluvium = 1.0\nshale = 1.0\n",
        );
        let (lo, hi) = (1.6e-5 * 0.5, 4.0e-4 * 1.5);
        let k = w.f64("erodibility").unwrap().as_slice();
        assert!(k.iter().all(|v| *v >= lo - 1e-18 && *v <= hi + 1e-18));
        assert!(k.iter().any(|v| *v > 1.6e-5 * 1.5), "varies beyond granite");
    }

    #[test]
    fn critical_slopes_follow_the_rock_order() {
        let slopes: Vec<f64> = Rock::ALL.iter().map(|r| props(*r).critical_slope).collect();
        assert!((slopes[0] - slopes[1]).abs() < 1e-12);
        assert!(slopes.windows(2).all(|p| p[0] >= p[1]));
        assert!(slopes.iter().all(|s| (0.4..=1.0).contains(s)));
    }

    #[test]
    fn unknown_rocks_are_parameter_errors() {
        let err = Geology
            .params(&table("[rock_mix]\nmarble = 1.0\n"))
            .unwrap_err();
        assert!(err.to_string().contains("marble"), "{err}");
    }

    #[test]
    fn a_fold_belt_adds_strata_along_the_orogen() {
        let mix = "plutons = [0, 0]\n[rock_mix]\nsandstone = 1.0\n";
        let plain = geo_world(OROGEN, mix);
        assert_eq!(count(&plain, Rock::Sandstone), 64 * 64);
        let folded = geo_world(
            OROGEN,
            &format!("{mix}[fold_belt]\nwidth = 0.2\nstrength = 1.0\n"),
        );
        assert!(count(&folded, Rock::Quartzite) > 20);
        assert!(count(&folded, Rock::Shale) > 20);
        // far from the belt nothing changes
        assert_eq!(folded.u8("rock").unwrap().get(0, 0), Rock::Sandstone.code());
    }

    #[test]
    fn plutons_make_granite_and_the_run_is_repeatable() {
        let mix = "plutons = [3, 3]\n[rock_mix]\nshale = 1.0\n";
        let a = geo_world("", mix);
        let b = geo_world("", mix);
        assert_eq!(a.u8("rock"), b.u8("rock"));
        assert!(count(&a, Rock::Granite) > 0);
        assert!(count(&a, Rock::Shale) > 0);
    }

    #[test]
    fn provinces_split_the_map_between_rocks() {
        let w = geo_world(
            "",
            "provinces = 12\nplutons = [0, 0]\n[rock_mix]\nlimestone = 1.0\nbasalt = 1.0\n",
        );
        assert!(count(&w, Rock::Limestone) > 0 && count(&w, Rock::Basalt) > 0);
        assert_eq!(
            count(&w, Rock::Limestone) + count(&w, Rock::Basalt),
            64 * 64
        );
    }

    #[test]
    fn bad_values_are_refused() {
        for (toml, parameter, text) in [
            ("provinces = 0", "provinces", "at least 1"),
            (
                "[rock_mix]\ngranite = -1.0",
                "rock_mix.granite",
                "must be at least 0 and finite",
            ),
            (
                "[rock_mix]\nshale = nan",
                "rock_mix.shale",
                "must be at least 0",
            ),
            ("[rock_mix]\ngranite = 0.0", "rock_mix", "positive"),
            ("plutons = [3, 1]", "plutons", "min <= max"),
            ("[fold_belt]\nperiod = 0.0", "fold_belt.period", "positive"),
            ("[fold_belt]\nwidth = -1.0", "fold_belt.width", "positive"),
            (
                "[fold_belt]\ncontrast = inf",
                "fold_belt.contrast",
                "positive",
            ),
            (
                "[fold_belt]\nstrength = nan",
                "fold_belt.strength",
                "finite",
            ),
            ("[fold_belt]\nzz = 1", "fold_belt.zz", "unknown"),
        ] {
            let err = Geology.params(&table(toml)).unwrap_err();
            assert_eq!(err.parameter.as_deref(), Some(parameter), "{toml}: {err}");
            assert!(err.to_string().contains(text), "{toml}: {err}");
        }
    }

    // Final review round 3, class B: a million plutons (the most a recipe may ask for) are a
    // million noise fields; the loop now stops at a cancel instead of running for many minutes.
    #[test]
    fn a_cancel_stops_the_plutons() {
        let mut w = world(64, 64);
        run_stage(&mut w, &GridStage, "").unwrap();
        run_stage(&mut w, &Tectonics, "").unwrap();
        let params = Geology
            .params(&table("plutons = [1000000, 1000000]"))
            .unwrap();
        let cancel = crate::progress::CancelToken::new();
        let trigger = cancel.clone();
        let started = std::time::Instant::now();
        let stopper = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            trigger.cancel();
        });
        let mut sink = |_: &crate::progress::Progress| {};
        let mut ctx = StageContext::new(
            &mut w,
            &Geology,
            params.as_ref(),
            "geology#0",
            11,
            &cancel,
            &mut sink,
        );
        let result = Stage::run(&Geology, &mut ctx, params.as_ref());
        stopper.join().unwrap();
        assert!(matches!(result, Err(StageError::Cancelled)), "{result:?}");
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
    }

    #[test]
    fn the_stage_needs_tectonics_first() {
        let mut w = world(8, 8);
        assert!(run_stage(&mut w, &Geology, "").is_err());
    }

    #[test]
    fn declarations_name_their_layers() {
        let params = Geology.params(&table("")).unwrap();
        let reads: Vec<_> = Stage::reads(&Geology, params.as_ref())
            .iter()
            .map(|l| l.name)
            .collect();
        let writes: Vec<_> = Stage::writes(&Geology, params.as_ref())
            .iter()
            .map(|l| l.name)
            .collect();
        assert_eq!(reads, ["frame", "uplift", "sag_rate", "belts", "warp"]);
        assert_eq!(
            writes,
            ["rock", "erodibility", "diffusivity", "critical_slope"]
        );
    }
}
