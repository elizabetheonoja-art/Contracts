
#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Env, Vec, Symbol, Address,
};

#[contract]
pub struct VestingContract;

const VAULT_COUNT: Symbol = symbol_short!("VCOUNT");
const VAULT_DATA: Symbol = symbol_short!("VDATA");
const USER_VAULTS: Symbol = symbol_short!("UVAULTS");
const INITIAL_SUPPLY: Symbol = symbol_short!("SUPPLY");
const ADMIN_BALANCE: Symbol = symbol_short!("ABAL");
const ADMIN_ADDRESS: Symbol = symbol_short!("ADMIN");
const PROPOSED_ADMIN: Symbol = symbol_short!("PADMIN");

#[contracttype]
pub struct Vault {
    pub owner: Address,
    pub total_amount: i128,
    pub released_amount: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub is_initialized: bool,
}

#[contracttype]
pub struct BatchCreateData {
    pub recipients: Vec<Address>,
    pub amounts: Vec<i128>,
    pub start_times: Vec<u64>,
    pub end_times: Vec<u64>,
}

#[contractimpl]
impl VestingContract {
    pub fn initialize(env: Env, admin: Address, initial_supply: i128) {
        env.storage().instance().set(&INITIAL_SUPPLY, &initial_supply);
        env.storage().instance().set(&ADMIN_BALANCE, &initial_supply);
        env.storage().instance().set(&ADMIN_ADDRESS, &admin);
        env.storage().instance().set(&VAULT_COUNT, &0u64);
    }

    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&ADMIN_ADDRESS)
            .unwrap_or_else(|| panic!("Admin not set"));
        let caller = env.current_contract_address();
        if caller != admin {
            panic!("Caller is not admin");
        }
    }

    pub fn propose_new_admin(env: Env, new_admin: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&PROPOSED_ADMIN, &new_admin);
    }

    pub fn accept_ownership(env: Env) {
        let proposed_admin: Address = env.storage().instance().get(&PROPOSED_ADMIN)
            .unwrap_or_else(|| panic!("No proposed admin found"));
        let caller = env.current_contract_address();
        if caller != proposed_admin {
            panic!("Caller is not the proposed admin");
        }
        env.storage().instance().set(&ADMIN_ADDRESS, &proposed_admin);
        env.storage().instance().remove(&PROPOSED_ADMIN);
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&ADMIN_ADDRESS)
            .unwrap_or_else(|| panic!("Admin not set"))
    }

    pub fn get_proposed_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&PROPOSED_ADMIN)
    }

    pub fn create_vault_full(env: Env, owner: Address, amount: i128, start_time: u64, end_time: u64) -> u64 {
        Self::require_admin(&env);
        let mut vault_count: u64 = env.storage().instance().get(&VAULT_COUNT).unwrap_or(0);
        vault_count += 1;
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        if admin_balance < amount {
            panic!("Insufficient admin balance");
        }
        admin_balance -= amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        let vault = Vault {
            owner: owner.clone(),
            total_amount: amount,
            released_amount: 0,
            start_time,
            end_time,
            is_initialized: true,
        };
        env.storage().instance().set(&VAULT_DATA, &vault_count, &vault);
        let mut user_vaults: Vec<u64> = env.storage().instance()
            .get(&USER_VAULTS, &owner)
            .unwrap_or(Vec::new(&env));
        user_vaults.push_back(vault_count);
        env.storage().instance().set(&USER_VAULTS, &owner, &user_vaults);
        env.storage().instance().set(&VAULT_COUNT, &vault_count);
        vault_count
    }

    pub fn create_vault_lazy(env: Env, owner: Address, amount: i128, start_time: u64, end_time: u64) -> u64 {
        Self::require_admin(&env);
        let mut vault_count: u64 = env.storage().instance().get(&VAULT_COUNT).unwrap_or(0);
        vault_count += 1;
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        if admin_balance < amount {
            panic!("Insufficient admin balance");
        }
        admin_balance -= amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        let vault = Vault {
            owner: owner.clone(),
            total_amount: amount,
            released_amount: 0,
            start_time,
            end_time,
            is_initialized: false,
        };
        env.storage().instance().set(&VAULT_DATA, &vault_count, &vault);
        env.storage().instance().set(&VAULT_COUNT, &vault_count);
        vault_count
    }

    pub fn initialize_vault_metadata(env: Env, vault_id: u64) -> bool {
        let vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| Vault {
                owner: env.current_contract_address(),
                total_amount: 0,
                released_amount: 0,
                start_time: 0,
                end_time: 0,
                is_initialized: false,
            });
        if !vault.is_initialized {
            let mut updated_vault = vault.clone();
            updated_vault.is_initialized = true;
            env.storage().instance().set(&VAULT_DATA, &vault_id, &updated_vault);
            let mut user_vaults: Vec<u64> = env.storage().instance()
                .get(&USER_VAULTS, &updated_vault.owner)
                .unwrap_or(Vec::new(&env));
            user_vaults.push_back(vault_id);
            env.storage().instance().set(&USER_VAULTS, &updated_vault.owner, &user_vaults);
            true
        } else {
            false
        }
    }

    pub fn claim_tokens(env: Env, vault_id: u64, claim_amount: i128) -> i128 {
        let mut vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| panic!("Vault not found"));
        if !vault.is_initialized {
            panic!("Vault not initialized");
        }
        if claim_amount <= 0 {
            panic!("Claim amount must be positive");
        }
        let available_to_claim = vault.total_amount - vault.released_amount;
        if claim_amount > available_to_claim {
            panic!("Insufficient tokens to claim");
        }
        vault.released_amount += claim_amount;
        env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
        claim_amount
    }

    // ── NEW: claim_all ────────────────────────────────────────────────────────
    pub fn claim_all(env: Env, vault_ids: Vec<u64>, claim_amounts: Vec<i128>) -> Vec<i128> {
        if vault_ids.len() != claim_amounts.len() {
            panic!("vault_ids and claim_amounts must be the same length");
        }
        if vault_ids.len() == 0 {
            panic!("Must provide at least one vault");
        }

        let mut results = Vec::new(&env);

        for i in 0..vault_ids.len() {
            let vault_id = vault_ids.get(i).unwrap();
            let claim_amount = claim_amounts.get(i).unwrap();

            let mut vault: Vault = env
                .storage()
                .instance()
                .get(&VAULT_DATA, &vault_id)
                .unwrap_or_else(|| panic!("Vault not found"));

            if !vault.is_initialized {
                panic!("Vault not initialized");
            }
            if claim_amount <= 0 {
                panic!("Claim amount must be positive");
            }
            let available = vault.total_amount - vault.released_amount;
            if claim_amount > available {
                panic!("Insufficient tokens in vault");
            }

            vault.released_amount += claim_amount;
            env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
            results.push_back(claim_amount);
        }

        results
    }

    pub fn batch_create_vaults_lazy(env: Env, batch_data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);
        let mut vault_ids = Vec::new(&env);
        let initial_count: u64 = env.storage().instance().get(&VAULT_COUNT).unwrap_or(0);
        let mut total_amount: i128 = 0;
        for a in batch_data.amounts.iter() {
            total_amount += a;
        }
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        if admin_balance < total_amount {
            panic!("Insufficient admin balance for batch");
        }
        admin_balance -= total_amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        for i in 0..batch_data.recipients.len() {
            let vault_id = initial_count + i as u64 + 1;
            let vault = Vault {
                owner: batch_data.recipients.get(i).unwrap(),
                total_amount: batch_data.amounts.get(i).unwrap(),
                released_amount: 0,
                start_time: batch_data.start_times.get(i).unwrap(),
                end_time: batch_data.end_times.get(i).unwrap(),
                is_initialized: false,
            };
            env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
            vault_ids.push_back(vault_id);
        }
        let final_count = initial_count + batch_data.recipients.len() as u64;
        env.storage().instance().set(&VAULT_COUNT, &final_count);
        vault_ids
    }

    pub fn batch_create_vaults_full(env: Env, batch_data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);
        let mut vault_ids = Vec::new(&env);
        let initial_count: u64 = env.storage().instance().get(&VAULT_COUNT).unwrap_or(0);
        let mut total_amount: i128 = 0;
        for a in batch_data.amounts.iter() {
            total_amount += a;
        }
        let mut admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        if admin_balance < total_amount {
            panic!("Insufficient admin balance for batch");
        }
        admin_balance -= total_amount;
        env.storage().instance().set(&ADMIN_BALANCE, &admin_balance);
        for i in 0..batch_data.recipients.len() {
            let vault_id = initial_count + i as u64 + 1;
            let vault = Vault {
                owner: batch_data.recipients.get(i).unwrap(),
                total_amount: batch_data.amounts.get(i).unwrap(),
                released_amount: 0,
                start_time: batch_data.start_times.get(i).unwrap(),
                end_time: batch_data.end_times.get(i).unwrap(),
                is_initialized: true,
            };
            env.storage().instance().set(&VAULT_DATA, &vault_id, &vault);
            let mut user_vaults: Vec<u64> = env.storage().instance()
                .get(&USER_VAULTS, &vault.owner)
                .unwrap_or(Vec::new(&env));
            user_vaults.push_back(vault_id);
            env.storage().instance().set(&USER_VAULTS, &vault.owner, &user_vaults);
            vault_ids.push_back(vault_id);
        }
        let final_count = initial_count + batch_data.recipients.len() as u64;
        env.storage().instance().set(&VAULT_COUNT, &final_count);
        vault_ids
    }

    pub fn get_vault(env: Env, vault_id: u64) -> Vault {
        let vault: Vault = env.storage().instance()
            .get(&VAULT_DATA, &vault_id)
            .unwrap_or_else(|| Vault {
                owner: env.current_contract_address(),
                total_amount: 0,
                released_amount: 0,
                start_time: 0,
                end_time: 0,
                is_initialized: false,
            });
        if !vault.is_initialized {
            Self::initialize_vault_metadata(env, vault_id);
            env.storage().instance().get(&VAULT_DATA, &vault_id).unwrap()
        } else {
            vault
        }
    }

    pub fn get_user_vaults(env: Env, user: Address) -> Vec<u64> {
        let vault_ids: Vec<u64> = env.storage().instance()
            .get(&USER_VAULTS, &user)
            .unwrap_or(Vec::new(&env));
        for vault_id in vault_ids.iter() {
            let vault: Vault = env.storage().instance()
                .get(&VAULT_DATA, vault_id)
                .unwrap_or_else(|| Vault {
                    owner: user.clone(),
                    total_amount: 0,
                    released_amount: 0,
                    start_time: 0,
                    end_time: 0,
                    is_initialized: false,
                });
            if !vault.is_initialized {
                Self::initialize_vault_metadata(env.clone(), *vault_id);
            }
        }
        vault_ids
    }

    pub fn get_contract_state(env: Env) -> (i128, i128, i128) {
        let admin_balance: i128 = env.storage().instance().get(&ADMIN_BALANCE).unwrap_or(0);
        let vault_count: u64 = env.storage().instance().get(&VAULT_COUNT).unwrap_or(0);
        let mut total_locked = 0i128;
        let mut total_claimed = 0i128;
        for i in 1..=vault_count {
            if let Some(vault) = env.storage().instance().get::<Symbol, Vault>(&VAULT_DATA) {
                total_locked += vault.total_amount - vault.released_amount;
                total_claimed += vault.released_amount;
            }
        }
        (total_locked, total_claimed, admin_balance)
    }

    pub fn check_invariant(env: Env) -> bool {
        let initial_supply: i128 = env.storage().instance().get(&INITIAL_SUPPLY).unwrap_or(0);
        let (total_locked, total_claimed, admin_balance) = Self::get_contract_state(env);
        let sum = total_locked + total_claimed + admin_balance;
        sum == initial_supply
    }
}

