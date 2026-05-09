#![allow(clippy::items_after_test_module)]

pub mod api;
pub mod coarsen;
pub mod error;
pub mod graph;
pub mod init;
pub mod multilevel;
pub mod refine;

pub use api::{CoarseningMethod, MetisParams, MetisPartitioner, ObjectiveType, Partitioner};
pub use error::PartitionError;
pub use graph::{
    check_contiguity, extract_subgraph, repair_contiguity, CoarseMap, CsrGraph, Partition,
};

// ── METIS 5.x compatible entry points ────────────────────────────────────

/// Partition a graph using multilevel recursive bisection.
/// Mirrors `METIS_PartGraphRecursive` from the C library.
///
/// # Arguments
/// - `xadj`: CSR row pointer array (length n+1)
/// - `adjncy`: CSR column indices (length `xadj[n]`)
/// - `vwgt`: vertex weights (length n); pass `&[]` for unit weights
/// - `adjwgt`: edge weights (length adjncy.len()); pass `&[]` for unit weights
/// - `nparts`: number of parts k
/// - `params`: partitioning parameters (see [`api::MetisParams`])
///
/// # Returns
/// Partition assignment vector (length n), each value in `0..nparts`
pub fn part_recursive(
    xadj: &[u32],
    adjncy: &[u32],
    vwgt: &[i32],
    adjwgt: &[i32],
    nparts: u32,
    mut params: api::MetisParams,
) -> Result<Vec<u32>, PartitionError> {
    let defaults = api::MetisParams::default();
    if params.ncuts == defaults.ncuts {
        params.ncuts = api::MetisParams::recursive().ncuts;
    }
    params.use_recursive = true;
    let g = graph::CsrGraph::from_csr(xadj, adjncy, vwgt, adjwgt)?;
    api::MetisPartitioner::with_params(params, nparts)
        .split(&g, nparts, None)
        .map(|p| p.into_assignment())
}

/// Partition a graph using direct multilevel k-way partitioning.
/// Mirrors `METIS_PartGraphKway` from the C library.
///
/// Prefer `part_kway` for larger `nparts` when direct k-way partitioning is
/// desired.
pub fn part_kway(
    xadj: &[u32],
    adjncy: &[u32],
    vwgt: &[i32],
    adjwgt: &[i32],
    nparts: u32,
    mut params: api::MetisParams,
) -> Result<Vec<u32>, PartitionError> {
    params.use_recursive = false;
    let g = graph::CsrGraph::from_csr(xadj, adjncy, vwgt, adjwgt)?;
    api::MetisPartitioner::with_params(params, nparts)
        .split(&g, nparts, None)
        .map(|p| p.into_assignment())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_xadj_adjncy(n: usize) -> (Vec<u32>, Vec<u32>) {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 {
                adjncy.push((i - 1) as u32);
            }
            if i < n - 1 {
                adjncy.push((i + 1) as u32);
            }
            xadj.push(adjncy.len() as u32);
        }
        (xadj, adjncy)
    }

    #[test]
    fn part_recursive_bisects_path() {
        let (xadj, adjncy) = path_xadj_adjncy(10);
        let result = part_recursive(&xadj, &adjncy, &[], &[], 2, api::MetisParams::default());
        let assignment = result.expect("part_recursive must succeed on valid path graph");
        assert_eq!(
            assignment.len(),
            10,
            "assignment length must equal vertex count"
        );
        assert!(
            assignment.contains(&0) && assignment.contains(&1),
            "both parts must be present"
        );
    }

    #[test]
    fn part_kway_four_parts_path() {
        let (xadj, adjncy) = path_xadj_adjncy(16);
        let assignment = part_kway(&xadj, &adjncy, &[], &[], 4, api::MetisParams::default())
            .expect("part_kway must succeed");
        assert_eq!(assignment.len(), 16);
        for part in 0..4u32 {
            assert!(assignment.contains(&part), "part {part} must be present");
        }
    }

    #[test]
    fn part_recursive_with_vertex_weights() {
        let (xadj, adjncy) = path_xadj_adjncy(6);
        let vwgt = vec![2i32, 1, 3, 1, 2, 1]; // non-uniform weights
        let assignment = part_recursive(&xadj, &adjncy, &vwgt, &[], 2, api::MetisParams::default())
            .expect("part_recursive must handle non-uniform vertex weights");
        assert_eq!(assignment.len(), 6);
        assert!(assignment.iter().all(|&a| a < 2));
    }

    #[test]
    fn part_kway_with_edge_weights() {
        let (xadj, adjncy) = path_xadj_adjncy(8);
        let adjwgt = vec![5i32; adjncy.len()]; // uniform edge weights
        let assignment = part_kway(&xadj, &adjncy, &[], &adjwgt, 2, api::MetisParams::default())
            .expect("part_kway must handle edge weights");
        assert_eq!(assignment.len(), 8);
    }

    #[test]
    fn part_recursive_k1_all_same_part() {
        let (xadj, adjncy) = path_xadj_adjncy(8);
        let assignment = part_recursive(&xadj, &adjncy, &[], &[], 1, api::MetisParams::default())
            .expect("k=1 must succeed");
        assert!(
            assignment.iter().all(|&a| a == 0),
            "k=1 must assign all vertices to part 0"
        );
    }

    #[test]
    fn metis_params_recursive_defaults_match_pmetis() {
        let params = api::MetisParams::recursive();
        assert!(params.use_recursive);
        assert_eq!(params.ncuts, 4);
    }

    #[test]
    fn part_recursive_promotes_default_ncuts_even_with_seed() {
        let params = api::MetisParams {
            seed: Some(7),
            ..api::MetisParams::default()
        };
        let defaults = api::MetisParams::default();
        assert_eq!(params.ncuts, defaults.ncuts);

        let (xadj, adjncy) = path_xadj_adjncy(10);
        let assignment = part_recursive(&xadj, &adjncy, &[], &[], 2, params)
            .expect("recursive partition should succeed");
        assert_eq!(assignment.len(), 10);
    }

    #[test]
    fn metis_params_kway_defaults_match_kmetis() {
        let params = api::MetisParams::kway();
        assert!(!params.use_recursive);
        assert_eq!(params.ncuts, 1);
    }
}
