# Unsafe Code Inventory

Every actual `unsafe` construct in `src/` is listed here with justification and Kani coverage.
CI scans for `unsafe {`, `unsafe fn`, `unsafe impl`, `unsafe trait`, and `unsafe extern`.

**Current status: Zero unsafe blocks.** All array indexing and arithmetic use safe Rust bounds checks.

| # | Location | Operation | Justifying invariant | Kani harness |
|---|----------|-----------|---------------------|--------------|
| — | (none) | — | — | — |

**CI gate**: `.github/workflows/ci.yml` rejects actual unsafe constructs while allowing the crate-level `#![forbid(unsafe_code)]` policy attribute.
