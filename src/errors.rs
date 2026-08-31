use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VaultError {
    NotInitialized = 1,
    /// Returned by initialize() when the vault has already been initialized,
    /// and by `enact_proposal()` when the proposal has already been enacted.
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
    /// when the reward pool does not hold enough tokens to pay the claim, and
    /// by `withdraw_from_yield()` when the requested amount exceeds what is
    /// currently tracked as deployed to the yield protocol.
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
    /// stake or unbonding position. Also returned by `create_proposal()` and
    /// `vote()` when the caller has no active position, and by `vote()` /
    /// `enact_proposal()` when the given proposal id does not exist.
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
    /// Returned by `vote()` when the caller has already voted on the given
    /// proposal.
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
    /// configured pool cap, and by `deploy_to_yield()` when the requested
    /// amount exceeds `available_for_yield()` (the 20% liquidity buffer).
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
    /// configured maximum number of active positions, and by
    /// `create_proposal()` when 10 governance proposals are already open.
    MaxPositionsReached = 37,
    /// Returned by `set_max_positions_per_user()` when the requested cap exceeds 10.
    MaxPositionsTooHigh = 38,
    /// Returned by `vote()` when the proposal's voting period has already
    /// ended, or the proposal has already been enacted. Also returned by
    /// `enact_proposal()` when the proposal has been vetoed (issue #241).
    BatchKycTooLarge = 39,
    /// Returned by `set_dynamic_fee_config()` when `base_fee_bps > max_fee_bps`
    /// or `utilization_threshold_bps` exceeds 10 000 (100%).
    InvalidRate = 40,
    /// Custom error message exceeds MAX_ERROR_MESSAGE_LENGTH (150 characters).
    MessageTooLong = 41,

    /// Returned by epoch-mode entrypoints when the contract is in the wrong mode.
    EpochModeConflict = 42,
    /// Returned when a vesting queue already holds the maximum supported entries.
    VestingQueueFull = 43,
    /// Returned when a vesting withdrawal is requested but nothing has matured yet.
    NothingToWithdraw = 44,
    /// Returned when an epoch cannot be finalized because the configured
    /// window has not elapsed, and by `enact_proposal()` when the voting
    /// period has not ended yet.
    EpochNotFinalized = 45,
    /// Caller is not an approved relayer for the target user (issue #118).
    RelayerNotApproved = 46,
    /// Caller is not on the yield source whitelist (issue #126).
    NotYieldSource = 47,
    /// notify_reward_added called with a zero or negative amount (issue #126).
    InvalidRewardAmount = 48,
    /// Returned when a new stake is attempted after `start_graceful_shutdown` has been called.
    PoolShuttingDown = 49,
    /// Reverts with NotInEpochMode error if pool is not configured for epoch mode.
    NotInEpochMode = 50,
}

