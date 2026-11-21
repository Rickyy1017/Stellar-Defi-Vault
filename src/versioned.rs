//! Versioned storage wrappers for long-lived persisted structs.
//!
//! A `#[contracttype]` struct is encoded on chain as a map keyed by its field
//! names. Adding, removing or renaming a field changes that encoding, so every
//! entry written by an older contract version stops decoding and the contract
//! traps on read. These wrappers avoid that: the struct is stored inside an
//! enum (`V1(..)`, `V2(..)`, ...). A new layout is a new variant; old variants
//! keep decoding forever, so no storage migration is needed.
//!
//! The three persisted structs covered here:
//!
//! | In-memory (latest) type | Stored as           | Stored at                       |
//! |-------------------------|---------------------|---------------------------------|
//! | `StakePosition`         | `VersionedPosition` | split positions (`splitpos`)    |
//! | `GovernanceProposal`    | `VersionedProposal` | `("prop", id)`, `gov_prop`      |
//! | `VeBoostLock`           | `VersionedLock`     | `("ve_lock", user)`             |
//!
//! # Reading
//!
//! Callers never see the enums. `read_*` returns the latest in-memory struct,
//! converting older variants on the way out ("normalizing"). Every `read_*`
//! also accepts the **legacy bare-struct encoding** that contracts deployed
//! before this change wrote, so existing entries keep working. They are
//! rewritten in the versioned form the next time they are saved.
//!
//! # Writing
//!
//! `write_*` always wraps the in-memory struct in the newest variant.
//!
//! # Adding a field safely
//!
//! Using `StakePosition` as the example (the other two work the same way):
//!
//! 1. **Freeze the old layout.** The `StakePositionV1` struct below is an exact
//!    copy of the layout V1 entries were written with. Never edit it.
//! 2. **Add the field to the in-memory struct** (`storage::StakePosition`),
//!    e.g. `pub boost_bps: u32`.
//! 3. **Add a variant** to `VersionedPosition`: `V2(StakePosition)`. Keep `V1`.
//! 4. **Update `normalize`**: the `V1(old)` arm builds a `StakePosition` from the
//!    old fields and a sensible default for the new one (e.g. `boost_bps: 0`);
//!    the `V2(p)` arm returns `p` unchanged.
//! 5. **Update `wrap`** to return `VersionedPosition::V2(position)`.
//! 6. **Update the legacy fallback** in `decode_position` only if the legacy
//!    bare layout should now map to a different version (it should not: legacy
//!    entries are V1 shaped, so leave it mapping to `V1`).
//! 7. **Add a test** that writes a V1 entry and checks it reads back as the
//!    new struct with the default value (see `test_versioned_storage.rs`).
//!
//! Rules: never reorder or reuse a variant, never change a frozen `*V1`/`*V2`
//! struct, and only ever add variants at the end. Reading code needs no change
//! beyond `normalize`, because callers only see the latest struct.
//!
//! # Compatibility notes
//!
//! These enums and the `*V1` structs are storage-only. The public ABI is
//! unchanged: entrypoints still take and return `StakePosition`,
//! `GovernanceProposal` and `VeBoostLock`.

use soroban_sdk::{contracttype, Env, IntoVal, TryFromVal, Val, Vec};

use crate::storage::{GovernanceProposal, ProposableParam, StakePosition, VeBoostLock, VoteWeightMode};

// ── StakePosition ────────────────────────────────────────────────────────────

/// Frozen V1 layout of a stake position. Do not edit; see the module docs.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct StakePositionV1 {
    /// Position size in token units.
    pub amount: i128,
    /// Ledger at which the position was first opened.
    pub staked_at_ledger: u32,
    /// Ledger of the most recent reward claim.
    pub last_claim_ledger: u32,
}

/// Versioned storage form of [`StakePosition`].
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum VersionedPosition {
    /// Original layout.
    V1(StakePositionV1),
}

