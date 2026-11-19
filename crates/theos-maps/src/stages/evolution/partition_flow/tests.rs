//! `route_steepest`, `resolve_sinks` then `partition_flow` on worlds of 100 m cells.

use libm::pow;

use super::{Exponent, PartitionFlow, PartitionFlowParams};
use crate::grid::Grid;
use crate::kernels::noise::spectral_noise;
use crate::rng::StageRng;
use crate::stage::{LayerSpec, Stage, StageError};
use crate::stages::evolution::flow::{FlowWeights, accumulate, cell_index, cell_u32, outflow};
use crate::stages::evolution::test_support::{evolve, processes};
use crate::stages::terrain::grid::GridStage;
use crate::stages::terrain::support::{run_stage, world};
use crate::world::{Layer, ValueType, World, WorldValue};

/// The side of the test grids.
const N: usize = 64;

/// Every edge drained.
const ALL_EDGES: &str = r#"["north", "east", "south", "west"]"#;

/// Both exponents.
const BOTH: [&str; 2] = ["steepest", "slope_adaptive"];

/// The recipe of the routing processes with `exponent`.
fn recipe(exponent: &str) -> String {
    format!(
        "[[process]]\nid = \"route_steepest\"\n[[process]]\nid = \"resolve_sinks\"\n\
         [[process]]\nid = \"partition_flow\"\nexponent = \"{exponent}\""
    )
}

/// A 64 x 64 world whose drained `edges` are fixed, holding smooth noise with 1 m of jitter.
fn noisy(edges: &str, seed: u64) -> World {
    let mut w = world(N, N);
    run_stage(&mut w, &GridStage, &format!("edges = {edges}")).unwrap();
    let mut rng = StageRng::for_key(seed, "smooth");
    let smooth = spectral_noise(N, N, &mut rng, 3.0, 0.0, None);
    let data = (0..N * N)
        .map(|i| 40.0 * smooth.get(i / N, i % N) + rng.range(0.0, 1.0))
        .collect();
    w.insert("height", Layer::F64(Grid::from_vec(N, N, data).unwrap()))
        .unwrap();
    w
}

/// `noisy` after the three routing processes with `exponent`.
fn routed(edges: &str, seed: u64, exponent: &str) -> World {
    let mut w = noisy(edges, seed);
    evolve(&mut w, &processes(&recipe(exponent)), 1, 2.5e4, 1).unwrap();
    w
}

fn weights(w: &World) -> &FlowWeights {
    w.value::<FlowWeights>("weights").unwrap()
}

fn stack(w: &World) -> Vec<u32> {
    w.u32("stack").unwrap().as_slice().to_vec()
}

/// A 3 x 3 world where the centre sends to N (drop `drop`) and E (drop `drop / 2`); every other
/// cell is an outlet that receives itself.
fn crossroads(drop: f64) -> World {
    let mut w = world(3, 3);
    let mut surface = [1010.0; 9];
    surface[4] = 1005.0;
    surface[1] = surface[4] - drop;
    surface[5] = surface[4] - drop / 2.0;
    let mut receivers: Vec<u32> = (0..9).collect();
    receivers[4] = 1;
    let surface = Grid::from_vec(3, 3, surface.to_vec()).unwrap();
    w.insert("routing_surface", Layer::F64(surface)).unwrap();
    let receivers = Grid::from_vec(3, 3, receivers).unwrap();
    w.insert("receivers", Layer::U32(receivers)).unwrap();
    w
}

fn partition(w: &mut World, exponent: &str) -> Result<(), StageError> {
    run_stage(w, &PartitionFlow, &format!("exponent = \"{exponent}\"")).map(|_| ())
}

/// Replaces the `receivers` of `w` with `edit` applied to them.
fn edit_receivers(w: &mut World, edit: impl FnOnce(&mut [u32])) {
    let mut rec = w.u32("receivers").unwrap().clone();
    edit(rec.as_mut_slice());
    w.insert("receivers", Layer::U32(rec)).unwrap();
}

