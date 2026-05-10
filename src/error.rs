use thiserror::Error;

#[derive(Debug, Error)]
pub enum PartitionError {
    #[error("invalid graph: {0}")]
    InvalidGraph(&'static str),
    #[error("invalid partition: {0}")]
    InvalidPartition(&'static str),
    #[error("invalid parameters: {0}")]
    InvalidParams(&'static str),
    #[error("xadj length must be n + 1")]
    BadXadjLength,
    #[error("xadj must start at zero")]
    BadXadjStart,
    #[error("xadj terminator must equal adjncy length")]
    BadXadjTerminator,
    #[error("xadj must be monotonically nondecreasing")]
    NonMonotonicXadj,
    #[error("adjncy contains an invalid neighbor")]
    InvalidNeighbor,
    #[error("adjncy must describe an undirected graph")]
    AsymmetricAdjacency,
    #[error("undirected edge weights must match")]
    AsymmetricEdgeWeight,
    #[error("graph must be connected")]
    DisconnectedGraph,
    #[error("vertex weights must be positive")]
    NonPositiveVertexWeight,
    #[error("edge weights must be positive")]
    NonPositiveEdgeWeight,
    #[error("target weights length ({len}) must equal k ({k})")]
    TargetWeightLength { len: usize, k: u32 },
    #[error("target weights must be finite and positive")]
    InvalidTargetWeight,
    #[error("target weights must sum to 1.0")]
    TargetWeightsDoNotSum,
    #[error("parameter `{name}` must be positive")]
    NonPositiveParam { name: &'static str },
    #[error("parameter `{name}` is outside the supported range")]
    ParamOutOfRange { name: &'static str },
    #[error("k must be >= 1")]
    ZeroParts,
    #[error("k ({k}) exceeds vertex count ({n})")]
    TooManyParts { k: u32, n: usize },
    #[error("coarsening stalled: MAX_LEVELS=50 reached")]
    CoarseningStalled,
    #[error("partitioning failed to produce a candidate")]
    PartitioningFailed,
    #[error("vertex weight overflow during coarsening")]
    WeightOverflow,
    #[error("empty graph")]
    EmptyGraph,
}
