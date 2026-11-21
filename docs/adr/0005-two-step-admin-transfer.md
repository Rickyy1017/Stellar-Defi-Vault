# ADR-0005: Two-step admin transfer

- **Status:** Accepted
- **Date:** 2026-11-21
- **Related:** issues #738, #881; `docs/DEPRECATION.md`

## Context

A single-step admin transfer changes the most powerful key in the contract in
one transaction. A typo, a wrong network, or a compromised session can hand the
vault to an address nobody controls, with no confirmation from the receiving
side and no way to cancel once the transaction lands.

## Decision

1. **Admin rotation uses propose/accept.** The current admin calls
   `propose_admin(admin, new_admin)`. The proposal is stored as the pending
   admin and emits `adm_prop`. The nominated address must then call
   `accept_admin(new_admin)` and sign; only then is the admin key changed and
   `adm_chg` / `adm_act` / `adm_acpt` are emitted.
2. **Proposals are cancellable.** The current admin can call
   `cancel_admin_proposal(admin)` before acceptance, which clears the pending
   address and emits `adm_canc`.
3. **Invalid targets are rejected early.** The zero/self/contract address cases
   are refused at proposal time, not deferred to acceptance.
4. **The legacy `transfer_admin` entrypoint is deprecated.** It remains callable
   for ABI compatibility but emits `deprecated_call` and is scheduled for
   removal under the policy in `docs/DEPRECATION.md`. New integrations must use
   `propose_admin` + `accept_admin`.

## Consequences

- A wrong recipient cannot silently take over: the nominee must prove control
  by signing.
- The vault can be left with a pending admin; this is visible through
  `get_pending_admin` and does not affect normal operation.
- Any tooling or runbook that calls `transfer_admin` must migrate before the
  removal release.
- The change adds storage for the pending admin and three events, all covered
  by the interface and event documentation.

## Alternatives considered

- **Keep single-step only.** Cheapest to implement, but no recipient
  confirmation and no cancellation window.
- **Require a timelock instead of acceptance.** A timelock delays the transfer
  but still does not prove the new key exists; the two mechanisms solve
  different problems and a timelock can be layered on later.
- **Remove `transfer_admin` immediately.** Would break existing callers and
  violate the deprecation notice period; the compatibility entrypoint is kept
  until the removal release.

## References

- `src/vault.rs` — `propose_admin`, `accept_admin`, `cancel_admin_proposal`,
  `transfer_admin`
- `src/events.rs` — `adm_prop`, `adm_acpt`, `adm_canc`, `deprecated_call`
- `docs/DEPRECATION.md` — removal policy
