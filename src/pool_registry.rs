//! Pool registry (issue #876).
//!
//! Lists known vault pools with metadata so frontends and cross-pool features
//! (for example `cross_pool_identity`) can discover siblings without being
//! configured with each address out of band.
//!
//! Registration verifies that the target really is a vault of the same
//! contract version and that it is controlled by the same operator. The
//! registry admin may delist a pool with a hash of the reason, preserving the
//! record for lookups while hiding it from `list_pools`.
//!
//! # Storage
//!
//! `DataKey` sits at Soroban's 50-variant cap, so this uses raw
//! `Symbol`-keyed storage, matching `balance.rs`.
//!
//! - `pr_pools` -> `Vec<Address>`: registration order.
//! - `(pr_info, pool)` -> `PoolInfo`: metadata and delisting state.

use soroban_sdk::{
    contractclient, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Env,
    String, Symbol, Vec,
};

use crate::admin;
use crate::errors::VaultError;
use crate::vault::{VaultContract, VaultContractArgs, VaultContractClient, CONTRACT_VERSION};

const POOLS_KEY: Symbol = symbol_short!("pr_pools");
const INFO_KEY: Symbol = symbol_short!("pr_info");

/// Largest page `list_pools` will return in one call.
pub const MAX_POOLS_PER_PAGE: u32 = 50;

/// Metadata stored for every registered pool.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PoolInfo {
    /// The pool contract address.
    pub pool: Address,
    /// Contract version reported by the pool when it was registered.
    pub version: String,
    /// Admin reported by the pool when it was registered.
    pub admin: Address,
    /// Ledger at which the pool was registered.
    pub registered_at: u32,
    /// `false` once the registry admin delists the pool.
    pub active: bool,
    /// Hash of the delisting reason, set only when `active` is `false`.
    pub delist_reason: Option<BytesN<32>>,
}

/// Errors returned by the pool registry.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PoolRegistryError {
    /// The caller is not the stored registry admin.
    Unauthorized = 1,
    /// The address does not implement the vault metadata interface.
    NotAPool = 2,
    /// The pool's contract version differs from the registry's.
    UnsupportedVersion = 3,
    /// The pool is controlled by a different admin.
    ForeignPool = 4,
    /// The pool is already registered.
    AlreadyRegistered = 5,
    /// The pool has not been registered.
    UnknownPool = 6,
    /// The pool has already been delisted.
    AlreadyDelisted = 7,
    /// `limit` is zero or above `MAX_POOLS_PER_PAGE`.
    InvalidPage = 8,
}

/// Minimal vault metadata surface the registry validates against.
#[contractclient(name = "PoolRegistryClient")]
pub trait PoolMetadata {
    /// Version string reported by the pool.
    fn get_version(env: Env) -> String;
    /// Current admin of the pool.
    fn get_admin(env: Env) -> Result<Address, VaultError>;
}

fn registered_pools(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&POOLS_KEY)
        .unwrap_or_else(|| Vec::new(env))
}

fn load_info(env: &Env, pool: &Address) -> Option<PoolInfo> {
    env.storage().persistent().get(&(INFO_KEY, pool.clone()))
}

fn store_info(env: &Env, info: &PoolInfo) {
    env.storage()
        .persistent()
        .set(&(INFO_KEY, info.pool.clone()), info);
}

