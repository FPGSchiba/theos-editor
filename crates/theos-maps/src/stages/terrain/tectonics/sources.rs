//! The tectonic source types and how each adds to the uplift and sag fields.

use serde::Deserialize;

use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::curves::{dense_curve, signed_distance};
use crate::kernels::noise::spectral_noise;
use crate::kernels::num::{max, smoothstep};
use crate::rng::StageRng;
use crate::stage::{Check, ParamError};

/// Samples of the dense curve through a path, as in 0.3.0.
const CURVE_SAMPLES: usize = 600;

/// The least flank width and radius a jittered source keeps, map fractions.
const MIN_SIZE: f64 = 1e-9;

const fn half() -> f64 {
    0.5
}
const fn jitter() -> f64 {
    0.06
}
const fn reach() -> f64 {
    0.25
}
const fn rift_width() -> f64 {
    0.06
}
const fn rift_depth() -> f64 {
    150.0
}
const fn basin_depth() -> f64 {
    80.0
}

/// One tectonic feature. Positions are `[x, y]` fractions of the map. The enum carries
/// `deny_unknown_fields` because `serde_ignored` does not see inside tagged enums.
///
/// The domains [`Source::check`] holds them to: positions finite, paths of at least two points,
/// `width`, `reach` and `r` positive, `rate` finite, `jitter` and `depth_m` at least 0, and
/// `asymmetry` in [0, 2).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Source {
    /// A mountain belt along a path, with a gaussian profile that can lean to one side.
    Orogen {
        /// Control points of the belt axis.
        path: Vec<[f64; 2]>,
        /// Half-width of the belt in map fractions.
        width: f64,
        /// Peak uplift in m/yr.
        rate: f64,
        /// How much the belt leans, in [0, 2): the flank on the positive side of the axis (to
        /// the right of the path as it runs) is `width·(1 − asymmetry/2)` wide, the other
        /// `width·(1 + asymmetry/2)`, so the positive flank is the narrower, steeper one; 0 is
        /// symmetric, and at 2 the positive flank would vanish.
        #[serde(default = "half")]
        asymmetry: f64,
        /// Standard deviation of the control point jitter.
        #[serde(default = "jitter")]
        jitter: f64,
    },
    /// An uplifted block on one side of a path.
    Fault {
        /// Control points of the fault trace.
        path: Vec<[f64; 2]>,
        /// Peak uplift in m/yr.
        rate: f64,
        /// Distance over which the uplift fades, in map fractions.
        #[serde(default = "reach")]
        reach: f64,
        /// Standard deviation of the control point jitter.
        #[serde(default = "jitter")]
        jitter: f64,
    },
    /// Uplifted shoulders along a path with a sagging graben between them.
    Rift {
        /// Control points of the rift axis.
        path: Vec<[f64; 2]>,
        /// Half-width of the graben in map fractions.
        #[serde(default = "rift_width")]
        width: f64,
        /// Peak shoulder uplift in m/yr.
        rate: f64,
        /// Depth the graben sags over the sag duration, in metres.
        #[serde(default = "rift_depth")]
        depth_m: f64,
        /// Standard deviation of the control point jitter.
        #[serde(default = "jitter")]
        jitter: f64,
    },
    /// A round uplift.
    Dome {
        /// The centre.
        at: [f64; 2],
        /// The radius in map fractions.
        r: f64,
        /// Peak uplift in m/yr.
        rate: f64,
        /// Standard deviation of the centre jitter.
        #[serde(default = "jitter")]
        jitter: f64,
    },
    /// A cone with a collapsed summit.
    Volcano {
        /// The centre.
        at: [f64; 2],
        /// The radius in map fractions.
        r: f64,
        /// Peak uplift in m/yr.
        rate: f64,
        /// Standard deviation of the centre jitter.
        #[serde(default = "jitter")]
        jitter: f64,
    },
    /// A bowl that damps the uplift and sags.
    Basin {
        /// The centre.
        at: [f64; 2],
        /// The radius in map fractions.
        r: f64,
        /// Depth the bowl sags over the sag duration, in metres. A basin has no rate: it damps
        /// the uplift instead (0.3.0's basin `rate` was never read, so P08 drops it, spec §10).
        #[serde(default = "basin_depth")]
        depth_m: f64,
        /// Standard deviation of the centre jitter.
        #[serde(default = "jitter")]
        jitter: f64,
    },
}

