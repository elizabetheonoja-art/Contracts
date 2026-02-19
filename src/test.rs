#![cfg(test)]

use super::*;
use soroban_sdk::{vec, Env, Address, Symbol, testutils::{Address as TestAddress}};

#[test]
fn test_basic_invariant() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);

    // Initialize contract
    let admin = TestAddress::generate(&env);
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);

    // Check invariant holds initially
    assert!(client.check_invariant(), "Invariant should hold initially");
    
    // Get initial state
    let (total_locked, total_claimed, admin_balance) = client.get_contract_state();
    assert_eq!(total_locked, 0);
    assert_eq!(total_claimed, 0);
    assert_eq!(admin_balance, initial_supply);
    
    println!("✅ Basic invariant test passed");
}

#[test]
fn test_invariant_after_vault_creation() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);

    // Initialize contract
    let admin = TestAddress::generate(&env);
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);

    // Create vaults
    let user1 = TestAddress::generate(&env);
    let user2 = TestAddress::generate(&env);
    
    client.create_vault_full(&user1, &100000i128, &1640995200u64, &1672531199u64);
    client.create_vault_full(&user2, &200000i128, &1640995200u64, &1672531199u64);

    // Check invariant holds
    assert!(client.check_invariant(), "Invariant should hold after vault creation");
    
    // Get state
    let (total_locked, total_claimed, admin_balance) = client.get_contract_state();
    assert_eq!(total_locked, 300000i128); // 100k + 200k
    assert_eq!(total_claimed, 0);
    assert_eq!(admin_balance, 700000i128); // 1M - 300k
    
    println!("✅ Invariant test after vault creation passed");
}

#[test]
fn test_invariant_after_token_claims() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);

    // Initialize contract
    let admin = TestAddress::generate(&env);
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);

    // Create vault
    let user = TestAddress::generate(&env);
    let vault_id = client.create_vault_full(&user, &100000i128, &1640995200u64, &1672531199u64);

    // Claim tokens
    client.claim_tokens(&vault_id, &50000i128);
    client.claim_tokens(&vault_id, &30000i128);

    // Check invariant holds
    assert!(client.check_invariant(), "Invariant should hold after token claims");
    
    // Get state
    let (total_locked, total_claimed, admin_balance) = client.get_contract_state();
    assert_eq!(total_locked, 20000i128); // 100k - 80k claimed
    assert_eq!(total_claimed, 80000i128); // 50k + 30k
    assert_eq!(admin_balance, 900000i128); // 1M - 100k
    
    println!("✅ Invariant test after token claims passed");
}

#[test]
fn test_property_based_invariant_100_transactions() {
    let env = Env::default();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);

    // Initialize contract
    let admin = TestAddress::generate(&env);
    let initial_supply = 1000000i128;
    client.initialize(&admin, &initial_supply);

    // Generate test users
    let mut users = Vec::new(&env);
    for _ in 0..10 {
        users.push_back(TestAddress::generate(&env));
    }

    // Run 100 random transactions
    println!("🎲 Running 100 random transactions...");
    
    for i in 0..100 {
        let transaction_type = i % 4;
        
        match transaction_type {
            0 => {
                // Create vault
                let user = users.get(i % users.len()).unwrap();
                let amount = (i as i128 % 10000 + 1000) * 10; // 10k to 100k
                client.create_vault_full(user, &amount, &1640995200u64, &1672531199u64);
                println!("📝 {}: Created vault with amount {}", i + 1, amount);
            }
            1 => {
                // Claim tokens (if vault exists)
                if i > 10 {
                    let vault_id = (i % 10) + 1;
                    let claim_amount = (i as i128 % 5000 + 100) * 10; // 1k to 50k
                    let _ = client.claim_tokens(&vault_id, &claim_amount);
                    println!("💸 {}: Claimed {} from vault {}", i + 1, claim_amount, vault_id);
                }
            }
            2 => {
                // Batch create vaults
                let batch_size = (i % 3) + 2; // 2-4 vaults
                let mut batch_recipients = Vec::new(&env);
                let mut batch_amounts = Vec::new(&env);
                let mut batch_start_times = Vec::new(&env);
                let mut batch_end_times = Vec::new(&env);
                
                for j in 0..batch_size {
                    batch_recipients.push_back(users.get((i + j) % users.len()).unwrap());
                    batch_amounts.push_back(((i + j) as i128 % 5000 + 1000) * 10);
                    batch_start_times.push_back(1640995200u64);
                    batch_end_times.push_back(1672531199u64);
                }
                
                let batch_data = BatchCreateData {
                    recipients: batch_recipients,
                    amounts: batch_amounts,
                    start_times: batch_start_times,
                    end_times: batch_end_times,
                };
                
                let _vault_ids = client.batch_create_vaults_full(&batch_data);
                println!("📦 {}: Created batch of {} vaults", i + 1, batch_size);
            }
            3 => {
                // Check invariant (this is our test)
                if client.check_invariant() {
                    println!("✅ {}: Invariant holds", i + 1);
                } else {
                    println!("❌ {}: INVARIANT VIOLATION!", i + 1);
                    panic!("Invariant violation detected!");
                }
            }
            _ => unreachable!(),
        }
    }
    
    // Final invariant check
    assert!(client.check_invariant(), "Invariant should hold after all transactions");
    
    // Get final state
    let (total_locked, total_claimed, admin_balance) = client.get_contract_state();
    let sum = total_locked + total_claimed + admin_balance;
    
    println!("\n🎯 Final State After 100 Transactions:");
    println!("  Total Locked: {}", total_locked);
    println!("  Total Claimed: {}", total_claimed);
    println!("  Admin Balance: {}", admin_balance);
    println!("  Sum: {}", sum);
    println!("  Initial Supply: {}", initial_supply);
    println!("  Invariant Holds: {}", sum == initial_supply);
    
    assert_eq!(sum, initial_supply, "Final invariant check failed");
    
    println!("✅ Property-based invariant test with 100 transactions passed");
}
