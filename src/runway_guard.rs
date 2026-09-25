//! Reward-pool runway guard.
//!
//! A safety rail around `set_reward_rate_bps`: the admin can require that any
//! new reward rate leaves the reward pool with at least a configured number of
//! ledgers of runway at the current TVL, catching accidental misconfiguration
//! before rewards start failing to pay out.
//!
//! # Storage
//!
//! `DataKey` is at Soroban's 50-variant cap, so the configured minimum runway
//! is kept under a raw `Symbol`-keyed instance entry (matching `balance.rs`
//! and the other feature modules).
//!
//! Storage key: `symbol_short!("min_run")` -> `u32` (0 = guard disabled)

use soroban_sdk::{contractimpl, symbol_short, token, Address, Env, Symbol};

use crate::admin;
use crate::balance;
use crate::errors::VaultOpsError;
use crate::events;
use crate::vault::{VaultContract, VaultContractClient, BOOST_BPS_BASE, LEDGERS_PER_DAY, STELLAR_LEDGERS_PER_YEAR};

const MIN_RUNWAY_KEY: Symbol = symbol_short!("min_run");

/// Smallest non-zero runway `set_min_runway_ledgers` accepts (one day). A
/// lower bound stops the guard from being configured so loose it never fires.
pub const MIN_RUNWAY_LEDGERS_FLOOR: u32 = LEDGERS_PER_DAY;

/// Reads the configured minimum runway in ledgers. `0` means the guard is
/// disabled (the default), preserving the previous behaviour for existing
/// deployments.
pub fn read_min_runway_ledgers(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&MIN_RUNWAY_KEY)
        .unwrap_or(0)
}

fn write_min_runway_ledgers(env: &Env, ledgers: u32) {
    env.storage().instance().set(&MIN_RUNWAY_KEY, &ledgers);
}

/// Projected reward-pool runway, in ledgers, at `rate_bps` and the current
/// `total_deposited`.
///
/// Reward emission is an APR: the pool-wide spend per ledger is
/// `total_deposited * rate_bps / (10_000 * STELLAR_LEDGERS_PER_YEAR)`, so
///
/// `runway = reward_pool_balance * 10_000 * STELLAR_LEDGERS_PER_YEAR
///           / (rate_bps * total_deposited)`
///
/// Returns `u32::MAX` when the rate or TVL is zero (rewards never drain the
/// pool), and `0` when the pool is empty while the rate and TVL are positive.
pub fn projected_runway_ledgers(env: &Env, rate_bps: u32) -> u32 {
    if rate_bps == 0 {
        return u32::MAX;
    }
    let total_deposited = balance::get_total_deposited(env);
    if total_deposited <= 0 {
        return u32::MAX;
    }
    let reward_pool = balance::get_reward_pool_balance(env);
    if reward_pool <= 0 {
        return 0;
    }

    let denominator = (rate_bps as i128).saturating_mul(total_deposited);
    if denominator <= 0 {
        return u32::MAX;
    }

    let numerator = reward_pool
        .checked_mul(BOOST_BPS_BASE as i128)
        .and_then(|v| v.checked_mul(STELLAR_LEDGERS_PER_YEAR as i128));
    match numerator {
        Some(numerator) => {
            let ledgers = numerator / denominator;
            if ledgers >= u32::MAX as i128 {
                u32::MAX
            } else if ledgers <= 0 {
                0
            } else {
                ledgers as u32
            }
        }
        // Overflow in the numerator only makes the projected runway larger,
        // so treat it as effectively infinite rather than rejecting.
        None => u32::MAX,
    }
}

/// Enforces the configured minimum runway for a proposed rate. A no-op when
/// the guard is disabled (`get_min_runway_ledgers() == 0`).
pub(crate) fn enforce_runway(env: &Env, rate_bps: u32) -> Result<(), VaultOpsError> {
    let minimum = read_min_runway_ledgers(env);
    if minimum == 0 {
        return Ok(());
    }
    if projected_runway_ledgers(env, rate_bps) < minimum {
        return Err(VaultOpsError::InsufficientRunway);
    }
    Ok(())
}

#[contractimpl]
impl VaultContract {
    /// Admin: set the annual reward rate (basis points), reverting with
    /// `InsufficientRunway` when the new rate would exhaust the reward pool
    /// before the configured minimum runway. Reverts with `RateTooHigh` above
    /// `MAX_RATE_BPS`.
    pub fn set_reward_rate_bps(env: Env, rate_bps: u32) -> Result<(), VaultOpsError> {
        admin::require_admin(&env)?;
        if rate_bps > balance::MAX_RATE_BPS
            || !crate::reward_rate_ceiling::within_ceiling(&env, rate_bps)
        {
            return Err(VaultOpsError::RateTooHigh);
        }
        // Runway is evaluated against the *new* rate, before it is applied.
        enforce_runway(&env, rate_bps)?;

        let old_rate = balance::get_reward_rate_bps(&env);
        balance::set_reward_rate_bps(&env, rate_bps);
        events::rate_changed(&env, old_rate, rate_bps);
        Ok(())
    }

    /// Admin: configure the minimum reward-pool runway, in ledgers, required
    /// by `set_reward_rate_bps`. `0` disables the guard. Non-zero values below
    /// `MIN_RUNWAY_LEDGERS_FLOOR` (one day) revert with `InvalidRunway`.
    pub fn set_min_runway_ledgers(
        env: Env,
        admin_addr: Address,
        ledgers: u32,
    ) -> Result<(), VaultOpsError> {
        admin_addr.require_auth();
        admin::require_admin(&env)?;
        if ledgers != 0 && ledgers < MIN_RUNWAY_LEDGERS_FLOOR {
            return Err(VaultOpsError::InvalidRunway);
        }
        write_min_runway_ledgers(&env, ledgers);
        Ok(())
    }

    /// Read-only: the configured minimum runway in ledgers (`0` = disabled).
    pub fn get_min_runway_ledgers(env: Env) -> u32 {
        read_min_runway_ledgers(&env)
    }

    /// Read-only: the current annual reward rate in basis points.
    pub fn get_reward_rate_bps(env: Env) -> u32 {
        balance::get_reward_rate_bps(&env)
    }

    /// Read-only: projected reward-pool runway at the current rate and TVL,
    /// in ledgers (`u32::MAX` = effectively infinite).
    pub fn get_projected_runway(env: Env) -> u32 {
        projected_runway_ledgers(&env, balance::get_reward_rate_bps(&env))
    }

    /// Admin: transfer `amount` of the reward token into the vault and credit
    /// it to the reward pool. Funds the rewards that `set_reward_rate_bps`
    /// commits to paying, which is what the runway guard protects.
    pub fn fund_reward_pool(
        env: Env,
        admin_addr: Address,
        amount: i128,
    ) -> Result<(), VaultOpsError> {
        admin_addr.require_auth();
        admin::require_admin(&env)?;
        if amount <= 0 {
            return Err(VaultOpsError::ZeroAmount);
        }

        let token_addr: Address = match balance::get_reward_token(&env) {
            Some(token) => token,
            None => env
                .storage()
                .instance()
                .get(&crate::storage::DataKey::Token)
                .ok_or(VaultOpsError::NotInitialized)?,
        };
        token::Client::new(&env, &token_addr).transfer(
            &admin_addr,
            &env.current_contract_address(),
            &amount,
        );

        let pool = balance::get_reward_pool_balance(&env);
        let updated = pool
            .checked_add(amount)
            .ok_or(VaultOpsError::ArithmeticError)?;
        balance::set_reward_pool_balance(&env, updated);
        Ok(())
    }
}
