# METIS-CORE

A pure Rust implementation of the [METIS](http://glaros.dtc.umn.edu/gkhome/metis/metis/overview) multilevel graph partitioning algorithm.

METIS is George Karypis's multilevel k-way and recursive bisection algorithm — the standard tool for partitioning large irregular graphs used in scientific computing, mesh decomposition, and combinatorial optimization. This crate reimplements the same algorithm from scratch in safe Rust with no C compiler required.

---

## What it does

Takes a graph in compressed-sparse-row (CSR) format and partitions its vertices into *k* balanced parts while minimizing the edge cut between parts. Mirrors the two public entry points of the original C library:

- **`part_recursive`** — multilevel recursive bisection (`METIS_PartGraphRecursive`)
- **`part_kway`** — direct multilevel k-way partitioning (`METIS_PartGraphKway`)

Both entry points run the same unified multilevel pipeline.

---

## The algorithm

Three phases, matching the C library:

1. **Coarsening** — graph is shrunk by successive heavy-edge matching (HEM, SHEM, or TwoHop) until it is small enough to partition directly.
2. **Initial partitioning** — small coarsened graph is bisected using greedy grow or random partitioning.
3. **Uncoarsening + refinement** — partition is projected back through the hierarchy and refined at each level using FM (Fiduccia-Mattheyses) boundary refinement with contiguity repair.

Optional extensions beyond the standard algorithm:

- **Multi-cut (`ncuts`)** — run multiple independent trials, return the best cut.
- **Contiguity enforcement** — partition repair guarantees each output part is connected.
- **Minimum-connectivity refinement** — post-processing pass minimizes inter-part adjacency counts.

---

## Usage

```toml
[dependencies]
metis-core = { git = "https://github.com/giodl73-repo/METIS-CORE.git" }
```

```rust
use metis_core::{part_recursive, api::MetisParams};

let xadj   = vec![0u32, 2, 4, 6, 8];    // 4-vertex cycle
let adjncy = vec![1, 3, 0, 2, 1, 3, 0, 2];
let assignment = part_recursive(&xadj, &adjncy, &[], &[], 2, MetisParams::default())?;
// assignment: each vertex labeled 0 or 1
```

For full control use `MetisPartitioner` directly:

```rust
use metis_core::{
    graph::CsrGraph,
    api::{MetisPartitioner, MetisParams, Partitioner, CoarseningMethod},
};

let g = CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1; n], adjwgt: None };
let params = MetisParams { coarsen_method: CoarseningMethod::Shem, ncuts: 3, ..Default::default() };
let partition = MetisPartitioner::with_params(params, k).split(&g, k, Some(seed))?;
```

---

## Design

| Property | Detail |
|----------|--------|
| **No C dependency** | Pure Rust; no `cc`, no `libmetis`, no `bindgen` |
| **Deterministic** | Seeded RNG (`rand_pcg`) — same seed, same partition |
| **Verified** | Kani model-checker harnesses in `verify/kani/`; Prusti postcondition stubs in `verify/prusti/` |
| **Tested** | 97.1% line coverage; proptest invariant suite; golden-file regression (Vermont 2020 census) |
| **No unsafe** | All partitioning code is safe Rust |

---

## Module layout

```
src/
  api.rs           MetisPartitioner, MetisParams, Partitioner trait
  graph/mod.rs     CsrGraph, Partition, CSR helpers, contiguity check/repair
  coarsen/         HEM, SHEM, TwoHop coarsening
  init/            Greedy-grow and random initial bisection
  refine/          FM boundary refinement, gain tables, minconn, LP balance
  multilevel/      Coarsening hierarchy + unified pipeline
  error.rs         PartitionError enum
```

---

## Running tests

```bash
cargo test                    # unit + integration tests
cargo test --test graph_ops   # CSR, contiguity, coarsening, balance (30 tests)
cargo test --test contracts   # algorithm contracts
cargo bench                   # criterion benchmarks
```

---

## Relationship to the C library

This is an independent Rust reimplementation of the METIS algorithm, not a binding to the C library. The public API mirrors `METIS_PartGraphRecursive` / `METIS_PartGraphKway` for drop-in compatibility where only the partition vector is needed. For the original C implementation see [KarypisLab/METIS](https://github.com/KarypisLab/METIS).

---

## License

[MIT](LICENSE) — © 2026 Gio Della-Libera.
