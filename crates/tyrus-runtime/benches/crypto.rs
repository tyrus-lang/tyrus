use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use tyrus_runtime::crypto_bench;

fn crypto_throughput(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("crypto-throughput");
    let key = [0x42; 32];
    let aad = b"benchmark-aad";
    for size in [0usize, 64, 1024, 64 * 1024, 1024 * 1024] {
        let payload = vec![0x5a; size];
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("sha256", size), &payload, |bench, data| {
            bench.iter(|| crypto_bench::sha256(black_box(data)));
        });
        group.bench_with_input(
            BenchmarkId::new("hmac-sha256", size),
            &payload,
            |bench, data| {
                bench.iter(|| crypto_bench::hmac_sha256(black_box(data), black_box(&key)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("aes-256-gcm-round-trip", size),
            &payload,
            |bench, data| {
                bench.iter(|| crypto_bench::aes_round_trip(black_box(data), black_box(aad)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("aes-256-gcm-one-shot", size),
            &payload,
            |bench, data| {
                bench.iter(|| crypto_bench::aes_encrypt_one_shot(black_box(data), black_box(aad)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("aes-256-gcm-in-place", size),
            &payload,
            |bench, data| {
                bench.iter_batched(
                    || data.clone(),
                    |mut buffer| {
                        crypto_bench::aes_encrypt_in_place(black_box(&mut buffer), black_box(aad));
                        black_box(buffer)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

fn password_hashing(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("password-hashing");
    group.sample_size(10);
    group.bench_function("argon2id-m65536-t3-p4", |bench| {
        bench.iter(|| crypto_bench::argon2id(black_box(b"benchmark-password")));
    });
    group.finish();
}

criterion_group!(benches, crypto_throughput, password_hashing);
criterion_main!(benches);
