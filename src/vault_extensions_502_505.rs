//! Vault Extensions for Issues #502, #503, #504, #505

use soroban_sdk::{contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec};
use crate::vault::VaultContract;
use crate::storage::AdminAction;
use crate::admin;
use crate::balance;
use crate::errors::{PublicApiError, VaultError};

// ----------------------------------------------------------------------------
// Issue #502: Withdrawal Queue
// ----------------------------------------------------------------------------
const WQ_ENABLED: Symbol = symbol_short!("wq_en");
const WQ_QUEUE: Symbol = symbol_short!("wq_q");

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct QueueEntry {
    pub user: Address,
    pub shares: i128,
    pub requested_at: u64,
}

pub fn is_withdrawal_queue_enabled(env: &Env) -> bool {
    env.storage().instance().get(&WQ_ENABLED).unwrap_or(false)
}

pub fn enqueue_withdrawal(env: &Env, user: Address, shares: i128) {
    let mut q: Vec<QueueEntry> = env.storage().instance().get(&WQ_QUEUE).unwrap_or(Vec::new(env));
    q.push_back(QueueEntry {
        user: user.clone(),
        shares,
        requested_at: env.ledger().timestamp(),
    });
    env.storage().instance().set(&WQ_QUEUE, &q);
    env.events().publish((symbol_short!("wq_queued"),), (user, shares));
}

// ----------------------------------------------------------------------------
// Issue #503: Admin Timelock
// ----------------------------------------------------------------------------
const TL_DELAY: Symbol = symbol_short!("tl_delay");
const TL_NEXT_ID: Symbol = symbol_short!("tl_nxt_id");
const TL_ACTIONS: Symbol = symbol_short!("tl_acts");

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct QueuedAction {
    pub action: AdminAction,
    pub executable_at: u32,
}

// ----------------------------------------------------------------------------
// Issue #504: Auto Compound
// ----------------------------------------------------------------------------
const AUTO_COMPOUND: Symbol = symbol_short!("auto_comp"); // Map<Address, bool>

// ----------------------------------------------------------------------------
// Issue #505: Tokenize Position
// ----------------------------------------------------------------------------
const NFT_BALANCES: Symbol = symbol_short!("nft_bals"); // Map<u32, i128>
const NFT_OWNERS: Symbol = symbol_short!("nft_owns"); // Map<u32, Address>
const NEXT_NFT_ID: Symbol = symbol_short!("nft_nxt");

#[contractimpl]
impl VaultContract {
    // ------------------------------------------------------------------------
    // Issue #502: Withdrawal Queue
    // ------------------------------------------------------------------------
    pub fn enable_withdrawal_queue(env: Env, admin: Address, enabled: bool) -> Result<(), PublicApiError> {
        admin.require_auth();
        admin::require_admin(&env)?;
        env.storage().instance().set(&WQ_ENABLED, &enabled);
        Ok(())
    }
    
    pub fn process_withdrawal_queue(env: Env, admin: Address, max_entries: u32) -> Result<(), VaultError> {
        admin.require_auth();
        admin::require_admin(&env)?;
        let mut q: Vec<QueueEntry> = env.storage().instance().get(&WQ_QUEUE).unwrap_or(Vec::new(&env));
        let mut processed = 0;
        
        while processed < max_entries && q.len() > 0 {
            let entry = q.get(0).unwrap();
            match Self::do_unstake(&env, &entry.user, entry.shares) {
                Ok(_) => {
                    env.events().publish((symbol_short!("wq_proc"),), (entry.user.clone(), entry.shares));
                    q.remove(0);
                    processed += 1;
                },
                Err(_) => {
                    break;
                }
            }
        }
        env.storage().instance().set(&WQ_QUEUE, &q);
        Ok(())
    }
    
    pub fn get_queue_position(env: Env, user: Address) -> Option<u32> {
        let q: Vec<QueueEntry> = env.storage().instance().get(&WQ_QUEUE).unwrap_or(Vec::new(&env));
        for i in 0..q.len() {
            if q.get(i).unwrap().user == user {
                return Some(i);
            }
        }
        None
    }

