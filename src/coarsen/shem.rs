use crate::graph::{CsrGraph, CoarseMap};
use crate::coarsen::Coarsener;
use crate::coarsen::hem::build_coarse_graph;

// ── structs ────────────────────────────────────────────────────────────────

pub struct SortedHeavyEdgeMatch;
pub struct SortedHeavyEdgeMatchWithParams { pub coarsen_to: u32, pub k: u32 }

// ── test helpers ──────────────────────────────────────────────────────────

#[cfg(test)]
fn weighted_path4() -> CsrGraph {
    // 0 --10-- 1 --1-- 2 --10-- 3  (undirected)
    CsrGraph {
        xadj:   vec![0, 1, 3, 5, 6],
        adjncy: vec![1,   0,2,   1,3,   2],
        ncon: 1,
        vwgt: vec![1; 4],
        adjwgt: Some(vec![10, 10, 1, 1, 10, 10]),
    }
}

#[cfg(test)]
fn path5() -> CsrGraph {
    let mut xadj = vec![0u32];
    let mut adjncy = Vec::new();
    for i in 0..5usize {
        if i > 0 { adjncy.push((i - 1) as u32); }
        if i < 4 { adjncy.push((i + 1) as u32); }
        xadj.push(adjncy.len() as u32);
    }
    CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; 5], adjwgt: None }
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shem_coarsened_valid() {
        let (c, cmap) = SortedHeavyEdgeMatch.coarsen(&weighted_path4());
        assert!(c.is_valid(), "coarsened graph must be valid");
        assert_eq!(cmap.cmap.len(), 4);
        assert!(c.n() < 4);
    }

    #[test]
    fn shem_prefers_heavy_edges() {
        // 0 --10-- 1 --1-- 2 --10-- 3
        // SHEM processes heaviest edges first: 0&1 matched, 2&3 matched
        let (_, cmap) = SortedHeavyEdgeMatch.coarsen(&weighted_path4());
        assert_eq!(cmap.cmap[0], cmap.cmap[1],
            "vertices 0 and 1 (connected by weight-10 edge) should be matched");
        assert_eq!(cmap.cmap[2], cmap.cmap[3],
            "vertices 2 and 3 (connected by weight-10 edge) should be matched");
    }

    #[test]
    fn shem_unweighted_valid() {
        let (c, cmap) = SortedHeavyEdgeMatch.coarsen(&path5());
        assert!(c.is_valid());
        assert_eq!(cmap.cmap.len(), 5);
        assert!(c.n() < 5);
    }

    #[test]
    fn shem_unweighted_stays_unweighted() {
        let (c, _) = SortedHeavyEdgeMatch.coarsen(&path5());
        assert!(c.adjwgt.is_none(),
            "unweighted input must produce unweighted output");
    }

    #[test]
    fn shem_weighted_stays_weighted() {
        let (c, _) = SortedHeavyEdgeMatch.coarsen(&weighted_path4());
        assert!(c.adjwgt.is_some(),
            "weighted input must produce weighted output");
    }

    #[test]
    fn shem_with_params_should_stop() {
        let shem = SortedHeavyEdgeMatchWithParams { coarsen_to: 20, k: 2 };
        assert!(shem.should_stop(&path5()),
            "path5 (5 vertices) < threshold max(40, 20*2)=40 → should stop");
    }

    #[test]
    fn shem_strictly_smaller() {
        let (c, _) = SortedHeavyEdgeMatch.coarsen(&path5());
        assert!(c.n() < path5().n());
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;
    use crate::coarsen::Coarsener;

    /// Helper: build a valid path graph of length n for Kani
    fn kani_path(n: usize) -> CsrGraph {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 { adjncy.push((i - 1) as u32); }
            if i < n - 1 { adjncy.push((i + 1) as u32); }
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
    }

    /// Proves: SortedHeavyEdgeMatch::coarsen() never panics or goes OOB
    /// for any valid path graph up to n=16.
    #[kani::proof]
    #[kani::unwind(17)]
    fn verify_shem_no_oob() {
        let n: usize = kani::any_where(|&n: &usize| n >= 2 && n <= 16);
        let g = kani_path(n);
        kani::assume(g.is_valid());
        let (coarsened, cmap) = SortedHeavyEdgeMatch.coarsen(&g);
        // These must hold without panic
        assert!(cmap.cmap.len() == g.n());
        assert!(coarsened.n() < g.n());
    }
}

