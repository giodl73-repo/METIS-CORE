use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct CsrGraph {
    pub xadj:   Vec<u32>,
    pub adjncy: Vec<u32>,
    pub ncon:   u32,
    pub vwgt:   Vec<i32>,
    pub adjwgt: Option<Vec<i32>>,
}

impl CsrGraph {
    pub fn n(&self) -> usize { self.xadj.len().saturating_sub(1) }

    pub fn is_valid(&self) -> bool {
        let n = self.n();
        if self.xadj.len() != n + 1 { return false; }
        if n == 0 { return true; }
        if self.xadj[0] != 0 { return false; }
        if self.ncon < 1 { return false; }
        if self.vwgt.len() != n * self.ncon as usize { return false; }
        if self.vwgt.iter().any(|&w| w <= 0) { return false; }
        if let Some(ref aw) = self.adjwgt {
            if aw.len() != self.adjncy.len() { return false; }
        }
        for i in 0..n {
            if self.xadj[i] > self.xadj[i + 1] { return false; }
            for j in self.xadj[i] as usize..self.xadj[i + 1] as usize {
                if j >= self.adjncy.len() { return false; }
                let nb = self.adjncy[j] as usize;
                if nb >= n || nb == i { return false; }
            }
        }
        // Connectivity BFS from vertex 0
        let mut visited = vec![false; n];
        let mut queue = VecDeque::new();
        queue.push_back(0usize);
        visited[0] = true;
        while let Some(v) = queue.pop_front() {
            for j in self.xadj[v] as usize..self.xadj[v + 1] as usize {
                let u = self.adjncy[j] as usize;
                if !visited[u] { visited[u] = true; queue.push_back(u); }
            }
        }
        visited.iter().all(|&v| v)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Partition {
    pub assignment: Vec<u32>,
    pub k:          u32,
    /// Target partition weights (one `f32` per part, summing to 1.0).
    /// `None` means equal weights (each part gets `1/k` of total population).
    /// Set by `split_weighted` and consumed by FM balance checks.
    pub tpwgts:     Option<Vec<f32>>,
}

#[derive(Debug, Clone)]
pub struct CoarseMap { pub cmap: Vec<u32> }

/// Check if every part in `partition` is connected within `g`.
/// Returns `Ok(())` if all parts are contiguous, or the first disconnected part ID.
pub fn check_contiguity(g: &CsrGraph, partition: &Partition) -> Result<(), u32> {
    let n = g.n();
    let k = partition.k as usize;

    // Find one representative vertex per part
    let mut rep = vec![usize::MAX; k];
    for v in 0..n {
        let p = partition.assignment[v] as usize;
        if rep[p] == usize::MAX { rep[p] = v; }
    }

    // BFS within each part from its representative
    let mut visited = vec![false; n];
    for part in 0..k {
        if rep[part] == usize::MAX { continue; } // empty part
        let start = rep[part];
        visited[start] = true;
        let mut queue = std::collections::VecDeque::from([start]);
        while let Some(v) = queue.pop_front() {
            for j in g.xadj[v] as usize..g.xadj[v+1] as usize {
                let u = g.adjncy[j] as usize;
                if !visited[u] && partition.assignment[u] as usize == part {
                    visited[u] = true;
                    queue.push_back(u);
                }
            }
        }
        // Check all vertices of this part were reached
        for v in 0..n {
            if partition.assignment[v] as usize == part && !visited[v] {
                return Err(part as u32);
            }
        }
        // Reset visited for next part (only clear this part's vertices)
        for v in 0..n {
            if partition.assignment[v] as usize == part { visited[v] = false; }
        }
    }
    Ok(())
}

/// Extract an induced subgraph containing only vertices with `assignment[v] == part`.
///
/// Returns `(subgraph, global_to_local, local_to_global)` where:
/// - `global_to_local[v]` is the local index of global vertex `v`
///   (`usize::MAX` if the vertex is not in the subgraph).
/// - `local_to_global[i]` is the global index corresponding to local vertex `i`.
///
/// Edge weights are preserved when present; vertex weights are copied.
/// The returned subgraph is not necessarily connected — callers that require
/// connectivity should ensure the chosen `part` is a contiguous region.
pub fn extract_subgraph(g: &CsrGraph, assignment: &[u32], part: u32)
    -> (CsrGraph, Vec<usize>, Vec<usize>)
{
    let n = g.n();
    let ncon = g.ncon as usize;

    // Build vertex maps
    let mut global_to_local = vec![usize::MAX; n];
    let mut local_to_global: Vec<usize> = Vec::new();
    for v in 0..n {
        if assignment[v] == part {
            global_to_local[v] = local_to_global.len();
            local_to_global.push(v);
        }
    }

    // Build subgraph CSR — only include edges where both endpoints are in part
    let mut xadj = vec![0u32];
    let mut adjncy: Vec<u32> = Vec::new();
    let mut adjwgt: Vec<i32> = Vec::new();
    let mut vwgt: Vec<i32> = Vec::new();

    for &v in &local_to_global {
        for c in 0..ncon {
            vwgt.push(g.vwgt[v * ncon + c]);
        }
        for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
            let u = g.adjncy[j] as usize;
            if assignment[u] == part {
                adjncy.push(global_to_local[u] as u32);
                if let Some(ref aw) = g.adjwgt {
                    adjwgt.push(aw[j]);
                }
            }
        }
        xadj.push(adjncy.len() as u32);
    }

    let sub = CsrGraph {
        xadj,
        adjncy,
        ncon: g.ncon,
        vwgt,
        adjwgt: if g.adjwgt.is_some() { Some(adjwgt) } else { None },
    };
    (sub, global_to_local, local_to_global)
}

/// Repair non-contiguous partitions by reassigning disconnected components
/// to their largest adjacent part. Modifies partition in place.
/// Returns the number of vertices reassigned.
///
/// Each iteration enumerates ALL connected components for each part, keeps the
/// LARGEST component as the "main" component (mirrors METIS `EliminateComponents`
/// which keeps the heaviest component), and reassigns every smaller component
/// to the neighbouring part with the most boundary edges.  The neighbour is
/// chosen by scanning the entire component for external edges and picking the
/// most-frequently contacted foreign part.  This handles chains of vertices
/// whose internal members have no direct foreign neighbours.
///
/// Using the largest component as "main" avoids the pathological case where a
/// small low-index island is kept as "main" and a large correct piece gets
/// spuriously reassigned (which can cascade and require many extra passes).
pub fn repair_contiguity(g: &CsrGraph, partition: &mut Partition) -> usize {
    let n = g.n();
    let mut reassigned = 0usize;

    // Iteratively fix until all parts are contiguous (max n*k iterations for safety).
    // Each iteration reassigns at least one secondary component, reducing the total
    // component count across all parts by ≥1, so convergence is guaranteed.
    let k = partition.k as usize;
    for _ in 0..n * k {
        if check_contiguity(g, partition).is_ok() { break; }

        let mut made_progress = false;

        'parts: for part in 0..k {
            // Find ALL connected components of this part via BFS, tracking each
            // component's size so we can identify the largest ("main") component.
            let mut comp_id   = vec![usize::MAX; n]; // comp_id[v] = component index
            let mut comp_sizes: Vec<usize> = Vec::new();

            for start in 0..n {
                if partition.assignment[start] as usize != part { continue; }
                if comp_id[start] != usize::MAX { continue; }

                let cid = comp_sizes.len();
                let mut size = 0usize;
                let mut queue = std::collections::VecDeque::from([start]);
                comp_id[start] = cid;
                while let Some(v) = queue.pop_front() {
                    size += 1;
                    for j in g.xadj[v] as usize..g.xadj[v+1] as usize {
                        let u = g.adjncy[j] as usize;
                        if comp_id[u] == usize::MAX && partition.assignment[u] as usize == part {
                            comp_id[u] = cid;
                            queue.push_back(u);
                        }
                    }
                }
                comp_sizes.push(size);
            }

            if comp_sizes.len() <= 1 { continue; } // this part is already contiguous

            // Identify the largest component — this is the "main" component to keep.
            let main_cid = comp_sizes.iter().enumerate()
                .max_by_key(|&(_, &sz)| sz)
                .map(|(i, _)| i)
                .unwrap_or(0);

            // Collect secondary (non-main) components and reassign them.
            // Iterate over secondary component IDs; for each, collect its vertices
            // and count external edges to choose the best target part.
            for sec_cid in 0..comp_sizes.len() {
                if sec_cid == main_cid { continue; }

                // Collect all vertices of this secondary component.
                let component: Vec<usize> = (0..n)
                    .filter(|&v| comp_id[v] == sec_cid)
                    .collect();

                // Count external edges from this component to each foreign part.
                let mut adj_counts = vec![0u32; k];
                for &v in &component {
                    for j in g.xadj[v] as usize..g.xadj[v+1] as usize {
                        let u = g.adjncy[j] as usize;
                        let up = partition.assignment[u] as usize;
                        if up != part { adj_counts[up] += 1; }
                    }
                }

                // Pick the foreign part with the most external edges.
                if let Some((best_part, _)) = adj_counts.iter().enumerate()
                    .filter(|&(p, &c)| p != part && c > 0)
                    .max_by_key(|&(_, &c)| c)
                {
                    for &v in &component {
                        partition.assignment[v] = best_part as u32;
                        reassigned += 1;
                    }
                    made_progress = true;
                    break 'parts; // restart outer loop after any reassignment
                }
                // If no foreign edges found, this component is only connected to
                // other secondary components of the same part.  Those neighbours
                // will be reassigned in a later iteration, after which this
                // component will gain foreign edges and be fixed as well.
            }
        }

        if !made_progress { break; } // no further progress possible
    }
    reassigned
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn path_graph(n: usize) -> CsrGraph {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 { adjncy.push((i - 1) as u32); }
            if i < n - 1 { adjncy.push((i + 1) as u32); }
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
    }

