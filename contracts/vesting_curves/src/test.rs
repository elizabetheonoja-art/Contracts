
#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env,
};

use crate::{VestingCurve, VestingVaultClient};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TOTAL: i128 = 1_000_000_000_i128;
const START: u64  = 1_000_000_u64;
const DURATION: u64 = 1_000_u64;

struct Setup {
    env: Env,
    vault: VestingVaultClient<'static>,
    token: Address,
    admin: Address,
    beneficiary: Address,
}

fn create_setup(curve: VestingCurve) -> Setup {
    let env = Env::default();
    env.mock_all_auths();

    // Create a native/SAC token
    let admin       = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token    = token_id.address();

    StellarAssetClient::new(&env, &token).mint(&admin, &TOTAL);

    // Register the vault contract
    let vault_id = env.register(crate::VestingVault, ());
    let vault    = VestingVaultClient::new(&env, &vault_id);

    // Transfer tokens from admin to the vault contract
    TokenClient::new(&env, &token).transfer(&admin, &vault_id, &TOTAL);

    // Set ledger time to START so initialization is clean
    env.ledger().with_mut(|l| l.timestamp = START);

    vault.initialize(
        &admin,
        &beneficiary,
        &token,
        &TOTAL,
        &START,
        &DURATION,
        &curve,
    );

    Setup { env, vault, token, admin, beneficiary }
}

// ---------------------------------------------------------------------------
// Pure maths tests (no Env storage needed – test compute_vested directly)
// ---------------------------------------------------------------------------

fn vested_at(env: &Env, vault: &VestingVaultClient, ts: u64) -> i128 {
    env.ledger().with_mut(|l| l.timestamp = ts);
    vault.vested_amount(&ts)
}

// ── Linear ──────────────────────────────────────────────────────────────────

#[test]
fn l1_linear_at_start_is_zero() {
    let s = create_setup(VestingCurve::Linear);
    assert_eq!(vested_at(&s.env, &s.vault, START), 0);
}

#[test]
fn l2_linear_at_half_is_fifty_percent() {
    let s = create_setup(VestingCurve::Linear);
    let expected = TOTAL / 2;
    let actual   = vested_at(&s.env, &s.vault, START + DURATION / 2);
    assert_eq!(actual, expected, "linear 50% failed: got {actual}");
}

#[test]
fn l3_linear_at_end_is_full() {
    let s = create_setup(VestingCurve::Linear);
    assert_eq!(vested_at(&s.env, &s.vault, START + DURATION), TOTAL);
}

#[test]
fn l4_linear_after_end_capped_at_full() {
    let s = create_setup(VestingCurve::Linear);
    assert_eq!(vested_at(&s.env, &s.vault, START + DURATION + 9999), TOTAL);
}

// ── Exponential ─────────────────────────────────────────────────────────────

#[test]
fn e1_expo_at_start_is_zero() {
    let s = create_setup(VestingCurve::Exponential);
    assert_eq!(vested_at(&s.env, &s.vault, START), 0);
}

#[test]
fn e2_expo_at_quarter_is_6_25_percent() {
    let s = create_setup(VestingCurve::Exponential);
    // elapsed=250, duration=1000 → (250/1000)^2 = 0.0625 → 62_500_000
    let elapsed   = DURATION / 4;
    let expected  = TOTAL * (elapsed as i128 * elapsed as i128)
                         / (DURATION as i128 * DURATION as i128);
    let actual    = vested_at(&s.env, &s.vault, START + elapsed);
    assert_eq!(actual, expected, "expo 25% elapsed failed: got {actual}");
}

#[test]
fn e3_expo_at_half_is_twenty_five_percent() {
    let s = create_setup(VestingCurve::Exponential);
    let expected = TOTAL / 4; // 0.5^2 = 0.25
    let actual   = vested_at(&s.env, &s.vault, START + DURATION / 2);
    assert_eq!(actual, expected, "expo 50% elapsed failed: got {actual}");
}

#[test]
fn e4_expo_at_three_quarters_is_56_25_percent() {
    let s = create_setup(VestingCurve::Exponential);
    let elapsed  = (DURATION * 3) / 4;
    let expected = TOTAL * (elapsed as i128 * elapsed as i128)
                         / (DURATION as i128 * DURATION as i128);
    let actual   = vested_at(&s.env, &s.vault, START + elapsed);
    assert_eq!(actual, expected, "expo 75% elapsed failed: got {actual}");
}

#[test]
fn e5_expo_at_end_is_full() {
    let s = create_setup(VestingCurve::Exponential);
    assert_eq!(vested_at(&s.env, &s.vault, START + DURATION), TOTAL);
}

#[test]
fn e6_expo_after_end_capped_at_full() {
    let s = create_setup(VestingCurve::Exponential);
    assert_eq!(vested_at(&s.env, &s.vault, START + DURATION + 5000), TOTAL);
}

// ── Comparison ──────────────────────────────────────────────────────────────

#[test]
fn c1_at_midpoint_exponential_less_than_linear() {
    let sl = create_setup(VestingCurve::Linear);
    let se = create_setup(VestingCurve::Exponential);
    let mid = START + DURATION / 2;

    let linear_mid = vested_at(&sl.env, &sl.vault, mid);
    let expo_mid   = vested_at(&se.env, &se.vault, mid);

    assert!(
        expo_mid < linear_mid,
        "Expected expo ({expo_mid}) < linear ({linear_mid}) at midpoint"
    );
}

