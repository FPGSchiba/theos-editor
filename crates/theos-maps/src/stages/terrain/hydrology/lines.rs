//! River lines (spec §5.9): the `river` cells traced over the receivers of the routing surface,
//! cut at confluences, smoothed by two Chaikin passes, with width, depth, discharge, bed height
//! and a Strahler order.
//!
//! The lines follow the same receivers as `discharge` and the `river` raster, so the three agree
//! by construction. A line starts at a source (a river cell without a river donor) or at a
//! confluence (two or more river donors) and follows the receivers while they are river cells; it
//! ends with the confluence it reaches, or with its last river cell before a lake, a seed or the
//! end of the river. A line needs two cells.
//!
//! Rivers end at lake shores and outlets start new lines (spec ruling 34): a line whose receiver
//! is a lake cell gets a last vertex where `kernels::contour::rings` puts the shore between its
//! last cell and that lake cell, and a line that starts where a lake spills gets a
//! first vertex where the ring crosses the line between them (halfway for neighbours that share
//! an edge, see `shore_fraction` for diagonals). Both carry the discharge of
//! the river cell next to them. A tributary's last vertex is the confluence cell and carries
//! the confluence's total discharge, as the first vertex of the downstream line does.

use serde::Serialize;

use super::Water;
use crate::grid::Grid;
use crate::kernels::num::{max, min, to_f64, to_index};

/// The hydraulic geometry of the channel, `width = a·Q^0.5` and `depth = c·Q^0.4` (Leopold and
/// Maddock 1953), with `Q` the discharge in m³/s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Geometry {
    /// The width constant `a`, m per (m³/s)^0.5.
    pub width_a: f64,
    /// The depth constant `c`, m per (m³/s)^0.4.
    pub depth_c: f64,
}

/// The exponent of the width, `Q^b`.
const WIDTH_EXPONENT: f64 = 0.5;
/// The exponent of the depth, `Q^f`.
const DEPTH_EXPONENT: f64 = 0.4;

/// A river line: smoothed points with the values at each of them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RiverLine {
    /// `[x, y, bed height]` in metres.
    pub points: Vec<[f64; 3]>,
    /// The channel width at each point, m.
    pub width_m: Vec<f64>,
    /// The channel depth at each point, m.
    pub depth_m: Vec<f64>,
    /// The discharge at each point, m³/s.
    pub discharge_m3s: Vec<f64>,
    /// The Strahler order of the line.
    pub order: u32,
}

/// The number of river donors of every cell: river cells whose receiver it is.
fn river_donors(receivers: &[usize], river: &[bool]) -> Vec<u32> {
    let mut donors = vec![0_u32; receivers.len()];
    for (i, &to) in receivers.iter().enumerate() {
        if to != i && river[i] && river[to] {
            donors[to] += 1;
        }
    }
    donors
}

/// The Strahler order of every river cell: 1 for a source; at a confluence the largest inflow
/// order, plus 1 when the two largest inflows are equal. `fill` lists the cells with receivers
/// before their donors; cells that are no river get 0.
pub(super) fn strahler(receivers: &[usize], fill: &[u32], river: &[bool]) -> Vec<u32> {
    let mut order = vec![0_u32; receivers.len()];
    // The largest inflow order of a cell, and how many inflows have it.
    let mut inflow = vec![(0_u32, 0_u32); receivers.len()];
    for &cell in fill.iter().rev() {
        let i = crate::stages::evolution::cell_index(cell);
        if !river[i] {
            continue;
        }
        let (top, count) = inflow[i];
        order[i] = match (top, count) {
            (0, _) => 1,
            (t, c) if c >= 2 => t + 1,
            (t, _) => t,
        };
        let to = receivers[i];
        if to != i && river[to] {
            let slot = &mut inflow[to];
            match order[i].cmp(&slot.0) {
                std::cmp::Ordering::Greater => *slot = (order[i], 1),
                std::cmp::Ordering::Equal => slot.1 += 1,
                std::cmp::Ordering::Less => {}
            }
        }
    }
    order
}

/// The cells of every line before smoothing, ordered by their first cell: each is a river
/// cell, followed by its receiver, with at least two cells.
pub(super) fn cell_paths(water: &Water, river: &Grid<bool>) -> Vec<Vec<usize>> {
    let receivers = &water.routing.drainage.receivers;
    let river = river.as_slice();
    let donors = river_donors(receivers, river);
    let mut paths = Vec::new();
    for start in (0..receivers.len()).filter(|&i| river[i] && donors[i] != 1) {
        let mut path = vec![start];
        let mut cell = start;
        loop {
            let to = receivers[cell];
            if to == cell || !river[to] {
                break;
            }
            path.push(to);
            cell = to;
            if donors[cell] >= 2 {
                break;
            }
        }
        if path.len() >= 2 {
            paths.push(path);
        }
    }
    paths
}

