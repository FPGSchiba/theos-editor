//! The `blueprint` render (spec §5.11) and its extra `blueprint-ruin`: `base` in a theme's
//! colours with margins, plus room labels, doors, key rooms, exits, the critical route, a scale
//! bar, a legend and a title with the recipe, the seed and the critical path.

use std::collections::BTreeSet;

use tiny_skia::{Pixmap, PixmapPaint, Transform};

use super::cells::{self, Masks};
use super::doors::{self, Present};
use super::geo::{self, Cell2, Placement};
use super::skin::{BLUEPRINT, RUIN, Theme};
use super::{canvas_with_margins, critical_path_title};
use crate::kernels::num::to_f32;
use crate::render::canvas::{self, LineStyle};
use crate::render::fonts::{Face, fonts};
use crate::render::legend::{self, Entry, Swatch};
use crate::render::marks::{Mark, Shape};
use crate::render::truth::{TRUTH_BOTTOM_PX, TRUTH_TOP_PX, draw_scale_bar, draw_title};
use crate::render::view::View;
use crate::render::{RenderError, RenderInput, text};
use crate::rooms::lattice::Lattice;
use crate::rooms::{Role, RoomGraph};
use crate::stage::LayerSpec;
use crate::stages::interior::playability_core::Analysis;
use crate::stages::interior::space_core::Doors;
use crate::world::ValueType;

/// The scale bar lengths an interior may have, metres.
const SCALE_STEPS_M: [u32; 9] = [1, 2, 5, 10, 20, 50, 100, 200, 500];
/// The size of a room label, pixels.
const LABEL_PX: f32 = 9.0;
/// The smallest room label, pixels; a label that needs less is left out.
const LABEL_MIN_PX: f32 = 5.0;
/// A label may be this much wider than a lattice cell.
const LABEL_OVERHANG: f64 = 1.5;
/// The width of the critical route, pixels.
const ROUTE_PX: f64 = 2.0;
/// The dash of the critical route, pixels.
const ROUTE_DASH: [f32; 2] = [6.0, 4.0];
/// The radius of an exit ring and a key mark, pixels.
const SYMBOL_PX: f64 = 3.5;
/// The gap between symbols above a label, pixels.
const SYMBOL_GAP_PX: f64 = 10.0;

/// The layers `blueprint` reads (ledger ruling P3): `base`'s, the rooms, doors and the analysis.
pub(super) const NEEDS: &[LayerSpec] = &[
    LayerSpec::mask("walkable"),
    LayerSpec::mask("walls"),
    LayerSpec::mask("cover"),
    LayerSpec::mask("envelope"),
    LayerSpec::value("room_graph", <RoomGraph as ValueType>::NAME),
    LayerSpec::value("lattice", <Lattice as ValueType>::NAME),
    LayerSpec::value("doors", <Doors as ValueType>::NAME),
    LayerSpec::value("analysis", <Analysis as ValueType>::NAME),
];

/// Draws `blueprint`.
pub(crate) fn blueprint(input: &RenderInput<'_>) -> Result<Pixmap, RenderError> {
    draw(input, &BLUEPRINT)
}

/// Draws `blueprint-ruin`.
pub(crate) fn blueprint_ruin(input: &RenderInput<'_>) -> Result<Pixmap, RenderError> {
    draw(input, &RUIN)
}

