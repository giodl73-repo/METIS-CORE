use super::fm::FiducciaMattheyses;
use crate::graph::{CsrGraph, Partition};
use crate::refine::Refiner;

pub struct GreedyKWay {
    niter: u32,
}

impl GreedyKWay {
    pub fn new(niter: u32) -> Self {
        Self { niter }
    }

    pub fn niter(&self) -> u32 {
        self.niter
    }
}

impl Refiner for GreedyKWay {
    fn refine(&self, g: &CsrGraph, p: Partition) -> Partition {
        FiducciaMattheyses {
            niter: self.niter,
            contig_fm: true,
            objective: crate::api::ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        }
        .refine(g, p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refine::Refiner;

    fn path_graph(n: usize) -> CsrGraph {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 {
                adjncy.push((i - 1) as u32);
            }
            if i < n - 1 {
                adjncy.push((i + 1) as u32);
            }
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph {
            xadj,
            adjncy,
            ncon: 1,
            vwgt: vec![1i32; n],
            adjwgt: None,
        }
    }

    fn edge_cut(g: &CsrGraph, assignment: &[u32]) -> u32 {
        let mut cut = 0u32;
        for v in 0..g.n() {
            for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                if assignment[g.adjncy[j] as usize] != assignment[v] {
                    cut += 1;
                }
            }
        }
        cut / 2
    }

    #[test]
    fn greedy_kway_does_not_increase_cut() {
        let g = path_graph(10);
        let p_bad = Partition {
            assignment: (0..10u32).map(|i| i % 2).collect(), // alternating — worst bisection
            k: 2,
            tpwgts: None,
        };
        let cut_before = edge_cut(&g, &p_bad.assignment);
        let p_ref = GreedyKWay::new(10).refine(&g, p_bad);
        let cut_after = edge_cut(&g, &p_ref.assignment);
        assert!(
            cut_after <= cut_before,
            "GreedyKWay must not increase cut: before={cut_before} after={cut_after}"
        );
    }

    #[test]
    fn greedy_kway_output_valid_k4() {
        let g = path_graph(12);
        let p_init = Partition {
            assignment: (0..12).map(|i| (i / 3) as u32).collect(), // 4 groups of 3
            k: 4,
            tpwgts: None,
        };
        let p = GreedyKWay::new(10).refine(&g, p_init);
        assert_eq!(p.assignment.len(), 12);
        assert!(
            p.assignment.iter().all(|&a| a < 4),
            "all assignments must be in [0, k)"
        );
    }
}
