//! The `envelope` stage (spec §5.5, §5.7): the buildable outline of an interior and the lattice
//! over it.
//!
//! The outline is the frame minus `margin_m` (`rect`), the largest centred circle minus the
//! margin (`circle`), or that circle with its rim eroded by noise (`ragged_circle`, 0.3.0's
//! `envelope` erosion; module [`super::outline`]). The outline noise is one call of P08's
//! `spectral_noise` on `sub_rng("outline")` (β 2.4, kmin 1). The lattice of `pitch_m` covers the
//! frame, its staggered wall offsets drawn from the stage's stream, and enables the cells whose
//! block lies inside the outline.

use serde::{Deserialize, Serialize};

use crate::kernels::noise::spectral_noise;
use crate::kernels::num::to_usize;
use crate::rooms::lattice::{Lattice, LatticeError, Stagger};
use crate::stage::{
    Check, LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::interior::outline;
use crate::stages::interior::support::cells;
use crate::stages::interior::units;
use crate::world::{Layer, MapFrame, ValueType};

/// The spectral exponent of the outline noise (0.3.0's `envelope`).
const OUTLINE_BETA: f64 = 2.4;

/// The outline of the buildable area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outline {
    /// The frame minus the margin.
    Rect,
    /// The largest centred circle minus the margin.
    Circle,
    /// The circle with its rim eroded where the noise exceeds `ragged_t`.
    RaggedCircle,
}

fn rect() -> Outline {
    Outline::Rect
}

fn rows() -> Stagger {
    Stagger::Rows
}

fn half() -> f64 {
    0.5
}

/// The parameters of `envelope`; lengths in metres, whole raster cells.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EnvelopeParams {
    /// The outline; `rect` when left out.
    #[serde(default = "rect")]
    pub shape: Outline,
    /// The margin kept free inside the frame, m.
    #[serde(default)]
    pub margin_m: f64,
    /// The size of a one-cell room plus its wall, m; at least 6 raster cells.
    pub pitch_m: f64,
    /// The largest offset of a staggered wall line, m; at most a quarter of the pitch.
    #[serde(default)]
    pub jitter_m: f64,
    /// Which wall lines move; `rows` when left out.
    #[serde(default = "rows")]
    pub stagger: Stagger,
    /// The noise threshold above which a `ragged_circle` loses a rim cell.
    #[serde(default = "half")]
    pub ragged_t: f64,
}

/// The lengths of the parameters in raster cells.
struct Cells {
    margin: u32,
    pitch: u32,
    jitter: u32,
}

/// The parameter a lattice error is about.
fn lattice_parameter(err: &LatticeError) -> &'static str {
    match err {
        LatticeError::JitterTooLarge { .. } => "jitter_m",
        LatticeError::PitchTooSmall(_) | LatticeError::TooSmall { .. } => "pitch_m",
    }
}

/// The stage `envelope`.
pub struct Envelope;

impl TypedStage for Envelope {
    type Params = EnvelopeParams;
    const ID: &'static str = "envelope";
    const SUMMARY: &'static str =
        "The buildable outline of an interior and the lattice of rooms over it.";
    const KINDS: &'static [MapKind] = &[MapKind::Interior];

    fn reads(&self, _: &EnvelopeParams) -> Vec<LayerSpec> {
        Vec::new()
    }

    fn writes(&self, _: &EnvelopeParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::mask("envelope"),
            LayerSpec::value("lattice", Lattice::NAME),
        ]
    }

    fn validate(&self, p: &mut EnvelopeParams) -> Result<(), ParamError> {
        let check = Check::of(Self::ID);
        check.at_least("margin_m", p.margin_m, 0.0)?;
        check.positive("pitch_m", p.pitch_m)?;
        check.at_least("jitter_m", p.jitter_m, 0.0)?;
        check.finite("ragged_t", p.ragged_t)
    }

    fn check_frame(&self, p: &EnvelopeParams, frame: &MapFrame) -> Result<(), ParamError> {
        let c = Cells {
            margin: units::cells(Self::ID, "margin_m", p.margin_m, frame)?,
            pitch: units::cells(Self::ID, "pitch_m", p.pitch_m, frame)?,
            jitter: units::cells(Self::ID, "jitter_m", p.jitter_m, frame)?,
        };
        Lattice::size(frame.rows, frame.cols, c.pitch, c.jitter)
            .map_err(|e| ParamError::at(Self::ID, lattice_parameter(&e), e.to_string()))?;
        if 2 * to_usize(c.margin) >= frame.rows.min(frame.cols) {
            return Err(
                Check::of(Self::ID).refuse("margin_m", "leaves no buildable cell inside the frame")
            );
        }
        Ok(())
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        p: &EnvelopeParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let margin = to_usize(cells(ctx, "margin_m", p.margin_m)?);
        let pitch = cells(ctx, "pitch_m", p.pitch_m)?;
        let jitter = cells(ctx, "jitter_m", p.jitter_m)?;
        let (rows, cols) = (frame.rows, frame.cols);
        let envelope = match p.shape {
            Outline::Rect => outline::rect(rows, cols, margin),
            Outline::Circle => outline::circle(rows, cols, margin),
            Outline::RaggedCircle => {
                let mut stream = ctx.sub_rng("outline");
                let noise = spectral_noise(rows, cols, &mut stream, OUTLINE_BETA, 1.0, None);
                outline::ragged_circle(rows, cols, margin, &noise, p.ragged_t)
            }
        };
        let mut lattice = Lattice::new(rows, cols, pitch, jitter, p.stagger, ctx.rng())
            .map_err(|e| ctx.fail(e.to_string()))?;
        lattice.enable_inside(&envelope);
        ctx.write("envelope", Layer::Mask(envelope))?;
        ctx.write("lattice", Layer::Value(Box::new(lattice)))?;
        Ok(StageReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::Stage;

    fn frame(n: usize, cell_m: f64) -> MapFrame {
        MapFrame {
            rows: n,
            cols: n,
            cell_m,
        }
    }

    fn refusal(text: &str, f: &MapFrame) -> Option<String> {
        let stage: &dyn Stage = &Envelope;
        match stage.params(&toml::from_str(text).unwrap()) {
            Err(e) => e.parameter,
            Ok(p) => stage
                .check_frame(p.as_ref(), f)
                .err()
                .and_then(|e| e.parameter),
        }
    }

    #[test]
    fn the_frame_check_refuses_what_the_lattice_cannot_build() {
        let f = frame(40, 1.0);
        assert_eq!(refusal("pitch_m = 8.0", &f), None);
        assert_eq!(
            refusal("pitch_m = 8.0\njitter_m = 2.0\nmargin_m = 3.0", &f),
            None
        );
        for (text, parameter) in [
            ("pitch_m = 15.5", "pitch_m"),
            ("pitch_m = 5.0", "pitch_m"),
            ("pitch_m = 41.0", "pitch_m"),
            ("pitch_m = 8.0\njitter_m = 3.0", "jitter_m"),
            ("pitch_m = 8.0\njitter_m = 0.5", "jitter_m"),
            ("pitch_m = 8.0\nmargin_m = 20.0", "margin_m"),
            ("pitch_m = 8.0\nmargin_m = 1.5", "margin_m"),
            ("pitch_m = 8.0\nmargin_m = -1.0", "margin_m"),
        ] {
            assert_eq!(refusal(text, &f).as_deref(), Some(parameter), "{text}");
        }
        // At 0.5 m cells, 8 m is 16 cells and 15.5 m is 31.
        assert_eq!(refusal("pitch_m = 15.5", &frame(80, 0.5)), None);
    }
}
