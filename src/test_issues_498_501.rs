#![cfg(test)]
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, String, Vec};
use crate::{VaultContract, VaultContractClient};
use crate::vault_extensions_498_501::{TreasurySplit, PositionSnapshot};

#[test]
fn test_issues_498_501() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&env, &contract_id);
    
    let user1 = Address::generate(&env);
    
    // 498
    let batch = client.batch_withdraw(&user1);
    assert_eq!(batch.len(), 0);
    
    // 499
    let mut recipients = Vec::new(&env);
    recipients.push_back(Address::generate(&env));
    let mut bps = Vec::new(&env);
    bps.push_back(10000);
    client.set_treasury_split(&admin, &recipients, &bps);
    
    // 500
    let snap = client.position_export(&user1);
    assert_eq!(snap.shares, 0);
    
    // 501
    client.set_reward_token_swap_path(&admin, &Address::generate(&env), &Address::generate(&env));
}
