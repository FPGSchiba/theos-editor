//! The legend of a render: a panel of entries, each a swatch and a label, in the lower right
//! corner. One builder serves `truth`, `geology` and `resources`.

use tiny_skia::{Path, PathBuilder, Pixmap, Rect};

use super::RenderError;
use super::canvas::{self, LineStyle};
use super::fonts::Face;
use super::marks::Mark;
use super::palette::Rgb;
use super::styles::INK;
use super::{feats, text};
use crate::features::FeatureSet;
use crate::kernels::num::{max, to_f32, to_f64, to_index};

/// The panel's colour.
const PANEL: Rgb = [0xff, 0xf8, 0xe6];
/// The panel's border colour.
const BORDER: Rgb = [0x3a, 0x2a, 0x14];
/// The height of a row, pixels.
const ROW_PX: f64 = 15.0;
/// The label size, pixels.
const LABEL_PX: f32 = 11.0;
/// The space around the panel's content and between swatch and label, pixels.
const PAD_PX: f64 = 6.0;
/// The width of the swatch column, pixels.
const SWATCH_PX: f64 = 22.0;
/// The panel's distance from the canvas edge, pixels.
const MARGIN_PX: f64 = 6.0;
/// The panel takes at most this share of the canvas height.
const MAX_HEIGHT_SHARE: f64 = 0.5;
/// The largest a mark grows in a swatch, pixels.
const MARK_MAX_PX: u8 = 10;

/// What an entry shows next to its label.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Swatch {
    /// A mark of a feature.
    Mark(Mark),
    /// A line, as a road or a river.
    Line(LineStyle),
    /// A coloured rectangle, as a rock or the lake.
    Patch(Rgb),
    /// A ring, as the exceptional tier.
    Ring(Rgb),
}

/// One row of the legend.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    /// The swatch.
    pub swatch: Swatch,
    /// The label.
    pub label: String,
}

impl Entry {
    /// An entry.
    pub(crate) fn new(swatch: Swatch, label: impl Into<String>) -> Self {
        Self {
            swatch,
            label: label.into(),
        }
    }
}

/// A rectangle as a path.
fn rect(left: f64, top: f64, width: f64, height: f64) -> Option<Path> {
    let bounds = Rect::from_xywh(to_f32(left), to_f32(top), to_f32(width), to_f32(height))?;
    Some(PathBuilder::from_rect(bounds))
}

/// Draws one swatch centred on `at`.
fn draw_swatch(pixmap: &mut Pixmap, swatch: &Swatch, at: [f64; 2]) {
    match swatch {
        Swatch::Mark(mark) => mark.sized(mark.size.min(MARK_MAX_PX)).draw(pixmap, at),
        Swatch::Line(style) => {
            let half = SWATCH_PX / 2.0 - 2.0;
            let points = [[at[0] - half, at[1]], [at[0] + half, at[1]]];
            canvas::polyline(pixmap, &points, style);
        }
        Swatch::Patch(colour) => {
            if let Some(path) = rect(at[0] - 6.0, at[1] - 5.0, 12.0, 10.0) {
                canvas::fill(pixmap, &path, *colour);
                canvas::outline(pixmap, &path, 0.8, BORDER);
            }
        }
        Swatch::Ring(colour) => canvas::ring(pixmap, at, 5.0, 1.6, *colour),
    }
}

/// One entry per type of `kind` among `features`, in type order: the mark `mark` gives the type,
/// and the type's name with spaces for hyphens.
pub(crate) fn marks_of(
    features: &FeatureSet,
    kind: &str,
    mark: impl Fn(&str) -> Mark,
) -> Vec<Entry> {
    feats::types_present(features, kind)
        .into_iter()
        .map(|ty| Entry::new(Swatch::Mark(mark(&ty)), ty.replace('-', " ")))
        .collect()
}

/// The entries split into columns of as many rows as fit under the height limit.
fn columns(entries: &[Entry], canvas_height: f64) -> Vec<&[Entry]> {
    let fit = to_index(
        (canvas_height * MAX_HEIGHT_SHARE - 2.0 * PAD_PX) / ROW_PX,
        usize::MAX,
    );
    entries.chunks(fit.max(1)).collect()
}

