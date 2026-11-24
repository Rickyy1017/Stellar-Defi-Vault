//! Regression test for issue #784: `mul_div` intermediate product overflow.
//!
//! Previously, `a * b / c` used `checked_mul` before `checked_div`, causing
//! false-positive overflow errors whenever `a * b > i128::MAX` even though the
//! final quotient fit comfortably in `i128`.

use stellar_defi_vault::fuzz_support::{
    mul_div_ceil, mul_div_floor, reward_for_ledgers, MathError,
};

#[test]
fn mul_div_handles_256bit_intermediate_without_false_overflow() {
    let a: i128 = 1_000_000_000_000_000_000_000_000;
    let b: i128 = 2_000_000_000_000_000_000_000_000;
    let c: i128 = 4_000_000_000_000_000_000_000_000;

    // `a * b` overflows `i128::MAX`, but `a * b / c == 5e23` fits easily.
    assert_eq!(mul_div_floor(a, b, c), Ok(500_000_000_000_000_000_000_000));
    assert_eq!(mul_div_ceil(a, b, c), Ok(500_000_000_000_000_000_000_000));
}

#[test]
fn mul_div_division_by_zero_and_true_overflow_return_errors() {
    assert_eq!(mul_div_floor(10, 20, 0), Err(MathError::DivisionByZero));
    assert_eq!(
        mul_div_floor(i128::MAX, 2, 1),
        Err(MathError::Overflow)
    );
}

#[test]
fn reward_for_ledgers_large_amount_does_not_false_overflow() {
    let large_amount = i128::MAX / 1_000_000;
    let reward = reward_for_ledgers(large_amount, 1_000, 10_000, 10_000, 10_000, 1_000);
    assert!(reward.is_ok());
    assert!(reward.unwrap() > 0);
}