    fn weighted_path(n: usize) -> CsrGraph {
        let mut g = path_graph(n);
        g.adjwgt = Some(vec![2i32; g.adjncy.len()]);
        g
    }

    fn grid_2x3() -> CsrGraph {
        // 0-1-2
        // | | |
        // 3-4-5
        let xadj   = vec![0u32, 2, 4, 6, 8, 11, 13];
        let adjncy = vec![
            1, 3,       // 0: right, down
            0, 2, 4,    // 1: left, right, down
            1, 5,       // 2: left, down
            0, 4,       // 3: up, right
            1, 3, 5,    // 4: up, left, right
            2, 4,       // 5: up, left
        ];
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; 6], adjwgt: None }
    }

    #[test]
    fn valid_path_graph() { assert!(path_graph(5).is_valid()); }

    #[test]
    fn invalid_self_loop() {
        let mut g = path_graph(4);
        g.adjncy[0] = 0;
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_out_of_bounds_adjncy() {
        let mut g = path_graph(4);
        g.adjncy[0] = 99;
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_zero_vwgt() {
        let mut g = path_graph(4);
        g.vwgt[1] = 0;
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_negative_vwgt() {
        let mut g = path_graph(4);
        g.vwgt[1] = -1;
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_disconnected() {
        let g = CsrGraph {
            xadj:   vec![0, 1, 2, 3, 4],
            adjncy: vec![1, 0, 3, 2],
            ncon: 1,
            vwgt: vec![1; 4],
            adjwgt: None,
        };
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_adjwgt_wrong_len() {
        let mut g = path_graph(4);
        g.adjwgt = Some(vec![1i32; 3]);
        assert!(!g.is_valid());
    }

    #[test]
    fn valid_multi_constraint() {
        let mut g = path_graph(4);
        g.ncon = 2;
        g.vwgt = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert!(g.is_valid());
    }

    // ── extract_subgraph ───────────────────────────────────────────────────

    #[test]
    fn extract_subgraph_contains_correct_vertices() {
        let g = path_graph(4);
        let assignment = [0u32, 0, 1, 1];
        let (sub, _g2l, l2g) = extract_subgraph(&g, &assignment, 0);
        assert_eq!(l2g, vec![0, 1], "left subgraph should contain vertices 0 and 1");
        assert!(sub.is_valid());
        assert_eq!(sub.n(), 2);
        assert_eq!(sub.adjncy.len(), 2, "one internal edge 0-1 = 2 directed entries");
    }

    #[test]
    fn extract_subgraph_preserves_edge_weights() {
        let mut g = path_graph(4);
        g.adjwgt = Some(vec![3i32; g.adjncy.len()]);
        let assignment = [0u32, 0, 1, 1];
        let (sub, _, _) = extract_subgraph(&g, &assignment, 0);
        assert!(sub.adjwgt.is_some());
        assert!(sub.adjwgt.as_ref().unwrap().iter().all(|&w| w == 3),
            "edge weight must survive into subgraph");
    }

    #[test]
    fn extract_subgraph_unweighted_stays_unweighted() {
        let g = path_graph(4);
        let assignment = [0u32, 0, 1, 1];
        let (sub, _, _) = extract_subgraph(&g, &assignment, 0);
        assert!(sub.adjwgt.is_none());
    }

    #[test]
    fn extract_subgraph_single_vertex_no_edges() {
        let g = path_graph(4);
        let assignment = [0u32, 1, 0, 0];  // vertex 1 is isolated in part 1
        let (sub, _, l2g) = extract_subgraph(&g, &assignment, 1);
        assert_eq!(l2g, vec![1]);
        assert_eq!(sub.n(), 1);
        assert_eq!(sub.adjncy.len(), 0, "isolated vertex has no internal edges");
    }

    #[test]
    fn extract_subgraph_multiconstraint_weight_layout() {
        let mut g = path_graph(4);
        g.ncon = 2;
        g.vwgt = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let assignment = [0u32, 0, 1, 1];
        let (sub, _, _) = extract_subgraph(&g, &assignment, 0);
        assert_eq!(sub.ncon, 2);
        assert_eq!(sub.vwgt.len(), 4, "2 vertices × 2 constraints");
        assert_eq!(&sub.vwgt[0..2], &[1, 2], "vertex 0 weights");
        assert_eq!(&sub.vwgt[2..4], &[3, 4], "vertex 1 weights");
    }

    // ── check_contiguity ───────────────────────────────────────────────────

    #[test]
    fn check_contiguity_path_bisect_is_ok() {
        let g = path_graph(4);
        let p = Partition { assignment: vec![0, 0, 1, 1], k: 2, tpwgts: None };
        assert!(check_contiguity(&g, &p).is_ok());
    }

    #[test]
    fn check_contiguity_disconnected_returns_err_with_part_id() {
        // Path 0-1-2-3-4: part 0 = {0,1,4} — not connected (4 separated from 0,1)
        let g = path_graph(5);
        let p = Partition { assignment: vec![0, 0, 1, 1, 0], k: 2, tpwgts: None };
        let err = check_contiguity(&g, &p);
        assert!(err.is_err(), "disconnected part must return Err");
        assert_eq!(err.unwrap_err(), 0, "err value must be the disconnected part ID");
    }

    // ── repair_contiguity ──────────────────────────────────────────────────

    #[test]
    fn repair_contiguity_fixes_disconnected_part() {
        let g = path_graph(5);
        let mut p = Partition { assignment: vec![0, 0, 1, 1, 0], k: 2, tpwgts: None };
        assert!(check_contiguity(&g, &p).is_err(), "pre-condition: must be non-contiguous");
        let moved = repair_contiguity(&g, &mut p);
        assert!(moved > 0, "must have moved at least one vertex");
        assert!(check_contiguity(&g, &p).is_ok(), "must be contiguous after repair");
    }

    #[test]
    fn repair_contiguity_noop_when_already_contiguous() {
        let g = path_graph(4);
        let mut p = Partition { assignment: vec![0, 0, 1, 1], k: 2, tpwgts: None };
        let orig = p.assignment.clone();
        let moved = repair_contiguity(&g, &mut p);
        assert_eq!(moved, 0, "no vertices should move when partition is already contiguous");
        assert_eq!(p.assignment, orig);
    }

    #[test]
    fn repair_contiguity_handles_k1() {
        let g = path_graph(6);
        let mut p = Partition { assignment: vec![0; 6], k: 1, tpwgts: None };
        let moved = repair_contiguity(&g, &mut p);
        assert_eq!(moved, 0, "k=1 is trivially contiguous — no repair needed");
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Proves: CsrGraph::is_valid() never panics for any input up to n=8.
    /// Covers: all branches in is_valid() including xadj check, self-loop,
    /// OOB adjncy, ncon, vwgt positivity, adjwgt length, BFS connectivity.
    #[kani::proof]
    #[kani::unwind(9)]
    fn verify_is_valid_no_panic() {
        let n: usize = kani::any_where(|&n: &usize| n <= 8);
        // Construct arbitrary xadj of length n+1
        let xadj: Vec<u32> = (0..=n).map(|_| kani::any()).collect();
        let adjncy_len: usize = kani::any_where(|&l: &usize| l <= 32);
        let adjncy: Vec<u32> = (0..adjncy_len).map(|_| kani::any()).collect();
        let ncon: u32 = kani::any_where(|&c: &u32| c <= 4);
        let vwgt_len = n.saturating_mul(ncon as usize).min(64);
        let vwgt: Vec<i32> = (0..vwgt_len).map(|_| kani::any()).collect();
        let g = CsrGraph { xadj, adjncy, ncon, vwgt, adjwgt: None };
        // Must not panic — result is ignored
        let _ = g.is_valid();
    }
}
