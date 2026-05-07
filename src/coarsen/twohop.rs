use rand_pcg::Pcg64;
use rand::{Rng, SeedableRng};
use crate::graph::{CsrGraph, CoarseMap};
use crate::coarsen::Coarsener;
use crate::coarsen::hem::build_coarse_graph;

/// Two-hop heavy-edge matching coarsener.
///
/// First pass: match each vertex with its heaviest unmatched direct neighbour
/// (identical to SHEM).  Second pass: unmatched vertices that found no direct
/// match look two hops away — neighbours-of-neighbours — for an unmatched
/// candidate.  This prevents stranded isolated vertices on sparse or irregular
/// graphs and improves the coarsening ratio.
///
/// Mirrors the `MATCH_2HOPALL` / `MATCH_2HOP` variants in METIS `coarsen.c`.
pub struct TwoHopMatch;
pub struct TwoHopMatchWithParams { pub coarsen_to: u32, pub k: u32 }

impl Coarsener for TwoHopMatch {
    fn coarsen(&self, g: &CsrGraph) -> (CsrGraph, CoarseMap) {
        twohop_coarsen(g, 0x1234_5678_9ABC_DEF0)
    }
    fn should_stop(&self, g: &CsrGraph) -> bool { g.n() <= 40 }
}

impl Coarsener for TwoHopMatchWithParams {
    fn coarsen(&self, g: &CsrGraph) -> (CsrGraph, CoarseMap) {
        twohop_coarsen(g, 0x1234_5678_9ABC_DEF0)
    }
    fn should_stop(&self, g: &CsrGraph) -> bool {
        g.n() <= (self.coarsen_to * self.k).max(40) as usize
    }
}

fn twohop_coarsen(g: &CsrGraph, seed: u64) -> (CsrGraph, CoarseMap) {
    let n = g.n();
    let mut rng = Pcg64::seed_from_u64(seed);

    // Bucket-sort vertices by max incident edge weight descending (SHEM order)
    let max_w: Vec<i32> = (0..n).map(|v| {
        (g.xadj[v] as usize..g.xadj[v + 1] as usize)
            .map(|j| g.adjwgt.as_ref().map_or(1i32, |aw| aw[j]))
            .max()
            .unwrap_or(0)
    }).collect();

    let max_bucket = max_w.iter().copied().max().unwrap_or(0).max(1) as usize;
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); max_bucket + 1];
    for v in 0..n { buckets[max_w[v].max(0) as usize].push(v); }

    let mut matched   = vec![false; n];
    let mut cmap      = vec![u32::MAX; n];
    let mut coarse_id = 0u32;

    // Pass 1: direct heavy-edge matching (same as SHEM)
    for bucket in buckets.iter().rev() {
        for &v in bucket {
            if matched[v] { continue; }
            let best = (g.xadj[v] as usize..g.xadj[v + 1] as usize)
                .filter(|&j| !matched[g.adjncy[j] as usize])
                .max_by_key(|&j| g.adjwgt.as_ref().map_or(1i32, |aw| aw[j]));
            if let Some(j) = best {
                let u = g.adjncy[j] as usize;
                cmap[v] = coarse_id; cmap[u] = coarse_id;
                matched[v] = true;  matched[u] = true;
                coarse_id += 1;
            }
        }
    }

    // Pass 2: 2-hop matching for vertices still unmatched after pass 1.
    // For each unmatched vertex v, look at neighbours-of-neighbours for an
    // unmatched candidate.  Pick the 2-hop neighbour with the highest edge
    // weight on the path v→w→u (approximated as min(ew_vw, ew_wu)).
    for v in 0..n {
        if matched[v] { continue; }

        let mut best_u: Option<usize> = None;
        let mut best_wt = i32::MIN;

        for j1 in g.xadj[v] as usize..g.xadj[v + 1] as usize {
            let w = g.adjncy[j1] as usize; // 1-hop neighbour
            let ew_vw = g.adjwgt.as_ref().map_or(1i32, |aw| aw[j1]);
            for j2 in g.xadj[w] as usize..g.xadj[w + 1] as usize {
                let u = g.adjncy[j2] as usize; // 2-hop neighbour
                if matched[u] || u == v { continue; }
                let ew_wu = g.adjwgt.as_ref().map_or(1i32, |aw| aw[j2]);
                let path_wt = ew_vw.min(ew_wu);
                if path_wt > best_wt {
                    best_wt = path_wt;
                    best_u  = Some(u);
                }
            }
        }

        if let Some(u) = best_u {
            cmap[v] = coarse_id; cmap[u] = coarse_id;
            matched[v] = true;  matched[u] = true;
        } else {
            // Still unmatched after 2-hop: becomes its own coarse vertex
            cmap[v] = coarse_id;
            matched[v] = true;
        }
        coarse_id += 1;
    }

    build_coarse_graph(g, &cmap, coarse_id as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_graph(n: usize) -> CsrGraph {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 { adjncy.push((i-1) as u32); }
            if i < n-1 { adjncy.push((i+1) as u32); }
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
    }

    /// A star graph has a high-degree hub; leaves can only be matched via 2-hop.
    fn star_graph(n: usize) -> CsrGraph {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 1..n { adjncy.push(i as u32); }
        xadj.push(adjncy.len() as u32);
        for _ in 1..n {
            adjncy.push(0u32);
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
    }

    #[test]
    fn twohop_valid_output_path() {
        let (c, cmap) = TwoHopMatch.coarsen(&path_graph(10));
        assert!(c.is_valid());
        assert_eq!(cmap.cmap.len(), 10);
        assert!(c.n() < 10);
    }

    #[test]
    fn twohop_valid_output_star() {
        let g = star_graph(8);
        let (c, cmap) = TwoHopMatch.coarsen(&g);
        assert!(c.is_valid());
        assert_eq!(cmap.cmap.len(), 8);
        assert!(c.n() < 8);
    }

    #[test]
    fn twohop_unweighted_stays_unweighted() {
        let (c, _) = TwoHopMatch.coarsen(&path_graph(8));
        assert!(c.adjwgt.is_none());
    }

    #[test]
    fn twohop_strictly_smaller() {
        let (c, _) = TwoHopMatch.coarsen(&path_graph(12));
        assert!(c.n() < 12);
    }

    #[test]
    fn twohop_with_params_should_stop() {
        let c = TwoHopMatchWithParams { coarsen_to: 20, k: 2 };
        assert!(c.should_stop(&path_graph(5)));
    }

    #[test]
    fn twohop_better_ratio_than_shem_on_star() {
        // Star graph: hub + 7 leaves. SHEM can only match hub with one leaf.
        // TwoHop can match leaves with each other via 2-hop through hub.
        // Result should be <= 4 coarse vertices (vs up to 7 with SHEM).
        let g = star_graph(8);
        let (c, _) = TwoHopMatch.coarsen(&g);
        // 2-hop should do better than leaving all unmatched leaves solo
        assert!(c.n() <= 5, "2-hop should coarsen star better: got {} coarse vertices", c.n());
    }
}
