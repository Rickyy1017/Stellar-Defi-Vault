#![cfg(test)]
//! Tests for the feature-flag registry that gates optional modules.

extern crate std;

use soroban_sdk::{testutils::Address as _, token, Address, Env};

use crate::{
    errors::{FeatureFlagError, VaultCampaignError, VaultError, VaultExtError},
    feature_flags::Feature,
    vault::{VaultContract, VaultContractClient},
};

const START_BALANCE: i128 = 1_000_000;

struct Fixture<'a> {
    env: Env,
    vault: VaultContractClient<'a>,
    admin: Address,
    alice: Address,
    bob: Address,
}

impl<'a> Fixture<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract(admin.clone());
        let token_admin = token::StellarAssetClient::new(&env, &token_address);
        token_admin.mint(&admin, &START_BALANCE);
        token_admin.mint(&alice, &START_BALANCE);
        token_admin.mint(&bob, &START_BALANCE);

        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &token_address, &0_u32, &None, &None);

        Fixture { env, vault, admin, alice, bob }
    }

    fn set(&self, feature: Feature, enabled: bool) {
        self.vault.set_feature_enabled(&self.admin, &feature, &enabled);
    }
}

const ALL: [Feature; 4] = [
    Feature::Lockdrop,
    Feature::Referrals,
    Feature::Mirroring,
    Feature::Hooks,
];

#[test]
fn every_feature_defaults_to_enabled() {
    let f = Fixture::new();
    for feature in ALL {
        assert!(f.vault.is_feature_enabled(&feature));
    }
}

#[test]
fn admin_can_toggle_a_feature_without_touching_the_others() {
    let f = Fixture::new();
    f.set(Feature::Mirroring, false);
    assert!(!f.vault.is_feature_enabled(&Feature::Mirroring));
    assert!(f.vault.is_feature_enabled(&Feature::Lockdrop));
    assert!(f.vault.is_feature_enabled(&Feature::Referrals));
    assert!(f.vault.is_feature_enabled(&Feature::Hooks));

    f.set(Feature::Mirroring, true);
    assert!(f.vault.is_feature_enabled(&Feature::Mirroring));
}

#[test]
fn only_the_admin_can_set_a_feature() {
    let f = Fixture::new();
    assert_eq!(
        f.vault.try_set_feature_enabled(&f.alice, &Feature::Hooks, &false),
        Err(Ok(FeatureFlagError::Unauthorized))
    );
    assert!(f.vault.is_feature_enabled(&Feature::Hooks));
}

#[test]
fn lockdrop_entrypoints_revert_when_off() {
    let f = Fixture::new();
    f.set(Feature::Lockdrop, false);
    assert_eq!(
        f.vault.try_start_lockdrop(&f.admin, &10_000, &1_000, &500),
        Err(Ok(VaultCampaignError::FeatureDisabled))
    );

    // Turning it back on restores the normal flow.
    f.set(Feature::Lockdrop, true);
    f.vault.start_lockdrop(&f.admin, &10_000, &1_000, &500);
    f.vault.commit_to_lockdrop(&f.alice, &1_000, &100);

    // Switching off after a commitment blocks new commitments only.
    f.set(Feature::Lockdrop, false);
    assert_eq!(
        f.vault.try_commit_to_lockdrop(&f.bob, &1_000, &100),
        Err(Ok(VaultCampaignError::FeatureDisabled))
    );
    assert!(f.vault.get_lockdrop_commitment(&f.alice).is_some());
}

#[test]
fn referral_entrypoints_revert_when_off_but_plain_deposit_works() {
    let f = Fixture::new();
    f.set(Feature::Referrals, false);

    assert_eq!(
        f.vault.try_register_referrer(&f.alice),
        Err(Ok(VaultError::FeatureDisabled))
    );
    assert_eq!(
        f.vault.try_stake_with_referral(&f.bob, &1_000, &f.alice),
        Err(Ok(VaultError::FeatureDisabled))
    );
    assert_eq!(
        f.vault.try_deposit(&f.bob, &1_000, &Some(f.alice.clone())),
        Err(Ok(VaultError::FeatureDisabled))
    );

    // A deposit without a referrer is not a referral action.
    f.vault.deposit(&f.bob, &1_000, &None);
    assert!(f.vault.shares_of(&f.bob) > 0);
}

#[test]
fn mirror_config_reverts_when_off_and_cancel_stays_open() {
    let f = Fixture::new();
    f.vault.set_mirror_config(&f.bob, &f.alice, &5_000, &1_000, &0);

    f.set(Feature::Mirroring, false);
    assert_eq!(
        f.vault.try_set_mirror_config(&f.bob, &f.alice, &5_000, &1_000, &0),
        Err(Ok(VaultExtError::FeatureDisabled))
    );

    // Opting out is always possible.
    f.vault.cancel_mirroring(&f.bob);
    assert!(f.vault.get_mirror_config(&f.bob).is_none());
}

#[test]
fn disabled_mirroring_stops_followers_copying_a_leader() {
    let f = Fixture::new();
    f.vault.set_mirror_config(&f.bob, &f.alice, &10_000, &1_000, &0);
    f.set(Feature::Mirroring, false);

    f.vault.deposit(&f.alice, &1_000, &None);
    assert_eq!(f.vault.shares_of(&f.bob), 0);
}

#[test]
fn register_hook_reverts_when_off_and_remove_stays_open() {
    let f = Fixture::new();
    let hook = Address::generate(&f.env);
    f.vault.register_hook(&f.admin, &hook);

    f.set(Feature::Hooks, false);
    let other = Address::generate(&f.env);
    assert_eq!(
        f.vault.try_register_hook(&f.admin, &other),
        Err(Ok(VaultError::FeatureDisabled))
    );

    f.vault.remove_hook(&f.admin, &hook);
    assert!(f.vault.get_hooks().is_empty());
}

#[test]
fn disabled_hooks_are_not_notified() {
    let f = Fixture::new();
    // An address with no contract behind it would make the call fail and emit
    // `hook_fail`; with the flag off the hook must not be called at all, so the
    // stake succeeds quietly.
    let hook = Address::generate(&f.env);
    f.vault.register_hook(&f.admin, &hook);
    f.set(Feature::Hooks, false);

    f.vault.deposit(&f.alice, &1_000, &None);
    assert!(f.vault.shares_of(&f.alice) > 0);
}
