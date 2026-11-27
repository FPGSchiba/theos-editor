//! Throwaway probe: Unexplored-style cyclic interiors on a coarse lattice (P34 phase-spec evidence).
//! Round 2 adds `space = dense` next to round 1's `sparse`.
mod analysis;
mod dense;
mod graph;
mod picture;
mod recipe;
mod rng;
mod rules;
mod space;

use recipe::{Recipe, Space, Stats};
use sha2::{Digest, Sha256};
use std::time::Instant;

struct Run {
    g: graph::Graph,
    ras: space::Raster,
    m: analysis::Metrics,
    stats: Stats,
    rooms_hash: String,
    graph_hash: String,
    ms: f64,
    enabled: usize,
}

fn hex(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    h.finalize().iter().map(|x| format!("{x:02x}")).collect::<String>()[..16].to_string()
}

fn run(r: &Recipe, seed: u64) -> Run {
    let t = Instant::now();
    let mut rng = rng::Rng::new(seed);
    let enabled = recipe::envelope(r.env, r.w, r.h, &mut rng);
    let en = enabled.iter().filter(|&&e| e).count();
    let mut g = graph::Graph::new(r.w, r.h, enabled);
    let mut stats = Stats::default();
    recipe::build_structure(r, &mut g, &mut rng, &mut stats);
    recipe::assign_types(r, &mut g, &mut rng);
    let ras = space::layout(r, &g, &mut rng);
    let mut m = analysis::analyse(&g, &ras, en * (r.cr * r.cr) as usize);
    if g.start == graph::NONE { m.fails.push("no main cycle".into()); }
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    let rb: Vec<u8> = ras.rooms.iter().flat_map(|v| v.to_le_bytes()).collect();
    Run { rooms_hash: hex(&rb), graph_hash: hex(g.canonical().as_bytes()), g, ras, m, stats, ms, enabled: en }
}

fn main() {
    std::fs::create_dir_all("out/dense").ok();
    let verbose = std::env::args().any(|a| a == "-v");
    for r in recipe::all() {
        let mode = if r.space == Space::Dense { "dense" } else { "sparse" };
        println!("== {} [{mode}] {}x{} lattice, pitch {} m, floor {} m, target {}", r.name, r.w, r.h, r.cr * 2, r.w * r.cr * 2, r.target);
        println!("seed rooms conn cyc lock 1way dead% foot% hall% crit_m sight_m ms  cells/room det rooms_hash graph_hash  fails");
        let (mut tot, mut foot, mut reached, mut hall) = (0.0, 0.0, 0, 0.0);
        for seed in 1..=8u64 {
            let a = run(&r, seed);
            let b = run(&r, seed);
            let det = a.rooms_hash == b.rooms_hash && a.graph_hash == b.graph_hash;
            tot += a.ms;
            let m = &a.m;
            foot += m.footprint_pct;
            hall += m.hall_share_pct;
            if m.rooms >= r.target { reached += 1; }
            println!(
                "{seed:>4} {:>5} {:>4} {:>3} {:>4} {:>4} {:>5.1} {:>5.1} {:>5.1} {:>6.0} {:>7.0} {:>6.1} {:>6.2} {} {} {}  {}",
                m.rooms, m.connections, m.cycles, m.locks, m.one_way, m.dead_end_pct, m.footprint_pct, m.hall_share_pct, m.critical_m, m.sightline_m,
                a.ms, a.enabled as f64 / m.rooms as f64, if det { "equal" } else { "DIFF" }, a.rooms_hash, a.graph_hash,
                if m.fails.is_empty() { "ok".to_string() } else { m.fails.join("; ") }
            );
            if seed <= 2 {
                let p = if r.space == Space::Dense { format!("out/dense/{}-seed{seed}.png", r.name) } else { format!("out/{}-seed{seed}.png", r.name) };
                if let Err(e) = picture::write(&p, &a.g, &a.ras, &a.m, r.cr) { println!("png failed: {e}"); }
            }
            if verbose || seed == 1 { println!("     rules (seed {seed}): {:?} failed_steps={} rollbacks={}", a.stats.applied, a.stats.failed_steps, a.g.rollbacks); }
        }
        println!("reached target {reached}/8; mean footprint {:.1}%; mean hall share {:.1}%; mean runtime {:.1} ms", foot / 8.0, hall / 8.0, tot / 8.0);
    }
}