impl Source {
    /// Checks the source's numbers against their domains (see [`Source`]); `at` is the source's
    /// own path, `source[<k>]`.
    ///
    /// # Errors
    /// [`ParamError`] naming `<at>.<parameter>`, such as `source[0].asymmetry`.
    pub(super) fn check(&self, at: &str, check: Check<'_>) -> Result<(), ParamError> {
        let name = |parameter: &str| format!("{at}.{parameter}");
        let path_of = |path: &[[f64; 2]]| points(path, &name("path"), check);
        let centre_of = |centre: &[f64; 2]| {
            for (i, &v) in centre.iter().enumerate() {
                check.finite(&name(&format!("at[{i}]")), v)?;
            }
            Ok(())
        };
        match self {
            Self::Orogen {
                path,
                width,
                rate,
                asymmetry,
                jitter,
            } => {
                path_of(path)?;
                check.positive(&name("width"), *width)?;
                check.finite(&name("rate"), *rate)?;
                check.in_closed_open(&name("asymmetry"), *asymmetry, 0.0, 2.0)?;
                check.at_least(&name("jitter"), *jitter, 0.0)
            }
            Self::Fault {
                path,
                rate,
                reach,
                jitter,
            } => {
                path_of(path)?;
                check.finite(&name("rate"), *rate)?;
                check.positive(&name("reach"), *reach)?;
                check.at_least(&name("jitter"), *jitter, 0.0)
            }
            Self::Rift {
                path,
                width,
                rate,
                depth_m,
                jitter,
            } => {
                path_of(path)?;
                check.positive(&name("width"), *width)?;
                check.finite(&name("rate"), *rate)?;
                check.at_least(&name("depth_m"), *depth_m, 0.0)?;
                check.at_least(&name("jitter"), *jitter, 0.0)
            }
            Self::Dome {
                at: centre,
                r,
                rate,
                jitter,
            }
            | Self::Volcano {
                at: centre,
                r,
                rate,
                jitter,
            } => {
                centre_of(centre)?;
                check.positive(&name("r"), *r)?;
                check.finite(&name("rate"), *rate)?;
                check.at_least(&name("jitter"), *jitter, 0.0)
            }
            Self::Basin {
                at: centre,
                r,
                depth_m,
                jitter,
            } => {
                centre_of(centre)?;
                check.positive(&name("r"), *r)?;
                check.at_least(&name("depth_m"), *depth_m, 0.0)?;
                check.at_least(&name("jitter"), *jitter, 0.0)
            }
        }
    }
}

/// Checks a source's `path` (named `parameter`): at least two points, each coordinate finite.
fn points(path: &[[f64; 2]], parameter: &str, check: Check<'_>) -> Result<(), ParamError> {
    if path.len() < 2 {
        return Err(check.refuse(parameter, "must hold at least two points"));
    }
    for (i, point) in path.iter().enumerate() {
        for (j, &v) in point.iter().enumerate() {
            check.finite(&format!("{parameter}[{i}][{j}]"), v)?;
        }
    }
    Ok(())
}

/// What a source reads and adds to.
pub(super) struct Env<'a> {
    /// Warped x coordinates, fractions.
    pub xx: &'a Grid<f64>,
    /// Warped y coordinates, fractions.
    pub yy: &'a Grid<f64>,
    /// The uplift field, m/yr.
    pub uplift: &'a mut Grid<f64>,
    /// The sag rate field, m/yr.
    pub sag_rate: &'a mut Grid<f64>,
    /// Years the sag depths are spread over.
    pub sag_duration_yr: f64,
    /// The stage's own stream (`rate`, jitter, radius draws, in 0.3.0's order).
    pub rng: &'a mut StageRng,
    /// The `along<k>` stream.
    pub along: StageRng,
    /// The `wid<k>` stream.
    pub wid: StageRng,
}

