#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger as _},
    token, Address, Env, Symbol, TryFromVal, Vec,
};

use crate::{
    errors::VaultError,
    nft::{StakeReceiptNFT, StakeReceiptNFTClient},
    vault::{VaultContract, VaultContractClient, BOOST_BPS_BASE, STELLAR_LEDGERS_PER_YEAR},
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn create_token<'a>(
    env: &Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let address = env.register_stellar_asset_contract(admin.clone());
    let client = token::Client::new(env, &address);
    let admin_client = token::StellarAssetClient::new(env, &address);
    (address, client, admin_client)
}

fn set_ledger(env: &Env, sequence: u32) {
    env.ledger().with_mut(|li| {
        li.sequence_number = sequence;
    });
}

fn boost_schedule(env: &Env, tiers: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut schedule = Vec::new(env);
    for tier in tiers {
        schedule.push_back(*tier);
    }
    schedule
}

fn topic_matches(env: &Env, topics: &Vec<soroban_sdk::Val>, name: &str) -> bool {
    match topics.get(0) {
        Some(val) => Symbol::try_from_val(env, &val)
            .map(|topic| topic == Symbol::new(env, name))
            .unwrap_or(false),
        None => false,
    }
}

struct VaultFixture<'a> {
    env: Env,
    vault: VaultContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    admin: Address,
    alice: Address,
    bob: Address,
}

impl<'a> VaultFixture<'a> {
    fn new() -> Self {
        Self::with_mock_auths(true)
    }

    fn with_mock_auths(mock_auths: bool) -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.min_temp_entry_ttl = 1_000_000;
            li.min_persistent_entry_ttl = 1_000_000;
            li.max_entry_ttl = 1_000_000;
        });

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        let (token_addr, token, token_admin) = create_token(&env, &admin);

        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);

        vault.initialize(&admin, &token_addr);

        // Mint starting balances
        token_admin.mint(&alice, &20_000_000);
        token_admin.mint(&bob, &20_000_000);

        if !mock_auths {
            env.set_auths(&[]);
        }

        VaultFixture {
            env,
            vault,
            token,
            token_admin,
            admin,
            alice,
            bob,
        }
    }
}

// ── initialization ────────────────────────────────────────────────────────────

#[test]
fn test_initialize_sets_state() {
    let f = VaultFixture::new();
    let (total_shares, total_deposited) = f.vault.vault_state();
    assert_eq!(total_shares, 0);
    assert_eq!(total_deposited, 0);
}

#[test]
fn test_double_initialize_fails() {
    let f = VaultFixture::new();
    let token_addr: soroban_sdk::Address = f
        .env
        .register_stellar_asset_contract(Address::generate(&f.env));
    let result = f.vault.try_initialize(&f.admin, &token_addr);
    assert_eq!(result, Err(Ok(VaultError::AlreadyInitialized)));
}

// ── deposit ───────────────────────────────────────────────────────────────────

#[test]
fn test_first_deposit_mints_1to1_shares() {
    let f = VaultFixture::new();
    let shares = f.vault.deposit(&f.alice, &500_000);
    assert_eq!(shares, 500_000);
    assert_eq!(f.vault.shares_of(&f.alice), 500_000);

    let (total_shares, total_deposited) = f.vault.vault_state();
    assert_eq!(total_shares, 500_000);
    assert_eq!(total_deposited, 500_000);
}

#[test]
fn test_deposit_zero_fails() {
    let f = VaultFixture::new();
    let result = f.vault.try_deposit(&f.alice, &0);
    assert_eq!(result, Err(Ok(VaultError::ZeroAmount)));
}

#[test]
fn test_deposit_negative_fails() {
    let f = VaultFixture::new();
    let result = f.vault.try_deposit(&f.alice, &-100);
    assert_eq!(result, Err(Ok(VaultError::ZeroAmount)));
}

#[test]
fn test_two_depositors_get_proportional_shares() {
    let f = VaultFixture::new();

    let alice_shares = f.vault.deposit(&f.alice, &400_000);
    let bob_shares = f.vault.deposit(&f.bob, &100_000);

    assert_eq!(alice_shares, 400_000);
    assert_eq!(bob_shares, 100_000);

    let (total_shares, _) = f.vault.vault_state();
    assert_eq!(total_shares, 500_000);
}

// ── withdraw ──────────────────────────────────────────────────────────────────

#[test]
fn test_withdraw_returns_correct_amount() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &600_000);

    let token_before = f.token.balance(&f.alice);
    let amount_back = f.vault.withdraw(&f.alice, &300_000);

    assert_eq!(amount_back, 300_000);
    assert_eq!(f.vault.shares_of(&f.alice), 300_000);
    assert_eq!(f.token.balance(&f.alice), token_before + 300_000);
}

#[test]
fn test_withdraw_more_than_owned_fails() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &100_000);

    let result = f.vault.try_withdraw(&f.alice, &200_000);
    assert_eq!(result, Err(Ok(VaultError::InsufficientShares)));
}

