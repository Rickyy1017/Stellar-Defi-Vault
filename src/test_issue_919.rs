#![cfg(test)]
//! Regression tests for issue #919 — reject duplicate addresses in `Vec` parameters.

extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::vec_unique::VecParamError;

#[test]
fn helper_rejects_duplicate_at_start() {
    let env = Env::default();
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let list = vec![&env, a.clone(), a, b];
    assert_eq!(
        crate::vec_unique::ensure_unique_addresses(&list),
        Err(VecParamError::DuplicateEntry)
    );
}

#[test]
fn helper_rejects_duplicate_at_end() {
    let env = Env::default();
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let list = vec![&env, a, b.clone(), b];
    assert_eq!(
        crate::vec_unique::ensure_unique_addresses(&list),
        Err(VecParamError::DuplicateEntry)
    );
}

#[test]
fn helper_rejects_duplicate_adjacent() {
    let env = Env::default();
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let list = vec![&env, b, a.clone(), a];
    assert_eq!(
        crate::vec_unique::ensure_unique_addresses(&list),
        Err(VecParamError::DuplicateEntry)
    );
}

#[test]
fn helper_rejects_duplicate_kyc_pairs() {
    let env = Env::default();
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let pairs = vec![&env, (a.clone(), true), (a, false), (b, true)];
    assert_eq!(
        crate::vec_unique::ensure_unique_address_pairs_bool(&pairs),
        Err(VecParamError::DuplicateEntry)
    );
}