/// Draws the legend of `entries` in the lower right corner of `pixmap`; nothing for no entries.
///
/// # Errors
/// [`RenderError::Font`] when a glyph cannot be drawn.
pub(crate) fn draw(pixmap: &mut Pixmap, face: &Face, entries: &[Entry]) -> Result<(), RenderError> {
    if entries.is_empty() {
        return Ok(());
    }
    let canvas_w = f64::from(pixmap.width());
    let canvas_h = f64::from(pixmap.height());
    let cols = columns(entries, canvas_h);
    let widths: Vec<f64> = cols
        .iter()
        .map(|col| {
            let label = col.iter().fold(0.0, |widest, e| {
                max(widest, f64::from(text::width(face, &e.label, LABEL_PX)))
            });
            SWATCH_PX + PAD_PX + label + PAD_PX
        })
        .collect();
    let rows = cols.first().map_or(0, |c| c.len());
    let panel_w = widths.iter().sum::<f64>() + PAD_PX;
    let panel_h = to_f64(rows) * ROW_PX + 2.0 * PAD_PX;
    let x0 = canvas_w - MARGIN_PX - panel_w;
    let y0 = canvas_h - MARGIN_PX - panel_h;
    if let Some(path) = rect(x0, y0, panel_w, panel_h) {
        canvas::fill_alpha(pixmap, &path, PANEL, 230);
        canvas::outline(pixmap, &path, 1.0, BORDER);
    }
    let mut x = x0 + PAD_PX;
    for (col, width) in cols.iter().zip(&widths) {
        for (i, entry) in col.iter().enumerate() {
            let cy = y0 + PAD_PX + (to_f64(i) + 0.5) * ROW_PX;
            draw_swatch(pixmap, &entry.swatch, [x + SWATCH_PX / 2.0, cy]);
            let origin = [to_f32(x + SWATCH_PX + PAD_PX), to_f32(cy + 4.0)];
            text::draw(pixmap, face, &entry.label, LABEL_PX, origin, INK)?;
        }
        x += width;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::fonts::fonts;
    use crate::render::marks::Shape;
    use crate::render::test_support::decode;

    #[test]
    fn a_legend_fills_the_lower_right_and_splits_into_columns_when_tall() {
        let face = &fonts().unwrap().inter;
        let entries: Vec<Entry> = (0..9)
            .map(|i| Entry::new(Swatch::Patch([200, 20, 20]), format!("entry {i}")))
            .collect();
        let mut pixmap = Pixmap::new(200, 100).unwrap();
        pixmap.fill(tiny_skia::Color::WHITE);
        draw(&mut pixmap, face, &entries).unwrap();
        let image = decode(&canvas::encode_png(&pixmap).unwrap());
        assert_eq!(image.rgb(2, 2), [255; 3], "the upper left stays empty");
        assert_ne!(
            image.rgb(193, 93),
            [255; 3],
            "the panel reaches the lower right corner"
        );
        assert_eq!(columns(&entries, 100.0).len(), 5);
        assert_eq!(columns(&entries, 400.0).len(), 1);
        let mut empty = Pixmap::new(10, 10).unwrap();
        draw(&mut empty, face, &[]).unwrap();
        assert!(empty.data().iter().all(|&b| b == 0));
    }

    #[test]
    fn every_swatch_kind_draws() {
        let mark = Mark {
            shape: Shape::Circle,
            size: 8,
            face: [10, 200, 10],
            edge: [0, 0, 0],
        };
        let line = LineStyle {
            width_px: 2.0,
            colour: [1, 2, 3],
            dash: Some([4.0, 2.0]),
        };
        let swatches = [
            Swatch::Mark(mark),
            Swatch::Line(line),
            Swatch::Patch([9, 9, 9]),
            Swatch::Ring([250, 210, 77]),
        ];
        for swatch in swatches {
            let mut pixmap = Pixmap::new(30, 30).unwrap();
            draw_swatch(&mut pixmap, &swatch, [15.0, 15.0]);
            assert!(pixmap.data().iter().any(|&b| b != 0), "{swatch:?}");
        }
    }
}
