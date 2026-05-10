//! Integration tests using METIS sample graph files from graphs/
//!
//! Graphs directory: C:\src\metis\graphs\
//!   4elt.graph     — 15 606 vertices, 45 878 edges  (2D FEM mesh)
//!   copter2.graph  — 55 476 vertices, 352 238 edges (3D FEM mesh)
//!   mdual.graph    — 258 569 vertices, 513 132 edges (large 3D FEM mesh)
//!   test.mgraph    — 766 vertices, two vertex weights (multi-constraint)
//!
//! Each test skips gracefully when the graphs/ directory is absent (CI without
//! the file corpus, or a developer clone that does not include the data
//! sub-directory).
//!
//! ## What these tests assert
//!
//! For real-world FEM graphs these tests focus on the structural invariants
//! that the implementation guarantees regardless of partition quality:
//!
//!   1. Termination — `split()` returns `Ok(_)` within the test timeout.
//!   2. Full coverage — every vertex is assigned to some part.
//!   3. Valid part IDs — every assignment value is in `0..k`.
//!   4. All parts occupied — no phantom empty part.
//!   5. Contiguity — every part is a connected subgraph.
//!   6. Determinism — identical seed → identical assignment.
//!
//! Balance quality metrics (edge-cut, imbalance ratio) are reported via
//! `eprintln!` for informational purposes but are NOT asserted.  The
//! post-hoc contiguity repair can reassign large fractions of vertices
//! on difficult graphs (copter2, mdual), which disrupts the weight
//! balance that the FM refinement phase achieved; tight balance assertions
//! would be fragile against seeds and implementation changes.
//!
//! ## Supported .graph format variants
//!
//!   fmt=0  (default)  — plain adjacency, no weights
//!   fmt=1             — edge weights interleaved in adjacency list
//!   fmt=10            — one vertex weight per line prefix
//!   fmt=11            — vertex weight + edge weights
//!   fmt=010 ncon=2    — TWO vertex weights per line prefix (e.g. test.mgraph)

use metis_core::api::{MetisParams, MetisPartitioner, Partitioner};
use metis_core::graph::{check_contiguity, CsrGraph};

// ── .graph parser ─────────────────────────────────────────────────────────────

/// Parse a METIS .graph file into a [`CsrGraph`].
///
/// Returns `None` when the file does not exist so callers can skip cleanly.
/// Panics on malformed content (bad header, truncated lines) so that corrupt
/// fixtures surface as test failures rather than silent skips.
///
/// Multi-constraint graphs (`ncon > 1`) are reduced to a single constraint by
/// keeping only the first vertex weight per vertex.  Zero vertex weights are
/// clamped to 1 because [`CsrGraph::is_valid`] requires all weights `> 0`.
fn load_graph(path: &str) -> Option<CsrGraph> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines().filter(|l| !l.starts_with('%'));

    // ── header ───────────────────────────────────────────────────────────────
    let header_line = lines.next().expect("graph file has no header line");
    let header: Vec<u64> = header_line
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    assert!(
        header.len() >= 2,
        "header must have at least n and m: {header_line:?}"
    );

    let n = header[0] as usize;
    let _m = header[1] as usize; // undirected edge count — kept for documentation
    let fmt = header.get(2).copied().unwrap_or(0);
    // ncon: number of vertex weight constraints per vertex; must be >= 1
    let ncon = header.get(3).copied().unwrap_or(1).max(1) as usize;

    // fmt is a decimal code where:
    //   ones digit  == 1 → edge weights interleaved in adjacency list
    //   tens digit  == 1 → ncon vertex weights at the start of each line
    let has_vwgt = (fmt / 10) % 10 == 1;
    let has_ewgt = fmt % 10 == 1;

    // ── per-vertex lines ─────────────────────────────────────────────────────
    let mut xadj: Vec<u32> = Vec::with_capacity(n + 1);
    let mut adjncy: Vec<u32> = Vec::new();
    let mut adjwgt: Vec<i32> = Vec::new();
    let mut vwgt: Vec<i32> = Vec::new();
    xadj.push(0);

    for line in lines.take(n) {
        let tokens: Vec<u64> = line
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        let mut i = 0;

        // Vertex weights: `ncon` values at the start of the line when has_vwgt.
        if has_vwgt {
            for _ in 0..ncon {
                // unwrap_or(&1): treat missing token as weight 1
                let w = *tokens.get(i).unwrap_or(&1) as i32;
                vwgt.push(w);
                i += 1;
            }
        }

        // Neighbor list (1-indexed → 0-indexed), optionally interleaved with edge weights.
        while i < tokens.len() {
            let neighbor = (tokens[i] - 1) as u32;
            adjncy.push(neighbor);
            i += 1;
            if has_ewgt {
                adjwgt.push(*tokens.get(i).unwrap_or(&1) as i32);
                i += 1;
            }
        }
        xadj.push(adjncy.len() as u32);
    }

    // ── build the primary (single-constraint) weight vector ──────────────────
    let vwgt_primary: Vec<i32> = if vwgt.is_empty() {
        // No vertex weights in file → unit weights
        vec![1i32; n]
    } else if ncon > 1 {
        // Multi-constraint: keep only the first weight per vertex.
        // Clamp zeros to 1: is_valid() requires all weights > 0.
        vwgt.chunks(ncon).map(|c| c[0].max(1)).collect()
    } else {
        // Single constraint: clamp zeros to 1.
        vwgt.into_iter().map(|w| w.max(1)).collect()
    };

    let adjwgt = if adjwgt.is_empty() {
        None
    } else {
        Some(adjwgt)
    };

    CsrGraph::new(xadj, adjncy, 1, vwgt_primary, adjwgt).ok()
}

