#![cfg(test)]
//! Tests for the on-chain pool metadata record.

extern crate std;

use soroban_sdk::{testutils::Address as _, token, Address, BytesN, Env, String};

use crate::{
    errors::PoolMetadataError,
    pool_metadata::PoolMetadata,
    vault::{VaultContract, VaultContractClient},
};

fn setup<'a>() -> (Env, VaultContractClient<'a>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract(admin.clone());
    let _ = token::Client::new(&env, &token_address);
    let vault_id = env.register_contract(None, VaultContract);
    let vault = VaultContractClient::new(&env, &vault_id);
    vault.initialize(&admin, &token_address, &0_u32, &None, &None);
    (env, vault, admin)
}

fn metadata(env: &Env, name: &str) -> PoolMetadata {
    PoolMetadata {
        name: String::from_str(env, name),
        description: String::from_str(env, "Stake XLM, earn rewards."),
        website: String::from_str(env, "https://example.org"),
        icon_hash: BytesN::from_array(env, &[7u8; 32]),
    }
}

fn repeat(env: &Env, len: usize) -> String {
    String::from_str(env, &"a".repeat(len))
}

#[test]
fn unset_by_default() {
    let (_env, vault, _admin) = setup();
    assert_eq!(vault.get_pool_metadata(), None);
}

#[test]
fn admin_can_set_and_read_back() {
    let (env, vault, admin) = setup();
    let m = metadata(&env, "Main Pool");
    vault.set_pool_metadata(&admin, &m);
    assert_eq!(vault.get_pool_metadata(), Some(m));
}

#[test]
fn update_replaces_the_whole_record() {
    let (env, vault, admin) = setup();
    vault.set_pool_metadata(&admin, &metadata(&env, "Old"));
    let new = metadata(&env, "New");
    vault.set_pool_metadata(&admin, &new);
    assert_eq!(vault.get_pool_metadata(), Some(new));
}

#[test]
fn non_admin_is_rejected() {
    let (env, vault, _admin) = setup();
    let stranger = Address::generate(&env);
    assert_eq!(
        vault.try_set_pool_metadata(&stranger, &metadata(&env, "Pool")),
        Err(Ok(PoolMetadataError::Unauthorized))
    );
}

#[test]
fn empty_name_is_rejected() {
    let (env, vault, admin) = setup();
    assert_eq!(
        vault.try_set_pool_metadata(&admin, &metadata(&env, "")),
        Err(Ok(PoolMetadataError::EmptyName))
    );
}

#[test]
fn length_limits_are_enforced_at_the_boundary() {
    let (env, vault, admin) = setup();

    let mut m = metadata(&env, "Pool");
    m.name = repeat(&env, 64);
    m.description = repeat(&env, 512);
    m.website = repeat(&env, 128);
    vault.set_pool_metadata(&admin, &m);

    let mut m = metadata(&env, "Pool");
    m.name = repeat(&env, 65);
    assert_eq!(
        vault.try_set_pool_metadata(&admin, &m),
        Err(Ok(PoolMetadataError::NameTooLong))
    );

    let mut m = metadata(&env, "Pool");
    m.description = repeat(&env, 513);
    assert_eq!(
        vault.try_set_pool_metadata(&admin, &m),
        Err(Ok(PoolMetadataError::DescriptionTooLong))
    );

    let mut m = metadata(&env, "Pool");
    m.website = repeat(&env, 129);
    assert_eq!(
        vault.try_set_pool_metadata(&admin, &m),
        Err(Ok(PoolMetadataError::WebsiteTooLong))
    );
}

#[test]
fn rejected_update_leaves_existing_metadata_untouched() {
    let (env, vault, admin) = setup();
    let good = metadata(&env, "Good");
    vault.set_pool_metadata(&admin, &good);
    let _ = vault.try_set_pool_metadata(&admin, &metadata(&env, ""));
    assert_eq!(vault.get_pool_metadata(), Some(good));
}

#[test]
fn changes_are_recorded_in_the_parameter_change_log() {
    let (env, vault, admin) = setup();
    let before = vault.pool_parameter_change_log(&0, &100).len();

    vault.set_pool_metadata(&admin, &metadata(&env, "First"));
    vault.set_pool_metadata(&admin, &metadata(&env, "Second"));

    let log = vault.pool_parameter_change_log(&0, &100);
    assert_eq!(log.len(), before + 2);

    let first = log.get(before).unwrap();
    assert_eq!(first.parameter, String::from_str(&env, "pool_metadata"));
    assert_eq!(first.changed_by, admin);
    assert_eq!((first.old_value, first.new_value), (0, 1));
    assert_eq!(first.reason, String::from_str(&env, "First"));

    let second = log.get(before + 1).unwrap();
    assert_eq!((second.old_value, second.new_value), (1, 2));
    assert_eq!(second.reason, String::from_str(&env, "Second"));
}
