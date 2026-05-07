pub mod error;
pub mod graph;
pub mod coarsen;
pub mod init;
pub mod refine;
pub mod multilevel;
pub mod api;

pub use error::PartitionError;
pub use graph::{CsrGraph, Partition, CoarseMap, check_contiguity, repair_contiguity, extract_subgraph};
pub use api::{Partitioner, MetisParams, ObjectiveType, CoarseningMethod};

// ── METIS 5.x compatible entry points ────────────────────────────────────

/// Partition a graph using multilevel recursive bisection.
/// Mirrors `METIS_PartGraphRecursive` from the C library.
///
/// # Arguments
/// - `xadj`: CSR row pointer array (length n+1)
/// - `adjncy`: CSR column indices (length xadj[n])
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
    params: api::MetisParams,
) -> Result<Vec<u32>, PartitionError> {
    let n = xadj.len().saturating_sub(1);
    let g = graph::CsrGraph {
        xadj:   xadj.to_vec(),
        adjncy: adjncy.to_vec(),
        ncon:   1,
        vwgt:   if vwgt.is_empty() { vec![1i32; n] } else { vwgt.to_vec() },
        adjwgt: if adjwgt.is_empty() { None } else { Some(adjwgt.to_vec()) },
    };
    api::MetisPartitioner::with_params(params, nparts)
        .split(&g, nparts, None)
        .map(|p| p.assignment)
}

/// Partition a graph using direct multilevel k-way partitioning.
/// Mirrors `METIS_PartGraphKway` from the C library.
///
/// Prefer `part_kway` for nparts > 8; both entry points use the same
/// algorithm in this implementation (unified multilevel pipeline).
pub fn part_kway(
    xadj: &[u32],
    adjncy: &[u32],
    vwgt: &[i32],
    adjwgt: &[i32],
    nparts: u32,
    params: api::MetisParams,
) -> Result<Vec<u32>, PartitionError> {
    part_recursive(xadj, adjncy, vwgt, adjwgt, nparts, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_xadj_adjncy(n: usize) -> (Vec<u32>, Vec<u32>) {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 { adjncy.push((i-1) as u32); }
            if i < n-1 { adjncy.push((i+1) as u32); }
            xadj.push(adjncy.len() as u32);
        }
        (xadj, adjncy)
    }

    #[test]
    fn part_recursive_bisects_path() {
        let (xadj, adjncy) = path_xadj_adjncy(10);
        let result = part_recursive(&xadj, &adjncy, &[], &[], 2, api::MetisParams::default());
        let assignment = result.expect("part_recursive must succeed on valid path graph");
        assert_eq!(assignment.len(), 10, "assignment length must equal vertex count");
        assert!(assignment.contains(&0) && assignment.contains(&1),
            "both parts must be present");
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
        assert!(assignment.iter().all(|&a| a == 0), "k=1 must assign all vertices to part 0");
    }
}
