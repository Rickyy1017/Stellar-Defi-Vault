use soroban_sdk::{contractclient, Address, Env};

#[contractclient(name = "IStakingPoolClient")]
pub trait IStakingPool {
    fn staked_amount(env: Env, user: Address) -> i128;
    fn pending_reward(env: Env, user: Address) -> i128;
    fn total_staked(env: Env) -> i128;
    fn is_paused(env: Env) -> bool;
}
