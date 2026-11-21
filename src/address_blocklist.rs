//! Admin address blocklist for compromised or sanctioned accounts.
//!
//! The admin can block an address with [`VaultContract::block_address`] (storing
//! a `reason_hash` for the audit trail) and lift it with `unblock_address`.
//!
//! # Policy for a blocked address
//!
//! - **Inflows are refused.** A blocked address cannot deposit, stake, be staked
//!   for, or be the sender or recipient of a position transfer.
//! - **Payouts are refused.** A blocked address cannot claim rewards, and nothing
//!   can be paid *to* a blocked address other than the principal path below.
//!   Accrued rewards are not forfeited: they stay accrued and are claimable
//!   again if the address is unblocked.
//! - **Principal stays recoverable.** A blocked user can still `withdraw` /
//!   `unstake` / `unstake_all` their own shares, and the tokens go to **their
//!   own original address only**. `withdraw_to` (or any withdrawal to a
//!   different recipient) is refused while they are blocked, so the principal
//!   cannot be redirected, and a withdrawal to a blocked recipient is refused.
//!
//! Blocking never seizes funds: it only freezes inflows, rewards and
//! redirection, so a mistaken block costs the user time, not principal.
//!
//! # Storage
//!
//! `DataKey` is at Soroban's 50-variant cap, so each block is a raw
//! `(Symbol, Address)`-keyed persistent entry holding the `reason_hash`.

use soroban_sdk::{contractimpl, symbol_short, Address, BytesN, Env, Symbol};

use crate::admin;
use crate::errors::VaultError;
use crate::vault::VaultContract;

const BLOCK_KEY: Symbol = symbol_short!("blocked");

/// Whether `user` is currently blocked.
pub fn is_blocked(env: &Env, user: &Address) -> bool {
    env.storage().persistent().has(&(BLOCK_KEY, user.clone()))
}

/// Revert with `AddressBlocked` when `user` is blocked. Used on inflow, claim
/// and transfer paths.
pub fn require_not_blocked(env: &Env, user: &Address) -> Result<(), VaultError> {
    if is_blocked(env, user) {
        return Err(VaultError::AddressBlocked);
    }
    Ok(())
}

/// Check a principal withdrawal by `owner` paid to `recipient`: it must not go
/// to a blocked recipient, and a blocked owner may only be paid at their own
/// address.
pub fn require_withdrawal_allowed(
    env: &Env,
    owner: &Address,
    recipient: &Address,
) -> Result<(), VaultError> {
    if is_blocked(env, recipient) || (is_blocked(env, owner) && owner != recipient) {
        return Err(VaultError::AddressBlocked);
    }
    Ok(())
}

#[contractimpl]
impl VaultContract {
    /// Admin: block `user`. See the module docs for what a block does and does
    /// not stop. Blocking an already-blocked address replaces its `reason_hash`.
    ///
    /// # Authorization
    /// `admin` must be the stored admin and must sign.
    ///
    /// # Errors
    /// - `NotInitialized`: no admin is stored.
    /// - `Unauthorized`: `admin` is not the stored admin.
    ///
    /// # Events
    /// - Topic `("blocked", user)` with data `(admin, reason_hash)`.
    pub fn block_address(
        env: Env,
        admin: Address,
        user: Address,
        reason_hash: BytesN<32>,
    ) -> Result<(), VaultError> {
        admin::require_admin_as(&env, &admin)?;
        env.storage()
            .persistent()
            .set(&(BLOCK_KEY, user.clone()), &reason_hash);
        env.events()
            .publish((symbol_short!("blocked"), user), (admin, reason_hash));
        Ok(())
    }

    /// Admin: lift the block on `user`. A no-op for an address that is not
    /// blocked.
    ///
    /// # Authorization
    /// `admin` must be the stored admin and must sign.
    ///
    /// # Errors
    /// - `NotInitialized`: no admin is stored.
    /// - `Unauthorized`: `admin` is not the stored admin.
    ///
    /// # Events
    /// - Topic `("unblocked", user)` with data `admin`, only when a block was removed.
    pub fn unblock_address(env: Env, admin: Address, user: Address) -> Result<(), VaultError> {
        admin::require_admin_as(&env, &admin)?;
        if is_blocked(&env, &user) {
            env.storage().persistent().remove(&(BLOCK_KEY, user.clone()));
            env.events().publish((symbol_short!("unblocked"), user), admin);
        }
        Ok(())
    }

    /// Read-only: whether `user` is blocked.
    pub fn is_blocked(env: Env, user: Address) -> bool {
        is_blocked(&env, &user)
    }

    /// Read-only: the `reason_hash` recorded when `user` was blocked.
    pub fn get_block_reason(env: Env, user: Address) -> Option<BytesN<32>> {
        env.storage().persistent().get(&(BLOCK_KEY, user))
    }
}
