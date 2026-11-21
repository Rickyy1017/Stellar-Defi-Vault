# ADR-0003: Fee routing order

- **Status:** Accepted
- **Date:** 2026-11-21
- **Related:** issues #213, #704, #754; `docs/ECONOMICS.md`

## Context

An unstake fee can be charged on a withdrawal. Several subsystems have a claim
on that fee: the insurance reserve, the reward pool, revenue-share
beneficiaries, configured fee recipients, and treasury splits. Applying them in
an arbitrary order changes who is paid and how much, and can make the parts sum
to more than the fee.

## Decision

Fees are routed in one fixed order, and each step receives only what earlier
steps left:

1. **Lock-up penalty**, if applicable. It is applied before the unstake fee and
   only to locked positions that are not exempt.
2. **Unstake fee**, calculated on the post-penalty amount, with these
   precedence rules:
   - `withdraw_only_mode` and an open rage-quit window set the fee to zero;
   - a dynamic fee configuration replaces the static `unstake_fee_bps`;
   - otherwise the static bps applies.
3. **Insurance share** (#704) comes off the top of the fee.
4. **Fee recycling** (#754) moves a configured share of the remaining fee into
   the reward pool.
5. **Revenue share** takes its configured bps of what is left.
6. **Treasury routing**: the remainder goes to configured fee recipients using
   their bps shares. If no recipients are configured, it is credited to the
   reward pool.

The returned principal and the fee both leave `total_deposited`; the fee is
never burned and never silently retained outside the accounting above.

## Consequences

- Fee distribution is auditable step by step, and every subsystem sees a
  strictly smaller remainder.
- Changing the order or the meaning of a step is an economic change that
  requires review and a new ADR.
- Integrators should read `docs/ECONOMICS.md` for the user-visible fee formulas;
  ADR-0003 fixes the internal order, not the rates.

## Alternatives considered

- **Pay recipients first.** Would let configured recipients consume the fee
  before the insurance reserve and reward pool are funded.
- **Split the fee by a single flat table.** Simpler on paper, but the insurance
  and recycling features have their own caps and fallbacks; a flat split would
  duplicate that policy in two places.
- **Burn the fee.** Conflicts with the documented model where the fee rewards
  remaining holders and the reward pool.

## References

- `src/vault.rs` — withdrawal fee calculation and routing
- `src/vault_extensions_701_704.rs` — insurance reserve share
- `src/fee_recycle.rs` — fee recycling
- `docs/ECONOMICS.md` — fee formulas and caps
