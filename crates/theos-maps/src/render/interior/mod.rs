//! The renders of an interior map (spec P34 §5.11): `base`, `blueprint` and `graph`, and the
//! extra `blueprint-ruin`, listed once in [`RENDERS`], the interior kind's table of renders.
//! Drawing follows P08's renders: `tiny-skia`, Inter through `skrifa`, the canvas that clamps
//! coordinates, and a cancel check per row.

mod base;
mod blueprint;
mod cells;
mod doors;
mod geo;
mod graph;
mod skin;
#[cfg(test)]
mod tests;

use tiny_skia::{Color, Pixmap};

use super::fonts::Face;
use super::legend::{self, Entry};
use super::truth::MARGIN;
use super::{Render, RenderError, RenderInput, Role};
use crate::kernels::num::{to_index, to_usize};
use crate::stages::interior::playability_core::Analysis;

/// Every render of an interior, the standard ones in the order a run draws them. A new render is
/// one module and one entry here.
pub(crate) const RENDERS: &[Render] = &[
    Render {
        name: "base",
        role: Role::Standard,
        draw: base::base,
        needs: base::NEEDS,
    },
    Render {
        name: "blueprint",
        role: Role::Standard,
        draw: blueprint::blueprint,
        needs: blueprint::NEEDS,
    },
    Render {
        name: "graph",
        role: Role::Standard,
        draw: graph::graph,
        needs: graph::NEEDS,
    },
    Render {
        name: "blueprint-ruin",
        role: Role::Extra,
        draw: blueprint::blueprint_ruin,
        needs: blueprint::NEEDS,
    },
];

/// A margin-coloured canvas as wide as `ground` and `top` and `bottom` pixels taller.
fn canvas_with_margins(ground: &Pixmap, top: u32, bottom: u32) -> Result<Pixmap, RenderError> {
    let height = ground.height().saturating_add(top).saturating_add(bottom);
    let size = RenderError::Canvas {
        width: to_usize(ground.width()),
        height: to_usize(height),
    };
    let mut pixmap = Pixmap::new(ground.width(), height).ok_or(size)?;
    pixmap.fill(Color::from_rgba8(MARGIN[0], MARGIN[1], MARGIN[2], u8::MAX));
    Ok(pixmap)
}

/// The title of a plan render: the run's title and the critical path in metres.
fn critical_path_title(input: &RenderInput<'_>, analysis: &Analysis) -> String {
    format!(
        "{} \u{b7} critical path {:.0} m",
        input.title, analysis.critical_path_m
    )
}

/// `plan` widened by a strip on its right that holds the legend of `entries`, so the legend
/// never covers the plan: an interior's plan is small (2 pixels per metre), and a legend in its
/// corner would hide half of it. No strip for no entries.
///
/// # Errors
/// [`RenderError::Canvas`] when the canvas cannot be made, [`RenderError::Font`] when a glyph
/// cannot be drawn.
fn legend_beside(plan: &Pixmap, face: &Face, entries: &[Entry]) -> Result<Pixmap, RenderError> {
    let strip = legend::width(face, entries, f64::from(plan.height())).ceil();
    let strip = u32::try_from(to_index(strip, usize::MAX)).unwrap_or(u32::MAX);
    let width = plan.width().saturating_add(strip);
    let size = RenderError::Canvas {
        width: to_usize(width),
        height: to_usize(plan.height()),
    };
    let mut out = Pixmap::new(width, plan.height()).ok_or(size)?;
    out.fill(Color::from_rgba8(MARGIN[0], MARGIN[1], MARGIN[2], u8::MAX));
    out.draw_pixmap(
        0,
        0,
        plan.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );
    legend::draw(&mut out, face, entries)?;
    Ok(out)
}
