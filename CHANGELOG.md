# Changelog

Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are grouped under **Unreleased** until they are cut into a release.
See [docs/RELEASING.md](./docs/RELEASING.md) for the release process
(tagging, the CI build, and its signed build-provenance attestation) — see
[docs/AUDIT_READINESS.md](./docs/AUDIT_READINESS.md) instead for the
separate commit/tag-freeze process ahead of an external audit.

When `Unreleased` entries are cut into a release, they move under a new
`## vX.Y.Z — YYYY-MM-DD` heading, which must record the released Wasm's
SHA-256 (see docs/RELEASING.md § Cutting a release):

```markdown
## vX.Y.Z — YYYY-MM-DD
- Wasm SHA-256: `<sha256 of the release build>`

### <function or area name>
...
```

## Why this file exists

Most changes to this contract don't need an entry here — commit messages
and PR descriptions cover routine work. An entry **is** required when a
pull request changes the contract's public interface (adds, removes, or
renames a function or parameter, or changes a type) — i.e. whenever
[`make interface-spec`](./docs/INTERFACE_SPEC.md) had to be re-run and its
output committed. The interface-spec diff shows *what* changed; the entry
here records *why*, for anyone (an auditor, an integrator, a future
maintainer) who needs the human reasoning without re-deriving it from the
diff alone.

Format for an entry:

```markdown
### <function or area name>
- **What:** one line, matching the shape of the interface-spec diff
  (e.g. "removed `set_old_rate_bps`, added `set_reward_rate_bps` with an
  extra `ramp_ledgers` parameter").
- **Why:** the reason, in enough detail that someone unfamiliar with the
  change understands the motivation without reading the PR.
- **Breaking:** yes/no, and what callers need to do differently if yes.
- **PR:** link or number.
```

## Unreleased

### Stellar Wave issue tracking
- **What:** links the follow-up PR to issues #739, #740, and #741 individually.
- **Why:** the changes for these issues were merged in PR #816, but editing
  that merged PR's body did not retroactively close the issues.
- **Breaking:** no.
- **PR:** closes #739, #740, and #741 when this follow-up is merged.

### Time units and event schema
- **What:** made the withdrawal cooldown's ledger field and selected
  cooldown/vesting arguments unit-explicit, clarified timestamp-only metadata,
  documented the time-unit inventory, and added a typed deposit event.
- **Why:** distinguish ledger counts from Unix timestamp seconds. The deposit
  event retains its existing name, topic order, and positional payload while
  exposing a typed schema.
- **Breaking:** generated field/argument names change for the explicitly
  renamed values; the event's wire encoding is unchanged.
- **PR:** references #739, #740, and #741.

This entry should land alongside the first commit of
`interface-spec/VaultContract.json` (see
[docs/INTERFACE_SPEC.md § One-time setup](./docs/INTERFACE_SPEC.md#one-time-setup) —
that initial snapshot establishes the baseline rather than describing a
change, so it does not itself need an entry, but the first *change*
against it does).
