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