// ── implementation ────────────────────────────────────────────────────────

impl Coarsener for SortedHeavyEdgeMatch {
    fn coarsen(&self, g: &CsrGraph) -> (CsrGraph, CoarseMap) {
        shem_coarsen(g)
    }
    fn should_stop(&self, g: &CsrGraph) -> bool {
        g.n() <= 40
    }
}

impl Coarsener for SortedHeavyEdgeMatchWithParams {
    fn coarsen(&self, g: &CsrGraph) -> (CsrGraph, CoarseMap) {
        shem_coarsen(g)
    }
    fn should_stop(&self, g: &CsrGraph) -> bool {
        let threshold = (self.coarsen_to * self.k).max(40);
        g.n() <= threshold as usize
    }
}

/// SHEM coarsening — O(n + m) bucket sort, NOT a comparison sort.
///
/// Algorithm:
///   1. Compute max incident edge weight per vertex.
///   2. Bucket-sort vertices by that weight (descending) using counting/bucket
///      sort — O(n + max_weight), no comparisons.
///   3. Iterate buckets high→low; for each unmatched vertex v, match with its
///      heaviest unmatched neighbour (HEM sub-step).
///
/// This guarantees high-weight edges are considered first, yielding better
/// coarsening quality than plain HEM without sacrificing linear complexity.
fn shem_coarsen(g: &CsrGraph) -> (CsrGraph, CoarseMap) {
    let n = g.n();

    // Step 1: max incident edge weight per vertex (unweighted → treat as 1)
    let max_w: Vec<i32> = (0..n).map(|v| {
        (g.xadj[v] as usize..g.xadj[v + 1] as usize)
            .map(|j| g.adjwgt.as_ref().map_or(1i32, |aw| aw[j]))
            .max()
            .unwrap_or(0)
    }).collect();

    // Step 2: bucket sort — O(n + max_weight), NOT comparison sort
    // Buckets indexed by weight value; we iterate them in reverse (highest first).
    let max_bucket = max_w.iter().copied().max().unwrap_or(0).max(1) as usize;
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); max_bucket + 1];
    for v in 0..n {
        let w = (max_w[v].max(0) as usize).min(max_bucket);
        buckets[w].push(v);
    }

    // Step 3: match vertices, processing highest-weight buckets first
    let mut matched   = vec![false; n];
    let mut cmap      = vec![u32::MAX; n];
    let mut coarse_id = 0u32;

    for bucket in buckets.iter().rev() {
        for &v in bucket {
            if matched[v] { continue; }
            // Among unmatched neighbours, pick the one connected by the heaviest edge
            let best = (g.xadj[v] as usize..g.xadj[v + 1] as usize)
                .filter(|&j| !matched[g.adjncy[j] as usize])
                .max_by_key(|&j| g.adjwgt.as_ref().map_or(1i32, |aw| aw[j]));
            match best {
                Some(j) => {
                    let u = g.adjncy[j] as usize;
                    cmap[v] = coarse_id; cmap[u] = coarse_id;
                    matched[v] = true;  matched[u] = true;
                }
                None => {
                    // isolated or all neighbours already matched — singleton supernode
                    cmap[v] = coarse_id; matched[v] = true;
                }
            }
            coarse_id += 1;
        }
    }

    build_coarse_graph(g, &cmap, coarse_id as usize)
}
