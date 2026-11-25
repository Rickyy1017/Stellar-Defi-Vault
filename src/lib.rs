//! Stellar DeFi Vault: a non-custodial yield vault built on Soroban.
//!
//! The canonical public API is the [`interface::VaultTrait`] trait, implemented
//! by [`vault::VaultContract`]. Entrypoint docs follow a common layout:
//! a summary, then `# Authorization`, `# Errors` and `# Events` sections
//! (issue #675).
#![no_std]
// Issue #658: the contract has never needed `unsafe`, and a deployed Soroban
// contract must not. Forbidding it crate-wide also keeps unsafe blocks out of
// the test-only code that shares this crate.
#![forbid(unsafe_code)]
// Issue #922: contract code must use typed error handling. Test code is excluded
// because test helpers may deliberately exercise panic behavior.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing))]
// Issue #675: every public item should be documented. Left at `warn` while the
// existing ~1,000 undocumented items are worked through; flip to `deny` once
// `cargo doc` is clean.
#![warn(missing_docs)]

mod admin;
mod balance;
mod constants;
pub mod claim_receipts;
mod errors;
mod events;
pub mod example_consumer;
pub mod interface;
mod math;
pub mod nft;
pub mod reward_accumulator;
pub mod reward_fee;
mod reward_math;
mod storage;
pub mod ttl_management;
pub mod user_ttl;
pub mod vault;
pub mod vec_unique;

pub mod commitment;
pub mod content_curation;
pub mod insurance;
pub mod nft_fractionalize;
#[cfg(feature = "mirroring")]
pub mod position_mirroring;
pub mod price_oracle;
pub mod reputation_decay;
pub mod stake_quota;

// Features added as their own modules rather than inside `vault.rs`. Soroban
// supports several `#[contractimpl]` blocks for one contract type, and
// `nft.rs` already establishes the pattern, so each of these keeps its storage
// keys, types, and entrypoints together instead of appending to a 25k-line
// file. `DataKey` is at Soroban's 50-variant cap for `#[contracttype]` enums,
// so all of them use raw `Symbol`-keyed storage as `balance.rs` does.
pub mod balance_checkpoints; // issues #709, #710 — balance checkpoints and TWAB
pub mod cross_pool_identity; // issue #470 — cross-pool identity
pub mod daily_community_tip; // issue #458 — daily stake-weighted featured tip vote
pub mod deposit_and_lock; // issue #747 — deposit_and_lock() atomic deposit + lock
pub mod gauge_incentives; // issue #936 — bribe and incentive market for gauge votes
pub mod governance_params; // issue #712 — governance quorum and pass threshold
pub mod lockdrop_campaign; // issue #460 — lockdrop campaign
pub mod minimum_unstake_amount; // issue #441 — minimum unstake amount
pub mod peg_stabilization;
pub mod position_health_auto_recovery; // issue #459 â€” position health auto-recovery
pub mod position_value_appreciation_log; // issue #469 â€” position value appreciation log
pub mod proof_of_humanity_hook; // issue #461 â€” proof-of-humanity hook
pub mod reward_token_audit_trail; // issue #467 â€” reward token audit trail
#[cfg(feature = "governance")]
pub mod roadmap_voting; // issue #462 â€” roadmap voting
pub mod stake_funded_bug_bounty; // issue #468 â€” stake-funded bug bounty
pub mod stake_hooks; // issue #711 â€” stake and unstake hook contracts
pub mod staker_favor_rounding; // issue #457 â€” always round in the staker's favor
#[cfg(any(feature = "referrals", feature = "mirroring"))]
pub mod staker_network_graph; // issue #456 â€” staker delegation/referral/mirror network graph
pub mod time_locked_admin_proposal; // issue #455 -- time-locked admin config-change announcements
pub mod vault_extensions_463_466; // issues #463-#466 â€” clawback, NFT boost, milestone, param log
#[cfg(feature = "strategies")]
pub mod vault_extensions_765_768; // issues #765-#768 â€” idle buffer, harvest, loss policy, emergency unwind
pub mod vesting_cliff; // issue #287 â€” reward vesting cliff // reward-token price peg stabilization

pub use nft::StakeReceiptNFT;
pub use vault::VaultContract;

#[cfg(test)]
mod test_issues_751;
#[cfg(test)]
mod test_issues_757_760;

#[cfg(test)]
mod test_admin_contract_auth;

#[cfg(test)]
mod test_passkey_account;

#[cfg(test)]
mod test_withdrawal_cooldown;

#[cfg(test)]
mod test;

#[cfg(test)]
mod test_share_decimals;

#[cfg(test)]
mod test_issues_628_631;

#[cfg(test)]
mod test_content_curation;

#[cfg(test)]
mod test_integration;

#[cfg(test)]
mod test_nft_fractionalize;

#[cfg(test)]
mod test_reputation_decay;

#[cfg(test)]
mod test_validator_rewards;

#[cfg(test)]
mod test_features_287_290;

#[cfg(test)]
mod test_issues_463_466;

#[cfg(test)]
mod test_issues_701_704;

// Issue #658: this module was missing its `#[cfg(test)]` gate, so its test
// helpers (and their `testutils` dependencies) were compiled into the release
// WASM. Every test module must stay behind `#[cfg(test)]`.
#[cfg(test)]
mod test_issues_467_470;

#[cfg(test)]
mod test_issues_697_700;

#[cfg(test)]
mod test_issues_459_462;

#[cfg(test)]
mod test_peg_stabilization;

#[cfg(test)]
#[cfg(feature = "governance")]
mod test_issues_709_712;

// â”€â”€ Issue #659: storage-layout upgrade-safety snapshot â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
#[cfg(test)]
mod test_storage_layout;

#[cfg(test)]
mod test_error_codes;

// â”€â”€ Issue #660: resource-footprint limits for batch/loop entrypoints â”€â”€â”€â”€â”€â”€â”€â”€â”€
#[cfg(test)]
mod test_resource_footprint;

#[cfg(test)]
mod test_issues_646_650_737_738;

#[cfg(test)]
mod test_token_interface_compliance;

#[cfg(test)]
mod test_vec_storage_capacity;

#[cfg(test)]
mod test_issues_765_768;

/// Pure math surfaces exposed for property tests and `cargo-fuzz` (issue #661).
///
/// Gated so it never lands in the deployed WASM: the fuzz crate enables the
/// `testutils` feature, and in-crate tests get it via `cfg(test)`.
#[cfg(any(test, feature = "testutils"))]
pub mod fuzz_support {
    pub use crate::balance::{
        amount_to_shares, amount_to_shares_with_decimals, shares_to_amount,
        shares_to_amount_with_decimals,
    };
    pub use crate::math::{mul_div_ceil, mul_div_floor, MathError};
    pub use crate::reward_math::reward_for_ledgers;
}
#[cfg(test)]
mod test_resource_costs;

#[cfg(test)]
mod test_runbook_interface_snapshot;

#[cfg(test)]
mod test_feature_gating_matrix;

#[cfg(test)]
mod test_gauge_incentives;

#[cfg(test)]
mod test_issue_919;
