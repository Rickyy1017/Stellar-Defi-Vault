#![cfg(test)]
//! Contract accounts use Soroban authorization through `__check_auth`, not
//! an ed25519 keypair. The mock auth invocation below is scoped to the exact
//! vault contract call; the account receives that call in its auth context.

extern crate std;

use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token, vec, Address, Env, IntoVal, Val, Vec,
};

use crate::vault::{VaultContract, VaultContractClient};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
enum MockAccountError {
    Rejected = 1,
}

#[contracttype]
enum MockAccountKey {
    RejectAuth,
}

#[contract]
struct MockCustomAccount;

#[contractimpl]
impl MockCustomAccount {
    pub fn set_reject_auth(env: Env, reject: bool) {
        env.storage()
            .instance()
            .set(&MockAccountKey::RejectAuth, &reject);
    }
}

#[contractimpl]
impl CustomAccountInterface for MockCustomAccount {
    type Signature = ();
    type Error = MockAccountError;

    fn __check_auth(
        env: Env,
        _signature_payload: Hash<32>,
        _signature: (),
        _auth_contexts: Vec<Context>,
    ) -> Result<(), MockAccountError> {
        if env
            .storage()
            .instance()
            .get(&MockAccountKey::RejectAuth)
            .unwrap_or(false)
        {
            Err(MockAccountError::Rejected)
        } else {
            Ok(())
        }
    }
}

fn fixture() -> (Env, VaultContractClient<'static>, Address) {
    let env = Env::default();
    let initial_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(initial_admin.clone());
    let vault_id = env.register_contract(None, VaultContract);
    let vault = VaultContractClient::new(&env, &vault_id);

    env.mock_all_auths();
    vault.initialize(&initial_admin, &token, &0, &None, &None);

    let account_id = env.register_contract(None, MockCustomAccount);
    env.as_contract(&vault_id, || crate::admin::set_admin(&env, &account_id));
    (env, vault, account_id)
}

fn mock_admin_auth(
    env: &Env,
    vault_id: &Address,
    account_id: &Address,
    fn_name: &str,
    args: Vec<Val>,
) {
    let invocation = MockAuthInvoke {
        contract: vault_id,
        fn_name,
        args,
        sub_invokes: &[],
    };
    env.mock_auths(&[MockAuth {
        address: account_id,
        invoke: &invocation,
    }]);
}

#[test]
fn contract_account_can_authorize_admin_entrypoint() {
    let (env, vault, account_id) = fixture();
    mock_admin_auth(
        &env,
        &vault.address,
        &account_id,
        "set_reward_rate_bps",
        vec![&env, 500_u32.into_val(&env)],
    );

    vault.set_reward_rate_bps(&500);

    assert_eq!(vault.get_reward_rate_bps(), 500);
}

#[test]
fn contract_account_authorizes_admin_entrypoints_with_shared_and_explicit_gates() {
    let (env, vault, account_id) = fixture();

    mock_admin_auth(
        &env,
        &vault.address,
        &account_id,
        "set_transfer_cooldown",
        vec![&env, 150_u32.into_val(&env)],
    );
    vault.set_transfer_cooldown(&150);
    assert_eq!(vault.get_transfer_cooldown(), 150);

    mock_admin_auth(
        &env,
        &vault.address,
        &account_id,
        "set_withdrawal_cooldown",
        vec![&env, account_id.clone().into_val(&env), 75_u32.into_val(&env)],
    );
    vault.set_withdrawal_cooldown(&account_id, &75);
    assert_eq!(vault.get_withdrawal_cooldown(), 75);

    mock_admin_auth(
        &env,
        &vault.address,
        &account_id,
        "set_claim_vesting_duration",
        vec![&env, account_id.clone().into_val(&env), 90_u32.into_val(&env)],
    );
    vault.set_claim_vesting_duration(&account_id, &90);
    assert_eq!(vault.get_claim_vesting_duration(), 90);
}

#[test]
fn rejecting_contract_account_blocks_admin_entrypoint() {
    let (env, vault, account_id) = fixture();
    let account = MockCustomAccountClient::new(&env, &account_id);
    account.set_reject_auth(&true);

    mock_admin_auth(
        &env,
        &vault.address,
        &account_id,
        "set_reward_rate_bps",
        vec![&env, 500_u32.into_val(&env)],
    );
    assert!(vault.try_set_reward_rate_bps(&500).is_err());
    assert_eq!(vault.get_reward_rate_bps(), 0);
}
