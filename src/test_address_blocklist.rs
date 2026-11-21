#![cfg(test)]
//! Tests for the admin address blocklist and its policy for blocked users' funds.

extern crate std;

use soroban_sdk::{testutils::Address as _, token, Address, BytesN, Env};

use crate::{
    errors::VaultError,
    vault::{VaultContract, VaultContractClient},
};

const STAKE: i128 = 100_000;

struct Fixture<'a> {
    env: Env,
    vault: VaultContractClient<'a>,
    token: token::Client<'a>,
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
        token_admin.mint(&alice, &1_000_000);
        token_admin.mint(&bob, &1_000_000);

        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &token_address, &0_u32, &None, &None);

        let token = token::Client::new(&env, &token_address);
        Fixture { env, vault, token, admin, alice, bob }
    }

    fn reason(&self) -> BytesN<32> {
        BytesN::from_array(&self.env, &[7u8; 32])
    }

    fn block(&self, user: &Address) {
        self.vault.block_address(&self.admin, user, &self.reason());
    }
}

#[test]
fn block_and_unblock_toggle_is_blocked() {
    let f = Fixture::new();
    assert!(!f.vault.is_blocked(&f.alice));

    f.block(&f.alice);
    assert!(f.vault.is_blocked(&f.alice));
    assert_eq!(f.vault.get_block_reason(&f.alice), Some(f.reason()));
    assert!(!f.vault.is_blocked(&f.bob));

    f.vault.unblock_address(&f.admin, &f.alice);
    assert!(!f.vault.is_blocked(&f.alice));
}

#[test]
fn only_the_admin_can_block_or_unblock() {
    let f = Fixture::new();
    assert_eq!(
        f.vault.try_block_address(&f.bob, &f.alice, &f.reason()),
        Err(Ok(VaultError::Unauthorized))
    );
    f.block(&f.alice);
    assert_eq!(
        f.vault.try_unblock_address(&f.bob, &f.alice),
        Err(Ok(VaultError::Unauthorized))
    );
    assert!(f.vault.is_blocked(&f.alice));
}

#[test]
fn blocked_address_cannot_deposit_or_stake() {
    let f = Fixture::new();
    f.block(&f.alice);
    assert_eq!(
        f.vault.try_stake(&f.alice, &STAKE, &0),
        Err(Ok(VaultError::AddressBlocked))
    );
    assert_eq!(
        f.vault.try_deposit(&f.alice, &STAKE, &None),
        Err(Ok(VaultError::AddressBlocked))
    );
    assert_eq!(f.vault.shares_of(&f.alice), 0);

    // Unblocking restores access.
    f.vault.unblock_address(&f.admin, &f.alice);
    f.vault.stake(&f.alice, &STAKE, &0);
    assert!(f.vault.shares_of(&f.alice) > 0);
}

#[test]
fn blocked_address_cannot_claim_rewards() {
    let f = Fixture::new();
    f.vault.stake(&f.alice, &STAKE, &0);
    f.block(&f.alice);
    assert_eq!(f.vault.try_claim(&f.alice), Err(Ok(VaultError::AddressBlocked)));
    assert_eq!(
        f.vault.try_claim_partial(&f.alice, &1),
        Err(Ok(VaultError::AddressBlocked))
    );
}

#[test]
fn blocked_address_cannot_send_or_receive_a_position() {
    let f = Fixture::new();
    f.vault.stake(&f.alice, &STAKE, &0);
    f.block(&f.bob);
    // Receiving.
    assert_eq!(
        f.vault.try_transfer_position(&f.alice, &f.bob),
        Err(Ok(VaultError::AddressBlocked))
    );
    // Sending.
    f.vault.unblock_address(&f.admin, &f.bob);
    f.block(&f.alice);
    assert_eq!(
        f.vault.try_transfer_position(&f.alice, &f.bob),
        Err(Ok(VaultError::AddressBlocked))
    );
}

#[test]
fn blocked_users_principal_is_withdrawable_to_their_own_address() {
    let f = Fixture::new();
    f.vault.stake(&f.alice, &STAKE, &0);
    let before = f.token.balance(&f.alice);
    f.block(&f.alice);

    let shares = f.vault.shares_of(&f.alice);
    let returned = f.vault.withdraw(&f.alice, &shares);

    assert_eq!(returned, STAKE);
    assert_eq!(f.token.balance(&f.alice), before + STAKE);
    assert_eq!(f.vault.shares_of(&f.alice), 0);
}

#[test]
fn blocked_users_principal_cannot_be_redirected() {
    let f = Fixture::new();
    f.vault.stake(&f.alice, &STAKE, &0);
    f.block(&f.alice);
    let shares = f.vault.shares_of(&f.alice);

    assert_eq!(
        f.vault.try_withdraw_to(&f.alice, &shares, &f.bob),
        Err(Ok(VaultError::AddressBlocked))
    );
    assert_eq!(f.vault.shares_of(&f.alice), shares);
}

#[test]
fn withdrawal_to_a_blocked_recipient_is_refused() {
    let f = Fixture::new();
    f.vault.stake(&f.alice, &STAKE, &0);
    f.block(&f.bob);
    let shares = f.vault.shares_of(&f.alice);

    assert_eq!(
        f.vault.try_withdraw_to(&f.alice, &shares, &f.bob),
        Err(Ok(VaultError::AddressBlocked))
    );
}
