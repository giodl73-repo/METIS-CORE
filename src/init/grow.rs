use rand_pcg::Pcg64;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;
use crate::graph::{CsrGraph, Partition};
use crate::init::InitialPartitioner;

pub struct GrowBisect;
pub struct GrowKway;

// ── test helpers ──────────────────────────────────────────────────────────

#[cfg(test)]
fn grid_4x4() -> CsrGraph {
    // 4×4 grid: vertex i*4+j connected to neighbours
    let n = 16usize;
    let mut xadj = vec![0u32];
    let mut adjncy = Vec::new();
    for i in 0..4usize {
        for j in 0..4usize {
            let mut nbrs = Vec::new();
            if i > 0 { nbrs.push((i - 1) * 4 + j); }
            if i < 3 { nbrs.push((i + 1) * 4 + j); }
            if j > 0 { nbrs.push(i * 4 + (j - 1)); }
            if j < 3 { nbrs.push(i * 4 + (j + 1)); }
            for &u in &nbrs { adjncy.push(u as u32); }
            xadj.push(adjncy.len() as u32);
        }
    }
    CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
}

#[cfg(test)]
fn path_graph(n: usize) -> CsrGraph {
    let mut xadj = vec![0u32];
    let mut adjncy = Vec::new();
    for i in 0..n {
        if i > 0 { adjncy.push((i - 1) as u32); }
        if i < n - 1 { adjncy.push((i + 1) as u32); }
        xadj.push(adjncy.len() as u32);
    }
    CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grow_bisect_valid_partition() {
        let g = grid_4x4();
        let p = GrowBisect.partition(&g, 2, 42);
        assert_eq!(p.assignment.len(), 16);
        assert_eq!(p.k, 2);
        assert!(p.assignment.iter().all(|&x| x < 2));
        assert!(p.assignment.contains(&0));
        assert!(p.assignment.contains(&1));
    }

    #[test]
    fn grow_bisect_k1_all_zero() {
        let g = path_graph(10);
        let p = GrowBisect.partition(&g, 1, 0);
        assert!(p.assignment.iter().all(|&x| x == 0));
        assert_eq!(p.k, 1);
    }

    #[test]
    fn grow_kway_valid_k4() {
        let g = grid_4x4();
        let p = GrowKway.partition(&g, 4, 99);
        assert_eq!(p.assignment.len(), 16);
        assert_eq!(p.k, 4);
        assert!(p.assignment.iter().all(|&x| x < 4));
        // All 4 parts must appear
        for part in 0..4u32 { assert!(p.assignment.contains(&part)); }
    }

    #[test]
    fn grow_kway_k1_all_zero() {
        let g = path_graph(8);
        let p = GrowKway.partition(&g, 1, 0);
        assert!(p.assignment.iter().all(|&x| x == 0));
    }

    #[test]
    fn recursive_bisect_grid_k4() {
        use crate::api::{MetisParams, MetisPartitioner, Partitioner};
        let g = grid_4x4();
        let params = MetisParams {
            use_recursive: true,
            ..MetisParams::default()
        };
        let p = MetisPartitioner::with_params(params, 4)
            .split(&g, 4, Some(0))
            .unwrap();
        assert_eq!(p.assignment.len(), 16);
        assert_eq!(p.k, 4);
        assert!(p.assignment.iter().all(|&x| x < 4));
        // All 4 parts must appear
        for part in 0..4u32 {
            assert!(
                p.assignment.contains(&part),
                "part {part} missing from assignment"
            );
        }
    }

    #[test]
    fn recursive_bisect_k2_same_validity_as_direct() {
        use crate::api::{MetisParams, MetisPartitioner, Partitioner};
        let g = path_graph(10);
        let params_rb = MetisParams {
            use_recursive: true,
            ..MetisParams::default()
        };
        let params_kw = MetisParams {
            use_recursive: false,
            ..MetisParams::default()
        };
        // k=2: use_recursive takes the direct bisection path (same as direct k-way)
        let p_rb = MetisPartitioner::with_params(params_rb, 2)
            .split(&g, 2, Some(42))
            .unwrap();
        let p_kw = MetisPartitioner::with_params(params_kw, 2)
            .split(&g, 2, Some(42))
            .unwrap();
        // Both must produce a valid bisection of the same graph
        assert_eq!(p_rb.assignment.len(), p_kw.assignment.len());
        assert_eq!(p_rb.k, 2);
        assert_eq!(p_kw.k, 2);
        assert!(p_rb.assignment.contains(&0));
        assert!(p_rb.assignment.contains(&1));
    }

    #[test]
    fn recursive_bisect_k1_all_zero() {
        use crate::api::{MetisParams, MetisPartitioner, Partitioner};
        let g = path_graph(10);
        let params = MetisParams {
            use_recursive: true,
            ..MetisParams::default()
        };
        let p = MetisPartitioner::with_params(params, 1)
            .split(&g, 1, Some(0))
            .unwrap();
        assert!(p.assignment.iter().all(|&x| x == 0));
        assert_eq!(p.k, 1);
    }

    #[test]
    fn recursive_bisect_k8_valid() {
        use crate::api::{MetisParams, MetisPartitioner, Partitioner};
        // 8×8 grid, k=8 — tests deeper recursion (depth 3)
        let n = 64usize;
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..8usize {
            for j in 0..8usize {
                let mut nbrs = Vec::new();
                if i > 0 {
                    nbrs.push((i - 1) * 8 + j);
                }
                if i < 7 {
                    nbrs.push((i + 1) * 8 + j);
                }
                if j > 0 {
                    nbrs.push(i * 8 + (j - 1));
                }
                if j < 7 {
                    nbrs.push(i * 8 + (j + 1));
                }
                for &u in &nbrs {
                    adjncy.push(u as u32);
                }
                xadj.push(adjncy.len() as u32);
            }
        }
        let g = CsrGraph {
            xadj,
            adjncy,
            ncon: 1,
            vwgt: vec![1i32; n],
            adjwgt: None,
        };
        let params = MetisParams {
            use_recursive: true,
            ..MetisParams::default()
        };
        let p = MetisPartitioner::with_params(params, 8)
            .split(&g, 8, Some(7))
            .unwrap();
        assert_eq!(p.assignment.len(), 64);
        assert_eq!(p.k, 8);
        assert!(p.assignment.iter().all(|&x| x < 8));
        for part in 0..8u32 {
            assert!(
                p.assignment.contains(&part),
                "part {part} missing from k=8 assignment"
            );
        }
    }
}

