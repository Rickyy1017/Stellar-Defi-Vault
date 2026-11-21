#![cfg(test)]
//! Tests for issues #874, #875 and #876.

extern crate std;

use soroban_sdk::{
    symbol_short, testutils::Address as _, token, Address, Bytes, BytesN, Env, String,
};

use crate::vault::{VaultContract, VaultContractClient, CONTRACT_VERSION};

fn setup_vault<'a>(env: &Env) -> (VaultContractClient<'a>, Address, Address, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract(token_admin);
    let vault_id = env.register_contract(None, VaultContract);
    let vault = VaultContractClient::new(env, &vault_id);
    vault.initialize(&admin, &token_id, &0_u32, &None, &None);
    (vault, admin, token_id, vault_id)
}

fn deploy_pool<'a>(
    env: &Env,
    admin: &Address,
    token: &Address,
) -> (VaultContractClient<'a>, Address) {
    let pool_id = env.register_contract(None, VaultContract);
    let pool = VaultContractClient::new(env, &pool_id);
    pool.initialize(admin, token, &0_u32, &None, &None);
    (pool, pool_id)
}

// ── Issue #874: deposit_from ────────────────────────────────────────────────

#[test]
fn deposit_from_pulls_allowance_and_credits_receiver() {
    let env = Env::default();
    let (vault, _admin, token_id, vault_id) = setup_vault(&env);
    let owner = Address::generate(&env);
    let spender = Address::generate(&env);
    let receiver = Address::generate(&env);

    let token = token::Client::new(&env, &token_id);
    let asset = token::StellarAssetClient::new(&env, &token_id);
    asset.mint(&owner, &1_000);
    token.approve(&owner, &vault_id, &600, &(env.ledger().sequence() + 100));

    let shares = vault.deposit_from(&spender, &owner, &600, &receiver);

    assert_eq!(shares, 600);
    assert_eq!(vault.shares_of(&receiver), 600);
    assert_eq!(vault.shares_of(&owner), 0);
    assert_eq!(token.balance(&owner), 400);
    assert_eq!(token.balance(&vault_id), 600);
}

#[test]
fn deposit_from_reverts_without_sufficient_allowance() {
    let env = Env::default();
    let (vault, _admin, token_id, vault_id) = setup_vault(&env);
    let owner = Address::generate(&env);
    let spender = Address::generate(&env);
    let receiver = Address::generate(&env);

    let token = token::Client::new(&env, &token_id);
    let asset = token::StellarAssetClient::new(&env, &token_id);
    asset.mint(&owner, &1_000);
    token.approve(&owner, &vault_id, &100, &(env.ledger().sequence() + 100));

    assert!(vault
        .try_deposit_from(&spender, &owner, &200, &receiver)
        .is_err());
    assert_eq!(vault.shares_of(&receiver), 0);
    assert_eq!(token.balance(&owner), 1_000);
}

#[test]
fn deposit_from_applies_cap_to_receiver() {
    let env = Env::default();
    let (vault, _admin, token_id, vault_id) = setup_vault(&env);
    let owner = Address::generate(&env);
    let spender = Address::generate(&env);
    let receiver = Address::generate(&env);

    env.as_contract(&vault_id, || {
        env.storage()
            .instance()
            .set(&symbol_short!("usr_cap"), &500_i128);
    });

    let token = token::Client::new(&env, &token_id);
    let asset = token::StellarAssetClient::new(&env, &token_id);
    asset.mint(&owner, &1_000);
    token.approve(&owner, &vault_id, &1_000, &(env.ledger().sequence() + 100));

    assert!(vault
        .try_deposit_from(&spender, &owner, &600, &receiver)
        .is_err());
    assert_eq!(vault.shares_of(&receiver), 0);
}

// ── Issue #875: pool factory ────────────────────────────────────────────────