#[cfg_attr(not(feature = "testutils"), contractimpl)]
impl VaultContract {
    /// Register `pool` as a sibling of this registry.
    ///
    /// The pool must report the same contract version as this registry and be
    /// controlled by the same admin. The stored registry admin must sign.
    ///
    /// # Errors
    /// - `Unauthorized`: the stored admin did not sign.
    /// - `NotAPool` / `UnsupportedVersion` / `ForeignPool`: validation failed.
    /// - `AlreadyRegistered`: `pool` is already listed.
    ///
    /// # Events
    /// `pool_reg` with `(pool)` as topic and `(version, admin, ledger)` as data.
    pub fn register_pool(env: Env, pool: Address) -> Result<(), PoolRegistryError> {
        let registry_admin = admin::get_admin(&env).map_err(|_| PoolRegistryError::Unauthorized)?;
        registry_admin.require_auth();

        if load_info(&env, &pool).is_some() {
            return Err(PoolRegistryError::AlreadyRegistered);
        }

        let client = PoolRegistryClient::new(&env, &pool);
        let version = match client.try_get_version() {
            Ok(Ok(version)) => version,
            _ => return Err(PoolRegistryError::NotAPool),
        };
        if version != String::from_str(&env, CONTRACT_VERSION) {
            return Err(PoolRegistryError::UnsupportedVersion);
        }
        let pool_admin = match client.try_get_admin() {
            Ok(Ok(admin)) => admin,
            _ => return Err(PoolRegistryError::NotAPool),
        };
        if pool_admin != registry_admin {
            return Err(PoolRegistryError::ForeignPool);
        }

        let info = PoolInfo {
            pool: pool.clone(),
            version: version.clone(),
            admin: pool_admin.clone(),
            registered_at: env.ledger().sequence(),
            active: true,
            delist_reason: None,
        };
        let mut pools = registered_pools(&env);
        pools.push_back(pool.clone());
        env.storage().instance().set(&POOLS_KEY, &pools);
        store_info(&env, &info);

        env.events().publish(
            (symbol_short!("pool_reg"), pool),
            (version, pool_admin, info.registered_at),
        );
        Ok(())
    }

    /// Admin: delist `pool` with a hash of the reason.
    ///
    /// The record is kept and still returned by `get_pool_info`, but the pool
    /// no longer appears in `list_pools`.
    ///
    /// # Errors
    /// - `Unauthorized`: the stored admin did not sign.
    /// - `UnknownPool`: `pool` was never registered.
    /// - `AlreadyDelisted`: `pool` is already inactive.
    ///
    /// # Events
    /// `pool_del` with `(pool)` as topic and `(reason_hash, ledger)` as data.
    pub fn delist_pool(
        env: Env,
        pool: Address,
        reason_hash: BytesN<32>,
    ) -> Result<(), PoolRegistryError> {
        let registry_admin = admin::get_admin(&env).map_err(|_| PoolRegistryError::Unauthorized)?;
        registry_admin.require_auth();

        let mut info = load_info(&env, &pool).ok_or(PoolRegistryError::UnknownPool)?;
        if !info.active {
            return Err(PoolRegistryError::AlreadyDelisted);
        }
        info.active = false;
        info.delist_reason = Some(reason_hash.clone());
        store_info(&env, &info);

        env.events().publish(
            (symbol_short!("pool_del"), pool),
            (reason_hash, env.ledger().sequence()),
        );
        Ok(())
    }

    /// Metadata for `pool`, including delisted pools, or `None` if unknown.
    pub fn get_pool_info(env: Env, pool: Address) -> Option<PoolInfo> {
        load_info(&env, &pool)
    }

    /// Active pools in registration order, paginated.
    ///
    /// `limit` must be between `1` and `MAX_POOLS_PER_PAGE`. An `offset` past
    /// the end returns an empty vector.
    ///
    /// # Errors
    /// - `InvalidPage`: `limit` is `0` or above `MAX_POOLS_PER_PAGE`.
    pub fn list_pools(
        env: Env,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<PoolInfo>, PoolRegistryError> {
        if limit == 0 || limit > MAX_POOLS_PER_PAGE {
            return Err(PoolRegistryError::InvalidPage);
        }

        let pools = registered_pools(&env);
        let mut out = Vec::new(&env);
        let mut skipped = 0u32;
        let mut i = 0u32;
        while i < pools.len() && out.len() < limit {
            if let Some(info) = load_info(&env, &pools.get(i).unwrap()) {
                if info.active {
                    if skipped < offset {
                        skipped += 1;
                    } else {
                        out.push_back(info);
                    }
                }
            }
            i += 1;
        }
        Ok(out)
    }

    /// Number of active pools in the registry.
    pub fn pool_registry_count(env: Env) -> u32 {
        let pools = registered_pools(&env);
        let mut count = 0u32;
        let mut i = 0u32;
        while i < pools.len() {
            if let Some(info) = load_info(&env, &pools.get(i).unwrap()) {
                if info.active {
                    count += 1;
                }
            }
            i += 1;
        }
        count
    }
}
