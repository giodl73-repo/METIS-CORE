use metis_core::{CsrGraph, MetisParams, MetisPartitioner, PartitionError, Partitioner};

#[test]
fn strict_csr_partition_is_accepted() {
    let graph = CsrGraph::from_csr_strict(&[0, 2, 4, 6, 8], &[1, 3, 0, 2, 1, 3, 0, 2], &[], &[])
        .expect("the reciprocal four-vertex cycle is valid");

    let partition = MetisPartitioner::from_params(MetisParams::recursive())
        .split(&graph, 2, Some(7))
        .expect("the valid cycle can be partitioned");

    partition
        .validate_for_graph(&graph)
        .expect("the accepted result satisfies graph invariants");
}

#[test]
fn asymmetric_csr_is_rejected_with_typed_error() {
    let result = CsrGraph::from_csr_strict(&[0, 1, 1], &[1], &[], &[]);

    assert!(matches!(result, Err(PartitionError::AsymmetricAdjacency)));
}
