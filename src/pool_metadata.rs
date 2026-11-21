//! On-chain pool metadata for frontends.
//!
//! The admin publishes a [`PoolMetadata`] record (name, description, website,
//! icon hash) so any UI can render the pool without a separate registry.
//! Every field is length-limited; an update replaces the whole record.
//!
//! Each update is appended to the parameter change log under the parameter
//! name `pool_metadata`. The log stores integers, so `old_value`/`new_value`
//! hold the metadata revision counter (0 = never set) and the log `reason`
//! holds the new pool name.
//!
//! # Storage
//!
//! `DataKey` is at Soroban's 50-variant cap, so this uses raw `Symbol`-keyed
//! instance storage, matching the other extension modules.

use soroban_sdk::{contractimpl, contracttype, symbol_short, Address, BytesN, Env, String, Symbol};

use crate::admin;
use crate::errors::PoolMetadataError;
use crate::vault::VaultContract;

/// Maximum length of the pool name, in bytes.
pub const MAX_NAME_LEN: u32 = 64;
/// Maximum length of the pool description, in bytes.
pub const MAX_DESCRIPTION_LEN: u32 = 512;
/// Maximum length of the website URL, in bytes.
pub const MAX_WEBSITE_LEN: u32 = 128;

const METADATA_KEY: Symbol = symbol_short!("pool_meta");
const REVISION_KEY: Symbol = symbol_short!("pm_rev");

/// Display metadata for the pool.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolMetadata {
    /// Human-readable pool name. Required, at most `MAX_NAME_LEN` bytes.
    pub name: String,
    /// Short description. May be empty, at most `MAX_DESCRIPTION_LEN` bytes.
    pub description: String,
    /// Website URL. May be empty, at most `MAX_WEBSITE_LEN` bytes.
    pub website: String,
    /// 32-byte hash (e.g. an IPFS/content digest) identifying the pool icon.
    pub icon_hash: BytesN<32>,
}

/// Check every field against its length limit.
pub fn validate_metadata(metadata: &PoolMetadata) -> Result<(), PoolMetadataError> {
    if metadata.name.len() == 0 {
        return Err(PoolMetadataError::EmptyName);
    }
    if metadata.name.len() > MAX_NAME_LEN {
        return Err(PoolMetadataError::NameTooLong);
    }
    if metadata.description.len() > MAX_DESCRIPTION_LEN {
        return Err(PoolMetadataError::DescriptionTooLong);
    }
    if metadata.website.len() > MAX_WEBSITE_LEN {
        return Err(PoolMetadataError::WebsiteTooLong);
    }
    Ok(())
}

/// The stored metadata, if the admin has set it.
pub fn get_metadata(env: &Env) -> Option<PoolMetadata> {
    env.storage().instance().get(&METADATA_KEY)
}

fn get_revision(env: &Env) -> u32 {
    env.storage().instance().get(&REVISION_KEY).unwrap_or(0)
}

#[contractimpl]
impl VaultContract {
    /// Admin: set (or replace) the pool's display metadata.
    ///
    /// Limits: `name` 1..=64 bytes, `description` up to 512 bytes, `website`
    /// up to 128 bytes. The previous record is overwritten in full.
    ///
    /// # Authorization
    /// `admin` must be the stored admin and must sign.
    ///
    /// # Errors
    /// - `NotInitialized`: no admin is stored.
    /// - `Unauthorized`: `admin` is not the stored admin.
    /// - `EmptyName`: `name` is empty.
    /// - `NameTooLong` / `DescriptionTooLong` / `WebsiteTooLong`: a field
    ///   exceeds its limit.
    ///
    /// # Events
    /// - Topic `("pool_meta",)` with data `(admin, name, revision, ledger)`.
    ///
    /// The change is also appended to the parameter change log as
    /// `pool_metadata`, with the old and new revision as the values and the
    /// new pool name as the reason.
    pub fn set_pool_metadata(
        env: Env,
        admin: Address,
        metadata: PoolMetadata,
    ) -> Result<(), PoolMetadataError> {
        admin::require_admin_as(&env, &admin)?;
        validate_metadata(&metadata)?;

        let old_revision = get_revision(&env);
        let new_revision = old_revision
            .checked_add(1)
            .ok_or(PoolMetadataError::ArithmeticError)?;

        env.storage().instance().set(&METADATA_KEY, &metadata);
        env.storage().instance().set(&REVISION_KEY, &new_revision);

        crate::vault_extensions_463_466::log_parameter_change_with_reason(
            &env,
            &admin,
            "pool_metadata",
            old_revision as i128,
            new_revision as i128,
            &metadata.name,
        );
        env.events().publish(
            (symbol_short!("pool_meta"),),
            (
                admin,
                metadata.name,
                new_revision,
                env.ledger().sequence(),
            ),
        );
        Ok(())
    }

    /// Read-only: the pool's display metadata, or `None` if the admin has not
    /// set it yet.
    pub fn get_pool_metadata(env: Env) -> Option<PoolMetadata> {
        get_metadata(&env)
    }
}
