#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events},
    token, Address, Env, IntoVal, Symbol,
};

use crate::{errors::VaultError, vault::VaultContract, vault::VaultContractClient};

// ── helpers ──────────────────────────────────────────────────────────────────

fn create_token<'a>(
    env: &Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_id = env.register_stellar_asset_contract_v2(admin.clone());
    let address = contract_id.address();
    let client = token::Client::new(env, &address);
    let admin_client = token::StellarAssetClient::new(env, &address);
    (address, client, admin_client)
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
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        let (token_addr, token, token_admin) = create_token(&env, &admin);

        let vault_id = env.register(VaultContract, ());
        let vault = VaultContractClient::new(&env, &vault_id);

        vault.initialize(&admin, &token_addr);

        // Mint starting balances
        token_admin.mint(&alice, &1_000_000);
        token_admin.mint(&bob, &1_000_000);

        VaultFixture { env, vault, token, token_admin, admin, alice, bob }
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
    let token_addr: soroban_sdk::Address = f.env.register_stellar_asset_contract_v2(
        Address::generate(&f.env)
    ).address();
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
fn test_add_yield_unauthorized_fails() {
    let f = VaultFixture::new();
    f.token_admin.mint(&f.admin, &10_000);

    let result = f.vault.try_add_yield(&f.alice, &10_000);
    assert_eq!(result, Err(Ok(VaultError::Unauthorized)));
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
fn test_set_withdrawal_limit_unauthorized_fails() {
    let f = VaultFixture::new();
    let result = f.vault.try_set_withdrawal_limit_as(&f.alice, &100_000);
    assert_eq!(result, Err(Ok(VaultError::Unauthorized)));
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
    let deposit_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.first() == Some(&Symbol::new(&f.env, "deposit"))
        })
        .collect();

    assert_eq!(deposit_events.len(), 1);
    let event = &deposit_events[0];
    assert_eq!(event.topics.get(1), Some(f.alice.clone().into_val(&f.env)));
}

#[test]
fn test_withdraw_emits_event() {
    let f = VaultFixture::new();
    f.vault.deposit(&f.alice, &100_000);

    f.vault.withdraw(&f.alice, &50_000);

    let events = f.env.events().all();
    let withdraw_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.first() == Some(&Symbol::new(&f.env, "withdraw"))
        })
        .collect();

    assert_eq!(withdraw_events.len(), 1);
    let event = &withdraw_events[0];
    assert_eq!(event.topics.get(1), Some(f.alice.clone().into_val(&f.env)));
}

#[test]
fn test_pause_emits_event() {
    let f = VaultFixture::new();

    f.vault.pause();

    let events = f.env.events().all();
    let paused_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.first() == Some(&Symbol::new(&f.env, "paused"))
        })
        .collect();

    assert_eq!(paused_events.len(), 1);
}

#[test]
fn test_unpause_emits_event() {
    let f = VaultFixture::new();
    f.vault.pause();

    f.vault.unpause();

    let events = f.env.events().all();
    let unpaused_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.first() == Some(&Symbol::new(&f.env, "unpaused"))
        })
        .collect();

    assert_eq!(unpaused_events.len(), 1);
}

#[test]
fn test_transfer_admin_emits_event() {
    let f = VaultFixture::new();

    f.vault.transfer_admin(&f.bob);

    let events = f.env.events().all();
    let admin_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.first() == Some(&Symbol::new(&f.env, "admin_set"))
        })
        .collect();

    assert_eq!(admin_events.len(), 1);
    let event = &admin_events[0];
    assert_eq!(event.topics.get(1), Some(f.admin.clone().into_val(&f.env)));
}

#[test]
fn test_withdrawal_limit_update_emits_event() {
    let f = VaultFixture::new();

    f.vault.set_withdrawal_limit(&100_000);

    let events = f.env.events().all();
    let limit_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.first() == Some(&Symbol::new(&f.env, "wd_limit"))
        })
        .collect();

    assert_eq!(limit_events.len(), 1);
}

#[test]
fn test_yield_added_emits_event() {
    let f = VaultFixture::new();
    f.token_admin.mint(&f.admin, &50_000);

    f.vault.add_yield(&f.admin, &50_000);

    let events = f.env.events().all();
    let yield_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.topics.first() == Some(&Symbol::new(&f.env, "yield_add"))
        })
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
fn test_transfer_admin_unauthorized_fails() {
    let f = VaultFixture::new();
    let result = f.vault.try_transfer_admin_as(&f.alice, &f.bob);
    assert_eq!(result, Err(Ok(VaultError::Unauthorized)));
}