/// One Chaikin pass (corners cut at 1/4 and 3/4) on `[x, y, q]` points, keeping both end points.
fn chaikin(points: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let mix = |a: [f64; 3], b: [f64; 3], t: f64| [0, 1, 2].map(|k| a[k] * (1.0 - t) + b[k] * t);
    let mut out = Vec::with_capacity(points.len() * 2);
    out.extend(points.first().copied());
    for pair in points.windows(2) {
        out.push(mix(pair[0], pair[1], 0.25));
        out.push(mix(pair[0], pair[1], 0.75));
    }
    out.extend(points.last().copied());
    out
}

/// `height` bilinearly interpolated at `(x, y)` metres, clamped to the cell centres' hull.
fn bilinear(height: &Grid<f64>, x: f64, y: f64, cell_m: f64) -> f64 {
    let at = |v: f64, len: usize| {
        let g = min(max(v / cell_m - 0.5, 0.0), to_f64(len - 1));
        let i = to_index(g, len);
        (i, (i + 1).min(len - 1), g - to_f64(i))
    };
    let (r0, r1, fr) = at(y, height.rows());
    let (c0, c1, fc) = at(x, height.cols());
    let top = height.get(r0, c0) * (1.0 - fc) + height.get(r0, c1) * fc;
    let bottom = height.get(r1, c0) * (1.0 - fc) + height.get(r1, c1) * fc;
    top * (1.0 - fr) + bottom * fr
}

/// The number of the lake of every cell (0 for none).
fn lake_ids(water: &Water, len: usize) -> Vec<u32> {
    let mut ids = vec![0; len];
    for l in water.basins.iter().filter_map(|b| b.lake.as_ref()) {
        for &c in &l.cells {
            ids[c] = l.id;
        }
    }
    ids
}

/// Where the ring of a lake crosses the line from the centre of river cell `river` to the centre
/// of the lake cell `lake` (cells `(r, c)`, the lake cell's number `id` in `ids` on a grid of
/// `cols` columns), as the fraction of that line from the river cell. `kernels::contour` cuts a
/// quad edge between an inside and an outside cell at its midpoint, so for cells that share an
/// edge it is 0.5. For diagonal neighbours the quad is never a saddle (the two diagonal cells
/// differ), and the ring crosses the diagonal once: at 0.75 when none of the two shared cells
/// belongs to the lake (the corner of the lake cell is cut), at 0.5 when one does, and at 0.25
/// when both do (the corner of the river cell is cut).
fn shore_fraction(ids: &[u32], cols: usize, river: usize, lake: usize) -> f64 {
    let (rr, rc) = (river / cols, river % cols);
    let (lr, lc) = (lake / cols, lake % cols);
    if rr == lr || rc == lc {
        return 0.5;
    }
    let shared = [rr * cols + lc, lr * cols + rc];
    let same = shared.iter().filter(|&&c| ids[c] == ids[lake]).count();
    0.75 - 0.25 * to_f64(same)
}

/// For every cell, the lake cell where a lake spills into it: the exit of a lake that passes
/// water on, when the exit's receiver is a river cell.
fn outlet_donors(water: &Water, river: &[bool], len: usize) -> Vec<Option<usize>> {
    let receivers = &water.routing.drainage.receivers;
    let mut donors = vec![None; len];
    for basin in water
        .basins
        .iter()
        .filter(|b| !b.terminal && b.lake.is_some())
    {
        let exit = basin.depression.exit;
        let to = receivers[exit];
        if to != exit && river[to] {
            donors[to] = Some(exit);
        }
    }
    donors
}