#[test]
fn weights_sum_to_one() {
    for exponent in BOTH {
        for edges in ["[]", ALL_EDGES] {
            let w = routed(edges, 2, exponent);
            let (weights, rec) = (weights(&w), w.u32("receivers").unwrap());
            assert_eq!(weights.cells(), N * N);
            for i in 0..N * N {
                let edges = weights.edges(i);
                if cell_index(rec.as_slice()[i]) == i {
                    assert!(edges.is_empty(), "{exponent}: cell {i} receives itself");
                    continue;
                }
                let sum: f64 = edges.clone().map(|e| weights.weight[e]).sum();
                assert!(
                    (sum - 1.0).abs() < 1e-12,
                    "{exponent}: cell {i} sums to {sum}"
                );
                for e in edges {
                    let to = cell_index(weights.to[e]);
                    assert!(weights.weight[e] > 0.0 && weights.dist[e] >= 100.0);
                    let apart = to.abs_diff(i);
                    let near = apart == 1 || apart.abs_diff(N) <= 1;
                    assert!(near, "{exponent}: {i} to {to} is not a neighbour");
                }
            }
        }
    }
}

#[test]
fn steepest_sends_everything_to_the_receiver() {
    let w = routed(ALL_EDGES, 3, "steepest");
    let (weights, rec) = (weights(&w), w.u32("receivers").unwrap());
    for i in 0..N * N {
        let to = cell_index(rec.as_slice()[i]);
        if to == i {
            assert!(weights.edges(i).is_empty());
        } else {
            let edges = weights.edges(i);
            assert_eq!(edges.len(), 1, "cell {i}");
            let e = edges.start;
            assert_eq!(cell_index(weights.to[e]), to);
            assert_eq!(weights.weight[e].to_bits(), 1.0_f64.to_bits());
        }
    }
}

#[test]
fn slope_adaptive_spreads_more_on_gentle_ground() {
    // The same ratio of drops (north : east = 2 : 1), gentle and steep.
    let mut east_shares = Vec::new();
    for drop in [1.0, 500.0] {
        let mut w = crossroads(drop);
        partition(&mut w, "slope_adaptive").unwrap();
        let weights = weights(&w);
        let edges = weights.edges(4);
        assert_eq!(edges.len(), 2);
        let p = 0.5 + 0.6 * (drop / 100.0);
        let east = pow(0.5, p) / (1.0 + pow(0.5, p));
        let (north_edge, east_edge) = (edges.start, edges.start + 1);
        assert_eq!(weights.to[north_edge], 1, "D8 order: north first");
        assert_eq!(weights.to[east_edge], 5);
        assert!(
            (weights.weight[east_edge] - east).abs() < 1e-12,
            "drop {drop}"
        );
        assert!((weights.dist[north_edge] - 100.0).abs() < 1e-12);
        east_shares.push(weights.weight[east_edge]);
    }
    assert!(east_shares[0] > east_shares[1] + 0.2, "{east_shares:?}");
}

#[test]
fn a_cell_without_a_lower_neighbour_sends_everything_to_its_receiver() {
    // A pass link: the centre has no lower neighbour on the routing surface but receives north.
    let mut w = crossroads(0.0);
    let mut surface = w.f64("routing_surface").unwrap().clone();
    surface.as_mut_slice().fill(1010.0);
    w.insert("routing_surface", Layer::F64(surface)).unwrap();
    partition(&mut w, "slope_adaptive").unwrap();
    let weights = weights(&w);
    assert_eq!(weights.edges(4).len(), 1);
    let e = weights.edges(4).start;
    assert_eq!(weights.to[e], 1);
    assert_eq!(weights.weight[e].to_bits(), 1.0_f64.to_bits());
}

#[test]
fn the_stack_puts_receivers_before_donors_and_covers_every_cell() {
    for exponent in BOTH {
        for edges in ["[]", ALL_EDGES] {
            let w = routed(edges, 4, exponent);
            let (stack, weights) = (stack(&w), weights(&w));
            let mut position = vec![usize::MAX; N * N];
            for (k, &cell) in stack.iter().enumerate() {
                assert_eq!(position[cell_index(cell)], usize::MAX, "cell listed twice");
                position[cell_index(cell)] = k;
            }
            assert_eq!(stack.len(), N * N);
            assert!(position.iter().all(|&k| k < N * N));
            for i in 0..N * N {
                for e in weights.edges(i) {
                    let to = cell_index(weights.to[e]);
                    assert!(position[to] < position[i], "{exponent}");
                }
            }
            // The cells that receive themselves come first, in cell order.
            let rec = w.u32("receivers").unwrap().as_slice();
            let roots: Vec<u32> = (0..N * N)
                .filter(|&i| cell_index(rec[i]) == i)
                .map(cell_u32)
                .collect();
            assert_eq!(stack[..roots.len()], roots[..], "{exponent}");
        }
    }
}

