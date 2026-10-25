# Time-based feature units

Soroban exposes two different clocks:

- `ledger().sequence()` is the monotonically increasing ledger number.
- `ledger().timestamp()` is the close time in Unix seconds.

Time-based contract rules use ledger sequence numbers. They never convert a
configured number of ledgers to seconds for enforcing a deadline. Ledger close
time is not fixed at five seconds, so any ledger-to-time estimate is
approximate only.

| Feature | Unit used for configuration and enforcement | Boundary |
| --- | --- | --- |
| Position lock, early-exit penalty, and lock boost (`vault.rs`) | ledgers (`*_ledgers`); start and unlock are ledger sequences | Unlocks at the configured unlock ledger |
| Lock-extension boost (`lock_extension.rs`) | additional/max extension ledgers | Accumulated against ledger-based position history |
| Transfer cooldown (`transfer_cooldown.rs`) | ledgers (`*_ledgers`); transfer receipt records a ledger sequence | Clears once the cooldown ledgers have elapsed |
| Withdrawal request cooldown (`withdrawal_cooldown.rs`) | ledgers (`*_ledgers`); request stores a ledger sequence | Executable at the exact elapsed-ledger boundary |
| Claim and anti-dump cooldowns (`claim_streak.rs`, `anti_dump_claim_cooldown.rs`, `mev_claim_protection.rs`) | ledgers (`*_ledgers`); last-action fields store ledger sequences | Eligible at the configured ledger boundary |
| Reward vesting and cliff (`claim_vesting.rs`, `vesting_cliff.rs`, `combined_vesting.rs`) | ledgers (`*_ledgers`); schedules store start/stake ledger sequences | Cliff and completed vesting are inclusive at the boundary |
| Admin recovery, succession, and announced config changes (`admin_recovery.rs`, `admin_succession.rs`, `time_locked_admin_proposal.rs`) | delay ledgers; proposal timestamps are ledger sequences | Executable at the exact target ledger |
| Timelocked admin actions (`balance.rs`, `vault_extensions_502_505.rs`) | delay ledgers; execution checks ledger sequence | Executable at the exact target ledger |
| Fee-change notices and pause/grace periods (`fee_change_notice.rs`, `pause_grace_period.rs`, `guardian_pause.rs`) | ledgers | Effective at the configured ledger boundary |
| Reward ramps, epochs, and rolling rate windows (`vault_extensions_538_541.rs`, `reward_epochs.rs`, `twa_reward_rate.rs`, `vault.rs`) | elapsed/window ledgers | Calculations use ledger sequence deltas |
| Age, heartbeat, and season gates (`boost_activation_age.rs`, `position_heartbeat.rs`, `competitive_season.rs`) | age/interval/season ledgers | Evaluated against ledger sequence |
| Activity-log metadata (`activity_log.rs`) | ledger sequence plus Unix timestamp seconds | Both values are observations only; neither controls eligibility |
| Withdrawal-queue metadata (`vault_extensions_502_505.rs`) | request timestamp seconds | Metadata only; queue ordering/processing does not use it as a deadline |

Public durations, cooldowns, delays, and windows should include `_ledgers` in
their parameter names. Unix wall-clock values should include `_seconds` (or
`_timestamp_seconds`) and must not be compared with a ledger sequence.

The shared inclusive comparison helpers are in `ledger_boundary.rs`. The
feature boundary tests and helper tests cover the ledger immediately before,
at, and after a deadline.
