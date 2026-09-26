# Event Schema

All events include the current ledger sequence number (`u32`) in their data payload so
that indexers can reconstruct full pool history from events alone.

## Topic conventions

Every event starts with two indexed topics:

1. `topic[0]`: a stable event-type `Symbol` (at most 9 ASCII characters).
2. `topic[1]`: the primary affected `Address` (user, administrator, recipient,
   guarantor, or the vault contract for pool-wide/system events).

Admin audit events use `topic[2]` for the `AdminAction` discriminator. Other
event data remains in the non-indexed payload; an address may also remain in
the payload when needed to preserve the event's existing data shape.

Indexers should filter by the event symbol first, then by the address in
`topic[1]`. Pool-wide events use the current vault contract address in that
slot because no individual account is the event's primary subject.

## Event Reference

### `deposit`
Emitted on every successful stake/deposit.

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"deposit"` |
| topic[1] | Address | depositor |
| data.0 | i128 | token amount deposited |
| data.1 | i128 | shares minted |
| data.2 | u32 | ledger sequence |

### `withdraw`
Emitted on every successful unstake/withdraw.

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"withdraw"` |
| topic[1] | Address | withdrawer |
| data.0 | i128 | shares burned |
| data.1 | i128 | token amount returned |
| data.2 | u32 | ledger sequence |

### `pos_open` — position_opened
Emitted only on the **first stake** for a user (when their position goes from 0 → non-zero).

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"pos_open"` |
| topic[1] | Address | user |
| data.0 | i128 | initial stake amount |
| data.1 | u32 | ledger sequence |

### `pos_clos` — position_closed
Emitted when a user fully unstakes (position reaches 0).

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"pos_clos"` |
| topic[1] | Address | user |
| data.0 | u32 | ledger sequence |

### `paused`
Emitted when the vault is paused by the admin.

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"paused"` |
| topic[1] | Address | admin |
| data.0 | u32 | ledger sequence |

### `unpaused`
Emitted when the vault is unpaused by the admin.

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"unpaused"` |
| topic[1] | Address | admin |
| data.0 | u32 | ledger sequence |

### `rate_chg` — rate_changed
Emitted when the admin changes the reward rate.

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"rate_chg"` |
| topic[1] | Address | vault contract |
| data.0 | u32 | old rate in basis points |
| data.1 | u32 | new rate in basis points |
| data.2 | u32 | ledger sequence |

### `yield_add` — yield_added
Emitted when the admin injects yield into the vault.

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"yield_add"` |
| topic[1] | Address | admin |
| data.0 | i128 | amount added |
| data.1 | u32 | ledger sequence |

### `tk_rescue` — token_rescued
Emitted when the admin rescues a non-stake, non-reward token from the vault.

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"tk_rescue"` |
| topic[1] | Address | recipient |
| data.0 | Address | token address rescued |
| data.1 | i128 | amount rescued |
| data.2 | Address | recipient address |
| data.3 | u32 | ledger sequence |

### `adm_act` — admin_action
Emitted for on-chain admin audit logging when an admin action is taken.

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"adm_act"` |
| topic[1] | Address | admin actor |
| topic[2] | AdminAction | admin action type |
| data.0 | Address | admin actor |
| data.1 | u32 | ledger sequence |
| data.2+ | mixed | action-specific parameters |

### `admin_set` — admin_transferred
Emitted when the admin role is transferred to a new address.

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"admin_set"` |
| topic[1] | Address | old admin |
| data.0 | Address | new admin |
| data.1 | u32 | ledger sequence |

### `wd_limit` — withdrawal_limit_updated
Emitted when the admin sets a new per-transaction withdrawal limit.

| Field | Type | Description |
|-------|------|-------------|
| topic[0] | Symbol | `"wd_limit"` |
| topic[1] | Address | admin |
| data.0 | i128 | new limit in shares |
| data.1 | u32 | ledger sequence |

### Pool-wide event topics

Events that do not have a single user or administrator as their primary
subject use the vault contract address as `topic[1]`. This applies to:
`boot_str`, `boot_end`, `act_queue`, `act_exec`, `rate_chg`, `rate_rbk`,
`auto_ps`, `rfil_alt`, `prop_enct`, `buyback`, `pen_rdst`,
`halving`, `pool_act`, `pool_dact`, `rwd_act`, `smth_sch`, `smth_rel`,
`auct_st`, `auct_fin`, `gini_cmp`, `seas_str`, `seas_end`, `snst_chg`,
`mkt_rsl`, and `rev_dist`.

Other event types index their affected account in `topic[1]`, including
deposit/withdraw/claim, admin configuration and audit events, governance
votes, guarantees, validator rewards, content curation, and position/NFT
operations. For example, `fee_buyb` indexes the admin actor, while `cc_apprv`
indexes the admin who closed the vote. `ins_dep`, `rd_set`, `vr_set`,
`vr_dist`, and `updated` likewise index the administrator; `test_ev` indexes
the vault contract address.
