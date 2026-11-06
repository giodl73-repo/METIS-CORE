# Portable seed replay candidate

Status: controlled experiment passes; downstream rehearsal and immutable-pin
adoption are separate gates. Default behavior remains legacy.

TERRAIN's unchanged original native64 and WASM32 planning results differed in
seven of 69 full controls. METIS-CORE documents target-specific replay. Three
random vertex-index draws use `usize`; Rand samples that type at different
widths. The additive, disabled-by-default `portable-seed-replay` feature changes
only these draws to `u64`. Random `u32` partition assignment is unchanged.
Private source modules, public Rust signatures, defaults, validation, errors,
zero-weight/empty-part semantics and upstream Apache-2.0/NOTICE are preserved.

## Controlled downstream evidence

A tracked-file copy of TERRAIN at `e98306ded98714e1c5ef8439d8e4a3361a8e0c31`
uses an explicit local Cargo patch to this dirty candidate, based on
`4b88dfc5ba89565f0441ce5091daec76c057dc6a`. Production Rust expressions stay
constant across the probe; the unit-test reference was strengthened during it.
This is causal evidence for this candidate, not a clean immutable-pin delivery.

- Legacy native output exactly matches all 6,651,179 original bytes.
- Legacy WASM output exactly matches all 6,651,186 original bytes, retaining
  the original seven discrepancies.
- Opt-in native and WASM each exactly match all 6,651,179 original native bytes.
- All 69 full results and independent outcome/boundary/conservation/zero-total
  expectations pass. Inputs and outputs are not sorted, relabeled or normalized.
- Both methods, 2,000 sites, 32 territories, quoted Unicode CSV, numeric limits,
  and ASCII/multibyte 500,000-byte boundaries remain in the controls. Raw JS
  coercion observations are separate from typed-native parity.

The unit test checks sampling values and remaining RNG stream across five
seeds, small and maximal bounds. On native64 its reference is the original
`usize` expression in both feature modes. Both modes pass 259 tests, with one
existing ignored test and four explicitly filtered external gpmetis cases.
Strict all-target Clippy, formatting and feature-enabled docs pass.

The original broad run failed three external gpmetis cases before comparison:
the installed executable exits `3221225781` even for `-help`, producing no
output. Preserve this failure, not an all-tests-passing claim. No gpmetis
quality, speed, broad graph-family performance or formal-proof claim follows.
See [custody](validation/CUSTODY.json) and the full command/result records.

## Review and promotion gates

Implementing-agent role review, not independent human approval:

- Partition Correctness Steward: sampling only; preserve all existing graph,
  balance, contiguity, occupancy and zero semantics. Internal suites and full
  downstream outputs are evidence; no stronger domain guarantee is inferred.
- API Contract Auditor: no exported symbol/signature/error/default changes;
  private helpers and safe Rust remain. The new Cargo feature is additive, and
  feature unification is documented for every consumer of the resolved package.
- Parity Performance Reviewer: exact replay checks are appropriate for this
  sampling compatibility experiment; they are not a gpmetis cut-quality test.
  No benchmark or broader quality result is asserted.

ROUTE owns the two synthetic consumer fixture rehearsals; its canonical source,
ignored config and research data must remain untouched. TERRAIN owns subsequent
explicit immutable Git-pin/feature adoption and a fresh source-bound comparison.
The existing METIS browser explorer remains on legacy defaults. No foundation,
ROUTE, release or portfolio-snapshot readiness is promoted by this draft.

## Required ROUTE rehearsal

Both named fixture tests pass individually in legacy and portable modes (four
checks, each exactly one passing test). The tracked-file source copy is ROUTE
`ae858ef592aa182f1d647d93fc871dd1e977df08`; its local repo-map generates
candidate METIS and existing FLETCH patches. Lockfiles before/after, generated
configuration, commands and raw results are retained. These are synthetic
service/dual-route fixtures, with no external research cache or local ignored
configuration copied. ROUTE has not adopted the feature or a new source pin.
Contract and consumer rehearsal gates pass for this scoped draft; promotion
and any broader readiness claim remain separate.

## Hosted qualification repairs

First hosted run `42162503491` rejected the candidate at the old absolute
unchanged-foundation guard. Retain that failure. Replace that assertion with an
explicit transition manifest: exact changed blobs/modes from both original
anchors to `0ad9f83`, followed by zero further protected drift. Build and package
share the guard; package provenance carries the transition. A separate local
clone accepts the reviewed transition and rejects changed source, changed grant
and untracked protected source (three negative controls). The browser companion
still uses legacy sampling, with its source wording corrected accordingly.

The Pages runner is pinned to Ubuntu 24.04 for the installed Playwright version;
TERRAIN's earlier hosted run failed on unsupported Ubuntu 26.04. Feature-enabled
native tests and both native/WASM Clippy are added to METIS Pages qualification.
No hosted replay success is claimed until the corrected workflow finishes.
