//! Regression test for issue #934 / PR #955: `compound` must apply `reward_fee_bps`.
//!
//! Previously, `compound` credited the full gross `pending` reward to the
//! user's shares without deducting `reward_fee_bps` or routing the fee share
//! to configured treasury recipients.

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env, Vec,
};
use stellar_defi_vault::vault::{VaultContract, VaultContractClient};

#[test]
fn compound_deducts_reward_fee_and_credits_net_shares() {
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
    let treasury = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract(admin.clone());
    let token = token::Client::new(&env, &token_addr);
    let token_admin = token::StellarAssetClient::new(&env, &token_addr);
    token_admin.mint(&admin, &100_000_000);
    token_admin.mint(&alice, &100_000_000);

    let vault_id = env.register_contract(None, VaultContract);
    let vault = VaultContractClient::new(&env, &vault_id);
    vault.initialize(&admin, &token_addr, &1_000_u32, &None, &None);
    vault.fund_reward_pool(&admin, &50_000_000);

    let mut recipients = Vec::new(&env);
    recipients.push_back(treasury.clone());
    let mut bps_shares = Vec::new(&env);
    bps_shares.push_back(10_000_u32);
    vault.set_treasury_split(&admin, &recipients, &bps_shares);
    vault.set_reward_fee_bps(&admin, &500_u32); // 5%

    vault.deposit(&alice, &10_000_000, &None);

    // Advance enough ledgers for non-trivial reward accrual.
    env.ledger().with_mut(|li| {
        li.sequence_number += 1_000_000;
    });

    let gross_pending = vault.pending_reward(&alice);
    assert!(gross_pending > 0);

    vault.compound(&alice);

    let expected_fee = (gross_pending * 500) / 10_000;
    let expected_net = gross_pending - expected_fee;

    assert_eq!(token.balance(&treasury), expected_fee);
    assert_eq!(vault.staked_amount(&alice), 10_000_000 + expected_net);
}
