
#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

// ---------------------------------------------------------------------------
// Storage key symbols
// ---------------------------------------------------------------------------
const ADMIN: Symbol        = symbol_short!("ADMIN");
const BENEFICIARY: Symbol  = symbol_short!("BENE");
const TOKEN: Symbol        = symbol_short!("TOKEN");
const TOTAL: Symbol        = symbol_short!("TOTAL");
const CLAIMED: Symbol      = symbol_short!("CLAIMED");
const START: Symbol        = symbol_short!("START");
const DURATION: Symbol     = symbol_short!("DURATION");
const CURVE: Symbol        = symbol_short!("CURVE");

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum VestingCurve {
    Linear,
    Exponential,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct VestingVault;

#[contractimpl]
impl VestingVault {
    // -----------------------------------------------------------------------
    // Initialisation
    // -----------------------------------------------------------------------

    pub fn initialize(
        env: Env,
        admin: Address,
        beneficiary: Address,
        token: Address,
        total_amount: i128,
        start: u64,
        duration: u64,
        curve: VestingCurve,
    ) {
        // Prevent re-initialisation
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialized");
        }

        assert!(total_amount > 0, "total_amount must be positive");
        assert!(duration > 0, "duration must be positive");

        admin.require_auth();

        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&BENEFICIARY, &beneficiary);
        env.storage().instance().set(&TOKEN, &token);
        env.storage().instance().set(&TOTAL, &total_amount);
        env.storage().instance().set(&CLAIMED, &0_i128);
        env.storage().instance().set(&START, &start);
        env.storage().instance().set(&DURATION, &duration);
        env.storage().instance().set(&CURVE, &curve);
    }

    // -----------------------------------------------------------------------
    // Core maths  (Issue #6 acceptance criterion 2)
    // -----------------------------------------------------------------------

    pub fn vested_amount(env: Env, now: u64) -> i128 {
        let total: i128 = env.storage().instance().get(&TOTAL).unwrap();
        let start: u64  = env.storage().instance().get(&START).unwrap();
        let duration: u64 = env.storage().instance().get(&DURATION).unwrap();
        let curve: VestingCurve = env.storage().instance().get(&CURVE).unwrap();

        Self::compute_vested(total, start, duration, now, &curve)
    }

    fn compute_vested(
        total: i128,
        start: u64,
        duration: u64,
        now: u64,
        curve: &VestingCurve,
    ) -> i128 {
        if now <= start {
            return 0;
        }

        let elapsed = now - start;

        if elapsed >= duration {
            return total; // fully vested
        }

        match curve {

            VestingCurve::Linear => {

                (total * elapsed as i128) / duration as i128
            }

            VestingCurve::Exponential => {
                let elapsed_u128  = elapsed as u128;
                let duration_u128 = duration as u128;
                let total_u128    = total as u128;

                let numerator   = total_u128 * elapsed_u128 * elapsed_u128;
                let denominator = duration_u128 * duration_u128;

                (numerator / denominator) as i128
            }
        }
    }

    // -----------------------------------------------------------------------
    // Claim
    // -----------------------------------------------------------------------

    pub fn claim(env: Env) -> i128 {
        let beneficiary: Address = env.storage().instance().get(&BENEFICIARY).unwrap();
        beneficiary.require_auth();

        let now = env.ledger().timestamp();
        let vested = Self::compute_vested(
            env.storage().instance().get(&TOTAL).unwrap(),
            env.storage().instance().get(&START).unwrap(),
            env.storage().instance().get(&DURATION).unwrap(),
            now,
            &env.storage().instance().get::<Symbol, VestingCurve>(&CURVE).unwrap(),
        );

        let claimed: i128 = env.storage().instance().get(&CLAIMED).unwrap();
        let claimable = vested - claimed;

        assert!(claimable > 0, "nothing to claim");

        // Transfer tokens from vault to beneficiary
        let token: Address = env.storage().instance().get(&TOKEN).unwrap();
        let token_client = soroban_sdk::token::Client::new(&env, &token);
        token_client.transfer(
            &env.current_contract_address(),
            &beneficiary,
            &claimable,
        );

        // Record the new claimed total
        env.storage().instance().set(&CLAIMED, &vested);

        claimable
    }

    // -----------------------------------------------------------------------
    // View helpers
    // -----------------------------------------------------------------------

    pub fn get_curve(env: Env) -> VestingCurve {
        env.storage().instance().get(&CURVE).unwrap()
    }

    pub fn status(env: Env) -> (i128, i128, i128, i128) {
        let total: i128 = env.storage().instance().get(&TOTAL).unwrap();
        let claimed: i128 = env.storage().instance().get(&CLAIMED).unwrap();
        let vested = Self::compute_vested(
            total,
            env.storage().instance().get(&START).unwrap(),
            env.storage().instance().get(&DURATION).unwrap(),
            env.ledger().timestamp(),
            &env.storage().instance().get::<Symbol, VestingCurve>(&CURVE).unwrap(),
        );
        (total, claimed, vested, vested - claimed)
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test;