/// Adds `f(r, c)` to every cell of `field`.
fn add(field: &mut Grid<f64>, f: impl Fn(usize, usize) -> f64) {
    for r in 0..field.rows() {
        for c in 0..field.cols() {
            field.set(r, c, field.get(r, c) + f(r, c));
        }
    }
}

fn square(v: f64) -> f64 {
    v * v
}

fn jittered_path(path: &[[f64; 2]], jit: f64, rng: &mut StageRng) -> Vec<[f64; 2]> {
    path.iter()
        .map(|p| {
            let dx = jit * rng.normal();
            let dy = jit * rng.normal();
            [p[0] + dx, p[1] + dy]
        })
        .collect()
}

/// The jittered centre `(x, y)` and radius of a round source.
fn centre(at: &[f64; 2], r: f64, jit: f64, rng: &mut StageRng) -> (f64, f64, f64) {
    let dx = jit * rng.normal();
    let dy = jit * rng.normal();
    // A draw below −5 would make the radius negative (a volcano's `pow` then NaN everywhere);
    // the floor keeps it a tiny source, as `orogen` floors its flanks.
    let radius = max(r * (1.0 + 0.2 * rng.normal()), MIN_SIZE);
    (at[0] + dx, at[1] + dy, radius)
}

/// The distance of every cell to `(cx, cy)`, in units of `radius`.
fn scaled_distance(xx: &Grid<f64>, yy: &Grid<f64>, cx: f64, cy: f64, radius: f64) -> Grid<f64> {
    let mut out = Grid::filled(xx.rows(), xx.cols(), 0.0);
    add(&mut out, |r, c| {
        let (dx, dy) = (xx.get(r, c) - cx, yy.get(r, c) - cy);
        (dx * dx + dy * dy).sqrt() / radius
    });
    out
}

/// A source along a path: the jittered, dense curve through its control points and the signed
/// and unsigned distance of every cell to it.
fn along_path(
    env: &mut Env<'_>,
    path: &[[f64; 2]],
    jit: f64,
) -> Result<(Grid<f64>, Grid<f64>), KernelError> {
    let curve = dense_curve(&jittered_path(path, jit, env.rng), CURVE_SAMPLES);
    signed_distance(&curve, env.xx, env.yy)
}

/// The draw every source makes first: its rate with 15 % scatter.
fn scattered(rate: f64, rng: &mut StageRng) -> f64 {
    rate * (1.0 + 0.15 * rng.normal())
}

impl Source {
    /// Adds the source to the fields of `env`. Returns the signed distance to the belt axis
    /// for an orogen (the first one becomes the `belts` value).
    ///
    /// # Errors
    /// [`KernelError`] when the distance to a curve cannot be computed.
    pub(super) fn apply(&self, env: &mut Env<'_>) -> Result<Option<Grid<f64>>, KernelError> {
        match self {
            Self::Orogen {
                path,
                width,
                rate,
                asymmetry,
                jitter,
            } => {
                let rate = scattered(*rate, env.rng);
                let (sd, _) = along_path(env, path, *jitter)?;
                orogen(env, &sd, rate, *width, *asymmetry);
                Ok(Some(sd))
            }
            Self::Fault {
                path,
                rate,
                reach,
                jitter,
            } => {
                let rate = scattered(*rate, env.rng);
                let (sd, d) = along_path(env, path, *jitter)?;
                fault(env, &sd, &d, rate, *reach);
                Ok(None)
            }
            Self::Rift {
                path,
                width,
                rate,
                depth_m,
                jitter,
            } => {
                let rate = scattered(*rate, env.rng);
                let (sd, _) = along_path(env, path, *jitter)?;
                rift(env, &sd, rate, *width, *depth_m);
                Ok(None)
            }
            Self::Dome {
                at,
                r,
                rate,
                jitter,
            }
            | Self::Volcano {
                at,
                r,
                rate,
                jitter,
            } => {
                let rate = scattered(*rate, env.rng);
                let (cx, cy, radius) = centre(at, *r, *jitter, env.rng);
                let d = scaled_distance(env.xx, env.yy, cx, cy, radius);
                round_uplift(env, &d, rate, matches!(self, Self::Volcano { .. }));
                Ok(None)
            }
            Self::Basin {
                at,
                r,
                depth_m,
                jitter,
            } => {
                // The rate draw 0.3.0 makes for every source, a basin too: it keeps the stream.
                let _ = scattered(0.0, env.rng);
                let (cx, cy, radius) = centre(at, *r, *jitter, env.rng);
                let d = scaled_distance(env.xx, env.yy, cx, cy, radius);
                basin(env, &d, *depth_m);
                Ok(None)
            }
        }
    }
}

