//! Loyalty points system (issue #392).
//!
//! Non-transferable, pool-internal points earned alongside staking rewards.
//! Points are awarded for actions (staking duration, claims, milestones,
//! governance, referrals) and can be redeemed for pool benefits.
//!
//! # Storage
//!
//! `DataKey` is at Soroban's 50-variant cap (`storage.rs:72-80`), so this
//! module uses raw `Symbol`-keyed storage, matching `balance.rs` and
//! `reputation_decay.rs` / `partial_freeze.rs`.

use soroban_sdk::{contractimpl, symbol_short, Address, Env, Symbol, Vec};

use crate::admin;
use crate::balance;
use crate::errors::VaultError;
use crate::storage::{PointsAction, PointsBenefit, PointsRule};
use crate::vault::{VaultContract, VaultContractClient};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/// Instance-storage key for the global points rules.
const LOY_CFG_KEY: Symbol = symbol_short!("loy_cfg");
/// Persistent-storage key for per-user current balance.
const LOY_BAL_KEY: Symbol = symbol_short!("loy_bal");
/// Persistent-storage key for per-user lifetime earned.
const LOY_EARN_KEY: Symbol = symbol_short!("loy_earn");

// Types are defined in storage.rs to allow events.rs to reference them.

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

fn get_rules(env: &Env) -> Vec<PointsRule> {
    env.storage()
        .instance()
        .get(&LOY_CFG_KEY)
        .unwrap_or(Vec::new(env))
}

fn set_rules(env: &Env, rules: &Vec<PointsRule>) {
    env.storage().instance().set(&LOY_CFG_KEY, rules);
}

fn get_balance(env: &Env, user: &Address) -> u32 {
    env.storage()
        .persistent()
        .get(&(LOY_BAL_KEY, user.clone()))
        .unwrap_or(0)
}

fn set_balance(env: &Env, user: &Address, bal: u32) {
    env.storage()
        .persistent()
        .set(&(LOY_BAL_KEY, user.clone()), &bal);
}

fn get_lifetime(env: &Env, user: &Address) -> u32 {
    env.storage()
        .persistent()
        .get(&(LOY_EARN_KEY, user.clone()))
        .unwrap_or(0)
}

fn set_lifetime(env: &Env, user: &Address, lifetime: u32) {
    env.storage()
        .persistent()
        .set(&(LOY_EARN_KEY, user.clone()), &lifetime);
}

/// Internal helper â€” adds `amount` points to `user`'s balance and lifetime.
/// Mirrors `balance::add_user_total_claimed` pattern `src/balance.rs:520-530`.
pub(crate) fn award_points(env: &Env, user: &Address, amount: u32) {
    if amount == 0 {
        return;
    }
    let bal = get_balance(env, user);
    let lifetime = get_lifetime(env, user);
    let new_bal = bal.checked_add(amount).unwrap_or(u32::MAX);
    let new_lifetime = lifetime.checked_add(amount).unwrap_or(u32::MAX);
    set_balance(env, user, new_bal);
    set_lifetime(env, user, new_lifetime);
    env.events().publish(
        (symbol_short!("loy_awd"), user.clone()),
        (amount, new_bal, env.ledger().sequence()),
    );
}

/// Award points for a specific action using the configured rules.
/// For `PerLedgerStaked`, `amount` is calculated as `elapsed * points_per_action`.
pub(crate) fn award_points_for_action(env: &Env, user: &Address, action: PointsAction) {
    let rules = get_rules(env);
    let mut points_per_action: Option<u32> = None;
    for r in rules.iter() {
        if r.action == action {
            points_per_action = Some(r.points_per_action);
            break;
        }
    }
    let Some(per_action) = points_per_action else {
        return;
    };
    if per_action == 0 {
        return;
    }
    let amount = match action {
        PointsAction::PerLedgerStaked => {
            // Per spec: awarded on claim based on elapsed ledgers since last claim.
            let last_claim = balance::get_last_claim_ledger(env, user);
            let current = env.ledger().sequence();
            let elapsed = current.saturating_sub(last_claim);
            // points = elapsed * per_action (per ledger)
            per_action.checked_mul(elapsed).unwrap_or(u32::MAX)
        }
        _ => per_action,
    };
    award_points(env, user, amount);
}

// ---------------------------------------------------------------------------
// Contract entrypoints
// ---------------------------------------------------------------------------

#[cfg_attr(not(test), contractimpl)]
impl VaultContract {
    /// Configure points earning rules. Admin only.
    pub fn set_points_rules(
        env: Env,
        admin: Address,
        rules: Vec<PointsRule>,
    ) -> Result<(), VaultError> {
        admin.require_auth();
        admin::require_admin(&env)?;
        if admin != crate::admin::get_admin(&env)? {
            return Err(VaultError::Unauthorized);
        }
        // Validate no duplicate actions and points_per_action >0
        let mut seen: u32 = 0;
        for r in rules.iter() {
            if r.points_per_action == 0 {
                return Err(VaultError::InvalidRate);
            }
            let flag = match r.action {
                PointsAction::PerLedgerStaked => 1,
                PointsAction::PerClaim => 2,
                PointsAction::PerGovernanceVote => 4,
                PointsAction::PerMilestone => 8,
                PointsAction::PerReferral => 16,
            };
            if seen & flag != 0 {
                return Err(VaultError::InvalidRate);
            }
            seen |= flag;
        }
        set_rules(&env, &rules);
        env.events().publish(
            (symbol_short!("loy_cfg"),),
            (rules, env.ledger().sequence()),
        );
        Ok(())
    }

    /// Read-only: current rules.
    pub fn get_points_rules(env: Env) -> Vec<PointsRule> {
        get_rules(&env)
    }

    /// Redeem points for a benefit. User must have sufficient balance.
    pub fn redeem_points(
        env: Env,
        user: Address,
        amount: u32,
        benefit: PointsBenefit,
    ) -> Result<(), VaultError> {
        user.require_auth();
        if amount == 0 {
            return Err(VaultError::ZeroAmount);
        }
        let bal = get_balance(&env, &user);
        if amount > bal {
            return Err(VaultError::InsufficientShares);
        }
        let new_bal = bal - amount;
        set_balance(&env, &user, new_bal);
        // Store benefit flag (pool-internal, non-transferable)
        let benefit_key = (
            Symbol::new(&env, "loy_bene"),
            user.clone(),
            benefit as u32,
        );
        env.storage()
            .persistent()
            .set(&benefit_key, &true);
        env.events().publish(
            (symbol_short!("loy_rdm"), user.clone()),
            (amount, benefit, new_bal, env.ledger().sequence()),
        );
        Ok(())
    }

    /// Read-only query for points balances.
    pub fn get_loyalty_points(env: Env, user: Address) -> (u32, u32) {
        (get_balance(&env, &user), get_lifetime(&env, &user))
    }

    /// Check if user has redeemed a specific benefit.
    pub fn has_loyalty_benefit(env: Env, user: Address, benefit: PointsBenefit) -> bool {
        let key = (
            Symbol::new(&env, "loy_bene"),
            user.clone(),
            benefit as u32,
        );
        env.storage().persistent().get(&key).unwrap_or(false)
    }
}


