#![cfg(test)]
//! Tests for the reward campaign window (`set_reward_window`).
//!
//! Boundary convention under test: the window is `[start, end)`. Start is
//! inclusive (accrual begins there), end is exclusive (nothing accrues from
//! the end ledger onward).

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};

use crate::{
    errors::RewardWindowError,
    vault::{VaultContract, VaultContractClient},
};

const STAKE: i128 = 100_000_000_000;
const STAKE_LEDGER: u32 = 1_000;
const RATE_BPS: u32 = 1_000;

struct Fixture<'a> {
    env: Env,
    vault: VaultContractClient<'a>,
    admin: Address,
    alice: Address,
}

fn set_ledger(env: &Env, sequence: u32) {
    env.ledger().with_mut(|ledger| ledger.sequence_number = sequence);
}

impl<'a> Fixture<'a> {
    /// A pool with alice staked at `STAKE_LEDGER` and no reward window.
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|ledger| {
            ledger.min_persistent_entry_ttl = 10_000_000;
            ledger.min_temp_entry_ttl = 10_000_000;
            ledger.max_entry_ttl = 10_000_000;
            ledger.sequence_number = STAKE_LEDGER;
        });

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract(admin.clone());
        token::StellarAssetClient::new(&env, &token_address).mint(&alice, &(STAKE * 10));

        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &token_address, &0_u32, &None, &None);
        vault.set_reward_rate_bps(&RATE_BPS);
        vault.stake(&alice, &STAKE, &0);

        Fixture { env, vault, admin, alice }
    }

    /// Fixture with the window `[start, end)` already set.
    fn with_window(start: u32, end: u32) -> Self {
        let f = Self::new();
        f.vault.set_reward_window(&f.admin, &start, &end);
        f
    }

    fn pending_at(&self, ledger: u32) -> i128 {
        set_ledger(&self.env, ledger);
        self.vault.calc_pending_reward(&self.alice)
    }
}

/// What `ledgers` ledgers of normal, un-windowed accrual are worth.
fn reward_for(ledgers: u32) -> i128 {
    Fixture::new().pending_at(STAKE_LEDGER + ledgers)
}

#[test]
fn no_window_by_default() {
    let f = Fixture::new();
    assert_eq!(f.vault.get_reward_window(), None);
}

#[test]
fn window_can_be_set_and_read_back() {
    let f = Fixture::with_window(2_000, 3_000);
    assert_eq!(f.vault.get_reward_window(), Some((2_000, 3_000)));
}

#[test]
fn nothing_accrues_before_the_start_ledger() {
    let f = Fixture::with_window(2_000, 3_000);
    assert_eq!(f.pending_at(1_500), 0);
    assert_eq!(f.pending_at(1_999), 0);
}

#[test]
fn start_ledger_is_inclusive() {
    let f = Fixture::with_window(2_000, 3_000);
    // At the start ledger itself nothing has accrued yet...
    assert_eq!(f.pending_at(2_000), 0);
    // ...and accrual is live from that ledger on.
    assert!(f.pending_at(2_001) > 0);
    assert_eq!(f.pending_at(2_500), reward_for(500));
}

#[test]
fn end_ledger_is_exclusive() {
    let f = Fixture::with_window(2_000, 3_000);
    let one_before = f.pending_at(2_999);
    let at_end = f.pending_at(3_000);

    assert_eq!(one_before, reward_for(999));
    // The window is exactly end - start = 1000 ledgers long.
    assert_eq!(at_end, reward_for(1_000));
    assert!(at_end > one_before);
}

#[test]
fn nothing_accrues_after_the_end_ledger() {
    let f = Fixture::with_window(2_000, 3_000);
    let at_end = f.pending_at(3_000);
    assert_eq!(f.pending_at(3_001), at_end);
    assert_eq!(f.pending_at(50_000), at_end);
}

#[test]
fn accrual_inside_the_window_is_normal() {
    let f = Fixture::with_window(2_000, 3_000);
    assert_eq!(f.pending_at(3_000), reward_for(1_000));
}

#[test]
fn a_stake_made_after_the_window_ends_earns_nothing() {
    let f = Fixture::with_window(1_500, 2_000);
    set_ledger(&f.env, 5_000);
    let bob = Address::generate(&f.env);
    let token_address = f.vault.get_stake_token();
    token::StellarAssetClient::new(&f.env, &token_address).mint(&bob, &STAKE);
    f.vault.stake(&bob, &STAKE, &0);

    set_ledger(&f.env, 9_000);
    assert_eq!(f.vault.calc_pending_reward(&bob), 0);
}