/// The blueprint of `input` in `theme`.
fn draw(input: &RenderInput<'_>, theme: &Theme) -> Result<Pixmap, RenderError> {
    input.check_cancel()?;
    let masks = Masks::read(input)?;
    let graph = input.value::<RoomGraph>("room_graph")?;
    let lattice = input.value::<Lattice>("lattice")?;
    let doors = input.value::<Doors>("doors")?;
    let analysis = input.value::<Analysis>("analysis")?;
    let ground = cells::paint(input, &masks, &theme.ground)?;
    let mut pixmap = canvas_with_margins(&ground, TRUTH_TOP_PX, TRUTH_BOTTOM_PX)?;
    pixmap.draw_pixmap(
        0,
        i32::try_from(TRUTH_TOP_PX).unwrap_or(0),
        ground.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    let place = Placement {
        offset: [0.0, f64::from(TRUTH_TOP_PX)],
    };
    let face = &fonts()?.inter;
    let centres = geo::centres(graph, lattice);
    let route = route_line(&mut pixmap, analysis, place, theme);
    let present = doors::draw(
        &mut pixmap,
        input,
        place,
        face,
        &theme.ink,
        (doors, &centres),
    )?;
    let marked = rooms(
        &mut pixmap,
        input,
        face,
        (graph, lattice, &centres),
        place,
        theme,
    )?;
    let view = View::new(input.world.frame(), place.offset);
    draw_title(&mut pixmap, face, &critical_path_title(input, analysis))?;
    draw_scale_bar(&mut pixmap, face, &view, &SCALE_STEPS_M)?;
    let items = [
        (route, Item::Route),
        (marked.0, Item::Key),
        (marked.1, Item::Exit),
        (has_cover(input), Item::Cover),
    ];
    let found = Found {
        items: items
            .into_iter()
            .filter(|(on, _)| *on)
            .map(|(_, i)| i)
            .collect(),
        doors: present,
    };
    legend::draw(&mut pixmap, face, &entries(theme, &found))?;
    Ok(pixmap)
}

/// Whether the map has any cover cell.
fn has_cover(input: &RenderInput<'_>) -> bool {
    input
        .mask("cover")
        .is_ok_and(|m| m.as_slice().iter().any(|&b| b))
}

/// Draws the critical route as a dashed line; whether there is one.
fn route_line(pixmap: &mut Pixmap, analysis: &Analysis, place: Placement, theme: &Theme) -> bool {
    let points: Vec<[f64; 2]> = analysis
        .route_points
        .iter()
        .map(|&(r, c)| place.px([f64::from(r) + 0.5, f64::from(c) + 0.5]))
        .collect();
    let style = LineStyle {
        width_px: ROUTE_PX,
        colour: theme.ink.route,
        dash: Some(ROUTE_DASH),
    };
    canvas::polyline(pixmap, &points, &style);
    points.len() > 1
}

/// Draws the room labels, key marks and exit rings; whether there were keys and exits.
fn rooms(
    pixmap: &mut Pixmap,
    input: &RenderInput<'_>,
    face: &Face,
    parts: (&RoomGraph, &Lattice, &[Option<Cell2>]),
    place: Placement,
    theme: &Theme,
) -> Result<(bool, bool), RenderError> {
    let (graph, lattice, centres) = parts;
    let (mut keys, mut exits) = (false, false);
    for (room, centre) in graph.rooms.iter().zip(centres) {
        input.check_cancel()?;
        let Some(centre) = centre else { continue };
        let at = place.px(*centre);
        let width = f64::from(lattice.pitch) * 2.0 * LABEL_OVERHANG;
        let size = text::fit(face, &room.ty, LABEL_PX, to_f32(width));
        if size >= LABEL_MIN_PX {
            geo::centred_text(pixmap, face, &room.ty, size, at, theme.ink.label)?;
        }
        let has_key = !room.keys.is_empty();
        let has_exit = room.role == Role::Exit;
        keys |= has_key;
        exits |= has_exit;
        let n = f64::from(u8::from(has_key) + u8::from(has_exit));
        let mut x = at[0] - (n - 1.0) * SYMBOL_GAP_PX / 2.0;
        let y = at[1] - SYMBOL_GAP_PX;
        if has_exit {
            canvas::ring(pixmap, [x, y], SYMBOL_PX, 1.5, theme.ink.exit);
            x += SYMBOL_GAP_PX;
        }
        if has_key {
            geo::dot(pixmap, [x, y], SYMBOL_PX - 0.5, theme.ink.key);
        }
    }
    Ok((keys, exits))
}

/// A kind of symbol or patch that may be present, for the legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Item {
    /// The critical route.
    Route,
    /// A key mark.
    Key,
    /// An exit ring.
    Exit,
    /// Cover cells.
    Cover,
}

/// What the map has, for its legend.
struct Found {
    items: BTreeSet<Item>,
    doors: Present,
}

/// The legend: what is present on the map.
fn entries(theme: &Theme, found: &Found) -> Vec<Entry> {
    let line = |colour, dash| {
        Swatch::Line(LineStyle {
            width_px: 2.5,
            colour,
            dash,
        })
    };
    let ink = &theme.ink;
    let mut out = vec![
        Entry::new(Swatch::Patch(theme.ground.floor), "floor"),
        Entry::new(Swatch::Patch(theme.ground.wall), "wall"),
    ];
    if found.items.contains(&Item::Cover) {
        out.push(Entry::new(Swatch::Patch(theme.ground.cover), "cover"));
    }
    if found.doors.open {
        out.push(Entry::new(line(ink.door, None), "open door"));
    }
    if found.doors.locked {
        out.push(Entry::new(line(ink.locked, None), "locked door"));
    }
    if found.doors.drop {
        out.push(Entry::new(line(ink.drop, None), "drop"));
    }
    if found.items.contains(&Item::Key) {
        let mark = Mark {
            shape: Shape::Circle,
            size: 6,
            face: ink.key,
            edge: ink.key,
        };
        out.push(Entry::new(Swatch::Mark(mark), "key"));
    }
    if found.items.contains(&Item::Exit) {
        out.push(Entry::new(Swatch::Ring(ink.exit), "exit"));
    }
    if found.items.contains(&Item::Route) {
        out.push(Entry::new(
            line(ink.route, Some(ROUTE_DASH)),
            "critical route",
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legend_lists_only_what_is_present() {
        let none = Found {
            items: BTreeSet::new(),
            doors: Present::default(),
        };
        let labels = |f: &Found| {
            entries(&BLUEPRINT, f)
                .into_iter()
                .map(|e| e.label)
                .collect::<Vec<_>>()
        };
        assert_eq!(labels(&none), ["floor", "wall"]);
        let some = Found {
            items: BTreeSet::from([Item::Route, Item::Key]),
            ..none
        };
        assert_eq!(labels(&some), ["floor", "wall", "key", "critical route"]);
    }
}
