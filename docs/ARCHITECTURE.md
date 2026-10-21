# Stellar DeFi Vault Architecture & Strategy Policy

## Overview

The Stellar DeFi Vault integrates external yield strategies through standard strategy adapters while preserving core vault safety, immediate withdrawal liquidity, and transparent loss accounting.

---

## Strategy Adapter Interface

Strategy adapters conform to a lightweight interface that isolates external protocol logic from core vault state:

- `deposit(from: Address, amount: i128)`: Transfers underlying assets from the vault into the strategy.
- `withdraw(to: Address, amount: i128) -> i128`: Recalls funds back to the vault.
- `assets() -> i128`: Reports total asset value currently controlled by the adapter.

All adapter invocations from the vault are contained using `try_invoke_contract`. If an external adapter reverts or fails, the failure is handled gracefully and never bricks vault operations or user withdrawals.

---

## Idle Buffer Ratio (#765)

To guarantee that routine user withdrawals do not depend on external strategy unwinding, the vault enforces a minimum idle liquidity ratio:

- **Configuration**: `set_min_idle_ratio_bps(admin, bps)` configures the minimum percentage of TVL (in basis points, 0–10,000) that must remain as idle balance in the vault contract.
- **Enforcement**: When `allocate_to_strategy(admin, adapter, amount)` is called, the transaction verifies:
  $$\text{Vault Idle Balance} - \text{Allocation Amount} \ge \frac{\text{TVL} \times \text{Min Idle BPS}}{10,000}$$
  If the remaining idle funds would breach this ratio, the call reverts with `StrategyError::IdleBufferBreached`.
- **Withdrawal Order**: If an extraordinary withdrawal exceeds current idle liquidity, the vault automatically calls `pull_from_strategies(needed)` in order of strategy registration, replenishing idle funds up to the requested withdrawal amount.

---

## Yield Harvesting & Performance Fees (#766)

Accumulated yield from strategies is realized through `harvest(caller)`:

- **Callable by Anyone**: Keepers, users, or operators can trigger harvests.
- **High-Water Mark (HWM)**: Each strategy tracks its peak asset valuation (`high_water_mark`). Performance fees are assessed strictly on gains exceeding this mark:
  $$\text{Gain} = \max(0, \text{Current Assets} - \text{HWM})$$
  During drawdowns, the high-water mark is preserved, ensuring users are never charged performance fees for recovering previous losses.
- **Fee Routing**:
  - A small keeper tip (5% of the performance fee) is paid to the caller.
  - The remaining fee is routed through the vault's configured treasury split (`TreasurySplit`).

---

## Loss Accounting & Socialization Policy (#767)

When an external protocol suffers an exploit, economic shortfall, or bad debt, `report_strategy_loss(caller, adapter)` reconciles the loss using an explicit two-tier hierarchy:

1. **First Layer (Insurance Reserve Absorption)**:
   - The loss is first absorbed by the operator-funded insurance reserve (issue #704).
   - If available reserve $\ge \text{Loss}$, the reserve pays the shortfall and vault share prices are unaffected.
2. **Second Layer (Share Price Socialization)**:
   - Any residual loss exceeding the insurance reserve is absorbed by reducing the vault's `total_deposited`.
   - Because share valuation is proportional to $\frac{\text{total\_deposited}}{\text{total\_shares}}$, this proportionally lowers the share price across all stakers.
3. **Anti-Front-Running Protection**:
   - Loss reporting adjusts `total_deposited` and strategy allocation atomically.
   - Any subsequent withdrawal executes against the lower post-loss share valuation, preventing informed actors from exiting at par before losses are realized.

---

## Emergency Unwind Strategy (#768)

When an external strategy is compromised:

- **Authorization**: Callable by the `admin` or `guardian` (`emergency_unwind(caller, adapter)`).
- **Immediate Disabling**: The strategy is marked `disabled = true`. Disabled strategies are barred from receiving new allocations.
- **Contained Recovery**: The vault attempts to withdraw all allocated principal from the adapter. Adapter panics or failures are trapped and contained.
- **Shortfall Reconciliation**: Any difference between allocated principal and recovered tokens is immediately channeled into the two-tier loss accounting pipeline.
