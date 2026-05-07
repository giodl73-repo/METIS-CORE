use crate::graph::{CsrGraph, CoarseMap};
use crate::coarsen::Coarsener;
use crate::coarsen::hem::build_coarse_graph;

// ── struct ─────────────────────────────────────────────────────────────────

pub struct MinDegreeMatch;

// ── test helpers ──────────────────────────────────────────────────────────

#[cfg(test)]
fn star_graph(n: usize) -> CsrGraph {
    // vertex 0 = center connected to vertices 1..n
    assert!(n >= 2);
    let mut xadj = vec![0u32];
    let mut adjncy = Vec::new();
    // Center vertex 0: connected to all leaves
    for i in 1..n { adjncy.push(i as u32); }
    xadj.push(adjncy.len() as u32);
    // Each leaf: connected only to center
    for _ in 1..n {
        adjncy.push(0u32);
        xadj.push(adjncy.len() as u32);
    }
    CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
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
    fn mindegree_valid_output() {
        let g = star_graph(6);
        let (c, cmap) = MinDegreeMatch.coarsen(&g);
        assert!(c.is_valid(), "coarsened graph must be valid");
        assert_eq!(cmap.cmap.len(), 6);
        assert!(c.n() < 6);
    }

    #[test]
    fn mindegree_unweighted_stays_unweighted() {
        let (c, _) = MinDegreeMatch.coarsen(&path5());
        assert!(c.adjwgt.is_none());
    }

    #[test]
    fn mindegree_cmap_targets_in_range() {
        let g = star_graph(6);
        let (c, cmap) = MinDegreeMatch.coarsen(&g);
        assert!(cmap.cmap.iter().all(|&t| (t as usize) < c.n()));
    }

    #[test]
    fn mindegree_strictly_smaller() {
        let (c, _) = MinDegreeMatch.coarsen(&path5());
        assert!(c.n() < 5);
    }
}

// ── implementation ────────────────────────────────────────────────────────

impl Coarsener for MinDegreeMatch {
    fn coarsen(&self, g: &CsrGraph) -> (CsrGraph, CoarseMap) {
        mindegree_coarsen(g)
    }
    fn should_stop(&self, g: &CsrGraph) -> bool {
        g.n() <= 40
    }
}

fn mindegree_coarsen(g: &CsrGraph) -> (CsrGraph, CoarseMap) {
    let n = g.n();

    // Compute degrees
    let degrees: Vec<u32> = (0..n)
        .map(|v| g.xadj[v + 1] - g.xadj[v])
        .collect();

    // Sort vertices by degree ascending — O(n log n)
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_unstable_by_key(|&v| degrees[v]);

    let mut matched   = vec![false; n];
    let mut cmap      = vec![u32::MAX; n];
    let mut coarse_id = 0u32;

    for v in order {
        if matched[v] { continue; }
        // Match with heaviest unmatched neighbour
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
