#![cfg(test)]

use super::*;
use soroban_sdk::{vec, Env, Address, contract, contractimpl};

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

    });
    assert!(result.is_err());
    
    let result = std::panic::catch_unwind(|| {

    });
    assert!(result.is_err());
    
    // Test: Admin can create vaults
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    

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
        keeper_fees: vec![&env, 0i128, 0i128],
        step_durations: vec![&env, 0u64, 0u64],
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
fn test_milestone_unlocking_and_claim_limits() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);

    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);

    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });

}

#[test]
fn test_step_vesting_fuzz() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let initial_supply = 1_000_000_000_000i128;
    client.initialize(&admin, &initial_supply);
    
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });

    // Fuzz testing with prime numbers to check for truncation errors
    // Primes: 1009 (amount), 17 (step), 101 (duration)
    let total_amount = 1009i128;
    let start_time = 1000u64;
    let duration = 101u64; // Prime duration
    let end_time = start_time + duration;
    let step_duration = 17u64; // Prime step
    
    let vault_id = client.create_vault_full(
        &beneficiary,
        &total_amount,
        &start_time,
        &end_time,
        &0i128,
        &true,
        &true,
        &step_duration,
    );

    // Advance time to end
    env.ledger().with_mut(|li| {
        li.timestamp = end_time + 1;
    });

    // Claim all
    let claimed = client.claim_tokens(&vault_id, &total_amount);
    
    // Assert full amount is claimed
    assert_eq!(claimed, total_amount);
    
    // Verify vault state
    let vault = client.get_vault(&vault_id);
    assert_eq!(vault.released_amount, total_amount);
}

// Mock Staking Contract for testing cross-contract calls
#[contract]
pub struct MockStakingContract;

#[contractimpl]
impl MockStakingContract {
    pub fn stake(env: Env, vault_id: u64, amount: i128, _validator: Address) {
        env.events().publish((Symbol::new(&env, "stake"), vault_id), amount);
    }
    pub fn unstake(env: Env, vault_id: u64, amount: i128) {
        env.events().publish((Symbol::new(&env, "unstake"), vault_id), amount);
    }
}

#[test]
fn test_staking_integration() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Register mock staking contract
    let staking_contract_id = env.register(MockStakingContract, ());
    let staking_client = MockStakingContractClient::new(&env, &staking_contract_id);

    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let validator = Address::generate(&env);

    let initial_supply = 1_000_000i128;
    client.initialize(&admin, &initial_supply);

    // Set staking contract
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    client.set_staking_contract(&staking_contract_id);

    // Create vault
    let total_amount = 1000i128;
    let now = env.ledger().timestamp();
    let vault_id = client.create_vault_full(
        &beneficiary, &total_amount, &now, &(now + 1000), &0i128, &true, &true, &0u64
    );

    // Stake tokens as beneficiary
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&beneficiary);
    });
    
    let stake_amount = 500i128;
    client.stake_tokens(&vault_id, &stake_amount, &validator);

    // Verify vault state
    let vault = client.get_vault(&vault_id);
    assert_eq!(vault.staked_amount, stake_amount);

    // Fast forward to end of vesting
    env.ledger().with_mut(|li| {
        li.timestamp = now + 1001;
    });

    // Claim ALL tokens (should trigger auto-unstake)
    client.claim_tokens(&vault_id, &total_amount);

    let vault_final = client.get_vault(&vault_id);
    assert_eq!(vault_final.staked_amount, 0);
    assert_eq!(vault_final.released_amount, total_amount);
}

#[test]
fn test_rotate_beneficiary_key() {
    let env = Env::default();
    env.mock_all_auths(); // Enable auth mocking for require_auth

    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let new_beneficiary = Address::generate(&env);
    
    let initial_supply = 1_000_000i128;
    client.initialize(&admin, &initial_supply);
    
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });

    // Create vault (non-transferable to test rotation bypass)
    let now = env.ledger().timestamp();
    let vault_id = client.create_vault_full(
        &beneficiary,
        &1000i128,
        &now,
        &(now + 1000),
        &0i128,
        &true, // revocable
        &false, // NOT transferable
        &0u64, // step
    );

    // Rotate key
    client.rotate_beneficiary_key(&vault_id, &new_beneficiary);

    // Verify new owner
    let vault_updated = client.get_vault(&vault_id);
    assert_eq!(vault_updated.owner, new_beneficiary);

    // Verify UserVaults
    let new_vaults = client.get_user_vaults(&new_beneficiary);
    assert_eq!(new_vaults.get(0).unwrap(), vault_id);
}
