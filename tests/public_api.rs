use metis_core::advanced::{
    Coarsener, CoarseningHierarchy, FiducciaMattheyses, GrowBisect, InitialPartitioner, Refiner,
    SortedHeavyEdgeMatchWithParams,
};
use metis_core::{
    check_contiguity, part_kway, part_recursive, CoarseningMethod, CsrGraph, MetisParams,
    MetisPartitioner, ObjectiveType, Partition, PartitionError, Partitioner,
};

fn cycle_graph() -> CsrGraph {
    CsrGraph::from_csr(&[0, 2, 4, 6, 8], &[1, 3, 0, 2, 1, 3, 0, 2], &[], &[])
        .expect("cycle graph is valid")
}

#[test]
fn root_api_supports_metis_style_entry_points() {
    let xadj = [0, 2, 4, 6, 8];
    let adjncy = [1, 3, 0, 2, 1, 3, 0, 2];

    let recursive = part_recursive(&xadj, &adjncy, &[], &[], 2, MetisParams::recursive())
        .expect("recursive partition should succeed");
    let kway =
        part_kway(&xadj, &adjncy, &[], &[], 2, MetisParams::kway()).expect("kway should succeed");

    assert_eq!(recursive.len(), 4);
    assert_eq!(kway.len(), 4);
    assert!(recursive.iter().all(|&part| part < 2));
    assert!(kway.iter().all(|&part| part < 2));
}

#[test]
fn root_api_rejects_malformed_csr() {
    let result = part_kway(&[0, 2], &[0], &[], &[], 2, MetisParams::kway());

    assert!(matches!(result, Err(PartitionError::InvalidGraph(_))));
}

#[test]
fn root_api_rejects_empty_graph() {
    let result = part_kway(&[0], &[], &[], &[], 1, MetisParams::kway());

    assert!(matches!(result, Err(PartitionError::EmptyGraph)));
}

#[test]
fn root_api_rejects_zero_parts() {
    let graph = cycle_graph();
    let result = MetisPartitioner::new(0).split(&graph, 0, None);

    assert!(matches!(result, Err(PartitionError::ZeroParts)));
}

#[test]
fn root_api_rejects_too_many_parts() {
    let result = part_recursive(&[0, 1, 2], &[1, 0], &[], &[], 3, MetisParams::recursive());

    assert!(matches!(
        result,
        Err(PartitionError::TooManyParts { k: 3, n: 2 })
    ));
}

#[test]
fn root_api_supports_configured_partitioner() {
    let graph = cycle_graph();
    let params = MetisParams::kway()
        .with_seed(7)
        .with_ufactor(30)
        .with_ncuts(2)
        .with_coarsening_method(CoarseningMethod::Shem)
        .with_objective(ObjectiveType::Cut);

    params.validate_for_k(2).expect("params should be valid");
    let partition = MetisPartitioner::with_params(params, 2)
        .split(&graph, 2, None)
        .expect("partition should succeed");

    partition
        .validate_for_graph(&graph)
        .expect("partition should match graph");
    assert_eq!(check_contiguity(&graph, &partition), Ok(()));
}

#[test]
fn advanced_api_exposes_intentional_algorithm_components() {
    let graph = cycle_graph();
    let coarsener = SortedHeavyEdgeMatchWithParams::new(20, 2);
    assert!(coarsener.should_stop(&graph));

    let hierarchy = CoarseningHierarchy::build(&graph, &coarsener)
        .expect("hierarchy should build from valid graph");
    let initial = GrowBisect
        .partition(hierarchy.coarsest(), 2, 42)
        .expect("initial partition should succeed");
    let refined = FiducciaMattheyses::new(10, false, ObjectiveType::Cut, 0, 30)
        .refine(hierarchy.coarsest(), initial)
        .expect("refinement should succeed");
    let partition = if hierarchy.depth() == 0 {
        refined
    } else {
        Partition::new(
            hierarchy.project_up(hierarchy.depth() - 1, refined.assignment()),
            refined.k(),
        )
        .expect("projected partition should remain valid")
    };

    assert_eq!(partition.k(), 2);
    assert!(partition.assignment().iter().all(|&part| part < 2));
}
