use crate::graph::{CsrGraph, Partition};
use crate::init::{InitialPartitioner, grow::GrowKway};

pub struct MultiConstraintInit;

impl InitialPartitioner for MultiConstraintInit {
    fn partition(&self, g: &CsrGraph, k: u32, seed: u64) -> Partition {
        // Delegate to GrowKway; multi-constraint balance is refined during FM
        GrowKway.partition(g, k, seed)
    }
}

// ── tests ─────────────────────────────────────────────────────────────────

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
fn grid_4x4() -> CsrGraph {
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
mod tests {
    use super::*;

    #[test]
    fn multi_constraint_respects_ncon() {
        let mut g = grid_4x4();
        g.ncon = 2;
        g.vwgt = vec![1; 32]; // 16 vertices x 2 constraints
        let p = MultiConstraintInit.partition(&g, 4, 0);
        assert_eq!(p.k, 4);
        assert_eq!(p.assignment.len(), 16);
        assert!(p.assignment.iter().all(|&x| x < 4));
    }

    #[test]
    fn multi_constraint_k1_trivial() {
        let g = path_graph(8);
        let p = MultiConstraintInit.partition(&g, 1, 0);
        assert!(p.assignment.iter().all(|&x| x == 0));
    }
}