impl VersionedPosition {
    /// Convert any stored version to the latest in-memory [`StakePosition`].
    pub fn normalize(self) -> StakePosition {
        match self {
            VersionedPosition::V1(p) => StakePosition {
                amount: p.amount,
                staked_at_ledger: p.staked_at_ledger,
                last_claim_ledger: p.last_claim_ledger,
            },
        }
    }

    /// Wrap the latest in-memory struct in the newest stored version.
    pub fn wrap(position: &StakePosition) -> Self {
        VersionedPosition::V1(StakePositionV1 {
            amount: position.amount,
            staked_at_ledger: position.staked_at_ledger,
            last_claim_ledger: position.last_claim_ledger,
        })
    }
}

/// Decode one stored position: versioned form first, then the legacy bare
/// struct written before versioning.
pub fn decode_position(env: &Env, raw: Val) -> Option<StakePosition> {
    if let Ok(v) = VersionedPosition::try_from_val(env, &raw) {
        return Some(v.normalize());
    }
    StakePosition::try_from_val(env, &raw).ok()
}

/// Decode a stored list of positions, skipping nothing: entries that decode
/// in neither form are dropped.
pub fn decode_positions(env: &Env, raw: Vec<Val>) -> Vec<StakePosition> {
    let mut out = Vec::new(env);
    for item in raw.iter() {
        if let Some(p) = decode_position(env, item) {
            out.push_back(p);
        }
    }
    out
}

/// Encode positions in the newest versioned form for storage.
pub fn encode_positions(env: &Env, positions: &Vec<StakePosition>) -> Vec<VersionedPosition> {
    let mut out = Vec::new(env);
    for p in positions.iter() {
        out.push_back(VersionedPosition::wrap(&p));
    }
    out
}

// ── GovernanceProposal ───────────────────────────────────────────────────────

/// Frozen V1 layout of a governance proposal. Do not edit; see the module docs.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct GovernanceProposalV1 {
    /// Proposal id.
    pub id: u32,
    /// Parameter the proposal would change.
    pub parameter: ProposableParam,
    /// Proposed new value.
    pub new_value: i128,
    /// Total weight voted in favour.
    pub votes_for: i128,
    /// Total weight voted against.
    pub votes_against: i128,
    /// Ledger at which voting ends.
    pub ends_at: u32,
    /// Whether the proposal has been enacted.
    pub enacted: bool,
    /// Vote weighting mode fixed at creation.
    pub weight_mode: VoteWeightMode,
}

/// Versioned storage form of [`GovernanceProposal`].
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum VersionedProposal {
    /// Original layout.
    V1(GovernanceProposalV1),
}

impl VersionedProposal {
    /// Convert any stored version to the latest in-memory [`GovernanceProposal`].
    pub fn normalize(self) -> GovernanceProposal {
        match self {
            VersionedProposal::V1(p) => GovernanceProposal {
                id: p.id,
                parameter: p.parameter,
                new_value: p.new_value,
                votes_for: p.votes_for,
                votes_against: p.votes_against,
                ends_at: p.ends_at,
                enacted: p.enacted,
                weight_mode: p.weight_mode,
            },
        }
    }

    /// Wrap the latest in-memory struct in the newest stored version.
    pub fn wrap(proposal: &GovernanceProposal) -> Self {
        VersionedProposal::V1(GovernanceProposalV1 {
            id: proposal.id,
            parameter: proposal.parameter,
            new_value: proposal.new_value,
            votes_for: proposal.votes_for,
            votes_against: proposal.votes_against,
            ends_at: proposal.ends_at,
            enacted: proposal.enacted,
            weight_mode: proposal.weight_mode,
        })
    }
}

/// Decode one stored proposal: versioned form first, then the legacy bare
/// struct written before versioning.
pub fn decode_proposal(env: &Env, raw: Val) -> Option<GovernanceProposal> {
    if let Ok(v) = VersionedProposal::try_from_val(env, &raw) {
        return Some(v.normalize());
    }
    GovernanceProposal::try_from_val(env, &raw).ok()
}