#[test]
fn test_withdraw_zero_fails() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &100_000);

    let result = f.vault.try_withdraw(&f.alice, &0);
    assert_eq!(result, Err(Ok(VaultError::ZeroAmount)));
}

#[test]
fn test_full_withdraw_clears_shares() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &400_000);
    f.vault.withdraw(&f.alice, &400_000);

    assert_eq!(f.vault.shares_of(&f.alice), 0);
    let (total_shares, total_deposited) = f.vault.vault_state();
    assert_eq!(total_shares, 0);
    assert_eq!(total_deposited, 0);
}

// ── preview_redeem ────────────────────────────────────────────────────────────

#[test]
fn test_preview_redeem_matches_actual_withdraw() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &500_000);

    let preview = f.vault.preview_redeem(&250_000);
    let actual = f.vault.withdraw(&f.alice, &250_000);

    assert_eq!(preview, actual);
}

// ── pause / unpause ───────────────────────────────────────────────────────────

#[test]
fn test_pause_blocks_deposit() {
    let f = VaultFixture::new();
    f.vault.pause();

    let result = f.vault.try_deposit(&f.alice, &100_000);
    assert_eq!(result, Err(Ok(VaultError::VaultPaused)));
}

#[test]
fn test_pause_blocks_withdraw() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &100_000);
    f.vault.pause();

    let result = f.vault.try_withdraw(&f.alice, &100_000);
    assert_eq!(result, Err(Ok(VaultError::VaultPaused)));
}

#[test]
fn test_unpause_restores_operations() {
    let f = VaultFixture::new();
    f.vault.pause();
    f.vault.unpause();

    let shares = f.vault.deposit(&f.alice, &100_000);
    assert_eq!(shares, 100_000);
}

// ── admin transfer ────────────────────────────────────────────────────────────

#[test]
fn test_transfer_admin() {
    let f = VaultFixture::new();
    f.vault.transfer_admin(&f.bob);
    // Bob is now admin — he should be able to pause
    f.vault.pause();
}

// ── yield accrual ────────────────────────────────────────────────────────────

#[test]
fn test_add_yield_increases_share_price() {
    let f = VaultFixture::new();

    // Alice deposits 500k -> 500k shares
    f.vault.deposit(&f.alice, &500_000);

    // Mint tokens to admin so they can add yield
    f.token_admin.mint(&f.admin, &100_000);

    // Preview before yield: 250k shares -> 250k tokens
    let preview_before = f.vault.preview_redeem(&250_000);
    assert_eq!(preview_before, 250_000);

    // Admin adds 100k yield
    f.vault.add_yield(&f.admin, &100_000);

    // Vault total_deposited should increase
    let (_total_shares, total_deposited) = f.vault.vault_state();
    assert_eq!(total_deposited, 600_000);

    // Preview after yield: 250k shares -> 300k tokens
    let preview_after = f.vault.preview_redeem(&250_000);
    assert_eq!(preview_after, 300_000);
}

#[test]
fn test_add_yield_requires_admin_auth() {
    let f = VaultFixture::new();
    f.token_admin.mint(&f.admin, &10_000);

    f.vault.add_yield(&f.admin, &10_000);
    assert_eq!(f.env.auths()[0].0, f.admin);
}

#[test]
fn test_add_yield_paused_blocks() {
    let f = VaultFixture::new();
    f.token_admin.mint(&f.admin, &50_000);
    f.vault.pause();

    let result = f.vault.try_add_yield(&f.admin, &50_000);
    assert_eq!(result, Err(Ok(VaultError::VaultPaused)));
}

#[test]
fn test_add_yield_zero_fails() {
    let f = VaultFixture::new();
    f.token_admin.mint(&f.admin, &10_000);

    let result = f.vault.try_add_yield(&f.admin, &0);
    assert_eq!(result, Err(Ok(VaultError::ZeroAmount)));
}

// ── withdrawal limit (Issue #8) ──────────────────────────────────────────────

#[test]
fn test_set_withdrawal_limit() {
    let f = VaultFixture::new();
    f.vault.set_withdrawal_limit(&100_000);
    assert_eq!(f.vault.get_withdrawal_limit(), 100_000);
}

#[test]
fn test_withdrawal_limit_blocks_large_withdrawal() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &500_000);
    f.vault.set_withdrawal_limit(&100_000);

    let result = f.vault.try_withdraw(&f.alice, &200_000);
    assert_eq!(result, Err(Ok(VaultError::WithdrawalLimitExceeded)));
}

#[test]
fn test_withdrawal_limit_allows_within_limit() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &500_000);
    f.vault.set_withdrawal_limit(&100_000);

    let amount = f.vault.withdraw(&f.alice, &100_000);
    assert_eq!(amount, 100_000);
    assert_eq!(f.vault.shares_of(&f.alice), 400_000);
}

#[test]
fn test_withdrawal_limit_exact_boundary() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &500_000);
    f.vault.set_withdrawal_limit(&100_000);

    // Exactly at limit should work
    let amount = f.vault.withdraw(&f.alice, &100_000);
    assert_eq!(amount, 100_000);
}