/// Multilevel recursive bisection for k > 2.
///
/// Bisects the graph into two halves (k_left = k/2, k_right = k - k/2) then
/// recursively partitions each half. Mirrors METIS `MlevelRecursiveBisection`
/// from `pmetis.c`.
///
/// The recursion bottoms out at k == 1 (trivial) and k == 2 (single bisection).
pub struct RecursiveBisect {
    pub niter:      u32,
    pub ncuts:      u32,
    pub coarsen_to: u32,
    pub ufactor:    u32,
    pub contig_fm:  bool,
}

impl Default for RecursiveBisect {
    fn default() -> Self {
        Self {
            niter:      10,
            ncuts:      1,
            coarsen_to: 20,
            ufactor:    5,
            contig_fm:  true,
        }
    }
}

impl RecursiveBisect {
    /// Recursively partition graph `g` into `k` parts.
    ///
    /// The algorithm:
    /// 1. If k == 1: return all-zeros partition.
    /// 2. If k == 2: run the standard multilevel bisection pipeline.
    /// 3. Else: bisect into two parts, extract subgraphs, recurse on each half,
    ///    then merge — right part IDs are offset by `k_left`.
    pub fn partition_graph(
        &self,
        g: &CsrGraph,
        k: u32,
        seed: u64,
    ) -> Result<crate::graph::Partition, crate::error::PartitionError> {
        use crate::api::{MetisParams, MetisPartitioner, Partitioner};
        use crate::graph::{extract_subgraph, repair_contiguity, Partition};

        if k == 1 {
            return Ok(Partition {
                assignment: vec![0; g.n()],
                k: 1,
                tpwgts: None,
            });
        }

        // Bisect g into two halves (parts 0 and 1).
        let bisect_params = MetisParams {
            ufactor:       self.ufactor,
            niter:         self.niter,
            seed:          Some(seed),
            coarsen_to:    self.coarsen_to,
            ncuts:         self.ncuts,
            tpwgts:        None,
            contig_fm:     self.contig_fm,
            use_recursive: false, // always use direct pipeline for the bisection step
            objective:     crate::api::ObjectiveType::Cut,
            min_conn:      false,
            lp_refine:      false,
            lp_iter:        0,
            coarsen_method: crate::api::CoarseningMethod::Shem,
        };
        let bisection = MetisPartitioner::with_params(bisect_params, 2).split(g, 2, Some(seed))?;

        if k == 2 {
            return Ok(bisection);
        }

        let k_left = k / 2;
        let k_right = k - k_left;

        // Extract induced subgraph for each half.
        let (left_g, _, l2g_left) = extract_subgraph(g, &bisection.assignment, 0);
        let (right_g, _, l2g_right) = extract_subgraph(g, &bisection.assignment, 1);

        // Use distinct seeds for each half to avoid correlated partitions.
        let left_seed = seed.wrapping_add(0x9E3779B9_7F4A7C15);
        let right_seed = seed.wrapping_add(0x6C62272E_07BB0142);

        // Recursively partition each half.
        let left_p = self.partition_graph(&left_g, k_left, left_seed)?;
        let right_p = self.partition_graph(&right_g, k_right, right_seed)?;

        // Merge: right part IDs are offset by k_left.
        let mut assignment = vec![0u32; g.n()];
        for (local_v, &global_v) in l2g_left.iter().enumerate() {
            assignment[global_v] = left_p.assignment[local_v];
        }
        for (local_v, &global_v) in l2g_right.iter().enumerate() {
            assignment[global_v] = right_p.assignment[local_v] + k_left;
        }

        let mut p = Partition {
            assignment,
            k,
            tpwgts: None,
        };
        repair_contiguity(g, &mut p);
        Ok(p)
    }
}

