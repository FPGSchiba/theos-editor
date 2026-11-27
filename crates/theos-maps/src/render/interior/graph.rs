//! The interior `graph` render (spec §5.11): the room graph at its lattice positions, nodes by
//! role with halls larger, locked connections red with the key, drops as arrows and the
//! critical route thick.

use tiny_skia::{Color, Pixmap};

use super::geo::{self, Cell2, Placement};
use super::skin::{self, BLUEPRINT, GRAPH_BACKGROUND, NODE_EDGE, ROLES};
use super::{canvas_with_margins, critical_path_title};
use crate::kernels::num::to_f32;
use crate::render::canvas::{self, LineStyle};
use crate::render::fonts::{Face, fonts};
use crate::render::legend::{self, Entry, Swatch};
use crate::render::truth::{MARGIN, TRUTH_TOP_PX, draw_title};
use crate::render::{RenderError, RenderInput, text};
use crate::rooms::lattice::Lattice;
use crate::rooms::{Passage, RoomGraph};
use crate::stage::LayerSpec;
use crate::stages::interior::playability_core::Analysis;
use crate::world::ValueType;

/// The radius of a node of a single-cell room, pixels.
const NODE_PX: f64 = 4.0;
/// How much a node grows per extra lattice cell of its room, pixels.
const HALL_GROWTH_PX: f64 = 2.0;
/// The largest node radius, pixels.
const NODE_MAX_PX: f64 = 14.0;
/// The width of a connection, pixels.
const EDGE_PX: f64 = 1.5;
/// The width of the critical route, pixels.
const ROUTE_PX: f64 = 5.0;
/// The size of a key name, pixels.
const KEY_NAME_PX: f32 = 8.0;

/// The layers `graph` reads (ledger ruling P3).
pub(super) const NEEDS: &[LayerSpec] = &[
    LayerSpec::value("room_graph", <RoomGraph as ValueType>::NAME),
    LayerSpec::value("lattice", <Lattice as ValueType>::NAME),
    LayerSpec::value("analysis", <Analysis as ValueType>::NAME),
];

/// The radius of the node of a room of `cells` lattice cells.
fn radius(cells: usize) -> f64 {
    let extra = f64::from(u32::try_from(cells.saturating_sub(1)).unwrap_or(u32::MAX));
    (NODE_PX + HALL_GROWTH_PX * extra).min(NODE_MAX_PX)
}

/// Draws `graph`.
pub(crate) fn graph(input: &RenderInput<'_>) -> Result<Pixmap, RenderError> {
    input.check_cancel()?;
    let rooms = input.value::<RoomGraph>("room_graph")?;
    let lattice = input.value::<Lattice>("lattice")?;
    let analysis = input.value::<Analysis>("analysis")?;
    let blank = canvas::pixmap(input.world.frame())?;
    let mut pixmap = canvas_with_margins(&blank, TRUTH_TOP_PX, 0)?;
    let [r, g, b] = GRAPH_BACKGROUND;
    pixmap.fill(Color::from_rgba8(r, g, b, u8::MAX));
    let top = f64::from(TRUTH_TOP_PX);
    margin(&mut pixmap, top);
    let place = Placement { offset: [0.0, top] };
    let face = &fonts()?.inter;
    let centres = geo::centres(rooms, lattice);
    let at = |n: u16| {
        centres
            .get(usize::from(n))
            .copied()
            .flatten()
            .map(|c| place.px(c))
    };
    draw_route(&mut pixmap, analysis, &at);
    draw_edges(&mut pixmap, input, face, rooms, &at)?;
    draw_nodes(&mut pixmap, input, rooms, &centres, place)?;
    draw_title(&mut pixmap, face, &critical_path_title(input, analysis))?;
    legend::draw(&mut pixmap, face, &entries(rooms))?;
    Ok(pixmap)
}

/// Paints the title margin.
fn margin(pixmap: &mut Pixmap, top: f64) {
    let width = to_f32(f64::from(pixmap.width()));
    if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 0.0, width, to_f32(top)) {
        let path = tiny_skia::PathBuilder::from_rect(rect);
        canvas::fill(pixmap, &path, MARGIN);
    }
}

