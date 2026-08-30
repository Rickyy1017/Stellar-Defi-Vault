#![cfg(test)]
extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, Symbol, TryFromVal, Vec,
};

use crate::{
    storage::{PointsAction, PointsBenefit, PointsRule},
    vault::{VaultContract, VaultContractClient},
};

fn set_ledger(env: &Env, seq: u32) {
    env.ledger().with_mut(|li| li.sequence_number = seq);
}

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
        env.ledger().with_mut(|li| {
            li.min_temp_entry_ttl = 10_000_000;
            li.min_persistent_entry_ttl = 10_000_000;
            li.max_entry_ttl = 10_000_000;
        });
        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);
        let token = env.register_stellar_asset_contract(admin.clone());
        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &token, &500_u32, &None, &None);
        set_ledger(&env, 1000);
        Fixture {
            env,
            vault,
            admin,
            alice,
            bob,
        }
    }
}

fn topic_matches(env: &Env, topics: &Vec<soroban_sdk::Val>, name: &str) -> bool {
    match topics.get(0) {
        Some(val) => Symbol::try_from_val(env, &val)
            .map(|s| s == Symbol::new(env, name))
            .unwrap_or(false),
        None => false,
    }
}

#[test]
fn points_earned_on_each_action_type() {
    let f = Fixture::new();
    let rules = Vec::from_array(
        &f.env,
        [
            PointsRule {
                action: PointsAction::PerLedgerStaked,
                points_per_action: 2,
            },
            PointsRule {
                action: PointsAction::PerClaim,
                points_per_action: 10,
            },
            PointsRule {
                action: PointsAction::PerGovernanceVote,
                points_per_action: 5,
            },
            PointsRule {
                action: PointsAction::PerMilestone,
                points_per_action: 20,
            },
            PointsRule {
                action: PointsAction::PerReferral,
                points_per_action: 15,
            },
        ],
    );
    f.vault.set_points_rules(&f.admin, &rules);
    // PerClaim
    f.env.as_contract(&f.vault.address, || {
        crate::loyalty_points::award_points(&f.env, &f.alice, 10);
    });
    let (bal, lifetime) = f.vault.get_loyalty_points(&f.alice);
    assert_eq!(bal, 10);
    assert_eq!(lifetime, 10);
    // PerGovernanceVote
    f.env.as_contract(&f.vault.address, || {
        crate::loyalty_points::award_points_for_action(&f.env, &f.alice, PointsAction::PerGovernanceVote);
    });
    let (bal, lifetime) = f.vault.get_loyalty_points(&f.alice);
    assert_eq!(bal, 15);
    assert_eq!(lifetime, 15);
    // PerMilestone
    f.env.as_contract(&f.vault.address, || {
        crate::loyalty_points::award_points_for_action(&f.env, &f.alice, PointsAction::PerMilestone);
    });
    let (bal, _) = f.vault.get_loyalty_points(&f.alice);
    assert_eq!(bal, 35);
    // PerReferral
    f.env.as_contract(&f.vault.address, || {
        crate::loyalty_points::award_points_for_action(&f.env, &f.alice, PointsAction::PerReferral);
    });
    let (bal, _) = f.vault.get_loyalty_points(&f.alice);
    assert_eq!(bal, 50);
}

#[test]
fn per_ledger_staked_points_based_on_elapsed() {
    let f = Fixture::new();
    let rules = Vec::from_array(
        &f.env,
        [PointsRule {
            action: PointsAction::PerLedgerStaked,
            points_per_action: 1,
        }],
    );
    f.vault.set_points_rules(&f.admin, &rules);
    // Simulate last claim at 1000, now at 1100, elapsed 100
    f.env.as_contract(&f.vault.address, || {
        crate::balance::set_last_claim_ledger(&f.env, &f.alice, 1000);
    });
    set_ledger(&f.env, 1100);
    f.env.as_contract(&f.vault.address, || {
        crate::loyalty_points::award_points_for_action(&f.env, &f.alice, PointsAction::PerLedgerStaked);
    });
    let (bal, lifetime) = f.vault.get_loyalty_points(&f.alice);
    assert_eq!(bal, 100);
    assert_eq!(lifetime, 100);
}

#[test]
fn redemption_deducts_balance() {
    let f = Fixture::new();
    f.env.as_contract(&f.vault.address, || {
        crate::loyalty_points::award_points(&f.env, &f.alice, 100);
    });
    f.vault.redeem_points(&f.alice, &60, &PointsBenefit::FeeWaiver);
    let (bal, lifetime) = f.vault.get_loyalty_points(&f.alice);
    assert_eq!(bal, 40);
    assert_eq!(lifetime, 100);
    assert!(f.vault.has_loyalty_benefit(&f.alice, &PointsBenefit::FeeWaiver));
    let events = f.env.events().all();
    let found = events.iter().any(|e| topic_matches(&f.env, &e.0, "loy_rdm"));
    assert!(found, "loy_rdm event not found");
}

#[test]
fn insufficient_balance_reverts() {
    let f = Fixture::new();
    f.env.as_contract(&f.vault.address, || {
        crate::loyalty_points::award_points(&f.env, &f.alice, 10);
    });
    let res = f.vault.try_redeem_points(&f.alice, &20, &PointsBenefit::BoostUnlock);
    assert!(res.is_err());
}

#[test]
fn benefit_applied_correctly() {
    let f = Fixture::new();
    f.env.as_contract(&f.vault.address, || {
        crate::loyalty_points::award_points(&f.env, &f.alice, 50);
        crate::loyalty_points::award_points(&f.env, &f.bob, 50);
    });
    f.vault.redeem_points(&f.alice, &10, &PointsBenefit::BoostUnlock);
    f.vault.redeem_points(&f.bob, &10, &PointsBenefit::EarlyAccess);
    assert!(f.vault.has_loyalty_benefit(&f.alice, &PointsBenefit::BoostUnlock));
    assert!(!f.vault.has_loyalty_benefit(&f.alice, &PointsBenefit::EarlyAccess));
    assert!(f.vault.has_loyalty_benefit(&f.bob, &PointsBenefit::EarlyAccess));
}
