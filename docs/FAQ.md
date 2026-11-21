# Staker FAQ

Plain-language answers for depositors and stakers. Numeric rules here are
cross-checked against [`docs/ECONOMICS.md`](./ECONOMICS.md); that document is
the reference if the two ever disagree.

## What is the vault?

The vault is a non-custodial, share-based yield pool on Stellar. You deposit a
Stellar token and receive **shares**. Shares represent a proportional claim on
the pool's deposited tokens. When yield is added, the value of each share rises
without minting new shares for the admin.

## How are shares priced?

Shares always use **7 decimal places**, regardless of the underlying token's
precision.

- **First deposit:** you receive the amount scaled to 7 decimals. A 7-decimal
  token is 1:1; a 6-decimal token is multiplied by 10.
- **Later deposits:** `shares = floor(amount × total_shares / total_deposited)`.
- **Redemption:** `tokens = floor(shares × total_deposited / total_shares)`.

The share price is `total_deposited / total_shares`, so it starts at 1.0 and
rises when yield is added without new shares. A later deposit at the current
ratio does not change the price. See
[ECONOMICS §1–2](./ECONOMICS.md#1-share-minting).

## How do rewards work?

Rewards accrue from a configured annual rate in basis points (bps), where
**10,000 bps = 100%**. `1,000 bps` is therefore 10% APR.

```
effective_rate = rate_bps × tier_mult × campaign_mult × season_mult / 10,000³
reward = amount × effective_rate × elapsed_ledgers / 10,000 / 6,307,200
```

`6,307,200` is the number of Stellar ledgers in a year at the 5-second ledger
target. The three multipliers default to 10,000 (1×) and stack
multiplicatively. Rewards are paid from the reward pool, so the pool must be
funded for claims to succeed. See
[ECONOMICS §3 and §5](./ECONOMICS.md#3-reward-accrual).

## What fees exist?

- **Unstake fee:** a percentage of the post-penalty withdrawal, capped at
  **500 bps (5%)**. It is applied after any lock-up penalty. The fee remains
  inside the vault ecosystem — routed to the reward pool, insurance reserve,
  revenue-share pool, or configured fee recipients — rather than burned.
- **Deposit fee:** optional and configured in bps; it is taken from the amount
  used to compute shares, which leaves existing holders slightly better off.
- **Claim fee and dynamic fees:** may exist when the admin or a configured
  policy enables them.

Withdraw-only mode and an open rage-quit window make exits fee-free. See
[ECONOMICS §4](./ECONOMICS.md#4-fees) and
[ADR-0003](./adr/0003-fee-routing.md).

## What are locks and penalties?

Locking a position is optional and admin-configured. A locked position may
carry an **early-exit penalty** charged before the unstake fee. The penalty can
be waived for a restaked position, when a configured oracle condition is met,
when the position is insured, or while withdraw-only mode is active. Lock
parameters are visible through the admin/query entrypoints; check them before
locking.

## What are the risks?

- **Smart-contract risk:** the contract is unaudited. A bug could lose funds.
- **Admin risk:** the admin can pause, change fees and rates, set caps, add or
  route yield, enable/disable inflows, and upgrade the code. The admin cannot
  take your principal through the normal withdrawal path, and cannot rescue the
  configured stake or reward token with `rescue_token`.
- **Reward-liquidity risk:** accrued rewards are only payable if the reward
  pool is funded.
- **Strategy risk:** funds allocated to external strategies can lose value;
  losses are absorbed by the insurance reserve first and then socialised
  through the share price.
- **Token risk:** Stellar Asset Contract tokens can have issuer-controlled
  behaviour such as revocation or clawback. Read the issuer's terms.
- **Rounding and dust:** integer arithmetic can leave tiny remainders; they stay
  in the pool rather than being paid to an operator.
- **Archival risk:** Soroban entries can be archived if their TTL expires.
  Reads fail clearly and can be restored (see [`RESTORE.md`](./RESTORE.md)).

## What can the admin do?

Pause or stop the vault, enter withdraw-only mode, start a one-way sunset,
change reward rates, fees, caps, boosts, and fee recipients, route treasury
funds, configure roles and multisig, upgrade the Wasm, and slash a position
under the documented rules. See the README's **Trust Assumptions and Admin
Powers** and [`SECURITY.md`](./SECURITY.md).

## What can the admin NOT do?

- Withdraw another user's shares or principal through the normal withdrawal
  path; those calls require the user's authorization.
- Move the configured stake or reward token with `rescue_token`.
- Reopen deposits after a sunset (sunset is one-way).
- Change the past: balances, checkpoints, and events already written remain.

## How do I exit?

`unstake`, `withdraw`, or `unstake_all` burns shares and returns tokens. While
the vault is paused or sunsetting, withdrawals, unstake, and claims stay open.
In withdraw-only mode exits are also fee-free. If a position is locked, the
early-exit penalty may apply unless a waiver is active.

## Where are the numbers defined?

All formulas, worked examples, caps, and constants are in
[`docs/ECONOMICS.md`](./ECONOMICS.md). Terms are defined in
[`docs/GLOSSARY.md`](./GLOSSARY.md).
