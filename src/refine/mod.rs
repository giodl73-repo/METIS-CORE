use crate::error::PartitionError;
use crate::graph::{CsrGraph, Partition};

pub trait Refiner: Send + Sync {
    /// Refine partition p on graph g. Output cut <= input cut.
    fn refine(&self, g: &CsrGraph, p: Partition) -> Result<Partition, PartitionError>;
}

pub mod boundary;
pub mod fm;
pub mod gain;
pub mod kway;
pub mod lp;
pub mod minconn;
