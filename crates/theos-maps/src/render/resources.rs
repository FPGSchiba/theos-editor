//! The `resources` render (spec §5.11): grey relief, deposits as circles by type sized by their
//! radius, a ring for the exceptional tier, and a legend. Roads and settlements are drawn
//! small and grey so the deposits stand out. Frame-exact.

use tiny_skia::{PathBuilder, Pixmap};

use super::canvas::{self, LineStyle};
use super::legend::{self, Entry, Swatch};
use super::marks::{Mark, Shape};
use super::palette::Rgb;
use super::roads;
use super::styles::{RoadStyle, resource_colour};
use super::view::View;
use super::{RenderError, RenderInput, feats, fonts::fonts, relief};
use crate::kernels::num::{max, to_f32, to_u8_unit};
use crate::stages::terrain::roads::RoadClass;

/// The grey of the relief: `BASE + SPAN · hillshade` (0.3.0).
const BASE: f64 = 0.35;
/// See [`BASE`].
const SPAN: f64 = 0.55;
/// The ink of roads on the grey relief.
const ROAD_INK: Rgb = [0x55, 0x55, 0x55];
/// The smallest radius of a deposit's circle, pixels.
const MIN_RADIUS_PX: f64 = 3.0;
/// How far the exceptional ring lies outside the circle, pixels.
const RING_GAP_PX: f64 = 2.5;
/// The opacity of a deposit's circle.
const CIRCLE_ALPHA: u8 = 217;
/// The gold of the exceptional ring.
const GOLD: Rgb = [0xff, 0xd3, 0x4d];
/// The tier that gets a ring.
const EXCEPTIONAL: &str = "exceptional";

/// Draws `resources`.
pub(crate) fn resources(input: &RenderInput<'_>) -> Result<Pixmap, RenderError> {
    let mut pixmap = relief::paint(input, |_, _, shade| {
        let v = to_u8_unit(BASE + SPAN * shade);
        [v; 3]
    })?;
    relief::water(&mut pixmap, input)?;
    let view = View::new(input.world.frame(), [0.0, 0.0]);
    let ink = RoadStyle {
        line: LineStyle {
            width_px: 1.2,
            colour: ROAD_INK,
            dash: None,
        },
        edge: None,
    };
    let plan: Vec<(RoadClass, RoadStyle)> = RoadClass::ALL.into_iter().map(|c| (c, ink)).collect();
    roads::draw(&mut pixmap, input, &view, &plan, &|_| true)?;
    let small = Mark {
        shape: Shape::Square,
        size: 5,
        face: [255; 3],
        edge: [0; 3],
    };
    for (_, at) in feats::points(input.features, "settlement") {
        input.check_cancel()?;
        small.draw(&mut pixmap, view.px(at));
    }
    let mut exceptional = false;
    for (f, at) in feats::points(input.features, "resource") {
        input.check_cancel()?;
        let radius = max(
            view.len_px(feats::number(f, "radius_m").unwrap_or(0.0)),
            MIN_RADIUS_PX,
        );
        let [x, y] = view.px(at);
        let colour = resource_colour(&f.ty);
        if let Some(path) = PathBuilder::from_circle(to_f32(x), to_f32(y), to_f32(radius)) {
            canvas::fill_alpha(&mut pixmap, &path, colour, CIRCLE_ALPHA);
            canvas::outline(&mut pixmap, &path, 0.8, [0; 3]);
        }
        if feats::text(f, "tier") == Some(EXCEPTIONAL) {
            exceptional = true;
            let ring = to_f32(radius + RING_GAP_PX);
            if let Some(path) = PathBuilder::from_circle(to_f32(x), to_f32(y), ring) {
                canvas::outline(&mut pixmap, &path, 1.4, GOLD);
            }
        }
    }
    let mut entries: Vec<Entry> = feats::types_present(input.features, "resource")
        .into_iter()
        .map(|ty| {
            let mark = Mark {
                shape: Shape::Circle,
                size: 9,
                face: resource_colour(&ty),
                edge: [0; 3],
            };
            Entry::new(Swatch::Mark(mark), ty.replace('-', " "))
        })
        .collect();
    if exceptional {
        entries.push(Entry::new(Swatch::Ring(GOLD), "exceptional tier"));
    }
    legend::draw(&mut pixmap, &fonts()?.inter, &entries)?;
    Ok(pixmap)
}
