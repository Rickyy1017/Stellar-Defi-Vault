//! Multi-recipient reward claim splitting (issue #831).
//!
//! A staker can route their reward payouts to up to four recipients by
//! configuring basis-point shares with `set_claim_split`. Every time a claim
//! actually pays out (the direct-transfer branch of `do_claim_to`), the payout
//! is divided by those shares; the rounding remainder from the floor division
//! is added to the first recipient so the full payout is always distributed.
//!
//! A configured split takes precedence over the payout destination passed to
//! the claim, and over the user's default claim destination (see
//! `set_default_claim_destination`). Paths that never move tokens at claim time
//! — vesting queues, escrow release, reward reinvestment and auto-conversion —
//! keep their existing behaviour and are not split.
//!
//! # Storage
//!
//! `DataKey` is at Soroban's 50-variant cap, so this uses raw `Symbol`-keyed
//! persistent storage, matching `balance.rs` and `auto_claim_on_withdraw.rs`.

use soroban_sdk::{contractimpl, symbol_short, token, Address, Env, Symbol, Vec};

use crate::constants::BPS_DENOMINATOR;
use crate::errors::VaultSafetyError;
use crate::vault::VaultContract;

/// Persistent key prefix for a staker's claim split.
const SPLIT_KEY: Symbol = symbol_short!("clm_splt");
/// Persistent key prefix for a staker's default claim destination.
const DEFAULT_DEST_KEY: Symbol = symbol_short!("clm_dest");

/// The most recipients a single claim split may route to.
const MAX_SPLIT_RECIPIENTS: u32 = 4;

fn split_key(user: &Address) -> (Symbol, Address) {
    (SPLIT_KEY, user.clone())
}

fn dest_key(user: &Address) -> (Symbol, Address) {
    (DEFAULT_DEST_KEY, user.clone())
}

/// The staker's configured split, or an empty vector when none is set.
pub fn get_split(env: &Env, user: &Address) -> Vec<(Address, u32)> {
    env.storage()
        .persistent()
        .get(&split_key(user))
        .unwrap_or_else(|| Vec::new(env))
}

/// The staker's default claim destination, if one is set.
pub fn get_default_destination(env: &Env, user: &Address) -> Option<Address> {
    env.storage().persistent().get(&dest_key(user))
}

/// Pay `payout` out of the contract for `staker`'s claim.
///
/// A configured split takes precedence; each recipient receives
/// `floor(payout * bps / 10_000)`, and the first recipient also receives any
/// rounding remainder. With no split, the payout goes to `destination`, or to
/// the staker's default claim destination when the call was self-directed
/// (`destination == staker`) and a default is set.
pub(crate) fn pay_reward(
    env: &Env,
    staker: &Address,
    destination: &Address,
    payout: i128,
    token_addr: &Address,
) {
    if payout <= 0 {
        return;
    }
    let token_client = token::Client::new(env, token_addr);
    let contract = env.current_contract_address();

    let split = get_split(env, staker);
    if split.is_empty() {
        let resolved = if destination == staker {
            get_default_destination(env, staker).unwrap_or_else(|| staker.clone())
        } else {
            destination.clone()
        };
        token_client.transfer(&contract, &resolved, &payout);
        return;
    }

    let mut distributed: i128 = 0;
    let mut amounts: Vec<(Address, i128)> = Vec::new(env);
    for i in 0..split.len() {
        if let Some((recipient, bps)) = split.get(i) {
            let amount =
                crate::math::mul_div_floor(payout, bps as i128, BPS_DENOMINATOR).unwrap_or(0);
            distributed = distributed.saturating_add(amount);
            amounts.push_back((recipient, amount));
        }
    }

    // The floors can leave a little undistributed; give it to the first
    // recipient so the whole `payout` leaves the contract.
    let remainder = payout.saturating_sub(distributed);
    for i in 0..amounts.len() {
        if let Some((recipient, amount)) = amounts.get(i) {
            let amount = if i == 0 {
                amount.saturating_add(remainder)
            } else {
                amount
            };
            if amount > 0 {
                token_client.transfer(&contract, &recipient, &amount);
            }
        }
    }

    env.events().publish(
        (symbol_short!("clm_split"), staker.clone()),
        (payout, amounts.len()),
    );
}

#[contractimpl]
impl VaultContract {
    /// Set (or replace) the caller's reward claim split (issue #831). The
    /// recipient shares must be between one and four entries and sum to exactly
    /// 10 000 bps.
    ///
    /// # Authorization
    /// `user` must sign.
    ///
    /// # Errors
    /// - `InvalidClaimSplitCount`: zero recipients or more than four.
    /// - `InvalidClaimSplitTotal`: the shares do not sum to 10 000 bps.
    ///
    /// # Events
    /// Emits `clm_split_set` with `(recipients, ledger)`.
    pub fn set_claim_split(
        env: Env,
        user: Address,
        recipients: Vec<(Address, u32)>,
    ) -> Result<(), VaultSafetyError> {
        user.require_auth();
        let count = recipients.len();
        if count == 0 || count > MAX_SPLIT_RECIPIENTS {
            return Err(VaultSafetyError::InvalidClaimSplitCount);
        }
        crate::vec_unique::ensure_unique_address_pairs_u32(&recipients)
            .map_err(|_| VaultSafetyError::DuplicateEntry)?;
        let mut total: u32 = 0;
        for i in 0..count {
            if let Some((_, bps)) = recipients.get(i) {
                total = total.saturating_add(bps);
            }
        }
        if total != crate::constants::BPS_DENOMINATOR_U32 {
            return Err(VaultSafetyError::InvalidClaimSplitTotal);
        }
        env.storage()
            .persistent()
            .set(&split_key(&user), &recipients);
        env.events().publish(
            (symbol_short!("clm_split_set"), user),
            (count, env.ledger().sequence()),
        );
        Ok(())
    }

    /// Remove the caller's reward claim split (issue #831).
    ///
    /// # Authorization
    /// `user` must sign.
    ///
    /// # Events
    /// Emits `clm_split_clr` with the ledger.
    pub fn clear_claim_split(env: Env, user: Address) -> Result<(), VaultSafetyError> {
        user.require_auth();
        env.storage().persistent().remove(&split_key(&user));
        env.events().publish(
            (symbol_short!("clm_split_clr"), user),
            env.ledger().sequence(),
        );
        Ok(())
    }

    /// Read-only: the caller's configured claim split as `(recipient, bps)`
    /// pairs; empty when none is set (issue #831).
    pub fn get_claim_split(env: Env, user: Address) -> Vec<(Address, u32)> {
        get_split(&env, &user)
    }

    /// Set (or clear with `None`) the caller's default claim destination
    /// (issue #831). When set, self-directed claims that are not otherwise
    /// routed pay out here. A configured claim split takes precedence.
    ///
    /// # Authorization
    /// `user` must sign.
    ///
    /// # Events
    /// Emits `clm_dest_set` with the ledger.
    pub fn set_default_claim_destination(
        env: Env,
        user: Address,
        destination: Option<Address>,
    ) -> Result<(), VaultSafetyError> {
        user.require_auth();
        match destination {
            Some(dest) => env.storage().persistent().set(&dest_key(&user), &dest),
            None => env.storage().persistent().remove(&dest_key(&user)),
        }
        env.events()
            .publish((symbol_short!("clm_dest_set"), user), env.ledger().sequence());
        Ok(())
    }

    /// Read-only: the caller's default claim destination, if any (issue #831).
    pub fn get_default_claim_destination(env: Env, user: Address) -> Option<Address> {
        get_default_destination(&env, &user)
    }
}