/// Soroban caps every `#[contracterror]`/`#[contracttype]` enum at 50 variants
/// (`ScSpecUdtUnionV0::cases` is a `VecM<_, 50>` in stellar-xdr) ΓÇö `VaultError`
/// above is already at exactly that cap, so new error cases for issues #205,
/// #206, and #209 can't be added to it. This second, separate error enum
/// holds just those new cases, plus mirrors of the handful of `VaultError`
/// cases the new functions can also hit (via the `From` impl below, so `?`
/// still works normally at call sites).
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VaultExtError {
    /// Mirrors `VaultError::Unauthorized`.
    Unauthorized = 1,
    /// Mirrors `VaultError::NotInitialized`.
    NotInitialized = 2,
    /// Mirrors `VaultError::ZeroAmount`.
    ZeroAmount = 3,
    /// Mirrors `VaultError::ArithmeticError`.
    ArithmeticError = 4,
    /// Mirrors `VaultError::AlreadyInitialized` ΓÇö returned by `import_state()`.
    AlreadyInitialized = 5,
    /// Mirrors `VaultError::PositionNotFound`.
    PositionNotFound = 6,
    /// Returned by `set_insurance_rate_bps()` when `bps` exceeds 500 (5%)
    /// (issue #199).
    InvalidInsuranceRate = 7,
    /// Returned by `export_state()` when more than 100 positions would be
    /// exported (issue #203).
    TooManyPositions = 8,
    /// Returned by `set_fee_recipients()` when `recipients.len() > 5`
    /// (issue #197), and by `add_output_token()` when the output-token
    /// whitelist is already full (issue #244).
    TooManyRecipients = 9,
    /// Returned by `set_fee_recipients()` when the recipients' `share_bps`
    /// values don't sum to exactly 10 000 (issue #197).
    InvalidFeeAllocation = 10,
    /// Returned by `queue_action()` when 5 actions are already pending
    /// (issue #195).
    TooManyPendingActions = 11,
    /// Returned by `execute_action()`/`cancel_action()` when the given
    /// action id doesn't exist or was already executed/cancelled (issue
    /// #195).
    ActionNotFound = 12,
    /// Returned by `execute_action()` when called before `executable_at`
    /// (issue #195).
    ActionNotYetExecutable = 13,
    /// Returned by `initialize_multisig()` when `admins.len()` is 0 or > 5,
    /// or `threshold` is 0 or greater than `admins.len()` (issue #196).
    InvalidMultisigConfig = 14,
    /// Returned by `propose_action()` when 10 open proposals already exist
    /// (issue #196).
    TooManyProposals = 15,
    /// Returned by `propose_action()`/`approve_action()` when the caller is
    /// not one of the configured multisig admins (issue #196).
    NotAMultisigAdmin = 16,
    /// Returned by `approve_action()` when the caller already approved this
    /// proposal (issue #196).
    AlreadyApproved = 17,
    /// Returned by `execute_proposal()` when approvals are below the
    /// configured threshold, or the proposal id doesn't exist / was already
    /// executed (issue #196).
    ProposalNotReady = 18,
    /// Returned by `rollback_last_rate_change()` when there is no previous
    /// rate to restore, or it was already rolled back (issue #206).
    RollbackUnavailable = 19,
    /// Returned by `swap_and_stake()` when the DEX swap's output amount is
    /// below the caller-supplied `min_stake_amount` (issue #205).
    SlippageExceeded = 20,
    /// Returned by `swap_and_stake()` when `input_token` is not registered
    /// with a DEX router capable of swapping it to the stake token, or when
    /// no DEX router has been configured at all (issue #205). Also returned
    /// by `claim_in_token()` for an `output_token` that is not on the
    /// admin-managed whitelist (issue #244).
    UnsupportedInputToken = 21,
    /// Returned by `position_split()` when `split_amount` is not strictly
    /// between 0 and the caller's current position amount (issue #209).
    InvalidSplitAmount = 22,
    /// Returned by `claim_merkle_reward()` when the Merkle proof is invalid.
    MerkleInvalidProof = 23,
    /// Returned by `claim_merkle_reward()` when the user has already claimed
    /// for the given epoch.
    MerkleAlreadyClaimed = 24,
    /// Returned by `create_tournament()` when a tournament is already active.
    TournamentAlreadyExists = 25,
    /// Returned by `finalize_tournament()` when the tournament has not ended yet.
    TournamentNotEnded = 26,
    /// Returned by `finalize_tournament()` when no tournament exists.
    TournamentNotFound = 27,
    /// Returned by `compare_pools()` when more than 5 external pools are supplied.
    /// Returned by `compare_pools()` when more than 5 external pools are supplied.
    TooManyPools = 28,
    /// Returned by `execute_buyback()` when buyback is not enabled.
    BuybackNotEnabled = 29,
    /// Returned by `execute_buyback()` when accrued fees are below the threshold.
    BuybackThresholdNotMet = 30,
    /// Returned by stake/claim rate-limit checks (issue #201).
    RateLimitExceeded = 31,
    /// Returned by `start_bootstrap()` when `initial_rate < base_rate`.
    InvalidBootstrapConfig = 32,
    /// Returned by delegation chain operations when a cycle is detected (issue #200).
    CircularDelegation = 33,
    /// Returned when a delegation chain would exceed the maximum length (issue #200).
    ChainTooLong = 34,
    /// Returned by `issue_certificate` when the user's stake is below the minimum (issue #222).
    IneligibleForCertificate = 35,
    /// Returned by `set_reward_smoothing()` when the smoothing period exceeds
    /// `MAX_SMOOTHING_PERIOD_LEDGERS`, or when `min_amount` is negative
    /// (issue #235).
    InvalidSmoothingConfig = 36,
    /// Returned by `referral_tree_data()` when `max_level` exceeds
    /// `MAX_REFERRAL_TREE_DEPTH` (issue #236).
    ReferralDepthTooDeep = 37,
    /// Returned by `start_capacity_auction()` when an auction is already open,
    /// and by `place_bid()` / `finalize_capacity_auction()` when no auction
    /// exists (issue #237).
    AuctionNotFound = 38,
    /// Returned by `start_capacity_auction()` when `spots` is 0 or above
    /// `MAX_AUCTION_SPOTS`, or `duration_ledgers` is 0 (issue #237).
    InvalidAuctionConfig = 39,
    /// Returned by `start_capacity_auction()` when the previous auction has not
    /// been finalized yet (issue #237).
    AuctionAlreadyActive = 40,
    /// Returned by `place_bid()` after the auction window has closed, and by
    /// `place_bid()` / `finalize_capacity_auction()` on an already-finalized
    /// auction (issue #237).
    AuctionClosed = 41,
    /// Returned by `finalize_capacity_auction()` when called before the auction
    /// window has elapsed (issue #237).
    AuctionNotEnded = 42,
    /// Returned by `place_bid()` when the resulting bid is below the auction's
    /// `min_bid` (issue #237).
    BidBelowMinimum = 43,
    /// Returned by `place_bid()` when the auction already holds
    /// `MAX_AUCTION_BIDS` distinct bidders (issue #237).
    TooManyBids = 44,
    /// Returned by `create_lottery()` when an undrawn lottery already exists,
    /// and by `draw_lottery()` when no lottery is configured or it was
    /// already drawn (issue #239).
    LotteryAlreadyActive = 45,
    /// Returned by `draw_lottery()` when called before `draw_at_ledger`
    /// (issue #239).
    LotteryNotReady = 46,
    /// Returned by `add_milestone()` when 10 milestones are already
    /// configured (issue #238).
    TooManyMilestones = 47,
    /// Returned by `check_and_release()` when no oracle contract has been
    /// registered via `set_oracle_contract()` (issue #240).
    NoOracleConfigured = 48,
    /// Returned by `veto_proposal()` when the caller's pool share is below
    /// the configured veto threshold, or the veto threshold is unset (0),
    /// which disables the feature entirely (issue #241).
    BelowVetoThreshold = 49,
    /// Returned by `veto_proposal()` when the proposal already has a vetoer,
    /// or has already been enacted and so can no longer be vetoed (issue
    /// #241).
    AlreadyVetoed = 50,
    /// Returned by `set_veto_threshold_bps()` when `bps` exceeds 10 000
    /// (100%) (issue #241). Both error enums are at Soroban's 50-variant cap,
    /// so this doubles as the generic "basis-points value out of range" code ΓÇö
    /// `start_matching_program()` also returns it for a `match_rate_bps` above
    /// 10 000 (issue #242).
    InvalidVetoThreshold = 51,
    /// Returned by `position_clawback` (issue #463) when the clawback window
    /// has expired and the position can no longer be reversed.
    ClawbackWindowExpired = 52,
}