#[test]
fn test_withdrawal_limit_one_over_fails() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &500_000);
    f.vault.set_withdrawal_limit(&100_000);

    // One over limit should fail
    let result = f.vault.try_withdraw(&f.alice, &100_001);
    assert_eq!(result, Err(Ok(VaultError::WithdrawalLimitExceeded)));
}

#[test]
fn test_admin_updates_withdrawal_limit() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &500_000);

    // Set initial limit
    f.vault.set_withdrawal_limit(&50_000);
    assert_eq!(f.vault.get_withdrawal_limit(), 50_000);

    // 60k fails with old limit
    let result = f.vault.try_withdraw(&f.alice, &60_000);
    assert_eq!(result, Err(Ok(VaultError::WithdrawalLimitExceeded)));

    // Admin raises limit
    f.vault.set_withdrawal_limit(&100_000);
    assert_eq!(f.vault.get_withdrawal_limit(), 100_000);

    // 60k now passes
    let amount = f.vault.withdraw(&f.alice, &60_000);
    assert_eq!(amount, 60_000);
}

#[test]
fn test_set_withdrawal_limit_zero_fails() {
    let f = VaultFixture::new();
    let result = f.vault.try_set_withdrawal_limit(&0);
    assert_eq!(result, Err(Ok(VaultError::ZeroAmount)));
}

#[test]
fn test_set_withdrawal_limit_negative_fails() {
    let f = VaultFixture::new();
    let result = f.vault.try_set_withdrawal_limit(&-100);
    assert_eq!(result, Err(Ok(VaultError::ZeroAmount)));
}

#[test]
fn test_set_withdrawal_limit_requires_admin_auth() {
    let f = VaultFixture::new();
    f.vault.set_withdrawal_limit(&100_000);
    assert_eq!(f.env.auths()[0].0, f.admin);
}

#[test]
fn test_no_withdrawal_limit_by_default() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &500_000);

    // No limit set, should be 0 (no restriction)
    assert_eq!(f.vault.get_withdrawal_limit(), 0);

    // Should be able to withdraw everything
    let amount = f.vault.withdraw(&f.alice, &500_000);
    assert_eq!(amount, 500_000);
}

// ── event emission (Issue #7) ─────────────────────────────────────────────────

#[test]
fn test_deposit_emits_event() {
    let f = VaultFixture::new();

    f.vault.deposit(&f.alice, &100_000);

    let events = f.env.events().all();
    let deposit_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "deposit"))
        .collect();

    assert_eq!(deposit_events.len(), 1);
    let event = &deposit_events[0];
    assert_eq!(
        Address::try_from_val(&f.env, &event.1.get(1).unwrap()).unwrap(),
        f.alice
    );
}

#[test]
fn test_withdraw_emits_event() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &100_000);

    f.vault.withdraw(&f.alice, &50_000);

    let events = f.env.events().all();
    let withdraw_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "withdraw"))
        .collect();

    assert_eq!(withdraw_events.len(), 1);
    let event = &withdraw_events[0];
    assert_eq!(
        Address::try_from_val(&f.env, &event.1.get(1).unwrap()).unwrap(),
        f.alice
    );
}

#[test]
fn test_pause_emits_event() {
    let f = VaultFixture::new();

    f.vault.pause();

    let events = f.env.events().all();
    let paused_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "paused"))
        .collect();

    assert_eq!(paused_events.len(), 1);
}

#[test]
fn test_unpause_emits_event() {
    let f = VaultFixture::new();
    f.vault.pause();

    f.vault.unpause();

    let events = f.env.events().all();
    let unpaused_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "unpaused"))
        .collect();

    assert_eq!(unpaused_events.len(), 1);
}

#[test]
fn test_transfer_admin_emits_event() {
    let f = VaultFixture::new();

    f.vault.transfer_admin(&f.bob);

    let events = f.env.events().all();
    let admin_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "admin_set"))
        .collect();

    assert_eq!(admin_events.len(), 1);
    let event = &admin_events[0];
    assert_eq!(
        Address::try_from_val(&f.env, &event.1.get(1).unwrap()).unwrap(),
        f.admin
    );
}

#[test]
fn test_withdrawal_limit_update_emits_event() {
    let f = VaultFixture::new();

    f.vault.set_withdrawal_limit(&100_000);

    let events = f.env.events().all();
    let limit_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "wd_limit"))
        .collect();

    assert_eq!(limit_events.len(), 1);
}

#[test]
fn test_yield_added_emits_event() {
    let f = VaultFixture::new();
    f.token_admin.mint(&f.admin, &50_000);

    f.vault.add_yield(&f.admin, &50_000);

    let events = f.env.events().all();
    let yield_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "yield_add"))
        .collect();

    assert_eq!(yield_events.len(), 1);
}

// ── error handling edge cases (Issue #9) ─────────────────────────────────────

#[test]
fn test_deposit_negative_amount_fails() {
    let f = VaultFixture::new();
    let result = f.vault.try_deposit(&f.alice, &-500);
    assert_eq!(result, Err(Ok(VaultError::ZeroAmount)));
}

