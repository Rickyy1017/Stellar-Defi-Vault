//! Vault extensions for issues #765, #766, #767, #768.
//!
//! - Issue #765: Enforce an idle buffer ratio so withdrawals stay liquid.
//!   (`set_min_idle_ratio_bps`, `get_idle_ratio_bps`, `allocate_to_strategy`,
//!    and strategy-backed liquidity pull on withdrawals).
//! - Issue #766: Add harvest() for strategy yield with a performance fee.
//!   (`harvest`, `set_performance_fee_bps`, `get_performance_fee_bps`).
//! - Issue #767: Define loss accounting and socialization policy for strategy shortfalls.
//!   (`report_strategy_loss`).
//! - Issue #768: Add emergency_unwind() to recall all funds from an adapter and disable it.
//!   (`emergency_unwind`).
//!
//! # Storage Keys
//! `DataKey` sits at Soroban's 50-variant limit, so all strategy state is
//! maintained under dedicated raw `Symbol` keys.

use soroban_sdk::{
    contractimpl, contracttype, symbol_short, token, Address, Env, IntoVal, Symbol, Val, Vec,
};

use crate::admin;
use crate::balance;
use crate::errors::{StrategyError, VaultError};
use crate::events;
use crate::vault::{VaultContract, BOOST_BPS_BASE};

/// Key for the minimum idle buffer ratio (bps).
const IDLE_RATIO_KEY: Symbol = symbol_short!("idle_bps");

/// Key for the performance fee ratio (bps).
const PERF_FEE_KEY: Symbol = symbol_short!("perf_bps");

/// Key for the list of registered strategy records.
const STRATEGIES_KEY: Symbol = symbol_short!("str_list");

/// Maximum allowable performance fee (5 000 bps = 50%).
pub const MAX_PERFORMANCE_FEE_BPS: u32 = 5_000;

/// Maximum allowable idle buffer ratio (10 000 bps = 100%).
pub const MAX_IDLE_BUFFER_BPS: u32 = 10_000;

/// Default keeper incentive share from the performance fee (500 bps = 5%).
pub const KEEPER_TIP_SHARE_BPS: u32 = 500;

/// Record tracking an active or disabled strategy adapter.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct StrategyRecord {
    /// Strategy adapter contract address.
    pub adapter: Address,
    /// Currently tracked principal allocated to this adapter.
    pub allocated: i128,
    /// High-water mark for performance fee calculation (monotonically non-decreasing
    /// during profit periods; preserved during drawdowns so recoveries are not double-charged).
    pub high_water_mark: i128,
    /// Whether this strategy has been emergency unwound or disabled.
    pub disabled: bool,
}

// ── Storage Helpers ─────────────────────────────────────────────────────────

/// Retrieve the list of all strategy records.
pub fn get_strategy_records(env: &Env) -> Vec<StrategyRecord> {
    env.storage()
        .instance()
        .get(&STRATEGIES_KEY)
        .unwrap_or_else(|| Vec::new(env))
}

/// Save the list of strategy records.
pub fn set_strategy_records(env: &Env, records: &Vec<StrategyRecord>) {
    env.storage().instance().set(&STRATEGIES_KEY, records);
}

/// Retrieve the configured minimum idle ratio in bps.
pub fn get_min_idle_ratio(env: &Env) -> u32 {
    env.storage().instance().get(&IDLE_RATIO_KEY).unwrap_or(0)
}

/// Retrieve the configured performance fee in bps.
pub fn get_perf_fee_bps(env: &Env) -> u32 {
    env.storage().instance().get(&PERF_FEE_KEY).unwrap_or(0)
}

/// Sum of all assets currently allocated to non-disabled strategies.
pub fn get_total_allocated_to_strategies(env: &Env) -> i128 {
    let records = get_strategy_records(env);
    let mut total: i128 = 0;
    for i in 0..records.len() {
        let rec = records.get(i).unwrap();
        if !rec.disabled && rec.allocated > 0 {
            total = total.saturating_add(rec.allocated);
        }
    }
    total
}

