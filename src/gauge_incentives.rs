//! Bribe and incentive market for gauge votes (issue #936).
//!
//! Third-party sponsors deposit incentive tokens for a specific
//! `(sub_pool_id, epoch)` pair. Voters who cast gauge votes via
//! [`VaultContract::vote_gauge`] during `epoch` earn a share of the incentive
//! pool proportional to their vote weight for `sub_pool_id` in that epoch.
//! Once `epoch` ends and the claim window expires, sponsors can recover any
//! unclaimed tokens.

use soroban_sdk::{
    contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol, Vec,
};

use crate::balance;
use crate::math::mul_div_floor;
use crate::vault::VaultContract;

/// Duration of one gauge incentive epoch in ledgers (~7 days at 5s/ledger).
pub const GAUGE_EPOCH_LEDGERS: u32 = 120_960;

/// Claim window after an epoch ends before sponsors may recover unclaimed
/// incentives (~30 days at 5s/ledger).
pub const INCENTIVE_EXPIRY_LEDGERS: u32 = 518_400;

const GI_POOL_KEY: Symbol = symbol_short!("gi_pool");
const GI_SPON_KEY: Symbol = symbol_short!("gi_spon");
const GI_CLAIM_KEY: Symbol = symbol_short!("gi_clm");
const GI_VWT_KEY: Symbol = symbol_short!("gi_vwt");
const GI_TWT_KEY: Symbol = symbol_short!("gi_twt");
const GI_VPOOLS_KEY: Symbol = symbol_short!("gi_vpl");

/// Errors returned by gauge incentive market entrypoints.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum GaugeIncentiveError {
    /// Incentive amount must be strictly positive.
    InvalidAmount = 93601,
    /// Cannot add incentives to an epoch that has already ended.
    EpochAlreadyEnded = 93602,
    /// Incentives cannot be claimed until the target epoch has ended.
    EpochNotEnded = 93603,
    /// No incentive pool exists for the given `(sub_pool_id, token, epoch)`.
    IncentiveNotFound = 93604,
    /// Caller has no recorded vote weight for this sub-pool in `epoch`.
    NoVoteWeight = 93605,
    /// Caller has already claimed their incentive share for this pool/epoch.
    AlreadyClaimed = 93606,
    /// Claim window has not expired yet; sponsor cannot recover yet.
    NotExpired = 93607,
    /// Nothing left to recover for this sponsor.
    NothingToRecover = 93608,
    /// Arithmetic overflow or division by zero.
    ArithmeticError = 93609,
}

/// Stored state for an incentive pool keyed by `(sub_pool_id, token, epoch)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncentivePool {
    /// Total tokens deposited across all sponsors for this pool/epoch.
    pub total_amount: i128,
    /// Total tokens already claimed by voters.
    pub claimed_amount: i128,
    /// Total tokens already recovered by sponsors after expiry.
    pub recovered_amount: i128,
}

/// Returns the current gauge epoch derived from the ledger sequence.
pub fn current_gauge_epoch(env: &Env) -> u32 {
    env.ledger().sequence() / GAUGE_EPOCH_LEDGERS
}

/// Returns the first ledger sequence after `epoch` ends.
pub fn epoch_end_ledger(epoch: u32) -> u32 {
    epoch
        .saturating_add(1)
        .saturating_mul(GAUGE_EPOCH_LEDGERS)
}

/// Returns the ledger sequence at or after which unclaimed incentives for
/// `epoch` become recoverable by sponsors.
pub fn incentive_expiry_ledger(epoch: u32) -> u32 {
    epoch_end_ledger(epoch).saturating_add(INCENTIVE_EXPIRY_LEDGERS)
}

