use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tyrus_runtime::crypto_bench;

struct CountingAllocator;

static TRACKING: AtomicBool = AtomicBool::new(false);
static RUST_ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static RUST_ALLOCATION_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACKING.load(Ordering::Relaxed) {
            RUST_ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
            RUST_ALLOCATION_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if TRACKING.load(Ordering::Relaxed) {
            RUST_ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
            RUST_ALLOCATION_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        System.alloc_zeroed(layout)
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if TRACKING.load(Ordering::Relaxed) {
            RUST_ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
            RUST_ALLOCATION_BYTES.fetch_add(new_size, Ordering::Relaxed);
        }
        System.realloc(pointer, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy)]
struct Budget {
    allocations: usize,
    bytes: fn(usize) -> usize,
}

#[derive(Clone, Copy, Debug)]
struct Measurement {
    allocations: usize,
    bytes: usize,
}

fn measure(operation: impl FnOnce()) -> Measurement {
    RUST_ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    RUST_ALLOCATION_BYTES.store(0, Ordering::Relaxed);
    crypto_bench::reset_ffi_allocation_counters();
    TRACKING.store(true, Ordering::SeqCst);
    operation();
    TRACKING.store(false, Ordering::SeqCst);
    let (ffi_allocations, ffi_bytes) = crypto_bench::ffi_allocation_counters();
    Measurement {
        allocations: RUST_ALLOCATION_COUNT.load(Ordering::Relaxed) + ffi_allocations,
        bytes: RUST_ALLOCATION_BYTES.load(Ordering::Relaxed) + ffi_bytes,
    }
}

fn check(
    failures: &mut Vec<String>,
    operation: &str,
    size: usize,
    measurement: Measurement,
    budget: Budget,
) {
    let byte_budget = (budget.bytes)(size);
    println!(
        "{operation},{size},{},{},{},{}",
        measurement.allocations, measurement.bytes, budget.allocations, byte_budget
    );
    if measurement.allocations > budget.allocations || measurement.bytes > byte_budget {
        failures.push(format!(
            "{operation}/{size}: measured {} allocations and {} bytes; budget is {} allocations and {} bytes",
            measurement.allocations, measurement.bytes, budget.allocations, byte_budget
        ));
    }
}

fn main() {
    let key = [0x42; 32];
    let aad = b"benchmark-aad";
    let mut failures = Vec::new();

    crypto_bench::sha256(b"warm-up");
    crypto_bench::hmac_sha256(b"warm-up", &key);
    crypto_bench::aes_encrypt_one_shot(b"warm-up", aad);
    let mut warm_up_in_place = Vec::with_capacity(32);
    warm_up_in_place.extend_from_slice(b"warm-up");
    crypto_bench::aes_encrypt_in_place(&mut warm_up_in_place, aad);
    crypto_bench::aes_round_trip(b"warm-up", aad);
    crypto_bench::argon2id(b"warm-up-password");
    crypto_bench::jwt_hs256_round_trip(r#"{"sub":"warm-up"}"#);

    println!("operation,payload_bytes,allocations,allocated_bytes,allocation_budget,byte_budget");
    for size in [0usize, 64, 1024, 64 * 1024, 1024 * 1024] {
        let payload = vec![0x5a; size];
        check(
            &mut failures,
            "sha256-abi",
            size,
            measure(|| drop(crypto_bench::sha256(black_box(&payload)))),
            Budget {
                allocations: 2,
                bytes: |_| 64,
            },
        );
        check(
            &mut failures,
            "hmac-sha256-abi",
            size,
            measure(|| {
                drop(crypto_bench::hmac_sha256(
                    black_box(&payload),
                    black_box(&key),
                ))
            }),
            Budget {
                allocations: 3,
                bytes: |_| 96,
            },
        );
        check(
            &mut failures,
            "aes-256-gcm-one-shot",
            size,
            measure(|| {
                drop(crypto_bench::aes_encrypt_one_shot(
                    black_box(&payload),
                    black_box(aad),
                ))
            }),
            Budget {
                allocations: 1,
                bytes: |payload| payload.saturating_add(16),
            },
        );
        let mut in_place = Vec::with_capacity(size.saturating_add(16));
        in_place.extend_from_slice(&payload);
        check(
            &mut failures,
            "aes-256-gcm-in-place",
            size,
            measure(|| {
                crypto_bench::aes_encrypt_in_place(black_box(&mut in_place), black_box(aad))
            }),
            Budget {
                allocations: 0,
                bytes: |_| 0,
            },
        );
        check(
            &mut failures,
            "aes-256-gcm-round-trip-abi",
            size,
            measure(|| {
                drop(crypto_bench::aes_round_trip(
                    black_box(&payload),
                    black_box(aad),
                ))
            }),
            Budget {
                allocations: if size == 0 { 7 } else { 9 },
                bytes: |payload| payload.saturating_mul(5).saturating_add(186),
            },
        );
    }

    check(
        &mut failures,
        "argon2id-m65536-t3-p4",
        b"benchmark-password".len(),
        measure(|| drop(crypto_bench::argon2id(black_box(b"benchmark-password")))),
        Budget {
            allocations: 8,
            bytes: |_| 64 * 1024 * 1024 + 1024,
        },
    );
    let jwt_payload = r#"{"sub":"allocation-budget","admin":false}"#;
    check(
        &mut failures,
        "jwt-hs256-round-trip-abi",
        jwt_payload.len(),
        measure(|| drop(crypto_bench::jwt_hs256_round_trip(black_box(jwt_payload)))),
        Budget {
            allocations: 52,
            bytes: |_| 8 * 1024,
        },
    );

    if !failures.is_empty() {
        eprintln!("crypto allocation budgets exceeded:");
        for failure in failures {
            eprintln!("- {failure}");
        }
        std::process::exit(1);
    }
}
