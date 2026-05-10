use criterion::{criterion_group, criterion_main, Criterion};
use metis_core::api::{MetisParams, MetisPartitioner, Partitioner};
use metis_core::coarsen::shem::SortedHeavyEdgeMatchWithParams;
use metis_core::graph::{CsrGraph, Partition};
use metis_core::init::{grow::GrowBisect, InitialPartitioner};
use metis_core::multilevel::hierarchy::CoarseningHierarchy;
use metis_core::refine::{fm::FiducciaMattheyses, Refiner};

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
    let g = grid_graph(96, 95); // 9120 vertices ≈ CA 9129 tracts
    let coarsener = SortedHeavyEdgeMatchWithParams {
        coarsen_to: 20,
        k: 53,
    };
    c.bench_function("ca_coarsen_only_n9120", |b| {
        b.iter(|| CoarseningHierarchy::build(&g, &coarsener).unwrap());
    });
}

// ── CA initial partition only — isolates seeding/grow on the coarsest graph ─

fn bench_ca_init_only(c: &mut Criterion) {
    let g = grid_graph(96, 95);
    let coarsener = SortedHeavyEdgeMatchWithParams {
        coarsen_to: 20,
        k: 53,
    };
    let hierarchy = CoarseningHierarchy::build(&g, &coarsener).unwrap();
    let init = GrowBisect;

    c.bench_function("ca_init_only_k53_n9120", |b| {
        b.iter(|| init.partition(hierarchy.coarsest(), 53, 42));
    });
}

// ── CA projection only — isolates cmap projection through the hierarchy ─────

fn bench_ca_projection_only(c: &mut Criterion) {
    let g = grid_graph(96, 95);
    let coarsener = SortedHeavyEdgeMatchWithParams {
        coarsen_to: 20,
        k: 53,
    };
    let hierarchy = CoarseningHierarchy::build(&g, &coarsener).unwrap();
    let init = GrowBisect;
    let coarse = init.partition(hierarchy.coarsest(), 53, 42);

    c.bench_function("ca_projection_only_k53_n9120", |b| {
        b.iter(|| {
            let mut assignment = coarse.assignment().to_vec();
            for lev in (0..hierarchy.depth()).rev() {
                assignment = hierarchy.project_up(lev, &assignment);
            }
            assignment
        });
    });
}

// ── CA refine+project only — isolates uncoarsening/refinement after coarsen ─

fn bench_ca_refine_project_only(c: &mut Criterion) {
    let g = grid_graph(96, 95);
    let coarsener = SortedHeavyEdgeMatchWithParams {
        coarsen_to: 20,
        k: 53,
    };
    let hierarchy = CoarseningHierarchy::build(&g, &coarsener).unwrap();
    let init = GrowBisect;
    let coarse = init.partition(hierarchy.coarsest(), 53, 42);
    let refiner = FiducciaMattheyses {
        niter: 10,
        contig_fm: false,
        objective: metis_core::api::ObjectiveType::Cut,
        lp_iter: 10,
        ufactor: 5,
    };

    c.bench_function("ca_refine_project_only_k53_n9120", |b| {
        b.iter(|| refine_and_project(&hierarchy, coarse.clone(), &refiner));
    });
}

// ── CA rebalance only — isolates the final equal-weight balance repair ─────

fn bench_ca_rebalance_only(c: &mut Criterion) {
    let g = grid_graph(96, 95);
    let params = MetisParams::default();
    let assignment = MetisPartitioner::with_params(params, 53)
        .split(&g, 53, Some(42))
        .unwrap()
        .into_assignment();
    let mut imbalanced = assignment;
    for part in imbalanced.iter_mut().take(200) {
        *part = 0;
    }
    let partition = Partition::new(imbalanced, 53).unwrap();

    c.bench_function("ca_rebalance_only_k53_n9120", |b| {
        b.iter(|| {
            let mut trial = partition.clone();
            metis_core::refine::lp::rebalance_to_ufactor(&g, &mut trial, 5);
            trial
        });
    });
}

fn refine_and_project(
    hierarchy: &CoarseningHierarchy,
    initial: Partition,
    refiner: &dyn Refiner,
) -> Partition {
    let depth = hierarchy.depth();
    let mut current = initial;

    for lev in (0..depth).rev() {
        current = refiner.refine(hierarchy.level(lev + 1).expect("level exists"), current);
        current = Partition::new(hierarchy.project_up(lev, current.assignment()), current.k())
            .expect("projected partition remains structurally valid");
    }

    refiner.refine(hierarchy.level(0).expect("level exists"), current)
}

criterion_group!(
    benches,
    bench_vt_bisect,
    bench_pa_kway,
    bench_tx_kway,
    bench_ny_kway,
    bench_ca_kway,
    bench_ca_coarsen_only,
    bench_ca_init_only,
    bench_ca_projection_only,
    bench_ca_refine_project_only,
    bench_ca_rebalance_only,
);
criterion_main!(benches);