/// Snapshot a voter's gauge vote weights for the current epoch.
///
/// Called by `VaultContract::vote_gauge`. If the voter already voted earlier
/// in the same epoch, their previous weights for that epoch are subtracted
/// from each sub-pool's epoch total before applying the new weights. Votes cast
/// in a later epoch never modify earlier epochs.
pub(crate) fn record_epoch_vote(env: &Env, user: &Address, allocations: &Vec<(u32, u32)>) {
    let epoch = current_gauge_epoch(env);
    let shares = balance::get_shares(env, user);

    // Clear previous weights recorded for this user in the current epoch.
    let prev_pools_key = (GI_VPOOLS_KEY, user.clone(), epoch);
    let prev_pools: Vec<u32> = env
        .storage()
        .persistent()
        .get(&prev_pools_key)
        .unwrap_or_else(|| Vec::new(env));
    for i in 0..prev_pools.len() {
        if let Some(pool_id) = prev_pools.get(i) {
            let v_key = (GI_VWT_KEY, user.clone(), pool_id, epoch);
            let old_weight: i128 = env.storage().persistent().get(&v_key).unwrap_or(0);
            if old_weight > 0 {
                let t_key = (GI_TWT_KEY, pool_id, epoch);
                let total: i128 = env.storage().persistent().get(&t_key).unwrap_or(0);
                env.storage()
                    .persistent()
                    .set(&t_key, &total.saturating_sub(old_weight));
                env.storage().persistent().remove(&v_key);
            }
        }
    }

    let mut new_pools: Vec<u32> = Vec::new(env);
    if shares > 0 {
        for i in 0..allocations.len() {
            if let Some((pool_id, bps)) = allocations.get(i) {
                let weight = shares.saturating_mul(bps as i128);
                if weight > 0 {
                    let v_key = (GI_VWT_KEY, user.clone(), pool_id, epoch);
                    let existing: i128 = env.storage().persistent().get(&v_key).unwrap_or(0);
                    env.storage()
                        .persistent()
                        .set(&v_key, &existing.saturating_add(weight));

                    let t_key = (GI_TWT_KEY, pool_id, epoch);
                    let total: i128 = env.storage().persistent().get(&t_key).unwrap_or(0);
                    env.storage()
                        .persistent()
                        .set(&t_key, &total.saturating_add(weight));

                    new_pools.push_back(pool_id);
                }
            }
        }
    }

    env.storage().persistent().set(&prev_pools_key, &new_pools);
}

#[contractimpl]
impl VaultContract {
    /// Deposit `amount` of `token` as a gauge vote incentive for
    /// `(sub_pool_id, epoch)`.
    ///
    /// # Authorization
    /// Requires authorization from `sponsor`.
    pub fn add_incentive(
        env: Env,
        sponsor: Address,
        sub_pool_id: u32,
        token: Address,
        amount: i128,
        epoch: u32,
    ) -> Result<(), GaugeIncentiveError> {
        sponsor.require_auth();
        if amount <= 0 {
            return Err(GaugeIncentiveError::InvalidAmount);
        }
        if epoch < current_gauge_epoch(&env) {
            return Err(GaugeIncentiveError::EpochAlreadyEnded);
        }

        let client = token::Client::new(&env, &token);
        client.transfer(&sponsor, &env.current_contract_address(), &amount);

        let pool_key = (GI_POOL_KEY, sub_pool_id, token.clone(), epoch);
        let mut pool: IncentivePool =
            env.storage()
                .persistent()
                .get(&pool_key)
                .unwrap_or(IncentivePool {
                    total_amount: 0,
                    claimed_amount: 0,
                    recovered_amount: 0,
                });

        pool.total_amount = pool
            .total_amount
            .checked_add(amount)
            .ok_or(GaugeIncentiveError::ArithmeticError)?;
        env.storage().persistent().set(&pool_key, &pool);

        let spon_key = (GI_SPON_KEY, sponsor.clone(), sub_pool_id, token.clone(), epoch);
        let prev_contrib: i128 = env.storage().persistent().get(&spon_key).unwrap_or(0);
        let new_contrib = prev_contrib
            .checked_add(amount)
            .ok_or(GaugeIncentiveError::ArithmeticError)?;
        env.storage().persistent().set(&spon_key, &new_contrib);

        env.events().publish(
            (symbol_short!("gi_add"), sponsor, sub_pool_id),
            (token, amount, epoch, env.ledger().sequence()),
        );

        Ok(())
    }

