//! Standalone linear vesting contract for team and partner allocations
//! (issue #935).
//!
//! Holds reward tokens and releases them linearly over `duration_ledgers`
//! starting from schedule creation, subject to an optional `cliff_ledgers`
//! delay and optional `revocable` flag.

#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing))]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol,
};

const TOKEN_KEY: Symbol = symbol_short!("v_tok");
const SCHED_KEY: Symbol = symbol_short!("v_sched");

/// Errors returned by [`LinearVestingContract`].
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VestingError {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Contract has not been initialized yet.
    NotInitialized = 2,
    /// Allocation amount must be strictly positive.
    InvalidAmount = 3,
    /// Vesting duration must be strictly positive.
    InvalidDuration = 4,
    /// Cliff duration cannot exceed total vesting duration.
    InvalidCliff = 5,
    /// A vesting schedule already exists for this beneficiary.
    ScheduleAlreadyExists = 6,
    /// No vesting schedule exists for this beneficiary.
    ScheduleNotFound = 7,
    /// Schedule is marked as non-revocable.
    NotRevocable = 8,
    /// Schedule has already been revoked.
    AlreadyRevoked = 9,
    /// Caller is not the schedule funder.
    Unauthorized = 10,
    /// Arithmetic overflow.
    ArithmeticError = 11,
}

/// Linear vesting schedule for a single beneficiary.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingSchedule {
    /// Address that funded the allocation and may revoke it if `revocable`.
    pub funder: Address,
    /// Recipient of the vested tokens.
    pub beneficiary: Address,
    /// Total tokens allocated (capped at the vested amount upon revocation).
    pub total_amount: i128,
    /// Tokens already released to `beneficiary`.
    pub released_amount: i128,
    /// Ledger sequence when vesting started.
    pub start_ledger: u32,
    /// Number of ledgers from `start_ledger` before any tokens vest.
    pub cliff_ledgers: u32,
    /// Total vesting duration in ledgers from `start_ledger`.
    pub duration_ledgers: u32,
    /// Whether `funder` may revoke unvested tokens.
    pub revocable: bool,
    /// Whether the schedule has been revoked.
    pub revoked: bool,
}

fn vested_amount(schedule: &VestingSchedule, current_ledger: u32) -> Result<i128, VestingError> {
    if schedule.revoked {
        return Ok(schedule.total_amount);
    }
    let elapsed = current_ledger.saturating_sub(schedule.start_ledger);
    if elapsed < schedule.cliff_ledgers {
        return Ok(0);
    }
    if elapsed >= schedule.duration_ledgers {
        return Ok(schedule.total_amount);
    }
    schedule
        .total_amount
        .checked_mul(elapsed as i128)
        .and_then(|v| v.checked_div(schedule.duration_ledgers as i128))
        .ok_or(VestingError::ArithmeticError)
}

#[contract]
pub struct LinearVestingContract;

#[contractimpl]
impl LinearVestingContract {
    /// Initialize the vesting contract with the reward `token` address.
    pub fn initialize(env: Env, token: Address) -> Result<(), VestingError> {
        if env.storage().instance().has(&TOKEN_KEY) {
            return Err(VestingError::AlreadyInitialized);
        }
        env.storage().instance().set(&TOKEN_KEY, &token);
        Ok(())
    }

