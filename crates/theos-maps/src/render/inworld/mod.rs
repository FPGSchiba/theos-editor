//! The `inworld` render (spec §5.11): a hand-inked map on paper of what the maker walked: the
//! known land around settlements and main roads, the roads of class `road` and `main`,
//! settlements with their generated names, lakes, big rivers, and simple peak and tree marks.
//! Frame-exact; everything is drawn inside the frame. Spectral is the font.
//!
//! Texts come only from the recipe's `[render.inworld]` table (`options`): the title and the
//! labels with their positions. The one exception, which the spec's row asks for, is the
//! settlements' names: they are the generated names of the features (spec §5.10), not lore, and
//! are drawn from the feature list, not from code.

mod known;
mod nature;
mod options;

use tiny_skia::Pixmap;

use super::canvas::{self, LineStyle, PX_PER_CELL};
use super::fonts::{Face, fonts};
use super::marks::{Mark, Shape};
use super::palette::{self, Rgb};
use super::styles::RoadStyle;
use super::view::View;
use super::{RenderError, RenderInput, feats, relief, roads, text};
use crate::grid::Grid;
use crate::kernels::num::{max, min, to_f32};
use crate::stages::terrain::hydrology::WaterFeatures;
use crate::stages::terrain::roads::RoadClass;
use known::Known;

/// The paper inside the known area and outside it (0.3.0).
const PAPER: Rgb = [0xe8, 0xd9, 0xb5];
/// See [`PAPER`].
const PAPER_FAR: Rgb = [0xcd, 0xb9, 0x8c];
/// The ink of lines and house marks.
const INK: Rgb = [0x4a, 0x35, 0x20];
/// The darker ink of settlements and their names.
const DARK_INK: Rgb = [0x2f, 0x20, 0x10];
/// The green of tree marks.
const TREE: Rgb = [0x3f, 0x5a, 0x2e];
/// The lake, washed (0.3.0).
const LAKE: Rgb = [0x9f, 0xb4, 0xbd];
/// A big river (0.3.0).
const BIG_RIVER: Rgb = [0x3d, 0x5f, 0x7a];
/// The width of a big river, pixels.
const BIG_RIVER_PX: f64 = 2.0;
/// The opacity of peak and tree marks.
const MARK_ALPHA: u8 = 190;
/// The title's largest size, pixels.
const TITLE_PX: f32 = 20.0;
/// A label's size, pixels.
const LABEL_PX: f32 = 14.0;
/// The space between the frame's edge and the title's baseline, pixels.
const TITLE_BASELINE_PX: f64 = 28.0;

/// The size of a settlement's house mark and its name, by class, pixels.
fn house_px(ty: &str) -> (u8, f32) {
    match ty {
        "city" => (20, 16.0),
        "town" => (16, 14.0),
        "village" => (12, 12.0),
        _ => (9, 0.0),
    }
}

/// Draws `inworld`.
pub(crate) fn inworld(input: &RenderInput<'_>) -> Result<Pixmap, RenderError> {
    let texts = options::read(input.options)?;
    let known = known::compute(input)?;
    let frame = *input.world.frame();
    let mut pixmap = paper(input, &known)?;
    let view = View::new(&frame, [0.0, 0.0]);
    let seen = |p: [f64; 2]| {
        let (r, c) = frame.cell_at_m(p[0], p[1]);
        known.mask.get(r, c)
    };
    fill_known_lakes(&mut pixmap, input.mask("lake")?, &known.mask);
    draw_big_rivers(&mut pixmap, input, &view, &seen)?;
    draw_nature(&mut pixmap, input, &known, &view)?;
    let ink = |width_px, dash| RoadStyle {
        line: LineStyle {
            width_px,
            colour: INK,
            dash,
        },
        edge: None,
    };
    let plan = [
        (RoadClass::Road, ink(1.4, Some([6.0, 3.0]))),
        (RoadClass::Main, ink(2.0, Some([6.0, 3.0]))),
    ];
    roads::draw(&mut pixmap, input, &view, &plan, &seen)?;
    let face = &fonts()?.spectral;
    draw_settlements(&mut pixmap, input, &view, face)?;
    draw_texts(&mut pixmap, face, &view, &texts)?;
    Ok(pixmap)
}

/// The paper: its tone follows the known area, darker away from it.
fn paper(input: &RenderInput<'_>, known: &Known) -> Result<Pixmap, RenderError> {
    relief::paint(input, |y, x, _| {
        let soft = canvas::sample(&known.soft, y, x);
        palette::rgb(palette::mix(
            palette::unit(PAPER),
            palette::unit(PAPER_FAR),
            1.0 - soft,
        ))
    })
}

