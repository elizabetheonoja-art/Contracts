#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Map, Symbol, Vec};

mod factory;
pub use factory::{VestingFactory, VestingFactoryClient};

#[contract]
pub struct VestingContract;



// Vault structure with lazy initialization
#[contracttype]
#[derive(Clone)]
pub struct Vault {
    pub owner: Address,
    pub delegate: Option<Address>, // Optional delegate address for claiming
    pub total_amount: i128,
    pub released_amount: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub keeper_fee: i128, // Fee paid to anyone who triggers auto_claim
    pub is_initialized: bool, // Lazy initialization flag
    pub is_irrevocable: bool, // Security flag to prevent admin withdrawal
    pub creation_time: u64, // Timestamp of creation for clawback grace period
    pub is_transferable: bool, // Can the beneficiary transfer this vault?
    pub step_duration: u64, // Duration of each vesting step in seconds (0 = linear)
    pub staked_amount: i128, // Amount currently staked in external contract
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
    // Initialize contract with initial supply
    pub fn initialize(env: Env, admin: Address, initial_supply: i128) {
        // Set initial supply
        env.storage()
            .instance()
            .set(&DataKey::InitialSupply, &initial_supply);

        // Set admin balance (initially all tokens go to admin)
        env.storage()
            .instance()
            .set(&DataKey::AdminBalance, &initial_supply);

        // Set admin address
        env.storage().instance().set(&DataKey::AdminAddress, &admin);

        // Initialize vault count
        env.storage().instance().set(&DataKey::VaultCount, &0u64);
    }

