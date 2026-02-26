# Formal Verification Invariants (Issue #46 / #72)

This document specifies vesting-math invariants for automated provers (Halmos/Certora) to ensure tokens are never trapped.

## Core Vesting Invariant

For any vault `v` at time `t`:

`Vested(v, t) + Unvested(v, t) == Total_Deposit(v)`

Where:
- `Total_Deposit(v)` is the vault's original allocation (`vault.total_amount`).
- `Vested(v, t)` is the amount unlocked by vesting math at time `t`.
- `Unvested(v, t)` is the remaining locked amount at time `t`.

Equivalent definition:
- `Unvested(v, t) := Total_Deposit(v) - Vested(v, t)`

Safety bounds:
- `0 <= Vested(v, t) <= Total_Deposit(v)`
- `0 <= Unvested(v, t) <= Total_Deposit(v)`

## No-Trapped-Tokens Consequence

At full maturity (`t >= end_time(v)`):
- `Vested(v, t) == Total_Deposit(v)`
- `Unvested(v, t) == 0`

This guarantees that vesting math cannot leave residual locked balance after vesting completion.

## Halmos-Style Invariant Skeleton

```solidity
/// @notice Vesting partition is conserved for any time t.
function invariant_vesting_partition(uint256 vaultId, uint64 t) public {
    uint256 total = totalDeposit(vaultId);
    uint256 vested = vestedAt(vaultId, t);
    uint256 unvested = total - vested;

    assert(vested + unvested == total);
    assert(vested <= total);
}
```

## Certora-Style Invariant Skeleton

```text
invariant VestingPartition(uint256 vaultId, uint64 t)
    vestedAt(vaultId, t) + (totalDeposit(vaultId) - vestedAt(vaultId, t)) == totalDeposit(vaultId);

invariant MaturedVaultFullyUnlocked(uint256 vaultId, uint64 t)
    t >= endTime(vaultId) => vestedAt(vaultId, t) == totalDeposit(vaultId);
```

## Mapping to Current Contract Fields

- `Total_Deposit(v)` maps to `Vault.total_amount`.
- `Vested(v, t)` maps to vesting math output (for example `calculate_time_vested_amount`).
- `Unvested(v, t)` maps to `Vault.total_amount - Vested(v, t)`.

These invariants are math-level and independent of transfer side effects.
