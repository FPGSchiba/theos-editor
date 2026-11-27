//! Dense space: rooms fill their lattice block minus a wall; doors are gaps in the shared wall.
//! A room's wall lies on its top/left side, so two touching rooms are separated by exactly `wall` cells.
use crate::graph::Graph;
use crate::recipe::{Recipe, Shape};
use crate::rng::Rng;
use crate::space::{Raster, Rect};
use std::collections::VecDeque;

fn inside_ellipse(x: i32, y: i32, r: &Rect) -> bool {
    let (w, h) = ((r.x1 - r.x0 + 1) as i64, (r.y1 - r.y0 + 1) as i64);
    let (dx, dy) = ((2 * x - r.x0 - r.x1) as i64, (2 * y - r.y0 - r.y1) as i64);
    dx * dx * h * h + dy * dy * w * w <= w * w * h * h
}

/// Shape mask of a room (round / ragged), pruned to the component holding the centre.
fn room_mask(rect: &Rect, shape: Shape, rng: &mut Rng) -> Vec<bool> {
    let (w, h) = (rect.x1 - rect.x0 + 1, rect.y1 - rect.y0 + 1);
    let mut m = vec![true; (w * h) as usize];
    for y in rect.y0..=rect.y1 { for x in rect.x0..=rect.x1 {
        let mut on = true;
        match shape {
            Shape::Rect => {}
            Shape::Round => on = inside_ellipse(x, y, rect),
            Shape::Ragged => {
                let d = (x - rect.x0).min(rect.x1 - x).min(y - rect.y0).min(rect.y1 - y);
                if (d == 0 && rng.pct(40)) || (d == 1 && rng.pct(15)) { on = false; }
            }
        }
        m[((y - rect.y0) * w + x - rect.x0) as usize] = on;
    } }
    let c = rect.centre();
    let ci = ((c.1 - rect.y0) * w + c.0 - rect.x0) as usize;
    m[ci] = true;
    let mut keep = vec![false; m.len()];
    let mut q = VecDeque::new();
    keep[ci] = true;
    q.push_back(ci);
    while let Some(i) = q.pop_front() {
        let (x, y) = (i as i32 % w, i as i32 / w);
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h { continue; }
            let j = (ny * w + nx) as usize;
            if m[j] && !keep[j] { keep[j] = true; q.push_back(j); }
        }
    }
    keep
}

/// Carve a straight run from the door into the room until it meets the room's carved cells.
fn carve_stub(ras: &mut Raster, rect: &Rect, room: usize, xy: &dyn Fn(i32, i32) -> (i32, i32), a0: i32, step: i32, b: i32) {
    let mut a = a0;
    let n = ras.n;
    loop {
        let (x, y) = xy(a, b);
        if x < rect.x0 || x > rect.x1 || y < rect.y0 || y > rect.y1 { break; }
        let i = (y * n + x) as usize;
        if ras.rooms[i] as usize == room + 1 { return; }
        ras.walkable[i] = true;
        ras.rooms[i] = (room + 1) as u16;
        a += step;
    }
    // fell off the far side (tiny eroded room): join the run to the centre
    let c = rect.centre();
    let (mut x, mut y) = xy(a - step, b);
    while y != c.1 { y += (c.1 - y).signum(); let i = (y * n + x) as usize; ras.walkable[i] = true; ras.rooms[i] = (room + 1) as u16; }
    while x != c.0 { x += (c.0 - x).signum(); let i = (y * n + x) as usize; ras.walkable[i] = true; ras.rooms[i] = (room + 1) as u16; }
}

pub fn layout(r: &Recipe, g: &Graph, rng: &mut Rng) -> Raster {
    let (p, t) = (r.cr, r.wall);
    let n = g.w * p;
    let mut ras = Raster { n, walkable: vec![false; (n * n) as usize], rooms: vec![0; (n * n) as usize], rects: vec![], edge_cells: vec![], marks: vec![], route_fail: 0, dense: true };
    for (i, nd) in g.nodes.iter().enumerate() {
        let shape = r.types.iter().find(|s| s.name == nd.ty).map_or(Shape::Rect, |s| s.shape);
        let rect = Rect { x0: nd.cell.0 * p + t, y0: nd.cell.1 * p + t, x1: (nd.cell.0 + nd.size.0) * p - 1, y1: (nd.cell.1 + nd.size.1) * p - 1 };
        let mask = room_mask(&rect, shape, rng);
        let w = rect.x1 - rect.x0 + 1;
        for y in rect.y0..=rect.y1 { for x in rect.x0..=rect.x1 {
            if mask[((y - rect.y0) * w + x - rect.x0) as usize] {
                ras.walkable[(y * n + x) as usize] = true;
                ras.rooms[(y * n + x) as usize] = (i + 1) as u16;
            }
        } }
        ras.rects.push(rect);
    }
    for e in &g.edges {
        let (lo_c, hi_c, lo, hi, axis_x) = if e.cb.1 == e.ca.1 {
            if e.cb.0 > e.ca.0 { (e.ca, e.cb, e.a, e.b, true) } else { (e.cb, e.ca, e.b, e.a, true) }
        } else if e.cb.1 > e.ca.1 { (e.ca, e.cb, e.a, e.b, false) } else { (e.cb, e.ca, e.b, e.a, false) };
        let _ = hi_c;
        let (al, bl) = if axis_x { (lo_c.0, lo_c.1) } else { (lo_c.1, lo_c.0) };
        let wa = (al + 1) * p;
        let (band_lo, band_hi) = (bl * p + t, (bl + 1) * p - 1);
        let dw = (if e.main || rng.pct(25) { 2 } else { 1 }).min(band_hi - band_lo + 1);
        let pb = band_lo + rng.below((band_hi - band_lo + 2 - dw) as usize) as i32;
        let xy = move |a: i32, b: i32| if axis_x { (a, b) } else { (b, a) };
        let mut cells = vec![];
        for k in 0..t { for d in 0..dw { cells.push(xy(wa + k, pb + d)); } }
        let (rl, rh) = (ras.rects[lo], ras.rects[hi]);
        for d in 0..dw {
            carve_stub(&mut ras, &rl, lo, &xy, wa - 1, -1, pb + d);
            carve_stub(&mut ras, &rh, hi, &xy, wa + t, 1, pb + d);
        }
        for &(x, y) in &cells { ras.walkable[(y * n + x) as usize] = true; }
        ras.marks.push(cells[cells.len() / 2]);
        ras.edge_cells.push(cells);
    }
    ras
}
