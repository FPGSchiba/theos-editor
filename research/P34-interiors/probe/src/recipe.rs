//! Recipes: lattice, envelope, room program, ordered steps and weighted fill rules.
use crate::graph::{Graph, Role};
use crate::rng::Rng;
use crate::rules::cycles::{ExtendArc, InsertCycle};
use crate::rules::leaves::LeafRule;
use crate::rules::main_cycle::{CycleType, MainCycle};
use crate::rules::merge::MergeHall;
use crate::rules::shortcut::Shortcut;
use crate::rules::{apply_guarded, Rule};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub enum Envelope { Rect, Circle { ragged: bool } }

#[derive(Clone, Copy, PartialEq)]
pub enum Shape { Rect, Round, Ragged }

#[derive(Clone, Copy, PartialEq)]
pub enum Space { Sparse, Dense }

pub struct TypeSpec {
    pub shape: Shape,
    /// Dense: type used for rooms covering 2+ lattice cells.
    pub hall: bool,
    /// Only exists in dense recipes (round-1 sparse results stay reproducible).
    pub dense_only: bool,
    pub name: &'static str,
    pub role: Role,
    pub count: (i32, i32),
    pub w: (i32, i32),
    pub h: (i32, i32),
    pub long: bool,
}

pub struct Fill { pub rule: Box<dyn Rule>, pub weight: u32, pub max: u32 }

pub struct Recipe {
    pub name: &'static str,
    pub space: Space,
    /// Wall thickness in raster cells (dense).
    pub wall: i32,
    pub w: i32,
    pub h: i32,
    /// Raster cells (2 m) per lattice cell (pitch).
    pub cr: i32,
    pub env: Envelope,
    pub types: Vec<TypeSpec>,
    pub steps: Vec<(Box<dyn Rule>, u32)>,
    pub fill: Vec<Fill>,
    pub target: usize,
}

