//! Structural reasoning analysis — computed in memory from the graph, never stored
//! (SPEC §3). This is the no-numbers replacement for confidence propagation:
//! "weakest link" = a bet/assumption that is also load-bearing.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::domain::{ChallengeStatus, EdgeType, MapGraph, NodeCriticality, NodeStatus};

#[cfg(test)]
mod tests;

/// Dependency direction: an arrow U -> V means "V depends on U" (U is upstream).
/// Rebuttals are attacks, not dependencies, so they are excluded here.
fn dependency_edges(graph: &MapGraph) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for e in &graph.edges {
        match e.edge_type {
            EdgeType::Support | EdgeType::PremiseOf => {
                out.push((e.from_node.clone(), e.to_node.clone()))
            }
            // "A depends_on B" => B is upstream of A.
            EdgeType::DependsOn => out.push((e.to_node.clone(), e.from_node.clone())),
            EdgeType::Rebut => {}
        }
    }
    out
}

/// Per-node structural criticality for the whole map.
pub fn criticality(graph: &MapGraph) -> Vec<NodeCriticality> {
    let node_ids: Vec<String> = graph.nodes.iter().map(|n| n.id.clone()).collect();
    let dep = dependency_edges(graph);

    // Downstream adjacency: U -> {V : V depends on U}.
    let mut downstream: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (u, v) in &dep {
        downstream.entry(u).or_default().push(v);
    }

    let articulation = articulation_points(&node_ids, &dep);

    // Open challenges per target.
    let mut open: BTreeMap<&str, usize> = BTreeMap::new();
    for c in &graph.challenges {
        if matches!(c.status, ChallengeStatus::Pending | ChallengeStatus::Conceded) {
            *open.entry(c.target_id.as_str()).or_default() += 1;
        }
    }

    graph
        .nodes
        .iter()
        .map(|n| {
            let downstream_count = reachable_count(&downstream, &n.id);
            let is_load_bearing = articulation.contains(n.id.as_str()) || downstream_count >= 2;
            let is_uncertain = matches!(n.status, NodeStatus::Bet | NodeStatus::Assumption);
            NodeCriticality {
                node_id: n.id.clone(),
                downstream_count,
                is_load_bearing,
                is_weak_link: is_uncertain && is_load_bearing,
                open_challenges: open.get(n.id.as_str()).copied().unwrap_or(0),
            }
        })
        .collect()
}

/// Count nodes reachable downstream from `start` (excluding itself).
fn reachable_count(downstream: &BTreeMap<&str, Vec<&str>>, start: &str) -> usize {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut q: VecDeque<&str> = VecDeque::new();
    q.push_back(start);
    while let Some(cur) = q.pop_front() {
        if let Some(next) = downstream.get(cur) {
            for &v in next {
                if seen.insert(v) {
                    q.push_back(v);
                }
            }
        }
    }
    seen.remove(start);
    seen.len()
}

/// Articulation points of the underlying UNDIRECTED dependency graph (Tarjan).
fn articulation_points(node_ids: &[String], dep: &[(String, String)]) -> BTreeSet<String> {
    let idx: BTreeMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();
    let n = node_ids.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (u, v) in dep {
        if let (Some(&a), Some(&b)) = (idx.get(u.as_str()), idx.get(v.as_str())) {
            adj[a].push(b);
            adj[b].push(a);
        }
    }

    let mut visited = vec![false; n];
    let mut disc = vec![0usize; n];
    let mut low = vec![0usize; n];
    let mut is_ap = vec![false; n];
    let mut timer = 0usize;

    // Iterative DFS to avoid stack overflow on large maps.
    for start in 0..n {
        if visited[start] {
            continue;
        }
        let mut root_children = 0usize;
        // stack of (node, parent, child-iterator-index)
        let mut stack: Vec<(usize, isize, usize)> = vec![(start, -1, 0)];
        visited[start] = true;
        disc[start] = timer;
        low[start] = timer;
        timer += 1;

        while let Some(&(u, parent, ci)) = stack.last() {
            if ci < adj[u].len() {
                stack.last_mut().unwrap().2 += 1;
                let v = adj[u][ci];
                if !visited[v] {
                    if parent == -1 {
                        root_children += 1;
                    }
                    visited[v] = true;
                    disc[v] = timer;
                    low[v] = timer;
                    timer += 1;
                    stack.push((v, u as isize, 0));
                } else if v as isize != parent {
                    low[u] = low[u].min(disc[v]);
                }
            } else {
                stack.pop();
                if let Some(&(p, _, _)) = stack.last() {
                    low[p] = low[p].min(low[u]);
                    if stack.len() >= 1 && low[u] >= disc[p] && p != start {
                        is_ap[p] = true;
                    }
                }
            }
        }
        if root_children > 1 {
            is_ap[start] = true;
        }
    }

    node_ids
        .iter()
        .enumerate()
        .filter(|(i, _)| is_ap[*i])
        .map(|(_, s)| s.clone())
        .collect()
}

/// Cycles in the dependency graph = circular reasoning (SPEC §5). Returns the node ids
/// that participate in at least one cycle.
pub fn circular_nodes(graph: &MapGraph) -> BTreeSet<String> {
    let dep = dependency_edges(graph);
    let ids: Vec<String> = graph.nodes.iter().map(|n| n.id.clone()).collect();
    let idx: BTreeMap<&str, usize> = ids.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
    let n = ids.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (u, v) in &dep {
        if let (Some(&a), Some(&b)) = (idx.get(u.as_str()), idx.get(v.as_str())) {
            adj[a].push(b);
        }
    }

    // Tarjan SCC; any SCC of size > 1 (or a self-loop) is circular.
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut counter = 0usize;
    let mut result: BTreeSet<String> = BTreeSet::new();

    for s in 0..n {
        if index[s] != usize::MAX {
            continue;
        }
        // iterative Tarjan
        let mut call: Vec<(usize, usize)> = vec![(s, 0)];
        while let Some(&(v, pi)) = call.last() {
            if pi == 0 {
                index[v] = counter;
                low[v] = counter;
                counter += 1;
                stack.push(v);
                on_stack[v] = true;
            }
            if pi < adj[v].len() {
                call.last_mut().unwrap().1 += 1;
                let w = adj[v][pi];
                if index[w] == usize::MAX {
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                if low[v] == index[v] {
                    let mut comp = Vec::new();
                    loop {
                        let w = stack.pop().unwrap();
                        on_stack[w] = false;
                        comp.push(w);
                        if w == v {
                            break;
                        }
                    }
                    let has_self_loop = adj[v].contains(&v);
                    if comp.len() > 1 || has_self_loop {
                        for w in comp {
                            result.insert(ids[w].clone());
                        }
                    }
                }
                call.pop();
                if let Some(&(p, _)) = call.last() {
                    low[p] = low[p].min(low[v]);
                }
            }
        }
    }
    result
}
