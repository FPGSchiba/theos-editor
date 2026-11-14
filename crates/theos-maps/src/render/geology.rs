//! The `geology` render (spec §5.11): rock colours over the hillshade, rivers and lakes, and a
//! legend of the rocks the map has. Frame-exact, no text but the legend.

use tiny_skia::Pixmap;

use super::canvas::PX_PER_CELL;
use super::legend::{self, Entry, Swatch};
use super::palette::{self, Unit};
use super::{RenderError, RenderInput, fonts::fonts, relief};
use crate::stages::terrain::rocks::{self, Rock};

/// The brightness of the hillshade over the rock colour: `AMBIENT + SPAN · hillshade` (0.3.0).
const AMBIENT: f64 = 0.45;
/// See [`AMBIENT`].
const SPAN: f64 = 0.65;

/// Draws `geology`.
pub(crate) fn geology(input: &RenderInput<'_>) -> Result<Pixmap, RenderError> {
    let rock = input.u8("rock")?;
    let colours = rock.map(|code| palette::unit(palette::rock(code)));
    let mut pixmap = relief::paint(input, |y, x, shade| {
        let c: Unit = colours.get(y / PX_PER_CELL, x / PX_PER_CELL);
        let k = AMBIENT + SPAN * shade;
        palette::rgb(palette::scale(c, k))
    })?;
    relief::water(&mut pixmap, input)?;
    let entries: Vec<Entry> = Rock::ALL
        .into_iter()
        .filter(|r| rock.as_slice().contains(&r.code()))
        .map(|r| Entry::new(Swatch::Patch(rocks::props(r).colour), r.name()))
        .collect();
    legend::draw(&mut pixmap, &fonts()?.inter, &entries)?;
    Ok(pixmap)
}
