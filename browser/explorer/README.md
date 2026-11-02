# METIS-CORE partition explorer

This standalone companion calls the byte-unchanged native Rust kernel. Release
status and dated evidence live in the wave; source implementation/local checks
alone are not live qualification. No formal proof or ROUTE readiness follows.
Readers edit a synthetic/own single-constraint CSR graph, choose native k-way or
recursive mode, part count, seed, coarsening and bounded controls, inspect the
native assignment/cut and actual part weights/contiguity, and retain separate
original/current/LF-copy/request/report downloads. The SVG is vertex-index layout,
not a geographic or optimized embedding. Partitioning is Rust, not JavaScript.

## Target-specific seed contract

Measured 60 requests across native 64-bit and WASM 32-bit: 25 reports match apart from target
label, 35 assignments differ and 4 cut values differ. Both targets repeat their own
full response exactly. Source uses machine-sized random ranges; cross-target
assignment/cut identity is not established. Native source/defaults/outputs are
unchanged. The browser report includes target_pointer_bits. All successful test
results pass independent coverage/range/occupancy/part-weight/cut arithmetic and
native partition validation. This is not gpmetis quality-envelope or optimality
certification; existing native proof/parity boundaries remain intact.

## Build and local checks

Rust 1.95.0, wasm32-unknown-unknown, clippy/rustfmt, wasm-bindgenCLI 0.2.127,
Node 22, Playwright 1.61.1 and Python 3.11+ are used. From the repository root:

```text
cargo +1.95.0 test --all-targets --locked
cargo +1.95.0 check --lib --target wasm32-unknown-unknown --locked
cargo +1.95.0 test --manifest-path browser/explorer/Cargo.toml --locked
cargo +1.95.0 fmt --manifest-path browser/explorer/Cargo.toml -- --check
python browser/explorer/build-prototype.py --bindgen /absolute/path/to/wasm-bindgen
```

CARGO_TARGET_DIR, if set, must be absolute. The builder freshly builds the native
control and WASM module and captures all native requests/responses. Native64
responses are intentionally not mislabeled as a WASM 32-bit golden oracle. Browser
checks record both outcomes and repeat actual WASM execution. Neither observed
assignment differences nor cut differences are hidden by label normalization.

From browser/explorer:

```text
npm ci --ignore-scripts
npx playwright install chromium
npm run test:pages
```

The prototype is served under /METIS-CORE/. Fifteen packaged Chromium cases cover 60
native/WASM requests, repeated outputs, independent structural/metric checks,
worker controls/diagram, native malformed CSR, original/request downloads and
mobile/keyboard. Private .cache/cross-target.json retains complete outcomes.
The suite also exercises exact worker/request replay, real file/startup/computation
deadlines, stale read/revision/request/generation fencing, crash/restart, maximum
graph/weight admission, memory ceiling, target-weight/seed errors and independent
source/build/legal fallback. Source replay repeats thirteen behavior cases outside
Git and compares all native64 and WASM32 outcomes separately. Exact hosted/default/
live gates still determine the actual published revision.

Browser admission: 1 MiB graph, 3 MiB request framing, 256 vertices, 4,096 adjacency entries,
weight magnitude 1,000, 16 parts, 8 trials, 20 iterations, 16 target weights, 4 MiB report,
64 MiB WASM memory. These are separate from native validation. Non-positive weights,
invalid CSR and unsupported native parameter combinations retain native errors
when inside resource bounds. Native equal-weight defaults and recursive/k-way
mode behavior remain unchanged; controls can make explicit bounded requests.

The baseline initially failed three gpmetis reference launches because the
protected Windows executable lacked a transitive DLL. A verified runtime supplied
through subprocess PATH fixes the environment without source/test/assertion or
protected-folder changes. The original full native suite then passes. Heavy
reference parity remains gated off, and one pre-existing native test is ignored.
See the wave's scoped evidence; no result establishes full formal verification.


## Source and delivery

BUILD.md documents clean-source packaging and independent source replay. source.zip
contains the complete exact Git tree and both locks, preserving every native
src/test/verify/bench/fixture/spec file plus the original Apache-2.0 LICENSE and
NOTICE. Browser dependency/Rust notices accompany the distributed artifact;
external reference EXE/DLLs and private QA data are excluded. Runtime/source/site
budgets are separate 2/20/25 MiB. Optional source is not startup data. A source
rebuild must reproduce each target's complete responses separately; binary and
cross-target output identity are not claimed. Native optional reference/fixture
availability is recorded explicitly, so green counts are not quality proof.

Qualification uses a 64-bit native host and wasm32-unknown-unknown. Other native
host widths are outside this replay recipe; the native library retains its own
platform support and compatibility contract.
