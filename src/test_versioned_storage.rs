#![cfg(test)]
//! Tests for the versioned enum wrappers around persisted structs.
//!
//! Each type is checked three ways: it round-trips through the public storage
//! helpers, the value on chain is the versioned enum, and an entry written in
//! the old bare-struct encoding (as released contracts wrote it) still reads
//! back correctly.

extern crate std;

use soroban_sdk::{
    symbol_short, testutils::Address as _, vec, Address, Env, IntoVal, Symbol, TryFromVal, Val,
};

use crate::{
    balance,
    storage::{GovernanceProposal, ProposableParam, StakePosition, VeBoostLock, VoteWeightMode},
    vault::VaultContract,
    versioned::{
        StakePositionV1, VeBoostLockV1, VersionedLock, VersionedPosition, VersionedProposal,
    },
};

fn position(amount: i128) -> StakePosition {
    StakePosition {
        amount,
        staked_at_ledger: 10,
        last_claim_ledger: 20,
    }
}

fn proposal(id: u32) -> GovernanceProposal {
    GovernanceProposal {
        id,
        parameter: ProposableParam::RewardRate,
        new_value: 500,
        votes_for: 7,
        votes_against: 3,
        ends_at: 1_000,
        enacted: false,
        weight_mode: VoteWeightMode::Linear,
    }
}

fn lock() -> VeBoostLock {
    VeBoostLock {
        amount: 5_000,
        unlock_ledger: 900,
        duration_ledgers: 400,
    }
}

fn split_key(env: &Env, user: &Address) -> (Symbol, Address) {
    (Symbol::new(env, "splitpos"), user.clone())
}

fn prop_key(env: &Env, id: u32) -> (Symbol, u32) {
    (Symbol::new(env, "prop"), id)
}

fn ve_key(user: &Address) -> (Symbol, Address) {
    (symbol_short!("ve_lock"), user.clone())
}

// ── StakePosition ────────────────────────────────────────────────────────────

#[test]
fn position_round_trips_and_is_stored_as_the_versioned_enum() {
    let env = Env::default();
    let contract = env.register_contract(None, VaultContract);
    let user = Address::generate(&env);

    env.as_contract(&contract, || {
        let positions = vec![&env, position(100), position(250)];
        balance::set_split_positions(&env, &user, &positions);
        assert_eq!(balance::get_split_positions(&env, &user), positions);

        let raw: soroban_sdk::Vec<Val> = env
            .storage()
            .persistent()
            .get(&split_key(&env, &user))
            .unwrap();
        for item in raw.iter() {
            assert!(VersionedPosition::try_from_val(&env, &item).is_ok());
            assert!(StakePosition::try_from_val(&env, &item).is_err());
        }
    });
}

#[test]
fn legacy_bare_position_list_still_reads() {
    let env = Env::default();
    let contract = env.register_contract(None, VaultContract);
    let user = Address::generate(&env);

    env.as_contract(&contract, || {
        let legacy = vec![&env, position(100), position(250)];
        env.storage()
            .persistent()
            .set(&split_key(&env, &user), &legacy);
        assert_eq!(balance::get_split_positions(&env, &user), legacy);

        // Saving again upgrades the entry to the versioned form.
        balance::set_split_positions(&env, &user, &legacy);
        assert_eq!(balance::get_split_positions(&env, &user), legacy);
    });
}

#[test]
fn position_list_with_mixed_old_and_new_entries_reads() {
    let env = Env::default();
    let contract = env.register_contract(None, VaultContract);
    let user = Address::generate(&env);

    env.as_contract(&contract, || {
        let mixed: soroban_sdk::Vec<Val> = vec![
            &env,
            position(1).into_val(&env),
            VersionedPosition::wrap(&position(2)).into_val(&env),
        ];
        env.storage()
            .persistent()
            .set(&split_key(&env, &user), &mixed);
        assert_eq!(
            balance::get_split_positions(&env, &user),
            vec![&env, position(1), position(2)]
        );
    });
}

