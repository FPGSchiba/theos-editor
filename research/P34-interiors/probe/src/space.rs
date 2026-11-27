//! Space: lattice cells -> rooms (inset rectangles) and corridors, rasterised at 2 m.
use crate::graph::Graph;
use crate::recipe::{Recipe, TypeSpec};
use crate::rng::Rng;

#[derive(Clone, Copy, Debug)]
pub struct Rect { pub x0: i32, pub y0: i32, pub x1: i32, pub y1: i32 }
impl Rect {
    pub fn centre(&self) -> (i32, i32) { ((self.x0 + self.x1) / 2, (self.y0 + self.y1) / 2) }
    fn t(&self) -> Rect { Rect { x0: self.y0, y0: self.x0, x1: self.y1, y1: self.x1 } }
}

pub struct Raster {
    pub n: i32,
    pub walkable: Vec<bool>,
    /// room number + 1, 0 = none/corridor
    pub rooms: Vec<u16>,
    pub rects: Vec<Rect>,
    pub edge_cells: Vec<Vec<(i32, i32)>>,
    pub marks: Vec<(i32, i32)>,
    pub route_fail: u32,
    pub dense: bool,
}

fn room_rect(spec: Option<&TypeSpec>, cell: (i32, i32), size: (i32, i32), cr: i32, rng: &mut Rng) -> Rect {
    let (mut w, mut h) = match spec { Some(s) => (rng.range(s.w.0, s.w.1), rng.range(s.h.0, s.h.1)), None => (6, 6) };
    if spec.map_or(false, |s| s.long) && rng.pct(50) { std::mem::swap(&mut w, &mut h); }
    let hall = size.0 * size.1 > 1;
    let (rw, rh) = (size.0 * cr, size.1 * cr);
    let lo = if hall { cr + 4 } else { 3 };
    let (w, h) = (w.clamp(lo, rw - 2), h.clamp(lo, rh - 2));
    let x0 = cell.0 * cr + 1 + rng.below((rw - 1 - w) as usize) as i32;
    let y0 = cell.1 * cr + 1 + rng.below((rh - 1 - h) as usize) as i32;
    Rect { x0, y0, x1: x0 + w - 1, y1: y0 + h - 1 }
}

/// Horizontal link between `l` (left) and `r` (right) inside the row `band`; straight when the rows overlap, else a Z.
fn route_h(l: &Rect, r: &Rect, band: (i32, i32), xm: i32, wd: i32, rng: &mut Rng) -> Option<Vec<(i32, i32)>> {
    let (alo, ahi) = (l.y0.max(band.0), l.y1.min(band.1));
    let (blo, bhi) = (r.y0.max(band.0), r.y1.min(band.1));
    if ahi - alo + 1 < wd || bhi - blo + 1 < wd { return None; }
    let (lo, hi) = (alo.max(blo), ahi.min(bhi));
    let mut c = vec![];
    if hi - lo + 1 >= wd {
        let y = lo + rng.below((hi - lo + 2 - wd) as usize) as i32;
        for x in l.x1 + 1..r.x0 { for d in 0..wd { c.push((x, y + d)); } }
    } else {
        let ya = alo + rng.below((ahi - alo + 2 - wd) as usize) as i32;
        let yb = blo + rng.below((bhi - blo + 2 - wd) as usize) as i32;
        for x in l.x1 + 1..xm { for d in 0..wd { c.push((x, ya + d)); } }
        for y in ya.min(yb)..=ya.max(yb) + wd - 1 { for d in 0..wd { c.push((xm + d, y)); } }
        for x in xm + wd..r.x0 { for d in 0..wd { c.push((x, yb + d)); } }
    }
    Some(c)
}

pub fn layout(r: &Recipe, g: &Graph, rng: &mut Rng) -> Raster {
    if r.space == crate::recipe::Space::Dense { return crate::dense::layout(r, g, rng); }
    let cr = r.cr;
    let n = g.w * cr;
    let mut ras = Raster { n, walkable: vec![false; (n * n) as usize], rooms: vec![0; (n * n) as usize], rects: vec![], edge_cells: vec![], marks: vec![], route_fail: 0, dense: false };
    for (i, nd) in g.nodes.iter().enumerate() {
        let spec = r.types.iter().find(|t| t.name == nd.ty);
        let rect = room_rect(spec, nd.cell, nd.size, cr, rng);
        for y in rect.y0..=rect.y1 { for x in rect.x0..=rect.x1 {
            ras.walkable[(y * n + x) as usize] = true;
            ras.rooms[(y * n + x) as usize] = (i + 1) as u16;
        } }
        ras.rects.push(rect);
    }
    for e in &g.edges {
        let wd = if e.main { 2 } else { 1 };
        let (ra, rb) = (ras.rects[e.a], ras.rects[e.b]);
        let (ca, cb) = (e.ca, e.cb);
        let cells = if cb.1 == ca.1 {
            let (l, rr, cl) = if cb.0 > ca.0 { (ra, rb, ca) } else { (rb, ra, cb) };
            route_h(&l, &rr, (ca.1 * cr, ca.1 * cr + cr - 1), (cl.0 + 1) * cr - 1, wd, rng)
        } else {
            let (l, rr, cl) = if cb.1 > ca.1 { (ra, rb, ca) } else { (rb, ra, cb) };
            route_h(&l.t(), &rr.t(), (ca.0 * cr, ca.0 * cr + cr - 1), (cl.1 + 1) * cr - 1, wd, rng).map(|v| v.into_iter().map(|(x, y)| (y, x)).collect())
        };
        let cells = cells.unwrap_or_else(|| { ras.route_fail += 1; vec![] });
        for &(x, y) in &cells { ras.walkable[(y * n + x) as usize] = true; }
        ras.marks.push(if cells.is_empty() { (0, 0) } else { cells[cells.len() / 2] });
        ras.edge_cells.push(cells);
    }
    ras
}
