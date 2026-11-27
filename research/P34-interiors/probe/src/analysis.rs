//! Analysis: carve/connectivity/key checks and metrics, measured on the raster.
use crate::graph::{Graph, Kind, Role};
use crate::space::Raster;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

#[derive(Default)]
pub struct Metrics {
    pub rooms: usize,
    pub connections: usize,
    pub cycles: i64,
    pub locks: usize,
    pub one_way: usize,
    pub dead_end_pct: f64,
    pub footprint_pct: f64,
    pub critical_m: f64,
    pub sightline_m: f64,
    pub hall_share_pct: f64,
    pub fails: Vec<String>,
    pub route: Vec<(i32, i32)>,
}

/// BFS on the raster between the centres of the two rooms of an edge, inside the two rooms and the edge's corridor.
fn edge_path(ras: &Raster, g: &Graph, ei: usize) -> Option<Vec<(i32, i32)>> {
    let e = &g.edges[ei];
    let (ra, rb) = (ras.rects[e.a], ras.rects[e.b]);
    let cells = &ras.edge_cells[ei];
    let (mut x0, mut y0, mut x1, mut y1) = (ra.x0.min(rb.x0), ra.y0.min(rb.y0), ra.x1.max(rb.x1), ra.y1.max(rb.y1));
    for &(x, y) in cells { x0 = x0.min(x); y0 = y0.min(y); x1 = x1.max(x); y1 = y1.max(y); }
    let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
    let mut ok = vec![false; (w * h) as usize];
    for y in y0..=y1 { for x in x0..=x1 {
        let r = ras.rooms[(y * ras.n + x) as usize] as usize;
        if r == e.a + 1 || r == e.b + 1 { ok[((y - y0) * w + x - x0) as usize] = true; }
    } }
    for &(x, y) in cells { ok[((y - y0) * w + x - x0) as usize] = true; }
    let (sa, sb) = (ra.centre(), rb.centre());
    let (s, t) = (((sa.1 - y0) * w + sa.0 - x0) as usize, ((sb.1 - y0) * w + sb.0 - x0) as usize);
    let mut prev = vec![usize::MAX; (w * h) as usize];
    let mut q = VecDeque::new();
    prev[s] = s;
    q.push_back(s);
    while let Some(i) = q.pop_front() {
        if i == t { break; }
        let (x, y) = (i as i32 % w, i as i32 / w);
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h { continue; }
            let j = (ny * w + nx) as usize;
            if ok[j] && prev[j] == usize::MAX { prev[j] = i; q.push_back(j); }
        }
    }
    if prev[t] == usize::MAX { return None; }
    let mut p = vec![];
    let mut i = t;
    loop { p.push((i as i32 % w + x0, i as i32 / w + y0)); if i == s { break; } i = prev[i]; }
    p.reverse();
    Some(p)
}

