// src/graph/utils.rs
//! Generic graph algorithms shared across GaussOS components.
//! These helpers are dependency-free and operate on caller-supplied adjacency closures
//! so they work for in-memory, DB-backed or distributed graphs.

use std::collections::{HashMap, HashSet, VecDeque};

/// Breadth-first search returning the visit order as a Vec.
///
/// `start` – root node; `adjacent` – closure yielding iterator of neighbour node IDs.
pub fn bfs<N, F, I>(start: N, mut adjacent: F) -> Vec<N>
where
    N: Clone + Eq + std::hash::Hash,
    F: FnMut(&N) -> I,
    I: IntoIterator<Item = N>,
{
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    let mut queue = VecDeque::new();

    visited.insert(start.clone());
    queue.push_back(start);

    while let Some(node) = queue.pop_front() {
        order.push(node.clone());
        for neighbour in adjacent(&node) {
            if visited.insert(neighbour.clone()) {
                queue.push_back(neighbour);
            }
        }
    }
    order
}

/// Depth-first search (pre-order) returning visit order.
pub fn dfs<N, F, I>(start: N, mut adjacent: F) -> Vec<N>
where
    N: Clone + Eq + std::hash::Hash,
    F: FnMut(&N) -> I,
    I: IntoIterator<Item = N>,
{
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    dfs_inner(start, &mut adjacent, &mut visited, &mut order);
    order
}

fn dfs_inner<N, F, I>(node: N, adjacent: &mut F, visited: &mut HashSet<N>, order: &mut Vec<N>)
where
    N: Clone + Eq + std::hash::Hash,
    F: FnMut(&N) -> I,
    I: IntoIterator<Item = N>,
{
    if !visited.insert(node.clone()) {
        return;
    }
    order.push(node.clone());
    for neighbour in adjacent(&node) {
        dfs_inner(neighbour, adjacent, visited, order);
    }
}

/// Kahn's algorithm for topological sort. Returns `None` on cycles.
pub fn topo_sort<N, F, I>(nodes: &[N], mut adjacent: F) -> Option<Vec<N>>
where
    N: Clone + Eq + std::hash::Hash,
    F: FnMut(&N) -> I,
    I: IntoIterator<Item = N>,
{
    // Compute in-degree
    let mut indegree: HashMap<N, usize> = nodes.iter().cloned().map(|n| (n, 0)).collect();
    for n in nodes {
        for nb in adjacent(n) {
            if let Some(e) = indegree.get_mut(&nb) {
                *e += 1;
            }
        }
    }

    // Collect zero-in-degree nodes
    let mut queue: VecDeque<N> = indegree
        .iter()
        .filter(|(_, &v)| v == 0)
        .map(|(k, _)| k.clone())
        .collect();
    let mut order = Vec::with_capacity(nodes.len());

    while let Some(n) = queue.pop_front() {
        order.push(n.clone());
        for nb in adjacent(&n) {
            if let Some(e) = indegree.get_mut(&nb) {
                *e -= 1;
                if *e == 0 {
                    queue.push_back(nb);
                }
            }
        }
    }

    if order.len() == nodes.len() {
        Some(order)
    } else {
        None
    } // cycle if not all visited
}
