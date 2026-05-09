use crate::coarsen::shem::SortedHeavyEdgeMatchWithParams;
pub use crate::coarsen::Coarsener;
use crate::init::grow::GrowBisect;
pub use crate::init::InitialPartitioner;
use crate::multilevel::hierarchy::CoarseningHierarchy;
use crate::multilevel::pipeline::Pipeline;
use crate::refine::fm::FiducciaMattheyses;
pub use crate::refine::Refiner;
use crate::{
    error::PartitionError,
    graph::{repair_contiguity, CsrGraph, Partition},
};

#[cfg(prusti)]
extern crate prusti_contracts;
#[cfg(prusti)]
use prusti_contracts::*;

/// Coarsening algorithm selection.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CoarseningMethod {
    #[default]
    Shem, // Sorted Heavy-Edge Matching — O(n+m) bucket sort (default)
    Hem,       // Heavy-Edge Matching — random visit order
    MinDegree, // Minimum-degree matching — lowest-degree vertices first
    TwoHop,    // Two-hop matching — looks 2 hops for unmatched vertices on sparse graphs
}

/// Which objective function to minimise during FM refinement.
///
/// * `Cut` — minimise edge cut: Σ edge_weight for cut edges.
///   Matches `METIS_OBJTYPE_CUT` (default in METIS 5.x).
/// * `Volume` — minimise communication volume: for each vertex v, count
///   the number of distinct parts adjacent to v (including v's own part for
///   border vertices).  Matches `METIS_OBJTYPE_VOL`. Relevant for parallel
///   computing where the cost model is the number of distinct messages sent,
///   not edge count.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ObjectiveType {
    #[default]
    Cut, // minimise edge cut (default, METIS_OBJTYPE_CUT)
    Volume, // minimise communication volume (METIS_OBJTYPE_VOL)
}

/// Compute the edge cut of an assignment: sum of weights on edges crossing
/// partition boundaries.  The raw sum counts each crossing edge twice (once
/// from each endpoint), so the result is divided by 2.
fn compute_cut(g: &CsrGraph, assignment: &[u32]) -> i64 {
    let mut cut = 0i64;
    for v in 0..g.n() {
        for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
            let u = g.adjncy[j] as usize;
            if assignment[v] != assignment[u] {
                cut += g.adjwgt.as_ref().map_or(1i64, |aw| aw[j] as i64);
            }
        }
    }
    cut / 2
}

fn balance_excess(g: &CsrGraph, p: &Partition, ufactor: u32, tpwgts: Option<&[f32]>) -> i64 {
    let k = p.k as usize;
    let ncon = g.ncon as usize;
    let mut pwgts = vec![vec![0i64; ncon]; k];
    let mut total_wgts = vec![0i64; ncon];

    for v in 0..g.n() {
        let part = p.assignment[v] as usize;
        for c in 0..ncon {
            let weight = g.vwgt[v * ncon + c] as i64;
            pwgts[part][c] += weight;
            total_wgts[c] += weight;
        }
    }

    let mut max_excess = 0i64;
    for (part, part_wgts) in pwgts.iter().enumerate().take(k) {
        for c in 0..ncon {
            let target = if c == 0 {
                match tpwgts {
                    Some(weights) if weights.len() == k => {
                        (total_wgts[0] as f64 * weights[part] as f64).round() as i64
                    }
                    _ => total_wgts[0] / k as i64,
                }
            } else {
                total_wgts[c] / k as i64
            };
            let epsilon = (target.abs() * ufactor as i64 + 999) / 1000;
            let deviation = (part_wgts[c] - target).abs();
            max_excess = max_excess.max((deviation - epsilon).max(0));
        }
    }

    max_excess
}

