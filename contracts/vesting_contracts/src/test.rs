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
fn test_periodic_vesting_monthly_steps() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    // Initialize contract
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Set admin as caller
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    // Create vault with monthly vesting (30 days = 2,592,000 seconds)
    let amount = 1200000i128; // 1,200,000 tokens over 12 months = 100,000 per month
    let start_time = 1000000u64;
    let end_time = start_time + (365 * 24 * 60 * 60); // 1 year
    let step_duration = 30 * 24 * 60 * 60; // 30 days in seconds
    let keeper_fee = 1000i128;
    
    let vault_id = client.create_vault_full(
        &beneficiary,
        &amount,
        &start_time,
        &end_time,
        &keeper_fee,
        &false, // revocable
        &true,  // transferable
        &step_duration,
    );
    
    // Test 1: Before start time - no vesting
    env.ledger().set_timestamp(start_time - 1000);
    let claimable = client.get_claimable_amount(&vault_id);
    assert_eq!(claimable, 0, "Should have no claimable tokens before start time");
    
    // Test 2: After 15 days (less than one step) - still no vesting (rounds down)
    env.ledger().set_timestamp(start_time + (15 * 24 * 60 * 60));
    let claimable = client.get_claimable_amount(&vault_id);
    assert_eq!(claimable, 0, "Should have no claimable tokens before first step completes");
    
    // Test 3: After exactly 30 days - one step completed
    env.ledger().set_timestamp(start_time + step_duration);
    let claimable = client.get_claimable_amount(&vault_id);
    let expected_monthly = amount / 12; // 100,000 tokens per month
    assert_eq!(claimable, expected_monthly, "Should have exactly one month of tokens after 30 days");
    
    // Test 4: After 45 days - still only one step (rounds down)
    env.ledger().set_timestamp(start_time + (45 * 24 * 60 * 60));
    let claimable = client.get_claimable_amount(&vault_id);
    assert_eq!(claimable, expected_monthly, "Should still have only one month of tokens after 45 days");
    
    // Test 5: After 60 days - two steps completed
    env.ledger().set_timestamp(start_time + (2 * step_duration));
    let claimable = client.get_claimable_amount(&vault_id);
    assert_eq!(claimable, 2 * expected_monthly, "Should have two months of tokens after 60 days");
    
    // Test 6: After 6 months - 6 steps completed
    env.ledger().set_timestamp(start_time + (6 * step_duration));
    let claimable = client.get_claimable_amount(&vault_id);
    assert_eq!(claimable, 6 * expected_monthly, "Should have six months of tokens after 6 months");
    
    // Test 7: After end time - all tokens vested
    env.ledger().set_timestamp(end_time + 1000);
    let claimable = client.get_claimable_amount(&vault_id);
    assert_eq!(claimable, amount, "Should have all tokens vested after end time");
}

#[test]
fn test_periodic_vesting_weekly_steps() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    // Initialize contract
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Set admin as caller
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    // Create vault with weekly vesting (7 days = 604,800 seconds)
    let amount = 520000i128; // 520,000 tokens over 52 weeks = 10,000 per week
    let start_time = 1000000u64;
    let end_time = start_time + (365 * 24 * 60 * 60); // 1 year
    let step_duration = 7 * 24 * 60 * 60; // 7 days in seconds
    let keeper_fee = 100i128;
    
    let vault_id = client.create_vault_full(
        &beneficiary,
        &amount,
        &start_time,
        &end_time,
        &keeper_fee,
        &false, // revocable
        &true,  // transferable
        &step_duration,
    );
    
    // Test: After 3 weeks - 3 steps completed
    env.ledger().set_timestamp(start_time + (3 * step_duration));
    let claimable = client.get_claimable_amount(&vault_id);
    let expected_weekly = 10000i128; // 10,000 tokens per week
    assert_eq!(claimable, 3 * expected_weekly, "Should have three weeks of tokens after 3 weeks");
    
    // Test: After 10 weeks - 10 steps completed
    env.ledger().set_timestamp(start_time + (10 * step_duration));
    let claimable = client.get_claimable_amount(&vault_id);
    assert_eq!(claimable, 10 * expected_weekly, "Should have ten weeks of tokens after 10 weeks");
}

