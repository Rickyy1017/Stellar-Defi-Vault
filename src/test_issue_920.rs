#![cfg(test)]
//! Regression tests for issue #920 — deterministic `Vec` query ordering.

extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::vec_ordering;

#[test]
fn address_tie_break_is_lexicographic() {
    let env = Env::default();
    let mut addrs: std::vec::Vec<Address> = (0..8).map(|_| Address::generate(&env)).collect();
    addrs.sort_by(|a, b| a.to_string().cmp(&b.to_string()));

    for i in 0..addrs.len() {
        for j in (i + 1)..addrs.len() {
            assert!(vec_ordering::address_lt(&addrs[i], &addrs[j]));
        }
    }
}

#[test]
fn amount_ranking_tie_breaks_on_address() {
    let env = Env::default();
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let (lo, hi) = if vec_ordering::address_lt(&a, &b) {
        (a, b)
    } else {
        (b, a)
    };
    assert!(vec_ordering::rank_by_amount_desc(100, &lo, 100, &hi));
    assert!(!vec_ordering::rank_by_amount_desc(100, &hi, 100, &lo));
    assert!(vec_ordering::rank_by_amount_desc(200, &hi, 100, &lo));
}

#[test]
fn get_top_depositors_order_is_stable_across_calls() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract(admin.clone());
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    let vault_id = env.register_contract(None, crate::vault::VaultContract);
    let vault = crate::vault::VaultContractClient::new(&env, &vault_id);
    vault.initialize(&admin, &token_addr, &0_u32, &None, &None);

    for _ in 0..5 {
        let user = Address::generate(&env);
        token_admin.mint(&user, &100_i128);
        vault.deposit(&user, &100, &0);
    }

    let first = vault.get_top_depositors(&10_u32);
    let second = vault.get_top_depositors(&10_u32);
    assert_eq!(first.len(), second.len());
    for i in 0..first.len() {
        assert_eq!(first.get(i).unwrap(), second.get(i).unwrap());
    }
}