#[test]
fn test_withdraw_negative_shares_fails() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &100_000);

    let result = f.vault.try_withdraw(&f.alice, &-500);
    assert_eq!(result, Err(Ok(VaultError::ZeroAmount)));
}

#[test]
fn test_transfer_admin_requires_admin_auth() {
    let f = VaultFixture::new();
    f.vault.transfer_admin(&f.bob);
    assert_eq!(f.env.auths()[0].0, f.admin);
}

#[test]
fn test_pause_requires_admin_auth() {
    let f = VaultFixture::new();
    f.vault.pause();
    assert_eq!(f.env.auths()[0].0, f.admin);
}

#[test]
fn test_unpause_requires_admin_auth() {
    let f = VaultFixture::new();
    f.vault.unpause();
    assert_eq!(f.env.auths()[0].0, f.admin);
}

#[test]
fn test_get_withdrawal_limit_before_init_fails() {
    let env = Env::default();
    let vault_id = env.register_contract(None, VaultContract);
    let vault = VaultContractClient::new(&env, &vault_id);
    let result = vault.try_get_withdrawal_limit();
    assert_eq!(result, Err(Ok(VaultError::NotInitialized)));
}

// ── lock-up period and early-unstake penalty tests ───────────────────────────

#[test]
fn test_set_lock_period_requires_admin_auth() {
    let f = VaultFixture::new();
    f.vault.set_lock_period(&100);
    assert_eq!(f.env.auths()[0].0, f.admin);
}

#[test]
fn test_set_early_exit_penalty_bps_requires_admin_auth() {
    let f = VaultFixture::new();
    f.vault.set_early_exit_penalty_bps(&500);
    assert_eq!(f.env.auths()[0].0, f.admin);
}

#[test]
fn test_set_early_exit_penalty_bps_exceeds_max_fails() {
    let f = VaultFixture::new();
    // 2001 BPS should fail
    let result = f.vault.try_set_early_exit_penalty_bps(&2001);
    assert_eq!(result, Err(Ok(VaultError::InvalidPenaltyBps)));
}

#[test]
fn test_lock_config_query() {
    let f = VaultFixture::new();
    // Default config
    let (lock_period, penalty_bps) = f.vault.get_lock_config();
    assert_eq!(lock_period, 0);
    assert_eq!(penalty_bps, 0);

    // Set new config
    f.vault.set_lock_period(&100);
    f.vault.set_early_exit_penalty_bps(&1500);

    let (lock_period, penalty_bps) = f.vault.get_lock_config();
    assert_eq!(lock_period, 100);
    assert_eq!(penalty_bps, 1500);
}

// ── governance vote weight snapshots (Issue #31) ─────────────────────────────

#[test]
fn test_vote_weight_tracks_stake_history() {
    let f = VaultFixture::new();

    assert_eq!(f.vault.vote_weight_at(&f.alice, &0), 0);

    set_ledger(&f.env, 1);
    f.vault.stake(&f.alice, &500_000);
    assert_eq!(f.vault.current_vote_weight(&f.alice), 500_000);
    assert_eq!(f.vault.total_vote_weight(), 500_000);
    assert_eq!(f.vault.vote_weight_at(&f.alice, &1), 500_000);

    set_ledger(&f.env, 2);
    f.vault.unstake(&f.alice, &200_000);

    assert_eq!(f.vault.current_vote_weight(&f.alice), 300_000);
    assert_eq!(f.vault.total_vote_weight(), 300_000);
    assert_eq!(f.vault.vote_weight_at(&f.alice, &1), 500_000);
    assert_eq!(f.vault.vote_weight_at(&f.alice, &2), 300_000);
}

#[test]
fn test_vote_weight_history_is_capped_at_100_snapshots() {
    let f = VaultFixture::new();

    for ledger in 1..=105 {
        set_ledger(&f.env, ledger);
        f.vault.stake(&f.alice, &1);
    }

    assert_eq!(f.vault.current_vote_weight(&f.alice), 105);
    assert_eq!(f.vault.vote_weight_at(&f.alice, &1), 0);
    assert_eq!(f.vault.vote_weight_at(&f.alice, &5), 0);
    assert_eq!(f.vault.vote_weight_at(&f.alice, &6), 6);
    assert_eq!(f.vault.vote_weight_at(&f.alice, &105), 105);
}

// ── minimum stake (Issue #35) ─────────────────────────────────────────────────

#[test]
fn test_stake_exactly_at_minimum_succeeds() {
    let f = VaultFixture::new();
    f.vault.set_min_stake(&100_000);

    assert_eq!(f.vault.get_min_stake(), 100_000);
    assert_eq!(f.vault.stake(&f.alice, &100_000), 100_000);
}

#[test]
fn test_stake_below_minimum_fails() {
    let f = VaultFixture::new();
    f.vault.set_min_stake(&100_000);

    let result = f.vault.try_stake(&f.alice, &99_999);
    assert_eq!(result, Err(Ok(VaultError::BelowMinimumStake)));
}

