# METIS-CORE Invariants

## METIS-INV-01: Strict CSR Rejects Malformed Graphs

**Status:** MITIGATED

**Claim:** Public CSR construction rejects missing reciprocal edges,
asymmetric weights, self-loops, disconnected graphs, invalid weights, trailing
adjacency, and out-of-bounds adjacency entries.

**Why it matters:** A pure-Rust partitioner must fail before invalid graph
input reaches coarsening or refinement.

**Enforcement:** Graph, proof-surface, and contract tests cover accepted and
rejected CSR surfaces.

**Evidence:** `cargo test --test proof_surface`,
`cargo test --test graph_ops`, and `cargo test --workspace --all-targets`.

## METIS-INV-02: Public Partitions Validate Against Their Graph

**Status:** MITIGATED

**Claim:** Public partition results have one in-range part per vertex, valid
occupancy, explicit objective metadata when requested, and graph-compatible
validation.

**Why it matters:** Downstream callers need result objects they can inspect and
trust without relying on internal pipeline state.

**Enforcement:** Public API and contract tests cover partition construction,
result metadata, contiguity checking, repair, and invalid assignments.

**Evidence:** `cargo test --test public_api`, `cargo test --test contracts`,
and `cargo test --workspace --all-targets`.

## METIS-INV-03: Seeded Runs Are Deterministic

**Status:** MITIGATED

**Claim:** The same seed and parameters produce the same partition under the
same algorithm path.

**Why it matters:** Reproducible partition evidence needs deterministic seeded
behavior.

**Enforcement:** Contract, graph-file, and workspace tests cover golden RNG
determinism, spread-seed determinism, and seeded `ncuts` behavior.

**Evidence:** `tests/contracts.rs`, `tests/graph_files.rs`, and
`cargo test --workspace --all-targets`.

## METIS-INV-04: Safe Rust Is Enforced

**Status:** MITIGATED

**Claim:** Partitioning code contains no actual unsafe constructs and the crate
root forbids unsafe code.

**Why it matters:** METIS-CORE's portability claim includes avoiding C and
unsafe Rust implementation dependencies.

**Enforcement:** Unsafe inventory and CI scan policy list zero unsafe blocks;
local scan finds only the documented scan string.

**Evidence:** `verify/kani/UNSAFE.md` and
`rg -n "unsafe \{|unsafe fn|unsafe impl|unsafe trait|unsafe extern" src verify .github Cargo.toml`.

## METIS-INV-05: Package Publication Is Deliberately Blocked

**Status:** MITIGATED

**Claim:** Repository metadata blocks accidental crates.io publishing until the
release policy is intentionally decided.

**Why it matters:** A mature internal foundation can look release-ready before
publication ownership, compatibility, and downstream rehearsals are complete.

**Enforcement:** Cargo metadata and package checks preserve package formation
without permitting accidental publication.

**Evidence:** `README.md`, `docs/PRODUCTION_PLAN.md`, and
`cargo package --allow-dirty --no-verify`.

## METIS-INV-06: Parity, Verification, And ROUTE Boundaries Are Machine-Readable

**Status:** VERIFIED

**Claim:** METIS-CORE keeps quality-parity, formal-verification, and ROUTE
rehearsal boundaries in a machine-readable manifest that is also routed through
roles, README, production, compatibility, and focused pitfall tests.

**Why it matters:** A pure-Rust partitioner can look production-ready when
structural tests pass, verification artifacts exist, or local tests are green,
even though quality envelopes, proof gaps, and downstream rehearsal still
matter.

**Enforcement:** `tests/pitfall_policy.rs` asserts the boundary manifest, role
routing, README claim limits, production-plan parity evidence, Prusti/Kani
limits, and ROUTE compatibility gate.

**Evidence:** `docs/pitfall-boundaries.v1.json`, `.roles/ROLE.md`,
`README.md`, `docs/PRODUCTION_PLAN.md`, `docs/compatibility.md`,
`verify/prusti/GAPS.md`, `verify/kani/BOUNDS.md`, and
`tests/pitfall_policy.rs`.

**Test:** `cargo test --test pitfall_policy`.
