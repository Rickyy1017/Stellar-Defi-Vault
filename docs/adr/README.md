# Architecture Decision Records

Architecture Decision Records (ADRs) capture **why** the vault is built the way
it is. They exist so the same design questions are not re-litigated in every
pull request, and so a reviewer can tell an intentional constraint from an
accident.

## Numbering and status

- Files live in `docs/adr/` and are named `NNNN-short-title.md`.
- Numbers are sequential and never reused, even after an ADR is superseded.
- Copy [`template.md`](./template.md) to start a new record.
- Allowed statuses: **Proposed**, **Accepted**, **Superseded by ADR-NNNN**,
  **Deprecated**.
- An accepted ADR is changed by writing a new ADR that supersedes it, not by
  rewriting history.

## Index

| ADR | Title | Status |
| --- | --- | --- |
| [0001](./0001-rounding-model.md) | Rounding model and dust handling | Accepted |
| [0002](./0002-storage-layout-and-namespacing.md) | Storage layout and key namespacing | Accepted |
| [0003](./0003-fee-routing.md) | Fee routing order | Accepted |
| [0004](./0004-gating-precedence.md) | Gating precedence for pool states | Accepted |
| [0005](./0005-two-step-admin-transfer.md) | Two-step admin transfer | Accepted |

## When a change needs an ADR

Open an ADR when a change alters one of the following:

- an economic rule (rounding, fees, rewards, caps, boosts),
- a storage layout or key convention,
- the order in which safety gates are evaluated,
- an authority or upgrade model,
- a public interface that cannot be changed without a migration.

The pull request template asks whether an ADR is needed. Small bug fixes,
formatting, and mechanical refactors do not need one.