#[test]
fn test_minimum_stake_can_be_disabled() {
    let f = VaultFixture::new();
    f.vault.set_min_stake(&100_000);
    f.vault.set_min_stake(&0);

    assert_eq!(f.vault.get_min_stake(), 0);
    assert_eq!(f.vault.stake(&f.alice, &1), 1);
}

#[test]
fn test_top_up_below_minimum_must_reach_threshold() {
    let f = VaultFixture::new();

    f.vault.set_min_stake(&0);
    f.vault.stake(&f.alice, &40_000);

    f.vault.set_min_stake(&100_000);
    let result = f.vault.try_stake(&f.alice, &50_000);
    assert_eq!(result, Err(Ok(VaultError::BelowMinimumStake)));

    assert_eq!(f.vault.stake(&f.alice, &60_000), 60_000);
    assert_eq!(f.vault.current_vote_weight(&f.alice), 100_000);
}

#[test]
fn test_admin_can_update_minimum_stake() {
    let f = VaultFixture::new();

    f.vault.set_min_stake(&100_000);
    assert_eq!(f.vault.get_min_stake(), 100_000);

    f.vault.set_min_stake(&50_000);
    assert_eq!(f.vault.get_min_stake(), 50_000);
}

// ── reward boost schedule (Issue #36) ─────────────────────────────────────────

#[test]
fn test_no_boost_schedule_means_base_multiplier_only() {
    let f = VaultFixture::new();
    let annual_stake = STELLAR_LEDGERS_PER_YEAR as i128;

    f.vault.set_reward_rate_bps(&BOOST_BPS_BASE);
    f.vault.stake(&f.alice, &annual_stake);

    set_ledger(&f.env, 20);
    assert_eq!(f.vault.get_boost_multiplier(&f.alice), BOOST_BPS_BASE);
    assert_eq!(f.vault.calc_pending_reward(&f.alice), 20);
}

#[test]
fn test_boost_schedule_round_trips_and_applies_by_tier() {
    let f = VaultFixture::new();
    let annual_stake = STELLAR_LEDGERS_PER_YEAR as i128;
    let schedule = boost_schedule(&f.env, &[(10, 11_000), (20, 12_500)]);

    f.vault.set_reward_rate_bps(&BOOST_BPS_BASE);
    f.vault.set_boost_schedule(&schedule);
    f.vault.stake(&f.alice, &annual_stake);

    let configured = f.vault.get_boost_schedule();
    assert_eq!(configured.len(), 2);
    assert_eq!(configured.get(0), Some((10, 11_000)));
    assert_eq!(configured.get(1), Some((20, 12_500)));

    set_ledger(&f.env, 9);
    assert_eq!(f.vault.get_boost_multiplier(&f.alice), BOOST_BPS_BASE);

    set_ledger(&f.env, 10);
    assert_eq!(f.vault.get_boost_multiplier(&f.alice), 11_000);

    set_ledger(&f.env, 20);
    assert_eq!(f.vault.get_boost_multiplier(&f.alice), 12_500);

    set_ledger(&f.env, 28);
    assert_eq!(f.vault.calc_pending_reward(&f.alice), 31);
}

#[test]
fn test_claim_does_not_reset_boost_tier() {
    let f = VaultFixture::new();
    let annual_stake = STELLAR_LEDGERS_PER_YEAR as i128;
    let schedule = boost_schedule(&f.env, &[(10, 11_000)]);

    f.token_admin.mint(&f.admin, &(annual_stake * 2));
    f.vault.set_reward_rate_bps(&BOOST_BPS_BASE);
    f.vault.set_boost_schedule(&schedule);
    f.vault.fund_reward_pool(&f.admin, &(annual_stake * 2));
    f.vault.stake(&f.alice, &annual_stake);

    set_ledger(&f.env, 20);
    assert_eq!(f.vault.claim(&f.alice), 21);
    assert_eq!(f.vault.get_boost_multiplier(&f.alice), 11_000);

    set_ledger(&f.env, 30);
    assert_eq!(f.vault.calc_pending_reward(&f.alice), 11);
}

#[test]
fn test_reward_checkpoint_on_top_up_avoids_overpaying() {
    let f = VaultFixture::new();
    let annual_stake = STELLAR_LEDGERS_PER_YEAR as i128;

    f.vault.set_reward_rate_bps(&BOOST_BPS_BASE);
    f.vault.stake(&f.alice, &annual_stake);

    set_ledger(&f.env, 100);
    f.vault.stake(&f.alice, &annual_stake);

    set_ledger(&f.env, 200);
    assert_eq!(f.vault.calc_pending_reward(&f.alice), 300);
}

// ── Issue #39: rescue_token ───────────────────────────────────────────────────

