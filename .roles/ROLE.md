# METIS-CORE Role Index

METIS-CORE is a pure Rust multilevel graph partitioning engine. Use these roles
when changing algorithm behavior, public API contracts, verification harnesses,
benchmarks, or METIS parity expectations.

## Parliament

| File | Role | Primary tension |
|---|---|---|
| `parliament/partition-correctness-steward.md` | Partition Correctness Steward | Cut quality and balance vs. invalid or disconnected assignments |
| `parliament/api-contract-auditor.md` | API Contract Auditor | Safe validated public API vs. convenient internal construction |
| `parliament/parity-performance-reviewer.md` | Parity Performance Reviewer | METIS-quality behavior vs. pure-Rust portability and speed |

## Review order

1. Use Partition Correctness Steward for coarsening, initialization, refinement, repair, and objective changes.
2. Use API Contract Auditor for public types, constructors, errors, and advanced extension traits.
3. Use Parity Performance Reviewer for parity harnesses, benchmark baselines, and production-readiness claims.
