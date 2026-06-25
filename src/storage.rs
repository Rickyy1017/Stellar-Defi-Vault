use soroban_sdk::{contracttype, Address, Vec};

/// Storage keys for all persistent and instance state in the vault.
///
/// Instance keys (fast, small): Admin, Token, TotalShares, TotalDeposited,
/// MinStake, RewardRateBps, RewardPoolBalance, BoostSchedule, Paused,
/// WithdrawalLimit, LockPeriod, EarlyExitPenaltyBps, TotalStakers,
/// TotalRewardsPaid, SlashTreasury, WhitelistEnabled, CooldownPeriod,
/// PoolCap, ClaimCap, ClaimCapWindow, StakeDecimals, RewardDecimals,
/// UnstakeFeeBps, AllStakers, InactivityThreshold.
///
/// Persistent keys (per-user, long-lived): ShareBalance, StakeHistory,
/// RewardCheckpointLedger, LastClaimLedger, AccruedReward, StakedAtLedger,
/// Delegate, Whitelisted, UnbondingPosition, UserClaimWindow, FrozenAt.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Token,
    TotalShares,
    TotalDeposited,
    MinStake,
    RewardRateBps,
    RewardPoolBalance,
    BoostSchedule,
    ShareBalance(Address),
    StakeHistory(Address),
    RewardCheckpointLedger(Address),
    LastClaimLedger(Address),
    AccruedReward(Address),
    Paused,
    WithdrawalLimit,
    LockPeriod,
    EarlyExitPenaltyBps,
    StakedAtLedger(Address),
    TotalStakers,
    TotalRewardsPaid,
    Delegate(Address),
    // Issue #39: rescue token
    RewardToken,
    // Issue #40: NFT receipt
    NftContract,
    // Issue #41: restake grace window
    RestakeWindow,
    LastUnstakeLedger(Address),
    Restaked(Address),
    // Issue #42: admin action audit log
    AdminActionCount,
    // Keys used throughout vault.rs / balance.rs
    SlashTreasury,
    WhitelistEnabled,
    Whitelisted(Address),
    CooldownPeriod,
    UnbondingPosition(Address),
    RewardRemainder(Address),
    UserClaimWindow(Address),
    StakeDecimals,
    RewardDecimals,
    UnstakeFeeBps,
    AllStakers,
    ClaimCap,
    ClaimCapWindow,
    RateHistory,
    BoostCampaign,
    Leaderboard,
    LeaderboardSize,
    // Issue #101: frozen positions
    InactivityThreshold,
    FrozenAt(Address),
}

/// Issue #42: enum of all admin actions for the audit log.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum AdminAction {
    SetRewardRate,
    Pause,
    Unpause,
    TransferAdmin,
    SetLockPeriod,
    SetCap,
    Slash,
    RescueToken,
    SetEarlyExitPenalty,
    SetMinStake,
    FundRewardPool,
    AddYield,
    SetBoostSchedule,
    SetNftContract,
    SetRestakeWindow,
    SetRewardToken,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct UnbondingPosition {
    pub amount: i128,
    pub unbonding_since: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct VaultState {
    pub total_shares: i128,
    pub total_deposited: i128,
    pub paused: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PoolStats {
    pub total_staked: i128,
    pub total_stakers: u32,
    pub reward_rate_bps: i128,
    pub reward_token_balance: i128,
    pub paused: bool,
    pub total_rewards_paid: i128,
}

/// Aggregate user stats used by `user_stats`.
///
/// - `position_amount`: the user's current position size expressed in token units.
/// - `pending_reward`: rewards accrued but not yet claimed.
/// - `staked_at_ledger`: the ledger sequence when the position was first opened.
/// - `last_claim_ledger`: the most recent ledger at which rewards were claimed.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct UserStats {
    pub position_amount: i128,
    pub pending_reward: i128,
    pub staked_at_ledger: u32,
    pub last_claim_ledger: u32,
}

/// Active boost campaign set by admin (#48).
///
/// - `multiplier_bps`: reward multiplier stacked on top of tier multipliers (10000 = 1x).
/// - `starts_at_ledger`: ledger when the campaign was activated.
/// - `ends_at_ledger`: ledger after which the campaign no longer applies.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CampaignInfo {
    pub multiplier_bps: u32,
    pub starts_at_ledger: u32,
    pub ends_at_ledger: u32,
}

/// A single entry in the staking leaderboard (#46).
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct LeaderboardEntry {
    pub staker: Address,
    pub amount: i128,
}

/// Type alias for the leaderboard vector used in storage and queries.
pub type Leaderboard = Vec<LeaderboardEntry>;

/// Current stake position for a user.
///
/// - `amount`: the user's current position size expressed in token units.
/// - `staked_at_ledger`: the ledger sequence when the position was first opened.
/// - `last_claim_ledger`: the most recent ledger at which rewards were claimed.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct StakePosition {
    pub amount: i128,
    pub staked_at_ledger: u32,
    pub last_claim_ledger: u32,
}

/// Snapshot of all pool-level configuration returned by `get_pool_config`.
///
/// Allows frontends to fetch all settings in a single RPC call instead of
/// querying each key individually.
///
/// - `admin`: current admin address.
/// - `stake_token`: token accepted for staking and used to pay rewards.
/// - `reward_token`: same as `stake_token` (single-token vault).
/// - `reward_rate_bps`: annual reward rate in basis points.
/// - `paused`: whether deposits and withdrawals are currently paused.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PoolConfig {
    pub admin: Address,
    pub stake_token: Address,
    pub reward_token: Address,
    pub reward_rate_bps: u32,
    pub paused: bool,
}

/// Per-user reward claim window used to enforce the optional claim cap.
///
/// - `claimed_in_window`: cumulative rewards claimed by this user in the current window.
/// - `window_started_at`: ledger sequence at which the current window began.
///
/// The window resets automatically when `current_ledger > window_started_at + window_ledgers`.
/// Any unclaimed remainder is deferred to the next window — it is not lost.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ClaimWindow {
    pub claimed_in_window: i128,
    pub window_started_at: u32,
}

/// Aggregated user state returned by `user_summary` (issue #103).
///
/// - `position`: current stake position, or `None` if user has no stake.
/// - `pending_reward`: rewards accrued but not yet claimed.
/// - `pool_share_bps`: user's share of the total pool in basis points (10000 = 100%).
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct UserSummary {
    pub position: Option<StakePosition>,
    pub pending_reward: i128,
    pub pool_share_bps: i128,
}
