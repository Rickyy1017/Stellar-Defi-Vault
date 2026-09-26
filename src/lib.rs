#![no_std]

mod admin;
mod balance;
mod errors;
mod events;
mod ledger_boundary;
#[cfg(not(feature = "vault-wasm"))]
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
pub use vault::vault_extensions_463_466; // issues #463-#466 — clawback, NFT boost, milestone, param log // child of `vault` — see the note at the bottom of vault.rs
pub use vault::vault_extensions_538_541; // issues #538-#541 — version, token fee, rate ramp, memo // child of `vault` — see the note at the bottom of vault.rs
pub use vault::vault_extensions_542_545; // child of `vault` — see the note at the bottom of vault.rs
pub use vault::vault_extensions_498_501; // issues #498-#501 // child of `vault` — see the note at the bottom of vault.rs
pub use vault::vault_extensions_502_505; // issues #502-#505 — withdrawal queue, timelock, compound, tokenize // issues #542-#545 — seed liquidity, APY history, low-balance alert, notifications // child of `vault` — see the note at the bottom of vault.rs
pub mod minimum_unstake_amount; // issue #441 — minimum unstake amount
pub mod reward_token_audit_trail; // issue #467 — reward token audit trail
pub mod stake_funded_bug_bounty; // issue #468 — stake-funded bug bounty
pub mod cross_pool_identity; // issue #470 — cross-pool identity
pub mod position_value_appreciation_log; // issue #469 — position value appreciation log
pub use vault::position_health_auto_recovery; // issue #459 — position health auto-recovery // child of `vault` — see the note at the bottom of vault.rs
pub use vault::lockdrop_campaign; // issue #460 — lockdrop campaign // child of `vault` — see the note at the bottom of vault.rs
pub use vault::proof_of_humanity_hook; // issue #461 — proof-of-humanity hook // child of `vault` — see the note at the bottom of vault.rs
pub use vault::roadmap_voting; // issue #462 — roadmap voting // child of `vault` — see the note at the bottom of vault.rs
pub use vault::staker_region_tag; // issue #430 — voluntary staker region tags // child of `vault` — see the note at the bottom of vault.rs
pub use vault::staker_network_graph; // issue #456 — staker delegation/referral/mirror network graph // child of `vault` — see the note at the bottom of vault.rs
pub use vault::staker_favor_rounding; // issue #457 — always round in the staker's favor // child of `vault` — see the note at the bottom of vault.rs
pub use vault::daily_community_tip; // issue #458 — daily stake-weighted featured tip vote // child of `vault` — see the note at the bottom of vault.rs
pub use vault::time_locked_admin_proposal; // issue #455 — time-locked admin config-change announcements // child of `vault` — see the note at the bottom of vault.rs
pub use vault::meta_staking; // meta-staking layer — restake reward tokens for a bonus meta-reward rate // child of `vault` — see the note at the bottom of vault.rs
pub use vault::batch_vote; // governance batch voting (issue #160) // child of `vault` — see the note at the bottom of vault.rs
pub use vault::daily_withdrawal_limit; // issue #554 — per-user rolling 24h withdrawal limit // child of `vault` — see the note at the bottom of vault.rs
pub mod position_multiplier; // issue #534 — per-position custom reward multiplier
pub mod inactivity_decay; // issue #536 — configurable inactivity-based reward decay
pub mod vault_extensions_546_549; // issues #546-#549 — positions cap, precision, large-deposit lock, min funding

// Issues #526-#529: scheduled exit, snapshot airdrop, external price oracle, co-sponsor.
pub use vault::scheduled_exit; // issue #526 — scheduled self-withdrawal // child of `vault` — see the note at the bottom of vault.rs
pub use vault::snapshot_airdrop; // issue #527 — snapshot-based airdrop distribution // child of `vault` — see the note at the bottom of vault.rs
pub use vault::external_price_oracle; // issue #528 — external price oracle for collateral valuation // child of `vault` — see the note at the bottom of vault.rs
pub use vault::co_sponsor; // issue #529 — third-party reward matching via co-sponsors // child of `vault` — see the note at the bottom of vault.rs

// Issues #530-#533.
pub use vault::pause_grace_period; // issue #533 — max pause duration + forced unpause // child of `vault` — see the note at the bottom of vault.rs
pub use vault::reward_rate_ceiling; // issue #532 — lower-only max reward rate ceiling // child of `vault` — see the note at the bottom of vault.rs
pub use vault::invariants; // issue #531 — core accounting invariant checker // child of `vault` — see the note at the bottom of vault.rs
pub use vault::activity_log; // issue #530 — per-user deposit/withdrawal history // child of `vault` — see the note at the bottom of vault.rs

// Pool insights, reward-runway guard, and time-delayed admin recovery.
pub use vault::pool_insights; // pool summary + rounding-policy transparency // child of `vault` — see the note at the bottom of vault.rs
pub use vault::runway_guard; // set_reward_rate_bps runway safety rail // child of `vault` — see the note at the bottom of vault.rs
pub use vault::admin_recovery; // long-delay admin key-loss recovery // child of `vault` — see the note at the bottom of vault.rs

// Pre-existing modules that `vault.rs` already calls into (e.g. `do_unstake`'s
// `community_treasury::route_fee_revenue` / `position_mirroring::maybe_mirror_action`)
// but that were never actually declared here, leaving `main` unable to compile
// before this PR. Wired in as a prerequisite to building/testing #554's change,
// not part of #554 itself.
pub mod claim_fee;
pub mod community_treasury;
pub mod mev_claim_protection;
pub mod peg_stabilization;
pub use vault::position_mirroring; // child of `vault` — see the note at the bottom of vault.rs

// More pre-existing modules that `vault.rs` / `runway_guard.rs` /
// `vault_extensions_542_545.rs` call into but that were never declared here,
// leaving `main` unable to compile. `access_roles` (issue #513) additionally
// backs `dynamic_reward_rate` (issue #510). Wired in as a prerequisite to
// building/testing the issue #593 error-handling refactor.
pub use vault::access_roles; // issue #513 — role-based access control // child of `vault` — see the note at the bottom of vault.rs
pub use vault::dynamic_reward_rate; // issue #510 — utilization-driven reward rate // child of `vault` — see the note at the bottom of vault.rs
pub use vault::keeper_registry; // admin-approved keeper registry // child of `vault` — see the note at the bottom of vault.rs
pub mod transfer_safety; // issue #512 — fee-on-transfer token safety

#[cfg(not(feature = "vault-wasm"))]
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

#[cfg(test)]
mod test_top_depositors; // issue #523 — get_top_depositors leaderboard query

#[cfg(test)]
mod test_issue_524; // issue #524 — configurable reward payout token

#[cfg(test)]
mod test_issue_522; // issue #522 — get_rate_history() rate-change changelog

#[cfg(test)]
mod test_issue_525; // issue #525 — graceful pool sunset via initiate_sunset()

#[cfg(test)]
mod test_issue_497; // issue #497 — paused break-glass principal withdrawal

#[cfg(test)]
mod test_issues_589_590_592;

#[cfg(test)]
mod test_issues_605_608;

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
#[cfg(test)]
mod test_issues_498_501;
#[cfg(test)]
mod test_issues_502_505;

#[cfg(test)]
mod test_issue_593; // issue #593 — typed error codes replace ad-hoc panics

#[cfg(test)]
mod test_issue_621_negative_balance; // issue #621 — negative-balance impossibility proof

#[cfg(test)]
mod test_issue_622_contractmeta; // issue #622 — contractmeta! name/version/description

#[cfg(test)]
mod test_issue_623_scale; // issue #623 — max realistic depositor count performance
