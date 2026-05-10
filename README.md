# METIS-CORE

A pure Rust implementation of multilevel graph partitioning.

Multilevel graph partitioning — sometimes called the METIS algorithm after Karypis and Kumar's 1995/1998 papers — is the standard approach for partitioning large irregular graphs in scientific computing, mesh decomposition, and combinatorial optimization. This crate implements the algorithm from scratch in safe Rust with no C compiler required and no dependency on any external METIS library.

---

## What it does

Takes a graph in compressed-sparse-row (CSR) format and partitions its vertices into *k* balanced parts while minimizing the edge cut between parts. Two entry points:

- **`part_recursive`** — multilevel recursive bisection
- **`part_kway`** — direct multilevel k-way partitioning

`part_recursive` uses recursive bisection; `part_kway` uses the direct k-way
pipeline.

---

## The algorithm

Three phases:

1. **Coarsening** — graph is shrunk by successive heavy-edge matching (HEM, SHEM, or TwoHop) until it is small enough to partition directly.
2. **Initial partitioning** — small coarsened graph is bisected using greedy grow or random partitioning.
3. **Uncoarsening + refinement** — partition is projected back through the hierarchy and refined at each level using FM (Fiduccia-Mattheyses) boundary refinement.

Optional extensions:

- **Multi-cut (`ncuts`)** — run multiple independent trials, return the best cut.
- **Target weights (`tpwgts`)** — direct k-way partitioning can target unequal
  part weights when the vector has one positive entry per part and sums to
  `1.0`.
- **Contiguity enforcement (`contig_fm`)** — skip FM moves that would disconnect a part and repair projected partitions.
- **Minimum-connectivity refinement** — post-processing pass minimizes inter-part adjacency counts.

Defaults follow METIS k-way behavior: `ncuts = 1`, `niter = 10`, `contig_fm = false`, and `min_conn = false`. `MetisParams::recursive()` switches to recursive-bisection defaults, including `ncuts = 4`. Direct k-way supports `tpwgts`; recursive bisection currently rejects `tpwgts` rather than silently ignoring target weights. Enable `contig_fm` or `min_conn` explicitly when a downstream workflow needs those stricter guarantees.

SHEM also follows the C implementation's important behavior: when edge weights are absent or all equal, it falls back to randomized heavy-edge matching instead of doing a sorted pass over indistinguishable weights.

---

## Usage

```toml
[dependencies]
metis-core = { git = "https://github.com/giodl73-repo/METIS-CORE.git" }
```

```rust
use metis_core::{part_recursive, MetisParams};

let xadj   = vec![0u32, 2, 4, 6, 8];    // 4-vertex cycle
let adjncy = vec![1, 3, 0, 2, 1, 3, 0, 2];
let assignment = part_recursive(&xadj, &adjncy, &[], &[], 2, MetisParams::default())?;
// assignment: each vertex labeled 0 or 1
```

CSR input must be exact and undirected: `xadj[n]` must equal `adjncy.len()`,
each adjacency entry must have its reciprocal entry, weights must be positive,
and the graph must be connected. Empty weight slices mean unit weights.

For full control use `MetisPartitioner` directly:

```rust
use metis_core::{
    CoarseningMethod, CsrGraph, MetisParams, MetisPartitioner, Partitioner,
};

let g = CsrGraph::from_csr(&xadj, &adjncy, &[], &[])?;
let params = MetisParams::kway()
    .with_coarsening_method(CoarseningMethod::Shem)
    .with_ncuts(3);
params.validate_for_k(k)?;
let partition = MetisPartitioner::with_params(params, k).split(&g, k, Some(seed))?;
partition.validate_for_graph(&g)?;
```

---

## Design

| Property | Detail |
|----------|--------|
| **No C dependency** | Pure Rust; no `cc`, no external library, no `bindgen` |
| **Deterministic** | Seeded RNG (`rand_pcg`) — same seed, same partition |
| **Verified** | Kani model-checker harnesses in `verify/kani/`; Prusti postcondition stubs in `verify/prusti/` |
| **Tested** | Unit, integration, proptest invariant, graph-file, and benchmark smoke suites |
| **No unsafe** | All partitioning code is safe Rust |

---

## Public API

The stable surface is exported from the crate root:

- `part_recursive`, `part_kway`
- `MetisParams`, `MetisPartitioner`, `Partitioner`
- `CsrGraph`, `Partition`, `CoarseMap`, `PartitionError`
- `check_contiguity`, `repair_contiguity`, `extract_subgraph`
- `CoarseningMethod`, `ObjectiveType`

Lower-level algorithm components for experiments and proofs live under
`metis_core::advanced`, including coarseners, initial partitioners, refiners,
and `CoarseningHierarchy`. Source modules are private so the implementation can
evolve without exposing the internal file layout as API.

---

## Running tests

```bash
cargo test                    # unit + integration tests
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps
cargo test --test graph_ops   # CSR, contiguity, coarsening, balance (30 tests)
cargo test --test contracts   # algorithm contracts
cargo bench                   # criterion benchmarks, including pipeline phase timings
```

The optional `tests/metis_parity.rs` harness compares against `gpmetis` when
available. Set `METIS_GPMETIS=C:\path\to\gpmetis.exe` to force a specific
binary; the test checks structural invariants plus cut and balance quality
envelopes, not exact vertex labels, because METIS partition labels are
seed-sensitive and implementation-dependent.

Set `METIS_CORE_HEAVY_PARITY=1` to include the larger `copter2.graph` parity
smoke test.

---

## License

[MIT](LICENSE) — © 2026 Gio Della-Libera.
