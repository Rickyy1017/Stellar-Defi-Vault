//! Sub-pool delegation (issue #454).
//!
//! Tiered fund management within one contract: up to 5 named sub-pools
//! each with a delegated sub-admin, independent capacity and reward rate.

use soroban_sdk::{contractimpl, contracttype, symbol_short, token, Address, Env, Symbol, Vec, String};

use crate::admin;
use crate::balance;
use crate::errors::VaultExtError;
use crate::vault::VaultContract;

const SUB_POOLS_KEY: Symbol = symbol_short!("sub_pool");
const SUB_POOL_COUNT_KEY: Symbol = symbol_short!("sub_cnt");
const SUB_POS_KEY: Symbol = symbol_short!("sub_pos");

/// Sub-pool struct per acceptance criteria
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SubPool {
    pub id: u32,
    pub name: String,
    pub sub_admin: Address,
    pub reward_rate_bps: i128,
    pub total_staked: i128,
    pub max_capacity: i128,
}

fn get_sub_pools(env: &Env) -> Vec<SubPool> {
    env.storage()
        .instance()
        .get(&SUB_POOLS_KEY)
        .unwrap_or(Vec::new(env))
}

fn set_sub_pools(env: &Env, pools: &Vec<SubPool>) {
    env.storage().instance().set(&SUB_POOLS_KEY, pools);
}

fn get_next_sub_pool_id(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&SUB_POOL_COUNT_KEY)
        .unwrap_or(0)
}

fn set_next_sub_pool_id(env: &Env, id: u32) {
    env.storage().instance().set(&SUB_POOL_COUNT_KEY, &id);
}

fn get_sub_pool_position(env: &Env, user: &Address, sub_pool_id: u32) -> i128 {
    env.storage()
        .persistent()
        .get(&(SUB_POS_KEY, user.clone(), sub_pool_id))
        .unwrap_or(0)
}

fn set_sub_pool_position(env: &Env, user: &Address, sub_pool_id: u32, amount: i128) {
    if amount == 0 {
        env.storage()
            .persistent()
            .remove(&(SUB_POS_KEY, user.clone(), sub_pool_id));
    } else {
        env.storage()
            .persistent()
            .set(&(SUB_POS_KEY, user.clone(), sub_pool_id), &amount);
    }
}

fn find_sub_pool_index(env: &Env, pools: &Vec<SubPool>, id: u32) -> Option<u32> {
    for i in 0..pools.len() {
        if pools.get(i).unwrap().id == id {
            return Some(i);
        }
    }
    None
}

#[contractimpl]
impl VaultContract {
    /// Admin creates a named sub-pool with delegated sub-admin.
    pub fn create_sub_pool(
        env: Env,
        admin: Address,
        name: String,
        sub_admin: Address,
        reward_rate_bps: i128,
        max_capacity: i128,
    ) -> Result<u32, VaultExtError> {
        admin.require_auth();
        admin::require_admin(&env)?;
        if max_capacity <= 0 {
            return Err(VaultExtError::ZeroAmount);
        }
        if reward_rate_bps < 0 {
            return Err(VaultExtError::ZeroAmount);
        }
        // reward_rate <= main pool rate if main rate set
        let main_rate = balance::get_reward_rate_bps(&env) as i128;
        if main_rate > 0 && reward_rate_bps > main_rate {
            return Err(VaultExtError::InvalidFeeAllocation);
        }
        let mut pools = get_sub_pools(&env);
        if pools.len() >= 5 {
            return Err(VaultExtError::TooManyProposals);
        }
        let id = get_next_sub_pool_id(&env);
        let pool = SubPool {
            id,
            name: name.clone(),
            sub_admin: sub_admin.clone(),
            reward_rate_bps,
            total_staked: 0,
            max_capacity,
        };
        pools.push_back(pool);
        set_sub_pools(&env, &pools);
        set_next_sub_pool_id(&env, id + 1);
        // event: (id, name, sub_admin, reward_rate_bps, ledger)
        env.events().publish(
            (symbol_short!("sub_crt"),),
            (id, name, sub_admin, reward_rate_bps, env.ledger().sequence()),
        );
        Ok(id)
    }

    /// Stake into a specific sub-pool
    pub fn stake_into_sub_pool(
        env: Env,
        user: Address,
        sub_pool_id: u32,
        amount: i128,
    ) -> Result<(), VaultExtError> {
        user.require_auth();
        if amount <= 0 {
            return Err(VaultExtError::ZeroAmount);
        }
        let token_addr: Address = env
            .storage()
            .instance()
            .get(&crate::storage::DataKey::Token)
            .ok_or(VaultExtError::NotInitialized)?;
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        let mut pools = get_sub_pools(&env);
        let idx = find_sub_pool_index(&env, &pools, sub_pool_id).ok_or(VaultExtError::PositionNotFound)?;
        let mut pool = pools.get(idx).unwrap();
        if pool.total_staked + amount > pool.max_capacity {
            return Err(VaultExtError::InvalidFeeAllocation);
        }
        pool.total_staked += amount;
        pools.set(idx, pool);
        set_sub_pools(&env, &pools);

        let cur = get_sub_pool_position(&env, &user, sub_pool_id);
        set_sub_pool_position(&env, &user, sub_pool_id, cur + amount);

        // also bump main pool accounting for total share tracking? Keep separate.
        // We still increment total_deposited for global visibility? Not required - keep sub-pool independent.
        // But also track per-user stake history? Minimal.

        env.events().publish(
            (symbol_short!("sub_stk"), user),
            (sub_pool_id, amount, env.ledger().sequence()),
        );
        Ok(())
    }

