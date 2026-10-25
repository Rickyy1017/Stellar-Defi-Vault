#![cfg(test)]
//! Ledger-boundary tests for the two-phase withdrawal cooldown.

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};

use crate::{errors::VaultError, vault::{VaultContract, VaultContractClient}};

fn set_ledger(env: &Env, sequence: u32) {
    env.ledger().with_mut(|ledger| ledger.sequence_number = sequence);
}

#[test]
fn withdrawal_cooldown_is_inclusive_at_the_exact_ledger_boundary() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract(admin.clone());
    let token_admin = token::StellarAssetClient::new(&env, &token_address);
    token_admin.mint(&user, &1_000_000);

    let vault_id = env.register_contract(None, VaultContract);
    let vault = VaultContractClient::new(&env, &vault_id);
    vault.initialize(&admin, &token_address, &0_u32, &None, &None);

    set_ledger(&env, 1_000);
    let shares = vault.stake(&user, &100_000, &0);
    vault.set_withdrawal_cooldown(&admin, &100);
    vault.request_withdrawal(&user, &shares);

    set_ledger(&env, 1_099);
    assert_eq!(
        vault.try_execute_withdrawal(&user),
        Err(Ok(VaultError::UseCooldownFlow))
    );

    set_ledger(&env, 1_100);
    assert!(vault.try_execute_withdrawal(&user).is_ok());
}
