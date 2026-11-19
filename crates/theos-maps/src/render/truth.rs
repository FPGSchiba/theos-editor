//! The `truth` render (spec §5.11): `base` with margins, plus roads by class, every feature
//! kind as a plain shape per type, settlement names, a legend of what is present, a scale bar
//! and the title (`RenderInput::title`). Texts are Inter; the names are the generated names
//! of the features, the title and the legend are numbers, units and type names.

use tiny_skia::{Color, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};

use super::canvas::{self, LineStyle};
use super::fonts::{Face, fonts};
use super::legend::{self, Entry, Swatch};
use super::marks::{Mark, Shape};
use super::palette::{self, Rgb};
use super::styles::{self, INK, road_style};
use super::view::View;
use super::{RenderError, RenderInput, base, feats, roads, text};
use crate::kernels::num::to_f32;
use crate::stage::LayerSpec;
use crate::stages::terrain::hydrology::WaterFeatures;
use crate::stages::terrain::roads::RoadClass;

/// The margin above the map, for the title, pixels.
pub(crate) const TRUTH_TOP_PX: u32 = 30;
/// The margin below the map, for the scale bar, pixels.
pub(crate) const TRUTH_BOTTOM_PX: u32 = 26;

/// The colour of the margins.
pub(crate) const MARGIN: Rgb = [0xf4, 0xea, 0xd2];
/// The largest title size, pixels.
const TITLE_PX: f32 = 15.0;
/// The gap between the canvas edge and the title or the scale bar, pixels.
const EDGE_PX: f32 = 8.0;
/// A settlement name's size by class, pixels: larger for towns and cities, smallest for hamlets.
fn name_px(ty: &str) -> f32 {
    match ty {
        "city" | "town" => 13.0,
        "village" => 11.0,
        _ => 9.0,
    }
}
/// The opacity of the box behind a name.
const NAME_BOX_ALPHA: u8 = 150;
/// The colour of the box behind a name.
const NAME_BOX: Rgb = [0xff, 0xf8, 0xe6];
/// The lengths a scale bar may have, metres.
const SCALE_STEPS_M: [u32; 10] = [
    100, 200, 500, 1_000, 2_000, 5_000, 10_000, 20_000, 50_000, 100_000,
];
/// The scale bar takes at most this share of the canvas width.
const SCALE_MAX_SHARE: f64 = 0.3;

/// The layers `truth` reads (ledger ruling P3): those of `base`, which it draws first.
pub(super) const NEEDS: &[LayerSpec] = base::NEEDS;