    /// Claim the caller's proportional share of gauge incentives for
    /// `(sub_pool_id, token, epoch)`.
    ///
    /// Only callable after `epoch` has ended so total epoch vote weights are
    /// final.
    pub fn claim_incentive(
        env: Env,
        voter: Address,
        sub_pool_id: u32,
        token: Address,
        epoch: u32,
    ) -> Result<i128, GaugeIncentiveError> {
        voter.require_auth();
        if current_gauge_epoch(&env) <= epoch {
            return Err(GaugeIncentiveError::EpochNotEnded);
        }

        let claim_key = (GI_CLAIM_KEY, voter.clone(), sub_pool_id, token.clone(), epoch);
        if env.storage().persistent().has(&claim_key) {
            return Err(GaugeIncentiveError::AlreadyClaimed);
        }

        let pool_key = (GI_POOL_KEY, sub_pool_id, token.clone(), epoch);
        let mut pool: IncentivePool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .ok_or(GaugeIncentiveError::IncentiveNotFound)?;

        let voter_weight: i128 = env
            .storage()
            .persistent()
            .get(&(GI_VWT_KEY, voter.clone(), sub_pool_id, epoch))
            .unwrap_or(0);
        let total_weight: i128 = env
            .storage()
            .persistent()
            .get(&(GI_TWT_KEY, sub_pool_id, epoch))
            .unwrap_or(0);

        if voter_weight <= 0 || total_weight <= 0 {
            return Err(GaugeIncentiveError::NoVoteWeight);
        }

        let raw_share = mul_div_floor(pool.total_amount, voter_weight, total_weight)
            .map_err(|_| GaugeIncentiveError::ArithmeticError)?;
        let remaining = pool
            .total_amount
            .saturating_sub(pool.claimed_amount)
            .saturating_sub(pool.recovered_amount);
        let payout = raw_share.min(remaining);

        env.storage().persistent().set(&claim_key, &true);
        pool.claimed_amount = pool
            .claimed_amount
            .checked_add(payout)
            .ok_or(GaugeIncentiveError::ArithmeticError)?;
        env.storage().persistent().set(&pool_key, &pool);

        if payout > 0 {
            token::Client::new(&env, &token).transfer(
                &env.current_contract_address(),
                &voter,
                &payout,
            );
        }

        env.events().publish(
            (symbol_short!("gi_claim"), voter, sub_pool_id),
            (token, payout, epoch, env.ledger().sequence()),
        );

        Ok(payout)
    }

    /// Recover unclaimed incentives for `(sub_pool_id, token, epoch)` after the
    /// claim window has expired.
    ///
    /// # Authorization
    /// Requires authorization from `sponsor`.
    pub fn recover_expired_incentive(
        env: Env,
        sponsor: Address,
        sub_pool_id: u32,
        token: Address,
        epoch: u32,
    ) -> Result<i128, GaugeIncentiveError> {
        sponsor.require_auth();
        if env.ledger().sequence() < incentive_expiry_ledger(epoch) {
            return Err(GaugeIncentiveError::NotExpired);
        }

        let pool_key = (GI_POOL_KEY, sub_pool_id, token.clone(), epoch);
        let mut pool: IncentivePool = env
            .storage()
            .persistent()
            .get(&pool_key)
            .ok_or(GaugeIncentiveError::IncentiveNotFound)?;

        let spon_key = (GI_SPON_KEY, sponsor.clone(), sub_pool_id, token.clone(), epoch);
        let contributed: i128 = env.storage().persistent().get(&spon_key).unwrap_or(0);
        if contributed <= 0 {
            return Err(GaugeIncentiveError::NothingToRecover);
        }

        let unclaimed = pool
            .total_amount
            .saturating_sub(pool.claimed_amount)
            .saturating_sub(pool.recovered_amount);
        if unclaimed <= 0 {
            return Err(GaugeIncentiveError::NothingToRecover);
        }

        let recoverable = mul_div_floor(unclaimed, contributed, pool.total_amount)
            .map_err(|_| GaugeIncentiveError::ArithmeticError)?;
        let recoverable = recoverable.min(unclaimed);
        if recoverable <= 0 {
            return Err(GaugeIncentiveError::NothingToRecover);
        }

        env.storage().persistent().set(&spon_key, &0_i128);
        pool.recovered_amount = pool
            .recovered_amount
            .checked_add(recoverable)
            .ok_or(GaugeIncentiveError::ArithmeticError)?;
        env.storage().persistent().set(&pool_key, &pool);

        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &sponsor,
            &recoverable,
        );

        env.events().publish(
            (symbol_short!("gi_recov"), sponsor, sub_pool_id),
            (token, recoverable, epoch, env.ledger().sequence()),
        );

        Ok(recoverable)
    }

    /// Read-only query for an incentive pool's totals.
    pub fn get_incentive_pool(
        env: Env,
        sub_pool_id: u32,
        token: Address,
        epoch: u32,
    ) -> Option<IncentivePool> {
        env.storage()
            .persistent()
            .get(&(GI_POOL_KEY, sub_pool_id, token, epoch))
    }

    /// Read-only query for a voter's recorded weight in `(sub_pool_id, epoch)`.
    pub fn get_epoch_voter_weight(
        env: Env,
        voter: Address,
        sub_pool_id: u32,
        epoch: u32,
    ) -> i128 {
        env.storage()
            .persistent()
            .get(&(GI_VWT_KEY, voter, sub_pool_id, epoch))
            .unwrap_or(0)
    }
}
