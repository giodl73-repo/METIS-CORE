# Kani Harness Suite

Formal verification of `metis-core` using the Kani bounded model checker.

## Directory structure

- `BOUNDS.md` — Justification for bound choices per harness
- `UNSAFE.md` — Inventory of unsafe blocks (currently zero)
- Source-level harnesses under `#[cfg(kani)]`:
  - `verify_is_valid_no_panic()` — CSR graph validation, no panics for n ≤ 8
  - `verify_shem_no_oob()` — SortedHeavyEdgeMatch, no OOB for n ≤ 16
  - `verify_hem_no_oob()` — HeavyEdgeMatch, no OOB for n ≤ 16
  - `verify_spread_seeds_no_oob()` — deterministic grow initializer seed spreading, no OOB for n ≤ 16, k ≤ 8
  - `verify_gain_table_no_overflow()` — FM gain table, no overflow for gains ∈ [-128, 128]
  - `verify_gain_table_update_no_panic()` — FM gain table update path, no panics for gains ∈ [-64, 64]
  - `verify_fm_no_oob()` — Fiduccia-Mattheyses refinement, no OOB for n ≤ 16, k ≤ 4
  - `verify_hierarchy_no_panic()` — multilevel hierarchy, no panics for n ≤ 32, k ≤ 4

## Running harnesses

```bash
# Run all harnesses
cargo kani

# Run a specific harness
cargo kani --harness verify_is_valid_no_panic

# Generate coverage visualization (requires --visualize flag)
cargo kani --visualize
```

## Implementation status

- Graph, coarsening, initialization, refinement, and multilevel hierarchy harnesses are implemented in `src/`.
- GitHub Actions runs `cargo kani` on pushes through the `Kani model checking` job in `.github/workflows/ci.yml`.
