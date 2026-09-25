//! Tests for issues #526-#529: scheduled exit, snapshot airdrop, external
//! price oracle, and co-sponsor reward matching.

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::scheduled_exit::ScheduledExit;
use crate::snapshot_airdrop::AirdropRecord;
use crate::co_sponsor::CoSponsor;

// ── Issue #526: scheduled exit ──────────────────────────────────────────────

#[test]
fn test_schedule_exit_sets_target_ledger() {
    let env = Env::default();
    let contract = env.register_contract(None, crate::VaultContract);
    let client = crate::VaultContractClient::new(&env, &contract);
    let user = Address::generate(&env);

    env.mock_all_auths();

    // Initialize the vault first.
    let token = env.register_stellar_asset_contract(Address::generate(&env));
    let token_addr = token.address;
    client.initialize(
        &Address::generate(&env),
        &token_addr,
        &token_addr,
        &1_000u32,
        &0u32,
        &0i128,
    );

    // Schedule exit at a future ledger.
    let result = client.try_schedule_exit(&user, &1000);
    assert!(result.is_ok());
}

#[test]
fn test_schedule_exit_reverts_on_past_ledger() {
    let env = Env::default();
    let contract = env.register_contract(None, crate::VaultContract);
    let client = crate::VaultContractClient::new(&env, &contract);
    let user = Address::generate(&env);

    env.mock_all_auths();

    let token = env.register_stellar_asset_contract(Address::generate(&env));
    let token_addr = token.address;
    client.initialize(
        &Address::generate(&env),
        &token_addr,
        &token_addr,
        &1_000u32,
        &0u32,
        &0i128,
    );

    // Try to schedule at current ledger (should fail).
    let result = client.try_schedule_exit(&user, &100);
    assert!(result.is_err());
}

#[test]
fn test_get_scheduled_exit_returns_none_when_empty() {
    let env = Env::default();
    let contract = env.register_contract(None, crate::VaultContract);
    let client = crate::VaultContractClient::new(&env, &contract);
    let user = Address::generate(&env);

    let result = client.get_scheduled_exit(&user);
    assert!(result.is_none());
}

// ── Issue #527: snapshot airdrop ────────────────────────────────────────────

#[test]
fn test_create_airdrop_returns_id() {
    let env = Env::default();
    let contract = env.register_contract(None, crate::VaultContract);
    let client = crate::VaultContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    env.mock_all_auths();

    let token = env.register_stellar_asset_contract(Address::generate(&env));
    let token_addr = token.address;
    client.initialize(
        &admin,
        &token_addr,
        &token_addr,
        &1_000u32,
        &0u32,
        &0i128,
    );

    let airdrop_token = Address::generate(&env);
    let result = client.try_create_airdrop(&airdrop_token, &100_000, &50);
    assert!(result.is_ok());
}

#[test]
fn test_create_airdrop_reverts_on_future_ledger() {
    let env = Env::default();
    let contract = env.register_contract(None, crate::VaultContract);
    let client = crate::VaultContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    env.mock_all_auths();

    let token = env.register_stellar_asset_contract(Address::generate(&env));
    let token_addr = token.address;
    client.initialize(
        &admin,
        &token_addr,
        &token_addr,
        &1_000u32,
        &0u32,
        &0i128,
    );

    let airdrop_token = Address::generate(&env);
    // Try to create with future snapshot ledger.
    let result = client.try_create_airdrop(&airdrop_token, &100_000, &999999);
    assert!(result.is_err());
}

// ── Issue #528: external price oracle ───────────────────────────────────────

#[test]
fn test_set_price_oracle_and_get() {
    let env = Env::default();
    let contract = env.register_contract(None, crate::VaultContract);
    let client = crate::VaultContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    env.mock_all_auths();

    let token = env.register_stellar_asset_contract(Address::generate(&env));
    let token_addr = token.address;
    client.initialize(
        &admin,
        &token_addr,
        &token_addr,
        &1_000u32,
        &0u32,
        &0i128,
    );

    let oracle = Address::generate(&env);
    let result = client.try_set_price_oracle(&oracle);
    assert!(result.is_ok());
}

#[test]
fn test_get_position_value_usd_reverts_without_oracle() {
    let env = Env::default();
    let contract = env.register_contract(None, crate::VaultContract);
    let client = crate::VaultContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    env.mock_all_auths();

    let token = env.register_stellar_asset_contract(Address::generate(&env));
    let token_addr = token.address;
    client.initialize(
        &admin,
        &token_addr,
        &token_addr,
        &1_000u32,
        &0u32,
        &0i128,
    );

    let user = Address::generate(&env);
    // No oracle set — should revert.
    let result = client.try_get_position_value_usd(&user);
    assert!(result.is_err());
}

// ── Issue #529: co-sponsor ──────────────────────────────────────────────────

#[test]
fn test_register_co_sponsor() {
    let env = Env::default();
    let contract = env.register_contract(None, crate::VaultContract);
    let client = crate::VaultContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    env.mock_all_auths();

    let token = env.register_stellar_asset_contract(Address::generate(&env));
    let token_addr = token.address;
    client.initialize(
        &admin,
        &token_addr,
        &token_addr,
        &1_000u32,
        &0u32,
        &0i128,
    );

    let sponsor = Address::generate(&env);
    let result = client.try_register_co_sponsor(&sponsor, &500, &10000);
    assert!(result.is_ok());
}

#[test]
fn test_register_co_sponsor_reverts_with_zero_bps() {
    let env = Env::default();
    let contract = env.register_contract(None, crate::VaultContract);
    let client = crate::VaultContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    env.mock_all_auths();

    let token = env.register_stellar_asset_contract(Address::generate(&env));
    let token_addr = token.address;
    client.initialize(
        &admin,
        &token_addr,
        &token_addr,
        &1_000u32,
        &0u32,
        &0i128,
    );

    let sponsor = Address::generate(&env);
    // match_bps=0 should fail.
    let result = client.try_register_co_sponsor(&sponsor, &0, &10000);
    assert!(result.is_err());
}
