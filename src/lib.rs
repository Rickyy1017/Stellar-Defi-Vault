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


// Features added as their own modules rather than inside `vault.rs`. Soroban
// supports several `#[contractimpl]` blocks for one contract type, and
// `nft.rs` already establishes the pattern, so each of these keeps its storage
// keys, types, and entrypoints together instead of appending to a 25k-line
// file. `DataKey` is at Soroban's 50-variant cap for `#[contracttype]` enums,
// so all of them use raw `Symbol`-keyed storage as `balance.rs` does.
pub mod vesting_cliff; // issue #287 — reward vesting cliff
pub mod vault_extensions_463_466; // issues #463-#466 — clawback, NFT boost, milestone, param log
pub mod minimum_unstake_amount; // issue #441 — minimum unstake amount
pub mod reward_token_audit_trail; // issue #467 — reward token audit trail
pub mod stake_funded_bug_bounty; // issue #468 — stake-funded bug bounty
pub mod cross_pool_identity; // issue #470 — cross-pool identity
pub mod position_value_appreciation_log; // issue #469 — position value appreciation log
pub mod position_health_auto_recovery; // issue #459 — position health auto-recovery
pub mod lockdrop_campaign; // issue #460 — lockdrop campaign
pub mod proof_of_humanity_hook; // issue #461 — proof-of-humanity hook
pub mod roadmap_voting; // issue #462 — roadmap voting
pub mod staker_region_tag; // issue #430 — voluntary staker region tags
pub mod staker_network_graph; // issue #456 — staker delegation/referral/mirror network graph
pub mod staker_favor_rounding; // issue #457 — always round in the staker's favor
pub mod daily_community_tip; // issue #458 — daily stake-weighted featured tip vote
pub mod time_locked_admin_proposal; // issue #455 — time-locked admin config-change announcements
pub mod meta_staking; // meta-staking layer — restake reward tokens for a bonus meta-reward rate
pub mod batch_vote; // governance batch voting (issue #160)
pub mod daily_withdrawal_limit; // issue #554 — per-user rolling 24h withdrawal limit
pub mod position_multiplier; // issue #534 — per-position custom reward multiplier
pub mod inactivity_decay; // issue #536 — configurable inactivity-based reward decay

// Issues #526-#529: scheduled exit, snapshot airdrop, external price oracle, co-sponsor.
pub mod scheduled_exit; // issue #526 — scheduled self-withdrawal
pub mod snapshot_airdrop; // issue #527 — snapshot-based airdrop distribution
pub mod external_price_oracle; // issue #528 — external price oracle for collateral valuation
pub mod co_sponsor; // issue #529 — third-party reward matching via co-sponsors

// Pool insights, reward-runway guard, and time-delayed admin recovery.
pub mod pool_insights; // pool summary + rounding-policy transparency
pub mod runway_guard; // set_reward_rate_bps runway safety rail
pub mod admin_recovery; // long-delay admin key-loss recovery

// Pre-existing modules that `vault.rs` already calls into (e.g. `do_unstake`'s
// `community_treasury::route_fee_revenue` / `position_mirroring::maybe_mirror_action`)
// but that were never actually declared here, leaving `main` unable to compile
// before this PR. Wired in as a prerequisite to building/testing #554's change,
// not part of #554 itself.
pub mod claim_fee;
pub mod community_treasury;
pub mod mev_claim_protection;
pub mod peg_stabilization;
pub mod position_mirroring;

pub use nft::StakeReceiptNFT;
pub use vault::VaultContract;

// Stale legacy test files from prior unmerged branches disabled; they call
// methods that no longer exist on the contract.
// #[cfg(test)]
// mod test;
// #[cfg(test)]
// mod test_content_curation;
// #[cfg(test)]
// mod test_integration;
// #[cfg(test)]
// mod test_nft_fractionalize;
// #[cfg(test)]
// mod test_reputation_decay;
// #[cfg(test)]
// mod test_validator_rewards;
// #[cfg(test)]
// mod test_features_287_290;

#[cfg(test)]
mod test_issues_568_571;

#[cfg(test)]
mod test_issues_526_529;

// #[cfg(test)]
// mod test_issues_463_466;
// #[cfg(test)]
// mod test_issues_467_470;
// #[cfg(test)]
// mod test_issues_459_462;
// #[cfg(test)]
// mod test_issue_554;
// #[cfg(test)]
// mod test_staker_region_tag;
