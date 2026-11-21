# Deprecation Policy

Interface changes need a predictable path. This document defines how a public
entrypoint is marked deprecated, how long it must stay callable, and when it
may be removed.

## What counts as deprecation

A function is deprecated when it is still callable but should no longer be used
by new integrations. It is **removed** only when it is deleted from the contract
and the public interface snapshot.

Deprecation does not apply to internal helpers, test fixtures, or admin-only
diagnostic functions that were never part of the documented interface.

## Notice period

A deprecated entrypoint stays callable for **at least one tagged release and at
least 90 days**, whichever is longer, before it can be removed.

- The notice starts in the release that contains the `#[deprecated]` marker and
  its `docs/DEPRECATION.md` entry.
- Removal requires a **major version** bump in `CONTRACT_VERSION` and a
  `CHANGELOG.md` entry under **Unreleased** describing the removal and the
  replacement.
- A security emergency may shorten the window, but the removal must still be
  documented and announced in the changelog.

## How deprecation is signalled

Every deprecated entrypoint uses all four signals:

1. **Rust marker** — `#[deprecated(note = "...")]` on the function, naming the
   replacement. This makes new in-repo callers fail review.
2. **Doc comment** — a `/// Deprecated: ...` sentence and a `# Deprecation`
   section naming the replacement and the removal release.
3. **Registry** — a row in the table below with the replacement and the release
   in which the notice started. The interface snapshot test
   (`src/test_runbook_interface_snapshot.rs`) embeds this document and checks
   that every entrypoint in its code-side list appears here, so editing one
   without the other fails the test.
4. **On-chain event** — the entrypoint emits a `deprecated_call` event on every
   invocation, with the deprecated function name in the data payload. Indexers
   can use it to find remaining callers before removal.

## Registry

| Entrypoint | Replacement | Notice started | Earliest removal |
| --- | --- | --- | --- |
| `transfer_admin(new_admin)` | `propose_admin(admin, new_admin)` followed by `accept_admin(new_admin)` | 0.1.0 | Next major release after 2027-02-19 (90 days) |

The single-step transfer is the first entrypoint under this policy. It is kept
for ABI compatibility while integrators move to the two-step flow (see
[ADR-0005](./adr/0005-two-step-admin-transfer.md)); it emits `deprecated_call`
and still performs the transfer if called.

## Removal checklist

Before deleting a deprecated entrypoint:

- [ ] The notice period has elapsed and the removal release is a major bump.
- [ ] The registry row is updated with the removal release.
- [ ] `docs/INTERFACE.md`, `docs/INTEGRATION.md`, the README interface table,
      SDK helpers, and scripts no longer reference the entrypoint.
- [ ] `make interface-spec` has been run and the refreshed snapshot committed.
- [ ] A `CHANGELOG.md` entry explains the removal and the migration path.
- [ ] The replacement is covered by tests and documented in `docs/EVENTS.md`
      and the interface reference.

## Deprecating a new entrypoint

1. Add `#[deprecated(note = "use <replacement> instead")]` and the doc section.
2. Emit `deprecated_call` from the entrypoint.
3. Add a registry row with the replacement and the release that starts the
   notice period.
4. Update `docs/INTERFACE.md` and the README interface table.
5. Add the entrypoint to `DEPRECATED_ENTRYPOINTS` in
   `src/test_runbook_interface_snapshot.rs`.
6. Note the deprecation in `CHANGELOG.md` under **Unreleased**.
