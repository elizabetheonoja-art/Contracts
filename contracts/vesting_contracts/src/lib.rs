#![no_std]
use soroban_sdk::{
    contract, contractimpl, vec, Env, String, Vec, Map, Symbol, Address, 
    token, IntoVal, TryFromVal, try_from_val, ConversionError
};

mod factory;
pub use factory::{VestingFactory, VestingFactoryClient};

#[contract]
pub struct VestingContract;

// Storage keys for efficient access
const VAULT_COUNT: Symbol = Symbol::new(&"VAULT_COUNT");
const VAULT_DATA: Symbol = Symbol::new(&"VAULT_DATA");
const USER_VAULTS: Symbol = Symbol::new(&"USER_VAULTS");
const INITIAL_SUPPLY: Symbol = Symbol::new(&"INITIAL_SUPPLY");
const ADMIN_BALANCE: Symbol = Symbol::new(&"ADMIN_BALANCE");
const ADMIN_ADDRESS: Symbol = Symbol::new(&"ADMIN_ADDRESS");
const PROPOSED_ADMIN: Symbol = Symbol::new(&"PROPOSED_ADMIN");
const KEEPER_FEES: Symbol = Symbol::new(&"KEEPER_FEES");

// Vault structure with lazy initialization
#[contracttype]
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
}

#[contracttype]
pub struct BatchCreateData {
    pub recipients: Vec<Address>,
    pub amounts: Vec<i128>,
    pub start_times: Vec<u64>,
    pub end_times: Vec<u64>,
    pub keeper_fees: Vec<i128>,
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
impl VestingContract {
    // Initialize contract with initial supply
    pub fn initialize(env: Env, admin: Address, initial_supply: i128) {
        // Set initial supply
        env.storage().instance().set(&INITIAL_SUPPLY, &initial_supply);
        
        // Set admin balance (initially all tokens go to admin)
        env.storage().instance().set(&ADMIN_BALANCE, &initial_supply);
        
        // Set admin address
        env.storage().instance().set(&ADMIN_ADDRESS, &admin);
        
        // Initialize vault count
        env.storage().instance().set(&VAULT_COUNT, &0u64);
    }
    
