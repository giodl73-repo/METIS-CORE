# METIS-CORE

A pure Rust implementation of multilevel graph partitioning.

**Review roles:** This repo uses
[ROLES](https://github.com/giodl73-repo/ROLES), the `.roles` convention for
repository-local review panels.

Multilevel graph partitioning — sometimes called the METIS algorithm after Karypis and Kumar's 1995/1998 papers — is the standard approach for partitioning large irregular graphs in scientific computing, mesh decomposition, and combinatorial optimization. This crate implements the algorithm from scratch in safe Rust with no C compiler required and no dependency on any external METIS library.

---

## What It Does

Takes a graph in compressed-sparse-row (CSR) format and partitions its vertices into *k* balanced parts while minimizing the edge cut between parts. Two entry points:

- **`part_recursive`** — multilevel recursive bisection
- **`part_kway`** — direct multilevel k-way partitioning
- **`part_recursive_result` / `part_kway_result`** — assignment plus objective metadata

`part_recursive` uses recursive bisection; `part_kway` uses the direct k-way
pipeline.

---

The simple helpers accept METIS-style CSR slices and return a part assignment
vector. Use the `_result` helpers, `CsrGraph`, and `MetisPartitioner` when a
caller needs a validated graph object, reusable partitioner, or objective
metadata.

---

## Algorithm

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

## Install

```toml
[dependencies]
metis-core = { git = "https://github.com/giodl73-repo/METIS-CORE.git" }
```

This crate is not published to crates.io yet; repository metadata intentionally
blocks accidental publishing until release policy is decided.

---

## Usage

METIS-style entry point:

```rust
use metis_core::{part_recursive, MetisParams};

fn main() -> Result<(), metis_core::PartitionError> {
    let xadj = vec![0u32, 2, 4, 6, 8]; // 4-vertex cycle
    let adjncy = vec![1, 3, 0, 2, 1, 3, 0, 2];

    let assignment = part_recursive(&xadj, &adjncy, &[], &[], 2, MetisParams::recursive())?;
    assert_eq!(assignment.len(), 4);
    assert!(assignment.iter().all(|&part| part < 2));

    Ok(())
}
```

CSR input is strict by default: `xadj[n]` must equal `adjncy.len()`, each
adjacency entry must have its reciprocal entry, reciprocal edge weights must
match, weights must be positive, and the graph must be connected. Empty weight
slices mean unit weights. Use `CsrGraph::from_csr_strict` when you want this
proof-oriented contract to be explicit at the call site; `CsrGraph::from_csr`
and `CsrGraph::from_csr_metis` currently enforce the same contract.

Reusable partitioner with validated graph and result objects:

```rust
use metis_core::{
    CoarseningMethod, CsrGraph, MetisParams, MetisPartitioner, Partitioner,
};

fn main() -> Result<(), metis_core::PartitionError> {
    let xadj = vec![0u32, 2, 4, 6, 8];
    let adjncy = vec![1, 3, 0, 2, 1, 3, 0, 2];
    let graph = CsrGraph::from_csr(&xadj, &adjncy, &[], &[])?;

    let k = 2;
    let params = MetisParams::kway()
        .with_coarsening_method(CoarseningMethod::Shem)
        .with_ncuts(3)
        .with_seed(7);
    params.validate_for_k(k)?;

    let partition = MetisPartitioner::from_params(params).split(&graph, k, None)?;
    partition.validate_for_graph(&graph)?;

    assert_eq!(partition.assignment().len(), graph.n());
    assert_eq!(partition.k(), k);

    Ok(())
}
```

Unequal target weights are supported for direct k-way partitioning:

```rust
use metis_core::{CsrGraph, MetisParams, MetisPartitioner, Partitioner};

fn main() -> Result<(), metis_core::PartitionError> {
    let xadj = vec![0u32, 1, 3, 5, 6];
    let adjncy = vec![1, 0, 2, 1, 3, 2];
    let graph = CsrGraph::from_csr(&xadj, &adjncy, &[], &[])?;

    let params = MetisParams::kway().with_target_weights(2, [0.25, 0.75])?;
    let partition = MetisPartitioner::from_params(params).split(&graph, 2, Some(11))?;
    partition.validate_for_graph(&graph)?;

    Ok(())
}
```

Advanced components are available for experiments and proofs. These extension
traits are fallible, so custom code can report invalid inputs or internal
contract failures without panicking:

```rust
use metis_core::advanced::{
    Coarsener, FiducciaMattheyses, GrowBisect, InitialPartitioner, Refiner,
    SortedHeavyEdgeMatchWithParams,
};
use metis_core::{CsrGraph, ObjectiveType};

fn main() -> Result<(), metis_core::PartitionError> {
    let graph = CsrGraph::from_csr(
        &[0, 2, 4, 6, 8],
        &[1, 3, 0, 2, 1, 3, 0, 2],
        &[],
        &[],
    )?;

    let coarsener = SortedHeavyEdgeMatchWithParams::new(20, 2);
    let (coarse, _map) = coarsener.coarsen(&graph)?;

    let init = GrowBisect;
    let initial = init.partition(&coarse, 2, 7)?;

    let refiner = FiducciaMattheyses::new(10, false, ObjectiveType::Cut, 10, 5);
    let refined = refiner.refine(&coarse, initial)?;
    refined.validate_for_graph(&coarse)?;

    Ok(())
}
```

---

## Design

| Property | Detail |
|----------|--------|
| **No C dependency** | Pure Rust; no `cc`, no external library, no `bindgen` |
| **Deterministic** | Seeded RNG (`rand_pcg`) — same seed and parameters, same partition |
| **Thread safe** | Public partitioners and algorithm traits are `Send + Sync`; no global RNG or mutable global state |
| **Validated API** | Public graph, partition, coarsening, initialization, refinement, repair, and subgraph operations return `Result` |
| **Verified** | Kani model-checker harnesses in `verify/kani/`; Prusti postcondition stubs in `verify/prusti/` |
| **Tested** | Unit, integration, proptest invariant, graph-file, and benchmark smoke suites |
| **No unsafe** | All partitioning code is safe Rust |

---

## Public API

The stable surface is exported from the crate root:

- `part_recursive`, `part_kway`, `part_recursive_result`, `part_kway_result`
- `MetisParams`, `MetisPartitioner`, `Partitioner`, `PartitionResult`
- `CsrGraph`, `Partition`, `CoarseMap`, `PartitionError`
- `check_contiguity`, `repair_contiguity`, `extract_subgraph`
- `CoarseningMethod`, `ObjectiveType`

Lower-level algorithm components for experiments and proofs live under
`metis_core::advanced`, including coarseners, initial partitioners, refiners,
and `CoarseningHierarchy`. Source modules are private so the implementation can
evolve without exposing the internal file layout as API.

Public construction is intentionally validated:

- Use `CsrGraph::from_csr_strict`, `CsrGraph::from_csr_metis`,
  `CsrGraph::from_csr`, or `CsrGraph::new` instead of struct literals.
- Use `Partition::new`, `partition.assignment()`, `partition.k()`, and
  `partition.into_assignment()` instead of direct field access.
- Use `MetisParams` builder methods and `validate_for_k` instead of struct
  literals.
- Use `MetisPartitioner::from_params(params)` for reusable partitioners; the
  requested part count belongs to each `split` call.
- Implement `advanced::Coarsener`, `advanced::InitialPartitioner`, or
  `advanced::Refiner` with `Result` returns so pipeline failures stay explicit.

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
