#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events as _, Ledger as _},
    token, Address, Env,
};

use crate::{
    balance,
    errors::StrategyError,
    vault::{VaultContract, VaultContractClient},
};

// ── Mock Strategy Adapters ──────────────────────────────────────────────────

#[contract]
pub struct MockStrategyAdapter;

#[contractimpl]
impl MockStrategyAdapter {
    pub fn deposit(_env: Env, _from: Address, _amount: i128) {}

    pub fn withdraw(env: Env, to: Address, amount: i128) -> i128 {
        let token_addr: Option<Address> = env.storage().instance().get(&symbol_short!("tok"));
        if let Some(tok) = token_addr {
            let client = token::Client::new(&env, &tok);
            let bal = client.balance(&env.current_contract_address());
            let to_send = amount.min(bal);
            if to_send > 0 {
                client.transfer(&env.current_contract_address(), &to, &to_send);
            }
            return to_send;
        }
        amount
    }

    pub fn assets(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&symbol_short!("val"))
            .unwrap_or(0)
    }

    pub fn set_assets(env: Env, val: i128) {
        env.storage().instance().set(&symbol_short!("val"), &val);
    }

    pub fn set_token(env: Env, token: Address) {
        env.storage().instance().set(&symbol_short!("tok"), &token);
    }
}

#[contract]
pub struct FailingStrategyAdapter;

#[contractimpl]
impl FailingStrategyAdapter {
    pub fn deposit(_env: Env, _from: Address, _amount: i128) {
        panic!("adapter deposit failed");
    }

    pub fn withdraw(_env: Env, _to: Address, _amount: i128) -> i128 {
        panic!("adapter withdraw failed");
    }

    pub fn assets(_env: Env) -> i128 {
        0
    }
}

// ── Test Fixture ────────────────────────────────────────────────────────────

struct Fixture<'a> {
    env: Env,
    vault: VaultContractClient<'a>,
    vault_id: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    token_addr: Address,
    admin: Address,
    guardian: Address,
    alice: Address,
    bob: Address,
}

impl<'a> Fixture<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.min_temp_entry_ttl = 10_000_000;
            li.min_persistent_entry_ttl = 10_000_000;
            li.max_entry_ttl = 10_000_000;
            li.sequence_number = 1000;
        });

        let admin = Address::generate(&env);
        let guardian = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        let token_addr = env.register_stellar_asset_contract(admin.clone());
        let token = token::Client::new(&env, &token_addr);
        let token_admin = token::StellarAssetClient::new(&env, &token_addr);

        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &token_addr, &1000, &None, &None);

        vault.set_guardian(&guardian);

        token_admin.mint(&alice, &1_000_000);
        token_admin.mint(&bob, &1_000_000);

        Fixture {
            env,
            vault,
            vault_id,
            token,
            token_admin,
            token_addr,
            admin,
            guardian,
            alice,
            bob,
        }
    }

    fn deploy_mock_adapter(&self) -> (Address, MockStrategyAdapterClient<'a>) {
        let adapter_id = self.env.register_contract(None, MockStrategyAdapter);
        let client = MockStrategyAdapterClient::new(&self.env, &adapter_id);
        client.set_token(&self.token_addr);
        (adapter_id, client)
    }

    fn deploy_failing_adapter(&self) -> Address {
        self.env.register_contract(None, FailingStrategyAdapter)
    }
}

// ── Issue #765 Tests: Idle Buffer Ratio ─────────────────────────────────────

#[test]
fn test_idle_buffer_ratio_configuration() {
    let f = Fixture::new();

    assert_eq!(f.vault.get_idle_ratio_bps(), 0);

    // Set idle ratio to 20% (2000 bps)
    f.vault.set_min_idle_ratio_bps(&f.admin, &2_000);
    assert_eq!(f.vault.get_idle_ratio_bps(), 2_000);

    // Exceeding 100% (10 000 bps) fails
    let err = f.vault.try_set_min_idle_ratio_bps(&f.admin, &10_001);
    assert!(err.is_err());
}

