//! Graph layout algorithms for <network> without explicit node positions.
//!
//! These replace the networkx layouts used by prefig/core/network.py. Because
//! networkx's exact coordinates depend on its own PRNG and iteration details,
//! these produce valid, deterministic layouts that are NOT coordinate-identical
//! to networkx (the network renderer re-centers and re-scales positions, so
//! only the relative arrangement matters). See RUST_PORT_OUTLINE.md §14.2.

use indexmap::IndexMap;
use std::collections::VecDeque;

pub type Positions = IndexMap<String, [f64; 2]>;

/// A small deterministic RNG (SplitMix64) for seeded layouts.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_add(0x9e37_79b9_7f4a_7c15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// uniform in [0, 1)
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Adjacency of a simple undirected graph over the given node order.
pub struct Graph {
    pub nodes: Vec<String>,
    index: IndexMap<String, usize>,
    adj: Vec<Vec<usize>>,
}

impl Graph {
    pub fn new(nodes: &[String], edges: &[(String, String)]) -> Graph {
        // Start from the declared nodes, then add any edge endpoint that isn't
        // already present, in first-seen order. This mirrors networkx's
        // `add_edge`, which implicitly creates missing endpoint nodes; without
        // it, a node that appears only as an edge destination (never as a graph
        // key or <node>) would get no position and later panic.
        let mut node_list: Vec<String> = nodes.to_vec();
        let mut index: IndexMap<String, usize> = node_list
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i))
            .collect();
        for endpoint in edges.iter().flat_map(|(a, b)| [a, b]) {
            if !index.contains_key(endpoint) {
                index.insert(endpoint.clone(), node_list.len());
                node_list.push(endpoint.clone());
            }
        }
        let mut adj = vec![Vec::new(); node_list.len()];
        for (a, b) in edges {
            if let (Some(&i), Some(&j)) = (index.get(a), index.get(b)) {
                if i != j && !adj[i].contains(&j) {
                    adj[i].push(j);
                    adj[j].push(i);
                }
            }
        }
        Graph {
            nodes: node_list,
            index,
            adj,
        }
    }

    fn n(&self) -> usize {
        self.nodes.len()
    }

    fn positions_from(&self, coords: Vec<[f64; 2]>) -> Positions {
        self.nodes.iter().cloned().zip(coords).collect()
    }
}

/// Nodes evenly spaced on a unit circle (matches networkx circular_layout up to
/// the shared re-centering the renderer applies).
pub fn circular(graph: &Graph) -> Positions {
    let n = graph.n();
    let coords: Vec<[f64; 2]> = (0..n)
        .map(|i| {
            let theta = 2.0 * std::f64::consts::PI * i as f64 / n.max(1) as f64;
            [theta.cos(), theta.sin()]
        })
        .collect();
    graph.positions_from(coords)
}

/// Uniform random positions in the unit square (seeded, deterministic).
pub fn random(graph: &Graph, seed: u64) -> Positions {
    let mut rng = Rng::new(seed);
    let coords: Vec<[f64; 2]> = (0..graph.n()).map(|_| [rng.unit(), rng.unit()]).collect();
    graph.positions_from(coords)
}

/// Fruchterman–Reingold force-directed layout (seeded initial positions).
pub fn spring(graph: &Graph, seed: u64) -> Positions {
    let n = graph.n();
    if n == 0 {
        return Positions::new();
    }
    let mut rng = Rng::new(seed);
    let mut pos: Vec<[f64; 2]> = (0..n)
        .map(|_| [rng.unit() - 0.5, rng.unit() - 0.5])
        .collect();

    let k = (1.0 / n as f64).sqrt(); // optimal distance
    let iterations = 50;
    let mut temp = 0.1;
    let cooling = temp / (iterations + 1) as f64;

    for _ in 0..iterations {
        let mut disp = vec![[0.0f64; 2]; n];
        // repulsive forces between all pairs
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let d = [pos[i][0] - pos[j][0], pos[i][1] - pos[j][1]];
                let dist = (d[0] * d[0] + d[1] * d[1]).sqrt().max(0.01);
                let force = k * k / dist;
                disp[i][0] += d[0] / dist * force;
                disp[i][1] += d[1] / dist * force;
            }
        }
        // attractive forces along edges
        for i in 0..n {
            for &j in &graph.adj[i] {
                let d = [pos[i][0] - pos[j][0], pos[i][1] - pos[j][1]];
                let dist = (d[0] * d[0] + d[1] * d[1]).sqrt().max(0.01);
                let force = dist * dist / k;
                disp[i][0] -= d[0] / dist * force;
                disp[i][1] -= d[1] / dist * force;
            }
        }
        // limit displacement by the temperature
        for i in 0..n {
            let len = (disp[i][0] * disp[i][0] + disp[i][1] * disp[i][1])
                .sqrt()
                .max(0.01);
            pos[i][0] += disp[i][0] / len * len.min(temp);
            pos[i][1] += disp[i][1] / len * len.min(temp);
        }
        temp -= cooling;
    }
    graph.positions_from(pos)
}

