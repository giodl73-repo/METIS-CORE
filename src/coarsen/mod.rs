use crate::error::PartitionError;
use crate::graph::{CoarseMap, CsrGraph};

pub trait Coarsener: Send + Sync {
    /// Collapse g by one level. Output graph has strictly fewer vertices.
    /// Requires: g.is_valid(), g.n() >= 2.
    fn coarsen(&self, g: &CsrGraph) -> Result<(CsrGraph, CoarseMap), PartitionError>;

    /// True when g is small enough to partition directly.
    /// Guaranteed to return true when g.n() <= max(coarsen_to * k, 40).
    fn should_stop(&self, g: &CsrGraph) -> bool;
}

pub mod hem;
pub mod mindegree;
pub mod shem;
pub mod twohop;
