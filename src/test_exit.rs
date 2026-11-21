#![cfg(test)]
//! Tests for `exit()`: claim rewards and withdraw the whole position atomically.

extern crate std;

use soroban_sdk::{testutils::{Address as _, Ledger as _}, token, Address, BytesN, Env};

use crate::{
    errors::VaultError,
    vault::{VaultContract, VaultContractClient},
};

const STAKE: i128 = 100_000;
const START_BALANCE: i128 = 1_000_000;

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
        token_admin.mint(&admin, &START_BALANCE);
        token_admin.mint(&alice, &START_BALANCE);
        token_admin.mint(&bob, &START_BALANCE);

        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &token_address, &0_u32, &None, &None);

        let token = token::Client::new(&env, &token_address);
        Fixture { env, vault, token, admin, alice, bob }
    }
}

#[test]
fn exit_withdraws_the_whole_position_to_the_recipient() {
    let f = Fixture::new();
    f.vault.stake(&f.alice, &STAKE, &0);
    let bob_before = f.token.balance(&f.bob);

    let (principal, _reward) = f.vault.exit(&f.alice, &f.bob);

    assert_eq!(principal, STAKE);
    assert_eq!(f.token.balance(&f.bob), bob_before + STAKE);
    assert_eq!(f.vault.shares_of(&f.alice), 0);
}

#[test]
fn exit_applies_the_standard_unstake_fee() {
    let f = Fixture::new();
    f.vault.set_unstake_fee_bps(&f.admin, &100); // 1%
    f.vault.stake(&f.alice, &STAKE, &0);
    let bob_before = f.token.balance(&f.bob);

    let (principal, _) = f.vault.exit(&f.alice, &f.bob);

    assert_eq!(principal, STAKE - STAKE / 100);
    assert_eq!(f.token.balance(&f.bob), bob_before + principal);
}

#[test]
fn exit_pays_accrued_rewards_to_the_user() {
    let f = Fixture::new();
    f.vault.set_reward_rate_bps(&1_000);
    f.vault.fund_reward_pool(&f.admin, &50_000);
    f.vault.stake(&f.alice, &STAKE, &0);
    f.env.ledger().with_mut(|l| l.sequence_number += 100_000);
    let alice_before = f.token.balance(&f.alice);

    let (_, reward) = f.vault.exit(&f.alice, &f.bob);

    // Whatever reward accrued lands on alice (the user), not the recipient.
    assert_eq!(f.token.balance(&f.alice), alice_before + reward);
}

#[test]
fn exit_without_a_position_reverts() {
    let f = Fixture::new();
    assert_eq!(
        f.vault.try_exit(&f.alice, &f.alice),
        Err(Ok(VaultError::PositionNotFound))
    );
}

#[test]
fn failed_exit_leaves_the_position_untouched() {
    let f = Fixture::new();
    f.vault.stake(&f.alice, &STAKE, &0);
    // A blocked recipient makes the principal leg fail; nothing may change.
    f.vault
        .block_address(&f.admin, &f.bob, &BytesN::from_array(&f.env, &[1u8; 32]));
    let shares = f.vault.shares_of(&f.alice);

    assert_eq!(
        f.vault.try_exit(&f.alice, &f.bob),
        Err(Ok(VaultError::AddressBlocked))
    );
    assert_eq!(f.vault.shares_of(&f.alice), shares);
}

#[test]
fn blocked_user_can_exit_to_their_own_address_only() {
    let f = Fixture::new();
    f.vault.stake(&f.alice, &STAKE, &0);
    f.vault
        .block_address(&f.admin, &f.alice, &BytesN::from_array(&f.env, &[1u8; 32]));

    assert_eq!(
        f.vault.try_exit(&f.alice, &f.bob),
        Err(Ok(VaultError::AddressBlocked))
    );

    let (principal, reward) = f.vault.exit(&f.alice, &f.alice);
    assert_eq!(principal, STAKE);
    assert_eq!(reward, 0);
    assert_eq!(f.vault.shares_of(&f.alice), 0);
}
