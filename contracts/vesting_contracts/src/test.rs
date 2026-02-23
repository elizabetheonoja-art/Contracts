#![cfg(test)]

use super::*;
use soroban_sdk::{vec, Env, Address, String};

#[test]
fn test_admin_ownership_transfer() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    
    // Initialize contract with admin
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Verify initial admin
    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_proposed_admin(), None);
    
    // Test: Unauthorized user cannot propose new admin
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&unauthorized_user);
    });
    
    // This should fail - unauthorized user cannot propose admin
    let result = std::panic::catch_unwind(|| {
        client.propose_new_admin(&new_admin);
    });
    assert!(result.is_err());
    
    // Reset to admin context
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    // Test: Admin can propose new admin
    client.propose_new_admin(&new_admin);
    assert_eq!(client.get_proposed_admin(), Some(new_admin));
    
    // Test: Unauthorized user cannot accept ownership
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&unauthorized_user);
    });
    
    let result = std::panic::catch_unwind(|| {
        client.accept_ownership();
    });
    assert!(result.is_err());
    
    // Test: Proposed admin can accept ownership
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&new_admin);
    });
    
    client.accept_ownership();
    
    // Verify admin transfer completed
    assert_eq!(client.get_admin(), new_admin);
    assert_eq!(client.get_proposed_admin(), None);
    
    // Test: Old admin cannot propose new admin anymore
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    let another_admin = Address::generate(&env);
    let result = std::panic::catch_unwind(|| {
        client.propose_new_admin(&another_admin);
    });
    assert!(result.is_err());
    
    // Test: New admin can propose admin changes
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&new_admin);
    });
    
    client.propose_new_admin(&another_admin);
    assert_eq!(client.get_proposed_admin(), Some(another_admin));
}

#[test]
fn test_admin_access_control() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let vault_owner = Address::generate(&env);
    
    // Initialize contract with admin
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Test: Unauthorized user cannot create vaults
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&unauthorized_user);
    });
    
    let result = std::panic::catch_unwind(|| {
        client.create_vault_full(&vault_owner, &1000i128, &100u64, &200u64);
    });
    assert!(result.is_err());
    
    let result = std::panic::catch_unwind(|| {
        client.create_vault_lazy(&vault_owner, &1000i128, &100u64, &200u64);
    });
    assert!(result.is_err());
    
    // Test: Admin can create vaults
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
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    
    // Initialize contract with admin
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Create batch data
    let batch_data = BatchCreateData {
        recipients: vec![&env, recipient1.clone(), recipient2.clone()],
        amounts: vec![&env, 1000i128, 2000i128],
        start_times: vec![&env, 100u64, 150u64],
        end_times: vec![&env, 200u64, 250u64],
    };
    
    // Test: Unauthorized user cannot create batch vaults
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
    
    // Test: Admin can create batch vaults
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    let vault_ids = client.batch_create_vaults_lazy(&batch_data);
    assert_eq!(vault_ids.len(), 2);
    assert_eq!(vault_ids.get(0), 1);
    assert_eq!(vault_ids.get(1), 2);
}

#[test]
fn test_delegate_functionality() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let vault_owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    
    // Initialize contract with admin
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Create a vault as admin
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    let vault_id = client.create_vault_full(&vault_owner, &1000i128, &100u64, &200u64);
    
    // Test: Initial vault has no delegate
    let vault = client.get_vault(&vault_id);
    assert_eq!(vault.owner, vault_owner);
    assert_eq!(vault.delegate, None);
    
    // Test: Unauthorized user cannot set delegate
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&unauthorized_user);
    });
    
    let result = std::panic::catch_unwind(|| {
        client.set_delegate(&vault_id, &Some(delegate.clone()));
    });
    assert!(result.is_err());
    
    // Test: Vault owner can set delegate
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&vault_owner);
    });
    
    client.set_delegate(&vault_id, &Some(delegate.clone()));
    
    // Verify delegate is set
    let updated_vault = client.get_vault(&vault_id);
    assert_eq!(updated_vault.owner, vault_owner);
    assert_eq!(updated_vault.delegate, Some(delegate.clone()));
    
    // Test: Delegate can claim tokens
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&delegate);
    });
    
    let claimed_amount = client.claim_as_delegate(&vault_id, &500i128);
    assert_eq!(claimed_amount, 500i128);
    
    // Verify vault state after claim
    let final_vault = client.get_vault(&vault_id);
    assert_eq!(final_vault.released_amount, 500i128);
    
    // Test: Unauthorized user cannot claim as delegate
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&unauthorized_user);
    });
    
    let result = std::panic::catch_unwind(|| {
        client.claim_as_delegate(&vault_id, &100i128);
    });
    assert!(result.is_err());
    
    // Test: Owner can remove delegate
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&vault_owner);
    });
    
    client.set_delegate(&vault_id, &None);
    
    // Verify delegate is removed
    let vault_no_delegate = client.get_vault(&vault_id);
    assert_eq!(vault_no_delegate.delegate, None);
    
    // Test: Cannot claim as delegate after removal
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&delegate);
    });
    
    let result = std::panic::catch_unwind(|| {
        client.claim_as_delegate(&vault_id, &100i128);
    });
    assert!(result.is_err());
}

#[test]
fn test_delegate_claim_limits() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let vault_owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    
    // Initialize contract with admin
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Create a vault as admin
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    let vault_id = client.create_vault_full(&vault_owner, &1000i128, &100u64, &200u64);
    
    // Set delegate
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&vault_owner);
    });
    client.set_delegate(&vault_id, &Some(delegate.clone()));
    
    // Test: Delegate cannot claim more than available
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&delegate);
    });
    
    let result = std::panic::catch_unwind(|| {
        client.claim_as_delegate(&vault_id, &1500i128); // More than total amount
    });
    assert!(result.is_err());
    
    // Test: Delegate can claim exact amount
    let claimed_amount = client.claim_as_delegate(&vault_id, &1000i128);
    assert_eq!(claimed_amount, 1000i128);
    
    // Test: Cannot claim after all tokens are claimed
    let result = std::panic::catch_unwind(|| {
        client.claim_as_delegate(&vault_id, &1i128);
    });
    assert!(result.is_err());
}

#[test]
fn test_delegate_with_uninitialized_vault() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let vault_owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    
    // Initialize contract with admin
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Create a lazy vault as admin
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    let vault_id = client.create_vault_lazy(&vault_owner, &1000i128, &100u64, &200u64);
    
    // Test: Cannot set delegate on uninitialized vault
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&vault_owner);
    });
    
    let result = std::panic::catch_unwind(|| {
        client.set_delegate(&vault_id, &Some(delegate.clone()));
    });
    assert!(result.is_err());
    
    // Test: Cannot claim as delegate on uninitialized vault
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&delegate);
    });
    
    let result = std::panic::catch_unwind(|| {
        client.claim_as_delegate(&vault_id, &100i128);
    });
    assert!(result.is_err());
    
    // Initialize the vault
    client.initialize_vault_metadata(&vault_id);
    
    // Now delegate operations should work
    client.set_delegate(&vault_id, &Some(delegate.clone()));
    
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&delegate);
    });
    
    let claimed_amount = client.claim_as_delegate(&vault_id, &500i128);
    assert_eq!(claimed_amount, 500i128);
}