fn orogen(env: &mut Env<'_>, sd: &Grid<f64>, rate: f64, width: f64, asymmetry: f64) {
    let (rows, cols) = (sd.rows(), sd.cols());
    let along = spectral_noise(rows, cols, &mut env.along, 3.8, 0.0, None);
    let wid = spectral_noise(rows, cols, &mut env.wid, 4.0, 0.0, None);
    add(env.uplift, |r, c| {
        let w = width * (1.0 + 0.25 * wid.get(r, c));
        let s = sd.get(r, c);
        let lean = if s > 0.0 {
            -0.5 * asymmetry
        } else {
            0.5 * asymmetry
        };
        let side = max(w * (1.0 + lean), MIN_SIZE);
        let a = max(1.0 + 0.45 * along.get(r, c), 0.2);
        rate * a * libm::exp(-0.5 * square(s / side))
    });
}

fn fault(env: &mut Env<'_>, sd: &Grid<f64>, d: &Grid<f64>, rate: f64, reach: f64) {
    let along = spectral_noise(sd.rows(), sd.cols(), &mut env.along, 3.8, 0.0, None);
    add(env.uplift, |r, c| {
        let a = max(1.0 + 0.45 * along.get(r, c), 0.3);
        rate * a
            * smoothstep(-0.01, 0.02, sd.get(r, c))
            * libm::exp(-0.5 * square(d.get(r, c) / reach))
    });
}

fn rift(env: &mut Env<'_>, sd: &Grid<f64>, rate: f64, hw: f64, depth_m: f64) {
    let sag = depth_m / env.sag_duration_yr;
    add(env.uplift, |r, c| {
        rate * libm::exp(-0.5 * square((sd.get(r, c).abs() - hw) / (hw * 0.6)))
    });
    add(env.sag_rate, |r, c| {
        sag * libm::exp(-0.5 * square(sd.get(r, c) / (hw * 0.7)))
    });
}

/// A dome or, with `cone`, a volcano; `d` is the distance in radii.
fn round_uplift(env: &mut Env<'_>, d: &Grid<f64>, rate: f64, cone: bool) {
    add(env.uplift, |r, c| {
        let v = d.get(r, c);
        if cone {
            rate * libm::exp(-libm::pow(v, 1.4)) * (1.0 - 0.6 * libm::exp(-square(v / 0.18)))
        } else {
            rate * libm::exp(-0.5 * square(v))
        }
    });
}

fn basin(env: &mut Env<'_>, d: &Grid<f64>, depth_m: f64) {
    let sag = depth_m / env.sag_duration_yr;
    let bowl = |r, c| libm::exp(-0.5 * square(d.get(r, c)));
    for r in 0..d.rows() {
        for c in 0..d.cols() {
            let u = env.uplift.get(r, c);
            env.uplift.set(r, c, u * (1.0 - 0.7 * bowl(r, c)));
        }
    }
    add(env.sag_rate, |r, c| sag * bowl(r, c));
}
