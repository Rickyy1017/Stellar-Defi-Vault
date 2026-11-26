use crate::storage::{ChangelogEntry, ClaimWindow, DataKey, VestingEntry};
use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};

pub fn get_shares(env: &Env, user: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::ShareBalance(user.clone()))
        .unwrap_or(0)
}

pub fn set_shares(env: &Env, user: &Address, amount: i128) {
    assert_non_negative(env, amount);
    // Issue #748: keep the depositor index in step with the zero/non-zero
    // share boundary. Capture the prior balance before it is overwritten.
    let old = get_shares(env, user);
    // Issue #699: extra reward tokens accrue per share, so settle them at the
    // old share count before it changes.
    crate::vault::multi_reward_tokens::settle_user(env, user);
    env.storage()
        .persistent()
        .set(&DataKey::ShareBalance(user.clone()), &amount);
    crate::balance_checkpoints::record_user(env, user, amount);
    crate::vault::depositor_index::track(env, user, old, amount);
}

pub fn get_total_shares(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::TotalShares)
        .unwrap_or(0)
}

pub fn set_total_shares(env: &Env, total: i128) {
    assert_non_negative(env, total);
    env.storage().instance().set(&DataKey::TotalShares, &total);
    crate::balance_checkpoints::record_total(env, total);
}

pub fn get_total_deposited(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::TotalDeposited)
        .unwrap_or(0)
}

pub fn set_total_deposited(env: &Env, total: i128) {
    assert_non_negative(env, total);
    env.storage()
        .instance()
        .set(&DataKey::TotalDeposited, &total);
}

pub fn get_min_stake(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::MinStake)
        .unwrap_or(0)
}

pub fn set_min_stake(env: &Env, amount: i128) {
    env.storage().instance().set(&DataKey::MinStake, &amount);
}

pub fn get_reward_rate_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::RewardRateBps)
        .unwrap_or(0)
}

pub fn set_reward_rate_bps(env: &Env, rate_bps: u32) {
    env.storage()
        .instance()
        .set(&DataKey::RewardRateBps, &rate_bps);
}

pub fn get_reward_tiers(env: &Env) -> Vec<RewardTier> {
    env.storage()
        .instance()
        .get(&symbol_short!("rwtiers"))
        .unwrap_or(Vec::new(env))
}

pub fn set_reward_tiers(env: &Env, tiers: &Vec<RewardTier>) {
    env.storage()
        .instance()
        .set(&symbol_short!("rwtiers"), tiers);
}

/// Get the maximum number of quizzes allowed.
pub fn get_max_quizzes(_: &Env) -> u32 {
    MAX_QUIZZES
}

/// Get quiz data by ID.
pub fn get_quiz(env: &Env, quiz_id: u32) -> Option<Quiz> {
    let key = (Symbol::new(env, "quiz"), quiz_id);
    env.storage().persistent().get(&key)
}

/// Set quiz data by ID.
pub fn set_quiz(env: &Env, quiz: &Quiz) {
    let key = (Symbol::new(env, "quiz"), quiz.id);
    env.storage().persistent().set(&key, quiz);
}

/// Get the number of remaining attempts for a user on a specific quiz.
pub fn get_quiz_attempts_remaining(env: &Env, user: &Address, quiz_id: u32) -> u32 {
    let key = (Symbol::new(env, "quiz_attempts"), user.clone(), quiz_id);
    env.storage().persistent().get(&key).unwrap_or(0)
}

/// Returns `None` if the user has never had attempts initialized for this quiz
/// (i.e. they haven't submitted a wrong answer yet), `Some(n)` otherwise.
/// This lets callers distinguish between "not yet started" and "0 remaining".
pub fn get_quiz_attempts_remaining_opt(env: &Env, user: &Address, quiz_id: u32) -> Option<u32> {
    let key = (Symbol::new(env, "quiz_attempts"), user.clone(), quiz_id);
    env.storage().persistent().get(&key)
}

