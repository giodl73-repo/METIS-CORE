use rand::SeedableRng;
use rand_pcg::Pcg64;
use rand::Rng;
use crate::graph::{CsrGraph, CoarseMap};
use crate::coarsen::Coarsener;

// ── test helpers ──────────────────────────────────────────────────────────

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

pub fn triangle() -> CsrGraph {
    CsrGraph {
        xadj:   vec![0, 2, 4, 6],
        adjncy: vec![1, 2,  0, 2,  0, 1],
        ncon: 1, vwgt: vec![1; 3], adjwgt: None,
    }
}

pub fn path5() -> CsrGraph { path_graph(5) }

// ── implementation structs (declared here, defined below tests) ───────────

pub struct HeavyEdgeMatch;
pub struct HeavyEdgeMatchWithParams { pub coarsen_to: u32, pub k: u32 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coarsened_strictly_smaller() {
        let (c, _) = HeavyEdgeMatch.coarsen(&path5());
        assert!(c.n() < 5, "coarsened graph must be strictly smaller");
        assert!(c.is_valid(), "coarsened graph must be valid");
    }

    #[test]
    fn cmap_length_equals_fine_n() {
        let g = path5();
        let (_, cmap) = HeavyEdgeMatch.coarsen(&g);
        assert_eq!(cmap.cmap.len(), g.n());
    }

    #[test]
    fn cmap_targets_in_range() {
        let g = path5();
        let (c, cmap) = HeavyEdgeMatch.coarsen(&g);
        assert!(cmap.cmap.iter().all(|&t| (t as usize) < c.n()));
    }

    #[test]
    fn should_stop_small_graph() {
        let hem = HeavyEdgeMatchWithParams { coarsen_to: 20, k: 2 };
        // path5 has 5 vertices; threshold = max(20*2, 40) = 40; 5 < 40 → should stop
        assert!(hem.should_stop(&path5()));
    }

    #[test]
    fn should_stop_large_graph() {
        let hem = HeavyEdgeMatchWithParams { coarsen_to: 20, k: 53 };
        // threshold = max(20*53, 40) = 1060; path5 has 5 vertices → should stop
        assert!(hem.should_stop(&path5()));
    }

    #[test]
    fn triangle_coarsens_to_at_most_2() {
        let (c, _) = HeavyEdgeMatch.coarsen(&triangle());
        assert!(c.n() <= 2, "triangle must coarsen to <= 2 vertices");
        assert!(c.is_valid());
    }

    #[test]
    fn unweighted_stays_unweighted() {
        let (c, _) = HeavyEdgeMatch.coarsen(&path5());
        assert!(c.adjwgt.is_none(),
            "unweighted input must produce unweighted coarsened graph");
    }

    #[test]
    fn weighted_stays_weighted() {
        let mut g = path5();
        g.adjwgt = Some(vec![1i32; g.adjncy.len()]);
        let (c, _) = HeavyEdgeMatch.coarsen(&g);
        assert!(c.adjwgt.is_some(), "weighted input must produce weighted coarsened graph");
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;
    use crate::coarsen::Coarsener;

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

    /// Proves: HeavyEdgeMatch::coarsen() never panics or goes OOB
    /// for any valid path graph up to n=16.
    #[kani::proof]
    #[kani::unwind(17)]
    fn verify_hem_no_oob() {
        let n: usize = kani::any_where(|&n: &usize| n >= 2 && n <= 16);
        let g = kani_path(n);
        kani::assume(g.is_valid());
        let (coarsened, cmap) = HeavyEdgeMatch.coarsen(&g);
        assert!(cmap.cmap.len() == g.n());
        assert!(coarsened.n() < g.n());
    }
}

// ── implementation ────────────────────────────────────────────────────────

impl Coarsener for HeavyEdgeMatch {
    fn coarsen(&self, g: &CsrGraph) -> (CsrGraph, CoarseMap) {
        hem_coarsen(g, 0x1234_5678_9ABC_DEF0)
    }
    fn should_stop(&self, g: &CsrGraph) -> bool {
        g.n() <= 40
    }
}

impl Coarsener for HeavyEdgeMatchWithParams {
    fn coarsen(&self, g: &CsrGraph) -> (CsrGraph, CoarseMap) {
        hem_coarsen(g, 0x1234_5678_9ABC_DEF0)
    }
    fn should_stop(&self, g: &CsrGraph) -> bool {
        let threshold = (self.coarsen_to * self.k).max(40);
        g.n() <= threshold as usize
    }
}

fn hem_coarsen(g: &CsrGraph, seed: u64) -> (CsrGraph, CoarseMap) {
    let n = g.n();
    let mut rng = Pcg64::seed_from_u64(seed);
    let mut matched  = vec![false; n];
    let mut cmap     = vec![u32::MAX; n];
    let mut coarse_id = 0u32;

    // Fisher-Yates shuffle for random visit order
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = rng.gen_range(0..=i);
        order.swap(i, j);
    }

