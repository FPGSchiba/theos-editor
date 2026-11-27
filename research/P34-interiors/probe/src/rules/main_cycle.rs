//! `main_cycle`: start on the south rim, goal near the centre, two disjoint arcs; cycle type lock_key or two_routes.
use super::{materialise, random_path, Rule};
use crate::graph::{Cell, Graph, Kind, Role, NONE};
use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq)]
pub enum CycleType { LockKey, TwoRoutes }

pub struct MainCycle { pub ty: CycleType, pub min_arc: usize, pub size: (i32, i32), pub noise: usize }

impl MainCycle {
    fn goal_site(&self, g: &Graph) -> (Cell, (i32, i32)) {
        let (sw, sh) = self.size;
        let o = ((g.w - sw) / 2, (g.h - sh) / 2);
        let mut cells = vec![];
        for dy in 0..sh { for dx in 0..sw { cells.push((o.0 + dx, o.1 + dy)); } }
        if sw * sh > 1 && cells.iter().all(|&c| g.free(c)) { return (o, self.size); }
        let mut best = (i32::MAX, (0, 0));
        for y in 0..g.h { for x in 0..g.w {
            if !g.free((x, y)) { continue; }
            let (dx, dy) = (2 * x + 1 - g.w, 2 * y + 1 - g.h);
            if dx * dx + dy * dy < best.0 { best = (dx * dx + dy * dy, (x, y)); }
        } }
        (best.1, (1, 1))
    }
}

impl Rule for MainCycle {
    fn name(&self) -> &'static str { match self.ty { CycleType::LockKey => "main_cycle:lock_key", CycleType::TwoRoutes => "main_cycle:two_routes" } }
    fn try_apply(&self, g: &mut Graph, rng: &mut Rng) -> bool {
        if g.start != NONE { return false; }
        let (gc, size) = self.goal_site(g);
        let gcells: Vec<Cell> = (0..size.1).flat_map(|dy| (0..size.0).map(move |dx| (gc.0 + dx, gc.1 + dy))).collect();
        // rim candidates: south-most enabled cell of each column with >= 2 free neighbours
        let mut cands = vec![];
        for x in 0..g.w {
            let Some(y) = (0..g.h).rev().find(|&y| g.enabled_at((x, y))) else { continue };
            let c = (x, y);
            if gcells.contains(&c) { continue; }
            let nbs = Graph::nb4(c).iter().filter(|&&n| g.free(n) && !gcells.contains(&n)).count();
            if nbs >= 2 { cands.push(c); }
        }
        let Some(&sc) = rng.pick(&cands) else { return false };
        g.goal = g.add_node(gc, size, Role::Goal);
        g.start = g.add_node(sc, (1, 1), Role::Start);
        let n = (g.w * g.h) as usize;
        for _ in 0..200 {
            let Some(pa) = random_path(g, sc, &gcells, &vec![false; n], false, self.noise, rng) else { return false };
            let mut blocked = vec![false; n];
            for &c in &pa[1..pa.len() - 1] { blocked[g.idx(c)] = true; }
            let Some(pb) = random_path(g, sc, &gcells, &blocked, false, self.noise, rng) else { continue };
            let (ia, ib) = (pa.len() - 2, pb.len() - 2);
            let need = if self.ty == CycleType::LockKey { self.min_arc.max(1) } else { self.min_arc };
            if ia < need || ib < need { continue; }
            let (pa, pb) = if ia <= ib { (pa, pb) } else { (pb, pa) };
            let (goal, start) = (g.goal, g.start);
            let (_, ea) = materialise(g, start, goal, &pa, true);
            let (nb, eb) = materialise(g, start, goal, &pb, true);
            if self.ty == CycleType::LockKey {
                let k = g.new_key();
                let la = *ea.last().unwrap();
                g.edges[la].kind = Kind::Locked(k);
                let lb = *eb.last().unwrap();
                let e = &mut g.edges[lb];
                std::mem::swap(&mut e.a, &mut e.b);
                std::mem::swap(&mut e.ca, &mut e.cb);
                e.kind = Kind::Drop;
                let kn = nb[rng.below(nb.len())];
                g.nodes[kn].key = Some(k);
            }
            return true;
        }
        false
    }
}
