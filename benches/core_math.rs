//! Criterion benchmarks for pure-Rust core math helpers (issue #931).
//!
//! Tracks the host-CPU cost of `mul_div`, share conversion, and reward accrual
//! separately from Soroban's on-chain budget tests. Run with:
//!
//! ```sh
//! cargo bench --bench core_math --features testutils
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use stellar_defi_vault::fuzz_support::{
    amount_to_shares, amount_to_shares_with_decimals, mul_div_ceil, mul_div_floor,
    reward_for_ledgers, shares_to_amount, shares_to_amount_with_decimals,
};

fn bench_mul_div(c: &mut Criterion) {
    let mut group = c.benchmark_group("mul_div");

    group.bench_function("mul_div_floor/small", |b| {
        b.iter(|| {
            mul_div_floor(
                black_box(1_000_000_000),
                black_box(2_500),
                black_box(10_000),
            )
        })
    });

    group.bench_function("mul_div_ceil/small", |b| {
        b.iter(|| {
            mul_div_ceil(
                black_box(1_000_000_000),
                black_box(2_501),
                black_box(10_000),
            )
        })
    });

    // Intermediate product exceeds i128::MAX and exercises full 256-bit division.
    let large_a: i128 = 1_000_000_000_000_000_000_000_000;
    let large_b: i128 = 500_000_000_000_000_000_000_000;
    let large_c: i128 = 750_000_000_000_000_000_000_000;

    group.bench_function("mul_div_floor/u256_intermediate", |b| {
        b.iter(|| mul_div_floor(black_box(large_a), black_box(large_b), black_box(large_c)))
    });

    group.bench_function("mul_div_ceil/u256_intermediate", |b| {
        b.iter(|| mul_div_ceil(black_box(large_a), black_box(large_b), black_box(large_c)))
    });

    group.bench_function("mul_div_floor/negative", |b| {
        b.iter(|| {
            mul_div_floor(
                black_box(-9_876_543_210_123_456),
                black_box(333_333_333),
                black_box(10_000),
            )
        })
    });

    group.finish();
}

fn bench_share_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("share_conversion");

    group.bench_function("amount_to_shares/initial_deposit", |b| {
        b.iter(|| amount_to_shares(black_box(0), black_box(0), black_box(50_000_000_000)))
    });

    group.bench_function("amount_to_shares/proportional", |b| {
        b.iter(|| {
            amount_to_shares(
                black_box(12_345_678_900_000),
                black_box(15_000_000_000_000),
                black_box(250_000_000_000),
            )
        })
    });

    for decimals in [6u32, 7, 18] {
        group.bench_with_input(
            BenchmarkId::new("amount_to_shares_with_decimals/initial", decimals),
            &decimals,
            |b, &dec| {
                b.iter(|| {
                    amount_to_shares_with_decimals(
                        black_box(0),
                        black_box(0),
                        black_box(1_000_000_000_000),
                        black_box(dec),
                    )
                })
            },
        );
    }

    group.bench_function("shares_to_amount/proportional", |b| {
        b.iter(|| {
            shares_to_amount(
                black_box(12_345_678_900_000),
                black_box(15_000_000_000_000),
                black_box(250_000_000_000),
            )
        })
    });

    group.bench_function("shares_to_amount_with_decimals/proportional", |b| {
        b.iter(|| {
            shares_to_amount_with_decimals(
                black_box(12_345_678_900_000),
                black_box(15_000_000_000_000),
                black_box(250_000_000_000),
                black_box(18),
            )
        })
    });

    group.finish();
}

fn bench_reward_accrual(c: &mut Criterion) {
    let mut group = c.benchmark_group("reward_accrual");

    group.bench_function("reward_for_ledgers/zero_elapsed", |b| {
        b.iter(|| {
            reward_for_ledgers(
                black_box(1_000_000_000),
                black_box(1_200),
                black_box(10_000),
                black_box(10_000),
                black_box(10_000),
                black_box(0),
            )
        })
    });

    group.bench_function("reward_for_ledgers/base_rate", |b| {
        b.iter(|| {
            reward_for_ledgers(
                black_box(1_000_000_000_000),
                black_box(1_200),
                black_box(10_000),
                black_box(10_000),
                black_box(10_000),
                black_box(17_280),
            )
        })
    });

    group.bench_function("reward_for_ledgers/stacked_multipliers", |b| {
        b.iter(|| {
            reward_for_ledgers(
                black_box(50_000_000_000_000),
                black_box(1_500),
                black_box(15_000),
                black_box(12_500),
                black_box(11_000),
                black_box(6_307_200),
            )
        })
    });

    group.bench_function("reward_for_ledgers/large_amount", |b| {
        b.iter(|| {
            reward_for_ledgers(
                black_box(i128::MAX / 1_000_000),
                black_box(2_500),
                black_box(20_000),
                black_box(15_000),
                black_box(12_000),
                black_box(1_000_000),
            )
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_mul_div,
    bench_share_conversion,
    bench_reward_accrual
);
criterion_main!(benches);