#[test]
fn test_rescue_third_token_succeeds() {
    let f = VaultFixture::new();

    // Create a third token (neither stake nor reward)
    let third_token_addr = f.env.register_stellar_asset_contract(f.admin.clone());
    let third_token_admin = token::StellarAssetClient::new(&f.env, &third_token_addr);
    let third_token = token::Client::new(&f.env, &third_token_addr);

    // Simulate a user accidentally sending the third token to the vault
    let vault_id = f.vault.address.clone();
    third_token_admin.mint(&vault_id, &5_000);

    assert_eq!(third_token.balance(&vault_id), 5_000);
    assert_eq!(third_token.balance(&f.alice), 0);

    // Admin rescues those tokens
    f.vault.rescue_token(&f.admin, &third_token_addr, &5_000, &f.alice);

    assert_eq!(third_token.balance(&vault_id), 0);
    assert_eq!(third_token.balance(&f.alice), 5_000);
}

#[test]
fn test_rescue_stake_token_fails() {
    let f = VaultFixture::new();
    let stake_token_addr = f.token.address.clone();

    // Alice stakes so the vault holds some stake tokens
    f.vault.stake(&f.alice, &100_000);

    let result = f.vault.try_rescue_token(&f.admin, &stake_token_addr, &100_000, &f.bob);
    assert_eq!(result, Err(Ok(VaultError::CannotRescueStakeToken)));
}

#[test]
fn test_rescue_reward_token_fails() {
    let f = VaultFixture::new();

    // Register a separate reward token address
    let reward_token_addr = f.env.register_stellar_asset_contract(f.admin.clone());
    let reward_token_admin = token::StellarAssetClient::new(&f.env, &reward_token_addr);
    f.vault.set_reward_token(&reward_token_addr);

    // Simulate some reward tokens ending up in the vault
    let vault_id = f.vault.address.clone();
    reward_token_admin.mint(&vault_id, &1_000);

    let result = f.vault.try_rescue_token(&f.admin, &reward_token_addr, &1_000, &f.bob);
    assert_eq!(result, Err(Ok(VaultError::CannotRescueRewardToken)));
}

#[test]
fn test_rescue_token_requires_admin_auth() {
    let f = VaultFixture::new();
    let third_token_addr = f.env.register_stellar_asset_contract(f.admin.clone());
    let third_token_admin = token::StellarAssetClient::new(&f.env, &third_token_addr);
    let vault_id = f.vault.address.clone();
    third_token_admin.mint(&vault_id, &1_000);

    f.vault.rescue_token(&f.admin, &third_token_addr, &1_000, &f.alice);
    // Verify admin auth was required (first recorded auth is the admin's)
    assert_eq!(f.env.auths()[0].0, f.admin);
}

#[test]
fn test_rescue_token_emits_token_rescued_event() {
    let f = VaultFixture::new();
    let third_token_addr = f.env.register_stellar_asset_contract(f.admin.clone());
    let third_token_admin = token::StellarAssetClient::new(&f.env, &third_token_addr);
    let vault_id = f.vault.address.clone();
    third_token_admin.mint(&vault_id, &2_000);

    f.vault.rescue_token(&f.admin, &third_token_addr, &2_000, &f.alice);

    let events = f.env.events().all();
    let rescue_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "tk_rescue"))
        .collect();
    assert_eq!(rescue_events.len(), 1);
}

// ── Issue #40: NFT receipt on stake ──────────────────────────────────────────

fn setup_nft<'a>(f: &'a VaultFixture<'a>) -> (Address, StakeReceiptNFTClient<'a>) {
    let nft_id = f.env.register_contract(None, StakeReceiptNFT);
    let nft = StakeReceiptNFTClient::new(&f.env, &nft_id);
    // The vault will be the minter
    nft.initialize(&f.vault.address);
    f.vault.set_nft_contract(&nft_id);
    (nft_id, nft)
}

#[test]
fn test_stake_mints_nft() {
    let f = VaultFixture::new();
    let (_nft_id, nft) = setup_nft(&f);

    assert!(!nft.has_receipt(&f.alice));
    f.vault.stake(&f.alice, &100_000);
    assert!(nft.has_receipt(&f.alice));
}

#[test]
fn test_full_unstake_burns_nft() {
    let f = VaultFixture::new();
    let (_nft_id, nft) = setup_nft(&f);

    f.vault.stake(&f.alice, &100_000);
    assert!(nft.has_receipt(&f.alice));

    f.vault.unstake(&f.alice, &100_000);
    assert!(!nft.has_receipt(&f.alice));
}

#[test]
fn test_partial_unstake_keeps_nft() {
    let f = VaultFixture::new();
    let (_nft_id, nft) = setup_nft(&f);

    f.vault.stake(&f.alice, &100_000);
    f.vault.unstake(&f.alice, &50_000); // partial — receipt should remain
    assert!(nft.has_receipt(&f.alice));

    f.vault.unstake(&f.alice, &50_000); // full — receipt should be burned
    assert!(!nft.has_receipt(&f.alice));
}