// ── Withdrawal Liquidity Helper ─────────────────────────────────────────────

/// Called during user unstakes/withdrawals when the vault's idle token balance
/// is insufficient to cover the withdrawal amount. Pulls `amount_needed`
/// from active strategy adapters in the order they were registered.
///
/// Contains failing adapters so a single broken strategy does not brick user withdrawals.
pub fn pull_from_strategies(env: &Env, amount_needed: i128) -> Result<i128, StrategyError> {
    if amount_needed <= 0 {
        return Ok(0);
    }

    let mut records = get_strategy_records(env);
    let mut total_pulled: i128 = 0;
    let mut modified = false;

    for i in 0..records.len() {
        if total_pulled >= amount_needed {
            break;
        }

        let mut rec = records.get(i).unwrap();
        if rec.disabled || rec.allocated <= 0 {
            continue;
        }

        let to_pull = (amount_needed - total_pulled).min(rec.allocated);
        let args: Vec<Val> = (env.current_contract_address(), to_pull).into_val(env);

        // Contained invocation: a failing/reverting adapter will not revert the vault transaction
        let pull_res = env.try_invoke_contract::<i128>(&rec.adapter, &symbol_short!("withdraw"), args);

        let pulled_amount = match pull_res {
            Ok(Ok(amt)) => amt.max(0),
            _ => 0,
        };

        if pulled_amount > 0 {
            rec.allocated = rec.allocated.saturating_sub(pulled_amount);
            total_pulled = total_pulled.saturating_add(pulled_amount);
            records.set(i, rec.clone());
            modified = true;
            events::strategy_pulled(env, &rec.adapter, pulled_amount, env.ledger().sequence());
        }
    }

    if modified {
        set_strategy_records(env, &records);
    }

    Ok(total_pulled)
}

// ── Entrypoints Implementation on VaultContract ─────────────────────────────

#[contractimpl]
impl VaultContract {
    // ── Issue #765: Idle Buffer Ratio ───────────────────────────────────────

    /// Admin: set the minimum fraction of TVL (in bps) required to remain idle
    /// in the vault contract to satisfy routine withdrawals without unwinding strategies.
    pub fn set_min_idle_ratio_bps(
        env: Env,
        admin: Address,
        bps: u32,
    ) -> Result<(), StrategyError> {
        admin.require_auth();
        admin::require_admin_as(&env, &admin)?;

        if bps > MAX_IDLE_BUFFER_BPS {
            return Err(StrategyError::InvalidRatio);
        }

        env.storage().instance().set(&IDLE_RATIO_KEY, &bps);
        events::min_idle_ratio_updated(&env, &admin, bps, env.ledger().sequence());
        Ok(())
    }

    /// Read the currently configured minimum idle buffer ratio in bps.
    pub fn get_idle_ratio_bps(env: Env) -> u32 {
        get_min_idle_ratio(&env)
    }