#[test]
fn test_allocate_blocked_when_idle_buffer_breached() {
    let f = Fixture::new();
    let (adapter_id, _) = f.deploy_mock_adapter();

    // Alice deposits 100,000 into vault
    f.vault.deposit(&f.alice, &100_000);

    // Set minimum idle ratio to 30% (3,000 bps) -> requires 30,000 idle
    f.vault.set_min_idle_ratio_bps(&f.admin, &3_000);

    // Allocating 80,000 would leave 20,000 idle (< 30,000 required), so it must revert
    let res = f.vault.try_allocate_to_strategy(&f.admin, &adapter_id, &80_000);
    assert_eq!(
        res.unwrap_err().unwrap(),
        StrategyError::IdleBufferBreached
    );

    // Allocating 70,000 leaves 30,000 idle (= 30% of 100,000), which succeeds
    f.vault.allocate_to_strategy(&f.admin, &adapter_id, &70_000);
    assert_eq!(f.vault.get_strategy_allocation(&adapter_id), 70_000);
    assert_eq!(f.token.balance(&f.vault_id), 30_000);
}

#[test]
fn test_withdrawal_triggers_pull_from_strategies() {
    let f = Fixture::new();
    let (adapter_id, adapter_client) = f.deploy_mock_adapter();

    // Alice deposits 100,000
    f.vault.deposit(&f.alice, &100_000);

    // Allocate 60,000 to strategy
    f.vault.allocate_to_strategy(&f.admin, &adapter_id, &60_000);
    assert_eq!(f.token.balance(&f.vault_id), 40_000);

    // Alice withdraws 70,000. Idle is only 40,000, so 30,000 must be pulled from strategy
    let shares = 70_000;
    let withdrawn = f.vault.withdraw(&f.alice, &shares);
    assert_eq!(withdrawn, 70_000);

    // Strategy tracked allocation is reduced to 30,000
    assert_eq!(f.vault.get_strategy_allocation(&adapter_id), 30_000);
    assert_eq!(f.token.balance(&f.alice), 970_000); // 1,000,000 - 100,000 + 70,000
}

// ── Issue #766 Tests: Harvest & Performance Fee ─────────────────────────────

#[test]
fn test_performance_fee_and_harvest_gains() {
    let f = Fixture::new();
    let (adapter_id, adapter_client) = f.deploy_mock_adapter();

    f.vault.deposit(&f.alice, &100_000);
    f.vault.allocate_to_strategy(&f.admin, &adapter_id, &50_000);

    // Set performance fee to 20% (2000 bps)
    f.vault.set_performance_fee_bps(&f.admin, &2_000);
    assert_eq!(f.vault.get_performance_fee_bps(), 2_000);

    // Simulate strategy value appreciated from 50,000 to 60,000 (gain = 10,000)
    adapter_client.set_assets(&60_000);

    let gain = f.vault.harvest(&f.bob);
    assert_eq!(gain, 10_000);

    // High water mark is now updated to 60,000
    // Calling harvest again with same assets yields 0 gain and 0 fee
    let flat_gain = f.vault.harvest(&f.bob);
    assert_eq!(flat_gain, 0);

    // If strategy experiences a drawdown from 60,000 to 45,000:
    adapter_client.set_assets(&45_000);
    let drawdown_gain = f.vault.harvest(&f.bob);
    assert_eq!(drawdown_gain, 0);

    // Recovery from 45,000 to 55,000 is below the 60,000 HWM -> no fee charged!
    adapter_client.set_assets(&55_000);
    let recovery_gain = f.vault.harvest(&f.bob);
    assert_eq!(recovery_gain, 0);

    // Once it breaks previous peak to 65,000 -> gain of 5,000 is harvested!
    adapter_client.set_assets(&65_000);
    let new_peak_gain = f.vault.harvest(&f.bob);
    assert_eq!(new_peak_gain, 5_000);
}

// ── Issue #767 Tests: Loss Accounting & Socialization ───────────────────────

