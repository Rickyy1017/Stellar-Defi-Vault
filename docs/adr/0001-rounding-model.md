# ADR-0001: Rounding model and dust handling

- **Status:** Accepted
- **Date:** 2026-11-21
- **Related:** issues #220, #457, #650; `docs/ECONOMICS.md`

## Context

All vault accounting uses integer arithmetic in the token's base unit. Integer
division loses a remainder, so every share mint, share burn, reward accrual,
and fee calculation has to choose a direction. Choosing inconsistently lets
value leak across users, and rounding in the vault's favour is indistinguishable
from a hidden fee.

## Decision

1. **Share conversion floors.** `amount_to_shares_with_decimals` uses
   `floor(amount * total_shares / total_deposited)` after the first deposit, and
   the first depositor receives the amount scaled to seven share decimals.
   `shares_to_amount_with_decimals` also floors. This is the default
   `RoundingPolicy::Floor` in `src/balance.rs`.
2. **Configurable policy is explicit.** The admin may select `Floor`,
   `Ceiling`, or `Nearest` through `set_rounding_policy`; the policy is stored
   under `rnd_pol` and defaults to `Floor` when unset.
3. **Staker-favour mode applies per operation.** When enabled (#457), rewards
   round up (ceiling division) and fees round down, so ambiguity never
   disadvantages the staker. This is a separate switch from `RoundingPolicy`
   and is off by default.
4. **Reward accrual uses a scaled accumulator.** Pending rewards are tracked
   with a per-share index scaled by `ACCUMULATOR_SCALE = 1e12` (#650) so that
   dust does not accumulate into material rounding loss across many ledgers.
5. **Dust is tracked, not silently discarded.** Remainders that cannot be
   attributed to a position remain in the pool and are reported by the rounding
   dust accounting (`rounding_dust`) rather than moved to an operator address.

## Consequences

- Depositors cannot receive more shares than the proportional formula allows,
  and the pool cannot mint value out of rounding.
- The first depositor and subsequent depositors use the same proportional
  model after decimal scaling.
- Every new arithmetic path must state its rounding direction and be covered by
  a determinism test; `src/test_issues_769_differential.rs` and the economics
  examples are the reference.
- Staker-favour mode can change reward and fee results, so it is an
  admin-visible parameter and must be announced as an economic change.

## Alternatives considered

- **Always round up.** Simple to reason about for the staker, but lets a user
  extract value from the pool by repeatedly minting and burning shares.
- **Always round to nearest.** Creates two-sided leakage and makes invariants
  harder to state.
- **Use floating point.** Not deterministic across platforms and not available
  in the Soroban VM for consensus-critical accounting.

## References

- `src/balance.rs` — share conversion, rounding policy, dust accounting
- `src/staker_favor_rounding.rs` — staker-favour rewards and fees
- `src/reward_accumulator.rs` — scaled per-share index
- `docs/ECONOMICS.md` — worked examples and constant values
