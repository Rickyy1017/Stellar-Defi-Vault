# ADR-0002: Storage layout and key namespacing

- **Status:** Accepted
- **Date:** 2026-11-21
- **Related:** issues #646, #659; `docs/STORAGE_LAYOUT.md`, `docs/STORAGE.md`

## Context

The vault is upgradeable in place. Soroban encodes stored values as XDR derived
from `#[contracttype]` definitions, so a field reorder, width change, variant
reorder, or new required field silently changes the bytes on disk without a
compiler error. Separately, a single `#[contracttype]` enum is limited to 50
variants (`VecM<_, 50>`), and the original `DataKey` reached that limit.

## Decision

1. **`DataKey` is frozen.** Do not add variants to the legacy `DataKey`. It is
   mapped to the namespaced `StorageKey` through `DataKey::to_storage_key()`.
2. **New keys go into a domain sub-enum.** Add typed variants to `AdminKey`,
   `PositionKey`, `RewardKey`, or `GovernanceKey` and access them through
   `StorageKey::<SubEnum>(...)`.
3. **Feature modules may use raw `Symbol` keys** when their state is local to
   the feature and does not belong in the shared enums. Such modules document
   the key next to the storage accessors, as `balance.rs` does.
4. **The layout is snapshotted.** `src/test_storage_layout.rs` serialises the
   core keys and values to XDR and compares them with
   `test-data/storage_layout.snapshot`. A layout change requires a migration
   and an updated snapshot in the same pull request (#659).

## Consequences

- Existing deployments remain readable across upgrades.
- Adding state requires a deliberate choice between a typed sub-enum and a
  documented `Symbol` key; it no longer competes for the last `DataKey` slot.
- A storage change is visible as a CI-failing snapshot diff, not a silent
  on-chain break.
- Migrations are mandatory for any change that touches an already-written type.

## Alternatives considered

- **Keep extending `DataKey`.** Blocked by the 50-variant cap and would require
  a one-time breaking migration anyway.
- **Use untyped `Symbol` keys everywhere.** Cheap to add, but loses compiler
  checking of key/value types and makes collisions easy.
- **Store everything in one serialized blob.** Avoids per-key limits but makes
  every read and write rewrite the whole state and increases cost.

## References

- `src/storage.rs` — `DataKey`, `StorageKey`, and domain sub-enums
- `src/test_storage_layout.rs` — encoding snapshot test
- `docs/STORAGE.md` — instance, persistent, and temporary storage choices
- `docs/STORAGE_LAYOUT.md` — namespacing convention and snapshot procedure
