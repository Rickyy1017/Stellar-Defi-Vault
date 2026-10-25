#![cfg(test)]
extern crate std;

use soroban_sdk::{testutils::Address as _, token, Address, Bytes, Env, String};

use crate::{
    errors::{VaultCampaignError, VaultError, VaultExtError, VaultFeatureError, VaultQuizError},
    vault::{VaultContract, VaultContractClient},
};

struct Fixture<'a> {
    env: Env,
    vault: VaultContractClient<'a>,
    admin: Address,
    user: Address,
}

impl<'a> Fixture<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let token_address = env.register_stellar_asset_contract(admin.clone());
        let token_admin = token::StellarAssetClient::new(&env, &token_address);
        token_admin.mint(&user, &10_000);
        let vault_id = env.register_contract(None, VaultContract);
        let vault = VaultContractClient::new(&env, &vault_id);
        vault.initialize(&admin, &token_address, &0_u32, &None, &None);
        Fixture { env, vault, admin, user }
    }

    fn bytes(&self, bytes: &[u8]) -> Bytes {
        Bytes::from_slice(&self.env, bytes)
    }

    fn text(&self, text: &str) -> String {
        String::from_str(&self.env, text)
    }
}

#[test]
fn sha256_inputs_reject_wrong_lengths() {
    let f = Fixture::new();
    let bad = f.bytes(b"short");
    assert_eq!(f.vault.try_set_pool_terms(&bad, &1), Err(Ok(VaultError::InvalidHashLength)));
    assert_eq!(f.vault.try_pay_bug_bounty(&f.admin, &f.user, &1, &bad), Err(Ok(VaultError::InvalidHashLength)));
    assert_eq!(f.vault.try_add_roadmap_item(&f.admin, &f.text("title"), &bad, &f.text("cat")), Err(Ok(VaultCampaignError::InvalidHashLength)));
    assert_eq!(f.vault.try_add_quiz(&f.admin, &bad, &bad, &0, &1), Err(Ok(VaultQuizError::InvalidHashLength)));
    assert_eq!(f.vault.try_publish_merkle_root(&f.admin, &bad, &1, &1), Err(Ok(VaultExtError::MerkleInvalidProof)));
    assert_eq!(f.vault.try_distribute_revenue(&f.admin, &bad), Err(Ok(VaultFeatureError::RevenueShareInvalidProof)));

    f.vault.stake(&f.user, &1_000, &0);
    assert_eq!(f.vault.try_submit_news_item(&f.user, &f.text("title"), &bad), Err(Ok(VaultError::InvalidHashLength)));
}

#[test]
fn bounded_public_strings_reject_values_over_the_named_limit() {
    let f = Fixture::new();
    let too_long_hash_id = f.text(&"h".repeat(65));
    assert_eq!(f.vault.try_submit_content(&f.user, &too_long_hash_id), Err(Ok(VaultError::DescriptionTooLong)));

    let too_long_url = f.text(&"u".repeat(201));
    assert_eq!(f.vault.try_set_webhook_url(&f.admin, &too_long_url), Err(Ok(VaultError::DescriptionTooLong)));

    let too_long_name = f.text(&"n".repeat(65));
    assert_eq!(f.vault.try_add_season(&f.admin, &too_long_name, &1, &2, &1), Err(Ok(VaultFeatureError::InvalidSeasonConfig)));
    assert_eq!(f.vault.try_announce_config_change(&f.admin, &too_long_name, &1, &1), Err(Ok(VaultQuizError::InputTooLong)));
}