#[test]
fn factory_wasm_hash_is_admin_configured() {
    let env = Env::default();
    let (_vault, admin, _, vault_id) = setup_vault(&env);
    let hash = BytesN::from_array(&env, &[7_u8; 32]);

    env.as_contract(&vault_id, || {
        VaultContract::set_factory_wasm_hash(env.clone(), admin.clone(), hash.clone())
    })
    .unwrap();
    assert_eq!(
        env.as_contract(&vault_id, || VaultContract::get_factory_wasm_hash(
            env.clone()
        )),
        Some(hash)
    );

    let stranger = Address::generate(&env);
    let other = BytesN::from_array(&env, &[8_u8; 32]);
    assert!(env
        .as_contract(&vault_id, || VaultContract::set_factory_wasm_hash(
            env.clone(),
            stranger,
            other
        ))
        .is_err());
}

#[test]
fn pool_address_for_salt_is_deterministic() {
    let env = Env::default();
    let (_vault, _, _, vault_id) = setup_vault(&env);
    let salt_a = Bytes::from_slice(&env, b"alpha");
    let salt_b = Bytes::from_slice(&env, b"beta");

    let first = env.as_contract(&vault_id, || {
        VaultContract::pool_address_for_salt(env.clone(), salt_a.clone())
    });
    let second = env.as_contract(&vault_id, || {
        VaultContract::pool_address_for_salt(env.clone(), salt_a.clone())
    });
    let different = env.as_contract(&vault_id, || {
        VaultContract::pool_address_for_salt(env.clone(), salt_b)
    });

    assert_eq!(first, second);
    assert_ne!(first, different);
}

#[test]
fn create_pool_requires_a_wasm_hash() {
    let env = Env::default();
    let (_vault, admin, token_id, vault_id) = setup_vault(&env);
    let salt = Bytes::from_slice(&env, b"salt");
    let config = crate::pool_factory::PoolFactoryConfig {
        admin: admin.clone(),
        stake_token: token_id,
        reward_token: None,
        reward_rate_bps: 0,
    };

    assert!(env
        .as_contract(&vault_id, || VaultContract::create_pool(
            env.clone(),
            admin,
            salt,
            config
        ))
        .is_err());
}

#[test]
fn create_pool_rejects_duplicate_salt_before_deploying() {
    let env = Env::default();
    let (_vault, admin, token_id, vault_id) = setup_vault(&env);
    let salt = Bytes::from_slice(&env, b"reused");
    let wasm_hash = BytesN::from_array(&env, &[9_u8; 32]);
    env.as_contract(&vault_id, || {
        VaultContract::set_factory_wasm_hash(env.clone(), admin.clone(), wasm_hash).unwrap();
    });

    let config = crate::pool_factory::PoolFactoryConfig {
        admin: admin.clone(),
        stake_token: token_id,
        reward_token: None,
        reward_rate_bps: 0,
    };
    let salt_hash = env.crypto().sha256(&salt).to_bytes();
    env.as_contract(&vault_id, || {
        env.storage()
            .persistent()
            .set(&crate::pool_factory::salt_key(&salt_hash), &admin);
    });

    assert!(env
        .as_contract(&vault_id, || VaultContract::create_pool(
            env.clone(),
            admin,
            salt,
            config
        ))
        .is_err());
}

#[test]
fn create_pool_rejects_oversized_salt() {
    let env = Env::default();
    let (_vault, admin, token_id, vault_id) = setup_vault(&env);
    env.as_contract(&vault_id, || {
        VaultContract::set_factory_wasm_hash(
            env.clone(),
            admin.clone(),
            BytesN::from_array(&env, &[1_u8; 32]),
        )
        .unwrap();
    });
    let salt = Bytes::from_slice(&env, &[0_u8; 65]);
    let config = crate::pool_factory::PoolFactoryConfig {
        admin: admin.clone(),
        stake_token: token_id,
        reward_token: None,
        reward_rate_bps: 0,
    };

    assert!(env
        .as_contract(&vault_id, || VaultContract::create_pool(
            env.clone(),
            admin,
            salt,
            config
        ))
        .is_err());
}

// ── Issue #876: pool registry ───────────────────────────────────────────────

#[test]
fn register_pool_rejects_non_vault_address() {
    let env = Env::default();
    let (_vault, _, token_id, vault_id) = setup_vault(&env);

    assert!(env
        .as_contract(&vault_id, || VaultContract::register_pool(
            env.clone(),
            token_id
        ))
        .is_err());
}

