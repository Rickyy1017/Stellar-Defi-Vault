//! Reward campaign window: explicit start and end ledgers for emissions.
//!
//! Without a window, rewards accrue for as long as the pool is staked. The
//! admin can instead set a window `[start_ledger, end_ledger)`:
//!
//! - **Start is inclusive, end is exclusive.** Rewards accrue for the ledger
//!   intervals from `start_ledger` up to, but not including, `end_ledger`, so a
//!   window of `[2000, 3000)` emits for exactly 1000 ledgers. At
//!   `start_ledger` itself nothing has accrued yet; at and after `end_ledger`
//!   nothing more does. (Same convention as the other ledger boundaries, see
//!   `docs/LEDGER_BOUNDARIES.md`.)
//! - **Outside the window accrual is zero; inside it is unchanged.** The
//!   window only clips *when* the normal reward math runs, so halving, boost
//!   tiers, campaigns, seasons and the rate are all unaffected.
//! - **Changing the window settles first.** Accrual is otherwise computed
//!   lazily from each user's last checkpoint, so replacing the window would
//!   silently re-price time that has already passed. `set_reward_window` and
//!   `clear_reward_window` first settle every staker up to the current ledger
//!   under the *old* window, then store the new one.
//!
//! This module is declared as a child of `vault` (see the tail of `vault.rs`)
//! so it can reuse the private accrual helpers.
//!
//! # Storage
//!
//! `DataKey` is at Soroban's 50-variant cap, so this uses raw `Symbol`-keyed
//! instance storage, matching the other extension modules.

use soroban_sdk::{contractimpl, symbol_short, Address, Env, Symbol};

use crate::admin;
use crate::balance;
use crate::errors::RewardWindowError;
use crate::vault::VaultContract;

const WINDOW_KEY: Symbol = symbol_short!("rw_win");

/// The configured `(start_ledger, end_ledger)` window, if any.
pub fn get_window(env: &Env) -> Option<(u32, u32)> {
    env.storage().instance().get(&WINDOW_KEY)
}

/// The part of `[start, end)` during which rewards may accrue, or `None` if
/// there is none. With no window configured this is `[start, end)` itself.
pub fn overlap(env: &Env, start: u32, end: u32) -> Option<(u32, u32)> {
    if end <= start {
        return None;
    }
    match get_window(env) {
        None => Some((start, end)),
        Some((window_start, window_end)) => {
            let from = start.max(window_start);
            let to = end.min(window_end);
            if to > from {
                Some((from, to))
            } else {
                None
            }
        }
    }
}

/// Settle every staker's accrual up to the current ledger under the window
/// that is stored right now.
fn settle_all(env: &Env) -> Result<(), RewardWindowError> {
    crate::reward_accumulator::update_global_accumulator(env)?;
    for user in balance::get_all_stakers(env).iter() {
        let shares = balance::get_shares(env, &user);
        if shares > 0 {
            VaultContract::accrue_rewards(env, &user, shares)?;
        }
    }
    Ok(())
}

/// Log the window change (old and new start/end) and emit `rw_window`.
fn record_change(
    env: &Env,
    admin: &Address,
    old: Option<(u32, u32)>,
    new: Option<(u32, u32)>,
) {
    let (old_start, old_end) = old.unwrap_or((0, 0));
    let (new_start, new_end) = new.unwrap_or((0, 0));
    crate::vault_extensions_463_466::log_parameter_change(
        env,
        admin,
        "reward_window_start",
        old_start as i128,
        new_start as i128,
    );
    crate::vault_extensions_463_466::log_parameter_change(
        env,
        admin,
        "reward_window_end",
        old_end as i128,
        new_end as i128,
    );
    env.events().publish(
        (symbol_short!("rw_window"),),
        (admin.clone(), new_start, new_end, env.ledger().sequence()),
    );
}

#[contractimpl]
impl VaultContract {
    /// Admin: restrict reward accrual to the ledger window
    /// `[start_ledger, end_ledger)` (start inclusive, end exclusive).
    ///
    /// Accrual is zero before `start_ledger` and from `end_ledger` onward, and
    /// normal in between. If a window is already set, or rewards have been
    /// accruing without one, every staker is first settled up to the current
    /// ledger under the previous rules, so the change only affects time that
    /// has not happened yet. That settlement touches every staker, so its cost
    /// grows with the number of stakers.
    ///
    /// # Authorization
    /// `admin` must be the stored admin and must sign.
    ///
    /// # Errors
    /// - `NotInitialized`: no admin is stored.
    /// - `Unauthorized`: `admin` is not the stored admin.
    /// - `InvalidWindow`: `end_ledger` is not after `start_ledger`.
    /// - `ArithmeticError`: settlement overflowed.
    ///
    /// # Events
    /// - Topic `("rw_window",)` with data `(admin, start_ledger, end_ledger, ledger)`.
    ///
    /// Also logged in the parameter change log as `reward_window_start` and
    /// `reward_window_end` (`0` meaning no window).
    pub fn set_reward_window(
        env: Env,
        admin: Address,
        start_ledger: u32,
        end_ledger: u32,
    ) -> Result<(), RewardWindowError> {
        admin::require_admin_as(&env, &admin)?;
        if end_ledger <= start_ledger {
            return Err(RewardWindowError::InvalidWindow);
        }

        settle_all(&env)?;

        let old = get_window(&env);
        env.storage()
            .instance()
            .set(&WINDOW_KEY, &(start_ledger, end_ledger));
        record_change(&env, &admin, old, Some((start_ledger, end_ledger)));
        Ok(())
    }

    /// Admin: remove the reward window so rewards accrue without a start or
    /// end again. Settles every staker first, like `set_reward_window`.
    ///
    /// # Authorization
    /// `admin` must be the stored admin and must sign.
    ///
    /// # Errors
    /// - `NotInitialized` / `Unauthorized`: admin check failed.
    /// - `NoWindow`: no window is set.
    /// - `ArithmeticError`: settlement overflowed.
    ///
    /// # Events
    /// - Topic `("rw_window",)` with data `(admin, 0, 0, ledger)`.
    pub fn clear_reward_window(env: Env, admin: Address) -> Result<(), RewardWindowError> {
        admin::require_admin_as(&env, &admin)?;
        let old = get_window(&env).ok_or(RewardWindowError::NoWindow)?;

        settle_all(&env)?;

        env.storage().instance().remove(&WINDOW_KEY);
        record_change(&env, &admin, Some(old), None);
        Ok(())
    }

    /// Read-only: the configured reward window as
    /// `Some((start_ledger, end_ledger))` (start inclusive, end exclusive),
    /// or `None` if rewards are not windowed.
    pub fn get_reward_window(env: Env) -> Option<(u32, u32)> {
        get_window(&env)
    }
}