    /// Create a linear vesting schedule for `beneficiary` funded by `funder`.
    ///
    /// Transfers `amount` tokens from `funder` into the contract.
    pub fn create_schedule(
        env: Env,
        funder: Address,
        beneficiary: Address,
        amount: i128,
        cliff_ledgers: u32,
        duration_ledgers: u32,
        revocable: bool,
    ) -> Result<(), VestingError> {
        funder.require_auth();

        let token_addr: Address = env
            .storage()
            .instance()
            .get(&TOKEN_KEY)
            .ok_or(VestingError::NotInitialized)?;

        if amount <= 0 {
            return Err(VestingError::InvalidAmount);
        }
        if duration_ledgers == 0 {
            return Err(VestingError::InvalidDuration);
        }
        if cliff_ledgers > duration_ledgers {
            return Err(VestingError::InvalidCliff);
        }

        let key = (SCHED_KEY, beneficiary.clone());
        if env.storage().persistent().has(&key) {
            return Err(VestingError::ScheduleAlreadyExists);
        }

        token::Client::new(&env, &token_addr).transfer(
            &funder,
            &env.current_contract_address(),
            &amount,
        );

        let schedule = VestingSchedule {
            funder: funder.clone(),
            beneficiary: beneficiary.clone(),
            total_amount: amount,
            released_amount: 0,
            start_ledger: env.ledger().sequence(),
            cliff_ledgers,
            duration_ledgers,
            revocable,
            revoked: false,
        };
        env.storage().persistent().set(&key, &schedule);

        env.events().publish(
            (symbol_short!("v_create"), funder, beneficiary),
            (amount, cliff_ledgers, duration_ledgers, revocable),
        );

        Ok(())
    }

    /// Release all currently vested, unreleased tokens to `beneficiary`.
    pub fn release(env: Env, beneficiary: Address) -> Result<i128, VestingError> {
        beneficiary.require_auth();

        let token_addr: Address = env
            .storage()
            .instance()
            .get(&TOKEN_KEY)
            .ok_or(VestingError::NotInitialized)?;

        let key = (SCHED_KEY, beneficiary.clone());
        let mut schedule: VestingSchedule = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(VestingError::ScheduleNotFound)?;

        let vested = vested_amount(&schedule, env.ledger().sequence())?;
        let releasable = vested.saturating_sub(schedule.released_amount);
        if releasable <= 0 {
            return Ok(0);
        }

        schedule.released_amount = schedule
            .released_amount
            .checked_add(releasable)
            .ok_or(VestingError::ArithmeticError)?;
        env.storage().persistent().set(&key, &schedule);

        token::Client::new(&env, &token_addr).transfer(
            &env.current_contract_address(),
            &beneficiary,
            &releasable,
        );

        env.events().publish(
            (symbol_short!("v_rel"), beneficiary),
            (releasable, env.ledger().sequence()),
        );

        Ok(releasable)
    }

    /// Revoke a revocable schedule and return only the unvested tokens to
    /// `funder`. Any tokens already vested remain claimable by `beneficiary`.
    pub fn revoke(env: Env, funder: Address, beneficiary: Address) -> Result<i128, VestingError> {
        funder.require_auth();

        let token_addr: Address = env
            .storage()
            .instance()
            .get(&TOKEN_KEY)
            .ok_or(VestingError::NotInitialized)?;

        let key = (SCHED_KEY, beneficiary.clone());
        let mut schedule: VestingSchedule = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(VestingError::ScheduleNotFound)?;

        if schedule.funder != funder {
            return Err(VestingError::Unauthorized);
        }
        if !schedule.revocable {
            return Err(VestingError::NotRevocable);
        }
        if schedule.revoked {
            return Err(VestingError::AlreadyRevoked);
        }

        let vested = vested_amount(&schedule, env.ledger().sequence())?;
        let unvested = schedule.total_amount.saturating_sub(vested);

        schedule.total_amount = vested;
        schedule.revoked = true;
        env.storage().persistent().set(&key, &schedule);

        if unvested > 0 {
            token::Client::new(&env, &token_addr).transfer(
                &env.current_contract_address(),
                &funder,
                &unvested,
            );
        }

        env.events().publish(
            (symbol_short!("v_revk"), funder, beneficiary),
            (unvested, vested, env.ledger().sequence()),
        );

        Ok(unvested)
    }

    /// Read-only query for a beneficiary's vesting schedule.
    pub fn get_schedule(env: Env, beneficiary: Address) -> Option<VestingSchedule> {
        env.storage()
            .persistent()
            .get(&(SCHED_KEY, beneficiary))
    }

    /// Read-only query for the currently releasable token amount.
    pub fn releasable_amount(env: Env, beneficiary: Address) -> i128 {
        let Some(schedule) = Self::get_schedule(env.clone(), beneficiary) else {
            return 0;
        };
        let Ok(vested) = vested_amount(&schedule, env.ledger().sequence()) else {
            return 0;
        };
        vested.saturating_sub(schedule.released_amount).max(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger as _};

    struct Fixture<'a> {
        env: Env,
        vesting: LinearVestingContractClient<'a>,
        token: token::Client<'a>,
        funder: Address,
        beneficiary: Address,
    }

