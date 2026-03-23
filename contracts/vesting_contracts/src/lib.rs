#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, vec, Address, Env, IntoVal, Map, Symbol, Val, Vec, String};

mod factory;
pub use factory::{VestingFactory, VestingFactoryClient};

// 10 years in seconds
pub const MAX_DURATION: u64 = 315_360_000;

#[contracttype]
pub enum WhitelistDataKey {
    WhitelistedTokens,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    AdminAddress,
    AdminBalance,
    InitialSupply,
    ProposedAdmin,
    VaultCount,
    VaultData(u64),
    VaultMilestones(u64),
    UserVaults(Address),
    IsPaused,
    IsDeprecated,
    MigrationTarget,
    Token,
    TotalShares,
    TotalStaked,
    StakingContract,
    VotingDelegate(Address),
    DelegatedBeneficiaries(Address),
    GlobalAccelerationPct,
}

#[contracttype]
#[derive(Clone)]
pub struct Vault {
    pub total_amount: i128,
    pub released_amount: i128,
    pub keeper_fee: i128,
    pub staked_amount: i128,
    pub owner: Address,
    pub delegate: Option<Address>,
    pub title: String,
    pub start_time: u64,
    pub end_time: u64,
    pub creation_time: u64,
    pub step_duration: u64,
    pub is_initialized: bool,
    pub is_irrevocable: bool,
    pub is_transferable: bool,
    pub is_frozen: bool,
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
pub struct VaultCreated {
    pub vault_id: u64,
    pub beneficiary: Address,
    pub total_amount: i128,
    pub cliff_duration: u64,
    pub start_time: u64,
    pub title: String,
}

#[contract]
pub struct VestingContract;

#[contractimpl]
impl VestingContract {
    pub fn initialize(env: Env, admin: Address, initial_supply: i128) {
        if env.storage().instance().has(&DataKey::AdminAddress) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::AdminAddress, &admin);
        env.storage().instance().set(&DataKey::AdminBalance, &initial_supply);
        env.storage().instance().set(&DataKey::InitialSupply, &initial_supply);
        env.storage().instance().set(&DataKey::VaultCount, &0u64);
        env.storage().instance().set(&DataKey::IsPaused, &false);
        env.storage().instance().set(&DataKey::IsDeprecated, &false);
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

    pub fn add_to_whitelist(env: Env, token: Address) {
        Self::require_admin(&env);
        let mut whitelist: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&WhitelistDataKey::WhitelistedTokens)
            .unwrap_or(Map::new(&env));
        whitelist.set(token.clone(), true);
        env.storage().instance().set(&WhitelistDataKey::WhitelistedTokens, &whitelist);
    }

