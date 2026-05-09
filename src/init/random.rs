use crate::graph::{CsrGraph, Partition};
use crate::init::InitialPartitioner;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;

pub struct RandomBisect;

impl InitialPartitioner for RandomBisect {
    fn partition(&self, g: &CsrGraph, k: u32, seed: u64) -> Partition {
        debug_assert!(g.is_valid(), "requires valid connected graph");
        if k == 1 {
            return Partition {
                assignment: vec![0; g.n()],
                k: 1,
                tpwgts: None,
            };
        }
        let mut rng = Pcg64::seed_from_u64(seed);
        let assignment = (0..g.n()).map(|_| rng.gen_range(0..k)).collect();
        Partition {
            assignment,
            k,
            tpwgts: None,
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_bisect_all_parts_present() {
        // 20-vertex path, k=2, seed 7 — both parts must appear
        let g = path_graph(20);
        let p = RandomBisect.partition(&g, 2, 7);
        assert!(p.assignment.contains(&0));
        assert!(p.assignment.contains(&1));
        assert_eq!(p.assignment.len(), 20);
        assert_eq!(p.k, 2);
    }

    #[test]
    fn random_bisect_k1_all_zero() {
        let g = path_graph(10);
        let p = RandomBisect.partition(&g, 1, 0);
        assert!(p.assignment.iter().all(|&x| x == 0));
    }

    #[test]
    fn random_bisect_deterministic() {
        let g = path_graph(10);
        let p1 = RandomBisect.partition(&g, 2, 42);
        let p2 = RandomBisect.partition(&g, 2, 42);
        assert_eq!(
            p1.assignment, p2.assignment,
            "same seed must produce same result"
        );
    }
}