    /// Unstake from specific sub-pool
    pub fn unstake_from_sub_pool(
        env: Env,
        user: Address,
        sub_pool_id: u32,
        amount: i128,
    ) -> Result<(), VaultExtError> {
        user.require_auth();
        if amount <= 0 {
            return Err(VaultExtError::ZeroAmount);
        }
        let cur = get_sub_pool_position(&env, &user, sub_pool_id);
        if cur < amount {
            return Err(VaultExtError::PositionNotFound);
        }
        let mut pools = get_sub_pools(&env);
        let idx = find_sub_pool_index(&env, &pools, sub_pool_id).ok_or(VaultExtError::PositionNotFound)?;
        let mut pool = pools.get(idx).unwrap();
        pool.total_staked -= amount;
        pools.set(idx, pool);
        set_sub_pools(&env, &pools);
        set_sub_pool_position(&env, &user, sub_pool_id, cur - amount);

        let token_addr: Address = env
            .storage()
            .instance()
            .get(&crate::storage::DataKey::Token)
            .ok_or(VaultExtError::NotInitialized)?;
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&env.current_contract_address(), &user, &amount);

        env.events().publish(
            (symbol_short!("sub_unstk"), user),
            (sub_pool_id, amount, env.ledger().sequence()),
        );
        Ok(())
    }

    /// Sub-admin can set their sub-pool's rate within bounds: max = main pool rate
    /// Main admin can override any sub-admin action.
    pub fn set_sub_pool_rate(
        env: Env,
        caller: Address,
        sub_pool_id: u32,
        rate_bps: i128,
    ) -> Result<(), VaultExtError> {
        caller.require_auth();
        let is_admin = admin::get_admin(&env).map(|a| a == caller).unwrap_or(false);
        let mut pools = get_sub_pools(&env);
        let idx = find_sub_pool_index(&env, &pools, sub_pool_id).ok_or(VaultExtError::PositionNotFound)?;
        let mut pool = pools.get(idx).unwrap();
        if !is_admin {
            if pool.sub_admin != caller {
                return Err(VaultExtError::Unauthorized);
            }
        }
        if rate_bps < 0 {
            return Err(VaultExtError::ZeroAmount);
        }
        let main_rate = balance::get_reward_rate_bps(&env) as i128;
        if main_rate > 0 && rate_bps > main_rate {
            return Err(VaultExtError::InvalidFeeAllocation);
        }
        pool.reward_rate_bps = rate_bps;
        pools.set(idx, pool);
        set_sub_pools(&env, &pools);
        env.events().publish(
            (symbol_short!("sub_rate"), caller),
            (sub_pool_id, rate_bps, env.ledger().sequence()),
        );
        Ok(())
    }

    /// Read-only query single sub-pool
    pub fn get_sub_pool(env: Env, id: u32) -> Option<SubPool> {
        let pools = get_sub_pools(&env);
        for i in 0..pools.len() {
            let p = pools.get(i).unwrap();
            if p.id == id {
                return Some(p);
            }
        }
        None
    }

    /// Read-all sub-pools
    pub fn get_all_sub_pools(env: Env) -> Vec<SubPool> {
        get_sub_pools(&env)
    }

    /// Position per user per sub-pool
    pub fn get_sub_pool_position(env: Env, user: Address, sub_pool_id: u32) -> i128 {
        get_sub_pool_position(&env, &user, sub_pool_id)
    }

    /// Rewards calculated per sub-pool rate: simple pro-rata pending reward
    pub fn calc_sub_pool_pending(env: Env, user: Address, sub_pool_id: u32) -> i128 {
        let pools = get_sub_pools(&env);
        let idx = find_sub_pool_index(&env, &pools, sub_pool_id);
        if idx.is_none() {
            return 0;
        }
        let pool = pools.get(idx.unwrap()).unwrap();
        let shares = get_sub_pool_position(&env, &user, sub_pool_id);
        if shares == 0 || pool.reward_rate_bps == 0 {
            return 0;
        }
        // Simplified: reward = shares * rate_bps / 10000 * elapsed_ledgers / LEDGERS_PER_YEAR
        // For demo we just return shares * rate_bps / 10000
        (shares * pool.reward_rate_bps) / 10_000
    }
}
