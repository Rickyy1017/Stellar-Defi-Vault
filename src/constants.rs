//! Shared protocol units and arithmetic scales.
//!
//! Keep these values limited to protocol-wide conventions. Feature-specific
//! policy caps (batch sizes, fee ceilings, and field lengths) remain with the
//! module that owns their behavior.

/// Basis points in one whole: 10,000 bps = 100%.
///
/// This denominator is used when converting bps rates, fees, and proportions
/// into integer fractions. Keep the `i128` form for amount arithmetic.
pub(crate) const BPS_DENOMINATOR: i128 = 10_000;

/// The basis-point denominator in the `u32` domain, for bps fields and caps.
pub(crate) const BPS_DENOMINATOR_U32: u32 = BPS_DENOMINATOR as u32;

/// Approximate Stellar ledgers in one day, assuming the protocol's 5-second
/// ledger-close target (86,400 seconds / 5 seconds per ledger).
pub(crate) const LEDGERS_PER_DAY: u32 = 17_280;

/// Approximate Stellar ledgers in one 7-day week at the 5-second target.
/// Used for weekly windows and forecast horizons.
pub(crate) const LEDGERS_PER_WEEK: u32 = LEDGERS_PER_DAY * 7;

/// Approximate Stellar ledgers in one 365-day year at the 5-second target.
/// This is the annualization unit used by reward-rate calculations.
pub(crate) const STELLAR_LEDGERS_PER_YEAR: u32 = LEDGERS_PER_DAY * 365;
