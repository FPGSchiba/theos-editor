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

use super::truth::MARGIN;
use super::{Render, RenderError, RenderInput, Role};
use crate::kernels::num::to_usize;
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
