//! Drawing the road network: lines through cell centres, by class and style.

use tiny_skia::Pixmap;

use super::canvas::{self, LineStyle};
use super::styles::{ROAD_EDGE_EXTRA_PX, RoadStyle};
use super::view::View;
use super::{RenderError, RenderInput};
use crate::kernels::num::to_usize;
use crate::stages::terrain::roads::{Road, RoadClass, RoadNetwork};
use crate::world::MapFrame;

/// The road network of the world, if it has one.
pub(crate) fn network<'a>(input: &RenderInput<'a>) -> Option<&'a RoadNetwork> {
    input.world.value::<RoadNetwork>("road_network")
}

/// The centres of the cells of `road`, in metres.
fn centres(frame: &MapFrame, road: &Road) -> Vec<[f64; 2]> {
    road.cells
        .iter()
        .map(|&cell| frame.index_centre_m(to_usize(cell)))
        .collect()
}

/// The runs of `points` that `keep` accepts, each at least two points long.
fn runs(points: &[[f64; 2]], keep: &dyn Fn([f64; 2]) -> bool) -> Vec<Vec<[f64; 2]>> {
    let mut out: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut open: Vec<[f64; 2]> = Vec::new();
    for &p in points {
        if keep(p) {
            open.push(p);
        } else if !open.is_empty() {
            out.push(std::mem::take(&mut open));
        }
    }
    if !open.is_empty() {
        out.push(open);
    }
    out.retain(|run| run.len() >= 2);
    out
}

/// Draws the roads of the classes in `plan`, in that order, each in its style, where `keep`
/// accepts the cell centres (metres): edges of a class first, then its lines. Checks the cancel
/// token before every road.
pub(crate) fn draw(
    pixmap: &mut Pixmap,
    input: &RenderInput<'_>,
    view: &View,
    plan: &[(RoadClass, RoadStyle)],
    keep: &dyn Fn([f64; 2]) -> bool,
) -> Result<(), RenderError> {
    let Some(network) = network(input) else {
        return Ok(());
    };
    let frame = input.world.frame();
    for (class, style) in plan {
        let lines: Vec<Vec<[f64; 2]>> = {
            let mut lines = Vec::new();
            for road in network.roads.iter().filter(|r| r.class == *class) {
                input.check_cancel()?;
                for run in runs(&centres(frame, road), keep) {
                    lines.push(run.iter().map(|&p| view.px(p)).collect());
                }
            }
            lines
        };
        if let Some(edge) = style.edge {
            let wide = LineStyle {
                width_px: style.line.width_px + ROAD_EDGE_EXTRA_PX,
                colour: edge,
                dash: None,
            };
            for line in &lines {
                input.check_cancel()?;
                canvas::polyline(pixmap, line, &wide);
            }
        }
        for line in &lines {
            input.check_cancel()?;
            canvas::polyline(pixmap, line, &style.line);
        }
    }
    Ok(())
}

/// The classes that have a road in the network, from the smallest.
pub(crate) fn classes_present(input: &RenderInput<'_>) -> Vec<RoadClass> {
    let Some(network) = network(input) else {
        return Vec::new();
    };
    RoadClass::ALL
        .into_iter()
        .filter(|c| network.roads.iter().any(|r| r.class == *c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_split_where_the_filter_refuses() {
        let pts = [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0], [4.0, 0.0]];
        let keep = |p: [f64; 2]| (p[0] - 2.0).abs() > 0.5;
        let out = runs(&pts, &keep);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].len(), 2);
        assert_eq!(out[1], vec![[3.0, 0.0], [4.0, 0.0]]);
        assert_eq!(runs(&pts[..1], &|_| true), Vec::<Vec<[f64; 2]>>::new());
    }
}
