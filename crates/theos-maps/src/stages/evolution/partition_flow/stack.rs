//! The stack of `partition_flow`: the cells in an order with receivers before donors.

use std::collections::VecDeque;

use crate::stages::evolution::flow::{FlowWeights, cell_index, cell_u32};

/// Kahn's algorithm over the multiple-flow graph, starting from the cells without an edge (the
/// ones that receive themselves) in cell order, first in, first out. A cell is listed once all its
/// receivers are. Returns the first cell that no order reaches (it lies on or below a cycle).
pub(super) fn order(w: &FlowWeights) -> Result<Vec<u32>, usize> {
    let n = w.cells();
    let mut donors: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut waiting: Vec<usize> = (0..n).map(|i| w.edges(i).len()).collect();
    for i in 0..n {
        for e in w.edges(i) {
            donors[cell_index(w.to[e])].push(i);
        }
    }
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| waiting[i] == 0).collect();
    let mut stack = Vec::with_capacity(n);
    while let Some(i) = queue.pop_front() {
        stack.push(cell_u32(i));
        for &d in &donors[i] {
            waiting[d] -= 1;
            if waiting[d] == 0 {
                queue.push_back(d);
            }
        }
    }
    match waiting.iter().position(|&left| left > 0) {
        Some(cell) => Err(cell),
        None => Ok(stack),
    }
}
