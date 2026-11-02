# Pulse 01 - PITFALL Use-Case Integration

Date: 2026-08-29

## Scope

Second-pass PITFALL integration for METIS-CORE, focused on public-claim,
verification, and downstream-adoption mistakes:

- `METIS-PF-01` - structural pass becomes quality parity
- `METIS-PF-02` - Prusti gap becomes full formal verification claim
- `METIS-PF-03` - ROUTE rehearsal is skipped

## Changes

- Added actor, task, surface, likely mistake, consequence, owner, and retained
  test fields to the three open PITFALL entries.
- Added `tests/pitfall_policy.rs` so claim-boundary, verification-limit, and
  ROUTE rehearsal language stays visible to tests and portfolio metrics.
- Tightened README language so METIS-style compatibility is envelope-based and
  bounded verification is not described as full formal verification.

## Validation

Run before commit:

```powershell
C:\Users\giodl\.cargo\bin\cargo.exe fmt --check
C:\Users\giodl\.cargo\bin\cargo.exe test --test pitfall_policy
C:\Users\giodl\.cargo\bin\cargo.exe test --workspace --all-targets
C:\Users\giodl\.cargo\bin\cargo.exe run --manifest-path C:\src\TRACKER\repos\standards-protocols\pitfall\Cargo.toml -q -p pitfall-cli -- C:\src\TRACKER\repos\tools-infra\metis-core --format json
python C:\src\TRACKER\repos\standards-protocols\pitfall\tools\check_pitfall.py C:\src\TRACKER\repos\tools-infra\metis-core
git diff --check
```
