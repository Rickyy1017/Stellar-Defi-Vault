use soroban_sdk::{symbol_short, Address, Env};
use crate::storage::AdminAction;

pub fn deposit(env: &Env, depositor: &Address, amount: i128, shares_minted: i128) {
    let topics = (symbol_short!("deposit"), depositor);
    env.events()
        .publish(topics, (amount, shares_minted, env.ledger().sequence()));
}

pub fn withdraw(env: &Env, withdrawer: &Address, shares_burned: i128, amount_returned: i128) {
    let topics = (symbol_short!("withdraw"), withdrawer);
    env.events()
        .publish(topics, (shares_burned, amount_returned, env.ledger().sequence()));
}

pub fn paused(env: &Env, admin: &Address) {
    let topics = (symbol_short!("paused"), admin);
    env.events().publish(topics, (env.ledger().sequence(),));
}

pub fn unpaused(env: &Env, admin: &Address) {
    let topics = (symbol_short!("unpaused"), admin);
    env.events().publish(topics, (env.ledger().sequence(),));
}

pub fn yield_added(env: &Env, admin: &Address, amount: i128) {
    let topics = (symbol_short!("yield_add"), admin);
    env.events()
        .publish(topics, (amount, env.ledger().sequence()));
}

pub fn admin_changed(env: &Env, old_admin: &Address, new_admin: &Address) {
    let topics = (symbol_short!("admin_set"), old_admin);
    env.events()
        .publish(topics, (new_admin, env.ledger().sequence()));
}

pub fn withdrawal_limit_updated(env: &Env, admin: &Address, new_limit: i128) {
    let topics = (symbol_short!("wd_limit"), admin);
    env.events()
        .publish(topics, (new_limit, env.ledger().sequence()));
}

pub fn rate_changed(env: &Env, old_rate_bps: u32, new_rate_bps: u32) {
    let topics = (symbol_short!("rate_chg"),);
    env.events()
        .publish(topics, (old_rate_bps, new_rate_bps, env.ledger().sequence()));
}

pub fn pool_cap_updated(env: &Env, admin: &Address, new_cap: i128) {
    let topics = (symbol_short!("cap_upd"), admin);
    env.events()
        .publish(topics, (new_cap, env.ledger().sequence()));
}

pub fn position_opened(env: &Env, user: &Address, amount: i128) {
    let topics = (symbol_short!("pos_open"), user);
    env.events()
        .publish(topics, (amount, env.ledger().sequence()));
}

pub fn position_closed(env: &Env, user: &Address) {
    let topics = (symbol_short!("pos_clos"), user);
    env.events().publish(topics, (env.ledger().sequence(),));
}

// ── Issue #39: rescue token event ────────────────────────────────────────────

pub fn token_rescued(env: &Env, token: &Address, amount: i128, recipient: &Address) {
    let topics = (symbol_short!("tk_rescue"),);
    env.events().publish(topics, (token.clone(), amount, recipient.clone(), env.ledger().sequence()));
}

// ── Issue #42: admin action audit events ─────────────────────────────────────

pub fn admin_action_set_reward_rate(env: &Env, actor: &Address, old_rate: u32, new_rate: u32) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::SetRewardRate),
        (actor.clone(), env.ledger().sequence(), old_rate, new_rate),
    );
}

pub fn admin_action_pause(env: &Env, actor: &Address) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::Pause),
        (actor.clone(), env.ledger().sequence()),
    );
}

pub fn admin_action_unpause(env: &Env, actor: &Address) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::Unpause),
        (actor.clone(), env.ledger().sequence()),
    );
}

pub fn admin_action_transfer_admin(env: &Env, actor: &Address, new_admin: &Address) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::TransferAdmin),
        (actor.clone(), env.ledger().sequence(), new_admin.clone()),
    );
}

pub fn admin_action_set_lock_period(env: &Env, actor: &Address, new_ledgers: u32) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::SetLockPeriod),
        (actor.clone(), env.ledger().sequence(), new_ledgers),
    );
}

pub fn admin_action_set_cap(env: &Env, actor: &Address, new_limit: i128) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::SetCap),
        (actor.clone(), env.ledger().sequence(), new_limit),
    );
}

pub fn admin_action_rescue_token(env: &Env, actor: &Address, token: &Address, amount: i128, recipient: &Address) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::RescueToken),
        (actor.clone(), env.ledger().sequence(), token.clone(), amount, recipient.clone()),
    );
}

pub fn admin_action_set_early_exit_penalty(env: &Env, actor: &Address, new_bps: u32) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::SetEarlyExitPenalty),
        (actor.clone(), env.ledger().sequence(), new_bps),
    );
}

pub fn admin_action_set_min_stake(env: &Env, actor: &Address, new_amount: i128) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::SetMinStake),
        (actor.clone(), env.ledger().sequence(), new_amount),
    );
}