    /// Admin: allocate vault idle funds to an external strategy adapter.
    ///
    /// Reverts with `IdleBufferBreached` if the allocation would cause idle funds
    /// to drop below the required idle buffer ratio.
    /// Reverts with `StrategyDisabled` if the target adapter has been disabled.
    pub fn allocate_to_strategy(
        env: Env,
        admin: Address,
        adapter: Address,
        amount: i128,
    ) -> Result<(), StrategyError> {
        admin.require_auth();
        admin::require_admin_as(&env, &admin)?;

        if amount <= 0 {
            return Err(StrategyError::ZeroAmount);
        }

        let token_addr = Self::token_address(&env)?;
        let token_client = token::Client::new(&env, &token_addr);
        let contract_addr = env.current_contract_address();

        let idle_balance = token_client.balance(&contract_addr);
        if amount > idle_balance {
            return Err(StrategyError::IdleBufferBreached);
        }

        // TVL = current vault idle balance + all funds currently deployed to active strategies
        let deployed_to_strategies = get_total_allocated_to_strategies(&env);
        let tvl = idle_balance
            .checked_add(deployed_to_strategies)
            .ok_or(StrategyError::ArithmeticError)?;

        let min_idle_bps = get_min_idle_ratio(&env);
        if min_idle_bps > 0 {
            let required_idle = tvl
                .checked_mul(min_idle_bps as i128)
                .and_then(|v| v.checked_div(BOOST_BPS_BASE as i128))
                .ok_or(StrategyError::ArithmeticError)?;

            let remaining_idle = idle_balance - amount;
            if remaining_idle < required_idle {
                return Err(StrategyError::IdleBufferBreached);
            }
        }

        let mut records = get_strategy_records(&env);
        let mut found_index: Option<u32> = None;

        for i in 0..records.len() {
            let rec = records.get(i).unwrap();
            if rec.adapter == adapter {
                if rec.disabled {
                    return Err(StrategyError::StrategyDisabled);
                }
                found_index = Some(i);
                break;
            }
        }

        if let Some(idx) = found_index {
            let mut rec = records.get(idx).unwrap();
            rec.allocated = rec.allocated.checked_add(amount).ok_or(StrategyError::ArithmeticError)?;
            rec.high_water_mark = rec.high_water_mark.checked_add(amount).ok_or(StrategyError::ArithmeticError)?;
            records.set(idx, rec);
        } else {
            records.push_back(StrategyRecord {
                adapter: adapter.clone(),
                allocated: amount,
                high_water_mark: amount,
                disabled: false,
            });
        }

        set_strategy_records(&env, &records);

        // Transfer funds to adapter
        token_client.transfer(&contract_addr, &adapter, &amount);

        // Notify adapter via deposit call (wrapped safely)
        let deposit_args: Vec<Val> = (contract_addr.clone(), amount).into_val(&env);
        let _ = env.try_invoke_contract::<()>(&adapter, &symbol_short!("deposit"), deposit_args);

        events::strategy_allocated(&env, &adapter, amount, env.ledger().sequence());
        Ok(())
    }

    /// Read the tracked allocation for a specific strategy adapter.
    pub fn get_strategy_allocation(env: Env, adapter: Address) -> i128 {
        let records = get_strategy_records(&env);
        for i in 0..records.len() {
            let rec = records.get(i).unwrap();
            if rec.adapter == adapter {
                return rec.allocated;
            }
        }
        0
    }

    /// Read all registered strategy adapter addresses.
    pub fn get_strategies(env: Env) -> Vec<Address> {
        let records = get_strategy_records(&env);
        let mut list = Vec::new(&env);
        for i in 0..records.len() {
            list.push_back(records.get(i).unwrap().adapter);
        }
        list
    }

    // ── Issue #766: Harvest & Performance Fee ───────────────────────────────

    /// Admin: set the performance fee charged on new net strategy yields (bps).
    /// `0` disables the fee; capped at `MAX_PERFORMANCE_FEE_BPS` (50%).
    pub fn set_performance_fee_bps(
        env: Env,
        admin: Address,
        bps: u32,
    ) -> Result<(), StrategyError> {
        admin.require_auth();
        admin::require_admin_as(&env, &admin)?;

        if bps > MAX_PERFORMANCE_FEE_BPS {
            return Err(StrategyError::InvalidFee);
        }

        env.storage().instance().set(&PERF_FEE_KEY, &bps);
        Ok(())
    }

    /// Read the currently configured performance fee in bps.
    pub fn get_performance_fee_bps(env: Env) -> u32 {
        get_perf_fee_bps(&env)
    }