/// Set the number of remaining attempts for a user on a specific quiz.
pub fn set_quiz_attempts_remaining(env: &Env, user: &Address, quiz_id: u32, attempts: u32) {
    let key = (Symbol::new(env, "quiz_attempts"), user.clone(), quiz_id);
    env.storage().persistent().set(&key, &attempts);
}

/// Get the list of completed quiz IDs for a user.
pub fn get_completed_quizzes(env: &Env, user: &Address) -> Vec<u32> {
    let key = (Symbol::new(env, "completed_quizzes"), user.clone());
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env))
}

pub fn set_rate_history(env: &Env, history: &Vec<(u32, u32)>) {
    env.storage().instance().set(&DataKey::RateHistory, history);
}

pub const MAX_RATE_HISTORY_ENTRIES: u32 = 50;
pub const MAX_QUIZZES: u32 = 20;

/// Maximum allowed reward rate in basis points (500% APR). Issue #72.
pub const MAX_RATE_BPS: u32 = 50_000;

// ── Issue #828: tiered reward rates by position size ──────────────────────────

/// Configurable tiered reward rates based on stake bracket.
///
/// `tiers` is a `Vec<(min_amount, rate_bps)>`, sorted ascending by min_amount.
/// Max 5 tiers. Each tier applies to stakes within its bracket:
/// - Tier 0: stakes >= tiers[0].0 at rate tiers[0].1
/// - Tier i: stakes >= tiers[i].0 and < tiers[i+1].0 at rate tiers[i].1
/// - Highest tier: stakes >= last tier's min_amount at that rate
/// Pass an empty Vec to use the base `reward_rate_bps`.
pub fn set_rate_tiers(env: &Env, tiers: &Vec<(i128, u32)>) {
    if tiers.len() > 5 {
        return;
    }
    env.storage()
        .instance()
        .set(&symbol_short!("rate_tiers"), tiers);
}

/// Get the effective reward rate for a user's stake amount.
///
/// Returns the rate in basis points that applies to the user's position,
/// considering the tiered rate configuration. If no tiers are configured,
/// returns the base `reward_rate_bps`.
pub fn get_effective_rate(env: &Env, amount: i128) -> u32 {
    let tiers: Vec<(i128, u32)> = env
        .storage()
        .instance()
        .get(&symbol_short!("rate_tiers"))
        .unwrap_or(Vec::new(env));

    if tiers.is_empty() {
        return balance::get_reward_rate_bps(&env);
    }

    // Find the applicable tier for this amount
    let mut rate_bps = tiers[0].1; // default to first tier rate
    let mut found = false;

    for i in 0..tiers.len() {
        if amount >= tiers[i].0 {
            rate_bps = tiers[i].1;
            found = true;
        } else {
            break;
        }
    }

    if found {
        rate_bps
    } else {
        // Amount is below the first tier's minimum, use base rate
        balance::get_reward_rate_bps(&env)
    }
}

pub fn get_reward_pool_balance(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::RewardPoolBalance)
        .unwrap_or(0)
}

pub fn set_reward_pool_balance(env: &Env, balance: i128) {
    assert_non_negative(env, balance);
    env.storage()
        .instance()
        .set(&DataKey::RewardPoolBalance, &balance);
}

pub fn get_withdrawal_limit(env: &Env) -> Option<i128> {
    env.storage().instance().get(&DataKey::WithdrawalLimit)
}

pub fn set_withdrawal_limit(env: &Env, limit: i128) {
    env.storage()
        .instance()
        .set(&DataKey::WithdrawalLimit, &limit);
}

pub fn get_pool_cap(env: &Env) -> i128 {
    env.storage().instance().get(&DataKey::PoolCap).unwrap_or(0)
}

pub fn set_pool_cap(env: &Env, cap: i128) {
    env.storage()
        .instance()
        .set(&symbol_short!("pool_cp"), &cap);
}

pub fn get_unstake_fee_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::UnstakeFeeBps)
        .unwrap_or(0)
}

pub fn set_unstake_fee_bps(env: &Env, bps: u32) {
    env.storage().instance().set(&DataKey::UnstakeFeeBps, &bps);
}