    impl<'a> Fixture<'a> {
        fn new() -> Self {
            let env = Env::default();
            env.mock_all_auths();
            env.ledger().with_mut(|li| {
                li.sequence_number = 1_000;
            });

            let admin = Address::generate(&env);
            let funder = Address::generate(&env);
            let beneficiary = Address::generate(&env);

            let token_addr = env.register_stellar_asset_contract(admin);
            let token = token::Client::new(&env, &token_addr);
            let token_admin = token::StellarAssetClient::new(&env, &token_addr);
            token_admin.mint(&funder, &10_000_000);

            let contract_id = env.register_contract(None, LinearVestingContract);
            let vesting = LinearVestingContractClient::new(&env, &contract_id);
            vesting.initialize(&token_addr);

            Fixture {
                env,
                vesting,
                token,
                funder,
                beneficiary,
            }
        }

        fn advance(&self, ledgers: u32) {
            self.env.ledger().with_mut(|li| {
                li.sequence_number += ledgers;
            });
        }
    }

    #[test]
    fn test_before_cliff() {
        let f = Fixture::new();
        f.vesting
            .create_schedule(&f.funder, &f.beneficiary, &1_000_000, &200, &1_000, &true);

        // At 199 ledgers (< 200 cliff), nothing is vested or released.
        f.advance(199);
        assert_eq!(f.vesting.releasable_amount(&f.beneficiary), 0);
        assert_eq!(f.vesting.release(&f.beneficiary), 0);
        assert_eq!(f.token.balance(&f.beneficiary), 0);

        // At 200 ledgers (cliff reached), 20% (200_000) vests.
        f.advance(1);
        assert_eq!(f.vesting.release(&f.beneficiary), 200_000);
        assert_eq!(f.token.balance(&f.beneficiary), 200_000);
    }

    #[test]
    fn test_mid_vesting() {
        let f = Fixture::new();
        f.vesting
            .create_schedule(&f.funder, &f.beneficiary, &1_000_000, &0, &1_000, &false);

        f.advance(500);
        assert_eq!(f.vesting.release(&f.beneficiary), 500_000);
        assert_eq!(f.token.balance(&f.beneficiary), 500_000);

        f.advance(250);
        assert_eq!(f.vesting.release(&f.beneficiary), 250_000);
        assert_eq!(f.token.balance(&f.beneficiary), 750_000);
    }

    #[test]
    fn test_after_end() {
        let f = Fixture::new();
        f.vesting
            .create_schedule(&f.funder, &f.beneficiary, &1_000_000, &100, &1_000, &false);

        f.advance(2_000);
        assert_eq!(f.vesting.release(&f.beneficiary), 1_000_000);
        assert_eq!(f.token.balance(&f.beneficiary), 1_000_000);

        // Subsequent release after full vesting returns 0.
        assert_eq!(f.vesting.release(&f.beneficiary), 0);
    }

    #[test]
    fn test_revoke() {
        let f = Fixture::new();
        f.vesting
            .create_schedule(&f.funder, &f.beneficiary, &1_000_000, &100, &1_000, &true);

        // Advance to 40% vested (400_000 vested, 600_000 unvested).
        f.advance(400);
        let unvested = f.vesting.revoke(&f.funder, &f.beneficiary);
        assert_eq!(unvested, 600_000);
        assert_eq!(f.token.balance(&f.funder), 9_600_000);

        // Even after advancing past original end, beneficiary only receives the
        // 400_000 that had vested prior to revocation.
        f.advance(1_000);
        assert_eq!(f.vesting.release(&f.beneficiary), 400_000);
        assert_eq!(f.token.balance(&f.beneficiary), 400_000);

        // Second revoke fails.
        assert_eq!(
            f.vesting.try_revoke(&f.funder, &f.beneficiary),
            Err(Ok(VestingError::AlreadyRevoked))
        );
    }
}
