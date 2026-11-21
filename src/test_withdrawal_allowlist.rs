#![cfg(test)]
//! Tests for the user-defined withdrawal destination allowlist.

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, vec, Address, Env, Vec,
};

use crate::{
    errors::VaultError,
    vault::{VaultContract, VaultContractClient},
};

const STAKE: i128 = 100_000;
const DELAY: u32 = 1_000;

struct Fixture<'a> {
    env: Env,
    vault: VaultContractClient<'a>,
    token: token::Client<'a>,
    admin: Address,
    alice: Address,
    cold: Address,
    attacker: Address,
}

impl<'a> Fixture<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.sequence_number = 100);

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let cold = Address::generate(&env);
        let attacker = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract(admin.clone());
        token::StellarAssetClient::new(&env, &token_address).mint(&alice, &1_000_000);

        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &token_address, &0_u32, &None, &None);
        vault.set_allowlist_delay(&admin, &DELAY);
        vault.stake(&alice, &STAKE, &0);

        let token = token::Client::new(&env, &token_address);
        Fixture { env, vault, token, admin, alice, cold, attacker }
    }

    fn advance(&self, ledgers: u32) {
        self.env.ledger().with_mut(|l| l.sequence_number += ledgers);
    }

    /// Set the list and let the delay elapse so it is in force.
    fn activate(&self, list: Vec<Address>) {
        self.vault.set_withdrawal_allowlist(&self.alice, &list);
        self.advance(DELAY);
    }
}

#[test]
fn no_list_means_no_restriction() {
    let f = Fixture::new();
    assert!(f.vault.get_withdrawal_allowlist(&f.alice).is_empty());
    let shares = f.vault.shares_of(&f.alice) / 2;
    f.vault.withdraw_to(&f.alice, &shares, &f.attacker);
}

#[test]
fn at_most_five_addresses() {
    let f = Fixture::new();
    let mut list = Vec::new(&f.env);
    for _ in 0..5 {
        list.push_back(Address::generate(&f.env));
    }
    f.vault.set_withdrawal_allowlist(&f.alice, &list);

    list.push_back(Address::generate(&f.env));
    assert_eq!(
        f.vault.try_set_withdrawal_allowlist(&f.alice, &list),
        Err(Ok(VaultError::AllowlistTooLong))
    );
}

#[test]
fn change_takes_effect_only_after_the_delay() {
    let f = Fixture::new();
    f.vault
        .set_withdrawal_allowlist(&f.alice, &vec![&f.env, f.cold.clone()]);

    // Still pending: not yet returned, and not yet enforced.
    assert!(f.vault.get_withdrawal_allowlist(&f.alice).is_empty());
    assert!(f.vault.get_pending_withdrawal_allowlist(&f.alice).is_some());
    let shares = f.vault.shares_of(&f.alice) / 4;
    f.vault.withdraw_to(&f.alice, &shares, &f.attacker);

    f.advance(DELAY);
    assert_eq!(
        f.vault.get_withdrawal_allowlist(&f.alice),
        vec![&f.env, f.cold.clone()]
    );
    assert!(f.vault.get_pending_withdrawal_allowlist(&f.alice).is_none());
}

#[test]
fn withdraw_to_must_be_on_the_list() {
    let f = Fixture::new();
    f.activate(vec![&f.env, f.cold.clone()]);
    let shares = f.vault.shares_of(&f.alice) / 2;

    assert_eq!(
        f.vault.try_withdraw_to(&f.alice, &shares, &f.attacker),
        Err(Ok(VaultError::DestinationNotAllowed))
    );

    let cold_before = f.token.balance(&f.cold);
    f.vault.withdraw_to(&f.alice, &shares, &f.cold);
    assert!(f.token.balance(&f.cold) > cold_before);
}

#[test]
fn claim_and_plain_withdraw_need_the_users_own_address_on_the_list() {
    let f = Fixture::new();
    f.activate(vec![&f.env, f.cold.clone()]);
    let shares = f.vault.shares_of(&f.alice) / 2;

    // Alice's own address is not listed, so the key itself cannot take funds.
    assert_eq!(
        f.vault.try_claim(&f.alice),
        Err(Ok(VaultError::DestinationNotAllowed))
    );
    assert_eq!(
        f.vault.try_withdraw(&f.alice, &shares),
        Err(Ok(VaultError::DestinationNotAllowed))
    );

    // Listing her own address as well re-opens that path.
    f.activate(vec![&f.env, f.cold.clone(), f.alice.clone()]);
    f.vault.withdraw(&f.alice, &shares);
}

#[test]
fn clearing_the_list_is_also_delayed() {
    let f = Fixture::new();
    f.activate(vec![&f.env, f.cold.clone()]);

    // A thief with the key tries to lift the restriction and withdraw at once.
    f.vault
        .set_withdrawal_allowlist(&f.alice, &Vec::new(&f.env));
    let shares = f.vault.shares_of(&f.alice) / 2;
    assert_eq!(
        f.vault.try_withdraw_to(&f.alice, &shares, &f.attacker),
        Err(Ok(VaultError::DestinationNotAllowed))
    );

    f.advance(DELAY);
    assert!(f.vault.get_withdrawal_allowlist(&f.alice).is_empty());
    f.vault.withdraw_to(&f.alice, &shares, &f.attacker);
}

#[test]
fn exit_respects_the_allowlist() {
    let f = Fixture::new();
    f.activate(vec![&f.env, f.cold.clone()]);

    assert_eq!(
        f.vault.try_exit(&f.alice, &f.attacker),
        Err(Ok(VaultError::DestinationNotAllowed))
    );
    assert!(f.vault.shares_of(&f.alice) > 0);
}

#[test]
fn only_the_admin_sets_the_delay() {
    let f = Fixture::new();
    assert_eq!(
        f.vault.try_set_allowlist_delay(&f.alice, &0),
        Err(Ok(VaultError::Unauthorized))
    );
    f.vault.set_allowlist_delay(&f.admin, &50);
    assert_eq!(f.vault.get_allowlist_delay(), 50);
}
