//! External price oracle integration for collateral valuation (issue #528).
//!
//! Registers an oracle contract implementing a standard price-feed interface
//! (e.g. Reflector or similar Stellar-ecosystem oracle) so external protocols
//! can value a position in a reference currency (e.g. USD) rather than only in
//! the underlying deposit token.
//!
//! # Storage
//!
//! Raw `Symbol`-keyed instance storage, matching `balance.rs`.

use soroban_sdk::{contractimpl, symbol_short, Address, Env, Symbol};

use crate::admin;
use crate::errors::VaultOverflowError;
use crate::balance;
use crate::vault::VaultContractClient;
use crate::VaultContract;

const ORACLE_KEY: Symbol = symbol_short!("ext_orcl");

/// Register an oracle contract implementing a standard price-feed interface.
pub fn set_oracle(env: &Env, oracle: &Address) {
    env.storage().instance().set(&ORACLE_KEY, oracle);
}

/// Read the registered oracle address, if any.
pub fn get_oracle(env: &Env) -> Option<Address> {
    env.storage().instance().get(&ORACLE_KEY)
}

#[cfg_attr(not(feature = "testutils"), contractimpl)]
impl VaultContract {
    /// Admin registers an oracle contract for collateral valuation.
    pub fn set_price_oracle(env: Env, oracle: Address) -> Result<(), VaultOverflowError> {
        admin::require_admin(&env)?;

        set_oracle(&env, &oracle);

        env.events().publish(
            (symbol_short!("orcl_upd"),),
            (oracle, env.ledger().sequence()),
        );
        Ok(())
    }

    /// Returns the user's position value in USD by combining the local
    /// `preview_redeem` output with the oracle's current price.
    /// Reverts with `OracleNotConfigured` if no oracle is set.
    pub fn get_position_value_usd(env: Env, user: Address) -> Result<i128, VaultOverflowError> {
        let oracle_addr = get_oracle(&env)
            .ok_or(VaultOverflowError::NoOracleConfigured)?;

        // Get the user's redeemable value in the vault's token.
        let shares = balance::get_shares(&env, &user);
        if shares <= 0 {
            return Ok(0);
        }

        // Call the oracle's get_price() function to get the price per token in USD.
        // The oracle contract must implement: fn get_price(token: Address) -> i128
        // returning the price in the smallest unit (e.g. cents).
        let token_addr = env
            .storage()
            .instance()
            .get(&symbol_short!("token"))
            .ok_or(VaultOverflowError::NotInitialized)?;

        let oracle_client = VaultContractClient::new(&env, &oracle_addr);
        let price_per_token: i128 = oracle_client
            .try_get_price(&token_addr)
            .map_err(|_| VaultOverflowError::ArithmeticError)?
            .ok_or(VaultOverflowError::ArithmeticError)?;

        // Calculate total value: shares * price_per_token.
        // Assumes price is in 7-decimal fixed point (same as the vault's token).
        let value_usd = shares
            .checked_mul(price_per_token)
            .ok_or(VaultOverflowError::ArithmeticError)?;

        Ok(value_usd)
    }
}
