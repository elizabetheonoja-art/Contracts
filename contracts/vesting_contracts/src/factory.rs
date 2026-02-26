use soroban_sdk::{
    contract, contractimpl, contractmeta, contracttype, Address, BytesN, Env, Map, Vec,
};

// Contract metadata for the factory
contractmeta!(
    key = "Description",
    val = "Factory contract for deploying vesting vault contracts"
);

#[contract]
pub struct VestingFactory;

#[contracttype]
enum DataKey {
    DeployedContracts,
    WasmHash,
}

#[contractimpl]
impl VestingFactory {
    /// Initialize the factory with the WASM hash of the vesting contract
    pub fn initialize_factory(env: Env, wasm_hash: BytesN<32>) {
        // Store the WASM hash for future deployments
        env.storage().instance().set(&DataKey::WasmHash, &wasm_hash);

        // Initialize the deployed contracts list
        let deployed_contracts: Vec<Address> = Vec::new(&env);
        env.storage()
            .instance()
            .set(&DataKey::DeployedContracts, &deployed_contracts);
    }

    /// Deploy a new vesting contract for an organization
    /// Only allows deployment if token is whitelisted
    pub fn deploy_new_vault_contract(
        env: Env,
        admin: Address,
        initial_supply: i128,
        token: Address,
    ) -> Address {
        let _wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::WasmHash)
            .unwrap_or_else(|| panic!("Factory not initialized - WASM hash not set"));

        // Check token whitelist
        let whitelist: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&crate::WhitelistDataKey::WhitelistedTokens)
            .unwrap_or(Map::new(&env));
        if !whitelist.get(token.clone()).unwrap_or(false) {
            panic!("Token not whitelisted");
        }

        let _ = (admin, initial_supply);
        panic!("Factory deployment is not implemented for this soroban-sdk version");
    }

    /// Get all deployed contract addresses
    pub fn get_deployed_contracts(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::DeployedContracts)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Get the WASM hash stored in the factory
    pub fn get_wasm_hash(env: Env) -> Option<BytesN<32>> {
        env.storage().instance().get(&DataKey::WasmHash)
    }

    /// Update the WASM hash (only callable by factory owner/admin)
    pub fn update_wasm_hash(env: Env, new_wasm_hash: BytesN<32>) {
        // In a real implementation, you'd want admin access control here
        // For now, we'll just update it directly
        env.storage()
            .instance()
            .set(&DataKey::WasmHash, &new_wasm_hash);
    }
}

#[cfg(test)]
mod test {}