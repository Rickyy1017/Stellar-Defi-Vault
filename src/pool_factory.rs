//! Pool factory (issue #875).
//!
//! Lets the factory admin deploy new vault instances from a single audited
//! Wasm hash, with deterministic addresses derived from the current contract
//! and the caller-supplied salt. The pool is initialized atomically through
//! the vault's `__constructor`, so there is no window between deployment and
//! initialization where the new pool could be taken over.
//!
//! # Storage
//!
//! `DataKey` sits at Soroban's 50-variant cap, so this uses raw
//! `Symbol`-keyed storage, matching `balance.rs`.
//!
//! - `fa_wasm` -> `BytesN<32>`: the Wasm hash every new pool is deployed from.
//! - `(fa_pool, sha256(salt))` -> `Address`: the pool created for a salt.

use soroban_sdk::{
    contracterror, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN,
    ContractExecutable, Env, Symbol,
};

use crate::admin;
use crate::vault::{VaultContract, VaultContractArgs, VaultContractClient};

const WASM_HASH_KEY: Symbol = symbol_short!("fa_wasm");
const POOL_BY_SALT_KEY: Symbol = symbol_short!("fa_pool");

/// Maximum salt length accepted by `create_pool`, in bytes.
pub const MAX_POOL_SALT_LEN: u32 = 64;

/// Errors returned by the pool factory.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PoolFactoryError {
    /// The factory has no Wasm hash configured yet.
    WasmHashNotSet = 1,
    /// The caller is not the stored factory admin.
    Unauthorized = 2,
    /// The salt is longer than `MAX_POOL_SALT_LEN`.
    SaltTooLong = 3,
    /// A pool has already been created with this salt.
    SaltAlreadyUsed = 4,
}

/// Configuration applied to a pool deployed by the factory.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PoolFactoryConfig {
    /// Admin of the new pool.
    pub admin: Address,
    /// Token the new pool accepts for deposits.
    pub stake_token: Address,
    /// Optional reward token; `None` pays rewards in the stake token.
    pub reward_token: Option<Address>,
    /// Initial reward rate in basis points.
    pub reward_rate_bps: u32,
}

fn salt_hash(env: &Env, salt: &Bytes) -> BytesN<32> {
    env.crypto().sha256(salt).to_bytes()
}

/// Storage key for the pool created from `salt_hash`.
pub(crate) fn salt_key(salt_hash: &BytesN<32>) -> (Symbol, BytesN<32>) {
    (POOL_BY_SALT_KEY, salt_hash.clone())
}

/// The configured factory Wasm hash, if any.
pub fn get_wasm_hash(env: &Env) -> Option<BytesN<32>> {
    env.storage().instance().get(&WASM_HASH_KEY)
}

/// The pool recorded for `salt_hash`, if any.
pub(crate) fn get_pool_for_salt_hash(env: &Env, salt_hash: &BytesN<32>) -> Option<Address> {
    env.storage().persistent().get(&salt_key(salt_hash))
}

#[cfg_attr(not(feature = "testutils"), contractimpl)]
impl VaultContract {
    /// Admin: set the Wasm hash new pools are deployed from.
    ///
    /// # Authorization
    /// The stored admin must equal `admin_addr` and sign.
    ///
    /// # Errors
    /// - `Unauthorized`: `admin_addr` is not the stored admin.
    ///
    /// # Events
    /// `fa_set` with `(admin, wasm_hash)`.
    pub fn set_factory_wasm_hash(
        env: Env,
        admin_addr: Address,
        wasm_hash: BytesN<32>,
    ) -> Result<(), PoolFactoryError> {
        admin::require_admin_as(&env, &admin_addr).map_err(|_| PoolFactoryError::Unauthorized)?;
        env.storage().instance().set(&WASM_HASH_KEY, &wasm_hash);
        env.events()
            .publish((symbol_short!("fa_set"),), (admin_addr, wasm_hash));
        Ok(())
    }

    /// The Wasm hash new pools are deployed from, or `None` if it has not been
    /// configured.
    pub fn get_factory_wasm_hash(env: Env) -> Option<BytesN<32>> {
        get_wasm_hash(&env)
    }

    /// Deploy and initialize a new vault from the configured Wasm hash.
    ///
    /// The address is deterministic: the same factory contract and salt always
    /// produce the same pool address, and reusing a salt is rejected.
    ///
    /// # Authorization
    /// `deployer` must be the stored factory admin and sign.
    ///
    /// # Errors
    /// - `WasmHashNotSet`: `set_factory_wasm_hash` has not been called.
    /// - `Unauthorized`: `deployer` is not the stored admin.
    /// - `SaltTooLong`: `salt` is longer than `MAX_POOL_SALT_LEN`.
    /// - `SaltAlreadyUsed`: a pool already exists for this salt.
    ///
    /// # Events
    /// `pool_created` with `(pool, deployer, stake_token, ledger)`.
    pub fn create_pool(
        env: Env,
        deployer: Address,
        salt: Bytes,
        config: PoolFactoryConfig,
    ) -> Result<Address, PoolFactoryError> {
        admin::require_admin_as(&env, &deployer).map_err(|_| PoolFactoryError::Unauthorized)?;
        if salt.len() > MAX_POOL_SALT_LEN {
            return Err(PoolFactoryError::SaltTooLong);
        }
        let wasm_hash = get_wasm_hash(&env).ok_or(PoolFactoryError::WasmHashNotSet)?;

        let salt_hash = salt_hash(&env, &salt);
        if get_pool_for_salt_hash(&env, &salt_hash).is_some() {
            return Err(PoolFactoryError::SaltAlreadyUsed);
        }

        // Initialize atomically: the constructor runs as part of deployment,
        // so the pool can never exist in an uninitialized state.
        let pool = env
            .deployer()
            .with_current_contract(salt_hash.clone())
            .deploy_contract(
                ContractExecutable::Wasm(wasm_hash),
                (
                    config.admin.clone(),
                    config.stake_token.clone(),
                    config.reward_token.clone(),
                    Some(config.reward_rate_bps),
                    None::<u32>,
                    None::<u32>,
                ),
            );

        env.storage().persistent().set(&salt_key(&salt_hash), &pool);

        env.events().publish(
            (Symbol::new(&env, "pool_created"),),
            (
                pool.clone(),
                deployer,
                config.stake_token,
                env.ledger().sequence(),
            ),
        );

        Ok(pool)
    }

    /// The pool created for `salt`, or `None` if the salt has not been used.
    pub fn get_pool_by_salt(env: Env, salt: Bytes) -> Option<Address> {
        get_pool_for_salt_hash(&env, &salt_hash(&env, &salt))
    }

    /// The deterministic address `create_pool` would deploy to for `salt`.
    ///
    /// Read-only: this does not deploy anything and does not mark the salt as
    /// used.
    pub fn pool_address_for_salt(env: Env, salt: Bytes) -> Address {
        env.deployer()
            .with_current_contract(salt_hash(&env, &salt))
            .deployed_address()
    }
}
