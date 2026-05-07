# Kani Harness Suite

Formal verification of redist-metis using Kani bounded model checker (Kani 0.55+).

## Directory structure

- `BOUNDS.md` — Justification for bound choices per harness
- `UNSAFE.md` — Inventory of unsafe blocks (currently zero)
- Harness files (linked to src/ via #[cfg(kani)]):
  - `verify_is_valid_no_panic()` — CSR graph validation, no panics for n ≤ 8
  - `verify_shem_no_oob()` — SortedHeavyEdgeMatch, no OOB for n ≤ 16
  - `verify_hem_no_oob()` — HeavyEdgeMatch, no OOB for n ≤ 16
  - `verify_gain_table_no_overflow()` — FM gain table, no overflow for gains ∈ [-128, 128]
  - `verify_fm_no_oob()` — Fiduccia-Mattheyses refinement, no OOB for n ≤ 16, k ≤ 4
  - `verify_hierarchy_no_panic()` — Multilevel hierarchy, no panics for ≤ 8 levels

## Running harnesses

```bash
# Run all harnesses (slow, ~35-70 min)
cargo +nightly kani

# Run a specific harness
cargo +nightly kani --harness verify_is_valid_no_panic

# Generate coverage visualization (requires --visualize flag)
cargo +nightly kani --visualize
```

## Implementation status

- Task 17: Graph + coarsen harnesses (pending)
- Task 18: Refine + multilevel harnesses (pending)

See redist-metis-verify.md for full specification.