/// Third error enum, for the same 50-variant reason `VaultExtError` exists:
/// both `VaultError` and `VaultExtError` are now at exactly the cap, so the
/// cases introduced by issues #258 (branding), #259 (staking insurance),
/// #260 (flash stake), #261 (stake-backed loans), #275 (reward Gini
/// coefficient), #276 (seasonal reward multiplier), #274 (staker bio), and
/// #298 (pool sunsetting workflow) live here, plus mirrors of the handful of
/// `VaultError` cases those functions can also hit (via the `From` impl
/// below, so `?` still works normally at call sites).
///
/// Note on `InvalidBrandingField`: a `#[contracterror]` variant cannot carry a
/// payload, so the offending field name is encoded in the variant itself ΓÇö
/// `InvalidBrandingDisplayName`, `InvalidBrandingLogoHash`,
/// `InvalidBrandingWebsiteUrl`, `InvalidBrandingTwitterHandle` ΓÇö rather than
/// as an inner `String` on a single generic variant.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VaultFeatureError {
    Unauthorized = 1,
    NotInitialized = 2,
    ZeroAmount = 3,
    ArithmeticError = 4,
    PositionNotFound = 5,
    VaultPaused = 6,
    InsufficientRewardPool = 7,
    InvalidBrandingField = 8,
    InvalidInsuranceConfig = 9,
    InsuranceProductNotSet = 10,
    InsuranceAlreadyActive = 11,
    InsurancePolicyNotFound = 12,
    InsuranceFundInsufficient = 13,
    TooManyAffectedUsers = 14,
    TooManyStakers = 15,
    InvalidSeasonConfig = 16,
    TooManySeasons = 17,
    SeasonOverlap = 18,
    SeasonNotFound = 19,
    BioTooLong = 20,
    InvalidSunsetTransition = 21,
    PositionsStillActive = 22,
    InvalidFlashStakeFee = 23,
    InvalidLoanConfig = 24,
    LoanConfigNotSet = 25,
    LoanNotFound = 26,
    ExceedsMaxLtv = 27,
    LoanNotLiquidatable = 28,
    InvalidRevenueShareConfig = 29,
    RevenueShareInvalidProof = 30,
    RevenueShareAlreadyClaimed = 31,
    TooManyAccessTiers = 32,
    IneligibleForAccessTier = 33,
    UserStillEligible = 34,
    InvalidLotSize = 35,
    FeeBuybackNotEnabled = 36,
    NoDexRouterConfigured = 37,
    NotAContractDelegate = 38,
    ContractDelegateCapExceeded = 39,
    ContractDelegatePerCallExceeded = 40,
    PositionCollateralized = 41,
    FaceValueExceedsPosition = 42,
    DebtNftNotFound = 43,
    NotNftHolder = 44,
    TooManyCompetitors = 45,
    TooManyOpenOffers = 46,
    OfferNotFound = 47,
    OfferExpired = 48,
    NotRequestedCounterparty = 49,
    InsufficientAmount = 50,
}