#[test]
fn moving_the_window_settles_accrual_up_to_the_change_first() {
    let f = Fixture::with_window(2_000, 4_000);

    // 1000 ledgers of the window have elapsed when the admin moves it to a
    // later campaign. Those 1000 ledgers must stay earned.
    set_ledger(&f.env, 3_000);
    f.vault.set_reward_window(&f.admin, &5_000, &6_000);
    assert_eq!(f.vault.calc_pending_reward(&f.alice), reward_for(1_000));

    // Nothing accrues in the gap before the new window...
    assert_eq!(f.pending_at(4_999), reward_for(1_000));
    // ...and the new window pays on top of what was already settled.
    assert_eq!(f.pending_at(6_000), reward_for(2_000));
    assert_eq!(f.pending_at(9_000), reward_for(2_000));
}

#[test]
fn shortening_the_window_mid_campaign_keeps_earned_rewards() {
    let f = Fixture::with_window(2_000, 5_000);

    set_ledger(&f.env, 3_000);
    f.vault.set_reward_window(&f.admin, &2_000, &3_500);

    assert_eq!(f.pending_at(3_500), reward_for(1_500));
    assert_eq!(f.pending_at(8_000), reward_for(1_500));
}

#[test]
fn setting_a_window_after_unwindowed_accrual_keeps_what_was_earned() {
    let f = Fixture::new();

    // 1000 ledgers accrue with no window, then a window opens later.
    set_ledger(&f.env, 2_000);
    f.vault.set_reward_window(&f.admin, &4_000, &5_000);

    assert_eq!(f.vault.calc_pending_reward(&f.alice), reward_for(1_000));
    assert_eq!(f.pending_at(3_500), reward_for(1_000));
    assert_eq!(f.pending_at(5_000), reward_for(2_000));
}

#[test]
fn clearing_the_window_restores_unbounded_accrual() {
    let f = Fixture::with_window(2_000, 3_000);

    set_ledger(&f.env, 4_000);
    f.vault.clear_reward_window(&f.admin);
    assert_eq!(f.vault.get_reward_window(), None);
    // The finished window's 1000 ledgers are kept...
    assert_eq!(f.vault.calc_pending_reward(&f.alice), reward_for(1_000));
    // ...and accrual runs again from the clear.
    assert_eq!(f.pending_at(5_000), reward_for(2_000));
}

#[test]
fn project_reward_respects_the_window() {
    let f = Fixture::with_window(2_000, 3_000);
    assert_eq!(f.vault.project_reward(&f.alice, &1_500), 0);
    assert_eq!(f.vault.project_reward(&f.alice, &2_500), reward_for(500));
    assert_eq!(f.vault.project_reward(&f.alice, &9_000), reward_for(1_000));
}

#[test]
fn window_must_end_after_it_starts() {
    let f = Fixture::new();
    assert_eq!(
        f.vault.try_set_reward_window(&f.admin, &3_000, &3_000),
        Err(Ok(RewardWindowError::InvalidWindow))
    );
    assert_eq!(
        f.vault.try_set_reward_window(&f.admin, &3_000, &2_000),
        Err(Ok(RewardWindowError::InvalidWindow))
    );
    assert_eq!(f.vault.get_reward_window(), None);
}

#[test]
fn only_the_admin_can_change_the_window() {
    let f = Fixture::new();
    let stranger = Address::generate(&f.env);
    assert_eq!(
        f.vault.try_set_reward_window(&stranger, &2_000, &3_000),
        Err(Ok(RewardWindowError::Unauthorized))
    );
    assert_eq!(
        f.vault.try_clear_reward_window(&stranger),
        Err(Ok(RewardWindowError::Unauthorized))
    );
}

#[test]
fn clearing_without_a_window_fails() {
    let f = Fixture::new();
    assert_eq!(
        f.vault.try_clear_reward_window(&f.admin),
        Err(Ok(RewardWindowError::NoWindow))
    );
}

#[test]
fn window_changes_are_recorded_in_the_parameter_change_log() {
    let f = Fixture::new();
    let before = f.vault.pool_parameter_change_log(&0, &100).len();

    f.vault.set_reward_window(&f.admin, &2_000, &3_000);
    f.vault.set_reward_window(&f.admin, &2_500, &3_500);

    let log = f.vault.pool_parameter_change_log(&0, &100);
    assert_eq!(log.len(), before + 4);

    let second_start = log.get(before + 2).unwrap();
    assert_eq!((second_start.old_value, second_start.new_value), (2_000, 2_500));
    let second_end = log.get(before + 3).unwrap();
    assert_eq!((second_end.old_value, second_end.new_value), (3_000, 3_500));
}
