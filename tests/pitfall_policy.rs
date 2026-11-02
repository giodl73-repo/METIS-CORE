// METIS-PF-01 / METIS-PF-02 / METIS-PF-03: retain claim-boundary coverage.

fn assert_contains(haystack: &str, needle: &str, label: &str) {
    assert!(haystack.contains(needle), "missing `{needle}` in {label}");
}

#[test]
fn open_pitfalls_are_use_case_first_and_test_backed() {
    let pitfalls = include_str!("../.pitfall/metis-core-pitfalls.md");
    for id in ["METIS-PF-01", "METIS-PF-02", "METIS-PF-03"] {
        assert_contains(pitfalls, id, ".pitfall/metis-core-pitfalls.md");
    }
    for field in [
        "**Actor:**",
        "**Task:**",
        "**Surface:**",
        "**Likely mistake:**",
        "**Consequence:**",
        "**Owner:**",
        "**Test:** `cargo test --test pitfall_policy`.",
    ] {
        assert_contains(pitfalls, field, ".pitfall/metis-core-pitfalls.md");
    }
    assert_contains(pitfalls, "MITIGATED", ".pitfall/metis-core-pitfalls.md");

    let boundaries = include_str!("../docs/pitfall-boundaries.v1.json");
    for phrase in [
        "METIS-PF-01",
        "METIS-PF-02",
        "METIS-PF-03",
        "gpmetis cut-quality parity",
        "fully formally verified",
        "downstream readiness without ROUTE",
        "benchmark envelope",
        "verify/prusti/GAPS.md",
        "ROUTE downstream rehearsal",
    ] {
        assert_contains(boundaries, phrase, "docs/pitfall-boundaries.v1.json");
    }

    let roles = include_str!("../.roles/ROLE.md");
    for phrase in [
        "PITFALL gate routing",
        "Parity Performance Reviewer",
        "Partition Correctness Steward",
        "API Contract Auditor",
        "fully formally verified",
        "portfolio-snapshot ready",
    ] {
        assert_contains(roles, phrase, ".roles/ROLE.md");
    }
}

#[test]
fn public_claims_stay_bounded_to_evidence() {
    let readme = include_str!("../README.md");
    assert_contains(readme, "A structurally valid partition or", "README.md");
    assert_contains(
        readme,
        "not a claim of `gpmetis` cut-quality parity",
        "README.md",
    );
    assert_contains(readme, "Bounded Kani model-checker harnesses", "README.md");
    assert_contains(
        readme,
        "Prusti postcondition stubs and documented gaps",
        "README.md",
    );

    let production_plan = include_str!("../docs/PRODUCTION_PLAN.md");
    assert_contains(
        production_plan,
        "compare against real `gpmetis` without expecting identical labels",
        "docs/PRODUCTION_PLAN.md",
    );
    assert_contains(
        production_plan,
        "Edge cut stays within an agreed envelope.",
        "docs/PRODUCTION_PLAN.md",
    );
    assert_contains(
        production_plan,
        "Runtime is not pathological.",
        "docs/PRODUCTION_PLAN.md",
    );
}

#[test]
fn verification_and_downstream_rehearsal_limits_remain_visible() {
    let gaps = include_str!("../verify/prusti/GAPS.md");
    assert_contains(
        gaps,
        "Current status: ONE DEFERRED ITEM",
        "verify/prusti/GAPS.md",
    );
    assert_contains(gaps, "population_balanced()", "verify/prusti/GAPS.md");

    let bounds = include_str!("../verify/kani/BOUNDS.md");
    assert_contains(
        bounds,
        "This file justifies the bound choices for each Kani harness.",
        "verify/kani/BOUNDS.md",
    );
    assert_contains(
        bounds,
        "A bound covers all code paths when increasing it further produces no new LLVM bitcode coverage.",
        "verify/kani/BOUNDS.md",
    );

    let compatibility = include_str!("../docs/compatibility.md");
    assert_contains(
        compatibility,
        "ROUTE is the required first consumer rehearsal",
        "docs/compatibility.md",
    );
    assert_contains(
        compatibility,
        "METIS-CORE foundation changes are not ready until both its contract tests and",
        "docs/compatibility.md",
    );
}