#[test]
fn register_pool_rejects_foreign_admin() {
    let env = Env::default();
    let (_vault, _, token_id, vault_id) = setup_vault(&env);
    let other_admin = Address::generate(&env);
    let (_pool, pool_id) = deploy_pool(&env, &other_admin, &token_id);

    assert!(env
        .as_contract(&vault_id, || VaultContract::register_pool(
            env.clone(),
            pool_id
        ))
        .is_err());
}

#[test]
fn register_list_and_delist_pool() {
    let env = Env::default();
    let (_vault, admin, token_id, vault_id) = setup_vault(&env);
    let (_pool, pool_id) = deploy_pool(&env, &admin, &token_id);

    env.as_contract(&vault_id, || {
        VaultContract::register_pool(env.clone(), pool_id.clone()).unwrap()
    });
    assert_eq!(
        env.as_contract(&vault_id, || VaultContract::pool_registry_count(
            env.clone()
        )),
        1
    );

    let listed = env
        .as_contract(&vault_id, || VaultContract::list_pools(env.clone(), 0, 10))
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed.get(0).unwrap().pool, pool_id);

    let info = env
        .as_contract(&vault_id, || {
            VaultContract::get_pool_info(env.clone(), pool_id.clone())
        })
        .unwrap();
    assert!(info.active);
    assert_eq!(info.admin, admin);
    assert_eq!(info.version, String::from_str(&env, CONTRACT_VERSION));

    let reason = BytesN::from_array(&env, &[3_u8; 32]);
    env.as_contract(&vault_id, || {
        VaultContract::delist_pool(env.clone(), pool_id.clone(), reason.clone()).unwrap()
    });

    let delisted = env
        .as_contract(&vault_id, || {
            VaultContract::get_pool_info(env.clone(), pool_id.clone())
        })
        .unwrap();
    assert!(!delisted.active);
    assert_eq!(delisted.delist_reason, Some(reason.clone()));
    assert_eq!(
        env.as_contract(&vault_id, || VaultContract::pool_registry_count(
            env.clone()
        )),
        0
    );
    assert_eq!(
        env.as_contract(&vault_id, || VaultContract::list_pools(env.clone(), 0, 10))
            .unwrap()
            .len(),
        0
    );
    assert!(env
        .as_contract(&vault_id, || VaultContract::delist_pool(
            env.clone(),
            pool_id.clone(),
            reason.clone()
        ))
        .is_err());
    assert!(env
        .as_contract(&vault_id, || VaultContract::delist_pool(
            env.clone(),
            Address::generate(&env),
            reason
        ))
        .is_err());
}

#[test]
fn list_pools_paginates_active_pools() {
    let env = Env::default();
    let (_vault, admin, token_id, vault_id) = setup_vault(&env);
    let (_, first) = deploy_pool(&env, &admin, &token_id);
    let (_, second) = deploy_pool(&env, &admin, &token_id);
    let (_, third) = deploy_pool(&env, &admin, &token_id);
    env.as_contract(&vault_id, || {
        VaultContract::register_pool(env.clone(), first.clone()).unwrap();
        VaultContract::register_pool(env.clone(), second.clone()).unwrap();
        VaultContract::register_pool(env.clone(), third.clone()).unwrap();
    });

    let page_one = env
        .as_contract(&vault_id, || VaultContract::list_pools(env.clone(), 0, 2))
        .unwrap();
    assert_eq!(page_one.len(), 2);
    assert_eq!(page_one.get(0).unwrap().pool, first);
    assert_eq!(page_one.get(1).unwrap().pool, second);

    let page_two = env
        .as_contract(&vault_id, || VaultContract::list_pools(env.clone(), 2, 2))
        .unwrap();
    assert_eq!(page_two.len(), 1);
    assert_eq!(page_two.get(0).unwrap().pool, third);

    assert_eq!(
        env.as_contract(&vault_id, || VaultContract::list_pools(env.clone(), 9, 2))
            .unwrap()
            .len(),
        0
    );
    assert!(env
        .as_contract(&vault_id, || VaultContract::list_pools(env.clone(), 0, 0))
        .is_err());
    assert!(env
        .as_contract(&vault_id, || VaultContract::list_pools(
            env.clone(),
            0,
            crate::pool_registry::MAX_POOLS_PER_PAGE + 1
        ))
        .is_err());
}
