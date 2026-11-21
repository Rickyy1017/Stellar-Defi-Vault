# Event Snapshots

Event payload changes are part of the public contract: indexers, wallets, and
audit pipelines decode topics and data positionally. `src/test_event_snapshots.rs`
turns an accidental schema change into a failing, reviewable diff.

This is separate from Soroban's `test_snapshots/` directory. Those capture raw
XDR of every invocation and are **not** committed (see
[`TEST_SNAPSHOTS.md`](./TEST_SNAPSHOTS.md)). The snapshots here are curated,
human-readable event traces and **are** committed.

## What is snapshotted

Each test drives one public entrypoint and records the events whose `topic[0]`
is the documented symbol for that entrypoint:

| Test | Entrypoint | Symbols captured |
| --- | --- | --- |
| `deposit_event_schema_is_stable` | `deposit` | `deposit` |
| `withdraw_event_schema_is_stable` | `withdraw` | `withdraw` |
| `claim_event_schema_is_stable` | `claim` | `claimed` |
| `pause_event_schema_is_stable` | `pause` | `paused` |
| `unpause_event_schema_is_stable` | `unpause` | `unpaused` |
| `fee_event_schema_is_stable` | `set_unstake_fee_bps` | `fee_set` |
| `yield_event_schema_is_stable` | `add_yield` | `yield_add` |
| `propose_admin_event_schema_is_stable` | `propose_admin` | `adm_prop` |
| `accept_admin_event_schema_is_stable` | `accept_admin` | `admin_set`, `adm_act`, `adm_acpt` |
| `deprecated_transfer_admin_event_schema_is_stable` | `transfer_admin` | `deprec`, `admin_set`, `adm_act` |
| `withdraw_only_event_schema_is_stable` | `enter_withdraw_only_mode` | `wd_only` |
| `sunset_event_schema_is_stable` | `initiate_sunset` | `snst_ini` |

The trace renders each event as its topic list and data value:

```
0: topics=[symbol(deposit), address, ] data=vec[i128(1000000), i128(1000000), u32(0)]
```

- `address` replaces any `Address` value, because test addresses are random.
- Symbols, integers, booleans, strings, vectors, and maps are rendered by value.
- The `0:` prefix is the position within the filtered trace.

The snapshots live in `src/snapshots/stellar_defi_vault__test_event_snapshots__*.snap`.

## Running and updating

Run just the event snapshots:

```bash
cargo test --features testutils test_event_snapshots
```

On a mismatch, `insta` writes a `.snap.new` file and prints a diff. Review it,
then accept the change deliberately:

```bash
cargo insta accept          # or: cargo insta review
# without cargo-insta:
INSTA_UPDATE=always cargo test --features testutils test_event_snapshots
```

Commit the updated `.snap` file with the code change and update
[`EVENTS.md`](./EVENTS.md) for any new or changed event. If the change alters a
public event, also consider the `CHANGELOG.md` entry required by the PR
template.

## Review flow

1. Change the event or the entrypoint.
2. Run the snapshot tests; expect a failure with a diff.
3. Read the diff: confirm the topic order, data order, and types are exactly
   what you intended. A reordered `Vec` field is a breaking change for indexers
   even though the function signature did not change.
4. Accept, commit the `.snap` file, and update `EVENTS.md`.
5. CI runs with `INSTA_UPDATE=no`, so a mismatch fails the build instead of
   silently writing a new snapshot.

## Adding coverage for a new entrypoint

Add a test to `src/test_event_snapshots.rs` that drives the entrypoint, filters
with `trace_for(&env, &["<symbol>"])`, asserts the trace is non-empty, and calls
`insta::assert_snapshot!("<name>", trace)`. Then run with
`INSTA_UPDATE=always` once to create the snapshot, and review it before
committing. Add a row to the table above.
