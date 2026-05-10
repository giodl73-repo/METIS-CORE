use crate::error::PartitionError;
use crate::graph::{CsrGraph, Partition};

pub trait InitialPartitioner: Send + Sync {
    fn partition(&self, g: &CsrGraph, k: u32, seed: u64) -> Result<Partition, PartitionError>;
}

pub mod grow;
pub mod multiconstraint;
pub mod random;
