// verify_gain_table_no_overflow harness lives in src/refine/gain.rs
// verify_gain_table_update_no_panic harness lives in src/refine/gain.rs
// verify_fm_no_oob harness lives in src/refine/fm.rs
// under #[cfg(kani)] mod kani_proofs
// Run: cargo kani --harness verify_gain_table_no_overflow
//      cargo kani --harness verify_gain_table_update_no_panic
//      cargo kani --harness verify_fm_no_oob
