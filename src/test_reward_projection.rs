#![cfg(test)]
//! Tests for `project_reward` (future-dated reward estimates).

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};

use crate::{
    errors::ProjectionError,
    vault::{VaultContract, VaultContractClient},
};

const STAKE: i128 = 100_000_000_000;
const START_LEDGER: u32 = 1_000;
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
    /// A pool at `START_LEDGER` with `RATE_BPS` and alice staked in it.
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|ledger| {
            ledger.min_persistent_entry_ttl = 10_000_000;
            ledger.min_temp_entry_ttl = 10_000_000;
            ledger.max_entry_ttl = 10_000_000;
            ledger.sequence_number = START_LEDGER;
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

    /// The pending reward `calc_pending_reward` reports once the chain has
    /// actually reached `ledger`.
    fn actual_at(&self, ledger: u32) -> i128 {
        set_ledger(&self.env, ledger);
        self.vault.calc_pending_reward(&self.alice)
    }
}

#[test]
fn projecting_the_current_ledger_matches_calc_pending_reward() {
    let f = Fixture::new();
    set_ledger(&f.env, START_LEDGER + 50_000);
    let now = f.env.ledger().sequence();
    assert_eq!(
        f.vault.project_reward(&f.alice, &now),
        f.vault.calc_pending_reward(&f.alice)
    );
}

#[test]
fn an_earlier_ledger_is_rejected() {
    let f = Fixture::new();
    set_ledger(&f.env, START_LEDGER + 100);
    assert_eq!(
        f.vault.try_project_reward(&f.alice, &(START_LEDGER + 99)),
        Err(Ok(ProjectionError::LedgerInPast))
    );
}

#[test]
fn user_without_a_position_projects_zero() {
    let f = Fixture::new();
    let stranger = Address::generate(&f.env);
    assert_eq!(f.vault.project_reward(&stranger, &(START_LEDGER + 1_000)), 0);
}

#[test]
fn projection_grows_with_the_target_ledger() {
    let f = Fixture::new();
    let near = f.vault.project_reward(&f.alice, &(START_LEDGER + 100_000));
    let far = f.vault.project_reward(&f.alice, &(START_LEDGER + 200_000));
    assert!(near > 0);
    assert!(far > near);
}

#[test]
fn projection_matches_what_accrues_at_a_constant_rate() {
    let f = Fixture::new();
    let target = START_LEDGER + 300_000;
    let projected = f.vault.project_reward(&f.alice, &target);
    assert!(projected > 0);
    assert_eq!(f.actual_at(target), projected);
}

#[test]
fn projection_does_not_change_state() {
    let f = Fixture::new();
    let before = f.vault.calc_pending_reward(&f.alice);
    let _ = f.vault.project_reward(&f.alice, &(START_LEDGER + 500_000));
    assert_eq!(f.vault.calc_pending_reward(&f.alice), before);
}

#[test]
fn projection_respects_a_halving_schedule() {
    let f = Fixture::new();
    let target = START_LEDGER + 250_000;
    let without_halving = f.vault.project_reward(&f.alice, &target);

    // The rate halves every 100_000 ledgers, so two halvings fall inside the
    // window.
    f.vault.set_halving_schedule(&f.admin, &100_000, &5_000, &0);
    let projected = f.vault.project_reward(&f.alice, &target);

    assert!(projected < without_halving);
    assert_eq!(f.actual_at(target), projected);
}

#[test]
fn projection_respects_a_rate_ramp() {
    let f = Fixture::new();
    let target = START_LEDGER + 200_000;
    let flat = f.vault.project_reward(&f.alice, &target);

    // Ramp the rate up to 3x over the whole window.
    f.vault.set_rate_ramp(&f.admin, &(RATE_BPS * 3), &200_000);
    let ramped = f.vault.project_reward(&f.alice, &target);

    // Linear ramp from 1x to 3x averages 2x over the window.
    assert!(ramped > flat);
    let tolerance = flat / 1_000;
    assert!((ramped - flat * 2).abs() <= tolerance);
}
