use crate::graph::{CsrGraph, CoarseMap};
use crate::coarsen::Coarsener;
use crate::error::PartitionError;

pub const MAX_LEVELS: usize = 50;

// ── arena ─────────────────────────────────────────────────────────────────

pub struct CoarseningHierarchy {
    pub levels: Vec<CsrGraph>,  // [0] = original … [depth] = coarsest
    pub cmaps:  Vec<CoarseMap>, // cmaps[i] maps levels[i+1] → levels[i]
}

impl CoarseningHierarchy {
    /// Build the coarsening hierarchy from `g` using `coarsener`.
    ///
    /// Coarsens repeatedly until `coarsener.should_stop()` returns true
    /// or the graph shrinks to a single vertex.  If neither condition is
    /// satisfied within MAX_LEVELS iterations, returns
    /// `Err(PartitionError::CoarseningStalled)`.
    pub fn build(g: &CsrGraph, coarsener: &dyn Coarsener) -> Result<Self, PartitionError> {
        let mut levels = vec![g.clone()];
        let mut cmaps  = Vec::new();

        for _ in 0..MAX_LEVELS {
            let current = levels.last().unwrap();
            if coarsener.should_stop(current) { break; }
            if current.n() <= 1 { break; }
            let (coarsened, cmap) = coarsener.coarsen(current);
            cmaps.push(cmap);
            levels.push(coarsened);
        }

        // If should_stop is still false on the last level we either hit the
        // n<=1 guard or exhausted MAX_LEVELS without stopping — both are stalls.
        if !coarsener.should_stop(levels.last().unwrap()) {
            return Err(PartitionError::CoarseningStalled);
        }

        Ok(Self { levels, cmaps })
    }

    /// Return a reference to the coarsest (deepest) level.
    pub fn coarsest(&self) -> &CsrGraph { self.levels.last().unwrap() }

    /// Number of coarsening steps performed (0 means no coarsening happened).
    pub fn depth(&self) -> usize { self.levels.len() - 1 }

    /// Project a partition assignment from coarser level `lev+1` down to
    /// finer level `lev`.
    ///
    /// Returns `fine` such that `fine[v] = coarse_assign[cmap[lev][v]]`.
    pub fn project_up(&self, lev: usize, coarse_assign: &[u32]) -> Vec<u32> {
        self.cmaps[lev].cmap.iter()
            .map(|&c| coarse_assign[c as usize])
            .collect()
    }
}

// ── test helpers ──────────────────────────────────────────────────────────

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

/// A coarsener whose `should_stop` always returns false — used to trigger
/// the CoarseningStalled error path.
#[cfg(test)]
struct NeverStops;

#[cfg(test)]
impl Coarsener for NeverStops {
    fn coarsen(&self, g: &CsrGraph) -> (CsrGraph, CoarseMap) {
        // Delegate to SHEM so the graph actually shrinks (avoids infinite loop),
        // but should_stop always returns false so we hit CoarseningStalled.
        crate::coarsen::shem::SortedHeavyEdgeMatch.coarsen(g)
    }
    fn should_stop(&self, _: &CsrGraph) -> bool { false }
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coarsen::shem::SortedHeavyEdgeMatchWithParams;

    #[test]
    fn hierarchy_builds_from_path() {
        let g = path_graph(100);
        let coarsener = SortedHeavyEdgeMatchWithParams { coarsen_to: 20, k: 2 };
        let h = CoarseningHierarchy::build(&g, &coarsener).unwrap();
        assert!(h.levels.len() >= 2, "should have at least 2 levels");
        assert!(
            h.coarsest().n() <= 40,
            "coarsest must satisfy should_stop threshold (<=40)"
        );
        assert!(h.coarsest().is_valid(), "coarsest graph must be valid");
        assert_eq!(
            h.cmaps.len(),
            h.levels.len() - 1,
            "one cmap per coarsening step"
        );
    }

    #[test]
    fn hierarchy_stalls_returns_error() {
        let g = path_graph(10);
        let result = CoarseningHierarchy::build(&g, &NeverStops);
        assert!(
            matches!(result, Err(PartitionError::CoarseningStalled)),
            "NeverStops coarsener must return CoarseningStalled error, got: {:?}",
            result.map(|_| ())
        );
    }

    #[test]
    fn project_up_correct() {
        let g = path_graph(100);
        let coarsener = SortedHeavyEdgeMatchWithParams { coarsen_to: 20, k: 2 };
        let h = CoarseningHierarchy::build(&g, &coarsener).unwrap();
        let depth = h.depth();
        // All-zero coarse assignment should project to all-zero fine assignment
        let coarse_assign: Vec<u32> = vec![0; h.coarsest().n()];
        let fine = h.project_up(depth - 1, &coarse_assign);
        assert_eq!(
            fine.len(),
            h.levels[depth - 1].n(),
            "projected partition must match finer level vertex count"
        );
        assert!(
            fine.iter().all(|&a| a == 0),
            "all-zero coarse assignment must project to all-zero fine assignment"
        );
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;
    use crate::coarsen::shem::SortedHeavyEdgeMatchWithParams;

    fn kani_path(n: usize) -> CsrGraph {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 { adjncy.push((i-1) as u32); }
            if i < n-1 { adjncy.push((i+1) as u32); }
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
    }

    /// Proves: CoarseningHierarchy::build() terminates without panic
    /// for path graphs up to n=32. Either returns Ok (hierarchy built)
    /// or Err(CoarseningStalled) — no panic.
    #[kani::proof]
    #[kani::unwind(33)]
    fn verify_hierarchy_no_panic() {
        let n: usize = kani::any_where(|&n: &usize| n >= 4 && n <= 32);
        let k: u32   = kani::any_where(|&k: &u32| k >= 1 && k <= 4);
        let g = kani_path(n);
        kani::assume(g.is_valid());

        let coarsener = SortedHeavyEdgeMatchWithParams { coarsen_to: 20, k };
        // Must not panic — either Ok or Err(CoarseningStalled)
        let _ = CoarseningHierarchy::build(&g, &coarsener);
    }
}