    // ------------------------------------------------------------------------
    // Issue #503: Admin Timelock
    // ------------------------------------------------------------------------
    pub fn set_legacy_timelock_delay(env: Env, admin: Address, ledgers: u32) -> Result<(), PublicApiError> {
        admin.require_auth();
        admin::require_admin(&env)?;
        env.storage().instance().set(&TL_DELAY, &ledgers);
        Ok(())
    }
    
    pub fn queue_admin_action(env: Env, admin: Address, action: AdminAction) -> Result<u32, PublicApiError> {
        admin.require_auth();
        admin::require_admin(&env)?;
        let delay: u32 = env.storage().instance().get(&TL_DELAY).unwrap_or(0);
        let action_id: u32 = env.storage().instance().get(&TL_NEXT_ID).unwrap_or(1);
        let next_id = action_id
            .checked_add(1)
            .ok_or(PublicApiError::ArithmeticError)?;
        env.storage().instance().set(&TL_NEXT_ID, &next_id);
        
        let queued = QueuedAction {
            action: action.clone(),
            executable_at: env.ledger().sequence() + delay,
        };
        let mut map: soroban_sdk::Map<u32, QueuedAction> = env.storage().instance().get(&TL_ACTIONS).unwrap_or(soroban_sdk::Map::new(&env));
        map.set(action_id, queued);
        env.storage().instance().set(&TL_ACTIONS, &map);
        env.events().publish((symbol_short!("act_queue"),), (action_id, action));
        Ok(action_id)
    }
    
    pub fn execute_admin_action(env: Env, admin: Address, action_id: u32) -> Result<(), PublicApiError> {
        admin.require_auth();
        admin::require_admin(&env)?;
        let mut map: soroban_sdk::Map<u32, QueuedAction> = env.storage().instance().get(&TL_ACTIONS).unwrap_or(soroban_sdk::Map::new(&env));
        if let Some(queued) = map.get(action_id) {
            if env.ledger().sequence() < queued.executable_at {
                return Err(PublicApiError::ActionNotYetExecutable);
            }
            env.events().publish((symbol_short!("act_exec"),), (action_id,));
            map.remove(action_id);
            env.storage().instance().set(&TL_ACTIONS, &map);
        } else {
            return Err(PublicApiError::ActionNotFound);
        }
        Ok(())
    }
    
    pub fn cancel_admin_action(env: Env, admin: Address, action_id: u32) -> Result<(), PublicApiError> {
        admin.require_auth();
        admin::require_admin(&env)?;
        let mut map: soroban_sdk::Map<u32, QueuedAction> = env.storage().instance().get(&TL_ACTIONS).unwrap_or(soroban_sdk::Map::new(&env));
        if map.contains_key(action_id) {
            map.remove(action_id);
            env.storage().instance().set(&TL_ACTIONS, &map);
            env.events().publish((symbol_short!("act_canc"),), (action_id,));
        } else {
            return Err(PublicApiError::ActionNotFound);
        }
        Ok(())
    }

    // ------------------------------------------------------------------------
    // Issue #504: Auto Compound
    // ------------------------------------------------------------------------
    pub fn set_auto_compound(env: Env, user: Address, enabled: bool) {
        user.require_auth();
        let mut map: soroban_sdk::Map<Address, bool> = env.storage().instance().get(&AUTO_COMPOUND).unwrap_or(soroban_sdk::Map::new(&env));
        map.set(user.clone(), enabled);
        env.storage().instance().set(&AUTO_COMPOUND, &map);
    }
    