#[test]
fn test_pause_unauthorized_fails() {
    let f = VaultFixture::new();
    let result = f.vault.try_pause_as(&f.alice);
    assert_eq!(result, Err(Ok(VaultError::Unauthorized)));
}

#[test]
fn test_unpause_unauthorized_fails() {
    let f = VaultFixture::new();
    let result = f.vault.try_unpause_as(&f.alice);
    assert_eq!(result, Err(Ok(VaultError::Unauthorized)));
}

#[test]
fn test_get_withdrawal_limit_before_init_fails() {
    let env = Env::default();
    let vault_id = env.register(VaultContract, ());
    let vault = VaultContractClient::new(&env, &vault_id);
    let result = vault.try_get_withdrawal_limit();
    assert_eq!(result, Err(Ok(VaultError::NotInitialized)));
}

// ── property-based invariant tests for vault accounting ─────────────────────

/// Extended fixture with 3 users for invariant testing.
struct InvariantFixture<'a> {
    env: Env,
    vault: VaultContractClient<'a>,
    token: token::Client<'a>,
    _token_admin: token::StellarAssetClient<'a>,
    admin: Address,
    users: [Address; 3],
}

impl<'a> InvariantFixture<'a> {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let users = [
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ];

        let (token_addr, token, token_admin) = create_token(&env, &admin);

        let vault_id = env.register(VaultContract, ());
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &token_addr);

        // Mint generous starting balances to all users and admin
        for user in &users {
            token_admin.mint(user, &10_000_000);
        }
        token_admin.mint(&admin, &10_000_000);

        InvariantFixture { env, vault, token, _token_admin: token_admin, admin, users }
    }

    /// Assert all core accounting invariants hold.
    ///
    /// Called after every operation in the deterministic sequence.
    fn assert_invariants(&self, step: &str) {
        let (total_shares, total_deposited) = self.vault.vault_state();

        // INV-1  total_shares == Σ shares_of(user)
        //        (maps to: total_staked == sum of individual positions)
        let sum_shares: i128 = self.users.iter()
            .map(|u| self.vault.shares_of(u))
            .sum();
        assert_eq!(
            total_shares, sum_shares,
            "[{}] INV-1: total_shares ({}) != sum of individual shares ({})",
            step, total_shares, sum_shares,
        );

        // INV-2  vault token balance == total_deposited
        //        (token conservation – no tokens appear or vanish)
        let vault_balance = self.token.balance(&self.vault.address);
        assert_eq!(
            vault_balance, total_deposited,
            "[{}] INV-2: vault token balance ({}) != total_deposited ({})",
            step, vault_balance, total_deposited,
        );

        // INV-3  preview_redeem(shares) >= 0 for every user
        //        (maps to: pending_reward is always >= 0)
        for (i, user) in self.users.iter().enumerate() {
            let shares = self.vault.shares_of(user);
            if shares > 0 {
                let redeemable = self.vault.preview_redeem(&shares);
                assert!(
                    redeemable >= 0,
                    "[{}] INV-3: user[{}] preview_redeem ({}) < 0",
                    step, i, redeemable,
                );
            }
        }

        // INV-4  no individual share balance is negative
        for (i, user) in self.users.iter().enumerate() {
            let shares = self.vault.shares_of(user);
            assert!(
                shares >= 0,
                "[{}] INV-4: user[{}] has negative shares ({})",
                step, i, shares,
            );
        }
    }
}