/// Spectral layout: coordinates from the eigenvectors of the graph Laplacian
/// for the two smallest nonzero eigenvalues.
pub fn spectral(graph: &Graph) -> Positions {
    let n = graph.n();
    if n < 2 {
        return circular(graph);
    }
    // Laplacian L = D - A
    let mut l = vec![vec![0.0f64; n]; n];
    for (i, row) in l.iter_mut().enumerate() {
        row[i] = graph.adj[i].len() as f64;
        for &j in &graph.adj[i] {
            row[j] -= 1.0;
        }
    }
    let (evals, evecs) = jacobi_eigen(l);
    // sort eigenvalue indices ascending; skip the smallest (the constant vector)
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        evals[a]
            .partial_cmp(&evals[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let ax = order[1.min(n - 1)];
    let ay = order[2.min(n - 1)];
    let coords: Vec<[f64; 2]> = (0..n).map(|i| [evecs[i][ax], evecs[i][ay]]).collect();
    graph.positions_from(coords)
}

/// BFS layout: x is the BFS layer from `start`, y spreads nodes within a layer.
pub fn bfs(graph: &Graph, start: &str) -> Option<Positions> {
    let &start_idx = graph.index.get(start)?;
    let n = graph.n();
    let mut layer = vec![usize::MAX; n];
    layer[start_idx] = 0;
    let mut queue = VecDeque::from([start_idx]);
    let mut max_layer = 0;
    while let Some(i) = queue.pop_front() {
        for &j in &graph.adj[i] {
            if layer[j] == usize::MAX {
                layer[j] = layer[i] + 1;
                max_layer = max_layer.max(layer[j]);
                queue.push_back(j);
            }
        }
    }
    // unreached nodes go one layer past the deepest
    for l in layer.iter_mut() {
        if *l == usize::MAX {
            *l = max_layer + 1;
        }
    }
    let max_layer = layer.iter().copied().max().unwrap_or(0);

    let mut coords = vec![[0.0f64; 2]; n];
    for target in 0..=max_layer {
        let in_layer: Vec<usize> = (0..n).filter(|&i| layer[i] == target).collect();
        let count = in_layer.len();
        for (k, &i) in in_layer.iter().enumerate() {
            let y = if count > 1 {
                k as f64 / (count - 1) as f64 - 0.5
            } else {
                0.0
            };
            coords[i] = [target as f64, y];
        }
    }
    Some(graph.positions_from(coords))
}

/// Bipartite layout: the given set in one column, the rest in another.
pub fn bipartite(graph: &Graph, set: &[String], horizontal: bool) -> Positions {
    let n = graph.n();
    let in_set: Vec<bool> = graph.nodes.iter().map(|node| set.contains(node)).collect();
    let left: Vec<usize> = (0..n).filter(|&i| in_set[i]).collect();
    let right: Vec<usize> = (0..n).filter(|&i| !in_set[i]).collect();

    let mut coords = vec![[0.0f64; 2]; n];
    let place = |coords: &mut [[f64; 2]], col: &[usize], x: f64| {
        let count = col.len();
        for (k, &i) in col.iter().enumerate() {
            let t = if count > 1 {
                k as f64 / (count - 1) as f64 - 0.5
            } else {
                0.0
            };
            coords[i] = if horizontal { [x, t] } else { [t, x] };
        }
    };
    place(&mut coords, &left, 0.0);
    place(&mut coords, &right, 1.0);
    graph.positions_from(coords)
}

/// Jacobi eigenvalue algorithm for a small symmetric matrix. Returns
/// (eigenvalues, eigenvectors) where `evecs[i][j]` is component i of
/// eigenvector j.
// A dense matrix kernel; row/column index arithmetic (a[i][p], a[p][i]) reads
// far clearer than iterator adaptors, so keep the explicit index loops.
#[allow(clippy::needless_range_loop)]
fn jacobi_eigen(mut a: Vec<Vec<f64>>) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = a.len();
    let mut v = vec![vec![0.0; n]; n];
    for (i, row) in v.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for _ in 0..100 {
        // find the largest off-diagonal element
        let mut p = 0;
        let mut q = 1;
        let mut max = 0.0;
        for i in 0..n {
            for j in i + 1..n {
                if a[i][j].abs() > max {
                    max = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if max < 1e-12 {
            break;
        }
        let theta = if (a[q][q] - a[p][p]).abs() < 1e-30 {
            std::f64::consts::FRAC_PI_4
        } else {
            0.5 * (2.0 * a[p][q] / (a[q][q] - a[p][p])).atan()
        };
        let (s, c) = theta.sin_cos();
        for i in 0..n {
            let aip = a[i][p];
            let aiq = a[i][q];
            a[i][p] = c * aip - s * aiq;
            a[i][q] = s * aip + c * aiq;
        }
        for i in 0..n {
            let api = a[p][i];
            let aqi = a[q][i];
            a[p][i] = c * api - s * aqi;
            a[q][i] = s * api + c * aqi;
        }
        for i in 0..n {
            let vip = v[i][p];
            let viq = v[i][q];
            v[i][p] = c * vip - s * viq;
            v[i][q] = s * vip + c * viq;
        }
    }
    let evals: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
    (evals, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|x| x.to_string()).collect()
    }

    fn e(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    // A graph dict like {0:[1,2]} registers only the keys as nodes; 1 and 2
    // appear solely as edge destinations. networkx's add_edge would create
    // them, and so must we -- otherwise they get no layout position and the
    // network handler panics indexing `positions[destination]`.
    #[test]
    fn new_registers_destination_only_endpoints() {
        let graph = Graph::new(&s(&["0"]), &e(&[("0", "1"), ("0", "2")]));
        assert_eq!(graph.n(), 3);
        for node in ["0", "1", "2"] {
            assert!(graph.index.contains_key(node), "missing node {node}");
        }
        // and every endpoint gets a bfs position
        let positions = bfs(&graph, "0").expect("bfs from 0");
        for node in ["0", "1", "2"] {
            assert!(positions.contains_key(node), "no position for {node}");
        }
    }

    // New endpoints are appended in first-seen order, after the declared nodes,
    // matching how networkx grows the node list as edges are added.
    #[test]
    fn destination_endpoints_keep_first_seen_order() {
        let graph = Graph::new(&s(&["0", "1"]), &e(&[("1", "3"), ("0", "2")]));
        assert_eq!(graph.nodes, s(&["0", "1", "3", "2"]));
    }
}