#[test]
fn test_report_strategy_loss_covered_by_insurance_reserve() {
    let f = Fixture::new();
    let (adapter_id, adapter_client) = f.deploy_mock_adapter();

    f.vault.deposit(&f.alice, &100_000);
    f.vault.allocate_to_strategy(&f.admin, &adapter_id, &50_000);

    // Pre-fund the insurance reserve with 10,000
    crate::vault_extensions_701_704::set_reserve(&f.env, 10_000);
    assert_eq!(f.vault.get_insurance_reserve(), 10_000);

    // Strategy suffers a 6,000 loss (assets drop from 50,000 to 44,000)
    adapter_client.set_assets(&44_000);

    let loss = f.vault.report_strategy_loss(&f.admin, &adapter_id);
    assert_eq!(loss, 6_000);

    // Entire loss absorbed by insurance reserve (10,000 - 6,000 = 4,000 remaining)
    assert_eq!(f.vault.get_insurance_reserve(), 4_000);

    // Share price / total_deposited is untouched because reserve absorbed it
    assert_eq!(balance::get_total_deposited(&f.env), 100_000);
}

#[test]
fn test_report_strategy_loss_excess_socialized_into_share_price() {
    let f = Fixture::new();
    let (adapter_id, adapter_client) = f.deploy_mock_adapter();

    f.vault.deposit(&f.alice, &100_000);
    f.vault.allocate_to_strategy(&f.admin, &adapter_id, &50_000);

    // Reserve only has 2,000
    crate::vault_extensions_701_704::set_reserve(&f.env, 2_000);

    // Strategy suffers a 10,000 loss (assets drop to 40,000)
    adapter_client.set_assets(&40_000);

    let loss = f.vault.report_strategy_loss(&f.admin, &adapter_id);
    assert_eq!(loss, 10_000);

    // Reserve is exhausted (0 remaining)
    assert_eq!(f.vault.get_insurance_reserve(), 0);

    // Excess 8,000 loss is socialized: total_deposited reduced from 100,000 to 92,000
    assert_eq!(balance::get_total_deposited(&f.env), 92_000);
}

// ── Issue #768 Tests: Emergency Strategy Unwind ─────────────────────────────

#[test]
fn test_emergency_unwind_and_disabled_enforcement() {
    let f = Fixture::new();
    let (adapter_id, adapter_client) = f.deploy_mock_adapter();

    f.vault.deposit(&f.alice, &100_000);
    f.vault.allocate_to_strategy(&f.admin, &adapter_id, &40_000);

    // Pre-fund adapter with tokens so it can return on unwind
    f.token_admin.mint(&adapter_id, &40_000);

    // Guardian triggers emergency unwind
    let recovered = f.vault.emergency_unwind(&f.guardian, &adapter_id);
    assert_eq!(recovered, 40_000);

    // Strategy is disabled
    assert!(f.vault.is_strategy_disabled(&adapter_id));
    assert_eq!(f.vault.get_strategy_allocation(&adapter_id), 0);

    // Disabled strategy cannot receive new allocations
    let alloc_res = f.vault.try_allocate_to_strategy(&f.admin, &adapter_id, &10_000);
    assert_eq!(
        alloc_res.unwrap_err().unwrap(),
        StrategyError::StrategyDisabled
    );
}

#[test]
fn test_emergency_unwind_reverting_adapter_contained() {
    let f = Fixture::new();
    let adapter_id = f.deploy_failing_adapter();

    f.vault.deposit(&f.alice, &100_000);

    // Record an allocation manually in storage to simulate pre-existing allocation
    let mut records = crate::vault_extensions_765_768::get_strategy_records(&f.env);
    records.push_back(crate::vault_extensions_765_768::StrategyRecord {
        adapter: adapter_id.clone(),
        allocated: 30_000,
        high_water_mark: 30_000,
        disabled: false,
    });
    crate::vault_extensions_765_768::set_strategy_records(&f.env, &records);

    // Emergency unwind on a reverting adapter does NOT brick or panic the vault
    let recovered = f.vault.emergency_unwind(&f.admin, &adapter_id);
    assert_eq!(recovered, 0);

    // Strategy is disabled and shortfall (30,000) was socialized into loss accounting
    assert!(f.vault.is_strategy_disabled(&adapter_id));
    assert_eq!(balance::get_total_deposited(&f.env), 70_000); // 100,000 - 30,000
}
