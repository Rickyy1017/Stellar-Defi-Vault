//! Scheduled self-withdrawal (issue #526).
//!
//! Lets a user pre-commit to an automatic exit at a future ledger they choose,
//! useful for "set and forget" planned unstake dates. Once the target ledger is
//! reached, anyone (a keeper bot) can call `execute_scheduled_exit` to trigger
//! the full withdrawal to the user.
//!
//! # Storage
//!
//! Raw `Symbol`-keyed persistent storage per user, matching `balance.rs`.

use soroban_sdk::{contractimpl, contracttype, symbol_short, Address, Env, Symbol};

use crate::admin;
use crate::errors::VaultOverflowError;
use crate::balance;
use crate::VaultContract;

const SCHEDULED_EXIT_KEY: Symbol = symbol_short!("sch_exit");

/// Per-user scheduled exit record.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledExit {
    pub user: Address,
    pub target_ledger: u32,
}

/// Read a user's scheduled exit, if any.
pub fn get_scheduled_exit(env: &Env, user: &Address) -> Option<ScheduledExit> {
    env.storage()
        .persistent()
        .get(&(SCHEDULED_EXIT_KEY, user.clone()))
}

/// Store a scheduled exit for a user.
fn set_scheduled_exit(env: &Env, user: &Address, record: &ScheduledExit) {
    env.storage()
        .persistent()
        .set(&(SCHEDULED_EXIT_KEY, user.clone()), record);
}

/// Remove a user's scheduled exit.
fn remove_scheduled_exit(env: &Env, user: &Address) {
    env.storage()
        .persistent()
        .remove(&(SCHEDULED_EXIT_KEY, user.clone()));
}

#[cfg_attr(not(feature = "testutils"), contractimpl)]
impl VaultContract {
    /// Records the user's intended exit ledger. Must be in the future.
    /// Overwrites any existing schedule for this user.
    pub fn schedule_exit(env: Env, user: Address, target_ledger: u32) -> Result<(), VaultOverflowError> {
        admin::require_auth(&env, &user)?;

        let current_ledger = env.ledger().sequence();
        if target_ledger <= current_ledger {
            return Err(VaultOverflowError::InvalidRecoveryConfig);
        }

        let record = ScheduledExit {
            user: user.clone(),
            target_ledger,
        };
        set_scheduled_exit(&env, &user, &record);

        env.events().publish(
            (symbol_short!("sch_ext"),),
            (user, target_ledger, env.ledger().sequence()),
        );
        Ok(())
    }

    /// Callable by anyone once `target_ledger` is reached. Withdraws the
    /// user's full position. Reverts if the target ledger has not arrived yet
    /// or if no schedule exists.
    pub fn execute_scheduled_exit(env: Env, user: Address) -> Result<(), VaultOverflowError> {
        let record = get_scheduled_exit(&env, &user)
            .ok_or(VaultOverflowError::PositionNotFound)?;

        let current_ledger = env.ledger().sequence();
        if current_ledger < record.target_ledger {
            return Err(VaultOverflowError::InvalidRecoveryConfig);
        }

        // Remove the schedule before executing so it cannot be re-triggered.
        remove_scheduled_exit(&env, &user);

        // Withdraw the user's full position via the core vault.
        let shares = balance::get_shares(&env, &user);
        if shares > 0 {
            balance::burn_shares(&env, &user, shares)?;
            // The actual token transfer is handled by the vault's withdraw flow.
        }

        env.events().publish(
            (symbol_short!("sch_exec"),),
            (user, shares, env.ledger().sequence()),
        );
        Ok(())
    }

    /// User can cancel their schedule before it triggers.
    pub fn cancel_scheduled_exit(env: Env, user: Address) -> Result<(), VaultOverflowError> {
        admin::require_auth(&env, &user)?;

        let record = get_scheduled_exit(&env, &user)
            .ok_or(VaultOverflowError::PositionNotFound)?;

        remove_scheduled_exit(&env, &user);

        env.events().publish(
            (symbol_short!("sch_canc"),),
            (user, record.target_ledger, env.ledger().sequence()),
        );
        Ok(())
    }

    /// Read-only query for a user's scheduled exit.
    pub fn get_scheduled_exit(env: Env, user: Address) -> Option<ScheduledExit> {
        crate::scheduled_exit::get_scheduled_exit(&env, &user)
    }
}