// Fourth error enum for overflow errors beyond the Soroban 50-variant cap.
// VaultFeatureError exceeded the cap; new errors go here.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VaultOverflowError {
    Unauthorized = 1,
    NotInitialized = 2,
    ZeroAmount = 3,
    ArithmeticError = 4,
    PositionNotFound = 5,
    VaultPaused = 6,
    InsufficientRewardPool = 7,
    InvalidBrandingDisplayName = 8,
    InvalidBrandingLogoHash = 9,
    InvalidBrandingWebsiteUrl = 10,
    InvalidBrandingTwitterHandle = 11,
    InvalidInsuranceProduct = 12,
    InsuranceProductNotSet = 13,
    InsuranceAlreadyActive = 14,
    InsurancePolicyNotFound = 15,
    InsuranceFundInsufficient = 16,
    TooManyAffectedUsers = 17,
    TooManyStakers = 18,
    InvalidSeasonConfig = 19,
    TooManySeasons = 20,
    SeasonOverlap = 21,
    SeasonNotFound = 22,
    BioTooLong = 23,
    InvalidSunsetTransition = 24,
    PositionsStillActive = 25,
    InvalidFlashStakeFee = 26,
    InvalidLoanConfig = 27,
    LoanConfigNotSet = 28,
    LoanNotFound = 29,
    ExceedsMaxLtv = 30,
    LoanNotLiquidatable = 31,
    InvalidRevenueShareConfig = 32,
    RevenueShareInvalidProof = 33,
    RevenueShareAlreadyClaimed = 34,
    TooManyAccessTiers = 35,
    IneligibleForAccessTier = 36,
    UserStillEligible = 37,
    InvalidLotSize = 38,
    FeeBuybackNotEnabled = 39,
    NoDexRouterConfigured = 40,
    NotAContractDelegate = 41,
    ContractDelegateCapExceeded = 42,
    ContractDelegatePerCallExceeded = 43,
    PositionCollateralized = 44,
    FaceValueExceedsPosition = 45,
    DebtNftNotFound = 46,
    NotNftHolder = 47,
    TooManyCompetitors = 48,
    TooManyOpenOffers = 49,
    /// Returned by staking entrypoints when the stake amount is insufficient.
    InsufficientStake = 50,
}

