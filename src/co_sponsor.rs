//! Third-party reward matching via co-sponsors (issue #529).
//!
//! Lets an external party (e.g. a partner protocol or token issuer) fund
//! additional matching rewards on top of the base reward rate, without
//! becoming an admin. Useful for co-marketing incentive campaigns.
//!
//! # Security
//!
//! A sponsor has no admin rights beyond funding their own registered matching
//! pool — they cannot alter core vault parameters, pause, slash, or modify
//! reward rates.
//!
//! # Storage
//!
//! Raw `Symbol`-keyed storage, matching `balance.rs`.

use soroban_sdk::{contractimpl, contracttype, symbol_short, Address, Env, Symbol};

use crate::admin;
use crate::errors::VaultOverflowError;
use crate::balance;
use crate::VaultContract;

const CO_SPONSOR_KEY: Symbol = symbol_short!("co_spg");
const CO_SPONSOR_FUND_KEY: Symbol = symbol_short!("co_spf");

/// A registered co-sponsor's configuration.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CoSponsor {
    pub sponsor: Address,
    pub match_bps: u32,
    pub expires_at: u32,
    pub registered_at: u32,
}

/// Read a co-sponsor record by address.
pub fn get_co_sponsor(env: &Env, sponsor: &Address) -> Option<CoSponsor> {
    env.storage()
        .persistent()
        .get(&(CO_SPONSOR_KEY, sponsor.clone()))
}

/// Store a co-sponsor record.
fn set_co_sponsor(env: &Env, record: &CoSponsor) {
    env.storage().persistent().set(
        &(CO_SPONSOR_KEY, record.sponsor.clone()),
        record,
    );
}

/// Read a co-sponsor's current fund balance.
pub fn get_co_sponsor_fund(env: &Env, sponsor: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&(CO_SPONSOR_FUND_KEY, sponsor.clone()))
        .unwrap_or(0)
}

/// Set a co-sponsor's fund balance.
fn set_co_sponsor_fund(env: &Env, sponsor: &Address, amount: i128) {
    env.storage().persistent().set(
        &(CO_SPONSOR_FUND_KEY, sponsor.clone()),
        &amount,
    );
}

/// Check if a co-sponsor is registered and not expired.
pub fn is_active_sponsor(env: &Env, sponsor: &Address) -> bool {
    match get_co_sponsor(env, sponsor) {
        Some(s) => {
            let current = env.ledger().sequence();
            s.expires_at > current
        }
        None => false,
    }
}

/// Calculate the matched reward amount for a given base reward, capped by the
/// sponsor's remaining fund balance. Returns the matched amount and deducts it
/// from the fund.
pub fn distribute_matched_reward(
    env: &Env,
    sponsor: &Address,
    base_reward: i128,
) -> Result<i128, VaultOverflowError> {
    let record = get_co_sponsor(env, sponsor)
        .ok_or(VaultOverflowError::PositionNotFound)?;

    let current = env.ledger().sequence();
    if record.expires_at <= current {
        return Err(VaultOverflowError::AlreadyInitialized);
    }

    let fund = get_co_sponsor_fund(env, sponsor);
    if fund <= 0 {
        return Ok(0);
    }

    // Matched reward = base_reward * match_bps / 10000, capped by fund.
    let matched = base_reward
        .checked_mul(record.match_bps as i128)
        .ok_or(VaultOverflowError::ArithmeticError)?
        .checked_div(10_000)
        .ok_or(VaultOverflowError::ArithmeticError)?;

    let actual = matched.min(fund);
    if actual > 0 {
        set_co_sponsor_fund(env, sponsor, fund - actual);
    }

    Ok(actual)
}

#[cfg_attr(not(feature = "testutils"), contractimpl)]
impl VaultContract {
    /// Admin approves a sponsor to contribute matched rewards.
    /// `match_bps` is the percentage of base reward to match (100 = 1%).
    /// `expires_at` is the ledger after which the sponsor can no longer fund.
    pub fn register_co_sponsor(
        env: Env,
        sponsor: Address,
        match_bps: u32,
        expires_at: u32,
    ) -> Result<(), VaultOverflowError> {
        admin::require_admin(&env)?;

        if match_bps == 0 || match_bps > 10_000 {
            return Err(VaultOverflowError::InvalidRecoveryConfig);
        }

        let current = env.ledger().sequence();
        if expires_at <= current {
            return Err(VaultOverflowError::InvalidRecoveryConfig);
        }

        let record = CoSponsor {
            sponsor: sponsor.clone(),
            match_bps,
            expires_at,
            registered_at: current,
        };
        set_co_sponsor(&env, &record);

        env.events().publish(
            (symbol_short!("co_reg"),),
            (sponsor, match_bps, expires_at, current),
        );
        Ok(())
    }

    /// Only the registered, non-expired sponsor can call this to fund their
    /// matching pool. Transfers `amount` from the sponsor to the contract.
    pub fn fund_co_sponsor_rewards(
        env: Env,
        sponsor: Address,
        amount: i128,
    ) -> Result<(), VaultOverflowError> {
        // Verify the caller is the sponsor.
        if sponsor != env_invoker(&env) {
            return Err(VaultOverflowError::Unauthorized);
        }

        if amount <= 0 {
            return Err(VaultOverflowError::ZeroAmount);
        }

        if !is_active_sponsor(&env, &sponsor) {
            return Err(VaultOverflowError::AlreadyInitialized);
        }

        // Transfer tokens from sponsor to contract.
        // The sponsor must have approved the contract to spend their tokens.
        let token_addr: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("token"))
            .ok_or(VaultOverflowError::NotInitialized)?;

        let token_client = crate::vault::VaultContractClient::new(&env, &token_addr);
        token_client.transfer(&sponsor, &env.current_contract_address(), &amount);

        let current_fund = get_co_sponsor_fund(&env, &sponsor);
        set_co_sponsor_fund(&env, &sponsor, current_fund + amount);

        env.events().publish(
            (symbol_short!("co_fund"),),
            (sponsor, amount, current_fund + amount, env.ledger().sequence()),
        );
        Ok(())
    }
}

/// Helper to get the transaction invoker address. In Soroban, this is the
/// address that authorized the invocation.
fn env_invoker(env: &Env) -> Address {
    env.invoker()
}
