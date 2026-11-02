# METIS-CORE partition explorer build and delivery

Use Rust 1.95.0 with wasm32-unknown-unknown, clippy, rustfmt and rust-docs;
wasm-bindgen CLI 0.2.127, Node 22, Playwright 1.61.1 and Python 3.11+.
Source commands are in browser/explorer/README.md inside source.zip.
CARGO_TARGET_DIR, when set, must be absolute. The prototype builder freshly
builds native and WASM controls; generated packages are not source files.

The complete source ZIP is the exact Git tree identified by provenance.json,
with every tracked src/test/verify/bench/fixture/spec file, both Cargo locks and
the original Apache-2.0 LICENSE and NOTICE. source-files.json pins every file;
the Git ZIP comment binds the commit. Cargo resolves pinned registry dependencies.
This is not a vendored offline compiler pack and does not ship external gpmetis
executables, DLLs, private QA cases, captures or scratch requests.

From clean source after building:

```text
python browser/explorer/package-site.py
python browser/explorer/verify-site.py target/partition-release/METIS-CORE --commit <HEAD>
python browser/explorer/verify-source.py --bindgen /absolute/path/to/wasm-bindgen
```

Read-only PR qualification precedes default-master-only publication of the tested
artifact. Deployment verifies the exact source and all transferred hashes again.
Fresh live bytes/browser checks are a separate gate. Initial runtime/source/site
budgets are 2/20/25 MiB respectively; optional source does not load at startup.
Original software, browser dependency and Rust library notices remain in force.
Native source build dependencies are pinned in the full locks under upstream terms.

Source replay extracts outside Git into a path with spaces, repeats native
all-target and adapter tests, rebuilds both companions and repeats browser
behavior checks. It must match the original native target's complete 60 responses
and the original WASM32 target's complete 60 responses separately. It preserves
observed native64/WASM32 differences (35 assignments/four cuts in the 60-case set),
never normalizes labels or selects favorable seeds. Cross-target identity and
cross-directory WASM binary identity are not established.

Native test counts include optional gpmetis/fixture bodies that return when tools
or data are absent. native-environment.json records availability; it does not
prove reference execution. The heavy comparison is gated separately and the
existing generate_golden test is ignored. A green best-effort Prusti job whose
Run step skipped is not formal verification. Browser preview acceptance gives
no gpmetis quality, optimality, broad speed or ROUTE readiness assurance.

Qualification uses a 64-bit native host and wasm32-unknown-unknown. Other native
host widths are outside this replay recipe; the native library retains its own
platform support and compatibility contract.

Fresh live QA copies the hosted wasm-cases.json into private QA as
hosted-wasm-cases.json and requires exact live WASM responses for every request.
Source replay clears live-site overrides and serves the rebuilt local prototype;
it cannot accidentally substitute the published site for the rebuild check.
