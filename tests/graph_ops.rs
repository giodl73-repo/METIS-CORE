//! L0 tests for graph construction, coarsening, bisection tree,
//! edge matching, balance checking, and pure math helpers.
//!
//! 30 new tests covering:
//!   - Graph construction / CSR format (10)
//!   - extract_subgraph (3)
//!   - check_contiguity / repair_contiguity (4)
//!   - Coarsening operations (4)
//!   - Bisection hierarchy (3)
//!   - Balance / population checks (3)
//!   - MetisParams / pure math (3)

use metis_core::advanced::{
    Coarsener, CoarseningHierarchy, HeavyEdgeMatch, SortedHeavyEdgeMatch,
    SortedHeavyEdgeMatchWithParams,
};
use metis_core::{
    check_contiguity, extract_subgraph, repair_contiguity, CoarseningMethod, CsrGraph, MetisParams,
    MetisPartitioner, ObjectiveType, Partition, Partitioner,
};

// ─── shared graph helpers ───────────────────────────────────────────────────

fn path_graph(n: usize) -> CsrGraph {
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
    CsrGraph::new(xadj, adjncy, 1, vec![1i32; n], None).expect("path graph is valid")
}

fn grid_graph(rows: usize, cols: usize) -> CsrGraph {
    let n = rows * cols;
    let mut xadj = vec![0u32];
    let mut adjncy = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let mut nbrs = Vec::new();
            if r > 0 {
                nbrs.push((r - 1) * cols + c);
            }
            if r < rows - 1 {
                nbrs.push((r + 1) * cols + c);
            }
            if c > 0 {
                nbrs.push(r * cols + (c - 1));
            }
            if c < cols - 1 {
                nbrs.push(r * cols + (c + 1));
            }
            for &u in &nbrs {
                adjncy.push(u as u32);
            }
            xadj.push(adjncy.len() as u32);
        }
    }
    CsrGraph::new(xadj, adjncy, 1, vec![1i32; n], None).expect("grid graph is valid")
}

fn weighted_path_4() -> CsrGraph {
    // 0 --10-- 1 --1-- 2 --10-- 3
    CsrGraph::new(
        vec![0, 1, 3, 5, 6],
        vec![1, 0, 2, 1, 3, 2],
        1,
        vec![1; 4],
        Some(vec![10, 10, 1, 1, 10, 10]),
    )
    .expect("weighted path graph is valid")
}

/// Compute population balance: max absolute deviation from target as a fraction.
fn max_balance_deviation(p: &Partition, g: &CsrGraph) -> f64 {
    let total: i64 = g.vwgt().iter().map(|&w| w as i64).sum();
    let target = total as f64 / p.k() as f64;
    (0..p.k())
        .map(|part| {
            let wgt: i64 = (0..g.n())
                .filter(|&v| p.assignment()[v] == part)
                .map(|v| g.vwgt()[v] as i64)
                .sum();
            (wgt as f64 - target).abs() / total as f64
        })
        .fold(0.0_f64, f64::max)
}

// ─── Graph construction / CSR format (10 tests) ───────────────────────────

#[test]
fn csr_xadj_length_is_n_plus_1() {
    let g = path_graph(8);
    assert_eq!(g.xadj().len(), g.n() + 1);
}

#[test]
fn csr_xadj_starts_at_zero() {
    let g = path_graph(5);
    assert_eq!(g.xadj()[0], 0);
}

#[test]
fn csr_xadj_monotone_nondecreasing() {
    let g = grid_graph(4, 4);
    for i in 0..g.n() {
        assert!(
            g.xadj()[i] <= g.xadj()[i + 1],
            "xadj[{i}]={} > xadj[{}]={}",
            g.xadj()[i],
            i + 1,
            g.xadj()[i + 1]
        );
    }
}

#[test]
fn csr_degree_path_endpoints_is_1() {
    // endpoints of a path have exactly 1 neighbour
    let g = path_graph(6);
    let deg_first = (g.xadj()[1] - g.xadj()[0]) as usize;
    let deg_last = (g.xadj()[6] - g.xadj()[5]) as usize;
    assert_eq!(deg_first, 1, "first endpoint degree must be 1");
    assert_eq!(deg_last, 1, "last endpoint degree must be 1");
}

#[test]
fn csr_degree_grid_interior_is_4() {
    // Interior vertex (1,1) of 4×4 grid → degree 4
    let g = grid_graph(4, 4);
    let v = 5usize; // row=1, col=1
    let deg = (g.xadj()[v + 1] - g.xadj()[v]) as usize;
    assert_eq!(deg, 4, "interior grid vertex must have degree 4");
}

#[test]
fn csr_total_edges_path_undirected() {
    let n = 10usize;
    let g = path_graph(n);
    // undirected: n-1 edges, each stored in both directions
    assert_eq!(g.adjncy().len(), 2 * (n - 1));
}