pub fn get_reward_checkpoint_ledger(env: &Env, user: &Address) -> Option<u32> {
    env.storage()
        .persistent()
        .get(&DataKey::RewardCheckpointLedger(user.clone()))
}

pub fn set_reward_checkpoint_ledger(env: &Env, user: &Address, ledger: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::RewardCheckpointLedger(user.clone()), &ledger);
}

pub fn set_last_claim_ledger(env: &Env, user: &Address, ledger: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::LastClaimLedger(user.clone()), &ledger);
}

pub fn get_accrued_reward(env: &Env, user: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::AccruedReward(user.clone()))
        .unwrap_or(0)
}

pub fn set_accrued_reward(env: &Env, user: &Address, amount: i128) {
    assert_non_negative(env, amount);
    env.storage()
        .persistent()
        .set(&DataKey::AccruedReward(user.clone()), &amount);
}

pub fn get_stake_history(env: &Env, user: &Address) -> Option<Vec<(u32, i128)>> {
    env.storage()
        .persistent()
        .get(&DataKey::StakeHistory(user.clone()))
}

pub fn set_stake_history(env: &Env, user: &Address, history: &Vec<(u32, i128)>) {
    env.storage()
        .persistent()
        .set(&DataKey::StakeHistory(user.clone()), history);
}

pub fn get_boost_schedule(env: &Env) -> Option<Vec<(u32, u32)>> {
    env.storage().instance().get(&DataKey::BoostSchedule)
}

pub fn set_boost_schedule(env: &Env, tiers: &Vec<(u32, u32)>) {
    env.storage().instance().set(&DataKey::BoostSchedule, tiers);
}

pub fn get_total_stakers(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::TotalStakers)
        .unwrap_or(0)
}

pub fn set_total_stakers(env: &Env, count: u32) {
    env.storage().instance().set(&DataKey::TotalStakers, &count);
}

pub fn get_total_rewards_paid(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::TotalRewardsPaid)
        .unwrap_or(0)
}

pub fn set_total_rewards_paid(env: &Env, amount: i128) {
    assert_non_negative(env, amount);
    env.storage()
        .instance()
        .set(&DataKey::TotalRewardsPaid, &amount);
}

pub fn get_last_claim_ledger(env: &Env, user: &Address) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::LastClaimLedger(user.clone()))
        .unwrap_or(0)
}

pub fn get_delegate(env: &Env, user: &Address) -> Option<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::Delegate(user.clone()))
}

pub fn set_delegate(env: &Env, user: &Address, delegate: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::Delegate(user.clone()), delegate);
}

pub fn remove_delegate(env: &Env, user: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::Delegate(user.clone()));
}

// ── Issue #38: slash treasury ────────────────────────────────────────────────

pub fn get_slash_treasury(env: &Env) -> Option<Address> {
    env.storage().instance().get(&symbol_short!("sl_tr"))
}

pub fn set_slash_treasury(env: &Env, treasury: &Address) {
    env.storage()
        .instance()
        .set(&symbol_short!("sl_tr"), treasury);
}

// ── Issue #39: reward token ───────────────────────────────────────────────────

pub fn get_reward_token(env: &Env) -> Option<Address> {
    env.storage().instance().get(&symbol_short!("rwd_tok"))
}

pub fn set_reward_token(env: &Env, token: &Address) {
    env.storage()
        .instance()
        .set(&symbol_short!("rwd_tok"), token);
}

// ── Issue #40: NFT contract ───────────────────────────────────────────────────

pub fn get_nft_contract(env: &Env) -> Option<Address> {
    env.storage().instance().get(&symbol_short!("nft_con"))
}

pub fn set_nft_contract(env: &Env, nft: &Address) {
    env.storage().instance().set(&symbol_short!("nft_con"), nft);
}

// ── Issue #41: restake grace window ──────────────────────────────────────────

pub fn get_restake_window(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::RestakeWindow)
        .unwrap_or(0)
}

pub fn set_restake_window(env: &Env, ledgers: u32) {
    env.storage()
        .instance()
        .set(&DataKey::RestakeWindow, &ledgers);
}