    pub fn compound(env: Env, user: Address) -> Result<(), PublicApiError> {
        user.require_auth();
        let map: soroban_sdk::Map<Address, bool> = env.storage().instance().get(&AUTO_COMPOUND).unwrap_or(soroban_sdk::Map::new(&env));
        if !map.get(user.clone()).unwrap_or(false) {
            return Err(PublicApiError::AutoCompoundDisabled);
        }
        
        let pending = balance::get_accrued_reward(&env, &user);
        if pending > 0 {
            balance::set_accrued_reward(&env, &user, 0);
            
            let total_shares = balance::get_total_shares(&env);
            let total_deposited = balance::get_total_deposited(&env);
            let shares = if total_deposited == 0 {
                pending
            } else {
                pending
                    .checked_mul(total_shares)
                    .and_then(|value| value.checked_div(total_deposited))
                    .ok_or(PublicApiError::ArithmeticError)?
            };
            
            balance::set_shares(
                &env,
                &user,
                balance::get_shares(&env, &user)
                    .checked_add(shares)
                    .ok_or(PublicApiError::ArithmeticError)?,
            );
            balance::set_total_shares(
                &env,
                total_shares
                    .checked_add(shares)
                    .ok_or(PublicApiError::ArithmeticError)?,
            );
            balance::set_total_deposited(
                &env,
                total_deposited
                    .checked_add(pending)
                    .ok_or(PublicApiError::ArithmeticError)?,
            );
            
            env.events().publish((symbol_short!("compound"),), (user, pending, shares, env.ledger().sequence()));
        }
        Ok(())
    }

    // ------------------------------------------------------------------------
    // Issue #505: Tokenize Position
    // ------------------------------------------------------------------------
    pub fn tokenize_position(env: Env, user: Address) -> Result<u32, PublicApiError> {
        user.require_auth();
        let shares = balance::get_shares(&env, &user);
        if shares <= 0 {
            return Err(PublicApiError::PositionNotFound);
        }
        
        balance::set_shares(&env, &user, 0); 
        
        let token_id: u32 = env.storage().instance().get(&NEXT_NFT_ID).unwrap_or(1);
        let next_token_id = token_id
            .checked_add(1)
            .ok_or(PublicApiError::ArithmeticError)?;
        env.storage().instance().set(&NEXT_NFT_ID, &next_token_id);
        
        let mut owns: soroban_sdk::Map<u32, Address> = env.storage().instance().get(&NFT_OWNERS).unwrap_or(soroban_sdk::Map::new(&env));
        owns.set(token_id, user.clone());
        env.storage().instance().set(&NFT_OWNERS, &owns);
        
        let mut bals: soroban_sdk::Map<u32, i128> = env.storage().instance().get(&NFT_BALANCES).unwrap_or(soroban_sdk::Map::new(&env));
        bals.set(token_id, shares);
        env.storage().instance().set(&NFT_BALANCES, &bals);
        
        env.events().publish((symbol_short!("tok_pos"),), (user, token_id, shares));
        Ok(token_id)
    }
    
    pub fn redeem_position_nft(env: Env, user: Address, token_id: u32) -> Result<(), PublicApiError> {
        user.require_auth();
        let mut owns: soroban_sdk::Map<u32, Address> = env.storage().instance().get(&NFT_OWNERS).unwrap_or(soroban_sdk::Map::new(&env));
        
        if owns.get(token_id) != Some(user.clone()) {
            return Err(PublicApiError::NotNftOwner);
        }
        
        let mut bals: soroban_sdk::Map<u32, i128> = env.storage().instance().get(&NFT_BALANCES).unwrap_or(soroban_sdk::Map::new(&env));
        let shares = bals.get(token_id).unwrap_or(0);
        
        owns.remove(token_id);
        bals.remove(token_id);
        env.storage().instance().set(&NFT_OWNERS, &owns);
        env.storage().instance().set(&NFT_BALANCES, &bals);
        
        let restored = balance::get_shares(&env, &user)
            .checked_add(shares)
            .ok_or(PublicApiError::ArithmeticError)?;
        balance::set_shares(&env, &user, restored);
        env.events().publish((symbol_short!("nft_redm"),), (user, token_id, shares));
        Ok(())
    }
}