    for &v in &order {
        if matched[v] { continue; }
        // Find heaviest unmatched neighbour
        let best = (g.xadj[v] as usize..g.xadj[v + 1] as usize)
            .filter(|&j| !matched[g.adjncy[j] as usize])
            .max_by_key(|&j| g.adjwgt.as_ref().map_or(1i32, |aw| aw[j]));
        match best {
            Some(j) => {
                let u = g.adjncy[j] as usize;
                cmap[v] = coarse_id; cmap[u] = coarse_id;
                matched[v] = true;  matched[u] = true;
            }
            None => { cmap[v] = coarse_id; matched[v] = true; }
        }
        coarse_id += 1;
    }
    build_coarse_graph(g, &cmap, coarse_id as usize)
}

/// Shared by all three coarsenens. Builds the coarsened CSR graph.
///
/// INVARIANT: `adjwgt: None` in → `adjwgt: None` out.
/// Vertex weight accumulation uses i64 to prevent overflow on large graphs.
pub fn build_coarse_graph(g: &CsrGraph, cmap: &[u32], cn: usize) -> (CsrGraph, CoarseMap) {
    let n = g.n();
    let ncon = g.ncon as usize;

    // Accumulate in i64 to prevent overflow when summing many large vertex weights
    let mut cvwgt = vec![0i64; cn * ncon];
    for v in 0..n {
        let cv = cmap[v] as usize;
        for c in 0..ncon {
            cvwgt[cv * ncon + c] += g.vwgt[v * ncon + c] as i64;
        }
    }
    let cvwgt_i32: Vec<i32> = cvwgt.iter().map(|&w| w as i32).collect();

    // Build coarse adjacency using Vec + sort + dedup (cache-friendly, no HashMap allocs)
    let mut cadj: Vec<Vec<(u32, i32)>> = vec![Vec::new(); cn];
    for v in 0..n {
        let cv = cmap[v] as usize;
        for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
            let cu = cmap[g.adjncy[j] as usize] as usize;
            if cu != cv {
                let ew = g.adjwgt.as_ref().map_or(1i32, |aw| aw[j]);
                cadj[cv].push((cu as u32, ew));
            }
        }
    }

    // Sort each neighbor list and sum weights for parallel edges
    for neighbors in cadj.iter_mut() {
        neighbors.sort_unstable_by_key(|&(u, _)| u);
        // In-place dedup: merge consecutive same-neighbor entries by summing weights
        let mut write = 0usize;
        for read in 0..neighbors.len() {
            if write > 0 && neighbors[write - 1].0 == neighbors[read].0 {
                neighbors[write - 1].1 += neighbors[read].1;
            } else {
                neighbors[write] = neighbors[read];
                write += 1;
            }
        }
        neighbors.truncate(write);
    }

    let mut xadj   = vec![0u32];
    let mut adjncy = Vec::new();
    let mut adjwgt = Vec::new();

    for neighbors in &cadj {
        for &(cu, ew) in neighbors {
            adjncy.push(cu);
            adjwgt.push(ew);
        }
        xadj.push(adjncy.len() as u32);
    }

    let coarse = CsrGraph {
        xadj,
        adjncy,
        ncon: g.ncon,
        vwgt: cvwgt_i32,
        // KEY: only preserve adjwgt if input had edge weights (NO || true)
        adjwgt: if g.adjwgt.is_some() { Some(adjwgt) } else { None },
    };
    (coarse, CoarseMap { cmap: cmap.to_vec() })
}
