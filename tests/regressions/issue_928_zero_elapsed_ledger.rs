//! Regression test for issue #928: zero-elapsed-ledger accrual must be a no-op.
//!
//! Calling `pending_reward` or `claim` at the exact same ledger sequence as
//! `deposit` must yield `0` and leave the user's staked balance untouched.

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};
use stellar_defi_vault::vault::{VaultContract, VaultContractClient};

#[test]
fn zero_elapsed_ledger_yields_zero_reward_and_preserves_stake() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.sequence_number = 1_000;
        li.min_temp_entry_ttl = 10_000_000;
        li.min_persistent_entry_ttl = 10_000_000;
        li.max_entry_ttl = 10_000_000;
    });

    let admin = Address::generate(&env);
    let alice = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract(admin.clone());
    let token_admin = token::StellarAssetClient::new(&env, &token_addr);
    token_admin.mint(&admin, &10_000_000);
    token_admin.mint(&alice, &10_000_000);

    let vault_id = env.register_contract(None, VaultContract);
    let vault = VaultContractClient::new(&env, &vault_id);
    vault.initialize(&admin, &token_addr, &1_000_u32, &None, &None);
    vault.fund_reward_pool(&admin, &5_000_000);

    vault.deposit(&alice, &1_000_000, &None);

    // Same ledger sequence: pending reward and claim must both be 0.
    assert_eq!(vault.pending_reward(&alice), 0);
    assert_eq!(vault.claim(&alice), 0);
    assert_eq!(vault.staked_amount(&alice), 1_000_000);
}
