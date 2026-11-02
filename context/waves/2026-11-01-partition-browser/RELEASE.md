# Partition explorer candidate qualification

Candidate `2f443a8b76f556f64282da98b5a7315c20cacd40` passes hosted
[run 41386647423](https://github.com/giodl73-repo/METIS-CORE/actions/runs/41386647423).
Fifteen packaged browser cases pass with zero skipped/flaky/unexpected cases
and no whole-test retries. Full native 258 tests and adapter 4 tests pass; the
existing `generate_golden` test remains ignored. Strict native all-target and
core/companion host/WASM lint, formatting, docs and foundation guards pass.
Inherited native CI passes Windows, macOS and Linux. No new Kani/Prusti proof
is claimed; an inherited green Prusti wrapper with Run skipped is not proof.

A complete source ZIP independently rebuilds outside Git in a path with spaces,
repeating 258 native / 4 adapter / 13 browser checks. It exactly reproduces all 60 native
control responses and all 60 WASM32 responses separately. Both local Windows and
hosted Linux replay pass. The 60-case experiment retains 25 reports equal apart
from target label, 35 different assignments and 4 different cut values; full raw
outcomes are retained without label/seed normalization. Successful results pass
native validation and independent coverage/range/occupancy/cut/weight/source-hash
checks; each target repeats exactly within its build. Neither cross-target output
identity nor cross-directory/platform WASM binary identity is established.

The downloaded hosted 101-file artifact verifies, with 400,000 initial runtime bytes,
99 exact tracked source files, 34 browser dependency notice groups and original
Apache-2.0 LICENSE/NOTICE plus Rust library copyright/twelve notices. Source/
build/artifact/legal routes remain independent of WASM, including late example
completion after engine failure. Optional source and startup are separate;
budgets are 2/20/25 MiB runtime/source/site. QA outputs, captures, raw metadata and
reference EXE/DLLs are excluded. Native source dependencies remain pinned in the
full locks; the source ZIP is not a vendored offline compiler pack.

Native core/Cargo/bench bytes also match public entry `b4dc966`. All protected
native tests/verification/doctrine/roles/CI/grants match canonical `df08dec`, retaining
both prior PITFALL commits. No algorithm, native default/error, deterministic
native output or quality-envelope change is introduced. ROUTE readiness and any
future foundation change remain separately gated by docs/compatibility.md.

Native reference scope is explicit. The local unchanged suite exercises three
enabled gpmetis comparisons after a hash-verified isolated DLL-runtime repair;
protected apportionment files remain untouched. Hosted/source replay records
that gpmetis and external fixtures are absent, so optional early-return bodies
are not fresh reference coverage. Heavy reference parity remains gated off.
No browser/gpmetis quality parity, optimality, broad speed, complete formal
verification or production/ROUTE readiness follows from this preview.

Retained failures: the initial native reference executable missed a transitive
DLL; the first browser helper used a wrong path and collected no tests. Clean
candidate `2184c38` then exposed a recovery-test import of cross-target.json produced
by another test. Local cached QA had masked the ordering bug. The stale message
is now a self-contained synthetic typed error, explicitly not a partition result
or golden oracle; identical stale-ID/revision/generation assertions remain.
The failed hosted artifact and independent local source log remain retained.

Reader evidence includes full-u64 seeds, native target-weight semantics,
maximum 256-vertex / 4,096-adjacency / weight-1,000 input, separate browser refusals,
64 MiB memory, actual 5/12/10-second read/startup/computation deadlines, crash/
restart and read/revision/request/generation fencing. Original/current/LF-copy/
request/report exports remain distinct, with exact worker/request WASM replay,
inert diagnostics, assignment diagram/table, mobile and keyboard flows. Timings
include JS/message/serialization overhead and are not speed comparisons.

This is dated candidate evidence, not certification of every future main revision.
Final review documentation must qualify again; only tested default-master artifacts
deploy, verified again after transfer. Fresh default/live hashes and browser proof
for the actual provenance commit gate TRACKER advancement. Roles are AI-simulated
lenses, not human approval, formal proof or consumer-wide compatibility acceptance.