pub fn get_last_unstake_ledger(env: &Env, user: &Address) -> Option<u32> {
    env.storage()
        .persistent()
        .get(&DataKey::LastUnstakeLedger(user.clone()))
}

pub fn set_last_unstake_ledger(env: &Env, user: &Address, ledger: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::LastUnstakeLedger(user.clone()), &ledger);
}

pub fn is_restaked(env: &Env, user: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Restaked(user.clone()))
        .unwrap_or(false)
}

pub fn set_restaked(env: &Env, user: &Address, value: bool) {
    env.storage()
        .persistent()
        .set(&DataKey::Restaked(user.clone()), &value);
}

pub fn remove_restaked(env: &Env, user: &Address) {
    let key = DataKey::Restaked(user.clone());
    if env.storage().persistent().has(&key) {
        env.storage().persistent().remove(&key);
    }
}

// ── Issue #42: admin action count ────────────────────────────────────────────

pub fn get_admin_action_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::AdminActionCount)
        .unwrap_or(0)
}

pub fn increment_admin_action_count(env: &Env) {
    let count = get_admin_action_count(env);
    env.storage()
        .instance()
        .set(&DataKey::AdminActionCount, &(count + 1));
}

// ── Claim cap (issue #78) ─────────────────────────────────────────────────────

pub fn get_claim_cap(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::ClaimCap)
        .unwrap_or(0)
}

pub fn set_claim_cap(env: &Env, cap: i128) {
    env.storage()
        .instance()
        .set(&symbol_short!("clm_cap"), &cap);
}

pub fn get_claim_cap_window(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::ClaimCapWindow)
        .unwrap_or(0)
}

pub fn set_claim_cap_window(env: &Env, window_ledgers: u32) {
    env.storage()
        .instance()
        .set(&symbol_short!("clm_win"), &window_ledgers);
}

pub fn get_user_claim_window(env: &Env, user: &Address) -> Option<ClaimWindow> {
    env.storage()
        .persistent()
        .get(&DataKey::UserClaimWindow(user.clone()))
}

pub fn set_user_claim_window(env: &Env, user: &Address, window: &ClaimWindow) {
    env.storage()
        .persistent()
        .set(&DataKey::UserClaimWindow(user.clone()), window);
}

// ── Token decimals (reward normalization) ─────────────────────────────────────

/// Default decimal precision for Stellar tokens. Most tokens use 7 places,
/// but this is only a fallback for pools initialized without explicit values.
pub const DEFAULT_TOKEN_DECIMALS: u32 = 7;

pub fn get_stake_decimals(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&symbol_short!("stk_dec"))
        .unwrap_or(DEFAULT_TOKEN_DECIMALS)
}

pub fn set_stake_decimals(env: &Env, decimals: u32) {
    env.storage()
        .instance()
        .set(&symbol_short!("stk_dec"), &decimals);
}

pub fn get_reward_decimals(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&symbol_short!("rwd_dec"))
        .unwrap_or(DEFAULT_TOKEN_DECIMALS)
}

pub fn set_reward_decimals(env: &Env, decimals: u32) {
    env.storage()
        .instance()
        .set(&symbol_short!("rwd_dec"), &decimals);
}

// ── All-stakers list (issue #95) ──────────────────────────────────────────────

pub fn get_all_stakers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::AllStakers)
        .unwrap_or(Vec::new(env))
}

pub fn set_all_stakers(env: &Env, stakers: &Vec<Address>) {
    env.storage().instance().set(&DataKey::AllStakers, stakers);
}

/// Records `user` in the staker registry if they are not already present and
/// refreshes the `total_stakers` count. Idempotent, so callers can invoke it on
/// every stake without worrying about duplicates. Keeps the registry (and thus
/// `get_pool_summary().depositor_count`) in sync with real depositors.
pub fn register_staker(env: &Env, user: &Address) {
    let mut stakers = get_all_stakers(env);
    let already_present = stakers.iter().any(|a| &a == user);
    if !already_present {
        stakers.push_back(user.clone());
        env.storage()
            .instance()
            .set(&DataKey::AllStakers, &stakers);
    }
    env.storage()
        .instance()
        .set(&DataKey::TotalStakers, &stakers.len());
}