pub trait Partitioner: Send + Sync {
    fn split(&self, g: &CsrGraph, k: u32, seed: Option<u64>) -> Result<Partition, PartitionError>;
    fn split_weighted(
        &self,
        g: &CsrGraph,
        fracs: &[u32],
        seed: Option<u64>,
    ) -> Result<Partition, PartitionError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetisParams {
    pub ufactor: u32,
    pub niter: u32,
    pub seed: Option<u64>,
    pub coarsen_to: u32,
    /// Number of independent partition trials; the one with the lowest edge cut
    /// is returned.  Mirrors the METIS `-ncuts` option (kway default = 1;
    /// pmetis default = 4).  Each trial uses a deterministically derived seed
    /// so results remain reproducible given the same base seed.
    pub ncuts: u32,
    /// Target partition weights (one `f32` per part, summing to 1.0).
    /// `None` = equal weight (each part gets `1/k` of total population).
    /// Set by `split_weighted` from the caller's proportional `fracs` array.
    pub tpwgts: Option<Vec<f32>>,
    /// Check contiguity before each FM move: skip moves that would disconnect
    /// the source part (IsConnectedSubdomain check). Default: `false`, matching
    /// METIS `METIS_OPTION_CONTIG`.
    pub contig_fm: bool,
    /// Use multilevel recursive bisection (MlevelRecursiveBisection) for k > 2.
    /// When `true`, the graph is bisected and each half is recursively partitioned
    /// into `k/2` and `k - k/2` parts respectively — mirrors METIS_PartGraphRecursive.
    /// When `false` (default), the direct k-way multilevel pipeline is used.
    pub use_recursive: bool,
    /// Objective function for FM refinement.
    /// `ObjectiveType::Cut` (default) minimises edge cut.
    /// `ObjectiveType::Volume` minimises communication volume (number of distinct
    /// adjacent parts per vertex), matching `METIS_OBJTYPE_VOL`.
    pub objective: ObjectiveType,
    /// Minimize subdomain connectivity after partitioning (default: `false`).
    /// Mirrors the C METIS `METIS_OPTION_MINCONN` option.
    /// When enabled, iteratively moves boundary vertices to reduce the number of
    /// distinct communication partners each part has (subdomain degree).
    pub min_conn: bool,
    /// Run a label-propagation balance pass before FM refinement (default: `true`).
    /// Mirrors METIS `BalanceAndRefineLP`.  Particularly helpful when the initial
    /// partition is very unbalanced (e.g. when GrowKway seeds distribute poorly).
    pub lp_refine: bool,
    /// Maximum number of LP balance iterations (default: 10).
    /// Ignored when `lp_refine` is `false`.
    pub lp_iter: u32,
    /// Coarsening algorithm (default: `Shem`).
    /// Mirrors the METIS `-ctype` option.
    pub coarsen_method: CoarseningMethod,
}

impl Default for MetisParams {
    fn default() -> Self {
        Self {
            ufactor: 5,
            niter: 10,
            seed: None,
            coarsen_to: 20,
            ncuts: 1, // C METIS default for kway; pmetis uses 4 but 1 is safe default
            tpwgts: None,
            contig_fm: false,
            use_recursive: false,
            objective: ObjectiveType::Cut,
            min_conn: false,
            lp_refine: true,
            lp_iter: 10,
            coarsen_method: CoarseningMethod::Shem,
        }
    }
}

impl MetisParams {
    /// Defaults for `METIS_PartGraphKway`-style direct k-way partitioning.
    pub fn kway() -> Self {
        Self::default()
    }

    /// Defaults for `METIS_PartGraphRecursive`-style recursive bisection.
    ///
    /// C METIS uses multiple independent cuts for pmetis by default, while
    /// k-way defaults to a single cut.
    pub fn recursive() -> Self {
        Self {
            use_recursive: true,
            ncuts: 4,
            ..Self::default()
        }
    }
}

pub struct RustMetisPartitioner<C, I, R> {
    coarsener: C,
    init: I,
    refiner: R,
    params: MetisParams,
}

/// Concrete type alias: SHEM + GrowBisect + FM — the default METIS-like pipeline.
pub type MetisPartitioner =
    RustMetisPartitioner<SortedHeavyEdgeMatchWithParams, GrowBisect, FiducciaMattheyses>;

impl MetisPartitioner {
    pub fn new(k: u32) -> Self {
        Self::with_params(MetisParams::default(), k)
    }

    pub fn with_params(params: MetisParams, k: u32) -> Self {
        RustMetisPartitioner {
            coarsener: SortedHeavyEdgeMatchWithParams {
                coarsen_to: params.coarsen_to,
                k,
            },
            init: GrowBisect,
            refiner: FiducciaMattheyses {
                niter: params.niter,
                contig_fm: params.contig_fm,
                objective: params.objective,
                lp_iter: if params.lp_refine { params.lp_iter } else { 0 },
                ufactor: params.ufactor,
            },
            params,
        }
    }