    // Helper function to check if caller is admin
    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&ADMIN_ADDRESS)
            .unwrap_or_else(|| panic!("Admin not set"));
        let caller = env.current_contract_address();
        require!(caller == admin, "Caller is not admin");
    }
    
    // Propose a new admin (first step of two-step process)
    pub fn propose_new_admin(env: Env, new_admin: Address) {
        Self::require_admin(&env);
        
        // Store the proposed admin
        env.storage().instance().set(&PROPOSED_ADMIN, &new_admin);
    }
    
    // Accept admin ownership (second step of two-step process)
    pub fn accept_ownership(env: Env) {
        let proposed_admin: Address = env.storage().instance().get(&PROPOSED_ADMIN)
            .unwrap_or_else(|| panic!("No proposed admin found"));
        
        let caller = env.current_contract_address();
        require!(caller == proposed_admin, "Caller is not the proposed admin");
        
        // Transfer admin rights
        env.storage().instance().set(&ADMIN_ADDRESS, &proposed_admin);
        
        // Clear the proposed admin
        env.storage().instance().remove(&PROPOSED_ADMIN);
    }
    
    // Get current admin address
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&ADMIN_ADDRESS)
            .unwrap_or_else(|| panic!("Admin not set"))
    }
    
    // Get proposed admin address (if any)
    pub fn get_proposed_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&PROPOSED_ADMIN)
    }
    
    // Full initialization - writes all metadata immediately

        Self::require_admin(&env);
        
        // Get next vault ID
        let mut vault_count: u64 = env.storage().instance().get(&VAULT_COUNT).unwrap_or(0);
        vault_count += 1;
        
        // Check admin balance and transfer tokens
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        require!(admin_balance >= amount, "Insufficient admin balance");
        admin_balance -= amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        
        // Create vault with full initialization
        let vault = Vault {
            owner: owner.clone(),
            delegate: None, // No delegate initially
            total_amount: amount,
            released_amount: 0,
            start_time,
            end_time,
            keeper_fee,
            is_initialized: true, // Mark as fully initialized
            is_irrevocable: !is_revocable, // Convert from is_revocable parameter
        };
        
        // Store vault data immediately (expensive gas usage)
        env.storage().instance().set(&VAULT_DATA, &vault_count, &vault);
        
        // Update user vaults list
        let mut user_vaults: Vec<u64> = env.storage().instance()
            .get(&USER_VAULTS, &owner)
            .unwrap_or(Vec::new(&env));
        user_vaults.push_back(vault_count);
        env.storage().instance().set(&USER_VAULTS, &owner, &user_vaults);
        
        // Update vault count
        env.storage().instance().set(&VAULT_COUNT, &vault_count);

        // Emit VaultCreated event with strictly typed fields
        let now = env.ledger().timestamp();
        let cliff_duration = if start_time > now { start_time - now } else { 0 };
        let vault_created = VaultCreated {
            vault_id: vault_count,
            beneficiary: owner.clone(),
            total_amount: amount,
            cliff_duration,
            start_time,
        };
        env.events().publish((Symbol::new(&env, "VaultCreated"), vault_count), vault_created);

        vault_count
    }
    
    // Lazy initialization - writes minimal data initially
   Self::require_admin(&env);
        
        // Get next vault ID
        let mut vault_count: u64 = env.storage().instance().get(&VAULT_COUNT).unwrap_or(0);
        vault_count += 1;
        
        // Check admin balance and transfer tokens
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        require!(admin_balance >= amount, "Insufficient admin balance");
        admin_balance -= amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        
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
        };
        
        // Store only essential data initially (cheaper gas)
        env.storage().instance().set(&VAULT_DATA, &vault_count, &vault);
        
        // Update vault count
        env.storage().instance().set(&VAULT_COUNT, &vault_count);
        
        // Don't update user vaults list yet (lazy)
        

        // Emit VaultCreated event with strictly typed fields
        let now = env.ledger().timestamp();
        let cliff_duration = if start_time > now { start_time - now } else { 0 };
        let vault_created = VaultCreated {
            vault_id: vault_count,
            beneficiary: owner.clone(),
            total_amount: amount,
            cliff_duration,
            start_time,
        };
        env.events().publish((Symbol::new(&env, "VaultCreated"), vault_count), vault_created);

        vault_count
    }
    
    // Initialize vault metadata when needed (on-demand)
    pub fn initialize_vault_metadata(env: Env, vault_id: u64) -> bool {
        let vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| {
                // Return empty vault if not found
                Vault {
                    owner: Address::from_contract_id(&env.current_contract_address()),
                    delegate: None,
                    total_amount: 0,
                    released_amount: 0,
                    start_time: 0,
                    end_time: 0,
                    keeper_fee: 0,
                    is_initialized: false,
                    is_irrevocable: false,
                }
            });
        
        // Only initialize if not already initialized
        if !vault.is_initialized {
            let mut updated_vault = vault.clone();
            updated_vault.is_initialized = true;
            
            // Store updated vault with full metadata
            env.storage().instance().set(&VAULT_DATA, &vault_id, &updated_vault);
            
            // Update user vaults list (deferred)
            let mut user_vaults: Vec<u64> = env.storage().instance()
                .get(&USER_VAULTS, &updated_vault.owner)
                .unwrap_or(Vec::new(&env));
            user_vaults.push_back(vault_id);
            env.storage().instance().set(&USER_VAULTS, &updated_vault.owner, &user_vaults);
            
            true
        } else {
            false // Already initialized
        }
    }
    
    // Helper function to calculate vested amount based on current time
    fn calculate_vested_amount(env: &Env, vault: &Vault) -> i128 {
        let now = env.ledger().timestamp();
        
        // If before start time, nothing is vested
        if now <= vault.start_time {
            return 0i128;
        }
        
        // If after end time, everything is vested
        if now >= vault.end_time {
            return vault.total_amount;
        }
        
        // Calculate linear vesting between start and end time
        let total_duration = vault.end_time - vault.start_time;
        let elapsed = now - vault.start_time;
        
        // Use i128 for calculation to avoid overflow
        let total_amount = vault.total_amount;
        let elapsed_i128 = elapsed as i128;
        let total_duration_i128 = total_duration as i128;
        
        // Linear interpolation: vested = total * elapsed / total_duration
        (total_amount * elapsed_i128) / total_duration_i128
    }
    
    // Claim tokens from vault
    pub fn claim_tokens(env: Env, vault_id: u64, claim_amount: i128) -> i128 {
        let mut vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });
        
        require!(vault.is_initialized, "Vault not initialized");
        require!(claim_amount > 0, "Claim amount must be positive");
        
        let available_to_claim = vault.total_amount - vault.released_amount;
        require!(claim_amount <= available_to_claim, "Insufficient tokens to claim");
        
        // Update vault
        vault.released_amount += claim_amount;
        env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
        
        claim_amount
    }

    /// Transfers the beneficiary role of a vault to a new address.
    /// Only the admin can perform this action (e.g., in case of lost keys).
    pub fn transfer_beneficiary(env: Env, vault_id: u64, new_address: Address) {
        Self::require_admin(&env);

        let mut vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| panic!("Vault not found"));

        let old_owner = vault.owner.clone();

        // Update user vaults index if the vault has been initialized
        if vault.is_initialized {
            // Remove vault_id from old owner's list
            let old_vaults: Vec<u64> = env.storage().instance()
                .get(&USER_VAULTS, &old_owner)
                .unwrap_or(Vec::new(&env));
            
            let mut updated_old_vaults = Vec::new(&env);
            for id in old_vaults.iter() {
                if id != vault_id {
                    updated_old_vaults.push_back(id);
                }
            }
            env.storage().instance().set(&USER_VAULTS, &old_owner, &updated_old_vaults);

            // Add vault_id to new owner's list
            let mut new_vaults: Vec<u64> = env.storage().instance()
                .get(&USER_VAULTS, &new_address)
                .unwrap_or(Vec::new(&env));
            new_vaults.push_back(vault_id);
            env.storage().instance().set(&USER_VAULTS, &new_address, &new_vaults);
        }

        // Update vault owner
        vault.owner = new_address.clone();
        env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);

        // Emit BeneficiaryChanged event
        env.events().publish(
            (Symbol::new(&env, "BeneficiaryChanged"), vault_id),
            (old_owner, new_address)
        );
    }
    
    // Set delegate address for a vault (only owner can call)
    pub fn set_delegate(env: Env, vault_id: u64, delegate: Option<Address>) {
        let mut vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });
        
        require!(vault.is_initialized, "Vault not initialized");
        
        // Check if caller is the vault owner
        let caller = env.current_contract_address();
        require!(caller == vault.owner, "Only vault owner can set delegate");
        
        // Update delegate
        vault.delegate = delegate;
        env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
    }
    
    // Claim tokens as delegate (tokens still go to owner)
    pub fn claim_as_delegate(env: Env, vault_id: u64, claim_amount: i128) -> i128 {
        let vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });
        
        require!(vault.is_initialized, "Vault not initialized");
        require!(claim_amount > 0, "Claim amount must be positive");
        
        // Check if caller is authorized delegate
        let caller = env.current_contract_address();
        require!(vault.delegate.is_some() && caller == vault.delegate.unwrap(), 
                "Caller is not authorized delegate for this vault");
        
        let available_to_claim = vault.total_amount - vault.released_amount;
        require!(claim_amount <= available_to_claim, "Insufficient tokens to claim");
        
        // Update vault (same as regular claim)
        let mut updated_vault = vault.clone();
        updated_vault.released_amount += claim_amount;
        env.storage().instance().set(&VAULT_DATA, &vault_id, &updated_vault);
        
        claim_amount // Tokens go to original owner, not delegate
    }
    
    // Batch create vaults with lazy initialization
    pub fn batch_create_vaults_lazy(env: Env, batch_data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);
        
        let mut vault_ids = Vec::new(&env);
        let initial_count: u64 = env.storage().instance().get(&VAULT_COUNT).unwrap_or(0);
        
        // Check total admin balance
        let total_amount: i128 = batch_data.amounts.iter().sum();
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        require!(admin_balance >= total_amount, "Insufficient admin balance for batch");
        admin_balance -= total_amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        
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
            };
            
            // Store vault data (minimal writes)
            env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
            vault_ids.push_back(vault_id);
            // Emit VaultCreated event for each created vault
            let now = env.ledger().timestamp();
            let start_time = batch_data.start_times.get(i).unwrap();
            let cliff_duration = if start_time > now { start_time - now } else { 0 };
            let vault_created = VaultCreated {
                vault_id,
                beneficiary: vault.owner.clone(),
                total_amount: vault.total_amount,
                cliff_duration,
                start_time,
            };
            env.events().publish((Symbol::new(&env, "VaultCreated"), vault_id), vault_created);
        }
        
        // Update vault count once (cheaper than individual updates)
        let final_count = initial_count + batch_data.recipients.len() as u64;
        env.storage().instance().set(&VAULT_COUNT, &final_count);
        
        vault_ids
    }
    
    // Batch create vaults with full initialization
    pub fn batch_create_vaults_full(env: Env, batch_data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);
        
        let mut vault_ids = Vec::new(&env);
        let initial_count: u64 = env.storage().instance().get(&VAULT_COUNT).unwrap_or(0);
        
        // Check total admin balance
        let total_amount: i128 = batch_data.amounts.iter().sum();
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        require!(admin_balance >= total_amount, "Insufficient admin balance for batch");
        admin_balance -= total_amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        
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
                is_initialized: true, // Full initialization
                is_irrevocable: false, // Default to revocable for batch operations
            };
            
            // Store vault data (expensive writes)
            env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
            
            // Update user vaults list for each vault (expensive)
            let mut user_vaults: Vec<u64> = env.storage().instance()
                .get(&USER_VAULTS, &vault.owner)
                .unwrap_or(Vec::new(&env));
            user_vaults.push_back(vault_id);
            env.storage().instance().set(&USER_VAULTS, &vault.owner, &user_vaults);
            
            vault_ids.push_back(vault_id);
            // Emit VaultCreated event for each created vault
            let now = env.ledger().timestamp();
            let start_time = batch_data.start_times.get(i).unwrap();
            let cliff_duration = if start_time > now { start_time - now } else { 0 };
            let vault_created = VaultCreated {
                vault_id,
                beneficiary: vault.owner.clone(),
                total_amount: vault.total_amount,
                cliff_duration,
                start_time,
            };
            env.events().publish((Symbol::new(&env, "VaultCreated"), vault_id), vault_created);
        }
        
        // Update vault count once
        let final_count = initial_count + batch_data.recipients.len() as u64;
        env.storage().instance().set(&VAULT_COUNT, &final_count);
        
        vault_ids
    }
    
    // Get vault info (initializes if needed)
    pub fn get_vault(env: Env, vault_id: u64) -> Vault {
        let vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| {
                Vault {
                    owner: Address::from_contract_id(&env.current_contract_address()),
                    delegate: None,
                    total_amount: 0,
                    released_amount: 0,
                    start_time: 0,
                    end_time: 0,
                    keeper_fee: 0,
                    is_initialized: false,
                    is_irrevocable: false,
                }
            });
        
        // Auto-initialize if lazy
        if !vault.is_initialized {
            Self::initialize_vault_metadata(env, vault_id);
            // Get updated vault
            env.storage().instance().get(&VAULT_DATA, &vault_id).unwrap()
        } else {
            vault
        }
    }
    
    // Get user vaults (initializes all if needed)
    pub fn get_user_vaults(env: Env, user: Address) -> Vec<u64> {
        let vault_ids: Vec<u64> = env.storage().instance()
            .get(&USER_VAULTS, &user)
            .unwrap_or(Vec::new(&env));
        
        // Initialize all lazy vaults for this user
        for vault_id in vault_ids.iter() {
            let vault: Vault = env.storage().instance()
                .get(&VAULT_DATA, vault_id)
                .unwrap_or_else(|| {
                    Vault {
                        owner: user.clone(),
                        delegate: None,
                        total_amount: 0,
                        released_amount: 0,
                        start_time: 0,
                        end_time: 0,
                        keeper_fee: 0,
                        is_initialized: false,
                        is_irrevocable: false,
                    }
                });
            
            if !vault.is_initialized {
                Self::initialize_vault_metadata(env, *vault_id);
            }
        }
        
        vault_ids
    }
    
    // Revoke a vault - returns vested tokens to beneficiary, unvested to admin
    // Can only be called by admin and only for revocable vaults
    pub fn revoke(env: Env, vault_id: u64) -> (i128, i128) {
        Self::require_admin(&env);
        
        let mut vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });
        
        // Check if vault is revocable (not irrevocable)
        require!(!vault.is_irrevocable, "Vault is not revocable");
        
        // Calculate vested and unvested amounts
        let vested_amount = Self::calculate_vested_amount(&env, &vault);
        let unvested_amount = vault.total_amount - vested_amount;
        
        // Ensure we're not trying to revoke already claimed tokens
        let remaining_vested = if vested_amount > vault.released_amount {
            vested_amount - vault.released_amount
        } else {
            0i128
        };
        
        require!(unvested_amount > 0 || remaining_vested > 0, "No tokens to revoke");
        
        // Update vault: mark all tokens as released
        // This effectively makes the beneficiary able to claim the vested portion
        // and returns unvested to admin
        vault.released_amount = vault.total_amount;
        env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
        
        // Return unvested tokens to admin
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        admin_balance += unvested_amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        
        // Get current timestamp
        let timestamp = env.ledger().timestamp();
        
        // Emit TokensRevoked event with detailed info
        let revoked_event = TokensRevoked {
            vault_id,
            vested_amount,
            unvested_amount,
            beneficiary: vault.owner.clone(),
            timestamp,
        };
        env.events().publish(
            (Symbol::new(&env, "TokensRevoked"), vault_id),
            revoked_event,
        );
        
        (vested_amount, unvested_amount)
    }
    
    // Revoke tokens from a vault and return them to admin
    pub fn revoke_tokens(env: Env, vault_id: u64) -> i128 {
        Self::require_admin(&env);
        
        let mut vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });
        
        // Security check: Cannot revoke from irrevocable vaults
        require!(!vault.is_irrevocable, "Vault is irrevocable");
        
        // Calculate amount to return (unreleased tokens)
        let unreleased_amount = vault.total_amount - vault.released_amount;
        require!(unreleased_amount > 0, "No tokens available to revoke");
        
        // Update vault to mark all tokens as released (effectively revoking them)
        vault.released_amount = vault.total_amount;
        env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
        
        // Return tokens to admin balance
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        admin_balance += unreleased_amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        
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
        
        let mut vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });
        
        // Security check: Cannot revoke from irrevocable vaults
        require!(!vault.is_irrevocable, "Vault is irrevocable");
        
        // Calculate unvested balance (tokens not yet released)
        let unvested_balance = vault.total_amount - vault.released_amount;
        require!(amount > 0, "Amount to revoke must be positive");
        require!(amount <= unvested_balance, "Amount exceeds unvested balance");
        
        // Update vault to increase released amount by the specified amount
        vault.released_amount += amount;
        env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
        
        // Return tokens to admin balance
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        admin_balance += amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        
        // Get current timestamp
        let timestamp = env.ledger().timestamp();
        
        // Emit TokensRevoked event
        env.events().publish(
            (Symbol::new(&env, "TokensRevoked"), vault_id),
            (amount, timestamp),
        );
        
        amount
    }
    
    // Mark a vault as irrevocable to prevent admin withdrawal
    pub fn mark_irrevocable(env: Env, vault_id: u64) {
        Self::require_admin(&env);
        
        let mut vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });
        
        // Cannot mark already irrevocable vaults
        require!(!vault.is_irrevocable, "Vault is already irrevocable");
        
        // Mark vault as irrevocable
        vault.is_irrevocable = true;
        env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
        
        // Emit IrrevocableMarked event
        let timestamp = env.ledger().timestamp();
        env.events().publish(
            (Symbol::new(&env, "IrrevocableMarked"), vault_id),
            (timestamp),
        );
    }
    
    // Check if a vault is irrevocable
    pub fn is_vault_irrevocable(env: Env, vault_id: u64) -> bool {
        let vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| {
                panic!("Vault not found");
            });
        
        vault.is_irrevocable
    }
    
    // Get contract state for invariant checking
    pub fn get_contract_state(env: Env) -> (i128, i128, i128) {
        let initial_supply: i128 = env.storage().instance().get(&INITIAL_SUPPLY).unwrap_or(0);
        let admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        
        // Calculate total locked and claimed amounts
        let vault_count: u64 = env.storage().instance().get(&VAULT_COUNT).unwrap_or(0);
        let mut total_locked = 0i128;
        let mut total_claimed = 0i128;
        
        for i in 1..=vault_count {
            if let Some(vault) = env.storage().instance().get::<_, Vault>(&VAULT_DATA, &i) {
                total_locked += vault.total_amount - vault.released_amount;
                total_claimed += vault.released_amount;
            }
        }
        
        (total_locked, total_claimed, admin_balance)
    }
    
    // Check invariant: Total Locked + Total Claimed + Admin Balance = Initial Supply
    pub fn check_invariant(env: Env) -> bool {
        let initial_supply: i128 = env.storage().instance().get(&INITIAL_SUPPLY).unwrap_or(0);
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

        let now = env.ledger().timestamp();
        
        if now <= vault.start_time {
            return 0;
        }

        let elapsed = if now >= vault.end_time {
            vault.end_time - vault.start_time
        } else {
            now - vault.start_time
        };

        let total_duration = vault.end_time - vault.start_time;
        
        let vested = if total_duration > 0 {
            // Use i128 for calculation to prevent overflow then back to i128
            (vault.total_amount * elapsed as i128) / total_duration as i128
        } else {
            vault.total_amount
        };

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
