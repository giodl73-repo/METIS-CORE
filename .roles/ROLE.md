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

## Productive tensions

| Pulls | Against | Because |
|---|---|---|
| Partition Correctness Steward | API Contract Auditor | Internal states useful to the algorithm may be unsafe or confusing in the public API. |
| API Contract Auditor | Parity Performance Reviewer | Safe validated boundaries can add copying, allocation, or checks on critical paths. |
| Parity Performance Reviewer | Partition Correctness Steward | Faster heuristics can degrade balance, connectivity, or objective quality. |

Invalid assignments, broken invariants, and unsound public behavior block first. After correctness
is established, use the smallest representative parity fixture or benchmark to adjudicate API
clarity versus performance. Do not trade correctness for an unmeasured speed claim; record any
accepted parity or budget compromise in the review evidence.

## Review order

1. Use Partition Correctness Steward for coarsening, initialization, refinement, repair, and objective changes.
2. Use API Contract Auditor for public types, constructors, errors, and advanced extension traits.
3. Use Parity Performance Reviewer for parity harnesses, benchmark baselines, and production-readiness claims.