/// Decode a stored list of proposals; entries that decode in neither form are
/// dropped.
pub fn decode_proposals(env: &Env, raw: Vec<Val>) -> Vec<GovernanceProposal> {
    let mut out = Vec::new(env);
    for item in raw.iter() {
        if let Some(p) = decode_proposal(env, item) {
            out.push_back(p);
        }
    }
    out
}

/// Encode proposals in the newest versioned form for storage.
pub fn encode_proposals(env: &Env, proposals: &Vec<GovernanceProposal>) -> Vec<VersionedProposal> {
    let mut out = Vec::new(env);
    for p in proposals.iter() {
        out.push_back(VersionedProposal::wrap(&p));
    }
    out
}

// ── VeBoostLock ──────────────────────────────────────────────────────────────

/// Frozen V1 layout of a vote-escrow boost lock. Do not edit; see the module
/// docs.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct VeBoostLockV1 {
    /// Locked token amount.
    pub amount: i128,
    /// Ledger at which the lock can be released.
    pub unlock_ledger: u32,
    /// Lock duration in ledgers.
    pub duration_ledgers: u32,
}

/// Versioned storage form of [`VeBoostLock`].
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum VersionedLock {
    /// Original layout.
    V1(VeBoostLockV1),
}

impl VersionedLock {
    /// Convert any stored version to the latest in-memory [`VeBoostLock`].
    pub fn normalize(self) -> VeBoostLock {
        match self {
            VersionedLock::V1(l) => VeBoostLock {
                amount: l.amount,
                unlock_ledger: l.unlock_ledger,
                duration_ledgers: l.duration_ledgers,
            },
        }
    }

    /// Wrap the latest in-memory struct in the newest stored version.
    pub fn wrap(lock: &VeBoostLock) -> Self {
        VersionedLock::V1(VeBoostLockV1 {
            amount: lock.amount,
            unlock_ledger: lock.unlock_ledger,
            duration_ledgers: lock.duration_ledgers,
        })
    }
}

/// Decode one stored lock: versioned form first, then the legacy bare struct
/// written before versioning.
pub fn decode_lock(env: &Env, raw: Val) -> Option<VeBoostLock> {
    if let Ok(v) = VersionedLock::try_from_val(env, &raw) {
        return Some(v.normalize());
    }
    VeBoostLock::try_from_val(env, &raw).ok()
}

// ── Typed storage access ─────────────────────────────────────────────────────

/// Read a single position stored under `key` in persistent storage.
pub fn read_position<K: IntoVal<Env, Val>>(env: &Env, key: &K) -> Option<StakePosition> {
    let raw: Val = env.storage().persistent().get(key)?;
    decode_position(env, raw)
}

/// Read a proposal stored under `key` in persistent storage.
pub fn read_proposal<K: IntoVal<Env, Val>>(env: &Env, key: &K) -> Option<GovernanceProposal> {
    let raw: Val = env.storage().persistent().get(key)?;
    decode_proposal(env, raw)
}

/// Write a proposal under `key` in persistent storage, in versioned form.
pub fn write_proposal<K: IntoVal<Env, Val>>(env: &Env, key: &K, proposal: &GovernanceProposal) {
    env.storage()
        .persistent()
        .set(key, &VersionedProposal::wrap(proposal));
}

/// Read a lock stored under `key` in persistent storage.
pub fn read_lock<K: IntoVal<Env, Val>>(env: &Env, key: &K) -> Option<VeBoostLock> {
    let raw: Val = env.storage().persistent().get(key)?;
    decode_lock(env, raw)
}

/// Write a lock under `key` in persistent storage, in versioned form.
pub fn write_lock<K: IntoVal<Env, Val>>(env: &Env, key: &K, lock: &VeBoostLock) {
    env.storage()
        .persistent()
        .set(key, &VersionedLock::wrap(lock));
}