pub fn analyse(g: &Graph, ras: &Raster, envelope_cells: usize) -> Metrics {
    let mut m = Metrics::default();
    let n = ras.n;
    m.rooms = g.nodes.len();
    m.connections = g.edges.len();
    m.cycles = g.edges.len() as i64 - g.nodes.len() as i64 + 1;
    m.locks = g.edges.iter().filter(|e| matches!(e.kind, Kind::Locked(_))).count();
    m.one_way = g.edges.iter().filter(|e| e.kind == Kind::Drop).count();
    let mut deg = vec![0usize; g.nodes.len()];
    for e in &g.edges { deg[e.a] += 1; deg[e.b] += 1; }
    m.dead_end_pct = 100.0 * deg.iter().filter(|&&d| d == 1).count() as f64 / m.rooms as f64;
    let walk = ras.walkable.iter().filter(|&&w| w).count();
    m.footprint_pct = 100.0 * walk as f64 / envelope_cells as f64;
    let hall_cells = ras.rooms.iter().filter(|&&r| r > 0 && { let s = g.nodes[r as usize - 1].size; s.0 * s.1 >= 2 }).count();
    m.hall_share_pct = 100.0 * hall_cells as f64 / walk.max(1) as f64;
    if ras.dense {
        let n = ras.n;
        let mut bad = 0;
        for y in 0..n { for x in 0..n {
            let a = ras.rooms[(y * n + x) as usize];
            if a == 0 { continue; }
            for (dx, dy) in [(1, 0), (0, 1)] {
                if x + dx >= n || y + dy >= n { continue; }
                let b = ras.rooms[((y + dy) * n + x + dx) as usize];
                if b != 0 && b != a { bad += 1; }
            }
        } }
        if bad > 0 { m.fails.push(format!("{bad} room cells touch another room without a wall")); }
        for (ei, e) in g.edges.iter().enumerate() {
            let mut ids = std::collections::BTreeSet::new();
            for &(x, y) in &ras.edge_cells[ei] { for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= n || ny >= n { continue; }
                let r = ras.rooms[(ny * n + nx) as usize];
                if r > 0 { ids.insert(r as usize - 1); }
            } }
            if ids != [e.a, e.b].into_iter().collect() { m.fails.push(format!("door {ei} touches rooms {ids:?}, expected {} and {}", e.a, e.b)); }
        }
    }

    // carved
    let mut carved = vec![false; g.nodes.len()];
    for &r in &ras.rooms { if r > 0 { carved[r as usize - 1] = true; } }
    if let Some(i) = carved.iter().position(|c| !c) { m.fails.push(format!("room {i} not carved")); }
    if ras.route_fail > 0 { m.fails.push(format!("{} corridors failed to route", ras.route_fail)); }

    // flood-fill connectivity over walkable
    let mut seen = vec![false; walk.max(1) * 0 + (n * n) as usize];
    let mut comps = 0;
    for s in 0..(n * n) as usize {
        if !ras.walkable[s] || seen[s] { continue; }
        comps += 1;
        let mut q = VecDeque::new();
        seen[s] = true;
        q.push_back(s);
        while let Some(i) = q.pop_front() {
            let (x, y) = (i as i32 % n, i as i32 / n);
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= n || ny >= n { continue; }
                let j = (ny * n + nx) as usize;
                if ras.walkable[j] && !seen[j] { seen[j] = true; q.push_back(j); }
            }
        }
    }
    if comps != 1 { m.fails.push(format!("walkable has {comps} components")); }

    // keys, reachability, drops
    if let Err(e) = g.check() { m.fails.push(e); }

    // sightline: longest axis-aligned run of walkable cells
    let mut best = 0;
    for y in 0..n { let mut run = 0; for x in 0..n { if ras.walkable[(y * n + x) as usize] { run += 1; best = best.max(run); } else { run = 0; } } }
    for x in 0..n { let mut run = 0; for y in 0..n { if ras.walkable[(y * n + x) as usize] { run += 1; best = best.max(run); } else { run = 0; } } }
    m.sightline_m = best as f64 * 2.0;

    // critical route: Dijkstra over (room, keys held), edge cost = raster BFS steps between room centres
    let paths: Vec<Option<Vec<(i32, i32)>>> = (0..g.edges.len()).map(|i| edge_path(ras, g, i)).collect();
    if paths.iter().any(|p| p.is_none()) { m.fails.push("an edge has no raster path between its rooms".into()); }
    let adj = g.adj();
    let key_bit = |v: usize| g.nodes[v].key.map_or(0u64, |k| 1u64 << k);
    let s0 = (g.start, key_bit(g.start));
    let mut dist: HashMap<(usize, u64), u32> = HashMap::new();
    let mut prev: HashMap<(usize, u64), ((usize, u64), usize)> = HashMap::new();
    let mut heap = BinaryHeap::new();
    dist.insert(s0, 0);
    heap.push(Reverse((0u32, s0.0, s0.1)));
    let mut found = None;
    while let Some(Reverse((d, u, mask))) = heap.pop() {
        if dist.get(&(u, mask)).map_or(true, |&x| x < d) { continue; }
        if g.nodes[u].role == Role::Goal { found = Some((u, mask)); break; }
        for &ei in &adj[u] {
            let Some(v) = g.edges[ei].pass(u, mask) else { continue };
            let Some(p) = &paths[ei] else { continue };
            let nm = mask | key_bit(v);
            let nd = d + p.len() as u32 - 1;
            if dist.get(&(v, nm)).map_or(true, |&x| nd < x) {
                dist.insert((v, nm), nd);
                prev.insert((v, nm), ((u, mask), ei));
                heap.push(Reverse((nd, v, nm)));
            }
        }
    }
    match found {
        None => m.fails.push("no critical route start -> goal".into()),
        Some(mut st) => {
            m.critical_m = dist[&st] as f64 * 2.0;
            let mut segs = vec![];
            while st != s0 {
                let (p, ei) = prev[&st];
                let mut c = paths[ei].clone().unwrap();
                if g.edges[ei].a != p.0 { c.reverse(); }
                segs.push(c);
                st = p;
            }
            segs.reverse();
            m.route = segs.into_iter().flatten().collect();
        }
    }
    m
}
