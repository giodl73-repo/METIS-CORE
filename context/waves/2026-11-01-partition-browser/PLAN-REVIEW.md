# API, correctness and parity review — AI-simulated lenses

Read .roles/ROLE.md and all three parliament roles, compatibility policy,
production plan, PITFALL principles/invariants, native public APIs and Prusti gaps.
No human/ROUTE-owner approval or formal proof is claimed by this review.

- Correctness steward: native src/Cargo/tests/verify/benches stay byte-identical.
  All browser graph construction, controls, partitioning and validation go through
  existing public Rust APIs. Separate browser limits bound counts/weights/work;
  positive-weight, connected-undirected CSR and error meanings remain native.
  Actual per-part vertex/weight totals, cut and contiguity stay visible.
- API auditor: browser JSON/framing errors and refusals stay distinct from exact
  native PartitionError variants/messages. Seed is a decimal u64 string, never
  a JavaScript floating-point value. Native mode defaults are retained when the
  companion request omits options. Original/current/LF-copy/request/report exports
  are distinct. No core symbol, default, error or native deterministic output changes.
- Parity/performance reviewer: the measured native 64-bit/WASM 32-bit experiment has 60 cases,
 25 equal reports apart from target label,35 different assignments and 4 different cuts.
  Both targets repeat exactly within their build; successful partitions pass
  independent coverage/range/occupancy/weight/cut checks plus native validation.
  Source uses machine-sized random ranges. This is target-specific replay,
  not cross-target identity or gpmetis quality parity. Preserve every raw outcome;
  do not normalize labels, choose favorable seeds or rewrite native RNG.
- Reference environment: initial full baseline reached three external reference
  tests but the discovered protected gpmetis.exe lacked libtre-5.dll transitively.
  Verified MSYS2 packages in a session tools folder supply only subprocess PATH;
  protected apportionment files remain unchanged. The original unmodified full
  suite then passes, including three enabled reference comparisons. Heavy parity
  remains gated off; no broad browser quality or formal-proof claim follows.

No native foundation change is proposed. ROUTE rehearsal remains required for
any later native API/default/validation/error/output/quality change or ROUTE
readiness promotion. This isolated browser preview grants none of those claims.
Publication still requires complete reader recovery, artifact/legal/source replay,
hosted/default and fresh live gates; current prototype evidence is not release proof.
