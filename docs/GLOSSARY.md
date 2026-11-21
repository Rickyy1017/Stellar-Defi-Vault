# Glossary

Terms used throughout the vault's documentation and code. Numeric definitions
match [`docs/ECONOMICS.md`](./ECONOMICS.md).

## Vault terms

**Share** — the vault's internal unit of account. Shares have **7 decimal
places** and represent a proportional claim on `total_deposited`. They are not
a separate token unless an NFT receipt or allowance feature is explicitly used.

**Share price** — `total_deposited / total_shares`. It starts at 1.0 and rises
when yield is added without minting shares; deposits at the current ratio leave
it unchanged.

**total_deposited (`D`)** — the pool's accounted underlying token amount. It is
the denominator/numerator in share conversions.

**total_shares (`S`)** — the sum of all minted shares, in 7-decimal precision.

**Share decimals** — always 7. A token with fewer decimals is scaled up on the
first deposit; a token with more decimals is scaled down.

**Deposit / stake** — moving tokens into the vault and minting shares. `stake`
is the staking-oriented alias.

**Withdraw / unstake** — burning shares and returning the proportional token
amount. `unstake` is the staking-oriented alias.

**Reward pool** — the pre-funded balance that pays claimable rewards. Accrued
rewards are not payable if this pool is empty.

**Reward accumulator** — a global per-share index used to accrue rewards in O(1).
It is scaled by `ACCUMULATOR_SCALE = 1e12` to keep rounding dust small.

**Effective rate** — the configured annual rate in bps multiplied by the active
boost multipliers (tier, campaign, season), each expressed in bps.

**Boost tier** — a time-in-vault threshold that raises the reward multiplier.
Up to **5** tiers are supported.

**Unstake fee** — a fee on withdrawal, capped at **500 bps (5%)**. It is applied
after any lock-up penalty and, with the fee's other routes, funds the insurance
reserve, reward pool, revenue share, or fee recipients.

**Deposit fee** — an optional bps fee taken from the amount used to mint shares.

**Lock-up penalty** — an early-exit charge on a locked position, applied before
the unstake fee. It can be waived under the documented conditions.

**bps (basis point)** — one hundredth of one percent. `10,000 bps = 100%`,
`100 bps = 1%`. The constant is `BPS_DENOMINATOR = 10,000`.

**Pause** — an admin-reversible state that blocks normal operations. A
scheduled `pause_until` auto-unpauses at its target ledger.

**Withdraw-only mode** — a state that blocks all inflows while leaving
withdrawals, unstake, and claims open and fee-free. It is reversible.

**Sunset** — a one-way wind-down with an exit deadline. New deposits stop;
withdrawals and claims stay open.

**Slash** — an admin action that reduces a position under the documented rules.
It does not require the affected user's signature.

**Treasury split** — a configured set of recipients and bps shares that receive
treasury or fee amounts.

**Reward vesting / cliff** — a schedule that releases claimed or accrued
rewards over time after a cliff.

## Stellar and Soroban terms

**Ledger** — Stellar's unit of time and state. A ledger closes roughly every
**5 seconds**; `LEDGERS_PER_DAY = 17,280` and `STELLAR_LEDGERS_PER_YEAR =
6,307,200` follow from that target.

**Stroop** — the smallest unit of XLM: `1 XLM = 10,000,000 stroops`, i.e. 7
decimal places. Token amounts in the vault are always base units of their own
token, which may have a different decimal precision.

**SAC (Stellar Asset Contract)** — the Soroban contract interface that wraps a
Stellar asset (including native XLM) so contracts can use the token interface.

**TTL (time to live)** — the number of ledgers a Soroban storage entry remains
live. Instance, persistent, and temporary entries have different TTL rules.

**Archival** — when a Soroban entry's TTL expires, it can be archived and later
restored. The vault's reads fail clearly rather than returning a default; see
[`RESTORE.md`](./RESTORE.md).

**Contract ID** — the address that identifies the deployed vault contract.

**Wasm** — the compiled Soroban contract bytecode deployed on-chain. The vault
is upgradeable by replacing its Wasm with a new hash.

**Interface spec** — the machine-readable description of exported functions,
parameters, and types, generated from the release Wasm; see
[`INTERFACE_SPEC.md`](./INTERFACE_SPEC.md).

**Deprecated entrypoint** — a callable function that should no longer be used
and is on a removal timeline; see [`DEPRECATION.md`](./DEPRECATION.md).

**ADR (Architecture Decision Record)** — a short document recording why a major
design choice was made; see [`adr/README.md`](./adr/README.md).
