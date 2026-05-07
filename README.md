# METIS-CORE

A pure Rust implementation of multilevel graph partitioning.

Multilevel graph partitioning — sometimes called the METIS algorithm after Karypis and Kumar's 1995/1998 papers — is the standard approach for partitioning large irregular graphs in scientific computing, mesh decomposition, and combinatorial optimization. This crate implements the algorithm from scratch in safe Rust with no C compiler required and no dependency on any external METIS library.

---

## What it does

Takes a graph in compressed-sparse-row (CSR) format and partitions its vertices into *k* balanced parts while minimizing the edge cut between parts. Two entry points:

- **`part_recursive`** — multilevel recursive bisection
- **`part_kway`** — direct multilevel k-way partitioning

Both run the same unified multilevel pipeline.

---

## The algorithm

Three phases:

1. **Coarsening** — graph is shrunk by successive heavy-edge matching (HEM, SHEM, or TwoHop) until it is small enough to partition directly.
2. **Initial partitioning** — small coarsened graph is bisected using greedy grow or random partitioning.
3. **Uncoarsening + refinement** — partition is projected back through the hierarchy and refined at each level using FM (Fiduccia-Mattheyses) boundary refinement with contiguity repair.

Optional extensions:

- **Multi-cut (`ncuts`)** — run multiple independent trials, return the best cut.
- **Contiguity enforcement** — repair guarantees each output part is connected.
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
| **No C dependency** | Pure Rust; no `cc`, no external library, no `bindgen` |
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

## License

[MIT](LICENSE) — © 2026 Gio Della-Libera.
