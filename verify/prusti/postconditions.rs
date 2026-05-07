//! Prusti postcondition documentation.
//!
//! Three correctness postconditions on `Partitioner::split`. Activate with:
//!   cargo prusti -p metis-core
//!
//! Prusti version: 0.2.x (Viper backend, ETH Zurich)
//! See verify/prusti/GAPS.md for functions that cannot be verified.
//! See verify/prusti/artifacts/ for committed .vpr proof files.

// The three postconditions live in src/api.rs on `Partitioner::split`.

/// Postcondition 1: Full coverage — every vertex assigned to exactly one part.
///
/// ```text
/// #[ensures(result.is_ok() ==>
///     result.as_ref().unwrap().assignment.len() == g.n())]
/// ```
pub const POSTCONDITION_1_COVERAGE: &str = "Every vertex assigned to exactly one part.";

/// Postcondition 2: Valid part IDs — all part IDs are in [0, k).
///
/// ```text
/// #[ensures(result.is_ok() ==>
///     forall(|i: usize| i < result.as_ref().unwrap().assignment.len()
///         ==> result.as_ref().unwrap().assignment[i] < k))]
/// ```
pub const POSTCONDITION_2_VALIDITY: &str = "All part IDs are valid (in [0, k)).";

/// Postcondition 3: Vertex-weight balance ≤ ε
/// ε = ceil(total_weight × 0.005) = (total_weight × 5 + 999) / 1000
///
/// ```text
/// #[ensures(result.is_ok() ==>
///     weight_balance(result.as_ref().unwrap(), g) <= epsilon(g))]
/// ```
///
/// Uses integer arithmetic only — no float — preserving determinism.
pub const POSTCONDITION_3_BALANCE: &str =
    "Vertex-weight balance ≤ 0.5%. Integer arithmetic: epsilon = (total * 5 + 999) / 1000.";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn postconditions_documented() {
        assert!(!POSTCONDITION_1_COVERAGE.is_empty());
        assert!(!POSTCONDITION_2_VALIDITY.is_empty());
        assert!(!POSTCONDITION_3_BALANCE.is_empty());
    }
}
