#![cfg(test)]

use super::*;
use soroban_sdk::{vec, Env, Address};

#[test]
fn test_admin_ownership_transfer() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_proposed_admin(), None);
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&unauthorized_user);
    });
    let result = std::panic::catch_unwind(|| {
        client.propose_new_admin(&new_admin);
    });
    assert!(result.is_err());
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    client.propose_new_admin(&new_admin);
    assert_eq!(client.get_proposed_admin(), Some(new_admin));
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&unauthorized_user);
    });
    let result = std::panic::catch_unwind(|| {
        client.accept_ownership();
    });
    assert!(result.is_err());
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&new_admin);
    });
    client.accept_ownership();
    assert_eq!(client.get_admin(), new_admin);
    assert_eq!(client.get_proposed_admin(), None);
}

#[test]
fn test_admin_access_control() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let vault_owner = Address::generate(&env);
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&unauthorized_user);
    });
    let result = std::panic::catch_unwind(|| {
        client.create_vault_full(&vault_owner, &1000i128, &100u64, &200u64);
    });
    assert!(result.is_err());
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    let vault_id = client.create_vault_full(&vault_owner, &1000i128, &100u64, &200u64);
    assert_eq!(vault_id, 1);
    let vault_id2 = client.create_vault_lazy(&vault_owner, &500i128, &150u64, &250u64);
    assert_eq!(vault_id2, 2);
}

#[test]
fn test_batch_operations_admin_control() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    let batch_data = BatchCreateData {
        recipients: vec![&env, recipient1.clone(), recipient2.clone()],
        amounts: vec![&env, 1000i128, 2000i128],
        start_times: vec![&env, 100u64, 150u64],
        end_times: vec![&env, 200u64, 250u64],
    };
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&unauthorized_user);
    });
    let result = std::panic::catch_unwind(|| {
        client.batch_create_vaults_lazy(&batch_data);
    });
    assert!(result.is_err());
    let result = std::panic::catch_unwind(|| {
        client.batch_create_vaults_full(&batch_data);
    });
    assert!(result.is_err());
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    let vault_ids = client.batch_create_vaults_lazy(&batch_data);
    assert_eq!(vault_ids.len(), 2);
    assert_eq!(vault_ids.get(0), Some(1u64));
    assert_eq!(vault_ids.get(1), Some(2u64));
}

// ── NEW: claim_all tests ──────────────────────────────────────────────────────

#[test]
fn test_claim_all_success() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    client.initialize(&admin, &1_000_000i128);
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    let id1 = client.create_vault_full(&owner, &1000i128, &0u64, &1000u64);
    let id2 = client.create_vault_full(&owner, &2000i128, &0u64, &1000u64);
    let id3 = client.create_vault_full(&owner, &3000i128, &0u64, &1000u64);
    let vault_ids = vec![&env, id1, id2, id3];
    let amounts = vec![&env, 100i128, 200i128, 300i128];
    let results = client.claim_all(&vault_ids, &amounts);
    assert_eq!(results.len(), 3);
    assert_eq!(results.get(0), Some(100i128));
    assert_eq!(results.get(1), Some(200i128));
    assert_eq!(results.get(2), Some(300i128));
    let v1 = client.get_vault(&id1);
    assert_eq!(v1.released_amount, 100);
    let v2 = client.get_vault(&id2);
    assert_eq!(v2.released_amount, 200);
    let v3 = client.get_vault(&id3);
    assert_eq!(v3.released_amount, 300);
}

#[test]
fn test_claim_all_atomic_rollback_invalid_vault() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    client.initialize(&admin, &1_000_000i128);
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    let id1 = client.create_vault_full(&owner, &1000i128, &0u64, &1000u64);
    let vault_ids = vec![&env, id1, 999u64];
    let amounts = vec![&env, 100i128, 100i128];
    let result = std::panic::catch_unwind(|| {
        client.claim_all(&vault_ids, &amounts);
    });
    assert!(result.is_err());
    let v1 = client.get_vault(&id1);
    assert_eq!(v1.released_amount, 0);
}

#[test]
fn test_claim_all_atomic_rollback_insufficient_tokens() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    client.initialize(&admin, &1_000_000i128);
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    let id1 = client.create_vault_full(&owner, &1000i128, &0u64, &1000u64);
    let id2 = client.create_vault_full(&owner, &500i128, &0u64, &1000u64);
    let vault_ids = vec![&env, id1, id2];
    let amounts = vec![&env, 100i128, 9999i128];
    let result = std::panic::catch_unwind(|| {
        client.claim_all(&vault_ids, &amounts);
    });
    assert!(result.is_err());
    let v1 = client.get_vault(&id1);
    assert_eq!(v1.released_amount, 0);
    let v2 = client.get_vault(&id2);
    assert_eq!(v2.released_amount, 0);
}

#[test]
fn test_claim_all_empty_list_fails() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &1_000_000i128);
    let empty_ids: Vec<u64> = vec![&env];
    let empty_amounts: Vec<i128> = vec![&env];
    let result = std::panic::catch_unwind(|| {
        client.claim_all(&empty_ids, &empty_amounts);
    });
    assert!(result.is_err());
}

#[test]
fn test_claim_all_mismatched_lengths_fails() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    client.initialize(&admin, &1_000_000i128);
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    let id1 = client.create_vault_full(&owner, &1000i128, &0u64, &1000u64);
    let vault_ids = vec![&env, id1];
    let amounts = vec![&env, 100i128, 200i128];
    let result = std::panic::catch_unwind(|| {
        client.claim_all(&vault_ids, &amounts);
    });
    assert!(result.is_err());
}