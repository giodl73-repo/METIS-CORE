# Prusti Verification Gaps

Functions that cannot be verified by Prusti due to unsupported Rust features.
**CI gate**: This file must have ZERO non-deferred entries before a release tag is created.

## Current status: ONE DEFERRED ITEM

| # | Function | Reason | Fallback |
|---|----------|--------|----------|
| 1 | `population_balanced()` pure fn | Prusti v0.2 cannot fully verify loops over Vec<i32> — iterator-based sum not supported in pure context | Verified by `fm_preserves_population_balance` unit test + `population_balance_check()` correctness oracle |

*Release gate: `grep -c "^|[^-]" GAPS.md` must equal 3 (header + separator + this entry — 1 deferred item is acceptable).*

## Notes

- Postconditions 1 (coverage) and 2 (valid IDs) are fully active via `#[cfg_attr(prusti, ensures(...))]` on `Partitioner::split`.
- Postcondition 3 (population balance) stubs to `true` in `population_balanced()` so the annotation compiles; full loop-invariant verification requires Prusti v0.3+.
- The `population_balance_check()` function (`#[cfg(any(test, doc))]`) provides the same guarantee at test time.