#[test]
fn test_nft_transfer_always_reverts() {
    use crate::nft::NftError;

    let f = VaultFixture::new();
    let (_nft_id, nft) = setup_nft(&f);

    f.vault.stake(&f.alice, &100_000);
    assert!(nft.has_receipt(&f.alice));

    let result = nft.try_transfer(&f.alice, &f.bob);
    assert_eq!(result, Err(Ok(NftError::NonTransferable)));
    // Receipt is still there
    assert!(nft.has_receipt(&f.alice));
}

// ── Issue #41: restake grace window ──────────────────────────────────────────

#[test]
fn test_restake_minimal_no_lock() {
    // Basic: set window, stake, full unstake, re-stake within window
    let f = VaultFixture::new();
    f.vault.set_restake_window(&100);
    f.vault.stake(&f.alice, &100_000);
    f.vault.unstake(&f.alice, &100_000);
    // At ledger 0, last_unstake = 0, current = 0, diff = 0 ≤ 100 → Restaked = true
    f.vault.stake(&f.alice, &100_000);
    f.vault.unstake(&f.alice, &100_000);
}

#[test]
fn test_restake_with_lock_no_penalty_after_expiry() {
    let f = VaultFixture::new();
    f.vault.set_lock_period(&100);
    f.vault.set_early_exit_penalty_bps(&1000);
    // NOTE: no set_restake_window here
    f.vault.stake(&f.alice, &500_000);
    // Unstake AFTER lock period → no penalty
    set_ledger(&f.env, 100);
    let first_return = f.vault.unstake(&f.alice, &500_000);
    assert_eq!(first_return, 500_000);
}

#[test]
fn test_restake_debug_set_window_then_stake_ledger() {
    let f = VaultFixture::new();
    f.vault.set_restake_window(&200);
    f.vault.stake(&f.alice, &500_000);
    set_ledger(&f.env, 100);
    let ret = f.vault.unstake(&f.alice, &500_000);
    assert_eq!(ret, 500_000);
}

#[test]
fn test_restake_debug_lock_period_only() {
    let f = VaultFixture::new();
    f.vault.set_lock_period(&100);  // only this
    f.vault.stake(&f.alice, &500_000);
    set_ledger(&f.env, 100);
    let ret = f.vault.unstake(&f.alice, &500_000);
    assert_eq!(ret, 500_000);
}

#[test]
fn test_restake_debug_a_penalty_call_only() {
    // Does calling set_early_exit_penalty_bps alone panic?
    let f = VaultFixture::new();
    let _ = f.vault.try_set_early_exit_penalty_bps(&1000);
}

#[test]
fn test_restake_debug_b_penalty_and_stake() {
    let f = VaultFixture::new();
    f.vault.set_early_exit_penalty_bps(&1000);
    f.vault.stake(&f.alice, &500_000);
}

#[test]
fn test_restake_debug_c_penalty_stake_unstake_no_ledger() {
    let f = VaultFixture::new();
    f.vault.set_early_exit_penalty_bps(&1000);
    f.vault.stake(&f.alice, &500_000);
    let result = f.vault.try_unstake(&f.alice, &500_000);
    // If it errors instead of panicking, we can see the error
    assert!(result.is_ok(), "Unstake failed: {:?}", result);
}

#[test]
fn test_restake_debug_d_penalty_stake_ledger_unstake() {
    let f = VaultFixture::new();
    f.vault.set_early_exit_penalty_bps(&1000);
    f.vault.stake(&f.alice, &500_000);
    set_ledger(&f.env, 100);
    f.vault.unstake(&f.alice, &500_000);
}

#[test]
fn test_restake_debug_e_reward_rate_stake_unstake() {
    // Does set_reward_rate_bps (another instance storage write) cause the same panic?
    let f = VaultFixture::new();
    f.vault.set_reward_rate_bps(&500);
    f.vault.stake(&f.alice, &500_000);
    f.vault.unstake(&f.alice, &500_000);
}

#[test]
fn test_restake_debug_f_withdrawal_limit_stake_unstake() {
    // Does set_withdrawal_limit (another instance storage write) cause the same panic?
    let f = VaultFixture::new();
    f.vault.set_withdrawal_limit(&2_000_000);
    f.vault.stake(&f.alice, &500_000);
    f.vault.unstake(&f.alice, &500_000);
}

#[test]
fn test_restake_within_window_is_penalty_free() {
    let f = VaultFixture::new();

    // Lock period 100, 10% early-exit penalty, 200-ledger restake window.
    f.vault.set_lock_period(&100);
    f.vault.set_early_exit_penalty_bps(&1000);
    f.vault.set_restake_window(&200);

    // Alice stakes at ledger 0.
    f.vault.stake(&f.alice, &500_000);

    // Alice unstakes AFTER the lock period expires (no penalty, no residual in vault).
    set_ledger(&f.env, 100);
    let first_return = f.vault.unstake(&f.alice, &500_000);
    assert_eq!(first_return, 500_000, "No penalty after lock expires");
    // LastUnstakeLedger = 100; vault is now empty.

    // Alice re-stakes 50 ledgers later — within the 200-ledger window → Restaked = true.
    set_ledger(&f.env, 150);
    f.vault.stake(&f.alice, &500_000);

    // Alice tries to exit at ledger 200 (50 after re-stake, still inside the new 100-ledger lock).
    // Normally 10% penalty; Restaked flag exempts her.
    set_ledger(&f.env, 200);
    let returned = f.vault.unstake(&f.alice, &500_000);
    assert_eq!(returned, 500_000, "Restaked user should receive full amount, no penalty");
}

