# METIS-CORE PITFALL Index

METIS-CORE uses PITFALL to preserve doctrine for pure-Rust graph partitioning,
validated public APIs, deterministic seeded behavior, parity/performance
evidence, formal-verification boundaries, and downstream ROUTE rehearsal.

| Namespace | Kind | Path | Owner |
|---|---|---|---|
| `metis-core` | `principles` | [metis-core-principles.md](metis-core-principles.md) | METIS-CORE maintainer |
| `metis-core` | `invariants` | [metis-core-invariants.md](metis-core-invariants.md) | METIS-CORE maintainer |
| `metis-core` | `pitfalls` | [metis-core-pitfalls.md](metis-core-pitfalls.md) | METIS-CORE maintainer |

## Integration

- ROLES: `.roles/ROLE.md` covers partition correctness, API contract safety,
  and parity/performance review.
- VTRACE: METIS-CORE does not currently carry a repo-local VTRACE matrix;
  PITFALL entries cite compatibility policy, production readiness, verification
  docs, role reviews, and executable validation until a trace slice exists.
- Tests: Rust tests, clippy, docs, package checks, unsafe scans, optional METIS
  parity, Kani/Prusti notes, and ROUTE rehearsal are the evidence hooks for
  PITFALL entries.