pub fn admin_action_fund_reward_pool(env: &Env, actor: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::FundRewardPool),
        (actor.clone(), env.ledger().sequence(), amount),
    );
}

pub fn admin_action_add_yield(env: &Env, actor: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::AddYield),
        (actor.clone(), env.ledger().sequence(), amount),
    );
}

pub fn admin_action_set_boost_schedule(env: &Env, actor: &Address, num_tiers: u32) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::SetBoostSchedule),
        (actor.clone(), env.ledger().sequence(), num_tiers),
    );
}

pub fn admin_action_set_nft_contract(env: &Env, actor: &Address, nft_addr: &Address) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::SetNftContract),
        (actor.clone(), env.ledger().sequence(), nft_addr.clone()),
    );
}

pub fn admin_action_set_restake_window(env: &Env, actor: &Address, window: u32) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::SetRestakeWindow),
        (actor.clone(), env.ledger().sequence(), window),
    );
}

pub fn admin_action_set_reward_token(env: &Env, actor: &Address, token: &Address) {
    env.events().publish(
        (symbol_short!("adm_act"), AdminAction::SetRewardToken),
        (actor.clone(), env.ledger().sequence(), token.clone()),
    );
}

pub fn slash(env: &Env, admin: &Address, user: &Address, amount: i128) {
    let topics = (symbol_short!("slash"), admin);
    env.events()
        .publish(topics, (user.clone(), amount, env.ledger().sequence()));
}

pub fn position_transferred(env: &Env, from: &Address, to: &Address, amount: i128) {
    let topics = (symbol_short!("pos_xfer"), from);
    env.events()
        .publish(topics, (to.clone(), amount, env.ledger().sequence()));
}

pub fn campaign_started(env: &Env, admin: &Address, multiplier_bps: u32, ends_at_ledger: u32) {
    let topics = (symbol_short!("camp_on"), admin);
    env.events()
        .publish(topics, (multiplier_bps, ends_at_ledger, env.ledger().sequence()));
}

pub fn campaign_ended(env: &Env, admin: &Address) {
    let topics = (symbol_short!("camp_off"), admin);
    env.events().publish(topics, (env.ledger().sequence(),));
}

/// Emitted when a user claims staking rewards (via `claim` or `stake_and_claim`).
pub fn claimed(env: &Env, user: &Address, reward: i128) {
    let topics = (symbol_short!("claimed"), user);
    env.events()
        .publish(topics, (reward, env.ledger().sequence()));
}

/// Emitted by `initialize` so indexers can detect new pool deployments on-chain.
pub fn pool_initialized(
    env: &Env,
    admin: &Address,
    stake_token: &Address,
    reward_token: &Address,
    reward_rate_bps: u32,
) {
    let topics = (symbol_short!("init"),);
    env.events().publish(
        topics,
        (admin.clone(), stake_token.clone(), reward_token.clone(), reward_rate_bps),
    );
}

// ── Issue #101: frozen position event ─────────────────────────────────────────

/// Emitted when an admin flags a user's position as frozen via `flag_frozen`.
pub fn frozen_position(env: &Env, admin: &Address, user: &Address, frozen_at: u32) {
    let topics = (symbol_short!("frz_pos"), admin);
    env.events()
        .publish(topics, (user.clone(), frozen_at, env.ledger().sequence()));
}

// ── Issue #107: emergency stop event ─────────────────────────────────────────

/// Emitted once when `emergency_stop` is triggered. This event is permanent
/// and irreversible — the contract will never accept new stakes again.
pub fn stopped(env: &Env, admin: &Address) {
    let topics = (symbol_short!("stopped"), admin);
    env.events().publish(topics, env.ledger().sequence());
}

// ── Issue #97: pool description event ────────────────────────────────────────

/// Emitted when the admin updates the pool description.
pub fn description_updated(env: &Env, admin: &Address, description: &soroban_sdk::String) {
    let topics = (symbol_short!("desc_upd"), admin);
    env.events()
        .publish(topics, (description.clone(), env.ledger().sequence()));
}

// ── Issue #129: auto-pause event ──────────────────────────────────────────────

/// Emitted when the pool auto-pauses because reward balance fell below threshold.
pub fn auto_paused(env: &Env, reward_balance: i128, threshold: i128) {
    let topics = (symbol_short!("auto_paus"),);
    env.events()
        .publish(topics, (reward_balance, threshold, env.ledger().sequence()));
}

// ── Issue #130: KYC status changed event ──────────────────────────────────────

/// Emitted for each address updated by bulk_set_kyc.
pub fn kyc_status_changed(env: &Env, user: &Address, approved: bool) {
    let topics = (symbol_short!("kyc_chg"), user);
    env.events()
        .publish(topics, (approved, env.ledger().sequence()));
}
