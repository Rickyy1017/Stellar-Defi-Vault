# ADR-0004: Gating precedence for pool states

- **Status:** Accepted
- **Date:** 2026-11-21
- **Related:** issues #525, #705; `docs/SECURITY.md`, `docs/INCIDENT_RESPONSE.md`

## Context

The vault has several independent states that restrict behaviour: paused,
stopped, sunsetting, and withdraw-only. They overlap in intent but differ in
what they block and whether they can be reversed. If checks appear in different
orders on different entrypoints, a user can reach an operation through a path
that should have been gated.

## Decision

Every state-changing path uses the same gates, in this order:

1. **Initialization** — the vault must be initialized before any operation that
   needs its token or admin.
2. **Stopped / emergency stop** — the hardest block; no normal operation runs.
3. **Paused** — user and admin operations that are not explicitly allowed while
   paused return `VaultPaused`. `apply_scheduled_unpause_if_due` runs first so a
   scheduled expiry is honoured.
4. **Sunset / withdraw-only** — checked only on **inflow** paths
   (`deposit`, `stake`, `stake_for`, `stake_and_claim`, `deposit_for_many`,
   compounding and locking). Both return `PoolShuttingDown` and both leave
   withdrawals, unstake, and claims open.
5. **Operation-specific validation** — amounts, caps, allowlists, cooldowns,
   and authorization for the specific entrypoint.

Sunset and withdraw-only are deliberately not interchangeable:

- **Sunset** is one-way and has an exit deadline.
- **Withdraw-only** is immediate, reversible, and also waives the unstake and
  claim fees while active.

## Consequences

- Refunds and exits remain available whenever new inflows are blocked.
- Adding a new inflow path requires routing it through `require_no_sunset`.
- Adding a new state requires deciding where it sits in this order and testing
  the interaction with pause, sunset, and withdraw-only.
- Error codes are stable: `VaultPaused`, `ContractStopped`, and
  `PoolShuttingDown` each identify a specific gate.

## Alternatives considered

- **One combined boolean.** Would lose the distinction between a temporary
  pause, an emergency stop, and a wind-down, and would make the fee waiver in
  withdraw-only mode impossible to express faithfully.
- **Check gates inside each helper.** Repetition invites divergence; the shared
  `require_*` helpers keep the order in one place.
- **Let sunset be reversed.** Would allow reopening deposits after holders were
  told the pool is exiting.

## References

- `src/vault.rs` — `require_not_paused`, `require_no_sunset`,
  `require_not_stopped`, `require_not_sunsetting`
- `src/withdraw_only_mode.rs` — mode semantics and fee waivers
- `docs/SECURITY.md` — admin powers and shutdown behaviour
