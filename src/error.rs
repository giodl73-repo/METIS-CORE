use thiserror::Error;

#[derive(Debug, Error)]
pub enum PartitionError {
    #[error("invalid graph: {0}")]
    InvalidGraph(&'static str),
    #[error("k must be >= 1")]
    ZeroParts,
    #[error("k ({k}) exceeds vertex count ({n})")]
    TooManyParts { k: u32, n: usize },
    #[error("coarsening stalled: MAX_LEVELS=50 reached")]
    CoarseningStalled,
    #[error("vertex weight overflow during coarsening")]
    WeightOverflow,
    #[error("empty graph")]
    EmptyGraph,
}
