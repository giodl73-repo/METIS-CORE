# Unsafe Code Inventory

Every `unsafe` block in `src/` is listed here with justification and Kani coverage.
CI asserts: count of `unsafe` in `src/` == count of entries in this file.

**Current status: Zero unsafe blocks.** All array indexing and arithmetic use safe Rust bounds checks.

| # | Location | Operation | Justifying invariant | Kani harness |
|---|----------|-----------|---------------------|--------------|
| — | (none) | — | — | — |

**CI gate**: `grep -r "unsafe" src/` returns empty.