#[test]
fn csr_adjncy_no_self_loops() {
    let g = grid_graph(3, 3);
    for v in 0..g.n() {
        for j in g.xadj()[v] as usize..g.xadj()[v + 1] as usize {
            assert_ne!(g.adjncy()[j] as usize, v, "self loop at vertex {v}");
        }
    }
}

#[test]
fn csr_all_vertex_weights_positive() {
    let g = path_graph(12);
    assert!(
        g.vwgt().iter().all(|&w| w > 0),
        "all vertex weights must be positive"
    );
}

#[test]
fn csr_adjwgt_length_matches_adjncy() {
    let g = weighted_path_4();
    let aw = g.adjwgt().unwrap();
    assert_eq!(
        aw.len(),
        g.adjncy().len(),
        "adjwgt.len() must equal adjncy.len()"
    );
}

#[test]
fn csr_is_valid_returns_true_for_well_formed_graph() {
    let g = grid_graph(5, 5);
    assert!(g.is_valid(), "5x5 grid must pass is_valid()");
}

// ─── extract_subgraph (3 tests) ───────────────────────────────────────────

#[test]
fn extract_subgraph_parts_cover_all_vertices() {
    let g = path_graph(6);
    let assignment: Vec<u32> = (0..6).map(|i| if i < 3 { 0 } else { 1 }).collect();
    let (_, _, l2g0) = extract_subgraph(&g, &assignment, 0);
    let (_, _, l2g1) = extract_subgraph(&g, &assignment, 1);
    assert_eq!(
        l2g0.len() + l2g1.len(),
        6,
        "both parts together must cover all 6 vertices"
    );
}

#[test]
fn extract_subgraph_size_equals_part_population() {
    let g = path_graph(8);
    let assignment: Vec<u32> = (0..8).map(|i| if i < 5 { 0 } else { 1 }).collect();
    let (sub, _, l2g) = extract_subgraph(&g, &assignment, 0);
    assert_eq!(sub.n(), 5, "part 0 subgraph must have 5 vertices");
    assert_eq!(l2g.len(), 5, "local-to-global map must have 5 entries");
}

#[test]
fn extract_subgraph_adjncy_in_range() {
    let g = path_graph(6);
    let assignment: Vec<u32> = (0..6).map(|i| if i < 3 { 0 } else { 1 }).collect();
    let (sub, _, _) = extract_subgraph(&g, &assignment, 0);
    for j in 0..sub.adjncy().len() {
        assert!(
            (sub.adjncy()[j] as usize) < sub.n(),
            "sub-graph adjacency index {} out of range for n={}",
            sub.adjncy()[j],
            sub.n()
        );
    }
}

// ─── check_contiguity / repair_contiguity (4 tests) ──────────────────────

#[test]
fn check_contiguity_ok_for_contiguous_bisection() {
    let g = path_graph(6);
    let partition = Partition::new(vec![0, 0, 0, 1, 1, 1], 2).expect("partition is valid");
    assert!(
        check_contiguity(&g, &partition).is_ok(),
        "contiguous bisection on path must pass contiguity check"
    );
}

#[test]
fn check_contiguity_err_for_checkerboard_assignment() {
    let g = path_graph(6);
    // Alternating 0,1,0,1,0,1 — every part is disconnected on a path
    let partition = Partition::new(vec![0, 1, 0, 1, 0, 1], 2).expect("partition is valid");
    assert!(
        check_contiguity(&g, &partition).is_err(),
        "checkerboard assignment must fail contiguity check"
    );
}

#[test]
fn repair_contiguity_leaves_already_contiguous_unchanged() {
    let g = path_graph(8);
    let mut partition =
        Partition::new(vec![0, 0, 0, 0, 1, 1, 1, 1], 2).expect("partition is valid");
    let reassigned = repair_contiguity(&g, &mut partition);
    assert_eq!(
        reassigned, 0,
        "no reassignments needed for contiguous partition"
    );
    assert!(check_contiguity(&g, &partition).is_ok());
}

#[test]
fn repair_contiguity_fixes_disconnected_assignment() {
    let g = path_graph(6);
    // Part 0 = {0, 5}, Part 1 = {1,2,3,4} — part 0 is disconnected
    let mut partition = Partition::new(vec![0, 1, 1, 1, 1, 0], 2).expect("partition is valid");
    let _ = repair_contiguity(&g, &mut partition);
    assert!(
        check_contiguity(&g, &partition).is_ok(),
        "repair_contiguity must restore contiguity"
    );
}

// ─── Coarsening operations (4 tests) ──────────────────────────────────────

#[test]
fn hem_preserves_adjwgt_none_invariant() {
    let g = path_graph(8);
    assert!(g.adjwgt().is_none());
    let (c, _) = HeavyEdgeMatch.coarsen(&g);
    assert!(
        c.adjwgt().is_none(),
        "unweighted in -> unweighted out (HEM)"
    );
}

#[test]
fn shem_coarsens_weighted_graph_to_fewer_vertices() {
    let g = weighted_path_4();
    let (c, _) = SortedHeavyEdgeMatch.coarsen(&g);
    assert!(c.n() < 4, "SHEM must reduce vertex count of 4-vertex graph");
    assert!(c.is_valid(), "SHEM output must be a valid graph");
}