#[test]
fn position_v1_normalizes_to_the_latest_struct() {
    let v1 = VersionedPosition::V1(StakePositionV1 {
        amount: 9,
        staked_at_ledger: 1,
        last_claim_ledger: 2,
    });
    assert_eq!(
        v1.clone().normalize(),
        StakePosition {
            amount: 9,
            staked_at_ledger: 1,
            last_claim_ledger: 2,
        }
    );
    assert_eq!(VersionedPosition::wrap(&v1.clone().normalize()), v1);
}

// ── GovernanceProposal ───────────────────────────────────────────────────────

#[test]
fn proposal_round_trips_and_is_stored_as_the_versioned_enum() {
    let env = Env::default();
    let contract = env.register_contract(None, VaultContract);

    env.as_contract(&contract, || {
        let p = proposal(1);
        balance::set_proposal(&env, 1, &p);
        assert_eq!(balance::get_proposal(&env, 1), Some(p));

        let raw: Val = env.storage().persistent().get(&prop_key(&env, 1)).unwrap();
        assert!(VersionedProposal::try_from_val(&env, &raw).is_ok());
        assert!(GovernanceProposal::try_from_val(&env, &raw).is_err());
    });
}

#[test]
fn legacy_bare_proposal_still_reads_and_can_be_updated() {
    let env = Env::default();
    let contract = env.register_contract(None, VaultContract);

    env.as_contract(&contract, || {
        let p = proposal(2);
        env.storage().persistent().set(&prop_key(&env, 2), &p);
        assert_eq!(balance::get_proposal(&env, 2), Some(p.clone()));

        let mut updated = p;
        updated.votes_for = 99;
        balance::set_proposal(&env, 2, &updated);
        assert_eq!(balance::get_proposal(&env, 2), Some(updated));
    });
}

#[test]
fn missing_proposal_is_none() {
    let env = Env::default();
    let contract = env.register_contract(None, VaultContract);
    env.as_contract(&contract, || {
        assert_eq!(balance::get_proposal(&env, 42), None);
    });
}

#[test]
fn proposal_list_round_trips_and_reads_legacy() {
    let env = Env::default();
    let contract = env.register_contract(None, VaultContract);

    env.as_contract(&contract, || {
        let list = vec![&env, proposal(1), proposal(2)];
        balance::set_governance_proposals(&env, &list);
        assert_eq!(balance::get_governance_proposals(&env), list);

        let legacy = vec![&env, proposal(3)];
        env.storage()
            .instance()
            .set(&symbol_short!("gov_prop"), &legacy);
        assert_eq!(balance::get_governance_proposals(&env), legacy);
    });
}

// ── VeBoostLock ──────────────────────────────────────────────────────────────

#[test]
fn lock_round_trips_and_is_stored_as_the_versioned_enum() {
    let env = Env::default();
    let contract = env.register_contract(None, VaultContract);
    let user = Address::generate(&env);

    env.as_contract(&contract, || {
        crate::versioned::write_lock(&env, &ve_key(&user), &lock());
        assert_eq!(
            crate::versioned::read_lock(&env, &ve_key(&user)),
            Some(lock())
        );

        let raw: Val = env.storage().persistent().get(&ve_key(&user)).unwrap();
        assert!(VersionedLock::try_from_val(&env, &raw).is_ok());
        assert!(VeBoostLock::try_from_val(&env, &raw).is_err());
    });
}

#[test]
fn legacy_bare_lock_still_reads() {
    let env = Env::default();
    let contract = env.register_contract(None, VaultContract);
    let user = Address::generate(&env);

    env.as_contract(&contract, || {
        env.storage().persistent().set(&ve_key(&user), &lock());
        assert_eq!(
            crate::versioned::read_lock(&env, &ve_key(&user)),
            Some(lock())
        );
    });
}

#[test]
fn lock_v1_normalizes_to_the_latest_struct() {
    let v1 = VersionedLock::V1(VeBoostLockV1 {
        amount: 1,
        unlock_ledger: 2,
        duration_ledgers: 3,
    });
    assert_eq!(
        v1.clone().normalize(),
        VeBoostLock {
            amount: 1,
            unlock_ledger: 2,
            duration_ledgers: 3,
        }
    );
    assert_eq!(VersionedLock::wrap(&v1.clone().normalize()), v1);
}
