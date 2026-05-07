//! L0 property-based tests — CsrGraph invariants through coarsening.

use proptest::prelude::*;
use metis_core::graph::CsrGraph;
use metis_core::coarsen::Coarsener;
use metis_core::coarsen::shem::SortedHeavyEdgeMatch;

fn arb_grid(max_rows: usize, max_cols: usize) -> impl Strategy<Value = CsrGraph> {
    (2usize..=max_rows, 2usize..=max_cols).prop_map(|(rows, cols)| {
        let n = rows * cols;
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let mut nbrs = Vec::new();
                if r > 0 { nbrs.push((r-1)*cols+c); }
                if r < rows-1 { nbrs.push((r+1)*cols+c); }
                if c > 0 { nbrs.push(r*cols+(c-1)); }
                if c < cols-1 { nbrs.push(r*cols+(c+1)); }
                for &u in &nbrs { adjncy.push(u as u32); }
                xadj.push(adjncy.len() as u32);
            }
        }
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
    })
}

fn arb_path(max_n: usize) -> impl Strategy<Value = CsrGraph> {
    (2usize..=max_n).prop_map(|n| {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 { adjncy.push((i-1) as u32); }
            if i < n-1 { adjncy.push((i+1) as u32); }
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
    })
}

proptest! {
    #[test]
    fn coarsen_preserves_validity(g in arb_path(32)) {
        let (coarsened, cmap) = SortedHeavyEdgeMatch.coarsen(&g);
        prop_assert!(coarsened.is_valid(),
            "coarsened graph must satisfy is_valid()");
        prop_assert_eq!(cmap.cmap.len(), g.n(),
            "cmap length must equal fine graph vertex count");
        prop_assert!(coarsened.n() < g.n(),
            "coarsened graph must be strictly smaller");
    }

    #[test]
    fn coarsen_cmap_targets_in_range(g in arb_path(32)) {
        let (coarsened, cmap) = SortedHeavyEdgeMatch.coarsen(&g);
        for &t in &cmap.cmap {
            prop_assert!((t as usize) < coarsened.n(),
                "cmap target {} out of range (coarsened n={})", t, coarsened.n());
        }
    }

    #[test]
    fn coarsen_grid_preserves_validity(g in arb_grid(4, 4)) {
        let (coarsened, cmap) = SortedHeavyEdgeMatch.coarsen(&g);
        prop_assert!(coarsened.is_valid(),
            "coarsened grid must satisfy is_valid()");
        prop_assert_eq!(cmap.cmap.len(), g.n());
        prop_assert!(coarsened.n() < g.n());
    }

    #[test]
    fn coarsen_grid_cmap_surjective(g in arb_grid(4, 4)) {
        let (coarsened, cmap) = SortedHeavyEdgeMatch.coarsen(&g);
        // Every coarse vertex must be the target of at least one fine vertex (surjectivity)
        let mut covered = vec![false; coarsened.n()];
        for &t in &cmap.cmap { covered[t as usize] = true; }
        prop_assert!(covered.iter().all(|&c| c),
            "CoarseMap must be surjective — every coarse vertex must be targeted");
    }
}
