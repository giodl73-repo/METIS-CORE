use crate::graph::{repair_contiguity, Partition};
use crate::init::InitialPartitioner;
use crate::multilevel::hierarchy::CoarseningHierarchy;
use crate::refine::Refiner;

// ── state markers ─────────────────────────────────────────────────────────

pub(crate) struct NeedsPartition;
pub(crate) struct NeedsRefinement {
    partition: Partition,
}
pub(crate) struct Complete {
    partition: Partition,
}

// ── typestate pipeline ────────────────────────────────────────────────────

pub(crate) struct Pipeline<S> {
    hierarchy: CoarseningHierarchy,
    repair_contiguity: bool,
    state: S,
}

impl Pipeline<NeedsPartition> {
    pub(crate) fn new(h: CoarseningHierarchy) -> Self {
        debug_assert!(
            h.cmaps().len() == h.levels().len().saturating_sub(1),
            "CoarseningHierarchy invariant violated: cmaps.len() != levels.len()-1"
        );
        Self {
            hierarchy: h,
            repair_contiguity: true,
            state: NeedsPartition,
        }
    }

    pub(crate) fn with_contiguity_repair(mut self, enabled: bool) -> Self {
        self.repair_contiguity = enabled;
        self
    }

    pub(crate) fn initial_partition(
        self,
        init: &dyn InitialPartitioner,
        k: u32,
        seed: u64,
    ) -> Pipeline<NeedsRefinement> {
        let coarsest_n = self.hierarchy.coarsest().n();
        debug_assert!(
            coarsest_n >= 1,
            "coarsest graph must have at least 1 vertex, got {coarsest_n}"
        );
        let p = init.partition(self.hierarchy.coarsest(), k, seed);
        Pipeline {
            hierarchy: self.hierarchy,
            repair_contiguity: self.repair_contiguity,
            state: NeedsRefinement { partition: p },
        }
    }
}

impl Pipeline<NeedsRefinement> {
    pub(crate) fn from_initial_partition(
        hierarchy: CoarseningHierarchy,
        partition: Partition,
        repair_contiguity: bool,
    ) -> Self {
        Self {
            hierarchy,
            repair_contiguity,
            state: NeedsRefinement { partition },
        }
    }

    pub(crate) fn refine_and_project(self, refiner: &dyn Refiner) -> Pipeline<Complete> {
        let depth = self.hierarchy.depth();
        let mut current_p = self.state.partition;

        // Repair contiguity of initial partition BEFORE first FM pass.
        // Mirrors METIS contig.c: EnsureConnectivity after initial partition.
        if self.repair_contiguity {
            repair_contiguity(self.hierarchy.coarsest(), &mut current_p);
        }

        // Uncoarsen: refine at each coarser level, then project to finer.
        // levels[depth] = coarsest (where initial partition was computed).
        // We move from depth-1 down to 0 (finer levels).
        for lev in (0..depth).rev() {
            // Refine at the coarser level (lev+1)
            current_p = refiner.refine(&self.hierarchy.levels()[lev + 1], current_p);
            // Project down to the finer level (lev)
            let fine_assign = self.hierarchy.project_up(lev, &current_p.assignment);
            current_p = Partition {
                assignment: fine_assign,
                k: current_p.k,
                tpwgts: current_p.tpwgts.clone(),
            };
            // Repair contiguity after projection, BEFORE next FM pass.
            // FM operates on an already-connected partition at every level.
            if self.repair_contiguity {
                repair_contiguity(&self.hierarchy.levels()[lev], &mut current_p);
            }
        }
        // Final refinement at original level (level 0)
        current_p = refiner.refine(&self.hierarchy.levels()[0], current_p);

        Pipeline {
            hierarchy: self.hierarchy,
            repair_contiguity: self.repair_contiguity,
            state: Complete {
                partition: current_p,
            },
        }
    }
}

impl Pipeline<Complete> {
    pub(crate) fn into_partition(self) -> Partition {
        self.state.partition
    }
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coarsen::shem::SortedHeavyEdgeMatchWithParams;
    use crate::graph::CsrGraph;
    use crate::init::grow::GrowBisect;
    use crate::refine::fm::FiducciaMattheyses;

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

    #[test]
    fn pipeline_path10_k2_produces_valid_partition() {
        let g = path_graph(10);
        let coarsener = SortedHeavyEdgeMatchWithParams {
            coarsen_to: 20,
            k: 2,
        };
        let hierarchy = CoarseningHierarchy::build(&g, &coarsener).unwrap();
        let init = GrowBisect;
        let refiner = FiducciaMattheyses {
            niter: 10,
            contig_fm: true,
            objective: crate::api::ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        };

        let p = Pipeline::new(hierarchy)
            .initial_partition(&init, 2, 42)
            .refine_and_project(&refiner)
            .into_partition();

        assert_eq!(p.assignment.len(), 10);
        assert_eq!(p.k, 2);
        assert!(p.assignment.iter().all(|&x| x < 2));
        assert!(p.assignment.contains(&0));
        assert!(p.assignment.contains(&1));
    }

    #[test]
    fn pipeline_path100_k4_produces_valid_partition() {
        let g = path_graph(100);
        let coarsener = SortedHeavyEdgeMatchWithParams {
            coarsen_to: 20,
            k: 4,
        };
        let hierarchy = CoarseningHierarchy::build(&g, &coarsener).unwrap();
        let init = GrowBisect;
        let refiner = FiducciaMattheyses {
            niter: 10,
            contig_fm: true,
            objective: crate::api::ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        };

        let p = Pipeline::new(hierarchy)
            .initial_partition(&init, 4, 99)
            .refine_and_project(&refiner)
            .into_partition();

        assert_eq!(p.assignment.len(), 100);
        assert_eq!(p.k, 4);
        assert!(p.assignment.iter().all(|&x| x < 4));
    }

    #[test]
    fn pipeline_no_coarsening_still_works() {
        // When coarsener stops immediately (graph already small enough),
        // depth == 0, so the loop body never executes — only the final
        // refinement at level 0 runs.
        let g = path_graph(5);
        let coarsener = SortedHeavyEdgeMatchWithParams {
            coarsen_to: 20,
            k: 2,
        };
        // path5 has 5 nodes which is < threshold 40, so should_stop = true immediately
        let hierarchy = CoarseningHierarchy::build(&g, &coarsener).unwrap();
        assert_eq!(hierarchy.depth(), 0, "should not have coarsened");
        let init = GrowBisect;
        let refiner = FiducciaMattheyses {
            niter: 10,
            contig_fm: true,
            objective: crate::api::ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        };

        let p = Pipeline::new(hierarchy)
            .initial_partition(&init, 2, 0)
            .refine_and_project(&refiner)
            .into_partition();

        assert_eq!(p.assignment.len(), 5);
        assert_eq!(p.k, 2);
    }
}