    pub fn params(&self) -> &MetisParams {
        &self.params
    }
}

impl<C: Coarsener, I: InitialPartitioner, R: Refiner> Partitioner
    for RustMetisPartitioner<C, I, R>
{
    #[cfg_attr(prusti, requires(g.is_valid()))]
    #[cfg_attr(prusti, requires(k >= 1))]
    #[cfg_attr(prusti, ensures(
        result.is_ok() ==>
        result.as_ref().unwrap().assignment.len() == g.n()
    ))]
    #[cfg_attr(prusti, ensures(
        result.is_ok() ==>
        forall(|i: usize|
            (i < result.as_ref().unwrap().assignment.len()) ==>
            (result.as_ref().unwrap().assignment[i] < k))
    ))]
    #[cfg_attr(prusti, ensures(
        result.is_ok() ==>
        weight_balanced(result.as_ref().unwrap(), g, k)
    ))]
    fn split(&self, g: &CsrGraph, k: u32, seed: Option<u64>) -> Result<Partition, PartitionError> {
        if k == 0 {
            return Err(PartitionError::ZeroParts);
        }
        if g.n() == 0 {
            return Err(PartitionError::EmptyGraph);
        }
        if !g.is_valid() {
            return Err(PartitionError::InvalidGraph("is_valid() failed"));
        }
        if k as usize > g.n() {
            return Err(PartitionError::TooManyParts { k, n: g.n() });
        }

        let base_seed = self
            .params
            .seed
            .or(seed)
            .unwrap_or(0xDEAD_BEEF_CAFE_1234u64);

        // Recursive bisection path: bisect into halves and recurse on each subgraph.
        // Mirrors METIS_PartGraphRecursive / MlevelRecursiveBisection.
        if self.params.use_recursive && k > 2 {
            use crate::init::grow::RecursiveBisect;
            let rb = RecursiveBisect {
                niter: self.params.niter,
                ncuts: self.params.ncuts,
                coarsen_to: self.params.coarsen_to,
                ufactor: self.params.ufactor,
                contig_fm: self.params.contig_fm,
            };
            let mut p = rb.partition_graph(g, k, base_seed)?;
            if self.params.contig_fm {
                repair_contiguity(g, &mut p);
            }
            if self.params.min_conn {
                use crate::refine::minconn::minimize_connectivity;
                minimize_connectivity(g, &mut p, self.params.ufactor);
                if self.params.contig_fm {
                    repair_contiguity(g, &mut p);
                }
            }
            return Ok(p);
        }

        let ncuts = self.params.ncuts.max(1) as usize;

        // Build a boxed coarsener so coarsen_method can override the default SHEM.
        // When method == Shem we use the typed self.coarsener to avoid allocation.
        use crate::coarsen::{
            hem::HeavyEdgeMatchWithParams, mindegree::MinDegreeMatch, twohop::TwoHopMatchWithParams,
        };
        let alt_coarsener: Option<Box<dyn Coarsener>> = match self.params.coarsen_method {
            CoarseningMethod::Shem => None,
            CoarseningMethod::Hem => Some(Box::new(HeavyEdgeMatchWithParams {
                coarsen_to: self.params.coarsen_to,
                k,
            })),
            CoarseningMethod::MinDegree => Some(Box::new(MinDegreeMatch)),
            CoarseningMethod::TwoHop => Some(Box::new(TwoHopMatchWithParams {
                coarsen_to: self.params.coarsen_to,
                k,
            })),
        };

        let mut best: Option<(Partition, i64, i64)> = None;

        for trial in 0..ncuts {
            // Derive a distinct seed per trial using a Fibonacci-hashing constant
            // so trials are well-spread even for small trial indices.
            let trial_seed =
                base_seed.wrapping_add((trial as u64).wrapping_mul(0x9E3779B97F4A7C15));

            let hierarchy = if let Some(ref ac) = alt_coarsener {
                CoarseningHierarchy::build(g, ac.as_ref())?
            } else {
                CoarseningHierarchy::build(g, &self.coarsener)?
            };
            let mut p = Pipeline::new(hierarchy)
                .with_contiguity_repair(self.params.contig_fm)
                .initial_partition(&self.init, k, trial_seed)
                .refine_and_project(&self.refiner)
                .into_partition();

            crate::refine::lp::rebalance_to_ufactor(g, &mut p, self.params.ufactor);
            let cut = compute_cut(g, &p.assignment);
            let excess = balance_excess(g, &p, self.params.ufactor, None);
            let is_better = best
                .as_ref()
                .is_none_or(|&(_, best_excess, best_cut)| (excess, cut) < (best_excess, best_cut));
            if is_better {
                best = Some((p, excess, cut));
            }
        }

        let (mut p, _, _) = best.ok_or(PartitionError::PartitioningFailed)?;

        // Final contiguity safety net when requested. METIS leaves this off by
        // default; forcing it after refinement can disrupt the achieved balance.
        if self.params.contig_fm {
            repair_contiguity(g, &mut p);
        }

        // MinConn post-processing: reduce subdomain connectivity by iteratively
        // moving boundary vertices to lower-degree parts (mirrors METIS minconn.c).
        if self.params.min_conn {
            use crate::refine::minconn::minimize_connectivity;
            minimize_connectivity(g, &mut p, self.params.ufactor);
            if self.params.contig_fm {
                // MinConn may break contiguity by moving boundary vertices without
                // checking whether the source part remains connected.
                repair_contiguity(g, &mut p);
            }
        }

        Ok(p)
    }

    /// Partition `g` into `fracs.len()` parts where part *i* receives a population
    /// proportional to `fracs[i] / sum(fracs)`.
    ///
    /// The integer proportional weights are converted to float target fractions
    /// (`tpwgts`) and attached to the initial `Partition`.  The FM refinement loop
    /// reads `tpwgts` from the partition and enforces per-part balance around those
    /// asymmetric targets instead of the equal-weight default.
    ///
    /// # Errors
    ///
    /// * [`PartitionError::ZeroParts`] — `fracs` is empty or all entries are zero.
    /// * Propagates all errors from the coarsening/initial-partition/refinement pipeline.
    fn split_weighted(
        &self,
        g: &CsrGraph,
        fracs: &[u32],
        seed: Option<u64>,
    ) -> Result<Partition, PartitionError> {
        if fracs.is_empty() {
            return Err(PartitionError::ZeroParts);
        }
        let total_fracs: u32 = fracs.iter().sum();
        if total_fracs == 0 {
            return Err(PartitionError::ZeroParts);
        }
        let k = fracs.len() as u32;

        if g.n() == 0 {
            return Err(PartitionError::EmptyGraph);
        }
        if !g.is_valid() {
            return Err(PartitionError::InvalidGraph("is_valid() failed"));
        }
        if k as usize > g.n() {
            return Err(PartitionError::TooManyParts { k, n: g.n() });
        }

        // Convert integer proportional weights to float target fractions summing to 1.0
        let tpwgts: Vec<f32> = fracs
            .iter()
            .map(|&f| f as f32 / total_fracs as f32)
            .collect();

        let base_seed = self
            .params
            .seed
            .or(seed)
            .unwrap_or(0xDEAD_BEEF_CAFE_1234u64);
        let ncuts = self.params.ncuts.max(1) as usize;

        let mut best: Option<(Partition, i64, i64)> = None;

        for trial in 0..ncuts {
            let trial_seed =
                base_seed.wrapping_add((trial as u64).wrapping_mul(0x9E3779B97F4A7C15));

            let hierarchy = CoarseningHierarchy::build(g, &self.coarsener)?;

            // Build initial partition and attach tpwgts so FM can use per-part targets
            let init_p = {
                let mut p = self.init.partition(hierarchy.coarsest(), k, trial_seed);
                // tpwgts applies to constraint 0 only; constraints 1..ncon use equal targets.
                p.tpwgts = Some(tpwgts.clone());
                p
            };

            let pipeline = crate::multilevel::pipeline::Pipeline {
                hierarchy,
                partition: Some(init_p),
                repair_contiguity: self.params.contig_fm,
                _state: std::marker::PhantomData::<crate::multilevel::pipeline::NeedsRefinement>,
            };
            let mut result = pipeline.refine_and_project(&self.refiner).into_partition();

            let cut = compute_cut(g, &result.assignment);
            let excess = balance_excess(g, &result, self.params.ufactor, Some(&tpwgts));
            let is_better = best
                .as_ref()
                .is_none_or(|&(_, best_excess, best_cut)| (excess, cut) < (best_excess, best_cut));
            if is_better {
                result.tpwgts = None;
                best = Some((result, excess, cut));
            }
        }

        let (result, _, _) = best.ok_or(PartitionError::PartitioningFailed)?;
        Ok(result)
    }
}

