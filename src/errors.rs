use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VaultError {
    /// Returned by initialize-dependent getters and stake/unstake flows when
    /// the admin, token, or other required contract state has not been stored yet.
    NotInitialized = 1,
    /// Returned by initialize() when the vault has already been initialized.
    AlreadyInitialized = 2,
    /// Returned by admin-only entrypoints that call `admin::require_admin()`
    /// and by `rescue_token()` / `slash()` when the supplied admin address does
    /// not match the stored admin.
    Unauthorized = 3,
    /// Returned by staking, unstaking, and amount-setting calls when a
    /// caller supplies zero or a negative amount where that is not allowed.
    ZeroAmount = 4,
    /// Returned by `withdraw()`, `unstake()`, and `unstake_all()` when the
    /// caller tries to burn more shares than they own.
    InsufficientShares = 5,
    /// Returned by staking, unstaking, and admin-yield entrypoints that require
    /// the pool to be unpaused.
    VaultPaused = 6,
    /// Reserved for token-validation failures during initialization or future
    /// token checks; no current public function returns this variant.
    InvalidToken = 7,
    /// Returned by staking, unstaking, claim, slash, preview, and reward math
    /// helpers when checked arithmetic or share conversion fails.
    ArithmeticError = 8,
    /// Returned by `withdraw()`, `unstake()`, and `unstake_all()` when the
    /// requested share amount exceeds the configured per-transaction limit.
    WithdrawalLimitExceeded = 9,
    /// Returned by `set_early_exit_penalty_bps()` when the admin sets a value
    /// above the supported cap.
    InvalidPenaltyBps = 10,
    /// Returned by `deposit()`, `stake()`, and `stake_for()` when the resulting
    /// position would fall below the configured minimum stake.
    BelowMinimumStake = 11,
    /// Returned by `set_boost_schedule()` when more than five boost tiers are
    /// supplied.
    TooManyBoostTiers = 12,
    /// Returned by `set_boost_schedule()` when a tier multiplier is below the
    /// base rate or the tier ledgers are not strictly increasing.
    InvalidBoostSchedule = 13,
    /// Returned by `claim()`, `stake_and_claim()`, and `claim_epoch_rewards()`
    /// when the reward pool does not hold enough tokens to pay the claim.
    InsufficientRewardPool = 14,
    /// Returned by `revoke_delegate()` when the caller revokes the wrong
    /// delegate, and by `stake_for()` when the caller is not the approved
    /// delegate for the beneficiary.
    NotADelegate = 15,
    /// Returned by `rescue_token()` when the admin tries to rescue the stake
    /// token itself.
    CannotRescueStakeToken = 16,
    /// Returned by `rescue_token()` when the admin tries to rescue the
    /// registered reward token.
    CannotRescueRewardToken = 17,
    /// Returned by position-dependent flows such as `unstake_all()`,
    /// `claimable_since()`, `position_age_ledgers()`, `time_since_last_claim()`,
    /// `request_unstake()`, `execute_unstake()`, `slash()`, `transfer_position()`,
    /// `merge_positions()`, and `flag_frozen()` when the user has no active
    /// stake or unbonding position.
    PositionNotFound = 18,
    /// Returned by `deposit()`, `stake()`, `stake_for()`, and `stake_and_claim()`
    /// when whitelist enforcement is enabled and the staker or beneficiary is
    /// not approved.
    NotWhitelisted = 19,
    /// Returned by `withdraw()` and `unstake()` when cooldown is enabled, and
    /// by `execute_unstake()` when the cooldown has not finished yet.
    UseCooldownFlow = 20,
    /// Returned by `set_unstake_fee_bps()` when the fee exceeds 500 bps (5%).
    UnstakeFeeTooHigh = 21,
    /// Returned by `batch_position_query()` when more than 20 addresses are supplied.
    BatchTooLarge = 22,
    /// Reserved for aggregate-claim or staker-count limit checks; no current
    /// public function returns this variant.
    TooManyStakers = 23,
    /// Returned by `transfer_position()` when the recipient already has an
    /// active staking position.
    RecipientAlreadyStaking = 24,
    /// Returned by `start_boost_campaign()` when a boost campaign is already active.
    CampaignAlreadyActive = 25,
    /// Returned by `end_boost_campaign()` when there is no active boost campaign
    /// to cancel.
    NoCampaignActive = 26,
    /// Returned by `set_leaderboard_size()` when the requested leaderboard cap
    /// exceeds 20.
    LeaderboardSizeTooLarge = 27,
    /// Returned by `view_all_positions()` when `page_size` is 0 or greater than 20.
    PageSizeTooLarge = 28,
    /// Returned by staking entrypoints when KYC enforcement is enabled and the
    /// staker is not approved.
    KycNotApproved = 29,
    /// Returned by `deposit()`, `stake()`, `stake_for()`, `stake_and_claim()`,
    /// `pause()`, and `unpause()` after `emergency_stop()` has permanently
    /// stopped the contract.
    ContractStopped = 30,
    /// Returned by staking entrypoints when the new deposit would exceed the
    /// configured pool cap.
    PoolCapReached = 31,
    /// Returned by `set_pool_description()` when the description exceeds 200
    /// characters.
    DescriptionTooLong = 32,
    /// Returned by `record_wave_activity()` when the supplied wave id is not
    /// greater than the last recorded wave.
    NonMonotonicWaveId = 33,
    /// Returned by `record_wave_activity()` when more than 50 active users are
    /// supplied in one call.
    TooManyActiveUsers = 34,
    /// Returned by `initialize()` when the admin or token address is invalid
    /// for this contract, such as matching the contract's own address.
    InvalidAddress = 35,
    /// Returned by `initialize()` and `set_reward_rate_bps()` when the reward
    /// APR exceeds the configured cap.
    RateTooHigh = 36,
    /// Returned by staking entrypoints when the user already holds the
    /// configured maximum number of active positions.
    MaxPositionsReached = 37,
    /// Returned by `set_max_positions_per_user()` when the requested cap exceeds 10.
    MaxPositionsTooHigh = 38,
    /// Reserved for future bulk-KYC updates; no current public function returns
    /// this variant.
    BatchKycTooLarge = 39,
    /// Reserved for future caller-supplied rate conversion flows; no current
    /// public function returns this variant.
    InvalidRate = 40,
    /// Custom error message exceeds MAX_ERROR_MESSAGE_LENGTH (150 characters).
    MessageTooLong = 41,

    /// Returned by epoch-mode entrypoints when the contract is in the wrong mode.
    EpochModeConflict = 41,
    /// Returned when a vesting queue already holds the maximum supported entries.
    VestingQueueFull = 42,
    /// Returned when a vesting withdrawal is requested but nothing has matured yet.
    NothingToWithdraw = 43,
    /// Returned when an epoch cannot be finalized because the configured window has not elapsed.
    EpochNotFinalized = 44,
    /// Caller is not an approved relayer for the target user (issue #118).
    RelayerNotApproved = 45,
    /// Caller is not on the yield source whitelist (issue #126).
    NotYieldSource = 46,
    /// notify_reward_added called with a zero or negative amount (issue #126).
    InvalidRewardAmount = 47,
    /// Reverts with NotInEpochMode error if pool is not configured for epoch mode.
    NotInEpochMode = 48,
}
