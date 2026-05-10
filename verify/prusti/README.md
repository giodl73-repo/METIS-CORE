# Prusti Verification

Prusti annotations live in `src/api.rs` behind `#[cfg(prusti)]` and document the intended postconditions for `Partitioner::split`.

Run locally on a supported Linux environment:

```bash
cargo prusti
```

GitHub Actions runs this as a best-effort job because Prusti availability depends on the external release bundle. See `GAPS.md` for the one deferred balance proof and its runtime test fallback.