/// Deterministic 16-step sequence across 3 users exercising deposit, withdraw
/// and add_yield.  All five invariants from the acceptance criteria are checked
/// after every single step.
#[test]
fn test_accounting_invariants_under_deterministic_sequence() {
    let f = InvariantFixture::setup();
    f.assert_invariants("init");

    // ── deposits ──────────────────────────────────────────────────────────

    // Step 1: User 0 deposits 500k (first deposit → 1:1 shares)
    f.vault.deposit(&f.users[0], &500_000);
    f.assert_invariants("step-01 deposit(u0, 500k)");

    // Step 2: User 1 deposits 300k
    f.vault.deposit(&f.users[1], &300_000);
    f.assert_invariants("step-02 deposit(u1, 300k)");

    // Step 3: User 2 deposits 200k
    f.vault.deposit(&f.users[2], &200_000);
    f.assert_invariants("step-03 deposit(u2, 200k)");

    // ── yield injection ───────────────────────────────────────────────────

    // Step 4: Admin adds 100k yield (share price rises)
    f.vault.add_yield(&f.admin, &100_000);
    f.assert_invariants("step-04 add_yield(100k)");

    // ── partial withdrawals ───────────────────────────────────────────────

    // Step 5: User 0 withdraws 100k shares — verify token-balance delta
    let pre_bal_5 = f.token.balance(&f.vault.address);
    let returned_5 = f.vault.withdraw(&f.users[0], &100_000);
    let post_bal_5 = f.token.balance(&f.vault.address);
    assert_eq!(
        pre_bal_5 - post_bal_5, returned_5,
        "step-05: vault balance delta != amount returned"
    );
    f.assert_invariants("step-05 withdraw(u0, 100k shares)");

    // Step 6: User 1 deposits 200k more
    f.vault.deposit(&f.users[1], &200_000);
    f.assert_invariants("step-06 deposit(u1, 200k)");

    // Step 7: User 2 full withdrawal — verify position zeroed
    let u2_shares = f.vault.shares_of(&f.users[2]);
    let pre_bal_7 = f.token.balance(&f.vault.address);
    let returned_7 = f.vault.withdraw(&f.users[2], &u2_shares);
    let post_bal_7 = f.token.balance(&f.vault.address);
    assert_eq!(
        pre_bal_7 - post_bal_7, returned_7,
        "step-07: vault balance delta != amount returned"
    );
    assert_eq!(
        f.vault.shares_of(&f.users[2]), 0,
        "step-07: full unstake did not zero position"
    );
    f.assert_invariants("step-07 full-withdraw(u2)");

    // ── more yield + mixed ops ────────────────────────────────────────────

    // Step 8: Admin adds 50k yield
    f.vault.add_yield(&f.admin, &50_000);
    f.assert_invariants("step-08 add_yield(50k)");

    // Step 9: User 0 deposits 150k
    f.vault.deposit(&f.users[0], &150_000);
    f.assert_invariants("step-09 deposit(u0, 150k)");

    // Step 10: User 1 partial withdraw 100k shares
    let pre_bal_10 = f.token.balance(&f.vault.address);
    let returned_10 = f.vault.withdraw(&f.users[1], &100_000);
    let post_bal_10 = f.token.balance(&f.vault.address);
    assert_eq!(
        pre_bal_10 - post_bal_10, returned_10,
        "step-10: vault balance delta != amount returned"
    );
    f.assert_invariants("step-10 withdraw(u1, 100k shares)");

    // Step 11: User 2 re-enters with 400k deposit
    f.vault.deposit(&f.users[2], &400_000);
    f.assert_invariants("step-11 deposit(u2, 400k)");

    // Step 12: User 0 partial withdraw 50k shares
    let pre_bal_12 = f.token.balance(&f.vault.address);
    let returned_12 = f.vault.withdraw(&f.users[0], &50_000);
    let post_bal_12 = f.token.balance(&f.vault.address);
    assert_eq!(
        pre_bal_12 - post_bal_12, returned_12,
        "step-12: vault balance delta != amount returned"
    );
    f.assert_invariants("step-12 withdraw(u0, 50k shares)");

    // Step 13: Admin adds 75k yield
    f.vault.add_yield(&f.admin, &75_000);
    f.assert_invariants("step-13 add_yield(75k)");

    // ── full exits ────────────────────────────────────────────────────────

    // Step 14: User 0 full exit
    let u0_shares = f.vault.shares_of(&f.users[0]);
    f.vault.withdraw(&f.users[0], &u0_shares);
    assert_eq!(f.vault.shares_of(&f.users[0]), 0, "step-14: u0 position not zeroed");
    f.assert_invariants("step-14 full-exit(u0)");

    // Step 15: User 1 full exit
    let u1_shares = f.vault.shares_of(&f.users[1]);
    f.vault.withdraw(&f.users[1], &u1_shares);
    assert_eq!(f.vault.shares_of(&f.users[1]), 0, "step-15: u1 position not zeroed");
    f.assert_invariants("step-15 full-exit(u1)");

    // Step 16: User 2 full exit
    let u2_shares_final = f.vault.shares_of(&f.users[2]);
    f.vault.withdraw(&f.users[2], &u2_shares_final);
    assert_eq!(f.vault.shares_of(&f.users[2]), 0, "step-16: u2 position not zeroed");
    f.assert_invariants("step-16 full-exit(u2)");

    // After all users exit, vault should be empty
    let (final_shares, _final_deposited) = f.vault.vault_state();
    assert_eq!(final_shares, 0, "vault not empty after all exits");
}

