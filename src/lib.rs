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
pub mod minimum_unstake_amount; // issue #441 — minimum unstake amount
pub mod reward_token_audit_trail; // issue #467 — reward token audit trail
pub mod stake_funded_bug_bounty; // issue #468 — stake-funded bug bounty
pub mod cross_pool_identity; // issue #470 — cross-pool identity
pub mod position_value_appreciation_log; // issue #469 — position value appreciation log

pub use nft::StakeReceiptNFT;
pub use vault::VaultContract;

#[cfg(test)]
mod test_issues_467_470;
