#![cfg(test)]
//! Unit tests for the gauge vote bribe and incentive market (issue #936).

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env, Vec,
};

use crate::gauge_incentives::{
    incentive_expiry_ledger, GaugeIncentiveError, GAUGE_EPOCH_LEDGERS,
};
use crate::vault::{VaultContract, VaultContractClient};

struct Fixture<'a> {
    env: Env,
    vault: VaultContractClient<'a>,
    bribe_token: token::Client<'a>,
    bribe_token_addr: Address,
    sponsor: Address,
    alice: Address,
    bob: Address,
}

impl<'a> Fixture<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();
        env.ledger().with_mut(|li| {
            li.sequence_number = 100; // Epoch 0
            li.min_temp_entry_ttl = 10_000_000;
            li.min_persistent_entry_ttl = 10_000_000;
            li.max_entry_ttl = 10_000_000;
        });

        let admin = Address::generate(&env);
        let sponsor = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        let vault_token_addr = env.register_stellar_asset_contract(admin.clone());
        let vault_token_admin = token::StellarAssetClient::new(&env, &vault_token_addr);
        vault_token_admin.mint(&alice, &10_000_000);
        vault_token_admin.mint(&bob, &10_000_000);

        let bribe_token_addr = env.register_stellar_asset_contract(admin.clone());
        let bribe_token = token::Client::new(&env, &bribe_token_addr);
        let bribe_token_admin = token::StellarAssetClient::new(&env, &bribe_token_addr);
        bribe_token_admin.mint(&sponsor, &10_000_000);

        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &vault_token_addr, &0_u32, &None, &None);

        Fixture {
            env,
            vault,
            bribe_token,
            bribe_token_addr,
            sponsor,
            alice,
            bob,
        }
    }

    fn advance_to_ledger(&self, seq: u32) {
        self.env.ledger().with_mut(|li| {
            li.sequence_number = seq;
        });
    }

    fn single_pool_alloc(&self, sub_pool_id: u32) -> Vec<(u32, u32)> {
        let mut alloc = Vec::new(&self.env);
        alloc.push_back((sub_pool_id, 10_000_u32));
        alloc
    }
}

#[test]
fn test_proportional_split() {
    let f = Fixture::new();

    // Alice holds 3x Bob's shares (75% vs 25%).
    f.vault.deposit(&f.alice, &3_000_000, &None);
    f.vault.deposit(&f.bob, &1_000_000, &None);

    let alloc = f.single_pool_alloc(1);
    f.vault.vote_gauge(&f.alice, &alloc);
    f.vault.vote_gauge(&f.bob, &alloc);

    f.vault
        .add_incentive(&f.sponsor, &1, &f.bribe_token_addr, &1_000_000, &0);

    // Advance past epoch 0 so weights are finalized.
    f.advance_to_ledger(GAUGE_EPOCH_LEDGERS + 1);

    let alice_payout = f
        .vault
        .claim_incentive(&f.alice, &1, &f.bribe_token_addr, &0);
    let bob_payout = f
        .vault
        .claim_incentive(&f.bob, &1, &f.bribe_token_addr, &0);

    assert_eq!(alice_payout, 750_000);
    assert_eq!(bob_payout, 250_000);
    assert_eq!(f.bribe_token.balance(&f.alice), 750_000);
    assert_eq!(f.bribe_token.balance(&f.bob), 250_000);

    // Double claim must be rejected.
    let dup = f
        .vault
        .try_claim_incentive(&f.alice, &1, &f.bribe_token_addr, &0);
    assert_eq!(dup, Err(Ok(GaugeIncentiveError::AlreadyClaimed)));
}

#[test]
fn test_late_vote_does_not_retroactively_claim() {
    let f = Fixture::new();

    f.vault.deposit(&f.alice, &1_000_000, &None);
    f.vault.deposit(&f.bob, &3_000_000, &None);

    // Only Alice votes during epoch 0.
    let alloc = f.single_pool_alloc(2);
    f.vault.vote_gauge(&f.alice, &alloc);

    f.vault
        .add_incentive(&f.sponsor, &2, &f.bribe_token_addr, &800_000, &0);

    // Advance into epoch 1; Bob votes late.
    f.advance_to_ledger(GAUGE_EPOCH_LEDGERS + 10);
    f.vault.vote_gauge(&f.bob, &alloc);

    // Bob's late vote in epoch 1 must not allow claiming epoch 0's incentive.
    let bob_res = f
        .vault
        .try_claim_incentive(&f.bob, &2, &f.bribe_token_addr, &0);
    assert_eq!(bob_res, Err(Ok(GaugeIncentiveError::NoVoteWeight)));

    // Alice receives 100% of epoch 0's incentive.
    let alice_payout = f
        .vault
        .claim_incentive(&f.alice, &2, &f.bribe_token_addr, &0);
    assert_eq!(alice_payout, 800_000);
}

#[test]
fn test_sponsor_recovery_after_expiry() {
    let f = Fixture::new();

    f.vault.deposit(&f.alice, &1_000_000, &None);
    f.vault.deposit(&f.bob, &1_000_000, &None);

    let alloc = f.single_pool_alloc(3);
    f.vault.vote_gauge(&f.alice, &alloc);
    f.vault.vote_gauge(&f.bob, &alloc);

    f.vault
        .add_incentive(&f.sponsor, &3, &f.bribe_token_addr, &1_000_000, &0);

    // Advance past epoch 0; only Alice claims her 50% share.
    f.advance_to_ledger(GAUGE_EPOCH_LEDGERS + 1);
    assert_eq!(
        f.vault
            .claim_incentive(&f.alice, &3, &f.bribe_token_addr, &0),
        500_000
    );

    // Recovery before expiry must fail.
    let early = f
        .vault
        .try_recover_expired_incentive(&f.sponsor, &3, &f.bribe_token_addr, &0);
    assert_eq!(early, Err(Ok(GaugeIncentiveError::NotExpired)));

    // Advance to expiry and recover Bob's unclaimed 500_000.
    f.advance_to_ledger(incentive_expiry_ledger(0));
    let recovered = f
        .vault
        .recover_expired_incentive(&f.sponsor, &3, &f.bribe_token_addr, &0);
    assert_eq!(recovered, 500_000);
    assert_eq!(f.bribe_token.balance(&f.sponsor), 9_500_000);

    // Second recovery attempt has nothing left.
    let second = f
        .vault
        .try_recover_expired_incentive(&f.sponsor, &3, &f.bribe_token_addr, &0);
    assert_eq!(second, Err(Ok(GaugeIncentiveError::NothingToRecover)));
}