    pub fn propose_new_admin(env: Env, new_admin: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&DataKey::ProposedAdmin, &new_admin);
    }

    pub fn accept_ownership(env: Env) {
        let proposed: Address = env.storage().instance().get(&DataKey::ProposedAdmin).expect("No proposed admin");
        proposed.require_auth();
        env.storage().instance().set(&DataKey::AdminAddress, &proposed);
        env.storage().instance().remove(&DataKey::ProposedAdmin);
    }

    pub fn toggle_pause(env: Env) {
        Self::require_admin(&env);
        let paused: bool = env.storage().instance().get(&DataKey::IsPaused).unwrap_or(false);
        env.storage().instance().set(&DataKey::IsPaused, &!paused);
    }

    pub fn create_vault_full(
        env: Env, owner: Address, amount: i128, start_time: u64, end_time: u64,
        keeper_fee: i128, is_revocable: bool, is_transferable: bool, step_duration: u64,
    ) -> u64 {
        Self::require_admin(&env);
        Self::create_vault_full_internal(&env, owner, amount, start_time, end_time, keeper_fee, is_revocable, is_transferable, step_duration)
    }

    pub fn create_vault_lazy(
        env: Env, owner: Address, amount: i128, start_time: u64, end_time: u64,
        keeper_fee: i128, is_revocable: bool, is_transferable: bool, step_duration: u64,
    ) -> u64 {
        Self::require_admin(&env);
        Self::create_vault_lazy_internal(&env, owner, amount, start_time, end_time, keeper_fee, is_revocable, is_transferable, step_duration)
    }

    pub fn batch_create_vaults_lazy(env: Env, data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);
        let mut ids = Vec::new(&env);
        for i in 0..data.recipients.len() {
            let id = Self::create_vault_lazy_internal(
                &env,
                data.recipients.get(i).unwrap(),
                data.amounts.get(i).unwrap(),
                data.start_times.get(i).unwrap(),
                data.end_times.get(i).unwrap(),
                data.keeper_fees.get(i).unwrap(),
                true,
                false,
                data.step_durations.get(i).unwrap_or(0),
            );
            ids.push_back(id);
        }
        ids
    }

    pub fn batch_create_vaults_full(env: Env, data: BatchCreateData) -> Vec<u64> {
        Self::require_admin(&env);
        let mut ids = Vec::new(&env);
        for i in 0..data.recipients.len() {
            let id = Self::create_vault_full_internal(
                &env,
                data.recipients.get(i).unwrap(),
                data.amounts.get(i).unwrap(),
                data.start_times.get(i).unwrap(),
                data.end_times.get(i).unwrap(),
                data.keeper_fees.get(i).unwrap(),
                true,
                false,
                data.step_durations.get(i).unwrap_or(0),
            );
            ids.push_back(id);
        }
        ids
    }

    pub fn claim_tokens(env: Env, vault_id: u64, claim_amount: i128) -> i128 {
        Self::require_not_paused(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        if vault.is_frozen { panic!("Vault frozen"); }
        if !vault.is_initialized { panic!("Vault not initialized"); }
        vault.owner.require_auth();

        let vested = Self::calculate_claimable(&env, vault_id, &vault);
        if claim_amount > (vested - vault.released_amount) {
            panic!("Insufficient vested tokens");
        }

        vault.released_amount += claim_amount;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
        
        let token: Address = env.storage().instance().get(&DataKey::Token).expect("Token not set");
        token::Client::new(&env, &token).transfer(&env.current_contract_address(), &vault.owner, &claim_amount);
        
        claim_amount
    }

    pub fn set_milestones(env: Env, vault_id: u64, milestones: Vec<Milestone>) {
        Self::require_admin(&env);
        let mut total_pct: u32 = 0;
        for m in milestones.iter() {
            total_pct += m.percentage;
        }
        if total_pct > 100 { panic!("Total percentage > 100"); }
        env.storage().instance().set(&DataKey::VaultMilestones(vault_id), &milestones);
    }

    pub fn get_milestones(env: Env, vault_id: u64) -> Vec<Milestone> {
        env.storage().instance().get(&DataKey::VaultMilestones(vault_id)).unwrap_or(Vec::new(&env))
    }

    pub fn unlock_milestone(env: Env, vault_id: u64, milestone_id: u64) {
        Self::require_admin(&env);
        let mut milestones = Self::get_milestones(env.clone(), vault_id);
        let mut found = false;
        let mut updated = Vec::new(&env);
        for m in milestones.iter() {
            if m.id == milestone_id {
                found = true;
                updated.push_back(Milestone { id: m.id, percentage: m.percentage, is_unlocked: true });
            } else {
                updated.push_back(m);
            }
        }
        if !found { panic!("Milestone not found"); }
        env.storage().instance().set(&DataKey::VaultMilestones(vault_id), &updated);
    }

    pub fn freeze_vault(env: Env, vault_id: u64, freeze: bool) {
        Self::require_admin(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        vault.is_frozen = freeze;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    pub fn mark_irrevocable(env: Env, vault_id: u64) {
        Self::require_admin(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        vault.is_irrevocable = true;
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
    }

    pub fn get_claimable_amount(env: Env, vault_id: u64) -> i128 {
        let vault = Self::get_vault_internal(&env, vault_id);
        let vested = Self::calculate_claimable(&env, vault_id, &vault);
        vested - vault.released_amount
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&DataKey::IsPaused).unwrap_or(false)
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::AdminAddress).expect("Admin not set")
    }

    pub fn get_vault(env: Env, vault_id: u64) -> Vault {
        Self::get_vault_internal(&env, vault_id)
    }

    pub fn get_user_vaults(env: Env, user: Address) -> Vec<u64> {
        env.storage().instance().get(&DataKey::UserVaults(user)).unwrap_or(Vec::new(&env))
    }

    pub fn get_voting_power(env: Env, user: Address) -> i128 {
        // If this user has delegated their power to someone else, they have 0
        if env.storage().instance().has(&DataKey::VotingDelegate(user.clone())) {
            return 0;
        }

        let mut total_power = Self::calculate_user_own_power(&env, &user);
        
        // Add power from others who delegated to this user
        let delegators: Vec<Address> = env.storage().instance().get(&DataKey::DelegatedBeneficiaries(user)).unwrap_or(vec![&env]);
        for delegator in delegators.iter() {
            total_power += Self::calculate_user_own_power(&env, &delegator);
        }
        
        total_power
    }

    pub fn delegate_voting_power(env: Env, beneficiary: Address, representative: Address) {
        beneficiary.require_auth();
        
        // 1. Get current representative if any
        let old_representative: Option<Address> = env.storage().instance().get(&DataKey::VotingDelegate(beneficiary.clone()));
        
        // 2. If same as before, do nothing
        if let Some(ref old) = old_representative {
            if old == &representative {
                return;
            }
            
            // Remove from old representative's list
            let mut old_list: Vec<Address> = env.storage().instance().get(&DataKey::DelegatedBeneficiaries(old.clone())).unwrap_or(vec![&env]);
            if let Some(idx) = old_list.first_index_of(&beneficiary) {
                old_list.remove(idx);
                env.storage().instance().set(&DataKey::DelegatedBeneficiaries(old.clone()), &old_list);
            }
        }
        
        // 3. Update to new representative
        // If representative is beneficiary itself, it means undelegate
        if beneficiary == representative {
            env.storage().instance().remove(&DataKey::VotingDelegate(beneficiary.clone()));
        } else {
            env.storage().instance().set(&DataKey::VotingDelegate(beneficiary.clone()), &representative);
            
            // Add to new representative's list
            let mut new_list: Vec<Address> = env.storage().instance().get(&DataKey::DelegatedBeneficiaries(representative.clone())).unwrap_or(vec![&env]);
            if !new_list.contains(&beneficiary) {
                new_list.push_back(beneficiary.clone());
                env.storage().instance().set(&DataKey::DelegatedBeneficiaries(representative), &new_list);
            }
        }
    }

    pub fn accelerate_all_schedules(env: Env, percentage: u32) {
        Self::require_admin(&env);
        if percentage > 100 { panic!("Percentage must be between 0 and 100"); }
        env.storage().instance().set(&DataKey::GlobalAccelerationPct, &percentage);
    }

    pub fn slash_unvested_balance(env: Env, vault_id: u64, treasury: Address) {
        Self::require_admin(&env);
        let mut vault = Self::get_vault_internal(&env, vault_id);
        
        let vested = Self::calculate_claimable(&env, vault_id, &vault);
        let unvested = vault.total_amount - vested;
        
        if unvested <= 0 { panic!("Nothing to slash"); }
        
        // The slashed tokens are taken from the vault's total capacity
        vault.total_amount = vested;
        // Effectively stop the clock for this vault
        vault.end_time = env.ledger().timestamp();
        vault.step_duration = 0;
        
        // Reset milestones to prevent future unlocks from a reduced total
        if env.storage().instance().has(&DataKey::VaultMilestones(vault_id)) {
            env.storage().instance().remove(&DataKey::VaultMilestones(vault_id));
        }
        
        env.storage().instance().set(&DataKey::VaultData(vault_id), &vault);
        
        // Update global tracking
        let total_shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        env.storage().instance().set(&DataKey::TotalShares, &(total_shares - unvested));
        
        // Transfer to community treasury
        let token: Address = env.storage().instance().get(&DataKey::Token).expect("Token not set");
        token::Client::new(&env, &token).transfer(&env.current_contract_address(), &treasury, &unvested);
        
        // Emit event
        env.events().publish((Symbol::new(&env, "slash"), vault_id), (vested, unvested, treasury));
    }

    // --- Internal Helpers ---

    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&DataKey::AdminAddress).expect("Admin not set");
        admin.require_auth();
    }

    fn require_not_paused(env: &Env) {
        if env.storage().instance().get(&DataKey::IsPaused).unwrap_or(false) {
            panic!("Paused");
        }
    }

    fn require_valid_duration(start: u64, end: u64) {
        if end <= start { panic!("Invalid duration"); }
        if (end - start) > MAX_DURATION { panic!("duration exceeds MAX_DURATION"); }
    }

    fn create_vault_full_internal(
        env: &Env, owner: Address, amount: i128, start_time: u64, end_time: u64,
        keeper_fee: i128, is_revocable: bool, is_transferable: bool, step_duration: u64,
    ) -> u64 {
        Self::require_valid_duration(start_time, end_time);
        let id = Self::increment_vault_count(env);
        Self::sub_admin_balance(env, amount);
        let vault = Vault {
            total_amount: amount,
            released_amount: 0,
            keeper_fee,
            staked_amount: 0,
            owner: owner.clone(),
            delegate: None,
            title: String::from_str(env, ""),
            start_time,
            end_time,
            creation_time: env.ledger().timestamp(),
            step_duration,
            is_initialized: true,
            is_irrevocable: !is_revocable,
            is_transferable,
            is_frozen: false,
        };
        env.storage().instance().set(&DataKey::VaultData(id), &vault);
        Self::add_user_vault_index(env, &owner, id);
        Self::add_total_shares(env, amount);
        id
    }

    fn create_vault_lazy_internal(
        env: &Env, owner: Address, amount: i128, start_time: u64, end_time: u64,
        keeper_fee: i128, is_revocable: bool, is_transferable: bool, step_duration: u64,
    ) -> u64 {
        Self::require_valid_duration(start_time, end_time);
        let id = Self::increment_vault_count(env);
        Self::sub_admin_balance(env, amount);
        let vault = Vault {
            total_amount: amount,
            released_amount: 0,
            keeper_fee,
            staked_amount: 0,
            owner: owner.clone(),
            delegate: None,
            title: String::from_str(env, ""),
            start_time,
            end_time,
            creation_time: env.ledger().timestamp(),
            step_duration,
            is_initialized: false,
            is_irrevocable: !is_revocable,
            is_transferable,
            is_frozen: false,
        };
        env.storage().instance().set(&DataKey::VaultData(id), &vault);
        Self::add_total_shares(env, amount);
        id
    }

    fn get_vault_internal(env: &Env, id: u64) -> Vault {
        env.storage().instance().get(&DataKey::VaultData(id)).expect("Vault not found")
    }

    fn increment_vault_count(env: &Env) -> u64 {
        let count: u64 = env.storage().instance().get(&DataKey::VaultCount).unwrap_or(0);
        let new_count = count + 1;
        env.storage().instance().set(&DataKey::VaultCount, &new_count);
        new_count
    }

    fn sub_admin_balance(env: &Env, amount: i128) {
        let bal: i128 = env.storage().instance().get(&DataKey::AdminBalance).unwrap_or(0);
        if bal < amount { panic!("Insufficient admin balance"); }
        env.storage().instance().set(&DataKey::AdminBalance, &(bal - amount));
    }

    fn add_total_shares(env: &Env, amount: i128) {
        let shares: i128 = env.storage().instance().get(&DataKey::TotalShares).unwrap_or(0);
        env.storage().instance().set(&DataKey::TotalShares, &(shares + amount));
    }

    fn add_user_vault_index(env: &Env, user: &Address, id: u64) {
        let mut vaults: Vec<u64> = env.storage().instance().get(&DataKey::UserVaults(user.clone())).unwrap_or(vec![env]);
        vaults.push_back(id);
        env.storage().instance().set(&DataKey::UserVaults(user.clone()), &vaults);
    }

    fn calculate_user_own_power(env: &Env, user: &Address) -> i128 {
        let vault_ids = env.storage().instance().get(&DataKey::UserVaults(user.clone())).unwrap_or(vec![env]);
        let mut total_power: i128 = 0;
        for id in vault_ids.iter() {
            let vault = Self::get_vault_internal(env, id);
            let balance = vault.total_amount - vault.released_amount;
            let weight = if vault.is_irrevocable { 100 } else { 50 };
            total_power += (balance * weight) / 100;
        }
        total_power
    }

    fn calculate_claimable(env: &Env, id: u64, vault: &Vault) -> i128 {
        if env.storage().instance().has(&DataKey::VaultMilestones(id)) {
            let milestones: Vec<Milestone> = env.storage().instance().get(&DataKey::VaultMilestones(id)).expect("No milestones");
            let mut pct = 0;
            for m in milestones.iter() {
                if m.is_unlocked { pct += m.percentage; }
            }
            if pct > 100 { pct = 100; }
            (vault.total_amount * pct as i128) / 100
        } else {
            let mut now = env.ledger().timestamp();
            let accel_pct: u32 = env.storage().instance().get(&DataKey::GlobalAccelerationPct).unwrap_or(0);
            
            let duration = (vault.end_time - vault.start_time) as i128;
            if accel_pct > 0 {
                let shift = (duration * accel_pct as i128 / 100) as u64;
                now += shift;
            }

            if now <= vault.start_time { return 0; }
            if now >= vault.end_time { return vault.total_amount; }
            
            let elapsed = (now - vault.start_time) as i128;
            
            if vault.step_duration > 0 {
                let steps = duration / vault.step_duration as i128;
                let completed = elapsed / vault.step_duration as i128;
                (vault.total_amount / steps) * completed
            } else {
                (vault.total_amount * elapsed) / duration
            }
        }
    }
}

#[cfg(test)]
mod test;
