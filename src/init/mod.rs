use crate::graph::{CsrGraph, Partition};

pub trait InitialPartitioner: Send + Sync {
    fn partition(&self, g: &CsrGraph, k: u32, seed: u64) -> Partition;
}

pub mod grow;
pub mod random;
pub mod multiconstraint;
