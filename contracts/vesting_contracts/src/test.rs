#[cfg(test)]
mod tests {
    use crate::{
        BatchCreateData, Milestone, VestingContract, VestingContractClient,
    };
    use soroban_sdk::{
        contract, contractimpl,
        testutils::{Address as _, Ledger},
        token, vec, Address, Env, Symbol, String,
    };

    // -------------------------------------------------------------------------
    // Helper: spin up a fresh contract
    // -------------------------------------------------------------------------

    fn setup() -> (Env, Address, VestingContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VestingContract, ());
        let client = VestingContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &1_000_000i128);
        (env, contract_id, client, admin)
    }

    fn register_token(env: &Env, admin: &Address) -> Address {
        env.register_stellar_asset_contract_v2(admin.clone())
            .address()
    }

    fn mint_to(env: &Env, token_addr: &Address, recipient: &Address, amount: i128) {
        token::StellarAssetClient::new(env, token_addr).mint(recipient, &amount);
    }

    // -------------------------------------------------------------------------
    // Admin ownership transfer
    // -------------------------------------------------------------------------

    #[test]
    fn test_admin_ownership_transfer() {
        let (env, _cid, client, admin) = setup();
        let new_admin = Address::generate(&env);

        assert_eq!(client.get_admin(), admin);
        assert_eq!(client.get_proposed_admin(), None);

        client.propose_new_admin(&new_admin);
        assert_eq!(client.get_proposed_admin(), Some(new_admin.clone()));

        client.accept_ownership();
        assert_eq!(client.get_admin(), new_admin);
        assert_eq!(client.get_proposed_admin(), None);
    }

    // -------------------------------------------------------------------------
    // Vault creation
    // -------------------------------------------------------------------------

    #[test]
    fn test_create_vault_full_increments_count() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let id1 = client.create_vault_full(
            &beneficiary, &1_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );
        let id2 = client.create_vault_full(
            &beneficiary, &500i128, &(now + 10), &(now + 2_000),
            &0i128, &true, &false, &0u64,
        );
        assert_eq!(id1, 1u64);
        assert_eq!(id2, 2u64);
    }

    #[test]
    fn test_create_vault_lazy_increments_count() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let id = client.create_vault_lazy(
            &beneficiary, &1_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );
        assert_eq!(id, 1u64);
    }

    // -------------------------------------------------------------------------
    // Batch vault creation
    // -------------------------------------------------------------------------

    #[test]
    fn test_batch_create_vaults_lazy() {
        let (env, _cid, client, _admin) = setup();
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);

        let batch = BatchCreateData {
            recipients:     vec![&env, r1.clone(), r2.clone()],
            amounts:        vec![&env, 1_000i128, 2_000i128],
            start_times:    vec![&env, 100u64, 150u64],
            end_times:      vec![&env, 200u64, 250u64],
            keeper_fees:    vec![&env, 0i128, 0i128],
            step_durations: vec![&env, 0u64, 0u64],
        };

        let ids = client.batch_create_vaults_lazy(&batch);
        assert_eq!(ids.len(), 2);
        assert_eq!(ids.get(0).unwrap(), 1u64);
        assert_eq!(ids.get(1).unwrap(), 2u64);
    }

    #[test]
    fn test_batch_create_vaults_full() {
        let (env, _cid, client, _admin) = setup();
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);

        let batch = BatchCreateData {
            recipients:     vec![&env, r1.clone(), r2.clone()],
            amounts:        vec![&env, 1_000i128, 2_000i128],
            start_times:    vec![&env, 100u64, 150u64],
            end_times:      vec![&env, 200u64, 250u64],
            keeper_fees:    vec![&env, 0i128, 0i128],
            step_durations: vec![&env, 0u64, 0u64],
        };

        let ids = client.batch_create_vaults_full(&batch);
        assert_eq!(ids.len(), 2);
    }

    // -------------------------------------------------------------------------
    // Step / lockup-only vesting
    // -------------------------------------------------------------------------

    #[test]
    fn test_step_vesting_full_claim_at_end() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let start = 1_000u64;
        let end   = start + 101u64;
        let step  = 17u64;
        let total = 1_009i128;

        let vault_id = client.create_vault_full(
            &beneficiary, &total, &start, &end,
            &0i128, &true, &true, &step,
        );

        env.ledger().with_mut(|l| l.timestamp = end + 1);
        let claimed = client.claim_tokens(&vault_id, &total);
        assert_eq!(claimed, total);

        let vault = client.get_vault(&vault_id);
        assert_eq!(vault.released_amount, total);
    }

    #[test]
    fn test_lockup_only_claim_succeeds_at_end() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now      = env.ledger().timestamp();
        let duration = 1_000u64;
        let total    = 100_000i128;

        let vault_id = client.create_vault_full(
            &beneficiary, &total, &now, &(now + duration),
            &0i128, &true, &false, &duration,
        );

        env.ledger().with_mut(|l| l.timestamp = now + duration);
        let claimed = client.claim_tokens(&vault_id, &total);
        assert_eq!(claimed, total);
    }

    #[test]
    #[should_panic]
    fn test_lockup_only_claim_fails_before_end() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now      = env.ledger().timestamp();
        let duration = 1_000u64;

        let vault_id = client.create_vault_full(
            &beneficiary, &100_000i128, &now, &(now + duration),
            &0i128, &true, &false, &duration,
        );

        env.ledger().with_mut(|l| l.timestamp = now + duration - 1);
        client.claim_tokens(&vault_id, &1i128);
    }

    #[test]
    fn test_periodic_vesting_monthly_steps() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        
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
        env.ledger().with_mut(|l| l.timestamp = start_time - 1000);
        let claimable = client.get_claimable_amount(&vault_id);
        assert_eq!(claimable, 0, "Should have no claimable tokens before start time");
        
        // Test 2: After 15 days (less than one step) - still no vesting (rounds down)
        env.ledger().with_mut(|l| l.timestamp = start_time + (15 * 24 * 60 * 60));
        let claimable = client.get_claimable_amount(&vault_id);
        assert_eq!(claimable, 0, "Should have no claimable tokens before first step completes");
        
        // Test 3: After exactly 30 days - one step completed
        env.ledger().with_mut(|l| l.timestamp = start_time + step_duration);
        let claimable = client.get_claimable_amount(&vault_id);
        let expected_monthly = amount / 12; // 100,000 tokens per month
        assert_eq!(claimable, expected_monthly, "Should have exactly one month of tokens after 30 days");
        
        // Test 4: After 45 days - still only one step (rounds down)
        env.ledger().with_mut(|l| l.timestamp = start_time + (45 * 24 * 60 * 60));
        let claimable = client.get_claimable_amount(&vault_id);
        assert_eq!(claimable, expected_monthly, "Should still have only one month of tokens after 45 days");
        
        // Test 5: After 60 days - two steps completed
        env.ledger().with_mut(|l| l.timestamp = start_time + (2 * step_duration));
        let claimable = client.get_claimable_amount(&vault_id);
        assert_eq!(claimable, 2 * expected_monthly, "Should have two months of tokens after 60 days");
        
        // Test 6: After 6 months - 6 steps completed
        env.ledger().with_mut(|l| l.timestamp = start_time + (6 * step_duration));
        let claimable = client.get_claimable_amount(&vault_id);
        assert_eq!(claimable, 6 * expected_monthly, "Should have six months of tokens after 6 months");
        
        // Test 7: After end time - all tokens vested
        env.ledger().with_mut(|l| l.timestamp = end_time + 1000);
        let claimable = client.get_claimable_amount(&vault_id);
        assert_eq!(claimable, amount, "Should have all tokens vested after end time");
    }

    #[test]
    fn test_periodic_vesting_weekly_steps() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        
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
        env.ledger().with_mut(|l| l.timestamp = start_time + (3 * step_duration));
        let claimable = client.get_claimable_amount(&vault_id);
        let expected_weekly = 10000i128; // 10,000 tokens per week
        assert_eq!(claimable, 3 * expected_weekly, "Should have three weeks of tokens after 3 weeks");
        
        // Test: After 10 weeks - 10 steps completed
        env.ledger().with_mut(|l| l.timestamp = start_time + (10 * step_duration));
        let claimable = client.get_claimable_amount(&vault_id);
        assert_eq!(claimable, 10 * expected_weekly, "Should have ten weeks of tokens after 10 weeks");
    }

    #[test]
    fn test_linear_vesting_step_duration_zero() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        
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
        env.ledger().with_mut(|l| l.timestamp = start_time + (182 * 24 * 60 * 60)); // ~6 months
        let claimable = client.get_claimable_amount(&vault_id);
        let expected_half = amount / 2; // 50% of tokens
        // Due to integer math and exactly 182 days vs 365, it will be close
        assert!(claimable > 598000i128 && claimable < 602000i128, "Should have ~50% of tokens after half the time for linear vesting");
        
        // Test: After 3 months (quarter of the duration) - should have 25% vested
        env.ledger().with_mut(|l| l.timestamp = start_time + (91 * 24 * 60 * 60)); // ~3 months
        let claimable = client.get_claimable_amount(&vault_id);
        assert!(claimable > 298000i128 && claimable < 302000i128, "Should have ~25% of tokens after quarter of the time for linear vesting");
    }

    #[test]
    fn test_periodic_vesting_claim_partial() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        
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
        env.ledger().with_mut(|l| l.timestamp = start_time + (3 * step_duration));
        
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
    fn test_step_vesting_fuzz() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        
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

    // -------------------------------------------------------------------------
    // Irrevocable vault
    // -------------------------------------------------------------------------

    #[test]
    fn test_mark_irrevocable_flag() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &1_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        assert!(!client.is_vault_irrevocable(&vault_id));
        client.mark_irrevocable(&vault_id);
        assert!(client.is_vault_irrevocable(&vault_id));
    }

    #[test]
    #[should_panic]
    fn test_revoke_irrevocable_vault_panics() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &1_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        client.mark_irrevocable(&vault_id);
        client.revoke_tokens(&vault_id);
    }

    // -------------------------------------------------------------------------
    // Clawback
    // -------------------------------------------------------------------------

    #[test]
    fn test_clawback_within_grace_period() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &5_000i128, &(now + 100), &(now + 10_000),
            &0i128, &true, &false, &0u64,
        );

        env.ledger().with_mut(|l| l.timestamp = now + 3_599);
        let returned = client.clawback_vault(&vault_id);
        assert_eq!(returned, 5_000i128);
    }

    #[test]
    #[should_panic]
    fn test_clawback_after_grace_period_panics() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &5_000i128, &(now + 100), &(now + 10_000),
            &0i128, &true, &false, &0u64,
        );

        env.ledger().with_mut(|l| l.timestamp = now + 3_601);
        client.clawback_vault(&vault_id);
    }

    // -------------------------------------------------------------------------
    // Milestones
    // -------------------------------------------------------------------------

    #[test]
    fn test_milestone_unlock_and_claim() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &1_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        let milestones = vec![
            &env,
            Milestone { id: 1, percentage: 50, is_unlocked: false },
            Milestone { id: 2, percentage: 50, is_unlocked: false },
        ];
        client.set_milestones(&vault_id, &milestones);

        client.unlock_milestone(&vault_id, &1u64);
        let claimed = client.claim_tokens(&vault_id, &500i128);
        assert_eq!(claimed, 500i128);

        client.unlock_milestone(&vault_id, &2u64);
        let claimed2 = client.claim_tokens(&vault_id, &500i128);
        assert_eq!(claimed2, 500i128);
    }

    #[test]
    #[should_panic]
    fn test_claim_before_any_milestone_unlocked_panics() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &1_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        let milestones = vec![
            &env,
            Milestone { id: 1, percentage: 100, is_unlocked: false },
        ];
        client.set_milestones(&vault_id, &milestones);
        client.claim_tokens(&vault_id, &1i128);
    }

    // -------------------------------------------------------------------------
    // Rotate beneficiary key
    // -------------------------------------------------------------------------

    #[test]
    fn test_rotate_beneficiary_key() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary     = Address::generate(&env);
        let new_beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &1_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        client.rotate_beneficiary_key(&vault_id, &new_beneficiary);

        let vault = client.get_vault(&vault_id);
        assert_eq!(vault.owner, new_beneficiary);
        assert_eq!(vault.delegate, None);
    }

    // -------------------------------------------------------------------------
    // Invariant
    // -------------------------------------------------------------------------

    #[test]
    fn test_invariant_holds_after_operations() {
        let (env, _cid, client, _admin) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();
        let initial_supply = 1_000_000i128;
        let vault_amount = 10_000i128;

        // After vault creation: admin_balance = 990_000, vault locked = 10_000
        let vault_id = client.create_vault_full(
            &beneficiary, &vault_amount, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );
        assert!(client.check_invariant(), "invariant failed after vault creation");
        let (locked, _claimed, admin_bal) = client.get_contract_state();
        assert_eq!(locked + admin_bal, initial_supply, "locked + admin should equal initial supply before any claims");

        // After partial claim: 5_000 paid out, 5_000 still locked
        env.ledger().with_mut(|l| l.timestamp = now + 500);
        client.claim_tokens(&vault_id, &5_000i128);
        assert!(client.check_invariant(), "invariant failed after partial claim");
        let (locked2, _claimed2, admin_bal2) = client.get_contract_state();
        // 5_000 was paid out to beneficiary, so locked + admin = initial - 5_000
        assert_eq!(locked2 + admin_bal2, initial_supply - 5_000i128, "5000 should have left the pool");

        // After revoke: remaining 5_000 returned to admin
        client.revoke_tokens(&vault_id);
        assert!(client.check_invariant(), "invariant failed after revoke");
        let (locked3, _claimed3, admin_bal3) = client.get_contract_state();
        // Still only 5_000 paid out total (the revoked portion came back to admin)
        assert_eq!(locked3, 0i128, "no tokens should remain locked after full revoke");
        assert_eq!(admin_bal3, initial_supply - 5_000i128, "admin should hold everything except what was claimed");
        assert_eq!(locked3 + admin_bal3, initial_supply - 5_000i128, "invariant: only claimed tokens are gone");
    }

    // =========================================================================
    // rescue_unallocated_tokens
    // =========================================================================

    #[test]
    fn test_rescue_basic_no_vaults() {
        let (env, contract_id, client, admin) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        mint_to(&env, &token_addr, &contract_id, 5_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 5_000i128);

        let tok = token::Client::new(&env, &token_addr);
        assert_eq!(tok.balance(&admin),       5_000i128);
        assert_eq!(tok.balance(&contract_id), 0i128);
    }

    #[test]
    fn test_rescue_only_surplus_above_vault_liability() {
        let (env, contract_id, client, admin) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        client.create_vault_full(
            &beneficiary, &3_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        // 3000 liability + 2000 stray
        mint_to(&env, &token_addr, &contract_id, 5_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 2_000i128);

        let tok = token::Client::new(&env, &token_addr);
        assert_eq!(tok.balance(&admin),       2_000i128);
        assert_eq!(tok.balance(&contract_id), 3_000i128);
    }

    #[test]
    fn test_rescue_after_partial_claim_adjusts_liability() {
        let (env, contract_id, client, admin) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &4_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        // Claim 1000 → remaining liability 3000
        env.ledger().with_mut(|l| l.timestamp = now + 1_001);
        client.claim_tokens(&vault_id, &1_000i128);

        mint_to(&env, &token_addr, &contract_id, 5_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 2_000i128);

        let tok = token::Client::new(&env, &token_addr);
        assert_eq!(tok.balance(&admin),       2_000i128);
        assert_eq!(tok.balance(&contract_id), 3_000i128);
    }

    #[test]
    fn test_rescue_multiple_vaults_correct_liability_sum() {
        let (env, contract_id, client, admin) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let now = env.ledger().timestamp();

        // Three vaults × 2000 = 6000 total liability
        for _ in 0..3 {
            let b = Address::generate(&env);
            client.create_vault_full(
                &b, &2_000i128, &now, &(now + 1_000),
                &0i128, &true, &false, &0u64,
            );
        }

        // 6000 liability + 1000 stray
        mint_to(&env, &token_addr, &contract_id, 7_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 1_000i128);

        let tok = token::Client::new(&env, &token_addr);
        assert_eq!(tok.balance(&admin),       1_000i128);
        assert_eq!(tok.balance(&contract_id), 6_000i128);
    }

    #[test]
    fn test_rescue_after_full_claim_all_tokens_rescuable() {
        let (env, contract_id, client, admin) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &2_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        // Claim everything → liability = 0
        env.ledger().with_mut(|l| l.timestamp = now + 1_001);
        client.claim_tokens(&vault_id, &2_000i128);

        // Stray deposit after full claim
        mint_to(&env, &token_addr, &contract_id, 500i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 500i128);

        let tok = token::Client::new(&env, &token_addr);
        assert_eq!(tok.balance(&admin),       500i128);
        assert_eq!(tok.balance(&contract_id), 0i128);
    }

    #[test]
    fn test_rescue_after_revoke_liability_drops_to_zero() {
        let (env, contract_id, client, admin) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &3_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        // Revoke → vault liability drops to 0
        client.revoke_tokens(&vault_id);

        mint_to(&env, &token_addr, &contract_id, 3_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 3_000i128);
    }

    #[test]
    fn test_rescue_tokens_go_to_current_admin_after_transfer() {
        let (env, contract_id, client, admin) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        // Transfer admin to new_admin
        let new_admin = Address::generate(&env);
        client.propose_new_admin(&new_admin);
        client.accept_ownership();
        assert_eq!(client.get_admin(), new_admin);

        mint_to(&env, &token_addr, &contract_id, 1_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 1_000i128);

        let tok = token::Client::new(&env, &token_addr);
        assert_eq!(tok.balance(&new_admin), 1_000i128); // new admin gets tokens
        assert_eq!(tok.balance(&admin),     0i128);     // old admin gets nothing
    }

    #[test]
    #[should_panic]
    fn test_rescue_panics_when_no_surplus() {
        let (env, contract_id, client, admin) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        client.create_vault_full(
            &beneficiary, &3_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        // Mint exactly the liability — zero surplus
        mint_to(&env, &token_addr, &contract_id, 3_000i128);

        client.rescue_unallocated_tokens(&token_addr); // must panic
    }

    #[test]
    #[should_panic]
    fn test_rescue_panics_when_contract_balance_zero() {
        let (env, _cid, client, admin) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        client.rescue_unallocated_tokens(&token_addr); // must panic
    }

    #[test]
    #[should_panic]
    fn test_rescue_panics_for_non_whitelisted_token() {
        let (env, contract_id, client, admin) = setup();
        // Register but do NOT whitelist
        let token_addr = register_token(&env, &admin);
        mint_to(&env, &token_addr, &contract_id, 1_000i128);

        client.rescue_unallocated_tokens(&token_addr); // must panic
    }
}
}

