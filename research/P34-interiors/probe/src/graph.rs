//! Lattice graph: nodes sit on lattice cells and never move; edges join orthogonal neighbours.
use std::collections::VecDeque;

pub type Cell = (i32, i32);
pub const NONE: usize = usize::MAX;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role { Start, Goal, Path, Leaf, LockedLeaf, Extraction }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind { Open, Locked(u8), Drop }

#[derive(Clone, Debug)]
pub struct Node {
    pub cell: Cell,
    /// Footprint in lattice cells (w, h); `cell` is the top-left cell.
    pub size: (i32, i32),
    pub role: Role,
    pub ty: &'static str,
    pub key: Option<u8>,
}

/// `Drop` edges go `a` -> `b` only. `ca` lies in node `a`, `cb` in node `b`.
#[derive(Clone, Debug)]
pub struct Edge { pub a: usize, pub b: usize, pub ca: Cell, pub cb: Cell, pub kind: Kind, pub main: bool }

impl Edge {
    pub fn pass(&self, from: usize, keys: u64) -> Option<usize> {
        let (to, fwd) = if self.a == from { (self.b, true) } else if self.b == from { (self.a, false) } else { return None };
        match self.kind {
            Kind::Open => Some(to),
            Kind::Locked(k) => if keys >> k & 1 == 1 { Some(to) } else { None },
            Kind::Drop => if fwd { Some(to) } else { None },
        }
    }
}

#[derive(Clone)]
pub struct Graph {
    pub w: i32,
    pub h: i32,
    pub enabled: Vec<bool>,
    pub occ: Vec<usize>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub keys: u8,
    pub start: usize,
    pub goal: usize,
    pub rollbacks: u32,
}

impl Graph {
    pub fn new(w: i32, h: i32, enabled: Vec<bool>) -> Self {
        Graph { w, h, occ: vec![NONE; (w * h) as usize], enabled, nodes: vec![], edges: vec![], keys: 0, start: NONE, goal: NONE, rollbacks: 0 }
    }
    pub fn inb(&self, c: Cell) -> bool { c.0 >= 0 && c.1 >= 0 && c.0 < self.w && c.1 < self.h }
    pub fn idx(&self, c: Cell) -> usize { (c.1 * self.w + c.0) as usize }
    pub fn cell_of(&self, i: usize) -> Cell { ((i as i32) % self.w, (i as i32) / self.w) }
    pub fn enabled_at(&self, c: Cell) -> bool { self.inb(c) && self.enabled[self.idx(c)] }
    pub fn free(&self, c: Cell) -> bool { self.enabled_at(c) && self.occ[self.idx(c)] == NONE }
    pub fn nb4(c: Cell) -> [Cell; 4] { [(c.0, c.1 - 1), (c.0 + 1, c.1), (c.0, c.1 + 1), (c.0 - 1, c.1)] }
    pub fn node_cells(&self, id: usize) -> Vec<Cell> {
        let n = &self.nodes[id];
        let (x, y) = n.cell;
        let mut v = vec![];
        for dy in 0..n.size.1 { for dx in 0..n.size.0 { v.push((x + dx, y + dy)); } }
        v
    }
    pub fn add_node(&mut self, cell: Cell, size: (i32, i32), role: Role) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node { cell, size, role, ty: "", key: None });
        for c in self.node_cells(id) { let i = self.idx(c); self.occ[i] = id; }
        id
    }
    pub fn add_edge(&mut self, a: usize, b: usize, ca: Cell, cb: Cell, kind: Kind, main: bool) -> usize {
        self.edges.push(Edge { a, b, ca, cb, kind, main });
        self.edges.len() - 1
    }
    pub fn new_key(&mut self) -> u8 { self.keys += 1; self.keys - 1 }
    /// (own cell, free neighbour cell) pairs of node `u`, sorted by lattice coordinates.
    pub fn slots(&self, u: usize) -> Vec<(Cell, Cell)> {
        let mut v = vec![];
        for c in self.node_cells(u) { for nb in Self::nb4(c) { if self.free(nb) { v.push((c, nb)); } } }
        v.sort_by_key(|&((ax, ay), (bx, by))| (ay, ax, by, bx));
        v
    }
    pub fn connected(&self, a: usize, b: usize) -> bool {
        self.edges.iter().any(|e| (e.a == a && e.b == b) || (e.a == b && e.b == a))
    }
    pub fn adj(&self) -> Vec<Vec<usize>> {
        let mut adj = vec![vec![]; self.nodes.len()];
        for (i, e) in self.edges.iter().enumerate() { adj[e.a].push(i); adj[e.b].push(i); }
        adj
    }

    /// Key fixed point: flood through passable edges, add keys found, repeat.
    pub fn reach(&self, from: usize, keys0: u64, skip_drops: bool) -> (Vec<bool>, u64) {
        let adj = self.adj();
        let mut keys = keys0;
        loop {
            let mut vis = vec![false; self.nodes.len()];
            let mut q = VecDeque::new();
            vis[from] = true;
            q.push_back(from);
            let mut found = keys;
            while let Some(u) = q.pop_front() {
                if let Some(k) = self.nodes[u].key { found |= 1u64 << k; }
                for &ei in &adj[u] {
                    let e = &self.edges[ei];
                    if skip_drops && e.kind == Kind::Drop { continue; }
                    if let Some(v) = e.pass(u, keys) { if !vis[v] { vis[v] = true; q.push_back(v); } }
                }
            }
            if found == keys { return (vis, keys); }
            keys = found;
        }
    }

    /// Hop distance ignoring locks; `directed` respects one-way edges.
    pub fn hops(&self, from: usize, directed: bool) -> Vec<u32> {
        let adj = self.adj();
        let mut d = vec![u32::MAX; self.nodes.len()];
        let mut q = VecDeque::new();
        d[from] = 0;
        q.push_back(from);
        while let Some(u) = q.pop_front() {
            for &ei in &adj[u] {
                let e = &self.edges[ei];
                let v = if e.a == u { e.b } else { e.a };
                if directed && e.kind == Kind::Drop && e.a != u { continue; }
                if d[v] == u32::MAX { d[v] = d[u] + 1; q.push_back(v); }
            }
        }
        d
    }

    /// Solvability on the room graph (no start yet = trivially fine).
    pub fn check(&self) -> Result<(), String> {
        if self.start == NONE || self.goal == NONE { return Ok(()); }
        let (vis, _) = self.reach(self.start, 0, false);
        if let Some(i) = vis.iter().position(|v| !v) { return Err(format!("room {i} unreachable")); }
        let (_, k0) = self.reach(self.start, 0, true);
        for e in self.edges.iter().filter(|e| e.kind == Kind::Drop) {
            let (v, _) = self.reach(e.b, k0, false);
            if !v[self.goal] { return Err(format!("dead end after drop into room {}", e.b)); }
        }
        Ok(())
    }

    pub fn canonical(&self) -> String {
        let mut s = String::new();
        for (i, n) in self.nodes.iter().enumerate() {
            s += &format!("N {i} {} {} {:?} {} {:?} {:?}\n", n.cell.0, n.cell.1, n.role, n.ty, n.key, n.size);
        }
        let mut es: Vec<String> = self.edges.iter().map(|e| format!("E {} {} {:?} {:?} {:?} {}", e.a, e.b, e.ca, e.cb, e.kind, e.main)).collect();
        es.sort();
        s + &es.join("\n")
    }
}
