//! Feature-flag registry for optional modules.
//!
//! One consistent switch per optional module, instead of a bespoke toggle in
//! each. The admin turns a module on or off with [`set_feature_enabled`]
//! (`VaultContract::set_feature_enabled`); anyone can read the state with
//! `is_feature_enabled`.
//!
//! # Defaults
//!
//! Every feature is **enabled by default**. Each module shipped before the
//! registry existed, so a deployment that upgrades in place keeps behaving
//! exactly as before until the admin explicitly switches something off. Only an
//! explicit admin choice is stored; an untouched feature reads its default.
//!
//! | Feature     | Default | Gated entrypoints (revert with `FeatureDisabled` when off)        |
//! |-------------|---------|--------------------------------------------------------------------|
//! | `Lockdrop`  | on      | `start_lockdrop`, `commit_to_lockdrop`                             |
//! | `Referrals` | on      | `register_referrer`, `stake_with_referral`, `deposit` with referrer |
//! | `Mirroring` | on      | `set_mirror_config`; automatic mirroring of leader actions stops   |
//! | `Hooks`     | on      | `register_hook`; registered hooks are no longer notified           |
//!
//! # Funds are never trapped by a switch
//!
//! A flag only stops *new* activity in a module. Entrypoints that let a user
//! take back or collect something they already hold stay open while the module
//! is off: `exit_lockdrop`, `claim_lockdrop_reward`, `claim_referral_earnings`,
//! `cancel_mirroring` and `remove_hook`.
//!
//! # Storage
//!
//! `DataKey` is at Soroban's 50-variant cap, so each override is a raw
//! `(Symbol, Feature)`-keyed instance entry holding a `bool`.
//!
//! # Gating from other modules
//!
//! Each gated entrypoint checks [`is_enabled`] and returns its own error
//! type's `FeatureDisabled` (a real variant on `VaultCampaignError` and
//! `FeatureFlagError`; an alias on the capped `VaultError` / `VaultExtError`).

use soroban_sdk::{contractimpl, contracttype, symbol_short, Address, Env, Symbol};

use crate::admin;
use crate::errors::FeatureFlagError;
use crate::vault::VaultContract;

const FLAG_KEY: Symbol = symbol_short!("feat_flg");

/// An optional module that the admin can switch on or off.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Feature {
    /// Lockdrop campaign (`lockdrop_campaign`).
    Lockdrop,
    /// Referral registration and referred deposits.
    Referrals,
    /// Position mirroring (copy-staking a leader).
    Mirroring,
    /// Stake / unstake hook contracts (`stake_hooks`).
    Hooks,
}

/// The state a feature has until the admin sets it explicitly.
///
/// Every feature defaults to enabled (see the module docs).
pub fn default_enabled(_feature: Feature) -> bool {
    true
}

/// Whether `feature` is currently switched on.
pub fn is_enabled(env: &Env, feature: Feature) -> bool {
    env.storage()
        .instance()
        .get(&(FLAG_KEY, feature))
        .unwrap_or_else(|| default_enabled(feature))
}

#[contractimpl]
impl VaultContract {
    /// Admin: switch an optional module on or off.
    ///
    /// Switching a module off only stops new activity in it; users can still
    /// take back or collect what they already hold (see the module docs).
    /// Setting a feature to the value it already has is allowed and still
    /// records an explicit choice.
    ///
    /// # Authorization
    /// `admin` must be the stored admin and must sign.
    ///
    /// # Errors
    /// - `NotInitialized`: no admin is stored.
    /// - `Unauthorized`: `admin` is not the stored admin.
    ///
    /// # Events
    /// - Topic `("feat_set", feature)` with data `(admin, enabled)`.
    pub fn set_feature_enabled(
        env: Env,
        admin: Address,
        feature: Feature,
        enabled: bool,
    ) -> Result<(), FeatureFlagError> {
        admin::require_admin_as(&env, &admin)?;
        env.storage().instance().set(&(FLAG_KEY, feature), &enabled);
        env.events()
            .publish((symbol_short!("feat_set"), feature), (admin, enabled));
        Ok(())
    }

    /// Read-only: whether `feature` is switched on. Untouched features report
    /// their default, which is `true` for every feature.
    pub fn is_feature_enabled(env: Env, feature: Feature) -> bool {
        is_enabled(&env, feature)
    }
}