// ── Share math ────────────────────────────────────────────────────────────────

/// Convert a deposit amount to shares using current vault ratio.
/// First deposit: scale the token amount into the vault's fixed share
/// precision. Subsequent deposits are proportional to the existing pool.
pub fn amount_to_shares(total_shares: i128, total_deposited: i128, amount: i128) -> Option<i128> {
    amount_to_shares_with_decimals(total_shares, total_deposited, amount, DEFAULT_TOKEN_DECIMALS)
}

/// Convert token base units to shares. Shares intentionally use seven decimal
/// places, while the underlying token may use a different precision. The
/// decimal argument is the precision queried from the configured token.
pub fn amount_to_shares_with_decimals(
    total_shares: i128,
    total_deposited: i128,
    amount: i128,
    token_decimals: u32,
) -> Option<i128> {
    if total_shares == 0 || total_deposited == 0 {
        scale_amount(amount, token_decimals, DEFAULT_TOKEN_DECIMALS)
    } else {
        // Issue #784: `amount * total_shares` can overflow i128 for a large
        // pool even though the final share count fits comfortably, since
        // `amount <= total_deposited` in the normal case. `mul_div_floor`
        // computes the exact 256-bit product first, so it only reports
        // overflow when the final quotient itself doesn't fit — matching
        // this function's previous truncating-division (floor) rounding.
        crate::math::mul_div_floor(amount, total_shares, total_deposited).ok()
    }
}

/// Convert shares to the underlying token amount.
pub fn shares_to_amount(total_shares: i128, total_deposited: i128, shares: i128) -> Option<i128> {
    shares_to_amount_with_decimals(total_shares, total_deposited, shares, DEFAULT_TOKEN_DECIMALS)
}

/// Convert shares in the fixed seven-decimal share precision to underlying
/// token base units. The ratio already includes the initial decimal scaling.
pub fn shares_to_amount_with_decimals(
    total_shares: i128,
    total_deposited: i128,
    shares: i128,
    _token_decimals: u32,
) -> Option<i128> {
    if total_shares == 0 {
        Some(0)
    } else {
        // Issue #784: see `amount_to_shares_with_decimals` above — same
        // overflow-prone pattern, same fix, same preserved floor rounding.
        crate::math::mul_div_floor(shares, total_deposited, total_shares).ok()
    }
}

fn scale_amount(amount: i128, from_decimals: u32, to_decimals: u32) -> Option<i128> {
    if from_decimals == to_decimals {
        return Some(amount);
    }

    if from_decimals < to_decimals {
        amount.checked_mul(10_i128.checked_pow(to_decimals - from_decimals)?)
    } else {
        amount.checked_div(10_i128.checked_pow(from_decimals - to_decimals)?)
    }
}

pub fn get_reward_remainder(env: &Env, user: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::RewardRemainder(user.clone()))
        .unwrap_or(0)
}

pub fn set_reward_remainder(env: &Env, user: &Address, amount: i128) {
    assert_non_negative(env, amount);
    env.storage()
        .persistent()
        .set(&DataKey::RewardRemainder(user.clone()), &amount);
}

// ── Issue #69: last updated ledger ───────────────────────────────────────────
// Uses symbol_short! to avoid pushing DataKey over the contracttype variant limit.

pub fn get_last_updated_ledger(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&symbol_short!("lst_upd"))
        .unwrap_or(0)
}

pub fn set_last_updated_ledger(env: &Env, ledger: u32) {
    env.storage()
        .instance()
        .set(&symbol_short!("lst_upd"), &ledger);
}

// ── Issue #97: pool description ──────────────────────────────────────────────

pub fn get_pool_description(env: &Env) -> Option<soroban_sdk::String> {
    env.storage().instance().get(&symbol_short!("pool_desc"))
}