#[derive(Default)]
pub struct Stats { pub applied: BTreeMap<&'static str, u32>, pub failed_steps: u32 }

fn ts(name: &'static str, role: Role, count: (i32, i32), w: (i32, i32), h: (i32, i32), long: bool) -> TypeSpec {
    TypeSpec { name, role, count, w, h, long, shape: Shape::Rect, hall: false, dense_only: false }
}
fn sh(mut t: TypeSpec, s: Shape) -> TypeSpec { t.shape = s; t }
fn hl(mut t: TypeSpec) -> TypeSpec { t.hall = true; t }
fn dn(mut t: TypeSpec) -> TypeSpec { t.dense_only = true; t }

pub fn envelope(env: Envelope, w: i32, h: i32, rng: &mut Rng) -> Vec<bool> {
    let mut v = vec![true; (w * h) as usize];
    if let Envelope::Circle { ragged } = env {
        for y in 0..h { for x in 0..w {
            let (dx, dy) = (2 * x + 1 - w, 2 * y + 1 - h);
            let d2 = dx * dx + dy * dy;
            let mut on = d2 <= w * w;
            // ragged edge: the outer ring loses cells at random
            if ragged && on && d2 > 36 * w * w / 100 && rng.pct(35) { on = false; }
            v[(y * w + x) as usize] = on;
        } }
    }
    v
}

/// Structure stage: steps, then weighted fill until the room target or no rule applies.
pub fn build_structure(r: &Recipe, g: &mut Graph, rng: &mut Rng, stats: &mut Stats) {
    for (rule, count) in &r.steps {
        for _ in 0..*count {
            let mut ok = false;
            for _ in 0..40 {
                if apply_guarded(rule.as_ref(), g, rng) { ok = true; break; }
            }
            if ok { *stats.applied.entry(rule.name()).or_default() += 1; } else { stats.failed_steps += 1; }
        }
    }
    let mut fails = 0;
    let mut used: Vec<u32> = vec![0; r.fill.len()];
    while g.nodes.len() < r.target && fails < 120 {
        let total: u32 = r.fill.iter().enumerate().filter(|(i, f)| used[*i] < f.max).map(|(_, f)| f.weight).sum();
        if total == 0 { break; }
        let mut roll = rng.below(total as usize) as u32;
        let mut pick = 0;
        for (i, f) in r.fill.iter().enumerate() {
            if used[i] >= f.max { continue; }
            if roll < f.weight { pick = i; break; }
            roll -= f.weight;
        }
        let f = &r.fill[pick];
        if apply_guarded(f.rule.as_ref(), g, rng) {
            fails = 0;
            used[pick] += 1;
            *stats.applied.entry(f.rule.name()).or_default() += 1;
        } else { fails += 1; }
    }
}

/// Room types by role; rooms covering 2+ cells (not the goal) draw from the role's hall types.
pub fn assign_types(r: &Recipe, g: &mut Graph, rng: &mut Rng) {
    for role in [Role::Start, Role::Goal, Role::Path, Role::Leaf, Role::LockedLeaf, Role::Extraction] {
        let all: Vec<&TypeSpec> = r.types.iter().filter(|t| t.role == role && (!t.dense_only || r.space == Space::Dense)).collect();
        let mut small: Vec<&TypeSpec> = all.iter().copied().filter(|t| !t.hall || r.space == Space::Sparse).collect();
        if small.is_empty() { small = all.clone(); }
        let mut big: Vec<&TypeSpec> = all.iter().copied().filter(|t| t.hall).collect();
        if big.is_empty() { big = all.clone(); }
        let mut bag: Vec<&'static str> = vec![];
        for t in &small { for _ in 0..rng.range(t.count.0, t.count.1) { bag.push(t.name); } }
        rng.shuffle(&mut bag);
        let ids: Vec<usize> = (0..g.nodes.len()).filter(|&i| g.nodes[i].role == role).collect();
        let mut k = 0;
        for &i in &ids {
            let area = g.nodes[i].size.0 * g.nodes[i].size.1;
            if area > 1 && role != Role::Goal {
                g.nodes[i].ty = rng.pick(&big).map_or("room", |t| t.name);
            } else {
                g.nodes[i].ty = if k < bag.len() { bag[k] } else if let Some(t) = rng.pick(&small) { t.name } else { "room" };
                k += 1;
            }
        }
    }
}

pub fn all() -> Vec<Recipe> {
    vec![choir(false), choir(true), mega(false), mega(true), cave(false), cave(true)]
}

fn choir(dense: bool) -> Recipe {
    let types = vec![
        ts("entrance", Role::Start, (1, 1), (6, 6), (8, 8), false),
        sh(ts("choir_hall", Role::Goal, (1, 1), (24, 28), (26, 30), false), Shape::Round),
        hl(ts("tuning_gallery", Role::Path, (2, 3), (4, 6), (14, 18), true)),
        ts("junction", Role::Path, (1, 2), (5, 7), (5, 7), false),
        ts("operator_room", Role::Path, (1, 1), (7, 9), (7, 9), false),
        sh(ts("echo_vault", Role::LockedLeaf, (1, 1), (9, 11), (9, 11), false), Shape::Round),
        sh(ts("collapsed", Role::Leaf, (1, 2), (6, 10), (6, 10), false), Shape::Ragged),
        ts("shrine_crypt", Role::Leaf, (1, 1), (6, 8), (6, 8), false),
        ts("store", Role::Leaf, (1, 2), (5, 7), (5, 7), false),
        sh(ts("extraction", Role::Extraction, (2, 2), (5, 6), (5, 6), false), Shape::Round),
    ];
    if !dense {
        return Recipe {
            name: "choir-hall-like", space: Space::Sparse, wall: 1, w: 5, h: 5, cr: 20, env: Envelope::Circle { ragged: false }, types,
            steps: vec![
                (Box::new(MainCycle { ty: CycleType::LockKey, min_arc: 1, size: (2, 2), noise: 3 }), 1),
                (Box::new(InsertCycle { lock_pct: 0, max_rooms: 3, noise: 3, close: false }), 1),
                (Box::new(LeafRule::locked_leaf()), 1),
                (Box::new(LeafRule::rim_leaf(false)), 2),
            ],
            fill: vec![
                Fill { rule: Box::new(LeafRule::leaf(false)), weight: 4, max: 99 },
                Fill { rule: Box::new(ExtendArc { max_rooms: 2, noise: 3 }), weight: 2, max: 2 },
            ],
            target: 14,
        };
    }
    Recipe {
        name: "choir-hall-like", space: Space::Dense, wall: 1, w: 5, h: 5, cr: 8, env: Envelope::Circle { ragged: false }, types,
        steps: vec![
            (Box::new(MainCycle { ty: CycleType::LockKey, min_arc: 1, size: (2, 2), noise: 2 }), 1),
            (Box::new(InsertCycle { lock_pct: 0, max_rooms: 3, noise: 2, close: true }), 1),
            (Box::new(LeafRule::locked_leaf()), 1),
            (Box::new(LeafRule::rim_leaf(true)), 2),
        ],
        fill: vec![
            Fill { rule: Box::new(LeafRule::leaf(true)), weight: 4, max: 99 },
            Fill { rule: Box::new(ExtendArc { max_rooms: 2, noise: 2 }), weight: 2, max: 2 },
            Fill { rule: Box::new(InsertCycle { lock_pct: 0, max_rooms: 3, noise: 2, close: true }), weight: 2, max: 2 },
            Fill { rule: Box::new(MergeHall { sizes: vec![(1, 2), (2, 1)] }), weight: 2, max: 2 },
        ],
        target: 14,
    }
}

fn mega(dense: bool) -> Recipe {
    let types = vec![
        ts("entrance", Role::Start, (1, 1), (8, 10), (8, 10), false),
        sh(ts("core_hall", Role::Goal, (1, 1), (36, 50), (36, 50), false), Shape::Round),
        hl(ts("gallery", Role::Path, (30, 40), (3, 6), (12, 24), true)),
        dn(hl(sh(ts("great_hall", Role::Path, (1, 1), (20, 30), (20, 30), false), Shape::Round))),
        ts("chamber", Role::Path, (30, 40), (8, 16), (8, 16), false),
        ts("junction", Role::Path, (10, 20), (5, 8), (5, 8), false),
        ts("vault", Role::LockedLeaf, (1, 1), (10, 14), (10, 14), false),
        sh(ts("cell", Role::Leaf, (10, 20), (5, 9), (5, 9), false), Shape::Ragged),
        ts("storeroom", Role::Leaf, (10, 20), (6, 10), (6, 10), false),
        sh(ts("extraction", Role::Extraction, (4, 4), (6, 8), (6, 8), false), Shape::Round),
    ];
    if !dense {
        return Recipe {
            name: "megastructure-150", space: Space::Sparse, wall: 1, w: 16, h: 16, cr: 31, env: Envelope::Rect, types,
            steps: vec![
                (Box::new(MainCycle { ty: CycleType::LockKey, min_arc: 4, size: (2, 2), noise: 5 }), 1),
                (Box::new(InsertCycle { lock_pct: 60, max_rooms: 6, noise: 4, close: false }), 6),
                (Box::new(LeafRule::locked_leaf()), 5),
                (Box::new(LeafRule::rim_leaf(false)), 4),
            ],
            fill: vec![
                Fill { rule: Box::new(ExtendArc { max_rooms: 4, noise: 4 }), weight: 4, max: 999 },
                Fill { rule: Box::new(InsertCycle { lock_pct: 0, max_rooms: 6, noise: 4, close: false }), weight: 3, max: 999 },
                Fill { rule: Box::new(LeafRule::leaf(false)), weight: 3, max: 999 },
                Fill { rule: Box::new(Shortcut { min_hops: 7 }), weight: 1, max: 12 },
            ],
            target: 150,
        };
    }
    Recipe {
        name: "megastructure-150", space: Space::Dense, wall: 1, w: 15, h: 15, cr: 8, env: Envelope::Rect, types,
        steps: vec![
            (Box::new(MainCycle { ty: CycleType::LockKey, min_arc: 4, size: (3, 3), noise: 4 }), 1),
            (Box::new(InsertCycle { lock_pct: 60, max_rooms: 6, noise: 3, close: true }), 6),
            (Box::new(LeafRule::locked_leaf()), 5),
            (Box::new(LeafRule::rim_leaf(true)), 4),
        ],
        fill: vec![
            Fill { rule: Box::new(ExtendArc { max_rooms: 4, noise: 3 }), weight: 3, max: 999 },
            Fill { rule: Box::new(InsertCycle { lock_pct: 0, max_rooms: 6, noise: 3, close: true }), weight: 3, max: 999 },
            Fill { rule: Box::new(LeafRule::leaf(true)), weight: 3, max: 999 },
            Fill { rule: Box::new(Shortcut { min_hops: 7 }), weight: 1, max: 12 },
            Fill { rule: Box::new(MergeHall { sizes: vec![(1, 2), (2, 1), (2, 2), (2, 3), (3, 2)] }), weight: 2, max: 14 },
        ],
        target: 150,
    }
}

fn cave(dense: bool) -> Recipe {
    let types = vec![
        ts("entrance", Role::Start, (1, 1), (6, 8), (6, 8), false),
        sh(ts("great_cavern", Role::Goal, (1, 1), (30, 40), (30, 40), false), Shape::Ragged),
        ts("passage", Role::Path, (3, 5), (3, 5), (10, 18), true),
        sh(ts("chamber", Role::Path, (3, 5), (8, 14), (8, 14), false), Shape::Ragged),
        dn(hl(sh(ts("long_cavern", Role::Path, (1, 1), (10, 16), (16, 24), true), Shape::Ragged))),
        sh(ts("sealed_vault", Role::LockedLeaf, (1, 1), (8, 11), (8, 11), false), Shape::Round),
        sh(ts("nook", Role::Leaf, (2, 4), (5, 9), (5, 9), false), Shape::Ragged),
        sh(ts("pit", Role::Leaf, (1, 3), (6, 10), (6, 10), false), Shape::Ragged),
        sh(ts("extraction", Role::Extraction, (2, 2), (5, 6), (5, 6), false), Shape::Round),
    ];
    if !dense {
        return Recipe {
            name: "cave-circle", space: Space::Sparse, wall: 1, w: 7, h: 7, cr: 25, env: Envelope::Circle { ragged: true }, types,
            steps: vec![
                (Box::new(MainCycle { ty: CycleType::TwoRoutes, min_arc: 2, size: (2, 2), noise: 4 }), 1),
                (Box::new(InsertCycle { lock_pct: 50, max_rooms: 4, noise: 4, close: false }), 2),
                (Box::new(LeafRule::locked_leaf()), 1),
                (Box::new(LeafRule::rim_leaf(false)), 2),
            ],
            fill: vec![
                Fill { rule: Box::new(LeafRule::leaf(false)), weight: 3, max: 999 },
                Fill { rule: Box::new(ExtendArc { max_rooms: 3, noise: 4 }), weight: 2, max: 999 },
                Fill { rule: Box::new(Shortcut { min_hops: 6 }), weight: 1, max: 2 },
            ],
            target: 22,
        };
    }
    Recipe {
        name: "cave-circle", space: Space::Dense, wall: 1, w: 7, h: 7, cr: 8, env: Envelope::Circle { ragged: true }, types,
        steps: vec![
            (Box::new(MainCycle { ty: CycleType::TwoRoutes, min_arc: 2, size: (2, 2), noise: 3 }), 1),
            (Box::new(InsertCycle { lock_pct: 50, max_rooms: 4, noise: 3, close: true }), 2),
            (Box::new(LeafRule::locked_leaf()), 1),
            (Box::new(LeafRule::rim_leaf(true)), 2),
        ],
        fill: vec![
            Fill { rule: Box::new(LeafRule::leaf(true)), weight: 3, max: 999 },
            Fill { rule: Box::new(ExtendArc { max_rooms: 3, noise: 3 }), weight: 2, max: 999 },
            Fill { rule: Box::new(InsertCycle { lock_pct: 0, max_rooms: 4, noise: 3, close: true }), weight: 2, max: 999 },
            Fill { rule: Box::new(Shortcut { min_hops: 6 }), weight: 1, max: 2 },
            Fill { rule: Box::new(MergeHall { sizes: vec![(1, 2), (2, 1), (2, 2)] }), weight: 2, max: 3 },
        ],
        target: 22,
    }
}