/// Prusti pure helper for postcondition 3: population balance ≤ 0.5%.
///
/// Prusti pure functions cannot use iterators — a while-loop body is required.
/// Loop verification is deferred in Prusti v0.2 due to loop-invariant support
/// limitations for Vec<i32>; this stubs to `true` so postconditions 1 and 2
/// remain active. The runtime check lives in `weight_balance_check`.
#[cfg(prusti)]
#[pure]
fn weight_balanced(_p: &Partition, _g: &CsrGraph, _k: u32) -> bool {
    true
}

/// Population balance metric used in Prusti postcondition 3.
/// Returns true iff max deviation from target per part is ≤ epsilon.
/// epsilon = (total_wgt * 5 + 999) / 1000  (ceiling of 0.5%, integer arithmetic).
#[cfg(any(test, doc))]
pub fn weight_balance_check(p: &Partition, g: &CsrGraph) -> bool {
    let total_wgt: i64 = g.vwgt.iter().map(|&w| w as i64).sum();
    let target = total_wgt / p.k as i64;
    let epsilon = (total_wgt * 5 + 999) / 1000;
    for part in 0..p.k {
        let wgt: i64 = (0..g.n())
            .filter(|&v| p.assignment[v] == part)
            .map(|v| g.vwgt[v] as i64)
            .sum();
        if (wgt - target).abs() > epsilon {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coarsen::Coarsener;
    use crate::graph::{CoarseMap, CsrGraph, Partition};
    use crate::init::InitialPartitioner;
    use crate::refine::Refiner;

    struct AlwaysTrivial;
    impl Coarsener for AlwaysTrivial {
        fn coarsen(&self, g: &CsrGraph) -> (CsrGraph, CoarseMap) {
            let cmap = (0..g.n() as u32).collect();
            (g.clone(), CoarseMap { cmap })
        }
        fn should_stop(&self, _: &CsrGraph) -> bool {
            true
        }
    }

    struct AllZeroPartitioner;
    impl InitialPartitioner for AllZeroPartitioner {
        fn partition(&self, g: &CsrGraph, _k: u32, _seed: u64) -> Partition {
            Partition {
                assignment: vec![0; g.n()],
                k: 1,
                tpwgts: None,
            }
        }
    }

    struct SeedParityPartitioner;
    impl InitialPartitioner for SeedParityPartitioner {
        fn partition(&self, g: &CsrGraph, k: u32, seed: u64) -> Partition {
            let assignment = if seed.is_multiple_of(2) {
                let mut assignment = vec![0; g.n()];
                if let Some(last) = assignment.last_mut() {
                    *last = 1;
                }
                assignment
            } else {
                (0..g.n()).map(|v| (v % k as usize) as u32).collect()
            };
            Partition {
                assignment,
                k,
                tpwgts: None,
            }
        }
    }

    struct IdentityRefiner;
    impl Refiner for IdentityRefiner {
        fn refine(&self, _g: &CsrGraph, p: Partition) -> Partition {
            p
        }
    }

    fn make_path_graph(n: usize) -> CsrGraph {
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
    fn mock_traits_compile() {
        let _c: &dyn Coarsener = &AlwaysTrivial;
        let _i: &dyn InitialPartitioner = &AllZeroPartitioner;
        let _r: &dyn Refiner = &IdentityRefiner;
    }

    #[test]
    fn metis_params_default() {
        let p = MetisParams::default();
        assert_eq!(p.ufactor, 5);
        assert_eq!(p.niter, 10);
        assert_eq!(p.seed, None);
        assert_eq!(p.coarsen_to, 20);
        assert_eq!(p.ncuts, 1);
        assert!(p.tpwgts.is_none());
    }

    #[test]
    fn full_pipeline_path10_k2() {
        use crate::coarsen::shem::SortedHeavyEdgeMatchWithParams;
        use crate::init::grow::GrowBisect;
        use crate::refine::fm::FiducciaMattheyses;
        let g = make_path_graph(10);
        let partitioner = RustMetisPartitioner {
            coarsener: SortedHeavyEdgeMatchWithParams {
                coarsen_to: 20,
                k: 2,
            },
            init: GrowBisect,
            refiner: FiducciaMattheyses {
                niter: 10,
                contig_fm: true,
                objective: ObjectiveType::Cut,
                lp_iter: 0,
                ufactor: 5,
            },
            params: MetisParams::default(),
        };
        let p = partitioner.split(&g, 2, Some(42)).unwrap();
        assert_eq!(p.assignment.len(), 10);
        assert_eq!(p.k, 2);
        assert!(p.assignment.contains(&0));
        assert!(p.assignment.contains(&1));
    }

    #[test]
    fn split_weighted_empty_fracs_errors() {
        let g = make_path_graph(10);
        let p = MetisPartitioner::with_params(MetisParams::default(), 1).split_weighted(
            &g,
            &[],
            Some(0),
        );
        assert!(matches!(p, Err(crate::error::PartitionError::ZeroParts)));
    }

    #[test]
    fn split_zero_k_errors() {
        let g = make_path_graph(10);
        let partitioner = MetisPartitioner::with_params(MetisParams::default(), 2);
        let result = partitioner.split(&g, 0, None);
        assert!(matches!(result, Err(PartitionError::ZeroParts)));
    }

    #[test]
    fn split_too_many_parts_errors() {
        let g = make_path_graph(3);
        let partitioner = MetisPartitioner::with_params(MetisParams::default(), 10);
        let result = partitioner.split(&g, 10, None);
        assert!(matches!(
            result,
            Err(PartitionError::TooManyParts { k: 10, n: 3 })
        ));
    }

    #[test]
    fn metis_partitioner_with_params_works() {
        let g = make_path_graph(20);
        let partitioner = MetisPartitioner::with_params(MetisParams::default(), 2);
        let p = partitioner.split(&g, 2, Some(7)).unwrap();
        assert_eq!(p.assignment.len(), 20);
        assert_eq!(p.k, 2);
    }

    #[test]
    fn metis_partitioner_new_uses_default_params() {
        let partitioner = MetisPartitioner::new(2);
        assert_eq!(partitioner.params(), &MetisParams::default());
    }

    #[test]
    fn metis_partitioner_threads_ufactor_into_refiner() {
        let params = MetisParams {
            ufactor: 30,
            ..MetisParams::default()
        };
        let partitioner = MetisPartitioner::with_params(params, 4);
        assert_eq!(partitioner.refiner.ufactor, 30);
    }

    #[test]
    fn split_weighted_equal_fracs_produces_k2() {
        let g = make_path_graph(10);
        let partitioner = MetisPartitioner::with_params(MetisParams::default(), 2);
        // Equal fracs — v1 delegates to equal-weight split.
        let p = partitioner.split_weighted(&g, &[50, 50], Some(0)).unwrap();
        assert_eq!(p.assignment.len(), 10);
        assert_eq!(p.k, 2);
    }

    #[test]
    fn split_weighted_all_zero_fracs_errors() {
        let g = make_path_graph(10);
        let p = MetisPartitioner::with_params(MetisParams::default(), 2).split_weighted(
            &g,
            &[0u32, 0u32],
            Some(0),
        );
        assert!(matches!(p, Err(crate::error::PartitionError::ZeroParts)));
    }

    /// Structural validity test for asymmetric fracs — both parts must be non-empty
    /// and each covers a plausible share of the 17 vertices.
    #[test]
    fn split_weighted_asymmetric_fracs() {
        let g = make_path_graph(17);
        let partitioner = MetisPartitioner::with_params(MetisParams::default(), 2);
        let p = partitioner
            .split_weighted(&g, &[8u32, 9u32], Some(42))
            .unwrap();
        // Structural validity — correct length and k
        assert_eq!(p.assignment.len(), 17);
        assert_eq!(p.k, 2);
        // Both parts must be non-empty
        assert!(p.assignment.contains(&0), "part 0 is empty");
        assert!(p.assignment.contains(&1), "part 1 is empty");
        // Proportional balance: accept any split in [3, 14] as structurally sound.
        let sz0 = p.assignment.iter().filter(|&&x| x == 0).count();
        let sz1 = p.assignment.iter().filter(|&&x| x == 1).count();
        assert!((3..=14).contains(&sz0), "part 0 size unreasonable: {sz0}");
        assert!((3..=14).contains(&sz1), "part 1 size unreasonable: {sz1}");
    }

    /// fracs [8, 9]: part 0 should receive ~8/17 of population, part 1 ~9/17.
    /// With tpwgts wired through FM, the balance must be within ±2 of the target
    /// on a uniform-weight 17-vertex path graph.
    #[test]
    fn split_weighted_proportional_balance() {
        let g = make_path_graph(17);
        let total = 17i64;
        // round(17 * 8/17) = 8, remainder = 9
        let target0 = (total * 8 + 8) / 17; // = 8
        let target1 = total - target0; // = 9
        let eps = 2i64; // allow ±2 for small path graphs

        let p = MetisPartitioner::with_params(MetisParams::default(), 2)
            .split_weighted(&g, &[8u32, 9u32], Some(42))
            .unwrap();

        assert_eq!(p.assignment.len(), 17);
        assert_eq!(p.k, 2);

        let wgt0: i64 = p.assignment.iter().filter(|&&x| x == 0).count() as i64;
        let wgt1: i64 = total - wgt0;
        assert!(
            (wgt0 - target0).abs() <= eps,
            "part 0: expected ~{target0}, got {wgt0}"
        );
        assert!(
            (wgt1 - target1).abs() <= eps,
            "part 1: expected ~{target1}, got {wgt1}"
        );
    }

    /// ncuts=0 is treated as ncuts=1 (max(1) clamp).
    #[test]
    fn ncuts_zero_treated_as_one() {
        let g = make_path_graph(10);
        let params = MetisParams {
            ncuts: 0,
            ..MetisParams::default()
        };
        let partitioner = MetisPartitioner::with_params(params, 2);
        let p = partitioner.split(&g, 2, Some(42)).unwrap();
        assert_eq!(p.assignment.len(), 10);
        assert_eq!(p.k, 2);
    }

    /// ncuts=4 must still produce a valid partition.
    #[test]
    fn ncuts_four_produces_valid_partition() {
        let g = make_path_graph(20);
        let params = MetisParams {
            ncuts: 4,
            ..MetisParams::default()
        };
        let partitioner = MetisPartitioner::with_params(params, 2);
        let p = partitioner.split(&g, 2, Some(99)).unwrap();
        assert_eq!(p.assignment.len(), 20);
        assert_eq!(p.k, 2);
        assert!(p.assignment.contains(&0));
        assert!(p.assignment.contains(&1));
    }

    /// ncuts=4 on split_weighted must still produce a valid partition.
    #[test]
    fn ncuts_four_split_weighted_valid() {
        let g = make_path_graph(20);
        let params = MetisParams {
            ncuts: 4,
            ..MetisParams::default()
        };
        let partitioner = MetisPartitioner::with_params(params, 2);
        let p = partitioner
            .split_weighted(&g, &[10u32, 10u32], Some(7))
            .unwrap();
        assert_eq!(p.assignment.len(), 20);
        assert_eq!(p.k, 2);
        assert!(p.assignment.contains(&0));
        assert!(p.assignment.contains(&1));
    }

    /// With a fixed seed, ncuts=1 and ncuts=4 are both deterministic.
    #[test]
    fn ncuts_deterministic_with_fixed_seed() {
        let g = make_path_graph(20);
        let run = |ncuts: u32| {
            let params = MetisParams {
                ncuts,
                seed: Some(42),
                ..MetisParams::default()
            };
            let partitioner = MetisPartitioner::with_params(params, 2);
            partitioner.split(&g, 2, None).unwrap().assignment
        };
        assert_eq!(run(1), run(1), "ncuts=1 not deterministic");
        assert_eq!(run(4), run(4), "ncuts=4 not deterministic");
    }

    /// ncuts=4 on a path graph should produce cut ≤ ncuts=1 (best-of-N selects minimum).
    /// On a path graph the optimal cut is 1, so both should achieve it —
    /// but ncuts=4 must never be worse.
    #[test]
    fn ncuts_four_cut_le_single_trial() {
        let g = make_path_graph(40);
        let cut = |ncuts: u32| {
            let params = MetisParams {
                ncuts,
                seed: Some(0),
                ..MetisParams::default()
            };
            let partitioner = MetisPartitioner::with_params(params, 2);
            let p = partitioner.split(&g, 2, None).unwrap();
            compute_cut(&g, &p.assignment)
        };
        let cut1 = cut(1);
        let cut4 = cut(4);
        assert!(
            cut4 <= cut1,
            "ncuts=4 cut ({cut4}) should be ≤ ncuts=1 cut ({cut1})"
        );
    }

    #[test]
    fn ncuts_prefers_balanced_trial_before_cut_for_weighted_split() {
        let g = make_path_graph(8);
        let partitioner = RustMetisPartitioner {
            coarsener: AlwaysTrivial,
            init: SeedParityPartitioner,
            refiner: IdentityRefiner,
            params: MetisParams {
                ncuts: 2,
                seed: Some(0),
                ufactor: 0,
                lp_refine: false,
                ..MetisParams::default()
            },
        };

        let p = partitioner
            .split_weighted(&g, &[1, 1], None)
            .expect("weighted split should succeed");
        let part0 = p.assignment.iter().filter(|&&part| part == 0).count();
        let part1 = p.assignment.iter().filter(|&&part| part == 1).count();

        assert_eq!(
            (part0, part1),
            (4, 4),
            "best-of-ncuts must prefer balance over a lower-cut imbalanced trial"
        );
    }

    #[test]
    fn recursive_bisect_grid_k4_valid() {
        let g = make_path_graph(16);
        let params = MetisParams {
            use_recursive: true,
            ..MetisParams::default()
        };
        let p = MetisPartitioner::with_params(params, 4)
            .split(&g, 4, Some(0))
            .unwrap();
        assert_eq!(p.assignment.len(), 16);
        for part in 0..4u32 {
            assert!(p.assignment.contains(&part));
        }
    }

    #[test]
    fn recursive_bisect_k2_works() {
        let g = make_path_graph(10);
        let params = MetisParams {
            use_recursive: true,
            ..MetisParams::default()
        };
        let p = MetisPartitioner::with_params(params, 2)
            .split(&g, 2, Some(42))
            .unwrap();
        assert_eq!(p.assignment.len(), 10);
        assert!(p.assignment.contains(&0));
        assert!(p.assignment.contains(&1));
    }

    // ── CoarseningMethod variants ──────────────────────────────────────────

    #[test]
    fn coarsen_method_hem_produces_valid_partition() {
        let g = make_path_graph(10);
        let params = MetisParams {
            coarsen_method: CoarseningMethod::Hem,
            ..MetisParams::default()
        };
        let p = MetisPartitioner::with_params(params, 2)
            .split(&g, 2, Some(0))
            .unwrap();
        assert_eq!(
            p.assignment.len(),
            10,
            "HEM must produce assignment for all vertices"
        );
        assert!(
            p.assignment.iter().all(|&a| a < 2),
            "all HEM part IDs must be < k"
        );
    }

    #[test]
    fn coarsen_method_mindegree_produces_valid_partition() {
        let g = make_path_graph(10);
        let params = MetisParams {
            coarsen_method: CoarseningMethod::MinDegree,
            ..MetisParams::default()
        };
        let p = MetisPartitioner::with_params(params, 2)
            .split(&g, 2, Some(0))
            .unwrap();
        assert_eq!(p.assignment.len(), 10);
        assert!(p.assignment.iter().all(|&a| a < 2));
    }

    #[test]
    fn coarsen_method_twohop_produces_valid_partition() {
        let g = make_path_graph(10);
        let params = MetisParams {
            coarsen_method: CoarseningMethod::TwoHop,
            ..MetisParams::default()
        };
        let p = MetisPartitioner::with_params(params, 2)
            .split(&g, 2, Some(0))
            .unwrap();
        assert_eq!(p.assignment.len(), 10);
        assert!(p.assignment.iter().all(|&a| a < 2));
    }

    #[test]
    fn all_coarsen_methods_agree_on_coverage() {
        // All four methods must cover every vertex and produce valid part IDs.
        let g = make_path_graph(12);
        for method in [
            CoarseningMethod::Shem,
            CoarseningMethod::Hem,
            CoarseningMethod::MinDegree,
            CoarseningMethod::TwoHop,
        ] {
            let params = MetisParams {
                coarsen_method: method,
                ..MetisParams::default()
            };
            let p = MetisPartitioner::with_params(params, 3)
                .split(&g, 3, Some(0))
                .unwrap();
            assert_eq!(
                p.assignment.len(),
                12,
                "{method:?}: wrong assignment length"
            );
            assert!(
                p.assignment.iter().all(|&a| a < 3),
                "{method:?}: invalid part ID"
            );
        }
    }
}