/// Targeted: after every withdraw the vault's token balance decreases by
/// exactly the amount the contract returned.
#[test]
fn test_invariant_withdraw_decreases_balance_by_exact_amount() {
    let f = InvariantFixture::setup();
    f.vault.deposit(&f.users[0], &600_000);
    f.vault.deposit(&f.users[1], &400_000);
    f.vault.add_yield(&f.admin, &200_000);

    // Partial withdraw user 0
    let pre = f.token.balance(&f.vault.address);
    let amt = f.vault.withdraw(&f.users[0], &200_000);
    let post = f.token.balance(&f.vault.address);
    assert_eq!(pre - post, amt);

    // Partial withdraw user 1
    let pre = f.token.balance(&f.vault.address);
    let amt = f.vault.withdraw(&f.users[1], &100_000);
    let post = f.token.balance(&f.vault.address);
    assert_eq!(pre - post, amt);
}

/// Targeted: after a full withdraw shares_of(user) returns 0.
#[test]
fn test_invariant_full_withdraw_zeroes_position() {
    let f = InvariantFixture::setup();
    f.vault.deposit(&f.users[0], &500_000);
    f.vault.deposit(&f.users[1], &300_000);
    f.vault.add_yield(&f.admin, &100_000);

    // Full exit user 0
    let shares = f.vault.shares_of(&f.users[0]);
    assert!(shares > 0);
    f.vault.withdraw(&f.users[0], &shares);
    assert_eq!(f.vault.shares_of(&f.users[0]), 0);

    // Full exit user 1
    let shares = f.vault.shares_of(&f.users[1]);
    assert!(shares > 0);
    f.vault.withdraw(&f.users[1], &shares);
    assert_eq!(f.vault.shares_of(&f.users[1]), 0);
}

/// Targeted: preview_redeem is always >= 0 for any user at any point.
#[test]
fn test_invariant_preview_redeem_non_negative() {
    let f = InvariantFixture::setup();

    f.vault.deposit(&f.users[0], &500_000);
    f.vault.deposit(&f.users[1], &300_000);
    f.vault.deposit(&f.users[2], &200_000);

    // Check after deposits
    for user in &f.users {
        let s = f.vault.shares_of(user);
        if s > 0 {
            assert!(f.vault.preview_redeem(&s) >= 0);
        }
    }

    // Check after yield
    f.vault.add_yield(&f.admin, &100_000);
    for user in &f.users {
        let s = f.vault.shares_of(user);
        if s > 0 {
            assert!(f.vault.preview_redeem(&s) >= 0);
        }
    }

    // Check after partial withdrawals
    f.vault.withdraw(&f.users[0], &100_000);
    for user in &f.users {
        let s = f.vault.shares_of(user);
        if s > 0 {
            assert!(f.vault.preview_redeem(&s) >= 0);
        }
    }
}

/// Targeted: total_shares always equals sum of individual shares_of(user).
#[test]
fn test_invariant_total_shares_equals_sum_of_individual() {
    let f = InvariantFixture::setup();

    let ops: [(usize, i128); 6] = [
        (0, 500_000),
        (1, 300_000),
        (2, 200_000),
        (0, 100_000),
        (1, 250_000),
        (2, 150_000),
    ];

    // Deposits
    for (idx, amount) in &ops {
        f.vault.deposit(&f.users[*idx], amount);

        let (total_shares, _) = f.vault.vault_state();
        let sum: i128 = f.users.iter().map(|u| f.vault.shares_of(u)).sum();
        assert_eq!(total_shares, sum);
    }

    // Withdrawals
    for user in &f.users {
        let s = f.vault.shares_of(user);
        if s > 0 {
            f.vault.withdraw(user, &(s / 2));
            let (total_shares, _) = f.vault.vault_state();
            let sum: i128 = f.users.iter().map(|u| f.vault.shares_of(u)).sum();
            assert_eq!(total_shares, sum);
        }
    }
}