/// Graphs directory path: sibling of the rust/ crate directory.
///
/// On disk layout:
/// ```text
/// C:\src\metis\
///   rust\        ← CARGO_MANIFEST_DIR
///   graphs\      ← returned path
/// ```
fn graphs_dir() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{manifest}/../graphs")
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Count cross-part edges (each undirected edge counted once).
fn edge_cut(g: &CsrGraph, assignment: &[u32]) -> u64 {
    let mut c = 0u64;
    for v in 0..g.n() {
        for j in g.xadj()[v] as usize..g.xadj()[v + 1] as usize {
            let u = g.adjncy()[j] as usize;
            if assignment[v] != assignment[u] {
                c += 1;
            }
        }
    }
    c / 2
}

/// Return the maximum imbalance ratio across all parts:
///   `max_part_size / target_size`
/// where sizes are in vertex count (unit weights).
fn max_imbalance_ratio(assignment: &[u32], k: u32) -> f64 {
    let n = assignment.len();
    let target = n as f64 / k as f64;
    let mut counts = vec![0usize; k as usize];
    for &a in assignment {
        counts[a as usize] += 1;
    }
    let max_count = *counts.iter().max().unwrap_or(&0);
    max_count as f64 / target
}

/// Assert the structural postconditions that every partition must satisfy:
///   - correct length
///   - all part IDs in `0..k`
///   - every part non-empty
///   - every part is a connected subgraph
fn assert_structural_invariants(g: &CsrGraph, assignment: &[u32], k: u32, label: &str) {
    assert_eq!(
        assignment.len(),
        g.n(),
        "{label}: assignment length should equal n"
    );
    assert!(
        assignment.iter().all(|&a| a < k),
        "{label}: all part IDs must be in 0..{k}"
    );
    for part in 0..k {
        assert!(
            assignment.contains(&part),
            "{label}: part {part} is missing (empty part)"
        );
    }
    let p = metis_core::Partition::new(assignment.to_vec(), k).expect("partition is valid");
    assert!(
        check_contiguity(g, &p).is_ok(),
        "{label}: partition is not contiguous"
    );
}

// ── loader unit tests ─────────────────────────────────────────────────────────

#[test]
fn loader_returns_none_for_missing_file() {
    assert!(load_graph("/nonexistent/path/graph.graph").is_none());
}

#[test]
fn loader_4elt_dimensions() {
    let path = format!("{}/4elt.graph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping loader_4elt_dimensions: {path} not found");
            return;
        }
    };
    assert_eq!(g.n(), 15_606, "4elt: vertex count");
    assert_eq!(g.xadj().len(), 15_607, "4elt: xadj length = n+1");
    // Each undirected edge appears twice in the adjacency list.
    let total_adj = g.xadj()[g.n()] as usize;
    assert_eq!(total_adj / 2, 45_878, "4elt: 45 878 undirected edges");
    assert!(g.is_valid(), "4elt: CsrGraph::is_valid()");
}

#[test]
fn loader_mgraph_ncon2_dropped_to_ncon1() {
    // test.mgraph: fmt=010 ncon=2 → loader strips the second constraint,
    // clamps zero weights to 1, and returns an ncon=1 graph.
    let path = format!("{}/test.mgraph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping loader_mgraph_ncon2_dropped_to_ncon1: {path} not found");
            return;
        }
    };
    assert_eq!(g.n(), 766, "test.mgraph: vertex count");
    assert_eq!(g.ncon(), 1, "loader normalises ncon=2 → ncon=1");
    assert_eq!(g.vwgt().len(), 766, "one weight per vertex");
    // After clamping, all weights must be >= 1.
    assert!(
        g.vwgt().iter().all(|&w| w >= 1),
        "weights must be >= 1 after clamping"
    );
    assert!(
        g.is_valid(),
        "test.mgraph: CsrGraph::is_valid() after clamping"
    );
}

