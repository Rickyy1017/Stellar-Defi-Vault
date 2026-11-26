#![cfg(test)]
//! Issue #920: deterministic tie-breaking on list queries.

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn top_depositors_tie_by_address() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    let mint = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    let vault_id = env.register_contract(None, crate::vault::VaultContract);
    let vault = crate::vault::VaultContractClient::new(&env, &vault_id);
    vault.initialize(&admin, &token, &0_u32, &None, &None);

    let u1 = Address::generate(&env);
    let u2 = Address::generate(&env);
    mint.mint(&u1, &100);
    mint.mint(&u2, &100);
    vault.deposit(&u1, &100, &0);
    vault.deposit(&u2, &100, &0);

    let rows = vault.get_top_depositors(&2);
    assert_eq!(rows.get(0).unwrap().1, 100);
    assert_eq!(rows.get(1).unwrap().1, 100);
    assert!(
        rows.get(0).unwrap().0.to_string() < rows.get(1).unwrap().0.to_string()
    );
    assert_eq!(rows, vault.get_top_depositors(&2));
}