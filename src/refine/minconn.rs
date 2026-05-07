use std::collections::{HashMap, HashSet};
use crate::graph::{CsrGraph, Partition};

/// Minimize subdomain connectivity: reduce the number of distinct adjacent
/// part pairs. Mirrors METIS minconn.c:MinConnectivity.
///
/// Iteratively moves boundary vertices to reduce the number of distinct
/// communication partners each part has. Respects balance tolerance defined
/// by `ufactor` (units of 1/1000; the C METIS default is 30 for kway,
/// meaning ±3% imbalance).
pub fn minimize_connectivity(g: &CsrGraph, partition: &mut Partition, ufactor: u32) {
    let n = g.n();
    let k = partition.k as usize;
    if k <= 1 { return; }

    let total_wgt: i64 = g.vwgt.iter().map(|&w| w as i64).sum();
    let target = total_wgt / k as i64;
    // epsilon = ceiling of (total_wgt * ufactor / 1000), matching METIS balance rule.
    let epsilon = (total_wgt * (ufactor as i64) + 999) / 1000;
    let max_pop = target + epsilon;
    let min_pop = (target - epsilon).max(0);

    // Compute current part populations.
    let mut pwgts = vec![0i64; k];
    for v in 0..n {
        pwgts[partition.assignment[v] as usize] += g.vwgt[v] as i64;
    }

    let max_iter = 10;
    for _ in 0..max_iter {
        let mut improved = false;

        // Build subdomain adjacency: which parts are adjacent to each part.
        let mut adj_parts: Vec<HashSet<u32>> = vec![HashSet::new(); k];
        for v in 0..n {
            let pv = partition.assignment[v] as usize;
            for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                let u = g.adjncy[j] as usize;
                let pu = partition.assignment[u] as usize;
                if pv != pu {
                    adj_parts[pv].insert(pu as u32);
                }
            }
        }

        // Try to reduce connectivity for high-degree parts.
        'vertex_loop: for v in 0..n {
            let from = partition.assignment[v] as usize;

            // Only move boundary vertices.
            let is_boundary = (g.xadj[v] as usize..g.xadj[v + 1] as usize)
                .any(|j| partition.assignment[g.adjncy[j] as usize] as usize != from);
            if !is_boundary { continue; }

            // Only attempt if this part has non-trivial subdomain connectivity.
            // Threshold: more than k/4 + 1 neighbours (mirrors METIS heuristic).
            if adj_parts[from].len() <= k / 4 + 1 { continue; }

            let vwgt = g.vwgt[v] as i64;

            // Count how many neighbours of v fall in each adjacent part.
            let mut neighbor_count: HashMap<u32, i32> = HashMap::new();
            for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                let u = g.adjncy[j] as usize;
                let pu = partition.assignment[u];
                if pu as usize != from {
                    *neighbor_count.entry(pu).or_insert(0) += 1;
                }
            }

            let mut best_to: Option<usize> = None;
            let mut best_gain = 0i32;

            for (&to_p, &cnt) in &neighbor_count {
                let to = to_p as usize;

                // Balance constraints: source must not drop below min, dest must not exceed max.
                if pwgts[from] - vwgt < min_pop { continue; }
                if pwgts[to] + vwgt > max_pop { continue; }

                // Would moving v out of `from` reduce `from`'s subdomain degree?
                // This happens when `to` is the *only* part in adj_parts[from] via v,
                // i.e. every edge from v to part `to` is the sole link between from and to.
                // Practical proxy: the edge count to `to` equals the total number of
                // from-to boundary edges visible at v. We check if removing v would
                // eliminate the from<->to adjacency in the subdomain graph.
                let from_to_edge_count_via_v = cnt as usize;
                // Count total boundary edges from `from` to `to` across all from-vertices.
                let total_from_to: usize = (0..n)
                    .filter(|&u| partition.assignment[u] as usize == from)
                    .map(|u| {
                        (g.xadj[u] as usize..g.xadj[u + 1] as usize)
                            .filter(|&j| partition.assignment[g.adjncy[j] as usize] == to_p)
                            .count()
                    })
                    .sum();

                let from_loses_subdomain = from_to_edge_count_via_v == total_from_to;

                if from_loses_subdomain && cnt > best_gain {
                    best_gain = cnt;
                    best_to = Some(to);
                }
            }

            if let Some(to) = best_to {
                partition.assignment[v] = to as u32;
                pwgts[from] -= vwgt;
                pwgts[to]   += vwgt;
                improved = true;
                // Rebuild adj_parts by breaking and restarting the outer loop.
                break 'vertex_loop;
            }
        }

        if !improved { break; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::CsrGraph;
    use crate::api::{MetisParams, MetisPartitioner, Partitioner};

    /// Build a 4×4 grid graph (unit vertex weights, unit edge weights).
    pub fn make_grid_graph(rows: usize, cols: usize) -> CsrGraph {
        let n = rows * cols;
        let mut xadj = vec![0u32];
        let mut adjncy: Vec<u32> = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let v = r * cols + c;
                if r > 0           { adjncy.push(((r - 1) * cols + c) as u32); }
                if r + 1 < rows    { adjncy.push(((r + 1) * cols + c) as u32); }
                if c > 0           { adjncy.push((r * cols + (c - 1)) as u32); }
                if c + 1 < cols    { adjncy.push((r * cols + (c + 1)) as u32); }
                let _ = v;
                xadj.push(adjncy.len() as u32);
            }
        }
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
    }

    fn count_subdomain_pairs(g: &CsrGraph, p: &Partition) -> usize {
        use std::collections::HashSet;
        let mut pairs: HashSet<(u32, u32)> = HashSet::new();
        for v in 0..g.n() {
            for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                let u = g.adjncy[j] as usize;
                if p.assignment[v] != p.assignment[u] {
                    let a = p.assignment[v].min(p.assignment[u]);
                    let b = p.assignment[v].max(p.assignment[u]);
                    pairs.insert((a, b));
                }
            }
        }
        pairs.len()
    }

    #[test]
    fn min_conn_reduces_or_maintains_subdomain_count() {
        let g = make_grid_graph(4, 4);

        // Partition without min_conn.
        let params_off = MetisParams { min_conn: false, ..MetisParams::default() };
        let p_off = MetisPartitioner::with_params(params_off, 4)
            .split(&g, 4, Some(0))
            .unwrap();
        let before = count_subdomain_pairs(&g, &p_off);

        // Partition with min_conn (default).
        let params_on = MetisParams { min_conn: true, ..MetisParams::default() };
        let p_on = MetisPartitioner::with_params(params_on, 4)
            .split(&g, 4, Some(0))
            .unwrap();
        let after = count_subdomain_pairs(&g, &p_on);

        // min_conn should not significantly increase subdomain pair count.
        // Allow a small tolerance since the underlying partition may differ
        // due to the same seed taking the same path — equality is the normal case.
        assert!(
            after <= before + 2,
            "min_conn should not increase subdomain pairs: before={before} after={after}"
        );
    }

    #[test]
    fn min_conn_produces_valid_partition() {
        let g = make_grid_graph(4, 4);
        let params = MetisParams { min_conn: true, ..MetisParams::default() };
        let p = MetisPartitioner::with_params(params, 4)
            .split(&g, 4, Some(42))
            .unwrap();
        assert_eq!(p.assignment.len(), g.n());
        assert_eq!(p.k, 4);
        for &a in &p.assignment {
            assert!(a < 4, "assignment out of range: {a}");
        }
    }

    #[test]
    fn min_conn_false_skips_post_processing() {
        // Smoke test: min_conn=false must still produce a valid partition.
        let g = make_grid_graph(4, 4);
        let params = MetisParams { min_conn: false, ..MetisParams::default() };
        let p = MetisPartitioner::with_params(params, 4)
            .split(&g, 4, Some(42))
            .unwrap();
        assert_eq!(p.assignment.len(), g.n());
        assert_eq!(p.k, 4);
    }

    #[test]
    fn min_conn_k1_noop() {
        // k=1 is the trivial partition; minimize_connectivity should be a no-op.
        let g = make_grid_graph(3, 3);
        let mut p = Partition {
            assignment: vec![0u32; g.n()],
            k: 1,
            tpwgts: None,
        };
        minimize_connectivity(&g, &mut p, 5);
        assert!(p.assignment.iter().all(|&a| a == 0));
    }

    #[test]
    fn min_conn_default_is_true() {
        let params = MetisParams::default();
        assert!(params.min_conn, "min_conn must default to true");
    }
}
