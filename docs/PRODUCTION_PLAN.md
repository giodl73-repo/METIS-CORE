# Production Readiness Plan

This plan uses graph families as phase themes so each quality step has a clear
reference signal.

## Phase 1: Canonical Graphs

Theme graphs: paths, grids, stars, dumbbells, spiders.

Goal: lock down correctness on graphs where the expected shape is known.

Checks:

- Path bisection cut is optimal or near-optimal.
- Dumbbell cuts favor the bridge.
- Grid partitions are balanced and compact.
- Spider/star graphs expose contiguity and hub behavior.
- Fixed Rust seeds are deterministic.
- Shared partitioners remain thread-safe.

## Phase 2: METIS Fixture Graphs

Theme graphs: `4elt.graph`, `test.mgraph`, then `copter2.graph`.

Goal: compare against real `gpmetis` without expecting identical labels.

Checks:

- Assignment length equals vertex count.
- Part IDs are in range.
- All parts are occupied.
- Imbalance stays within an agreed envelope.
- Edge cut stays within an agreed envelope.
- Runtime is not pathological.

This phase turns the optional `gpmetis` harness into a reference suite.

Status:

- Done: optional `gpmetis` discovery through environment variables, known local
  paths, and `PATH`.
- Done: quality envelopes for synthetic grids, `4elt.graph`, and `test.mgraph`.
- Done: public CSR validation now requires exact undirected adjacency and
  symmetric edge weights, matching METIS graph semantics.
- Done: heavier `copter2.graph` parity smoke test gated behind
  `METIS_CORE_HEAVY_PARITY=1`.
- Done: heavy `copter2.graph` parity run after Phase 4 improvements:
  `gpmetis` cut around `13720`, Rust cut around `14299`, Rust imbalance around
  `1.030`.

## Phase 3: Balance Semantics

Theme graphs: weighted paths, weighted grids, asymmetric target weights,
`test.mgraph`.

Goal: make `ufactor`, `tpwgts`, and multi-constraint behavior consistent.

Deliverables:

- Done: pass `ufactor` through FM/LP instead of hard-coding it.
- Done: define the balance formula for equal and weighted splits.
- Done: add tests for strict and loose `ufactor` behavior.
- Done: LP pre-balance respects `tpwgts` instead of pulling asymmetric
  partition requests back toward equal weights.
- Done: direct k-way `split` now validates and applies `MetisParams::tpwgts`
  instead of silently ignoring it.
- Done: direct target weights and split-weight fractions now reject zero-weight
  parts instead of accepting impossible partition targets.
- Done: equal-weight post-rebalance now chooses the best legal move across all
  overweight parts instead of only the most overweight source part.
- Done: recursive bisection rejects `tpwgts` until asymmetric recursive targets
  are implemented.
- Done: post-rebalance minimizes cut damage by selecting the lowest-penalty
  legal boundary move across all overweight parts.

## Phase 4: Cut Quality

Theme graphs: 24x24 grid, `4elt.graph`, dumbbell variants.

Goal: close the cut-quality gap versus `gpmetis`.

Current signal:

- `gpmetis` grid k=8: cut around `105`.
- Rust grid k=8: cut around `145`.

Likely targets:

- Done: FM k-way gain uses the best single destination part instead of total
  external degree across all parts.
- Done: FM destination choice selects the best balance-legal adjacent target,
  instead of picking the best target first and skipping it if illegal.
- Done: FM gain table updates reuse candidate buffers in hot paths.
- Done: initial grow partitioning uses spread-out graph seeds, capped for high
  `k` to preserve speed.
- Done: spread-seed unit tests cover determinism, uniqueness, and path
  bisection geometry.
- Done: best-of-`ncuts` selection ranks balance-envelope excess before cut and
  scores equal-weight trials after final rebalancing.
- Done: keep the default coarsening threshold at `20 * k`; `10 * k` regressed
  grid, `4elt.graph`, and `test.mgraph` cut quality.