// ── 4elt.graph tests (15 606 vertices, 45 878 edges) ─────────────────────────

#[test]
fn test_4elt_k4() {
    let path = format!("{}/4elt.graph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping test_4elt_k4: {path} not found");
            return;
        }
    };
    assert_eq!(g.n(), 15_606);

    let p = MetisPartitioner::with_params(MetisParams::default(), 4)
        .split(&g, 4, Some(0))
        .expect("4elt k=4 should succeed");

    assert_structural_invariants(&g, p.assignment(), 4, "4elt k=4");

    let cut = edge_cut(&g, p.assignment());
    let imbal = max_imbalance_ratio(p.assignment(), 4);
    eprintln!(
        "4elt k=4   n={:6}  cut={cut:7}  max_imbal={imbal:.3}",
        g.n()
    );
}

#[test]
fn test_4elt_k8() {
    let path = format!("{}/4elt.graph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping test_4elt_k8: {path} not found");
            return;
        }
    };

    let p = MetisPartitioner::with_params(MetisParams::default(), 8)
        .split(&g, 8, Some(42))
        .expect("4elt k=8 should succeed");

    assert_structural_invariants(&g, p.assignment(), 8, "4elt k=8");

    let cut = edge_cut(&g, p.assignment());
    let imbal = max_imbalance_ratio(p.assignment(), 8);
    eprintln!(
        "4elt k=8   n={:6}  cut={cut:7}  max_imbal={imbal:.3}",
        g.n()
    );
}

/// Compares contig_fm=true vs contig_fm=false on 4elt k=8 using the same seed.
/// Reports cut and imbalance for both modes; both must pass structural invariants.
#[test]
fn test_4elt_k8_contig_fm_comparison() {
    let path = format!("{}/4elt.graph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping test_4elt_k8_contig_fm_comparison: {path} not found");
            return;
        }
    };

    let p_on = MetisPartitioner::with_params(MetisParams::default().with_contiguity(true), 8)
        .split(&g, 8, Some(42))
        .expect("contig_fm=true k=8 should succeed");
    let p_off = MetisPartitioner::with_params(MetisParams::default().with_contiguity(false), 8)
        .split(&g, 8, Some(42))
        .expect("contig_fm=false k=8 should succeed");

    assert_structural_invariants(&g, p_on.assignment(), 8, "4elt k=8 contig_fm=true");
    assert_structural_invariants(&g, p_off.assignment(), 8, "4elt k=8 contig_fm=false");

    let cut_on = edge_cut(&g, p_on.assignment());
    let cut_off = edge_cut(&g, p_off.assignment());
    let imb_on = max_imbalance_ratio(p_on.assignment(), 8);
    let imb_off = max_imbalance_ratio(p_off.assignment(), 8);
    eprintln!("4elt k=8 contig_fm=true   cut={cut_on:7}  max_imbal={imb_on:.3}");
    eprintln!("4elt k=8 contig_fm=false  cut={cut_off:7}  max_imbal={imb_off:.3}");
}

#[test]
fn test_4elt_k16() {
    let path = format!("{}/4elt.graph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping test_4elt_k16: {path} not found");
            return;
        }
    };

    let p = MetisPartitioner::with_params(MetisParams::default(), 16)
        .split(&g, 16, Some(7))
        .expect("4elt k=16 should succeed");

    assert_structural_invariants(&g, p.assignment(), 16, "4elt k=16");

    let cut = edge_cut(&g, p.assignment());
    let imbal = max_imbalance_ratio(p.assignment(), 16);
    eprintln!(
        "4elt k=16  n={:6}  cut={cut:7}  max_imbal={imbal:.3}",
        g.n()
    );
}

/// ncuts=4 on 4elt k=8 — verifies best-of-4 selects a cut ≤ single-trial cut
/// and reports the improvement for the paper trail.
#[test]
fn test_4elt_k8_ncuts4() {
    let path = format!("{}/4elt.graph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping test_4elt_k8_ncuts4: {path} not found");
            return;
        }
    };

    // single-trial baseline (same seed as test_4elt_k8 for comparability)
    let p1 = MetisPartitioner::with_params(MetisParams::default().with_ncuts(1), 8)
        .split(&g, 8, Some(42))
        .expect("4elt k=8 ncuts=1 should succeed");
    let cut1 = edge_cut(&g, p1.assignment());

    // best-of-4 trials
    let p4 = MetisPartitioner::with_params(MetisParams::default().with_ncuts(4), 8)
        .split(&g, 8, Some(42))
        .expect("4elt k=8 ncuts=4 should succeed");
    assert_structural_invariants(&g, p4.assignment(), 8, "4elt k=8 ncuts=4");
    let cut4 = edge_cut(&g, p4.assignment());
    let imbal4 = max_imbalance_ratio(p4.assignment(), 8);

    eprintln!("4elt k=8 ncuts=1  cut={cut1:7}  (baseline)");
    eprintln!(
        "4elt k=8 ncuts=4  cut={cut4:7}  max_imbal={imbal4:.3}  delta={:+}",
        cut4 as i64 - cut1 as i64
    );
    assert!(
        cut4 <= cut1,
        "ncuts=4 cut ({cut4}) must be ≤ ncuts=1 cut ({cut1})"
    );
}