/// The river lines of `water` over the `river` cells on cells of `cell_m` metres, in the index
/// order of their first cell.
pub(crate) fn lines(
    water: &Water,
    river: &Grid<bool>,
    cell_m: f64,
    geometry: &Geometry,
) -> Vec<RiverLine> {
    let drainage = &water.routing.drainage;
    let orders = strahler(&drainage.receivers, &drainage.order, river.as_slice());
    let discharge = &water.routing.discharge;
    let lake = lake_ids(water, river.len());
    let outlets = outlet_donors(water, river.as_slice(), river.len());
    let centre = |i: usize| {
        let (r, c) = river.cell(i);
        [(to_f64(c) + 0.5) * cell_m, (to_f64(r) + 0.5) * cell_m]
    };
    // The shore vertex between river cell `i` and lake cell `lake`, with `i`'s discharge.
    let shore = |i: usize, lake_cell: usize| {
        let (a, b) = (centre(i), centre(lake_cell));
        let t = shore_fraction(&lake, river.cols(), i, lake_cell);
        [
            a[0] + t * (b[0] - a[0]),
            a[1] + t * (b[1] - a[1]),
            discharge.as_slice()[i],
        ]
    };
    cell_paths(water, river)
        .into_iter()
        .map(|path| {
            let first = path[0];
            let last = path[path.len() - 1];
            let mut raw: Vec<[f64; 3]> = Vec::with_capacity(path.len() + 2);
            raw.extend(outlets[first].map(|exit| shore(first, exit)));
            raw.extend(path.iter().map(|&i| {
                let [x, y] = centre(i);
                [x, y, discharge.as_slice()[i]]
            }));
            let to = water.routing.drainage.receivers[last];
            if to != last && lake[to] != 0 {
                raw.push(shore(last, to));
            }
            let smooth = chaikin(&chaikin(&raw));
            let mut line = RiverLine {
                points: Vec::with_capacity(smooth.len()),
                width_m: Vec::with_capacity(smooth.len()),
                depth_m: Vec::with_capacity(smooth.len()),
                discharge_m3s: Vec::with_capacity(smooth.len()),
                order: orders[path[0]],
            };
            for [x, y, q] in smooth {
                let bed = bilinear(&water.height, x, y, cell_m);
                line.points.push([x, y, bed]);
                line.width_m
                    .push(geometry.width_a * libm::pow(q, WIDTH_EXPONENT));
                line.depth_m
                    .push(geometry.depth_c * libm::pow(q, DEPTH_EXPONENT));
                line.discharge_m3s.push(q);
            }
            line
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stages::terrain::hydrology::outlines::outlines;
    use crate::stages::terrain::hydrology::rasters::rasters;
    use crate::stages::terrain::hydrology::tests::{
        CELL_M, N, WET, all_edges, at, bowl, compute, grid, mask, plane, south_edge,
    };
    use crate::stages::terrain::hydrology::{Water, WaterThresholds};

    const GEOMETRY: Geometry = Geometry {
        width_a: 3.0,
        depth_c: 0.3,
    };
    const THRESHOLDS: WaterThresholds = WaterThresholds {
        river_q: 0.003,
        big_river_q: 0.2,
    };

    /// A main valley down column 32 from row 20 and a tributary valley along row 20 from the
    /// west, joining it at (20, 32); every cell slopes to the nearest channel cell.
    fn valley() -> Grid<f64> {
        let mut channel: Vec<(usize, usize, f64)> = Vec::new();
        for r in 20..N {
            channel.push((r, 32, to_f64(N - 1 - r)));
        }
        for c in 4..32 {
            channel.push((20, c, to_f64(N - 1 - 20 + 32 - c)));
        }
        grid(|r, c| {
            let nearest = channel
                .iter()
                .map(|&(cr, cc, s)| {
                    let (dr, dc) = (to_f64(r) - to_f64(cr), to_f64(c) - to_f64(cc));
                    (dr * dr + dc * dc, s)
                })
                .fold((f64::INFINITY, 0.0), |a, b| if b.0 < a.0 { b } else { a });
            2.0 * nearest.0.sqrt() + 0.3 * nearest.1
        })
    }

    struct Case {
        water: Water,
        river: Grid<bool>,
    }

    fn case(height: &Grid<f64>, fixed: &Grid<bool>) -> Case {
        let water = compute(height, fixed, WET);
        let river = rasters(&water, &THRESHOLDS).river;
        Case { water, river }
    }

    /// The river donors of every cell, counted from the outside.
    fn donors_of(case: &Case) -> Vec<usize> {
        let receivers = &case.water.routing.drainage.receivers;
        let mut count = vec![0; N * N];
        for (i, &to) in receivers.iter().enumerate() {
            if to != i && case.river.as_slice()[i] && case.river.as_slice()[to] {
                count[to] += 1;
            }
        }
        count
    }

    #[test]
    fn lines_end_at_confluences_and_lake_shores() {
        let valley = case(&valley(), &south_edge());
        let donors = donors_of(&valley);
        let confluences: Vec<usize> = (0..N * N).filter(|&i| donors[i] >= 2).collect();
        assert!(confluences.contains(&at(20, 32)), "{confluences:?}");
        let paths = cell_paths(&valley.water, &valley.river);
        let receivers = &valley.water.routing.drainage.receivers;
        for path in &paths {
            let last = *path.last().unwrap();
            let next = receivers[last];
            assert!(
                donors[last] >= 2 || next == last || !valley.river.as_slice()[next],
                "path {path:?} ends mid-river"
            );
            assert!(
                path[1..path.len() - 1].iter().all(|&c| donors[c] < 2),
                "path {path:?} runs through a confluence"
            );
        }
        for &confluence in &confluences {
            assert!(paths.iter().any(|p| p.last() == Some(&confluence)));
            let next = receivers[confluence];
            if valley.river.as_slice()[next] && next != confluence {
                assert!(paths.iter().any(|p| p.first() == Some(&confluence)));
            }
        }

        // The wet bowl's lake: a river runs down its wall and stops at the shore.
        let bowl = case(&bowl(), &all_edges());
        let lake = rasters(&bowl.water, &THRESHOLDS).lake;
        let receivers = &bowl.water.routing.drainage.receivers;
        let paths = cell_paths(&bowl.water, &bowl.river);
        let shore = paths
            .iter()
            .filter(|p| lake.as_slice()[receivers[*p.last().unwrap()]])
            .count();
        assert!(shore > 0, "a line ends at the lake shore");
        for path in &paths {
            assert!(path.iter().all(|&c| !lake.as_slice()[c]));
        }
    }

    #[test]
    fn river_lines_run_over_river_cells() {
        let valley = case(&valley(), &south_edge());
        let receivers = &valley.water.routing.drainage.receivers;
        let discharge = valley.water.routing.discharge.as_slice();
        let paths = cell_paths(&valley.water, &valley.river);
        let mut seen = vec![false; N * N];
        for path in &paths {
            assert!(path.len() >= 2);
            for pair in path.windows(2) {
                assert_eq!(receivers[pair[0]], pair[1]);
                assert!(discharge[pair[1]] >= discharge[pair[0]]);
            }
            for &c in path {
                assert!(valley.river.as_slice()[c], "cell {c} is no river cell");
                seen[c] = true;
            }
        }
        // Every river cell that has a river neighbour on its way lies on a line.
        let donors = donors_of(&valley);
        for i in 0..N * N {
            let next = receivers[i];
            let linked = next != i && valley.river.as_slice()[next];
            if valley.river.as_slice()[i] && (linked || donors[i] > 0) {
                assert!(seen[i], "river cell {i} lies on no line");
            }
        }
    }

    #[test]
    fn single_cell_rivers_make_no_line() {
        let case = case(&plane(), &south_edge());
        let alone = mask(|r, c| r == N - 1 && c == 10);
        assert_eq!(lines(&case.water, &alone, CELL_M, &GEOMETRY).len(), 0);
        // Two cells make one line; two Chaikin passes turn 2 points into 8, keeping the ends.
        let pair = mask(|r, c| c == 10 && r >= N - 2);
        let found = lines(&case.water, &pair, CELL_M, &GEOMETRY);
        assert_eq!(found.len(), 1);
        let line = &found[0];
        assert_eq!(line.points.len(), 8);
        let (x, y0, y1) = (10.5 * CELL_M, 62.5 * CELL_M, 63.5 * CELL_M);
        assert_eq!(line.points[0][..2], [x, y0]);
        assert_eq!(line.points[7][..2], [x, y1]);
    }

    #[test]
    fn strahler_orders_follow_their_rule() {
        // Cells 0 to 3 and 8, 10 are sources. 4 takes 0 and 1 (1 and 1: order 2); 5 takes 4 and
        // 2 (2 and 1: stays 2); 6 takes 5 and 3 (2 and 1: stays 2); 9 takes 8 and 10 (1 and 1:
        // order 2); 7 takes 6 and 9 (2 and 2: order 3); 11 takes 7 alone and keeps 3.
        let receivers = [4, 4, 5, 6, 5, 6, 7, 11, 9, 7, 9, 11];
        let river = [true; 12];
        // Fill order: receivers before their donors.
        let fill = [11, 7, 6, 9, 5, 3, 4, 2, 0, 1, 8, 10];
        let orders = strahler(&receivers, &fill, &river);
        assert_eq!(orders, [1, 1, 1, 1, 2, 2, 2, 3, 1, 2, 1, 3]);
    }

    #[test]
    fn per_vertex_arrays_match_the_points() {
        let valley = case(&valley(), &south_edge());
        let found = lines(&valley.water, &valley.river, CELL_M, &GEOMETRY);
        assert!(found.len() >= 3);
        let discharge = valley.water.routing.discharge.as_slice();
        for line in &found {
            let n = line.points.len();
            assert!(n >= 2);
            assert_eq!(line.width_m.len(), n);
            assert_eq!(line.depth_m.len(), n);
            assert_eq!(line.discharge_m3s.len(), n);
            assert!(line.order >= 1);
            for k in 0..n {
                let q = line.discharge_m3s[k];
                assert!(q > 0.0);
                let width = 3.0 * libm::pow(q, 0.5);
                let depth = 0.3 * libm::pow(q, 0.4);
                assert!((line.width_m[k] - width).abs() < 1e-12 * width);
                assert!((line.depth_m[k] - depth).abs() < 1e-12 * depth);
            }
            // The ends keep their cells' discharge.
            for (k, end) in [(0, line.points[0]), (n - 1, line.points[n - 1])] {
                let (r, c) = (to_index(end[1] / CELL_M, N), to_index(end[0] / CELL_M, N));
                let want = discharge[at(r, c)];
                assert_eq!(line.discharge_m3s[k].to_bits(), want.to_bits());
            }
            // Discharge never falls downstream.
            assert!(line.discharge_m3s.windows(2).all(|w| w[1] >= w[0]));
        }
    }

    #[test]
    fn bed_height_is_the_terrain_height_at_the_vertex() {
        // On the plane every river is a straight column and height is linear in the row, so the
        // bed height at a vertex is the plane's height at its y.
        let flat = case(&plane(), &south_edge());
        let found = lines(&flat.water, &flat.river, CELL_M, &GEOMETRY);
        assert!(found.len() >= 60, "{} lines", found.len());
        for line in &found {
            for p in &line.points {
                let column = p[0] / CELL_M - 0.5;
                let row = p[1] / CELL_M - 0.5;
                let expected = 63.0 - row + 0.01 * (to_f64(to_index(column, N)) - 32.0).abs();
                assert!((p[2] - expected).abs() < 1e-9, "{p:?} vs {expected}");
            }
        }
        // On a bent line every bed height lies within the heights of its four cells.
        let bent = case(&valley(), &south_edge());
        let h = &bent.water.height;
        for line in lines(&bent.water, &bent.river, CELL_M, &GEOMETRY) {
            for p in &line.points {
                let (x, y) = (p[0] / CELL_M - 0.5, p[1] / CELL_M - 0.5);
                let (r0, c0) = (to_index(y, N), to_index(x, N));
                let (r1, c1) = ((r0 + 1).min(N - 1), (c0 + 1).min(N - 1));
                let corners = [h.get(r0, c0), h.get(r0, c1), h.get(r1, c0), h.get(r1, c1)];
                let lo = corners.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = corners.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                assert!(p[2] >= lo - 1e-9 && p[2] <= hi + 1e-9);
            }
        }
    }

    /// The distance from `p` to the segment `a`-`b`.
    fn distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        let (ex, ey) = (p[0] - a[0] - t * dx, p[1] - a[1] - t * dy);
        (ex * ex + ey * ey).sqrt()
    }

    fn on_ring(p: [f64; 3], rings: &[Vec<[f64; 2]>]) -> bool {
        rings
            .iter()
            .flat_map(|r| r.windows(2))
            .any(|w| distance([p[0], p[1]], w[0], w[1]) < 1e-9)
    }

    fn inside_ring(p: [f64; 3], ring: &[[f64; 2]]) -> bool {
        let mut inside = false;
        for w in ring.windows(2) {
            let (a, b) = (w[0], w[1]);
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0])
            {
                inside = !inside;
            }
        }
        inside
    }

    #[test]
    fn lines_meet_the_lake_shore_and_start_again_at_its_outlet() {
        let wet = case(&bowl(), &all_edges());
        let shapes = outlines(&wet.water, CELL_M);
        let rings = &shapes[0].rings;
        let receivers = &wet.water.routing.drainage.receivers;
        let lake = rasters(&wet.water, &THRESHOLDS).lake;
        let exit = wet.water.basins[0].depression.exit;
        let paths = cell_paths(&wet.water, &wet.river);
        let found = lines(&wet.water, &wet.river, CELL_M, &GEOMETRY);
        assert_eq!(paths.len(), found.len());
        let (mut inflows, mut outflows) = (0, 0);
        for (path, line) in paths.iter().zip(&found) {
            let (first, last) = (path[0], *path.last().unwrap());
            let ends_at_lake = lake.as_slice()[receivers[last]];
            let starts_at_outlet = receivers[exit] == first;
            let n = line.points.len();
            if ends_at_lake {
                inflows += 1;
                assert!(
                    on_ring(line.points[n - 1], rings),
                    "{:?}",
                    line.points[n - 1]
                );
                let q = wet.water.routing.discharge.as_slice()[last];
                assert_eq!(line.discharge_m3s[n - 1].to_bits(), q.to_bits());
            }
            if starts_at_outlet {
                outflows += 1;
                assert!(on_ring(line.points[0], rings), "{:?}", line.points[0]);
            }
            for (k, p) in line.points.iter().enumerate() {
                let shore = (ends_at_lake && k == n - 1) || (starts_at_outlet && k == 0);
                if !shore {
                    assert!(
                        rings.iter().all(|r| !inside_ring(*p, r)),
                        "{p:?} lies in the lake"
                    );
                }
            }
        }
        assert!(inflows > 0 && outflows == 1, "{inflows} in, {outflows} out");
    }

    /// A 5 x 5 world with lake 1 on cell (2, 2) and, by `shared`, on the cells (1, 2) and (2, 1)
    /// next to it. In both cases the river cell (1, 1) touches the lake cell (2, 2) diagonally.
    /// With `inflow` the river runs (1, 0) -> (1, 1) -> lake; otherwise the lake spills from
    /// (2, 2) into (1, 1), which flows on to (1, 0).
    fn diagonal(shared: usize, inflow: bool) -> (Water, Grid<bool>) {
        use crate::stages::terrain::hydrology::discharge::{Drainage, Routing};
        use crate::stages::terrain::hydrology::lakes::{Balance, Basin, Depression, Lake};
        let n = 5;
        let at = |r: usize, c: usize| r * n + c;
        let mut cells = vec![at(2, 2)];
        cells.extend([at(1, 2), at(2, 1)].into_iter().take(shared));
        cells.sort_unstable();
        let mut receivers: Vec<usize> = (0..n * n).collect();
        if inflow {
            receivers[at(1, 0)] = at(1, 1);
            receivers[at(1, 1)] = at(2, 2);
        } else {
            receivers[at(2, 2)] = at(1, 1);
            receivers[at(1, 1)] = at(1, 0);
        }
        let height = Grid::filled(n, n, 1.0);
        let basin = Basin {
            depression: Depression {
                cells: cells.clone(),
                spill_level: 2.0,
                bottom: at(2, 2),
                exit: at(2, 2),
            },
            balance: Balance::default(),
            terminal: false,
            lake: Some(Lake {
                id: 1,
                cells,
                level: 2.0,
            }),
        };
        let order = (0..n * n).map(|c| u32::try_from(c).unwrap()).collect();
        let routing = Routing {
            drainage: Drainage {
                surface: height.clone(),
                order,
                receivers,
            },
            discharge: Grid::filled(n, n, 1.0),
        };
        let water = Water {
            height,
            basins: vec![basin],
            filled_pits: 0,
            filled_nested: 0,
            routing,
        };
        let river = mask_n(n, |r, c| r == 1 && c < 2);
        (water, river)
    }

    fn mask_n(n: usize, f: impl Fn(usize, usize) -> bool) -> Grid<bool> {
        Grid::from_vec(n, n, (0..n * n).map(|i| f(i / n, i % n)).collect()).unwrap()
    }

    #[test]
    fn diagonal_shore_vertices_lie_on_the_ring() {
        for shared in 0..=2 {
            for inflow in [true, false] {
                let (water, river) = diagonal(shared, inflow);
                let shapes = outlines(&water, CELL_M);
                let found = lines(&water, &river, CELL_M, &GEOMETRY);
                assert_eq!(found.len(), 1, "k = {shared}");
                let line = &found[0];
                let p = if inflow {
                    line.points[line.points.len() - 1]
                } else {
                    line.points[0]
                };
                assert!(
                    on_ring(p, &shapes[0].rings),
                    "k = {shared}, inflow = {inflow}: {p:?}"
                );
            }
        }
    }
}