- Done: keep SHEM as default; TwoHop remains available but regressed grid and
  `4elt.graph` cuts in the parity suite.

## Phase 5: Performance

Theme graphs: VT/PA/TX/NY/CA benchmark graphs, plus coarsen-only CA.

Goal: make speed regressions visible.

Checks:

- Done: benchmark before and after each algorithmic change.
- Done: separate CA benchmark coverage for coarsening, init, projection,
  refinement/projection, and final rebalance timing.
- Done: scratch buffers in final rebalance are reused across move selection
  iterations to avoid repeated allocation.
- Done: full k-way benchmarks cover VT/PA/TX/NY/CA-sized synthetic grids.

Current baseline from `cargo bench` after the final API and balance scrub:

- `vt_bisect_k1_n255`: about `80 us`
- `pa_kway_k17_n5256`: about `7.0 ms`
- `tx_kway_k38_n5256`: about `7.6 ms`
- `ny_kway_k26_n4900`: about `6.7 ms`
- `ca_kway_k53_n9120`: about `15.2 ms`
- `ca_coarsen_only_n9120`: about `2.8 ms`
- `ca_init_only_k53_n9120`: about `770 us`
- `ca_projection_only_k53_n9120`: about `6.3 us`
- `ca_refine_project_only_k53_n9120`: about `9.3 ms`
- `ca_rebalance_only_k53_n9120`: about `8.6 ms` on the deliberately
  imbalanced rebalance fixture.

## Phase 6: Release Hardening

Theme graph: full suite.

Goal: make the crate maintainable and releasable.

Deliverables:

- Done: `cargo test --all-targets`
- Done: `cargo test --release`
- Done: `cargo clippy --all-targets -- -D warnings`
- Done: `cargo fmt --check`
- Done: `cargo doc --no-deps`
- Done: unsafe scan
- Done: crate root now forbids unsafe code.
- Done: package check
- Done: README examples
- Done: API decision: this crate is unreleased, so `CsrGraph` construction now
  goes through validated constructors and read-only accessors instead of public
  fields.
- Done: `Partition` is now a result object with read-only accessors and
  `into_assignment()` for callers that need ownership of the assignment vector.
- Done: `MetisParams` now has builder-style helpers and `validate_for_k`, so
  callers can construct and preflight mode-specific options without relying on
  ad hoc struct literals.
- Done: multilevel `Pipeline` typestate internals are no longer publicly
  forgeable; target-weight seeding now goes through a crate-private constructor.
- Done: multilevel `Pipeline` typestate now carries stage-specific state instead
  of an optional partition slot.
- Done: `CoarseningHierarchy` storage is private with read-only accessors, so
  callers cannot construct inconsistent hierarchy levels/maps.
- Done: `MetisParams` fields are crate-private with public builders/getters, so
  external callers configure partitioning through the validated API surface.
- Done: `CoarseMap` construction now validates length, target range, and
  surjectivity while exposing maps through read-only slices.
- Done: raw implementation modules are crate-private; lower-level extension
  points now live under the explicit `advanced` module instead of leaking the
  internal source layout.
- Done: `graph` and `error` source modules are private; their stable types and
  helpers are available from the crate root.
- Done: advanced coarsener/refiner parameter structs use constructors and
  read-only accessors instead of public configuration fields.
- Done: `part_recursive` now forces recursive-bisection semantics and promotes
  default `ncuts` to the METIS pmetis-style value of 4, even when callers set
  unrelated options such as seed.
- Done: add `Partition::validate_for_graph` and make contiguity checks reject
  malformed partitions without panicking.
- Done: CI now gates formatting, debug tests, Linux release tests, clippy, docs,
  unsafe scan, packaging, Kani, and best-effort Prusti.
- Done: CI unsafe scan ignores the crate-level `forbid(unsafe_code)` policy
  attribute and scans for actual unsafe constructs.