    /// Realize accumulated strategy yield into the vault and collect the
    /// performance fee. Callable by anyone (keepers, users, or admins).
    ///
    /// Respects a high-water mark per strategy so users are never charged
    /// fees on capital recovering from a prior drawdown.
    pub fn harvest(env: Env, caller: Address) -> Result<i128, StrategyError> {
        caller.require_auth();

        let mut records = get_strategy_records(&env);
        let fee_bps = get_perf_fee_bps(&env);
        let mut total_harvested: i128 = 0;
        let mut modified = false;

        let token_addr = Self::token_address(&env)?;
        let token_client = token::Client::new(&env, &token_addr);
        let contract_addr = env.current_contract_address();

        for i in 0..records.len() {
            let mut rec = records.get(i).unwrap();
            if rec.disabled {
                continue;
            }

            // Query current strategy asset value
            let args: Vec<Val> = ().into_val(&env);
            let asset_query = env.try_invoke_contract::<i128>(&rec.adapter, &symbol_short!("assets"), args);

            let current_value = match asset_query {
                Ok(Ok(val)) => val,
                _ => rec.allocated,
            };

            // High-water mark rule: fee applies strictly to gains exceeding previous peak
            if current_value > rec.high_water_mark {
                let gain = current_value - rec.high_water_mark;
                rec.high_water_mark = current_value;
                rec.allocated = current_value;

                let fee = if fee_bps > 0 {
                    (gain * fee_bps as i128) / (BOOST_BPS_BASE as i128)
                } else {
                    0
                };

                if fee > 0 {
                    // Optional keeper incentive tip (5% of performance fee)
                    let keeper_tip = (fee * KEEPER_TIP_SHARE_BPS as i128) / (BOOST_BPS_BASE as i128);
                    let treasury_portion = fee - keeper_tip;

                    if keeper_tip > 0 && token_client.balance(&contract_addr) >= keeper_tip {
                        token_client.transfer(&contract_addr, &caller, &keeper_tip);
                    }

                    if treasury_portion > 0 {
                        // Route through treasury split if configured, otherwise fallback to slash treasury
                        let _ = crate::vault_extensions_498_501::apply_treasury_split(&env, treasury_portion, treasury_portion);
                    }
                }

                events::harvested(&env, &rec.adapter, gain, fee, env.ledger().sequence());
                total_harvested = total_harvested.saturating_add(gain);
                records.set(i, rec);
                modified = true;
            }
        }

        if modified {
            set_strategy_records(&env, &records);
        }

        Ok(total_harvested)
    }

    // ── Issue #767: Loss Accounting & Socialization ─────────────────────────

    /// Record a realized strategy shortfall or loss.
    ///
    /// Hierarchical loss absorption:
    /// 1. First layer: Absorbed by the operator-funded insurance reserve (issue #704).
    /// 2. Second layer: Any excess loss is socialized across all vault shares by reducing
    ///    `total_deposited`, which lowers share price proportionally for all depositors.
    ///
    /// Front-running protection:
    /// Reconciles state immediately on-chain so subsequent user withdrawals price against
    /// the post-loss share valuation.
    pub fn report_strategy_loss(
        env: Env,
        caller: Address,
        adapter: Address,
    ) -> Result<i128, StrategyError> {
        caller.require_auth();

        let mut records = get_strategy_records(&env);
        let mut found_index: Option<u32> = None;

        for i in 0..records.len() {
            let rec = records.get(i).unwrap();
            if rec.adapter == adapter {
                found_index = Some(i);
                break;
            }
        }

        let idx = found_index.ok_or(StrategyError::StrategyNotFound)?;
        let mut rec = records.get(idx).unwrap();

        // Query current adapter assets to determine realized loss
        let args: Vec<Val> = ().into_val(&env);
        let asset_query = env.try_invoke_contract::<i128>(&rec.adapter, &symbol_short!("assets"), args);
        let current_value = match asset_query {
            Ok(Ok(val)) => val.max(0),
            _ => 0,
        };

        if current_value >= rec.allocated {
            return Err(StrategyError::NoLoss);
        }

        let total_loss = rec.allocated - current_value;

        // Layer 1: Insurance reserve absorption
        let available_reserve = crate::vault_extensions_701_704::get_reserve(&env);
        let reserve_absorbed = total_loss.min(available_reserve);

        if reserve_absorbed > 0 {
            crate::vault_extensions_701_704::set_reserve(&env, available_reserve - reserve_absorbed);
        }

        // Layer 2: Share price socialization of remaining shortfall
        let share_price_absorbed = total_loss - reserve_absorbed;
        if share_price_absorbed > 0 {
            let total_deposited = balance::get_total_deposited(&env);
            balance::set_total_deposited(&env, (total_deposited - share_price_absorbed).max(0));
        }

        // Adjust strategy accounting
        rec.allocated = current_value;
        rec.high_water_mark = current_value;
        records.set(idx, rec);
        set_strategy_records(&env, &records);

        events::strategy_loss_recorded(
            &env,
            &adapter,
            total_loss,
            reserve_absorbed,
            share_price_absorbed,
            env.ledger().sequence(),
        );

        Ok(total_loss)
    }

