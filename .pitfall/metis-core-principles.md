# METIS-CORE Principles

## METIS-P-01: Correct Partitions Before Cut Quality

**Decision rule:** Invalid assignments, broken balance semantics,
out-of-range part IDs, disconnected guarantees, and malformed CSR behavior block
before cut-quality or speed improvements.

**Rationale:** A partitioner can be fast or METIS-like only after it preserves
the mathematical contract downstream callers rely on.

**Test:** Contract, graph-ops, proof-surface, public-API, and workspace tests
validate structural invariants before quality envelopes.

**Evidence:** `.roles/parliament/partition-correctness-steward.md`,
`docs/PRODUCTION_PLAN.md`, and `cargo test --workspace --all-targets`.

## METIS-P-02: Public API Construction Is Validated

**Decision rule:** Public graph, partition, parameter, coarsening, refinement,
repair, and subgraph operations must use validated constructors, builders,
accessors, and `Result`-returning APIs.

**Rationale:** Downstream crates should not be able to construct impossible
graph or partition states through public fields or leaked internals.

**Test:** Public API tests and release-hardening docs cover validated
constructors, private internals, typed errors, and advanced extension traits.

**Evidence:** `.roles/parliament/api-contract-auditor.md`,
`docs/PRODUCTION_PLAN.md`, and `cargo test --test public_api`.

## METIS-P-03: Parity Is Envelope-Based

**Decision rule:** METIS parity compares structural invariants, balance, cut
quality, and performance envelopes, not exact vertex labels.

**Rationale:** Label identity is not stable across implementations; useful
parity is about valid, balanced, useful partitions with visible regression
signals.

**Test:** Optional `gpmetis` parity tests and benchmark smokes preserve quality
and speed evidence without importing a C dependency.

**Evidence:** `.roles/parliament/parity-performance-reviewer.md`,
`README.md`, `docs/PRODUCTION_PLAN.md`, and
`cargo test --workspace --all-targets`.

## METIS-P-04: Verification Claims Stay Bounded

**Decision rule:** Kani and Prusti evidence must be described by actual harness
coverage, bound choices, unsafe inventory, and deferred proof gaps.

**Rationale:** Formal-verification language is valuable only when its limits
remain explicit.

**Test:** Verification docs list Kani harnesses, zero unsafe blocks, Prusti
postconditions, and the one deferred balance proof with runtime fallback.

**Evidence:** `verify/kani/README.md`, `verify/kani/UNSAFE.md`,
`verify/prusti/README.md`, and `verify/prusti/GAPS.md`.

## METIS-P-05: ROUTE Rehearsal Gates Foundation Changes

**Decision rule:** Public API, default, validation, error, deterministic-output,
or quality-envelope changes are not ready until METIS-CORE tests and the ROUTE
downstream rehearsal pass or are explicitly scoped out.

**Rationale:** ROUTE is the first consumer of the public graph, partitioner,
trait, result, and error surfaces.

**Test:** Compatibility policy names focused ROUTE tests and expected failure
signals.

**Evidence:** `docs/compatibility.md`.
