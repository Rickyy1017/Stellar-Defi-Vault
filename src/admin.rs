use crate::errors::VaultError;
use crate::storage::DataKey;
use soroban_sdk::{symbol_short, Address, Env};

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_admin(env: &Env) -> Result<Address, VaultError> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(VaultError::NotInitialized)
}

/// Sets a new emergency admin address.
/// CAN ONLY be called by the primary admin.
pub fn set_emergency_admin(env: &Env, emergency_admin: &Address) -> Result<(), VaultError> {
    // Enforce requirement 4: Only the primary admin can designate another emergency admin
    let primary_admin = get_admin(env)?;
    primary_admin.require_auth();

    env.storage().instance().set(&symbol_short!("emg_adm"), emergency_admin);

    // Enforce requirement 5: Emit event: emergency_admin_set
    env.events().publish(
        (symbol_short!("emer_set"),),
        emergency_admin.clone(),
    );

    Ok(())
}

pub fn get_emergency_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&symbol_short!("emg_adm"))
}

/// Revokes the current emergency admin address.
/// CAN ONLY be called by the primary admin.
pub fn clear_emergency_admin(env: &Env) -> Result<(), VaultError> {
    // Only the primary admin can clear/revoke an emergency admin
    let primary_admin = get_admin(env)?;
    primary_admin.require_auth();

    if env.storage().instance().has(&symbol_short!("emg_adm")) {
        env.storage().instance().remove(&symbol_short!("emg_adm"));

        // Enforce requirement 5: Emit event: emergency_admin_revoked
        env.events().publish(
            (symbol_short!("emer_rvk"),),
            (),
        );
    }

    Ok(())
}

pub fn require_admin(env: &Env) -> Result<(), VaultError> {
    let admin = get_admin(env)?;
    admin.require_auth();
    Ok(())
}

/// Validates that the active transaction context is authorized by either 
/// the primary administrator or the secondary crisis administrator.
pub fn require_admin_or_emergency_admin(env: &Env, caller: &Address) -> Result<(), VaultError> {
    let primary = get_admin(env)?;
    if caller == &primary {
        primary.require_auth();
        return Ok(());
    }

    if let Some(emergency) = get_emergency_admin(env) {
        if caller == &emergency {
            emergency.require_auth();
            return Ok(());
        }
    }

    // Falls back to NotAuthorized/Unauthorized variant present in your error definitions
    Err(VaultError::Unauthorized)
}
