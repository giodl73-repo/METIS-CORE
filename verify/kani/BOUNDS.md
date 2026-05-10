# Kani Verification Bounds

This file justifies the bound choices for each Kani harness.
A bound covers all code paths when increasing it further produces no new LLVM bitcode coverage.

| Harness | Bound | Coverage justification |
|---------|-------|----------------------|
| `verify_is_valid_no_panic` | n ≤ 8 | All branches in `is_valid()` covered: xadj check, self-loop, OOB, vwgt, adjwgt, BFS. n=3 covers all; n=8 adds confidence. |
| `verify_shem_no_oob` | n ≤ 16 | Bucket sort with star topology (1 center, n-1 leaves) requires n=6 to exercise all paths. 16 adds margin. |
| `verify_hem_no_oob` | n ≤ 16 | Same reasoning as SHEM. |
| `verify_spread_seeds_no_oob` | n ≤ 16, k ≤ 8 | Exercises BFS-distance seed spreading, uniqueness tracking, and bounded path endpoints without entering high-k randomized fallback. |
| `verify_gain_table_no_overflow` | gains ∈ [-128, 128] | Exercises full bucket range, top_bucket scan, swap-with-last dedup. |
| `verify_gain_table_update_no_panic` | gains ∈ [-64, 64], n ≤ 8 | Exercises remove + insert update behavior after an arbitrary valid insertion. |
| `verify_fm_no_oob` | n ≤ 16, k ≤ 4 | FM inner loop branches covered at n=4; 16 adds margin for gain updates. |
| `verify_hierarchy_no_panic` | n ≤ 32, k ≤ 4 | Covers repeated hierarchy construction and valid coarsening maps on bounded path graphs. |

All bounds verified by inspecting LLVM bitcode coverage output from `cargo kani --visualize`.

## Windows / CI Status

**Kani 0.55+ is not available as a native Windows binary.** Compilation fails on Windows 11 with internal rustc errors in the kani crate itself (not this project's code). The harnesses are structurally correct and compile under `#[cfg(kani)]`, but verification must run on Linux.

**Verification runs on Linux via GitHub Actions.** See the `Kani model checking` job in `.github/workflows/ci.yml`.

**To run locally on Windows:** Use WSL2 with Ubuntu 22.04+:
```powershell
wsl -- cargo kani --harness verify_is_valid_no_panic
```

**Prusti status:** Also not available on Windows (requires Java + SMT solver stack). Runs as a best-effort Linux CI job.

All source-level Kani harnesses are wired into CI.