    // Helper function to check if caller is admin
    fn require_admin(env: &Env) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::AdminAddress)
            .unwrap_or_else(|| panic!("Admin not set"));
        let caller = env.current_contract_address();
        if caller != admin {
            panic!("Caller is not admin");
        }
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
        if pct > 100 {
            100
        } else {
            pct
        }
    }

    fn unlocked_amount(total_amount: i128, unlocked_percentage: u32) -> i128 {
        (total_amount * unlocked_percentage as i128) / 100i128
    }

    // Propose a new admin (first step of two-step process)
    pub fn propose_new_admin(env: Env, new_admin: Address) {
        Self::require_admin(&env);

        // Store the proposed admin
        env.storage()
            .instance()
            .set(&DataKey::ProposedAdmin, &new_admin);
    }

    // Accept admin ownership (second step of two-step process)
    pub fn accept_ownership(env: Env) {
        let proposed_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::ProposedAdmin)
            .unwrap_or_else(|| panic!("No proposed admin found"));

        let caller = env.current_contract_address();
        if caller != proposed_admin {
            panic!("Caller is not the proposed admin");
        }

        // Transfer admin rights
        env.storage()
            .instance()
            .set(&DataKey::AdminAddress, &proposed_admin);

        // Clear the proposed admin
        env.storage().instance().remove(&DataKey::ProposedAdmin);
    }

    // Get current admin address
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::AdminAddress)
            .unwrap_or_else(|| panic!("Admin not set"))
    }

    // Get proposed admin address (if any)
    pub fn get_proposed_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::ProposedAdmin)
    }

    // Full initialization - writes all metadata immediately

        Self::require_admin(&env);

        // Get next vault ID
        let mut vault_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VaultCount)
            .unwrap_or(0);
        vault_count += 1;

        // Check admin balance and transfer tokens
        let mut admin_balance: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AdminBalance)
            .unwrap_or(0);
        if admin_balance < amount {
            panic!("Insufficient admin balance");
        }
        admin_balance -= amount;
        env.storage()
            .instance()
            .set(&DataKey::AdminBalance, &admin_balance);

        let now = env.ledger().timestamp();

        // Create vault with full initialization
        let vault = Vault {
            owner: owner.clone(),
            delegate: None, // No delegate initially
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

        // Store vault data immediately (expensive gas usage)
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_count), &vault);

        // Update user vaults list
        let mut user_vaults: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::UserVaults(owner.clone()))
            .unwrap_or(Vec::new(&env));
        user_vaults.push_back(vault_count);
        env.storage()
            .instance()
            .set(&DataKey::UserVaults(owner.clone()), &user_vaults);

        // Update vault count
        env.storage()
            .instance()
            .set(&DataKey::VaultCount, &vault_count);

        // Emit VaultCreated event with strictly typed fields
        let cliff_duration = start_time.saturating_sub(now);
        let vault_created = VaultCreated {
            vault_id: vault_count,
            beneficiary: owner,
            total_amount: amount,
            cliff_duration,
            start_time,
        };
        env.events().publish(
            (Symbol::new(&env, "VaultCreated"), vault_count),
            vault_created,
        );

        vault_count
    }

    // Lazy initialization - writes minimal data initially

        // Get next vault ID
        let mut vault_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VaultCount)
            .unwrap_or(0);
        vault_count += 1;

        // Check admin balance and transfer tokens
        let mut admin_balance: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AdminBalance)
            .unwrap_or(0);
        if admin_balance < amount {
            panic!("Insufficient admin balance");
        }
        admin_balance -= amount;
        env.storage()
            .instance()
            .set(&DataKey::AdminBalance, &admin_balance);

        let now = env.ledger().timestamp();

        // Create vault with lazy initialization (minimal storage)
        let vault = Vault {
            owner: owner.clone(),
            delegate: None, // No delegate initially
            total_amount: amount,
            released_amount: 0,
            start_time,
            end_time,
            keeper_fee,
            is_initialized: false, // Mark as lazy initialized
            is_irrevocable: !is_revocable, // Convert from is_revocable parameter
            creation_time: now,
            is_transferable,
            step_duration,
            staked_amount: 0,
        };

        // Store only essential data initially (cheaper gas)
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_count), &vault);

        // Update vault count
        env.storage()
            .instance()
            .set(&DataKey::VaultCount, &vault_count);

        // Don't update user vaults list yet (lazy)

        // Emit VaultCreated event with strictly typed fields
        let cliff_duration = start_time.saturating_sub(now);
        let vault_created = VaultCreated {
            vault_id: vault_count,
            beneficiary: owner.clone(),
            total_amount: amount,
            cliff_duration,
            start_time,
        };
        env.events().publish(
            (Symbol::new(&env, "VaultCreated"), vault_count),
            vault_created,
        );

        vault_count
    }

    // Initialize vault metadata when needed (on-demand)

        // Only initialize if not already initialized
        if !vault.is_initialized {
            let mut updated_vault = vault.clone();
            updated_vault.is_initialized = true;

            // Store updated vault with full metadata
            env.storage()
                .instance()
                .set(&DataKey::VaultData(vault_id), &updated_vault);

            // Update user vaults list (deferred)
            let mut user_vaults: Vec<u64> = env
                .storage()
                .instance()
                .get(&DataKey::UserVaults(updated_vault.owner.clone()))
                .unwrap_or(Vec::new(env));
            user_vaults.push_back(vault_id);
            env.storage()
                .instance()
                .set(&DataKey::UserVaults(updated_vault.owner), &user_vaults);

            true
        } else {
            false // Already initialized
        }
    }

    // Helper to calculate vested amount based on time (linear or step)
    fn calculate_time_vested_amount(env: &Env, vault: &Vault) -> i128 {
        let now = env.ledger().timestamp();
        if now < vault.start_time {
            return 0;
        }
        if now >= vault.end_time {
            return vault.total_amount;
        }
        let duration = vault.end_time - vault.start_time;
        if duration == 0 {
            return vault.total_amount;
        }
        let elapsed = now - vault.start_time;
        let effective_elapsed = if vault.step_duration > 0 {
            (elapsed / vault.step_duration) * vault.step_duration
        } else {
            elapsed
        };
        
        // Use i128 math
        (vault.total_amount * effective_elapsed as i128) / duration as i128
    }

    // Claim tokens from vault
    pub fn claim_tokens(env: Env, vault_id: u64, claim_amount: i128) -> i128 {
        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        if !vault.is_initialized {
            panic!("Vault not initialized");
        }
        if claim_amount <= 0 {
            panic!("Claim amount must be positive");
        }

        // Check if milestones are configured
        let unlocked_amount = if env.storage().instance().has(&DataKey::VaultMilestones(vault_id)) {
            let milestones = Self::require_milestones_configured(&env, vault_id);
            let unlocked_pct = Self::unlocked_percentage(&milestones);
            Self::unlocked_amount(vault.total_amount, unlocked_pct)
        } else {
            // Fallback to time-based vesting
            Self::calculate_time_vested_amount(&env, &vault)
        };

        // Auto-unstake logic if needed
        let liquid_balance = vault.total_amount - vault.released_amount - vault.staked_amount;
        if claim_amount > liquid_balance {
            let deficit = claim_amount - liquid_balance;
            
            // Get staking contract
            let staking_contract: Address = env.storage().instance()
                .get(&Symbol::new(&env, "StakingContract"))
                .expect("Staking contract not set");

            // Call unstake on external contract
            let args = vec![&env, vault_id.into_val(&env), deficit.into_val(&env)];
            env.invoke_contract::<()>(&staking_contract, &Symbol::new(&env, "unstake"), args);

            // Update local state
            vault.staked_amount -= deficit;
            // Note: We don't save vault here yet, it's saved at the end of function
        }

        let available_to_claim = unlocked_amount - vault.released_amount;
        if available_to_claim <= 0 {
            panic!("No tokens available to claim");
        }
        if claim_amount > available_to_claim {
            panic!("Insufficient unlocked tokens to claim");
        }

        // Update vault
        vault.released_amount += claim_amount;
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_id), &vault);

        claim_amount
    }

    /// Transfers the beneficiary role of a vault to a new address.
    /// Only the admin can perform this action (e.g., in case of lost keys).
    pub fn transfer_beneficiary(env: Env, vault_id: u64, new_address: Address) {
        Self::require_admin(&env);

        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| panic!("Vault not found"));

        let old_owner = vault.owner.clone();

        // Update user vaults index if the vault has been initialized
        if vault.is_initialized {
            // Remove vault_id from old owner's list
            let old_vaults: Vec<u64> = env
                .storage()
                .instance()
                .get(&DataKey::UserVaults(old_owner.clone()))
                .unwrap_or(Vec::new(&env));

            let mut updated_old_vaults = Vec::new(&env);
            for id in old_vaults.iter() {
                if id != vault_id {
                    updated_old_vaults.push_back(id);
                }
            }
            env.storage()
                .instance()
                .set(&DataKey::UserVaults(old_owner.clone()), &updated_old_vaults);

            // Add vault_id to new owner's list
            let mut new_vaults: Vec<u64> = env
                .storage()
                .instance()
                .get(&DataKey::UserVaults(new_address.clone()))
                .unwrap_or(Vec::new(&env));
            new_vaults.push_back(vault_id);
            env.storage()
                .instance()
                .set(&DataKey::UserVaults(new_address.clone()), &new_vaults);
        }

        // Update vault owner
        vault.owner = new_address.clone();
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_id), &vault);

        // Emit BeneficiaryUpdated event
        env.events().publish(
            (Symbol::new(&env, "BeneficiaryUpdated"), vault_id),
            (old_owner.clone(), new_address),
        );
    }

    // Set delegate address for a vault (only owner can call)
    pub fn set_delegate(env: Env, vault_id: u64, delegate: Option<Address>) {
        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        if !vault.is_initialized {
            panic!("Vault not initialized");
        }

        // Check if caller is the vault owner
        let caller = env.current_contract_address();
        if caller != vault.owner {
            panic!("Only vault owner can set delegate");
        }

        let old_delegate = vault.delegate.clone();

        // Update delegate
        vault.delegate = delegate.clone();
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_id), &vault);

        // Emit DelegateUpdated event
        env.events().publish(
            (Symbol::new(&env, "DelegateUpdated"), vault_id),
            (old_delegate, delegate),
        );
    }

    // Claim tokens as delegate (tokens still go to owner)
    pub fn claim_as_delegate(env: Env, vault_id: u64, claim_amount: i128) -> i128 {
        let vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        if !vault.is_initialized {
            panic!("Vault not initialized");
        }
        if claim_amount <= 0 {
            panic!("Claim amount must be positive");
        }

        // Check if caller is authorized delegate
        let caller = env.current_contract_address();
        let delegate = vault.delegate.clone();
        if !(delegate.is_some() && caller == delegate.unwrap()) {
            panic!("Caller is not authorized delegate for this vault");
        }

        let milestones = Self::require_milestones_configured(&env, vault_id);
        let unlocked_pct = Self::unlocked_percentage(&milestones);
        let unlocked_amount = Self::unlocked_amount(vault.total_amount, unlocked_pct);
        let available_to_claim = unlocked_amount - vault.released_amount;
        if available_to_claim <= 0 {
            panic!("No tokens available to claim");
        }
        if claim_amount > available_to_claim {
            panic!("Insufficient unlocked tokens to claim");
        }

        // Update vault (same as regular claim)
        let mut updated_vault = vault.clone();
        updated_vault.released_amount += claim_amount;
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_id), &updated_vault);

        claim_amount // Tokens go to original owner, not delegate
    }

    pub fn set_milestones(env: Env, vault_id: u64, milestones: Vec<Milestone>) {
        Self::require_admin(&env);

        let vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| panic!("Vault not found"));
        if !vault.is_initialized {
            panic!("Vault not initialized");
        }

        if milestones.is_empty() {
            panic!("No milestones provided");
        }

        let mut total_pct: u32 = 0;
        let mut seen: Map<u64, bool> = Map::new(&env);
        for m in milestones.iter() {
            if m.percentage == 0 {
                panic!("Milestone percentage must be positive");
            }
            if m.percentage > 100 {
                panic!("Milestone percentage too large");
            }
            if seen.contains_key(m.id) {
                panic!("Duplicate milestone id");
            }
            seen.set(m.id, true);
            total_pct = total_pct.saturating_add(m.percentage);
        }
        if total_pct > 100 {
            panic!("Total milestone percentage exceeds 100");
        }

        env.storage()
            .instance()
            .set(&DataKey::VaultMilestones(vault_id), &milestones);
        env.events().publish(
            (Symbol::new(&env, "MilestonesSet"), vault_id),
            (milestones.len(), total_pct),
        );
    }

    pub fn get_milestones(env: Env, vault_id: u64) -> Vec<Milestone> {
        env.storage()
            .instance()
            .get(&DataKey::VaultMilestones(vault_id))
            .unwrap_or(Vec::new(&env))
    }

    pub fn unlock_milestone(env: Env, vault_id: u64, milestone_id: u64) {
        Self::require_admin(&env);

        let _vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| panic!("Vault not found"));

        let milestones = Self::require_milestones_configured(&env, vault_id);

        let mut found = false;
        let mut updated = Vec::new(&env);
        for m in milestones.iter() {
            if m.id == milestone_id {
                found = true;
                if m.is_unlocked {
                    panic!("Milestone already unlocked");
                }
                updated.push_back(Milestone {
                    id: m.id,
                    percentage: m.percentage,
                    is_unlocked: true,
                });
            } else {
                updated.push_back(m);
            }
        }
        if !found {
            panic!("Milestone not found");
        }

        env.storage()
            .instance()
            .set(&DataKey::VaultMilestones(vault_id), &updated);
        let timestamp = env.ledger().timestamp();
        env.events().publish(
            (Symbol::new(&env, "MilestoneUnlocked"), vault_id),
            (milestone_id, timestamp),
        );
    }

    // Batch create vaults with lazy initialization
    pub fn batch_create_vaults_lazy(env: Env, batch_data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);

        let mut vault_ids = Vec::new(&env);
        let initial_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VaultCount)
            .unwrap_or(0);

        // Check total admin balance
        let total_amount: i128 = batch_data.amounts.iter().sum();
        let mut admin_balance: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AdminBalance)
            .unwrap_or(0);
        if admin_balance < total_amount {
            panic!("Insufficient admin balance for batch");
        }
        admin_balance -= total_amount;
        env.storage()
            .instance()
            .set(&DataKey::AdminBalance, &admin_balance);

        let now = env.ledger().timestamp();
        for i in 0..batch_data.recipients.len() {
            let vault_id = initial_count + i as u64 + 1;

            // Create vault with lazy initialization
            let vault = Vault {
                owner: batch_data.recipients.get(i).unwrap(),
                delegate: None, // No delegate initially
                total_amount: batch_data.amounts.get(i).unwrap(),
                released_amount: 0,
                start_time: batch_data.start_times.get(i).unwrap(),
                end_time: batch_data.end_times.get(i).unwrap(),
                keeper_fee: batch_data.keeper_fees.get(i).unwrap(),
                is_initialized: false, // Lazy initialization
                is_irrevocable: false, // Default to revocable for batch operations
                creation_time: now,
                is_transferable: false, // Default to non-transferable for batch
                step_duration: batch_data.step_durations.get(i).unwrap_or(0),
                staked_amount: 0,
            };

            // Store vault data (minimal writes)
            env.storage()
                .instance()
                .set(&DataKey::VaultData(vault_id), &vault);
            vault_ids.push_back(vault_id);
            // Emit VaultCreated event for each created vault
            let start_time = batch_data.start_times.get(i).unwrap();
            let cliff_duration = start_time.saturating_sub(now);
            let vault_created = VaultCreated {
                vault_id,
                beneficiary: vault.owner.clone(),
                total_amount: vault.total_amount,
                cliff_duration,
                start_time,
            };
            env.events()
                .publish((Symbol::new(&env, "VaultCreated"), vault_id), vault_created);
        }

        // Update vault count once (cheaper than individual updates)
        let final_count = initial_count + batch_data.recipients.len() as u64;
        env.storage()
            .instance()
            .set(&DataKey::VaultCount, &final_count);

        vault_ids
    }

    // Batch create vaults with full initialization
    pub fn batch_create_vaults_full(env: Env, batch_data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);

        let mut vault_ids = Vec::new(&env);
        let initial_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VaultCount)
            .unwrap_or(0);

        // Check total admin balance
        let total_amount: i128 = batch_data.amounts.iter().sum();
        let mut admin_balance: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AdminBalance)
            .unwrap_or(0);
        if admin_balance < total_amount {
            panic!("Insufficient admin balance for batch");
        }
        admin_balance -= total_amount;
        env.storage()
            .instance()
            .set(&DataKey::AdminBalance, &admin_balance);

        let now = env.ledger().timestamp();
        for i in 0..batch_data.recipients.len() {
            let vault_id = initial_count + i as u64 + 1;

            // Create vault with full initialization
            let vault = Vault {
                owner: batch_data.recipients.get(i).unwrap(),
                delegate: None, // No delegate initially
                total_amount: batch_data.amounts.get(i).unwrap(),
                released_amount: 0,
                start_time: batch_data.start_times.get(i).unwrap(),
                end_time: batch_data.end_times.get(i).unwrap(),
                keeper_fee: batch_data.keeper_fees.get(i).unwrap(),
                is_initialized: true,
                is_irrevocable: false, // Default to revocable for batch operations
                creation_time: now,
                is_transferable: false, // Default to non-transferable for batch
                step_duration: batch_data.step_durations.get(i).unwrap_or(0),
                staked_amount: 0,
            };

            // Store vault data (expensive writes)
            env.storage()
                .instance()
                .set(&DataKey::VaultData(vault_id), &vault);

            // Update user vaults list for each vault (expensive)
            let mut user_vaults: Vec<u64> = env
                .storage()
                .instance()
                .get(&DataKey::UserVaults(vault.owner.clone()))
                .unwrap_or(Vec::new(&env));
            user_vaults.push_back(vault_id);
            env.storage()
                .instance()
                .set(&DataKey::UserVaults(vault.owner.clone()), &user_vaults);

            vault_ids.push_back(vault_id);
            // Emit VaultCreated event for each created vault
            let start_time = batch_data.start_times.get(i).unwrap();
            let cliff_duration = start_time.saturating_sub(now);
            let vault_created = VaultCreated {
                vault_id,
                beneficiary: vault.owner.clone(),
                total_amount: vault.total_amount,
                cliff_duration,
                start_time,
            };
            env.events()
                .publish((Symbol::new(&env, "VaultCreated"), vault_id), vault_created);
        }

        // Update vault count once
        let final_count = initial_count + batch_data.recipients.len() as u64;
        env.storage()
            .instance()
            .set(&DataKey::VaultCount, &final_count);

        vault_ids
    }

    // Get vault info (initializes if needed)
    pub fn get_vault(env: Env, vault_id: u64) -> Vault {

        // Auto-initialize if lazy
        if !vault.is_initialized {
            Self::initialize_vault_metadata(&env, vault_id);
            // Get updated vault
            env.storage()
                .instance()
                .get(&DataKey::VaultData(vault_id))
                .unwrap()
        } else {
            vault
        }
    }

    // Get user vaults (initializes all if needed)
    pub fn get_user_vaults(env: Env, user: Address) -> Vec<u64> {
        let vault_ids: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::UserVaults(user.clone()))
            .unwrap_or(Vec::new(&env));

        // Initialize all lazy vaults for this user
        for vault_id in vault_ids.iter() {

                });

            if !vault.is_initialized {
                Self::initialize_vault_metadata(&env, vault_id);
            }
        }

        vault_ids
    }

    // Revoke tokens from a vault and return them to admin
    pub fn revoke_tokens(env: Env, vault_id: u64) -> i128 {
        Self::require_admin(&env);

        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        // Security check: Cannot revoke from irrevocable vaults
        if vault.is_irrevocable {
            panic!("Vault is irrevocable");
        }

        // Calculate amount to return (unreleased tokens)
        let unreleased_amount = vault.total_amount - vault.released_amount;
        if unreleased_amount <= 0 {
            panic!("No tokens available to revoke");
        }

        // Update vault to mark all tokens as released (effectively revoking them)
        vault.released_amount = vault.total_amount;
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_id), &vault);

        // Return tokens to admin balance
        let mut admin_balance: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AdminBalance)
            .unwrap_or(0);
        admin_balance += unreleased_amount;
        env.storage()
            .instance()
            .set(&DataKey::AdminBalance, &admin_balance);

        // Get current timestamp
        let timestamp = env.ledger().timestamp();

        // Emit TokensRevoked event
        env.events().publish(
            (Symbol::new(&env, "TokensRevoked"), vault_id),
            (unreleased_amount, timestamp),
        );

        unreleased_amount
    }

    // Revoke a specific amount of tokens from a vault and return them to admin
    pub fn revoke_partial(env: Env, vault_id: u64, amount: i128) -> i128 {
        Self::require_admin(&env);

        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        // Security check: Cannot revoke from irrevocable vaults
        if vault.is_irrevocable {
            panic!("Vault is irrevocable");
        }

        // Calculate unvested balance (tokens not yet released)
        let unvested_balance = vault.total_amount - vault.released_amount;
        if amount <= 0 {
            panic!("Amount to revoke must be positive");
        }
        if amount > unvested_balance {
            panic!("Amount exceeds unvested balance");
        }

        // Update vault to increase released amount by the specified amount
        vault.released_amount += amount;
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_id), &vault);

        // Return tokens to admin balance
        let mut admin_balance: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AdminBalance)
            .unwrap_or(0);
        admin_balance += amount;
        env.storage()
            .instance()
            .set(&DataKey::AdminBalance, &admin_balance);

        // Get current timestamp
        let timestamp = env.ledger().timestamp();

        // Emit TokensRevoked event
        env.events().publish(
            (Symbol::new(&env, "TokensRevoked"), vault_id),
            (amount, timestamp),
        );

        amount
    }

    // Clawback a vault within the grace period (1 hour)
    pub fn clawback_vault(env: Env, vault_id: u64) -> i128 {
        Self::require_admin(&env);

        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        let now = env.ledger().timestamp();
        let grace_period = 3600; // 1 hour in seconds

        if now > vault.creation_time + grace_period {
            panic!("Grace period expired");
        }

        if vault.released_amount > 0 {
            panic!("Tokens already claimed");
        }

        // Refund admin
        let mut admin_balance: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AdminBalance)
            .unwrap_or(0);
        admin_balance += vault.total_amount;
        env.storage()
            .instance()
            .set(&DataKey::AdminBalance, &admin_balance);

        // Mark as released/revoked so it can't be claimed
        vault.released_amount = vault.total_amount;
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_id), &vault);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "VaultClawedBack"), vault_id),
            vault.total_amount,
        );

        vault.total_amount
    }

    // Transfer vault ownership to another beneficiary (if transferable)
    pub fn transfer_vault(env: Env, vault_id: u64, new_beneficiary: Address) {
        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        if !vault.is_initialized {
            panic!("Vault not initialized");
        }

        if !vault.is_transferable {
            panic!("Vault is non-transferable");
        }

        // Check if caller is the vault owner
        let caller = env.current_contract_address();
        if caller != vault.owner {
            panic!("Only vault owner can transfer");
        }

        let old_owner = vault.owner.clone();

        // Update UserVaults
        // Remove from old owner
        let mut old_user_vaults: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::UserVaults(old_owner.clone()))
            .unwrap_or(Vec::new(&env));
        
        let mut new_old_user_vaults = Vec::new(&env);
        for id in old_user_vaults.iter() {
            if id != vault_id {
                new_old_user_vaults.push_back(id);
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::UserVaults(old_owner.clone()), &new_old_user_vaults);

        // Add to new owner
        let mut new_user_vaults: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::UserVaults(new_beneficiary.clone()))
            .unwrap_or(Vec::new(&env));
        new_user_vaults.push_back(vault_id);
        env.storage()
            .instance()
            .set(&DataKey::UserVaults(new_beneficiary.clone()), &new_user_vaults);

        // Update vault
        vault.owner = new_beneficiary.clone();
        vault.delegate = None; // Reset delegate on transfer
        
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_id), &vault);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "BeneficiaryUpdated"), vault_id),
            (old_owner, new_beneficiary),
        );
    }

    // Rotate beneficiary key (security feature, allows self-transfer even if non-transferable)
    pub fn rotate_beneficiary_key(env: Env, vault_id: u64, new_address: Address) {
        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        if !vault.is_initialized {
            panic!("Vault not initialized");
        }

        // Require authorization from the current owner
        vault.owner.require_auth();

        let old_owner = vault.owner.clone();

        // Update UserVaults
        // Remove from old owner
        let mut old_user_vaults: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::UserVaults(old_owner.clone()))
            .unwrap_or(Vec::new(&env));
        
        let mut new_old_user_vaults = Vec::new(&env);
        for id in old_user_vaults.iter() {
            if id != vault_id {
                new_old_user_vaults.push_back(id);
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::UserVaults(old_owner.clone()), &new_old_user_vaults);

        // Add to new owner
        let mut new_user_vaults: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::UserVaults(new_address.clone()))
            .unwrap_or(Vec::new(&env));
        new_user_vaults.push_back(vault_id);
        env.storage()
            .instance()
            .set(&DataKey::UserVaults(new_address.clone()), &new_user_vaults);

        // Update vault
        vault.owner = new_address.clone();
        vault.delegate = None; // Reset delegate on rotation for security
        
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_id), &vault);

        // Emit BeneficiaryRotated event
        env.events().publish(
            (Symbol::new(&env, "BeneficiaryRotated"), vault_id),
            (old_owner, new_address),
        );
    }

    // Set the whitelisted staking contract address
    pub fn set_staking_contract(env: Env, contract: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&Symbol::new(&env, "StakingContract"), &contract);
    }

    // Stake unvested tokens to the whitelisted staking contract
    pub fn stake_tokens(env: Env, vault_id: u64, amount: i128, validator: Address) {
        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        if !vault.is_initialized {
            panic!("Vault not initialized");
        }

        // Check auth (owner or delegate?) - usually owner
        let caller = env.current_contract_address();
        if caller != vault.owner {
            panic!("Only vault owner can stake");
        }

        // Check available balance (total - released - staked)
        let available = vault.total_amount - vault.released_amount - vault.staked_amount;
        if amount <= 0 {
            panic!("Amount must be positive");
        }
        if amount > available {
            panic!("Insufficient funds to stake");
        }

        // Get staking contract
        let staking_contract: Address = env.storage().instance()
            .get(&Symbol::new(&env, "StakingContract"))
            .expect("Staking contract not set");

        // Call stake on external contract
        let args = vec![&env, vault_id.into_val(&env), amount.into_val(&env), validator.into_val(&env)];
        env.invoke_contract::<()>(&staking_contract, &Symbol::new(&env, "stake"), args);

        // Update vault state
        vault.staked_amount += amount;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    // Mark a vault as irrevocable to prevent admin withdrawal
    pub fn mark_irrevocable(env: Env, vault_id: u64) {
        Self::require_admin(&env);

        let mut vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        // Cannot mark already irrevocable vaults
        if vault.is_irrevocable {
            panic!("Vault is already irrevocable");
        }

        // Mark vault as irrevocable
        vault.is_irrevocable = true;
        env.storage()
            .instance()
            .set(&DataKey::VaultData(vault_id), &vault);

        // Emit IrrevocableMarked event
        let timestamp = env.ledger().timestamp();
        env.events().publish(
            (Symbol::new(&env, "IrrevocableMarked"), vault_id),
            timestamp,
        );
    }

    // Check if a vault is irrevocable
    pub fn is_vault_irrevocable(env: Env, vault_id: u64) -> bool {
        let vault: Vault = env
            .storage()
            .instance()
            .get(&DataKey::VaultData(vault_id))
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });

        vault.is_irrevocable
    }

    // Get contract state for invariant checking
    pub fn get_contract_state(env: Env) -> (i128, i128, i128) {
        let admin_balance: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AdminBalance)
            .unwrap_or(0);

        // Calculate total locked and claimed amounts
        let vault_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VaultCount)
            .unwrap_or(0);
        let mut total_locked = 0i128;
        let mut total_claimed = 0i128;

        for i in 1..=vault_count {
            if let Some(vault) = env
                .storage()
                .instance()
                .get::<DataKey, Vault>(&DataKey::VaultData(i))
            {
                total_locked += vault.total_amount - vault.released_amount;
                total_claimed += vault.released_amount;
            }
        }

        (total_locked, total_claimed, admin_balance)
    }

    // Check invariant: Total Locked + Total Claimed + Admin Balance = Initial Supply
    pub fn check_invariant(env: Env) -> bool {
        let initial_supply: i128 = env
            .storage()
            .instance()
            .get(&DataKey::InitialSupply)
            .unwrap_or(0);
        let (total_locked, total_claimed, admin_balance) = Self::get_contract_state(env);

        let sum = total_locked + total_claimed + admin_balance;
        sum == initial_supply
    }

    // --- New Auto-Claim Logic ---

    // Calculate currently claimable tokens based on linear vesting
    pub fn get_claimable_amount(env: Env, vault_id: u64) -> i128 {
        let vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| panic!("Vault not found"));

        let vested = Self::calculate_time_vested_amount(&env, &vault);

        if vested > vault.released_amount {
            vested - vault.released_amount
        } else {
            0
        }
    }

    // Auto-claim function that anyone can call.
    // Tokens go to beneficiary, but keeper can get a tip.
    pub fn auto_claim(env: Env, vault_id: u64, keeper: Address) {
        let mut vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| panic!("Vault not found"));

        require!(vault.is_initialized, "Vault not initialized");

        let claimable = Self::get_claimable_amount(env.clone(), vault_id);
        
        // Ensure there's enough to cover the fee and something left for beneficiary
        require!(claimable > vault.keeper_fee, "Insufficient claimable tokens to cover fee");

        let beneficiary_amount = claimable - vault.keeper_fee;
        
        // Update vault
        vault.released_amount += claimable;
        env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);

        // Update keeper fees
        let mut fees: Map<Address, i128> = env.storage().instance().get(&KEEPER_FEES).unwrap_or(Map::new(&env));
        let current_fees = fees.get(keeper.clone()).unwrap_or(0);
        fees.set(keeper.clone(), current_fees + vault.keeper_fee);
        env.storage().instance().set(&KEEPER_FEES, &fees);

        // Emit KeeperClaim event
        env.events().publish(
            (Symbol::new(&env, "KeeperClaim"), vault_id, keeper),
            (beneficiary_amount, vault.keeper_fee)
        );
    }

    // Get accumulated fees for a keeper
    pub fn get_keeper_fee(env: Env, keeper: Address) -> i128 {
        let fees: Map<Address, i128> = env.storage().instance().get(&KEEPER_FEES).unwrap_or(Map::new(&env));
        fees.get(keeper).unwrap_or(0)
    }
}