#[test]
fn test_restake_outside_window_incurs_normal_penalty() {
    let f = VaultFixture::new();

    // Lock 100 ledgers, 10% penalty, but only a 10-ledger restake window.
    f.vault.set_lock_period(&100);
    f.vault.set_early_exit_penalty_bps(&1000);
    f.vault.set_restake_window(&10);

    f.vault.stake(&f.alice, &500_000);

    // Clean unstake after lock period.
    set_ledger(&f.env, 100);
    f.vault.unstake(&f.alice, &500_000);

    // Re-stake 50 ledgers later — OUTSIDE the 10-ledger window → Restaked NOT set.
    set_ledger(&f.env, 150);
    f.vault.stake(&f.alice, &500_000);

    // Early exit inside the new lock period — normal penalty applies.
    set_ledger(&f.env, 200);
    let returned = f.vault.unstake(&f.alice, &500_000);
    let penalty = 500_000_i128 * 1000 / 10_000;
    assert_eq!(returned, 500_000 - penalty, "Outside window: normal penalty applies");
}

#[test]
fn test_restake_window_zero_disables_feature() {
    let f = VaultFixture::new();

    // Lock 100 ledgers, 10% penalty, window disabled.
    f.vault.set_lock_period(&100);
    f.vault.set_early_exit_penalty_bps(&1000);
    f.vault.set_restake_window(&0);

    f.vault.stake(&f.alice, &500_000);

    // Clean unstake after lock period.
    set_ledger(&f.env, 100);
    f.vault.unstake(&f.alice, &500_000);

    // Re-stake 1 ledger later — window = 0 means Restaked is never set.
    set_ledger(&f.env, 101);
    f.vault.stake(&f.alice, &500_000);

    // Early exit inside lock period — penalty must apply since window = 0.
    set_ledger(&f.env, 150);
    let returned = f.vault.unstake(&f.alice, &500_000);
    let penalty = 500_000_i128 * 1000 / 10_000;
    assert_eq!(returned, 500_000 - penalty, "Window=0: normal penalty must apply");
}

// ── Issue #42: admin action audit log ────────────────────────────────────────

#[test]
fn test_admin_action_count_increments() {
    let f = VaultFixture::new();

    let before = f.vault.get_admin_action_count();
    f.vault.set_reward_rate_bps(&500);
    let after = f.vault.get_admin_action_count();
    assert_eq!(after, before + 1, "Count should increment after each admin action");

    f.vault.pause();
    assert_eq!(f.vault.get_admin_action_count(), before + 2);

    f.vault.unpause();
    assert_eq!(f.vault.get_admin_action_count(), before + 3);
}

#[test]
fn test_admin_action_set_reward_rate_emits_audit_event() {
    let f = VaultFixture::new();
    f.vault.set_reward_rate_bps(&1000);

    let events = f.env.events().all();
    let audit_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "adm_act"))
        .collect();
    assert!(!audit_events.is_empty(), "adm_act event should be emitted");
}

#[test]
fn test_admin_action_pause_emits_audit_event() {
    let f = VaultFixture::new();
    f.vault.pause();

    let events = f.env.events().all();
    let audit_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "adm_act"))
        .collect();
    assert!(!audit_events.is_empty(), "adm_act event should be emitted on pause");
}

#[test]
fn test_admin_action_transfer_admin_emits_audit_event() {
    let f = VaultFixture::new();
    f.vault.transfer_admin(&f.bob);

    let events = f.env.events().all();
    let audit_events: std::vec::Vec<_> = events
        .into_iter()
        .filter(|(_, topics, _)| topic_matches(&f.env, topics, "adm_act"))
        .collect();
    assert!(!audit_events.is_empty(), "adm_act event should be emitted on transfer_admin");
}

#[test]
fn test_admin_action_count_increments_across_all_admin_fns() {
    let f = VaultFixture::new();
    let mut expected = 0u32;

    f.vault.set_reward_rate_bps(&500);
    expected += 1;
    assert_eq!(f.vault.get_admin_action_count(), expected);

    f.vault.pause();
    expected += 1;
    assert_eq!(f.vault.get_admin_action_count(), expected);

    f.vault.unpause();
    expected += 1;
    assert_eq!(f.vault.get_admin_action_count(), expected);

    f.vault.set_lock_period(&100);
    expected += 1;
    assert_eq!(f.vault.get_admin_action_count(), expected);

    f.vault.set_withdrawal_limit(&1_000_000);
    expected += 1;
    assert_eq!(f.vault.get_admin_action_count(), expected);

    f.vault.transfer_admin(&f.bob);
    expected += 1;
    assert_eq!(f.vault.get_admin_action_count(), expected);
}
