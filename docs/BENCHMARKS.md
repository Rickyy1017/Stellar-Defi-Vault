# Core Math Benchmarks

Pure-Rust Criterion benchmarks for `Stellar-Defi-Vault`'s arithmetic helpers live in [`benches/core_math.rs`](../benches/core_math.rs) (issue #931). They measure host CPU timing of `mul_div`, share conversion, and reward accrual independently of Soroban's on-chain budget tests (`src/test_resource_costs.rs`).

## Benchmark Groups

| Group | Helpers Covered | Source Module |
|:---|:---|:---|
| `mul_div` | `mul_div_floor`, `mul_div_ceil` (small, 256-bit intermediate, negative) | `src/math.rs` |
| `share_conversion` | `amount_to_shares`, `amount_to_shares_with_decimals`, `shares_to_amount`, `shares_to_amount_with_decimals` | `src/balance/shares.rs` |
| `reward_accrual` | `reward_for_ledgers` (zero elapsed, base rate, stacked multipliers, large amount) | `src/reward_math.rs` |

## Running Locally

All benchmark targets require the `testutils` feature so the pure-math helpers remain gated out of release WASM builds:

```sh
cargo bench --bench core_math --features testutils
```

To run a single group or filter by benchmark name:

```sh
cargo bench --bench core_math --features testutils -- mul_div
cargo bench --bench core_math --features testutils -- share_conversion
cargo bench --bench core_math --features testutils -- reward_accrual
```

## Comparing Against `main`

1. Save a baseline from `main`:

   ```sh
   git switch main
   cargo bench --bench core_math --features testutils -- --save-baseline main
   ```

2. Switch back to your feature branch and compare against the saved `main` baseline:

   ```sh
   git switch -
   cargo bench --bench core_math --features testutils -- --baseline main
   ```

Criterion prints per-benchmark time deltas and highlights statistically significant regressions or improvements, and writes HTML reports under `target/criterion/report/index.html`.

## CI Integration

Pull requests that modify `src/math.rs`, `src/reward_math.rs`, `src/balance/shares.rs`, or `benches/**` trigger the optional [`bench-math.yml`](../.github/workflows/bench-math.yml) workflow, which runs the benchmark suite and writes a summary to the GitHub Actions job summary.
