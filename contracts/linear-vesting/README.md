# Linear Vesting Contract

Standalone Soroban contract for linear token vesting with an optional cliff and optional funder revocation (issue #935).

## Entrypoints

- `initialize(token)` — Configures the reward/vesting token address once.
- `create_schedule(funder, beneficiary, amount, cliff_ledgers, duration_ledgers, revocable)` — Transfers `amount` from `funder` into the contract and creates a linear vesting schedule starting at the current ledger.
- `release(beneficiary)` — Transfers the currently vested, unreleased portion to `beneficiary`.
- `revoke(funder, beneficiary)` — For revocable schedules, freezes vesting at the current ledger and returns only unvested tokens to `funder`.
- `get_schedule(beneficiary)` / `releasable_amount(beneficiary)` — Read-only schedule queries.

## Testing

```sh
cargo test --manifest-path contracts/linear-vesting/Cargo.toml
```