    // ── Issue #768: Emergency Strategy Unwind ────────────────────────────────

    /// Admin or Guardian: recall all funds from a strategy adapter and disable it.
    ///
    /// - Marks the strategy as disabled immediately (cannot receive new allocations).
    /// - Safely attempts to withdraw all allocated funds from the adapter.
    /// - A failing or reverting adapter is contained and does not brick the vault.
    /// - Any shortfall between allocated principal and recovered tokens is reconciled
    ///   via the two-layer loss accounting pipeline.
    pub fn emergency_unwind(
        env: Env,
        caller: Address,
        adapter: Address,
    ) -> Result<i128, StrategyError> {
        caller.require_auth();

        let is_admin = admin::require_admin_as(&env, &caller).is_ok();
        let is_guardian = crate::guardian_pause::is_guardian(env.clone(), caller.clone());

        if !is_admin && !is_guardian {
            return Err(StrategyError::Unauthorized);
        }

        let mut records = get_strategy_records(&env);
        let mut found_index: Option<u32> = None;

        for i in 0..records.len() {
            let rec = records.get(i).unwrap();
            if rec.adapter == adapter {
                found_index = Some(i);
                break;
            }
        }

        let idx = found_index.ok_or(StrategyError::StrategyNotFound)?;
        let mut rec = records.get(idx).unwrap();

        // Immediately disable strategy
        rec.disabled = true;

        let allocated = rec.allocated;
        let contract_addr = env.current_contract_address();

        // Safely invoke adapter withdraw
        let args: Vec<Val> = (contract_addr.clone(), allocated).into_val(&env);
        let withdraw_res = env.try_invoke_contract::<i128>(&adapter, &symbol_short!("withdraw"), args);

        let recovered = match withdraw_res {
            Ok(Ok(amt)) => amt.max(0),
            _ => 0,
        };

        let loss = (allocated - recovered).max(0);

        if loss > 0 {
            // Reconcile loss via absorption hierarchy
            let available_reserve = crate::vault_extensions_701_704::get_reserve(&env);
            let reserve_absorbed = loss.min(available_reserve);

            if reserve_absorbed > 0 {
                crate::vault_extensions_701_704::set_reserve(&env, available_reserve - reserve_absorbed);
            }

            let share_price_absorbed = loss - reserve_absorbed;
            if share_price_absorbed > 0 {
                let total_deposited = balance::get_total_deposited(&env);
                balance::set_total_deposited(&env, (total_deposited - share_price_absorbed).max(0));
            }
        }

        rec.allocated = 0;
        rec.high_water_mark = 0;
        records.set(idx, rec);
        set_strategy_records(&env, &records);

        events::strategy_unwound(&env, &adapter, recovered, loss, env.ledger().sequence());

        Ok(recovered)
    }

    /// Read whether a given strategy adapter is currently disabled.
    pub fn is_strategy_disabled(env: Env, adapter: Address) -> bool {
        let records = get_strategy_records(&env);
        for i in 0..records.len() {
            let rec = records.get(i).unwrap();
            if rec.adapter == adapter {
                return rec.disabled;
            }
        }
        false
    }
}
