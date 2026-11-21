//! Future-dated reward estimates (`project_reward`).
//!
//! `calc_pending_reward` reports only what a position has earned so far. UIs
//! also want "you will have X by ledger N", and should not have to
//! reimplement the reward math client-side. `project_reward` runs the
//! contract's own accrual math forward to a target ledger.
//!
//! This module is declared as a child of `vault` (see the tail of
//! `vault.rs`) so it can reuse the private accrual helpers.

use soroban_sdk::{contractimpl, symbol_short, Address, Env};

use crate::admin;
use crate::balance;
use crate::errors::ProjectionError;
use crate::vault::VaultContract;

/// Estimate the reward `user` will have pending at `at_ledger`, in reward
/// token decimals, as `calc_pending_reward` would report it at that ledger.
pub fn project(env: &Env, user: &Address, at_ledger: u32) -> Result<i128, ProjectionError> {
    let now = env.ledger().sequence();
    if at_ledger < now {
        return Err(ProjectionError::LedgerInPast);
    }
    let _ = admin::get_admin(env)?;

    let shares = balance::get_shares(env, user);
    let accrued = balance::get_accrued_reward(env, user);
    let checkpoint = balance::get_reward_checkpoint_ledger(env, user).unwrap_or(now);

    // Past stretch (checkpoint -> now): exactly what `calc_pending_reward`
    // adds today, using the stored rate that accrual actually used.
    let earned_so_far = VaultContract::reward_between_ledgers_at_rate(
        env, user, shares, checkpoint, now, false, None,
    )?;

    // Future stretch (now -> at_ledger): current position and rate
    // schedules held fixed. If a rate ramp is configured, use its
    // time-weighted average rate over the stretch; halving, boost tiers,
    // campaigns and seasons are applied segment by segment inside the math.
    let ramp_rate = crate::vault_extensions_538_541::average_rate_over(env, now, at_ledger);
    let future = VaultContract::reward_between_ledgers_at_rate(
        env, user, shares, now, at_ledger, false, ramp_rate,
    )?;

    let raw = accrued
        .checked_add(earned_so_far)
        .and_then(|v| v.checked_add(future))
        .ok_or(ProjectionError::ArithmeticError)?;

    // Same read-side gates `calc_pending_reward` applies, evaluated at the
    // target ledger instead of the current one.
    let gated = match crate::vesting_cliff::cliff_unlock_ledger_for(env, user) {
        Some(unlock_at) if !crate::ledger_boundary::is_reached(at_ledger, unlock_at) => 0,
        _ => raw,
    };

    let lock: Option<(u32, u32)> = env
        .storage()
        .persistent()
        .get(&(symbol_short!("p_lock"), user.clone()));
    let boosted = match lock {
        Some((unlocks_at, boost_bps))
            if boost_bps > 0 && crate::ledger_boundary::is_before(at_ledger, unlocks_at) =>
        {
            gated
                .checked_mul(crate::constants::BPS_DENOMINATOR + boost_bps as i128)
                .and_then(|v| v.checked_div(crate::constants::BPS_DENOMINATOR))
                .ok_or(ProjectionError::ArithmeticError)?
        }
        _ => gated,
    };

    Ok(VaultContract::normalize_to_reward_decimals(env, boosted)?)
}

#[contractimpl]
impl VaultContract {
    /// Read-only estimate of the reward `user` will have pending at
    /// `at_ledger`, in reward-token decimals.
    ///
    /// **This is a projection, not a promise.** It assumes the user's
    /// position (shares, boost tier clock, lock) and the pool's reward
    /// configuration stay exactly as they are now. Scheduled changes that are
    /// already configured are honoured: halving steps, boost-tier steps,
    /// campaigns, seasons, the bootstrap window, a reward-rate ramp (using its
    /// average rate over the stretch) and a vesting cliff. Anything that has
    /// not happened yet is not modelled: deposits, withdrawals and claims by
    /// the user or others, admin rate changes, the epoch emission budget, and
    /// the reward pool running dry. With a rate ramp active the result can be
    /// off by up to one reward unit of rounding.
    ///
    /// Calling it with `at_ledger` equal to the current ledger returns the
    /// same figure as `calc_pending_reward`.
    ///
    /// # Authorization
    /// None. Read-only.
    ///
    /// # Errors
    /// - `LedgerInPast`: `at_ledger` is earlier than the current ledger.
    /// - `NotInitialized`: no admin is stored.
    /// - `ArithmeticError`: an intermediate value overflowed.
    pub fn project_reward(env: Env, user: Address, at_ledger: u32) -> Result<i128, ProjectionError> {
        project(&env, &user, at_ledger)
    }
}
