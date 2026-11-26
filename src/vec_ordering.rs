//! Deterministic ordering helpers for query `Vec` returns (issue #920).

use soroban_sdk::Address;

/// Lexicographic order on Stellar address string forms (stable across runs).
pub fn address_lt(a: &Address, b: &Address) -> bool {
    a.to_string() < b.to_string()
}

/// `true` when `(a_size, a)` should appear before `(b_size, b)` in a
/// descending leaderboard (higher size first; ties broken by lower address).
pub fn rank_by_amount_desc(a_size: i128, a: &Address, b_size: i128, b: &Address) -> bool {
    if a_size != b_size {
        a_size > b_size
    } else {
        address_lt(a, b)
    }
}