// ── 4elt determinism: same seed → same partition ──────────────────────────────

#[test]
fn test_4elt_k4_determinism() {
    let path = format!("{}/4elt.graph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping test_4elt_k4_determinism: {path} not found");
            return;
        }
    };

    let p1 = MetisPartitioner::with_params(MetisParams::default(), 4)
        .split(&g, 4, Some(99))
        .unwrap();
    let p2 = MetisPartitioner::with_params(MetisParams::default(), 4)
        .split(&g, 4, Some(99))
        .unwrap();

    assert_eq!(
        p1.assignment(),
        p2.assignment(),
        "4elt k=4: identical seed must produce identical assignment"
    );
}

// ── copter2.graph tests (55 476 vertices, 352 238 edges) ─────────────────────

#[test]
fn test_copter2_k4() {
    let path = format!("{}/copter2.graph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping test_copter2_k4: {path} not found");
            return;
        }
    };
    assert_eq!(g.n(), 55_476);

    let p = MetisPartitioner::with_params(MetisParams::default(), 4)
        .split(&g, 4, Some(0))
        .expect("copter2 k=4 should succeed");

    assert_structural_invariants(&g, p.assignment(), 4, "copter2 k=4");

    let cut = edge_cut(&g, p.assignment());
    let imbal = max_imbalance_ratio(p.assignment(), 4);
    eprintln!(
        "copter2 k=4  n={:6}  cut={cut:7}  max_imbal={imbal:.3}",
        g.n()
    );
}

#[test]
fn test_copter2_k8() {
    let path = format!("{}/copter2.graph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping test_copter2_k8: {path} not found");
            return;
        }
    };

    let p = MetisPartitioner::with_params(MetisParams::default(), 8)
        .split(&g, 8, Some(13))
        .expect("copter2 k=8 should succeed");

    assert_structural_invariants(&g, p.assignment(), 8, "copter2 k=8");

    let cut = edge_cut(&g, p.assignment());
    let imbal = max_imbalance_ratio(p.assignment(), 8);
    eprintln!(
        "copter2 k=8  n={:6}  cut={cut:7}  max_imbal={imbal:.3}",
        g.n()
    );
}

// ── mdual.graph tests (258 569 vertices, 513 132 edges) ──────────────────────

#[test]
fn test_mdual_k4() {
    let path = format!("{}/mdual.graph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping test_mdual_k4: {path} not found");
            return;
        }
    };
    assert_eq!(g.n(), 258_569);

    let p = MetisPartitioner::with_params(MetisParams::default(), 4)
        .split(&g, 4, Some(0))
        .expect("mdual k=4 should succeed");

    assert_structural_invariants(&g, p.assignment(), 4, "mdual k=4");

    let cut = edge_cut(&g, p.assignment());
    let imbal = max_imbalance_ratio(p.assignment(), 4);
    eprintln!(
        "mdual k=4    n={:6}  cut={cut:7}  max_imbal={imbal:.3}",
        g.n()
    );
}

// ── test.mgraph: multi-constraint (fmt=010, ncon=2, 766 vertices) ─────────────

#[test]
fn test_mgraph_k4() {
    let path = format!("{}/test.mgraph", graphs_dir());
    let g = match load_graph(&path) {
        Some(g) => g,
        None => {
            eprintln!("Skipping test_mgraph_k4: {path} not found");
            return;
        }
    };
    assert_eq!(g.n(), 766);

    let p = MetisPartitioner::with_params(MetisParams::default(), 4)
        .split(&g, 4, Some(0))
        .expect("test.mgraph k=4 should succeed");

    assert_structural_invariants(&g, p.assignment(), 4, "test.mgraph k=4");

    let cut = edge_cut(&g, p.assignment());
    let imbal = max_imbalance_ratio(p.assignment(), 4);
    eprintln!(
        "test.mgraph k=4  n={:6}  cut={cut:7}  max_imbal={imbal:.3}",
        g.n()
    );
}
