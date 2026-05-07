use criterion::{criterion_group, criterion_main, Criterion};
use metis_core::api::{MetisPartitioner, MetisParams, Partitioner};
use metis_core::graph::CsrGraph;

/// Build a `rows × cols` grid graph (connected, no self-loops).
///
/// Interior vertices have 4 neighbours; edge/corner vertices have fewer.
/// All vertex weights are 1 (uniform population).
fn grid_graph(rows: usize, cols: usize) -> CsrGraph {
    let n = rows * cols;
    let mut xadj = vec![0u32];
    let mut adjncy: Vec<u32> = Vec::new();

    for r in 0..rows {
        for c in 0..cols {
            let mut nbrs: Vec<usize> = Vec::with_capacity(4);
            if r > 0         { nbrs.push((r - 1) * cols + c); }
            if r < rows - 1  { nbrs.push((r + 1) * cols + c); }
            if c > 0         { nbrs.push(r * cols + (c - 1)); }
            if c < cols - 1  { nbrs.push(r * cols + (c + 1)); }
            for &u in &nbrs { adjncy.push(u as u32); }
            xadj.push(adjncy.len() as u32);
        }
    }

    CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
}

// ── VT ── 255 tracts, k=1 (smoke test / bisection baseline) ─────────────────

fn bench_vt_bisect(c: &mut Criterion) {
    let g = grid_graph(15, 17); // 255 vertices ≈ VT 255 tracts
    let params = MetisParams::default();
    c.bench_function("vt_bisect_k1_n255", |b| {
        b.iter(|| {
            MetisPartitioner::with_params(params.clone(), 1)
                .split(&g, 1, Some(42))
                .unwrap()
        });
    });
}

// ── PA ── 5,268 tracts, k=17 ────────────────────────────────────────────────

fn bench_pa_kway(c: &mut Criterion) {
    let g = grid_graph(72, 73); // 5256 vertices ≈ PA 5268 tracts
    let params = MetisParams::default();
    c.bench_function("pa_kway_k17_n5256", |b| {
        b.iter(|| {
            MetisPartitioner::with_params(params.clone(), 17)
                .split(&g, 17, Some(42))
                .unwrap()
        });
    });
}

// ── TX ── 5,265 tracts, k=38 ────────────────────────────────────────────────

fn bench_tx_kway(c: &mut Criterion) {
    let g = grid_graph(72, 73); // 5256 vertices ≈ TX 5265 tracts
    let params = MetisParams::default();
    c.bench_function("tx_kway_k38_n5256", |b| {
        b.iter(|| {
            MetisPartitioner::with_params(params.clone(), 38)
                .split(&g, 38, Some(42))
                .unwrap()
        });
    });
}

// ── NY ── 4,919 tracts, k=26 ────────────────────────────────────────────────

fn bench_ny_kway(c: &mut Criterion) {
    let g = grid_graph(70, 70); // 4900 vertices ≈ NY 4919 tracts
    let params = MetisParams::default();
    c.bench_function("ny_kway_k26_n4900", |b| {
        b.iter(|| {
            MetisPartitioner::with_params(params.clone(), 26)
                .split(&g, 26, Some(42))
                .unwrap()
        });
    });
}

// ── CA ── 9,129 tracts, k=53 — the bottleneck ───────────────────────────────

fn bench_ca_kway(c: &mut Criterion) {
    let g = grid_graph(96, 95); // 9120 vertices ≈ CA 9129 tracts
    let params = MetisParams::default();
    c.bench_function("ca_kway_k53_n9120", |b| {
        b.iter(|| {
            MetisPartitioner::with_params(params.clone(), 53)
                .split(&g, 53, Some(42))
                .unwrap()
        });
    });
}

// ── CA coarsening only — isolates the coarsen phase ─────────────────────────

fn bench_ca_coarsen_only(c: &mut Criterion) {
    use metis_core::coarsen::shem::SortedHeavyEdgeMatchWithParams;
    use metis_core::multilevel::hierarchy::CoarseningHierarchy;

    let g = grid_graph(96, 95); // 9120 vertices ≈ CA 9129 tracts
    let coarsener = SortedHeavyEdgeMatchWithParams { coarsen_to: 20, k: 53 };
    c.bench_function("ca_coarsen_only_n9120", |b| {
        b.iter(|| {
            CoarseningHierarchy::build(&g, &coarsener).unwrap()
        });
    });
}

criterion_group!(
    benches,
    bench_vt_bisect,
    bench_pa_kway,
    bench_tx_kway,
    bench_ny_kway,
    bench_ca_kway,
    bench_ca_coarsen_only,
);
criterion_main!(benches);