/// Draws `truth`.
pub(crate) fn truth(input: &RenderInput<'_>) -> Result<Pixmap, RenderError> {
    let ground = base::base(input)?;
    let size = RenderError::Size {
        rows: usize::try_from(ground.height()).unwrap_or(0),
        cols: usize::try_from(ground.width()).unwrap_or(0),
    };
    let mut pixmap = Pixmap::new(
        ground.width(),
        ground.height() + TRUTH_TOP_PX + TRUTH_BOTTOM_PX,
    )
    .ok_or(size)?;
    pixmap.fill(Color::from_rgba8(MARGIN[0], MARGIN[1], MARGIN[2], u8::MAX));
    let top = i32::try_from(TRUTH_TOP_PX).unwrap_or(0);
    pixmap.draw_pixmap(
        0,
        top,
        ground.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    let view = View::new(input.world.frame(), [0.0, f64::from(TRUTH_TOP_PX)]);
    let face = &fonts()?.inter;
    let plan: Vec<_> = RoadClass::ALL
        .into_iter()
        .map(|c| (c, road_style(c)))
        .collect();
    roads::draw(&mut pixmap, input, &view, &plan, &|_| true)?;
    draw_features(&mut pixmap, input, &view, face)?;
    draw_title(&mut pixmap, face, &input.title)?;
    draw_scale_bar(&mut pixmap, face, &view)?;
    legend::draw(&mut pixmap, face, &entries(input))?;
    Ok(pixmap)
}

/// Draws the features: sites, deposits, settlements with their names, then points of interest.
fn draw_features(
    pixmap: &mut Pixmap,
    input: &RenderInput<'_>,
    view: &View,
    face: &Face,
) -> Result<(), RenderError> {
    let features = input.features;
    for (f, at) in feats::points(features, "site") {
        input.check_cancel()?;
        styles::site_mark(&f.ty).draw(pixmap, view.px(at));
    }
    for (f, at) in feats::points(features, "resource") {
        input.check_cancel()?;
        let mark = Mark {
            shape: Shape::Circle,
            size: 5,
            face: styles::resource_colour(&f.ty),
            edge: [0; 3],
        };
        mark.draw(pixmap, view.px(at));
    }
    for (f, at) in feats::points(features, "settlement") {
        input.check_cancel()?;
        let mark = styles::settlement_mark(&f.ty);
        let [x, y] = view.px(at);
        if f.ty == "city" {
            let radius = f64::from(mark.size) / 2.0 + 4.0;
            let centre = (to_f32(x), to_f32(y));
            if let Some(ring) = PathBuilder::from_circle(centre.0, centre.1, to_f32(radius)) {
                canvas::outline(pixmap, &ring, 1.5, styles::SETTLEMENT_EDGE);
            }
        }
        mark.draw(pixmap, [x, y]);
        if let Some(name) = feats::text(f, "name") {
            let mark_px = f64::from(mark.size);
            draw_name(pixmap, face, name, name_px(&f.ty), [x, y], mark_px)?;
        }
    }
    for (f, at) in feats::points(features, "poi") {
        input.check_cancel()?;
        styles::poi_mark(&f.ty).draw(pixmap, view.px(at));
    }
    Ok(())
}

/// Draws a settlement's `name` up and to the right of its mark, on a translucent box.
fn draw_name(
    pixmap: &mut Pixmap,
    face: &Face,
    name: &str,
    size: f32,
    at: [f64; 2],
    mark_px: f64,
) -> Result<(), RenderError> {
    let origin = [to_f32(at[0] + mark_px * 0.9), to_f32(at[1] - mark_px * 0.9)];
    let width = text::width(face, name, size);
    let pad = 2.0;
    if let Some(rect) = Rect::from_xywh(
        origin[0] - pad,
        origin[1] - size * 0.8 - pad,
        width + 2.0 * pad,
        size + 2.0 * pad,
    ) {
        let path = PathBuilder::from_rect(rect);
        canvas::fill_alpha(pixmap, &path, NAME_BOX, NAME_BOX_ALPHA);
    }
    text::draw(pixmap, face, name, size, origin, INK)
}

/// Draws the title in the top margin, as large as fits.
fn draw_title(pixmap: &mut Pixmap, face: &Face, title: &str) -> Result<(), RenderError> {
    let room = to_f32(f64::from(pixmap.width())) - 2.0 * EDGE_PX;
    let size = text::fit(face, title, TITLE_PX, room);
    let baseline = to_f32(f64::from(TRUTH_TOP_PX)) * 0.65;
    text::draw(pixmap, face, title, size, [EDGE_PX, baseline], INK)
}

/// The length and label of the longest scale bar that fits.
fn scale(view: &View, canvas_w: f64) -> (u32, String) {
    let fits = |m: &&u32| view.len_px(f64::from(**m)) <= canvas_w * SCALE_MAX_SHARE;
    let metres = SCALE_STEPS_M
        .iter()
        .rev()
        .find(fits)
        .copied()
        .unwrap_or(SCALE_STEPS_M[0]);
    let label = if metres >= 1_000 {
        format!("{} km", metres / 1_000)
    } else {
        format!("{metres} m")
    };
    (metres, label)
}

/// Draws the scale bar in the bottom margin.
fn draw_scale_bar(pixmap: &mut Pixmap, face: &Face, view: &View) -> Result<(), RenderError> {
    let (metres, label) = scale(view, f64::from(pixmap.width()));
    let y = f64::from(pixmap.height()) - 13.0;
    let x0 = f64::from(EDGE_PX);
    let x1 = x0 + view.len_px(f64::from(metres));
    let bar = LineStyle {
        width_px: 3.0,
        colour: INK,
        dash: None,
    };
    canvas::polyline(pixmap, &[[x0, y], [x1, y]], &bar);
    let tick = LineStyle {
        width_px: 1.5,
        ..bar
    };
    for x in [x0, x1] {
        canvas::polyline(pixmap, &[[x, y - 5.0], [x, y + 5.0]], &tick);
    }
    let origin = [to_f32(x1 + 8.0), to_f32(y + 4.0)];
    text::draw(pixmap, face, &label, 11.0, origin, INK)
}

/// The legend: roads, settlement classes, points of interest, sites, deposits and water that
/// the map has.
fn entries(input: &RenderInput<'_>) -> Vec<Entry> {
    let mut out = Vec::new();
    for class in roads::classes_present(input).into_iter().rev() {
        let style = road_style(class);
        let colour = style.edge.unwrap_or(style.line.colour);
        let line = LineStyle {
            colour,
            ..style.line
        };
        let label = match class {
            RoadClass::Main => "main road",
            other => other.name(),
        };
        out.push(Entry::new(Swatch::Line(line), label));
    }
    let features = input.features;
    for ty in ["city", "town", "village", "hamlet"] {
        if features.count_type(ty) > 0 {
            out.push(Entry::new(Swatch::Mark(styles::settlement_mark(ty)), ty));
        }
    }
    for ty in feats::types_present(features, "poi") {
        out.push(Entry::new(
            Swatch::Mark(styles::poi_mark(&ty)),
            ty.replace('-', " "),
        ));
    }
    for ty in feats::types_present(features, "site") {
        out.push(Entry::new(
            Swatch::Mark(styles::site_mark(&ty)),
            ty.replace('-', " "),
        ));
    }
    for ty in feats::types_present(features, "resource") {
        let mark = Mark {
            shape: Shape::Circle,
            size: 5,
            face: styles::resource_colour(&ty),
            edge: [0; 3],
        };
        out.push(Entry::new(Swatch::Mark(mark), ty.replace('-', " ")));
    }
    let has_rivers = input
        .value::<WaterFeatures>("water_features")
        .is_ok_and(|w| !w.rivers.is_empty());
    if has_rivers {
        let line = LineStyle {
            width_px: 2.0,
            colour: palette::RIVER,
            dash: None,
        };
        out.push(Entry::new(Swatch::Line(line), "river"));
    }
    if input
        .mask("lake")
        .is_ok_and(|l| l.as_slice().iter().any(|&b| b))
    {
        out.push(Entry::new(Swatch::Patch(palette::LAKE), "lake"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::CancelToken;
    use crate::render::test_support::TestWorld;
    use crate::world::MapFrame;

    #[test]
    fn the_scale_bar_is_the_longest_step_that_fits() {
        let frame = MapFrame {
            rows: 10,
            cols: 10,
            cell_m: 50.0,
        };
        let view = View::new(&frame, [0.0, 0.0]);
        // 25 m per pixel: 1000 px wide, at most 300 px = 7500 m.
        assert_eq!(scale(&view, 1000.0), (5_000, "5 km".to_owned()));
        assert_eq!(scale(&view, 100.0), (500, "500 m".to_owned()));
        assert_eq!(scale(&view, 1.0), (100, "100 m".to_owned()));
    }

    #[test]
    fn the_legend_lists_water_only_when_the_map_has_it() {
        let labels = |w: &TestWorld| {
            let cancel = CancelToken::new();
            let found = entries(&w.input(&cancel));
            found.into_iter().map(|e| e.label).collect::<Vec<_>>()
        };
        let mut w = TestWorld::flat(10, 10, 50.0);
        assert_eq!(labels(&w), Vec::<String>::new());
        w.river(&[[0.0, 100.0], [400.0, 100.0]], 20.0);
        assert_eq!(labels(&w), ["river"]);
        w.lake(2..4, 2..4);
        assert_eq!(labels(&w), ["river", "lake"]);
    }
}