/// Fills the pixels of known lake cells.
fn fill_known_lakes(pixmap: &mut Pixmap, lake: &Grid<bool>, known: &Grid<bool>) {
    for (y, row) in canvas::rows_mut(pixmap).enumerate() {
        let r = y / PX_PER_CELL;
        for (x, px) in row.iter_mut().enumerate() {
            let c = x / PX_PER_CELL;
            if lake.get(r, c) && known.get(r, c) {
                canvas::put(px, LAKE);
            }
        }
    }
}

/// Draws the segments of rivers that lie in a big river and in the known area.
fn draw_big_rivers(
    pixmap: &mut Pixmap,
    input: &RenderInput<'_>,
    view: &View,
    seen: &dyn Fn([f64; 2]) -> bool,
) -> Result<(), RenderError> {
    let big = input.mask("big_river")?;
    let frame = input.world.frame();
    let water = input.value::<WaterFeatures>("water_features")?;
    for line in &water.rivers {
        input.check_cancel()?;
        for ends in line.points.windows(2) {
            let mid = [
                f64::midpoint(ends[0][0], ends[1][0]),
                f64::midpoint(ends[0][1], ends[1][1]),
            ];
            let (r, c) = frame.cell_at_m(mid[0], mid[1]);
            if big.get(r, c) && seen(mid) {
                let from = view.px([ends[0][0], ends[0][1]]);
                let to = view.px([ends[1][0], ends[1][1]]);
                canvas::stroke(pixmap, from, to, BIG_RIVER_PX, BIG_RIVER);
            }
        }
    }
    Ok(())
}

/// Draws peaks on the ridges and trees in the forests of the known area.
fn draw_nature(
    pixmap: &mut Pixmap,
    input: &RenderInput<'_>,
    known: &Known,
    view: &View,
) -> Result<(), RenderError> {
    let frame = input.world.frame();
    let marks = [
        (
            nature::peaks(input.f64("height")?, &known.mask, frame),
            Shape::Triangle,
            8.0,
            INK,
        ),
        (
            nature::trees(input.f64("vegetation")?, &known.mask, frame),
            Shape::Triangle,
            4.0,
            TREE,
        ),
    ];
    for (points, shape, size, colour) in marks {
        for at in points {
            input.check_cancel()?;
            let [x, y] = view.px(at);
            if let Some(path) = super::marks::path(shape, [x, y], size) {
                canvas::fill_alpha(pixmap, &path, colour, MARK_ALPHA);
            }
        }
    }
    Ok(())
}

/// Draws a house mark for every settlement and the names of all but hamlets.
fn draw_settlements(
    pixmap: &mut Pixmap,
    input: &RenderInput<'_>,
    view: &View,
    face: &Face,
) -> Result<(), RenderError> {
    for (f, at) in feats::points(input.features, "settlement") {
        input.check_cancel()?;
        let (px, name_px) = house_px(&f.ty);
        let [x, y] = view.px(at);
        let size = f64::from(px);
        let body = Mark {
            shape: Shape::Square,
            size: px * 7 / 10,
            face: DARK_INK,
            edge: DARK_INK,
        };
        let roof = Mark {
            shape: Shape::Triangle,
            size: px,
            ..body
        };
        body.draw(pixmap, [x, y + size * 0.15]);
        roof.draw(pixmap, [x, y - size * 0.3]);
        if name_px > 0.0
            && let Some(name) = feats::text(f, "name")
        {
            let origin = [x + size * 0.6, y - size * 0.4];
            draw_inside(pixmap, face, name, name_px, origin, DARK_INK)?;
        }
    }
    Ok(())
}

/// Draws the recipe's title, centred at the top, and its labels.
fn draw_texts(
    pixmap: &mut Pixmap,
    face: &Face,
    view: &View,
    texts: &options::Texts,
) -> Result<(), RenderError> {
    let frame_w = f64::from(pixmap.width());
    if let Some(title) = &texts.title {
        let size = text::fit(face, title, TITLE_PX, to_f32(frame_w - 16.0));
        let x = (frame_w - f64::from(text::width(face, title, size))) / 2.0;
        draw_inside(pixmap, face, title, size, [x, TITLE_BASELINE_PX], INK)?;
    }
    for label in &texts.labels {
        draw_inside(pixmap, face, &label.text, LABEL_PX, view.px(label.at), INK)?;
    }
    Ok(())
}

/// Draws `string` with its baseline at `origin` (pixels), moved as little as needed to lie
/// wholly inside the pixmap.
fn draw_inside(
    pixmap: &mut Pixmap,
    face: &Face,
    string: &str,
    size: f32,
    origin: [f64; 2],
    colour: Rgb,
) -> Result<(), RenderError> {
    let (w, h) = (f64::from(pixmap.width()), f64::from(pixmap.height()));
    let width = f64::from(text::width(face, string, size));
    let size_px = f64::from(size);
    let x = min(max(origin[0], 0.0), max(w - width, 0.0));
    let y = min(max(origin[1], size_px * 0.8), max(h - size_px * 0.3, 0.0));
    text::draw(pixmap, face, string, size, [to_f32(x), to_f32(y)], colour)
}