#[test]
fn discharge_routing_loses_nothing() {
    for exponent in BOTH {
        for seed in 1..=3 {
            let w = routed(ALL_EDGES, seed, exponent);
            let mut rng = StageRng::for_key(seed, "runoff");
            let data = (0..N * N).map(|_| rng.range(0.0, 2.0)).collect();
            let runoff = Grid::from_vec(N, N, data).unwrap();
            let total: f64 = runoff.as_slice().iter().sum();
            let out = outflow(&stack(&w), weights(&w), &runoff, w.mask("fixed").unwrap());
            assert!(
                (out - total).abs() <= 1e-9 * total,
                "{exponent}, seed {seed}: {out} of {total}"
            );
        }
    }
}

#[test]
fn on_a_closed_map_the_water_gathers_in_the_sink() {
    for exponent in BOTH {
        let w = routed("[]", 5, exponent);
        let runoff = Grid::filled(N, N, 1.0);
        let (stack, weights) = (stack(&w), weights(&w));
        let out = outflow(&stack, weights, &runoff, w.mask("fixed").unwrap());
        assert_eq!(out.to_bits(), 0.0_f64.to_bits());
        let sink = cell_index(stack[0]);
        let total = accumulate(&stack, weights, &runoff).as_slice()[sink];
        assert!((total - 4096.0).abs() < 1e-9, "{exponent}: {total}");
    }
}

#[test]
fn a_cycle_of_receivers_is_a_failure_naming_a_cell() {
    let mut w = crossroads(0.0);
    edit_receivers(&mut w, |rec| {
        rec[4] = 1;
        rec[1] = 4;
    });
    let flat = Grid::filled(3, 3, 7.0);
    w.insert("routing_surface", Layer::F64(flat)).unwrap();
    let err = partition(&mut w, "steepest").unwrap_err();
    assert!(
        matches!(&err, StageError::Failed { message, .. } if message.contains("cell 1")),
        "{err}"
    );
}

#[test]
fn a_receiver_that_is_no_neighbour_is_a_failure() {
    for to in [8, 9] {
        let mut w = crossroads(1.0);
        edit_receivers(&mut w, |rec| rec[0] = to);
        let err = partition(&mut w, "steepest").unwrap_err();
        assert!(
            matches!(&err, StageError::Failed { message, .. } if message.contains("cell 0")),
            "receiver {to}: {err}"
        );
    }
}

#[test]
fn partition_flow_declares_its_layers_and_parameters() {
    let stage: &dyn Stage = &PartitionFlow;
    assert!(
        stage.params(&toml::Table::new()).is_err(),
        "the recipe states the exponent"
    );
    for bad in ["exponent = \"bogus\"", "exponent = \"steepest\"\nextra = 1"] {
        assert!(stage.params(&bad.parse().unwrap()).is_err(), "{bad}");
    }
    let params = stage
        .params(&"exponent = \"slope_adaptive\"".parse().unwrap())
        .unwrap();
    assert_eq!(
        stage.reads(params.as_ref()),
        [
            LayerSpec::f64("routing_surface"),
            LayerSpec::u32("receivers")
        ]
    );
    assert_eq!(
        stage.writes(params.as_ref()),
        [
            LayerSpec::value("weights", FlowWeights::NAME),
            LayerSpec::u32("stack")
        ]
    );
    let typed = PartitionFlowParams {
        exponent: Exponent::Steepest,
    };
    assert_eq!(typed.exponent, Exponent::Steepest);
}

#[test]
fn weights_have_a_canonical_json() {
    let w = FlowWeights {
        start: vec![0, 1, 1],
        to: vec![1],
        weight: vec![1.0],
        dist: vec![100.0],
    };
    assert_eq!(
        w.canonical_json().unwrap(),
        serde_json::json!({"start": [0, 1, 1], "to": [1], "weight": [1.0], "dist": [100.0]})
    );
    assert_eq!(w.type_name(), "weights");
}