impl From<VaultError> for VaultOverflowError {
    fn from(e: VaultError) -> Self {
        match e {
            VaultError::Unauthorized => VaultOverflowError::Unauthorized,
            VaultError::NotInitialized => VaultOverflowError::NotInitialized,
            VaultError::ZeroAmount => VaultOverflowError::ZeroAmount,
            VaultError::ArithmeticError => VaultOverflowError::ArithmeticError,
            VaultError::PositionNotFound => VaultOverflowError::PositionNotFound,
            VaultError::VaultPaused => VaultOverflowError::VaultPaused,
            VaultError::InsufficientRewardPool => VaultOverflowError::InsufficientRewardPool,
            _ => VaultOverflowError::Unauthorized,
        }
    }
}



impl From<VaultError> for VaultExtError {
    fn from(err: VaultError) -> Self {
        match err {
            VaultError::Unauthorized => VaultExtError::Unauthorized,
            VaultError::NotInitialized => VaultExtError::NotInitialized,
            VaultError::ZeroAmount => VaultExtError::ZeroAmount,
            VaultError::ArithmeticError => VaultExtError::ArithmeticError,
            VaultError::PositionNotFound => VaultExtError::PositionNotFound,
            // Any other VaultError reaching here (shouldn't happen given how
            // these functions are written) maps to the closest generic case.
            _ => VaultExtError::Unauthorized,
        }
    }
}

impl From<VaultError> for VaultFeatureError {
    fn from(err: VaultError) -> Self {
        match err {
            VaultError::Unauthorized => VaultFeatureError::Unauthorized,
            VaultError::NotInitialized => VaultFeatureError::NotInitialized,
            VaultError::ZeroAmount => VaultFeatureError::ZeroAmount,
            VaultError::ArithmeticError => VaultFeatureError::ArithmeticError,
            VaultError::PositionNotFound => VaultFeatureError::PositionNotFound,
            VaultError::VaultPaused => VaultFeatureError::VaultPaused,
            VaultError::InsufficientRewardPool => VaultFeatureError::InsufficientRewardPool,
            // Any other VaultError reaching here (shouldn't happen given how
            // these functions are written) maps to the closest generic case.
            _ => VaultFeatureError::Unauthorized,
        }
    }
}

