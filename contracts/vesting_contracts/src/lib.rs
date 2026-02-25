#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, vec, Address, Env, IntoVal, Map, Symbol, Vec};

// DataKey for whitelisted tokens
#[contracttype]
pub enum WhitelistDataKey {
    WhitelistedTokens,
}

mod factory;
pub use factory::{VestingFactory, VestingFactoryClient};

#[contract]
pub struct VestingContract;

#[contracttype]
pub enum DataKey {
    AdminAddress,
    AdminBalance,
    InitialSupply,
    ProposedAdmin,
    VaultCount,
    VaultData(u64),
    VaultMilestones(u64),
    UserVaults(Address),
    KeeperFees,
    Token,        // yield-bearing token
    TotalShares,  // remaining initial_deposit_shares
    TotalStaked,
}

#[contracttype]
#[derive(Clone)]
pub struct Vault {
    pub owner: Address,
    pub delegate: Option<Address>,
    pub total_amount: i128, // = initial_deposit_shares
    pub released_amount: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub keeper_fee: i128,
    pub is_initialized: bool,
    pub is_irrevocable: bool,
    pub creation_time: u64,
    pub is_transferable: bool,
    pub step_duration: u64,
    pub staked_amount: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct Milestone {
    pub id: u64,
    pub percentage: u32,
    pub is_unlocked: bool,
}

#[contracttype]
pub struct BatchCreateData {
    pub recipients: Vec<Address>,
    pub amounts: Vec<i128>,
    pub start_times: Vec<u64>,
    pub end_times: Vec<u64>,
    pub keeper_fees: Vec<i128>,
    pub step_durations: Vec<u64>,
}

#[contracttype]
pub struct TokensRevoked {
    pub vault_id: u64,
    pub vested_amount: i128,
    pub unvested_amount: i128,
    pub beneficiary: Address,
    pub timestamp: u64,
}

#[contracttype]
pub struct VaultCreated {
    pub vault_id: u64,
    pub beneficiary: Address,
    pub total_amount: i128,
    pub cliff_duration: u64,
    pub start_time: u64,
}

#[contractimpl]
#[allow(deprecated)]
impl VestingContract {
    pub fn add_to_whitelist(env: Env, token: Address) {
        Self::require_admin(&env);
        let mut whitelist: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&WhitelistDataKey::WhitelistedTokens)
            .unwrap_or(Map::new(&env));
        whitelist.set(token.clone(), true);
        env.storage()
            .instance()
            .set(&WhitelistDataKey::WhitelistedTokens, &whitelist);
    }