// ── implementation ────────────────────────────────────────────────────────

impl InitialPartitioner for GrowBisect {
    fn partition(&self, g: &CsrGraph, k: u32, seed: u64) -> Partition {
        debug_assert!(g.is_valid(), "requires valid connected graph");
        if k == 1 { return Partition { assignment: vec![0; g.n()], k: 1, tpwgts: None }; }
        grow_bisect(g, k, seed)
    }
}

impl InitialPartitioner for GrowKway {
    fn partition(&self, g: &CsrGraph, k: u32, seed: u64) -> Partition {
        debug_assert!(g.is_valid(), "requires valid connected graph");
        if k == 1 { return Partition { assignment: vec![0; g.n()], k: 1, tpwgts: None }; }
        grow_kway(g, k, seed)
    }
}

fn grow_bisect(g: &CsrGraph, k: u32, seed: u64) -> Partition {
    // For k=2: BFS from 2 random seeds alternating.
    // For k>2: delegate to grow_kway (full k-way BFS expansion).
    if k > 2 { return grow_kway(g, k, seed); }

    let n = g.n();
    let mut rng = Pcg64::seed_from_u64(seed);
    let mut assignment = vec![u32::MAX; n];

    // Pick 2 distinct random seeds
    let seed_a = rng.gen_range(0..n);
    let mut seed_b = rng.gen_range(0..n);
    while seed_b == seed_a && n > 1 {
        seed_b = rng.gen_range(0..n);
    }

    assignment[seed_a] = 0;
    assignment[seed_b] = 1;
    let mut queues = [VecDeque::from([seed_a]), VecDeque::from([seed_b])];

    // Count initially assigned vertices
    let initially_assigned = if seed_a == seed_b { 1 } else { 2 };
    let mut unassigned = n - initially_assigned;

    'outer: while unassigned > 0 {
        let mut progress = false;
        for (part, queue) in queues.iter_mut().enumerate() {
            if let Some(v) = queue.pop_front() {
                for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                    let u = g.adjncy[j] as usize;
                    if assignment[u] == u32::MAX {
                        assignment[u] = part as u32;
                        queue.push_back(u);
                        unassigned -= 1;
                        progress = true;
                        if unassigned == 0 { break 'outer; }
                    }
                }
            }
        }
        // If both queues empty but vertices remain (shouldn't happen on valid connected graph)
        if !progress && queues[0].is_empty() && queues[1].is_empty() { break; }
    }

    // Safe fallback: assign any remaining (disconnected) vertices to part 0
    for a in assignment.iter_mut() {
        if *a == u32::MAX { *a = 0; }
    }
    Partition { assignment, k: 2, tpwgts: None }
}

fn grow_kway(g: &CsrGraph, k: u32, seed: u64) -> Partition {
    let n = g.n();
    let k = k as usize;
    let mut rng = Pcg64::seed_from_u64(seed);
    let mut assignment = vec![u32::MAX; n];
    let mut queues: Vec<VecDeque<usize>> = (0..k).map(|_| VecDeque::new()).collect();

    // Pick k distinct seed vertices
    let mut seeds: Vec<usize> = Vec::with_capacity(k);
    let mut attempts = 0usize;
    while seeds.len() < k && attempts < n * 10 {
        let v = rng.gen_range(0..n);
        if !seeds.contains(&v) {
            seeds.push(v);
        }
        attempts += 1;
    }
    // Fallback: if we couldn't find k distinct seeds (k > n), fill with wrap-around
    while seeds.len() < k {
        for v in 0..n {
            if seeds.len() >= k { break; }
            if !seeds.contains(&v) { seeds.push(v); }
        }
        // Last resort: allow duplicates if n < k
        if seeds.len() < k { seeds.push(seeds.len() % n); }
    }

    // Seed each part; duplicates will be silently skipped (already assigned)
    let mut initially_assigned = 0usize;
    for (part, &sv) in seeds.iter().enumerate().take(k) {
        if assignment[sv] == u32::MAX {
            assignment[sv] = part as u32;
            queues[part].push_back(sv);
            initially_assigned += 1;
        }
    }

    let mut unassigned = n - initially_assigned;

    // Round-robin BFS: one dequeue per part per round
    let mut round_part = 0usize;
    while unassigned > 0 {
        let part = round_part % k;
        round_part += 1;

        if let Some(v) = queues[part].pop_front() {
            for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                let u = g.adjncy[j] as usize;
                if assignment[u] == u32::MAX {
                    assignment[u] = part as u32;
                    queues[part].push_back(u);
                    unassigned -= 1;
                    if unassigned == 0 { break; }
                }
            }
        }

        // Circuit break: all queues empty but unassigned remains (disconnected graph)
        if queues.iter().all(|q| q.is_empty()) { break; }
    }

    // Safe fallback for any truly unreachable vertices
    for a in assignment.iter_mut() {
        if *a == u32::MAX { *a = 0; }
    }
    Partition { assignment, k: k as u32, tpwgts: None }
}