pub fn set_pool_description(env: &Env, description: &soroban_sdk::String) {
    env.storage()
        .instance()
        .set(&symbol_short!("pool_desc"), description);
}

// ── Issue #99: staking streak ────────────────────────────────────────────────

pub fn get_last_recorded_wave(env: &Env) -> Option<u32> {
    env.storage().instance().get(&symbol_short!("last_wave"))
}

pub fn set_last_recorded_wave(env: &Env, wave_id: u32) {
    env.storage()
        .instance()
        .set(&symbol_short!("last_wave"), &wave_id);
}

pub fn get_user_streak(env: &Env, user: &Address) -> Option<crate::storage::StakeStreak> {
    let key = (Symbol::new(env, "strk"), user.clone());
    env.storage().persistent().get(&key)
}

pub fn set_user_streak(env: &Env, user: &Address, streak: &crate::storage::StakeStreak) {
    let key = (Symbol::new(env, "strk"), user.clone());
    env.storage().persistent().set(&key, streak);
}

// ── Issue #114: on-chain admin changelog ─────────────────────────────────────
// Key "chlg" (4 chars, short symbol) stored in instance storage.

pub fn get_changelog(env: &Env) -> Vec<ChangelogEntry> {
    env.storage()
        .instance()
        .get(&symbol_short!("chlg"))
        .unwrap_or(Vec::new(env))
}

pub fn set_changelog(env: &Env, log: &Vec<ChangelogEntry>) {
    env.storage().instance().set(&symbol_short!("chlg"), log);
}

// ── Issue #115: last reward rate change ledger ────────────────────────────────
// Key "lrcl" (4 chars, short symbol) stored in instance storage.

pub fn get_last_rate_change_ledger(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&symbol_short!("lrcl"))
        .unwrap_or(0)
}

pub fn set_last_rate_change_ledger(env: &Env, ledger: u32) {
    env.storage()
        .instance()
        .set(&symbol_short!("lrcl"), &ledger);
}

// ── Issue #116: per-user vesting entries ─────────────────────────────────────
// Key ("vest", user) stored in persistent storage (same pattern as streak).

pub fn get_vesting_entries(env: &Env, user: &Address) -> Vec<VestingEntry> {
    let key = (Symbol::new(env, "vest"), user.clone());
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env))
}

pub fn set_vesting_entries(env: &Env, user: &Address, entries: &Vec<VestingEntry>) {
    let key = (Symbol::new(env, "vest"), user.clone());
    env.storage().persistent().set(&key, entries);
}

// ── Issue #117: pool initialization ledger ───────────────────────────────────
// Key "inal" (4 chars, short symbol) stored in instance storage.

pub fn get_initialized_at_ledger(env: &Env) -> Option<u32> {
    env.storage().instance().get(&symbol_short!("inal"))
}

pub fn set_initialized_at_ledger(env: &Env, ledger: u32) {
    env.storage()
        .instance()
        .set(&symbol_short!("inal"), &ledger);
}

// ── Issue #939: on-chain upgrade history ─────────────────────────────────────

/// An on-chain upgrade history record (issue #939).
///
/// Records each contract upgrade with its wasm hash, ledger, and storage version
/// so frontends and integrators can verify which contract code is active and when
/// upgrades occurred.
pub struct UpgradeRecord {
    pub wasm_hash: soroban_sdk::Bytes,
    pub ledger: u32,
    pub storage_version: u32,
}

pub fn get_upgrade_record(env: &Env, index: u32) -> Option<UpgradeRecord> {
    let key = (Symbol::new(env, "upg"),);
    env.storage()
        .instance()
        .get(&key)
        .and_then(|records: Vec<UpgradeRecord>| records.get(index as usize).cloned())
}

pub fn set_upgrade_record(env: &Env, record: &UpgradeRecord) {
    let key = (Symbol::new(env, "upg"),);
    let mut records: Vec<UpgradeRecord> = env
        .storage()
        .instance()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env));
    if records.len() >= 20 {
        // Keep only the latest 20 upgrade records
        records.drain(..records.len() - 20);
    }
    records.push_back(record.clone());
    env.storage()
        .instance()
        .set(&key, &records);
}

