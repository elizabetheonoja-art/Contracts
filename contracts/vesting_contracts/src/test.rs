 #[cfg(test)]
mod tests {
    use crate::{
        BatchCreateData, Milestone, VestingContract, VestingContractClient,
    };
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, vec, Address, Env,
    };

    // -------------------------------------------------------------------------
    // Helper: fresh contract + yield-bearing token + tokens actually in contract
    // -------------------------------------------------------------------------

    fn setup() -> (Env, Address, VestingContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VestingContract, ());
        let client = VestingContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin, &1_000_000i128);

        let token_addr = register_token(&env, &admin);
        client.set_token(&token_addr);
        client.add_to_whitelist(&token_addr);

        // Mint initial supply to contract
        let stellar = token::StellarAssetClient::new(&env, &token_addr);
        stellar.mint(&contract_id, &1_000_000i128);

        (env, contract_id, client, admin, token_addr)
    }

    fn register_token(env: &Env, admin: &Address) -> Address {
        env.register_stellar_asset_contract_v2(admin.clone())
            .address()
    }

    fn mint_to(env: &Env, token_addr: &Address, recipient: &Address, amount: i128) {
        token::StellarAssetClient::new(env, token_addr).mint(recipient, &amount);
    }

    // -------------------------------------------------------------------------
    // Original tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_admin_ownership_transfer() {
        let (env, _cid, client, admin, _token) = setup();
        let new_admin = Address::generate(&env);

        assert_eq!(client.get_admin(), admin);
        assert_eq!(client.get_proposed_admin(), None);

        client.propose_new_admin(&new_admin);
        assert_eq!(client.get_proposed_admin(), Some(new_admin.clone()));

        client.accept_ownership();
        assert_eq!(client.get_admin(), new_admin);
        assert_eq!(client.get_proposed_admin(), None);
    }

    #[test]
    fn test_create_vault_full_increments_count() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let id1 = client.create_vault_full(&beneficiary, &1_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);
        let id2 = client.create_vault_full(&beneficiary, &500i128, &(now + 10), &(now + 2_000), &0i128, &true, &false, &0u64);
        assert_eq!(id1, 1u64);
        assert_eq!(id2, 2u64);
    }

    #[test]
    fn test_create_vault_lazy_increments_count() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let id = client.create_vault_lazy(&beneficiary, &1_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);
        assert_eq!(id, 1u64);
    }

    #[test]
    fn test_batch_create_vaults_lazy() {
        let (env, _cid, client, _admin, _token) = setup();
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);

        let batch = BatchCreateData {
            recipients: vec![&env, r1.clone(), r2.clone()],
            amounts: vec![&env, 1_000i128, 2_000i128],
            start_times: vec![&env, 100u64, 150u64],
            end_times: vec![&env, 200u64, 250u64],
            keeper_fees: vec![&env, 0i128, 0i128],
            step_durations: vec![&env, 0u64, 0u64],
        };

        let ids = client.batch_create_vaults_lazy(&batch);
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_batch_create_vaults_full() {
        let (env, _cid, client, _admin, _token) = setup();
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);

        let batch = BatchCreateData {
            recipients: vec![&env, r1.clone(), r2.clone()],
            amounts: vec![&env, 1_000i128, 2_000i128],
            start_times: vec![&env, 100u64, 150u64],
            end_times: vec![&env, 200u64, 250u64],
            keeper_fees: vec![&env, 0i128, 0i128],
            step_durations: vec![&env, 0u64, 0u64],
        };

        let ids = client.batch_create_vaults_full(&batch);
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_step_vesting_full_claim_at_end() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let start = 1_000u64;
        let end = start + 101u64;
        let step = 17u64;
        let total = 1_009i128;

        let vault_id = client.create_vault_full(&beneficiary, &total, &start, &end, &0i128, &true, &true, &step);

        env.ledger().with_mut(|l| l.timestamp = end + 1);
        let claimed = client.claim_tokens(&vault_id, &total);
        assert_eq!(claimed, total);

        let vault = client.get_vault(&vault_id);
        assert_eq!(vault.released_amount, total);
    }

    #[test]
    fn test_lockup_only_claim_succeeds_at_end() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();
        let duration = 1_000u64;
        let total = 100_000i128;

        let vault_id = client.create_vault_full(&beneficiary, &total, &now, &(now + duration), &0i128, &true, &false, &duration);

        env.ledger().with_mut(|l| l.timestamp = now + duration);
        let claimed = client.claim_tokens(&vault_id, &total);
        assert_eq!(claimed, total);
    }

    #[test]
    #[should_panic]
    fn test_lockup_only_claim_fails_before_end() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();
        let duration = 1_000u64;

        let vault_id = client.create_vault_full(&beneficiary, &100_000i128, &now, &(now + duration), &0i128, &true, &false, &duration);

        env.ledger().with_mut(|l| l.timestamp = now + duration - 1);
        client.claim_tokens(&vault_id, &1i128);
    }

    #[test]
    fn test_mark_irrevocable_flag() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &1_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);

        assert!(!client.is_vault_irrevocable(&vault_id));
        client.mark_irrevocable(&vault_id);
        assert!(client.is_vault_irrevocable(&vault_id));
    }

    #[test]
    #[should_panic]
    fn test_revoke_irrevocable_vault_panics() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &1_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);

        client.mark_irrevocable(&vault_id);
        client.revoke_tokens(&vault_id);
    }

    #[test]
    fn test_clawback_within_grace_period() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &5_000i128, &(now + 100), &(now + 10_000), &0i128, &true, &false, &0u64);

        env.ledger().with_mut(|l| l.timestamp = now + 3_599);
        let returned = client.clawback_vault(&vault_id);
        assert_eq!(returned, 5_000i128);
    }

    #[test]
    #[should_panic]
    fn test_clawback_after_grace_period_panics() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &5_000i128, &(now + 100), &(now + 10_000), &0i128, &true, &false, &0u64);

        env.ledger().with_mut(|l| l.timestamp = now + 3_601);
        client.clawback_vault(&vault_id);
    }

    #[test]
    fn test_milestone_unlock_and_claim() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &1_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);

        let milestones = vec![&env, Milestone { id: 1, percentage: 50, is_unlocked: false }, Milestone { id: 2, percentage: 50, is_unlocked: false }];
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
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &1_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);

        let milestones = vec![&env, Milestone { id: 1, percentage: 100, is_unlocked: false }];
        client.set_milestones(&vault_id, &milestones);
        client.claim_tokens(&vault_id, &1i128);
    }

    #[test]
    fn test_rotate_beneficiary_key() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let new_beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &1_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);

        client.rotate_beneficiary_key(&vault_id, &new_beneficiary);

        let vault = client.get_vault(&vault_id);
        assert_eq!(vault.owner, new_beneficiary);
    }

    #[test]
    fn test_invariant_holds_after_operations() {
        let (env, _cid, client, _admin, _token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &10_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);
        assert!(client.check_invariant());

        env.ledger().with_mut(|l| l.timestamp = now + 500);
        client.claim_tokens(&vault_id, &5_000i128);
        assert!(client.check_invariant());

        client.revoke_tokens(&vault_id);
        assert!(client.check_invariant());
    }

    // =========================================================================
    // rescue tests
    // =========================================================================

    #[test]
    fn test_rescue_basic_no_vaults() {
        let (env, contract_id, client, admin, _main_token) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        mint_to(&env, &token_addr, &contract_id, 5_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 5_000i128);
    }

    #[test]
    fn test_rescue_only_surplus_above_vault_liability() {
        let (env, contract_id, client, admin, _main_token) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        client.create_vault_full(&beneficiary, &3_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);

        mint_to(&env, &token_addr, &contract_id, 5_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 2_000i128);
    }

    #[test]
    fn test_rescue_after_partial_claim_adjusts_liability() {
        let (env, contract_id, client, admin, _main_token) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &4_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);

        env.ledger().with_mut(|l| l.timestamp = now + 1_001);
        client.claim_tokens(&vault_id, &1_000i128);

        mint_to(&env, &token_addr, &contract_id, 5_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 2_000i128);
    }

    #[test]
    fn test_rescue_multiple_vaults_correct_liability_sum() {
        let (env, contract_id, client, admin, _main_token) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let now = env.ledger().timestamp();

        for _ in 0..3 {
            let b = Address::generate(&env);
            client.create_vault_full(&b, &2_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);
        }

        mint_to(&env, &token_addr, &contract_id, 7_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 1_000i128);
    }

    #[test]
    fn test_rescue_after_full_claim_all_tokens_rescuable() {
        let (env, contract_id, client, admin, _main_token) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &2_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);

        env.ledger().with_mut(|l| l.timestamp = now + 1_001);
        client.claim_tokens(&vault_id, &2_000i128);

        mint_to(&env, &token_addr, &contract_id, 500i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 500i128);
    }

    #[test]
    fn test_rescue_after_revoke_liability_drops_to_zero() {
        let (env, contract_id, client, admin, _main_token) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(&beneficiary, &3_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);

        client.revoke_tokens(&vault_id);

        mint_to(&env, &token_addr, &contract_id, 3_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 3_000i128);
    }

    #[test]
    fn test_rescue_tokens_go_to_current_admin_after_transfer() {
        let (env, contract_id, client, admin, _main_token) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let new_admin = Address::generate(&env);
        client.propose_new_admin(&new_admin);
        client.accept_ownership();

        mint_to(&env, &token_addr, &contract_id, 1_000i128);

        let rescued = client.rescue_unallocated_tokens(&token_addr);
        assert_eq!(rescued, 1_000i128);
    }

    #[test]
    #[should_panic]
    fn test_rescue_panics_when_no_surplus() {
        let (env, contract_id, client, admin, _main_token) = setup();
        let token_addr = register_token(&env, &admin);
        client.add_to_whitelist(&token_addr);

        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        client.create_vault_full(&beneficiary, &3_000i128, &now, &(now + 1_000), &0i128, &true, &false, &0u64);

        mint_to(&env, &token_addr, &contract_id, 3_000i128);

        client.rescue_unallocated_tokens(&token_addr);
    }

    #[test]
    #[should_panic]
    fn test_rescue_panics_when_contract_balance_zero() {
        let (env, _cid, client, _admin, _token) = setup();
        let token_addr = register_token(&env, &_admin);
        client.add_to_whitelist(&token_addr);

        client.rescue_unallocated_tokens(&token_addr);
    }

    #[test]
    #[should_panic]
    fn test_rescue_panics_for_non_whitelisted_token() {
        let (env, contract_id, client, admin, _main_token) = setup();
        let token_addr = register_token(&env, &admin);
        mint_to(&env, &token_addr, &contract_id, 1_000i128);

        client.rescue_unallocated_tokens(&token_addr);
    }

    // =========================================================================
    // Yield demonstration tests
    // =========================================================================

    #[test]
    fn test_yield_is_distributed_on_claim() {
        let (env, contract_id, client, _admin, token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &10_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        // Simulate yield
        let stellar = token::StellarAssetClient::new(&env, &token);
        stellar.mint(&contract_id, &2_000i128);

        env.ledger().with_mut(|l| l.timestamp = now + 1_001);
        let claimed = client.claim_tokens(&vault_id, &10_000i128);

        assert_eq!(claimed, 12_000i128); // principal + all yield
    }

    #[test]
    fn test_yield_on_partial_claim() {
        let (env, contract_id, client, _admin, token) = setup();
        let beneficiary = Address::generate(&env);
        let now = env.ledger().timestamp();

        let vault_id = client.create_vault_full(
            &beneficiary, &10_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        let stellar = token::StellarAssetClient::new(&env, &token);
        stellar.mint(&contract_id, &2_000i128);

        env.ledger().with_mut(|l| l.timestamp = now + 500);
        let claimed = client.claim_tokens(&vault_id, &5_000i128);

        assert_eq!(claimed, 6_000i128); // 5k principal + 1k yield
    }

    #[test]
    fn test_yield_proportional_with_multiple_vaults() {
        let (env, contract_id, client, _admin, token) = setup();
        let now = env.ledger().timestamp();

        let v1 = client.create_vault_full(
            &Address::generate(&env), &10_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );
        let v2 = client.create_vault_full(
            &Address::generate(&env), &20_000i128, &now, &(now + 1_000),
            &0i128, &true, &false, &0u64,
        );

        let stellar = token::StellarAssetClient::new(&env, &token);
        stellar.mint(&contract_id, &6_000i128);

        env.ledger().with_mut(|l| l.timestamp = now + 1_001);

        let claimed1 = client.claim_tokens(&v1, &10_000i128);
        let claimed2 = client.claim_tokens(&v2, &20_000i128);

        assert_eq!(claimed1, 12_000i128);
        assert_eq!(claimed2, 24_000i128);
    }

    #[test]
    #[should_panic(expected = "Cannot rescue yield-bearing token")]
    fn test_rescue_yield_token_panics() {
        let (env, _cid, client, _admin, token) = setup();
        client.rescue_unallocated_tokens(&token);
    }
}