#[test]
fn coarsening_preserves_total_vertex_weight() {
    let g = path_graph(6);
    let total_original: i32 = g.vwgt().iter().sum();
    let (c, _) = SortedHeavyEdgeMatch.coarsen(&g);
    let total_coarse: i32 = c.vwgt().iter().sum();
    assert_eq!(
        total_original, total_coarse,
        "total vertex weight must be preserved through coarsening"
    );
}

#[test]
fn heavy_edge_match_path4_builds_valid_smaller_graph() {
    let g = path_graph(4);
    let (c, cmap) = HeavyEdgeMatch.coarsen(&g);
    assert!(c.n() < g.n(), "path4 should coarsen to fewer vertices");
    assert_eq!(cmap.len(), 4, "cmap must retain length of original graph");
    assert!(c.is_valid(), "coarse graph must be valid");
}

// ─── Bisection hierarchy (3 tests) ────────────────────────────────────────

#[test]
fn hierarchy_depth_at_least_1_for_large_graph() {
    let g = path_graph(100);
    let coarsener = SortedHeavyEdgeMatchWithParams {
        coarsen_to: 20,
        k: 2,
    };
    let h = CoarseningHierarchy::build(&g, &coarsener).unwrap();
    assert!(
        h.depth() >= 1,
        "hierarchy must have at least 1 coarsening level"
    );
}

#[test]
fn hierarchy_coarsest_satisfies_should_stop() {
    let g = path_graph(50);
    let coarsener = SortedHeavyEdgeMatchWithParams {
        coarsen_to: 20,
        k: 2,
    };
    let h = CoarseningHierarchy::build(&g, &coarsener).unwrap();
    // threshold = max(coarsen_to * k, 40) = max(40, 40) = 40
    assert!(
        h.coarsest().n() <= 40,
        "coarsest level n={} must satisfy stop condition (<=40)",
        h.coarsest().n()
    );
}

#[test]
fn hierarchy_all_intermediate_levels_valid() {
    let g = path_graph(60);
    let coarsener = SortedHeavyEdgeMatchWithParams {
        coarsen_to: 20,
        k: 2,
    };
    let h = CoarseningHierarchy::build(&g, &coarsener).unwrap();
    for (i, level) in h.levels().iter().enumerate() {
        assert!(
            level.is_valid(),
            "hierarchy level {i} must be a valid CSR graph"
        );
    }
}

// ─── Balance / population checks (3 tests) ────────────────────────────────

#[test]
fn metis_uniform_bisection_within_ufactor_tolerance() {
    let g = path_graph(20);
    let partitioner = MetisPartitioner::with_params(MetisParams::default(), 2);
    let p = partitioner.split(&g, 2, Some(0)).unwrap();
    let sz0 = p.assignment().iter().filter(|&&x| x == 0).count();
    let sz1 = p.assignment().iter().filter(|&&x| x == 1).count();
    assert_eq!(sz0 + sz1, 20, "all vertices assigned");
    assert!(
        (8..=12).contains(&sz0),
        "part 0 size {sz0} must be within +-2 of target 10"
    );
    assert!(
        (8..=12).contains(&sz1),
        "part 1 size {sz1} must be within +-2 of target 10"
    );
}

#[test]
fn manual_imbalanced_partition_exceeds_half_percent() {
    let g = path_graph(10);
    // Assign 9 to part 0, 1 to part 1: deviation = |9/10 - 0.5| = 0.4 -> 40%
    let mut assignment = vec![0u32; 10];
    assignment[9] = 1;
    let p = Partition::new(assignment, 2).expect("partition is valid");
    let dev = max_balance_deviation(&p, &g);
    assert!(
        dev > 0.005,
        "9+1 split deviation {:.4} must exceed 0.5% threshold",
        dev
    );
}

#[test]
fn trivial_k1_partition_has_zero_deviation() {
    let g = path_graph(8);
    let p = Partition::new(vec![0u32; 8], 1).expect("partition is valid");
    let dev = max_balance_deviation(&p, &g);
    assert!(
        (dev - 0.0).abs() < 1e-12,
        "k=1 partition must have zero deviation, got {dev}"
    );
}

// ─── MetisParams / pure math (3 tests) ────────────────────────────────────

#[test]
fn metis_params_coarsening_method_default_is_shem() {
    let p = MetisParams::default();
    assert!(
        matches!(p.coarsening_method(), CoarseningMethod::Shem),
        "default coarsening method must be Shem"
    );
}

#[test]
fn metis_params_objective_default_is_cut() {
    let p = MetisParams::default();
    assert!(
        matches!(p.objective(), ObjectiveType::Cut),
        "default objective must be Cut"
    );
}

#[test]
fn metis_params_ufactor_5_and_niter_10() {
    let p = MetisParams::default();
    assert_eq!(p.ufactor(), 5, "default ufactor must be 5");
    assert_eq!(p.niter(), 10, "default niter must be 10");
}