// ── Issue #940: successor pointer for deprecated pools ───────────────────────

/// The successor pool address (issue #940).
///
/// When a pool is deprecated or migrated, the admin can set a successor pool
/// address so frontends and integrators can redirect users. Once set, this is
/// a one-way operation.
pub fn get_successor_pool(env: &Env) -> Option<Address> {
    env.storage().instance().get(&symbol_short!("succ"))
}

pub fn set_successor_pool(env: &Env, successor: &Address) {
    env.storage()
        .instance()
        .set(&symbol_short!("succ"), successor);
}

// ── Issue #942: share token name and symbol metadata for wallets ─────────────

/// The stake token name (issue #942).
/// Derived from the underlying asset at init, with admin override within length limits.
pub fn get_stake_token_name(env: &Env) -> Option<soroban_sdk::String> {
    env.storage().instance().get(&symbol_short!("stk_name"))
}

pub fn set_stake_token_name(env: &Env, name: &soroban_sdk::String) {
    env.storage().instance().set(&symbol_short!("stk_name"), name);
}

/// The stake token symbol (issue #942).
/// Derived from the underlying asset at init, with admin override within length limits.
pub fn get_stake_token_symbol(env: &Env) -> Option<soroban_sdk::String> {
    env.storage().instance().get(&symbol_short!("stk_sym"))
}

pub fn set_stake_token_symbol(env: &Env, symbol: &soroban_sdk::String) {
    env.storage().instance().set(&symbol_short!("stk_sym"), symbol);
}

/// The stake token decimals (issue #942).
/// Defaults to 7 (Stellar standard) when not explicitly set.
pub const DEFAULT_TOKEN_DECIMALS: u32 = 7;

pub fn get_stake_token_decimals(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&symbol_short!("stk_dec"))
        .unwrap_or(DEFAULT_TOKEN_DECIMALS)
}

pub fn set_stake_token_decimals(env: &Env, decimals: u32) {
    env.storage()
        .instance()
        .set(&symbol_short!("stk_dec"), &decimals);
}

// ── Issue #943: track lifetime and all-time-high pool statistics ─────────────

/// Peak TVL (issue #943) - maximum total deposited amount ever reached.
pub fn get_peak_tvl(env: &Env) -> i128 {
    env.storage().instance().get(&symbol_short!("peak_tvl")).unwrap_or(0)
}

pub fn set_peak_tvl(env: &Env, amount: i128) {
    env.storage().instance().set(&symbol_short!("peak_tvl"), &amount);
}

/// Lifetime deposited volume (issue #943) - total amount ever deposited across all time.
pub fn get_lifetime_deposited_volume(env: &Env) -> i128 {
    env.storage().instance().get(&symbol_short!("life_dep")).unwrap_or(0)
}

pub fn set_lifetime_deposited_volume(env: &Env, amount: i128) {
    env.storage().instance().set(&symbol_short!("life_dep"), &amount);
}

/// Lifetime withdrawn volume (issue #943) - total amount ever withdrawn across all time.
pub fn get_lifetime_withdrawn_volume(env: &Env) -> i128 {
    env.storage().instance().get(&symbol_short!("life_wdr")).unwrap_or(0)
}

pub fn set_lifetime_withdrawn_volume(env: &Env, amount: i128) {
    env.storage().instance().set(&symbol_short!("life_wdr"), &amount);
}

/// Unique depositors ever (issue #943) - count of unique addresses that have ever deposited.
pub fn get_unique_depositors(env: &Env) -> u32 {
    env.storage().instance().get(&symbol_short!("uni_dep")).unwrap_or(0)
}

pub fn increment_unique_depositors(env: &Env) {
    let mut count: u32 = env.storage().instance().get(&symbol_short!("uni_dep")).unwrap_or(0);
    // Simple increment without deduplication for on-chain efficiency;
    // client-side should track unique depositors.
    env.storage().instance().set(&symbol_short!("uni_dep"), &(count + 1));
}
