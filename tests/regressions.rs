//! Named regression tests — one module per fixed bug (issue #927).
//!
//! Every bug fix PR must add a `tests/regressions/issue_<number>_<slug>.rs`
//! module and register it below so fixed bugs stay fixed.

#[cfg(feature = "testutils")]
#[path = "regressions/issue_784_mul_div_overflow.rs"]
mod issue_784_mul_div_overflow;

#[cfg(feature = "testutils")]
#[path = "regressions/issue_928_zero_elapsed_ledger.rs"]
mod issue_928_zero_elapsed_ledger;

#[cfg(feature = "testutils")]
#[path = "regressions/issue_934_reward_fee_on_compound.rs"]
mod issue_934_reward_fee_on_compound;
