#![no_std]
use soroban_sdk::{
    contract, contractimpl, contractmeta, Address, BytesN, Env, Vec, Symbol
};
use crate::{VestingContract, Vault};

// Contract metadata for the factory
contractmeta!(
    key = "Description",
    val = "Factory contract for deploying vesting vault contracts"
);

#[contract]
pub struct VestingFactory;

// Storage keys for factory
const DEPLOYED_CONTRACTS: Symbol = Symbol::new(&"DEPLOYED_CONTRACTS");
const Wasm_HASH: Symbol = Symbol::new(&"WASM_HASH");

#[contractimpl]
impl VestingFactory {
    /// Initialize the factory with the WASM hash of the vesting contract
    pub fn initialize(env: Env, wasm_hash: BytesN<32>) {
        // Store the WASM hash for future deployments
        env.storage().instance().set(&Wasm_HASH, &wasm_hash);
        
        // Initialize the deployed contracts list
        let deployed_contracts: Vec<Address> = Vec::new(&env);
        env.storage().instance().set(&DEPLOYED_CONTRACTS, &deployed_contracts);
    }
    
    /// Deploy a new vesting contract for an organization
    pub fn deploy_new_vault_contract(env: Env, admin: Address, initial_supply: i128) -> Address {
        // Get the stored WASM hash
        let wasm_hash: BytesN<32> = env.storage().instance()
            .get(&Wasm_HASH)
            .unwrap_or_else(|| panic!("Factory not initialized - WASM hash not set"));
        
        // Generate a unique salt based on admin address and current timestamp
        let salt = env.crypto().sha256(&admin.to_bytes());
        let timestamp_bytes = env.ledger().timestamp().to_be_bytes();
        let mut salt_bytes = salt.to_array();
        for i in 0..8 {
            salt_bytes[i] ^= timestamp_bytes[i];
        }
        let unique_salt = BytesN::from_array(&env, &salt_bytes);
        
        // Deploy the new contract using the factory pattern
        let deployed_address = env.deployer()
            .with_current_contract_salt()
            .deploy(wasm_hash);
        
        // Initialize the newly deployed contract
        let client = VestingContract::new(&env, &deployed_address);
        client.initialize(&admin, &initial_supply);
        
        // Store the deployed contract address
        let mut deployed_contracts: Vec<Address> = env.storage().instance()
            .get(&DEPLOYED_CONTRACTS)
            .unwrap_or_else(|| Vec::new(&env));
        deployed_contracts.push_back(deployed_address.clone());
        env.storage().instance().set(&DEPLOYED_CONTRACTS, &deployed_contracts);
        
        deployed_address
    }
    
    /// Get all deployed contract addresses
    pub fn get_deployed_contracts(env: Env) -> Vec<Address> {
        env.storage().instance()
            .get(&DEPLOYED_CONTRACTS)
            .unwrap_or_else(|| Vec::new(&env))
    }
    
    /// Get the WASM hash stored in the factory
    pub fn get_wasm_hash(env: Env) -> Option<BytesN<32>> {
        env.storage().instance().get(&Wasm_HASH)
    }
    
    /// Update the WASM hash (only callable by factory owner/admin)
    pub fn update_wasm_hash(env: Env, new_wasm_hash: BytesN<32>) {
        // In a real implementation, you'd want admin access control here
        // For now, we'll just update it directly
        env.storage().instance().set(&Wasm_HASH, &new_wasm_hash);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{vec, Env, Address, BytesN};

    #[test]
    fn test_factory_deployment() {
        let env = Env::default();
        let factory_id = env.register(VestingFactory, ());
        let factory_client = VestingFactoryClient::new(&env, &factory_id);
        
        // Create a mock WASM hash
        let wasm_hash = BytesN::from_array(&env, &[0u8; 32]);
        
        // Initialize the factory
        factory_client.initialize(&wasm_hash);
        
        // Verify WASM hash is stored
        let stored_hash = factory_client.get_wasm_hash();
        assert_eq!(stored_hash, Some(wasm_hash));
        
        // Create admin address
        let admin = Address::generate(&env);
        let initial_supply = 1000000i128;
        
        // Deploy a new vault contract
        let deployed_contract = factory_client.deploy_new_vault_contract(&admin, &initial_supply);
        
        // Verify the contract was deployed and stored
        let deployed_contracts = factory_client.get_deployed_contracts();
        assert_eq!(deployed_contracts.len(), 1);
        assert_eq!(deployed_contracts.get(0), deployed_contract);
        
        // Test that we can deploy multiple contracts
        let admin2 = Address::generate(&env);
        let deployed_contract2 = factory_client.deploy_new_vault_contract(&admin2, &initial_supply);
        
        let deployed_contracts = factory_client.get_deployed_contracts();
        assert_eq!(deployed_contracts.len(), 2);
        assert_eq!(deployed_contracts.get(1), deployed_contract2);
    }
    
    #[test]
    fn test_factory_without_initialization() {
        let env = Env::default();
        let factory_id = env.register(VestingFactory, ());
        let factory_client = VestingFactoryClient::new(&env, &factory_id);
        
        let admin = Address::generate(&env);
        let initial_supply = 1000000i128;
        
        // This should fail because factory is not initialized
        let result = std::panic::catch_unwind(|| {
            factory_client.deploy_new_vault_contract(&admin, &initial_supply);
        });
        assert!(result.is_err());
    }
}