/// Fifth error enum for issue #391 (stake_to_learn). Both `VaultError`,
/// `VaultExtError`, `VaultFeatureError`, and `VaultOverflowError` are at
/// Soroban's 50-variant cap, so quiz-specific errors live here.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VaultQuizError {
    /// Mirrors `VaultError::Unauthorized`.
    Unauthorized = 1,
    /// Mirrors `VaultError::NotInitialized`.
    NotInitialized = 2,
    /// Mirrors `VaultError::ZeroAmount`.
    ZeroAmount = 3,
    /// Mirrors `VaultError::ArithmeticError`.
    ArithmeticError = 4,
    /// Mirrors `VaultError::VaultPaused`.
    VaultPaused = 5,
    /// Returned by `submit_quiz_answer()` when the given `quiz_id` does not
    /// correspond to any stored quiz.
    QuizNotFound = 6,
    /// Returned by `submit_quiz_answer()` when the user has already
    /// successfully completed this quiz.
    QuizAlreadyCompleted = 7,
    /// Returned by `submit_quiz_answer()` when the user has exhausted all
    /// allowed attempts for this quiz.
    QuizMaxAttemptsReached = 8,
    /// Returned by `add_quiz()` when the contract already holds the maximum
    /// of 20 quizzes.
    TooManyQuizzes = 9,
}

impl From<VaultError> for VaultQuizError {
    fn from(err: VaultError) -> Self {
        match err {
            VaultError::Unauthorized => VaultQuizError::Unauthorized,
            VaultError::NotInitialized => VaultQuizError::NotInitialized,
            VaultError::ZeroAmount => VaultQuizError::ZeroAmount,
            VaultError::ArithmeticError => VaultQuizError::ArithmeticError,
            VaultError::VaultPaused => VaultQuizError::VaultPaused,
            _ => VaultQuizError::Unauthorized,
        }
    }
    /// Returned by `sign_covenant` (issue #413) when the supplied
    /// `terms_hash` does not match the currently published pool terms.
    TermsMismatch = 71,
    /// Returned by `stake_with_covenant` (issue #413) when the caller has
    /// not signed the currently published pool terms.
    CovenantRequired = 72,
    /// Returned by `pin_ipfs_hash` (issue #439) when the caller's staked
    /// position amount is below the configured `min_stake` for the
    /// stake-gated IPFS storage service.
    InsufficientStakeForStorage = 73,
    /// Returned by `unstake` (issue #441) when the requested amount is below
    /// the configured minimum unstake amount and is not a full position exit.
    BelowMinimumUnstake = 74,
    /// Returned by `position_clawback` (issue #463) when the clawback window
    /// has expired and the position can no longer be reversed.
    ClawbackWindowExpired = 75,
}

