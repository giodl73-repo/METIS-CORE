use crate::graph::{CsrGraph, CoarseMap};

pub trait Coarsener: Send + Sync {
    /// Collapse g by one level. Output graph has strictly fewer vertices.
    /// Requires: g.is_valid(), g.n() >= 2.
    fn coarsen(&self, g: &CsrGraph) -> (CsrGraph, CoarseMap);

    /// True when g is small enough to partition directly.
    /// Guaranteed to return true when g.n() <= max(coarsen_to * k, 40).
    fn should_stop(&self, g: &CsrGraph) -> bool;
}

pub mod hem;
pub mod shem;
pub mod mindegree;
pub mod twohop;
