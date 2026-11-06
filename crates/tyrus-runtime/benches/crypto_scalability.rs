use std::hint::black_box;
use std::time::{Duration, Instant};
use tyrus_runtime::crypto_bench;

const PAYLOAD_BYTES: usize = 1024 * 1024;
const ITERATIONS_PER_WORKER: usize = 16;
const MAX_WORKERS: usize = 4;
const MIN_PARALLEL_THROUGHPUT_RATIO: f64 = 0.70;
const DEFAULT_MAX_RSS_MIB: u64 = 128;

struct LatencyBudget {
    environment: &'static str,
    default_micros: u64,
}

fn configured_budget(budget: &LatencyBudget) -> Duration {
    let micros = std::env::var(budget.environment)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(budget.default_micros);
    Duration::from_micros(micros)
}

fn percentile_95(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[(samples.len() * 95).div_ceil(100).saturating_sub(1)]
}

fn latency_gate(
    failures: &mut Vec<String>,
    operation: &str,
    samples: usize,
    budget: LatencyBudget,
    mut run: impl FnMut(),
) {
    run();
    let mut durations = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        run();
        durations.push(started.elapsed());
    }
    let p95 = percentile_95(durations);
    let maximum = configured_budget(&budget);
    println!(
        "latency,{operation},p95_us,{},{},{}",
        p95.as_micros(),
        maximum.as_micros(),
        budget.environment
    );
    if p95 > maximum {
        failures.push(format!(
            "{operation}: p95 {} us exceeds {} us ({})",
            p95.as_micros(),
            maximum.as_micros(),
            budget.environment
        ));
    }
}

fn elapsed_for(operation: &(impl Fn(&[u8]) + Sync), payload: &[u8], iterations: usize) -> Duration {
    let started = Instant::now();
    for _ in 0..iterations {
        operation(black_box(payload));
    }
    started.elapsed()
}

fn scalability_gate(
    failures: &mut Vec<String>,
    operation_name: &str,
    workers: usize,
    payload: &[u8],
    operation: impl Fn(&[u8]) + Sync,
) {
    operation(payload);
    let total_iterations = workers * ITERATIONS_PER_WORKER;
    let sequential = elapsed_for(&operation, payload, total_iterations);
    let parallel_started = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let operation = &operation;
            scope.spawn(move || {
                for _ in 0..ITERATIONS_PER_WORKER {
                    operation(black_box(payload));
                }
            });
        }
    });
    let parallel = parallel_started.elapsed();
    let sequential_throughput = total_iterations as f64 / sequential.as_secs_f64();
    let parallel_throughput = total_iterations as f64 / parallel.as_secs_f64();
    let ratio = parallel_throughput / sequential_throughput;
    println!(
        "scalability,{operation_name},{workers},{sequential_throughput:.2},{parallel_throughput:.2},{ratio:.3}"
    );
    if workers > 1 && ratio < MIN_PARALLEL_THROUGHPUT_RATIO {
        failures.push(format!(
            "{operation_name}: parallel throughput ratio {ratio:.3} is below {MIN_PARALLEL_THROUGHPUT_RATIO:.2}"
        ));
    }
}

#[cfg(target_os = "linux")]
fn resident_memory_gate(failures: &mut Vec<String>) {
    let maximum_mib = std::env::var("TYRUS_CRYPTO_MAX_RSS_MIB")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MAX_RSS_MIB);
    let status = std::fs::read_to_string("/proc/self/status")
        .expect("Linux performance gate must be able to read /proc/self/status");
    let peak_kib = status
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")
                .and_then(|value| value.split_whitespace().next())
                .and_then(|value| value.parse::<u64>().ok())
        })
        .expect("Linux /proc/self/status must expose VmHWM");
    let maximum_kib = maximum_mib * 1024;
    println!("memory,process-peak-rss,kib,{peak_kib},{maximum_kib},TYRUS_CRYPTO_MAX_RSS_MIB");
    if peak_kib > maximum_kib {
        failures.push(format!(
            "process peak RSS {peak_kib} KiB exceeds {maximum_kib} KiB (TYRUS_CRYPTO_MAX_RSS_MIB)"
        ));
    }
}

#[cfg(not(target_os = "linux"))]
fn resident_memory_gate(_: &mut Vec<String>) {
    println!("memory,process-peak-rss,unsupported,0,0,platform");
}

fn main() {
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(MAX_WORKERS);
    let payload = vec![0x5a; PAYLOAD_BYTES];
    let key = [0x42; 32];
    let aad = b"benchmark-aad";
    let mut failures = Vec::new();

    println!(
        "kind,operation,workers_or_percentile,measured,budget_or_parallel,configuration_or_ratio"
    );
    scalability_gate(&mut failures, "sha256-1m", workers, &payload, |data| {
        drop(crypto_bench::sha256(data))
    });
    scalability_gate(&mut failures, "hmac-sha256-1m", workers, &payload, |data| {
        drop(crypto_bench::hmac_sha256(data, &key))
    });
    scalability_gate(&mut failures, "aes-256-gcm-1m", workers, &payload, |data| {
        drop(crypto_bench::aes_encrypt_one_shot(data, aad))
    });

    latency_gate(
        &mut failures,
        "sha256-1m",
        20,
        LatencyBudget {
            environment: "TYRUS_CRYPTO_MAX_SHA256_1M_US",
            default_micros: 25_000,
        },
        || drop(crypto_bench::sha256(black_box(&payload))),
    );
    latency_gate(
        &mut failures,
        "hmac-sha256-1m",
        20,
        LatencyBudget {
            environment: "TYRUS_CRYPTO_MAX_HMAC_SHA256_1M_US",
            default_micros: 25_000,
        },
        || {
            drop(crypto_bench::hmac_sha256(
                black_box(&payload),
                black_box(&key),
            ))
        },
    );
    latency_gate(
        &mut failures,
        "aes-256-gcm-1m",
        20,
        LatencyBudget {
            environment: "TYRUS_CRYPTO_MAX_AES_GCM_1M_US",
            default_micros: 10_000,
        },
        || {
            drop(crypto_bench::aes_encrypt_one_shot(
                black_box(&payload),
                black_box(aad),
            ))
        },
    );
    latency_gate(
        &mut failures,
        "argon2id-m65536-t3-p4",
        3,
        LatencyBudget {
            environment: "TYRUS_CRYPTO_MAX_ARGON2_US",
            default_micros: 1_000_000,
        },
        || drop(crypto_bench::argon2id(black_box(b"benchmark-password"))),
    );
    latency_gate(
        &mut failures,
        "jwt-hs256-round-trip",
        20,
        LatencyBudget {
            environment: "TYRUS_CRYPTO_MAX_JWT_HS256_US",
            default_micros: 5_000,
        },
        || {
            drop(crypto_bench::jwt_hs256_round_trip(black_box(
                r#"{"sub":"performance-gate"}"#,
            )))
        },
    );
    resident_memory_gate(&mut failures);

    if !failures.is_empty() {
        eprintln!("crypto performance budgets exceeded:");
        for failure in failures {
            eprintln!("- {failure}");
        }
        std::process::exit(1);
    }
}