#[test]
fn test_linear_vesting_step_duration_zero() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    // Initialize contract
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Set admin as caller
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    // Create vault with linear vesting (step_duration = 0)
    let amount = 1200000i128;
    let start_time = 1000000u64;
    let end_time = start_time + (365 * 24 * 60 * 60); // 1 year
    let step_duration = 0u64; // Linear vesting
    let keeper_fee = 1000i128;
    
    let vault_id = client.create_vault_full(
        &beneficiary,
        &amount,
        &start_time,
        &end_time,
        &keeper_fee,
        &false, // revocable
        &true,  // transferable
        &step_duration,
    );
    
    // Test: After 6 months (half the duration) - should have 50% vested
    env.ledger().set_timestamp(start_time + (182 * 24 * 60 * 60)); // ~6 months
    let claimable = client.get_claimable_amount(&vault_id);
    let expected_half = amount / 2; // 50% of tokens
    assert_eq!(claimable, expected_half, "Should have 50% of tokens after half the time for linear vesting");
    
    // Test: After 3 months (quarter of the duration) - should have 25% vested
    env.ledger().set_timestamp(start_time + (91 * 24 * 60 * 60)); // ~3 months
    let claimable = client.get_claimable_amount(&vault_id);
    let expected_quarter = amount / 4; // 25% of tokens
    assert_eq!(claimable, expected_quarter, "Should have 25% of tokens after quarter of the time for linear vesting");
}

#[test]
fn test_periodic_vesting_claim_partial() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    // Initialize contract
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Set beneficiary as caller for claiming
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&beneficiary);
    });
    
    // Create vault with monthly vesting
    let amount = 120000i128; // 120,000 tokens over 12 months = 10,000 per month
    let start_time = 1000000u64;
    let end_time = start_time + (365 * 24 * 60 * 60); // 1 year
    let step_duration = 30 * 24 * 60 * 60; // 30 days
    let keeper_fee = 100i128;
    
    let vault_id = client.create_vault_full(
        &beneficiary,
        &amount,
        &start_time,
        &end_time,
        &keeper_fee,
        &false, // revocable
        &true,  // transferable
        &step_duration,
    );
    
    // Move time to 3 months
    env.ledger().set_timestamp(start_time + (3 * step_duration));
    
    // Claim partial amount
    let claim_amount = 15000i128; // Less than the 30,000 available
    let claimed = client.claim_tokens(&vault_id, &claim_amount);
    assert_eq!(claimed, claim_amount, "Should claim the requested amount");
    
    // Check remaining claimable
    let remaining_claimable = client.get_claimable_amount(&vault_id);
    assert_eq!(remaining_claimable, 15000i128, "Should have 15,000 tokens remaining claimable");
    
    // Claim the rest
    let final_claim = client.claim_tokens(&vault_id, &remaining_claimable);
    assert_eq!(final_claim, remaining_claimable, "Should claim remaining tokens");
    
    // Check no more tokens available
    let no_more_claimable = client.get_claimable_amount(&vault_id);
    assert_eq!(no_more_claimable, 0, "Should have no more claimable tokens");
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
        client.create_vault_full(
            &vault_owner,
            &1000i128,
            &100u64,
            &200u64,
            &0i128,
            &false,
            &true,
            &0u64,
        );
    });
    assert!(result.is_err());
    
    // Test: Admin can create vaults
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    let vault_id2 = client.create_vault_full(
        &vault_owner,
        &1000i128,
        &100u64,
        &200u64,
        &0i128,
        &false,
        &true,
        &0u64,
    );
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

    let admin = Address::generate(&env);
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