// ── Integration tests ────────────────────────────────────────────────────────

#[test]
fn i1_linear_claim_at_halfway() {
    let s = create_setup(VestingCurve::Linear);

    // Advance ledger to 50 % of vesting period
    s.env.ledger().with_mut(|l| l.timestamp = START + DURATION / 2);

    let claimed = s.vault.claim();
    assert_eq!(claimed, TOTAL / 2, "linear claim at 50%: got {claimed}");

    // Beneficiary balance should match
    let bal = TokenClient::new(&s.env, &s.token).balance(&s.beneficiary);
    assert_eq!(bal, TOTAL / 2);
}

#[test]
fn i2_exponential_claim_at_three_quarters() {
    let s = create_setup(VestingCurve::Exponential);

    let elapsed  = (DURATION * 3) / 4;
    let expected = TOTAL * (elapsed as i128 * elapsed as i128)
                         / (DURATION as i128 * DURATION as i128);

    s.env.ledger().with_mut(|l| l.timestamp = START + elapsed);

    let claimed = s.vault.claim();
    assert_eq!(claimed, expected, "expo claim at 75%: got {claimed}");

    let bal = TokenClient::new(&s.env, &s.token).balance(&s.beneficiary);
    assert_eq!(bal, expected);
}

#[test]
fn i3_get_curve_returns_correct_variant() {
    let sl = create_setup(VestingCurve::Linear);
    let se = create_setup(VestingCurve::Exponential);

    assert_eq!(sl.vault.get_curve(), VestingCurve::Linear);
    assert_eq!(se.vault.get_curve(), VestingCurve::Exponential);
}

#[test]
#[should_panic(expected = "nothing to claim")]
fn i4_claim_before_any_vesting_panics() {
    let s = create_setup(VestingCurve::Linear);
    // Ledger is at START – nothing vested yet
    s.vault.claim();
}

#[test]
fn i5_status_helper_is_consistent() {
    let s = create_setup(VestingCurve::Linear);

    s.env.ledger().with_mut(|l| l.timestamp = START + DURATION / 4);
    let (total, claimed, vested, claimable) = s.vault.status();

    assert_eq!(total, TOTAL);
    assert_eq!(claimed, 0);
    assert_eq!(vested, TOTAL / 4);
    assert_eq!(claimable, TOTAL / 4);

    // Now claim and re-check
    s.vault.claim();
    let (_, claimed2, vested2, claimable2) = s.vault.status();
    assert_eq!(claimed2, TOTAL / 4);
    assert_eq!(vested2, TOTAL / 4);
    assert_eq!(claimable2, 0);
}

#[test]
fn i6_double_claim_only_yields_incremental_amount() {
    let s = create_setup(VestingCurve::Exponential);

    // First claim at 50 %
    s.env.ledger().with_mut(|l| l.timestamp = START + DURATION / 2);
    let first_claim = s.vault.claim();
    assert_eq!(first_claim, TOTAL / 4); // 0.5^2 * TOTAL

    // Advance to 100 %
    s.env.ledger().with_mut(|l| l.timestamp = START + DURATION);
    let second_claim = s.vault.claim();
    assert_eq!(second_claim, TOTAL - TOTAL / 4); // remaining 75 %

    // Total received = TOTAL
    let bal = TokenClient::new(&s.env, &s.token).balance(&s.beneficiary);
    assert_eq!(bal, TOTAL);
}

// ── Zero-duration / zero-amount edge cases (Issue #41) ──────────────────────

#[test]
#[should_panic(expected = "duration must be positive")]
fn z1_zero_duration_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let admin       = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token    = token_id.address();
    StellarAssetClient::new(&env, &token).mint(&admin, &TOTAL);

    let vault_id = env.register(crate::VestingVault, ());
    let vault    = VestingVaultClient::new(&env, &vault_id);
    TokenClient::new(&env, &token).transfer(&admin, &vault_id, &TOTAL);

    env.ledger().with_mut(|l| l.timestamp = START);

    vault.initialize(
        &admin,
        &beneficiary,
        &token,
        &TOTAL,
        &START,
        &0u64,
        &VestingCurve::Linear,
    );
}

#[test]
#[should_panic(expected = "total_amount must be positive")]
fn z2_zero_amount_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let admin       = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token    = token_id.address();

    let vault_id = env.register(crate::VestingVault, ());
    let vault    = VestingVaultClient::new(&env, &vault_id);

    env.ledger().with_mut(|l| l.timestamp = START);

    vault.initialize(
        &admin,
        &beneficiary,
        &token,
        &0i128,
        &START,
        &DURATION,
        &VestingCurve::Linear,
    );
}

#[test]
#[should_panic(expected = "duration must be positive")]
fn z3_zero_duration_exponential_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let admin       = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token    = token_id.address();
    StellarAssetClient::new(&env, &token).mint(&admin, &TOTAL);

    let vault_id = env.register(crate::VestingVault, ());
    let vault    = VestingVaultClient::new(&env, &vault_id);
    TokenClient::new(&env, &token).transfer(&admin, &vault_id, &TOTAL);

    env.ledger().with_mut(|l| l.timestamp = START);

    vault.initialize(
        &admin,
        &beneficiary,
        &token,
        &TOTAL,
        &START,
        &0u64,
        &VestingCurve::Exponential,
    );
}