/// Draws the critical route as a thick line under the connections.
fn draw_route(pixmap: &mut Pixmap, analysis: &Analysis, at: &impl Fn(u16) -> Option<[f64; 2]>) {
    let points: Vec<[f64; 2]> = analysis.route.iter().filter_map(|&n| at(n)).collect();
    let style = LineStyle {
        width_px: ROUTE_PX,
        colour: BLUEPRINT.ink.route,
        dash: None,
    };
    canvas::polyline(pixmap, &points, &style);
}

/// Draws the connections: open as lines, locked in red with the key, drops as arrows.
fn draw_edges(
    pixmap: &mut Pixmap,
    input: &RenderInput<'_>,
    face: &Face,
    graph: &RoomGraph,
    at: &impl Fn(u16) -> Option<[f64; 2]>,
) -> Result<(), RenderError> {
    let ink = &BLUEPRINT.ink;
    for c in &graph.connections {
        input.check_cancel()?;
        let (Some(a), Some(b)) = (at(c.a), at(c.b)) else {
            continue;
        };
        let line = |colour| LineStyle {
            width_px: EDGE_PX,
            colour,
            dash: None,
        };
        match c.kind {
            Passage::Open => canvas::polyline(pixmap, &[a, b], &line(ink.door)),
            Passage::Locked(key) => {
                canvas::polyline(pixmap, &[a, b], &line(ink.locked));
                let mid = [f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1])];
                let origin = [to_f32(mid[0]) + 3.0, to_f32(mid[1]) - 3.0];
                let name = RoomGraph::key_name(key);
                text::draw(pixmap, face, &name, KEY_NAME_PX, origin, ink.locked)?;
            }
            Passage::Drop { from } => {
                let (tail, tip) = if from == c.a { (a, b) } else { (b, a) };
                geo::arrow(pixmap, tail, tip, EDGE_PX, ink.drop);
            }
        }
    }
    Ok(())
}

/// Draws one node per room, coloured by role, sized by footprint.
fn draw_nodes(
    pixmap: &mut Pixmap,
    input: &RenderInput<'_>,
    graph: &RoomGraph,
    centres: &[Option<Cell2>],
    place: Placement,
) -> Result<(), RenderError> {
    for (room, centre) in graph.rooms.iter().zip(centres) {
        input.check_cancel()?;
        let Some(centre) = centre else { continue };
        let at = place.px(*centre);
        let r = radius(room.cells.len());
        geo::dot(pixmap, at, r + 1.0, NODE_EDGE);
        geo::dot(pixmap, at, r, skin::role_colour(room.role));
    }
    Ok(())
}

/// The legend: the roles the graph has, then the connection kinds it has.
fn entries(graph: &RoomGraph) -> Vec<Entry> {
    let ink = &BLUEPRINT.ink;
    let mut out: Vec<Entry> = ROLES
        .into_iter()
        .filter(|role| graph.rooms.iter().any(|r| r.role == *role))
        .map(|role| {
            Entry::new(
                Swatch::Patch(skin::role_colour(role)),
                skin::role_name(role),
            )
        })
        .collect();
    let line = |colour| {
        Swatch::Line(LineStyle {
            width_px: 2.5,
            colour,
            dash: None,
        })
    };
    let has = |f: fn(&Passage) -> bool| graph.connections.iter().any(|c| f(&c.kind));
    if has(|k| matches!(k, Passage::Locked(_))) {
        out.push(Entry::new(line(ink.locked), "locked"));
    }
    if has(|k| matches!(k, Passage::Drop { .. })) {
        out.push(Entry::new(line(ink.drop), "drop"));
    }
    out.push(Entry::new(line(ink.route), "critical route"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halls_are_larger_than_single_rooms_up_to_a_limit() {
        assert!(radius(4) > radius(1));
        assert!((radius(1000) - NODE_MAX_PX).abs() < f64::EPSILON);
        assert!((radius(0) - NODE_PX).abs() < f64::EPSILON);
    }
}
