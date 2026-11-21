# Formal verification evaluation: Certora Sunbeam (issue #877)

This is the deliverable for the exploratory Certora evaluation. It records what
was prototyped, what was not proved, and whether the tool should run in CI.

## Scope

Target: the core share-accounting invariants in [`docs/INVARIANTS.md`](./INVARIANTS.md)
(INV-1 to INV-9), not feature-specific invariant sets.

Prototype spec: [`formal/certora/vault_invariants.spec`](../formal/certora/vault_invariants.spec)
with a starter config in [`formal/certora/vault_invariants.conf`](../formal/certora/vault_invariants.conf).

## Setup

Certora Sunbeam is the Soroban/SVM front end for the Certora Prover.

```bash
# Requires a Certora account and the Sunbeam package available on the host.
pipx install certora-cli
export CERTORAKEY="<your key>"

# From the repository root:
certoraRun formal/certora/vault_invariants.conf
```

The config points at `src/lib.rs` and the `VaultContract` entrypoints. Depending
on the Sunbeam release, the harness may need an explicit compiled Wasm artifact
and a generated ABI in place of the source path.

## Status

- Spec skeleton written for INV-1, INV-2, INV-3 (obligation only), INV-4, INV-5
  and INV-6.
- No rule was run or proved. The local environment has no Certora key or
  Sunbeam installation, and the repository's current `main` does not compile,
  so a harness could not be produced.
- The spec is therefore an executable-intent document, not a CI gate.

## Findings

- The pure conversion functions (`amount_to_shares_with_decimals`,
  `shares_to_amount`) are the best proof targets: they have no storage or
  cross-contract calls, so CVL can reason about them directly.
- The rest of the vault is one large `#[contractimpl]` surface split across
  many extension modules. A Sunbeam harness has to select the deposit,
  withdraw, yield and balance entrypoints explicitly; proving the whole
  contract at once is not realistic.
- Storage is raw `Symbol`-keyed (`src/balance.rs`), so ghost variables must
  mirror the setters rather than being derived from a typed state object.
- Token transfers and balances are external calls. Solvency (INV-3) can only
  be proved by modelling the token contract as uninterpreted and assuming the
  transfer semantics; it is not a closed-system property.
- The first-deposit decimal scaling path (INV-8) depends on a token query at
  initialization, which the harness must model as a fixed symbolic decimal
  value.

## Unproven properties

- INV-1 share conservation: stated; not proved.
- INV-2 non-negativity: stated; not proved.
- INV-3 solvency: written as an obligation only; not proved.
- INV-4 full-pool redeemability: stated; floor-division arithmetic not proved.
- INV-5 price monotonicity: stated; overflow and zero-share cases not proved.
- INV-6 no value creation over a cycle: stated; not proved.
- INV-7 donation isolation and INV-8 first-deposit scaling: not specified.
- INV-9 mint/burn accounting: not specified.

## Decision

Do not run Certora in CI for now.

Reasons:

- it needs a Certora key and cloud prover, which this repository does not hold;
- Sunbeam's Soroban support is still a preview and the harness format is likely
  to change;
- the property suite and `invariants.rs` checks cover the cheapest cases, and
  gaps G1-G6 in `docs/INVARIANTS.md` are a better first investment.

Revisit after a compiling baseline and after closing G1-G6. If the project
adopts it later, start with INV-4/INV-6 on the two pure conversion functions,
then extend to INV-1/INV-2 with ghost state.