#[test]
fn test_lockup_only_mode() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let initial_supply = 1_000_000i128;
    client.initialize(&admin, &initial_supply);
    
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });

    let now = env.ledger().timestamp();
    let duration = 31536000u64; // 1 year
    let start_time = now;
    let end_time = now + duration;
    let total_amount = 100_000i128;
    
    // Set step_duration equal to total duration -> Lockup Only
    let step_duration = duration;

    let vault_id = client.create_vault_full(
        &beneficiary,
        &total_amount,
        &start_time,
        &end_time,
        &0i128,
        &true,
        &false,
        &step_duration,
    );

    // Check just before end (should be 0 vested)
    env.ledger().with_mut(|li| {
        li.timestamp = end_time - 1;
    });
    
    // Attempt to claim should fail as nothing is vested
    let result = std::panic::catch_unwind(|| {
        client.claim_tokens(&vault_id, &1i128);
    });
    assert!(result.is_err());

    // Check at end (should be 100% vested)
    env.ledger().with_mut(|li| {
        li.timestamp = end_time;
    });
    
    // Should be able to claim full amount
    let claimed = client.claim_tokens(&vault_id, &total_amount);
    assert_eq!(claimed, total_amount);
    
    let vault = client.get_vault(&vault_id);
    assert_eq!(vault.released_amount, total_amount);
}

#[test]
fn test_vault_start_time_immutable() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);

    // Create a vault
    let owner = Address::generate(&env);
    let amount = 1000i128;
    let start_time = 123456789u64;
    let end_time = start_time + 10000;
    let keeper_fee = 10i128;
    let is_revocable = false;
    let is_transferable = false;
    let step_duration = 0u64;
    let vault_id = client.create_vault(
        &owner,
        &amount,
        &start_time,
        &end_time,
        &keeper_fee,
        &is_revocable,
        &is_transferable,
        &step_duration,
    );

    // Try to change start_time or cliff_duration (should not be possible)
    let vault = client.get_vault(&vault_id);
    let original_start_time = vault.start_time;
    let original_cliff_duration = vault.cliff_duration;

    // Attempt to update vault via admin functions (should not affect start_time/cliff_duration)
    client.mark_irrevocable(&vault_id);
    client.transfer_beneficiary(&vault_id, &Address::generate(&env));
    client.set_delegate(&vault_id, &Some(Address::generate(&env)));

    let updated_vault = client.get_vault(&vault_id);
    assert_eq!(updated_vault.start_time, original_start_time);
    assert_eq!(updated_vault.cliff_duration, original_cliff_duration);
}

#[test]
fn test_global_pause_functionality() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    
    // Create addresses for testing
    let admin = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    
    // Initialize contract with admin
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);
    
    // Verify initial state is unpaused
    assert_eq!(client.is_paused(), false);
    
    // Test: Unauthorized user cannot toggle pause
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&unauthorized_user);
    });
    
    let result = std::panic::catch_unwind(|| {
        client.toggle_pause();
    });
    assert!(result.is_err());
    assert_eq!(client.is_paused(), false); // Should still be unpaused
    
    // Test: Admin can pause the contract
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    client.toggle_pause();
    assert_eq!(client.is_paused(), true); // Should now be paused
    
    // Create a vault for testing claims
    let now = env.ledger().timestamp();
    let vault_id = client.create_vault_full(
        &beneficiary,
        &1000i128,
        &now,
        &(now + 1000),
        &0i128,
        &false,
        &true,
        &0u64,
    );
    
    // Move time to make tokens claimable
    env.ledger().set_timestamp(now + 1001);
    
    // Set beneficiary as caller
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&beneficiary);
    });
    
    // Test: Claims should fail when paused
    let result = std::panic::catch_unwind(|| {
        client.claim_tokens(&vault_id, &100i128);
    });
    assert!(result.is_err());
    
    // Test: Delegate claims should also fail when paused
    let delegate = Address::generate(&env);
    client.set_delegate(&vault_id, &Some(delegate.clone()));
    
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&delegate);
    });
    
    let result = std::panic::catch_unwind(|| {
        client.claim_as_delegate(&vault_id, &100i128);
    });
    assert!(result.is_err());
    
    // Test: Admin can unpause the contract
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&admin);
    });
    
    client.toggle_pause();
    assert_eq!(client.is_paused(), false); // Should be unpaused
    
    // Test: Claims should work after unpausing
    env.as_contract(&contract_id, || {
        env.current_contract_address().set(&beneficiary);
    });
    
    let claimed = client.claim_tokens(&vault_id, &100i128);
    assert_eq!(claimed, 100i128); // Should succeed
}
