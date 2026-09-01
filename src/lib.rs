#![no_std]

mod admin;
mod balance;
mod errors;
mod events;
pub mod example_consumer;
pub mod interface;
pub mod nft;
mod storage;
pub mod vault;

pub mod commitment;
pub mod content_curation;
pub mod insurance;
pub mod nft_fractionalize;
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
pub mod cross_pool_identity; // issue #470 — cross-pool identity
pub mod daily_community_tip; // issue #458 — daily stake-weighted featured tip vote
pub mod lockdrop_campaign; // issue #460 — lockdrop campaign
pub mod minimum_unstake_amount; // issue #441 — minimum unstake amount
pub mod peg_stabilization;
pub mod position_health_auto_recovery; // issue #459 — position health auto-recovery
pub mod position_value_appreciation_log; // issue #469 — position value appreciation log
pub mod proof_of_humanity_hook; // issue #461 — proof-of-humanity hook
pub mod reward_token_audit_trail; // issue #467 — reward token audit trail
pub mod roadmap_voting; // issue #462 — roadmap voting
pub mod stake_funded_bug_bounty; // issue #468 — stake-funded bug bounty
pub mod staker_favor_rounding; // issue #457 — always round in the staker's favor
pub mod staker_network_graph; // issue #456 — staker delegation/referral/mirror network graph
pub mod time_locked_admin_proposal; // issue #455 -- time-locked admin config-change announcements
pub mod vault_extensions_463_466; // issues #463-#466 — clawback, NFT boost, milestone, param log
pub mod vesting_cliff; // issue #287 — reward vesting cliff // reward-token price peg stabilization

pub use nft::StakeReceiptNFT;
pub use vault::VaultContract;

#[cfg(test)]
mod test;

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
mod test_issues_467_470;

#[cfg(test)]
mod test_issues_459_462;

#[cfg(test)]
mod test_peg_stabilization;