/// Sixth error enum, added for the same 50-variant reason the earlier
/// `Vault*Error` enums exist: every prior `#[contracterror]` enum is already at
/// Soroban's cap. Holds the cases introduced by issues #459 (position health
/// auto-recovery), #460 (lockdrop campaign), #461 (proof-of-humanity hook), and
/// #462 (roadmap voting), plus mirrors of the handful of `VaultError` cases
/// those functions can also hit (via the `From` impl below, so `?` keeps
/// working at call sites that mix the two).
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VaultCampaignError {
    /// Mirrors `VaultError::Unauthorized`.
    Unauthorized = 1,
    /// Mirrors `VaultError::NotInitialized`.
    NotInitialized = 2,
    /// Mirrors `VaultError::ZeroAmount`.
    ZeroAmount = 3,
    /// Mirrors `VaultError::ArithmeticError`.
    ArithmeticError = 4,
    /// Mirrors `VaultError::PositionNotFound`.
    PositionNotFound = 5,
    /// Mirrors `VaultError::VaultPaused`.
    VaultPaused = 6,

    // ── Issue #459: position health auto-recovery ────────────────────────────
    /// Returned by `check_and_recover()` / `cancel_recovery_config()` when the
    /// user has no active recovery configuration.
    RecoveryNotConfigured = 7,
    /// Returned by `check_and_recover()` when the user's current position
    /// health is at or above their configured trigger threshold (or the
    /// position carries no loan, so health is unbounded).
    RecoveryNotTriggered = 8,
    /// Returned by `check_and_recover()` when the per-user daily cooldown has
    /// not elapsed since the last recovery.
    RecoveryOnCooldown = 9,
    /// Returned by `set_recovery_config()` when `trigger_health_bps` is 0, or
    /// `action_amount` is not positive for an action that needs one.
    InvalidRecoveryConfig = 10,
    /// Returned by `check_and_recover()` when an `AutoRepayLoan` action is
    /// configured but the user has no outstanding loan.
    NoActiveLoan = 11,

    // ── Issue #460: lockdrop campaign ───────────────────────────────────────
    /// Returned by `start_lockdrop()` when a campaign that has not been
    /// finalized already exists.
    LockdropAlreadyActive = 12,
    /// Returned by lockdrop entrypoints when no campaign has been started.
    LockdropNotActive = 13,
    /// Returned by `commit_to_lockdrop()` after the commitment window
    /// (`ends_at`) has passed.
    LockdropEnded = 14,
    /// Returned by `finalize_lockdrop()` before `ends_at`.
    LockdropNotEnded = 15,
    /// Returned by `claim_lockdrop_reward()` before the campaign is finalized.
    LockdropNotFinalized = 16,
    /// Returned by `finalize_lockdrop()` when the campaign is already finalized.
    LockdropAlreadyFinalized = 17,
    /// Returned by `commit_to_lockdrop()` when the caller already has a
    /// commitment in the active campaign.
    AlreadyCommitted = 18,
    /// Returned by `exit_lockdrop()` / `claim_lockdrop_reward()` when the
    /// caller has no commitment.
    CommitmentNotFound = 19,
    /// Returned by `exit_lockdrop()` before the caller's chosen lock duration
    /// has elapsed.
    LockStillActive = 20,
    /// Returned by `claim_lockdrop_reward()` when the caller already claimed.
    AlreadyClaimed = 21,
    /// Returned by `claim_lockdrop_reward()` when the caller's proportional
    /// allocation is zero.
    NothingToClaim = 22,
    /// Returned by `commit_to_lockdrop()` when the campaign already holds the
    /// maximum supported number of committers.
    LockdropFull = 23,
    /// Returned by `commit_to_lockdrop()` when `lock_duration_ledgers` is 0 or
    /// exceeds the campaign's `max_lock_ledgers`.
    InvalidLockDuration = 24,

    // ── Issue #461: proof-of-humanity hook ─────────────────────────────────
    /// Returned by `stake_verified()` when no humanity oracle / config has
    /// been registered.
    OracleNotConfigured = 25,
    /// Returned by `stake_verified()` when the staked amount is below the
    /// minimum that applies to the caller's verification status.
    BelowHumanityMinStake = 26,
    /// Returned by `set_humanity_config()` when a bps field is out of range.
    InvalidHumanityConfig = 27,

    // ── Issue #462: roadmap voting ────────────────────────────────────────
    /// Returned by `add_roadmap_item()` when 20 items already exist.
    TooManyRoadmapItems = 28,
    /// Returned by `remove_roadmap_item()` / `vote_roadmap_item()` for an
    /// unknown item id.
    RoadmapItemNotFound = 29,
    /// Returned by `add_roadmap_item()` when the title exceeds 80 characters.
    TitleTooLong = 30,
    /// Returned by `vote_roadmap_item()` when the caller's allocations for the
    /// current epoch would exceed the 100-point budget.
    VoteBudgetExceeded = 31,
    /// Returned by `vote_roadmap_item()` when `weight` alone exceeds 100.
    InvalidVoteWeight = 32,
}

impl From<VaultError> for VaultCampaignError {
    fn from(err: VaultError) -> Self {
        match err {
            VaultError::Unauthorized => VaultCampaignError::Unauthorized,
            VaultError::NotInitialized => VaultCampaignError::NotInitialized,
            VaultError::ZeroAmount => VaultCampaignError::ZeroAmount,
            VaultError::ArithmeticError => VaultCampaignError::ArithmeticError,
            VaultError::PositionNotFound => VaultCampaignError::PositionNotFound,
            VaultError::VaultPaused => VaultCampaignError::VaultPaused,
            // Any other VaultError reaching here maps to the closest generic case.
            _ => VaultCampaignError::Unauthorized,
        }
    }
}
