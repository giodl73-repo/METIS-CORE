# METIS-CORE Pitfalls

## METIS-PF-01: Structural Pass Becomes Quality Parity

**Status:** OPEN

**Pattern:** A structurally valid partition is promoted as METIS-quality parity
or production performance without cut, balance, and benchmark envelope evidence.

**Actor:** Maintainer, benchmark author, README editor, release-note writer,
ROUTE adopter, or research-paper author.

**Task:** Explain readiness, compare against `gpmetis`, publish benchmark
claims, or justify downstream adoption.

**Surface:** Public README claims, optional `gpmetis` parity, benchmark
baselines, ROUTE adoption, and release notes.

**Likely mistake:** Treat accepted CSR input, valid assignments, deterministic
seeds, or a green structural suite as proof of METIS-quality cut/balance parity
or production performance.

**Consequence:** Downstream users select METIS-CORE for a quality or speed
promise that has not been proven for their graph family or envelope.

**Owner:** METIS-CORE maintainers, with Parity Performance Reviewer approval
before quality or speed claims are upgraded.

**Domain:** Public README claims, optional `gpmetis` parity, benchmark baselines,
ROUTE adoption, and release notes.

**Detection difficulty:** Structural checks are necessary and numerous, so they
can look sufficient unless parity/performance review names the missing quality
claim.

**Structural solution:** Require Parity Performance Reviewer evidence before
upgrading quality or speed claims.

**Evidence:** `.roles/parliament/parity-performance-reviewer.md`,
`docs/PRODUCTION_PLAN.md`, and `tests/metis_parity.rs`.

**Test:** `cargo test --test pitfall_policy`.

## METIS-PF-02: Prusti Gap Becomes Full Formal Verification Claim

**Status:** OPEN

**Pattern:** Kani harnesses, zero unsafe blocks, and Prusti annotations are
described as full formal verification even though one Prusti balance proof is
explicitly deferred and bounded Kani coverage has documented limits.

**Actor:** Maintainer, release reviewer, README editor, verification-doc author,
customer-facing integrator, or research-paper author.

**Task:** Describe verification status, add badges, summarize release evidence,
or compare assurance with native METIS or formally verified alternatives.

**Surface:** Verification docs, release claims, README badges, customer-facing
proof statements, and future paper evidence.

**Likely mistake:** Collapse Kani bounds, zero unsafe blocks, Prusti stubs, and
runtime fallback tests into a broad "fully formally verified" claim.

**Consequence:** Consumers over-trust the assurance story and miss the deferred
balance proof and bounded model-checking limits.

**Owner:** METIS-CORE maintainers, with Partition Correctness Steward and API
Contract Auditor review before verification claims change.

**Domain:** Verification docs, release claims, README badges, customer-facing
proof statements, and future paper evidence.

**Detection difficulty:** The verification surface is real and broad, so it is
easy to omit the remaining proof gap in summary language.

**Structural solution:** Keep `verify/prusti/GAPS.md` and `verify/kani/BOUNDS.md`
cited whenever formal-verification claims are made.

**Evidence:** `verify/prusti/GAPS.md`, `verify/kani/BOUNDS.md`, and
`verify/kani/README.md`.

**Test:** `cargo test --test pitfall_policy`.

## METIS-PF-03: ROUTE Rehearsal Is Skipped

**Status:** OPEN

**Pattern:** A public API, default, validation, error, deterministic-output, or
quality-envelope change is accepted after METIS-CORE-local tests only, without
the required ROUTE downstream rehearsal.

**Actor:** METIS-CORE maintainer, ROUTE adopter, portfolio snapshotter,
release-note writer, compatibility reviewer, or future agent.

**Task:** Change public APIs, defaults, validation behavior, error meanings,
deterministic outputs, or quality envelopes and decide whether the change is
ready to publish or snapshot.

**Surface:** Shared foundation changes, ROUTE service-graph partitioning,
dependency adoption, and portfolio snapshots.

**Likely mistake:** Stop after METIS-CORE-local tests because structural,
public-API, and parity smokes are green.

**Consequence:** ROUTE breaks at compile time or silently changes service-graph
partition behavior after the TRACKER pointer is advanced.

**Owner:** METIS-CORE maintainers, with ROUTE rehearsal evidence before
foundation changes are promoted.

**Domain:** Shared foundation changes, ROUTE service-graph partitioning,
dependency adoption, and portfolio snapshots.

**Detection difficulty:** METIS-CORE has strong local tests and parity smokes,
so downstream breakage can look unlikely until ROUTE compiles and verifies its
service graph fixtures.

**Structural solution:** Treat the ROUTE rehearsal in `docs/compatibility.md`
as a release gate for affected foundation changes.

**Evidence:** `docs/compatibility.md`.

**Test:** `cargo test --test pitfall_policy`.

## METIS-PF-04: Unsafe-Free Portability Regresses

**Status:** MITIGATED

**Pattern:** A performance or parity change introduces unsafe Rust, a C
dependency, bindgen, or external METIS dependency while README still describes a
pure safe-Rust implementation.

**Domain:** Algorithm hot paths, coarsening/refinement internals, release
metadata, and portability claims.

**Detection difficulty:** Performance pressure can make small unsafe or native
shortcuts look local until package/release review catches the dependency drift.

**Structural solution:** Keep crate-level unsafe prohibition, unsafe scan,
package checks, and no-C-dependency review in release hardening.

**Evidence:** `verify/kani/UNSAFE.md`, `README.md`,
`cargo check --locked`, and `cargo package --allow-dirty --no-verify`.

## METIS-PF-05: Public API Reopens Impossible States

**Status:** MITIGATED

**Pattern:** Convenience exposes public fields, internal modules, or infallible
constructors that allow impossible graph, partition, parameter, hierarchy, or
coarse-map states.

**Domain:** Public API, advanced extension traits, downstream embedding, and
pre-1.0 compatibility.

**Detection difficulty:** Internal construction shortcuts are useful for the
algorithm and tests, so they can leak into public API under ergonomics pressure.

**Structural solution:** Preserve validated constructors, read-only accessors,
crate-private internals, and `Result`-returning extension points.

**Evidence:** `.roles/parliament/api-contract-auditor.md`,
`docs/PRODUCTION_PLAN.md`, `cargo test --test public_api`, and
`cargo test --workspace --all-targets`.
