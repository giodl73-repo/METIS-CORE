use crate::graph::{CsrGraph, Partition};

pub trait Refiner: Send + Sync {
    /// Refine partition p on graph g. Output cut <= input cut.
    fn refine(&self, g: &CsrGraph, p: Partition) -> Partition;
}

pub mod boundary;
pub mod fm;
pub mod gain;
pub mod kway;
pub mod lp;
pub mod minconn;
