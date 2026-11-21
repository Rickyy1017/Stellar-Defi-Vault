//! User-defined withdrawal destination allowlist.
//!
//! A user can restrict where their funds may be paid to a short list of
//! addresses they control, so a compromised signing key cannot send funds to an
//! attacker's address.
//!
//! # Behaviour
//!
//! - `set_withdrawal_allowlist(user, addresses)` accepts at most
//!   [`MAX_ALLOWLIST_LEN`] addresses. An empty list means "no restriction".
//! - **Changes are delayed.** A new list becomes effective only after the
//!   configured delay (`set_allowlist_delay`, default one day of ledgers), and
//!   that includes clearing the list. Until then the previous list stays in
//!   force, so someone who steals a key cannot lift the restriction and
//!   withdraw in the same breath, and the owner has the delay window to notice.
//!   Submitting another change replaces a pending one and restarts the delay.
//! - **Where it applies.** While a list is in force, every payout of the user's
//!   funds must go to an address on it: `withdraw_to`, plain `withdraw` /
//!   `unstake` / `unstake_all` / `exit` (the destination is the user's own
//!   address, or the recipient), and `claim` / `claim_partial` (paid to the
//!   user). Because a plain withdrawal pays the user's own address, **a user
//!   who sets a list must include their own address if they still want to
//!   withdraw to it**; otherwise only the listed addresses can receive funds.
//! - Claim paths other than `claim` / `claim_partial` / `exit` (for example
//!   relayer or keeper claims) are not covered by this list.
//!
//! # Storage
//!
//! `DataKey` is at Soroban's 50-variant cap, so this uses raw `Symbol`-keyed
//! storage: the active list and any pending change per user in persistent
//! storage, and the delay in instance storage.

use soroban_sdk::{contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec};

use crate::admin;
use crate::constants::LEDGERS_PER_DAY;
use crate::errors::VaultError;
use crate::vault::VaultContract;

/// Maximum number of addresses on one allowlist.
pub const MAX_ALLOWLIST_LEN: u32 = 5;

const LIST_KEY: Symbol = symbol_short!("wl_list");
const PENDING_KEY: Symbol = symbol_short!("wl_pend");
const DELAY_KEY: Symbol = symbol_short!("wl_delay");

/// A queued allowlist change and the ledger it takes effect at.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAllowlist {
    /// The list that becomes active (empty = clear the restriction).
    pub addresses: Vec<Address>,
    /// First ledger at which `addresses` is in force.
    pub effective_at: u32,
}

/// The configured change delay in ledgers (default: one day).
pub fn get_delay(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DELAY_KEY)
        .unwrap_or(LEDGERS_PER_DAY)
}

fn stored_list(env: &Env, user: &Address) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&(LIST_KEY, user.clone()))
        .unwrap_or(Vec::new(env))
}

fn pending(env: &Env, user: &Address) -> Option<PendingAllowlist> {
    env.storage().persistent().get(&(PENDING_KEY, user.clone()))
}

/// The list in force right now: a pending change once its delay has elapsed,
/// otherwise the last active list.
pub fn effective_list(env: &Env, user: &Address) -> Vec<Address> {
    match pending(env, user) {
        Some(p) if env.ledger().sequence() >= p.effective_at => p.addresses,
        _ => stored_list(env, user),
    }
}

/// Whether `user` may currently pay funds to `destination`. Always true while
/// `user` has no list in force.
pub fn is_destination_allowed(env: &Env, user: &Address, destination: &Address) -> bool {
    let list = effective_list(env, user);
    list.is_empty() || list.contains(destination)
}

/// Revert with `DestinationNotAllowed` unless `destination` may receive
/// `user`'s funds.
pub fn require_destination_allowed(
    env: &Env,
    user: &Address,
    destination: &Address,
) -> Result<(), VaultError> {
    if is_destination_allowed(env, user, destination) {
        Ok(())
    } else {
        Err(VaultError::DestinationNotAllowed)
    }
}

#[contractimpl]
impl VaultContract {
    /// Set the addresses `user`'s funds may be withdrawn or claimed to. An
    /// empty list removes the restriction. The change takes effect after the
    /// configured delay (see the module docs); the current list stays in force
    /// until then, and a newer call replaces a pending change.
    ///
    /// # Authorization
    /// `user` must sign.
    ///
    /// # Errors
    /// - `AllowlistTooLong`: more than [`MAX_ALLOWLIST_LEN`] addresses.
    /// - `ArithmeticError`: the effective ledger overflowed.
    ///
    /// # Events
    /// - Topic `("wl_set", user)` with data `(addresses, effective_at)`.
    pub fn set_withdrawal_allowlist(
        env: Env,
        user: Address,
        addresses: Vec<Address>,
    ) -> Result<(), VaultError> {
        user.require_auth();
        if addresses.len() > MAX_ALLOWLIST_LEN {
            return Err(VaultError::AllowlistTooLong);
        }
        // Fold an already-matured pending change into the active list first, so
        // replacing the pending entry can never un-apply a change that took effect.
        let current = effective_list(&env, &user);
        env.storage()
            .persistent()
            .set(&(LIST_KEY, user.clone()), &current);

        let effective_at = env
            .ledger()
            .sequence()
            .checked_add(get_delay(&env))
            .ok_or(VaultError::ArithmeticError)?;
        env.storage().persistent().set(
            &(PENDING_KEY, user.clone()),
            &PendingAllowlist {
                addresses: addresses.clone(),
                effective_at,
            },
        );
        env.events()
            .publish((symbol_short!("wl_set"), user), (addresses, effective_at));
        Ok(())
    }

    /// Read-only: the addresses `user`'s funds may currently be paid to. An
    /// empty list means no restriction. A queued change is not included until
    /// its delay has elapsed; see `get_pending_withdrawal_allowlist`.
    pub fn get_withdrawal_allowlist(env: Env, user: Address) -> Vec<Address> {
        effective_list(&env, &user)
    }

    /// Read-only: the queued allowlist change for `user` that has not taken
    /// effect yet, if any.
    pub fn get_pending_withdrawal_allowlist(env: Env, user: Address) -> Option<PendingAllowlist> {
        pending(&env, &user).filter(|p| env.ledger().sequence() < p.effective_at)
    }

    /// Admin: set the delay, in ledgers, before an allowlist change takes
    /// effect. `0` makes changes immediate. Affects changes submitted after
    /// this call; a change already queued keeps its original effective ledger.
    ///
    /// # Authorization
    /// `admin` must be the stored admin and must sign.
    ///
    /// # Errors
    /// - `NotInitialized`: no admin is stored.
    /// - `Unauthorized`: `admin` is not the stored admin.
    pub fn set_allowlist_delay(env: Env, admin: Address, ledgers: u32) -> Result<(), VaultError> {
        admin::require_admin_as(&env, &admin)?;
        env.storage().instance().set(&DELAY_KEY, &ledgers);
        Ok(())
    }

    /// Read-only: the delay, in ledgers, before an allowlist change takes effect.
    pub fn get_allowlist_delay(env: Env) -> u32 {
        get_delay(&env)
    }
}