    fn is_token_whitelisted(env: &Env, token: &Address) -> bool {
        let whitelist: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&WhitelistDataKey::WhitelistedTokens)
            .unwrap_or(Map::new(env));
        whitelist.get(token.clone()).unwrap_or(false)
    }

    pub fn initialize(env: Env, admin: Address, initial_supply: i128) {
        env.storage().instance().set(&DataKey::InitialSupply, &initial_supply);
        env.storage().instance().set(&DataKey::AdminBalance, &initial_supply);
        env.storage().instance().set(&DataKey::AdminAddress, &admin);
        env.storage().instance().set(&DataKey::VaultCount, &0u64);

        let whitelist: Map<Address, bool> = Map::new(&env);
        env.storage().instance().set(&WhitelistDataKey::WhitelistedTokens, &whitelist);

        env.storage().instance().set(&DataKey::TotalShares, &0i128);
        env.storage().instance().set(&DataKey::TotalStaked, &0i128);
    }

    pub fn set_token(env: Env, token: Address) {
        Self::require_admin(&env);
        if env.storage().instance().has(&DataKey::Token) {
            panic!("Token already set");
        }
        env.storage().instance().set(&DataKey::Token, &token);
    }

    fn get_token_client(env: &Env) -> token::Client {
        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic!("Token not set - call set_token first"));
        token::Client::new(env, &token)
    }

    fn require_admin(env: &Env) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::AdminAddress)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();
    }

    fn require_milestones_configured(env: &Env, vault_id: u64) -> Vec<Milestone> {
        let milestones: Vec<Milestone> = env
            .storage()
            .instance()
            .get(&DataKey::VaultMilestones(vault_id))
            .unwrap_or(Vec::new(env));
        if milestones.is_empty() {
            panic!("Milestones not configured");
        }
        milestones
    }

    fn unlocked_percentage(milestones: &Vec<Milestone>) -> u32 {
        let mut pct: u32 = 0;
        for m in milestones.iter() {
            if m.is_unlocked {
                pct = pct.saturating_add(m.percentage);
            }
        }
        if pct > 100 { 100 } else { pct }
    }

    fn unlocked_amount(total_amount: i128, unlocked_percentage: u32) -> i128 {
        (total_amount * unlocked_percentage as i128) / 100i128
    }

    pub fn propose_new_admin(env: Env, new_admin: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::ProposedAdmin, &new_admin);
    }

    pub fn accept_ownership(env: Env) {
        let proposed_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::ProposedAdmin)
            .unwrap_or_else(|| panic!("No proposed admin found"));
        proposed_admin.require_auth();
        env.storage().instance().set(&DataKey::AdminAddress, &proposed_admin);
        env.storage().instance().remove(&DataKey::ProposedAdmin);
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::AdminAddress).unwrap_or_else(|| panic!("Admin not set"))
    }

    pub fn get_proposed_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::ProposedAdmin)
    }

    pub fn create_vault_full(
        env: Env,
        owner: Address,
        amount: i128,
        start_time: u64,
        end_time: u64,
        keeper_fee: i128,
        is_revocable: bool,
        is_transferable: bool,
        step_duration: u64,
    ) -> u64 {
        Self::require_admin(&env);

        let mut vault_count: u64 = env.storage().instance().get(&DataKey::VaultCount).unwrap_or(0);
        vault_count += 1;

        let mut admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);
        if admin_balance < amount { panic!("Insufficient admin balance"); }
        admin_balance -= amount;
        env.storage().instance().set(&DataKey::AdminBalance, &admin_balance);

        let now = env.ledger().timestamp();

        let vault = Vault {
            owner: owner.clone(),
            delegate: None,
            total_amount: amount,
            released_amount: 0,
            start_time,
            end_time,
            keeper_fee,
            is_initialized: true,
            is_irrevocable: !is_revocable,
            creation_time: now,
            is_transferable,
            step_duration,
            staked_amount: 0,
        };

        env.storage().instance().set(&DataKey::VaultData(vault_count), &vault);

        let mut user_vaults: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(owner.clone())).unwrap_or(Vec::new(&env));
        user_vaults.push_back(vault_count);
        env.storage().instance().set(&DataKey::UserVaults(owner.clone()), &user_vaults);

        env.storage().instance().set(&DataKey::VaultCount, &vault_count);

        let mut total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        total_shares += amount;
        env.storage().instance().set(&DataKey::TotalShares, &total_shares);

        let cliff_duration = start_time.saturating_sub(now);
        let vault_created = VaultCreated { vault_id: vault_count, beneficiary: owner, total_amount: amount, cliff_duration, start_time };
        env.events().publish((Symbol::new(&env, "VaultCreated"), vault_count), vault_created);

        vault_count
    }

    pub fn create_vault_lazy(
        env: Env,
        owner: Address,
        amount: i128,
        start_time: u64,
        end_time: u64,
        keeper_fee: i128,
        is_revocable: bool,
        is_transferable: bool,
        step_duration: u64,
    ) -> u64 {
        Self::require_admin(&env);

        let mut vault_count: u64 = env.storage().instance().get(&DataKey::VaultCount).unwrap_or(0);
        vault_count += 1;

        let mut admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);
        if admin_balance < amount { panic!("Insufficient admin balance"); }
        admin_balance -= amount;
        env.storage().instance().set(&DataKey::AdminBalance, &admin_balance);

        let now = env.ledger().timestamp();

        let vault = Vault {
            owner: owner.clone(),
            delegate: None,
            total_amount: amount,
            released_amount: 0,
            start_time,
            end_time,
            keeper_fee,
            is_initialized: false,
            is_irrevocable: !is_revocable,
            creation_time: now,
            is_transferable,
            step_duration,
            staked_amount: 0,
        };

        env.storage().instance().set(&DataKey::VaultData(vault_count), &vault);
        env.storage().instance().set(&DataKey::VaultCount, &vault_count);

        let mut total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        total_shares += amount;
        env.storage().instance().set(&DataKey::TotalShares, &total_shares);

        let cliff_duration = start_time.saturating_sub(now);
        let vault_created = VaultCreated { vault_id: vault_count, beneficiary: owner.clone(), total_amount: amount, cliff_duration, start_time };
        env.events().publish((Symbol::new(&env, "VaultCreated"), vault_count), vault_created);

        vault_count
    }

    fn initialize_vault_metadata(env: &Env, vault_id: u64) -> bool {
        let vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if !vault.is_initialized {
            let mut updated_vault = vault.clone();
            updated_vault.is_initialized = true;

            env.storage().instance().set(&DataKey::VaultData(vault_id), &updated_vault);

            let mut user_vaults: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(updated_vault.owner.clone())).unwrap_or(Vec::new(env));
            user_vaults.push_back(vault_id);
            env.storage().instance().set(&DataKey::UserVaults(updated_vault.owner), &user_vaults);

            true
        } else {
            false
        }
    }

    fn calculate_time_vested_amount(env: &Env, vault: &Vault) -> i128 {
        let now = env.ledger().timestamp();
        if now < vault.start_time { return 0; }
        if now >= vault.end_time { return vault.total_amount; }
        let duration = vault.end_time - vault.start_time;
        if duration == 0 { return vault.total_amount; }
        let elapsed = now - vault.start_time;
        let effective_elapsed = if vault.step_duration > 0 {
            (elapsed / vault.step_duration) * vault.step_duration
        } else {
            elapsed
        };
        (vault.total_amount * effective_elapsed as i128) / duration as i128
    }

    pub fn claim_tokens(env: Env, vault_id: u64, claim_amount: i128) -> i128 {
        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| panic!("Vault not found"));

        if !vault.is_initialized { panic!("Vault not initialized"); }
        if claim_amount <= 0 { panic!("Claim amount must be positive"); }

        vault.owner.require_auth();

        let unlocked_amount = if env.storage().instance().has(&DataKey::VaultMilestones(vault_id)) {
            let milestones = Self::require_milestones_configured(&env, vault_id);
            let unlocked_pct = Self::unlocked_percentage(&milestones);
            Self::unlocked_amount(vault.total_amount, unlocked_pct)
        } else {
            Self::calculate_time_vested_amount(&env, &vault)
        };

        let liquid_balance = vault.total_amount - vault.released_amount - vault.staked_amount;
        if claim_amount > liquid_balance {
            let deficit = claim_amount - liquid_balance;

            let staking_contract: Address = env
                .storage()
                .instance()
                .get(&Symbol::new(&env, "StakingContract"))
                .expect("Staking contract not set");

            let args = vec![&env, vault_id.into_val(&env), deficit.into_val(&env)];
            env.invoke_contract::<()>(&staking_contract, &Symbol::new(&env, "unstake"), args);

            vault.staked_amount -= deficit;

            let mut total_staked: i128 = env.storage().instance().get(&DataKey::TotalStaked).unwrap_or(0);
            total_staked -= deficit;
            env.storage().instance().set(&DataKey::TotalStaked, &total_staked);
        }

        let available_to_claim = unlocked_amount - vault.released_amount;
        if available_to_claim <= 0 { panic!("No tokens available to claim"); }
        if claim_amount > available_to_claim { panic!("Insufficient unlocked tokens to claim"); }

        // YIELD DISTRIBUTION - only vault-owned portion
        let token_client = Self::get_token_client(&env);
        let current_balance = token_client.balance(&env.current_contract_address());
        let admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);

        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        let total_staked: i128 = env.storage().instance().get(&DataKey::TotalStaked).unwrap_or(0);
        let liquid_shares = total_shares - total_staked;

        let vault_portion = (current_balance - admin_balance).max(0);
        let transfer_amount = if liquid_shares > 0 {
            (claim_amount * vault_portion) / liquid_shares
        } else {
            claim_amount
        };

        vault.released_amount += claim_amount;
        let mut updated_total_shares = total_shares;
        updated_total_shares -= claim_amount;
        env.storage().instance().set(&DataKey::TotalShares, &updated_total_shares);
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        token_client.transfer(&env.current_contract_address(), &vault.owner, &transfer_amount);

        transfer_amount
    }

    pub fn transfer_beneficiary(env: Env, vault_id: u64, new_address: Address) {
        Self::require_admin(&env);

        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        let old_owner = vault.owner.clone();

        if vault.is_initialized {
            let old_vaults: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(old_owner.clone())).unwrap_or(Vec::new(&env));
            let mut updated_old_vaults = Vec::new(&env);
            for id in old_vaults.iter() {
                if id != vault_id {
                    updated_old_vaults.push_back(id);
                }
            }
            env.storage().instance().set(&DataKey::UserVaults(old_owner.clone()), &updated_old_vaults);

            let mut new_vaults: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(new_address.clone())).unwrap_or(Vec::new(&env));
            new_vaults.push_back(vault_id);
            env.storage().instance().set(&DataKey::UserVaults(new_address.clone()), &new_vaults);
        }

        vault.owner = new_address.clone();
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        env.events().publish((Symbol::new(&env, "BeneficiaryUpdated"), vault_id), (old_owner.clone(), new_address));
    }

    pub fn set_delegate(env: Env, vault_id: u64, delegate: Option<Address>) {
        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if !vault.is_initialized { panic!("Vault not initialized"); }

        vault.owner.require_auth();

        let old_delegate = vault.delegate.clone();

        vault.delegate = delegate.clone();
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        env.events().publish((Symbol::new(&env, "DelegateUpdated"), vault_id), (old_delegate, delegate));
    }

    pub fn claim_as_delegate(env: Env, vault_id: u64, claim_amount: i128) -> i128 {
        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if !vault.is_initialized { panic!("Vault not initialized"); }
        if claim_amount <= 0 { panic!("Claim amount must be positive"); }

        let delegate = vault.delegate.clone().unwrap_or_else(|| panic!("No delegate set for this vault"));
        delegate.require_auth();

        let milestones = Self::require_milestones_configured(&env, vault_id);
        let unlocked_pct = Self::unlocked_percentage(&milestones);
        let unlocked_amount = Self::unlocked_amount(vault.total_amount, unlocked_pct);
        let available_to_claim = unlocked_amount - vault.released_amount;
        if available_to_claim <= 0 { panic!("No tokens available to claim"); }
        if claim_amount > available_to_claim { panic!("Insufficient unlocked tokens to claim"); }

        // YIELD DISTRIBUTION - only vault-owned portion
        let token_client = Self::get_token_client(&env);
        let current_balance = token_client.balance(&env.current_contract_address());
        let admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);

        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        let total_staked: i128 = env.storage().instance().get(&DataKey::TotalStaked).unwrap_or(0);
        let liquid_shares = total_shares - total_staked;

        let vault_portion = (current_balance - admin_balance).max(0);
        let transfer_amount = if liquid_shares > 0 {
            (claim_amount * vault_portion) / liquid_shares
        } else {
            claim_amount
        };

        let mut updated_vault = vault.clone();
        updated_vault.released_amount += claim_amount;

        let mut updated_total_shares = total_shares;
        updated_total_shares -= claim_amount;
        env.storage().instance().set(&DataKey::TotalShares, &updated_total_shares);
        env.storage().instance().set(&DataKey::VaultData(vault_id), &updated_vault);

        token_client.transfer(&env.current_contract_address(), &updated_vault.owner, &transfer_amount);

        transfer_amount
    }

    pub fn set_milestones(env: Env, vault_id: u64, milestones: Vec<Milestone>) {
        Self::require_admin(&env);

        let vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));
        if !vault.is_initialized { panic!("Vault not initialized"); }

        if milestones.is_empty() { panic!("No milestones provided"); }

        let mut total_pct: u32 = 0;
        let mut seen: Map<u64, bool> = Map::new(&env);
        for m in milestones.iter() {
            if m.percentage == 0 { panic!("Milestone percentage must be positive"); }
            if m.percentage > 100 { panic!("Milestone percentage too large"); }
            if seen.contains_key(m.id) { panic!("Duplicate milestone id"); }
            seen.set(m.id, true);
            total_pct = total_pct.saturating_add(m.percentage);
        }
        if total_pct > 100 { panic!("Total milestone percentage exceeds 100"); }

        env.storage().instance().set(&DataKey::VaultMilestones(vault_id), &milestones);
        env.events().publish((Symbol::new(&env, "MilestonesSet"), vault_id), (milestones.len(), total_pct));
    }

    pub fn get_milestones(env: Env, vault_id: u64) -> Vec<Milestone> {
        env.storage().instance().get(&DataKey::VaultMilestones(vault_id)).unwrap_or(Vec::new(&env))
    }

    pub fn unlock_milestone(env: Env, vault_id: u64, milestone_id: u64) {
        Self::require_admin(&env);

        let _vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        let milestones = Self::require_milestones_configured(&env, vault_id);

        let mut found = false;
        let mut updated = Vec::new(&env);
        for m in milestones.iter() {
            if m.id == milestone_id {
                found = true;
                if m.is_unlocked { panic!("Milestone already unlocked"); }
                updated.push_back(Milestone { id: m.id, percentage: m.percentage, is_unlocked: true });
            } else {
                updated.push_back(m);
            }
        }
        if !found { panic!("Milestone not found"); }

        env.storage().instance().set(&DataKey::VaultMilestones(vault_id), &updated);
        let timestamp = env.ledger().timestamp();
        env.events().publish((Symbol::new(&env, "MilestoneUnlocked"), vault_id), (milestone_id, timestamp));
    }

    pub fn batch_create_vaults_lazy(env: Env, batch_data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);

        let mut vault_ids = Vec::new(&env);
        let initial_count: u64 = env.storage().instance().get(&DataKey::VaultCount).unwrap_or(0);

        let total_amount: i128 = batch_data.amounts.iter().sum();
        let mut admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);
        if admin_balance < total_amount { panic!("Insufficient admin balance for batch"); }
        admin_balance -= total_amount;
        env.storage().instance().set(&DataKey::AdminBalance, &admin_balance);

        let now = env.ledger().timestamp();
        for i in 0..batch_data.recipients.len() {
            let vault_id = initial_count + i as u64 + 1;

            let vault = Vault {
                owner: batch_data.recipients.get(i).unwrap(),
                delegate: None,
                total_amount: batch_data.amounts.get(i).unwrap(),
                released_amount: 0,
                start_time: batch_data.start_times.get(i).unwrap(),
                end_time: batch_data.end_times.get(i).unwrap(),
                keeper_fee: batch_data.keeper_fees.get(i).unwrap(),
                is_initialized: false,
                is_irrevocable: false,
                creation_time: now,
                is_transferable: false,
                step_duration: batch_data.step_durations.get(i).unwrap_or(0),
                staked_amount: 0,
            };

            env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
            vault_ids.push_back(vault_id);

            let start_time = batch_data.start_times.get(i).unwrap();
            let cliff_duration = start_time.saturating_sub(now);
            let vault_created = VaultCreated { vault_id, beneficiary: vault.owner.clone(), total_amount: vault.total_amount, cliff_duration, start_time };
            env.events().publish((Symbol::new(&env, "VaultCreated"), vault_id), vault_created);
        }

        let mut total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        total_shares += total_amount;
        env.storage().instance().set(&DataKey::TotalShares, &total_shares);

        let final_count = initial_count + batch_data.recipients.len() as u64;
        env.storage().instance().set(&DataKey::VaultCount, &final_count);

        vault_ids
    }

    pub fn batch_create_vaults_full(env: Env, batch_data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);

        let mut vault_ids = Vec::new(&env);
        let initial_count: u64 = env.storage().instance().get(&DataKey::VaultCount).unwrap_or(0);

        let total_amount: i128 = batch_data.amounts.iter().sum();
        let mut admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);
        if admin_balance < total_amount { panic!("Insufficient admin balance for batch"); }
        admin_balance -= total_amount;
        env.storage().instance().set(&DataKey::AdminBalance, &admin_balance);

        let now = env.ledger().timestamp();
        for i in 0..batch_data.recipients.len() {
            let vault_id = initial_count + i as u64 + 1;

            let vault = Vault {
                owner: batch_data.recipients.get(i).unwrap(),
                delegate: None,
                total_amount: batch_data.amounts.get(i).unwrap(),
                released_amount: 0,
                start_time: batch_data.start_times.get(i).unwrap(),
                end_time: batch_data.end_times.get(i).unwrap(),
                keeper_fee: batch_data.keeper_fees.get(i).unwrap(),
                is_initialized: true,
                is_irrevocable: false,
                creation_time: now,
                is_transferable: false,
                step_duration: batch_data.step_durations.get(i).unwrap_or(0),
                staked_amount: 0,
            };

            env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

            let mut user_vaults: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(vault.owner.clone())).unwrap_or(Vec::new(&env));
            user_vaults.push_back(vault_id);
            env.storage().instance().set(&DataKey::UserVaults(vault.owner.clone()), &user_vaults);

            vault_ids.push_back(vault_id);

            let start_time = batch_data.start_times.get(i).unwrap();
            let cliff_duration = start_time.saturating_sub(now);
            let vault_created = VaultCreated { vault_id, beneficiary: vault.owner.clone(), total_amount: vault.total_amount, cliff_duration, start_time };
            env.events().publish((Symbol::new(&env, "VaultCreated"), vault_id), vault_created);
        }

        let mut total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        total_shares += total_amount;
        env.storage().instance().set(&DataKey::TotalShares, &total_shares);

        let final_count = initial_count + batch_data.recipients.len() as u64;
        env.storage().instance().set(&DataKey::VaultCount, &final_count);

        vault_ids
    }

    pub fn get_vault(env: Env, vault_id: u64) -> Vault {
        let vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if !vault.is_initialized {
            Self::initialize_vault_metadata(&env, vault_id);
            env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap()
        } else {
            vault
        }
    }

    pub fn get_user_vaults(env: Env, user: Address) -> Vec<u64> {
        let vault_ids: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(user.clone())).unwrap_or(Vec::new(&env));

        for vault_id in vault_ids.iter() {
            let vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

            if !vault.is_initialized {
                Self::initialize_vault_metadata(&env, vault_id);
            }
        }

        vault_ids
    }

    pub fn revoke_tokens(env: Env, vault_id: u64) -> i128 {
        Self::require_admin(&env);

        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if vault.is_irrevocable { panic!("Vault is irrevocable"); }

        let unreleased_amount = vault.total_amount - vault.released_amount;
        if unreleased_amount <= 0 { panic!("No tokens available to revoke"); }

        vault.released_amount = vault.total_amount;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        let mut admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);
        admin_balance += unreleased_amount;
        env.storage().instance().set(&DataKey::AdminBalance, &admin_balance);

        let mut total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        total_shares -= unreleased_amount;
        env.storage().instance().set(&DataKey::TotalShares, &total_shares);

        let timestamp = env.ledger().timestamp();
        env.events().publish((Symbol::new(&env, "TokensRevoked"), vault_id), (unreleased_amount, timestamp));

        unreleased_amount
    }

    pub fn revoke_partial(env: Env, vault_id: u64, amount: i128) -> i128 {
        Self::require_admin(&env);

        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if vault.is_irrevocable { panic!("Vault is irrevocable"); }

        let unvested_balance = vault.total_amount - vault.released_amount;
        if amount <= 0 { panic!("Amount to revoke must be positive"); }
        if amount > unvested_balance { panic!("Amount exceeds unvested balance"); }

        vault.released_amount += amount;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        let mut admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);
        admin_balance += amount;
        env.storage().instance().set(&DataKey::AdminBalance, &admin_balance);

        let mut total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        total_shares -= amount;
        env.storage().instance().set(&DataKey::TotalShares, &total_shares);

        let timestamp = env.ledger().timestamp();
        env.events().publish((Symbol::new(&env, "TokensRevoked"), vault_id), (amount, timestamp));

        amount
    }

    pub fn clawback_vault(env: Env, vault_id: u64) -> i128 {
        Self::require_admin(&env);

        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        let now = env.ledger().timestamp();
        let grace_period = 3600u64;

        if now > vault.creation_time + grace_period { panic!("Grace period expired"); }
        if vault.released_amount > 0 { panic!("Tokens already claimed"); }

        let mut admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);
        admin_balance += vault.total_amount;
        env.storage().instance().set(&DataKey::AdminBalance, &admin_balance);

        vault.released_amount = vault.total_amount;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        let mut total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        total_shares -= vault.total_amount;
        env.storage().instance().set(&DataKey::TotalShares, &total_shares);

        env.events().publish((Symbol::new(&env, "VaultClawedBack"), vault_id), vault.total_amount);

        vault.total_amount
    }

    pub fn transfer_vault(env: Env, vault_id: u64, new_beneficiary: Address) {
        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if !vault.is_initialized { panic!("Vault not initialized"); }
        if !vault.is_transferable { panic!("Vault is non-transferable"); }

        vault.owner.require_auth();

        let old_owner = vault.owner.clone();

        let old_user_vaults: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(old_owner.clone())).unwrap_or(Vec::new(&env));

        let mut new_old_user_vaults = Vec::new(&env);
        for id in old_user_vaults.iter() {
            if id != vault_id {
                new_old_user_vaults.push_back(id);
            }
        }
        env.storage().instance().set(&DataKey::UserVaults(old_owner.clone()), &new_old_user_vaults);

        let mut new_user_vaults: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(new_beneficiary.clone())).unwrap_or(Vec::new(&env));
        new_user_vaults.push_back(vault_id);
        env.storage().instance().set(&DataKey::UserVaults(new_beneficiary.clone()), &new_user_vaults);

        vault.owner = new_beneficiary.clone();
        vault.delegate = None;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        env.events().publish((Symbol::new(&env, "BeneficiaryUpdated"), vault_id), (old_owner, new_beneficiary));
    }

    pub fn rotate_beneficiary_key(env: Env, vault_id: u64, new_address: Address) {
        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if !vault.is_initialized { panic!("Vault not initialized"); }

        vault.owner.require_auth();

        let old_owner = vault.owner.clone();

        let old_user_vaults: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(old_owner.clone())).unwrap_or(Vec::new(&env));

        let mut new_old_user_vaults = Vec::new(&env);
        for id in old_user_vaults.iter() {
            if id != vault_id {
                new_old_user_vaults.push_back(id);
            }
        }
        env.storage().instance().set(&DataKey::UserVaults(old_owner.clone()), &new_old_user_vaults);

        let mut new_user_vaults: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(new_address.clone())).unwrap_or(Vec::new(&env));
        new_user_vaults.push_back(vault_id);
        env.storage().instance().set(&DataKey::UserVaults(new_address.clone()), &new_user_vaults);

        vault.owner = new_address.clone();
        vault.delegate = None;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        env.events().publish((Symbol::new(&env, "BeneficiaryRotated"), vault_id), (old_owner, new_address));
    }

    pub fn set_staking_contract(env: Env, contract: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&Symbol::new(&env, "StakingContract"), &contract);
    }

    pub fn stake_tokens(env: Env, vault_id: u64, amount: i128, validator: Address) {
        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if !vault.is_initialized { panic!("Vault not initialized"); }

        vault.owner.require_auth();

        let available = vault.total_amount - vault.released_amount - vault.staked_amount;
        if amount <= 0 { panic!("Amount must be positive"); }
        if amount > available { panic!("Insufficient funds to stake"); }

        let staking_contract: Address = env.storage().instance().get(&Symbol::new(&env, "StakingContract")).expect("Staking contract not set");

        let args = vec![&env, vault_id.into_val(&env), amount.into_val(&env), validator.into_val(&env)];
        env.invoke_contract::<()>(&staking_contract, &Symbol::new(&env, "stake"), args);

        vault.staked_amount += amount;

        let mut total_staked: i128 = env.storage().instance().get(&DataKey::TotalStaked).unwrap_or(0);
        total_staked += amount;
        env.storage().instance().set(&DataKey::TotalStaked, &total_staked);

        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    pub fn mark_irrevocable(env: Env, vault_id: u64) {
        Self::require_admin(&env);

        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if vault.is_irrevocable { panic!("Vault is already irrevocable"); }

        vault.is_irrevocable = true;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        let timestamp = env.ledger().timestamp();
        env.events().publish((Symbol::new(&env, "IrrevocableMarked"), vault_id), timestamp);
    }

    pub fn is_vault_irrevocable(env: Env, vault_id: u64) -> bool {
        let vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));
        vault.is_irrevocable
    }

    pub fn get_contract_state(env: Env) -> (i128, i128, i128) {
        let admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);

        let vault_count: u64 = env.storage().instance().get(&DataKey::VaultCount).unwrap_or(0);
        let mut total_locked = 0i128;
        let mut total_claimed = 0i128;

        for i in 1..=vault_count {
            if let Some(vault) = env.storage().instance().get::<DataKey, Vault>(&DataKey::VaultData(i)) {
                total_locked += vault.total_amount - vault.released_amount;
                total_claimed += vault.released_amount;
            }
        }

        (total_locked, total_claimed, admin_balance)
    }

    pub fn check_invariant(env: Env) -> bool {
        let initial_supply: i128 = env.storage().instance().get(&DataKey::InitialSupply).unwrap_or(0);
        let (total_locked, _total_claimed, admin_balance) = Self::get_contract_state(env);

        let net_paid_out = initial_supply - total_locked - admin_balance;
        net_paid_out >= 0
    }

    pub fn get_claimable_amount(env: Env, vault_id: u64) -> i128 {
        let vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        let vested = Self::calculate_time_vested_amount(&env, &vault);

        if vested > vault.released_amount {
            vested - vault.released_amount
        } else {
            0
        }
    }

    pub fn auto_claim(env: Env, vault_id: u64, keeper: Address) {
        let mut vault: Vault = env.storage().instance().get(&DataKey::VaultData(vault_id)).unwrap_or_else(|| panic!("Vault not found"));

        if !vault.is_initialized { panic!("Vault not initialized"); }

        let claimable = Self::get_claimable_amount(env.clone(), vault_id);

        if claimable <= vault.keeper_fee { panic!("Insufficient claimable tokens to cover fee"); }

        let beneficiary_amount = claimable - vault.keeper_fee;
        let keeper_fee = vault.keeper_fee;

        // YIELD DISTRIBUTION - only vault-owned portion
        let token_client = Self::get_token_client(&env);
        let current_balance = token_client.balance(&env.current_contract_address());
        let admin_balance: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);

        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        let total_staked: i128 = env.storage().instance().get(&DataKey::TotalStaked).unwrap_or(0);
        let liquid_shares = total_shares - total_staked;

        let vault_portion = (current_balance - admin_balance).max(0);

        let beneficiary_tokens = if liquid_shares > 0 {
            (beneficiary_amount * vault_portion) / liquid_shares
        } else {
            beneficiary_amount
        };
        let keeper_tokens = if liquid_shares > 0 {
            (keeper_fee * vault_portion) / liquid_shares
        } else {
            keeper_fee
        };

        vault.released_amount += claimable;
        let mut updated_total_shares = total_shares;
        updated_total_shares -= claimable;
        env.storage().instance().set(&DataKey::TotalShares, &updated_total_shares);
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);

        token_client.transfer(&env.current_contract_address(), &vault.owner, &beneficiary_tokens);
        token_client.transfer(&env.current_contract_address(), &keeper, &keeper_tokens);

        let mut fees: Map<Address, i128> = env.storage().instance().get(&DataKey::KeeperFees).unwrap_or(Map::new(&env));
        let current_fees = fees.get(keeper.clone()).unwrap_or(0);
        fees.set(keeper.clone(), current_fees + keeper_fee);
        env.storage().instance().set(&DataKey::KeeperFees, &fees);

        env.events().publish((Symbol::new(&env, "KeeperClaim"), vault_id), (keeper, beneficiary_amount, keeper_fee));
    }

    pub fn get_keeper_fee(env: Env, keeper: Address) -> i128 {
        let fees: Map<Address, i128> = env.storage().instance().get(&DataKey::KeeperFees).unwrap_or(Map::new(&env));
        fees.get(keeper).unwrap_or(0)
    }

    pub fn rescue_unallocated_tokens(env: Env, token_address: Address) -> i128 {
        Self::require_admin(&env);

        if !Self::is_token_whitelisted(&env, &token_address) {
            panic!("Token is not whitelisted");
        }

        let token_client = token::Client::new(&env, &token_address);
        let contract_balance: i128 = token_client.balance(&env.current_contract_address());

        if let Some(main_token) = env.storage().instance().get::<_, Address>(&DataKey::Token) {
            if main_token == token_address {
                panic!("Cannot rescue yield-bearing token. Yield is distributed to beneficiaries on claim.");
            }
        }

        let vault_count: u64 = env.storage().instance().get(&DataKey::VaultCount).unwrap_or(0);

        let mut total_liabilities: i128 = 0;
        for i in 1..=vault_count {
            if let Some(vault) = env.storage().instance().get::<DataKey, Vault>(&DataKey::VaultData(i)) {
                let unreleased = vault.total_amount - vault.released_amount;
                if unreleased > 0 {
                    total_liabilities += unreleased;
                }
            }
        }

        let unallocated_balance = contract_balance - total_liabilities;

        if unallocated_balance <= 0 {
            panic!("No unallocated tokens to rescue");
        }

        let admin: Address = env.storage().instance().get(&DataKey::AdminAddress).unwrap_or_else(|| panic!("Admin not set"));

        token_client.transfer(&env.current_contract_address(), &admin, &unallocated_balance);

        env.events().publish((Symbol::new(&env, "RescueExecuted"), token_address), (unallocated_balance, admin));

        unallocated_balance
    }
}

mod test;