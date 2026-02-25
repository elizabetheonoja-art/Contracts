#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, U256};

#[test]
fn test_basic_grant() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(GrantContract, ());
    let client = GrantContractClient::new(&env, &contract_id);

    let recipient = Address::generate(&env);
    let total_amount = U256::from_u32(&env, 1000);
    let duration = 100u64;

    client.initialize_grant(&recipient, &total_amount, &duration);
    
    let claimable = client.claimable_balance();
    assert_eq!(claimable, U256::from_u32(&env, 0));
}
